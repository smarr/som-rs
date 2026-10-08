use anyhow::Error;
use once_cell::sync::Lazy;

use crate::interpreter::Interpreter;
use crate::primitives::PrimInfo;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::Primitive;
use crate::value::Value;
use crate::{stack_drop, stack_last, stack_pop, stack_push};

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("not", self::not.into_func(), true),
        ("or:", self::or.into_func(), true),
        ("||", self::or.into_func(), true),
        ("and:", self::and_if_true.into_func(), true),
        ("&&", self::and_if_true.into_func(), true),
        ("ifTrue:", self::and_if_true.into_func(), true),
        ("ifFalse:", self::if_false.into_func(), true),
    ])
});

pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn not(_: bool) -> Result<bool, Error> {
    Ok(false)
}

fn or(_self: Value, _other: Value) -> Result<bool, Error> {
    Ok(true)
}

/// See equivalent function for the false primitive.
fn and_if_true(interpreter: &mut Interpreter, universe: &mut Universe) -> Result<(), Error> {
    let cond_val = stack_last!(interpreter.sp);

    if cond_val.as_block().is_some() {
        interpreter.push_block_frame(1, &mut universe.gc_interface);
        let stack_marker = stack_pop!(interpreter.sp);
        *stack_last!(interpreter.sp) = stack_marker; // the "True". the "Block" was already consumed and put into the new frame
    } else {
        stack_drop!(interpreter.sp);
        stack_drop!(interpreter.sp);
        stack_push!(interpreter.sp, *cond_val);
    }
    Ok(())
}

fn if_false(_self: Value, _other: Value) -> Result<Value, Error> {
    Ok(Value::NIL)
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
