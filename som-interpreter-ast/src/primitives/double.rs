use crate::convert::{DoubleLike, Primitive, StringLike};
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::{bail, Error};
use num_traits::ToPrimitive;
use once_cell::sync::Lazy;

pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("+", self::plus.into_func(), true),
        ("-", self::minus.into_func(), true),
        ("*", self::times.into_func(), true),
        ("//", self::divide.into_func(), true),
        ("%", self::modulo.into_func(), true),
        ("=", self::eq.into_func(), true),
        ("<", self::lt.into_func(), true),
        ("sqrt", self::sqrt.into_func(), true),
        ("round", self::round.into_func(), true),
        ("cos", self::cos.into_func(), true),
        ("sin", self::sin.into_func(), true),
        ("asString", self::as_string.into_func(), true),
        ("asInteger", self::as_integer.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("fromString:", self::from_string.into_func(), true),
        (
            "PositiveInfinity",
            self::positive_infinity.into_func(),
            true,
        ),
    ])
});

macro_rules! promote {
    ($signature:expr, $value:expr) => {
        match $value {
            DoubleLike::Double(value) => value,
            DoubleLike::Integer(value) => value as f64,
            DoubleLike::BigInteger(value) => match value.to_f64() {
                Some(value) => value,
                None => {
                    panic!(
                        "'{}': `Integer` too big to be converted to `Double`",
                        $signature
                    )
                }
            },
        }
    };
}

fn from_string(universe: &mut Universe, _: Value, string: StringLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#fromString:";

    let string = match string {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    match string.parse() {
        Ok(parsed) => Ok(Value::Double(parsed)),
        Err(err) => bail!(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn as_string(universe: &mut Universe, receiver: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#asString";

    let value = promote!(SIGNATURE, receiver);

    Ok(Value::String(
        universe.gc_interface.alloc(value.to_string()),
    ))
}

fn as_integer(_: &mut Universe, receiver: f64) -> Result<Value, Error> {
    Ok(Value::Integer(receiver.trunc() as i32))
}

fn sqrt(_: &mut Universe, receiver: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#sqrt";

    let value = promote!(SIGNATURE, receiver);

    Ok(Value::Double(value.sqrt()))
}

fn round(_: &mut Universe, receiver: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#round";

    let value = promote!(SIGNATURE, receiver);

    Ok(Value::Double(value.round()))
}

fn cos(_: &mut Universe, value: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#cos";

    let value = promote!(SIGNATURE, value);

    Ok(Value::Double(value.cos()))
}

fn sin(_: &mut Universe, receiver: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#sin";

    let value = promote!(SIGNATURE, receiver);

    Ok(Value::Double(value.sin()))
}

fn eq(_: &mut Universe, a: Value, b: Value) -> Result<Value, Error> {
    Ok(Value::Boolean(a == b))
}

fn lt(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#<";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Ok(Value::Boolean(a < b))
}

fn plus(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#+";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Ok(Value::Double(a + b))
}

fn minus(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#-";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Ok(Value::Double(a - b))
}

fn times(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#*";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Ok(Value::Double(a * b))
}

fn divide(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#//";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Ok(Value::Double(a / b))
}

fn modulo(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Result<Value, Error> {
    const SIGNATURE: &str = "Double>>#%";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Ok(Value::Double(a % b))
}

fn positive_infinity(_: &mut Universe, _: Value) -> Result<Value, Error> {
    const _: &str = "Double>>#positiveInfinity";

    Ok(Value::Double(f64::INFINITY))
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
