use std::cell::Cell;
use std::ffi::CString;
use std::panic::{self, UnwindSafe};

use backtrace::Backtrace;

use crate::unsafe_wrappers::dome::Context;
use crate::Api;

#[derive(Debug)]
pub(crate) struct PanicInfo {
    message: CString,
    backtrace: Backtrace,
}

thread_local! {
    static PANIC_INFO: Cell<Option<PanicInfo>> = Cell::new(None);
}

#[inline]
pub(crate) fn catch_panic<R>(callback: impl FnOnce() -> R + UnwindSafe) -> Result<R, PanicInfo> {
    let prev_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(|info| {
        let message = if let Some(&s) = info.payload().downcast_ref::<&str>() {
            CString::new(s)
                .unwrap_or_else(|_| CString::new("Panic message contains null byte(s).").unwrap())
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            CString::new(s.clone())
                .unwrap_or_else(|_| CString::new("Panic message contains null byte(s).").unwrap())
        } else {
            CString::new("Could not retrieve panic message.").unwrap()
        };

        // TODO: Should we hide the symbols resolve step behind some configuration,
        // like Rust does with the RUST_BACKTRACE environment variable?
        let backtrace = Backtrace::new();

        PANIC_INFO.with(|panic_info| panic_info.set(Some(PanicInfo { message, backtrace })));
    }));
    let result = panic::catch_unwind(callback).map_err(|_err| {
        // Safe to `.unwrap()` because the standard library calls the panic hook which sets
        // this variable before `catch_unwind()` returns.
        PANIC_INFO.with(|panic_info| panic_info.replace(None).unwrap())
    });
    panic::set_hook(prev_panic_hook);
    result
}

#[inline]
pub(crate) fn log_panic(ctx: Context, panic_info: &PanicInfo) {
    let fmt = CString::new("Plugin panicked: %s\n%s\n\n").unwrap();
    let backtrace = CString::new(format!("Backtrace:\n{:?}", panic_info.backtrace))
        .unwrap_or_else(|_| CString::new("Backtrace contains null byte(s).").unwrap());
    // SAFETY: We respect C formatting.
    unsafe {
        (Api::dome().log)(
            ctx,
            fmt.as_ptr(),
            panic_info.message.as_ptr(),
            backtrace.as_ptr(),
        );
    }
}

#[inline]
pub(crate) fn handle_wren_callback_panic(vm: crate::unsafe_wren::VM, panic_info: &PanicInfo) {
    let mut vm = crate::safe_wrappers::wren::VM(vm);

    log_panic(vm.get_context().0, panic_info);

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
