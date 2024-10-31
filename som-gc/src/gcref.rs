use crate::gc_interface::GCInterface;
use mmtk::util::Address;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// A pointer to the heap for GC.
#[derive(Debug)]
#[repr(transparent)]
pub struct GCRef<T> {
    pub ptr: Address,
    pub _phantom: PhantomData<T>,
}

impl<T> Clone for GCRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for GCRef<T> {}

// Ugly, but sometimes we want a placeholder. Code may be refactorable to never need this though, I think.
impl<T> Default for GCRef<T> {
    fn default() -> Self {
        unsafe {
            GCRef {
                ptr: Address::from_usize(0),
                _phantom: PhantomData,
            }
        }
    }
}

impl<T> PartialEq for GCRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> Deref for GCRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        debug_assert!(!self.ptr.is_zero());
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for GCRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(!self.ptr.is_zero());
        unsafe { self.ptr.as_mut_ref() }
    }
}

impl<T> GCRef<T> {
    /// Does the address not point to any data?
    /// We use this to avoid using an Option type in interpreter frames. Not sure if it's worth it though.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ptr.is_zero()
    }

    /// Convert a u64 into an address. Useful since we use NaN boxing, which returns values as 64 bits.
    #[inline(always)]
    pub fn from_u64(ptr: u64) -> GCRef<T> {
        unsafe {
            GCRef {
                ptr: Address::from_usize(ptr as usize),
                _phantom: PhantomData,
            }
        }
    }
}

/// Custom alloc function.
///
/// Exists for that traits to be able to choose how to allocate their data.
/// Must call GCRef::<T>::alloc(_with_size) internally to get a GCRef, but I can't strictly enforce that with Rust's type system.
/// In practice, that's usually allowing for more memory than Rust might be able to infer from the struct size, and filling it with our own data.
pub trait CustomAlloc<T> {
    fn alloc(obj: T, mutator: &mut GCInterface) -> GCRef<T>;
}
