// Import from std when available
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::string::ToString;

// Import from alloc when not using std but using alloc
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::string::ToString;

/// Unified error handling system for WRT
///
/// This module provides a comprehensive error handling system that covers all error cases
/// across the WRT codebase. It includes error types, categories, and helper functions.
use core::fmt;

// Use alloc if alloc feature is enabled
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::string::String;

// Use std if std feature is enabled
#[cfg(feature = "std")]
use std::boxed::Box;
#[cfg(feature = "std")]
use std::string::String;

// For no_std without alloc, use the String struct from kinds
#[cfg(not(any(feature = "std", feature = "alloc")))]
use crate::kinds::String;

use crate::kinds;
use crate::prelude::{str, Debug, Eq, PartialEq};
use crate::{FromError, ToErrorCategory};

// Only import 'format' if std or alloc is enabled
#[cfg(any(feature = "std", feature = "alloc"))]
use crate::prelude::format;

/// Error categories for WRT operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Core WebAssembly errors
    Core = 1,
    /// Component model errors
    Component = 2,
    /// Resource errors (memory, tables, etc.)
    Resource = 3,
    /// Memory errors
    Memory = 4,
    /// Validation errors
    Validation = 5,
    /// Type errors
    Type = 6,
    /// Runtime errors (general)
    Runtime = 7,
    /// System errors
    System = 8,
    /// Unknown errors
    Unknown = 9,
    /// Parse errors
    Parse = 10,
    /// Concurrency errors
    Concurrency = 11,
    /// Capacity errors
    Capacity = 12,
    /// WebAssembly trap errors (specific runtime errors defined by Wasm spec)
    RuntimeTrap = 13,
}

/// Base trait for all error types - std version with Send+Sync
#[cfg(feature = "std")]
pub trait ErrorSource: fmt::Debug + Send + Sync {
    /// Get the error code
    fn code(&self) -> u16;

    /// Get the error message
    fn message(&self) -> &str;

    /// Get the error category
    fn category(&self) -> ErrorCategory;

    /// Get the source error if any
    fn source(&self) -> Option<&(dyn ErrorSource + 'static)> {
        None
    }
}

/// Base trait for all error types - no_std version
#[cfg(not(feature = "std"))]
pub trait ErrorSource: fmt::Debug {
    /// Get the error code
    fn code(&self) -> u16;

    /// Get the error message
    fn message(&self) -> &str;

    /// Get the error category
    fn category(&self) -> ErrorCategory;

    /// Get the source error if any
    fn source(&self) -> Option<&(dyn ErrorSource + 'static)> {
        None
    }
}

/// WRT Error type
///
/// This is the main error type for the WebAssembly Runtime.
/// It provides categorized errors with error codes and optional messages.
#[derive(Debug)]
pub struct Error {
    /// Error category
    pub category: ErrorCategory,
    /// Error code
    pub code: u16,
    /// Error message
    #[cfg(feature = "alloc")]
    pub message: String,
    /// Optional source error
    #[cfg(all(feature = "alloc", feature = "std"))]
    pub source: Option<Box<dyn ErrorSource + Send + Sync + 'static>>,
    /// Optional source error (no_std version)
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub source: Option<Box<dyn ErrorSource + 'static>>,
}

#[cfg(feature = "alloc")]
impl Clone for Error {
    fn clone(&self) -> Self {
        let base_error = Self::new(self.category, self.code, self.message.clone());

        #[cfg(feature = "std")]
        {
            if self.source.is_some() {
                let mut error_with_source = base_error;
                error_with_source.source =
                    Some(Box::new(Self::with_message("Source error (cloned)")));
                return error_with_source;
            }
        }
        base_error
    }
}

impl Error {
    /// Create a new error with allocation support
    #[cfg(feature = "alloc")]
    pub fn new<S: Into<String>>(category: ErrorCategory, code: u16, message: S) -> Self {
        Self { category, code, message: message.into(), source: None }
    }

    /// Create a new error in no_std mode (no message, no source)
    #[cfg(not(feature = "alloc"))]
    pub fn new<D: core::fmt::Display>(category: ErrorCategory, code: u16, _message: D) -> Self {
        Self { category, code }
    }

    /// Create a new error from a kinds type
    #[cfg(feature = "alloc")]
    pub fn from_kind<K: core::fmt::Display>(kind: K, code: u16, category: ErrorCategory) -> Self {
        Self { category, code, message: kind.to_string(), source: None }
    }

    /// Create a new error from a kinds type in no_std mode (no message storage)
    #[cfg(not(feature = "alloc"))]
    pub fn from_kind<K: core::fmt::Display>(_kind: K, code: u16, category: ErrorCategory) -> Self {
        Self { category, code }
    }

    /// Create a new error with a source
    #[cfg(all(feature = "alloc", feature = "std"))]
    pub fn with_source(
        category: ErrorCategory,
        code: u16,
        message: impl Into<String>,
        source: Box<dyn ErrorSource + Send + Sync>,
    ) -> Self {
        Self { category, code, message: message.into(), source: Some(source) }
    }

    /// Create a new error with a source (no_std version)
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub fn with_source(
        category: ErrorCategory,
        code: u16,
        message: impl Into<String>,
        source: Box<dyn ErrorSource>,
    ) -> Self {
        Self { category, code, message: message.into(), source: Some(source) }
    }

    /// Create a new error with a source in no_std mode (ignores source)
    #[cfg(not(feature = "alloc"))]
    pub fn with_source(
        category: ErrorCategory,
        code: u16,
        _message: impl core::fmt::Display,
        _source: impl core::fmt::Debug,
    ) -> Self {
        Self { category, code }
    }

    /// Check if this is a resource error
    #[must_use]
    pub fn is_resource_error(&self) -> bool {
        self.category == ErrorCategory::Resource
    }

    /// Check if this is a memory error
    #[must_use]
    pub fn is_memory_error(&self) -> bool {
        self.category == ErrorCategory::Memory
    }

    /// Check if this is a validation error
    #[must_use]
    pub fn is_validation_error(&self) -> bool {
        self.category == ErrorCategory::Validation
    }

    /// Check if this is a type error
    #[must_use]
    pub fn is_type_error(&self) -> bool {
        self.category == ErrorCategory::Type
    }

    /// Check if this is a runtime error
    #[must_use]
    pub fn is_runtime_error(&self) -> bool {
        self.category == ErrorCategory::Runtime
    }

    /// Check if this is a system error
    #[must_use]
    pub fn is_system_error(&self) -> bool {
        self.category == ErrorCategory::System
    }

    /// Check if this is a core error
    #[must_use]
    pub fn is_core_error(&self) -> bool {
        self.category == ErrorCategory::Core
    }

    /// Check if this is a component error
    #[must_use]
    pub fn is_component_error(&self) -> bool {
        self.category == ErrorCategory::Component
    }

    // Factory methods for std/alloc environments

    /// Create a resource error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn resource_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_ERROR, message)
    }

    /// Create a memory error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn memory_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, message)
    }

    /// Create a validation error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, message)
    }

    /// Create a type error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn type_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, message)
    }

    /// Create a runtime error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, message)
    }

    /// Create a system error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn system_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, message)
    }

    /// Create a core error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn core_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Core, codes::EXECUTION_ERROR, message)
    }

    /// Create a component error - std/alloc version
    #[cfg(feature = "alloc")]
    pub fn component_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_TYPE_MISMATCH, message)
    }

    /// Create a parse error
    #[cfg(feature = "alloc")]
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Parse, codes::PARSE_ERROR, message)
    }

    /// Create an invalid type error
    #[cfg(feature = "alloc")]
    pub fn invalid_type(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Type, codes::INVALID_TYPE, message)
    }

    /// Create an invalid type error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn invalid_type(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Type, codes::INVALID_TYPE, message)
    }

    // Factory methods for no_std mode

    /// Create a resource error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn resource_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_ERROR, message)
    }

    /// Create a memory error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn memory_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, message)
    }

    /// Create a validation error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn validation_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, message)
    }

    /// Create a type error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn type_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, message)
    }

    /// Create a runtime error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn runtime_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, message)
    }

    /// Create a system error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn system_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, message)
    }

    /// Create a core error - no_std version
    #[cfg(not(feature = "alloc"))]
    pub fn core_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Core, codes::EXECUTION_ERROR, message)
    }

    /// No allocation version of component_error
    #[cfg(not(feature = "alloc"))]
    pub fn component_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_TYPE_MISMATCH, message)
    }

    /// No allocation version of parse_error
    #[cfg(not(feature = "alloc"))]
    pub fn parse_error(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Parse, codes::PARSE_ERROR, message)
    }

    /// Creates a new error with a specific message.
    ///
    /// This is typically used when the error category and code are contextually known
    /// or less important than the specific message.
    /// Defaults to `ErrorCategory::Unknown` and `codes::UNKNOWN`.
    // Create a new error with just a message - for legacy error patterns
    #[cfg(feature = "alloc")]
    pub fn with_message(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Unknown, codes::UNKNOWN, message)
    }

    /// Creates a new error with a specific message (no_std, no-alloc version).
    ///
    /// This is typically used when the error category and code are contextually known
    /// or less important than the specific message.
    /// Defaults to `ErrorCategory::Unknown` and `codes::UNKNOWN`.
    // Create a new error with just a message - for no_std
    #[cfg(not(feature = "alloc"))]
    pub fn with_message(message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Unknown, codes::UNKNOWN, message)
    }

    /// Creates a new legacy error with a specific message.
    ///
    /// This function is for compatibility or specific legacy error scenarios.
    /// Defaults to `ErrorCategory::Unknown` and `codes::UNKNOWN`.
    // Create a new legacy-style error (unknown category, unknown code)
    #[cfg(feature = "alloc")]
    pub fn new_legacy(message: impl Into<String> + core::fmt::Display) -> Self {
        // Create a generic error with unknown category and code
        Self::new(ErrorCategory::Unknown, codes::UNKNOWN, message)
    }

    /// Creates a new legacy error (no_std, no-alloc version).
    ///
    /// This function is for compatibility or specific legacy error scenarios.
    /// Defaults to `ErrorCategory::Unknown` and `codes::UNKNOWN`.
    // Create a new legacy-style error (unknown category, unknown code) for no_std
    #[cfg(not(feature = "alloc"))]
    pub fn new_legacy(message: impl core::fmt::Display) -> Self {
        // Create a generic error with unknown category and code
        Self::new(ErrorCategory::Unknown, codes::UNKNOWN, message)
    }

    // Quick factory methods for different error categories with default code
    /// Creates a new resource error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn resource_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::Resource, code, message)
    }

    /// Creates a new resource error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn resource_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Resource, code, "resource error")
    }

    /// Creates a new validation error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn validation_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::Validation, code, message)
    }

    /// Creates a new validation error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn validation_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Validation, code, "validation error")
    }

    /// Creates a new type error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn type_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::Type, code, message)
    }

    /// Creates a new type error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn type_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Type, code, "type error")
    }

    /// Creates a new runtime error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn runtime_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::Runtime, code, message)
    }

    /// Creates a new runtime error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn runtime_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Runtime, code, "runtime error")
    }

    /// Creates a new system error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn system_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::System, code, message)
    }

    /// Creates a new system error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn system_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::System, code, "system error")
    }

    /// Creates a new core error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn core_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::Core, code, message)
    }

    /// Creates a new core error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn core_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Core, code, "core error")
    }

    /// Creates a new component error with a specific code and message.
    #[cfg(feature = "alloc")]
    pub fn component_error_with_code(
        code: u16,
        message: impl Into<String> + core::fmt::Display,
    ) -> Self {
        Self::new(ErrorCategory::Component, code, message)
    }

    /// Creates a new component error with a specific code and message (no alloc).
    #[cfg(not(feature = "alloc"))]
    pub fn component_error_with_code(code: u16, _message: impl core::fmt::Display) -> Self {
        Self::new(ErrorCategory::Component, code, "component error")
    }

    /// Helper function to create a parse error from a `ParseError` kind
    #[cfg(feature = "alloc")]
    #[must_use]
    pub fn parse_error_from_kind(kind: crate::kinds::ParseError) -> Self {
        Self::from_kind(kind, codes::PARSE_ERROR, ErrorCategory::Validation)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "alloc")]
        {
            write!(f, "{} (code: {})", self.message, self.code)
        }
        #[cfg(not(feature = "alloc"))]
        {
            write!(f, "Error category {:?} (code: {})", self.category, self.code)
        }
    }
}

impl ErrorSource for Error {
    fn code(&self) -> u16 {
        self.code
    }

    #[cfg(feature = "alloc")]
    fn message(&self) -> &str {
        &self.message
    }

    #[cfg(not(feature = "alloc"))]
    fn message(&self) -> &str {
        ""
    }

    fn category(&self) -> ErrorCategory {
        self.category
    }

    #[cfg(all(feature = "alloc", feature = "std"))]
    fn source(&self) -> Option<&(dyn ErrorSource + 'static)> {
        // Safe implementation without unsafe code
        self.source.as_ref().map(|s| &**s as &(dyn ErrorSource + 'static))
    }

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    fn source(&self) -> Option<&(dyn ErrorSource + 'static)> {
        self.source.as_ref().map(|s| s.as_ref())
    }

    #[cfg(not(feature = "alloc"))]
    fn source(&self) -> Option<&(dyn ErrorSource + 'static)> {
        None
    }
}

/// Error codes for different categories
pub mod codes {
    // Core WebAssembly errors (1000-1999)
    /// Error code for stack underflow.
    pub const STACK_UNDERFLOW: u16 = 1000;
    /// Error code for stack overflow.
    pub const STACK_OVERFLOW: u16 = 1001;
    /// Error code for unaligned memory access.
    pub const UNALIGNED_MEMORY_ACCESS: u16 = 1002;
    /// Error code for invalid memory access.
    pub const INVALID_MEMORY_ACCESS: u16 = 1003;
    /// Error code for invalid instance index.
    pub const INVALID_INSTANCE_INDEX: u16 = 1004;
    /// Error code for a general execution error.
    pub const EXECUTION_ERROR: u16 = 1005;
    /// Error code for a not implemented feature.
    pub const NOT_IMPLEMENTED: u16 = 1006;
    /// Error code for a memory access error (generic).
    pub const MEMORY_ACCESS_ERROR: u16 = 1007;
    /// Error code for an initialization error.
    pub const INITIALIZATION_ERROR: u16 = 1008;
    /// Error code for a type mismatch.
    pub const TYPE_MISMATCH: u16 = 1009;
    /// Error code for a parsing error.
    pub const PARSE_ERROR: u16 = 1010;
    /// Error code for an invalid version.
    pub const INVALID_VERSION: u16 = 1011;
    /// Error code for an out-of-bounds access.
    pub const OUT_OF_BOUNDS_ERROR: u16 = 1012;
    /// Error code for capacity exceeded.
    pub const CAPACITY_EXCEEDED: u16 = 1013;
    // New parse-related error codes
    /// Error code for invalid UTF-8 encoding.
    pub const INVALID_UTF8_ENCODING: u16 = 1014;
    /// Error code for an unsupported constant expression operation.
    pub const UNSUPPORTED_CONST_EXPR_OPERATION: u16 = 1015;
    /// Error code for an unimplemented parsing feature.
    pub const UNIMPLEMENTED_PARSING_FEATURE: u16 = 1016;

    // Component Model errors (2000-2999)
    /// Error code for an invalid function index.
    pub const INVALID_FUNCTION_INDEX: u16 = 2000;
    /// Error code for a component type mismatch.
    pub const COMPONENT_TYPE_MISMATCH: u16 = 2001;
    /// Error code for an encoding error.
    pub const ENCODING_ERROR: u16 = 2002;
    /// Error code for execution limit exceeded.
    pub const EXECUTION_LIMIT_EXCEEDED: u16 = 2003;
    /// Error code for a component instantiation error.
    pub const COMPONENT_INSTANTIATION_ERROR: u16 = 2004;
    /// Error code for a canonical ABI error.
    pub const CANONICAL_ABI_ERROR: u16 = 2005;
    /// Error code for a component linking error.
    pub const COMPONENT_LINKING_ERROR: u16 = 2006;

    // Resource Management errors (3000-3999)
    /// Error code for a generic resource error.
    pub const RESOURCE_ERROR: u16 = 3000;
    /// Error code for resource limit exceeded.
    pub const RESOURCE_LIMIT_EXCEEDED: u16 = 3001;
    /// Error code for a resource access error.
    pub const RESOURCE_ACCESS_ERROR: u16 = 3002;
    /// Error code for resource not found.
    pub const RESOURCE_NOT_FOUND: u16 = 3003;
    /// Error code for an invalid resource handle.
    pub const RESOURCE_INVALID_HANDLE: u16 = 3004;

    // Memory Management errors (4000-4999)
    /// Error code for memory out of bounds.
    pub const MEMORY_OUT_OF_BOUNDS: u16 = 4000;
    /// Error code for memory grow error.
    pub const MEMORY_GROW_ERROR: u16 = 4001;
    /// Error code for memory access out of bounds.
    pub const MEMORY_ACCESS_OUT_OF_BOUNDS: u16 = 4002;
    /// Error code for unaligned memory access.
    pub const MEMORY_ACCESS_UNALIGNED: u16 = 4003;

    // Validation errors (5000-5999)
    /// Error code for a generic validation error.
    pub const VALIDATION_ERROR: u16 = 5000;
    /// Error code for a validation failure.
    pub const VALIDATION_FAILURE: u16 = 5001;
    /// Error code for a checksum mismatch.
    pub const CHECKSUM_MISMATCH: u16 = 5002;
    /// Error code for an integrity violation.
    pub const INTEGRITY_VIOLATION: u16 = 5003;
    /// Error code for a verification level violation.
    pub const VERIFICATION_LEVEL_VIOLATION: u16 = 5004;

    // Type System errors (6000-6999)
    /// Error code for an invalid type.
    pub const INVALID_TYPE: u16 = 6000;
    /// Error code for a type mismatch error.
    pub const TYPE_MISMATCH_ERROR: u16 = 6001;
    /// Error code for an invalid function type.
    pub const INVALID_FUNCTION_TYPE: u16 = 6002;
    /// Error code for an invalid value type.
    pub const INVALID_VALUE_TYPE: u16 = 6003;

    // Runtime errors (7000-7999)
    /// Error code for a generic runtime error.
    pub const RUNTIME_ERROR: u16 = 7000;
    /// Error code for an execution timeout.
    pub const EXECUTION_TIMEOUT: u16 = 7001;
    /// Error code for fuel exhausted.
    pub const FUEL_EXHAUSTED: u16 = 7002;
    /// Error code for a poisoned lock.
    pub const POISONED_LOCK: u16 = 7003;

    // System errors (8000-8999)
    /// Error code for a generic system error.
    pub const SYSTEM_ERROR: u16 = 8000;
    /// Error code for an unsupported operation.
    pub const UNSUPPORTED_OPERATION: u16 = 8001;
    /// Error code for a conversion error.
    pub const CONVERSION_ERROR: u16 = 8002;
    /// Error code for a decoding error.
    pub const DECODING_ERROR: u16 = 8003;

    // Unknown error (9999)
    /// Error code for an unknown error.
    pub const UNKNOWN: u16 = 9999;

    // Wasm 2.0 specific errors (from Wasm 2.0 spec and internal)
    // These may overlap or be more specific versions of core errors
    /// Error code for an unsupported Wasm 2.0 feature.
    pub const UNSUPPORTED_WASM20_FEATURE_ERROR: u16 = 1100;
    /// Error code for invalid usage of a reference type.
    pub const INVALID_REFERENCE_TYPE_USAGE_ERROR: u16 = 1101;
    /// Error code for a bulk operation error.
    pub const BULK_OPERATION_ERROR: u16 = 1102;
    /// Error code for a SIMD operation error.
    pub const SIMD_OPERATION_ERROR: u16 = 1103;
    /// Error code for a tail call error.
    pub const TAIL_CALL_ERROR: u16 = 1104;
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Test runtime error");
        assert_eq!(err.category, ErrorCategory::Runtime);
        assert_eq!(err.code, codes::RUNTIME_ERROR);
        assert_eq!(err.message, "Test runtime error");
    }

    #[test]
    fn test_error_with_source() {
        let source_err =
            Error::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, "Memory out of bounds");

        let err = Error::with_source(
            ErrorCategory::Runtime,
            codes::RUNTIME_ERROR,
            "Runtime error with source",
            Box::new(source_err),
        );

        assert_eq!(err.category, ErrorCategory::Runtime);
        assert_eq!(err.code, codes::RUNTIME_ERROR);
        assert_eq!(err.message, "Runtime error with source");
        assert!(err.source.is_some());
    }

    #[test]
    fn test_error_category_checks() {
        let err = Error::memory_error("Test memory error");
        assert!(err.is_memory_error());
        assert!(!err.is_resource_error());
    }

    #[test]
    fn test_error_display() {
        let err = Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Test runtime error");
        assert_eq!(format!("{err}"), "Test runtime error (code: 7000)");
    }

    #[test]
    fn test_factory_methods() {
        let resource_err = Error::resource_error("Resource error");
        assert_eq!(resource_err.category, ErrorCategory::Resource);
        assert_eq!(resource_err.code, codes::RESOURCE_ERROR);

        let memory_err = Error::memory_error("Memory error");
        assert_eq!(memory_err.category, ErrorCategory::Memory);
        assert_eq!(memory_err.code, codes::MEMORY_OUT_OF_BOUNDS);
    }
}

#[cfg(feature = "alloc")]
impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::new(ErrorCategory::Runtime, codes::UNKNOWN, message)
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self::new(ErrorCategory::Runtime, codes::UNKNOWN, message)
    }
}

#[cfg(all(feature = "alloc", feature = "std"))]
impl<T: ErrorSource + Send + Sync + 'static> From<Box<T>> for Error {
    fn from(source: Box<T>) -> Self {
        let category = source.category();
        let code = source.code();
        let message = source.message().to_string();

        // Safe conversion using standard downcasting approach
        let source_box: Box<dyn ErrorSource + Send + Sync + 'static> = source;

        Self { category, code, message, source: Some(source_box) }
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
impl<T: ErrorSource + 'static> From<Box<T>> for Error {
    fn from(source: Box<T>) -> Self {
        let category = source.category();
        let code = source.code();
        let message = source.message().to_string();

        Self { category, code, message, source: Some(source) }
    }
}

// Implement From for standard error types
impl From<core::fmt::Error> for Error {
    fn from(_: core::fmt::Error) -> Self {
        #[cfg(feature = "alloc")]
        return Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, "Formatting error");

        #[cfg(not(feature = "alloc"))]
        return Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, "");
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            ErrorCategory::System,
            codes::SYSTEM_ERROR,
            #[cfg(feature = "alloc")]
            alloc::format!("IO error: {e}"),
            #[cfg(not(feature = "alloc"))]
            "IO error",
        )
    }
}

// Conversion helpers for error kinds
#[cfg(feature = "alloc")]
impl From<kinds::ValidationError> for Error {
    fn from(e: kinds::ValidationError) -> Self {
        Self::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::ParseError> for Error {
    fn from(e: kinds::ParseError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::PARSE_ERROR, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::OutOfBoundsError> for Error {
    fn from(e: kinds::OutOfBoundsError) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::InvalidType> for Error {
    fn from(e: kinds::InvalidType) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::ResourceError> for Error {
    fn from(e: kinds::ResourceError) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_ERROR, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::ComponentError> for Error {
    fn from(e: kinds::ComponentError) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_LINKING_ERROR, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::RuntimeError> for Error {
    fn from(e: kinds::RuntimeError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, e.0)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::PoisonedLockError> for Error {
    fn from(e: kinds::PoisonedLockError) -> Self {
        Self::from_kind(e, codes::POISONED_LOCK, ErrorCategory::Runtime)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::TypeMismatchError> for Error {
    fn from(e: kinds::TypeMismatchError) -> Self {
        Self::from_kind(e, codes::TYPE_MISMATCH_ERROR, ErrorCategory::Type)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::MemoryAccessOutOfBoundsError> for Error {
    fn from(e: kinds::MemoryAccessOutOfBoundsError) -> Self {
        Self::from_kind(e, codes::MEMORY_ACCESS_OUT_OF_BOUNDS, ErrorCategory::Memory)
    }
}

#[cfg(feature = "alloc")]
impl From<kinds::TableAccessOutOfBounds> for Error {
    fn from(e: kinds::TableAccessOutOfBounds) -> Self {
        Self::from_kind(e, codes::OUT_OF_BOUNDS_ERROR, ErrorCategory::Memory)
    }
}

impl From<kinds::ConversionError> for Error {
    fn from(e: kinds::ConversionError) -> Self {
        Self::from_kind(e, codes::CONVERSION_ERROR, ErrorCategory::System)
    }
}

impl From<kinds::DivisionByZeroError> for Error {
    fn from(e: kinds::DivisionByZeroError) -> Self {
        Self::from_kind(e, codes::RUNTIME_ERROR, ErrorCategory::Runtime)
    }
}

impl From<kinds::IntegerOverflowError> for Error {
    fn from(e: kinds::IntegerOverflowError) -> Self {
        Self::from_kind(e, codes::RUNTIME_ERROR, ErrorCategory::Runtime)
    }
}

impl From<kinds::StackUnderflow> for Error {
    fn from(e: kinds::StackUnderflow) -> Self {
        Self::from_kind(e, codes::STACK_UNDERFLOW, ErrorCategory::Runtime)
    }
}

impl From<kinds::TypeMismatch> for Error {
    fn from(e: kinds::TypeMismatch) -> Self {
        Self::from_kind(e, codes::TYPE_MISMATCH_ERROR, ErrorCategory::Type)
    }
}

impl From<kinds::InvalidTableIndexError> for Error {
    fn from(e: kinds::InvalidTableIndexError) -> Self {
        Self::from_kind(e, codes::OUT_OF_BOUNDS_ERROR, ErrorCategory::Memory)
    }
}

// Implement the ToErrorCategory trait for Error
impl ToErrorCategory for Error {
    fn to_category(&self) -> ErrorCategory {
        self.category
    }
}

// Implement FromError for Error (self conversion)
impl FromError<Self> for Error {
    fn from_error(error: Self) -> Self {
        error
    }
}

// Standard implementation of FromError for common error types
#[cfg(feature = "std")]
impl FromError<std::io::Error> for Error {
    fn from_error(error: std::io::Error) -> Self {
        #[cfg(feature = "alloc")]
        {
            Self::system_error(alloc::format!("IO error: {error}"))
        }
        #[cfg(not(feature = "alloc"))]
        {
            Self::system_error("IO error")
        }
    }
}

// Implement FromError for string types when alloc is available
#[cfg(feature = "alloc")]
impl FromError<String> for Error {
    fn from_error(error: String) -> Self {
        Self::runtime_error(error)
    }
}

// Implement FromError for &str
impl FromError<&str> for Error {
    fn from_error(error: &str) -> Self {
        #[cfg(feature = "alloc")]
        {
            Self::runtime_error(error)
        }
        #[cfg(not(feature = "alloc"))]
        {
            Self::runtime_error(error)
        }
    }
}

// Add FromError implementations for the kinds error types
#[cfg(feature = "alloc")]
impl FromError<kinds::ValidationError> for Error {
    fn from_error(error: kinds::ValidationError) -> Self {
        Self::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::ParseError> for Error {
    fn from_error(error: kinds::ParseError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::PARSE_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::OutOfBoundsError> for Error {
    fn from_error(error: kinds::OutOfBoundsError) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidType> for Error {
    fn from_error(error: kinds::InvalidType) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::ResourceError> for Error {
    fn from_error(error: kinds::ResourceError) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::ComponentError> for Error {
    fn from_error(error: kinds::ComponentError) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_LINKING_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::RuntimeError> for Error {
    fn from_error(error: kinds::RuntimeError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::PoisonedLockError> for Error {
    fn from_error(error: kinds::PoisonedLockError) -> Self {
        Self::from_kind(error, codes::POISONED_LOCK, ErrorCategory::Runtime)
    }
}

// Add FromError implementations for the new error types
#[cfg(feature = "alloc")]
impl FromError<kinds::ArithmeticError> for Error {
    fn from_error(error: kinds::ArithmeticError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::MemoryAccessError> for Error {
    fn from_error(error: kinds::MemoryAccessError) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_ACCESS_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::ResourceExhaustionError> for Error {
    fn from_error(error: kinds::ResourceExhaustionError) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_LIMIT_EXCEEDED, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidIndexError> for Error {
    fn from_error(error: kinds::InvalidIndexError) -> Self {
        Self::new(ErrorCategory::Validation, codes::OUT_OF_BOUNDS_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::ExecutionError> for Error {
    fn from_error(error: kinds::ExecutionError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::StackUnderflowError> for Error {
    fn from_error(error: kinds::StackUnderflowError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::ExportNotFoundError> for Error {
    fn from_error(error: kinds::ExportNotFoundError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RESOURCE_NOT_FOUND, error.0)
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidInstanceIndexError> for Error {
    fn from_error(error: kinds::InvalidInstanceIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::INVALID_INSTANCE_INDEX,
            format!("Invalid instance index: {}", error.0),
        )
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidFunctionIndexError> for Error {
    fn from_error(error: kinds::InvalidFunctionIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::INVALID_FUNCTION_INDEX,
            format!("Invalid function index: {}", error.0),
        )
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidElementIndexError> for Error {
    fn from_error(error: kinds::InvalidElementIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::OUT_OF_BOUNDS_ERROR,
            format!("Invalid element index: {}", error.0),
        )
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidMemoryIndexError> for Error {
    fn from_error(error: kinds::InvalidMemoryIndexError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::MEMORY_OUT_OF_BOUNDS,
            format!("Invalid memory index: {}", error.0),
        )
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidGlobalIndexError> for Error {
    fn from_error(error: kinds::InvalidGlobalIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::OUT_OF_BOUNDS_ERROR,
            format!("Invalid global index: {}", error.0),
        )
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidDataSegmentIndexError> for Error {
    fn from_error(error: kinds::InvalidDataSegmentIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::OUT_OF_BOUNDS_ERROR,
            format!("Invalid data segment index: {}", error.0),
        )
    }
}

#[cfg(feature = "alloc")]
impl FromError<kinds::InvalidFunctionTypeError> for Error {
    fn from_error(e: kinds::InvalidFunctionTypeError) -> Self {
        Self::type_error_with_code(codes::INVALID_FUNCTION_TYPE, e.0)
    }
}

// --- START Wasm 2.0 From Impls ---

impl From<kinds::UnsupportedWasm20Feature> for Error {
    fn from(e: kinds::UnsupportedWasm20Feature) -> Self {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            Self::validation_error_with_code(
                codes::UNSUPPORTED_WASM20_FEATURE_ERROR,
                e.feature_name,
            )
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            Self::validation_error_with_code(
                codes::UNSUPPORTED_WASM20_FEATURE_ERROR,
                "unsupported wasm20 feature",
            )
        }
    }
}

impl From<kinds::InvalidReferenceTypeUsage> for Error {
    fn from(e: kinds::InvalidReferenceTypeUsage) -> Self {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            Self::type_error_with_code(codes::INVALID_REFERENCE_TYPE_USAGE_ERROR, e.message)
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            Self::type_error_with_code(
                codes::INVALID_REFERENCE_TYPE_USAGE_ERROR,
                "invalid reference type usage",
            )
        }
    }
}

impl From<kinds::BulkOperationError> for Error {
    fn from(e: kinds::BulkOperationError) -> Self {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            Self::runtime_error_with_code(
                codes::BULK_OPERATION_ERROR,
                format!("Bulk op '{}' failed: {}", e.operation_name, e.reason),
            )
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            Self::runtime_error_with_code(codes::BULK_OPERATION_ERROR, "Bulk operation error")
        }
    }
}

impl From<kinds::SimdOperationError> for Error {
    fn from(e: kinds::SimdOperationError) -> Self {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            Self::runtime_error_with_code(
                codes::SIMD_OPERATION_ERROR,
                format!("SIMD op '{}' error: {}", e.instruction_name, e.reason),
            )
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            Self::runtime_error_with_code(codes::SIMD_OPERATION_ERROR, "SIMD operation error")
        }
    }
}

impl From<kinds::TailCallError> for Error {
    fn from(e: kinds::TailCallError) -> Self {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            Self::validation_error_with_code(codes::TAIL_CALL_ERROR, e.message)
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            Self::validation_error_with_code(codes::TAIL_CALL_ERROR, "tail call error")
        }
    }
}

// --- END Wasm 2.0 From Impls ---
