use crate::api::mmtk_alloc;
use crate::gc_interface::{GCInterface, HasTypeInfoForGC, IS_WORLD_STOPPED};
use crate::object_model::OBJECT_REF_OFFSET;
use crate::SOMVM;
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::Address;
use mmtk::AllocationSemantics;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering;

static GC_OFFSET: usize = 0;
static GC_ALIGN: usize = 8;
static GC_SEMANTICS: AllocationSemantics = AllocationSemantics::Default;

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

// -------------------------------------

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

impl<T: HasTypeInfoForGC> GCRef<T> {
    // Allocates a type on the heap and returns a pointer to it.
    pub fn alloc(obj: T, gc_interface: &mut GCInterface) -> GCRef<T> {
        debug_assert_eq!(IS_WORLD_STOPPED.load(Ordering::SeqCst), false);
        Self::alloc_with_size(obj, gc_interface, size_of::<T>())
    }

    // Allocates a type, but with a given size. Useful when an object needs more than what we tell Rust through defining a struct.
    // (e.g. Value arrays stored directly in the heap - see BC Frame)
    pub fn alloc_with_size(obj: T, gc_interface: &mut GCInterface, size: usize) -> GCRef<T> {
        // Self::alloc_with_size_cached_allocator(obj, gc_interface, size)
        Self::alloc_with_size_allocator_uncached(obj, gc_interface, size)
    }

    #[inline(always)]
    // #[allow(dead_code, unused)]
    fn alloc_with_size_allocator_uncached(
        obj: T,
        gc_interface: &mut GCInterface,
        size: usize,
    ) -> GCRef<T> {
        debug_assert!(size >= MIN_OBJECT_SIZE);
        let mutator = gc_interface.mutator.as_mut();

        // not sure that's correct? adding VM header size (type info) to amount we allocate.
        let size = size + OBJECT_REF_OFFSET;

        let header_addr = mmtk_alloc(mutator, size, GC_ALIGN, GC_OFFSET, GC_SEMANTICS);

        debug_assert!(!header_addr.is_zero());
        let obj_addr = SOMVM::object_start_to_ref(header_addr);

        // AFAIK, this is not needed.
        // mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, GC_SEMANTICS);

        unsafe {
            // *addr.as_mut_ref() = obj;
            *header_addr.as_mut_ref() = T::get_magic_gc_id();
            *(obj_addr.to_raw_address().as_mut_ref()) = obj;
        }

        GCRef {
            ptr: obj_addr.to_raw_address(),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code, unused)]
    fn alloc_with_size_cached_allocator(
        obj: T,
        gc_interface: &mut GCInterface,
        size: usize,
    ) -> GCRef<T> {
        todo!("should not be ran before being adapted to match the cached version");

        // debug_assert!(size >= MIN_OBJECT_SIZE);
        // let allocator = unsafe {&mut (*gc_interface.default_allocator)};
        //
        // // not sure that's correct? adding VM header size (type info) to amount we allocate.
        // let size = size + OBJECT_REF_OFFSET;
        //
        // let addr = allocator.alloc(size, GC_ALIGN, GC_OFFSET);
        // debug_assert!(!addr.is_zero());
        // let obj_addr = SOMVM::object_start_to_ref(addr);
        //
        //
        // // let obj = SOMVM::object_start_to_ref(addr);
        // // let space = allocator.get_space();
        // // debug_assert!(!obj.to_raw_address().is_zero());
        // // space.initialize_object_metadata(obj, true);
        //
        // let space = allocator.get_space();
        // dbg!(space.name());
        // space.initialize_object_metadata(obj_addr, true);
        //
        // dbg!("wa");
        // unsafe {
        //     // *addr.as_mut_ref() = obj;
        //
        //     // dbg!(addr);
        //     // dbg!(obj_addr);
        //     // dbg!();
        //     let header_ref: *mut usize = addr.as_mut_ref();
        //     *header_ref = 4774451407313061000; // 4242424242424242
        //
        //     *(obj_addr.to_raw_address().as_mut_ref()) = obj;
        //     // obj_addr.to_header()
        // }
        //
        // GCRef {
        //     ptr: obj_addr.to_raw_address(),
        //     _phantom: PhantomData,
        // }
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
