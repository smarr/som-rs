use mmtk::Mutator;
use crate::api::*;
use mmtk::util::opaque_pointer::*;
use crate::{SINGLETON, SOMVM};

pub fn init_gc() -> (VMMutatorThread, Box<Mutator<SOMVM>>) {
    if SINGLETON.get().is_none() {
        let mut builder = mmtk_create_builder();

        // let heap_success = mmtk_set_fixed_heap_size(&mut builder, 1048576);
        // assert!(heap_success, "Couldn't set MMTk fixed heap size");

        let gc_success = builder.set_option("plan", "SemiSpace");
        assert!(gc_success, "Couldn't set GC plan");

        // builder.options.threads.set(1);

        mmtk_init(&mut builder);
        // let worked_thread = VMWorkerThread(VMThread(OpaquePointer::UNINITIALIZED));
        mmtk_initialize_collection(VMThread(OpaquePointer::UNINITIALIZED));
    }
    
    let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED)); // FIXME: Use the actual thread pointer or identifier
    let mutator = mmtk_bind_mutator(tls);
    
    (tls, mutator)
}