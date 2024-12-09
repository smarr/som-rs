use super::nan_boxed_val::{ARRAY_TAG, BLOCK_TAG, CLASS_TAG, INSTANCE_TAG, INVOKABLE_TAG};
use crate::gc::VecValue;
use crate::value::Value;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::instance::Instance;
use crate::vm_objects::method::Method;
use som_core::value::{HasPointerTag, TypedPtrValue};
use som_gc::gcref::Gc;
use std::marker::PhantomData;

impl<T> From<Value> for TypedPtrValue<T, Gc<T>> {
    fn from(value: Value) -> Self {
        value.0.into()
    }
}

impl<T> From<TypedPtrValue<T, Gc<T>>> for Value {
    fn from(val: TypedPtrValue<T, Gc<T>>) -> Self {
        Value(val.into())
    }
}

impl HasPointerTag for VecValue {
    fn get_tag() -> u64 {
        ARRAY_TAG
    }
}

impl HasPointerTag for Block {
    fn get_tag() -> u64 {
        BLOCK_TAG
    }
}

impl HasPointerTag for Class {
    fn get_tag() -> u64 {
        CLASS_TAG
    }
}

impl HasPointerTag for Method {
    fn get_tag() -> u64 {
        INVOKABLE_TAG
    }
}

impl HasPointerTag for Instance {
    fn get_tag() -> u64 {
        INSTANCE_TAG
    }
}

/// A reference to a value, and its associated type as a pointer.
/// This isn't actually used outside of the static case... But I wish it were.
pub struct ValuePtrRef<'a, T> {
    pub value_ref: &'a Value,
    _phantom: PhantomData<T>,
}

/// A static reference to a value, and its associated pointer type.
/// This is a HACK: the pointer is NOT static, but as with other places in this codebase, it's "static-ish":
/// completely static unless GC happens, in which case we're holding onto a reference to the pointer which gets updated. So it's as if it was static?
pub type HeapValPtr<T> = ValuePtrRef<'static, T>;

impl<T: HasPointerTag> HeapValPtr<T> {
    /// Creates a new static reference from the provided `Value` reference.
    /// # Safety
    /// `value_ref` must NOT be a temporary reference, and MUST point to the GC heap. Otherwise, all hell breaks loose.
    pub unsafe fn new_static(value_ref: &Value) -> HeapValPtr<T> {
        debug_assert!(value_ref.is_ptr_type() && value_ref.tag() == T::get_tag());
        Self {
            value_ref: std::mem::transmute::<&Value, &'static Value>(value_ref),
            _phantom: PhantomData,
        }
    }
}

impl<T: HasPointerTag> ValuePtrRef<'_, T> {
    pub fn new(value_ref: &'static Value) -> Self {
        ValuePtrRef {
            value_ref,
            _phantom: PhantomData,
        }
    }

    pub fn deref(&self) -> Gc<T> {
        *self.value_ref.as_value_gc_ptr().as_ref().unwrap()
    }
}
