use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use crate::expect_args;

pub static INSTANCE_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[
    ("holder", self::holder, true),
    ("signature", self::signature, true),
    ("invokeOn:with:", self::invoke_on_with, true),
];
pub static CLASS_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[];

fn holder(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Method>>#holder";

    expect_args!(SIGNATURE, args, [
        Value::Invokable(invokable)
    ]);

    match invokable.holder().upgrade() {
        Some(holder) => interpreter.stack.push(Value::Class(holder)),
        None => panic!("'{}': method sholder has been collected", SIGNATURE),
    }
}

fn signature(interpreter: &mut Interpreter, args: Vec<Value>, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Method>>#signature";

    expect_args!(SIGNATURE, args, [
        Value::Invokable(invokable)
    ]);

    let sym = universe.intern_symbol(invokable.signature());
    interpreter.stack.push(Value::Symbol(sym))
}

fn invoke_on_with(interpreter: &mut Interpreter, args: Vec<Value>, universe: &mut UniverseBC) {
    const SIGNATURE: &str = "Method>>#invokeOn:with:";

    expect_args!(SIGNATURE, args, [
        Value::Invokable(invokable),
        receiver,
        Value::Array(args)
    ]);

    let args = args.borrow().iter().cloned().collect();
    invokable.clone().invoke(interpreter, universe, receiver.clone(), args);
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
