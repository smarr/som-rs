use mmtk::Mutator;
use crate::api::*;
use mmtk::util::opaque_pointer::*;
use crate::SOMVM;

pub fn init_gc() -> Box<Mutator<SOMVM>> {
    let mut builder = mmtk_create_builder();
    mmtk_init(&mut builder);

    let success = mmtk_set_fixed_heap_size(&mut builder, 1048576);
    assert!(success);

    let success = mmtk_set_option_from_string(&mut builder, "plan", "SemiSpace");
    assert!(success);

    let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED)); // FIXME: Use the actual thread pointer or identifier
    let mutator = mmtk_bind_mutator(tls);
    
    mutator
}