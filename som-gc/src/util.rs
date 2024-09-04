use std::ffi::CString;
use crate::api::*;
use mmtk::util::opaque_pointer::*;

pub fn init_gc() -> crate::SOMVM {
    let builder = mmtk_create_builder();
    mmtk_init(builder);

    // Set option by value using extern "C" wrapper.
    let success = mmtk_set_fixed_heap_size(builder, 1048576);
    assert!(success);

    // not sure that's needed, but we *are* doing NoGC for now. CStrings DEFINITELY not needed.
    let name = CString::new("plan").unwrap();
    let val = CString::new("NoGC").unwrap();
    let success = mmtk_set_option_from_string(builder, name.as_ptr(), val.as_ptr());
    assert!(success);

    let tls = VMMutatorThread(VMThread(OpaquePointer::UNINITIALIZED)); // FIXME: Use the actual thread pointer or identifier
    let _mutator = mmtk_bind_mutator(tls);

    let vm = crate::SOMVM::default();

    vm
}