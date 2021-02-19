use libc::{c_char, c_int, c_void};
use std::any::TypeId;
use std::convert::TryInto;
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::slice;
use std::str;

use super::dome;
use crate::unsafe_wrappers::wren as unsafe_wren;
use crate::Api;
pub use unsafe_wren::Type;

// This is repr(C) so we know that the type ID is always at byte 0,
// and align(1) because the memory is allocated by Wren and we do
// not control its alignment.
#[repr(C, align(1))]
pub struct ForeignWrapper<T: 'static> {
    type_id: TypeId,
    foreign: T,
}

impl<T> ForeignWrapper<T> {
    #[inline]
    fn new(foreign: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            foreign,
        }
    }
}

/// This is the gate for all operations using Wren.
///
/// You can only get one in foreign methods.
#[derive(Debug)]
#[repr(transparent)]
pub struct VM(pub(crate) unsafe_wren::VM);

/// A handle is a long-lived value, as opposed to a slot which is short-lived.
///
/// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Handle(unsafe_wren::Handle);

pub(crate) type ForeignMethodFn = extern "C" fn(VM);
pub(crate) type FinalizerFn = extern "C" fn(*mut c_void);

impl VM {
    /// Retrieve a [`Context`] from this [`VM`].
    #[inline]
    pub fn get_context(&self) -> dome::Context {
        dome::Context((Api::dome().get_context)(self.0), PhantomData)
    }

    /// Ensure that there are _at least_ `slot_count` slots.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn ensure_slots(&mut self, slot_count: usize) {
        (Api::wren().ensure_slots)(self.0, slot_count.try_into().unwrap())
    }

    /// Returns the number of available slots.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_count(&self) -> usize {
        (Api::wren().get_slot_count)(self.0).try_into().unwrap()
    }

    #[inline]
    fn validate_slot(&self, slot: usize) {
        let slots_count = self.get_slot_count();
        assert!(
            slot < slots_count,
            "Slot out of bounds: the count is {} but the slot is {}.",
            slots_count,
            slot
        );
    }

    /// Returns the type of the object is `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn get_slot_type_unchecked(&self, slot: usize) -> Type {
        (Api::wren().get_slot_type)(self.0, slot.try_into().unwrap())
    }
    /// Returns the type of the object is `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_type(&self, slot: usize) -> Type {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.get_slot_type_unchecked(slot) }
    }

    #[inline]
    fn validate_slot_type(&self, slot: usize, expected: Type) {
        let slot_type = self.get_slot_type(slot);
        assert!(
            slot_type == expected,
            "Slot {} is of the incorrect type - expected {:?}, got {:?}.",
            slot,
            expected,
            slot_type
        );
    }

    /// Sets `slot` to `null`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_null_unchecked(&mut self, slot: usize) {
        (Api::wren().set_slot_null)(self.0, slot.try_into().unwrap())
    }
    /// Sets `slot` to `null`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_null(&mut self, slot: usize) {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.set_slot_null_unchecked(slot) }
    }

    /// Sets `slot` to a `Bool`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_bool_unchecked(&mut self, slot: usize, value: bool) {
        (Api::wren().set_slot_bool)(self.0, slot.try_into().unwrap(), value)
    }
    /// Sets `slot` to a `Bool`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_bool(&mut self, slot: usize, value: bool) {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.set_slot_bool_unchecked(slot, value) }
    }

    /// Sets `slot` to a `Num`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_double_unchecked(&mut self, slot: usize, value: f64) {
        (Api::wren().set_slot_double)(self.0, slot.try_into().unwrap(), value)
    }
    /// Sets `slot` to a `Num`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_double(&mut self, slot: usize, value: f64) {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.set_slot_double_unchecked(slot, value) }
    }

    /// Sets `slot` to a `String` from Rust bytes slice.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_bytes_unchecked(&mut self, slot: usize, data: &[u8]) {
        (Api::wren().set_slot_bytes)(
            self.0,
            slot.try_into().unwrap(),
            data.as_ptr() as *const c_char,
            data.len().try_into().unwrap(),
        )
    }
    /// Sets `slot` to a `String` from Rust bytes slice.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_bytes(&mut self, slot: usize, data: &[u8]) {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.set_slot_bytes_unchecked(slot, data) }
    }

    /// Sets `slot` to a `String` from Rust `str`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_string_unchecked(&mut self, slot: usize, text: &str) {
        self.set_slot_bytes_unchecked(slot, text.as_bytes())
    }
    /// Sets `slot` to a `String` from Rust `str`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_string(&mut self, slot: usize, text: &str) {
        self.set_slot_bytes(slot, text.as_bytes())
    }

    /// Sets `slot` to a new `List`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_new_list_unchecked(&mut self, slot: usize) {
        (Api::wren().set_slot_new_list)(self.0, slot.try_into().unwrap())
    }
    /// Sets `slot` to a new `List`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_new_list(&mut self, slot: usize) {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.set_slot_new_list_unchecked(slot) }
    }

    /// Sets `slot` to a new `Map`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid.
    #[inline]
    pub unsafe fn set_slot_new_map_unchecked(&mut self, slot: usize) {
        (Api::wren().set_slot_new_map)(self.0, slot.try_into().unwrap())
    }
    /// Sets `slot` to a new `Map`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_new_map(&mut self, slot: usize) {
        self.validate_slot(slot);
        // SAFETY: We verified that the slot exists.
        unsafe { self.set_slot_new_map_unchecked(slot) }
    }

    /// Sets `slot` to a new foreign object, where the foreign class is stored in `class_slot`
    /// and the expected size (in bytes) is `length`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid, nor a `class_slot`
    /// that is either invalid or does not contain a foreign class.
    ///
    /// There isn't a safe counterpart to this method, because there is no way from the C API
    /// to verify that a slot contains a foreign class ([`get_slot_type()`] returns [`Type::Unknown`]
    /// for them).
    #[inline]
    pub unsafe fn set_slot_new_raw_foreign_unchecked(
        &mut self,
        slot: usize,
        class_slot: usize,
        length: usize,
    ) -> *mut c_void {
        (Api::wren().set_slot_new_foreign)(
            self.0,
            slot.try_into().unwrap(),
            class_slot.try_into().unwrap(),
            length.try_into().unwrap(),
        )
    }
    /// Sets `slot` to a new Rust foreign object, where the foreign class is stored in `class_slot`
    /// and the Rust type is passed as a generic parameter.
    ///
    /// Note that this is **not** equal to the following:
    /// ```
    /// let p = self.set_slot_new_raw_foreign_unchecked(slot, class_slot, std::mem::size_of::<T>());
    /// std::ptr::write(p, instance);
    /// ```
    /// Because this method does some bookkeeping to ensure that we get the right type back
    /// (it stores a [`TypeId`]), and also to work around the fact that Rust types are
    /// required to be properly aligned, but Wren does not provide alignment guarantees.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must not provide this function a `slot` that is not valid, nor a `class_slot`
    /// that is either invalid or does not contain a foreign class.
    ///
    /// There isn't a safe counterpart to this method, because there is no way from the C API
    /// to verify that a slot contains a foreign class ([`get_slot_type()`] returns [`Type::Unknown`]
    /// for them).
    #[inline]
    pub unsafe fn set_slot_new_foreign_unchecked<T: 'static>(
        &mut self,
        slot: usize,
        class_slot: usize,
        instance: T,
    ) -> &mut T {
        let foreign_size = mem::size_of::<ForeignWrapper<T>>();
        let foreign = self.set_slot_new_raw_foreign_unchecked(slot, class_slot, foreign_size);
        let foreign = foreign as *mut ForeignWrapper<T>;
        ptr::write(foreign, ForeignWrapper::new(instance));
        &mut (*foreign).foreign
    }

    /// Gets `Bool` from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a `Bool`.
    #[inline]
    pub unsafe fn get_slot_bool_unchecked(&self, slot: usize) -> bool {
        (Api::wren().get_slot_bool)(self.0, slot.try_into().unwrap())
    }
    /// Gets `Bool` from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_bool(&self, slot: usize) -> bool {
        self.validate_slot_type(slot, Type::Bool);
        // SAFETY: We verified that the slot exists and contains a `Bool`.
        unsafe { self.get_slot_bool_unchecked(slot) }
    }

    /// Gets `Num` from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a `Num`.
    #[inline]
    pub unsafe fn get_slot_double_unchecked(&self, slot: usize) -> f64 {
        (Api::wren().get_slot_double)(self.0, slot.try_into().unwrap())
    }
    /// Gets `Num` from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_double(&self, slot: usize) -> f64 {
        self.validate_slot_type(slot, Type::Num);
        // SAFETY: We verified that the slot exists and contains a `Num`.
        unsafe { self.get_slot_double_unchecked(slot) }
    }

    /// Gets a `String` as a sequence of bytes from `slot`.
    ///
    /// This function copies the string and so it remains valid even after you give
    /// the control back to Wren.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a `String`.
    #[inline]
    pub unsafe fn get_slot_bytes_unchecked(&self, slot: usize) -> Vec<u8> {
        let mut length: c_int = 0;
        let data = (Api::wren().get_slot_bytes)(self.0, slot.try_into().unwrap(), &mut length)
            as *const u8;
        slice::from_raw_parts(data, length.try_into().unwrap()).to_owned()
    }
    /// Gets a `String` as a sequence of bytes from `slot`.
    ///
    /// This function copies the string and so it remains valid even after you give
    /// the control back to Wren.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_bytes(&self, slot: usize) -> Vec<u8> {
        self.validate_slot_type(slot, Type::String);
        // SAFETY: We verified that the slot exists and contains a `String`.
        unsafe { self.get_slot_bytes_unchecked(slot) }
    }

    /// Gets a `String` as Rust [`String`] from `slot`.
    ///
    /// Note that Rust strings are required to be valid UTF-8 sequences, but this
    /// requirement does not exist for Wren. So, this function may fail. If you
    /// only want a sequence of bytes (and not a textual string), you should use
    /// [`get_slot_bytes_unchecked()`].
    ///
    /// This function copies the string and so it remains valid even after you give
    /// the control back to Wren.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a `String`.
    #[inline]
    pub unsafe fn get_slot_string_unchecked(
        &self,
        slot: usize,
    ) -> std::result::Result<String, str::Utf8Error> {
        let mut length: c_int = 0;
        let data = (Api::wren().get_slot_bytes)(self.0, slot.try_into().unwrap(), &mut length)
            as *const u8;
        Ok(str::from_utf8(slice::from_raw_parts(data, length.try_into().unwrap()))?.to_owned())
    }
    /// Gets a `String` as Rust [`String`] from `slot`.
    ///
    /// Note that Rust strings are required to be valid UTF-8 sequences, but this
    /// requirement does not exist for Wren. So, this function may fail. If you
    /// only want a sequence of bytes (and not a textual string), you should use
    /// [`get_slot_bytes()`].
    ///
    /// This function copies the string and so it remains valid even after you give
    /// the control back to Wren.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_string(&self, slot: usize) -> std::result::Result<String, str::Utf8Error> {
        self.validate_slot_type(slot, Type::String);
        // SAFETY: We verified that the slot exists and contains a `String`.
        unsafe { self.get_slot_string_unchecked(slot) }
    }

    /// Gets a foreign object from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a foreign
    /// class instance.
    #[inline]
    pub unsafe fn get_slot_raw_foreign_unchecked(&self, slot: usize) -> *mut c_void {
        (Api::wren().get_slot_foreign)(self.0, slot.try_into().unwrap())
    }
    /// Gets a foreign object from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_raw_foreign(&self, slot: usize) -> *mut c_void {
        self.validate_slot_type(slot, Type::Foreign);
        // SAFETY: We verified that the slot exists and contains a foreign object.
        unsafe { self.get_slot_raw_foreign_unchecked(slot) }
    }
    /// Gets a Rust foreign object from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a Rust foreign
    /// class instance, created using [`set_slot_new_foreign::<T>()`], with the same `T`.
    #[inline]
    pub unsafe fn get_slot_foreign_unchecked<T: 'static>(&self, slot: usize) -> &mut T {
        let foreign = self.get_slot_raw_foreign_unchecked(slot);
        let foreign = foreign as *mut ForeignWrapper<T>;
        &mut (*foreign).foreign
    }
    /// Gets a Rust foreign object from `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// This function is less unsafe than [`get_slot_foreign_unchecked()`] because it validates
    /// the slot it takes, so it will panic on invalid slot or if it does not contain a foreign
    /// object, or if it _does_ contain a Rust foreign object but not of the type `T`.
    ///
    /// See the gap? This function _does_ validate that it got a foreign instance,
    /// but _does not_ validate that this instance is a Rust instance created via
    /// [`set_slot_new_foreign()`]. Verifying that is, unfortunately, impossible.
    ///
    /// Still, you should prefer using this function over [`get_slot_foreign_unchecked()`]
    /// when performance are not a concern, because there is less risk for bugs.
    #[inline]
    pub unsafe fn get_slot_foreign<T: 'static>(&self, slot: usize) -> &mut T {
        let foreign = self.get_slot_raw_foreign(slot);
        let foreign = foreign as *mut ForeignWrapper<T>;
        assert!(
            TypeId::of::<T>() == (*foreign).type_id,
            "Incorrect type in slot."
        );
        &mut (*foreign).foreign
    }

    /// Retrieves the list length from the list object at `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a `List`.
    #[inline]
    pub unsafe fn get_list_count_unchecked(&self, slot: usize) -> usize {
        (Api::wren().get_list_count)(self.0, slot.try_into().unwrap())
            .try_into()
            .unwrap()
    }
    /// Retrieves the list length from the list object at `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_list_count(&self, slot: usize) -> usize {
        self.validate_slot_type(slot, Type::List);
        // SAFETY: We verified that the slot exists and contains a `List`.
        unsafe { self.get_list_count_unchecked(slot) }
    }

    #[inline]
    fn validate_list_element(&self, list_slot: usize, index: usize) {
        let list_count: isize = self.get_list_count(list_slot).try_into().unwrap();
        let index: isize = index.try_into().unwrap();
        assert!(
            (-list_count..list_count).contains(&index),
            "Index {} out of bounds - size of list is {}.",
            index,
            list_count
        );
    }

    /// Retrieves the index-th list element from the list object at `list_slot` into `element_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `list_slot` that is valid and contains a `List`,
    /// a valid `element_slot`, and `index` that is in bounds.
    #[inline]
    pub unsafe fn get_list_element_unchecked(
        &mut self,
        list_slot: usize,
        index: usize,
        element_slot: usize,
    ) {
        (Api::wren().get_list_element)(
            self.0,
            list_slot.try_into().unwrap(),
            index.try_into().unwrap(),
            element_slot.try_into().unwrap(),
        )
    }
    /// Retrieves the index-th list element from the list object at `list_slot` into `element_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_list_element(&mut self, list_slot: usize, index: usize, element_slot: usize) {
        self.validate_list_element(list_slot, index);
        self.validate_slot(element_slot);
        // SAFETY: We verified that `list_slot` exists and contains a `List`, `element_slot`
        // exists and `index` is valid.
        unsafe { self.get_list_element_unchecked(list_slot, index, element_slot) }
    }

    /// Sets the index-th list element from the list object at `list_slot` to `element_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `list_slot` that is valid and contains a `List`,
    /// a valid `element_slot`, and `index` that is in bounds.
    #[inline]
    pub unsafe fn set_list_element_unchecked(
        &mut self,
        list_slot: usize,
        index: usize,
        element_slot: usize,
    ) {
        (Api::wren().set_list_element)(
            self.0,
            list_slot.try_into().unwrap(),
            index.try_into().unwrap(),
            element_slot.try_into().unwrap(),
        )
    }
    /// Sets the index-th list element from the list object at `list_slot` to `element_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_list_element(&mut self, list_slot: usize, index: usize, element_slot: usize) {
        self.validate_list_element(list_slot, index);
        self.validate_slot(element_slot);
        // SAFETY: We verified that `list_slot` exists and contains a `List`, `element_slot`
        // exists and `index` is valid.
        unsafe { self.set_list_element_unchecked(list_slot, index, element_slot) }
    }

    /// Inserts the item at `element_slot` to the list at `slot` at `index`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `list_slot` that is valid and contains a `List`,
    /// a valid `element_slot`, and `index` that is in bounds.
    #[inline]
    pub unsafe fn insert_in_list_unchecked(
        &mut self,
        list_slot: usize,
        index: usize,
        element_slot: usize,
    ) {
        (Api::wren().insert_in_list)(
            self.0,
            list_slot.try_into().unwrap(),
            index.try_into().unwrap(),
            element_slot.try_into().unwrap(),
        )
    }
    /// Inserts the item at `element_slot` to the list at `list_slot` at `index`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn insert_in_list(&mut self, list_slot: usize, index: usize, element_slot: usize) {
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
        // SAFETY: We verified that `list_slot` exists and contains a `List`, `element_slot`
        // exists and `index` is valid.
        unsafe { self.insert_in_list_unchecked(list_slot, index, element_slot) }
    }

    /// Gets the number of elements in the `Map` at `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `slot` that is valid and contains a `Map`.
    #[inline]
    pub unsafe fn get_map_count_unchecked(&self, slot: usize) -> usize {
        (Api::wren().get_map_count)(self.0, slot.try_into().unwrap())
            .try_into()
            .unwrap()
    }
    /// Gets the number of elements in the `Map` at `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_map_count(&self, slot: usize) -> usize {
        self.validate_slot_type(slot, Type::Map);
        // SAFETY: We verified that `slot` exists and contains a `Map`.
        unsafe { self.get_map_count_unchecked(slot) }
    }

    /// Inserts the value with the key at `key_slot` in the `Map` at `map_slot` into `value_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `map_slot` that is valid and contains a `Map`,
    /// a `key_slot` that is valid and contains a hashable object, and a `value_slot`
    /// that is valid.
    #[inline]
    pub unsafe fn get_map_value_unchecked(
        &mut self,
        map_slot: usize,
        key_slot: usize,
        value_slot: usize,
    ) {
        (Api::wren().get_map_value)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
            value_slot.try_into().unwrap(),
        );
    }
    /// Inserts the value with the key at `key_slot` in the `Map` at `map_slot` into `value_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// The value inside `key_slot` must be hashable.
    #[inline]
    pub unsafe fn get_map_value(&mut self, map_slot: usize, key_slot: usize, value_slot: usize) {
        self.validate_slot_type(map_slot, Type::Map);
        self.validate_slot(key_slot);
        self.validate_slot(value_slot);
        self.get_map_value_unchecked(map_slot, key_slot, value_slot);
    }

    /// Sets the value with the key at `key_slot` in the `Map` at `map_slot` to `value_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `map_slot` that is valid and contains a `Map`,
    /// a `key_slot` that is valid and contains a hashable object, and a `value_slot`
    /// that is valid.
    #[inline]
    pub unsafe fn set_map_value_unchecked(
        &mut self,
        map_slot: usize,
        key_slot: usize,
        value_slot: usize,
    ) {
        (Api::wren().set_map_value)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
            value_slot.try_into().unwrap(),
        )
    }
    /// Sets the value with the key at `key_slot` in the `Map` at `map_slot` to `value_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// The value inside `key_slot` must be hashable.
    #[inline]
    pub unsafe fn set_map_value(&mut self, map_slot: usize, key_slot: usize, value_slot: usize) {
        self.validate_slot_type(map_slot, Type::Map);
        self.validate_slot(key_slot);
        self.validate_slot(value_slot);
        self.set_map_value_unchecked(map_slot, key_slot, value_slot);
    }

    /// Returns `true` if the `Map` at `map_slot` contains `key_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `map_slot` that is valid and contains a `Map`,
    /// and a `key_slot` that is valid and contains a hashable object.
    #[inline]
    pub unsafe fn map_contains_key_unchecked(&self, map_slot: usize, key_slot: usize) -> bool {
        (Api::wren().get_map_contains_key)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
        )
    }
    /// Returns `true` if the `Map` at `map_slot` contains `key_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// The value inside `key_slot` must be hashable.
    #[inline]
    pub unsafe fn map_contains_key(&self, map_slot: usize, key_slot: usize) {
        self.validate_slot_type(map_slot, Type::Map);
        self.validate_slot(key_slot);
        self.map_contains_key_unchecked(map_slot, key_slot);
    }

    /// Removes the value with the key at `key_slot` in the `Map` at `map_slot` and stores
    /// the removed value at `value_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a `map_slot` that is valid and contains a `Map`,
    /// a `key_slot` that is valid and contains a hashable object, and a `value_slot`
    /// that is valid.
    #[inline]
    pub unsafe fn remove_map_value_unchecked(
        &mut self,
        map_slot: usize,
        key_slot: usize,
        removed_value_slot: usize,
    ) {
        (Api::wren().remove_map_value)(
            self.0,
            map_slot.try_into().unwrap(),
            key_slot.try_into().unwrap(),
            removed_value_slot.try_into().unwrap(),
        )
    }
    /// Removes the value with the key at `key_slot` in the `Map` at `map_slot` and stores
    /// the removed value at `value_slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// The value inside `key_slot` must be hashable.
    #[inline]
    pub unsafe fn remove_map_value(
        &mut self,
        map_slot: usize,
        key_slot: usize,
        removed_value_slot: usize,
    ) {
        self.validate_slot_type(map_slot, Type::Map);
        self.validate_slot(key_slot);
        self.validate_slot(removed_value_slot);
        self.remove_map_value_unchecked(map_slot, key_slot, removed_value_slot);
    }

    /// Aborts the current fiber with the error at `slot`.
    ///
    /// # Safety
    ///
    /// `slot` must be valid.
    #[inline]
    pub unsafe fn abort_fiber_unchecked(&mut self, slot: usize) {
        (Api::wren().abort_fiber)(self.0, slot.try_into().unwrap())
    }
    /// Aborts the current fiber with the error at `slot`.
    #[inline]
    pub fn abort_fiber(&mut self, slot: usize) {
        self.validate_slot(slot);
        // SAFETY: We verified that `slot` exists.
        unsafe { self.abort_fiber_unchecked(slot) }
    }

    /// Retrieves the variable with `name` in `module` int `slot`..
    ///
    /// # Safety
    ///
    /// `slot` must be valid. `name` must exist inside `module`.
    #[inline]
    pub unsafe fn get_variable_unchecked(&mut self, module: &str, name: &str, slot: usize) {
        let module = CString::new(module).expect("Module name contains null byte(s).");
        let name = CString::new(name).expect("Variable name contains null byte(s).");
        (Api::wren().get_variable)(
            self.0,
            module.as_ptr(),
            name.as_ptr(),
            slot.try_into().unwrap(),
        )
    }
    /// Retrieves the variable with `name` in `module` int `slot`..
    ///
    /// # Safety
    ///
    /// `name` must exist inside `module`.
    #[inline]
    pub unsafe fn get_variable(&mut self, module: &str, name: &str, slot: usize) {
        self.validate_slot(slot);
        self.get_variable_unchecked(module, name, slot)
    }

    /// Retrieves a long-lived [`Handle`] from a short-lived `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a valid `slot`.
    #[inline]
    pub unsafe fn get_slot_handle_unchecked(&mut self, slot: usize) -> Handle {
        Handle((Api::wren().get_slot_handle)(
            self.0,
            slot.try_into().unwrap(),
        ))
    }
    /// Retrieves a long-lived [`Handle`] from a short-lived `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn get_slot_handle(&mut self, slot: usize) -> Handle {
        self.validate_slot(slot);
        // SAFETY: We just validated the slot.
        unsafe { self.get_slot_handle_unchecked(slot) }
    }

    /// Sets `slot` to `handle`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    ///
    /// # Safety
    ///
    /// You must provide this function a valid `slot`.
    #[inline]
    pub unsafe fn set_slot_handle_unchecked(&mut self, slot: usize, handle: Handle) {
        (Api::wren().set_slot_handle)(self.0, slot.try_into().unwrap(), handle.0)
    }
    /// Retrieves a long-lived [`Handle`] from a short-lived `slot`.
    ///
    /// See [Wren docs](https://wren.io/embedding/slots-and-handles.html) for more.
    #[inline]
    pub fn set_slot_handle(&mut self, slot: usize, handle: Handle) {
        self.validate_slot(slot);
        // SAFETY: We just validated the slot.
        unsafe { self.set_slot_handle_unchecked(slot, handle) }
    }
}
