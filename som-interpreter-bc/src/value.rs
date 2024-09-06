use std::fmt;
use std::rc::Rc;

use num_bigint::BigInt;

use crate::block::Block;
use crate::class::Class;
use crate::gc::GCRef;
use crate::instance::Instance;
use crate::interner::Interned;
use crate::method::Method;
use crate::universe::UniverseBC;
use crate::SOMRef;

/// Represents an SOM value.
#[derive(Clone)]
pub enum Value {
    /// The **nil** value.
    Nil,
    /// The **system** value.
    System,
    /// A boolean value (**true** or **false**).
    Boolean(bool),
    /// An integer value.
    Integer(i64),
    /// A big integer value (arbitrarily big).
    BigInteger(BigInt),
    /// An floating-point value.
    Double(f64),
    /// An interned symbol value.
    Symbol(Interned),
    /// A string value.
    String(Rc<String>),
    /// An array of values.
    Array(SOMRef<Vec<Self>>),
    /// A block value, ready to be evaluated.
    Block(Rc<Block>),
    /// A generic (non-primitive) class instance.
    Instance(GCRef<Instance>),
    /// A bare class object.
    Class(GCRef<Class>),
    /// A bare invokable.
    Invokable(Rc<Method>),
}

impl Value {
    /// Get the class of the current value.
    pub fn class(&self, universe: &UniverseBC) -> GCRef<Class> {
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
            Self::Instance(instance_ptr) => instance_ptr.to_obj().class(),
            Self::Class(class) => class.to_obj().class(),
            Self::Invokable(invokable) => invokable.class(universe),
        }
    }

    /// Search for a given method for this value.
    pub fn lookup_method(&self, universe: &UniverseBC, signature: Interned) -> Option<Rc<Method>> {
        self.class(universe).to_obj().lookup_method(signature)
    }

    /// Search for a local binding within this value.
    pub fn lookup_local(&self, idx: usize) -> Self {
        match self {
            Self::Instance(instance_ptr) => instance_ptr.lookup_local(idx),
            Self::Class(class) => class.to_obj().lookup_local(idx),
            v => unreachable!("Attempting to look up a local in {:?}", v),
        }
    }

    /// Assign a value to a local binding within this value.
    pub fn assign_local(&mut self, idx: usize, value: Self) {
        match self {
            Self::Instance(instance_ptr) => instance_ptr.assign_local(idx, value),
            Self::Class(class) => class.to_obj().assign_local(idx, value),
            v => unreachable!("Attempting to assign a local in {:?}", v),
        }
    }
    
    /// Checks if a value has a local variable (field) at the given index. Used by the instVarAt and instVarAtPut primitives.
    /// Basically, we want normal field lookups/assignments to not be able to fail (through unsafe) to be fast, since we know the bytecode we emitted that needs them is sound.
    /// But those prims are free to be used and abused by devs, so they CAN fail, and we need to check that they won't fail before we invoke them. Hence this `has_local`.
    pub fn has_local(&self, index: usize) -> bool {
        match self {
            Self::Instance(instance_ptr) => instance_ptr.to_obj().has_local(index),
            Self::Class(class) => class.to_obj().has_local(index),
            _ => false,
        }
    }

    /// Get the string representation of this value.
    pub fn to_string(&self, universe: &UniverseBC) -> String {
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
            Self::String(value) => value.to_string(),
            Self::Array(values) => {
                // TODO (from nicolas): I think we can do better here (less allocations).
                let strings: Vec<String> = values
                    .borrow()
                    .iter()
                    .map(|value| value.to_string(universe))
                    .collect();
                format!("#({})", strings.join(" "))
            }
            Self::Block(block) => format!("instance of Block{}", block.nb_parameters() + 1),
            Self::Instance(instance_ptr) => format!(
                "instance of {} class",
                instance_ptr.to_obj().class().to_obj().name(),
            ),
            Self::Class(class) => class.to_obj().name().to_string(),
            Self::Invokable(invokable) => {
                format!("{}>>#{}", invokable.holder().to_obj().name(), invokable.signature())
            }
        }
    }
}

impl PartialEq for Value {
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
                a.eq(&BigInt::from(*b))
            }
            (Self::Symbol(a), Self::Symbol(b)) => a.eq(b),
            (Self::String(a), Self::String(b)) => Rc::ptr_eq(a, b),
            (Self::Array(a), Self::Array(b)) => Rc::ptr_eq(a, b),
            (Self::Instance(a), Self::Instance(b)) => a == b,
            (Self::Class(a), Self::Class(b)) => a == b,
            (Self::Block(a), Self::Block(b)) => Rc::ptr_eq(a, b),
            (Self::Invokable(a), Self::Invokable(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Debug for Value {
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
            Self::Array(val) => f.debug_tuple("Array").field(&val.borrow()).finish(),
            Self::Block(val) => f.debug_tuple("Block").field(val).finish(),
            Self::Instance(val) => f.debug_tuple("Instance").field(&val.to_obj()).finish(),
            Self::Class(val) => f.debug_tuple("Class").field(&val.to_obj()).finish(),
            Self::Invokable(val) => {
                let signature = format!("{}>>#{}", val.holder.to_obj().name(), val.signature());
                f.debug_tuple("Invokable").field(&signature).finish()
            }
        }
    }
}
