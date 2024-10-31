use std::convert::{TryFrom, TryInto};

use crate::convert::Primitive;
use crate::gc::VecValue;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::{Context, Error};
use once_cell::sync::Lazy;
use som_gc::gcref::GCRef;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("at:", self::at.into_func(), true),
        ("at:put:", self::at_put.into_func(), true),
        ("length", self::length.into_func(), true),
    ])
});

pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([("new:", self::new.into_func(), true)]));

fn at(
    _: &mut Interpreter,
    _: &mut Universe,
    receiver: GCRef<VecValue>,
    index: i32,
) -> Result<Value, Error> {
    const _: &str = "Array>>#at:";

    let index = usize::try_from(index - 1)?;

    receiver
        .0
        .get(index)
        .cloned()
        .context("index out of bounds")
}

fn at_put(
    _: &mut Interpreter,
    _: &mut Universe,
    mut receiver: GCRef<VecValue>,
    index: i32,
    value: Value,
) -> Result<GCRef<VecValue>, Error> {
    const _: &str = "Array>>#at:put:";

    let index = usize::try_from(index - 1)?;

    if let Some(location) = receiver.0.get_mut(index) {
        *location = value;
    }

    Ok(receiver)
}

fn length(_: &mut Interpreter, _: &mut Universe, receiver: GCRef<VecValue>) -> Result<i32, Error> {
    const _: &str = "Array>>#length";

    receiver
        .0
        .len()
        .try_into()
        .context("could not convert `usize` to `i32`")
}

fn new(
    _: &mut Interpreter,
    universe: &mut Universe,
    _: Value,
    count: i32,
) -> Result<GCRef<VecValue>, Error> {
    const _: &str = "Array>>#new:";

    let count = usize::try_from(count)?;
    let allocated = universe
        .gc_interface
        .alloc(VecValue(vec![Value::NIL; count]));

    Ok(allocated)
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}
