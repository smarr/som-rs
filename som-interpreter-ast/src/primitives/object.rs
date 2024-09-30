use crate::convert::Primitive;
use crate::invokable::{Invoke, Return};
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseAST;
use crate::value::Value;
use crate::value::Value::Nil;
use once_cell::sync::Lazy;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use som_core::gc::GCRef;
use crate::class::Class;
use crate::interner::Interned;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("halt", self::halt.into_func(), true),
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


fn halt(_: &mut UniverseAST, _: Value) -> Return {
    println!("HALT"); // so a breakpoint can be put
    Return::Local(Nil)
}

fn class(universe: &mut UniverseAST, object: Value) -> Return {
    Return::Local(Value::Class(object.class(universe)))
}

fn object_size(_: &mut UniverseAST, _: Value) -> Return {
    const _: &'static str = "Object>>#objectSize";

    Return::Local(Value::Integer(std::mem::size_of::<Value>() as i32))
}

fn hashcode(_: &mut UniverseAST, receiver: Value) -> Return {
    let mut hasher = DefaultHasher::new();
    receiver.hash(&mut hasher);
    let hash = (hasher.finish() as i32).abs();

    Return::Local(Value::Integer(hash))
}

fn eq(_: &mut UniverseAST, receiver: Value, other: Value) -> Return {
    Return::Local(Value::Boolean(receiver == other))
}

fn perform(universe: &mut UniverseAST, object: Value, sym: Interned) -> Return {
    const SIGNATURE: &'static str = "Object>>#perform:";

    let signature = universe.lookup_symbol(sym);
    let method = object.lookup_method(universe, signature);

    match method {
        Some(invokable) => invokable.to_obj().invoke(universe, vec![object]),
        None => {
            let signature = signature.to_string();
            universe
                .does_not_understand(object.clone(), signature.as_str(), vec![object.clone()])
                .unwrap_or_else(|| {
                    Return::Exception(format!(
                        "'{}': method '{}' not found for '{}'",
                        SIGNATURE,
                        signature,
                        object.to_string(universe)
                    ))
                    // Return::Local(Value::Nil)
                })
        }
    }
}

fn perform_with_arguments(universe: &mut UniverseAST, object: Value, sym: Interned, arr: GCRef<Vec<Value>>) -> Return {
    const SIGNATURE: &'static str = "Object>>#perform:withArguments:";

    let signature = universe.lookup_symbol(sym);
    let method = object.lookup_method(universe, signature);

    match method {
        Some(invokable) => {
            // let args = std::iter::once(object)
            //     .chain(arr.replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object).chain(arr.to_obj().clone()).collect();
            invokable.to_obj().invoke(universe, args)
        }
        None => {
            let signature = signature.to_string();
            // let args = std::iter::once(object.clone())
            //     .chain(arr.to_obj().replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object.clone()).chain(arr.to_obj().clone()).collect();

            universe
                .does_not_understand(object.clone(), signature.as_str(), args)
                .unwrap_or_else(|| {
                    Return::Exception(format!(
                        "'{}': method '{}' not found for '{}'",
                        SIGNATURE,
                        signature,
                        object.to_string(universe)
                    ))
                    // Return::Local(Value::Nil)
                })
        }
    }
}

fn perform_in_super_class(universe: &mut UniverseAST, object: Value, sym: Interned, class: GCRef<Class>) -> Return {
    const SIGNATURE: &'static str = "Object>>#perform:inSuperclass:";

    let signature = universe.lookup_symbol(sym);
    let method = class.borrow().lookup_method(signature);

    match method {
        Some(invokable) => invokable.to_obj().invoke(universe, vec![object]),
        None => {
            let signature = signature.to_string();
            let args = vec![object.clone()];
            universe
                .does_not_understand(Value::Class(class), signature.as_str(), args)
                .unwrap_or_else(|| {
                    Return::Exception(format!(
                        "'{}': method '{}' not found for '{}'",
                        SIGNATURE,
                        signature,
                        object.to_string(universe)
                    ))
                    // Return::Local(Value::Nil)
                })
        }
    }
}

fn perform_with_arguments_in_super_class(universe: &mut UniverseAST, object: Value, sym: Interned, arr: GCRef<Vec<Value>>, class: GCRef<Class>) -> Return {
    const SIGNATURE: &'static str = "Object>>#perform:withArguments:inSuperclass:";

    let signature = universe.lookup_symbol(sym);
    let method = class.borrow().lookup_method(signature);

    match method {
        Some(invokable) => {
            // let args = std::iter::once(object)
            //     .chain(arr.to_obj().replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object).chain(arr.to_obj().clone()).collect();

            invokable.to_obj().invoke(universe, args)
        }
        None => {
            // let args = std::iter::once(object.clone())
            //     .chain(arr.to_obj().replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object.clone()).chain(arr.to_obj().clone()).collect();

            let signature = signature.to_string();
            universe
                .does_not_understand(Value::Class(class), signature.as_str(), args)
                .unwrap_or_else(|| {
                    Return::Exception(format!(
                        "'{}': method '{}' not found for '{}'",
                        SIGNATURE,
                        signature,
                        object.to_string(universe)
                    ))
                    // Return::Local(Value::Nil)
                })
        }
    }
}

fn inst_var_at(_: &mut UniverseAST, object: Value, index: i32) -> Return {
    const SIGNATURE: &'static str = "Object>>#instVarAt:";

    let index = match usize::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => return Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    };

    let local = match object {
        Value::Instance(c) => {
            c.borrow().locals.get(index).cloned().unwrap_or(Value::Nil)
        }
        Value::Class(c) => {
            c.clone().borrow().fields.get(index).cloned().unwrap_or(Value::Nil)
        }
        _ => unreachable!("instVarAt called not on an instance or a class")
    };

    Return::Local(local)
}

fn inst_var_at_put(_: &mut UniverseAST, object: Value, index: i32, value: Value) -> Return {
    const SIGNATURE: &'static str = "Object>>#instVarAt:put:";

    let index = match usize::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => return Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    };

    let does_have_local = match &object {
        Value::Instance(c) => { c.borrow().locals.len() > index }
        Value::Class(c) => { c.clone().borrow().fields.len() > index }
        _ => unreachable!("instVarAtPut called not on an instance or a class")
    };

    if does_have_local {
        match object {
            Value::Instance(instance) => instance.borrow_mut().assign_local(index, value.clone()),
            Value::Class(class) => class.borrow_mut().assign_field(index, value.clone()),
            v => unreachable!("Assigning a local binding in a {:?} value type?", v),
        }
    }

    Return::Local(value)
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
