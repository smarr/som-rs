use std::convert::TryFrom;

use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use crate::{expect_args, reverse};
use som_core::gc::GCRef;

pub static INSTANCE_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[
    ("at:", self::at, true),
    ("at:put:", self::at_put, true),
    ("length", self::length, true),
];

pub static CLASS_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[("new:", self::new, true)];

fn at(interpreter: &mut Interpreter, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Array>>#at:";

    expect_args!(SIGNATURE, interpreter, [
        Value::Array(values) => values,
        Value::Integer(index) => index,
    ]);

    let index = match usize::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => panic!("'{}': {}", SIGNATURE, err),
    };
    let value = values.to_obj().get(index).cloned().unwrap_or(Value::Nil);
    interpreter.stack.push(value)
}

fn at_put(interpreter: &mut Interpreter, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Array>>#at:put:";

    expect_args!(SIGNATURE, interpreter, [
        Value::Array(values) => values,
        Value::Integer(index) => index,
        value => value,
    ]);

    let index = match usize::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => panic!("'{}': {}", SIGNATURE, err),
    };
    if let Some(location) = values.to_obj().get_mut(index) {
        *location = value;
    }
    interpreter.stack.push(Value::Array(values))
}

fn length(interpreter: &mut Interpreter, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Array>>#length";

    expect_args!(SIGNATURE, interpreter, [
        Value::Array(values) => values,
    ]);

    let length = values.to_obj().len();
    match i64::try_from(length) {
        Ok(length) => interpreter.stack.push(Value::Integer(length)),
        Err(err) => panic!("'{}': {}", SIGNATURE, err),
    }
}

fn new(interpreter: &mut Interpreter, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Array>>#new:";

    expect_args!(SIGNATURE, interpreter, [
        _,
        Value::Integer(count) => count,
    ]);

    match usize::try_from(count) {
        Ok(length) => interpreter
            .stack
            .push(Value::Array(
                GCRef::<Vec<Value>>::alloc(vec![
                Value::Nil;
                length
            ], &mut universe.gc_interface))),
        Err(err) => panic!("'{}': {}", SIGNATURE, err),
    }
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<PrimitiveFn> {
    INSTANCE_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<PrimitiveFn> {
    CLASS_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}
