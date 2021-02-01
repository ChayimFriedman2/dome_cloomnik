use libc::{c_char, c_int, c_void};
use std::ffi::CString;
use std::mem;
use std::panic;
use std::ptr;

mod audio;
mod dome;
mod wren;

#[repr(C)]
pub(crate) enum ApiType {
    Dome,
    Wren,
    Audio,
}

pub(crate) type GetApiFunction = extern "C" fn(api: ApiType, version: c_int) -> *mut c_void;

pub struct Api {
    dome: *mut dome::ApiV0,
    wren: *mut wren::ApiV0,
    audio: *mut audio::ApiV0,
}
static mut API: Api = Api {
    dome: ptr::null_mut(),
    wren: ptr::null_mut(),
    audio: ptr::null_mut(),
};

type Hook = fn() -> Result<(), ()>;
struct Hooks {
    pre_update: Option<Hook>,
    post_update: Option<Hook>,
    pre_draw: Option<Hook>,
    post_draw: Option<Hook>,
    on_shutdown: Option<Hook>,
}
static mut HOOKS: Hooks = Hooks {
    pre_update: None,
    post_update: None,
    pre_draw: None,
    post_draw: None,
    on_shutdown: None,
};

fn invoke_callback(ctx: dome::Context, callback: Hook) -> dome::Result {
    let prev_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(|_info| {}));
    let result = match panic::catch_unwind(callback) {
        Ok(Ok(())) => dome::Result::Success,
        Ok(Err(())) => dome::Result::Failure,
        Err(err) => {
            let fmt_owned = CString::new("Plugin '%s' panicked: %s\n").unwrap();
            let plugin_name_owned = CString::new("<unknown>").unwrap();

            let fmt = fmt_owned.as_ptr() as *const c_char;
            let plugin_name = plugin_name_owned.as_ptr() as *const c_char; // TODO
            let panic_message_owned = if let Some(&s) = err.downcast_ref::<&str>() {
                CString::new(s).unwrap_or_else(|_| {
                    CString::new("Panic message contains null byte(s).").unwrap()
                })
            } else if let Some(s) = err.downcast_ref::<String>() {
                CString::new(s.clone()).unwrap_or_else(|_| {
                    CString::new("Panic message contains null byte(s).").unwrap()
                })
            } else {
                CString::new("Could not retrieve panic message.").unwrap()
            };
            let panic_message = panic_message_owned.as_ptr() as *const c_char;
            unsafe {
                ((*API.dome).log)(ctx, fmt, plugin_name, panic_message);
            }
            dome::Result::Failure
        }
    };
    panic::set_hook(prev_panic_hook);
    result
}

pub fn init_plugin(get_api: *mut c_void, ctx: *mut c_void, callback: Hook) -> c_int {
    assert!(!get_api.is_null(), "Got null pointer for DOME_getAPI");
    assert!(!ctx.is_null(), "Got null pointer for ctx");

    let get_api: GetApiFunction = unsafe { mem::transmute(get_api) };
    let ctx = ctx as dome::Context;

    unsafe {
        API.dome = get_api(ApiType::Dome, dome::API_VERSION) as *mut dome::ApiV0;
        API.wren = get_api(ApiType::Wren, wren::API_VERSION) as *mut wren::ApiV0;
        API.audio = get_api(ApiType::Audio, audio::API_VERSION) as *mut audio::ApiV0;

        if API.dome.is_null() || API.wren.is_null() || API.audio.is_null() {
            return dome::Result::Failure as c_int;
        }
    }

    invoke_callback(ctx, callback) as c_int
}

fn invoke_hook(ctx: dome::Context, callback: Option<Hook>) -> dome::Result {
    callback.map_or(dome::Result::Success, |callback| {
        invoke_callback(ctx, callback)
    })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preUpdate(ctx: dome::Context) -> dome::Result {
    invoke_hook(ctx, unsafe { HOOKS.pre_update })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postUpdate(ctx: dome::Context) -> dome::Result {
    invoke_hook(ctx, unsafe { HOOKS.post_update })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preDraw(ctx: dome::Context) -> dome::Result {
    invoke_hook(ctx, unsafe { HOOKS.pre_draw })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postDraw(ctx: dome::Context) -> dome::Result {
    invoke_hook(ctx, unsafe { HOOKS.post_draw })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onShutdown(ctx: dome::Context) -> dome::Result {
    invoke_hook(ctx, unsafe { HOOKS.on_shutdown })
}
