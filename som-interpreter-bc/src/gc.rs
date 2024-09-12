use std::marker::PhantomData;
use mmtk::{AllocationSemantics, Mutator};
use mmtk::util::Address;
use som_gc::api::{mmtk_alloc, mmtk_post_alloc};
use som_gc::SOMVM;
use core::mem::size_of;

pub static GC_OFFSET: usize = 0;
pub static GC_ALIGN: usize = 8;
pub static GC_SEMANTICS: AllocationSemantics = AllocationSemantics::Default;

/// A pointer to the heap for GC.
#[derive(Debug)]
pub struct GCRef<T> {
    pub ptr: Address,
    pub _phantom: PhantomData<T>
}

impl<T> Clone for GCRef<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Copy for GCRef<T> {}

// Ugly, but sometimes we want a placeholder. Code may be refactorable to never need this though, I think.
impl<T> Default for GCRef<T> {
    fn default() -> Self {
        unsafe {
            GCRef {
                ptr: Address::from_usize(0),
                _phantom: PhantomData
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
    
    /// Does the address not point to any data?
    /// We use this to avoid using an Option type in interpreter frames. Not sure it's worth it though.
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.ptr.is_zero() }
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

        let addr = mmtk_alloc(mutator, size, GC_ALIGN, GC_OFFSET, GC_SEMANTICS);
        debug_assert!(!addr.is_zero());
        
        // println!("{}", mmtk_free_bytes());

        mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, GC_SEMANTICS);

        unsafe {
            *addr.as_mut_ref() = obj;
        }

        GCRef {
            ptr: addr,
            _phantom: PhantomData
        }
    }
}

// for convenience, but removable
impl GCRef<String> {
    pub fn as_str(&self) -> &str {
        self.to_obj().as_str()
    }
}