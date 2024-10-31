use crate::api::{
    mmtk_alloc, mmtk_bind_mutator, mmtk_destroy_mutator, mmtk_handle_user_collection_request,
    mmtk_initialize_collection, mmtk_set_fixed_heap_size,
};
use crate::gcref::GCRef;
use crate::object_model::OBJECT_REF_OFFSET;
use crate::slot::SOMSlot;
use crate::{MMTK_SINGLETON, MMTK_TO_VM_INTERFACE, MUTATOR_WRAPPER, SOMVM};
use log::debug;
use mmtk::util::alloc::{Allocator, BumpAllocator};
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::{Address, ObjectReference, OpaquePointer, VMMutatorThread, VMThread};
use mmtk::vm::SlotVisitor;
use mmtk::{memory_manager, AllocationSemantics, MMTKBuilder, Mutator};
use num_bigint::BigInt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::mem::size_of;

pub static IS_WORLD_STOPPED: AtomicBool = AtomicBool::new(false);

static GC_OFFSET: usize = 0;
static GC_ALIGN: usize = 8;
static GC_SEMANTICS: AllocationSemantics = AllocationSemantics::Default;

/// TODO rename, maybe MutatorWrapper
pub struct GCInterface {
    mutator: Box<Mutator<SOMVM>>,
    default_allocator: *mut BumpAllocator<SOMVM>,
    mutator_thread: VMMutatorThread,
    start_the_world_count: usize,
}

impl Drop for GCInterface {
    fn drop(&mut self) {
        mmtk_destroy_mutator(self.mutator.as_mut())
    }
}

pub struct MMTKtoVMCallbacks {
    pub scan_object_fn: fn(ObjectReference, &mut dyn SlotVisitor<SOMSlot>),
    pub get_roots_in_mutator_thread_fn: fn(_mutator: &mut Mutator<SOMVM>) -> Vec<SOMSlot>,
}

impl GCInterface {
    /// Initialize the GCInterface. Internally inits MMTk and fetches everything needed to actually communicate with the GC.
    pub fn init<'a>(heap_size: usize, vm_callbacks: MMTKtoVMCallbacks) -> &'a mut Self {
        let (mutator_thread, mutator, default_allocator) = Self::init_mmtk(heap_size);
        let mut self_ = Box::new(Self {
            mutator_thread,
            mutator,
            default_allocator,
            start_the_world_count: 0,
        });

        unsafe {
            // in the context of tests, this function gets invoked many times, so they can have already been initialized.

            if MUTATOR_WRAPPER.get().is_none() {
                MUTATOR_WRAPPER
                    .set(&mut *self_)
                    .unwrap_or_else(|_| panic!("couldn't set mutator wrapper?"));
            }

            if MMTK_TO_VM_INTERFACE.get().is_none() {
                MMTK_TO_VM_INTERFACE.set(vm_callbacks).unwrap_or_else(|_| {
                    panic!("couldn't set callbacks to establish MMTk=>VM connection?")
                });
            }
        }

        Box::leak(self_)
    }

    /// Initialize MMTk, and get from it all the info we need to initialize our interface
    fn init_mmtk(
        heap_size: usize,
    ) -> (
        VMMutatorThread,
        Box<Mutator<SOMVM>>,
        *mut BumpAllocator<SOMVM>,
    ) {
        let builder: MMTKBuilder = {
            let mut builder = MMTKBuilder::new();

            let heap_success = mmtk_set_fixed_heap_size(&mut builder, heap_size);
            assert!(heap_success, "Couldn't set MMTk fixed heap size");

            // let gc_success = builder.set_option("plan", "NoGC");
            let gc_success = builder.set_option("plan", "MarkSweep");
            // let gc_success = builder.set_option("plan", "SemiSpace");
            assert!(gc_success, "Couldn't set GC plan");

            // let ok = builder.set_option("stress_factor", DEFAULT_STRESS_FACTOR.to_string().as_str());
            // assert!(ok);
            // let ok = builder.set_option("analysis_factor", DEFAULT_STRESS_FACTOR.to_string().as_str());
            // assert!(ok);

            builder
        };

        if MMTK_SINGLETON.get().is_none() {
            MMTK_SINGLETON
                .set({
                    let mmtk = mmtk::memory_manager::mmtk_init::<SOMVM>(&builder);
                    *mmtk
                })
                .unwrap_or_else(|_| panic!("couldn't set the MMTk singleton"));

            mmtk_initialize_collection(VMThread(OpaquePointer::UNINITIALIZED));
        }

        let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED));
        let mutator = mmtk_bind_mutator(tls);

        let selector = memory_manager::get_allocator_mapping(
            MMTK_SINGLETON.get().unwrap(),
            AllocationSemantics::Default,
        );
        let default_allocator_offset = Mutator::<SOMVM>::get_allocator_base_offset(selector);

        // At run time: allocate with the default semantics without resolving allocator
        let default_allocator: *mut BumpAllocator<SOMVM> = {
            let mutator_addr = Address::from_ref(&*mutator);
            unsafe {
                let ptr = mutator_addr + default_allocator_offset;
                ptr.as_mut_ref()
                // (mutator_addr + default_allocator_offset).as_mut_ref::<BumpAllocator<SOMVM>>()
            }
        };

        // (tls, mutator)
        (tls, mutator, default_allocator)
    }
}

impl GCInterface {
    // Allocates a type on the heap and returns a pointer to it.
    pub fn alloc<T: HasTypeInfoForGC>(&mut self, obj: T) -> GCRef<T> {
        self.alloc_with_size(obj, size_of::<T>())
    }

    // Allocates a type, but with a given size. Useful when an object needs more than what we tell Rust through defining a struct.
    // (e.g. Value arrays stored directly in the heap - see BC Frame)
    pub fn alloc_with_size<T: HasTypeInfoForGC>(&mut self, obj: T, size: usize) -> GCRef<T> {
        // self.alloc_with_size_cached_allocator(obj, size);
        self.alloc_with_size_slow(obj, size)
    }

    #[inline(always)]
    #[allow(dead_code, unused)]
    fn alloc_with_size_slow<T: HasTypeInfoForGC>(&mut self, obj: T, size: usize) -> GCRef<T> {
        debug_assert!(size >= MIN_OBJECT_SIZE);
        let mutator = self.mutator.as_mut();

        // adding VM header size (type info) to amount we allocate
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
    fn alloc_with_size_cached_allocator<T: HasTypeInfoForGC>(
        &mut self,
        obj: T,
        size: usize,
    ) -> GCRef<T> {
        debug_assert!(size >= MIN_OBJECT_SIZE);
        let allocator = unsafe { &mut (*self.default_allocator) };

        // adding VM header size (type info) to amount we allocate
        let size = size + OBJECT_REF_OFFSET;

        dbg!("wo");
        let addr = allocator.alloc(size, GC_ALIGN, GC_OFFSET);

        debug_assert!(!addr.is_zero());
        debug_assert!(size >= MIN_OBJECT_SIZE);
        let mutator = self.mutator.as_mut();

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

    /// Dispatches a manual collection request to MMTk.
    pub fn full_gc_request(&self) {
        mmtk_handle_user_collection_request(self.mutator_thread);
    }

    /// Returns the number of total GC collections.
    pub fn get_nbr_collections(&self) -> usize {
        self.start_the_world_count
    }

    pub(crate) fn block_for_gc(&mut self, _tls: VMMutatorThread) {
        AtomicBool::store(&IS_WORLD_STOPPED, true, Ordering::SeqCst);
        debug!("block_for_gc: stopped the world!");
        while AtomicBool::load(&IS_WORLD_STOPPED, Ordering::SeqCst) {}
        debug!("block_for_gc: world no longer stopped.");
    }

    pub(crate) fn stop_all_mutators<F>(&'static mut self, mut mutator_visitor: F)
    where
        F: FnMut(&'static mut Mutator<SOMVM>),
    {
        debug!("stop_all_mutators called");

        while !AtomicBool::load(&IS_WORLD_STOPPED, Ordering::SeqCst) {
            // wait for world to be properly stopped (might not be needed)
        }

        mutator_visitor(self.mutator.as_mut())
    }

    pub(crate) fn resume_mutators(&mut self) {
        debug!("resuming mutators.");
        self.start_the_world_count += 1;
        AtomicBool::store(&IS_WORLD_STOPPED, false, Ordering::SeqCst);
    }

    pub(crate) fn get_mutator(&mut self, _tls: VMMutatorThread) -> &mut Mutator<SOMVM> {
        self.mutator.as_mut()
    }

    pub(crate) fn get_all_mutators(
        &mut self,
    ) -> Box<dyn Iterator<Item = &mut Mutator<SOMVM>> + '_> {
        debug!("calling get_all_mutators");
        Box::new(std::iter::once(self.mutator.as_mut()))
    }
}

// ------------------

/// Implements a per-type magic number.
/// GC needs to access type info from raw ObjectReference types, so data that gets put on the GC heap has an associated type ID that gets put in a per-allocation header.
pub trait HasTypeInfoForGC {
    fn get_magic_gc_id() -> u8;
}

pub const STRING_MAGIC_ID: u8 = 10;
pub const BIGINT_MAGIC_ID: u8 = 11;
pub const VECU8_MAGIC_ID: u8 = 12;

impl HasTypeInfoForGC for String {
    fn get_magic_gc_id() -> u8 {
        STRING_MAGIC_ID
    }
}
impl HasTypeInfoForGC for BigInt {
    fn get_magic_gc_id() -> u8 {
        BIGINT_MAGIC_ID
    }
}

impl HasTypeInfoForGC for Vec<u8> {
    fn get_magic_gc_id() -> u8 {
        VECU8_MAGIC_ID
    }
}
