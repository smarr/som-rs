//! Debugging facilities.

/// Facilities for disassembling bytecode.
pub mod disassembler;

/// Facilities for profiling the SOM VM during execution.
#[cfg(feature = "profiler")]
pub mod profiler;
