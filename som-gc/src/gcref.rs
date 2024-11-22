use mmtk::util::Address;
use std::fmt::{Debug, Formatter};

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// A pointer to the heap for GC.
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

impl<T: Debug> Debug for Gc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match !self.is_empty() {
            true => (**self).fmt(f),
            false => f.write_str("(empty)"),
        }
    }
}

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
            #[cfg(feature = "strategy-semispace")]
            {
                // checking we're not holding onto references from the old space
                let gc_interface = &**crate::MUTATOR_WRAPPER.get().unwrap();
                if gc_interface.get_nbr_collections() % 2 == 1 && self.ptr.to_string().chars().nth(0).unwrap() == '2' {
                    dbg!(ptr as usize);
                    dbg!("BREAKPOINT");
                }
            }
            &*ptr
        }
    }
}

impl<T> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ptr = self.ptr as *mut T;
            #[cfg(feature = "strategy-semispace")]
            {
                // checking we're not holding onto references from the old space
                let gc_interface = &**crate::MUTATOR_WRAPPER.get().unwrap();
                if gc_interface.get_nbr_collections() % 2 == 1 && self.ptr.to_string().chars().nth(0).unwrap() == '2' {
                    dbg!(ptr as usize);
                    dbg!("BREAKPOINT");
                }
            }
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

impl<T> From<u64> for Gc<T> {
    fn from(ptr: u64) -> Self {
        debug_assert!(ptr != 0);
        Gc {
            ptr: ptr as usize,
            _phantom: PhantomData,
        }
    }
}

/// Convert an MMTk address into a GCRef.
impl<T> From<Address> for Gc<T> {
    fn from(ptr: Address) -> Self {
        Gc {
            ptr: ptr.as_usize(),
            _phantom: PhantomData,
        }
    }
}

impl<T> Gc<T> {
    /// Checks if a frame is "empty", i.e. contains the default value
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ptr == GCREF_EMPTY_VALUE
    }

    /// Get a const pointer to the underlying data.
    pub fn to_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    /// Get a mutable pointer to the underlying data.
    pub fn to_mut_ptr(&self) -> *mut T {
        self.ptr as *mut T
    }

    // /// Return a mutable pointer to the underlying data as an arbitrary type.
    // /// Usage discouraged, and would be better off going unused entirely.
    // pub unsafe fn unsafe_cast<U>(&self) -> *mut U {
    //     self.ptr as *mut U
    // }
}
