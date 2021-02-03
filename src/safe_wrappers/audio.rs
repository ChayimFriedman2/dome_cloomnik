use libc::{c_float, c_void, size_t};
use std::convert::TryInto;
use std::marker::PhantomData;
use std::mem;
use std::slice;

use super::wren;
use crate::panic::catch_panic;
use crate::unsafe_wrappers::audio as unsafe_audio;
use crate::unsafe_wrappers::wren as unsafe_wren;
use crate::API;
pub use unsafe_audio::ChannelState;

#[repr(C)]
pub(crate) struct ChannelData {
    pub(crate) mix: fn(unsafe_audio::ChannelRef, &mut [f32], usize),
    pub(crate) update: Option<fn(unsafe_audio::ChannelRef, unsafe_wren::VM)>,
    pub(crate) finish: Option<fn(unsafe_audio::ChannelRef, unsafe_wren::VM)>,
    pub(crate) user_data: *mut c_void,
}

pub(crate) extern "C" fn mix(
    channel_ref: unsafe_audio::ChannelRef,
    buffer: *mut c_float,
    requested_samples: size_t,
) {
    let data = (unsafe { (*API.audio).get_data })(channel_ref);
    let data: &mut ChannelData = unsafe { mem::transmute(data) };
    let requested_samples = requested_samples.try_into().unwrap();
    let buffer = unsafe { slice::from_raw_parts_mut(buffer, requested_samples * 2) };
    (data.mix)(channel_ref, buffer, requested_samples)
}

#[inline]
fn invoke_callback(
    channel_ref: unsafe_audio::ChannelRef,
    vm: unsafe_wren::VM,
    callback: Option<fn(unsafe_audio::ChannelRef, unsafe_wren::VM)>,
) {
    callback.map(|callback| {
        catch_panic((unsafe { (*API.dome).get_context })(vm), || {
            callback(channel_ref, vm)
        })
        .map_err(|()| {
            let vm = wren::VM(vm);
            vm.ensure_slots(2);
            vm.set_slot_string(1, "Plugin panicked. See DOME's log for details.");
            vm.abort_fiber(1);
        })
    });
}

pub(crate) extern "C" fn update(channel_ref: unsafe_audio::ChannelRef, vm: unsafe_wren::VM) {
    let data = (unsafe { (*API.audio).get_data })(channel_ref);
    let data: &mut ChannelData = unsafe { mem::transmute(data) };
    invoke_callback(channel_ref, vm, data.update);
}

pub(crate) extern "C" fn finish(channel_ref: unsafe_audio::ChannelRef, vm: unsafe_wren::VM) {
    let data = (unsafe { (*API.audio).get_data })(channel_ref);
    let data: &mut ChannelData = unsafe { mem::transmute(data) };
    invoke_callback(channel_ref, vm, data.finish);

    unsafe {
        Box::from_raw(data as *mut _);
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Channel<'a, T: 'a = ()>(
    pub(crate) unsafe_audio::ChannelRef,
    pub(crate) PhantomData<&'a mut T>,
);

impl<'a, T> Channel<'a, T> {
    #[inline]
    pub fn state(&self) -> ChannelState {
        (unsafe { (*API.audio).get_state })(self.0)
    }
    #[inline]
    pub fn set_state(&mut self, state: ChannelState) {
        (unsafe { (*API.audio).set_state })(self.0, state)
    }
    #[inline]
    pub fn stop(&mut self) {
        (unsafe { (*API.audio).stop })(self.0)
    }
    #[inline]
    pub fn data(&self) -> &'a mut T {
        let data = (unsafe { (*API.audio).get_data })(self.0);
        let data: &mut ChannelData = unsafe { mem::transmute(data) };
        unsafe { mem::transmute(data.user_data) }
    }
}

impl<T> Drop for Channel<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.stop();
    }
}

pub type ChannelMix<'a, T = ()> =
    fn(channel: Channel<'a, T>, buffer: &mut [f32], requested_samples: usize);
pub type ChannelCallback<'a, T = ()> = fn(channel: Channel<'a, T>, vm: wren::VM);
