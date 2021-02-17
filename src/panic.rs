use std::ffi::CString;
use std::panic::{self, UnwindSafe};

use crate::unsafe_wrappers::dome::Context;
use crate::Api;

#[inline]
pub(crate) fn catch_panic<R>(callback: impl FnOnce() -> R + UnwindSafe) -> Result<R, CString> {
    let prev_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(|_info| {}));
    let result = panic::catch_unwind(callback).map_err(|err| {
        if let Some(&s) = err.downcast_ref::<&str>() {
            CString::new(s)
                .unwrap_or_else(|_| CString::new("Panic message contains null byte(s).").unwrap())
        } else if let Some(s) = err.downcast_ref::<String>() {
            CString::new(s.clone())
                .unwrap_or_else(|_| CString::new("Panic message contains null byte(s).").unwrap())
        } else {
            CString::new("Could not retrieve panic message.").unwrap()
        }
    });
    panic::set_hook(prev_panic_hook);
    result
}

#[inline]
pub(crate) fn log_panic(ctx: Context, panic_message: &CString) {
    let fmt = CString::new("Plugin panicked: %s\n").unwrap();
    // SAFETY: We respect C formatting.
    unsafe {
        (Api::dome().log)(ctx, fmt.as_ptr(), panic_message.as_ptr());
    }
}

#[inline]
pub(crate) fn handle_wren_callback_panic(vm: crate::unsafe_wren::VM, panic_message: &CString) {
    let mut vm = crate::safe_wrappers::wren::VM(vm);

    log_panic(vm.get_context().0, panic_message);

    vm.ensure_slots(2);
    vm.set_slot_string(1, "Plugin panicked. See DOME's log for details.");
    vm.abort_fiber(1);
}

#[inline]
pub(crate) fn catch_and_log_panic<R>(
    ctx: Context,
    callback: impl FnOnce() -> R + UnwindSafe,
) -> Option<R> {
    catch_panic(callback)
        .map_err(|panic_message| log_panic(ctx, &panic_message))
        .ok()
}
