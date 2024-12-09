use std::marker::PhantomData;

use som_core::value::BaseValue;
use som_gc::gcref::Gc;

use crate::gc::VecValue;
use crate::value::{ARRAY_TAG, BLOCK_TAG, CLASS_TAG, INSTANCE_TAG, INVOKABLE_TAG};
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::instance::Instance;
use crate::{value::Value, vm_objects::method::Method};

#[repr(transparent)]
pub struct ValuePtr<T> {
    value: BaseValue,
    _phantom: PhantomData<T>,
}

// impl<T> Deref for ValuePtr<T> {
//     type Target = BaseValue;
//
//     fn deref(&self) -> &Self::Target {
//         &self.value
//     }
// }

pub trait HasPointerTag {
    fn get_tag() -> u64;
}

impl<T> ValuePtr<T>
where
    T: HasPointerTag,
{
    pub fn new(value: Gc<T>) -> Self {
        Self {
            value: BaseValue::new(T::get_tag(), value.into()),
            _phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn is_valid_ptr_to(&self) -> bool {
        self.value.tag() == T::get_tag()
    }

    #[inline(always)]
    pub fn get_ptr(&self) -> Option<Gc<T>> {
        self.is_valid_ptr_to().then(|| self.value.extract_gc_cell())
    }

    #[inline(always)]
    pub fn _as_ptr_unchecked(&self) -> Gc<T> {
        self.value.extract_gc_cell()
    }
}

impl<T> From<Value> for ValuePtr<T> {
    fn from(value: Value) -> Self {
        Self {
            value: value.0,
            _phantom: PhantomData,
        }
    }
}

impl<T> From<ValuePtr<T>> for Value {
    fn from(val: ValuePtr<T>) -> Self {
        Value(val.value)
    }
}

// ----

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
