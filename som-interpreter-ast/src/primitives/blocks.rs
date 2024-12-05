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
    use crate::vm_objects::block::Block;
    use anyhow::Error;
    use som_gc::gcref::Gc;

    pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> =
        Lazy::new(|| Box::new([("value", self::value.into_func(), true), ("restart", self::restart.into_func(), false)]));
    pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

    fn value(universe: &mut Universe, block: Gc<Block>) -> Result<Return, Error> {
        let nbr_locals = block.block.nbr_locals;
        universe.stack_args.push(Value::Block(block));
        Ok(universe.eval_block_with_frame(nbr_locals, 1))
    }

    fn restart(_: &mut Universe, _: Gc<Block>) -> Result<Return, Error> {
        #[cfg(feature = "inlining-disabled")]
        return Ok(Return::Restart);
        #[cfg(not(feature = "inlining-disabled"))]
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
    use crate::vm_objects::block::Block;
    use anyhow::Error;
    use som_gc::debug_assert_valid_semispace_ptr;
    use som_gc::gcref::Gc;

    pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("value:", self::value.into_func(), true)]));
    pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

    fn value(universe: &mut Universe, block: Gc<Block>, argument: Value) -> Result<Return, Error> {
        debug_assert_valid_semispace_ptr!(block);

        let nbr_locals = block.block.nbr_locals;
        universe.stack_args.push(Value::Block(block));
        universe.stack_args.push(argument);

        Ok(universe.eval_block_with_frame(nbr_locals, 2))
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
    use crate::vm_objects::block::Block;
    use anyhow::Error;
    use som_gc::gcref::Gc;

    pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([("value:with:", self::value_with.into_func(), true)]));
    pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

    fn value_with(universe: &mut Universe, receiver: Gc<Block>, argument1: Value, argument2: Value) -> Result<Return, Error> {
        let nbr_locals = receiver.block.nbr_locals;

        universe.stack_args.push(Value::Block(receiver));
        universe.stack_args.push(argument1);
        universe.stack_args.push(argument2);

        Ok(universe.eval_block_with_frame(nbr_locals, 3))
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
