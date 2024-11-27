use crate::value::Value;
use crate::vm_objects::class::Class;
use crate::HACK_INSTANCE_CLASS_PTR;
use core::mem::size_of;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;
use std::fmt;

/// Represents a generic (non-primitive) class instance.
#[derive(Clone, PartialEq)]
pub struct Instance {
    /// The class of which this is an instance from.
    pub class: Gc<Class>,
    /// Pointer to the fields of this instance
    pub fields_ptr: *mut Value,
}

impl Instance {
    /// Construct an instance for a given class.
    pub fn from_class(class: Gc<Class>, gc_interface: &mut GCInterface) -> Gc<Instance> {
        let nbr_fields = class.get_nbr_fields();

        let instance = Self {
            class: Gc::default(),
            fields_ptr: std::ptr::null_mut(),
        };

        unsafe { HACK_INSTANCE_CLASS_PTR = Some(class) }

        let post_alloc_closure = |mut instance_ref: Gc<Instance>| {
            unsafe {
                let mut values_addr = (instance_ref.ptr + size_of::<Instance>()) as *mut Value;
                instance_ref.fields_ptr = values_addr;
                for _ in 0..nbr_fields {
                    *values_addr = Value::NIL;
                    values_addr = values_addr.wrapping_add(1);
                }

                instance_ref.class = HACK_INSTANCE_CLASS_PTR.unwrap();
                HACK_INSTANCE_CLASS_PTR = None;
            };
        };

        let size = size_of::<Instance>() + (nbr_fields * size_of::<Value>());
        gc_interface.alloc_with_post_init(instance, size, post_alloc_closure)
    }

    // /// Construct an instance for a given class.
    // pub fn from_static_class(class: Gc<Class>, mutator: &mut GCInterface) -> Gc<Instance> {
    //     let instance = Self {
    //         class,
    //         fields_ptr: std::ptr::null_mut(),
    //     };
    //
    //     let mut instance_ref = mutator.alloc(instance);
    //
    //     // instance_ref.fields_ptr = class.fields.as_mut_slice();
    //
    //     instance_ref
    // }

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

    pub(crate) fn lookup_field(&self, idx: usize) -> &Value {
        unsafe {
            let local_ref = self.fields_ptr.add(idx);
            &*local_ref
        }
    }

    pub(crate) fn assign_field(&mut self, idx: usize, value: Value) {
        unsafe {
            let ptr_to_local = self.fields_ptr.add(idx);
            *ptr_to_local = value
        }
    }

    pub fn get_nbr_fields(&self) -> usize {
        self.class.get_nbr_fields()
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
