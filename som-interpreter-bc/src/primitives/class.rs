use som_core::gc::GCRef;
use crate::instance::Instance;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use crate::{expect_args, reverse};

pub static INSTANCE_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[
    ("new", self::new, true),
    ("name", self::name, true),
    ("fields", self::fields, true),
    ("methods", self::methods, true),
    ("superclass", self::superclass, true),
];
pub static CLASS_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[];

fn superclass(interpreter: &mut Interpreter, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Class>>#superclass";

    expect_args!(SIGNATURE, interpreter, [
        Value::Class(class) => class,
    ]);

    let super_class = class.to_obj().super_class();
    interpreter
        .stack
        .push(super_class.map(Value::Class).unwrap_or(Value::Nil));
}

fn new(interpreter: &mut Interpreter, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Class>>#new";

    expect_args!(SIGNATURE, interpreter, [
        Value::Class(class) => class,
    ]);

    let instance_ref = Instance::from_class(class, &mut universe.gc_interface);
    interpreter.stack.push(Value::Instance(instance_ref));
}

fn name(interpreter: &mut Interpreter, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Class>>#name";

    expect_args!(SIGNATURE, interpreter, [
        Value::Class(class) => class,
    ]);

    let sym = universe.intern_symbol(class.to_obj().name());
    interpreter.stack.push(Value::Symbol(sym));
}

fn methods(interpreter: &mut Interpreter, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Class>>#methods";

    expect_args!(SIGNATURE, interpreter, [
        Value::Class(class) => class,
    ]);

    let methods = class
        .to_obj()
        .methods
        .values()
        .map(|invokable| Value::Invokable(invokable.clone()))
        .collect();

    interpreter
        .stack
        .push(Value::Array(GCRef::<Vec<Value>>::alloc(methods, &mut universe.gc_interface)));
}

fn fields(interpreter: &mut Interpreter, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Class>>#fields";

    expect_args!(SIGNATURE, interpreter, [
        Value::Class(class) => class,
    ]);

    interpreter.stack.push(Value::Array(GCRef::<Vec<Value>>::alloc(
        class
            .to_obj()
            .locals
            .keys()
            .copied()
            .map(Value::Symbol)
            .collect(),
        &mut universe.gc_interface,
    )));
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<PrimitiveFn> {
    INSTANCE_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<PrimitiveFn> {
    CLASS_PRIMITIVES
        .iter()
        .find(|it| it.0 == signature)
        .map(|it| it.1)
}
