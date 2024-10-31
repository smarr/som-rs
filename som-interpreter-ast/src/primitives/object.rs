use crate::class::Class;
use crate::convert::Primitive;
use crate::gc::VecValue;
use crate::invokable::{Invoke, Return};
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::{bail, Error};
use once_cell::sync::Lazy;
use som_core::interner::Interned;
use som_gc::gcref::GCRef;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};

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


fn halt(_: &mut Universe, _: Value)-> Result<Value, Error> {
    println!("HALT"); // so a breakpoint can be put
    Ok(Value::NIL)
}

fn class(universe: &mut Universe, object: Value)-> Result<Value, Error> {
    Ok(Value::Class(object.class(universe)))
}

fn object_size(_: &mut Universe, _: Value)-> Result<Value, Error> {
    const _: &'static str = "Object>>#objectSize";

    Ok(Value::Integer(std::mem::size_of::<Value>() as i32))
}

fn hashcode(_: &mut Universe, receiver: Value)-> Result<Value, Error> {
    let mut hasher = DefaultHasher::new();
    receiver.hash(&mut hasher);
    let hash = (hasher.finish() as i32).abs();

    Ok(Value::Integer(hash))
}

fn eq(_: &mut Universe, receiver: Value, other: Value)-> Result<Value, Error> {
    Ok(Value::Boolean(receiver == other))
}

fn perform(universe: &mut Universe, object: Value, sym: Interned)-> Result<Return, Error> {
    const SIGNATURE: &'static str = "Object>>#perform:";

    let signature = universe.lookup_symbol(sym);
    let method = object.lookup_method(universe, signature);

    match method {
        Some(invokable) => Ok(invokable.to_obj().invoke(universe, vec![object])),
        None => {
            let signature = signature.to_string();
            Ok(universe
                .does_not_understand(object.clone(), signature.as_str(), vec![object.clone()])
                .unwrap_or_else(|| {
                    panic!("'{}': method '{}' not found for '{}'",
                           SIGNATURE,
                           signature,
                           object.to_string(universe)
                    )
                    // Ok(Value::Nil)
                }))
        }
    }
}

fn perform_with_arguments(universe: &mut Universe, object: Value, sym: Interned, arr: GCRef<VecValue>)-> Result<Return, Error> {
    const SIGNATURE: &'static str = "Object>>#perform:withArguments:";

    let signature = universe.lookup_symbol(sym);
    let method = object.lookup_method(universe, signature);

    match method {
        Some(invokable) => {
            // let args = std::iter::once(object)
            //     .chain(arr.replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object).chain(arr.to_obj().clone()).collect();
            Ok(invokable.to_obj().invoke(universe, args))
        }
        None => {
            let signature = signature.to_string();
            // let args = std::iter::once(object.clone())
            //     .chain(arr.to_obj().replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object.clone()).chain(arr.to_obj().clone()).collect();

            Ok(universe
                .does_not_understand(object.clone(), signature.as_str(), args)
                .unwrap_or_else(|| {
                    panic!("'{}': method '{}' not found for '{}'",
                        SIGNATURE,
                        signature,
                        object.to_string(universe)
                    )
                    // Ok(Value::Nil)
                }))
        }
    }
}

fn perform_in_super_class(universe: &mut Universe, object: Value, sym: Interned, class: GCRef<Class>)-> Result<Return, Error> {
    const SIGNATURE: &'static str = "Object>>#perform:inSuperclass:";

    let signature = universe.lookup_symbol(sym);
    let method = class.borrow().lookup_method(signature);

    match method {
        Some(invokable) => Ok(invokable.to_obj().invoke(universe, vec![object])),
        None => {
            let signature = signature.to_string();
            let args = vec![object.clone()];
            Ok(universe
                .does_not_understand(Value::Class(class), signature.as_str(), args)
                .unwrap_or_else(|| {
                    panic!("'{}': method '{}' not found for '{}'",
                           SIGNATURE,
                           signature,
                           object.to_string(universe)
                    )
                    // Ok(Value::Nil)
                }))
        }
    }
}

fn perform_with_arguments_in_super_class(universe: &mut Universe, object: Value, sym: Interned, arr: GCRef<VecValue>, class: GCRef<Class>)-> Result<Return, Error> {
    const SIGNATURE: &'static str = "Object>>#perform:withArguments:inSuperclass:";

    let signature = universe.lookup_symbol(sym);
    let method = class.borrow().lookup_method(signature);

    match method {
        Some(invokable) => {
            // let args = std::iter::once(object)
            //     .chain(arr.to_obj().replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object).chain(arr.to_obj().clone()).collect();

            Ok(invokable.to_obj().invoke(universe, args))
        }
        None => {
            // let args = std::iter::once(object.clone())
            //     .chain(arr.to_obj().replace(Vec::default()))
            //     .collect();
            let args = std::iter::once(object.clone()).chain(arr.to_obj().clone()).collect();

            let signature = signature.to_string();
            Ok(universe
                .does_not_understand(Value::Class(class), signature.as_str(), args)
                .unwrap_or_else(|| {
                    panic!("'{}': method '{}' not found for '{}'",
                           SIGNATURE,
                           signature,
                           object.to_string(universe)
                    )
                    // Ok(Value::Nil)
                }))
        }
    }
}

fn inst_var_at(_: &mut Universe, object: Value, index: i32)-> Result<Value, Error> {
    const SIGNATURE: &'static str = "Object>>#instVarAt:";

    let index = match usize::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    };

    let local = {
        if let Some(instance) = object.as_instance() {
            instance.borrow().locals.get(index).cloned().unwrap_or(Value::NIL)
        } else if let Some(cls) = object.as_class() {
            cls.clone().borrow().fields.get(index).cloned().unwrap_or(Value::NIL)
        } else {
            unreachable!("instVarAt called not on an instance or a class")
        }
    };

    Ok(local)
}

fn inst_var_at_put(_: &mut Universe, object: Value, index: i32, value: Value)-> Result<Value, Error> {
    const SIGNATURE: &'static str = "Object>>#instVarAt:put:";

    let index = match u8::try_from(index - 1) {
        Ok(index) => index,
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    };
    
    if let Some(instance) = object.as_instance() {
        if instance.borrow().locals.len() as u8 > index {
            instance.borrow_mut().assign_local(index, value.clone())
        }
    } else if let Some(cls) = object.as_class() {
        if cls.borrow().fields.len() as u8 > index {
            cls.borrow_mut().assign_field(index, value.clone())
        }
    } else {
        unreachable!("instVarAtPut called not on an instance or a class")
    }

    Ok(value)
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
