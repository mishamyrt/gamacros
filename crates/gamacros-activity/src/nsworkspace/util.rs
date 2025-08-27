use cocoa::base::id;
use objc::{class, msg_send, sel, sel_impl};

#[allow(unexpected_cfgs, improper_ctypes)]
pub(crate) unsafe fn make_nsstring(string: &str) -> id {
    let cls = class!(NSString);
    let string = std::ffi::CString::new(string).unwrap();
    msg_send![cls, stringWithUTF8String:string.as_ptr()]
}

#[cfg(test)]
#[allow(improper_ctypes, unexpected_cfgs)]
mod tests {
    use std::ffi::c_char;

    use super::*;

    #[test]
    fn test_make_nsstring() {
        unsafe {
            let string = make_nsstring("test");
            assert!(!string.is_null());
            let utf8: *const c_char = msg_send![string, UTF8String];
            assert!(!utf8.is_null());
            let utf8_str = std::ffi::CStr::from_ptr(utf8);
            assert_eq!(utf8_str.to_str().unwrap(), "test");
        }
    }
}
