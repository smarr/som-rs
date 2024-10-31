use std::collections::hash_map::DefaultHasher;
use std::convert::{TryFrom, TryInto};
use std::hash::{Hash, Hasher};

use crate::class::Class;
use crate::convert::Primitive;
use crate::gc::VecValue;
use crate::interpreter::Interpreter;
use crate::method::Invoke;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::{Context, Error};
use once_cell::sync::Lazy;
use som_core::interner::Interned;
use som_gc::gcref::GCRef;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("class", self::class.into_func(), true),
        ("objectSize", self::object_size.into_func(), true),
        ("hashcode", self::hashcode.into_func(), true),
        ("perform:", self::perform.into_func(), true),
        (
            "perform:withArguments:",
            self::perform_with_arguments.into_func(),
            true,
        ),
        (
            "perform:inSuperclass:",
            self::perform_in_super_class.into_func(),
            true,
        ),
        (
            "perform:withArguments:inSuperclass:",
            self::perform_with_arguments_in_super_class.into_func(),
            true,
        ),
        ("instVarAt:", self::inst_var_at.into_func(), true),
        ("instVarAt:put:", self::inst_var_at_put.into_func(), true),
        ("==", self::eq.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([]));

fn class(
    _: &mut Interpreter,
    universe: &mut Universe,
    receiver: Value,
) -> Result<GCRef<Class>, Error> {
    Ok(receiver.class(universe))
}

fn object_size(_: &mut Interpreter, _: &mut Universe, receiver: Value) -> Result<i32, Error> {
    const SIGNATURE: &'static str = "Object>>#objectSize";

    std::mem::size_of_val(&receiver)
        .try_into()
        .with_context(|| format!("`{SIGNATURE}`: could not convert `usize` to `i32`"))
}

fn hashcode(_: &mut Interpreter, _: &mut Universe, receiver: Value) -> Result<i32, Error> {
    let mut hasher = DefaultHasher::new();
    receiver.hash(&mut hasher);
    let hash = (hasher.finish() as i32).abs();
    Ok(hash)
}

fn eq(_: &mut Interpreter, _: &mut Universe, receiver: Value, other: Value) -> Result<bool, Error> {
    Ok(receiver == other)
}

fn perform(
    interpreter: &mut Interpreter,
    universe: &mut Universe,
    receiver: Value,
    signature: Interned,
) -> Result<(), Error> {
    const SIGNATURE: &str = "Object>>#perform:";

    let Some(invokable) = receiver.lookup_method(universe, signature) else {
        let signature_str = universe.lookup_symbol(signature).to_owned();
        let args = vec![receiver.clone()];
        return universe
            .does_not_understand(interpreter, receiver.clone(), signature, args)
            .with_context(|| {
                format!(
                    "`{SIGNATURE}`: method `{signature_str}` not found for `{}`",
                    receiver.to_string(universe),
                )
            });
    };

    invokable.invoke(interpreter, universe, receiver, vec![]);
    Ok(())
}

fn perform_with_arguments(
    interpreter: &mut Interpreter,
    universe: &mut Universe,
    receiver: Value,
    signature: Interned,
    arguments: GCRef<VecValue>,
) -> Result<(), Error> {
    const SIGNATURE: &'static str = "Object>>#perform:withArguments:";

    let Some(invokable) = receiver.lookup_method(universe, signature) else {
        let signature_str = universe.lookup_symbol(signature).to_owned();
        let args = std::iter::once(receiver.clone())
            .chain(arguments.0.clone())
            .collect(); // lame clone
        return universe
            .does_not_understand(interpreter, receiver.clone(), signature, args)
            .with_context(|| {
                format!(
                    "`{SIGNATURE}`: method `{signature_str}` not found for `{}`",
                    receiver.to_string(universe)
                )
            });
    };

    invokable.invoke(interpreter, universe, receiver, arguments.0.clone());
    Ok(())
}

fn perform_in_super_class(
    interpreter: &mut Interpreter,
    universe: &mut Universe,
    receiver: Value,
    signature: Interned,
    class: GCRef<Class>,
) -> Result<(), Error> {
    const SIGNATURE: &'static str = "Object>>#perform:inSuperclass:";

    let Some(invokable) = class.lookup_method(signature) else {
        let signature_str = universe.lookup_symbol(signature).to_owned();
        let args = vec![receiver.clone()];
        return universe
            .does_not_understand(interpreter, Value::Class(class), signature, args)
            .with_context(|| {
                format!(
                    "`{SIGNATURE}`: method `{signature_str}` not found for `{}`",
                    receiver.to_string(universe)
                )
            });
    };

    invokable.invoke(interpreter, universe, receiver, vec![]);
    Ok(())
}

fn perform_with_arguments_in_super_class(
    interpreter: &mut Interpreter,
    universe: &mut Universe,
    receiver: Value,
    signature: Interned,
    arguments: GCRef<VecValue>,
    class: GCRef<Class>,
) -> Result<(), Error> {
    const SIGNATURE: &'static str = "Object>>#perform:withArguments:inSuperclass:";

    let method = class.lookup_method(signature);

    let Some(invokable) = method else {
        let signature_str = universe.lookup_symbol(signature).to_owned();
        let args = std::iter::once(receiver.clone())
            .chain(arguments.0.clone())
            .collect(); // lame to clone args, right?
        return universe
            .does_not_understand(interpreter, Value::Class(class), signature, args)
            .with_context(|| {
                format!(
                    "`{SIGNATURE}`: method `{signature_str}` not found for `{}`",
                    receiver.to_string(universe)
                )
            });
    };

    invokable.invoke(interpreter, universe, receiver, arguments.0.clone());
    Ok(())
}

fn inst_var_at(
    _: &mut Interpreter,
    _: &mut Universe,
    receiver: Value,
    index: i32,
) -> Result<Option<Value>, Error> {
    // expect_args!(SIGNATURE, interpreter, [
    //     object => object,
    //     Value::Integer(index) => index,
    // ]);
    //
    // let index = match usize::try_from(index - 1) {
    //     Ok(index) => index,
    //     Err(err) => panic!("'{}': {}", SIGNATURE, err),
    // };
    //
    // let local = match object.has_local(index) {
    //     true => object.lookup_local(index),
    //     false => Value::NIL
    // };
    //
    // interpreter.stack.push(local);

    const _: &'static str = "Object>>#instVarAt:";

    let index = usize::try_from(index.saturating_sub(1))?;

    Ok(Some(receiver.lookup_local(index)))
}

fn inst_var_at_put(
    _: &mut Interpreter,
    _: &mut Universe,
    mut receiver: Value,
    index: i32,
    value: Value,
) -> Result<Option<Value>, Error> {
    const _: &'static str = "Object>>#instVarAt:put:";

    let index = usize::try_from(index.saturating_sub(1))?;

    receiver.assign_local(index, value.clone());

    Ok(Some(value))
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
