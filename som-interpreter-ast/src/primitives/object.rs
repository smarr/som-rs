use super::PrimInfo;
use crate::convert::Primitive;
use crate::gc::VecValue;
use crate::invokable::{Invoke, Return};
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use crate::vm_objects::class::Class;
use anyhow::{bail, Error};
use once_cell::sync::Lazy;
use som_core::interner::Interned;
use som_gc::gcref::Gc;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("halt", self::halt.into_func(), true),
        ("class", self::class.into_func(), true),
        ("objectSize", self::object_size.into_func(), true),
        ("hashcode", self::hashcode.into_func(), true),
        ("perform:", self::perform.into_func(), true),
        ("perform:withArguments:", self::perform_with_arguments.into_func(), true),
        ("perform:inSuperclass:", self::perform_in_super_class.into_func(), true),
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
pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn halt(_: &mut Universe, _: Value) -> Result<Value, Error> {
    println!("HALT"); // so a breakpoint can be put
    Ok(Value::NIL)
}

fn class(universe: &mut Universe, object: Value) -> Result<Value, Error> {
    Ok(Value::Class(object.class(universe)))
}

fn object_size(_: &mut Universe, _: Value) -> Result<Value, Error> {
    Ok(Value::Integer(core::mem::size_of::<Value>() as i32))
}

fn hashcode(_: &mut Universe, receiver: Value) -> Result<Value, Error> {
    let mut hasher = DefaultHasher::new();
    receiver.hash(&mut hasher);
    let hash = (hasher.finish() as i32).abs();

    Ok(Value::Integer(hash))
}

fn eq(_: &mut Universe, receiver: Value, other: Value) -> Result<Value, Error> {
    Ok(Value::Boolean(receiver == other))
}

fn perform(universe: &mut Universe, object: Value, sym: Interned) -> Result<Return, Error> {
    const SIGNATURE: &str = "Object>>#perform:";

    let method = object.lookup_method(universe, sym);

    match method {
        Some(mut invokable) => {
            universe.stack_args.push(object);
            Ok(invokable.invoke(universe, 1))
        }
        None => {
            Ok(universe.does_not_understand(object, sym, vec![object]).unwrap_or_else(|| {
                panic!(
                    "'{}': method '{}' not found for '{}'",
                    SIGNATURE,
                    universe.lookup_symbol(sym),
                    object.to_string(universe)
                )
                // Ok(Value::Nil)
            }))
        }
    }
}

fn perform_with_arguments(universe: &mut Universe, object: Value, sym: Interned, arr: Gc<VecValue>) -> Result<Return, Error> {
    const SIGNATURE: &str = "Object>>#perform:withArguments:";

    let method = object.lookup_method(universe, sym);

    match method {
        Some(mut invokable) => {
            // let args = std::iter::once(object)
            //     .chain(arr.replace(Vec::default()))
            //     .collect();
            universe.stack_args.push(object);
            for val in arr.0.iter() {
                universe.stack_args.push(*val)
            }
            Ok(invokable.invoke(universe, arr.0.len() + 1))
        }
        None => {
            // let args = std::iter::once(object.clone())
            //     .chain(arr.replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object).chain((*arr).clone()).collect();

            Ok(universe.does_not_understand(object, sym, args).unwrap_or_else(|| {
                panic!(
                    "'{}': method '{}' not found for '{}'",
                    SIGNATURE,
                    universe.lookup_symbol(sym),
                    object.to_string(universe)
                )
                // Ok(Value::Nil)
            }))
        }
    }
}

fn perform_in_super_class(universe: &mut Universe, object: Value, sym: Interned, class: Gc<Class>) -> Result<Return, Error> {
    const SIGNATURE: &str = "Object>>#perform:inSuperclass:";

    let method = class.lookup_method(sym);

    match method {
        Some(mut invokable) => {
            universe.stack_args.push(object);
            Ok(invokable.invoke(universe, 1))
        }
        None => {
            let args = vec![object];
            Ok(universe.does_not_understand(Value::Class(class), sym, args).unwrap_or_else(|| {
                panic!(
                    "'{}': method '{}' not found for '{}'",
                    SIGNATURE,
                    universe.lookup_symbol(sym),
                    object.to_string(universe)
                )
                // Ok(Value::Nil)
            }))
        }
    }
}

fn perform_with_arguments_in_super_class(
    universe: &mut Universe,
    object: Value,
    sym: Interned,
    arr: Gc<VecValue>,
    class: Gc<Class>,
) -> Result<Return, Error> {
    const SIGNATURE: &str = "Object>>#perform:withArguments:inSuperclass:";

    let method = class.lookup_method(sym);

    match method {
        // Some(mut invokable) => {
        Some(_invokable) => {
            todo!();
            // let args = std::iter::once(object)
            //     .chain(arr.replace(Vec::default()))
            //     .collect();
            // let args = std::iter::once(object).chain((*arr).clone()).collect();

            // Ok(invokable.invoke(universe, args))
        }
        None => {
            // let args = std::iter::once(object.clone())
            //     .chain(arr.replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object).chain((*arr).clone()).collect();

            Ok(universe.does_not_understand(Value::Class(class), sym, args).unwrap_or_else(|| {
                panic!(
                    "'{}': method '{}' not found for '{}'",
                    SIGNATURE,
                    universe.lookup_symbol(sym),
                    object.to_string(universe)
                )
                // Ok(Value::Nil)
            }))
        }
    }
}

fn inst_var_at(_: &mut Universe, object: Value, index: i32) -> Result<Value, Error> {
    const SIGNATURE: &str = "Object>>#instVarAt:";

    let index = match usize::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    };

    let local = {
        if let Some(instance) = object.as_instance() {
            instance.fields.get(index).cloned().unwrap_or(Value::NIL)
        } else if let Some(cls) = object.as_class() {
            cls.fields.get(index).cloned().unwrap_or(Value::NIL)
        } else {
            unreachable!("instVarAt called not on an instance or a class")
        }
    };

    Ok(local)
}

fn inst_var_at_put(_: &mut Universe, object: Value, index: i32, value: Value) -> Result<Value, Error> {
    const SIGNATURE: &str = "Object>>#instVarAt:put:";

    let index = match u8::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    };

    if let Some(mut instance) = object.as_instance() {
        if instance.fields.len() as u8 > index {
            instance.assign_field(index, value)
        }
    } else if let Some(mut cls) = object.as_class() {
        if cls.fields.len() as u8 > index {
            cls.assign_field(index, value)
        }
    } else {
        unreachable!("instVarAtPut called not on an instance or a class")
    }

    Ok(value)
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
