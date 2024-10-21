use crate::gc::api::{mmtk_alloc, mmtk_bind_mutator, mmtk_destroy_mutator, mmtk_handle_user_collection_request};
use crate::gc::{SOMSlot, MMTK_SINGLETON, SOMVM};
use crate::INTERPRETER_RAW_PTR;
use core::mem::size_of;
use log::info;
use mmtk::util::alloc::{Allocator, BumpAllocator};
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::{Address, OpaquePointer, VMMutatorThread, VMThread};
use mmtk::vm::RootsWorkFactory;
use mmtk::{memory_manager, AllocationSemantics, Mutator};
use std::collections::VecDeque;
use std::marker::PhantomData;
use structopt::lazy_static;

static GC_OFFSET: usize = 0;
static GC_ALIGN: usize = 8;
static GC_SEMANTICS: AllocationSemantics = AllocationSemantics::Default;

pub struct GCInterface {
    mutator: Box<Mutator<SOMVM>>,
    mutator_thread: VMMutatorThread,
    default_allocator: *mut BumpAllocator<SOMVM>,
    start_the_world_count: usize
}

impl Drop for GCInterface {
    fn drop(&mut self) {
        mmtk_destroy_mutator(self.mutator.as_mut())
    }
}

impl GCInterface {
    /// Initialize the GCInterface. Internally inits MMTk and fetches everything needed to actually communicate with the GC.
    pub fn init() -> Self {
        let (mutator_thread, mutator, default_allocator) = Self::init_mmtk();
        Self {
            mutator_thread,
            mutator,
            default_allocator,
            start_the_world_count: 0
        }
    }

    fn init_mmtk() -> (VMMutatorThread, Box<Mutator<SOMVM>>, *mut BumpAllocator<SOMVM>) {
        // pub fn init_gc() -> (VMMutatorThread, Box<Mutator<SOMVM>>) {
/*        if MMTK_SINGLETON.get().is_none() {
            let mut builder = mmtk_create_builder();

            // let heap_success = mmtk_set_fixed_heap_size(&mut builder, 1048576);
            // assert!(heap_success, "Couldn't set MMTk fixed heap size");

            // let gc_success = builder.set_option("plan", "NoGC");
            let gc_success = builder.set_option("plan", "SemiSpace");
            assert!(gc_success, "Couldn't set GC plan");

            // let ok = builder.set_option("stress_factor", DEFAULT_STRESS_FACTOR.to_string().as_str());
            // assert!(ok);
            // let ok = builder.set_option("analysis_factor", DEFAULT_STRESS_FACTOR.to_string().as_str());
            // assert!(ok);

            // let worked_thread = VMWorkerThread(VMThread(OpaquePointer::UNINITIALIZED));
            mmtk_initialize_collection(VMThread(OpaquePointer::UNINITIALIZED));
        }*/

        lazy_static::initialize(&MMTK_SINGLETON);

        let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED)); // TODO: do I need a thread pointer here?
        let mutator = mmtk_bind_mutator(tls);

        let selector = memory_manager::get_allocator_mapping(
            &MMTK_SINGLETON,
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

    /// Dispatches a manual collection request to MMTk.
    pub fn full_gc_request(&self) {
        mmtk_handle_user_collection_request(self.mutator_thread)
    }

    pub fn allocate<T>(&mut self, obj: T) -> GCRef<T> {
        GCRef::<T>::alloc(obj, self)
    }
}

// copied off the openjdk implem? not sure what the point of this is really
struct SOMMutatorIterator<'a> {
    mutators: VecDeque<&'a mut Mutator<SOMVM>>,
    phantom_data: PhantomData<&'a ()>,
}

impl<'a> Iterator for SOMMutatorIterator<'a> {
    type Item = &'a mut Mutator<SOMVM>;

    fn next(&mut self) -> Option<Self::Item> {
        self.mutators.pop_front()
    }
}

impl GCInterface {
    pub fn block_for_gc(&mut self) {
        info!("block for gc called");

        let before_blocking_count = self.start_the_world_count;
        while self.start_the_world_count <= before_blocking_count {
            // ... wait
        }
    }

    pub fn resume_mutators(&mut self) {
        info!("resume_mutators called");
        self.start_the_world_count += 1;
    }

    pub fn stop_all_mutators<F>(&mut self, _mutator_visitor: F)
    where
        F: FnMut(&'static mut Mutator<SOMVM>),
    {
        info!("stop_all_mutators called");
        // mutator_visitor(self.mutator.as_mut());
        // todo need to actually stop our mutator thread
    }

    pub(crate) fn get_mutator(&mut self, _tls: VMMutatorThread) -> &mut Mutator<SOMVM> {
        debug_assert!(self.mutator_thread == _tls); // not even sure that's correct
        self.mutator.as_mut()
    }
    pub fn get_all_mutators(&mut self) -> Box<dyn Iterator<Item = &mut Mutator<SOMVM>> + '_> {
        info!("calling get_all_mutators");
        // frankly not sure how to implement that one
        // Box::new(vec![self.mutator.as_mut()].iter())

        let mut mutators = VecDeque::new();
        mutators.push_back(self.mutator.as_mut()); 
        
        let iterator = SOMMutatorIterator {
            mutators,
            phantom_data: PhantomData,
        };
        
        Box::new(iterator)
        
        // unsafe { Box::from_raw(std::ptr::null_mut())}
    }

    pub fn scan_vm_specific_roots(&self, mut factory: impl RootsWorkFactory<SOMSlot> + Sized) {
        info!("calling scan_vm_specific_roots");
        
        unsafe {
            let frame_to_scan = (*INTERPRETER_RAW_PTR).current_frame;
            let to_process: Vec<SOMSlot> = vec![SOMSlot::from_address(frame_to_scan.ptr)];
            dbg!(&to_process);
            factory.create_process_roots_work(to_process)
        }
    }

    pub fn scan_roots_in_mutator_thread(&self, _mutator: &mut Mutator<SOMVM>, _factory: impl RootsWorkFactory<SOMSlot> + Sized) {
        info!("calling scan_roots_in_mutator_thread (DOES NOTHING AT THE MOMENT");
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

    #[inline(always)]
    pub fn as_ref(&self) -> &T {
        debug_assert!(!self.ptr.is_zero());
        unsafe { self.ptr.as_ref() }
    }
    
    /// Hacks for convenience, since I'm refactoring from Refcounts. TODO remove
    #[inline(always)]
    pub fn borrow(&self) -> &mut T {
        Self::to_obj(self)
    }

    /// same deal
    #[inline(always)]
    pub fn borrow_mut(&self) -> &mut T {
        Self::to_obj(self)
    }

    /// same dealll
    #[inline(always)]
    pub fn as_ptr(&self) -> &mut T {
        Self::to_obj(self)
    }

    /// Does the address not point to any data?
    /// We use this to avoid using an Option type in interpreter frames. Not sure if it's worth it though.
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.ptr.is_zero() }

    /// Convert a u64 into an address. Useful since we use NaN boxing, which returns values as 64 bits.
    #[inline(always)]
    pub fn from_u64(ptr: u64) -> GCRef<T> {
        unsafe {
            GCRef {
                ptr: Address::from_usize(ptr as usize),
                _phantom: PhantomData
            }
        }
    }
}

impl<T> GCRef<T> {
    // Allocates a type on the heap and returns a pointer to it.
    pub fn alloc(obj: T, gc_interface: &mut GCInterface) -> GCRef<T> {
        Self::alloc_with_size(obj, gc_interface, size_of::<T>())
        // Self::alloc_with_size(obj, gc_interface, size_of::<T>())
    }

    // Allocates a type, but with a given size. Useful when an object needs more than what we tell Rust through defining a struct. 
    // (e.g. Value arrays stored directly in the heap - see BC Frame)
    pub fn alloc_with_size(obj: T, gc_interface: &mut GCInterface, size: usize) -> GCRef<T> {
        Self::alloc_with_size_cached_allocator(obj, gc_interface, size)
        // Self::alloc_with_size_allocator_uncached(obj, gc_interface, size)
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn alloc_with_size_allocator_uncached(obj: T, gc_interface: &mut GCInterface, size: usize) -> GCRef<T> {
        let mutator = gc_interface.mutator.as_mut();
        let addr = mmtk_alloc(mutator, size, GC_ALIGN, GC_OFFSET, GC_SEMANTICS);
        debug_assert!(!addr.is_zero());

        // println!("{}", mmtk_free_bytes());

        // AFAIK, this is not needed.
        // mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, GC_SEMANTICS);

        unsafe {
            *addr.as_mut_ref() = obj;
        }

        GCRef {
            ptr: addr,
            _phantom: PhantomData,
        }
    }
    
    fn alloc_with_size_cached_allocator(obj: T, gc_interface: &mut GCInterface, size: usize) -> GCRef<T> {
        debug_assert!(size >= MIN_OBJECT_SIZE);
        let allocator = unsafe {&mut (*gc_interface.default_allocator)};
        let addr = allocator.alloc(size, GC_ALIGN, GC_OFFSET);
        debug_assert!(!addr.is_zero());


        // let obj = SOMVM::object_start_to_ref(addr);
        // let space = allocator.get_space();
        // debug_assert!(!obj.to_raw_address().is_zero());
        // space.initialize_object_metadata(obj, true);

        allocator.get_space().initialize_object_metadata(SOMVM::object_start_to_ref(addr), true);

        unsafe {
            //*(addr.sub(1).as_mut_ref()) = 424242;
            *addr.as_mut_ref() = obj;
            
            // let header_ref: *mut usize = addr.as_mut_ref();
            // *header_ref = 42424242; // set the header value? TODO: are we doing this right?
            // *(SOMVM::object_start_to_ref(addr).to_raw_address().as_mut_ref()) = obj;
        }

        GCRef {
            ptr: addr,
            _phantom: PhantomData,
        }
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

// for convenience, but easily removable
impl GCRef<String> {
    pub fn as_str(&self) -> &str {
        self.to_obj().as_str()
    }
}
