//!
//! This is the interpreter for the Simple Object Machine.
//!

use crate::universe::Universe;
use crate::value::Value;
use std::ptr::NonNull;

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
/// Inlining specific messages, such as control flow.
pub mod inliner;

/// Facilities for manipulating blocks.
pub mod block;
/// Facilities for manipulating classes.
pub mod class;
/// Facilities for evaluating nodes and expressions.
pub mod evaluate;
/// Facilities for manipulating stack frames.
pub mod frame;
/// Facilities for manipulating values.
pub mod hashcode;
/// Facilities for manipulating class instances.
pub mod instance;
/// Facilities for invoking methods and/or primitives.
pub mod invokable;
/// Facilities for manipulating class methods.
pub mod method;
/// Definitions for all supported primitives.
pub mod primitives;
/// The interpreter's main data structure.
pub mod universe;

mod convert;
/// To interact with the GC.
pub mod gc;
/// Specialized AST nodes
pub mod specialized;
/// Facilities for manipulating values.
pub mod value;

/// Raw pointer needed to trace GC roots. Meant to be accessed only non-mutably, hence the "CONST" in the name.
pub static mut UNIVERSE_RAW_PTR_CONST: Option<NonNull<Universe>> = None;

// When GC triggers while we're allocating a frame, the arguments we want to add to that frame are being passed as an argument to the frame allocation function.
// This means the GC does NOT know how to reach them, and we have to inform it ourselves... So whenever we allocate a frame, we store a pointer to its arguments before that.
// It's possible this isn't just an issue when allocating frames, and that we need argument pointers to other values being initialized when we trigger GC. But I assume, and hope, not.
// It's not very pretty. But I'm not sure how else to fix it at the moment.
pub static mut FRAME_ARGS_PTR: Option<NonNull<Vec<Value>>> = None;
