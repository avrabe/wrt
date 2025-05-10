// WRT - wrt-error
// SW-REQ-ID: [SW-REQ-ID-wrt-error]
//
// Copyright (c) 2024 [Your Name/Organization]
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! WRT Error handling library
//!
//! This library provides a comprehensive error handling system for the WRT runtime.
//! It includes error types, helper functions, and utilities for creating and managing errors.
//!
//! # Error Categories
//!
//! Errors are organized into several categories, each with its own range of error codes:
//!
//! ## Core Errors (1000-1008)
//! - Stack underflow
//! - Memory access violations
//! - Instance index errors
//! - Execution errors
//! - Type mismatches
//!
//! ## Runtime Errors (2000-2021)
//! - Invalid indices (local, global, function)
//! - Memory and table access errors
//! - Resource exhaustion
//! - Validation failures
//! - Parse errors
//!
//! ## Component Errors (3000-3013)
//! - Function index errors
//! - Type mismatches
//! - Resource limits
//! - Component lifecycle errors
//! - ABI errors
//!
//! ## System Errors (0x1000-0x1001)
//! - Async operation errors
//! - Threading errors
//!
//! # Usage
//!
//! The library provides both low-level error types and high-level helper functions:
//!
//! ```
//! # #[cfg(feature = "alloc")]
//! # {
//! use wrt_error::{Error, kinds};
//!
//! // Using helper functions for common errors
//! let error = Error::new(
//!     wrt_error::ErrorCategory::Core,
//!     wrt_error::codes::INVALID_FUNCTION_INDEX,
//!     "Invalid function index: 42".to_string()
//! );
//!
//! // Using kind functions for common errors
//! let index_error = kinds::invalid_index_error("function");
//! let memory_error = kinds::memory_access_error("Memory access out of bounds");
//! # }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]
#![warn(clippy::pedantic)]
#![warn(clippy::missing_panics_doc)]
#![deny(missing_docs)]
#![allow(clippy::negative_feature_names)]
#![allow(clippy::module_name_repetitions)]

//! Core error types for WRT

// Import external crates when std feature is enabled
#[cfg(feature = "std")]
extern crate std;

// Always import core
extern crate core;

// Import alloc when either std or alloc is enabled
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

/// Error codes for wrt
pub mod codes;
/// Error and error handling types
pub mod errors;
/// Error kind definitions
pub mod kinds;

// Modules
pub mod context;
pub mod helpers;
pub mod prelude;

// Include verification module conditionally, but exclude during coverage builds
#[cfg(all(not(coverage), doc))]
pub mod verify;

// Re-export key types
pub use errors::Error;
pub use errors::{ErrorCategory, ErrorSource};

#[cfg(feature = "alloc")]
pub use context::ResultExt;

/// A specialized `Result` type for WRT operations.
///
/// When the `alloc` feature is enabled, this defaults to using `wrt_error::Error` as the error type.
/// When `alloc` is not available, the specific error type must be provided.
pub type Result<T> = core::result::Result<T, Error>;

// Re-export error kinds for convenience
pub use kinds::{
    component_error, invalid_type, out_of_bounds_error, parse_error, poisoned_lock_error,
    resource_error, runtime_error, validation_error, ComponentError, InvalidType, OutOfBoundsError,
    ParseError, PoisonedLockError, ResourceError, RuntimeError, ValidationError,
};

/// Error conversion trait for converting between error types
///
/// This trait provides a standardized way to convert between error types
/// across the WRT codebase. It is used to ensure a consistent error
/// handling approach across all crates.
pub trait FromError<E> {
    /// Convert from the source error type to the target error type
    fn from_error(error: E) -> Self;
}

/// Error conversion trait for converting to specific error categories
///
/// This trait provides a way to convert any error to a specific error
/// category, which is useful for creating category-specific errors.
pub trait ToErrorCategory {
    /// Convert the error to a specific category
    fn to_category(&self) -> ErrorCategory;
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let resource_err = Error::resource_error("Resource not found");
        assert!(resource_err.is_resource_error());
        assert!(!resource_err.is_memory_error());

        let memory_err = Error::memory_error("Memory access out of bounds");
        assert!(memory_err.is_memory_error());
        assert!(!memory_err.is_resource_error());
    }

    #[test]
    fn test_error_codes() {
        let err = Error::resource_error("Test error");
        assert_eq!(err.code, codes::RESOURCE_ERROR);
    }
}

// Re-export additional helpers
pub use helpers::*;
