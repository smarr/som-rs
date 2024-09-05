
#[derive(Clone, PartialEq)]
pub struct GCRefToInstance(pub usize);
// TODO pub struct GCRef<T>(usize) instead? do we need phantomdata though?
