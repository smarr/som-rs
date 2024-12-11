use std::convert::TryFrom;

use anyhow::{bail, Context, Error};
use som_core::value_ptr::HasPointerTag;

use crate::gc::VecValue;
use crate::interpreter::Interpreter;
use crate::primitives::PrimitiveFn;
use crate::universe::Universe;
use crate::value::{HeapValPtr, Value};
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::instance::Instance;
use crate::vm_objects::method::Method;
use num_bigint::BigInt;
use som_core::interner::Interned;
use som_gc::gcref::Gc;

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
        if value.is_nil() {
            Ok(Self)
        } else {
            bail!("could not resolve `Value` as `Nil`");
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
        if value.is_system() {
            Ok(Self)
        } else {
            bail!("could not resolve `Value` as `System`");
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
    fn from_args(arg: &Value) -> Result<Self, Error>;
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
        arg.as_integer().context("could not resolve `Value` as `Integer`")
    }
}

impl FromArgs for f64 {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_double().context("could not resolve `Value` as `Double`")
    }
}

impl FromArgs for Interned {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        arg.as_symbol().context("could not resolve `Value` as `Symbol`")
    }
}

// impl<T: HasPointerTag> FromArgs for Gc<T> {
//     fn from_args(arg: &Value) -> Result<Self, Error> {
//         arg.as_ptr().context("could not resolve `Value` as correct pointer")
//     }
// }

impl<T: HasPointerTag> FromArgs for HeapValPtr<T> {
    fn from_args(arg: &Value) -> Result<Self, Error> {
        unsafe { Ok(HeapValPtr::new_static(arg)) }
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
    fn invoke(&self, interpreter: &mut Interpreter, universe: &mut Universe, nbr_args: usize) -> Result<(), Error>;

    fn into_func(self) -> &'static PrimitiveFn {
        let boxed =
            Box::new(move |interpreter: &mut Interpreter, universe: &mut Universe, nbr_args: usize| self.invoke(interpreter, universe, nbr_args));
        Box::leak(boxed)
    }
}

pub trait IntoReturn {
    fn into_return(self, interpreter: &mut Interpreter, nbr_args: usize) -> Result<(), Error>;
}

impl<T: IntoValue> IntoReturn for T {
    fn into_return(self, interpreter: &mut Interpreter, nbr_args: usize) -> Result<(), Error> {
        interpreter.current_frame.remove_n_last_elements(nbr_args);
        interpreter.current_frame.stack_push(self.into_value());
        Ok(())
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

impl IntoReturn for () {
    fn into_return(self, _: &mut Interpreter, _: usize) -> Result<(), Error> {
        Ok(())
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

macro_rules! reverse {
    ($universe:expr, $stack_iter:expr, [], [ $($ty:ident),* $(,)? ]) => {
        $(
            #[allow(non_snake_case)]
            let val = $stack_iter.next().unwrap();
            #[allow(non_snake_case)]
            let $ty = $ty::from_args(val)?;
        )*
    };
    ($universe:expr, $stack_iter:expr, [ $ty:ident $(,)? ], [ $($ty2:ident),* $(,)? ]) => {
        reverse!($universe, $stack_iter, [], [ $ty , $($ty2),* ])
    };
    ($universe:expr, $stack_iter:expr, [ $ty:ident , $($ty1:ident),* $(,)? ], [ $($ty2:ident),* $(,)? ]) => {
        reverse!($universe, $stack_iter, [ $($ty1),* ], [ $ty , $($ty2),* ])
    };
}

macro_rules! derive_stuff {
    ($($ty:ident),* $(,)?) => {

        impl <F, R, $($ty),*> $crate::value::convert::Primitive<($($ty),*,)> for F
        where
            F: Fn(&mut $crate::interpreter::Interpreter, &mut $crate::universe::Universe, $($ty),*) -> Result<R, Error> + Send + Sync + 'static,
            R: $crate::value::convert::IntoReturn,
            $($ty: $crate::value::convert::FromArgs),*,
        {
            fn invoke(&self, interpreter: &mut $crate::interpreter::Interpreter, universe: &mut $crate::universe::Universe, nbr_args: usize) -> Result<(), Error> {
                let mut stack_iter = crate::vm_objects::frame::FrameStackIter::from(&*interpreter.current_frame);
                reverse!(universe, stack_iter, [$($ty),*], []);
                let result = (self)(interpreter, universe, $($ty),*,)?;
                result.into_return(interpreter, nbr_args)
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

// TODO: adapt macro instead
impl<F> Primitive<()> for F
where
    F: Fn(&mut Interpreter, &mut Universe) -> Result<(), Error> + Send + Sync + 'static,
{
    fn invoke(&self, interpreter: &mut Interpreter, universe: &mut Universe, _: usize) -> Result<(), Error> {
        self(interpreter, universe)
    }
}
