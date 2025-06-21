/// Error types for DWARF debug information parsing
use wrt_error::{codes, Error, ErrorCategory};

/// Debug-specific error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugError {
    /// Invalid or corrupted DWARF data
    InvalidData,
    /// Unexpected end of data
    UnexpectedEof,
    /// Unsupported DWARF version
    UnsupportedVersion(u16),
    /// Invalid abbreviation code
    InvalidAbbreviation(u64),
    /// String table access error
    StringError,
}

/// Result type for debug operations
#[allow(dead_code)]
pub type DebugResult<T> = Result<T, DebugError>;

impl From<wrt_error::Error> for DebugError {
    fn from(_err: wrt_error::Error) -> Self {
        // For simplicity, map all WRT errors to InvalidData
        // A more sophisticated implementation could map specific error categories
        DebugError::InvalidData
    }
}

impl From<DebugError> for wrt_error::Error {
    fn from(err: DebugError) -> Self {
        match err {
            DebugError::InvalidData => Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Invalid DWARF data",
            ),
            DebugError::UnexpectedEof => Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Unexpected end of DWARF data",
            ),
            DebugError::UnsupportedVersion(_version) => Error::new(
                ErrorCategory::Parse,
                codes::VALIDATION_UNSUPPORTED_FEATURE,
                "Unsupported DWARF version",
            ),
            DebugError::InvalidAbbreviation(_code) => Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Invalid abbreviation code",
            ),
            DebugError::StringError => Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "String table access error",
            ),
        }
    }
}
