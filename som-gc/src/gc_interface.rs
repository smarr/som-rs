use log::debug;
use mmtk::util::alloc::BumpAllocator;
use mmtk::util::{Address, ObjectReference, OpaquePointer, VMMutatorThread, VMThread};
use mmtk::vm::{RootsWorkFactory, SlotVisitor};
use mmtk::{memory_manager, AllocationSemantics, MMTKBuilder, Mutator};
use std::sync::atomic::{AtomicBool, Ordering};
use num_bigint::BigInt;
use crate::api::{mmtk_bind_mutator, mmtk_destroy_mutator, mmtk_handle_user_collection_request, mmtk_initialize_collection, mmtk_set_fixed_heap_size};
use crate::{MMTK_SINGLETON, MMTK_TO_VM_INTERFACE, MUTATOR_WRAPPER, SOMVM};
use crate::gcref::GCRef;
use crate::slot::SOMSlot;

pub static IS_WORLD_STOPPED: AtomicBool = AtomicBool::new(false);

pub const STRING_MAGIC_ID: u8 = 10;
pub const BIGINT_MAGIC_ID: u8 = 11;
pub const VECU8_MAGIC_ID: u8 = 12;

/// Implements a per-type magic number.
/// GC needs to access type info from raw ObjectReference types, so data that gets put on the GC heap has an associated type ID that gets put in a per-allocation header.
pub trait HasTypeInfoForGC {
    fn get_magic_gc_id() -> u8;
}

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

/// TODO rename, maybe MutatorWrapper
pub struct GCInterface {
    pub(crate) mutator: Box<Mutator<SOMVM>>,
    mutator_thread: VMMutatorThread,
    _default_allocator: *mut BumpAllocator<SOMVM>,
    start_the_world_count: usize
}

impl Drop for GCInterface {
    fn drop(&mut self) {
        mmtk_destroy_mutator(self.mutator.as_mut())
    }
}

pub struct MMTKtoVMCallbacks {
    pub scan_object_fn: fn(ObjectReference, &mut dyn SlotVisitor<SOMSlot>),
    pub get_roots_in_mutator_thread_fn: fn(_mutator: &mut Mutator<SOMVM>) -> Vec<SOMSlot>
}

impl GCInterface {
    /// Initialize the GCInterface. Internally inits MMTk and fetches everything needed to actually communicate with the GC.
    pub fn init<'a>(heap_size: usize, vm_callbacks: MMTKtoVMCallbacks) -> &'a mut Self {
        let (mutator_thread, mutator, default_allocator) = Self::init_mmtk(heap_size);
        let mut self_ = Box::new(Self {
            mutator_thread,
            mutator,
            _default_allocator: default_allocator,
            start_the_world_count: 0
        });

        unsafe {
            // in the context of tests, this function gets invoked many times, so they can have already been initialized.
            
            if MUTATOR_WRAPPER.get().is_none() {
                MUTATOR_WRAPPER.set(&mut *self_).unwrap_or_else(|_| panic!("couldn't set mutator wrapper?"));
            }
            
            if MMTK_TO_VM_INTERFACE.get().is_none() {
                MMTK_TO_VM_INTERFACE.set(vm_callbacks).unwrap_or_else(|_| panic!("couldn't set callbacks to establish MMTk=>VM connection?"));
            } 
        }

        
        Box::leak(self_)
    }

    fn init_mmtk(heap_size: usize) -> (VMMutatorThread, Box<Mutator<SOMVM>>, *mut BumpAllocator<SOMVM>) {
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
            MMTK_SINGLETON.set({
                let mmtk = mmtk::memory_manager::mmtk_init::<SOMVM>(&builder);
                *mmtk
            }).unwrap_or_else(|_| panic!("couldn't set the MMTk singleton"));

            mmtk_initialize_collection(VMThread(OpaquePointer::UNINITIALIZED));
        }

        let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED)); // TODO: do I need a thread pointer here?
        let mutator = mmtk_bind_mutator(tls);

        let selector = memory_manager::get_allocator_mapping(
            &MMTK_SINGLETON.get().unwrap(),
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
    pub fn allocate<T: HasTypeInfoForGC>(&mut self, obj: T) -> GCRef<T> {
        GCRef::<T>::alloc(obj, self)
    }

    /// Dispatches a manual collection request to MMTk.
    pub fn full_gc_request(&self) {
        mmtk_handle_user_collection_request(self.mutator_thread);
    }
    
    pub fn block_for_gc(&mut self, _tls: VMMutatorThread) {
        AtomicBool::store(&IS_WORLD_STOPPED, true, Ordering::SeqCst);
        debug!("block_for_gc: stopped the world!");
        while AtomicBool::load(&IS_WORLD_STOPPED, Ordering::SeqCst) {}
        debug!("block_for_gc: world no longer stopped.");
    }

    pub fn stop_all_mutators<F>(&'static mut self, mut mutator_visitor: F)
    where
        F: FnMut(&'static mut Mutator<SOMVM>),
    {
        debug!("stop_all_mutators called");

        while !AtomicBool::load(&IS_WORLD_STOPPED, Ordering::SeqCst) {
            // wait for world to be properly stopped
        }

        mutator_visitor(self.mutator.as_mut())
    }

    pub fn resume_mutators(&mut self) {
        debug!("resuming mutators.");
        self.start_the_world_count += 1;
        AtomicBool::store(&IS_WORLD_STOPPED, false, Ordering::SeqCst);
    }

    pub fn get_mutator(&mut self, _tls: VMMutatorThread) -> &mut Mutator<SOMVM> {
        debug_assert!(self.mutator_thread == _tls); // not even sure that's correct
        self.mutator.as_mut()
    }
    
    pub fn get_all_mutators(&mut self) -> Box<dyn Iterator<Item = &mut Mutator<SOMVM>> + '_> {
        debug!("calling get_all_mutators");
        Box::new(std::iter::once(self.mutator.as_mut()))
    }

    pub fn scan_vm_specific_roots(&self, _factory: impl RootsWorkFactory<SOMSlot> + Sized) {
        debug!("calling scan_vm_specific_roots (unused)");
    }
}