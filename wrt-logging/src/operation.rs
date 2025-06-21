//! Log operation for the WebAssembly Runtime.
//!
//! This module provides types for representing log operations in component
//! logging.


#[cfg(feature = "std")]
use std::string::String;
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::string::String;

use crate::level::LogLevel;

// Binary std/no_std choice
#[cfg(feature = "std")]
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

// For pure no_std configuration, use bounded strings
#[cfg(all(not(feature = "std"), not(feature = "std")))]
/// Log operation from a WebAssembly component
#[derive(Debug, Clone)]
pub struct LogOperation<P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq = wrt_foundation::NoStdProvider<512>> {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: wrt_foundation::BoundedString<256, P>,
    /// Component ID (optional)
    pub component_id: Option<wrt_foundation::BoundedString<64, P>>,
}

// Binary std/no_std choice
#[cfg(feature = "std")]
impl LogOperation {
    /// Create a new log operation
    #[must_use]
    pub fn new(level: LogLevel, message: String) -> Self {
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

// Implementation for pure no_std configuration
#[cfg(all(not(feature = "std"), not(feature = "std")))]
impl<P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq> LogOperation<P> {
    /// Create a new log operation
    pub fn new(level: LogLevel, message: &str, provider: P) -> wrt_foundation::Result<Self> {
        let bounded_message = wrt_foundation::BoundedString::from_str(message, provider)?;
        Ok(Self { level, message: bounded_message, component_id: None })
    }

    /// Create a new log operation with a component ID
    pub fn with_component(
        level: LogLevel,
        message: &str,
        component_id: &str,
        provider: P,
    ) -> wrt_foundation::Result<Self> {
        let bounded_message = wrt_foundation::BoundedString::from_str(message, provider.clone())?;
        let bounded_component_id = wrt_foundation::BoundedString::from_str(component_id, provider)?;
        Ok(Self { level, message: bounded_message, component_id: Some(bounded_component_id) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LogLevel;
    #[cfg(feature = "std")]
    use std::string::ToString;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    #[test]
    fn test_log_operation_creation() {
        #[cfg(feature = "std")]
        {
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
        
        #[cfg(not(feature = "std"))]
        {
            use wrt_foundation::safe_memory::NoStdProvider;
            
            // Create a provider for testing
            let provider = NoStdProvider::<512>::default();
            
            // Test basic creation
            let op = LogOperation::new(LogLevel::Info, "test message", provider.clone()).unwrap();
            assert_eq!(op.level, LogLevel::Info);
            assert_eq!(op.message.as_str().unwrap(), "test message");
            assert!(op.component_id.is_none());

            // Test with component ID
            let op = LogOperation::with_component(LogLevel::Debug, "test message", "component-1", provider.clone()).unwrap();
            assert_eq!(op.level, LogLevel::Debug);
            assert_eq!(op.message.as_str().unwrap(), "test message");
            assert_eq!(op.component_id.as_ref().map(|s| s.as_str().unwrap()), Some("component-1"));
        }
    }
}
