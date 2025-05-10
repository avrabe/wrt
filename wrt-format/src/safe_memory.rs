//! Safe memory management re-exports from wrt-types
//!
//! This module re-exports the safe memory types and functionality from wrt-types
//! to provide a consistent interface for both wrt-format and other crates.

// Re-export the safe memory types from wrt-types
pub use wrt_types::safe_memory::*;

// Re-export memory providers matching wrt-types feature-gating
#[cfg(feature = "std")]
pub use wrt_types::StdMemoryProvider;

#[cfg(not(feature = "std"))]
pub use wrt_types::safe_memory::NoStdMemoryProvider;

// Re-export common memory types always
pub use wrt_types::safe_memory::{MemoryProvider, SafeMemoryHandler, SafeSlice, SafeStack};

/// Create a safe slice from binary data
pub fn safe_slice(data: &[u8]) -> wrt_types::safe_memory::SafeSlice<'_> {
    wrt_types::safe_memory::SafeSlice::new(data)
}

/// Get the default verification level for memory operations
pub fn default_verification_level() -> wrt_types::verification::VerificationLevel {
    wrt_types::verification::VerificationLevel::Standard
}
