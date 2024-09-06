use std::marker::PhantomData;
use mmtk::Mutator;
use mmtk::util::Address;
use som_gc::SOMVM;

/// A pointer to the heap for GC.
#[derive(Debug)]
pub struct GCRef<T> {
    pub ptr: Address,
    pub _phantom: PhantomData<T>
}

impl<T> Clone for GCRef<T> {
    fn clone(&self) -> Self {
        GCRef {
            ptr: self.ptr,
            _phantom: self._phantom
        }
    }
}

impl<T> Copy for GCRef<T> {}

// Ugly, but sometimes we want a placeholder. Code may be refactorable to never need this though.
impl<T> Default for GCRef<T> {
    fn default() -> Self {
        unsafe {
            GCRef {
                ptr: Address::from_usize(0),
                _phantom: PhantomData::default()
            }
        }
    }
}

impl<T> PartialEq for GCRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> GCRef<T> {
    // Turn a GC pointer back into the type itself (as a reference)
    pub fn to_obj(&self) -> &mut T {
        debug_assert!(!self.ptr.is_zero());
        unsafe { &mut *(self.ptr.as_mut_ref()) }
    }
}

/// Trait used by all GCRef pointers to convert to/from objects. TODO should only be implementable by GCRef<T>, and MUST be implemented by all GCRef<T>
pub trait Alloc<T> {
    // Allocates a type on the heap and returns a pointer to it
    fn alloc(obj: T, mutator: &mut Mutator<SOMVM>) -> GCRef<T>;
}