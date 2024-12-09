//!
//! This is the interpreter for the Simple Object Machine.
//!

use crate::universe::Universe;
use crate::value::Value;
use std::sync::atomic::AtomicPtr;

macro_rules! propagate {
    ($expr:expr) => {
        match $expr {
            Return::Local(value) => value,
            ret => return ret,
        }
    };
}

/// AST specific to the AST interpreter
pub mod ast;
/// Generates the AST
pub mod compiler;
/// Facilities for evaluating nodes and expressions.
pub mod evaluate;
/// Facilities for manipulating values.
pub mod hashcode;
/// Facilities for invoking methods and/or primitives.
pub mod invokable;
/// Definitions for all supported primitives.
pub mod primitives;
/// The interpreter's main data structure.
pub mod universe;

/// VM-specific objects.
pub mod vm_objects;

mod convert;
/// To interact with the GC.
pub mod gc;
/// Specialized AST nodes
pub mod specialized;
/// Facilities for manipulating values.
pub mod value;
/// For values that are to pointer types.
pub mod value_ptr;

/// Raw pointer needed to trace GC roots. Meant to be accessed only non-mutably, hence the "CONST" in the name.
/// TODO: actually enforce that non-mutable access.
pub static UNIVERSE_RAW_PTR_CONST: AtomicPtr<Universe> = AtomicPtr::new(std::ptr::null_mut());
