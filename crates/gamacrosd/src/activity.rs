#[cfg(target_os = "macos")]
pub use nsworkspace::{Event as ActivityEvent, Monitor, NotificationListener};

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone)]
pub enum ActivityEvent {
    DidActivateApplication(String),
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy)]
pub enum NotificationListener {
    DidActivateApplication,
}

#[cfg(not(target_os = "macos"))]
pub struct Monitor {
    stop_rx: std::sync::mpsc::Receiver<()>,
}

#[cfg(not(target_os = "macos"))]
impl Monitor {
    pub fn new() -> Option<(
        Self,
        std::sync::mpsc::Receiver<ActivityEvent>,
        std::sync::mpsc::Sender<()>,
    )> {
        let (activity_tx, activity_rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();
        let monitor = Monitor { stop_rx };
        let _ = activity_tx; // unused on non-macOS
        Some((monitor, activity_rx, stop_tx))
    }

    pub fn subscribe(&self, _listener: NotificationListener) {}

    pub fn get_active_application(&self) -> Option<String> {
        None
    }

    pub fn run(&self) {
        let _ = self.stop_rx.recv();
    }
}
