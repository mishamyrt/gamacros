use cocoa::base::id;
use objc::{class, msg_send, sel, sel_impl};
use std::os::raw::c_char;
use std::sync::mpsc;

use crate::Event;

use super::util::make_nsstring;
use super::NSWorkspaceError;

pub(crate) struct AppState {
    event_tx: mpsc::Sender<Event>,
}

#[allow(improper_ctypes, unexpected_cfgs)]
impl AppState {
    pub(crate) fn new(event_tx: mpsc::Sender<Event>) -> Self {
        AppState { event_tx }
    }

    pub(crate) fn notify_active_app(&self, notification: id) -> Result<(), NSWorkspaceError> {
        unsafe {
            let user_info: id = msg_send![notification, userInfo];
            if user_info.is_null() {
                return Err(NSWorkspaceError::GetUserInfo);
            }

            let app_key = make_nsstring("NSWorkspaceApplicationKey");
            let app: id = msg_send![user_info, objectForKey:app_key];
            if app.is_null() {
                return Err(NSWorkspaceError::GetFrontmostApplication);
            }

            let bundle_id: id = msg_send![app, bundleIdentifier];
            if bundle_id.is_null() {
                return Err(NSWorkspaceError::GetBundleIdentifier);
            }

            let utf8: *const c_char = msg_send![bundle_id, UTF8String];
            if utf8.is_null() {
                return Err(NSWorkspaceError::GetUTF8String);
            }

            let cstr = std::ffi::CStr::from_ptr(utf8);
            match cstr.to_str() {
                Ok(bundle_str) => {
                    let event = Event::AppChange(bundle_str.to_string());
                    if let Err(e) = self.event_tx.send(event) {
                        return Err(NSWorkspaceError::SendEventError(e));
                    }

                    Ok(())
                }
                Err(e) => Err(NSWorkspaceError::ConvertStringError(e)),
            }
        }
    }

    pub(crate) fn setup_notifications(&self, delegate: id) -> Result<(), NSWorkspaceError> {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let frontmost_app: id = msg_send![workspace, frontmostApplication];
            if !frontmost_app.is_null() {
                let bundle_id: id = msg_send![frontmost_app, bundleIdentifier];
                if !bundle_id.is_null() {
                    let utf8: *const c_char = msg_send![bundle_id, UTF8String];
                    if !utf8.is_null() {
                        let cstr = std::ffi::CStr::from_ptr(utf8);
                        if let Ok(bundle_str) = cstr.to_str() {
                            if let Err(e) =
                                self.event_tx.send(Event::AppChange(bundle_str.to_string()))
                            {
                                return Err(NSWorkspaceError::SendEventError(e));
                            }
                        }
                    }
                }
            }

            let workspace_notification_center: id = msg_send![workspace, notificationCenter];
            let app_active = make_nsstring("NSWorkspaceDidActivateApplicationNotification");

            let _: () = msg_send![workspace_notification_center,
                addObserver:delegate
                selector:sel!(updateActiveApplication:)
                name:app_active
                object:workspace];
        }

        Ok(())
    }
}
