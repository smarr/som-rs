use std::convert::{TryFrom, TryInto};

use crate::convert::{DoubleLike, IntegerLike, Primitive, StringLike};
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use anyhow::{bail, Context, Error};
use num_bigint::{BigInt, BigUint, ToBigInt};
use num_traits::{Signed, ToPrimitive};
use once_cell::sync::Lazy;
use rand::distributions::Uniform;
use rand::Rng;
use crate::block::Block;
use som_gc::gcref::GCRef;


pub static INSTANCE_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> = Lazy::new(|| {
    Box::new([
        ("<", self::lt.into_func(), true),
        ("=", self::eq.into_func(), true),
        ("+", self::plus.into_func(), true),
        ("-", self::minus.into_func(), true),
        ("*", self::times.into_func(), true),
        ("/", self::divide.into_func(), true),
        ("//", self::divide_float.into_func(), true),
        ("%", self::modulo.into_func(), true),
        ("rem:", self::remainder.into_func(), true),
        ("&", self::bitand.into_func(), true),
        ("<<", self::shift_left.into_func(), true),
        (">>>", self::shift_right.into_func(), true),
        ("bitXor:", self::bitxor.into_func(), true),
        ("sqrt", self::sqrt.into_func(), true),
        ("asString", self::as_string.into_func(), true),
        ("asDouble", self::as_double.into_func(), true),
        ("atRandom", self::at_random.into_func(), true),
        (
            "as32BitSignedValue",
            self::as_32bit_signed_value.into_func(),
            true,
        ),
        (
            "as32BitUnsignedValue",
            self::as_32bit_unsigned_value.into_func(),
            true,
        ),
        ("to:do:", self::to_do.into_func(), true),
        ("to:by:do:", self::to_by_do.into_func(), true),
        ("downTo:do:", self::down_to_do.into_func(), true),
        ("downTo:by:do:", self::down_to_by_do.into_func(), true),
        ("timesRepeat:", self::times_repeat.into_func(), true),
    ])
});
pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([("fromString:", self::from_string.into_func(), true)]));

macro_rules! demote {
    ($heap:expr, $expr:expr) => {{
        let value = $expr;
        match value.to_i32() {
            Some(value) => Value::Integer((value)),
            None => Value::BigInteger($heap.allocate(value)),
        }
    }};
}

fn from_string(
    _: &mut Interpreter,
    universe: &mut Universe,
    _: Value,
    string: StringLike,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#fromString:";

    let string = match string {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym),
    };

    // bad implem, can be improved
    match string.parse::<i32>() {
        Ok(a) => Ok(Value::Integer(a)),
        Err(_) => {
            match string.parse::<BigInt>() {
                Ok(b) => Ok(Value::BigInteger(GCRef::<BigInt>::alloc(b, &mut universe.gc_interface))),
                _ => panic!("couldn't turn an int/bigint into a string")
            }
        }
    }
}

fn as_string(
    _: &mut Interpreter,
    universe: &mut Universe,
    receiver: IntegerLike,
) -> Result<GCRef<String>, Error> {
    const _: &str = "Integer>>#asString";

    let receiver = match receiver {
        IntegerLike::Integer(value) => value.to_string(),
        IntegerLike::BigInteger(value) => value.to_obj().to_string(),
    };

    Ok(universe.gc_interface.allocate(receiver))
}

fn as_double(
    _: &mut Interpreter,
    _: &mut Universe,
    receiver: IntegerLike,
) -> Result<f64, Error> {
    const _: &str = "Integer>>#asDouble";

    let value = match receiver {
        IntegerLike::Integer(value) => value as f64,
        IntegerLike::BigInteger(value) => value.to_obj()
            .to_f64()
            .context("could not convert big integer to f64")?,
    };

    Ok(value)
}

fn at_random(
    _: &mut Interpreter,
    _: &mut Universe,
    receiver: IntegerLike,
) -> Result<i32, Error> {
    const SIGNATURE: &str = "Integer>>#atRandom";

    let chosen = match receiver {
        IntegerLike::Integer(value) => {
            let distribution = Uniform::new(0, value);
            let mut rng = rand::thread_rng();
            rng.sample(distribution)
        }
        IntegerLike::BigInteger(_) => {
            bail!("'{SIGNATURE}': the range is too big to pick a random value from");
        }
    };

    Ok(chosen)
}

fn as_32bit_signed_value(
    _: &mut Interpreter,
    _: &mut Universe,
    receiver: IntegerLike,
) -> Result<i32, Error> {
    const _: &str = "Integer>>#as32BitSignedValue";

    let value = match receiver {
        IntegerLike::Integer(value) => value,
        IntegerLike::BigInteger(value) => {
            // We do this gymnastic to get the 4 lowest bytes from the two's-complement representation.
            let mut values = value.to_obj().to_signed_bytes_le();
            values.resize(4, 0);
            i32::from_le_bytes(values.try_into().unwrap())
        }
    };

    Ok(value)
}

fn as_32bit_unsigned_value(
    _: &mut Interpreter,
    universe: &mut Universe,
    receiver: IntegerLike,
) -> Result<IntegerLike, Error> {
    let value = match receiver {
        IntegerLike::Integer(value) => value as u32,
        IntegerLike::BigInteger(value) => {
            // We do this gymnastic to get the 4 lowest bytes from the two's-complement representation.
            let mut values = value.as_ref().to_signed_bytes_le();
            values.resize(4, 0);
            u32::from_le_bytes(values.try_into().unwrap())
        }
    };

    let value = match value.try_into() {
        Ok(value) => IntegerLike::Integer(value),
        Err(_) => {
            IntegerLike::BigInteger(GCRef::<BigInt>::alloc(BigInt::from(value), &mut universe.gc_interface))
        }
    };

    Ok(value)
}

fn plus(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: DoubleLike,
    b: DoubleLike,
) -> Result<Value, Error> {
    const SIGNATURE: &str = "Integer>>#+";

    let heap = &mut universe.gc_interface;

    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_add(b) {
            Some(value) => Value::Integer(value),
            None => demote!(heap, BigInt::from(a) + BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, a.as_ref() + b.as_ref())
        }
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b))
        | (DoubleLike::Integer(b), DoubleLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() + BigInt::from(b))
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Value::Double(a + b),
        (DoubleLike::Integer(a), DoubleLike::Double(b))
        | (DoubleLike::Double(b), DoubleLike::Integer(a)) => Value::Double((a as f64) + b),
        (DoubleLike::BigInteger(a), DoubleLike::Double(b))
        | (DoubleLike::Double(b), DoubleLike::BigInteger(a)) => match a.to_obj().to_f64() {
            Some(a) => Value::Double(a + b),
            None => panic!(
                "'{}': `Integer` too big to be converted to `Double`",
                SIGNATURE
            ),
        },
    };

    Ok(value)
}

fn minus(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: DoubleLike,
    b: DoubleLike,
) -> Result<Value, Error> {
    const SIGNATURE: &str = "Integer>>#-";

    let heap = &mut universe.gc_interface;
    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_sub(b) {
            Some(value) => Value::Integer(value),
            None => demote!(heap, BigInt::from(a) - BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, a.as_ref() - b.as_ref())
        }
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) => {
            demote!(heap, a.as_ref() - BigInt::from(b))
        }
        (DoubleLike::Integer(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, BigInt::from(a) - b.as_ref())
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Value::Double(a - b),
        (DoubleLike::Integer(a), DoubleLike::Double(b))
        | (DoubleLike::Double(b), DoubleLike::Integer(a)) => Value::Double((a as f64) - b),
        (DoubleLike::BigInteger(a), DoubleLike::Double(b)) => match a.to_obj().to_f64() {
            Some(a) => Value::Double(a - b),
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
        (DoubleLike::Double(a), DoubleLike::BigInteger(b)) => match b.to_obj().to_f64() {
            Some(b) => Value::Double(a - b),
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
    };

    Ok(value)
}

fn times(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: DoubleLike,
    b: DoubleLike,
) -> Result<Value, Error> {
    const SIGNATURE: &str = "Integer>>#*";

    let heap = &mut universe.gc_interface;

    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_mul(b) {
            Some(value) => Value::Integer(value),
            None => demote!(heap, BigInt::from(a) * BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, a.as_ref() * b.as_ref())
        }
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b))
        | (DoubleLike::Integer(b), DoubleLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() * BigInt::from(b))
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Value::Double(a * b),
        (DoubleLike::Integer(a), DoubleLike::Double(b))
        | (DoubleLike::Double(b), DoubleLike::Integer(a)) => Value::Double((a as f64) * b),
        (DoubleLike::BigInteger(a), DoubleLike::Double(b))
        | (DoubleLike::Double(b), DoubleLike::BigInteger(a)) => match a.to_obj().to_f64() {
            Some(a) => Value::Double(a * b),
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
    };

    Ok(value)
}

fn divide(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: DoubleLike,
    b: DoubleLike,
) -> Result<Value, Error> {
    const SIGNATURE: &str = "Integer>>#/";

    let heap = &mut universe.gc_interface;

    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_div(b) {
            Some(value) => Value::Integer(value),
            None => demote!(heap, BigInt::from(a) / BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, a.as_ref() / b.as_ref())
        }
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) => {
            demote!(heap, a.as_ref() / BigInt::from(b))
        }
        (DoubleLike::Integer(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, BigInt::from(a) / b.as_ref())
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Value::Double(a / b),
        (DoubleLike::Integer(a), DoubleLike::Double(b))
        | (DoubleLike::Double(b), DoubleLike::Integer(a)) => Value::Double((a as f64) / b),
        (DoubleLike::BigInteger(a), DoubleLike::Double(b)) => match a.to_obj().to_f64() {
            Some(a) => Value::Double(a / b),
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
        (DoubleLike::Double(a), DoubleLike::BigInteger(b)) => match b.to_obj().to_f64() {
            Some(b) => Value::Double(a / b),
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
    };

    Ok(value)
}

fn divide_float(
    _: &mut Interpreter,
    _: &mut Universe,
    a: DoubleLike,
    b: DoubleLike,
) -> Result<f64, Error> {
    const SIGNATURE: &str = "Integer>>#//";

    let a = match a {
        DoubleLike::Double(a) => a,
        DoubleLike::Integer(a) => a as f64,
        DoubleLike::BigInteger(a) => match a.to_obj().to_f64() {
            Some(a) => a,
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
    };

    let b = match b {
        DoubleLike::Double(b) => b,
        DoubleLike::Integer(b) => b as f64,
        DoubleLike::BigInteger(b) => match b.to_obj().to_f64() {
            Some(b) => b,
            None => {
                bail!("'{SIGNATURE}': `Integer` too big to be converted to `Double`");
            }
        },
    };

    Ok(a / b)
}

fn modulo(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: IntegerLike,
    b: i32,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#%";

    let result = match a {
        IntegerLike::Integer(a) => {
            let result = a % b;
            if result.signum() != b.signum() {
                Value::Integer((result + b) % b)
            } else {
                Value::Integer(result)
            }
        }
        IntegerLike::BigInteger(a) => {
            let result = a.as_ref() % b;
            if result.is_positive() != b.is_positive() {
                demote!(&mut universe.gc_interface, (result + b) % b)
            } else {
                demote!(&mut universe.gc_interface, result)
            }
        }
    };

    Ok(result)
}

fn remainder(
    _: &mut Interpreter,
    _: &mut Universe,
    a: i32,
    b: i32,
) -> Result<i32, Error> {
    const _: &str = "Integer>>#rem:";

    let result = a % b;
    if result.signum() != a.signum() {
        Ok((result + a) % a)
    } else {
        Ok(result)
    }
}

fn sqrt(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: DoubleLike,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#sqrt";

    let value = match a {
        DoubleLike::Double(a) => Value::Double(a.sqrt()),
        DoubleLike::Integer(a) => {
            let sqrt = (a as f64).sqrt();
            let trucated = sqrt.trunc();
            if sqrt == trucated {
                Value::Integer(trucated as i32)
            } else {
                Value::Double(sqrt)
            }
        }
        DoubleLike::BigInteger(a) => demote!(&mut universe.gc_interface, a.to_obj().sqrt()),
    };

    Ok(value)
}

fn bitand(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: IntegerLike,
    b: IntegerLike,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#&";

    let value = match (a, b) {
        (IntegerLike::Integer(a), IntegerLike::Integer(b)) => Value::Integer(a & b),
        (IntegerLike::BigInteger(a), IntegerLike::BigInteger(b)) => {
            demote!(&mut universe.gc_interface, a.as_ref() & b.as_ref())
        }
        (IntegerLike::BigInteger(a), IntegerLike::Integer(b))
        | (IntegerLike::Integer(b), IntegerLike::BigInteger(a)) => {
            demote!(&mut universe.gc_interface, a.as_ref() & BigInt::from(b))
        }
    };

    Ok(value)
}

fn bitxor(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: IntegerLike,
    b: IntegerLike,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#bitXor:";

    let value = match (a, b) {
        (IntegerLike::Integer(a), IntegerLike::Integer(b)) => Value::Integer(a ^ b),
        (IntegerLike::BigInteger(a), IntegerLike::BigInteger(b)) => {
            demote!(&mut universe.gc_interface, a.as_ref() ^ b.as_ref())
        }
        (IntegerLike::BigInteger(a), IntegerLike::Integer(b))
        | (IntegerLike::Integer(b), IntegerLike::BigInteger(a)) => {
            demote!(&mut universe.gc_interface, a.as_ref() ^ BigInt::from(b))
        }
    };

    Ok(value)
}

fn lt(
    _: &mut Interpreter,
    _: &mut Universe,
    a: DoubleLike,
    b: DoubleLike,
) -> Result<Value, Error> {
    const SIGNATURE: &str = "Integer>>#<";

    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => Value::Boolean(a < b),
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => Value::Boolean(a.as_ref() < b.as_ref()),
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Value::Boolean(a < b),
        (DoubleLike::Integer(a), DoubleLike::Double(b)) => Value::Boolean((a as f64) < b),
        (DoubleLike::Double(a), DoubleLike::Integer(b)) => Value::Boolean(a < (b as f64)),
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) => {
            Value::Boolean(a.as_ref() < &BigInt::from(b))
        }
        (DoubleLike::Integer(a), DoubleLike::BigInteger(b)) => {
            Value::Boolean(&BigInt::from(a) < b.as_ref())
        }
        _ => {
            bail!("'{SIGNATURE}': wrong types");
        }
    };

    Ok(value)
}

fn eq(
    _: &mut Interpreter,
    _: &mut Universe,
    a: Value,
    b: Value,
) -> Result<bool, Error> {
    const _: &str = "Integer>>#=";

    let Ok(a) = DoubleLike::try_from(a) else {
        return Ok(false);
    };

    let Ok(b) = DoubleLike::try_from(b) else {
        return Ok(false);
    };

    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => a == b,
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => a.to_obj() == b.to_obj(),
        (DoubleLike::Double(a), DoubleLike::Double(b)) => a == b,
        (DoubleLike::Integer(a), DoubleLike::Double(b)) => (a as f64) == b,
        (DoubleLike::Double(a), DoubleLike::Integer(b)) => a == (b as f64),
        _ => false,
    };

    Ok(value)
}

fn shift_left(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: IntegerLike,
    b: i32,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#<<";

    // SOM's test suite are (loosely) checking that bit-shifting operations are:
    // - logical shifts rather than arithmetic shifts
    // - performed using 32-bit integers TODO in nicolas' code, but not right now
    //
    // Since our unboxed integers are signed 32-bit integers (`i64`), we need to:
    // - perform integer promotion to an unsigned 64-bit integer (`u64`)
    // - perform the logical bit-shift (bitshifts on unsigned types are logical shifts in Rust)
    // - attempt to demote it back to `i64`, otherwise we store it as a `BigInt`

    let heap = &mut universe.gc_interface;

    match a {
        IntegerLike::Integer(a) => match (a as u64).checked_shl(b as u32) {
            Some(value) => match value.try_into() {
                Ok(value) => Ok(Value::Integer(value)),
                Err(_) => {
                    Ok(Value::BigInteger(GCRef::<BigInt>::alloc(BigInt::from(value as i64), heap)))
                }
            },
            None => Ok(demote!(heap, BigInt::from(a) << (b as u32))),
        },
        IntegerLike::BigInteger(a) => Ok(demote!(heap, a.as_ref() << (b as u32)))
    }
}

fn shift_right(
    _: &mut Interpreter,
    universe: &mut Universe,
    a: IntegerLike,
    b: i32,
) -> Result<Value, Error> {
    const _: &str = "Integer>>#>>";

    // SOM's test suite are (loosely) checking that bit-shifting operations are:
    // - logical shifts rather than arithmetic shifts
    // - performed using 64-bit integers
    //
    // Since our unboxed integers are signed 32-bit integers (`i64`), we need to:
    // - perform integer promotion to an unsigned 64-bit integer (`u64`)
    // - perform the logical bit-shift (bitshifts on unsigned types are logical shifts in Rust)
    // - attempt to demote it back to `i64`, otherwise we store it as a `BigInt`

    let heap = &mut universe.gc_interface;

    let value = match a {
        IntegerLike::Integer(a) => match (a as u64).checked_shr(b as u32) {
            Some(value) => match value.try_into() {
                Ok(value) => Value::Integer(value),
                Err(_) => {
                    let allocated = universe.gc_interface.allocate(BigInt::from(value as i32));
                    Value::BigInteger(allocated)
                }
            },
            None => {
                let uint = BigUint::from_bytes_le(&a.to_bigint().unwrap().to_signed_bytes_le());
                let result = uint >> (b as u32);
                demote!(heap, BigInt::from_signed_bytes_le(&result.to_bytes_le()))
            }
        },
        IntegerLike::BigInteger(a) => {
            let uint = BigUint::from_bytes_le(&a.to_obj().to_signed_bytes_le());
            let result = uint >> (b as u32);
            demote!(heap, BigInt::from_signed_bytes_le(&result.to_bytes_le()))
        }
    };

    Ok(value)
}

// Nota Bene: blocks for to:do: and friends get instrumented as a special case in the parser, so that they don't leave their "self" on the stack.
fn to_do(interpreter: &mut Interpreter, universe: &mut Universe, start: i32, end: i32, blk: GCRef<Block>) -> Result<i32, Error> {
    // calling rev() because it's a stack of frames: LIFO means we want to add the last one first, then the penultimate one, etc., til the first
    for i in (start..=end).rev() {
        interpreter.push_block_frame_with_args(blk, &[Value::Block(blk), Value::Integer(i)], &mut universe.gc_interface);
    }

    Ok(start)
}

fn to_by_do(interpreter: &mut Interpreter, universe: &mut Universe, start: i32, step: i32, end: i32, blk: GCRef<Block>) -> Result<i32, Error> {
    for i in (start..=end).rev().step_by(step as usize) {
        interpreter.push_block_frame_with_args(blk, &[Value::Block(blk), Value::Integer(i)], &mut universe.gc_interface);
    }

    Ok(start)
}

fn down_to_do(interpreter: &mut Interpreter, universe: &mut Universe, start: i32, end: i32, blk: GCRef<Block>) -> Result<i32, Error> {
    for i in end..=start {
        interpreter.push_block_frame_with_args(blk, &[Value::Block(blk), Value::Integer(i)], &mut universe.gc_interface);
    }

    Ok(start)
}

// NB: this guy isn't a speedup, it's never used in our benchmarks as far as I'm aware.
fn down_to_by_do(interpreter: &mut Interpreter, universe: &mut Universe, start: i32, step: i32, end: i32, blk: GCRef<Block>) -> Result<i32, Error> {
    for i in (start..=end).step_by(step as usize) {
        interpreter.push_block_frame_with_args(blk, &[Value::Block(blk), Value::Integer(i)], &mut universe.gc_interface);
    }

    Ok(start)
}

// NB: also not a speedup, also unused.
fn times_repeat(interpreter: &mut Interpreter, universe: &mut Universe, n: i32, blk: GCRef<Block>) -> Result<i32, Error> {
    for _ in 1..=n {
        interpreter.push_block_frame_with_args(blk, &[Value::Block(blk)], &mut universe.gc_interface); // NB: this doesn't take the index as an argument
    }
    Ok(n)
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
