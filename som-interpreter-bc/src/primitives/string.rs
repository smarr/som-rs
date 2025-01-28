use std::collections::hash_map::DefaultHasher;
use std::convert::{TryFrom, TryInto};
use std::hash::Hasher;

use crate::interpreter::Interpreter;
use crate::primitives::PrimInfo;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::convert::{Primitive, StringLike};
use crate::value::Value;
use anyhow::Error;
use num_bigint::BigInt;
use once_cell::sync::Lazy;
use som_core::interner::Interned;
use som_gc::gcref::Gc;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| {
    Box::new([
        ("length", self::length.into_func(), true),
        ("hashcode", self::hashcode.into_func(), true),
        ("isLetters", self::is_letters.into_func(), true),
        ("isDigits", self::is_digits.into_func(), true),
        ("isWhiteSpace", self::is_whitespace.into_func(), true),
        ("asSymbol", self::as_symbol.into_func(), true),
        ("concatenate:", self::concatenate.into_func(), true),
        ("primSubstringFrom:to:", self::prim_substring_from_to.into_func(), true),
        ("=", self::eq.into_func(), true),
        ("charAt:", self::char_at.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[PrimInfo]>> = Lazy::new(|| Box::new([]));

fn length(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike) -> Result<Value, Error> {
    const _: &str = "String>>#length";

    let string = match receiver {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
        StringLike::Char(_) => {
            return Ok(Value::Integer(1));
        }
    };

    let length = string.chars().count();
    let value = match length.try_into() {
        Ok(value) => Value::Integer(value),
        Err(_) => Value::BigInteger(universe.gc_interface.alloc(BigInt::from(length))),
    };

    Ok(value)
}

fn hashcode(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike) -> Result<i32, Error> {
    let string = receiver.as_str(|sym| universe.lookup_symbol(sym));
    let mut hasher = DefaultHasher::new();
    hasher.write(string.as_bytes());
    let hash = (hasher.finish() as i32).abs();

    Ok(hash)
}

fn is_letters(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike) -> Result<bool, Error> {
    let string = receiver.as_str(|sym| universe.lookup_symbol(sym));
    Ok(!string.is_empty() && string.chars().all(char::is_alphabetic))
}

fn is_digits(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike) -> Result<bool, Error> {
    const _: &str = "String>>#isDigits";

    let string = receiver.as_str(|sym| universe.lookup_symbol(sym));

    Ok(!string.is_empty() && string.chars().all(char::is_numeric))
}

fn is_whitespace(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike) -> Result<bool, Error> {
    const _: &str = "String>>#isWhiteSpace";

    let string = receiver.as_str(|sym| universe.lookup_symbol(sym));

    Ok(!string.is_empty() && string.chars().all(char::is_whitespace))
}

fn concatenate(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike, other: StringLike) -> Result<Gc<String>, Error> {
    let s1 = receiver.as_str(|sym| universe.lookup_symbol(sym));
    let s2 = other.as_str(|sym| universe.lookup_symbol(sym));
    Ok(universe.gc_interface.alloc(format!("{s1}{s2}")))
}

fn as_symbol(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike) -> Result<Interned, Error> {
    const _: &str = "String>>#asSymbol";

    let symbol = match receiver {
        StringLike::String(ref value) => universe.intern_symbol(value.as_str()),
        StringLike::Char(char) => universe.intern_symbol(&String::from(char)),
        StringLike::Symbol(symbol) => symbol,
    };

    Ok(symbol)
}

fn eq(_: &mut Interpreter, universe: &mut Universe, a: Value, b: Value) -> Result<bool, Error> {
    const _: &str = "String>>#=";

    let Ok(a) = StringLike::try_from(a.0) else {
        return Ok(false);
    };

    let Ok(b) = StringLike::try_from(b.0) else {
        return Ok(false);
    };

    match (&a, &b) {
        (StringLike::Char(c1), StringLike::Char(c2)) => Ok(*c1 == *c2),
        (StringLike::Symbol(sym1), StringLike::Symbol(sym2)) => Ok(*sym1 == *sym2),
        (StringLike::String(str1), StringLike::String(str2)) => Ok(str1.as_str().eq(str2.as_str())),
        _ => {
            let a = a.as_str(|sym| universe.lookup_symbol(sym));
            let b = b.as_str(|sym| universe.lookup_symbol(sym));
            Ok(*a == *b)
        }
    }
}

fn prim_substring_from_to(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike, from: i32, to: i32) -> Result<Gc<String>, Error> {
    const _: &str = "String>>#primSubstringFrom:to:";

    let from = usize::try_from(from - 1)?;
    let to = usize::try_from(to)?;

    let string = receiver.as_str(|sym| universe.lookup_symbol(sym));

    Ok(universe.gc_interface.alloc(string.chars().skip(from).take(to - from).collect()))
}

fn char_at(_: &mut Interpreter, universe: &mut Universe, receiver: StringLike, idx: i32) -> Result<Value, Error> {
    let string = receiver.as_str(|sym| universe.lookup_symbol(sym));
    let char = string.chars().nth((idx - 1) as usize).unwrap();
    Ok(Value::Char(char))
}

/// Search for an instance primitive matching the given signature.
pub fn get_instance_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    INSTANCE_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}

/// Search for a class primitive matching the given signature.
pub fn get_class_primitive(signature: &str) -> Option<&'static PrimitiveFn> {
    CLASS_PRIMITIVES.iter().find(|it| it.0 == signature).map(|it| it.1)
}
