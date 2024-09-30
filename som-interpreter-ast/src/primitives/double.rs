use crate::convert::{DoubleLike, Primitive, StringLike};
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseAST;
use crate::value::Value;
use num_traits::ToPrimitive;
use once_cell::sync::Lazy;
use som_core::gc::GCRef;

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
            DoubleLike::BigInteger(value) => match value.to_obj().to_f64() {
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

fn from_string(universe: &mut UniverseAST, _: Value, string: StringLike) -> Return {
    const SIGNATURE: &str = "Double>>#fromString:";

    let string = match string {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    match string.parse() {
        Ok(parsed) => Return::Local(Value::Double(parsed)),
        Err(err) => Return::Exception(format!("'{}': {}", SIGNATURE, err)),
    }
}

fn as_string(universe: &mut UniverseAST, receiver: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#asString";

    let value = promote!(SIGNATURE, receiver);

    Return::Local(Value::String(GCRef::<String>::alloc(value.to_string(), &mut universe.gc_interface)))
}

fn as_integer(_: &mut UniverseAST, receiver: f64) -> Return {
    Return::Local(Value::Integer(receiver.trunc() as i32))
}

fn sqrt(_: &mut UniverseAST, receiver: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#sqrt";

    let value = promote!(SIGNATURE, receiver);

    Return::Local(Value::Double(value.sqrt()))
}

fn round(_: &mut UniverseAST, receiver: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#round";

    let value = promote!(SIGNATURE, receiver);

    Return::Local(Value::Double(value.round()))
}

fn cos(_: &mut UniverseAST, value: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#cos";

    let value = promote!(SIGNATURE, value);

    Return::Local(Value::Double(value.cos()))
}

fn sin(_: &mut UniverseAST, receiver: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#sin";

    let value = promote!(SIGNATURE, receiver);

    Return::Local(Value::Double(value.sin()))
}

fn eq(_: &mut UniverseAST, a: Value, b: Value) -> Return {
    Return::Local(Value::Boolean(a == b))
}

fn lt(_: &mut UniverseAST, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#<";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Return::Local(Value::Boolean(a < b))
}

fn plus(_: &mut UniverseAST, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#+";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Return::Local(Value::Double(a + b))
}

fn minus(_: &mut UniverseAST, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#-";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Return::Local(Value::Double(a - b))
}

fn times(_: &mut UniverseAST, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#*";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Return::Local(Value::Double(a * b))
}

fn divide(_: &mut UniverseAST, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#//";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Return::Local(Value::Double(a / b))
}

fn modulo(_: &mut UniverseAST, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Double>>#%";

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    Return::Local(Value::Double(a % b))
}

fn positive_infinity(_: &mut UniverseAST, _: Vec<Value>) -> Return {
    const _: &str = "Double>>#positiveInfinity";

    Return::Local(Value::Double(f64::INFINITY))
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