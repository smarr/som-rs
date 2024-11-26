use crate::gcref::Gc;
use log::debug;
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::slot::{SimpleSlot, Slot};
// pub type SOMSlot = mmtk::vm::slot::SimpleSlot;

// because of NaN boxing, we make a new slot specifically for accessing values, which contain internally a GCRef
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SOMSlot {
    Simple(SimpleSlot),
    RefValueSlot(RefValueSlot),
}

/// Most common case: turn any pointer to any heap address into a slot.
/// This pointer must be on the heap or in a static variable! Otherwise, it becomes invalid when Rust discards it.
impl<T> From<&Gc<T>> for SOMSlot {
    fn from(value: &Gc<T>) -> Self {
        SOMSlot::Simple(SimpleSlot::from_address(Address::from(value)))
    }
}

impl SOMSlot {
    /// Turn a pointer to a value type to a slot.
    /// Could be implemented as a `From` impl, but this is clearer
    pub fn from_value_ptr(value: *mut u64) -> SOMSlot {
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
    const BASE_TAG: u64 = 0x7FF8;
    const CELL_BASE_TAG: u64 = 0x8000 | Self::BASE_TAG;
    const TAG_SHIFT: u64 = 48;
    const IS_PTR_PATTERN: u64 = Self::CELL_BASE_TAG << Self::TAG_SHIFT;
    const TAG_EXTRACTION: u64 = 0xFFFF << Self::TAG_SHIFT;
    const CANON_NAN_BITS: u64 = 0x7FF8000000000000;

    pub fn from_ref(value: *mut u64) -> Self {
        Self { value }
    }

    pub fn to_address(&self) -> Address {
        unsafe { Address::from_usize((((*self.value << 16) as i64) >> 16) as usize) }
    }

    /// For debugging purposes. Copies code from som-core/value.rs, naively
    pub fn is_ptr_type(val: u64) -> bool {
        (val & Self::IS_PTR_PATTERN) == Self::IS_PTR_PATTERN
    }

    // fn extract_pointer_bits(val: u64) -> u64 {
    //     (((val << 16) as i64) >> 16) as u64
    // }

    fn tag(val: u64) -> u64 {
        (val & Self::TAG_EXTRACTION) >> Self::TAG_SHIFT
    }

    fn new_val(tag: u64, value: u64) -> u64 {
        Self::CANON_NAN_BITS | ((tag << Self::TAG_SHIFT) & Self::TAG_EXTRACTION) | (value & !Self::TAG_EXTRACTION)
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
        unsafe {
            debug_assert!(Self::is_ptr_type(*self.value));

            // let addr = self.to_address();
            let tag = Self::tag(*self.value);

            // dbg!(format!("{:x}", Self::extract_pointer_bits(*self.value)));

            // *addr.to_mut_ptr() = Self::new_val(tag, object.to_raw_address().as_usize() as u64);
            *self.value = Self::new_val(tag, object.to_raw_address().as_usize() as u64);

            // dbg!(format!("{:x}", Self::extract_pointer_bits(*self.value)));
            debug_assert!(Self::is_ptr_type(*self.value))
        }
        debug!("refvalueslot store ok");
    }
}
