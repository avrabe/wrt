//! Test `no_std` compatibility for wrt-error
//!
//! This file validates that the wrt-error crate works correctly in `no_std` environments.

// For testing in a no_std environment
#![cfg_attr(not(feature = "std"), no_std)]

// External crate imports
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unnecessary_literal_unwrap, clippy::panic)]
mod tests {
    // Import necessary types for no_std environment
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::{format, string::ToString};

    #[cfg(feature = "std")]
    use std::string::ToString;

    // Import from wrt-error
    use wrt_error::{codes, Error, ErrorCategory, Result};

    // Only import ResultExt when using alloc feature
    #[cfg(feature = "alloc")]
    use wrt_error::context::ResultExt;

    use wrt_error::kinds;

    #[test]
    fn test_error_creation() {
        // Create an error
        let error = Error::new(
            ErrorCategory::Core,
            codes::INVALID_MEMORY_ACCESS,
            #[cfg(any(feature = "std", feature = "alloc"))]
            "Invalid memory access".to_string(),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            "Invalid memory access",
        );

        // Verify error properties
        assert_eq!(error.category, ErrorCategory::Core);
        assert_eq!(error.code, codes::INVALID_MEMORY_ACCESS);
        #[cfg(feature = "alloc")]
        assert_eq!(error.message, "Invalid memory access");
    }

    // Only run this test when alloc feature is enabled
    #[test]
    #[cfg(feature = "alloc")]
    fn test_error_with_context() {
        // Create an error with context
        let result: Result<()> = Err(Error::new(
            ErrorCategory::Core,
            codes::INVALID_MEMORY_ACCESS,
            "Invalid memory access".to_string(),
        ));

        // Add context using key-value
        let result_with_context = result.with_key_value("test", 42);

        // Verify the error remains
        assert!(result_with_context.is_err());

        // Check the error message contains the key and value
        let err_message = format!("{}", result_with_context.unwrap_err());
        assert!(err_message.contains("test: 42"));
    }

    #[test]
    fn test_result_operations() {
        // Test successful result
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        // Test error result
        let error = Error::new(
            ErrorCategory::Core,
            codes::INVALID_MEMORY_ACCESS,
            #[cfg(any(feature = "std", feature = "alloc"))]
            "Invalid memory access".to_string(),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            "Invalid memory access",
        );

        let err_result: Result<i32> = Err(error);
        assert!(err_result.is_err());

        let extracted_error = err_result.unwrap_err();
        assert_eq!(extracted_error.category, ErrorCategory::Core);
    }

    #[test]
    fn test_error_categories() {
        // Test error categories
        assert_ne!(ErrorCategory::Core, ErrorCategory::Resource);
        assert_ne!(ErrorCategory::Memory, ErrorCategory::Validation);
        assert_ne!(ErrorCategory::Validation, ErrorCategory::Runtime);
        assert_ne!(ErrorCategory::Runtime, ErrorCategory::System);
    }

    #[test]
    fn test_error_kind() {
        let validation_error = kinds::validation_error("Validation error");
        let _memory_error = kinds::memory_access_error("Memory error");
        let _runtime_error = kinds::runtime_error("Runtime error");

        assert!(format!("{validation_error:?}").contains("ValidationError"));
    }

    // Helper to get the concrete type
    #[allow(dead_code)]
    fn types_of<T>(_: T) -> T {
        panic!("This function should never be called")
    }
}
