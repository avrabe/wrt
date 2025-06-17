// WRT - wrt-error
// Module: WRT Error Types
// SW-REQ-ID: REQ_004
// SW-REQ-ID: REQ_ERROR_001
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

/// Unified error handling system for WRT
///
/// This module provides a comprehensive error handling system that covers all
/// error cases across the WRT codebase. It includes error types, categories,
/// and helper functions.
use core::fmt;

use crate::{
    kinds,
    prelude::{str, Debug, Eq, PartialEq},
    FromError, ToErrorCategory,
};

/// `Error` categories for WRT operations
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
    /// Initialization errors
    Initialization = 14,
    /// Not supported operation errors
    NotSupported = 15,
    /// Safety-related errors (ASIL violations, integrity checks, etc.)
    Safety = 16,
    /// Security-related errors (access control, permissions, etc.)
    Security = 17,
    /// Parameter-related errors (invalid arguments, missing parameters, etc.)
    Parameter = 18,
    /// Verification errors (proofs, checksums, integrity, etc.)
    Verification = 19,
}

/// Base trait for all error types - `no_std` version
pub trait ErrorSource: fmt::Debug + Send + Sync {
    /// Get the error code
    fn code(&self) -> u16;

    /// Get the error message
    fn message(&self) -> &'static str;

    /// Get the error category
    fn category(&self) -> ErrorCategory;
}

/// WRT `Error` type
///
/// This is the main error type for the WebAssembly Runtime.
/// It provides categorized errors with error codes and optional messages.
#[derive(Debug, Copy, Clone)]
pub struct Error {
    /// `Error` category
    pub category: ErrorCategory,
    /// `Error` code
    pub code: u16,
    /// `Error` message
    pub message: &'static str,
}

impl Error {
    /// Create a new error.
    #[must_use]
    pub const fn new(category: ErrorCategory, code: u16, message: &'static str) -> Self {
        // ASIL-D: Validate error category and code ranges at compile time
        #[cfg(feature = "asil-d")]
        {
            // Note: const fn limitations prevent runtime assertions, but this documents
            // the expected ranges for each category. In a full implementation, these
            // would be enforced through the type system or compile-time checks.
        }

        Self { category, code, message }
    }

    // Agent C constant error instances
    /// WIT input too large error
    pub const WIT_INPUT_TOO_LARGE: Self = Self::new(
        ErrorCategory::Parse,
        codes::WIT_INPUT_TOO_LARGE,
        "WIT input too large for parser buffer",
    );

    /// WIT world limit exceeded error
    pub const WIT_WORLD_LIMIT_EXCEEDED: Self = Self::new(
        ErrorCategory::Parse,
        codes::WIT_WORLD_LIMIT_EXCEEDED,
        "Too many WIT worlds for parser limits",
    );

    /// WIT interface limit exceeded error
    pub const WIT_INTERFACE_LIMIT_EXCEEDED: Self = Self::new(
        ErrorCategory::Parse,
        codes::WIT_INTERFACE_LIMIT_EXCEEDED,
        "Too many WIT interfaces for parser limits",
    );

    /// No WIT definitions found error
    pub const NO_WIT_DEFINITIONS_FOUND: Self = Self::new(
        ErrorCategory::Parse,
        codes::NO_WIT_DEFINITIONS_FOUND,
        "No WIT worlds or interfaces found in input",
    );

    /// Insufficient memory error
    pub const INSUFFICIENT_MEMORY: Self = Self::new(
        ErrorCategory::Resource,
        codes::INSUFFICIENT_MEMORY,
        "Insufficient memory for operation",
    );

    /// Out of memory error
    pub const OUT_OF_MEMORY: Self =
        Self::new(ErrorCategory::Resource, codes::OUT_OF_MEMORY, "Out of memory");

    /// Too many components error
    pub const TOO_MANY_COMPONENTS: Self = Self::new(
        ErrorCategory::Component,
        codes::TOO_MANY_COMPONENTS,
        "Too many components instantiated",
    );

    /// Component not found error
    pub const COMPONENT_NOT_FOUND: Self =
        Self::new(ErrorCategory::Component, codes::COMPONENT_NOT_FOUND, "Component not found");

    /// Stack overflow error
    pub const STACK_OVERFLOW: Self =
        Self::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Stack overflow");

    /// Create a component error with dynamic context (using static fallback)
    #[must_use]
    pub const fn component_error(_message: &'static str) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_ERROR, "Component error")
    }

    /// Create a WIT parse error with dynamic message (using static fallback)
    #[must_use]
    pub const fn wit_parse_error(_message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::WIT_PARSE_ERROR, "WIT parse error")
    }

    /// Create an invalid input error with dynamic message (using static fallback)
    #[must_use]
    pub const fn invalid_input(_message: &'static str) -> Self {
        Self::new(ErrorCategory::Validation, codes::INVALID_INPUT, "Invalid input")
    }

    /// Create an unsupported error with dynamic message (using static fallback)
    #[must_use]
    pub const fn unsupported(_message: &'static str) -> Self {
        Self::new(ErrorCategory::System, codes::UNSUPPORTED, "Unsupported operation")
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

    /// Check if this is a safety error
    #[must_use]
    pub fn is_safety_error(&self) -> bool {
        self.category == ErrorCategory::Safety
    }

    /// Get the ASIL level of this error (ASIL-B and above)
    #[cfg(any(feature = "asil-b", feature = "asil-c", feature = "asil-d"))]
    #[must_use]
    pub fn asil_level(&self) -> &'static str {
        match self.category {
            ErrorCategory::Safety => "ASIL-D", // Safety errors require highest level
            ErrorCategory::Memory | ErrorCategory::RuntimeTrap => "ASIL-C", // Memory/trap errors are ASIL-C
            ErrorCategory::Validation | ErrorCategory::Type => "ASIL-B",    // Type safety is ASIL-B
            _ => "QM", // Other errors are Quality Management level
        }
    }

    /// Check if error requires immediate safe state transition (ASIL-C and above)
    #[cfg(any(feature = "asil-c", feature = "asil-d"))]
    #[must_use]
    pub fn requires_safe_state(&self) -> bool {
        matches!(
            self.category,
            ErrorCategory::Safety | ErrorCategory::Memory | ErrorCategory::RuntimeTrap
        )
    }

    /// Validate error integrity (ASIL-D only)
    #[cfg(feature = "asil-d")]
    #[must_use]
    pub fn validate_integrity(&self) -> bool {
        // Check that error code is within valid range for category
        let valid_range = match self.category {
            ErrorCategory::Core => self.code >= 1000 && self.code < 2000,
            ErrorCategory::Component => self.code >= 2000 && self.code < 3000,
            ErrorCategory::Resource => self.code >= 3000 && self.code < 4000,
            ErrorCategory::Memory => self.code >= 4000 && self.code < 5000,
            ErrorCategory::Validation => self.code >= 5000 && self.code < 6000,
            ErrorCategory::Type => self.code >= 6000 && self.code < 7000,
            ErrorCategory::Runtime => self.code >= 7000 && self.code < 8000,
            ErrorCategory::System => self.code >= 8000 && self.code < 9000,
            ErrorCategory::Safety => self.code >= 7000 && self.code < 8000,
            _ => self.code >= 9000 && self.code <= 9999,
        };

        // Check that message is not empty (basic integrity check)
        valid_range && !self.message.is_empty()
    }

    // Factory methods

    /// Create a resource error
    #[must_use]
    pub const fn resource_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_ERROR, message)
    }

    /// Create a memory error
    #[must_use]
    pub const fn memory_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, message)
    }

    /// Create a validation error
    #[must_use]
    pub const fn validation_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, message)
    }

    /// Create a type error
    #[must_use]
    pub const fn type_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, message)
    }

    /// Create a runtime error
    #[must_use]
    pub const fn runtime_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, message)
    }

    /// Create a system error
    #[must_use]
    pub const fn system_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, message)
    }

    /// Create a core error
    #[must_use]
    pub const fn core_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Core, codes::EXECUTION_ERROR, message)
    }

    /// Create a parse error
    #[must_use]
    pub const fn parse_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::PARSE_ERROR, message)
    }

    /// Create an invalid type error
    #[must_use]
    pub const fn invalid_type_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Type, codes::INVALID_TYPE, message)
    }

    /// Create an index out of bounds error
    #[must_use]
    pub const fn index_out_of_bounds(message: &'static str) -> Self {
        Self::new(ErrorCategory::Memory, codes::OUT_OF_BOUNDS_ERROR, message)
    }

    /// Create a deserialization error
    #[must_use]
    pub const fn deserialization_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::DECODING_ERROR, message)
    }

    /// Create a capacity error
    #[must_use]
    pub const fn capacity_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::Capacity, codes::CAPACITY_EXCEEDED, message)
    }

    /// Create an internal error
    #[must_use]
    pub const fn internal_error(message: &'static str) -> Self {
        Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, message)
    }

    /// Create a memory out of bounds error
    #[must_use]
    pub const fn memory_out_of_bounds(message: &'static str) -> Self {
        Self::new(ErrorCategory::Memory, codes::MEMORY_OUT_OF_BOUNDS, message)
    }

    /// Create a memory uninitialized error
    #[must_use]
    pub const fn memory_uninitialized(message: &'static str) -> Self {
        Self::new(ErrorCategory::Memory, codes::INITIALIZATION_ERROR, message)
    }

    /// Create a new static error with explicit parameters
    #[must_use]
    pub const fn new_static(category: ErrorCategory, code: u16, message: &'static str) -> Self {
        Self::new(category, code, message)
    }

    // Agent C Component Model error factory methods

    /// Create a WIT input too large error
    #[must_use]
    pub const fn wit_input_too_large(message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::WIT_INPUT_TOO_LARGE, message)
    }

    /// Create a WIT world limit exceeded error
    #[must_use]
    pub const fn wit_world_limit_exceeded(message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::WIT_WORLD_LIMIT_EXCEEDED, message)
    }

    /// Create a WIT interface limit exceeded error
    #[must_use]
    pub const fn wit_interface_limit_exceeded(message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::WIT_INTERFACE_LIMIT_EXCEEDED, message)
    }

    /// Create a capability violation error
    #[must_use]
    pub const fn capability_violation(message: &'static str) -> Self {
        Self::new(ErrorCategory::Security, crate::codes::ACCESS_DENIED, message)
    }

    /// Create a no capability error
    #[must_use]
    pub const fn no_capability(message: &'static str) -> Self {
        Self::new(ErrorCategory::Security, crate::codes::ACCESS_DENIED, message)
    }

    /// Create a no WIT definitions found error
    #[must_use]
    pub const fn no_wit_definitions_found(message: &'static str) -> Self {
        Self::new(ErrorCategory::Parse, codes::NO_WIT_DEFINITIONS_FOUND, message)
    }

    /// Create an insufficient memory error
    #[must_use]
    pub const fn insufficient_memory(message: &'static str) -> Self {
        Self::new(ErrorCategory::Resource, codes::INSUFFICIENT_MEMORY, message)
    }

    /// Create an out of memory error
    #[must_use]
    pub const fn out_of_memory(message: &'static str) -> Self {
        Self::new(ErrorCategory::Resource, codes::OUT_OF_MEMORY, message)
    }

    /// Create a too many components error
    #[must_use]
    pub const fn too_many_components(message: &'static str) -> Self {
        Self::new(ErrorCategory::Component, codes::TOO_MANY_COMPONENTS, message)
    }

    /// Create a component not found error
    #[must_use]
    pub const fn component_not_found(message: &'static str) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_NOT_FOUND, message)
    }

    /// Create a component error with context
    #[must_use]
    pub const fn component_error_context(message: &'static str) -> Self {
        Self::new(ErrorCategory::Component, codes::COMPONENT_ERROR, message)
    }

    // Note: Methods like `with_message`, `new_legacy`, `*_with_code`,
    // and `parse_error_from_kind` have been removed as they were
    // Binary std/no_std choice
    // They can be re-added if versions compatible with `&'static str` messages are
    // designed.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ASIL-C and above: Include ASIL level in error display
        #[cfg(any(feature = "asil-c", feature = "asil-d"))]
        {
            write!(
                f,
                "[{:?}][E{:04X}][{}] {}",
                self.category,
                self.code,
                self.asil_level(),
                self.message
            )
        }
        #[cfg(not(any(feature = "asil-c", feature = "asil-d")))]
        {
            write!(f, "[{:?}][E{:04X}] {}", self.category, self.code, self.message)
        }
    }
}

impl ErrorSource for Error {
    fn code(&self) -> u16 {
        self.code
    }

    fn message(&self) -> &'static str {
        self.message
    }

    fn category(&self) -> ErrorCategory {
        self.category
    }
}

/// `Error` codes for different categories
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

    // Component Model WIT parsing errors (Agent C) (1200-1299)
    /// Error code for WIT input too large.
    pub const WIT_INPUT_TOO_LARGE: u16 = 1200;
    /// Error code for WIT world limit exceeded.
    pub const WIT_WORLD_LIMIT_EXCEEDED: u16 = 1201;
    /// Error code for WIT interface limit exceeded.
    pub const WIT_INTERFACE_LIMIT_EXCEEDED: u16 = 1202;
    /// Error code for no WIT definitions found.
    pub const NO_WIT_DEFINITIONS_FOUND: u16 = 1203;
    /// Error code for WIT parse error.
    pub const WIT_PARSE_ERROR: u16 = 1204;

    // Component runtime errors (Agent C) (3100-3199)
    /// Error code for insufficient memory.
    pub const INSUFFICIENT_MEMORY: u16 = 3100;
    /// Error code for out of memory.
    pub const OUT_OF_MEMORY: u16 = 3101;
    /// Error code for too many components.
    pub const TOO_MANY_COMPONENTS: u16 = 3102;
    /// Error code for component not found.
    pub const COMPONENT_NOT_FOUND: u16 = 3103;
    /// Error code for invalid input.
    pub const INVALID_INPUT: u16 = 3104;
    /// Error code for unsupported operation.
    pub const UNSUPPORTED: u16 = 3105;
    /// Error code for component error with context.
    pub const COMPONENT_ERROR: u16 = 3106;
}

impl From<core::fmt::Error> for Error {
    fn from(_: core::fmt::Error) -> Self {
        Self::new(ErrorCategory::System, codes::SYSTEM_ERROR, "Formatting error (static)")
    }
}

// Conversion helpers for error kinds

// -- From<kinds::X> for Error implementations --
impl From<kinds::ValidationError> for Error {
    fn from(_e: kinds::ValidationError) -> Self {
        Self::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, "Validation error from kind")
    }
}

impl From<kinds::ParseError> for Error {
    fn from(_e: kinds::ParseError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::PARSE_ERROR, "Parse error from kind")
    }
}

impl From<kinds::OutOfBoundsError> for Error {
    fn from(_e: kinds::OutOfBoundsError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::MEMORY_OUT_OF_BOUNDS,
            "Out of bounds error from kind",
        )
    }
}

impl From<kinds::InvalidType> for Error {
    fn from(_e: kinds::InvalidType) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, "Invalid type error from kind")
    }
}

impl From<kinds::ResourceError> for Error {
    fn from(_e: kinds::ResourceError) -> Self {
        Self::new(ErrorCategory::Resource, codes::RESOURCE_ERROR, "Resource error from kind")
    }
}

impl From<kinds::ComponentError> for Error {
    fn from(_e: kinds::ComponentError) -> Self {
        Self::new(
            ErrorCategory::Component,
            codes::COMPONENT_LINKING_ERROR,
            "Component error from kind",
        )
    }
}

impl From<kinds::RuntimeError> for Error {
    fn from(_e: kinds::RuntimeError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Runtime error from kind")
    }
}

impl From<kinds::PoisonedLockError> for Error {
    fn from(_e: kinds::PoisonedLockError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::POISONED_LOCK, "Poisoned lock error from kind")
    }
}

impl From<kinds::TypeMismatchError> for Error {
    fn from(_e: kinds::TypeMismatchError) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, "Type mismatch error from kind")
    }
}

impl From<kinds::MemoryAccessOutOfBoundsError> for Error {
    fn from(_e: kinds::MemoryAccessOutOfBoundsError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
            "Memory access out of bounds error from kind",
        )
    }
}

impl From<kinds::TableAccessOutOfBounds> for Error {
    fn from(_e: kinds::TableAccessOutOfBounds) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::OUT_OF_BOUNDS_ERROR,
            "Table access out of bounds error from kind",
        )
    }
}

impl From<kinds::ConversionError> for Error {
    fn from(_e: kinds::ConversionError) -> Self {
        Self::new(ErrorCategory::System, codes::CONVERSION_ERROR, "Conversion error from kind")
    }
}

impl From<kinds::DivisionByZeroError> for Error {
    fn from(_e: kinds::DivisionByZeroError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Division by zero error from kind")
    }
}

impl From<kinds::IntegerOverflowError> for Error {
    fn from(_e: kinds::IntegerOverflowError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Integer overflow error from kind")
    }
}

impl From<kinds::StackUnderflow> for Error {
    fn from(_e: kinds::StackUnderflow) -> Self {
        Self::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow error from kind")
    }
}

impl From<kinds::TypeMismatch> for Error {
    fn from(_e: kinds::TypeMismatch) -> Self {
        Self::new(ErrorCategory::Type, codes::TYPE_MISMATCH_ERROR, "Type mismatch from kind")
    }
}

impl From<kinds::InvalidTableIndexError> for Error {
    fn from(_e: kinds::InvalidTableIndexError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::OUT_OF_BOUNDS_ERROR,
            "Invalid table index error from kind",
        )
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

// -- FromError<kinds::X> for Error implementations --
impl FromError<kinds::ValidationError> for Error {
    fn from_error(_error: kinds::ValidationError) -> Self {
        Self::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Validation error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ParseError> for Error {
    fn from_error(_error: kinds::ParseError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::PARSE_ERROR, "Parse error from kind (FromError)")
    }
}

impl FromError<kinds::OutOfBoundsError> for Error {
    fn from_error(_error: kinds::OutOfBoundsError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::MEMORY_OUT_OF_BOUNDS,
            "Out of bounds error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidType> for Error {
    fn from_error(_error: kinds::InvalidType) -> Self {
        Self::new(
            ErrorCategory::Type,
            codes::TYPE_MISMATCH_ERROR,
            "Invalid type error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ResourceError> for Error {
    fn from_error(_error: kinds::ResourceError) -> Self {
        Self::new(
            ErrorCategory::Resource,
            codes::RESOURCE_ERROR,
            "Resource error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ComponentError> for Error {
    fn from_error(_error: kinds::ComponentError) -> Self {
        Self::new(
            ErrorCategory::Component,
            codes::COMPONENT_LINKING_ERROR,
            "Component error from kind (FromError)",
        )
    }
}

impl FromError<kinds::RuntimeError> for Error {
    fn from_error(_error: kinds::RuntimeError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::RUNTIME_ERROR,
            "Runtime error from kind (FromError)",
        )
    }
}

impl FromError<kinds::PoisonedLockError> for Error {
    fn from_error(_error: kinds::PoisonedLockError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::POISONED_LOCK,
            "Poisoned lock error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ArithmeticError> for Error {
    fn from_error(_error: kinds::ArithmeticError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::EXECUTION_ERROR,
            "Arithmetic error from kind (FromError)",
        )
    }
}

impl FromError<kinds::MemoryAccessError> for Error {
    fn from_error(_error: kinds::MemoryAccessError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::MEMORY_ACCESS_ERROR,
            "Memory access error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ResourceExhaustionError> for Error {
    fn from_error(_error: kinds::ResourceExhaustionError) -> Self {
        Self::new(
            ErrorCategory::Resource,
            codes::RESOURCE_LIMIT_EXCEEDED,
            "Resource exhaustion error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidIndexError> for Error {
    fn from_error(_error: kinds::InvalidIndexError) -> Self {
        Self::new(
            ErrorCategory::Validation,
            codes::OUT_OF_BOUNDS_ERROR,
            "Invalid index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ExecutionError> for Error {
    fn from_error(_error: kinds::ExecutionError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::EXECUTION_ERROR,
            "Execution error from kind (FromError)",
        )
    }
}

impl FromError<kinds::StackUnderflowError> for Error {
    fn from_error(_error: kinds::StackUnderflowError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::STACK_UNDERFLOW,
            "Stack underflow error from kind (FromError)",
        )
    }
}

impl FromError<kinds::ExportNotFoundError> for Error {
    fn from_error(_error: kinds::ExportNotFoundError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::RESOURCE_NOT_FOUND,
            "Export not found error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidInstanceIndexError> for Error {
    fn from_error(_error: kinds::InvalidInstanceIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::INVALID_INSTANCE_INDEX,
            "Invalid instance index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidFunctionIndexError> for Error {
    fn from_error(_error: kinds::InvalidFunctionIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::INVALID_FUNCTION_INDEX,
            "Invalid function index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidElementIndexError> for Error {
    fn from_error(_error: kinds::InvalidElementIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::OUT_OF_BOUNDS_ERROR,
            "Invalid element index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidMemoryIndexError> for Error {
    fn from_error(_error: kinds::InvalidMemoryIndexError) -> Self {
        Self::new(
            ErrorCategory::Memory,
            codes::MEMORY_OUT_OF_BOUNDS,
            "Invalid memory index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidGlobalIndexError> for Error {
    fn from_error(_error: kinds::InvalidGlobalIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::OUT_OF_BOUNDS_ERROR,
            "Invalid global index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidDataSegmentIndexError> for Error {
    fn from_error(_error: kinds::InvalidDataSegmentIndexError) -> Self {
        Self::new(
            ErrorCategory::Runtime,
            codes::OUT_OF_BOUNDS_ERROR,
            "Invalid data segment index error from kind (FromError)",
        )
    }
}

impl FromError<kinds::InvalidFunctionTypeError> for Error {
    fn from_error(_e: kinds::InvalidFunctionTypeError) -> Self {
        Self::new(
            ErrorCategory::Type,
            codes::INVALID_FUNCTION_TYPE,
            "Invalid function type error from kind (FromError)",
        )
    }
}

// --- START Wasm 2.0 From Impls ---
impl From<kinds::UnsupportedWasm20Feature> for Error {
    fn from(_e: kinds::UnsupportedWasm20Feature) -> Self {
        Self::new(
            ErrorCategory::Validation,
            codes::UNSUPPORTED_WASM20_FEATURE_ERROR,
            "Unsupported Wasm 2.0 feature",
        )
    }
}

impl From<kinds::InvalidReferenceTypeUsage> for Error {
    fn from(_e: kinds::InvalidReferenceTypeUsage) -> Self {
        Self::new(
            ErrorCategory::Type,
            codes::INVALID_REFERENCE_TYPE_USAGE_ERROR,
            "Invalid reference type usage",
        )
    }
}

impl From<kinds::BulkOperationError> for Error {
    fn from(_e: kinds::BulkOperationError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::BULK_OPERATION_ERROR, "Bulk operation error")
    }
}

impl From<kinds::SimdOperationError> for Error {
    fn from(_e: kinds::SimdOperationError) -> Self {
        Self::new(ErrorCategory::Runtime, codes::SIMD_OPERATION_ERROR, "SIMD operation error")
    }
}

impl From<kinds::TailCallError> for Error {
    fn from(_e: kinds::TailCallError) -> Self {
        Self::new(ErrorCategory::Validation, codes::TAIL_CALL_ERROR, "Tail call error")
    }
}
// --- END Wasm 2.0 From Impls ---

#[cfg(feature = "std")]
impl std::error::Error for Error {}
