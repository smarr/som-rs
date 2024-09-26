use crate::convert::{Primitive, StringLike};
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use once_cell::sync::Lazy;
use som_core::gc::GCRef;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::Hasher;
use anyhow::{bail, Error};

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("length", self::length.into_func(), true),
        ("hashcode", self::hashcode.into_func(), true),
        ("isLetters", self::is_letters.into_func(), true),
        ("isDigits", self::is_digits.into_func(), true),
        ("isWhiteSpace", self::is_whitespace.into_func(), true),
        ("asSymbol", self::as_symbol.into_func(), true),
        ("concatenate:", self::concatenate.into_func(), true),
        (
            "primSubstringFrom:to:",
            self::prim_substring_from_to.into_func(),
            true,
        ),
        ("=", self::eq.into_func(), true),
        ("charAt:", self::char_at.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([]));

fn length(universe: &mut Universe, value: StringLike)-> Result<Value, Error> {
    const SIGNATURE: &str = "String>>#length";

    let value = match value {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym)
    };

    match i32::try_from(value.chars().count()) {
        Ok(idx) => Ok(Value::Integer(idx)),
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn hashcode(universe: &mut Universe, value: StringLike)-> Result<Value, Error> {
    let value = match value {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let mut hasher = DefaultHasher::new();

    hasher.write(value.as_bytes());

    // match i32::try_from(hasher.finish()) {
    //     Ok(hash) => Ok(Value::Integer(hash)),
    //     Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    // }

    Ok(Value::Integer((hasher.finish() as i32).abs()))
}

fn is_letters(universe: &mut Universe, value: StringLike)-> Result<Value, Error> {
    let value = match value {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(Value::Boolean(
        !value.is_empty() && !value.is_empty() && value.chars().all(char::is_alphabetic),
    ))
}

fn is_digits(universe: &mut Universe, value: StringLike)-> Result<Value, Error> {
    let value = match value {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(Value::Boolean(
        !value.is_empty() && value.chars().all(char::is_numeric),
    ))
}

fn is_whitespace(universe: &mut Universe, value: StringLike)-> Result<Value, Error> {
    let value = match value {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(Value::Boolean(
        !value.is_empty() && value.chars().all(char::is_whitespace),
    ))
}

fn concatenate(universe: &mut Universe, receiver: StringLike, other: StringLike)-> Result<Value, Error> {
    let s1 = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let s2 = match other {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(Value::String(GCRef::<String>::alloc(format!("{}{}", s1, s2), &mut universe.gc_interface)))
}

fn as_symbol(universe: &mut Universe, value: StringLike)-> Result<Value, Error> {
    match value {
        StringLike::String(ref value) => {
            Ok(Value::Symbol(universe.intern_symbol(value.as_str())))
        }
        StringLike::Symbol(sym) => Ok(Value::Symbol(sym)),
    }
}

fn char_at(universe: &mut Universe, receiver: StringLike, idx: i32)-> Result<Value, Error> {
    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(Value::String(GCRef::<String>::alloc(String::from(string.chars().nth((idx - 1) as usize).unwrap()), &mut universe.gc_interface)))
}

fn eq(universe: &mut Universe, a: Value, b: Value)-> Result<bool, Error> {
    let Ok(a) = StringLike::try_from(a) else {
        return Ok(false);
    };

    let Ok(b) = StringLike::try_from(b) else {
        return Ok(false);
    };

    let a = match a {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let b = match b {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(a == b)
}

fn prim_substring_from_to(universe: &mut Universe, receiver: StringLike, from: i32, to: i32)-> Result<Value, Error> {
    let from = usize::try_from(from - 1).unwrap();
    let to = usize::try_from(to).unwrap();

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let s = universe.gc_interface.allocate(string.chars().skip(from).take(to - from).collect());

    Ok(Value::String(s))
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