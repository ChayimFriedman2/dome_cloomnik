use libc::{c_char, c_void};
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;

use super::audio;
use super::wren;
use crate::unsafe_wrappers::dome as unsafe_dome;
use crate::API;

type Result = std::result::Result<(), ()>;

pub(crate) type ForeignFn = wren::ForeignMethodFn;
pub(crate) type FinalizerFn = wren::FinalizerFn;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Context(pub(crate) unsafe_dome::Context);

impl Context {
    #[inline]
    pub fn register_module(self, name: &str, source: &str) -> Result {
        let name = CString::new(name).expect("Module name contains null byte(s).");
        let source = CString::new(source).expect("Source contains null byte(s).");
        (unsafe { (*API.dome).register_module })(self.0, name.into_raw(), source.into_raw()).into()
    }

    #[inline]
    pub fn register_fn(self, name: &str, signature: &str, method: ForeignFn) -> Result {
        let name = CString::new(name).expect("Method name contains null byte(s).");
        let signature = CString::new(signature).expect("Method signature contains null byte(s).");
        (unsafe { (*API.dome).register_fn })(
            self.0,
            name.into_raw(),
            signature.into_raw(),
            unsafe { mem::transmute(method) },
        )
        .into()
    }

    #[inline]
    pub fn register_class(
        self,
        module_name: &str,
        class_name: &str,
        allocate: ForeignFn,
        finalize: Option<FinalizerFn>,
    ) -> Result {
        let module_name = CString::new(module_name).expect("Module name contains null byte(s).");
        let class_name = CString::new(class_name).expect("Class name contains null byte(s).");
        (unsafe { (*API.dome).register_class })(
            self.0,
            module_name.into_raw(),
            class_name.into_raw(),
            unsafe { mem::transmute(allocate) },
            finalize,
        )
        .into()
    }

    #[inline]
    pub fn lock_module(self, name: &str) {
        let name = CString::new(name).expect("Module name contains null byte(s).");
        (unsafe { (*API.dome).lock_module })(self.0, name.as_ptr() as *const c_char)
    }

    #[inline]
    pub fn log(self, text: &str) {
        let text = text.replace("%", "%%"); // Escape '%'s
        let text = CString::new(text).expect("Text contains null byte(s).");
        unsafe { ((*API.dome).log)(self.0, text.as_ptr() as *const c_char) }
    }

    #[inline]
    pub fn create_channel(
        self,
        mix: audio::ChannelMix<'static>,
        update: audio::ChannelCallback<'static>,
        finish: audio::ChannelCallback<'static>,
    ) -> audio::Channel<'static> {
        self.create_channel_impl(mix, update, finish, NonNull::dangling().as_ptr())
    }
    #[inline]
    pub fn create_channel_with_data<'a, T>(
        self,
        mix: audio::ChannelMix<'a, T>,
        update: audio::ChannelCallback<'a, T>,
        finish: audio::ChannelCallback<'a, T>,
        user_data: &'a mut T,
    ) -> audio::Channel<'a, T> {
        self.create_channel_impl(mix, update, finish, user_data)
    }
    #[inline]
    fn create_channel_impl<'a, T>(
        self,
        mix: audio::ChannelMix<'a, T>,
        update: audio::ChannelCallback<'a, T>,
        finish: audio::ChannelCallback<'a, T>,
        user_data: *mut T,
    ) -> audio::Channel<'a, T> {
        let data = Box::leak(Box::new(audio::ChannelData {
            mix: unsafe { mem::transmute(mix) },
            update: unsafe { mem::transmute(update) },
            finish: unsafe { mem::transmute(finish) },
            user_data: user_data as *mut c_void,
        }));
        audio::Channel(
            (unsafe { (*API.audio).channel_create })(
                self.0,
                audio::mix,
                audio::update,
                audio::finish,
                data as *mut _ as *mut c_void,
            ),
            PhantomData,
        )
    }
}
