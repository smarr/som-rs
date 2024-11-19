use mmtk::util::{Address, ObjectReference};
use mmtk::vm::slot::{SimpleSlot, Slot};
// pub type SOMSlot = mmtk::vm::slot::SimpleSlot;

// because of NaN boxing, we make a new slot specifically for accessing values, which contain internally a GCRef
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SOMSlot {
    Simple(SimpleSlot),
    RefValueSlot(RefValueSlot),
}

impl SOMSlot {
    pub fn from_address(addr: Address) -> SOMSlot {
        SOMSlot::Simple(SimpleSlot::from_address(addr))
    }

    pub fn from_ref(value: *mut u64) -> SOMSlot {
        SOMSlot::RefValueSlot(RefValueSlot::from_ref(value))
    }
}

impl Slot for SOMSlot {
    fn load(&self) -> Option<ObjectReference> {
        match self {
            SOMSlot::Simple(e) => e.load(),
            SOMSlot::RefValueSlot(e) => e.load(),
        }
    }

    fn store(&self, object: ObjectReference) {
        match self {
            SOMSlot::Simple(e) => e.store(object),
            SOMSlot::RefValueSlot(e) => e.store(object),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RefValueSlot {
    value: *mut u64,
}

impl RefValueSlot {
    pub fn from_ref(value: *mut u64) -> Self {
        Self { value }
    }
}

unsafe impl Send for RefValueSlot {}

impl Slot for RefValueSlot {
    fn load(&self) -> Option<ObjectReference> {
        unsafe {
            let a = (((*self.value << 16) as i64) >> 16) as usize;
            ObjectReference::from_raw_address(Address::from_usize(a))
        }
    }

    fn store(&self, object: ObjectReference) {
        unsafe {
            let a = (((*self.value << 16) as i64) >> 16) as usize as *mut usize;
            *a = object.to_raw_address().as_usize();
        }
        // unsafe { (MMTK_TO_VM_INTERFACE.get().unwrap().store_in_value_fn)(self.value, object) }
    }
}
