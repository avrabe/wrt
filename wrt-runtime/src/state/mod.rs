//! WebAssembly runtime state management.
//!
//! This module provides utilities for managing and serializing WebAssembly
//! runtime state including stack frames, globals, and memory.

pub mod serialization;

pub use serialization::{
    StateSection, StateHeader, STATE_SECTION_PREFIX,
};

// Re-export functions conditionally
#[cfg(any(feature = "std", feature = "alloc"))]
pub use serialization::{
    create_state_section, extract_state_section,
    has_state_sections, is_state_section_name,
};