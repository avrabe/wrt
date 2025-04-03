// Use our reexported types
use crate::values::Value;

// Core imports only - no std or duplicates
use core::any::Any;

#[cfg(not(feature = "std"))]
use core::str::FromStr;
#[cfg(feature = "std")]
use std::str::FromStr;

// Import std when available
#[cfg(feature = "std")]
use std::{boxed::Box, collections::HashMap, format, string::String, vec::Vec};

// Import alloc for no_std
#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box,
    collections::BTreeMap as HashMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Log levels for WebAssembly component logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Trace-level messages (detailed debugging information)
    Trace,
    /// Debug-level messages (useful for developers)
    Debug,
    /// Informational messages (general runtime information)
    Info,
    /// Warning messages (potential issues)
    Warn,
    /// Error messages (recoverable errors)
    Error,
    /// Critical error messages (severe issues)
    Critical,
}

/// Custom error for parsing log levels
#[derive(Debug)]
pub struct ParseLogLevelError {
    /// The invalid log level string that was provided
    pub invalid_level: String,
}

#[cfg(feature = "std")]
impl std::fmt::Display for ParseLogLevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid log level: {}", self.invalid_level)
    }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for ParseLogLevelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid log level: {}", self.invalid_level)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseLogLevelError {}

#[cfg(feature = "std")]
impl std::str::FromStr for LogLevel {
    type Err = ParseLogLevelError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" | "err" => Ok(Self::Error),
            "critical" | "fatal" => Ok(Self::Critical),
            _ => Err(ParseLogLevelError {
                invalid_level: s.to_string(),
            }),
        }
    }
}

#[cfg(not(feature = "std"))]
impl core::str::FromStr for LogLevel {
    type Err = ParseLogLevelError;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" | "err" => Ok(Self::Error),
            "critical" | "fatal" => Ok(Self::Critical),
            _ => Err(ParseLogLevelError {
                invalid_level: s.to_string(),
            }),
        }
    }
}

impl LogLevel {
    /// Creates a `LogLevel` from a string, defaulting to Info for invalid levels
    #[must_use]
    pub fn from_string_or_default(s: &str) -> Self {
        Self::from_str(s).unwrap_or(Self::Info)
    }

    /// Convert `LogLevel` to a string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

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
        Self {
            level,
            message,
            component_id: None,
        }
    }

    /// Create a new log operation with a component ID
    pub fn with_component<S1: Into<String>, S2: Into<String>>(
        level: LogLevel,
        message: S1,
        component_id: S2,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            component_id: Some(component_id.into()),
        }
    }
}

/// Log handler type for processing WebAssembly log operations
pub type LogHandler = Box<dyn Fn(LogOperation) + Send + Sync>;

/// A trait for functions that can be cloned and operate on `Vec<Value>`.
/// This is used for storing host functions that can be called by the Wasm engine.
pub trait FnWithVecValue: Send + Sync {
    /// Calls the function with the given target and arguments.
    fn call(&self, target: &mut dyn Any, args: Vec<Value>) -> crate::error::Result<Vec<Value>>;
    /// Clones the function into a `Box`.
    fn clone_box(&self) -> Box<dyn FnWithVecValue>;
}

// Implement the trait for function types that meet the requirements
impl<F> FnWithVecValue for F
where
    F: Fn(&mut dyn Any, Vec<Value>) -> crate::error::Result<Vec<Value>>
        + Send
        + Sync
        + Clone
        + 'static,
{
    fn call(&self, target: &mut dyn Any, args: Vec<Value>) -> crate::error::Result<Vec<Value>> {
        self(target, args)
    }

    fn clone_box(&self) -> Box<dyn FnWithVecValue> {
        Box::new(self.clone())
    }
}

/// A wrapper struct that makes a closure implementing `Fn` cloneable
/// by boxing it and handling the cloning via the `FnWithVecValue` trait.
pub struct CloneableFn(Box<dyn FnWithVecValue>);

impl CloneableFn {
    /// Creates a new `CloneableFn` from a closure.
    ///
    /// The closure must be `Send`, `Sync`, `Clone`, and `'static`.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut dyn Any, Vec<Value>) -> crate::error::Result<Vec<Value>>
            + Send
            + Sync
            + Clone
            + 'static,
    {
        Self(Box::new(f))
    }

    /// Calls the wrapped function.
    pub fn call(&self, target: &mut dyn Any, args: Vec<Value>) -> crate::error::Result<Vec<Value>> {
        self.0.call(target, args)
    }
}

impl Clone for CloneableFn {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

/// Host function handler type for implementing WebAssembly imports
pub type HostFunctionHandler = CloneableFn;

/// A callback registry for handling WebAssembly component operations
#[derive(Default)]
pub struct CallbackRegistry {
    /// Log handler (if registered)
    #[allow(clippy::type_complexity)]
    log_handler: Option<LogHandler>,

    /// Host functions registry (module name -> function name -> handler)
    #[cfg(feature = "std")]
    host_functions: HashMap<String, HashMap<String, HostFunctionHandler>>,

    /// Host functions registry (module name -> function name -> handler)
    #[cfg(not(feature = "std"))]
    host_functions: HashMap<String, HashMap<String, HostFunctionHandler>>,
}

#[cfg(feature = "std")]
impl std::fmt::Debug for CallbackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackRegistry")
            .field("has_log_handler", &self.log_handler.is_some())
            .field("registered_modules", &self.host_functions.keys())
            .finish()
    }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Debug for CallbackRegistry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CallbackRegistry")
            .field("has_log_handler", &self.log_handler.is_some())
            .field("registered_modules", &self.host_functions.keys())
            .finish()
    }
}

impl CallbackRegistry {
    /// Create a new callback registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            log_handler: None,
            #[cfg(feature = "std")]
            host_functions: HashMap::new(),
            #[cfg(not(feature = "std"))]
            host_functions: HashMap::new(),
        }
    }

    /// Register a log handler
    pub fn register_log_handler<F>(&mut self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static,
    {
        self.log_handler = Some(Box::new(handler));
    }

    /// Register a log handler (alias for `register_log_handler` for backward compatibility)
    pub fn register_log<F>(&mut self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static,
    {
        self.register_log_handler(handler);
    }

    /// Handle a log operation
    pub fn handle_log(&self, operation: LogOperation) {
        if let Some(handler) = &self.log_handler {
            handler(operation);
        }
    }

    /// Check if a log handler is registered
    #[must_use]
    pub fn has_log_handler(&self) -> bool {
        self.log_handler.is_some()
    }

    /// Register a host function
    pub fn register_host_function(
        &mut self,
        module_name: &str,
        function_name: &str,
        handler: HostFunctionHandler,
    ) {
        let module_name = module_name.to_string();
        let function_name = function_name.to_string();

        let module_functions = self.host_functions.entry(module_name).or_insert_with(|| {
            #[cfg(feature = "std")]
            return HashMap::new();
            #[cfg(not(feature = "std"))]
            return HashMap::new();
        });

        module_functions.insert(function_name, handler);
    }

    /// Check if a host function is registered
    #[must_use]
    pub fn has_host_function(&self, module_name: &str, function_name: &str) -> bool {
        self.host_functions
            .get(module_name)
            .and_then(|funcs| funcs.get(function_name))
            .is_some()
    }

    /// Call a host function
    ///
    /// # Errors
    ///
    /// Returns an error if the host function is not found or fails during execution
    pub fn call_host_function(
        &self,
        engine: &mut dyn Any,
        module_name: &str,
        function_name: &str,
        args: Vec<Value>,
    ) -> crate::error::Result<Vec<Value>> {
        if let Some(module_functions) = self.host_functions.get(module_name) {
            if let Some(handler) = module_functions.get(function_name) {
                return handler.call(engine, args);
            }
        }

        Err(crate::error::Error::Execution(format!(
            "Host function {module_name}.{function_name} not found"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    use std::sync::{Arc, Mutex};

    #[cfg(not(feature = "std"))]
    use crate::sync::Mutex;

    #[cfg(not(feature = "std"))]
    use alloc::sync::Arc;

    #[test]
    fn test_log_level_parsing() {
        // Test valid log levels
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("critical".parse::<LogLevel>().unwrap(), LogLevel::Critical);

        // Test case insensitivity
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("Warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);

        // Test invalid log levels
        assert!("invalid".parse::<LogLevel>().is_err());
        assert!("".parse::<LogLevel>().is_err());

        // Test error message
        let err = "invalid".parse::<LogLevel>().unwrap_err();
        assert_eq!(err.invalid_level, "invalid");
        assert_eq!(err.to_string(), "Invalid log level: invalid");
    }

    #[test]
    fn test_log_level_from_string_or_default() {
        // Test valid log levels
        assert_eq!(LogLevel::from_string_or_default("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from_string_or_default("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_string_or_default("info"), LogLevel::Info);
        assert_eq!(LogLevel::from_string_or_default("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::from_string_or_default("error"), LogLevel::Error);
        assert_eq!(
            LogLevel::from_string_or_default("critical"),
            LogLevel::Critical
        );

        // Test invalid log levels default to Info
        assert_eq!(LogLevel::from_string_or_default("invalid"), LogLevel::Info);
        assert_eq!(LogLevel::from_string_or_default(""), LogLevel::Info);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
        assert_eq!(LogLevel::Critical.as_str(), "critical");
    }

    #[test]
    fn test_log_operation_creation() {
        // Test basic creation
        let op = LogOperation::new(LogLevel::Info, "test message".to_string());
        assert_eq!(op.level, LogLevel::Info);
        assert_eq!(op.message, "test message");
        assert!(op.component_id.is_none());

        // Test creation with component ID
        let op = LogOperation::with_component(LogLevel::Debug, "test message", "component-1");
        assert_eq!(op.level, LogLevel::Debug);
        assert_eq!(op.message, "test message");
        assert_eq!(op.component_id.as_deref(), Some("component-1"));

        // Test Clone implementation
        let op2 = op.clone();
        assert_eq!(op2.level, LogLevel::Debug);
        assert_eq!(op2.message, "test message");
        assert_eq!(op2.component_id.as_deref(), Some("component-1"));
    }

    #[test]
    fn test_callback_registry() {
        let mut registry = CallbackRegistry::new();
        assert!(!registry.has_log_handler());

        // Test log handler registration and message handling
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = Arc::clone(&received);

        registry.register_log_handler(move |op| {
            received_clone.lock().unwrap().push((op.level, op.message));
        });

        assert!(registry.has_log_handler());

        // Test handling multiple messages
        registry.handle_log(LogOperation::new(
            LogLevel::Info,
            "info message".to_string(),
        ));
        registry.handle_log(LogOperation::new(
            LogLevel::Error,
            "error message".to_string(),
        ));

        let received = received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0], (LogLevel::Info, "info message".to_string()));
        assert_eq!(received[1], (LogLevel::Error, "error message".to_string()));
    }

    #[test]
    fn test_callback_registry_debug() {
        let registry = CallbackRegistry::new();
        assert_eq!(
            format!("{:?}", registry),
            "CallbackRegistry { has_log_handler: false, registered_modules: [] }"
        );

        let mut registry = CallbackRegistry::new();
        registry.register_log_handler(|_| {});
        assert_eq!(
            format!("{:?}", registry),
            "CallbackRegistry { has_log_handler: true, registered_modules: [] }"
        );
    }

    #[test]
    fn test_logging_integration() {
        let mut registry = CallbackRegistry::new();

        // Create thread-safe storage for captured values
        let received_level = Arc::new(Mutex::new(None));
        let received_message = Arc::new(Mutex::new(None));

        let received_level_clone = Arc::clone(&received_level);
        let received_message_clone = Arc::clone(&received_message);

        registry.register_log_handler(move |op| {
            let mut level = received_level_clone.lock().unwrap();
            let mut message = received_message_clone.lock().unwrap();
            *level = Some(op.level);
            *message = Some(op.message.clone());
        });

        assert!(registry.has_log_handler());

        // Test log message handling
        let log_op = LogOperation::new(LogLevel::Info, "test message".to_string());
        registry.handle_log(log_op);

        // Verify the message was received correctly
        assert_eq!(*received_level.lock().unwrap(), Some(LogLevel::Info));
        assert_eq!(
            received_message.lock().unwrap().as_deref(),
            Some("test message")
        );
    }
}
