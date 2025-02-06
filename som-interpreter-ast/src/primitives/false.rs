use anyhow::Error;
use once_cell::sync::Lazy;

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
    ])
});

pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn not(_universe: &mut Universe, _value_stack: &mut GlobalValueStack, _: Value) -> Result<bool, Error> {
    Ok(true)
}

fn and(_universe: &mut Universe, _value_stack: &mut GlobalValueStack, _self: Value, _other: Value) -> Result<bool, Error> {
    Ok(false)
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
