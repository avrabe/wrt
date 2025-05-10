//! Log handler for the WebAssembly Runtime.
//!
//! This module provides types for handling logs from WebAssembly components.

#[cfg(feature = "std")]
use std::{boxed::Box, string::String};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{boxed::Box, string::String};

use crate::operation::LogOperation;
use wrt_host::{callback::CallbackType, CallbackRegistry};

/// Function type for handling log operations
pub type LogHandler = Box<dyn Fn(LogOperation) + Send + Sync>;

/// Extension trait for CallbackRegistry to add logging-specific methods
pub trait LoggingExt {
    /// Register a log handler
    fn register_log_handler<F>(&mut self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static;

    /// Handle a log operation
    fn handle_log(&self, operation: LogOperation);

    /// Check if a log handler is registered
    fn has_log_handler(&self) -> bool;
}

impl LoggingExt for CallbackRegistry {
    fn register_log_handler<F>(&mut self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static,
    {
        self.register_callback(CallbackType::Logging, Box::new(handler) as LogHandler);
    }

    fn handle_log(&self, operation: LogOperation) {
        if let Some(handler) = self.get_callback::<LogHandler>(&CallbackType::Logging) {
            handler(operation);
        }
    }

    fn has_log_handler(&self) -> bool {
        self.get_callback::<LogHandler>(&CallbackType::Logging).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LogLevel;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_logging_extension() {
        let mut registry = CallbackRegistry::new();

        // Test without handler
        assert!(!registry.has_log_handler());

        // Logging without handler should not panic
        registry.handle_log(LogOperation::new(LogLevel::Info, "test message".to_string()));

        // Register handler
        let received = Arc::new(Mutex::new(Vec::new()));
        {
            let received = received.clone();
            registry.register_log_handler(move |log_op| {
                received.lock().unwrap().push((log_op.level, log_op.message));
            });
        }

        // Test with handler
        assert!(registry.has_log_handler());

        // Log some messages
        registry.handle_log(LogOperation::new(LogLevel::Info, "info message".to_string()));

        registry.handle_log(LogOperation::new(LogLevel::Error, "error message".to_string()));

        // Check received messages
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0], (LogLevel::Info, "info message".to_string()));
        assert_eq!(received[1], (LogLevel::Error, "error message".to_string()));
    }
}
