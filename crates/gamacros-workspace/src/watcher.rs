use std::time::Duration;
use std::{fs, path::Path};
use std::sync::mpsc;

use notify::{
    Config, Error as NotifyError, FsEventWatcher, RecursiveMode,
};
use notify_debouncer_mini::{new_debouncer_opt, DebounceEventResult, DebouncedEventKind, Debouncer};
use thiserror::Error;

use crate::{parse_profile, ProfileError, Workspace};

#[derive(Error, Debug)]
pub enum Error {
    #[error("notify error: {0}")]
    Notify(#[from] NotifyError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] ProfileError),
}

pub struct WorkspaceWatcher {
    #[allow(dead_code)]
    watcher: Debouncer<FsEventWatcher>,
}

pub enum WorkspaceEvent {
    Changed(Workspace),
    Removed,
    Error(Error),
}

type WorkspaceEventSender = mpsc::Sender<WorkspaceEvent>;
type WorkspaceEventReceiver = mpsc::Receiver<WorkspaceEvent>;

fn send_workspace_event(path: &Path, tx: &WorkspaceEventSender) {
    match fs::read_to_string(path) {
        Ok(content) => match parse_profile(&content) {
            Ok(workspace) => {
                let _ = tx.send(WorkspaceEvent::Changed(workspace));
            }
            Err(e) => {
                let error = Error::Parse(e);
                let _ = tx.send(WorkspaceEvent::Error(error));
            }
        },
        Err(e) => {
            let error = Error::Io(e);
            let _ = tx.send(WorkspaceEvent::Error(error));
        }
    };
}

impl WorkspaceWatcher {
    pub fn new_with_sender(
        path: &Path,
        tx: WorkspaceEventSender,
    ) -> Result<Self, Error> {
        let path_c = path.to_owned();
        let tx_c = tx.clone();

        let debouncer_config = notify_debouncer_mini::Config::default()
            .with_timeout(Duration::from_millis(1000))
            .with_notify_config(Config::default());
        // select backend via fish operator, here PollWatcher backend
        let mut debouncer = new_debouncer_opt::<_, notify::FsEventWatcher>(
            debouncer_config,
            move |events: DebounceEventResult| {
                match events {
                    Ok(events) => {
                        for event in events {
                            match event.kind {
                                DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous => {
                                    if !path_c.exists() {
                                        let _ = tx_c.send(WorkspaceEvent::Removed);
                                    } else {
                                        send_workspace_event(&path_c, &tx_c);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(event) => {
                        let error = Error::Notify(event);
                        let _ = tx_c.send(WorkspaceEvent::Error(error));
                    }
                }
            },
        )?;

        debouncer.watcher().watch(path, RecursiveMode::NonRecursive)?;

        Ok(Self { watcher: debouncer })
    }

    pub fn new(path: &Path) -> Result<(Self, WorkspaceEventReceiver), Error> {
        let (tx, rx) = mpsc::channel();

        Ok((Self::new_with_sender(path, tx)?, rx))
    }

    pub fn new_with_starting_event(
        path: &Path,
    ) -> Result<(Self, WorkspaceEventReceiver), Error> {
        let (tx, rx) = mpsc::channel();

        // Send initial workspace event
        send_workspace_event(path, &tx);
        Ok((Self::new_with_sender(path, tx)?, rx))
    }
}
