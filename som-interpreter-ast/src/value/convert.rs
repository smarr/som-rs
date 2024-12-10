use std::convert::TryFrom;

use anyhow::{bail, Context, Error};

use crate::gc::VecValue;
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::value_ptr::HeapValPtr;
use crate::value::Value;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::instance::Instance;
use crate::vm_objects::method::Method;
use num_bigint::BigInt;
use som_core::interner::Interned;
use som_core::value::HasPointerTag;
use som_gc::gcref::Gc;

pub trait IntoValue {
    #[allow(clippy::wrong_self_convention)] // though i guess we could/should rename it
    fn into_value(&self) -> Value;
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

impl FromArgs<'_> for Nil {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(*arg)
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

impl FromArgs<'_> for System {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(*arg)
    }
}

#[derive(Debug, Clone)]
pub enum StringLike {
    String(Gc<String>),
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

impl FromArgs<'_> for StringLike {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(*arg)
    }
}

#[derive(Debug, Clone)]
pub enum DoubleLike {
    Double(f64),
    Integer(i32),
    BigInteger(Gc<BigInt>),
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

impl FromArgs<'_> for DoubleLike {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(*arg)
    }
}

#[derive(Debug, Clone)]
pub enum IntegerLike {
    Integer(i32),
    BigInteger(Gc<BigInt>),
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

impl FromArgs<'_> for IntegerLike {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(*arg)
    }
}

pub trait FromArgs<'a>: Sized {
    fn from_args(arg: &'a Value) -> Result<Self, Error>;
}

impl FromArgs<'_> for Value {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Ok(*arg)
    }
}

impl FromArgs<'_> for bool {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_boolean().context("could not resolve `Value` as `Boolean`")
    }
}

impl FromArgs<'_> for i32 {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_integer().context("could not resolve `Value` as `Integer`")
    }
}

impl FromArgs<'_> for f64 {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_double().context("could not resolve `Value` as `Double`")
    }
}

impl FromArgs<'_> for Interned {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_symbol().context("could not resolve `Value` as `Symbol`")
    }
}

impl<T> FromArgs<'_> for HeapValPtr<T>
where
    T: HasPointerTag,
{
    fn from_args(arg: &Value) -> Result<Self, Error> {
        unsafe { Ok(HeapValPtr::new_static(arg)) }
    }
}

impl FromArgs<'_> for Return {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Ok(Return::Local(*arg))
    }
}

impl IntoValue for bool {
    fn into_value(&self) -> Value {
        Value::Boolean(*self)
    }
}

impl IntoValue for i32 {
    fn into_value(&self) -> Value {
        Value::Integer(*self)
    }
}

impl IntoValue for f64 {
    fn into_value(&self) -> Value {
        Value::Double(*self)
    }
}

impl IntoValue for Interned {
    fn into_value(&self) -> Value {
        Value::Symbol(*self)
    }
}

impl IntoValue for Gc<String> {
    fn into_value(&self) -> Value {
        Value::String(*self)
    }
}

impl IntoValue for Gc<BigInt> {
    fn into_value(&self) -> Value {
        Value::BigInteger(*self)
    }
}

impl IntoValue for Gc<VecValue> {
    fn into_value(&self) -> Value {
        Value::Array(*self)
    }
}

impl IntoValue for Gc<Class> {
    fn into_value(&self) -> Value {
        Value::Class(*self)
    }
}

impl IntoValue for Gc<Instance> {
    fn into_value(&self) -> Value {
        Value::Instance(*self)
    }
}

impl IntoValue for Gc<Block> {
    fn into_value(&self) -> Value {
        Value::Block(*self)
    }
}

impl IntoValue for Gc<Method> {
    fn into_value(&self) -> Value {
        Value::Invokable(*self)
    }
}

pub trait Primitive<T>: Sized + Send + Sync + 'static {
    fn invoke(&self, universe: &mut Universe, nbr_args: usize) -> Return;

    fn into_func(self) -> &'static PrimitiveFn {
        let boxed = Box::new(move |universe: &mut Universe, nbr_args: usize| self.invoke(universe, nbr_args));
        Box::leak(boxed)
    }
}

pub trait IntoReturn {
    fn into_return(self) -> Return;
}

impl<T: IntoValue> IntoReturn for T {
    fn into_return(self) -> Return {
        Return::Local(self.into_value())
    }
}

impl IntoReturn for Return {
    fn into_return(self) -> Return {
        self
    }
}

impl IntoValue for Value {
    fn into_value(&self) -> Value {
        *self
    }
}

impl IntoValue for Nil {
    fn into_value(&self) -> Value {
        Value::NIL
    }
}

impl IntoValue for System {
    fn into_value(&self) -> Value {
        Value::SYSTEM
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(&self) -> Value {
        self.as_ref().map_or(Value::NIL, |it| it.into_value())
    }
}

impl IntoValue for StringLike {
    fn into_value(&self) -> Value {
        match self {
            StringLike::String(value) => value.into_value(),
            StringLike::Symbol(value) => value.into_value(),
        }
    }
}

impl IntoValue for IntegerLike {
    fn into_value(&self) -> Value {
        match self {
            IntegerLike::Integer(value) => value.into_value(),
            IntegerLike::BigInteger(value) => value.into_value(),
        }
    }
}

impl IntoValue for DoubleLike {
    fn into_value(&self) -> Value {
        match self {
            DoubleLike::Double(value) => value.into_value(),
            DoubleLike::Integer(value) => value.into_value(),
            DoubleLike::BigInteger(value) => value.into_value(),
        }
    }
}

macro_rules! derive_stuff {
    ($($ty:ident),* $(,)?) => {
        impl <F, R, $($ty),*> $crate::value::convert::Primitive<($($ty),*,)> for F
        where
            F: Fn(&mut $crate::universe::Universe, $($ty),*) -> Result<R, Error> + Send + Sync + 'static,
            R: $crate::value::convert::IntoReturn,
            $(for<'a> $ty: $crate::value::convert::FromArgs<'a>),*,
        {
            fn invoke(&self, universe: &mut $crate::universe::Universe, nbr_args: usize) -> Return {
                // let args = universe.stack_n_last_elems(nbr_args);

                // We need to keep the elements on the stack to have them be reachable still when GC happens.
                // But borrowing them means borrowing the universe immutably, so we duplicate the reference.
                // # Safety
                // AFAIK this is safe since the stack isn't going to move in the meantime.
                // HOWEVER, if it gets resized/reallocated by Rust... Maybe? I'm not sure...
                let args: &[Value] = unsafe { &* (universe.stack_borrow_n_last_elems(nbr_args) as *const _) };
                let mut args_iter = args.iter();
                $(
                    #[allow(non_snake_case)]
                    let $ty = $ty::from_args(args_iter.next().unwrap()).unwrap();
                )*

                let result = (self)(universe, $($ty),*,).unwrap();
                universe.stack_pop_n(nbr_args);
                result.into_return()
            }
        }
    };
}

derive_stuff!(_A);
derive_stuff!(_A, _B);
derive_stuff!(_A, _B, _C);
derive_stuff!(_A, _B, _C, _D);
derive_stuff!(_A, _B, _C, _D, _E);
derive_stuff!(_A, _B, _C, _D, _E, _F);

// for blocks. TODO: from a macro instead.
impl<F, R> Primitive<()> for F
where
    F: Fn(&mut Universe) -> Result<R, Error> + Send + Sync + 'static,
    R: IntoReturn,
{
    fn invoke(&self, universe: &mut Universe, _nbr_args: usize) -> Return {
        let result = self(universe).unwrap();
        result.into_return()
    }
}
