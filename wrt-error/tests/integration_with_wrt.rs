//! Integration tests for wrt-error with the main wrt crate.
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Tests the integration of wrt-error with the main wrt crate.
//
// This file tests the error handling functionality in the context of how it would be used
/// in the main wrt crate.

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::unnecessary_literal_unwrap,
    clippy::needless_borrows_for_generic_args,
    clippy::uninlined_format_args
)]
mod tests {
    use wrt_error::{Error, Result};

    // Note: `format!` macro resolves to `alloc::format!` or `std::format!` based on context
    // if the respective crate (alloc or std) is linked.
    // The `Display` trait on `wrt_error::Error` handles this internally.

    #[test]
    fn test_result_operations() {
        let result: Result<()> = Ok(());
        assert!(result.is_ok());
        result.unwrap(); // Should not panic

        let error_result: Result<()> = Err(Error::runtime_error("Test error"));
        assert!(error_result.is_err());
        // To display, e.g., in a no_std,alloc test if direct formatting is needed:
        // #[cfg(all(not(feature = "std"), feature = "alloc"))]
        // let _ = alloc::format!("{}", error_result.as_ref().err().unwrap());
        // #[cfg(feature = "std")]
        // let _ = std::format!("{}", error_result.as_ref().err().unwrap());
    }

    #[cfg(all(feature = "alloc", feature = "std"))]
    mod std_alloc_tests {
        use wrt_error::kinds;
        use wrt_error::Error;
        // Re-import necessary items if not directly available or for clarity
        // use crate::Error; // Assuming Error is pub from lib.rs

        #[test]
        fn test_error_conversion_memory() {
            let mem_error = kinds::MemoryAccessOutOfBoundsError { address: 100, length: 32 };
            // In std mode, plain format! is from std.
            let error = Error::memory_error(format!("Memory error: {mem_error}"));
            assert!(error.is_memory_error());
        }

        #[test]
        fn test_error_conversion_division() {
            let div_error = kinds::division_by_zero_error();
            let error = Error::from(div_error);
            // In std mode, plain format! is from std.
            assert!(format!("{error}").contains("Division by zero"));
        }
    }
}
