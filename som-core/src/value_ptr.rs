use std::{marker::PhantomData, ops::Deref};

use num_bigint::BigInt;

use crate::value::{BaseValue, BIG_INTEGER_TAG, STRING_TAG};

// TODO: sort out these structs a bit, I think at least one of the 3 is redundant.

/// Bundles a value to a pointer with the type to its pointer.
#[repr(transparent)]
pub struct TypedPtrValue<T, PTR> {
    value: BaseValue,
    _phantom: PhantomData<T>,
    _phantom2: PhantomData<PTR>,
}

pub trait HasPointerTag {
    fn get_tag() -> u64;
}

impl<T, PTR> TypedPtrValue<T, PTR>
where
    T: HasPointerTag,
    PTR: Deref<Target = T> + Into<u64> + From<u64>,
{
    pub fn new(value: PTR) -> Self {
        Self {
            value: BaseValue::new(T::get_tag(), value.into()),
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.value.tag() == T::get_tag()
    }

    /// Returns the underlying pointer value.
    #[inline(always)]
    pub fn get(&self) -> Option<PTR> {
        self.is_valid().then(|| self.value.extract_gc_cell())
    }

    /// Returns the underlying pointer value, without checking if it is valid.
    /// # Safety
    /// Fine to invoke so long as we've previously checked we're working with a valid pointer.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self) -> PTR {
        debug_assert!(self.get().is_some());
        self.value.extract_gc_cell()
    }
}

impl<T, PTR> From<BaseValue> for TypedPtrValue<T, PTR> {
    fn from(value: BaseValue) -> Self {
        Self {
            value,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<T, PTR> From<TypedPtrValue<T, PTR>> for BaseValue {
    fn from(val: TypedPtrValue<T, PTR>) -> Self {
        val.value
    }
}

impl HasPointerTag for String {
    fn get_tag() -> u64 {
        STRING_TAG
    }
}

impl HasPointerTag for BigInt {
    fn get_tag() -> u64 {
        BIG_INTEGER_TAG
    }
}
/// A reference to a value, and its associated type as a pointer.
/// This isn't actually used outside of the static case... But I wish it were.
pub struct ValuePtrRef<'a, T, PTR> {
    pub value_ref: &'a BaseValue,
    _phantom: PhantomData<T>,
    _phantom2: PhantomData<PTR>,
}

/// A static reference to a value, and its associated pointer type.
/// This is a HACK: in our usages the pointer is NOT static, but a pointer to the GC heap, which means it's "pretty-much-static-ish":
/// it's completely static unless GC happens, in which case we're holding onto a ref to the pointer which gets updated. So it's as if it was static?
pub type ValStaticPtr<T, PTR> = ValuePtrRef<'static, T, PTR>;

impl<T: HasPointerTag, PTR> ValStaticPtr<T, PTR> {
    /// Creates a new static reference from the provided `Value` reference.
    /// # Safety
    /// `value_ref` must NOT be a temporary reference, and MUST point to the GC heap. Otherwise, all hell breaks loose.
    pub unsafe fn new_static(value_ref: &BaseValue) -> ValStaticPtr<T, PTR> {
        debug_assert!(value_ref.is_ptr_type() && value_ref.tag() == T::get_tag());
        Self {
            value_ref: std::mem::transmute::<&BaseValue, &'static BaseValue>(value_ref),
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<T: HasPointerTag, PTR> ValuePtrRef<'_, T, PTR>
where
    PTR: Deref<Target = T> + From<u64> + Into<u64>,
{
    pub fn new(value_ref: &'static BaseValue) -> Self {
        ValuePtrRef {
            value_ref,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }

    pub fn deref(&self) -> PTR {
        unsafe { self.value_ref.as_ptr_unchecked::<T, PTR>() }
    }
}
