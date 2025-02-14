use crate::gc::VecValue;
use crate::interpreter::Interpreter;
use crate::primitives::PrimInfo;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::Primitive;
use crate::value::{HeapValPtr, Value};
use crate::vm_objects::class::Class;
use crate::vm_objects::method::{Invoke, Method};
use anyhow::Error;
use once_cell::sync::Lazy;
use som_gc::gcref::Gc;
use som_value::interned::Interned;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("holder", self::holder.into_func(), true),
        ("signature", self::signature.into_func(), true),
        ("invokeOn:with:", self::invoke_on_with.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn holder(invokable: HeapValPtr<Method>) -> Result<Gc<Class>, Error> {
    const _: &str = "Method>>#holder";

    Ok(*invokable.deref().holder())
}

fn signature(_: &mut Interpreter, universe: &mut Universe, invokable: HeapValPtr<Method>) -> Result<Interned, Error> {
    Ok(universe.intern_symbol(invokable.deref().signature()))
}

fn invoke_on_with(
    interpreter: &mut Interpreter,
    universe: &mut Universe,
    invokable: HeapValPtr<Method>,
    receiver: Value,
    arguments: HeapValPtr<VecValue>,
) -> Result<(), Error> {
    const _: &str = "Method>>#invokeOn:with:";

    invokable.deref().invoke(
        interpreter,
        universe,
        receiver,
        arguments.deref().0.clone(), // todo lame to clone tbh
    );
    Ok(())
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
