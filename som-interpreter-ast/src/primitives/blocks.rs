use super::PrimInfo;
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use once_cell::sync::Lazy;

/// Primitives for the **Block** and **Block1** class.
pub mod block1 {
    use super::*;
    use crate::convert::Primitive;
    use crate::evaluate::Evaluate;
    use crate::vm_objects::block::Block;
    use anyhow::Error;
    use som_gc::gcref::Gc;

    pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> =
        Lazy::new(|| Box::new([("value", self::value.into_func(), true), ("restart", self::restart.into_func(), false)]));
    pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

    fn value(universe: &mut Universe, mut block: Gc<Block>) -> Result<Return, Error> {
        let nbr_locals = block.block.nbr_locals;
        Ok(universe.with_frame(nbr_locals, vec![Value::Block(block)], |universe| block.evaluate(universe)))
    }

    fn restart(_: &mut Universe, _: Gc<Block>) -> Result<Return, Error> {
        #[cfg(feature = "inlining-disabled")]
        return Ok(Return::Restart);
        panic!("calling restart even though inlining is enabled. we don't support this")
    }

    /// Search for an instance primitive matching the given signature.
    pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
        INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
    }

    /// Search for a class primitive matching the given signature.
    pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
        CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
    }
}

/// Primitives for the **Block2** class.
pub mod block2 {
    use super::*;
    use crate::convert::Primitive;
    use crate::evaluate::Evaluate;
    use crate::vm_objects::block::Block;
    use anyhow::Error;
    use som_gc::gcref::Gc;

    pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("value:", self::value.into_func(), true)]));
    pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

    fn value(universe: &mut Universe, mut block: Gc<Block>, argument: Value) -> Result<Return, Error> {
        let nbr_locals = block.block.nbr_locals;

        Ok(universe.with_frame(nbr_locals, vec![Value::Block(block), argument], |universe| block.evaluate(universe)))
    }

    /// Search for an instance primitive matching the given signature.
    pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
        INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
    }

    /// Search for a class primitive matching the given signature.
    pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
        CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
    }
}

/// Primitives for the **Block3** class.
pub mod block3 {
    use super::*;
    use crate::convert::Primitive;
    use crate::evaluate::Evaluate;
    use crate::vm_objects::block::Block;
    use anyhow::Error;
    use som_gc::gcref::Gc;

    pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("value:with:", self::value_with.into_func(), true)]));
    pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

    fn value_with(universe: &mut Universe, mut receiver: Gc<Block>, argument1: Value, argument2: Value) -> Result<Return, Error> {
        let nbr_locals = receiver.block.nbr_locals;

        Ok(
            universe.with_frame(nbr_locals, vec![Value::Block(receiver), argument1, argument2], |universe| {
                receiver.evaluate(universe)
            }),
        )
    }

    /// Search for an instance primitive matching the given signature.
    pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
        INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
    }

    /// Search for a class primitive matching the given signature.
    pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
        CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
    }
}
