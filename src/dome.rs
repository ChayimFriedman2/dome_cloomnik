use libc::{c_char, c_int, c_void};

use super::wren;

#[repr(C)]
pub(crate) enum ApiType {
    Dome,
    Wren,
    Audio,
}

pub(crate) const DOME_API_VERSION: c_int = 0;
pub(crate) const WREN_API_VERSION: c_int = 0;
pub(crate) const AUDIO_API_VERSION: c_int = 0;

#[repr(C)]
pub(crate) struct FakeContext {
    _private: [u8; 0],
}
pub(crate) type Context = *mut FakeContext;

#[repr(C)]
pub(crate) enum Result {
    Success,
    Failure,
    Unknown,
}

pub(crate) type PluginHook = extern "C" fn(context: Context);
pub(crate) type ForeignFn = wren::ForeignMethodFn;
pub(crate) type FinalizerFn = wren::FinalizerFn;

#[repr(C)]
pub(crate) struct Plugin {
    pub(crate) name: *const c_char,
    pub(crate) pre_update: PluginHook,
    pub(crate) post_update: PluginHook,
    pub(crate) pre_draw: PluginHook,
    pub(crate) post_draw: PluginHook,
    pub(crate) on_shutdown: PluginHook,
}

#[repr(C)]
pub(crate) struct ApiV0 {
    pub(crate) register_module:
        extern "C" fn(ctx: Context, name: *const c_char, source: *const c_char) -> Result,
    pub(crate) register_fn: extern "C" fn(
        ctx: Context,
        name: *const c_char,
        signature: *const c_char,
        method: ForeignFn,
    ) -> Result,
    pub(crate) register_class: extern "C" fn(
        ctx: Context,
        module_name: *const c_char,
        class_name: *const c_char,
        allocate: ForeignFn,
        finalize: Option<FinalizerFn>,
    ) -> Result,
    pub(crate) lock_module: extern "C" fn(ctx: Context, name: *const c_char),
    pub(crate) get_context: extern "C" fn(vm: wren::VM) -> Context,
    pub(crate) log: unsafe extern "C" fn(ctx: Context, text: *const c_char, ...) -> Context,
}

pub(crate) type GetApiFunction = extern "C" fn(api: ApiType, version: c_int) -> *mut c_void;
extern "C" {
    #[allow(non_snake_case)]
    fn DOME_getAPI(api: ApiType, version: c_int);
}
