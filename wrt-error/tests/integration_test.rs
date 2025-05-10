#![allow(clippy::unwrap_used)]
//! Integration tests for the wrt-error crate.

#[cfg(test)]
mod tests {
    use wrt_error::codes;
    use wrt_error::kinds;
    use wrt_error::{Error, ErrorCategory, Result};

    #[test]
    fn test_error_creation() {
        let error =
            Error::new(ErrorCategory::Memory, codes::MEMORY_ACCESS_OUT_OF_BOUNDS, "Test error");
        assert!(error.is_memory_error());
        assert_eq!(error.code, codes::MEMORY_ACCESS_OUT_OF_BOUNDS);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_error_from_kind() {
        let kind = kinds::validation_error("Validation failed");
        let error = Error::from(kind);

        // Just verify the error was created (no specific category check)
        assert!(format!("{error}").contains("Validation failed"));
    }

    #[test]
    fn test_result_with_error() {
        let result: Result<i32> = Err(Error::runtime_error("Runtime error"));
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(error.is_runtime_error());
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_error_conversion() {
        // StringError can be converted to Error
        let message = "Error message".to_string();
        let error: Error = message.into();

        assert_eq!(error.code, codes::UNKNOWN);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_error_source() {
        // Create an error with a source
        let stack_error = kinds::stack_underflow();
        let error = Error::from(stack_error);

        // Just verify the error was created with the correct message
        assert!(format!("{error}").contains("Stack underflow"));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_error_conversion_from_structs() {
        // Test OutOfBoundsError
        let bounds_error = kinds::out_of_bounds_error("Index out of bounds");
        let error = Error::from(bounds_error);

        // Just verify the error was created with the correct message
        assert!(format!("{error}").contains("Index out of bounds"));

        // Test another type of error
        let table_error = kinds::invalid_table_index_error(5);
        let error = Error::from(table_error);

        // Verify the error message mentions index 5
        assert!(format!("{error}").contains('5'));
    }
}
