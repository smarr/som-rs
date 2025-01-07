use crate::{mmtk, SOMVM};
use mmtk::memory_manager;
use mmtk::util::opaque_pointer::*;
use mmtk::util::VMMutatorThread;
use mmtk::util::{Address, ObjectReference};
use mmtk::AllocationSemantics;
use mmtk::MMTKBuilder;
use mmtk::Mutator;

// More APIs: https://docs.mmtk.io/api/mmtk/memory_manager/index.html.

pub fn mmtk_create_builder() -> MMTKBuilder {
    mmtk::MMTKBuilder::new()
}

pub fn mmtk_set_fixed_heap_size(builder: &mut MMTKBuilder, heap_size: usize) -> bool {
    builder.options.gc_trigger.set(mmtk::util::options::GCTriggerSelector::FixedHeapSize(heap_size))
}

pub fn mmtk_bind_mutator(tls: VMMutatorThread) -> Box<Mutator<SOMVM>> {
    memory_manager::bind_mutator(mmtk(), tls)
}

pub fn mmtk_destroy_mutator(mutator: &mut Mutator<SOMVM>) {
    memory_manager::destroy_mutator(mutator);
}

pub fn mmtk_alloc(mutator: &mut Mutator<SOMVM>, size: usize, align: usize, offset: usize, semantics: AllocationSemantics) -> Address {
    memory_manager::alloc::<SOMVM>(mutator, size, align, offset, semantics)
}

pub fn mmtk_post_alloc(mutator: &mut Mutator<SOMVM>, refer: ObjectReference, bytes: usize, semantics: AllocationSemantics) {
    memory_manager::post_alloc::<SOMVM>(mutator, refer, bytes, semantics)
}

pub fn mmtk_initialize_collection(tls: VMThread) {
    memory_manager::initialize_collection(mmtk(), tls)
}

pub fn mmtk_used_bytes() -> usize {
    memory_manager::used_bytes(mmtk())
}

pub fn mmtk_free_bytes() -> usize {
    memory_manager::free_bytes(mmtk())
}

pub fn mmtk_total_bytes() -> usize {
    memory_manager::total_bytes(mmtk())
}

pub fn mmtk_handle_user_collection_request(tls: VMMutatorThread) -> bool {
    memory_manager::handle_user_collection_request::<SOMVM>(mmtk(), tls)
}
