//! Log operation for the WebAssembly Runtime.
//!
//! This module provides types for representing log operations in component logging.

#[cfg(feature = "std")]
use std::string::String;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::string::String;

use crate::level::LogLevel;

/// Log operation from a WebAssembly component
#[derive(Debug, Clone)]
pub struct LogOperation {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Component ID (optional)
    pub component_id: Option<String>,
}

impl LogOperation {
    /// Create a new log operation
    #[must_use]
    pub const fn new(level: LogLevel, message: String) -> Self {
        Self { level, message, component_id: None }
    }

    /// Create a new log operation with a component ID
    pub fn with_component<S1: Into<String>, S2: Into<String>>(
        level: LogLevel,
        message: S1,
        component_id: S2,
    ) -> Self {
        Self { level, message: message.into(), component_id: Some(component_id.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LogLevel;

    #[test]
    fn test_log_operation_creation() {
        // Test basic creation
        let op = LogOperation::new(LogLevel::Info, "test message".to_string());
        assert_eq!(op.level, LogLevel::Info);
        assert_eq!(op.message, "test message");
        assert!(op.component_id.is_none());

        // Test with component ID
        let op = LogOperation::with_component(LogLevel::Debug, "test message", "component-1");
        assert_eq!(op.level, LogLevel::Debug);
        assert_eq!(op.message, "test message");
        assert_eq!(op.component_id, Some("component-1".to_string()));

        // Test with string conversion
        let op2 = LogOperation::with_component(
            LogLevel::Debug,
            String::from("test message"),
            String::from("component-1"),
        );
        assert_eq!(op2.level, LogLevel::Debug);
        assert_eq!(op2.message, "test message");
        assert_eq!(op2.component_id, Some("component-1".to_string()));
    }
}
