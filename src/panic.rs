use libc::c_char;
use std::ffi::CString;
use std::panic::{self, UnwindSafe};

use crate::unsafe_dome::Context;
use crate::API;

#[inline]
pub(crate) fn catch_panic<R>(
    ctx: Context,
    callback: impl FnOnce() -> R + UnwindSafe,
) -> Result<R, ()> {
    let prev_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(|_info| {}));
    let result = panic::catch_unwind(callback).map_err(|err| {
        let fmt_owned = CString::new("Plugin panicked: %s\n").unwrap();
        let fmt = fmt_owned.as_ptr() as *const c_char;
        let panic_message_owned = if let Some(&s) = err.downcast_ref::<&str>() {
            CString::new(s)
                .unwrap_or_else(|_| CString::new("Panic message contains null byte(s).").unwrap())
        } else if let Some(s) = err.downcast_ref::<String>() {
            CString::new(s.clone())
                .unwrap_or_else(|_| CString::new("Panic message contains null byte(s).").unwrap())
        } else {
            CString::new("Could not retrieve panic message.").unwrap()
        };
        let panic_message = panic_message_owned.as_ptr() as *const c_char;
        unsafe {
            ((*API.dome).log)(ctx, fmt, panic_message);
        }
    });
    panic::set_hook(prev_panic_hook);
    result
}
