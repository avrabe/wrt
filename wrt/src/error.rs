use crate::String;
use std::fmt;

#[cfg(not(feature = "std"))]
use alloc::string::ToString;

/// Represents errors that can occur in the WebAssembly runtime.
///
/// This enum covers the various categories of errors that can occur during WebAssembly
/// module validation, execution, parsing, I/O operations, and component model operations.
///
/// # Examples
///
/// ```
/// use wrt::Error;
///
/// let err = Error::Validation("Invalid type".to_string());
/// assert!(err.to_string().contains("Validation error"));
/// ```
#[derive(Debug, Clone)]
pub enum Error {
    /// Represents validation errors that occur when a WebAssembly module
    /// fails to meet the WebAssembly specification requirements.
    Validation(String),

    /// Represents errors that occur during the execution of a WebAssembly module,
    /// such as out-of-bounds memory access, stack overflow, or type mismatches.
    Execution(String),

    /// Represents that execution has paused due to fuel exhaustion.
    /// This is not a true error but a signal that execution can be resumed.
    FuelExhausted,

    /// Represents input/output errors, typically when reading from or writing to
    /// a file system or other I/O operations.
    IO(String),

    /// Represents parsing errors that occur when decoding a WebAssembly binary
    /// or parsing a WebAssembly text format.
    Parse(String),

    /// Represents errors related to the WebAssembly Component Model, such as
    /// component instantiation failures or interface type mismatches.
    Component(String),

    /// Represents custom errors that don't fit into the other categories.
    /// This is useful for extension points or for wrapping errors from
    /// other libraries.
    Custom(String),

    /// Represents a stack underflow error that occurs when trying to pop from an empty stack.
    StackUnderflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Validation(msg) => write!(f, "Validation error: {}", msg),
            Error::Execution(msg) => write!(f, "Execution error: {}", msg),
            Error::FuelExhausted => write!(f, "Execution paused: out of fuel"),
            Error::IO(msg) => write!(f, "IO error: {}", msg),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::Component(msg) => write!(f, "Component error: {}", msg),
            Error::Custom(msg) => write!(f, "{}", msg),
            Error::StackUnderflow => write!(f, "Stack underflow error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// A type alias for Result with the error type fixed to [`Error`].
///
/// This allows for more concise return types in functions that can fail with
/// a WebAssembly runtime error.
///
/// # Examples
///
/// ```
/// use wrt::{Result, Error};
///
/// fn some_operation() -> Result<i32> {
///     // Implementation that might fail
///     Ok(42)
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::format;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    #[test]
    fn test_error_creation() {
        // Test each error variant
        let validation_err = Error::Validation("invalid type".to_string());
        let execution_err = Error::Execution("stack overflow".to_string());
        let fuel_err = Error::FuelExhausted;
        let io_err = Error::IO("file not found".to_string());
        let parse_err = Error::Parse("invalid binary".to_string());
        let component_err = Error::Component("instantiation failed".to_string());
        let custom_err = Error::Custom("custom error".to_string());

        // Verify Debug implementation
        assert!(format!("{:?}", validation_err).contains("Validation"));
        assert!(format!("{:?}", execution_err).contains("Execution"));
        assert!(format!("{:?}", fuel_err).contains("FuelExhausted"));
        assert!(format!("{:?}", io_err).contains("IO"));
        assert!(format!("{:?}", parse_err).contains("Parse"));
        assert!(format!("{:?}", component_err).contains("Component"));
        assert!(format!("{:?}", custom_err).contains("Custom"));
    }

    #[test]
    fn test_error_display() {
        // Test Display implementation for each variant
        assert_eq!(
            Error::Validation("test".to_string()).to_string(),
            "Validation error: test"
        );
        assert_eq!(
            Error::Execution("test".to_string()).to_string(),
            "Execution error: test"
        );
        assert_eq!(
            Error::FuelExhausted.to_string(),
            "Execution paused: out of fuel"
        );
        assert_eq!(Error::IO("test".to_string()).to_string(), "IO error: test");
        assert_eq!(
            Error::Parse("test".to_string()).to_string(),
            "Parse error: test"
        );
        assert_eq!(
            Error::Component("test".to_string()).to_string(),
            "Component error: test"
        );
        assert_eq!(Error::Custom("test".to_string()).to_string(), "test");
        assert_eq!(Error::StackUnderflow.to_string(), "Stack underflow error");
    }

    #[test]
    fn test_error_clone() {
        let original = Error::Validation("test".to_string());
        let cloned = original.clone();

        // Use format! to avoid moving the values
        assert_eq!(format!("{}", original), format!("{}", cloned));
    }

    #[test]
    fn test_result_type() {
        // Test Ok case
        let ok_result: Result<i32> = Ok(42);
        assert_eq!(ok_result.unwrap(), 42);

        // Test Err case
        let err_result: Result<i32> = Err(Error::Validation("test".to_string()));
        assert!(err_result.is_err());
        assert!(matches!(err_result.unwrap_err(), Error::Validation(_)));
    }

    #[test]
    fn test_error_conversion() {
        // Test converting from string slice
        let msg = "test error";
        let validation_err = Error::Validation(msg.to_string());
        assert_eq!(validation_err.to_string(), "Validation error: test error");

        // Test error chaining with explicit type annotations
        let first_err: Result<()> = Err(Error::Validation("first".to_string()));
        let second_err: Result<()> = Err(Error::Execution("second".to_string()));
        let result = first_err.and_then(|_| second_err);
        assert!(matches!(result.unwrap_err(), Error::Validation(_)));
    }

    #[test]
    fn test_error_patterns() {
        let err = Error::Validation("test".to_string());

        // Test pattern matching
        match &err {
            Error::Validation(msg) => assert_eq!(msg, "test"),
            _ => panic!("Expected Validation variant"),
        }

        // Test if-let pattern with explicit type annotation
        if let Error::Validation(ref msg) = err {
            assert_eq!(msg, "test");
        } else {
            panic!("Expected Validation variant");
        }
    }

    #[test]
    fn test_result_combinators() {
        // Test map with explicit type annotations
        let ok_result: Result<i32> = Ok(42);
        let mapped: Result<i32> = ok_result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 84);

        // Test map_err
        let err_result: Result<i32> = Err(Error::Validation("test".to_string()));
        let mapped_err: Result<i32> = err_result.map_err(|_| Error::Custom("mapped".to_string()));
        assert!(matches!(mapped_err.unwrap_err(), Error::Custom(_)));

        // Test and_then with explicit type annotation
        let chained: Result<i32> = Ok(42).and_then(|x| Ok(x * 2)).and_then(|x| Ok(x + 1));
        assert_eq!(chained.unwrap(), 85);
    }

    #[test]
    fn test_error_handling() {
        // Test validation error
        let err = Error::Validation("test error".into());
        assert!(matches!(err, Error::Validation(_)));

        // Test execution error
        let err = Error::Execution("test error".into());
        assert!(matches!(err, Error::Execution(_)));

        // Test parse error
        let err = Error::Parse("test error".into());
        assert!(matches!(err, Error::Parse(_)));
    }
}
