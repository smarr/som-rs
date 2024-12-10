//!
//! This crate contains common types that are useful to be shared across multiple tools when manipulating SOM-related things.
//!

/// The SOM Abstract Syntax Tree definitions: the common parser output.
pub mod ast;
/// The SOM bytecode definitions. Used only by the bytecode interpreter, so should maybe be moved.
pub mod bytecode;
/// The SOM core classes.
pub mod core_classes;
/// Facilities for string interning.
pub mod interner;
/// Shared value representation logic (NaN boxing really)
pub mod value;
/// Class for storing a value itself as a typed pointer.
pub mod value_ptr;
