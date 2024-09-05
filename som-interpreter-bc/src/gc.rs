use std::marker::PhantomData;
use mmtk::util::Address;

#[derive(Clone, PartialEq)]
pub struct GCRef<T> {
    pub ptr: Address,
    pub _phantom: PhantomData<T>
}