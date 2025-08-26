mod app_delegate;
mod app_state;
mod util;
mod listener;

use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NSWorkspaceError {
    #[error("Failed to get frontmost application")]
    GetFrontmostApplication,
    #[error("Failed to get bundle identifier")]
    GetBundleIdentifier,
    #[error("Failed to get UTF8 string")]
    GetUTF8String,
    #[error("Failed to convert string")]
    ConvertStringError(Utf8Error),
    #[error("Failed to send event")]
    SendEventError(std::sync::mpsc::SendError<Event>),
    #[error("Failed to get user info")]
    GetUserInfo,
}

pub(crate) use listener::start_nsworkspace_listener;

use crate::Event;

// Request the NSApplication run loop to stop on the next iteration.
// Safe to call from any thread; it dispatches onto the main queue.
pub fn request_stop() {
    unsafe { request_stop_impl(); }
}

#[allow(improper_ctypes)]
unsafe fn request_stop_impl() {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    let app: id = msg_send![class!(NSApplication), sharedApplication];
    let _: () = msg_send![app,
        performSelectorOnMainThread: sel!(stop:)
        withObject: nil
        waitUntilDone: false
    ];
}