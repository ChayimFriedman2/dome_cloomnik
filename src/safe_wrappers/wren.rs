use libc::{c_char, c_int, c_void};
use std::convert::TryInto;
use std::slice;
use std::str;

use super::dome;
use crate::unsafe_wrappers::wren as unsafe_wren;
pub use unsafe_wren::Type;
use crate::API;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct VM(pub(crate) unsafe_wren::VM);

pub(crate) type ForeignMethodFn = extern "C" fn(VM);
pub(crate) type FinalizerFn = extern "C" fn(*mut c_void);

impl VM {
    #[inline]
    pub fn get_context(self) -> dome::Context {
        dome::Context((unsafe { (*API.dome).get_context })(self.0))
    }

    #[inline]
    pub fn ensure_slots(self, slot_count: usize) {
        (unsafe { (*API.wren).ensure_slots })(self.0, slot_count.try_into().unwrap())
    }

    #[inline]
    pub fn get_slot_count(self) -> usize {
        (unsafe { (*API.wren).get_slot_count })(self.0)
            .try_into()
            .unwrap()
    }

    #[inline]
    fn validate_slot(self, slot: usize) {
        let slots_count = self.get_slot_count();
        assert!(
            slot < slots_count,
            "Slot out of bounds: the count is {} but the slot is {}.",
            slots_count,
            slot
        );
    }

    #[inline]
    pub unsafe fn get_slot_type_unchecked(self, slot: usize) -> Type {
        ((*API.wren).get_slot_type)(self.0, slot.try_into().unwrap())
            .try_into()
            .unwrap()
    }
    #[inline]
    pub fn get_slot_type(self, slot: usize) -> Type {
        self.validate_slot(slot);
        unsafe { self.get_slot_type_unchecked(slot) }
    }

    #[inline]
    fn validate_slot_type(self, slot: usize, expected: Type) {
        let slot_type = self.get_slot_type(slot);
        assert!(
            slot_type == expected,
            "Slot {} is of the incorrect type - expected {:?}, got {:?}.",
            slot,
            expected,
            slot_type
        );
    }

    #[inline]
    pub unsafe fn set_slot_null_unchecked(self, slot: usize) {
        ((*API.wren).set_slot_null)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn set_slot_null(self, slot: usize) {
        self.validate_slot(slot);
        unsafe { self.set_slot_null_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn set_slot_bool_unchecked(self, slot: usize, value: bool) {
        ((*API.wren).set_slot_bool)(self.0, slot.try_into().unwrap(), value)
    }
    #[inline]
    pub fn set_slot_bool(self, slot: usize, value: bool) {
        self.validate_slot(slot);
        unsafe { self.set_slot_bool_unchecked(slot, value) }
    }

    #[inline]
    pub unsafe fn set_slot_double_unchecked(self, slot: usize, value: f64) {
        ((*API.wren).set_slot_double)(self.0, slot.try_into().unwrap(), value)
    }
    #[inline]
    pub fn set_slot_double(self, slot: usize, value: f64) {
        self.validate_slot(slot);
        unsafe { self.set_slot_double_unchecked(slot, value) }
    }

    #[inline]
    pub unsafe fn set_slot_bytes_unchecked(self, slot: usize, data: &[u8]) {
        ((*API.wren).set_slot_bytes)(
            self.0,
            slot.try_into().unwrap(),
            data.as_ptr() as *const c_char,
            data.len().try_into().unwrap(),
        )
    }
    #[inline]
    pub fn set_slot_bytes(self, slot: usize, data: &[u8]) {
        self.validate_slot(slot);
        unsafe { self.set_slot_bytes_unchecked(slot, data) }
    }

    #[inline]
    pub unsafe fn set_slot_string_unchecked(self, slot: usize, text: &str) {
        self.set_slot_bytes_unchecked(slot, text.as_bytes())
    }
    #[inline]
    pub fn set_slot_string(self, slot: usize, text: &str) {
        self.set_slot_bytes(slot, text.as_bytes())
    }

    #[inline]
    pub unsafe fn set_slot_new_list_unchecked(self, slot: usize) {
        ((*API.wren).set_slot_new_list)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn set_slot_new_list(self, slot: usize) {
        self.validate_slot(slot);
        unsafe { self.set_slot_new_list_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn set_slot_new_map_unchecked(self, slot: usize) {
        ((*API.wren).set_slot_new_map)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn set_slot_new_map(self, slot: usize) {
        self.validate_slot(slot);
        unsafe { self.set_slot_new_map_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn set_slot_new_foreign_unchecked(
        self,
        slot: usize,
        class_slot: usize,
        length: usize,
    ) {
        ((*API.wren).set_slot_new_foreign)(
            self.0,
            slot.try_into().unwrap(),
            class_slot.try_into().unwrap(),
            length.try_into().unwrap(),
        )
    }
    // No safe counterpart set_slot_new_foreign(), because we cannot validate that
    // `class_slot` contains a foreign class

    #[inline]
    pub unsafe fn get_slot_bool_unchecked(self, slot: usize) -> bool {
        ((*API.wren).get_slot_bool)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn get_slot_bool(self, slot: usize) -> bool {
        self.validate_slot_type(slot, Type::Bool);
        unsafe { self.get_slot_bool_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn get_slot_double_unchecked(self, slot: usize) -> f64 {
        ((*API.wren).get_slot_double)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn get_slot_double(self, slot: usize) -> f64 {
        self.validate_slot_type(slot, Type::Num);
        unsafe { self.get_slot_double_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn get_slot_bytes_unchecked(self, slot: usize) -> Vec<u8> {
        let mut length: c_int = 0;
        let data = ((*API.wren).get_slot_bytes)(self.0, slot.try_into().unwrap(), &mut length)
            as *const u8;
        slice::from_raw_parts(data, length.try_into().unwrap()).to_owned()
    }
    #[inline]
    pub fn get_slot_bytes(self, slot: usize) -> Vec<u8> {
        self.validate_slot_type(slot, Type::String);
        unsafe { self.get_slot_bytes_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn get_slot_string_unchecked(
        self,
        slot: usize,
    ) -> std::result::Result<String, str::Utf8Error> {
        let mut length: c_int = 0;
        let data = ((*API.wren).get_slot_bytes)(self.0, slot.try_into().unwrap(), &mut length)
            as *const u8;
        Ok(str::from_utf8(slice::from_raw_parts(data, length.try_into().unwrap()))?.to_owned())
    }
    #[inline]
    pub fn get_slot_string(self, slot: usize) -> std::result::Result<String, str::Utf8Error> {
        self.validate_slot_type(slot, Type::String);
        unsafe { self.get_slot_string_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn get_slot_foreign_unchecked(self, slot: usize) -> *mut c_void {
        ((*API.wren).get_slot_foreign)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn get_slot_foreign(self, slot: usize) -> *mut c_void {
        self.validate_slot_type(slot, Type::Foreign);
        unsafe { self.get_slot_foreign_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn get_list_count_unchecked(self, slot: usize) -> usize {
        ((*API.wren).get_list_count)(self.0, slot.try_into().unwrap())
            .try_into()
            .unwrap()
    }
    #[inline]
    pub fn get_list_count(self, slot: usize) -> usize {
        self.validate_slot_type(slot, Type::List);
        unsafe { self.get_list_count_unchecked(slot) }
    }

    #[inline]
    fn validate_list_element(self, list_slot: usize, index: usize) {
        let list_count: isize = self.get_list_count(list_slot).try_into().unwrap();
        let index: isize = index.try_into().unwrap();
        assert!(
            (-list_count..list_count).contains(&index),
            "Index {} out of bounds - size of list is {}.",
            index,
            list_count
        );
    }

    #[inline]
    pub unsafe fn get_list_element_unchecked(
        self,
        list_slot: usize,
        index: usize,
        element_slot: usize,
    ) {
        ((*API.wren).get_list_element)(
            self.0,
            list_slot.try_into().unwrap(),
            index.try_into().unwrap(),
            element_slot.try_into().unwrap(),
        )
    }
    #[inline]
    pub fn get_list_element(self, list_slot: usize, index: usize, element_slot: usize) {
        self.validate_list_element(list_slot, index);
        self.validate_slot(element_slot);
        unsafe { self.get_list_element_unchecked(list_slot, index, element_slot) }
    }

    #[inline]
    pub unsafe fn set_list_element_unchecked(
        self,
        list_slot: usize,
        index: usize,
        element_slot: usize,
    ) {
        ((*API.wren).set_list_element)(
            self.0,
            list_slot.try_into().unwrap(),
            index.try_into().unwrap(),
            element_slot.try_into().unwrap(),
        )
    }
    #[inline]
    pub fn set_list_element(self, list_slot: usize, index: usize, element_slot: usize) {
        self.validate_list_element(list_slot, index);
        self.validate_slot(element_slot);
        unsafe { self.set_list_element_unchecked(list_slot, index, element_slot) }
    }

    #[inline]
    pub unsafe fn insert_in_list_unchecked(
        self,
        list_slot: usize,
        index: usize,
        element_slot: usize,
    ) {
        ((*API.wren).insert_in_list)(
            self.0,
            list_slot.try_into().unwrap(),
            index.try_into().unwrap(),
            element_slot.try_into().unwrap(),
        )
    }
    #[inline]
    pub fn insert_in_list(self, list_slot: usize, index: usize, element_slot: usize) {
        // We don't use `validate_list_element()` because insert allows one past the end
        let list_count: isize = self.get_list_count(list_slot).try_into().unwrap();
        let index_isize: isize = index.try_into().unwrap();
        assert!(
            // We're not vulnerable to overflow here because `-isize::MAX - 1` is equals to
            // `isize::MIN` (`isize::MIN` is one larger than `isize::MAX`, absolute value).
            (-list_count - 1..=list_count).contains(&index_isize),
            "Index {} out of bounds - size of list is {}.",
            index,
            list_count
        );
        self.validate_slot(element_slot);
        unsafe { self.insert_in_list_unchecked(list_slot, index, element_slot) }
    }

    #[inline]
    pub unsafe fn get_map_count_unchecked(self, slot: usize) -> usize {
        ((*API.wren).get_map_count)(self.0, slot.try_into().unwrap())
            .try_into()
            .unwrap()
    }
    #[inline]
    pub fn get_map_count(self, slot: usize) -> usize {
        self.validate_slot_type(slot, Type::Map);
        unsafe { self.get_map_count_unchecked(slot) }
    }

    #[inline]
    pub unsafe fn get_map_value_unchecked(
        self,
        map_slot: usize,
        key_slot: usize,
        value_slot: usize,
    ) {
        ((*API.wren).get_map_value)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
            value_slot.try_into().unwrap(),
        )
    }
    // No safe counterparts get_map_value(), get_map_contains_key(), remove_map_value()
    // and set_map_value(), because we cannot validate that a key is hashable (need to
    // test for `Range` and `Class` for that)

    #[inline]
    pub unsafe fn set_map_value_unchecked(
        self,
        map_slot: usize,
        key_slot: usize,
        value_slot: usize,
    ) {
        ((*API.wren).set_map_value)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
            value_slot.try_into().unwrap(),
        )
    }

    #[inline]
    pub unsafe fn get_map_contains_key_unchecked(self, map_slot: usize, key_slot: usize) -> bool {
        ((*API.wren).get_map_contains_key)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
        )
    }

    #[inline]
    pub unsafe fn remove_map_value_unchecked(
        self,
        map_slot: usize,
        key_slot: usize,
        removed_value_slot: usize,
    ) {
        ((*API.wren).remove_map_value)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
            removed_value_slot.try_into().unwrap(),
        )
    }

    #[inline]
    pub unsafe fn abort_fiber_unchecked(self, slot: usize) {
        ((*API.wren).abort_fiber)(self.0, slot.try_into().unwrap())
    }
    #[inline]
    pub fn abort_fiber(self, slot: usize) {
        self.validate_slot(slot);
        unsafe { self.abort_fiber_unchecked(slot) }
    }
}
