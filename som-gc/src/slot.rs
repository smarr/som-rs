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

    pub fn to_address(&self) -> Address {
        unsafe { Address::from_usize((((*self.value << 16) as i64) >> 16) as usize) }
    }

    /// For debugging purposes. Copies code from som-core/value.rs, naively
    pub fn is_ptr_type(val: u64) -> bool {
        const BASE_TAG: u64 = 0x7FF8;
        const CELL_BASE_TAG: u64 = 0x8000 | BASE_TAG;
        const TAG_SHIFT: u64 = 48;
        const IS_PTR_PATTERN: u64 = CELL_BASE_TAG << TAG_SHIFT;

        (val & IS_PTR_PATTERN) == IS_PTR_PATTERN
    }
}

unsafe impl Send for RefValueSlot {}

impl Slot for RefValueSlot {
    fn load(&self) -> Option<ObjectReference> {
        // debug!("refvalueslot load ok");
        unsafe { debug_assert!(Self::is_ptr_type(*self.value)) }
        let addr = self.to_address();
        ObjectReference::from_raw_address(addr)
    }

    fn store(&self, object: ObjectReference) {
        // debug!("refvalueslot store ok");
        // debug_assert!(Self::is_ptr_type(object.to_raw_address().as_usize() as u64))
        let addr = self.to_address();
        unsafe { debug_assert!(Self::is_ptr_type(*self.value)) }
        unsafe {
            *addr.to_mut_ptr() = object.to_raw_address().as_usize();
        }
    }
}
