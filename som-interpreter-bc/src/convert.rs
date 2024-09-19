// This is all Nicolas Polomack (https://github.com/Hirevo)'s work, despite what the commit history says.
// Nicolas is the original dev for som-rs, and had this code in an unmerged PR about Nan boxing. 
// I didn't merge with his commits directly because his version of som-rs and mine have diverged a lot. But the credit is his, my edits are minor so far

use std::convert::TryFrom;

use anyhow::{anyhow, bail, Context, Error};

use crate::block::Block;
use crate::class::Class;
use crate::instance::Instance;
use crate::interner::Interned;
use crate::interpreter::Interpreter;
use crate::method::Method;
use crate::primitives::PrimitiveFn;
use crate::universe::UniverseBC;
use crate::value::Value;
use num_bigint::BigInt;
use som_core::gc::{GCInterface, GCRef};

pub trait IntoValue {
    fn into_value(&self, gc_interface: &mut GCInterface) -> Value;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nil;

impl TryFrom<Value> for Nil {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Nil => Ok(Self),
            _ => bail!("could not resolve `Value` as `Nil`")
        }
    }
}

impl FromArgs for Nil {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let value = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        Self::try_from(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct System;

impl TryFrom<Value> for System {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Nil => Ok(Self),
            _ => bail!("could not resolve `Value` as `System`")
        }
    }
}

impl FromArgs for System {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let value = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        Self::try_from(value)
    }
}

#[derive(Debug, Clone)]
pub enum StringLike {
    String(GCRef<String>),
    Symbol(Interned),
}

impl TryFrom<Value> for StringLike {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(string) => {
                Ok(StringLike::String(string))
            },
            Value::Symbol(i) => {
                Ok(StringLike::Symbol(i))
            }
            _ => Err(anyhow!("could not resolve `Value` as `String`, or `Symbol`"))
        }
    }
}

impl FromArgs for StringLike {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let value = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        Self::try_from(value)
    }
}

#[derive(Debug, Clone)]
pub enum DoubleLike {
    Double(f64),
    Integer(i64),
    BigInteger(GCRef<BigInt>),
}

impl TryFrom<Value> for DoubleLike {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Double(double) => {
                Ok(DoubleLike::Double(double))
            },
            Value::Integer(i) => {
                Ok(DoubleLike::Integer(i))
            },
            Value::BigInteger(i) => {
                Ok(DoubleLike::BigInteger(i))
            }
            _ => Err(anyhow!("could not resolve `Value` as `Double`, `Integer`, or `BigInteger`"))
        }
    }
}

impl FromArgs for DoubleLike {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let value = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        Self::try_from(value)
    }
}

#[derive(Debug, Clone)]
pub enum IntegerLike {
    Integer(i64),
    BigInteger(GCRef<BigInt>),
}

impl TryFrom<Value> for IntegerLike {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(i) => {
                Ok(IntegerLike::Integer(i))
            },
            Value::BigInteger(i) => {
                Ok(IntegerLike::BigInteger(i))
            }
            _ => Err(anyhow!("could not resolve `Value` as `Integer`, or `BigInteger`"))
        }
    }
}

impl FromArgs for IntegerLike {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let value = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        Self::try_from(value)
    }
}

pub trait FromArgs: Sized {
    fn from_args(
        interpreter: &mut Interpreter,
        universe: &mut UniverseBC,
    ) -> Result<Self, Error>;
}

impl FromArgs for Value {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        interpreter
            .stack
            .pop()
            .context("message send with missing argument")
    }
}

impl FromArgs for bool {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;
        match arg {
            Value::Boolean(b) => Ok(b),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Boolean`")),
        }
    }
}

impl FromArgs for i64 {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Integer(i) => Ok(i),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Integer`")),
        }
    }
}

impl FromArgs for f64 {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Double(i) => Ok(i),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Double`")),
        }
    }
}

impl FromArgs for Interned {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;
        
        match arg {
            Value::Symbol(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Symbol`")),
        }
    }
}

impl FromArgs for GCRef<String> {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::String(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `String`")),
        }
    }
}

impl FromArgs for GCRef<Vec<Value>> {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Array(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Array`")),
        }
    }
}

impl FromArgs for GCRef<Class> {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Class(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Class`")),
        }
    }
}

impl FromArgs for GCRef<Instance> {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Instance(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Instance`")),
        }
    }
}

impl FromArgs for GCRef<Block> {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Block(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Block`")),
        }
    }
}

impl FromArgs for GCRef<Method> {
    fn from_args(
        interpreter: &mut Interpreter,
        _: &mut UniverseBC,
    ) -> Result<Self, Error> {
        let arg = interpreter
            .stack
            .pop()
            .context("message send with missing argument")?;

        match arg {
            Value::Invokable(val) => Ok(val),
            _ => Err(anyhow::anyhow!("could not resolve `Value` as `Method`")),
        }
    }
}

impl IntoValue for bool {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Boolean(*self)
    }
}

impl IntoValue for i64 {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Integer(*self)
    }
}

impl IntoValue for f64 {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Double(*self)
    }
}

impl IntoValue for Interned {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Symbol(*self)
    }
}

impl IntoValue for GCRef<String> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::String(*self)
    }
}

impl IntoValue for GCRef<BigInt> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::BigInteger(*self)
    }
}

impl IntoValue for GCRef<Vec<Value>> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Array(*self)
    }
}

impl IntoValue for GCRef<Class> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Class(*self)
    }
}

impl IntoValue for GCRef<Instance> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Instance(*self)
    }
}

impl IntoValue for GCRef<Block> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Block(*self)
    }
}

impl IntoValue for GCRef<Method> {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Invokable(*self)
    }
}

pub trait Primitive<T>: Sized + Send + Sync + 'static {
    fn invoke(
        &self,
        interpreter: &mut Interpreter,
        universe: &mut UniverseBC,
    ) -> Result<(), Error>;

    fn into_func(self) -> &'static PrimitiveFn {
        let boxed = Box::new(
            move |interpreter: &mut Interpreter, universe: &mut UniverseBC| {
                self.invoke(interpreter, universe)
            },
        );
        Box::leak(boxed)
    }
}

pub trait IntoReturn {
    fn into_return(self, interpreter: &mut Interpreter, heap: &mut GCInterface) -> Result<(), Error>;
}

macro_rules! reverse {
    ($interpreter:expr, $universe:expr, [], [ $($ty:ident),* $(,)? ]) => {
        $(
            #[allow(non_snake_case)]
            let $ty = $ty::from_args($interpreter, $universe)?;
        )*
    };
    ($interpreter:expr, $universe:expr, [ $ty:ident $(,)? ], [ $($ty2:ident),* $(,)? ]) => {
        reverse!($interpreter, $universe, [], [ $ty , $($ty2),* ])
    };
    ($interpreter:expr, $universe:expr, [ $ty:ident , $($ty1:ident),* $(,)? ], [ $($ty2:ident),* $(,)? ]) => {
        reverse!($interpreter, $universe, [ $($ty1),* ], [ $ty , $($ty2),* ])
    };
}

macro_rules! derive_stuff {
    ($($ty:ident),* $(,)?) => {
        impl <$($ty: $crate::convert::IntoValue),*> $crate::convert::IntoValue for ($($ty),*,) {
            fn into_value(&self, heap: &mut GCInterface) -> $crate::value::Value {
                #[allow(non_snake_case)]
                let ($($ty),*,) = self;
                let mut values = Vec::default();
                $(
                    values.push($crate::convert::IntoValue::into_value($ty, heap));
                )*
                // let allocated = heap.allocate(::std::cell::RefCell::new(values));
                let allocated = GCRef::<Vec<Value>>::alloc(values, heap);
                $crate::value::Value::Array(allocated)
            }
        }

        impl <$($ty: $crate::convert::FromArgs),*> $crate::convert::FromArgs for ($($ty),*,) {
            fn from_args(interpreter: &mut $crate::interpreter::Interpreter, universe: &mut $crate::universe::UniverseBC) -> Result<Self, Error> {
                $(
                    #[allow(non_snake_case)]
                    let $ty = $ty::from_args(interpreter, universe)?;
                )*
                Ok(($($ty),*,))
            }
        }

        impl <F, R, $($ty),*> $crate::convert::Primitive<($($ty),*,)> for F
        where
            F: Fn(&mut $crate::interpreter::Interpreter, &mut $crate::universe::UniverseBC, $($ty),*) -> Result<R, Error> + Send + Sync + 'static,
            R: $crate::convert::IntoReturn,
            $($ty: $crate::convert::FromArgs),*,
        {
            fn invoke(&self, interpreter: &mut $crate::interpreter::Interpreter, universe: &mut $crate::universe::UniverseBC) -> Result<(), Error> {
                reverse!(interpreter, universe, [$($ty),*], []);
                let result = (self)(interpreter, universe, $($ty),*,)?;
                result.into_return(interpreter, &mut universe.gc_interface)
            }
        }
    };
}

impl<T: IntoValue> IntoReturn for T {
    fn into_return(self, interpreter: &mut Interpreter, heap: &mut GCInterface) -> Result<(), Error> {
        interpreter.stack.push(self.into_value(heap));
        Ok(())
    }
}

impl IntoValue for Value {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        self.clone()
    }
}

impl IntoValue for Nil {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Nil
    }
}

impl IntoValue for System {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::System
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(&self, heap: &mut GCInterface) -> Value {
        self.as_ref().map_or(Value::Nil, |it| it.into_value(heap))
    }
}

impl IntoReturn for () {
    fn into_return(self, _: &mut Interpreter, _: &mut GCInterface) -> Result<(), Error> {
        Ok(())
    }
}

impl IntoValue for StringLike {
    fn into_value(&self, heap: &mut GCInterface) -> Value {
        match self {
            StringLike::String(value) => value.into_value(heap),
            StringLike::Symbol(value) => value.into_value(heap),
        }
    }
}

impl IntoValue for IntegerLike {
    fn into_value(&self, heap: &mut GCInterface) -> Value {
        match self {
            IntegerLike::Integer(value) => value.into_value(heap),
            IntegerLike::BigInteger(value) => value.into_value(heap),
        }
    }
}

impl IntoValue for DoubleLike {
    fn into_value(&self, heap: &mut GCInterface) -> Value {
        match self {
            DoubleLike::Double(value) => value.into_value(heap),
            DoubleLike::Integer(value) => value.into_value(heap),
            DoubleLike::BigInteger(value) => value.into_value(heap),
        }
    }
}

impl<F> Primitive<()> for F
where
    F: Fn(&mut Interpreter, &mut UniverseBC) -> Result<(), Error>
    + Send
    + Sync
    + 'static,
{
    fn invoke(
        &self,
        interpreter: &mut Interpreter,
        universe: &mut UniverseBC,
    ) -> Result<(), Error> {
        (self)(interpreter, universe)
    }
}

derive_stuff!(_A);
derive_stuff!(_A, _B);
derive_stuff!(_A, _B, _C);
derive_stuff!(_A, _B, _C, _D);
derive_stuff!(_A, _B, _C, _D, _E);
derive_stuff!(_A, _B, _C, _D, _E, _F);
