use crate::api::{
    mmtk_bind_mutator, mmtk_destroy_mutator, mmtk_handle_user_collection_request, mmtk_initialize_collection, mmtk_set_fixed_heap_size,
    mmtk_used_bytes,
};
use crate::gcref::Gc;
use crate::gcslice::GcSlice;
use crate::object_model::OBJECT_REF_OFFSET;
use crate::slot::SOMSlot;
use crate::{MMTK_SINGLETON, MMTK_TO_VM_INTERFACE, MUTATOR_WRAPPER, SOMVM};
use core::mem::size_of;
use log::debug;
use mmtk::util::alloc::Allocator;
#[cfg(feature = "marksweep")]
use mmtk::util::alloc::FreeListAllocator;
#[cfg(feature = "semispace")]
use mmtk::util::alloc::{BumpAllocator, BumpPointer};
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::{Address, ObjectReference, OpaquePointer, VMMutatorThread, VMThread};
use mmtk::vm::SlotVisitor;
use mmtk::{memory_manager, AllocationSemantics, MMTKBuilder, Mutator};
use num_bigint::BigInt;
use std::sync::{Condvar, LazyLock, Mutex};
use std::time::{Duration, Instant};

#[cfg(not(any(feature = "marksweep", feature = "semispace")))]
compile_error!("Either marksweep or semispace must be enabled for this crate.");

#[cfg(all(feature = "semispace", feature = "marksweep"))]
compile_error!("Several GC strategies enabled: only one is allowed at a time.");

pub static WORLD_LOCK: LazyLock<(Mutex<bool>, Condvar)> = LazyLock::new(|| (Mutex::new(false), Condvar::new()));

static GC_OFFSET: usize = 0;
static GC_ALIGN: usize = 8;
// static GC_SEMANTICS: AllocationSemantics = AllocationSemantics::Default;

/// TODO rename, maybe MutatorWrapper
pub struct GCInterface {
    mutator: Box<Mutator<SOMVM>>,
    #[cfg(feature = "marksweep")]
    default_allocator: *mut FreeListAllocator<SOMVM>,
    #[cfg(feature = "semispace")]
    default_allocator: *mut mmtk::util::alloc::BumpAllocator<SOMVM>,
    #[cfg(feature = "semispace")]
    #[allow(unused)]
    alloc_bump_ptr: BumpPointer,
    mutator_thread: VMMutatorThread,
    is_collecting: bool,
    start_the_world_count: usize,
    total_gc_time: Duration,
}

impl Drop for GCInterface {
    fn drop(&mut self) {
        // mmtk_handle_user_collection_request(self.mutator_thread);
        mmtk_destroy_mutator(self.mutator.as_mut())
    }
}

/// Callbacks used to provide MMTk->VM communication.
pub struct MMTKtoVMCallbacks {
    /// Scans an object. Needed for tracing.
    pub scan_object: fn(ObjectReference, &mut dyn SlotVisitor<SOMSlot>),
    /// Get the VM roots.
    pub get_roots_in_mutator_thread: fn(&mut Mutator<SOMVM>) -> Vec<SOMSlot>,
    /// Adapt an object after being copied elsewhere (not really at the moment needed except in one case)
    pub adapt_post_copy: fn(ObjectReference, ObjectReference),
    /// Get the size of the object. Needed when copying it
    pub get_object_size: fn(ObjectReference) -> usize,
}

impl GCInterface {
    /// Initialize the GCInterface. Internally inits MMTk and fetches everything needed to actually communicate with the GC.
    pub fn init<'a>(heap_size: usize, vm_callbacks: MMTKtoVMCallbacks) -> &'a mut Self {
        let (mutator_thread, mutator) = Self::init_mmtk(heap_size);
        #[cfg(feature = "marksweep")]
        let default_allocator = Self::get_default_allocator::<FreeListAllocator<SOMVM>>(mutator.as_ref());
        #[cfg(feature = "semispace")]
        let default_allocator = Self::get_default_allocator::<BumpAllocator<SOMVM>>(mutator.as_ref());

        let self_ = Box::new(Self {
            mutator_thread,
            mutator,
            is_collecting: false,
            default_allocator,
            #[cfg(feature = "semispace")]
            alloc_bump_ptr: BumpPointer::default(),
            start_the_world_count: 0,
            total_gc_time: Duration::new(0, 0),
        });

        let gc_interface_ptr = Box::leak(self_);

        unsafe {
            // in the context of tests, this function gets invoked many times, so they can have already been initialized.
            // TODO: which makes me realize that this function's structure is subpar. Why do we return a NEW GCInterface at all, then?
            // The universe should likely use a reference to the OnceCell, or something... That'd be better.

            if MUTATOR_WRAPPER.get().is_none() {
                // very unsafe, very ugly: we duplicate a mutable reference to the GC interface ptr. need to avoid by implementing above idea
                let dup_ptr = &mut *(gc_interface_ptr as *mut GCInterface);
                MUTATOR_WRAPPER.set(dup_ptr).unwrap_or_else(|_| panic!("couldn't set mutator wrapper?"));
            }

            if MMTK_TO_VM_INTERFACE.get().is_none() {
                MMTK_TO_VM_INTERFACE.get_or_init(|| vm_callbacks);
            }
        }

        gc_interface_ptr
    }

    /// Initialize MMTk, and get from it all the info we need to initialize our interface
    fn init_mmtk(heap_size: usize) -> (VMMutatorThread, Box<Mutator<SOMVM>>) {
        let builder: MMTKBuilder = {
            let mut builder = MMTKBuilder::new();

            let heap_success = mmtk_set_fixed_heap_size(&mut builder, heap_size);
            assert!(heap_success, "Couldn't set MMTk fixed heap size");

            if cfg!(feature = "marksweep") {
                assert!(builder.set_option("plan", "MarkSweep"));
            } else if cfg!(feature = "semispace") {
                assert!(builder.set_option("plan", "SemiSpace"));
            } else {
                panic!("No GC plan set!")
            }

            #[cfg(feature = "stress_test")]
            assert!(builder.set_option("stress_factor", "4000000"));

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

        (tls, mutator)
    }

    fn get_default_allocator<T>(mutator: &Mutator<SOMVM>) -> *mut T
    where
        T: Allocator<SOMVM>,
    {
        let selector = memory_manager::get_allocator_mapping(MMTK_SINGLETON.get().unwrap(), AllocationSemantics::Default);
        let default_allocator_offset = Mutator::<SOMVM>::get_allocator_base_offset(selector);

        // At run time: allocate with the default semantics without resolving allocator
        let default_allocator: *mut T = {
            let mutator_addr = Address::from_ref(mutator);
            unsafe {
                let allocator_ptr = mutator_addr + default_allocator_offset;
                allocator_ptr.as_mut_ref()
            }
        };

        default_allocator
    }
}

impl GCInterface {
    /// Allocates a type on the heap and returns a pointer to it.
    /// Considers that the provided object's size can be trivially inferred with a `size_of` call (which isn't the case for all of our objects, e.g. frames)
    pub fn alloc<T: HasTypeInfoForGC>(&mut self, obj: T) -> Gc<T> {
        self.alloc_with_size(obj, size_of::<T>())
    }

    /// Allocates a type, but with a given size.
    pub fn alloc_with_size<T: HasTypeInfoForGC>(&mut self, obj: T, size: usize) -> Gc<T> {
        debug_assert!(size >= MIN_OBJECT_SIZE);

        // adding VM header size (type info) to amount we allocate
        let header_addr = self.request_bytes(size + OBJECT_REF_OFFSET);

        debug_assert!(!header_addr.is_zero());
        let obj_addr = SOMVM::object_start_to_ref(header_addr);

        // AFAIK, this is not needed.
        // mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, GC_SEMANTICS);

        unsafe {
            *header_addr.as_mut_ref() = T::get_magic_gc_id();
            *(obj_addr.to_raw_address().as_mut_ref()) = obj;
        }

        Gc::from(obj_addr.to_raw_address())
    }

    // Allocates a type on the heap and returns a pointer to it.
    pub fn alloc_slice<T: SupportedSliceType + std::fmt::Debug>(&mut self, obj: &[T]) -> GcSlice<T> {
        let len = obj.len();
        let size = {
            match std::mem::size_of_val(obj) {
                v if v < MIN_OBJECT_SIZE => MIN_OBJECT_SIZE,
                v => v,
            }
        };

        let header_addr: Address = self.request_bytes(size + OBJECT_REF_OFFSET);
        let obj_addr = SOMVM::object_start_to_ref(header_addr);

        unsafe {
            *header_addr.as_mut_ref() = T::get_magic_gc_slice_id();
            std::ptr::copy_nonoverlapping(obj.as_ptr(), obj_addr.to_raw_address().as_mut_ref(), obj.len());
        }

        GcSlice::new(obj_addr.to_raw_address(), len)
    }

    #[cfg(feature = "marksweep")]
    /// Request `size` bytes from MMTk.
    /// Importantly, this MAY TRIGGER A COLLECTION. Which means any function that relies on it must be mindful of this,
    /// such as by making sure no arguments are dangling on the Rust stack away from the GC's reach.
    pub fn request_bytes(&mut self, size: usize) -> Address {
        unsafe { &mut (*self.default_allocator) }.alloc(size, GC_ALIGN, GC_OFFSET)
        // slow path, for debugging
        // crate::api::mmtk_alloc(&mut self.mutator, size, GC_ALIGN, GC_OFFSET, AllocationSemantics::Default)
    }

    #[cfg(feature = "semispace")]
    /// Request `size` bytes from MMTk.
    /// Importantly, this MAY TRIGGER A COLLECTION. Which means any function that relies on it must be mindful of this,
    /// such as by making sure no arguments are dangling on the Rust stack away from the GC's reach.
    pub fn request_bytes(&mut self, size: usize) -> Address {
        //unsafe { &mut (*self.default_allocator) }.alloc(size, GC_ALIGN, GC_OFFSET)
        unsafe { &mut (*self.default_allocator) }.alloc(size, GC_ALIGN, GC_OFFSET)

        // TODO: this code should work, and -does-, but sometimes returns references to the old space, as far as i can tell.
        // code taken from MMTk docs. https://docs.mmtk.io/portingguide/perf_tuning/alloc.html#option-3-embed-the-fast-path-struct
        // let new_cursor = self.alloc_bump_ptr.cursor + size;
        // if new_cursor < self.alloc_bump_ptr.limit {
        //     let addr = self.alloc_bump_ptr.cursor;
        //     self.alloc_bump_ptr.cursor = new_cursor;
        //     addr
        // } else {
        //     let default_allocator = unsafe { &mut *self.default_allocator };
        //     default_allocator.bump_pointer = self.alloc_bump_ptr;
        //     let addr = default_allocator.alloc(size, GC_ALIGN, GC_OFFSET);
        //     // Copy bump pointer values to the fastpath BumpPointer so we will have an allocation buffer.
        //     self.alloc_bump_ptr = default_allocator.bump_pointer;
        //     addr
        // }
    }

    /// TODO doc + should likely deduce the size from the type
    pub fn request_memory_for_type<T: HasTypeInfoForGC>(&mut self, type_size: usize) -> Gc<T> {
        let mut bytes = self.request_bytes(type_size + OBJECT_REF_OFFSET);
        unsafe {
            *bytes.as_mut_ref::<u8>() = T::get_magic_gc_id();
            bytes += OBJECT_REF_OFFSET;
            bytes.into()
        }
    }

    /// Custom alloc function, for traits to be able to choose how to allocate their data.
    /// In practice, that's usually allowing for more memory than Rust might be able to infer from the struct size, and filling it with our own data.
    /// TODO: Even more in practice, it's not used much anymore. The issue is that if the alloc triggers and we use moving GC, the closure can now be holding outdated pointers.
    pub fn alloc_with_post_init<T: HasTypeInfoForGC, F>(&mut self, obj: T, size: usize, mut post_alloc_init_closure: F) -> Gc<T>
    where
        F: FnMut(Gc<T>),
    {
        let instance_ref = self.alloc_with_size(obj, size);
        post_alloc_init_closure(instance_ref);
        instance_ref
    }

    /// Dispatches a manual collection request to MMTk.
    pub fn full_gc_request(&self) -> bool {
        mmtk_handle_user_collection_request(self.mutator_thread)
    }

    /// Returns the number of total GC collections.
    pub fn get_nbr_collections(&self) -> usize {
        self.start_the_world_count
    }

    /// Returns the number of used bytes
    pub fn get_used_bytes(&self) -> usize {
        mmtk_used_bytes()
    }

    /// Returns the total time spent performing GC.
    pub fn get_total_gc_time(&self) -> usize {
        self.total_gc_time.as_micros() as usize
    }

    /// Whether or not we're currently performing GC.
    /// Might be redundant with `is_world_stopped`, to be honest.
    pub fn is_currently_collecting(&self) -> bool {
        self.is_collecting
    }

    /// Block the main thread to perform GC.
    pub(crate) fn block_for_gc(&mut self, _tls: VMMutatorThread) {
        debug!("block_for_gc: stopping the world!");
        self.is_collecting = true;

        let (is_world_stopped, cvar) = &*WORLD_LOCK;
        {
            let mut lock = is_world_stopped.lock().unwrap();
            *lock = true;
        }

        let time_pre_gc = Instant::now();

        let result = cvar.wait_timeout_while(is_world_stopped.lock().unwrap(), Duration::from_secs(15), |pending| *pending).unwrap();
        if result.1.timed_out() {
            panic!("GC timed out: highly likely to be a crash in a GC thread.")
        }

        debug!("block_for_gc: world no longer stopped.");
        self.is_collecting = false;
        self.total_gc_time += Instant::now() - time_pre_gc;
    }

    pub(crate) fn stop_all_mutators<F>(&'static mut self, mut mutator_visitor: F)
    where
        F: FnMut(&'static mut Mutator<SOMVM>),
    {
        debug!("stop_all_mutators called");

        //while !AtomicBool::load(&IS_WORLD_STOPPED, Ordering::SeqCst) {
        //    // wait for world to be properly stopped (might not be needed)
        //}

        mutator_visitor(self.mutator.as_mut())
    }

    pub(crate) fn resume_mutators(&mut self) {
        debug!("resuming mutators.");
        self.start_the_world_count += 1;

        let (is_world_stopped, cvar) = &*WORLD_LOCK;
        let mut pending = is_world_stopped.lock().unwrap();
        *pending = false;
        cvar.notify_one();
    }

    pub(crate) fn get_mutator(&mut self, _tls: VMMutatorThread) -> &mut Mutator<SOMVM> {
        self.mutator.as_mut()
    }

    pub(crate) fn get_all_mutators(&mut self) -> Box<dyn Iterator<Item = &mut Mutator<SOMVM>> + '_> {
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

//impl<T> HasTypeInfoForGC for GCSlice<T> {
//    fn get_magic_gc_id() -> u8 {
//        GCSLICE_MAGIC_ID
//    }
//}

pub trait SupportedSliceType {
    fn get_magic_gc_slice_id() -> u8;
}

impl<T: SupportedSliceType> HasTypeInfoForGC for GcSlice<T> {
    fn get_magic_gc_id() -> u8 {
        T::get_magic_gc_slice_id()
    }
}
