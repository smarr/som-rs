use crate::value::Value;
use crate::vm_objects::class::Class;
use crate::HACK_INSTANCE_CLASS_PTR;
use core::mem::size_of;
use som_gc::gc_interface::GCInterface;
use som_gc::gcref::Gc;
use std::fmt;
use std::marker::PhantomData;

/// Represents a generic (non-primitive) class instance.
#[derive(Clone, PartialEq)]
pub struct Instance {
    /// The class of which this is an instance from.
    pub class: Gc<Class>,
    /// We store the fields right after the instance in memory.
    pub fields_marker: PhantomData<[Value]>,
}

impl Instance {
    /// Construct an instance for a given class.
    pub fn from_class(class: Gc<Class>, gc_interface: &mut GCInterface) -> Gc<Instance> {
        let nbr_fields = class.get_nbr_fields();

        let instance = Self {
            class: Gc::default(),
            fields_marker: PhantomData,
        };

        unsafe { HACK_INSTANCE_CLASS_PTR = Some(class) }

        let post_alloc_closure = |mut instance_ref: Gc<Instance>| {
            unsafe {
                for idx in 0..nbr_fields {
                    Instance::assign_field(instance_ref, idx, Value::NIL)
                }
                instance_ref.class = HACK_INSTANCE_CLASS_PTR.unwrap();
                HACK_INSTANCE_CLASS_PTR = None;
            };
        };

        let size = size_of::<Instance>() + (nbr_fields * size_of::<Value>());
        gc_interface.alloc_with_post_init(instance, size, post_alloc_closure)
    }

    /// Get the class of which this is an instance from.
    pub fn class(&self) -> Gc<Class> {
        self.class
    }

    /// Get the superclass of this instance's class.
    pub fn super_class(&self) -> Option<Gc<Class>> {
        self.class.super_class()
    }

    #[inline(always)]
    fn get_field_ptr(ptr: usize, n: usize) -> *mut Value {
        (ptr + size_of::<Instance>() + n * size_of::<Value>()) as *mut Value
    }

    /// Lookup a field in an instance.
    pub(crate) fn lookup_field(_self: Gc<Instance>, idx: usize) -> &'static Value {
        unsafe {
            let field_ptr = Self::get_field_ptr(_self.ptr, idx);
            &*field_ptr
        }
    }

    /// Assign a field to an instance.
    pub(crate) fn assign_field(_self: Gc<Self>, idx: usize, value: Value) {
        unsafe {
            let field_ptr = Self::get_field_ptr(_self.ptr, idx);
            *field_ptr = value
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
