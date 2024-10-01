use num_bigint::{BigInt, BigUint, Sign, ToBigInt};
use num_traits::{Signed, ToPrimitive};
use once_cell::sync::Lazy;
use rand::distributions::Uniform;
use rand::Rng;

use crate::convert::{DoubleLike, IntegerLike, Primitive, StringLike};
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use som_core::gc::GCRef;

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
        )
    ])
});

pub static CLASS_PRIMITIVES: Lazy<Box<[(&str, &'static PrimitiveFn, bool)]>> =
    Lazy::new(|| Box::new([("fromString:", self::from_string.into_func(), true)]));


macro_rules! demote {
    ($heap:expr, $expr:expr) => {{
        let value = $expr;
        match value.to_i32() {
            Some(value) => Return::Local(Value::Integer(value)),
            None => Return::Local(Value::BigInteger(GCRef::<BigInt>::alloc(value, $heap))),
        }
    }};
}

fn from_string(universe: &mut Universe, _: Value, string: StringLike) -> Return {
    let value = match string {
        StringLike::String(ref value) => value.as_str(),
        StringLike::Symbol(sym) => universe.lookup_symbol(sym)
    };

    match value.parse::<i32>() {
        Ok(a) => Return::Local(Value::Integer(a)),
        Err(_) => {
            match value.parse::<BigInt>() {
                Ok(b) => Return::Local(Value::BigInteger(GCRef::<BigInt>::alloc(b, &mut universe.gc_interface))),
                _ => panic!("couldn't turn an int/bigint into a string")
            }
        }
    }
}

fn as_string(universe: &mut Universe, receiver: IntegerLike) -> Return {
    let value = match receiver {
        IntegerLike::Integer(value) => value.to_string(),
        IntegerLike::BigInteger(value) => value.as_ref().to_string(),
    };

    Return::Local(Value::String(GCRef::<String>::alloc(value, &mut universe.gc_interface)))
}

fn as_double(_: &mut Universe, receiver: IntegerLike) -> Return {
    const SIGNATURE: &str = "Integer>>#asDouble";

    match receiver {
        IntegerLike::Integer(value) => Return::Local(Value::Double(value as f64)),
        IntegerLike::BigInteger(value) => match value.as_ref().to_i64() {
            Some(value) => Return::Local(Value::Double(value as f64)),
            None => Return::Exception(format!(
                "'{}': `Integer` too big to be converted to `Double`",
                SIGNATURE
            )),
        }
    }
}

fn at_random(_: &mut Universe, receiver: IntegerLike) -> Return {
    const SIGNATURE: &str = "Integer>>#atRandom";

    let chosen = match receiver {
        IntegerLike::Integer(value) => {
            let distribution = Uniform::new(0, value);
            let mut rng = rand::thread_rng();
            rng.sample(distribution)
        }
        IntegerLike::BigInteger(_) => {
            return Return::Exception(format!(
                "'{}': the range is too big to pick a random value from",
                SIGNATURE,
            ))
        }
    };

    Return::Local(Value::Integer(chosen))
}

fn as_32bit_signed_value(_: &mut Universe, receiver: IntegerLike) -> Return {
    let value = match receiver {
        IntegerLike::Integer(value) => value,
        IntegerLike::BigInteger(value) => match value.as_ref().to_u32_digits() {
            (Sign::Minus, values) => -(values[0] as i32),
            (Sign::Plus, values) | (Sign::NoSign, values) => values[0] as i32,
        },
    };

    Return::Local(Value::Integer(value))
}

fn as_32bit_unsigned_value(universe: &mut Universe, receiver: IntegerLike) -> Return {
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
        Ok(value) => Value::Integer(value),
        Err(_) => {
            Value::BigInteger(GCRef::<BigInt>::alloc(BigInt::from(value), &mut universe.gc_interface))
        }
    };

    Return::Local(value)
}

fn plus(universe: &mut Universe, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Integer>>#+";

    let heap = &mut universe.gc_interface;
    match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_add(b) {
            Some(value) => Return::Local(Value::Integer(value)),
            None => demote!(heap, BigInt::from(a) + BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => demote!(heap, a.as_ref() + b.as_ref()),
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) | (DoubleLike::Integer(b), DoubleLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() + BigInt::from(b))
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Return::Local(Value::Double(a + b)),
        (DoubleLike::Integer(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::Integer(a)) => {
            Return::Local(Value::Double((a as f64) + b))
        }
        (DoubleLike::BigInteger(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::BigInteger(a)) => {
            match a.as_ref().to_f64() {
                Some(a) => Return::Local(Value::Double(a + b)),
                None => Return::Exception(format!(
                    "'{}': `Integer` too big to be converted to `Double`",
                    SIGNATURE
                )),
            }
        }
    }
}

fn minus(universe: &mut Universe, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Integer>>#-";

    let heap = &mut universe.gc_interface;

    match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_sub(b) {
            Some(value) => Return::Local(Value::Integer(value)),
            None => demote!(heap, BigInt::from(a) - BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => demote!(heap, a.as_ref() - b.as_ref()),
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) => {
            demote!(heap, a.as_ref() - BigInt::from(b))
        }
        (DoubleLike::Integer(a), DoubleLike::BigInteger(b)) => {
            demote!(heap, BigInt::from(a) - b.as_ref())
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Return::Local(Value::Double(a - b)),
        (DoubleLike::Integer(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::Integer(a)) => {
            Return::Local(Value::Double((a as f64) - b))
        }
        (DoubleLike::BigInteger(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::BigInteger(a)) => {
            match a.as_ref().to_f64() {
                Some(a) => Return::Local(Value::Double(a - b)),
                None => Return::Exception(format!(
                    "'{}': `Integer` too big to be converted to `Double`",
                    SIGNATURE
                )),
            }
        }
    }
}

fn times(universe: &mut Universe, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Integer>>#*";

    let heap = &mut universe.gc_interface;

    match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_mul(b) {
            Some(value) => Return::Local(Value::Integer(value)),
            None => demote!(heap, BigInt::from(a) * BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => demote!(heap, a.as_ref() * b.as_ref()),
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) | (DoubleLike::Integer(b), DoubleLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() * BigInt::from(b))
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Return::Local(Value::Double(a * b)),
        (DoubleLike::Integer(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::Integer(a)) => {
            Return::Local(Value::Double((a as f64) * b))
        }
        _ => Return::Exception(format!("'{}': wrong types", SIGNATURE)),
    }
}

fn divide(universe: &mut Universe, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Integer>>#/";

    let heap = &mut universe.gc_interface;

    match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => match a.checked_div(b) {
            Some(value) => Return::Local(Value::Integer(value)),
            None => demote!(heap, BigInt::from(a) / BigInt::from(b)),
        },
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => demote!(heap, a.as_ref() / b.as_ref()),
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) | (DoubleLike::Integer(b), DoubleLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() / BigInt::from(b))
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Return::Local(Value::Double(a / b)),
        (DoubleLike::Integer(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::Integer(a)) => {
            Return::Local(Value::Double((a as f64) / b))
        }
        _ => Return::Exception(format!("'{}': wrong types", SIGNATURE)),
    }
}

fn divide_float(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Integer>>#//";

    match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => {
            Return::Local(Value::Double((a as f64) / (b as f64)))
        }
        (DoubleLike::Integer(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::Integer(a)) => {
            Return::Local(Value::Double((a as f64) / b))
        }
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Return::Local(Value::Double(a / b)),
        _ => Return::Exception(format!("'{}': wrong types", SIGNATURE)),
    }
}

fn modulo(universe: &mut Universe, a: IntegerLike, b: i32) -> Return {
    match a {
        IntegerLike::Integer(a) => {
            let result = a % b;
            if result.signum() != b.signum() {
                Return::Local(Value::Integer((result + b) % b))
            } else {
                Return::Local(Value::Integer(result))
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
    }
}

fn remainder(_: &mut Universe, a: i32, b: i32) -> Return {
    let result = a % b;
    if result.signum() != a.signum() {
        Return::Local(Value::Integer((result + a) % a))
    } else {
        Return::Local(Value::Integer(result))
    }
}

fn sqrt(universe: &mut Universe, a: DoubleLike) -> Return {
    match a {
        DoubleLike::Integer(a) => {
            let sqrt = (a as f64).sqrt();
            let trucated = sqrt.trunc();
            if sqrt == trucated {
                Return::Local(Value::Integer(trucated as i32))
            } else {
                Return::Local(Value::Double(sqrt))
            }
        }
        DoubleLike::BigInteger(a) => demote!(&mut universe.gc_interface, a.as_ref().sqrt()),
        DoubleLike::Double(a) => Return::Local(Value::Double(a.sqrt()))
    }
}

fn bitand(universe: &mut Universe, a: IntegerLike, b: IntegerLike) -> Return {
    let heap = &mut universe.gc_interface;
    match (a, b) {
        (IntegerLike::Integer(a), IntegerLike::Integer(b)) => Return::Local(Value::Integer(a & b)),
        (IntegerLike::BigInteger(a), IntegerLike::BigInteger(b)) => demote!(heap, a.as_ref() & b.as_ref()),
        (IntegerLike::BigInteger(a), IntegerLike::Integer(b)) | (IntegerLike::Integer(b), IntegerLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() & BigInt::from(b))
        }
    }
}

fn bitxor(universe: &mut Universe, a: IntegerLike, b: IntegerLike) -> Return {
    let heap = &mut universe.gc_interface;
    match (a, b) {
        (IntegerLike::Integer(a), IntegerLike::Integer(b)) => Return::Local(Value::Integer(a ^ b)),
        (IntegerLike::BigInteger(a), IntegerLike::BigInteger(b)) => demote!(heap, a.as_ref() ^ b.as_ref()),
        (IntegerLike::BigInteger(a), IntegerLike::Integer(b)) | (IntegerLike::Integer(b), IntegerLike::BigInteger(a)) => {
            demote!(heap, a.as_ref() ^ BigInt::from(b))
        }
    }
}

fn lt(_: &mut Universe, a: DoubleLike, b: DoubleLike) -> Return {
    const SIGNATURE: &str = "Integer>>#<";

    match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => Return::Local(Value::Boolean(a < b)),
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => Return::Local(Value::Boolean(a.as_ref() < b.as_ref())),
        (DoubleLike::Double(a), DoubleLike::Double(b)) => Return::Local(Value::Boolean(a < b)),
        (DoubleLike::Integer(a), DoubleLike::Double(b)) | (DoubleLike::Double(b), DoubleLike::Integer(a)) => {
            Return::Local(Value::Boolean((a as f64) < b))
        }
        (DoubleLike::BigInteger(a), DoubleLike::Integer(b)) => {
            Return::Local(Value::Boolean(a.as_ref() < &BigInt::from(b)))
        }
        (DoubleLike::Integer(a), DoubleLike::BigInteger(b)) => {
            Return::Local(Value::Boolean(&BigInt::from(a) < b.as_ref()))
        }
        _ => Return::Exception(format!("'{}': wrong types", SIGNATURE)),
    }
}

fn eq(_: &mut Universe, a: Value, b: Value) -> Return {
    // match (a, b) {
    //     (Value::Integer(a), Value::Integer(b)) => Return::Local(Value::Boolean(a == b)),
    //     (Value::BigInteger(a), Value::BigInteger(b)) => Return::Local(Value::Boolean(a.as_ref() == b.as_ref())),
    //     (Value::Integer(a), Value::BigInteger(b)) | (Value::BigInteger(b), Value::Integer(a)) => {
    //         Return::Local(Value::Boolean(BigInt::from(a) == *b.as_ref()))
    //     }
    //     (Value::Double(a), Value::Double(b)) => Return::Local(Value::Boolean(a == b)),
    //     (Value::Integer(a), Value::Double(b)) | (Value::Double(b), Value::Integer(a)) => {
    //         Return::Local(Value::Boolean((a as f64) == b))
    //     }
    //     _ => Return::Local(Value::Boolean(false)),
    // }

    let Ok(a) = DoubleLike::try_from(a) else {
        return Return::Local(Value::Boolean(false));
    };

    let Ok(b) = DoubleLike::try_from(b) else {
        return Return::Local(Value::Boolean(false));
    };
    
    let value = match (a, b) {
        (DoubleLike::Integer(a), DoubleLike::Integer(b)) => a == b,
        (DoubleLike::BigInteger(a), DoubleLike::BigInteger(b)) => a.as_ref() == b.as_ref(),
        (DoubleLike::Double(a), DoubleLike::Double(b)) => a == b,
        (DoubleLike::Integer(a), DoubleLike::Double(b)) => (a as f64) == b,
        (DoubleLike::Double(a), DoubleLike::Integer(b)) => a == (b as f64),
        _ => false,
    };

    Return::Local(Value::Boolean(value))
}

fn shift_left(universe: &mut Universe, a: IntegerLike, b: i32) -> Return {
    // old code pre integers being i32 because of nan boxing:

    // match a {
    //     Value::Integer(a) => match a.checked_shl(b as u32) {
    //         Some(value) => Return::Local(Value::Integer(value)),
    //         None => demote!(BigInt::from(a) << (b as usize)),
    //     },
    //     Value::BigInteger(a) => demote!(a << (b as usize)),
    //     _ => Return::Exception(format!("'{}': wrong types", SIGNATURE)),
    // }

    let heap = &mut universe.gc_interface;

    match a {
        IntegerLike::Integer(a) => match (a as u64).checked_shl(b as u32) {
            Some(value) => match value.try_into() {
                Ok(value) => Return::Local(Value::Integer(value)),
                Err(_) => {
                    Return::Local(Value::BigInteger(GCRef::<BigInt>::alloc(BigInt::from(value as i64), heap)))
                }
            },
            None => demote!(heap, BigInt::from(a) << (b as u32)),
        },
        IntegerLike::BigInteger(a) => demote!(heap, a.as_ref() << (b as u32))
    }
}

fn shift_right(universe: &mut Universe, a: IntegerLike, b: i32) -> Return {
    
    // match a {
    //     Value::Integer(a) => match a.checked_shr(b as u32) {
    //         Some(value) => Return::Local(Value::Integer(value)),
    //         None => demote!(BigInt::from(a) >> (b as usize)),
    //     },
    //     Value::BigInteger(a) => demote!(a >> (b as usize)),
    //     _ => Return::Exception(format!("'{}': wrong types", SIGNATURE)),
    // }

    let heap = &mut universe.gc_interface;

    match a {
        IntegerLike::Integer(a) => match (a as u64).checked_shr(b as u32) {
            Some(value) => {
                match value.try_into() {
                    Ok(value) => Return::Local(Value::Integer(value)),
                    Err(_) => {
                        Return::Local(Value::BigInteger(GCRef::<BigInt>::alloc(BigInt::from(value), heap)))
                    }
                }
            }
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
    }
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
