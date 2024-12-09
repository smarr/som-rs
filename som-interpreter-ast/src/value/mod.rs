use som_core::value::BaseValue;

/// The main value type.
pub mod nan_boxed_val;

/// Automatically convert values to their underlying type. Useful for primitives.
pub mod convert;

/// For values that are to pointer types.
pub mod value_ptr;

#[derive(Clone, Copy)]
pub struct Value(pub(crate) BaseValue);
