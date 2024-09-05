use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use mmtk::AllocationSemantics;
use som_gc::api::{mmtk_alloc, mmtk_post_alloc};
use som_gc::SOMVM;
use crate::class::Class;
use crate::value::Value;
use crate::SOMRef;
use core::mem::size_of;
use mmtk::util::Address;
use crate::gc::GCRef;

/// Represents a generic (non-primitive) class instance.
#[derive(Clone, PartialEq)]
pub struct Instance {
    /// The class of which this is an instance from.
    pub class: *mut Class,
    /// will be used for packed repr of locals
    pub nbr_fields: usize,
    // /// This instance's locals.
    pub locals_marker: ()
}

// pub struct InstanceLayout {
//     pub class: *mut Class,
//     pub nbr_fields: usize,
//     pub idk: usize // sizeof::Value * nbr_fields
// }

impl Instance {
    /// Construct an instance for a given class.
    pub fn from_class(class: SOMRef<Class>, mutator: *mut mmtk::Mutator<SOMVM>) -> GCRef<Instance> {
        fn get_nbr_fields(class: &SOMRef<Class>) -> usize {
            let mut nbr_locals = class.borrow().locals.len();
            if let Some(super_class) = class.borrow().super_class() {
                nbr_locals += get_nbr_fields(&super_class)
            }
            nbr_locals
        }

        let nbr_fields = get_nbr_fields(&class);

        let instance = Self { class: class.as_ptr(), nbr_fields, locals_marker: () };
        Self::alloc_instance(instance, nbr_fields, mutator)
    }

    fn alloc_instance(instance: Instance, nbr_fields: usize, mutator: *mut mmtk::Mutator<SOMVM>) -> GCRef<Instance> {
        let size = size_of::<Instance>() + (instance.nbr_fields * size_of::<Value>());
        // let size = std::mem::size_of::<Instance>();
        let align= 8;
        let offset= 0;
        let semantics = AllocationSemantics::Default;

        let instance_addr = mmtk_alloc(mutator, size, align, offset, semantics);
        debug_assert!(!instance_addr.is_zero());

        mmtk_post_alloc(mutator, SOMVM::object_start_to_ref(instance_addr), size, semantics);

        // dbg!(&size);
        // dbg!(size_of::<Instance>());
        
        let mut gc_ref_to_instance = GCRef {
            ptr: instance_addr,
            _phantom: PhantomData
        };
        
        unsafe {
            let class_ptr = instance_addr.as_mut_ref();
            *class_ptr = instance.class;
            let nbr_fields_addr = instance_addr.add(size_of::<*mut Class>()).as_mut_ref();
            *nbr_fields_addr = instance.nbr_fields;

            // let mut values_addr = instance_addr.add(size_of::<*mut Class>()).add(size_of::<usize>());
            // dbg!(&instance_addr);
            // dbg!(&values_addr);
            // dbg!(size_of::<Value>());
            
            for i in 0..nbr_fields {
                gc_ref_to_instance.assign_local(i, Value::Nil); // todo do a memset! not sure there isn't a bug right now.
                // *values_addr.as_mut_ref() = Value::Nil;
                // values_addr = values_addr.add(size_of::<Value>());
            }
        };

        // println!("allocation OK");

        gc_ref_to_instance
    }

    pub fn from_gc_ptr(gc_ptr: &GCRef<Instance>) -> &mut Instance {
        unsafe { &mut *(gc_ptr.ptr.as_mut_ref()) }
    }

    /// Get the class of which this is an instance from.
    pub fn class(&self) -> SOMRef<Class> {
        Rc::new(RefCell::new(unsafe { (*self.class).clone() })) // todo this is stupid, but otherwise every SOMRef<Class> in related code must be turned into a pointer. which I will have to do at some point tbh, I have to
    }

    /// Get the superclass of this instance's class.
    pub fn super_class(&self) -> Option<SOMRef<Class>> {
        unsafe { (*self.class).super_class() }
    }

    // /// Search for a local binding.
    // pub fn lookup_local(&self, idx: usize) -> Value {
    //     unsafe { self.locals.get_unchecked(idx).clone() }
    // }
    //
    // /// Assign a value to a local binding.
    // pub fn assign_local(&mut self, idx: usize, value: Value) {
    //     unsafe { *self.locals.get_unchecked_mut(idx) = value; }
    // }
    
    /// Checks whether there exists a local binding of a given index.
    pub fn has_local(&self, idx: usize) -> bool {
        idx < self.nbr_fields
    }
}

impl GCRef<Instance> {
    fn get_field_addr(&self, idx: usize) -> Address {
        self.ptr.add(size_of::<*mut Class>()).add(idx * size_of::<Value>())
    }

    pub fn lookup_local(&self, idx: usize) -> Value {
        unsafe { 
            let local_ref: &Value = self.get_field_addr(idx).as_ref();
            local_ref.clone() 
        }
    }

    pub fn assign_local(&mut self, idx: usize, value: Value) {
        unsafe {
            // dbg!(&value);
            let ptr_to_local = self.get_field_addr(idx).as_mut_ref();
            *ptr_to_local = value
        }
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
