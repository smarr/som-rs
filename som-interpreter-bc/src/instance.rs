use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use mmtk::AllocationSemantics;
use som_gc::api::{mmtk_alloc, mmtk_post_alloc};
use som_gc::SOMVM;
use crate::class::Class;
use crate::gc::GCRefToInstance;
use crate::value::Value;
use crate::SOMRef;

/// Represents a generic (non-primitive) class instance.
#[derive(Clone)]
pub struct Instance {
    /// The class of which this is an instance from.
    pub class: *mut Class,
    /// This instance's locals.
    pub locals: Vec<Value>,
}

impl Instance {
    /// Construct an instance for a given class.
    pub fn from_class(class: SOMRef<Class>, mutator: *mut mmtk::Mutator<SOMVM>) -> GCRefToInstance {
        let mut locals = Vec::new();

        fn collect_locals(class: &SOMRef<Class>, locals: &mut Vec<Value>) {
            if let Some(class) = class.borrow().super_class() {
                collect_locals(&class, locals);
            }
            locals.extend(class.borrow().locals.iter().map(|_| Value::Nil));
        }

        collect_locals(&class, &mut locals);

        // let locals = class.borrow().locals.iter().map(|_| Value::Nil).collect();

        let instance = Self { class: class.as_ptr(), locals };
        Self::alloc_instance(instance, mutator)
    }

    fn alloc_instance(instance: Instance, mutator: *mut mmtk::Mutator<SOMVM>) -> GCRefToInstance {
        let size = std::mem::size_of::<Instance>();
        let align= 8;
        let offset= 0;
        let semantics = AllocationSemantics::Default;

        let addr = mmtk_alloc(mutator, size, align, offset, semantics);
        debug_assert!(!addr.is_zero());

        mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(addr), size, semantics);

        // dbg!(&size);
        // dbg!(size_of::<Instance>());
        
        unsafe {
            let instance_ptr = addr.as_usize() as *mut Instance;
            *instance_ptr = instance 
        };

        // println!("allocation OK");

        GCRefToInstance(addr.as_usize())
    }

    pub fn from_gc_ptr(gc_ptr: &GCRefToInstance) -> &mut Instance {
        unsafe { &mut *(gc_ptr.0 as *mut Instance) }
    }

    /// Get the class of which this is an instance from.
    pub fn class(&self) -> SOMRef<Class> {
        Rc::new(RefCell::new(unsafe { (*self.class).clone() })) // todo this is stupid, but otherwise every SOMRef<Class> in related code must be turned into a pointer. which I will do tbh, I have to
    }

    /// Get the superclass of this instance's class.
    pub fn super_class(&self) -> Option<SOMRef<Class>> {
        unsafe { (*self.class).super_class() }
    }

    /// Search for a local binding.
    pub fn lookup_local(&self, idx: usize) -> Value {
        unsafe { self.locals.get_unchecked(idx).clone() }
    }

    /// Assign a value to a local binding.
    pub fn assign_local(&mut self, idx: usize, value: Value) {
        unsafe { *self.locals.get_unchecked_mut(idx) = value; }
    }
    
    /// Checks whether there exists a local binding of a given index.
    pub fn has_local(&self, idx: usize) -> bool {
        idx < self.locals.len()
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instance")
            .field("name", &unsafe {&*self.class}.name())
            // .field("locals", &self.locals.keys())
            .finish()
    }
}
