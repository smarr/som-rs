use crate::{SINGLETON, SOMVM};
use mmtk::util::opaque_pointer::*;
use mmtk::vm::Collection;
use mmtk::vm::GCThreadContext;
use mmtk::Mutator;

pub struct VMCollection {}

// Documentation: https://docs.mmtk.io/api/mmtk/vm/collection/trait.Collection.html
impl Collection<SOMVM> for VMCollection {
    fn stop_all_mutators<F>(_tls: VMWorkerThread, _mutator_visitor: F)
    where
        F: FnMut(&'static mut Mutator<SOMVM>),
    {
        unimplemented!()
    }

    fn resume_mutators(_tls: VMWorkerThread) {
        unimplemented!()
    }

    fn block_for_gc(_tls: VMMutatorThread) {
        unimplemented!()
    }

    fn spawn_gc_thread(_tls: VMThread, ctx: GCThreadContext<SOMVM>) {
        // unimplemented!()
        
        // copied from julia mmtk code
        // Just drop the join handle. The thread will run until the process quits.
        let _ = std::thread::spawn(move || {
            // let worker_tls = VMWorkerThread(VMThread(OpaquePointer::from_address(unsafe {
            //     Address::from_usize(thread_id::get())
            // })));

            let worker_tls = VMWorkerThread(VMThread(OpaquePointer::UNINITIALIZED));
            match ctx {
                GCThreadContext::Worker(w) => {
                    mmtk::memory_manager::start_worker(SINGLETON.get().unwrap(), worker_tls, w)
                }
            }
        });

    }
}
