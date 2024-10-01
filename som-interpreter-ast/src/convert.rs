// This is all Nicolas Polomack (https://github.com/Hirevo)'s work, despite what the commit history says.
// Nicolas is the original dev for som-rs, and had this code in an unmerged PR about Nan boxing. 
// I didn't merge with his commits directly because his version of som-rs and mine have diverged a lot. But the credit is his, my edits are minor so far

use std::convert::TryFrom;

use anyhow::{bail, Context, Error};

use crate::block::Block;
use crate::class::Class;
use crate::instance::Instance;
use crate::invokable::Return;
use crate::method::Method;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::Value;
use num_bigint::BigInt;
use som_core::gc::{GCInterface, GCRef};
use som_core::interner::Interned;

pub trait IntoValue {
    fn into_value(&self, gc_interface: &mut GCInterface) -> Value;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nil;

impl TryFrom<Value> for Nil {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value == Value::NIL {
            Ok(Self)
        } else {
            bail!("could not resolve `Value` as `Nil`")
        }
    }
}

impl FromArgs for Nil {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        Self::try_from(arg)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct System;

impl TryFrom<Value> for System {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value == Value::SYSTEM {
            Ok(Self)
        } else {
            bail!("could not resolve `Value` as `System`")
        }
    }
}

impl FromArgs for System {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        Self::try_from(arg)
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
        value
            .as_string()
            .map(Self::String)
            .or_else(|| value.as_symbol().map(Self::Symbol))
            .context("could not resolve `Value` as `String`, or `Symbol`")
    }
}

impl FromArgs for StringLike {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        Self::try_from(arg)
    }
}

#[derive(Debug, Clone)]
pub enum DoubleLike {
    Double(f64),
    Integer(i32),
    BigInteger(GCRef<BigInt>),
}

impl TryFrom<Value> for DoubleLike {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .as_double()
            .map(Self::Double)
            .or_else(|| value.as_integer().map(Self::Integer))
            .or_else(|| value.as_big_integer().map(Self::BigInteger))
            .context("could not resolve `Value` as `Double`, `Integer`, or `BigInteger`")
    }
}

impl FromArgs for DoubleLike {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        Self::try_from(arg)
    }
}

#[derive(Debug, Clone)]
pub enum IntegerLike {
    Integer(i32),
    BigInteger(GCRef<BigInt>),
}

impl TryFrom<Value> for IntegerLike {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .as_integer()
            .map(Self::Integer)
            .or_else(|| value.as_big_integer().map(Self::BigInteger))
            .context("could not resolve `Value` as `Integer`, or `BigInteger`")
    }
}

impl FromArgs for IntegerLike {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        Self::try_from(arg)
    }
}

pub trait FromArgs: Sized {
    fn from_args(
        arg: Value,
        universe: &mut Universe,
    ) -> Result<Self, Error>;
}

impl FromArgs for Value {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        Ok(arg)
    }
}

impl FromArgs for bool {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_boolean().context("could not resolve `Value` as `Boolean`")
    }
}

impl FromArgs for i32 {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_integer().context("could not resolve `Value` as `Integer`")
    }
}

impl FromArgs for f64 {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_double().context("could not resolve `Value` as `Double`")
    }
}

impl FromArgs for Interned {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_symbol().context("could not resolve `Value` as `Symbol`")
    }
}

impl FromArgs for GCRef<String> {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_string().context("could not resolve `Value` as `String`")
    }
}

impl FromArgs for GCRef<Vec<Value>> {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_array().context("could not resolve `Value` as `Array`")
    }
}

impl FromArgs for GCRef<Class> {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_class().context("could not resolve `Value` as `Class`")
    }
}

impl FromArgs for GCRef<Instance> {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_instance().context("could not resolve `Value` as `Instance`")
    }
}

impl FromArgs for GCRef<Block> {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_block().context("could not resolve `Value` as `Block`")
    }
}

impl FromArgs for GCRef<Method> {
    fn from_args(
        arg: Value,
        _: &mut Universe,
    ) -> Result<Self, Error> {
        arg.as_invokable().context("could not resolve `Value` as `Method`")
    }
}

impl IntoValue for bool {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::Boolean(*self)
    }
}

impl IntoValue for i32 {
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
        universe: &mut Universe,
        args: Vec<Value>,
    ) -> Return;

    fn into_func(self) -> &'static PrimitiveFn {
        let boxed = Box::new(
            move |universe: &mut Universe, args: Vec<Value>| {
                self.invoke(universe, args)
            },
        );
        Box::leak(boxed)
    }
}

macro_rules! derive_stuff {
    ($($ty:ident),* $(,)?) => {
        impl <F, $($ty),*> $crate::convert::Primitive<($($ty),*,)> for F
        where
            F: Fn(&mut $crate::universe::Universe, $($ty),*) -> Return + Send + Sync + 'static,
            $($ty: $crate::convert::FromArgs),*,
        {
            fn invoke(&self, universe: &mut $crate::universe::Universe, args: Vec<Value>) -> Return {
                //no_longer_need_reverse!(universe, args.iter(), [$($ty),*]);
                
                let mut args_iter = args.iter();
                $(
                    #[allow(non_snake_case)]
                    let $ty = $ty::from_args(args_iter.next().unwrap().clone(), universe).unwrap();
                )*
                
                (self)(universe, $($ty),*,)
            }
        }
    };
}

impl IntoValue for Value {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        self.clone()
    }
}

impl IntoValue for Nil {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::NIL
    }
}

impl IntoValue for System {
    fn into_value(&self, _: &mut GCInterface) -> Value {
        Value::SYSTEM
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(&self, heap: &mut GCInterface) -> Value {
        self.as_ref().map_or(Value::NIL, |it| it.into_value(heap))
    }
}

// impl IntoReturn for () {
//     fn into_return(self, _: &mut GCInterface) -> Return {
//         todo!("err, not sure?")
//     }
// }

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
    F: Fn(&mut Universe, Vec<Value>) -> Return
    + Send
    + Sync
    + 'static,
{
    fn invoke(&self, universe: &mut Universe, args: Vec<Value>) -> Return {
        self(universe, args)
    }
}

derive_stuff!(_A);
derive_stuff!(_A, _B);
derive_stuff!(_A, _B, _C);
derive_stuff!(_A, _B, _C, _D);
derive_stuff!(_A, _B, _C, _D, _E);
derive_stuff!(_A, _B, _C, _D, _E, _F);
