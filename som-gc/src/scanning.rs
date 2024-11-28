use crate::slot::SOMSlot;
use crate::{MMTK_TO_VM_INTERFACE, SOMVM};
use log::debug;
use mmtk::util::opaque_pointer::*;
use mmtk::util::ObjectReference;
use mmtk::vm::Scanning;
use mmtk::vm::SlotVisitor;
use mmtk::vm::{ObjectTracer, RootsWorkFactory};
use mmtk::Mutator;

pub struct VMScanning {}

// Documentation: https://docs.mmtk.io/api/mmtk/vm/scanning/trait.Scanning.html
impl Scanning<SOMVM> for VMScanning {
    fn scan_object<SV: SlotVisitor<SOMSlot>>(_tls: VMWorkerThread, object: ObjectReference, slot_visitor: &mut SV) {
        let scan_object_callback = (&MMTK_TO_VM_INTERFACE).get().unwrap().scan_object;
        scan_object_callback(object, slot_visitor)
    }

    fn scan_object_and_trace_edges<OT: ObjectTracer>(_tls: VMWorkerThread, _object: ObjectReference, _object_tracer: &mut OT) {
        unimplemented!()
    }

    fn notify_initial_thread_scan_complete(_partial_scan: bool, _tls: VMWorkerThread) {
        // do nothing.
    }

    fn scan_roots_in_mutator_thread(_tls: VMWorkerThread, mutator: &'static mut Mutator<SOMVM>, mut factory: impl RootsWorkFactory<SOMSlot>) {
        let get_roots_fn = (&MMTK_TO_VM_INTERFACE).get().unwrap().get_roots_in_mutator_thread;
        factory.create_process_roots_work(get_roots_fn(mutator));
    }
    fn scan_vm_specific_roots(_tls: VMWorkerThread, _factory: impl RootsWorkFactory<SOMSlot>) {
        debug!("scan_vm_specific_roots (unimplemented)");
    }
    fn supports_return_barrier() -> bool {
        unimplemented!()
    }
    fn prepare_for_roots_re_scanning() {
        unimplemented!()
    }
}
