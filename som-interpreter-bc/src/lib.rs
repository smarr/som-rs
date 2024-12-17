//!
//! This is the interpreter for the Simple Object Machine.
//!

use crate::interpreter::Interpreter;
use crate::universe::Universe;
use crate::value::Value;
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

/// Hack! at the moment, we pass a copied reference to a class' method when allocating a frame. When GC triggers from a frame allocation, that pointer isn't a root and doesn't get moved.
/// that one's the ugliest of hacks and we can definitely remove it somehow..
#[allow(static_mut_refs)]
pub static mut HACK_FRAME_FRAME_ARGS_PTR: Option<Vec<Value>> = None;
