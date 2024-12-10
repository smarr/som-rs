use super::PrimInfo;
use crate::gc::VecValue;
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::Primitive;
use crate::value::HeapValPtr;
use crate::value::Value;
use crate::vm_objects::method::Method;
use anyhow::Error;
use once_cell::sync::Lazy;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("holder", self::holder.into_func(), true),
        ("signature", self::signature.into_func(), true),
        ("invokeOn:with:", self::invoke_on_with.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn holder(_: &mut Universe, invokable: HeapValPtr<Method>) -> Result<Value, Error> {
    let method = invokable.deref();
    let holder = method.holder();
    Ok(Value::Class(*holder))

    // match maybe_holder {
    //     Some(holder) => Ok(Value::Class(holder)),
    //     None => bail!(format!(
    //         "'{}': method holder has been collected",
    //         SIGNATURE
    //     )),
    // }
}

fn signature(universe: &mut Universe, invokable: HeapValPtr<Method>) -> Result<Value, Error> {
    let sym = universe.intern_symbol(invokable.deref().signature());
    Ok(Value::Symbol(sym))
}

#[allow(unused)]
fn invoke_on_with(
    universe: &mut Universe,
    mut invokable: HeapValPtr<Method>,
    receiver: Value,
    arguments: HeapValPtr<VecValue>,
) -> Result<Return, Error> {
    todo!()
    // let args = std::iter::once(receiver).chain(arguments.iter().cloned()).collect();

    // Ok(invokable.invoke(universe, args))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
