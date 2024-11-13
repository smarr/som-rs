use crate::gcref::Gc;
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::slot::{SimpleSlot, Slot};

// pub type SOMSlot = mmtk::vm::slot::SimpleSlot;

// because of NaN boxing, we make a new slot specifically for accessing values, which contain internally a GCRef
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SOMSlot {
    Simple(SimpleSlot),
    Value(ValueSlot),
}

impl SOMSlot {
    pub fn from_address(addr: Address) -> SOMSlot {
        SOMSlot::Simple(SimpleSlot::from_address(addr))
    }

    pub fn from_value(value: u64) -> SOMSlot {
        SOMSlot::Value(ValueSlot::from_value(value))
    }
}

impl Slot for SOMSlot {
    fn load(&self) -> Option<ObjectReference> {
        match self {
            SOMSlot::Simple(e) => e.load(),
            SOMSlot::Value(e) => e.load(),
        }
    }

    fn store(&self, object: ObjectReference) {
        match self {
            SOMSlot::Simple(e) => e.store(object),
            SOMSlot::Value(e) => e.store(object),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueSlot {
    value: u64, // should be a pointer instead, probably (but that might be slower?). using the non nan boxed val makes slots MASSIVE otherwise.
}

impl ValueSlot {
    pub fn from_value(value: u64) -> Self {
        Self { value }
    }
}

unsafe impl Send for ValueSlot {}

impl Slot for ValueSlot {
    fn load(&self) -> Option<ObjectReference> {
        // debug_assert!(self.value.is_ptr_type());
        // let gcref: GCRef<()> = self.value.extract_gc_cell();
        let gcref: Gc<()> = Gc::from_u64((((self.value << 16) as i64) >> 16) as u64);
        unsafe { ObjectReference::from_raw_address(Address::from_usize(gcref.ptr)) }
    }

    fn store(&self, _object: ObjectReference) {
        unimplemented!()
    }
}
