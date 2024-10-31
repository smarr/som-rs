use crate::class::Class;
use crate::convert::Primitive;
use crate::gc::VecValue;
use crate::instance::Instance;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::Error;
use once_cell::sync::Lazy;
use som_gc::gcref::GCRef;

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

fn superclass(_: &mut Universe, receiver: GCRef<Class>) -> Result<Value, Error> {
    let super_class = receiver.super_class();
    Ok(super_class.map(Value::Class).unwrap_or(Value::NIL))
}

fn new(universe: &mut Universe, receiver: GCRef<Class>) -> Result<Value, Error> {
    let instance = Instance::from_class(receiver);
    let instance_ptr = universe.gc_interface.alloc(instance);
    Ok(Value::Instance(instance_ptr))
}

fn name(universe: &mut Universe, receiver: GCRef<Class>) -> Result<Value, Error> {
    let sym = universe.intern_symbol(receiver.name());
    Ok(Value::Symbol(sym))
}

fn methods(universe: &mut Universe, receiver: GCRef<Class>) -> Result<Value, Error> {
    let methods = receiver
        .methods
        .values()
        .map(|invokable| Value::Invokable(invokable.clone()))
        .collect();

    Ok(Value::Array(universe.gc_interface.alloc(VecValue(methods))))
}

fn fields(universe: &mut Universe, receiver: GCRef<Class>) -> Result<Value, Error> {
    let fields = receiver
        .get_all_field_names()
        .iter()
        .map(|field_name| Value::String(universe.gc_interface.alloc(field_name.clone())))
        .collect();

    Ok(Value::Array(universe.gc_interface.alloc(VecValue(fields))))
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
