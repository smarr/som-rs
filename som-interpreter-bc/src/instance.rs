use crate::class::Class;
use crate::value::Value;
use core::mem::size_of;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::{CustomAlloc, Gc};
use std::fmt;
use std::marker::PhantomData;

/// Represents a generic (non-primitive) class instance.
#[derive(Clone, PartialEq)]
pub struct Instance {
    /// The class of which this is an instance from.
    pub class: Gc<Class>,
    /// will be used for packed repr of locals
    pub nbr_fields: usize,
    /// This instance's locals. Contiguous "Value" instances in memory
    pub locals_marker: PhantomData<Vec<Value>>,
}

impl Instance {
    /// Construct an instance for a given class.
    pub fn from_class(class: Gc<Class>, mutator: &mut GCInterface) -> Gc<Instance> {
        fn get_nbr_fields(class: &Gc<Class>) -> usize {
            let mut nbr_locals = class.locals.len();
            if let Some(super_class) = class.super_class() {
                nbr_locals += get_nbr_fields(&super_class)
            }
            nbr_locals
        }

        let nbr_fields = get_nbr_fields(&class);

        let instance = Self {
            class,
            nbr_fields,
            locals_marker: PhantomData,
        };
        Instance::alloc(instance, mutator)
    }

    /// Get the class of which this is an instance from.
    pub fn class(&self) -> Gc<Class> {
        self.class
    }

    /// Get the superclass of this instance's class.
    pub fn super_class(&self) -> Option<Gc<Class>> {
        self.class.super_class()
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

impl CustomAlloc<Instance> for Instance {
    fn alloc(instance: Instance, gc_interface: &mut GCInterface) -> Gc<Self> {
        let size = size_of::<Instance>() + (instance.nbr_fields * size_of::<Value>());

        let nbr_fields = instance.nbr_fields;

        let instance_ref = gc_interface.alloc_with_size(instance, size);

        unsafe {
            let mut values_addr = (instance_ref.ptr + size_of::<Instance>()) as *mut Value;
            for _ in 0..nbr_fields {
                *values_addr = Value::NIL;
                values_addr = values_addr.wrapping_add(1);
            }
        };

        // println!("instance allocation OK");

        instance_ref
    }
}

pub trait InstanceAccess {
    // technically internally works with an MMTk Address type, and should return it. but Address is just a usize newtype, and we don't want to depend on MMTk, so we say usize.
    fn get_field_addr(&self, idx: usize) -> usize;
    fn lookup_local(&self, idx: usize) -> Value;
    fn assign_local(&mut self, idx: usize, value: Value);
}

impl InstanceAccess for Gc<Instance> {
    fn get_field_addr(&self, idx: usize) -> usize {
        self.ptr + size_of::<Instance>() + (idx * size_of::<Value>())
    }

    fn lookup_local(&self, idx: usize) -> Value {
        unsafe {
            let local_ref: &Value = &*(self.get_field_addr(idx) as *const Value);
            *local_ref
        }
    }

    fn assign_local(&mut self, idx: usize, value: Value) {
        unsafe {
            let ptr_to_local = self.get_field_addr(idx) as *mut Value;
            *ptr_to_local = value
        }
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instance")
            .field("name", &self.class.name())
            // .field("locals", &self.locals.keys())
            .finish()
    }
}
