#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use cocoa::appkit::NSApplicationActivationPolicy;
use cocoa::base::id;
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;
use std::sync::mpsc;

use crate::nsworkspace::app_state::AppState;
use crate::Event;

use super::NSWorkspaceError;

pub(crate) struct AppDelegate {
    _delegate: id,
}

#[allow(improper_ctypes, unexpected_cfgs)]
impl AppDelegate {
    pub(crate) fn new(
        event_tx: mpsc::Sender<Event>,
    ) -> Result<Self, NSWorkspaceError> {
        unsafe {
            let mut decl =
                objc::declare::ClassDecl::new("RustAppDelegate", class!(NSObject))
                    .unwrap();

            decl.add_ivar::<*mut c_void>("_rustState");

            extern "C" fn update_active_application(
                this: &Object,
                _sel: objc::runtime::Sel,
                notification: id,
            ) {
                unsafe {
                    let state_ptr: *mut c_void = *this.get_ivar("_rustState");
                    let state = &*(state_ptr as *const AppState);
                    if let Err(e) = state.notify_active_app(notification) {
                        println!("❌ Error in update_active_application: {e:?}");
                    }
                }
            }

            decl.add_method(
                sel!(updateActiveApplication:),
                update_active_application as extern "C" fn(&Object, _, _),
            );

            decl.register();

            let delegate_class = class!(RustAppDelegate);
            let delegate: id = msg_send![delegate_class, new];

            let state = Box::new(AppState::new(event_tx));
            let state_ptr = Box::into_raw(state) as *mut c_void;
            (*delegate).set_ivar("_rustState", state_ptr);

            let state = &*(state_ptr as *const AppState);
            state.setup_notifications(delegate)?;

            Ok(AppDelegate {
                _delegate: delegate,
            })
        }
    }

    fn setup_application(&self) {
        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, setActivationPolicy:
                NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory];
        }
    }

    pub(crate) fn start_listening(self) {
        self.setup_application();

        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, setDelegate:self._delegate];
            let _: () = msg_send![app, run];
        }
    }
}
