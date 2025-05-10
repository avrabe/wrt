//! Formal verification for the error handling system using Kani.
//!
//! This module contains proofs that verify core properties of the error handling system.
//! These proofs only run with Kani and are isolated from normal compilation and testing.

// Only compile Kani verification code when documentation is being generated
// or when explicitly running cargo kani. This prevents interference with
// coverage testing.
#[cfg(any(doc, kani))]
/// Kani verification proofs for error handling.
pub mod kani_verification {
    use crate::Result;
    use crate::{ErrorCategory, ErrorSource};

    // Only import Error and ResultExt when alloc is available
    #[cfg(feature = "alloc")]
    use crate::{Error, ResultExt};
    #[cfg(feature = "alloc")]
    use alloc::format;

    use core::fmt::{self, Debug, Display};

    // A simple test error for verification
    #[derive(Debug, Clone)]
    struct VerifyError(&'static str);

    impl Display for VerifyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "VerifyError: {}", self.0)
        }
    }

    impl ErrorSource for VerifyError {
        #[cfg(feature = "std")]
        fn source(&self) -> Option<&(dyn ErrorSource + 'static)> {
            None
        }

        fn code(&self) -> u16 {
            // Use a fixed code for verification errors
            9999
        }

        fn message(&self) -> &str {
            self.0
        }

        fn category(&self) -> ErrorCategory {
            // Use validation category for verification errors
            ErrorCategory::Validation
        }
    }

    /// Verify that creating and displaying an error works correctly
    #[cfg(feature = "alloc")]
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_error_creation() {
        let error = Error::new(VerifyError("verification test"));
        let error_str = format!("{}", error);

        // Verify display formatting
        assert!(error_str.contains("VerifyError: verification test"));
    }

    /// Verify that error context chaining works correctly
    #[cfg(feature = "alloc")]
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_error_context() {
        let result: core::result::Result<(), VerifyError> = Err(VerifyError("base error"));
        let with_context = result.context("operation failed");

        assert!(with_context.is_err());
        let error = with_context.unwrap_err();
        let error_str = format!("{}", error);

        // Verify both context and original error appear in display string
        assert!(error_str.contains("operation failed"));
        assert!(error_str.contains("base error"));
    }

    /// Verify that multiple contexts chain correctly
    #[cfg(feature = "alloc")]
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_multiple_contexts() {
        let result: Result<()> = Err(Error::new(VerifyError("original error")))
            .context("first context")
            .context("second context");

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_str = format!("{}", error);

        // All context levels should be present
        assert!(error_str.contains("second context"));
        assert!(error_str.contains("first context"));
        assert!(error_str.contains("original error"));
    }

    /// Verify that factory methods create the correct error types
    #[cfg(feature = "alloc")]
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_factory_methods() {
        let div_error = Error::division_by_zero();
        assert!(format!("{}", div_error) == "Division by zero");

        let memory_error = Error::memory_access_out_of_bounds(1000, 8);
        assert!(
            format!("{}", memory_error) == "Memory access out of bounds: address 1000, length 8"
        );
    }

    /// Verify that Error::from works correctly
    #[cfg(feature = "alloc")]
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_error_from() {
        let original_error = VerifyError("source error");
        let error = Error::from(original_error);

        assert!(format!("{}", error) == "VerifyError: source error");
    }

    /// Verify that Result works correctly with different error types
    #[cfg_attr(kani, kani::proof)]
    pub fn verify_result_type() {
        // Test with the default error type
        #[cfg(feature = "alloc")]
        {
            let result1: Result<i32> = Ok(42);
            assert_eq!(result1.unwrap(), 42);

            let result2: Result<i32> = Err(Error::division_by_zero());
            assert!(result2.is_err());
        }

        // Test with a custom error type
        let result3: Result<i32, VerifyError> = Ok(42);
        assert_eq!(result3.unwrap(), 42);

        let result4: Result<i32, VerifyError> = Err(VerifyError("custom error"));
        assert!(result4.is_err());
    }
}

// Expose the verification module in docs but not for normal compilation
#[cfg(any(doc, kani))]
pub use kani_verification::*;
