//!
//! This is the interpreter for the Simple Object Machine.
//!

// TODO: remove we should NOT rely on static mutables.
// I thought it was justified, and I've got a decent use case, but that doesn't mean we shouldn't use some Rust wrapper like OnceLock<T>
#![allow(static_mut_refs)]

use crate::interpreter::Interpreter;
use crate::universe::Universe;
use crate::value::Value;
use som_gc::gcref::Gc;
use std::sync::atomic::AtomicPtr;
use vm_objects::class::Class;

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
pub static mut HACK_FRAME_FRAME_ARGS_PTR: Option<Vec<Value>> = None;

/// For instance initializations... we really need to pass pointers by references to primitives...
pub static mut HACK_INSTANCE_CLASS_PTR: Option<Gc<Class>> = None;
