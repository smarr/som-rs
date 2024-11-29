/// Facilities for manipulating blocks.
pub mod block;
/// Facilities for manipulating classes.
pub mod class;
/// Facilities for manipulating stack frames.
pub mod frame;
/// Facilities for manipulating class instances.
pub mod instance;
/// Facilities for manipulating class methods.
pub mod method;

#[cfg(test)]
mod tests {
    pub mod frame;
}
