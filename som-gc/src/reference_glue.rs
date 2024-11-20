use crate::SOMVM;
use mmtk::util::opaque_pointer::VMWorkerThread;
use mmtk::util::ObjectReference;
use mmtk::vm::ReferenceGlue;

pub struct VMReferenceGlue {}

// Documentation: https://docs.mmtk.io/api/mmtk/vm/reference_glue/trait.ReferenceGlue.html
impl ReferenceGlue<SOMVM> for VMReferenceGlue {
    type FinalizableType = ObjectReference;

    fn set_referent(_reference: ObjectReference, _referent: ObjectReference) {
        dbg!("wahii");
    }
    fn get_referent(_object: ObjectReference) -> Option<ObjectReference> {
        dbg!("wahoo");
        None
        // unimplemented!()
    }
    fn clear_referent(_object: ObjectReference) {
        unimplemented!()
    }
    fn enqueue_references(_references: &[ObjectReference], _tls: VMWorkerThread) {
        unimplemented!()
    }
}
