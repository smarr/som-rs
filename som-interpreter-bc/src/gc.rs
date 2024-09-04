use mmtk::AllocationSemantics;
use mmtk::util::ObjectReference;
use som_gc::api::{mmtk_alloc, mmtk_post_alloc};
use som_gc::SOMVM;
use crate::instance::Instance;

#[derive(Clone, PartialEq)]
pub struct GCRefToInstance(usize);
// TODO pub struct GCRef<T>(usize) instead? do we need phantomdata though?

impl GCRefToInstance {
    pub fn to_instance(&self) -> &mut Instance {
        unsafe { &mut *(self.0 as *mut Instance) }
    }

    pub fn from_instance(instance: Instance, mutator: *mut mmtk::Mutator<SOMVM>) -> Self {
        let size = size_of::<Instance>();
        let align = 8;
        let offset = 0;
        let semantics = AllocationSemantics::Default;
        
        let addr = mmtk_alloc(mutator, size, align, offset, semantics);
        
        mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, semantics);
        
        unsafe {
            std::ptr::copy(&instance, addr.as_usize() as *mut Instance, size); // reaaally not sure about this one
        }
        
        Self(addr.as_usize())
    }
}