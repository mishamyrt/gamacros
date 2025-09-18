use std::{process::Command, time::Duration};

use colored::Colorize;
use gamacros_control::Performer;
use gamacros_gamepad::ControllerManager;

use crate::{app::Action, print_error, print_info};

const DEFAULT_SHELL: &str = "/bin/zsh";

pub(crate) struct ActionRunner<'a> {
    keypress: &'a mut Performer,
    manager: &'a ControllerManager,
    shell: Option<Box<str>>,
}

impl<'a> ActionRunner<'a> {
    pub fn new(keypress: &'a mut Performer, manager: &'a ControllerManager) -> Self {
        Self {
            keypress,
            manager,
            shell: None,
        }
    }

    pub fn run(&mut self, action: Action) {
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
                    let _ = h.rumble(1.0, 1.0, Duration::from_millis(ms as u64));
                }
            }
        }
    }

    fn run_shell(&mut self, cmd: &str) -> Result<String, String> {
        let shell = self.shell.clone().unwrap_or(DEFAULT_SHELL.into());
        let result = Command::new(shell.into_string().as_str())
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

    pub fn set_shell(&mut self, shell: Box<str>) {
        self.shell = Some(shell);
    }
}
