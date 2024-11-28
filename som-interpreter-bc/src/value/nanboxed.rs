use super::Value;
use crate::gc::VecValue;
use crate::universe::Universe;
use crate::value::value_enum::ValueEnum;
use crate::vm_objects::block::Block;
use crate::vm_objects::class::Class;
use crate::vm_objects::instance::Instance;
use crate::vm_objects::method::MethodOrPrim;
use num_bigint::BigInt;
use som_core::delegate_to_base_value;
use som_core::interner::Interned;
use som_core::value::*;
use som_gc::gcref::Gc;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

/// Tag bits for the `Array` type.
const ARRAY_TAG: u64 = 0b010 | CELL_BASE_TAG;
/// Tag bits for the `Block` type.
const BLOCK_TAG: u64 = 0b100 | CELL_BASE_TAG;
/// Tag bits for the `Class` type.
const CLASS_TAG: u64 = 0b101 | CELL_BASE_TAG;
/// Tag bits for the `Instance` type.
const INSTANCE_TAG: u64 = 0b110 | CELL_BASE_TAG;
/// Tag bits for the `Invokable` type.
const INVOKABLE_TAG: u64 = 0b111 | CELL_BASE_TAG;

impl Deref for Value {
    type Target = BaseValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BaseValue> for Value {
    fn from(value: BaseValue) -> Self {
        Value(value)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value(BaseValue::from(value))
    }
}

#[allow(non_snake_case)]
impl Value {
    pub const TRUE: Self = Value(BaseValue::TRUE);
    pub const FALSE: Self = Value(BaseValue::FALSE);
    pub const NIL: Self = Value(BaseValue::NIL);
    pub const SYSTEM: Self = Value(BaseValue::SYSTEM);
    pub const INTEGER_ZERO: Self = Value(BaseValue::INTEGER_ZERO);
    pub const INTEGER_ONE: Self = Value(BaseValue::INTEGER_ONE);

    delegate_to_base_value!(
        new_boolean(value: bool) -> Self,
        new_integer(value: i32) -> Self,
        new_double(value: f64) -> Self,
        new_symbol(value: Interned) -> Self,
        new_big_integer(value: Gc<BigInt>) -> Self,
        new_string(value: Gc<String>) -> Self,
        Boolean(value: bool) -> Self,
        Integer(value: i32) -> Self,
        Double(value: f64) -> Self,
        Symbol(value: Interned) -> Self,
        BigInteger(value: Gc<BigInt>) -> Self,
        String(value: Gc<String>) -> Self,
    );

    /// Returns a new array value.
    #[inline(always)]
    pub fn new_array(value: Gc<VecValue>) -> Self {
        BaseValue::new(ARRAY_TAG, u64::from(value)).into()
    }
    /// Returns a new block value.
    #[inline(always)]
    pub fn new_block(value: Gc<Block>) -> Self {
        BaseValue::new(BLOCK_TAG, u64::from(value)).into()
    }
    /// Returns a new class value.
    #[inline(always)]
    pub fn new_class(value: Gc<Class>) -> Self {
        BaseValue::new(CLASS_TAG, u64::from(value)).into()
    }
    /// Returns a new instance value.
    #[inline(always)]
    pub fn new_instance(value: Gc<Instance>) -> Self {
        BaseValue::new(INSTANCE_TAG, u64::from(value)).into()
    }
    /// Returns a new invocable value.
    #[inline(always)]
    pub fn new_invokable(value: Gc<MethodOrPrim>) -> Self {
        BaseValue::new(INVOKABLE_TAG, u64::from(value)).into()
    }

    /// Returns whether this value is an array.
    #[inline(always)]
    pub fn is_array(self) -> bool {
        self.tag() == ARRAY_TAG
    }
    /// Returns whether this value is a block.
    #[inline(always)]
    pub fn is_block(self) -> bool {
        self.tag() == BLOCK_TAG
    }
    /// Returns whether this value is a class.
    #[inline(always)]
    pub fn is_class(self) -> bool {
        self.tag() == CLASS_TAG
    }
    /// Returns whether this value is an instance.
    #[inline(always)]
    pub fn is_instance(self) -> bool {
        self.tag() == INSTANCE_TAG
    }
    /// Returns whether this value is an invocable.
    #[inline(always)]
    pub fn is_invocable(self) -> bool {
        self.tag() == INVOKABLE_TAG
    }

    /// Returns this value as an array, if such is its type.
    #[inline(always)]
    pub fn as_array(self) -> Option<Gc<VecValue>> {
        self.is_array().then(|| self.extract_gc_cell())
    }
    /// Returns this value as a block, if such is its type.
    #[inline(always)]
    pub fn as_block(self) -> Option<Gc<Block>> {
        self.is_block().then(|| self.extract_gc_cell())
    }

    /// Returns this value as a class, if such is its type.
    #[inline(always)]
    pub fn as_class(self) -> Option<Gc<Class>> {
        self.is_class().then(|| self.extract_gc_cell())
    }
    /// Returns this value as an instance, if such is its type.
    #[inline(always)]
    pub fn as_instance(self) -> Option<Gc<Instance>> {
        self.is_instance().then(|| self.extract_gc_cell())
    }
    /// Returns this value as an invocable, if such is its type.
    #[inline(always)]
    pub fn as_invokable(self) -> Option<Gc<MethodOrPrim>> {
        self.is_invocable().then(|| self.extract_gc_cell())
    }

    /// Get the class of the current value.
    #[inline(always)]
    pub fn class(&self, universe: &Universe) -> Gc<Class> {
        match self.tag() {
            NIL_TAG => universe.nil_class(),
            SYSTEM_TAG => universe.system_class(),
            BOOLEAN_TAG => {
                if self.as_boolean().unwrap() {
                    universe.true_class()
                } else {
                    universe.false_class()
                }
            }
            INTEGER_TAG | BIG_INTEGER_TAG => universe.integer_class(),
            SYMBOL_TAG => universe.symbol_class(),
            STRING_TAG => universe.string_class(),
            ARRAY_TAG => universe.array_class(),
            BLOCK_TAG => self.as_block().unwrap().class(universe),
            INSTANCE_TAG => self.as_instance().unwrap().class(),
            CLASS_TAG => self.as_class().unwrap().class(),
            INVOKABLE_TAG => self.as_invokable().unwrap().class(universe),
            _ => {
                if self.is_double() {
                    universe.double_class()
                } else {
                    panic!("unknown tag");
                }
            }
        }
    }

    /// Search for a given method for this value.
    pub fn lookup_method(&self, universe: &Universe, signature: Interned) -> Option<Gc<MethodOrPrim>> {
        self.class(universe).lookup_method(signature)
    }

    /// Get the string representation of this value.
    pub fn to_string(&self, universe: &Universe) -> String {
        match self.tag() {
            NIL_TAG => "nil".to_string(),
            SYSTEM_TAG => "system".to_string(),
            BOOLEAN_TAG => self.as_boolean().unwrap().to_string(),
            INTEGER_TAG => self.as_integer().unwrap().to_string(),
            BIG_INTEGER_TAG => self.as_big_integer::<Gc<BigInt>>().unwrap().to_string(),
            _ if self.is_double() => self.as_double().unwrap().to_string(),
            SYMBOL_TAG => {
                let symbol = universe.lookup_symbol(self.as_symbol().unwrap());
                if symbol.chars().any(|ch| ch.is_whitespace() || ch == '\'') {
                    format!("#'{}'", symbol.replace("'", "\\'"))
                } else {
                    format!("#{}", symbol)
                }
            }
            STRING_TAG => self.as_string::<Gc<String>>().unwrap().to_string(),
            ARRAY_TAG => {
                // TODO: I think we can do better here (less allocations).
                let strings: Vec<String> = self.as_array().unwrap().0.iter().map(|value| value.to_string(universe)).collect();
                format!("#({})", strings.join(" "))
            }
            BLOCK_TAG => {
                let block = self.as_block().unwrap();
                format!("instance of Block{}", block.nb_parameters() + 1)
            }
            INSTANCE_TAG => {
                let instance = self.as_instance().unwrap();
                format!("instance of {} class", instance.class().name(),)
            }
            CLASS_TAG => self.as_class().unwrap().name().to_string(),
            INVOKABLE_TAG => {
                let invokable = self.as_invokable().unwrap();
                format!("{}>>#{}", invokable.holder().name(), invokable.signature(),)
            }
            _ => {
                panic!("unknown tag")
            }
        }
    }
}

// for backwards compatibility with current code... and maybe easy replacement with ValueEnum?
#[allow(non_snake_case)]
impl Value {
    #[inline(always)]
    pub fn Array(value: Gc<VecValue>) -> Self {
        Value::new_array(value)
    }

    #[inline(always)]
    pub fn Block(value: Gc<Block>) -> Self {
        Value::new_block(value)
    }

    #[inline(always)]
    pub fn Class(value: Gc<Class>) -> Self {
        Value::new_class(value)
    }

    #[inline(always)]
    pub fn Instance(value: Gc<Instance>) -> Self {
        Value::new_instance(value)
    }

    #[inline(always)]
    pub fn Invokable(value: Gc<MethodOrPrim>) -> Self {
        Value::new_invokable(value)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.as_u64() == other.as_u64() {
            // this encapsulates every comparison between values of the same primitive type, e.g. comparing two i32s or two booleans, and pointer comparisons
            true
        } else if let (Some(a), Some(b)) = (self.as_double(), other.as_double()) {
            a == b
        } else if let (Some(a), Some(b)) = (self.as_integer(), other.as_double()) {
            (a as f64) == b
        } else if let (Some(a), Some(b)) = (self.as_double(), other.as_integer()) {
            (b as f64) == a
        } else if let (Some(a), Some(b)) = (self.as_big_integer::<Gc<BigInt>>(), other.as_big_integer()) {
            a == b
        } else if let (Some(a), Some(b)) = (self.as_big_integer::<Gc<BigInt>>(), other.as_integer()) {
            (*a).eq(&BigInt::from(b))
        } else if let (Some(a), Some(b)) = (self.as_integer(), other.as_big_integer::<Gc<BigInt>>()) {
            BigInt::from(a).eq(&*b)
        } else if let (Some(a), Some(b)) = (self.as_string::<Gc<String>>(), other.as_string::<Gc<String>>()) {
            a == b
        } else {
            false
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        ValueEnum::from(*self).fmt(f)
    }
}
