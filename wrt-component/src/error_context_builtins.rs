// WRT - wrt-component
// Module: Error Context Canonical Built-ins
// SW-REQ-ID: REQ_ERROR_CONTEXT_001
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]

//! Error Context Canonical Built-ins
//!
//! This module provides implementation of the `error-context.*` built-in functions
//! required by the WebAssembly Component Model for managing error contexts and
//! debugging information.


extern crate alloc;

use std::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, collections::HashMap, string::String, vec::Vec};

use wrt_error::{Error, ErrorCategory, Result};
use wrt_foundation::{
    atomic_memory::AtomicRefCell,
    bounded::{BoundedMap, BoundedString, BoundedVec},
    component_value::ComponentValue,
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};


use crate::async_::async_types::{ErrorContext, ErrorContextHandle};

// Constants for no_std environments
#[cfg(not(any(feature = "std", )))]
const MAX_ERROR_CONTEXTS: usize = 64;
#[cfg(not(any(feature = "std", )))]
const MAX_DEBUG_MESSAGE_SIZE: usize = 512;
#[cfg(not(any(feature = "std", )))]
const MAX_STACK_FRAMES: usize = 32;
#[cfg(not(any(feature = "std", )))]
const MAX_METADATA_ENTRIES: usize = 16;
#[cfg(not(any(feature = "std", )))]
const MAX_METADATA_KEY_SIZE: usize = 64;

/// Error context identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorContextId(pub u64);

impl ErrorContextId {
    pub fn new() -> Self {
        static COUNTER: core::sync::atomic::AtomicU64 = 
            core::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ErrorContextId {
    fn default() -> Self {
        Self::new()
    }
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning", 
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Error => 2,
            Self::Critical => 3,
        }
    }

    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Info),
            1 => Some(Self::Warning),
            2 => Some(Self::Error),
            3 => Some(Self::Critical),
            _ => None,
        }
    }
}

/// Stack frame information for error contexts
#[derive(Debug, Clone)]
pub struct StackFrame {
    #[cfg(feature = "std")]
    pub function_name: String,
    #[cfg(not(any(feature = "std", )))]
    pub function_name: BoundedString<MAX_DEBUG_MESSAGE_SIZE>,
    
    #[cfg(feature = "std")]
    pub file_name: Option<String>,
    #[cfg(not(any(feature = "std", )))]
    pub file_name: Option<BoundedString<MAX_DEBUG_MESSAGE_SIZE>>,
    
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
}

impl StackFrame {
    #[cfg(feature = "std")]
    pub fn new(function_name: String) -> Self {
        Self {
            function_name,
            file_name: None,
            line_number: None,
            column_number: None,
        }
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn new(function_name: &str) -> Result<Self> {
        let bounded_name = BoundedString::new_from_str(function_name)
            .map_err(|_| Error::memory_allocation_failed("Function name too long for no_std environment")
            ))?;
        Ok(Self {
            function_name: bounded_name,
            file_name: None,
            line_number: None,
            column_number: None,
        })
    }

    #[cfg(feature = "std")]
    pub fn with_location(mut self, file_name: String, line: u32, column: u32) -> Self {
        self.file_name = Some(file_name);
        self.line_number = Some(line);
        self.column_number = Some(column);
        self
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn with_location(mut self, file_name: &str, line: u32, column: u32) -> Result<Self> {
        let bounded_file = BoundedString::new_from_str(file_name)
            .map_err(|_| Error::memory_allocation_failed("File name too long for no_std environment")
            ))?;
        self.file_name = Some(bounded_file);
        self.line_number = Some(line);
        self.column_number = Some(column);
        Ok(self)
    }

    pub fn function_name(&self) -> &str {
        #[cfg(feature = "std")]
        return &self.function_name;
        #[cfg(not(any(feature = "std", )))]
        return self.function_name.as_str();
    }

    pub fn file_name(&self) -> Option<&str> {
        match &self.file_name {
            #[cfg(feature = "std")]
            Some(name) => Some(name),
            #[cfg(not(any(feature = "std", )))]
            Some(name) => Some(name.as_str()),
            None => None,
        }
    }
}

/// Error context implementation with debugging information
#[derive(Debug, Clone)]
pub struct ErrorContextImpl {
    pub id: ErrorContextId,
    pub handle: ErrorContextHandle,
    pub severity: ErrorSeverity,
    
    #[cfg(feature = "std")]
    pub debug_message: String,
    #[cfg(not(any(feature = "std", )))]
    pub debug_message: BoundedString<MAX_DEBUG_MESSAGE_SIZE>,
    
    #[cfg(feature = "std")]
    pub stack_trace: Vec<StackFrame>,
    #[cfg(not(any(feature = "std", )))]
    pub stack_trace: BoundedVec<StackFrame, MAX_STACK_FRAMES, NoStdProvider<65536>>,
    
    #[cfg(feature = "std")]
    pub metadata: HashMap<String, ComponentValue>,
    #[cfg(not(any(feature = "std", )))]
    pub metadata: BoundedMap<BoundedString<MAX_METADATA_KEY_SIZE>, ComponentValue, MAX_METADATA_ENTRIES>,
    
    pub error_code: Option<u32>,
    pub source_error: Option<Box<ErrorContextImpl>>,
}

impl ErrorContextImpl {
    #[cfg(feature = "std")]
    pub fn new(message: String, severity: ErrorSeverity) -> Self {
        Self {
            id: ErrorContextId::new(),
            handle: ErrorContextHandle::new(),
            severity,
            debug_message: message,
            stack_trace: Vec::new(),
            metadata: HashMap::new(),
            error_code: None,
            source_error: None,
        }
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn new(message: &str, severity: ErrorSeverity) -> Result<Self> {
        let bounded_message = BoundedString::new_from_str(message)
            .map_err(|_| Error::memory_allocation_failed("Debug message too long for no_std environment")
            ))?;
        Ok(Self {
            id: ErrorContextId::new(),
            handle: ErrorContextHandle::new(),
            severity,
            debug_message: bounded_message,
            stack_trace: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    Error::memory_allocation_failed("Failed to create stack trace vector")
                })?
            },
            metadata: BoundedMap::new(),
            error_code: None,
            source_error: None,
        })
    }

    pub fn with_error_code(mut self, code: u32) -> Self {
        self.error_code = Some(code);
        self
    }

    pub fn with_source_error(mut self, source: ErrorContextImpl) -> Self {
        self.source_error = Some(Box::new(source));
        self
    }

    #[cfg(feature = "std")]
    pub fn add_stack_frame(&mut self, frame: StackFrame) {
        self.stack_trace.push(frame);
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn add_stack_frame(&mut self, frame: StackFrame) -> Result<()> {
        self.stack_trace.push(frame)
            .map_err(|_| Error::memory_allocation_failed("Stack trace full")
            ))?;
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn set_metadata(&mut self, key: String, value: ComponentValue) {
        self.metadata.insert(key, value);
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn set_metadata(&mut self, key: &str, value: ComponentValue) -> Result<()> {
        let bounded_key = BoundedString::new_from_str(key)
            .map_err(|_| Error::memory_allocation_failed("Metadata key too long for no_std environment")
            ))?;
        self.metadata.insert(bounded_key, value)
            .map_err(|_| Error::memory_allocation_failed("Metadata storage full")
            ))?;
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn get_metadata(&self, key: &str) -> Option<&ComponentValue> {
        self.metadata.get(key)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn get_metadata(&self, key: &str) -> Option<&ComponentValue> {
        if let Ok(bounded_key) = BoundedString::new_from_str(key) {
            self.metadata.get(&bounded_key)
        } else {
            None
        }
    }

    pub fn debug_message(&self) -> &str {
        #[cfg(feature = "std")]
        return &self.debug_message;
        #[cfg(not(any(feature = "std", )))]
        return self.debug_message.as_str();
    }

    pub fn stack_frame_count(&self) -> usize {
        self.stack_trace.len()
    }

    pub fn get_stack_frame(&self, index: usize) -> Option<&StackFrame> {
        self.stack_trace.get(index)
    }

    #[cfg(feature = "std")]
    pub fn format_stack_trace(&self) -> String {
        let mut output = String::new();
        for (i, frame) in self.stack_trace.iter().enumerate() {
            output.push_str(&format!("  #{}: {}", i, frame.function_name()));
            if let Some(file) = frame.file_name() {
                output.push_str(&format!(" at {}:{}", file, frame.line_number.unwrap_or(0)));
            }
            output.push('\n');
        }
        output
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn format_stack_trace(&self) -> core::result::Result<BoundedString<1024, NoStdProvider<65536>>> {
        let mut output = BoundedString::new();
        for (i, frame) in self.stack_trace.iter().enumerate() {
            // Binary std/no_std choice
            output.push_str("  #").map_err(|_| Error::memory_allocation_failed("Stack trace format buffer full")
            ))?;
            output.push_str(": ").map_err(|_| Error::memory_allocation_failed("Stack trace format buffer full")
            ))?;
            output.push_str(frame.function_name()).map_err(|_| Error::memory_allocation_failed("Stack trace format buffer full")
            ))?;
            output.push('\n').map_err(|_| Error::memory_allocation_failed("Stack trace format buffer full")
            ))?;
        }
        Ok(output)
    }
}

/// Global registry for error contexts
static ERROR_CONTEXT_REGISTRY: AtomicRefCell<Option<ErrorContextRegistry>> = 
    AtomicRefCell::new(None);

/// Registry that manages all error contexts
#[derive(Debug)]
pub struct ErrorContextRegistry {
    #[cfg(feature = "std")]
    contexts: HashMap<ErrorContextId, ErrorContextImpl>,
    #[cfg(not(any(feature = "std", )))]
    contexts: BoundedMap<ErrorContextId, ErrorContextImpl, MAX_ERROR_CONTEXTS>,
}

impl ErrorContextRegistry {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            contexts: HashMap::new(),
            #[cfg(not(any(feature = "std", )))]
            contexts: BoundedMap::new(),
        }
    }

    pub fn register_context(&mut self, context: ErrorContextImpl) -> Result<ErrorContextId> {
        let id = context.id;
        #[cfg(feature = "std")]
        {
            self.contexts.insert(id, context);
            Ok(id)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.contexts.insert(id, context)
                .map_err(|_| Error::memory_allocation_failed("Error context registry full")
                ))?;
            Ok(id)
        }
    }

    pub fn get_context(&self, id: ErrorContextId) -> Option<&ErrorContextImpl> {
        self.contexts.get(&id)
    }

    pub fn get_context_mut(&mut self, id: ErrorContextId) -> Option<&mut ErrorContextImpl> {
        self.contexts.get_mut(&id)
    }

    pub fn remove_context(&mut self, id: ErrorContextId) -> Option<ErrorContextImpl> {
        self.contexts.remove(&id)
    }

    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }
}

impl Default for ErrorContextRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Error context built-ins providing canonical functions
pub struct ErrorContextBuiltins;

impl ErrorContextBuiltins {
    /// Initialize the global error context registry
    pub fn initialize() -> Result<()> {
        let mut registry_ref = ERROR_CONTEXT_REGISTRY.try_borrow_mut()
            .map_err(|_| Error::runtime_invalid_state("Error context registry borrow failed")
            ))?;
        *registry_ref = Some(ErrorContextRegistry::new());
        Ok(())
    }

    /// Get the global registry
    fn with_registry<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&ErrorContextRegistry) -> R,
    {
        let registry_ref = ERROR_CONTEXT_REGISTRY.try_borrow()
            .map_err(|_| Error::runtime_invalid_state("Error context registry borrow failed")
            ))?;
        let registry = registry_ref.as_ref()
            .ok_or_else(|| Error::runtime_invalid_state("Error context registry not initialized")
            ))?;
        Ok(f(registry))
    }

    /// Get the global registry mutably
    fn with_registry_mut<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&mut ErrorContextRegistry) -> Result<R>,
    {
        let mut registry_ref = ERROR_CONTEXT_REGISTRY.try_borrow_mut()
            .map_err(|_| Error::runtime_invalid_state("Error context registry borrow failed")
            ))?;
        let registry = registry_ref.as_mut()
            .ok_or_else(|| Error::runtime_invalid_state("Error context registry not initialized")
            ))?;
        f(registry)
    }

    /// `error-context.new` canonical built-in
    /// Creates a new error context
    #[cfg(feature = "std")]
    pub fn error_context_new(message: String, severity: ErrorSeverity) -> Result<ErrorContextId> {
        let context = ErrorContextImpl::new(message, severity);
        Self::with_registry_mut(|registry| {
            registry.register_context(context)
        })?
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn error_context_new(message: &str, severity: ErrorSeverity) -> Result<ErrorContextId> {
        let context = ErrorContextImpl::new(message, severity)?;
        Self::with_registry_mut(|registry| {
            registry.register_context(context)
        })?
    }

    /// `error-context.debug-message` canonical built-in
    /// Gets the debug message from an error context
    #[cfg(feature = "std")]
    pub fn error_context_debug_message(context_id: ErrorContextId) -> Result<String> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.debug_message.clone()
            } else {
                String::new()
            }
        })
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn error_context_debug_message(context_id: ErrorContextId) -> Result<BoundedString<MAX_DEBUG_MESSAGE_SIZE>> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.debug_message.clone()
            } else {
                BoundedString::new()
            }
        })
    }

    /// `error-context.drop` canonical built-in
    /// Drops an error context
    pub fn error_context_drop(context_id: ErrorContextId) -> Result<()> {
        Self::with_registry_mut(|registry| {
            registry.remove_context(context_id);
            Ok(())
        })?
    }

    /// Get error context severity
    pub fn error_context_severity(context_id: ErrorContextId) -> Result<ErrorSeverity> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.severity
            } else {
                ErrorSeverity::Error
            }
        })
    }

    /// Get error context error code if set
    pub fn error_context_error_code(context_id: ErrorContextId) -> Result<Option<u32>> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.error_code
            } else {
                None
            }
        })
    }

    /// Get stack trace from error context
    #[cfg(feature = "std")]
    pub fn error_context_stack_trace(context_id: ErrorContextId) -> Result<String> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.format_stack_trace()
            } else {
                String::new()
            }
        })
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn error_context_stack_trace(context_id: ErrorContextId) -> core::result::Result<BoundedString<1024, NoStdProvider<65536>>> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.format_stack_trace()
            } else {
                Ok(BoundedString::new())
            }
        })?
    }

    /// Add a stack frame to an error context
    #[cfg(feature = "std")]
    pub fn error_context_add_stack_frame(
        context_id: ErrorContextId, 
        function_name: String,
        file_name: Option<String>,
        line: Option<u32>,
        column: Option<u32>
    ) -> Result<()> {
        Self::with_registry_mut(|registry| {
            if let Some(context) = registry.get_context_mut(context_id) {
                let mut frame = StackFrame::new(function_name);
                if let (Some(file), Some(line_num)) = (file_name, line) {
                    frame = frame.with_location(file, line_num, column.unwrap_or(0));
                }
                context.add_stack_frame(frame);
                Ok(())
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    #[cfg(not(any(feature = ")))]
    pub fn error_context_add_stack_frame(
        context_id: ErrorContextId, 
        function_name: &str,
        file_name: Option<&str>,
        line: Option<u32>,
        column: Option<u32>
    ) -> Result<()> {
        Self::with_registry_mut(|registry| {
            if let Some(context) = registry.get_context_mut(context_id) {
                let mut frame = StackFrame::new(function_name)?;
                if let (Some(file), Some(line_num)) = (file_name, line) {
                    frame = frame.with_location(file, line_num, column.unwrap_or(0))?;
                }
                context.add_stack_frame(frame)?;
                Ok(())
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    /// Set metadata on an error context
    #[cfg(feature = ")]
    pub fn error_context_set_metadata(
        context_id: ErrorContextId,
        key: String,
        value: ComponentValue
    ) -> Result<()> {
        Self::with_registry_mut(|registry| {
            if let Some(context) = registry.get_context_mut(context_id) {
                context.set_metadata(key, value);
                Ok(())
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    #[cfg(not(any(feature = ")))]
    pub fn error_context_set_metadata(
        context_id: ErrorContextId,
        key: &str,
        value: ComponentValue
    ) -> Result<()> {
        Self::with_registry_mut(|registry| {
            if let Some(context) = registry.get_context_mut(context_id) {
                context.set_metadata(key, value)?;
                Ok(())
            } else {
                Err(Error::runtime_execution_error("
                ))
            }
        })?
    }

    /// Get metadata from an error context
    pub fn error_context_get_metadata(
        context_id: ErrorContextId,
        key: &str
    ) -> Result<Option<ComponentValue>> {
        Self::with_registry(|registry| {
            if let Some(context) = registry.get_context(context_id) {
                context.get_metadata(key).cloned()
            } else {
                None
            }
        })
    }
}

/// Convenience functions for working with error contexts
pub mod error_context_helpers {
    use super::*;

    /// Create an error context from a standard error
    #[cfg(feature = ")]
    pub fn from_error(error: &Error) -> Result<ErrorContextId> {
        let message = error.message().to_string();
        let severity = match error.category() {
            ErrorCategory::InvalidInput | ErrorCategory::Type => ErrorSeverity::Warning,
            ErrorCategory::Runtime | ErrorCategory::Memory => ErrorSeverity::Error,
            _ => ErrorSeverity::Critical,
        };
        
        let context_id = ErrorContextBuiltins::error_context_new(message, severity)?;
        ErrorContextBuiltins::error_context_set_metadata(
            context_id,
            "error_code".to_string(),
            ComponentValue::I32(error.code() as i32)
        )?;
        Ok(context_id)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn from_error(error: &Error) -> Result<ErrorContextId> {
        let severity = match error.category() {
            ErrorCategory::InvalidInput | ErrorCategory::Type => ErrorSeverity::Warning,
            ErrorCategory::Runtime | ErrorCategory::Memory => ErrorSeverity::Error,
            _ => ErrorSeverity::Critical,
        };
        
        let context_id = ErrorContextBuiltins::error_context_new(error.message(), severity)?;
        ErrorContextBuiltins::error_context_set_metadata(
            context_id,
            "error_code",
            ComponentValue::I32(error.code() as i32)
        )?;
        Ok(context_id)
    }

    /// Create a simple error context with just a message
    #[cfg(feature = "std")]
    pub fn create_simple(message: String) -> Result<ErrorContextId> {
        ErrorContextBuiltins::error_context_new(message, ErrorSeverity::Error)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn create_simple(message: &str) -> Result<ErrorContextId> {
        ErrorContextBuiltins::error_context_new(message, ErrorSeverity::Error)
    }

    /// Create an error context with stack trace
    #[cfg(feature = "std")]
    pub fn create_with_stack_trace(
        message: String, 
        function_name: String,
        file_name: Option<String>,
        line: Option<u32>
    ) -> Result<ErrorContextId> {
        let context_id = ErrorContextBuiltins::error_context_new(message, ErrorSeverity::Error)?;
        ErrorContextBuiltins::error_context_add_stack_frame(
            context_id, 
            function_name, 
            file_name, 
            line, 
            None
        )?;
        Ok(context_id)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn create_with_stack_trace(
        message: &str, 
        function_name: &str,
        file_name: Option<&str>,
        line: Option<u32>
    ) -> Result<ErrorContextId> {
        let context_id = ErrorContextBuiltins::error_context_new(message, ErrorSeverity::Error)?;
        ErrorContextBuiltins::error_context_add_stack_frame(
            context_id, 
            function_name, 
            file_name, 
            line, 
            None
        )?;
        Ok(context_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_id_generation() {
        let id1 = ErrorContextId::new();
        let id2 = ErrorContextId::new();
        assert_ne!(id1, id2);
        assert!(id1.as_u64() > 0);
        assert!(id2.as_u64() > 0);
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(ErrorSeverity::Info.as_str(), "info");
        assert_eq!(ErrorSeverity::Warning.as_str(), "warning");
        assert_eq!(ErrorSeverity::Error.as_str(), "error");
        assert_eq!(ErrorSeverity::Critical.as_str(), "critical");

        assert_eq!(ErrorSeverity::Info.as_u32(), 0);
        assert_eq!(ErrorSeverity::Warning.as_u32(), 1);
        assert_eq!(ErrorSeverity::Error.as_u32(), 2);
        assert_eq!(ErrorSeverity::Critical.as_u32(), 3);

        assert_eq!(ErrorSeverity::from_u32(0), Some(ErrorSeverity::Info));
        assert_eq!(ErrorSeverity::from_u32(3), Some(ErrorSeverity::Critical));
        assert_eq!(ErrorSeverity::from_u32(999), None);
    }

    #[test]
    fn test_stack_frame_creation() {
        #[cfg(feature = "std")]
        {
            let frame = StackFrame::new("test_function".to_string())
                .with_location("test.rs".to_string(), 42, 10);
            assert_eq!(frame.function_name(), "test_function");
            assert_eq!(frame.file_name(), Some("test.rs"));
            assert_eq!(frame.line_number, Some(42));
            assert_eq!(frame.column_number, Some(10));
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let frame = StackFrame::new("test_function").unwrap()
                .with_location("test.rs", 42, 10).unwrap();
            assert_eq!(frame.function_name(), "test_function");
            assert_eq!(frame.file_name(), Some("test.rs"));
            assert_eq!(frame.line_number, Some(42));
            assert_eq!(frame.column_number, Some(10));
        }
    }

    #[test]
    fn test_error_context_creation() {
        #[cfg(feature = "std")]
        {
            let context = ErrorContextImpl::new("Test error".to_string(), ErrorSeverity::Error);
            assert_eq!(context.debug_message(), "Test error");
            assert_eq!(context.severity, ErrorSeverity::Error);
            assert_eq!(context.stack_frame_count(), 0);
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let context = ErrorContextImpl::new("Test error", ErrorSeverity::Error).unwrap();
            assert_eq!(context.debug_message(), "Test error");
            assert_eq!(context.severity, ErrorSeverity::Error);
            assert_eq!(context.stack_frame_count(), 0);
        }
    }

    #[test]
    fn test_error_context_with_metadata() {
        #[cfg(feature = "std")]
        {
            let mut context = ErrorContextImpl::new("Test error".to_string(), ErrorSeverity::Error);
            context.set_metadata("key1".to_string(), ComponentValue::I32(42));
            context.set_metadata("key2".to_string(), ComponentValue::Bool(true));

            assert_eq!(context.get_metadata("key1"), Some(&ComponentValue::I32(42)));
            assert_eq!(context.get_metadata("key2"), Some(&ComponentValue::Bool(true)));
            assert_eq!(context.get_metadata("missing"), None);
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let mut context = ErrorContextImpl::new("Test error", ErrorSeverity::Error).unwrap();
            context.set_metadata("key1", ComponentValue::I32(42)).unwrap();
            context.set_metadata("key2", ComponentValue::Bool(true)).unwrap();

            assert_eq!(context.get_metadata("key1"), Some(&ComponentValue::I32(42)));
            assert_eq!(context.get_metadata("key2"), Some(&ComponentValue::Bool(true)));
            assert_eq!(context.get_metadata("missing"), None);
        }
    }

    #[test]
    fn test_error_context_stack_trace() {
        #[cfg(feature = "std")]
        {
            let mut context = ErrorContextImpl::new("Test error".to_string(), ErrorSeverity::Error);
            let frame1 = StackFrame::new("function1".to_string())
                .with_location("file1.rs".to_string(), 10, 5);
            let frame2 = StackFrame::new("function2".to_string())
                .with_location("file2.rs".to_string(), 20, 15);

            context.add_stack_frame(frame1);
            context.add_stack_frame(frame2);

            assert_eq!(context.stack_frame_count(), 2);
            let trace = context.format_stack_trace();
            assert!(trace.contains("function1"));
            assert!(trace.contains("function2"));
            assert!(trace.contains("file1.rs"));
            assert!(trace.contains("file2.rs"));
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let mut context = ErrorContextImpl::new("Test error", ErrorSeverity::Error).unwrap();
            let frame1 = StackFrame::new("function1").unwrap()
                .with_location("file1.rs", 10, 5).unwrap();
            let frame2 = StackFrame::new("function2").unwrap()
                .with_location("file2.rs", 20, 15).unwrap();

            context.add_stack_frame(frame1).unwrap();
            context.add_stack_frame(frame2).unwrap();

            assert_eq!(context.stack_frame_count(), 2);
            let trace = context.format_stack_trace().unwrap();
            assert!(trace.as_str().contains("function1"));
            assert!(trace.as_str().contains("function2"));
        }
    }

    #[test]
    fn test_error_context_registry() {
        let mut registry = ErrorContextRegistry::new();
        assert_eq!(registry.context_count(), 0);

        #[cfg(feature = "std")]
        let context = ErrorContextImpl::new("Test error".to_string(), ErrorSeverity::Error);
        #[cfg(not(any(feature = "std", )))]
        let context = ErrorContextImpl::new("Test error", ErrorSeverity::Error).unwrap();

        let context_id = context.id;
        registry.register_context(context).unwrap();
        assert_eq!(registry.context_count(), 1);

        let retrieved_context = registry.get_context(context_id);
        assert!(retrieved_context.is_some());
        assert_eq!(retrieved_context.unwrap().debug_message(), "Test error");

        let removed_context = registry.remove_context(context_id);
        assert!(removed_context.is_some());
        assert_eq!(registry.context_count(), 0);
    }

    #[test]
    fn test_error_context_builtins() {
        // Initialize the registry
        ErrorContextBuiltins::initialize().unwrap();

        // Create a new error context
        #[cfg(feature = "std")]
        let context_id = ErrorContextBuiltins::error_context_new(
            "Test error message".to_string(), 
            ErrorSeverity::Error
        ).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let context_id = ErrorContextBuiltins::error_context_new(
            "Test error message", 
            ErrorSeverity::Error
        ).unwrap();

        // Test getting debug message
        let debug_msg = ErrorContextBuiltins::error_context_debug_message(context_id).unwrap();
        #[cfg(feature = "std")]
        assert_eq!(debug_msg, "Test error message");
        #[cfg(not(any(feature = "std", )))]
        assert_eq!(debug_msg.as_str(), "Test error message");

        // Test getting severity
        let severity = ErrorContextBuiltins::error_context_severity(context_id).unwrap();
        assert_eq!(severity, ErrorSeverity::Error);

        // Test setting metadata
        #[cfg(feature = "std")]
        ErrorContextBuiltins::error_context_set_metadata(
            context_id,
            "test_key".to_string(),
            ComponentValue::I32(123)
        ).unwrap();
        #[cfg(not(any(feature = "std", )))]
        ErrorContextBuiltins::error_context_set_metadata(
            context_id,
            "test_key",
            ComponentValue::I32(123)
        ).unwrap();

        // Test getting metadata
        let metadata = ErrorContextBuiltins::error_context_get_metadata(context_id, "test_key").unwrap();
        assert_eq!(metadata, Some(ComponentValue::I32(123)));

        // Test adding stack frame
        #[cfg(feature = "std")]
        ErrorContextBuiltins::error_context_add_stack_frame(
            context_id,
            "test_function".to_string(),
            Some("test.rs".to_string()),
            Some(42),
            Some(10)
        ).unwrap();
        #[cfg(not(any(feature = "std", )))]
        ErrorContextBuiltins::error_context_add_stack_frame(
            context_id,
            "test_function",
            Some("test.rs"),
            Some(42),
            Some(10)
        ).unwrap();

        // Test getting stack trace
        let stack_trace = ErrorContextBuiltins::error_context_stack_trace(context_id).unwrap();
        #[cfg(feature = "std")]
        assert!(stack_trace.contains("test_function"));
        #[cfg(not(any(feature = "std", )))]
        assert!(stack_trace.as_str().contains("test_function"));

        // Test dropping context
        ErrorContextBuiltins::error_context_drop(context_id).unwrap();
    }

    #[test]
    fn test_error_context_helpers() {
        ErrorContextBuiltins::initialize().unwrap();

        // Test creating simple error context
        #[cfg(feature = "std")]
        let simple_id = error_context_helpers::create_simple("Simple error".to_string()).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let simple_id = error_context_helpers::create_simple("Simple error").unwrap();

        let severity = ErrorContextBuiltins::error_context_severity(simple_id).unwrap();
        assert_eq!(severity, ErrorSeverity::Error);

        // Test creating error context with stack trace
        #[cfg(feature = "std")]
        let trace_id = error_context_helpers::create_with_stack_trace(
            "Error with trace".to_string(),
            "main".to_string(),
            Some("main.rs".to_string()),
            Some(10)
        ).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let trace_id = error_context_helpers::create_with_stack_trace(
            "Error with trace",
            "main",
            Some("main.rs"),
            Some(10)
        ).unwrap();

        let stack_trace = ErrorContextBuiltins::error_context_stack_trace(trace_id).unwrap();
        #[cfg(feature = "std")]
        assert!(stack_trace.contains("main"));
        #[cfg(not(any(feature = "std", )))]
        assert!(stack_trace.as_str().contains("main"));
    }
}