//! Error handling module for format specification
//!
//! This module provides error handling functionality for the format specification.

use core::fmt;
use wrt_error::Error;

// Import String and format macro for both std and no_std
#[cfg(feature = "std")]
use std::{format, string::String};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{format, string::String};

/// Module for error codes
pub mod codes {
    /// Parse error code
    pub const PARSE_ERROR: u16 = 1010;
    /// Validation error code
    pub const VALIDATION_ERROR: u16 = 5000;
    /// Type mismatch error code
    pub const TYPE_MISMATCH_ERROR: u16 = 6001;
    /// Runtime error code
    pub const RUNTIME_ERROR: u16 = 7000;
}

/// Create a simple parse error with the given message
pub fn parse_error(message: impl Into<String>) -> Error {
    Error::parse_error(message)
}

/// Create a new runtime error with the given message
pub fn runtime_error(message: impl Into<String>) -> Error {
    Error::runtime_error(message)
}

/// Create a new validation error with the given message
pub fn validation_error(message: impl Into<String>) -> Error {
    Error::validation_error(message)
}

/// Create a new type error with the given message
pub fn type_error(message: impl Into<String>) -> Error {
    Error::type_error(message)
}

/// Convert an error to a WRT error
pub fn to_wrt_error<E: fmt::Display>(err: E) -> Error {
    Error::system_error(format!("{}", err))
}

/// Trait for converting a type to a WRT error
pub trait IntoError {
    /// Convert self to a WRT error
    fn into_error(self) -> Error;
}

/// Create a new runtime error with the given message
pub fn wrt_runtime_error(message: impl Into<String>) -> Error {
    Error::runtime_error(message)
}

/// Create a new validation error with the given message
pub fn wrt_validation_error(message: impl Into<String>) -> Error {
    Error::validation_error(message)
}

/// Create a new type error with the given message
pub fn wrt_type_error(message: impl Into<String>) -> Error {
    Error::type_error(message)
}

/// Create a parse error with the given message
#[deprecated(since = "0.2.0", note = "use Error::parse_error instead")]
pub fn wrt_parse_error(message: impl Into<String>) -> Error {
    parse_error(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_error::{ErrorCategory, ErrorSource};

    #[test]
    fn test_error_creation() {
        let error = parse_error("test error");
        assert_eq!(error.category(), ErrorCategory::Parse);
        assert_eq!(error.code(), codes::PARSE_ERROR);
    }
}
