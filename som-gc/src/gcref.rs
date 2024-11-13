use mmtk::util::Address;

use crate::gc_interface::GCInterface;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// A pointer to the heap for GC.
#[derive(Debug)]
#[repr(transparent)]
pub struct Gc<T> {
    pub ptr: usize,
    pub _phantom: PhantomData<T>,
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Gc<T> {}

const GCREF_EMPTY_VALUE: usize = 0;
// Occasionally we want a placeholder. Code definitely refactorable to never need this (we could just use `Option<GCRef>`), but it would likely be a minor perf hit.
impl<T> Default for Gc<T> {
    fn default() -> Self {
        Gc {
            ptr: GCREF_EMPTY_VALUE,
            _phantom: PhantomData,
        }
    }
}

impl<T> PartialEq for Gc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            let ptr = self.ptr as *const T;
            &*ptr
        }
    }
}

impl<T> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ptr = self.ptr as *mut T;
            &mut *ptr
        }
    }
}

impl<T> From<Gc<T>> for u64 {
    fn from(value: Gc<T>) -> Self {
        debug_assert!(!value.is_empty());
        value.ptr as u64
    }
}

impl<T> Gc<T> {
    /// Checks if a frame is "empty", i.e. contains the default value
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ptr == GCREF_EMPTY_VALUE
    }

    /// Convert an MMTk address into a GCRef.
    #[inline(always)]
    pub fn from_address(ptr: Address) -> Gc<T> {
        debug_assert!(!ptr.is_zero());
        Gc {
            ptr: ptr.as_usize(),
            _phantom: PhantomData,
        }
    }

    /// Convert a u64 into an address. Useful since we use NaN boxing, which returns values as 64 bits.
    #[inline(always)]
    pub fn from_u64(ptr: u64) -> Gc<T> {
        debug_assert!(ptr != 0);
        Gc {
            ptr: ptr as usize,
            _phantom: PhantomData,
        }
    }
}

/// Custom alloc function.
///
/// Exists for that traits to be able to choose how to allocate their data.
/// Must call GCRef::<T>::alloc(_with_size) internally to get a GCRef, but I can't strictly enforce that with Rust's type system.
/// In practice, that's usually allowing for more memory than Rust might be able to infer from the struct size, and filling it with our own data.
pub trait CustomAlloc<T> {
    fn alloc(obj: T, mutator: &mut GCInterface) -> Gc<T>;
}
