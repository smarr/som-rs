use std::marker::PhantomData;

#[derive(Clone, PartialEq)]
pub struct GCRef<T> {
    pub ptr: usize,
    pub _phantom: PhantomData<T>
}