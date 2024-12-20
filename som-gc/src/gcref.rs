use mmtk::util::Address;
use std::fmt::{Debug, Formatter};

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[macro_export]
macro_rules! debug_assert_valid_semispace_ptr {
    ($self:expr) => {
        // #[cfg(all(feature = "semispace", debug_assertions))]
        #[cfg(debug_assertions)]
        assert!($self.is_pointer_to_valid_space(), "Pointer to invalid space.");
    };
}

#[macro_export]
macro_rules! debug_assert_valid_semispace_ptr_value {
    ($value:expr) => {
        #[cfg(debug_assertions)]
        unsafe {
            if let Some(ptr) = $value.0.as_something::<Gc<()>>() {
                assert!(ptr.is_pointer_to_valid_space(), "Pointer to invalid space.");
            }
        }
    };
}

/// A pointer to the heap for GC.
///
/// To note: it could be a `NonNull` instead, and have places that need a "default" value rely on Option<Gc<T>>.
/// That might be a mild speedup for all I know.
#[repr(transparent)]
pub struct Gc<T> {
    pub ptr: usize, // TODO: fine as is, but I'd rather not have the underlying pointer be public, and support arithmetic operations on Gc<T> directly.
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
            debug_assert_valid_semispace_ptr!(self);

            let ptr = self.ptr as *const T;
            &*ptr
        }
    }
}

impl<T> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            debug_assert_valid_semispace_ptr!(self);

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

impl<T> From<u64> for Gc<T> {
    fn from(ptr: u64) -> Self {
        debug_assert!(ptr != 0);
        Gc {
            ptr: ptr as usize,
            _phantom: PhantomData,
        }
    }
}

/// Convert an MMTk address into a GC pointer.
impl<T> From<Address> for Gc<T> {
    fn from(ptr: Address) -> Self {
        Gc {
            ptr: ptr.as_usize(),
            _phantom: PhantomData,
        }
    }
}

impl<T> From<&Gc<T>> for Address {
    fn from(ptr: &Gc<T>) -> Self {
        Address::from_ref(ptr)
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

    #[cfg(feature = "semispace")]
    /// Checks if the pointer points to a valid space.
    /// Because of our semispace GC, pointers can move from one space to the other, and the number one bug cause is pointers not having been moved.
    /// So this function is tremendously useful for debugging.
    pub fn is_pointer_to_valid_space(&self) -> bool {
        fn leftmost_digit(mut number: usize) -> u8 {
            while number >= 10 {
                number /= 10;
            }
            number as u8
        }

        let gc_interface = unsafe { &**crate::MUTATOR_WRAPPER.get().unwrap() };

        // if we're collecting, we're handling both new and old pointers, so we just say they're all valid for simplicity.
        if gc_interface.is_currently_collecting() {
            return true;
        }

        match gc_interface.get_nbr_collections() % 2 == 0 {
            true => leftmost_digit(self.ptr) == 2,
            false => leftmost_digit(self.ptr) == 4,
        }
    }
}
