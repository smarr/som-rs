//!
//! This crate contains common types that are useful to be shared across multiple tools when manipulating SOM-related things.
//!

/// The SOM Abstract Syntax Tree definitions.
pub mod ast;
/// The SOM bytecode definitions.
pub mod bytecode;
// /// GC-related, mostly GC heap references/allocation handling.
// pub mod gc;
/// Facilities for string interning.
pub mod interner;
