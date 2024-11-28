use anyhow::Error;
use once_cell::sync::Lazy;
use som_gc::gcref::Gc;

use crate::interpreter::Interpreter;
use crate::primitives::PrimInfo;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::Primitive;
use som_core::interner::Interned;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("asString", self::as_string.into_func(), true)]));
pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn as_string(_: &mut Interpreter, universe: &mut Universe, symbol: Interned) -> Result<Gc<String>, Error> {
    const _: &str = "Symbol>>#asString";

    Ok(universe.gc_interface.alloc(universe.lookup_symbol(symbol).to_owned()))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
