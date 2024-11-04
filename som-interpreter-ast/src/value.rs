use std::convert::TryInto;
use std::fmt;

use crate::block::Block;
use crate::class::Class;
use crate::gc::VecValue;
use crate::instance::Instance;
use crate::method::Method;
use crate::universe::Universe;
use num_bigint::BigInt;
use som_core::interner::Interned;
use som_core::nan_boxed_val_base_impl;
use som_core::value::{CELL_BASE_TAG, INTEGER_TAG, BIG_INTEGER_TAG, STRING_TAG, SYMBOL_TAG, BOOLEAN_TAG, CANON_NAN_BITS, NIL_TAG, SYSTEM_TAG};
use som_core::value::{IS_PTR_PATTERN, TAG_SHIFT, TAG_EXTRACTION};
use som_gc::gcref::GCRef;

// The following non-pointer type tags are still available (maybe useful for optimisations ?):

// /// Tag bits for the `???` type.
// const RESERVED1_TAG: u64 = 0b110 | BASE_TAG;
// /// Tag bits for the `???` type.
// const RESERVED2_TAG: u64 = 0b111 | BASE_TAG;

// Tags for pointer types

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

pub type Value = AstNaNBoxedVal;
// pub type Value = ValueEnum;

/// Represents an SOM value.
#[derive(Clone, Copy, Eq, Hash)]
pub struct AstNaNBoxedVal {
    /// The 64-bit value that is used to store SOM values using NaN-boxing.
    encoded: u64,
}

nan_boxed_val_base_impl!(AstNaNBoxedVal);

impl AstNaNBoxedVal {
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

    // `is_*` methods for pointer types

    // `as_*` for pointer types
    
    /// Returns this value as an array, if such is its type.
    #[inline(always)]
    pub fn as_array(self) -> Option<GCRef<VecValue>> {
        self.is_array().then(|| self.extract_gc_cell())
    }
    /// Returns this value as a block, if such is its type.
    #[inline(always)]
    pub fn as_block(self) -> Option<GCRef<Block>> {
        self.is_block().then(|| self.extract_gc_cell())
    }

    /// Returns this value as a class, if such is its type.
    #[inline(always)]
    pub fn as_class(self) -> Option<GCRef<Class>> {
        self.is_class().then(|| self.extract_gc_cell())
    }
    /// Returns this value as an instance, if such is its type.
    #[inline(always)]
    pub fn as_instance(self) -> Option<GCRef<Instance>> {
        self.is_instance().then(|| self.extract_gc_cell())
    }
    /// Returns this value as an invocable, if such is its type.
    #[inline(always)]
    pub fn as_invokable(self) -> Option<GCRef<Method>> {
        self.is_invocable().then(|| self.extract_gc_cell())
    }

    // `as_*` for non pointer types
    
    /// Returns the value as a boolean, but without checking if it actually is one.
    #[inline(always)]
    pub fn as_boolean_unchecked(self) -> bool {
        self.payload() != 0
    }
    
    /// Returns a new array value.
    #[inline(always)]
    pub fn new_array(value: GCRef<VecValue>) -> Self {
        Self::new(ARRAY_TAG, value.ptr.as_usize().try_into().unwrap())
    }
    /// Returns a new block value.
    #[inline(always)]
    pub fn new_block(value: GCRef<Block>) -> Self {
        Self::new(BLOCK_TAG, value.ptr.as_usize().try_into().unwrap())
    }
    /// Returns a new class value.
    #[inline(always)]
    pub fn new_class(value: GCRef<Class>) -> Self {
        Self::new(CLASS_TAG, value.ptr.as_usize().try_into().unwrap())
    }
    /// Returns a new instance value.
    #[inline(always)]
    pub fn new_instance(value: GCRef<Instance>) -> Self {
        Self::new(INSTANCE_TAG, value.ptr.as_usize().try_into().unwrap())
    }
    /// Returns a new invocable value.
    #[inline(always)]
    pub fn new_invokable(value: GCRef<Method>) -> Self {
        Self::new(INVOKABLE_TAG, value.ptr.as_usize().try_into().unwrap())
    }

    // #[inline(always)]
    // fn extract_gc_cell<T: Trace>(self) -> GCRef<T> {
    //     let ptr: *const GcBox<T> = self.extract_pointer::<GcBox<T>>();
    //     let ptr = NonNull::new(ptr as *mut _).unwrap();
    //     Gc::from_raw(ptr)
    // }

    /// Get the class of the current value.
    #[inline(always)]
    pub fn class(&self, universe: &Universe) -> GCRef<Class> {
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
    pub fn lookup_method(&self, universe: &Universe, signature: &str) -> Option<GCRef<Method>> {
        self.class(universe).lookup_method(signature)
    }

    /// Search for a local binding within this value.
    pub fn lookup_local(&self, idx: u8) -> Self {
        if let Some(instance) = self.as_instance() {
            instance.lookup_local(idx)
        } else if let Some(class) = self.as_class() {
            class.lookup_field(idx)
        } else {
            panic!("looking up a local not from an instance or a class")
        }
    }

    /// Assign a value to a local binding within this value.
    pub fn assign_local(&mut self, idx: u8, value: Self) -> Option<()> {
        if let Some(mut instance) = self.as_instance() {
            Some(instance.assign_local(idx, value))
        } else if let Some(mut class) = self.as_class() {
            Some(class.assign_field(idx, value))
        } else {
            None
        }
    }

    /// Get the string representation of this value.
    pub fn to_string(&self, universe: &Universe) -> String {
        match self.tag() {
            NIL_TAG => "nil".to_string(),
            SYSTEM_TAG => "system".to_string(),
            BOOLEAN_TAG => self.as_boolean().unwrap().to_string(),
            INTEGER_TAG => self.as_integer().unwrap().to_string(),
            BIG_INTEGER_TAG => self.as_big_integer().unwrap().to_string(),
            _ if self.is_double() => self.as_double().unwrap().to_string(),
            SYMBOL_TAG => {
                let symbol = universe.lookup_symbol(self.as_symbol().unwrap());
                if symbol.chars().any(|ch| ch.is_whitespace() || ch == '\'') {
                    format!("#'{}'", symbol.replace("'", "\\'"))
                } else {
                    format!("#{}", symbol)
                }
            }
            STRING_TAG => self.as_string().unwrap().to_string(),
            ARRAY_TAG => {
                // TODO: I think we can do better here (less allocations).
                let strings: Vec<String> = self
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.to_string(universe))
                    .collect();
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
                format!("{}>>#{}", invokable.holder.name(), invokable.signature(),)
            }
            _ => {
                panic!("unknown tag")
            }
        }
    }

    // pub fn dbg_get_bits(&self) -> u64 {
    //     self.encoded
    // }
}

// TODO: remove all these. it's for backwards compatibility (i.e.: i don't want to do massive amounts of refactoring)
#[allow(non_snake_case)]
impl AstNaNBoxedVal {
    #[inline(always)]
    pub fn Array(value: GCRef<VecValue>) -> Self {
        AstNaNBoxedVal::new_array(value)
    }

    #[inline(always)]
    pub fn Block(value: GCRef<Block>) -> Self {
        AstNaNBoxedVal::new_block(value)
    }

    #[inline(always)]
    pub fn Class(value: GCRef<Class>) -> Self {
        AstNaNBoxedVal::new_class(value)
    }

    #[inline(always)]
    pub fn Instance(value: GCRef<Instance>) -> Self {
        AstNaNBoxedVal::new_instance(value)
    }

    #[inline(always)]
    pub fn Invokable(value: GCRef<Method>) -> Self {
        AstNaNBoxedVal::new_invokable(value)
    }
}

impl From<AstNaNBoxedVal> for ValueEnum {
    fn from(value: AstNaNBoxedVal) -> Self {
        if let Some(value) = value.as_double() {
            Self::Double(value)
        } else if value.is_nil() {
            Self::Nil
        } else if value.is_system() {
            Self::System
        } else if let Some(value) = value.as_integer() {
            Self::Integer(value)
        } else if let Some(value) = value.as_big_integer() {
            Self::BigInteger(value)
        } else if let Some(value) = value.as_boolean() {
            Self::Boolean(value)
        } else if let Some(value) = value.as_symbol() {
            Self::Symbol(value)
        } else if let Some(value) = value.as_string() {
            Self::String(value)
        } else if let Some(_value) = value.as_array() {
            // Self::Array(value)
            eprintln!("no From<NanBoxedVal> impl for arr. returning Nil.");
            Self::Nil
        } else if let Some(value) = value.as_block() {
            Self::Block(value)
        } else if let Some(value) = value.as_instance() {
            Self::Instance(value)
        } else if let Some(value) = value.as_class() {
            Self::Class(value)
        } else if let Some(value) = value.as_invokable() {
            Self::Invokable(value)
        } else {
            todo!()
        }
    }
}

impl From<ValueEnum> for AstNaNBoxedVal {
    fn from(value: ValueEnum) -> Self {
        match value {
            ValueEnum::Nil => Self::NIL,
            ValueEnum::System => Self::SYSTEM,
            ValueEnum::Boolean(value) => Self::new_boolean(value),
            ValueEnum::Integer(value) => Self::new_integer(value),
            ValueEnum::BigInteger(value) => Self::new_big_integer(value),
            ValueEnum::Double(value) => Self::new_double(value),
            ValueEnum::Symbol(value) => Self::new_symbol(value),
            ValueEnum::String(value) => Self::new_string(value),
            // ValueEnum::Array(value) => Self::new_array(value),
            ValueEnum::Array(_value) => unimplemented!("no impl for arr, same as BC"),
            ValueEnum::Block(value) => Self::new_block(value),
            ValueEnum::Instance(value) => Self::new_instance(value),
            ValueEnum::Class(value) => Self::new_class(value),
            ValueEnum::Invokable(value) => Self::new_invokable(value),
        }
    }
}

impl fmt::Debug for AstNaNBoxedVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ValueEnum::from(*self).fmt(f)
    }
}

/// Represents an SOM value.
#[derive(Clone)]
pub enum ValueEnum {
    /// The **nil** value.
    Nil,
    /// The **system** value.
    System,
    /// A boolean value (**true** or **false**).
    Boolean(bool),
    /// An integer value.
    Integer(i32),
    /// A big integer value (arbitrarily big).
    BigInteger(GCRef<BigInt>),
    /// An floating-point value.
    Double(f64),
    /// An interned symbol value.
    Symbol(Interned),
    /// A string value.
    String(GCRef<String>),
    /// An array of values.
    Array(GCRef<Vec<AstNaNBoxedVal>>),
    /// A block value, ready to be evaluated.
    Block(GCRef<Block>),
    /// A generic (non-primitive) class instance.
    Instance(GCRef<Instance>),
    /// A bare class object.
    Class(GCRef<Class>),
    /// A bare invokable.
    Invokable(GCRef<Method>),
}

impl ValueEnum {
    /// Get the class of the current value.
    pub fn class(&self, universe: &Universe) -> GCRef<Class> {
        match self {
            Self::Nil => universe.nil_class(),
            Self::System => universe.system_class(),
            Self::Boolean(true) => universe.true_class(),
            Self::Boolean(false) => universe.false_class(),
            Self::Integer(_) => universe.integer_class(),
            Self::BigInteger(_) => universe.integer_class(),
            Self::Double(_) => universe.double_class(),
            Self::Symbol(_) => universe.symbol_class(),
            Self::String(_) => universe.string_class(),
            Self::Array(_) => universe.array_class(),
            Self::Block(block) => block.class(universe),
            Self::Instance(instance_ptr) => instance_ptr.class(),
            Self::Class(class) => class.class(),
            Self::Invokable(invokable) => invokable.class(universe),
        }
    }

    /// Search for a given method for this value.
    #[inline(always)]
    pub fn lookup_method(&self, universe: &Universe, signature: &str) -> Option<GCRef<Method>> {
        self.class(universe).lookup_method(signature)
    }

    /// Search for a local binding within this value.
    #[inline(always)]
    pub fn lookup_local(&self, idx: u8) -> Option<Self> {
        match self {
            Self::Instance(instance_ptr) => Some(instance_ptr.lookup_local(idx).into()),
            Self::Class(class) => Some(class.lookup_field(idx).into()),
            v => unreachable!("Attempting to look up a local in {:?}", v),
        }
    }

    /// Assign a value to a local binding within this value.
    pub fn assign_local(&mut self, idx: u8, value: Self) {
        match self {
            Self::Instance(instance_ptr) => instance_ptr.assign_local(idx, value.into()),
            Self::Class(class) => class.assign_field(idx, value.into()),
            v => unreachable!("Attempting to assign a local in {:?}", v),
        }
    }

    /// Get the string representation of this value.
    pub fn to_string(&self, universe: &Universe) -> String {
        match self {
            Self::Nil => "nil".to_string(),
            Self::System => "system".to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::BigInteger(value) => value.to_string(),
            Self::Double(value) => value.to_string(),
            Self::Symbol(value) => {
                let symbol = universe.lookup_symbol(*value);
                if symbol.chars().any(|ch| ch.is_whitespace() || ch == '\'') {
                    format!("#'{}'", symbol.replace("'", "\\'"))
                } else {
                    format!("#{}", symbol)
                }
            }
            Self::String(value) => value.as_str().to_string(),
            Self::Array(values) => {
                // TODO (from nicolas): I think we can do better here (less allocations).
                let strings: Vec<String> = values
                    .iter()
                    .map(|value| value.to_string(universe))
                    .collect();
                format!("#({})", strings.join(" "))
            }
            Self::Block(block) => format!("instance of Block{}", block.nb_parameters() + 1),
            Self::Instance(instance_ptr) => {
                format!("instance of {} class", instance_ptr.class().name(),)
            }
            Self::Class(class) => class.name().to_string(),
            Self::Invokable(invokable) => {
                format!("{}>>#{}", invokable.holder().name(), invokable.signature())
            }
        }
    }
}

impl PartialEq for ValueEnum {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) | (Self::System, Self::System) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a.eq(b),
            (Self::Integer(a), Self::Integer(b)) => a.eq(b),
            (Self::Integer(a), Self::Double(b)) | (Self::Double(b), Self::Integer(a)) => {
                (*a as f64).eq(b)
            }
            (Self::Double(a), Self::Double(b)) => a.eq(b),
            (Self::BigInteger(a), Self::BigInteger(b)) => a.eq(b),
            (Self::BigInteger(a), Self::Integer(b)) | (Self::Integer(b), Self::BigInteger(a)) => {
                (&**a).eq(&BigInt::from(*b))
            }
            (Self::Symbol(a), Self::Symbol(b)) => a.eq(b),
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Instance(a), Self::Instance(b)) => a == b,
            (Self::Class(a), Self::Class(b)) => a == b,
            (Self::Block(a), Self::Block(b)) => a == b,
            (Self::Invokable(a), Self::Invokable(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for ValueEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => f.debug_tuple("Nil").finish(),
            Self::System => f.debug_tuple("System").finish(),
            Self::Boolean(val) => f.debug_tuple("Boolean").field(val).finish(),
            Self::Integer(val) => f.debug_tuple("Integer").field(val).finish(),
            Self::BigInteger(val) => f.debug_tuple("BigInteger").field(val).finish(),
            Self::Double(val) => f.debug_tuple("Double").field(val).finish(),
            Self::Symbol(val) => f.debug_tuple("Symbol").field(val).finish(),
            Self::String(val) => f.debug_tuple("String").field(val).finish(),
            Self::Array(val) => f.debug_tuple("Array").field(&val).finish(),
            Self::Block(val) => f.debug_tuple("Block").field(val).finish(),
            Self::Instance(val) => f.debug_tuple("Instance").field(&val).finish(),
            Self::Class(val) => f.debug_tuple("Class").field(&val).finish(),
            Self::Invokable(val) => {
                let signature = format!("{}>>#{}", val.holder.name(), val.signature());
                f.debug_tuple("Invokable").field(&signature).finish()
            }
        }
    }
}
