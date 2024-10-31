//!
//! This is the interpreter for the Simple Object Machine.
//!

use crate::universe::Universe;

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

/// Specialized AST nodes
pub mod specialized;
mod convert;
/// Facilities for manipulating values.
pub mod value;
/// To interact with the GC.
pub mod gc;

pub static mut UNIVERSE_RAW_PTR: *mut Universe = std::ptr::null_mut();
