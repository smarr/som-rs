use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use crate::expect_args;

pub static INSTANCE_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[
    ("+", self::plus, true),
    ("-", self::minus, true),
    ("*", self::times, true),
    ("//", self::divide, true),
    ("%", self::modulo, true),
    ("=", self::eq, true),
    ("<", self::lt, true),
    ("sqrt", self::sqrt, true),
    ("round", self::round, true),
    ("cos", self::cos, true),
    ("sin", self::sin, true),
    ("asString", self::as_string, true),
    ("asInteger", self::as_integer, true),
];
pub static CLASS_PRIMITIVES: &[(&str, PrimitiveFn, bool)] = &[
    ("fromString:", self::from_string, true),
    ("PositiveInfinity", self::positive_infinity, true),
];

macro_rules! promote {
    ($signature:expr, $value:expr) => {
        match $value {
            Value::Integer(value) => *value as f64,
            Value::BigInteger(value) => match (*value).to_f64() {
                Some(value) => value,
                None => {
                    panic!(
                        "'{}': `Integer` too big to be converted to `Double`",
                        $signature
                    )
                }
            },
            Value::Double(value) => *value,
            _ => panic!(
                "'{}': wrong type (expected `integer` or `double`)",
                $signature
            ),
        }
    };
}

fn from_string(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#fromString:";

    expect_args!(SIGNATURE, args, [
        _,
        Value::String(string)
    ]);

    match string.parse() {
        Ok(parsed) => interpreter.stack.push(Value::Double(parsed)),
        Err(err) => panic!("'{}': {}", SIGNATURE, err),
    }
}

fn as_string(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#asString";

    expect_args!(SIGNATURE, args, [
        value
    ]);

    let value = promote!(SIGNATURE, value);

    interpreter
        .stack
        .push(Value::String(Rc::new(value.to_string())));
}

fn as_integer(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#asInteger";

    expect_args!(SIGNATURE, args, [
        Value::Double(value)
    ]);

    interpreter.stack.push(Value::Integer(value.trunc() as i64));
}

fn sqrt(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#sqrt";

    expect_args!(SIGNATURE, args, [
        value
    ]);

    let value = promote!(SIGNATURE, value);

    interpreter.stack.push(Value::Double(value.sqrt()));
}

fn round(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#round";

    expect_args!(SIGNATURE, args, [
        value
    ]);

    let value = promote!(SIGNATURE, value);

    interpreter.stack.push(Value::Double(value.round()));
}

fn cos(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#cos";

    expect_args!(SIGNATURE, args, [
        value
    ]);

    let value = promote!(SIGNATURE, value);

    interpreter.stack.push(Value::Double(value.cos()));
}

fn sin(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#sin";

    expect_args!(SIGNATURE, args, [
        value
    ]);

    let value = promote!(SIGNATURE, value);

    interpreter.stack.push(Value::Double(value.sin()));
}

fn eq(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#=";

    expect_args!(SIGNATURE, args, [
        // Value::Double(a) => a,
        // Value::Double(b) => b,
        a,
        b
    ]);

    interpreter.stack.push(Value::Boolean(a == b));
}

fn lt(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#<";

    expect_args!(SIGNATURE, args, [
        a,
        b
    ]);

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    interpreter.stack.push(Value::Boolean(a < b));
}

fn plus(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#+";

    expect_args!(SIGNATURE, args, [
        a,
        b
    ]);

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    interpreter.stack.push(Value::Double(a + b));
}

fn minus(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#-";

    expect_args!(SIGNATURE, args, [
        a,
        b
    ]);

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    interpreter.stack.push(Value::Double(a - b));
}

fn times(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#*";

    expect_args!(SIGNATURE, args, [
        a,
        b
    ]);

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    interpreter.stack.push(Value::Double(a * b));
}

fn divide(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#//";

    expect_args!(SIGNATURE, args, [
        a,
        b
    ]);

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    interpreter.stack.push(Value::Double(a / b));
}

fn modulo(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#%";

    expect_args!(SIGNATURE, args, [
        a,
        b
    ]);

    let a = promote!(SIGNATURE, a);
    let b = promote!(SIGNATURE, b);

    interpreter.stack.push(Value::Double(a % b));
}

fn positive_infinity(interpreter: &mut Interpreter, args: Vec<Value>, _: &mut UniverseBC) {
    const SIGNATURE: &str = "Double>>#positiveInfinity";

    expect_args!(SIGNATURE, args, [_]);

    interpreter.stack.push(Value::Double(f64::INFINITY));
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
