use crate::class::Class;
use crate::convert::Primitive;
use crate::instance::Instance;
use crate::interner::Interned;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use anyhow::Error;
use once_cell::sync::Lazy;
use som_core::gc::GCRef;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new({
        [
            ("new", self::new.into_func(), true),
            ("name", self::name.into_func(), true),
            ("fields", self::fields.into_func(), true),
            ("methods", self::methods.into_func(), true),
            ("superclass", self::superclass.into_func(), true),
        ]
    })
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([]));

fn superclass(
    interpreter: &mut Interpreter,
    _: &mut UniverseBC,
    receiver: GCRef<Class>,
) -> Result<(), Error> {
    const _: &str = "Class>>#superclass";

    let super_class = receiver.borrow().super_class();
    let super_class = super_class.map_or(Value::NIL, |it| Value::Class(it));
    interpreter.stack.push(super_class);

    Ok(())
}

fn new(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: GCRef<Class>,
) -> Result<GCRef<Instance>, Error> {
    const _: &str = "Class>>#new";

    let instance = Instance::from_class(receiver, &mut universe.gc_interface);

    Ok(instance)
}

fn name(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: GCRef<Class>,
) -> Result<Interned, Error> {
    const _: &str = "Class>>#name";

    Ok(universe.intern_symbol(receiver.borrow().name()))
}

fn methods(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: GCRef<Class>,
) -> Result<GCRef<Vec<Value>>, Error> {
    const _: &str = "Class>>#methods";

    let methods = receiver
        .borrow()
        .methods
        .values()
        .copied()
        .map(Value::Invokable)
        .collect();

    Ok(universe.gc_interface.allocate(methods))
}

fn fields(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: GCRef<Class>,
) -> Result<GCRef<Vec<Value>>, Error> {
    const _: &str = "Class>>#fields";

    let fields = receiver
        .borrow()
        .locals
        .keys()
        .copied()
        .map(Value::Symbol)
        .collect();

    Ok(universe.gc_interface.allocate(fields))
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
