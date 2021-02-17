use libc::{c_char, c_int};

use super::wren;

pub(crate) const API_VERSION: c_int = 0;

#[repr(C)]
pub(crate) struct FakeContext {
    _private: [u8; 0],
}
pub(crate) type Context = *mut FakeContext;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) enum Result {
    Success,
    Failure,
    #[allow(unused)]
    Unknown,
}

impl Result {
    pub(crate) fn to_result(
        self,
        err: impl FnOnce() -> crate::errors::Error,
    ) -> crate::errors::Result {
        match self {
            Result::Success => Ok(()),
            _ => Err(err()),
        }
    }
}

pub(crate) type ForeignFn = wren::ForeignMethodFn;
pub(crate) type FinalizerFn = wren::FinalizerFn;

#[repr(C)]
#[derive(Debug)]
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
    pub(crate) log: unsafe extern "C" fn(ctx: Context, text: *const c_char, ...),
}
