//! Compiles parser AST to bytecode.
//! This module only needs to expose the compile_class() function: the rest of the VM should not
//! need access to more than that, barring testing.

use crate::vm_objects::block::Block;
use num_bigint::BigInt;
use som_core::interner::Interned;
use som_gc::gcref::{Gc, GcSlice};
use std::hash::{Hash, Hasher};

/// Facilities to compile code.
pub mod compile;

/// Inlining some calls to a select few builtin functions for sizeable perf gains.
mod inliner;

#[derive(Debug, Clone)]
pub enum Literal {
    Symbol(Interned),
    String(Gc<String>),
    Double(f64),
    Integer(i32),
    BigInteger(Gc<BigInt>),
    Array(GcSlice<u8>),
    Block(Gc<Block>),
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::Symbol(val1), Literal::Symbol(val2)) => val1.eq(val2),
            (Literal::String(val1), Literal::String(val2)) => val1.eq(val2),
            (Literal::Double(val1), Literal::Double(val2)) => val1.eq(val2),
            (Literal::Integer(val1), Literal::Integer(val2)) => val1.eq(val2),
            (Literal::BigInteger(val1), Literal::BigInteger(val2)) => val1.eq(val2),
            (Literal::Array(val1), Literal::Array(val2)) => val1.eq(val2),
            (Literal::Block(val1), Literal::Block(val2)) => val1 == val2,
            _ => false,
        }
    }
}

impl Eq for Literal {}

impl Hash for Literal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Literal::Symbol(val) => {
                state.write(b"sym#");
                val.hash(state);
            }
            Literal::String(val) => {
                state.write(b"string#");
                val.hash(state);
            }
            Literal::Double(val) => {
                state.write(b"dbl#");
                val.to_bits().hash(state);
            }
            Literal::Integer(val) => {
                state.write(b"int#");
                val.hash(state);
            }
            Literal::BigInteger(val) => {
                state.write(b"bigint#");
                val.hash(state);
            }
            Literal::Array(val) => {
                state.write(b"array#");
                for elem in val.iter() {
                    elem.hash(state)
                }
            }
            Literal::Block(val) => {
                state.write(b"blk");
                val.hash(state);
            }
        }
    }
}
