mod app;
mod logging;
mod cli;

use std::{fs, process, time::Duration};
use std::process::Command as StdCommand;

use colored::Colorize;
use crossbeam_channel::{select, unbounded};
use fern::Dispatch;
use clap::Parser;
use lunchctl::{LaunchAgent, LaunchControllable};
use nsworkspace::{Event as ActivityEvent, Monitor, NotificationListener};

use gamacros_gamepad::{ControllerEvent, ControllerManager};
use gamacros_control::Performer;
use gamacros_workspace::{parse_profile, resolve_profile, Workspace};

use app::{Gamacros, Action};
use cli::Cli;

use crate::{app::ButtonPhase, cli::Command};

const APP_LABEL: &str = "co.myrt.gamacros";
const DEFAULT_SHELL: &str = "/bin/zsh";

fn main() -> process::ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.verbose, cli.no_color);

    let bin_path = std::env::current_exe().unwrap();

    match cli.command {
        Command::Run { profile } => {
            let Some(profile) = load_workspace(profile.as_deref()) else {
                return process::ExitCode::FAILURE;
            };
            run_event_loop(profile);
        }
        Command::Start { profile } => {
            let profile_path = match resolve_profile(profile.as_deref()) {
                Ok(path) => path,
                Err(e) => {
                    print_error!("failed to resolve profile: {e}");
                    return process::ExitCode::FAILURE;
                }
            };

            let mut arguments = vec![bin_path.display().to_string()];
            if cli.verbose {
                arguments.push("--verbose".to_string());
            }
            arguments.push("run".to_string());
            arguments.push("--profile".to_string());
            arguments.push(profile_path.display().to_string());

            let agent = LaunchAgent {
                label: APP_LABEL.to_string(),
                program_arguments: arguments,
                standard_out_path: "/tmp/gamacros.out".to_string(),
                standard_error_path: "/tmp/gamacros.err".to_string(),
                keep_alive: true,
                run_at_load: true,
            };

            if let Err(e) = agent.write() {
                print_error!("Failed to write agent: {}", e);
                return process::ExitCode::FAILURE;
            }

            match agent.is_running() {
                Ok(true) => {
                    print_info!("Agent is already running");
                }
                Ok(false) => {
                    print_info!("Starting agent");
                    if let Err(e) = agent.bootstrap() {
                        print_error!("Failed to bootstrap agent: {}", e);
                        return process::ExitCode::FAILURE;
                    }
                    print_info!("Agent started");
                }
                Err(e) => {
                    print_error!("Failed to check if agent is running: {}", e);
                    return process::ExitCode::FAILURE;
                }
            }
        }
        Command::Stop => {
            if !LaunchAgent::exists(APP_LABEL) {
                print_error!("Agent does not exist");
                return process::ExitCode::FAILURE;
            }

            let agent = LaunchAgent::from_file(APP_LABEL).unwrap();

            match agent.is_running() {
                Ok(true) => {
                    print_info!("Stopping agent");
                    if let Err(e) = agent.boot_out() {
                        print_error!("Failed to stop agent: {}", e);
                        return process::ExitCode::FAILURE;
                    }
                    print_info!("Agent stopped");
                }
                Ok(false) => {
                    print_info!("Agent is not running");
                }
                Err(e) => {
                    print_error!("Failed to check if agent is running: {}", e);
                    return process::ExitCode::FAILURE;
                }
            }
        }
        Command::Status => {
            if !LaunchAgent::exists(APP_LABEL) {
                print_info!("Agent does not exist");
                return process::ExitCode::FAILURE;
            }

            let agent = LaunchAgent::from_file(APP_LABEL).unwrap();
            match agent.is_running() {
                Ok(true) => {
                    print_info!("Agent is running");
                }
                Ok(false) => {
                    print_info!("Agent is not running");
                }
                Err(e) => {
                    print_error!("Failed to check if agent is running: {}", e);
                    return process::ExitCode::FAILURE;
                }
            }
        }
    }

    process::ExitCode::SUCCESS
}

fn setup_logging(verbose: bool, no_color: bool) {
    let log_level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    Dispatch::new()
        .level(log_level)
        .chain(std::io::stdout())
        .apply()
        .expect("Unable to set up logger");

    if no_color {
        colored::control::set_override(false);
    }
}

fn load_workspace(target_path: Option<&str>) -> Option<Workspace> {
    let profile_path = match resolve_profile(target_path) {
        Ok(path) => path,
        Err(e) => {
            print_error!("failed to resolve profile: {e}");
            return None;
        }
    };
    print_info!("loading profile from {}", profile_path.display());
    let content = match fs::read_to_string(&profile_path) {
        Ok(content) => content,
        Err(e) => {
            print_error!("failed to read profile: {e}");
            return None;
        }
    };

    match parse_profile(&content) {
        Ok(profile) => Some(profile),
        Err(e) => {
            print_error!("failed to parse profile: {e}");
            None
        }
    }
}

fn run_event_loop(workspace: Workspace) {
    // Activity monitor must run on the main thread.
    // We keep its std::mpsc receiver and poll it from the event loop (no bridge thread).
    let Some((monitor, activity_std_rx, monitor_stop_tx)) = Monitor::new() else {
        print_error!("failed to start activity monitor");
        return;
    };

    monitor.subscribe(NotificationListener::DidActivateApplication);
    let mut gamacros = Gamacros::new(workspace);
    if let Some(app) = monitor.get_active_application() {
        gamacros.set_active_app(&app)
    }

    // Handle Ctrl+C to exit cleanly
    let (stop_tx, stop_rx) = unbounded::<()>();
    ctrlc::set_handler(move || {
        let _ = stop_tx.send(());
        let _ = monitor_stop_tx.send(());
    })
    .expect("failed to set Ctrl+C handler");

    // Run the main event loop in a background thread while the main thread runs the monitor loop.
    let event_loop = std::thread::Builder::new()
        .name("event-loop".into())
        .stack_size(512 * 1024)
        .spawn(move || {
        let manager =
            ControllerManager::new().expect("failed to start controller manager");
        let rx = manager.subscribe();
        let mut keypress = Performer::new().expect("failed to start keypress");
        // Stick processing is owned by Gamacros now
        let ticker = crossbeam_channel::tick(Duration::from_millis(10));

        let shell = gamacros.workspace.shell.clone().unwrap_or(DEFAULT_SHELL.into());
        let mut action_runner = ActionRunner::new(&mut keypress, &manager, shell);

        print_info!(
            "gamacrosd started. Listening for controller and activity events."
        );
        loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(rx) -> msg => {
                    match msg {
                        Ok(ControllerEvent::Connected(info)) => {
                            let id = info.id;
                            if gamacros.is_known(id) {
                                continue;
                            }

                            gamacros.add_controller(info)
                        }
                        Ok(ControllerEvent::Disconnected(id)) => {
                            gamacros.remove_controller(id);
                            gamacros.on_controller_disconnected(id);
                        }
                        Ok(ControllerEvent::ButtonPressed { id, button }) => {
                            gamacros.on_button_with(id, button, ButtonPhase::Pressed, |action| {
                                action_runner.run(action);
                            });
                        }
                        Ok(ControllerEvent::ButtonReleased { id, button }) => {
                            gamacros.on_button_with(id, button, ButtonPhase::Released, |action| {
                                action_runner.run(action);
                            });
                        }
                        Ok(ControllerEvent::AxisMotion { id, axis, value }) => {
                            gamacros.on_axis_motion(id, axis, value);
                        }
                        Err(err) => {
                            print_error!("event channel closed: {err}");
                            break;
                        }
                    }
                }
                recv(ticker) -> _ => {
                    gamacros.on_tick_with(|action| {
                        action_runner.run(action);
                    });
                }
            }
            while let Ok(msg) = activity_std_rx.try_recv() {
                if let ActivityEvent::DidActivateApplication(bundle_id) = msg {
                    gamacros.set_active_app(&bundle_id)
                }
            }
        }
    }).expect("failed to spawn event loop thread");

    // Start monitoring on the main thread (blocks until error/exit)
    monitor.run();
    if let Err(e) = event_loop.join() {
        print_error!("event loop error: {e:?}");
    }
}

struct ActionRunner<'a> {
    keypress: &'a mut Performer,
    manager: &'a ControllerManager,
    shell: Box<str>,
}

impl<'a> ActionRunner<'a> {
    fn new(
        keypress: &'a mut Performer,
        manager: &'a ControllerManager,
        shell: Box<str>,
    ) -> Self {
        Self {
            keypress,
            manager,
            shell,
        }
    }

    fn run(&mut self, action: Action) {
        match action {
            Action::KeyTap(k) => {
                let _ = self.keypress.perform(&k);
            }
            Action::KeyPress(k) => {
                let _ = self.keypress.press(&k);
            }
            Action::KeyRelease(k) => {
                let _ = self.keypress.release(&k);
            }
            Action::Macros(m) => {
                for k in m.iter() {
                    let _ = self.keypress.perform(k);
                }
            }
            Action::Shell(s) => {
                let _ = self.run_shell(&s);
            }
            Action::MouseMove { dx, dy } => {
                let _ = self.keypress.mouse_move(dx, dy);
            }
            Action::Scroll { h, v } => {
                if h != 0 {
                    let _ = self.keypress.scroll_x(h);
                }
                if v != 0 {
                    let _ = self.keypress.scroll_y(v);
                }
            }
            Action::Rumble { id, ms } => {
                if let Some(h) = self.manager.controller(id) {
                    let _ = h.rumble(0.2, 0.2, Duration::from_millis(ms as u64));
                }
            }
        }
    }

    fn run_shell(&mut self, cmd: &str) -> Result<String, String> {
        let shell = self.shell.clone();
        let result = StdCommand::new(shell.into_string().as_str())
            .args(["-c", cmd])
            .output();

        match result {
            Ok(output) => {
                print_info!(
                    "shell command output: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            Err(e) => {
                print_error!("shell command error: {}", e);
                Err(e.to_string())
            }
        }
    }
}
