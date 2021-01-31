use libc::{c_float, c_void, size_t};

use super::dome;
use super::wren;

pub(crate) type ChannelId = u64;

#[repr(C)]
pub(crate) struct FakeEngine {
    _private: [u8; 0],
}
pub(crate) type Engine = *mut FakeEngine;

#[repr(C)]
pub(crate) struct ChannelRef {
    pub(crate) id: ChannelId,
    pub(crate) engine: Engine,
}

#[repr(C)]
pub(crate) enum ChannelState {
    Invalid,
    Initialize,
    ToPlay,
    Devirtualize,
    Loading,
    Playing,
    Stopping,
    Stopped,
    Virtualizing,
    Virtual,
    Last,
}

pub(crate) type ChannelMix =
    extern "C" fn(channel_ref: ChannelRef, buffer: *mut c_float, requested_samples: size_t);
pub(crate) type ChannelCallback = extern "C" fn(channel_ref: ChannelRef, vm: wren::VM);

#[repr(C)]
pub(crate) struct ApiV0 {
    channel_create: extern "C" fn(
        ctx: dome::Context,
        mix: ChannelMix,
        update: ChannelCallback,
        finish: ChannelCallback,
        user_data: *mut c_void,
    ),
    get_state: extern "C" fn(channel_ref: ChannelRef) -> ChannelState,
    set_state: extern "C" fn(channel_ref: ChannelRef, state: ChannelState),
    stop: extern "C" fn(channel_ref: ChannelRef),
    get_data: extern "C" fn(channel_ref: ChannelRef) -> *mut c_void,
}
