//! Log handler for the WebAssembly Runtime.
//!
//! This module provides types for handling logs from WebAssembly components.


#[cfg(feature = "std")]
use std::boxed::Box;

use wrt_host::{CallbackRegistry, CallbackType};

use crate::operation::LogOperation;

// Binary std/no_std choice
#[cfg(feature = "std")]
/// Function type for handling log operations
pub type LogHandler = Box<dyn Fn(LogOperation) + Send + Sync>;

#[cfg(feature = "std")]
/// Extension trait for CallbackRegistry to add logging-specific methods
pub trait LoggingExt {
    /// Register a log handler
    fn register_log_handler<F>(&mut self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static;

    /// Handle a log operation
    fn handle_log(&self, operation: LogOperation;

    /// Check if a log handler is registered
    fn has_log_handler(&self) -> bool;
}

// For pure no_std configuration
#[cfg(all(not(feature = "std"), not(feature = "std")))]
/// Function type for handling log operations (no dynamic dispatch in `no_std`)
pub type LogHandler<P> = fn(LogOperation<P>;

#[cfg(all(not(feature = "std"), not(feature = "std")))]
/// Extension trait for `CallbackRegistry` to add logging-specific methods (`no_std`)
pub trait LoggingExt {
    /// Register a simple log handler function (`no_std` only supports function pointers)
    fn register_log_handler<P>(&mut self, handler: LogHandler<P>)
    where
        P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq;

    /// Handle a log operation
    fn handle_log<P>(&self, operation: LogOperation<P>) -> wrt_foundation::Result<()>
    where
        P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq;

    /// Check if a log handler is registered
    fn has_log_handler(&self) -> bool;
}

// Binary std/no_std choice
#[cfg(feature = "std")]
impl LoggingExt for CallbackRegistry {
    fn register_log_handler<F>(&mut self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static,
    {
        self.register_callback(CallbackType::Logging, Box::new(handler) as LogHandler;
    }

    fn handle_log(&self, operation: LogOperation) {
        if let Some(handler) = self.get_callback::<LogHandler>(&CallbackType::Logging) {
            handler(operation;
        }
    }

    fn has_log_handler(&self) -> bool {
        self.get_callback::<LogHandler>(&CallbackType::Logging).is_some()
    }
}

// Implementation for pure no_std configuration
#[cfg(all(not(feature = "std"), not(feature = "std")))]
impl LoggingExt for CallbackRegistry {
    fn register_log_handler<P>(&mut self, handler: LogHandler<P>)
    where
        P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq,
    {
        // In no_std mode, we can't store dynamic handlers
        // This is a limitation - only one handler per type can be stored
        let _ = handler; // Acknowledge the parameter
        // Note: Actual registration would require a more complex design
    }

    fn handle_log<P>(&self, operation: LogOperation<P>) -> wrt_foundation::Result<()>
    where
        P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq,
    {
        // In no_std mode, we can't dynamically dispatch to handlers
        let _ = operation; // Acknowledge the parameter
        // Default no-op implementation for no_std
        Ok(())
    }

    fn has_log_handler(&self) -> bool {
        // In no_std mode, we can't track handlers dynamically
        false
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::level::LogLevel;

    #[test]
    fn test_logging_extension() {
        let mut registry = CallbackRegistry::new);

        // Test without handler
        assert!(!registry.has_log_handler();

        // Logging without handler should not panic
        registry.handle_log(LogOperation::new(LogLevel::Info, "test message".to_string();

        // Register handler
        let received = Arc::new(Mutex::new(Vec::new();
        {
            let received = received.clone();
            registry.register_log_handler(move |log_op| {
                received.lock().unwrap().push((log_op.level, log_op.message);
            };
        }

        // Test with handler
        assert!(registry.has_log_handler();

        // Log some messages
        registry.handle_log(LogOperation::new(LogLevel::Info, "info message".to_string();

        registry.handle_log(LogOperation::new(LogLevel::Error, "error message".to_string();

        // Check received messages
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 2;
        assert_eq!(received[0], (LogLevel::Info, "info message".to_string();
        assert_eq!(received[1], (LogLevel::Error, "error message".to_string();
    }
}

// Binary std/no_std choice
#[cfg(test)]
mod no_std_alloc_tests {
    use std::vec::Vec;
    use core::cell::RefCell;

    use super::*;
    use crate::level::LogLevel;

    #[test]
    fn test_no_std_logging_extension() {
        let mut registry = CallbackRegistry::new);

        // Test without handler
        assert!(!registry.has_log_handler();

        // Logging without handler should not panic
        registry.handle_log(LogOperation::new(LogLevel::Info, "test message".to_string();

        // Use RefCell instead of Mutex for no_std
        let received = RefCell::new(Vec::new);

        registry.register_log_handler(move |log_op| {
            received.borrow_mut().push((log_op.level, log_op.message);
        };

        // Test with handler
        assert!(registry.has_log_handler();

        // Log some messages
        registry.handle_log(LogOperation::new(LogLevel::Info, "info message".to_string();
        registry.handle_log(LogOperation::new(LogLevel::Error, "error message".to_string();

        // Check received messages
        let borrowed = received.borrow);
        assert_eq!(borrowed.len(), 2;
        assert_eq!(borrowed[0], (LogLevel::Info, "info message".to_string();
        assert_eq!(borrowed[1], (LogLevel::Error, "error message".to_string();
    }
}
