use core::mem::size_of;
use mmtk::util::{Address, VMMutatorThread};
use som_gc::SOMVM;
use std::marker::PhantomData;
use mmtk::{AllocationSemantics, Mutator};
// use mmtk::util::alloc::BumpAllocator;
use som_gc::api::{mmtk_alloc, mmtk_destroy_mutator, mmtk_handle_user_collection_request, mmtk_post_alloc};
use som_gc::entry_point::init_gc;

static GC_OFFSET: usize = 0;
static GC_ALIGN: usize = 8;
static GC_SEMANTICS: AllocationSemantics = AllocationSemantics::Default;

pub struct GCInterface {
    mutator_thread: VMMutatorThread,
    mutator: Box<Mutator<SOMVM>>,
    // pub default_allocator: Box<BumpAllocator<SOMVM>>
}

impl Drop for GCInterface {
    fn drop(&mut self) {
        mmtk_destroy_mutator(self.mutator.as_mut())
    }
}

impl GCInterface {
    /// Initialize the GCInterface. Internally inits MMTk and fetches everything needed to actually communicate with the GC. 
    pub fn init() -> Self {
        let (mutator_thread, mutator) = init_gc();
        Self {
            mutator_thread,
            mutator,
            // default_allocator
        }
    }
    
    /// Dispatches a manual collection request to MMTk.
    pub fn full_gc_request(&self) {
        mmtk_handle_user_collection_request(self.mutator_thread)
    }
}

// ------

/// A pointer to the heap for GC.
#[derive(Debug)]
pub struct GCRef<T> {
    pub ptr: Address,
    pub _phantom: PhantomData<T>,
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

// -------------------------------------

impl<T> GCRef<T> {
    /// Turn a GC pointer back into the type itself (as a reference)
    pub fn to_obj(&self) -> &mut T {
        debug_assert!(!self.ptr.is_zero());
        unsafe { &mut *(self.ptr.as_mut_ref()) }
    }
    
    /// Hacks for convenience, since I'm refactoring from Refcounts. TODO remove
    pub fn borrow(&self) -> &mut T {
        Self::to_obj(self)
    }

    /// same deal
    pub fn borrow_mut(&self) -> &mut T {
        Self::to_obj(self)
    }

    /// same dealll
    pub fn as_ptr(&self) -> &mut T {
        Self::to_obj(self)
    }

    /// Does the address not point to any data?
    /// We use this to avoid using an Option type in interpreter frames. Not sure if it's worth it though.
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.ptr.is_zero() }
}

impl<T> GCRef<T> {
    // Allocates a type on the heap and returns a pointer to it.
    pub fn alloc(obj: T, gc_interface: &mut GCInterface) -> GCRef<T> {
        // Self::alloc_with_size_cached_allocator(obj, gc_interface, size_of::<T>())
        Self::alloc_with_size(obj, gc_interface, size_of::<T>())
    }

    // Allocates a type, but with a given size. Useful when an object needs more than what we tell Rust through defining a struct. 
    // (e.g. Value arrays stored directly in the heap - see BC Frame)
    #[inline(always)]
    pub fn alloc_with_size(obj: T, gc_interface: &mut GCInterface, size: usize) -> GCRef<T> {
        let mutator = gc_interface.mutator.as_mut();
        let addr = mmtk_alloc(mutator, size, GC_ALIGN, GC_OFFSET, GC_SEMANTICS);
        debug_assert!(!addr.is_zero());

        // println!("{}", mmtk_free_bytes());

        mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, GC_SEMANTICS);

        unsafe {
            *addr.as_mut_ref() = obj;
        }

        GCRef {
            ptr: addr,
            _phantom: PhantomData,
        }
    }
    
    // pub fn alloc_with_size_cached_allocator(obj: T, gc_interface: &mut GCInterface, size: usize) -> GCRef<T> {
    //     debug_assert!(size >= MIN_OBJECT_SIZE);
    //     let addr = gc_interface.default_allocator.alloc(size, GC_ALIGN, GC_OFFSET);
    //     debug_assert!(!addr.is_zero());
    //
    //     // println!("{}", mmtk_free_bytes());
    //
    //     // let obj = SOMVM::object_start_to_ref(addr);
    //     // let space = allocator.get_space();
    //     // debug_assert!(!obj.to_raw_address().is_zero());
    //     // space.initialize_object_metadata(obj, true);
    //
    //     gc_interface.default_allocator.get_space().initialize_object_metadata(SOMVM::object_start_to_ref(addr), true);
    //
    //     unsafe {
    //         *addr.as_mut_ref() = obj;
    //     }
    //
    //     GCRef {
    //         ptr: addr,
    //         _phantom: PhantomData,
    //     }
    // }
}

/// Custom alloc function.
/// 
/// Exists for that traits to be able to choose how to allocate their data. 
/// Must call GCRef::<T>::alloc(_with_size) internally to get a GCRef, but I can't strictly enforce that with Rust's type system.
/// In practice, that's usually allowing for more memory than Rust might be able to infer from the struct size, and filling it with our own data. 
pub trait CustomAlloc<T> {
    fn alloc(obj: T, mutator: &mut GCInterface) -> GCRef<T>;
}

// for convenience, but easily removable
impl GCRef<String> {
    pub fn as_str(&self) -> &str {
        self.to_obj().as_str()
    }
}