use super::PrimInfo;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::Primitive;
use crate::value::Value;
use anyhow::Error;
use once_cell::sync::Lazy;
use som_core::interner::Interned;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("asString", self::as_string.into_func(), true)]));

pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn as_string(universe: &mut Universe, sym: Interned) -> Result<Value, Error> {
    Ok(Value::String(universe.gc_interface.alloc(universe.lookup_symbol(sym).to_string())))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
