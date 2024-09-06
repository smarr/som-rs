use std::marker::PhantomData;
use mmtk::{AllocationSemantics, Mutator};
use mmtk::util::Address;
use som_gc::api::{mmtk_alloc, mmtk_post_alloc};
use som_gc::SOMVM;
use core::mem::size_of;

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

// Ugly, but sometimes we want a placeholder. Code may be refactorable to never need this though, I think.
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

// -------------------------------------

impl<T> GCRef<T> {
    /// Turn a GC pointer back into the type itself (as a reference)
    pub fn to_obj(&self) -> &mut T {
        debug_assert!(!self.ptr.is_zero());
        unsafe { &mut *(self.ptr.as_mut_ref()) }
    }
}

/// Trait used by GCRef pointers to be created from objects.
pub trait Alloc<T> {
    // Allocates a type on the heap and returns a pointer to it
    fn alloc(obj: T, mutator: &mut Mutator<SOMVM>) -> GCRef<T>;
}

impl<T> Alloc<T> for GCRef<T> {
    /// A normal, straightforward alloc. Structures can implement their own instead (e.g. Instance and its arbitrary field array size) 
     fn alloc(obj: T, mutator: &mut Mutator<SOMVM>) -> GCRef<T> {
        let size = size_of::<T>();
        let align= 8;
        let offset= 0;
        let semantics = AllocationSemantics::Default;

        let addr = mmtk_alloc(mutator, size, align, offset, semantics);
        debug_assert!(!addr.is_zero());

        mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, semantics);

        unsafe {
            *addr.as_mut_ref() = obj;
        }

        GCRef {
            ptr: addr,
            _phantom: PhantomData::default()
        }
    }
}

// for convenience, but removable
impl GCRef<String> {
    pub fn as_str(&self) -> &str {
        self.to_obj().as_str()
    }
}