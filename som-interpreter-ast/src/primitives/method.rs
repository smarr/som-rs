use crate::convert::Primitive;
use crate::gc::VecValue;
use crate::invokable::{Invoke, Return};
use crate::method::Method;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::Error;
use once_cell::sync::Lazy;
use som_gc::gcref::GCRef;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("holder", self::holder.into_func(), true),
        ("signature", self::signature.into_func(), true),
        ("invokeOn:with:", self::invoke_on_with.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([]));

fn holder(_: &mut Universe, invokable: GCRef<Method>) -> Result<Value, Error> {
    let holder = invokable.holder();
    Ok(Value::Class(*holder))

    // match maybe_holder {
    //     Some(holder) => Ok(Value::Class(holder)),
    //     None => bail!(format!(
    //         "'{}': method holder has been collected",
    //         SIGNATURE
    //     )),
    // }
}

fn signature(universe: &mut Universe, invokable: GCRef<Method>) -> Result<Value, Error> {
    let sym = universe.intern_symbol(invokable.signature());
    Ok(Value::Symbol(sym))
}

fn invoke_on_with(
    universe: &mut Universe,
    mut invokable: GCRef<Method>,
    receiver: Value,
    arguments: GCRef<VecValue>,
) -> Result<Return, Error> {
    let args = std::iter::once(receiver.clone())
        .chain(arguments.iter().cloned())
        .collect();

    Ok(invokable.invoke(universe, args))
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
