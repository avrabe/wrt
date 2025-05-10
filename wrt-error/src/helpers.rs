//! Error helper functions for common error patterns.
//!
//! This module primarily re-exports functionality from the kinds module
//! for backward compatibility with existing code.

// Re-export error kind creation functions
pub use crate::kinds::*;

// Import ErrorCategory and Error for compatibility functions
#[cfg(feature = "alloc")]
use crate::Error;

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::string::ToString;

#[cfg(feature = "alloc")]
use alloc::format as alloc_format;

// For backward compatibility - simple error creation functions
/// Creates a simple validation error. Deprecated: use `Error::validation_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::validation_error instead")]
pub fn create_simple_validation_error<T: core::fmt::Display>(message: T) -> Error {
    Error::validation_error(message.to_string())
}

/// Creates a simple type error. Deprecated: use `Error::type_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::type_error instead")]
pub fn create_simple_type_error<T: core::fmt::Display>(message: T) -> Error {
    Error::type_error(message.to_string())
}

/// Creates a simple memory error. Deprecated: use `Error::memory_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::memory_error instead")]
pub fn create_simple_memory_error<T: core::fmt::Display>(message: T) -> Error {
    Error::memory_error(message.to_string())
}

/// Creates a simple resource error. Deprecated: use `Error::resource_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::resource_error instead")]
pub fn create_simple_resource_error<T: core::fmt::Display>(message: T) -> Error {
    Error::resource_error(message.to_string())
}

/// Creates a simple component error. Deprecated: use `Error::component_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::component_error instead")]
pub fn create_simple_component_error<T: core::fmt::Display>(message: T) -> Error {
    Error::component_error(message.to_string())
}

/// Creates a simple runtime error. Deprecated: use `Error::runtime_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::runtime_error instead")]
pub fn create_simple_runtime_error<T: core::fmt::Display>(message: T) -> Error {
    Error::runtime_error(message.to_string())
}

/// Creates a simple system error. Deprecated: use `Error::system_error` instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::system_error instead")]
pub fn create_simple_system_error<T: core::fmt::Display>(message: T) -> Error {
    Error::system_error(message.to_string())
}

/// Creates a resource limit error. Deprecated: use `Error::resource_error` with a formatted message instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::resource_error with formatted message instead")]
#[must_use]
pub fn create_resource_limit_error(resource_type: &str, current: usize, limit: usize) -> Error {
    Error::resource_error(alloc_format!(
        "{resource_type} limit exceeded: current={current}, limit={limit}"
    ))
}

/// Creates a memory access error. Deprecated: use `Error::memory_error` with a formatted message instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::memory_error with formatted message instead")]
#[must_use]
pub fn create_memory_access_error(
    address: u64,
    size: u64,
    memory_size: u64,
    operation: &str,
) -> Error {
    Error::memory_error(alloc_format!(
        "Memory access out of bounds during {operation}: address={address}, size={size}, memory_size={memory_size}"
    ))
}

/// Creates a type mismatch error. Deprecated: use `Error::type_error` with a formatted message instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::type_error with formatted message instead")]
#[must_use]
pub fn create_type_mismatch_error(expected: &str, actual: &str, context: &str) -> Error {
    Error::type_error(alloc_format!(
        "Type mismatch in {context}: expected {expected}, got {actual}"
    ))
}

/// Creates an index error. Deprecated: use `Error::core_error` with a formatted message instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::core_error with formatted message instead")]
#[must_use]
pub fn create_index_error(index_type: &str, index: usize) -> Error {
    Error::core_error(alloc_format!("Invalid {index_type} index: {index}"))
}

/// Creates a bounded error (capacity exceeded). Deprecated: use `Error::resource_error` with a formatted message instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::resource_error with formatted message instead")]
pub fn convert_bounded_error<E: core::fmt::Display, T: core::fmt::Display>(
    capacity: E,
    requested: T,
) -> Error {
    Error::resource_error(alloc_format!(
        "Capacity exceeded: capacity={capacity}, requested={requested}"
    ))
}

/// Creates a bounded index error (index out of bounds). Deprecated: use `Error::core_error` with a formatted message instead.
#[cfg(feature = "alloc")]
#[deprecated(since = "0.2.0", note = "use Error::core_error with formatted message instead")]
pub fn convert_bounded_index_error<I: core::fmt::Display, M: core::fmt::Display>(
    index: I,
    max: M,
) -> Error {
    Error::core_error(alloc_format!("Index out of bounds: index={index}, max={max}"))
}
