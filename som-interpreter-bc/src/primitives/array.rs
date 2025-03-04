use std::convert::{TryFrom, TryInto};

use super::PrimInfo;
use crate::gc::VecValue;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::{IntoValue, Primitive};
use crate::value::{HeapValPtr, Value};
use anyhow::{Context, Error};
use once_cell::sync::Lazy;
use som_gc::gc_interface::AllocSiteMarker;
use som_gc::gcref::Gc;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("at:", self::at.into_func(), true),
        ("at:put:", self::at_put.into_func(), true),
        ("length", self::length.into_func(), true),
        ("copy:", self::copy.into_func(), true),
        //("putAll:", self::put_all.into_func(), true),
        //("do:", self::do.into_func(), true),
        //("doIndexes:", self::do_indexes.into_func(), true),
    ])
});

pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("new:", self::new.into_func(), true)]));

fn at(receiver: HeapValPtr<VecValue>, index: i32) -> Result<Value, Error> {
    const _: &str = "Array>>#at:";

    let index = usize::try_from(index - 1)?;

    receiver.deref().0.get(index).cloned().context("index out of bounds")
}

fn at_put(receiver: HeapValPtr<VecValue>, index: i32, value: Value) -> Result<Gc<VecValue>, Error> {
    const _: &str = "Array>>#at:put:";

    let index = usize::try_from(index - 1)?;

    if let Some(location) = receiver.deref().0.get_mut(index) {
        *location = value;
    }

    Ok(receiver.deref())
}

fn length(receiver: HeapValPtr<VecValue>) -> Result<i32, Error> {
    receiver.deref().0.len().try_into().context("could not convert `usize` to `i32`")
}

fn new(interp: &mut Interpreter, universe: &mut Universe) -> Result<(), Error> {
    // this whole thing is an attempt at fixing a GC related bug when allocating an Array
    // I think it did work, to be fair... but TODO clean up.

    std::hint::black_box(&interp.current_frame);

    let count = usize::try_from(interp.get_current_frame().stack_pop().as_integer().unwrap())?;
    interp.get_current_frame().stack_pop(); // receiver is just an unneeded Array class

    let mut arr_ptr: Gc<VecValue> = universe.gc_interface.request_memory_for_type(size_of::<VecValue>(), Some(AllocSiteMarker::Array));
    *arr_ptr = VecValue(vec![Value::NIL; count]);

    interp.get_current_frame().stack_push(arr_ptr.into_value());
    Ok(())
}

fn copy(_: &mut Interpreter, universe: &mut Universe, arr: HeapValPtr<VecValue>) -> Result<Gc<VecValue>, Error> {
    let copied_arr = VecValue((*arr.deref()).0.clone());
    let allocated: Gc<VecValue> = universe.gc_interface.alloc(copied_arr);
    Ok(allocated)
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
