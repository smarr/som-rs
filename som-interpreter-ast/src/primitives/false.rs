use anyhow::Error;
use once_cell::sync::Lazy;

use crate::invokable::Return;
use crate::primitives::PrimInfo;
use crate::primitives::PrimitiveFn;
use crate::universe::GlobalValueStack;
use crate::universe::Universe;
use crate::value::convert::Primitive;
use crate::value::Value;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("not", self::not.into_func(), true),
        ("and:", self::and.into_func(), true),
        ("&&", self::and.into_func(), true),
        ("or:", self::or_if_false.into_func(), true),
        ("||", self::or_if_false.into_func(), true),
        ("ifFalse:", self::or_if_false.into_func(), true),
        ("ifTrue:", self::if_true.into_func(), true),
    ])
});

pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn not(_universe: &mut Universe, _value_stack: &mut GlobalValueStack, _: Value) -> Result<bool, Error> {
    Ok(true)
}

fn and(_universe: &mut Universe, _value_stack: &mut GlobalValueStack, _self: Value, _other: Value) -> Result<bool, Error> {
    Ok(false)
}

fn or_if_false(universe: &mut Universe, value_stack: &mut GlobalValueStack, _self: Value, other: Value) -> Result<Return, Error> {
    if let Some(blk) = other.as_block() {
        value_stack.push(other);
        Ok(universe.eval_block_with_frame(value_stack, blk.block.nbr_locals, 1))
    } else {
        Ok(Return::Local(other))
    }
}

fn if_true(_universe: &mut Universe, _value_stack: &mut GlobalValueStack, _self: Value, _other: Value) -> Result<Return, Error> {
    Ok(Return::Local(Value::NIL))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
