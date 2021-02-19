use libc::{c_char, c_double, c_int, c_void, size_t};

use super::dome;

pub(crate) const API_VERSION: c_int = 0;

#[repr(C)]
pub(crate) struct FakeVM {
    _private: [u8; 0],
}
pub(crate) type VM = *mut FakeVM;

#[repr(C)]
pub(crate) struct FakeHandle {
    _private: [u8; 0],
}
pub(crate) type Handle = *mut FakeHandle;

pub(crate) type ForeignMethodFn = extern "C" fn(VM);
pub(crate) type FinalizerFn = extern "C" fn(*mut c_void);

/// A Wren type.
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Type {
    Bool,
    Num,
    Foreign,
    List,
    Map,
    Null,
    String,

    /// The object is of a type that isn't accessible by the C API.
    Unknown,
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct ApiV0 {
    pub(crate) ensure_slots: extern "C" fn(vm: VM, slot_count: c_int),

    pub(crate) set_slot_null: unsafe extern "C" fn(vm: VM, slot: c_int),
    pub(crate) set_slot_bool: unsafe extern "C" fn(vm: VM, slot: c_int, value: bool),
    pub(crate) set_slot_double: unsafe extern "C" fn(vm: VM, slot: c_int, value: c_double),
    pub(crate) set_slot_string: unsafe extern "C" fn(vm: VM, slot: c_int, text: *const c_char),
    pub(crate) set_slot_bytes:
        unsafe extern "C" fn(vm: VM, slot: c_int, data: *const c_char, length: size_t),
    pub(crate) set_slot_new_foreign:
        unsafe extern "C" fn(vm: VM, slot: c_int, class_slot: c_int, length: size_t) -> *mut c_void,
    pub(crate) set_slot_new_list: unsafe extern "C" fn(vm: VM, slot: c_int),
    pub(crate) set_slot_new_map: unsafe extern "C" fn(vm: VM, slot: c_int),

    pub(crate) get_user_data: extern "C" fn(vm: VM) -> dome::Context,
    pub(crate) get_slot_bool: unsafe extern "C" fn(vm: VM, slot: c_int) -> bool,
    pub(crate) get_slot_double: unsafe extern "C" fn(vm: VM, slot: c_int) -> c_double,
    pub(crate) get_slot_string: unsafe extern "C" fn(vm: VM, slot: c_int) -> *const c_char,
    pub(crate) get_slot_bytes:
        unsafe extern "C" fn(vm: VM, slot: c_int, length: *mut c_int) -> *const c_char,
    pub(crate) get_slot_foreign: unsafe extern "C" fn(vm: VM, slot: c_int) -> *mut c_void,

    pub(crate) abort_fiber: unsafe extern "C" fn(vm: VM, slot: c_int),
    pub(crate) get_slot_count: extern "C" fn(vm: VM) -> c_int,
    pub(crate) get_slot_type: unsafe extern "C" fn(vm: VM, slot: c_int) -> Type,

    pub(crate) get_list_count: unsafe extern "C" fn(vm: VM, slot: c_int) -> c_int,
    pub(crate) get_list_element:
        unsafe extern "C" fn(vm: VM, list_slot: c_int, index: c_int, element_slot: c_int),
    pub(crate) set_list_element:
        unsafe extern "C" fn(vm: VM, list_slot: c_int, index: c_int, element_slot: c_int),
    pub(crate) insert_in_list:
        unsafe extern "C" fn(vm: VM, list_slot: c_int, index: c_int, element_slot: c_int),

    pub(crate) get_map_count: unsafe extern "C" fn(vm: VM, slot: c_int) -> c_int,
    pub(crate) get_map_contains_key:
        unsafe extern "C" fn(vm: VM, map_slot: c_int, key_slot: c_int) -> bool,
    pub(crate) get_map_value:
        unsafe extern "C" fn(vm: VM, map_slot: c_int, key_slot: c_int, value_slot: c_int),
    pub(crate) set_map_value:
        unsafe extern "C" fn(vm: VM, map_slot: c_int, key_slot: c_int, value_slot: c_int),
    pub(crate) remove_map_value:
        unsafe extern "C" fn(vm: VM, map_slot: c_int, key_slot: c_int, removed_value_slot: c_int),

    pub(crate) get_variable:
        extern "C" fn(vm: VM, module: *const c_char, name: *const c_char, slot: c_int),
    pub(crate) get_slot_handle: unsafe extern "C" fn(vm: VM, slot: c_int) -> Handle,
    pub(crate) set_slot_handle: unsafe extern "C" fn(vm: VM, slot: c_int, handle: Handle),
}
