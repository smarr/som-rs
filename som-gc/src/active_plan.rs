use crate::SOMVM;
use mmtk::util::opaque_pointer::*;
use mmtk::vm::ActivePlan;
use mmtk::Mutator;

pub struct VMActivePlan {}

// Documentation: https://docs.mmtk.io/api/mmtk/vm/active_plan/trait.ActivePlan.html
impl ActivePlan<SOMVM> for VMActivePlan {
    fn number_of_mutators() -> usize {
        unimplemented!()
    }

    fn is_mutator(_tls: VMThread) -> bool {
        // FIXME: Properly check if the thread is a mutator
        true
    }

    fn mutator(_tls: VMMutatorThread) -> &'static mut Mutator<SOMVM> {
        unimplemented!()
    }

    fn mutators<'a>() -> Box<dyn Iterator<Item = &'a mut Mutator<SOMVM>> + 'a> {
        unimplemented!()
    }
}
