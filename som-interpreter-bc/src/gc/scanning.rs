use crate::class::Class;
use crate::frame::Frame;
use crate::gc::object_model::{VMObjectModel, GC_MAGIC_CLASS, GC_MAGIC_FRAME, GC_MAGIC_METHOD};
use crate::gc::SOMSlot;
use crate::gc::SOMVM;
use crate::method::Method;
use crate::MMTK_TO_VM_INTERFACE;
use log::info;
use mmtk::util::opaque_pointer::*;
use mmtk::util::{Address, ObjectReference};
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
                info!("frame is: {}", &frame.current_method.to_obj().signature);

                debug_assert!(!frame.current_method.to_obj().signature.is_empty()); // rough way of checking with reasonable certainty that the cast to a frame succeeded
                if !frame.prev_frame.is_empty() {
                    let prev_frame_slot_addr = Address::from_ref(&frame.prev_frame);
                    slot_visitor.visit_slot(SOMSlot::from_address(prev_frame_slot_addr));
                }

                let method_slot_addr = Address::from_ref(&frame.current_method);
                slot_visitor.visit_slot(SOMSlot::from_address(method_slot_addr))

            } else if *gc_id == GC_MAGIC_METHOD.into() {
                info!("scan_object: method type");
                let method: &mut Method = object.to_raw_address().as_mut_ref();

                // kind doesn't contain GCRefs, nothing to do.
                match method.kind { _ => {} }

                let holder_slot_addr = Address::from_ref(&method.holder);
                slot_visitor.visit_slot(SOMSlot::from_address(holder_slot_addr))
            }
            else if *gc_id == GC_MAGIC_CLASS.into() {
                info!("scan_object: class type");
                let class: &mut Class = object.to_raw_address().as_mut_ref();

                slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(&class.class)));

                // if let Some(super_cls) = class.super_class {
                //     slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(&class.super_class)));
                // }

                // for (_, method_ref) in class.methods.iter() {
                //     slot_visitor.visit_slot(SOMSlot::from_address(Address::from_ref(&method_ref)))
                // }

            }
            else {
                todo!("scanning something of an unhandled type?")
                // info!("scanning something of an unhandled type?")
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

#[allow(dead_code)]
/// Taken from the Julia code.
fn slot_checker_dbg(slot: SOMSlot) {
    {
        use mmtk::vm::slot::Slot;

        if let Some(objref) = slot.load() {
            debug_assert!(
                mmtk::memory_manager::is_in_mmtk_spaces(objref),
                "Object {:?} in slot {:?} is not mapped address",
                objref,
                slot
            );

            let raw_addr_usize = objref.to_raw_address().as_usize();
            debug_assert!(
                raw_addr_usize % 16 == 0 || raw_addr_usize % 8 == 0,
                "Object {:?} in slot {:?} is not aligned to 8 or 16",
                objref,
                slot
            );
        }
    }
}