use mmtk::Mutator;
use crate::api::*;
use mmtk::util::opaque_pointer::*;
use crate::SOMVM;

pub fn init_gc() -> (VMMutatorThread, Box<Mutator<SOMVM>>) {
    let mut builder = mmtk_create_builder();
    mmtk_init(&mut builder);

    let success = mmtk_set_fixed_heap_size(&mut builder, 1048576);
    assert!(success, "Couldn't set MMTk fixed heap size");

    let success = mmtk_set_option_from_string(&mut builder, "plan", "SemiSpace");
    assert!(success, "Couldn't set GC plan");

    // todo remove/change?
    builder.options.threads.set(1);

    // VMMutatorThread(VMThread::)
    let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED)); // FIXME: Use the actual thread pointer or identifier
    let mutator = mmtk_bind_mutator(tls);

    // let worked_thread = VMWorkerThread(VMThread(OpaquePointer::UNINITIALIZED));
    mmtk_initialize_collection(VMThread(OpaquePointer::UNINITIALIZED));

    (tls, mutator)
}