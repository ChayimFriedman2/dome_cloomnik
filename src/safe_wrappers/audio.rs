use libc::{c_float, size_t};
use std::alloc::{self, Layout};
use std::cell::UnsafeCell;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::slice;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::wren;
use crate::panic::{catch_panic, handle_wren_callback_panic, PanicInfo};
use crate::unsafe_wrappers::audio as unsafe_audio;
use crate::unsafe_wrappers::wren as unsafe_wren;
use crate::Api;
pub use unsafe_audio::ChannelState;

pub(crate) struct InternalChannelData {
    mix: fn(&unsafe_audio::ChannelRef, &mut [[f32; 2]], usize),
    update: Option<fn(&unsafe_audio::ChannelRef, &unsafe_wren::VM)>,
    mix_error: Mutex<Option<PanicInfo>>,

    drop_fn: unsafe fn(*mut InternalChannelData),
    layout: Layout,
}

// This is repr(C) so that we can know that at offset 0 there is always
// `InternalChannelData`.
#[repr(C)]
pub(crate) struct ChannelData<T: Send + Sync> {
    internal_data: InternalChannelData,
    user_data: RwLock<T>,
}

impl<T: Send + Sync> ChannelData<T> {
    pub(crate) fn new(mix: ChannelMix<T>, update: ChannelUpdate<T>, user_data: T) -> Self {
        Self {
            internal_data: InternalChannelData {
                // SAFETY: `Channel<T>` is `repr(transparent)` over `ChannelRef`,
                // and so the ABI matches.
                mix: unsafe { mem::transmute(mix) },
                update: unsafe { mem::transmute(update) },
                mix_error: Mutex::new(None),

                // SAFETY: `ChannelData<T>` is `repr(C)` and its first member is
                // `InternalChannelData` (which guarantees it to be at offset 0),
                // And so passing a pointer to `InternalChannelData` to a function
                // that takes `ChannelData<T>` is valid.
                drop_fn: unsafe { mem::transmute::<unsafe fn(_), _>(ptr::drop_in_place::<Self>) },
                layout: Layout::new::<Self>(),
            },
            user_data: RwLock::new(user_data),
        }
    }
}

#[inline]
fn get_internal_data(channel_ref: unsafe_audio::ChannelRef) -> *mut InternalChannelData {
    (Api::audio().get_data)(channel_ref) as _
}

pub(crate) extern "C" fn mix(
    channel_ref: unsafe_audio::ChannelRef,
    buffer: *mut c_float,
    requested_samples: size_t,
) {
    // SAFETY: If we're here `finish()` wasn't called, and so the user data is valid.
    let internal_data = unsafe { &mut *get_internal_data(channel_ref) };
    let callback = internal_data.mix;
    let error = catch_panic(|| {
        let requested_samples = requested_samples.try_into().unwrap();
        let buffer = buffer as *mut [c_float; 2];
        // SAFETY: DOME guarantees a zeroes buffer of size `2 * requested_samples`.
        // Array layout is sequence of elements, so `&mut [f32]` of `2 * size`
        // can be transmuted into `&mut [[f32; 2]]` of `size`.
        let buffer = unsafe { slice::from_raw_parts_mut(buffer, requested_samples) };
        callback(&channel_ref, buffer, requested_samples)
    });
    if let Err(error) = error {
        // OK to `.unwrap()` the mutex lock (even though panicking across FFI is undefined
        // behavior) since the mutex locking can only fail if it is poisoned (a thread
        // panicked while holding it), and we know we never panic while holding this mutex
        internal_data.mix_error.lock().unwrap().replace(error);
    }
}

#[inline]
fn handle_mix_error(vm: unsafe_wren::VM, mix_error: &Mutex<Option<PanicInfo>>) {
    // OK to `.unwrap()` the mutex lock (even though panicking across FFI is undefined
    // behavior) since the mutex locking can only fail if it is poisoned (a thread
    // panicked while holding it), and we know we never panic while holding this mutex
    if let Some(panic_info) = mix_error.lock().unwrap().take() {
        handle_wren_callback_panic(vm, &panic_info);
    };
}

pub(crate) extern "C" fn update(channel_ref: unsafe_audio::ChannelRef, vm: unsafe_wren::VM) {
    // SAFETY: If we're here `finish()` wasn't called, and so the user data is valid.
    let internal_data = unsafe { &mut *get_internal_data(channel_ref) };

    handle_mix_error(vm, &internal_data.mix_error);

    internal_data.update.map(|callback| {
        let error = catch_panic(|| callback(&channel_ref, &vm));
        if let Err(error) = error {
            handle_wren_callback_panic(vm, &error);
        }
    });
}

pub(crate) extern "C" fn finish(channel_ref: unsafe_audio::ChannelRef, vm: unsafe_wren::VM) {
    let internal_data = get_internal_data(channel_ref);

    // SAFETY: We didn't free the memory yet, and `finish()` is guaranteed to be called
    // at most once.
    handle_mix_error(vm, unsafe { &(*internal_data).mix_error });

    // Cache the layout before we run the destructor
    // SAFETY: We didn't free the memory yet, and `finish()` is guaranteed to be called
    // at most once.
    let layout = unsafe { (*internal_data).layout };
    // Catch destructor's panics
    // SAFETY: We know the memory is valid as required by `drop_in_place()` - we allocated
    // it using `Box`.
    let error = catch_panic(|| unsafe { ((*internal_data).drop_fn)(internal_data) });
    if let Err(error) = error {
        handle_wren_callback_panic(vm, &error);
        return;
    }
    // SAFETY: The memory was allocated via `Box`.
    unsafe {
        alloc::dealloc(internal_data as _, layout);
    }
}

pub(crate) extern "C" fn finish_no_drop(
    channel_ref: unsafe_audio::ChannelRef,
    vm: unsafe_wren::VM,
) {
    let internal_data = get_internal_data(channel_ref);

    // SAFETY: We didn't free the memory yet, and `finish()` is guaranteed to be called
    // at most once.
    handle_mix_error(vm, unsafe { &(*internal_data).mix_error });

    // SAFETY: The memory was allocated via `Box`.
    unsafe {
        alloc::dealloc(internal_data as _, (*internal_data).layout);
    }
}

/// A DOME audio channel.
///
/// A channel provides various methods to handle it. Note that the
/// main work in channels happen in their callbacks, and not in other
/// code, but it's nice anyway. The main thing that you can do with
/// a channel is to stop it (using the [`Channel::stop()`] method)
/// but this can only be done when the channel is not shared.
///
/// Channels are thread-safe.
///
/// When a channel drops, it is automagically stopped. If this is not
/// desired, use [`mem::forget()`][std::mem::forget] to not drop it.
#[derive(Debug)]
#[repr(transparent)]
pub struct Channel<T: Send + Sync = ()>(
    pub(crate) unsafe_audio::ChannelRef,
    pub(crate) PhantomData<UnsafeCell<T>>,
);

// SAFETY: We use `RwLock` to access the mutable user data.
unsafe impl Send for Channel {}
unsafe impl Sync for Channel {}

impl<T: Send + Sync> Channel<T> {
    /// Queries the state of this channel.
    #[inline]
    pub fn state(&self) -> ChannelState {
        (Api::audio().get_state)(self.0)
    }

    /// Sets the state for this channel.
    #[inline]
    pub fn set_state(&mut self, state: ChannelState) {
        (Api::audio().set_state)(self.0, state)
    }

    /// Stops the channel. This is not a magic: it just takes ownership of
    /// the channel (see [`Channel`]).
    #[inline]
    pub fn stop(self) {}

    #[inline]
    fn user_data(&self) -> Option<&RwLock<T>> {
        if let ChannelState::Stopped = self.state() {
            return None;
        }

        let data = (Api::audio().get_data)(self.0) as *mut ChannelData<T>;
        // SAFETY: We just validated that the channel wasn't stopped, and so `finish()`
        // wasn't called and the memory wasn't dropped.
        Some(unsafe { &(*data).user_data })
    }
    /// Gets the user data associated with this channel, for read only.
    /// Channels are using [`RwLock`][std::sync::RwLock] to hold user data,
    /// so you can have multiple read-only references but only one read-write
    /// reference at a time.
    #[inline]
    pub fn data(&self) -> Option<RwLockReadGuard<T>> {
        Some(self.user_data()?.read().unwrap())
    }
    /// Gets the user data associated with this channel, for read and write.
    /// Channels are using [`RwLock`][std::sync::RwLock] to hold user data,
    /// so you can have multiple read-only references but only one read-write
    /// reference at a time.
    #[inline]
    pub fn data_mut(&self) -> Option<RwLockWriteGuard<T>> {
        Some(self.user_data()?.write().unwrap())
    }
}

impl<T: Send + Sync> Drop for Channel<T> {
    #[inline]
    fn drop(&mut self) {
        (Api::audio().stop)(self.0);
    }
}

#[derive(Debug)]
#[repr(transparent)]
/// A DOME audio channel, as passed to the channel callbacks (`mix` and `update`).
pub struct CallbackChannel<T: Send + Sync>(Channel<T>);

impl<T: Send + Sync> CallbackChannel<T> {
    /// Queries the state of this channel.
    #[inline]
    pub fn state(&self) -> ChannelState {
        self.0.state()
    }

    /// Sets the state for this channel.
    #[inline]
    pub fn set_state(&mut self, state: ChannelState) {
        self.0.set_state(state)
    }

    /// Stops the channel. This is equivalent to `self.set_state(ChannelState::Stopped)`.
    #[inline]
    pub fn stop(&mut self) {
        self.set_state(ChannelState::Stopped);
    }

    #[inline]
    fn user_data(&self) -> &RwLock<T> {
        let data = (Api::audio().get_data)(self.0 .0) as *mut ChannelData<T>;
        // SAFETY: We are inside channel callback (`mix` or `update`) and DOME does not call
        // them after `finish()`.
        unsafe { &(*data).user_data }
    }
    /// Gets the user data associated with this channel, for read only.
    /// Channels are using [`RwLock`][std::sync::RwLock] to hold user data,
    /// so you can have multiple read-only references but only one read-write
    /// reference at a time.
    #[inline]
    pub fn data(&self) -> RwLockReadGuard<T> {
        self.user_data().read().unwrap()
    }
    /// Gets the user data associated with this channel, for read and write.
    /// Channels are using [`RwLock`][std::sync::RwLock] to hold user data,
    /// so you can have multiple read-only references but only one read-write
    /// reference at a time.
    #[inline]
    pub fn data_mut(&self) -> RwLockWriteGuard<T> {
        self.user_data().write().unwrap()
    }
}

/// The `mix` callback of channel. It is responsible to fill `buffer`.
/// See [DOME's documentation](https://domeengine.com/plugins/#audio) for more details.
///
/// It takes a reference to, and not a copy of, `CallbackChannel`, because we
/// don't want it to drop the channel at the end, which will stop it.
pub type ChannelMix<T = ()> = fn(channel: &CallbackChannel<T>, buffer: &mut [[f32; 2]]);
/// The `update` callback of channel. It is called between frames.
/// See [DOME's documentation](https://domeengine.com/plugins/#audio) for more details.
///
/// It takes a reference to, and not a copy of, `CallbackChannel`, because we
/// don't want it to drop the channel at the end, which will stop it.
pub type ChannelUpdate<T = ()> = fn(channel: &CallbackChannel<T>, vm: &wren::VM);
