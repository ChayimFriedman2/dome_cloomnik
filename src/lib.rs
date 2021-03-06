//! A framework for building DOME plugins.
//!
//! The basic structure of every plugin using this framework is:
//!
//! Cargo.toml:
//! ```toml
//! [package]
//! name = "my_awesome_dome_plugin"
//! description = "Really, really awesome DOME plugin written in Rust!"
//! version = "0.1.0"
//! authors = ["Me <me@gmail.com>"]
//! edition = "2018"
//!
//! [dependencies]
//! libc = "0.2"
//! dome_cloomnik = "0.1"
//!
//! [lib]
//! crate-type = ["cdylib"]
//! ```
//!
//! lib.rs:
//! ```
//! use dome_cloomnik::{Context, WrenVM, register_modules, HookResult};
//!
//! #[no_mangle]
//! #[allow(non_snake_case)]
//! extern "C" fn PLUGIN_onInit(get_api: *mut libc::c_void, ctx: *mut libc::c_void) -> libc::c_int {
//!     unsafe {
//!         dome_cloomnik::init_plugin(
//!             get_api,
//!             ctx,
//!             dome_cloomnik::Hooks {
//!                 on_init: Some(on_init),
//!                 pre_update: Some(pre_update),
//!                 post_update: Some(post_update),
//!                 pre_draw: Some(pre_draw),
//!                 post_draw: Some(post_draw),
//!                 on_shutdown: Some(on_shutdown),
//!             }
//!         )
//!     }
//! }
//!
//! fn on_init(mut ctx: Context) -> HookResult {
//!     (register_modules! {
//!         ctx,
//!         ...
//!     })?;
//!
//!     // ...
//! }
//!
//! fn pre_update(mut ctx: Context) -> HookResult {
//!     // ...
//! }
//!
//! fn post_update(mut ctx: Context) -> HookResult {
//!     // ...
//! }
//!
//! fn pre_draw(mut ctx: Context) -> HookResult {
//!     // ...
//! }
//!
//! fn post_draw(mut ctx: Context) -> HookResult {
//!     // ...
//! }
//!
//! fn on_shutdown(mut ctx: Context) -> HookResult {
//!     // ...
//! }
//! ```
//!
//! Go ahead, and start with [learning DOME plugins from the docs](https://domeengine.com/plugins/).
//! Don't worry, much of the things there will apply to doom_cloomnik too!

mod errors;
mod panic;
mod safe_wrappers;
mod unsafe_wrappers;

use libc::{c_int, c_void};
use std::marker::PhantomData;
use std::mem;
use std::ptr;

use panic::catch_and_log_panic;
use unsafe_wrappers::audio as unsafe_audio;
use unsafe_wrappers::dome::{self as unsafe_dome, Result as DomeResult};
use unsafe_wrappers::wren as unsafe_wren;

pub use errors::{Error, Result};
pub use safe_wrappers::audio::{CallbackChannel, Channel, ChannelMix, ChannelState, ChannelUpdate};
pub use safe_wrappers::dome::Context;
pub use safe_wrappers::wren::{Handle as WrenHandle, Type as WrenType, VM as WrenVM};

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub type __c_void = c_void;
#[doc(hidden)]
#[allow(non_camel_case_types)]
pub type __ForeignWrapper<T> = safe_wrappers::wren::ForeignWrapper<T>;
#[doc(hidden)]
#[allow(non_camel_case_types)]
#[inline]
pub fn __catch_panic_from_foreign<R>(
    vm: &WrenVM,
    callback: impl FnOnce() -> R + std::panic::UnwindSafe,
) -> Option<R> {
    panic::catch_panic(callback)
        .map_err(|panic_message| panic::handle_wren_callback_panic(vm.0, &panic_message))
        .ok()
}
#[doc(hidden)]
#[allow(non_camel_case_types)]
#[inline]
pub unsafe fn __clone_vm(vm: &WrenVM) -> WrenVM {
    WrenVM(vm.0)
}

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
static mut API: Api = Api {
    dome: ptr::null_mut(),
    wren: ptr::null_mut(),
    audio: ptr::null_mut(),
};
impl Api {
    #[inline]
    pub(crate) fn dome() -> &'static unsafe_dome::ApiV0 {
        // SAFETY: The code should call `init_plugin()` before doing anything else,
        // which initializes this pointer.
        // Technically speaking, it is possible for this to be null if the user didn't
        // call `init_plugin()`, but protecting against that would make user code
        // awkward, and you'll get immediate crash because of dereferencing null pointer
        // once you'll do something with this crate.
        unsafe { &*API.dome }
    }
    #[inline]
    pub(crate) fn wren() -> &'static unsafe_wren::ApiV0 {
        // SAFETY: The code should call `init_plugin()` before doing anything else,
        // which initializes this pointer.
        // Technically speaking, it is possible for this to be null if the user didn't
        // call `init_plugin()`, but protecting against that would make user code
        // awkward, and you'll get immediate crash because of dereferencing null pointer
        // once you'll do something with this crate.
        unsafe { &*API.wren }
    }
    #[inline]
    pub(crate) fn audio() -> &'static unsafe_audio::ApiV0 {
        // SAFETY: The code should call `init_plugin()` before doing anything else,
        // which initializes this pointer.
        // Technically speaking, it is possible for this to be null if the user didn't
        // call `init_plugin()`, but protecting against that would make user code
        // awkward, and you'll get immediate crash because of dereferencing null pointer
        // once you'll do something with this crate.
        unsafe { &*API.audio }
    }
}

/// DOME plugin hook result.
pub type HookResult = anyhow::Result<()>;
/// DOME plugin hook.
pub type Hook = fn(Context) -> HookResult;
#[derive(Clone, Copy)]
/// A struct containing all plugin hooks. All hooks are optional.
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
fn invoke_hook(ctx: unsafe_dome::Context, callback: Option<Hook>) -> DomeResult {
    callback.map_or(DomeResult::Success, |callback| {
        match catch_and_log_panic(ctx, || callback(Context(ctx, PhantomData))) {
            Some(Ok(())) => DomeResult::Success,
            Some(Err(err)) => {
                Context(ctx, PhantomData).log(&err.to_string());
                DomeResult::Failure
            }
            None => DomeResult::Failure,
        }
    })
}

/// This function must be called from the `PLUGIN_onInit()` function, with exactly
/// the same arguments.
///
/// # Safety
///
/// As long as you pass the arguments of `PLUGIN_onInit()` exactly as-is, everything
/// is fine.
///
/// If not, expect nasal demons!
#[inline]
pub unsafe fn init_plugin(get_api: *mut c_void, ctx: *mut c_void, hooks: Hooks) -> c_int {
    if get_api.is_null() || ctx.is_null() {
        return DomeResult::Failure as c_int;
    }

    let get_api: GetApiFunction = mem::transmute(get_api);
    let ctx = ctx as unsafe_dome::Context;

    API.dome = get_api(ApiType::Dome, unsafe_dome::API_VERSION) as *mut unsafe_dome::ApiV0;
    API.wren = get_api(ApiType::Wren, unsafe_wren::API_VERSION) as *mut unsafe_wren::ApiV0;
    API.audio = get_api(ApiType::Audio, unsafe_audio::API_VERSION) as *mut unsafe_audio::ApiV0;

    if API.dome.is_null() || API.wren.is_null() || API.audio.is_null() {
        return DomeResult::Failure as c_int;
    }

    HOOKS = hooks;

    invoke_hook(ctx, HOOKS.on_init) as c_int
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preUpdate(ctx: unsafe_dome::Context) -> DomeResult {
    // SAFETY: The code should call `init_plugin()` before doing anything else,
    // which initializes this pointer.
    // Technically speaking, it is possible for this to be null if the user didn't
    // call `init_plugin()`, but protecting against that would make user code
    // awkward, and you'll get immediate crash because of dereferencing null pointer
    // once you'll do something with this crate.
    invoke_hook(ctx, unsafe { HOOKS.pre_update })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postUpdate(ctx: unsafe_dome::Context) -> DomeResult {
    // SAFETY: The code should call `init_plugin()` before doing anything else,
    // which initializes this pointer.
    // Technically speaking, it is possible for this to be null if the user didn't
    // call `init_plugin()`, but protecting against that would make user code
    // awkward, and you'll get immediate crash because of dereferencing null pointer
    // once you'll do something with this crate.
    invoke_hook(ctx, unsafe { HOOKS.post_update })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_preDraw(ctx: unsafe_dome::Context) -> DomeResult {
    // SAFETY: The code should call `init_plugin()` before doing anything else,
    // which initializes this pointer.
    // Technically speaking, it is possible for this to be null if the user didn't
    // call `init_plugin()`, but protecting against that would make user code
    // awkward, and you'll get immediate crash because of dereferencing null pointer
    // once you'll do something with this crate.
    invoke_hook(ctx, unsafe { HOOKS.pre_draw })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_postDraw(ctx: unsafe_dome::Context) -> DomeResult {
    // SAFETY: The code should call `init_plugin()` before doing anything else,
    // which initializes this pointer.
    // Technically speaking, it is possible for this to be null if the user didn't
    // call `init_plugin()`, but protecting against that would make user code
    // awkward, and you'll get immediate crash because of dereferencing null pointer
    // once you'll do something with this crate.
    invoke_hook(ctx, unsafe { HOOKS.post_draw })
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onShutdown(ctx: unsafe_dome::Context) -> DomeResult {
    // SAFETY: The code should call `init_plugin()` before doing anything else,
    // which initializes this pointer.
    // Technically speaking, it is possible for this to be null if the user didn't
    // call `init_plugin()`, but protecting against that would make user code
    // awkward, and you'll get immediate crash because of dereferencing null pointer
    // once you'll do something with this crate.
    invoke_hook(ctx, unsafe { HOOKS.on_shutdown })
}
