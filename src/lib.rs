mod panic;
mod safe_wrappers;
mod unsafe_wrappers;

use libc::{c_int, c_void};
use std::convert;
use std::mem;
use std::ptr;

use panic::catch_panic;
use unsafe_wrappers::audio::{self as unsafe_audio, API_VERSION as AUDIO_API_VERSION};
use unsafe_wrappers::dome::{
    self as unsafe_dome, Result as DomeResult, API_VERSION as DOME_API_VERSION,
};
use unsafe_wrappers::wren::{self as unsafe_wren, API_VERSION as WREN_API_VERSION};

pub use safe_wrappers::audio::{Channel, ChannelCallback, ChannelMix, ChannelState};
pub use safe_wrappers::dome::Context;
pub use safe_wrappers::wren::{Type as WrenType, VM as WrenVM};

#[repr(C)]
pub(crate) enum ApiType {
    Dome,
    Wren,
    Audio,
}

pub(crate) type GetApiFunction = extern "C" fn(api: ApiType, version: c_int) -> *mut c_void;

pub(crate) struct Api {
    dome: *mut unsafe_dome::ApiV0,
    wren: *mut unsafe_wren::ApiV0,
    audio: *mut unsafe_audio::ApiV0,
}
pub(crate) static mut API: Api = Api {
    dome: ptr::null_mut(),
    wren: ptr::null_mut(),
    audio: ptr::null_mut(),
};

pub type Hook = fn(Context) -> Result<(), ()>;
#[derive(Debug, Clone, Copy)]
pub struct Hooks {
    pub on_init: Option<Hook>,
    pub pre_update: Option<Hook>,
    pub post_update: Option<Hook>,
    pub pre_draw: Option<Hook>,
    pub post_draw: Option<Hook>,
    pub on_shutdown: Option<Hook>,
}
static mut HOOKS: Hooks = Hooks {
    on_init: None,
    pre_update: None,
    post_update: None,
    pre_draw: None,
    post_draw: None,
    on_shutdown: None,
};

#[inline]
fn invoke_callback(ctx: unsafe_dome::Context, callback: Hook) -> DomeResult {
    catch_panic(ctx, || callback(Context(ctx)))
        .and_then(convert::identity) // TODO: Replace with `.flatten()` once stabilized
        .into()
}

#[inline]
fn invoke_hook(ctx: unsafe_dome::Context, callback: Option<Hook>) -> DomeResult {
    callback.map_or(DomeResult::Success, |callback| {
        invoke_callback(ctx, callback)
    })
}

#[inline]
pub fn init_plugin(get_api: *mut c_void, ctx: *mut c_void, hooks: Hooks) -> c_int {
    assert!(!get_api.is_null(), "Got null pointer for DOME_getAPI");
    assert!(!ctx.is_null(), "Got null pointer for ctx");

    let get_api: GetApiFunction = unsafe { mem::transmute(get_api) };
    let ctx = ctx as unsafe_dome::Context;

    unsafe {
        API.dome = get_api(ApiType::Dome, DOME_API_VERSION) as *mut unsafe_dome::ApiV0;
        API.wren = get_api(ApiType::Wren, WREN_API_VERSION) as *mut unsafe_wren::ApiV0;
        API.audio = get_api(ApiType::Audio, AUDIO_API_VERSION) as *mut unsafe_audio::ApiV0;

        if API.dome.is_null() || API.wren.is_null() || API.audio.is_null() {
            return DomeResult::Failure as c_int;
        }
    }

    unsafe {
        HOOKS = hooks;
    }

    invoke_hook(ctx, unsafe { HOOKS.on_init }) as c_int
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preUpdate(ctx: unsafe_dome::Context) -> DomeResult {
    invoke_hook(ctx, unsafe { HOOKS.pre_update })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postUpdate(ctx: unsafe_dome::Context) -> DomeResult {
    invoke_hook(ctx, unsafe { HOOKS.post_update })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preDraw(ctx: unsafe_dome::Context) -> DomeResult {
    invoke_hook(ctx, unsafe { HOOKS.pre_draw })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postDraw(ctx: unsafe_dome::Context) -> DomeResult {
    invoke_hook(ctx, unsafe { HOOKS.post_draw })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onShutdown(ctx: unsafe_dome::Context) -> DomeResult {
    invoke_hook(ctx, unsafe { HOOKS.on_shutdown })
}
