use std::marker::PhantomData;
use mmtk::Mutator;
use mmtk::util::Address;
use som_gc::SOMVM;

/// A pointer to the heap for GC.
#[derive(Debug, Clone, PartialEq)]
pub struct GCRef<T> {
    pub ptr: Address,
    pub _phantom: PhantomData<T>
}

/// Trait used by all GCRef pointers to convert to/from objects. TODO should only be implementable by GCRef<T>, and MUST be implemented by all GCRef<T>
pub trait GCPtr<T> {
    // Turn a GC pointer back into the type itself (as a reference)
    fn ptr_to_obj(&self) -> &mut T;

    // Allocates a type on the heap and returns a pointer to it
    fn alloc(obj: T, mutator: &mut Mutator<SOMVM>) -> Self;
}