use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use crossbeam_channel::{unbounded, Sender};

use crate::command::Command;
use crate::error::Result;
use crate::events::{ControllerEvent, EventReceiver};
use crate::handle::ControllerHandle;
use crate::runtime::start_runtime_thread;
use crate::types::{ControllerId, ControllerInfo};

/// Shared state used by the manager, the runtime loop and controller handles.
pub(crate) struct Inner {
    pub subscribers: Mutex<Vec<Sender<ControllerEvent>>>,
    pub controllers_info: RwLock<HashMap<ControllerId, ControllerInfo>>,
    pub cmd_tx: Sender<Command>,
}

/// Manager responsible for discovering controllers and emitting events.
pub struct ControllerManager {
    pub(crate) inner: Arc<Inner>,
}

impl ControllerManager {
    /// Creates a new manager and starts the background runtime thread.
    /// Blocks briefly until the initial device enumeration completes (up to 1s).
    pub fn new() -> Result<Self> {
        let (cmd_tx, cmd_rx) = unbounded::<Command>();
        let inner = Arc::new(Inner {
            subscribers: Mutex::new(Vec::new()),
            controllers_info: RwLock::new(HashMap::new()),
            cmd_tx,
        });

        let inner_clone = inner.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        start_runtime_thread(inner_clone, cmd_rx, Some(ready_tx));

        // Best-effort wait for the initial enumeration. Time out if backend fails.
        let _ = ready_rx.recv_timeout(Duration::from_secs(1));

        Ok(Self { inner })
    }

    /// Subscribes to controller events. Dropped subscribers are cleaned automatically.
    pub fn subscribe(&self) -> EventReceiver {
        let (tx, rx) = unbounded();
        if let Ok(mut subs) = self.inner.subscribers.lock() {
            subs.push(tx);
        }
        rx
    }

    /// Returns a snapshot of currently known controllers.
    pub fn controllers(&self) -> Vec<ControllerInfo> {
        if let Ok(map) = self.inner.controllers_info.read() {
            return map.values().cloned().collect();
        }
        Vec::new()
    }

    /// Returns a handle to a controller by id if it is currently known.
    pub fn controller(&self, id: ControllerId) -> Option<ControllerHandle> {
        if let Ok(map) = self.inner.controllers_info.read() {
            if map.contains_key(&id) {
                return Some(ControllerHandle {
                    id,
                    inner: self.inner.clone(),
                });
            }
        }
        None
    }
}
