use crate::frame::Frame;
use crate::gc::object_model::{VMObjectModel, GC_MAGIC_FRAME, OBJECT_HEADER_OFFSET};
use crate::gc::SOMSlot;
use crate::gc::SOMVM;
use crate::MMTK_TO_VM_INTERFACE;
use log::info;
use mmtk::util::opaque_pointer::*;
use mmtk::util::ObjectReference;
use mmtk::vm::SlotVisitor;
use mmtk::vm::{ObjectModel, Scanning};
use mmtk::vm::{ObjectTracer, RootsWorkFactory};
use mmtk::Mutator;

pub struct VMScanning {}

// Documentation: https://docs.mmtk.io/api/mmtk/vm/scanning/trait.Scanning.html
impl Scanning<SOMVM> for VMScanning {
    fn scan_object<SV: SlotVisitor<SOMSlot>>(
        _tls: VMWorkerThread,
        object: ObjectReference,
        slot_visitor: &mut SV,
    ) {
        info!("entering scan_object");

        unsafe {
            // let _ptr: *mut usize = unsafe { obj_addr.as_mut_ref() };
            let gc_id: &usize = VMObjectModel::ref_to_header(object).as_ref();
            
            if *gc_id == GC_MAGIC_FRAME.into() {
                info!("scan_object: frame type");
                let frame: &mut Frame = object.to_raw_address().as_mut_ref();
                debug_assert!(!frame.current_method.to_obj().signature.is_empty()); // rough way of checking with reasonable certainty that the cast to a frame succeeded
                if !frame.prev_frame.is_empty() {
                    slot_visitor.visit_slot(SOMSlot::from_address(frame.prev_frame.ptr));
                    
                    let method_slot = SOMSlot::from_address(frame.current_method.ptr.sub(OBJECT_HEADER_OFFSET));
                    dbg!(method_slot);
                    slot_visitor.visit_slot(method_slot)
                }
            } else {
                info!("scanning something that isn't a frame?")
            }
             
        }
    }

    fn scan_object_and_trace_edges<OT: ObjectTracer>(_tls: VMWorkerThread, _object: ObjectReference, _object_tracer: &mut OT) {
        todo!()
    }

    fn notify_initial_thread_scan_complete(_partial_scan: bool, _tls: VMWorkerThread) {
        unimplemented!()
    }
    fn scan_roots_in_mutator_thread(
        _tls: VMWorkerThread,
        mutator: &'static mut Mutator<SOMVM>,
        factory: impl RootsWorkFactory<SOMSlot>,
    ) {
        unsafe { (*MMTK_TO_VM_INTERFACE).scan_roots_in_mutator_thread(mutator, factory) }
    }
    fn scan_vm_specific_roots(_tls: VMWorkerThread, factory: impl RootsWorkFactory<SOMSlot>) {
        unsafe { (*MMTK_TO_VM_INTERFACE).scan_vm_specific_roots(factory) }
    }
    fn supports_return_barrier() -> bool {
        unimplemented!()
    }
    fn prepare_for_roots_re_scanning() {
        unimplemented!()
    }
}
