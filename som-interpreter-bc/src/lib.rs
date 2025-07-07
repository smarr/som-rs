//!
//! This is the interpreter for the Simple Object Machine.
//!

use crate::interpreter::Interpreter;
use crate::universe::Universe;
use std::sync::atomic::AtomicPtr;

/// VM objects.
pub mod vm_objects;

/// Facilities for compiling code into bytecode.
pub mod compiler;
/// Facilities for manipulating values.
pub mod hashcode;
/// The interpreter's main data structure.
pub mod interpreter;
/// Definitions for all supported primitives.
pub mod primitives;
/// The collection of all known SOM objects during execution.
pub mod universe;
/// Facilities for manipulating values.
pub mod value;

/// Structs and info related to interacting with the GC
pub mod gc;

/// Used for debugging.
pub mod debug;

/// Raw pointer needed to trace GC roots. Meant to be accessed only non-mutably, hence the "CONST" in the name.
pub static UNIVERSE_RAW_PTR_CONST: AtomicPtr<Universe> = AtomicPtr::new(std::ptr::null_mut());

/// See `UNIVERSE_RAW_PTR_CONST`.
pub static INTERPRETER_RAW_PTR_CONST: AtomicPtr<Interpreter> = AtomicPtr::new(std::ptr::null_mut());
