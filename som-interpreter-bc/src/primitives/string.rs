use std::collections::hash_map::DefaultHasher;
use std::convert::{TryFrom, TryInto};
use std::hash::Hasher;

use crate::convert::{Primitive, StringLike};
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use anyhow::Error;
use num_bigint::BigInt;
use once_cell::sync::Lazy;
use som_core::gc::GCRef;
use som_core::interner::Interned;

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

fn length(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
) -> Result<Value, Error> {
    const _: &str = "String>>#length";

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let length = string.chars().count();
    let value = match length.try_into() {
        Ok(value) => Value::Integer(value),
        Err(_) => {
            Value::BigInteger(GCRef::<BigInt>::alloc(BigInt::from(length), &mut universe.gc_interface))
        }
    };

    Ok(value)
}

fn hashcode(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
) -> Result<i32, Error> {
    const _: &str = "String>>#hashcode";

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let mut hasher = DefaultHasher::new();
    hasher.write(string.as_bytes());
    let hash = (hasher.finish() as i32).abs();

    Ok(hash)
}

fn is_letters(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
) -> Result<bool, Error> {
    const _: &str = "String>>#isLetters";

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(!string.is_empty() && !string.is_empty() && string.chars().all(char::is_alphabetic))
}

fn is_digits(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
) -> Result<bool, Error> {
    const _: &str = "String>>#isDigits";

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(!string.is_empty() && string.chars().all(char::is_numeric))
}

fn is_whitespace(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
) -> Result<bool, Error> {
    const _: &str = "String>>#isWhiteSpace";

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(!string.is_empty() && string.chars().all(char::is_whitespace))
}

fn concatenate(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
    other: StringLike,
) -> Result<GCRef<String>, Error> {
    const _: &str = "String>>#concatenate:";

    let s1 = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    let s2 = match other {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(universe.gc_interface.allocate(format!("{s1}{s2}")))
}

fn as_symbol(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
) -> Result<Interned, Error> {
    const _: &str = "String>>#asSymbol";

    let symbol = match receiver {
        StringLike::String(ref value) => universe.intern_symbol(value.as_str()),
        StringLike::Symbol(symbol) => symbol,
    };

    Ok(symbol)
}

fn eq(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    a: Value,
    b: Value,
) -> Result<bool, Error> {
    const _: &str = "String>>#=";

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

fn prim_substring_from_to(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
    from: i32,
    to: i32,
) -> Result<GCRef<String>, Error> {
    const _: &str = "String>>#primSubstringFrom:to:";

    let from = usize::try_from(from - 1)?;
    let to = usize::try_from(to)?;

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    Ok(universe.gc_interface.allocate(string.chars().skip(from).take(to - from).collect()))
}

fn char_at(
    _: &mut Interpreter,
    universe: &mut UniverseBC,
    receiver: StringLike,
    idx: i32,
) -> Result<GCRef<String>, Error> {
    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    // TODO opt: just return a pointer to the char in question, right?
    Ok(GCRef::<String>::alloc(String::from(string.chars().nth((idx - 1) as usize).unwrap()), &mut universe.gc_interface))
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
