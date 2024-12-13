use std::convert::TryFrom;

use anyhow::{bail, Context, Error};
use som_core::value_ptr::HasPointerTag;

use crate::gc::VecValue;
use crate::invokable::Return;
use crate::primitives::PrimitiveFn;
use crate::universe::{GlobalValueStack, Universe};
use crate::value::Value;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::instance::Instance;
use crate::vm_objects::method::Method;
use num_bigint::BigInt;
use som_core::interner::Interned;
use som_gc::gcref::Gc;

use super::HeapValPtr;

pub type DoubleLike = som_core::convert::DoubleLike<Gc<BigInt>>;
pub type IntegerLike = som_core::convert::IntegerLike<Gc<BigInt>>;
pub type StringLike = som_core::convert::StringLike<Gc<String>>;

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

impl FromArgs for Nil {
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

impl FromArgs for System {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(*arg)
    }
}

impl FromArgs for StringLike {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(arg.0)
    }
}

impl FromArgs for DoubleLike {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(arg.0)
    }
}

impl FromArgs for IntegerLike {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Self::try_from(arg.0)
    }
}

pub trait FromArgs: Sized {
    fn from_args(arg: &'static Value) -> Result<Self, Error>;
}

impl FromArgs for Value {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        Ok(*arg)
    }
}

impl FromArgs for bool {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_boolean().context("could not resolve `Value` as `Boolean`")
    }
}

impl FromArgs for i32 {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        i32::try_from(arg.0)
    }
}

impl FromArgs for f64 {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        f64::try_from(arg.0)
    }
}

impl FromArgs for Interned {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_symbol().context("could not resolve `Value` as `Symbol`")
    }
}

impl<T> FromArgs for HeapValPtr<T>
where
    T: HasPointerTag,
{
    fn from_args(arg: &'static Value) -> Result<Self, Error> {
        Ok(HeapValPtr::new_static(arg))
    }
}

impl FromArgs for Return {
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
    fn invoke(&self, universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return;

    fn into_func(self) -> &'static PrimitiveFn {
        let boxed = Box::new(move |universe: &mut Universe, value_stack: &mut GlobalValueStack, nbr_args: usize| {
            self.invoke(universe, value_stack, nbr_args)
        });
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
            F: Fn(&mut $crate::universe::Universe, &mut GlobalValueStack, $($ty),*) -> Result<R, Error> + Send + Sync + 'static,
            R: $crate::value::convert::IntoReturn,
            $(for<'a> $ty: $crate::value::convert::FromArgs),*,
        {
            fn invoke(&self, universe: &mut $crate::universe::Universe, value_stack: &mut GlobalValueStack, nbr_args: usize) -> Return {
                // let args = Universe::stack_n_last_elems(value_stack, nbr_args);

                // We need to keep the elements on the stack to have them be reachable still when GC happens.
                // But borrowing them means borrowing the universe immutably, so we duplicate the reference.
                // # Safety
                // AFAIK this is safe since the stack isn't going to move in the meantime.
                // HOWEVER, if it gets resized/reallocated by Rust... Maybe? I'm not sure...
                let args: &[Value] = unsafe { &* (value_stack.borrow_n_last(nbr_args) as *const _) };
                let mut args_iter = args.iter();
                $(
                    #[allow(non_snake_case)]
                    let $ty = $ty::from_args(args_iter.next().unwrap()).unwrap();
                )*

                let result = (self)(universe, value_stack, $($ty),*,).unwrap();
                value_stack.remove_n_last(nbr_args);
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
    F: Fn(&mut Universe, &mut GlobalValueStack) -> Result<R, Error> + Send + Sync + 'static,
    R: IntoReturn,
{
    fn invoke(&self, universe: &mut Universe, value_stack: &mut GlobalValueStack, _nbr_args: usize) -> Return {
        let result = self(universe, value_stack).unwrap();
        result.into_return()
    }
}
