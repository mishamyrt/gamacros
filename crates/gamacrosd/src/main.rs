mod app;
mod logging;
mod cli;
mod runner;
mod api;
mod activity;

use std::path::PathBuf;
use std::{process, time::Duration};

use colored::Colorize;
use crossbeam_channel::{select, unbounded};
use clap::Parser;
use lunchctl::{LaunchAgent, LaunchControllable};
use crate::activity::{ActivityEvent, Monitor, NotificationListener};

use gamacros_gamepad::{ControllerEvent, ControllerManager};
use gamacros_control::Performer;
use gamacros_workspace::{Workspace, ProfileEvent};

use crate::app::{Gamacros, ButtonPhase};
use crate::cli::{Cli, Command, ControlCommand};
use crate::runner::ActionRunner;
use crate::api::{UnixSocket, ApiTransport, Command as ApiCommand};

const APP_LABEL: &str = "co.myrt.gamacros";

fn main() -> process::ExitCode {
    let cli = Cli::parse();
    if cli.command != Command::Observe {
        logging::setup(cli.verbose, cli.no_color);
    }

    let bin_path = std::env::current_exe().unwrap();

    match cli.command {
        Command::Run { workspace } => {
            let workspace_path = resolve_workspace_path(workspace.as_deref());
            run_event_loop(Some(workspace_path));
        }
        Command::Start { workspace } => {
            let workspace_path = resolve_workspace_path(workspace.as_deref());

            let mut arguments = vec![bin_path.display().to_string()];
            if cli.verbose {
                arguments.push("--verbose".to_string());
            }
            arguments.push("run".to_string());
            arguments.push("--workspace".to_string());
            arguments.push(workspace_path.display().to_string());

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
        Command::Observe => {
            logging::setup(true, cli.no_color);
            run_event_loop(None);
        }
        Command::Command { workspace, command } => match command {
            ControlCommand::Rumble { id, ms } => {
                let workspace_path = resolve_workspace_path(workspace.as_deref());
                match UnixSocket::new(workspace_path)
                    .send_event(ApiCommand::Rumble { id, ms })
                {
                    Ok(_) => {
                        print_info!("Rumbled controller {:?} for {ms}ms", id);
                    }
                    Err(e) => {
                        print_error!("failed to send rumble command: {e}");
                    }
                };
            }
        },
    }

    process::ExitCode::SUCCESS
}

fn resolve_workspace_path(workspace: Option<&str>) -> PathBuf {
    let workspace = workspace.map(PathBuf::from);
    if let Some(workspace) = workspace {
        return workspace;
    }

    match Workspace::default_path() {
        Ok(path) => path,
        Err(e) => {
            print_error!("failed to resolve workspace: {e}");

            process::exit(1);
        }
    }
}

fn run_event_loop(maybe_workspace_path: Option<PathBuf>) {
    // Activity monitor must run on the main thread.
    // We keep its std::mpsc receiver and poll it from the event loop (no bridge thread).
    let Some((monitor, activity_std_rx, monitor_stop_tx)) = Monitor::new() else {
        print_error!("failed to start activity monitor");
        return;
    };

    monitor.subscribe(NotificationListener::DidActivateApplication);
    let mut gamacros = Gamacros::new();
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

    let workspace_path = maybe_workspace_path.to_owned();

    // Start control socket on the main thread and forward commands into the event loop.
    let (api_tx, api_rx) = unbounded::<ApiCommand>();
    let _control_handle = workspace_path.clone().map(|workspace_path| {
        UnixSocket::new(workspace_path)
            .listen_events(api_tx)
            .expect("failed to start api server")
    });

    // Run the main event loop in a background thread while the main thread runs the monitor loop.
    let event_loop = std::thread::Builder::new()
        .name("event-loop".into())
        .stack_size(512 * 1024)
        .spawn(move || {
        let manager =
            ControllerManager::new().expect("failed to start controller manager");
        let rx = manager.subscribe();
        let mut keypress = Performer::new().expect("failed to start keypress");
        // Single coalesced wake timer: earliest of movement tick and repeat deadlines.
        let mut wake_rx = crossbeam_channel::never::<std::time::Instant>();
        let idle_period = Duration::from_millis(16);
        let fast_period = Duration::from_millis(10);
        let mut ticking_enabled = false;
        let mut fast_mode = false;
        let mut fast_until = std::time::Instant::now();
        let mut next_tick_due: Option<std::time::Instant> = None;
        let mut need_reschedule_wake = true;

        let workspace = match Workspace::new(workspace_path.as_deref()) {
            Ok(workspace) => workspace,
            Err(e) => {
                print_error!("failed to start workspace: {e}");
                return;
            }
        };

        let maybe_watcher = workspace_path
            .as_ref()
            .map(|_| workspace.start_profile_watcher())
            .transpose()
            .expect("failed to start workspace watcher");

        let maybe_workspace_rx = maybe_watcher.map(|(_watcher, rx)| rx);

        let mut action_runner = ActionRunner::new(&mut keypress, &manager);

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

                            gamacros.add_controller(info);
                            need_reschedule_wake = true;
                        }
                        Ok(ControllerEvent::Disconnected(id)) => {
                            gamacros.remove_controller(id);
                            gamacros.on_controller_disconnected(id);
                            need_reschedule_wake = true;
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
                            // Axis moved: if previously gated by neutral, re-arm wake.
                            need_reschedule_wake = true;
                        }
                        Err(err) => {
                            print_error!("event channel closed: {err}");
                            break;
                        }
                    }
                }
                recv(api_rx) -> cmd => {
                    match cmd {
                        Ok(ApiCommand::Rumble { id, ms }) => {
                            match id {
                                Some(cid) => {
                                    action_runner.run(crate::app::Action::Rumble { id: cid, ms });
                                }
                                None => {
                                    for info in manager.controllers() {
                                        action_runner.run(crate::app::Action::Rumble { id: info.id, ms });
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // control channel closed; continue running
                        }
                    }
                }
                recv(wake_rx) -> _ => {
                    let now = std::time::Instant::now();
                    // Run movement tick if due
                    if let Some(due) = next_tick_due {
                        if now >= due {
                            gamacros.on_tick_with(|action| {
                                action_runner.run(action);
                            });
                            // Update adaptive mode hints
                            if gamacros.wants_fast_tick() {
                                fast_mode = true;
                                fast_until = now + Duration::from_millis(250);
                            } else if fast_mode && now >= fast_until {
                                fast_mode = false;
                            }
                        }
                    }
                    // Run repeats due (may be multiple)
                    gamacros.process_due_repeats(now, |action| { action_runner.run(action); });
                    need_reschedule_wake = true;
                }
            }
            while let Ok(msg) = activity_std_rx.try_recv() {
                let ActivityEvent::DidActivateApplication(bundle_id) = msg else {
                    continue;
                };
                gamacros.set_active_app(&bundle_id);
                // App change may alter stick modes; mark for reschedule
                need_reschedule_wake = true;
            }
            let Some(workspace_rx) = maybe_workspace_rx.as_ref() else {
                continue;
            };

            while let Ok(msg) = workspace_rx.try_recv() {
                match msg {
                    ProfileEvent::Changed(workspace) => {
                        print_info!("profile changed, updating workspace");
                        if let Some(shell) = workspace.shell.clone() {
                            action_runner.set_shell(shell);
                        }
                        gamacros.set_workspace(workspace);
                        need_reschedule_wake = true;
                    }
                    ProfileEvent::Removed => {
                        gamacros.remove_workspace();
                        need_reschedule_wake = true;
                    }
                    ProfileEvent::Error(error) => {
                        print_error!("profile error: {error}");
                    }
                }
            }
            if need_reschedule_wake {
                let now = std::time::Instant::now();
                // Recompute next tick due
                if gamacros.needs_tick() {
                    if !ticking_enabled {
                        fast_mode = gamacros.wants_fast_tick();
                        if fast_mode {
                            fast_until = now + Duration::from_millis(250);
                        }
                    }
                    let period = if fast_mode { fast_period } else { idle_period };
                    next_tick_due = Some(now + period);
                    ticking_enabled = true;
                } else {
                    next_tick_due = None;
                    ticking_enabled = false;
                }
                // Recompute next repeat due
                let repeat_due = gamacros.next_repeat_due();

                // Arm single wake for the earliest deadline
                let next_due = match (next_tick_due, repeat_due) {
                    (Some(a), Some(b)) => Some(core::cmp::min(a, b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };
                if let Some(due) = next_due {
                    let dur = if due > now { due - now } else { Duration::ZERO };
                    wake_rx = crossbeam_channel::after(dur);
                } else {
                    wake_rx = crossbeam_channel::never();
                }
                need_reschedule_wake = false;
            }
        }
    }).expect("failed to spawn event loop thread");

    // Start monitoring on the main thread (blocks until error/exit)
    monitor.run();
    if let Err(e) = event_loop.join() {
        print_error!("event loop error: {e:?}");
    }
}
