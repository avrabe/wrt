// WRT - wrt-component
// Module: Async Context Management Built-ins
// SW-REQ-ID: REQ_ASYNC_CONTEXT_001
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]

//! Async Context Management Built-ins
//!
//! This module provides implementation of the `context.get` and `context.set`
//! built-in functions required by the WebAssembly Component Model for managing
//! async execution contexts.


#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::{boxed::Box, collections::HashMap, vec::Vec};
#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, collections::BTreeMap as HashMap, vec::Vec};

use wrt_error::{Error, ErrorCategory, Result};
use wrt_foundation::{
    // atomic_memory::AtomicRefCell, // Not available in wrt-foundation
    BoundedMap,
    types::ValueType,
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

#[cfg(not(any(feature = "std", )))]
use wrt_foundation::{BoundedString, BoundedVec, safe_memory::NoStdProvider};

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;

// Constants for no_std environments
#[cfg(not(any(feature = "std", )))]
const MAX_CONTEXT_ENTRIES: usize = 32;
#[cfg(not(any(feature = "std", )))]
const MAX_CONTEXT_VALUE_SIZE: usize = 256;
#[cfg(not(any(feature = "std", )))]
const MAX_CONTEXT_KEY_SIZE: usize = 64;

/// Context key identifier for async contexts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg(feature = "std")]
pub struct ContextKey(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg(not(any(feature = "std", )))]
pub struct ContextKey(BoundedString<MAX_CONTEXT_KEY_SIZE>);

impl ContextKey {
    #[cfg(feature = "std")]
    pub fn new(key: String) -> Self {
        Self(key)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn new(key: &str) -> Result<Self> {
        let bounded_key = BoundedString::new_from_str(key)
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        Ok(Self(bounded_key))
    }

    pub fn as_str(&self) -> &str {
        #[cfg(feature = ")]
        return &self.0;
        #[cfg(not(any(feature = "std", )))]
        return self.0.as_str();
    }
}

/// Context value that can be stored in an async context
#[derive(Debug, Clone)]
pub enum ContextValue {
    /// Simple value types
    Simple(ComponentValue),
    /// Binary data (for serialized complex types)
    #[cfg(feature = "std")]
    Binary(Vec<u8>),
    #[cfg(not(any(feature = "std", )))]
    Binary(BoundedVec<u8, MAX_CONTEXT_VALUE_SIZE, NoStdProvider<65536>>),
}

impl ContextValue {
    pub fn from_component_value(value: ComponentValue) -> Self {
        Self::Simple(value)
    }

    #[cfg(feature = "std")]
    pub fn from_binary(data: Vec<u8>) -> Self {
        Self::Binary(data)
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn from_binary(data: &[u8]) -> Result<Self> {
        let bounded_data = BoundedVec::new_from_slice(data)
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        Ok(Self::Binary(bounded_data))
    }

    pub fn as_component_value(&self) -> Option<&ComponentValue> {
        match self {
            Self::Simple(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            #[cfg(feature = ")]
            Self::Binary(data) => Some(data),
            #[cfg(not(any(feature = "std", )))]
            Self::Binary(data) => Some(data.as_slice()),
            _ => None,
        }
    }
}

/// Async execution context that stores key-value pairs
#[derive(Debug, Clone)]
pub struct AsyncContext {
    #[cfg(feature = "std")]
    data: BTreeMap<ContextKey, ContextValue>,
    #[cfg(not(any(feature = "std", )))]
    data: BoundedMap<ContextKey, ContextValue, MAX_CONTEXT_ENTRIES>,
}

impl AsyncContext {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            data: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            data: BoundedMap::new(),
        }
    }

    pub fn get(&self, key: &ContextKey) -> Option<&ContextValue> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: ContextKey, value: ContextValue) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.data.insert(key, value);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.data.insert(key, value)
                .map_err(|_| Error::runtime_execution_error("
                ))?;
            Ok(())
        }
    }

    pub fn remove(&mut self, key: &ContextKey) -> Option<ContextValue> {
        self.data.remove(key)
    }

    pub fn contains_key(&self, key: &ContextKey) -> bool {
        self.data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl Default for AsyncContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-local storage for async contexts in each execution thread
#[cfg(feature = ")]
thread_local! {
    static ASYNC_CONTEXT_STACK: AtomicRefCell<Vec<AsyncContext>> = 
        AtomicRefCell::new(Vec::new());
}

/// Global context storage for no_std environments
#[cfg(not(feature = "std"))]
static GLOBAL_ASYNC_CONTEXT: AtomicRefCell<Option<AsyncContext>> = 
    AtomicRefCell::new(None);

/// Context manager that provides the canonical built-in functions
pub struct AsyncContextManager;

impl AsyncContextManager {
    /// Get the current async context
    /// Implements the `context.get` canonical built-in
    #[cfg(feature = "std")]
    pub fn context_get() -> Result<Option<AsyncContext>> {
        ASYNC_CONTEXT_STACK.with(|stack| {
            let stack_ref = stack.try_borrow()
                .map_err(|_| Error::runtime_execution_error("
                ))?;
            Ok(stack_ref.last().cloned())
        })
    }

    #[cfg(not(feature = "))]
    pub fn context_get() -> Result<Option<AsyncContext>> {
        let context_ref = GLOBAL_ASYNC_CONTEXT.try_borrow()
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        Ok(context_ref.clone())
    }

    /// Set the current async context
    /// Implements the `context.set` canonical built-in
    #[cfg(feature = ")]
    pub fn context_set(context: AsyncContext) -> Result<()> {
        ASYNC_CONTEXT_STACK.with(|stack| {
            let mut stack_ref = stack.try_borrow_mut()
                .map_err(|_| Error::runtime_execution_error("
                ))?;
            stack_ref.push(context);
            Ok(())
        })
    }

    #[cfg(not(feature = "))]
    pub fn context_set(context: AsyncContext) -> Result<()> {
        let mut context_ref = GLOBAL_ASYNC_CONTEXT.try_borrow_mut()
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        *context_ref = Some(context);
        Ok(())
    }

    /// Push a new context onto the stack (for nested async operations)
    #[cfg(feature = ")]
    pub fn context_push(context: AsyncContext) -> Result<()> {
        Self::context_set(context)
    }

    #[cfg(not(feature = "std"))]
    pub fn context_push(context: AsyncContext) -> Result<()> {
        Self::context_set(context)
    }

    /// Pop the current context from the stack
    #[cfg(feature = "std")]
    pub fn context_pop() -> Result<Option<AsyncContext>> {
        ASYNC_CONTEXT_STACK.with(|stack| {
            let mut stack_ref = stack.try_borrow_mut()
                .map_err(|_| Error::runtime_execution_error("
                ))?;
            Ok(stack_ref.pop())
        })
    }

    #[cfg(not(feature = "))]
    pub fn context_pop() -> Result<Option<AsyncContext>> {
        let mut context_ref = GLOBAL_ASYNC_CONTEXT.try_borrow_mut()
            .map_err(|_| Error::runtime_execution_error("
            ))?;
        Ok(context_ref.take())
    }

    /// Get a value from the current context by key
    pub fn get_context_value(key: &ContextKey) -> Result<Option<ContextValue>> {
        let context = Self::context_get()?;
        Ok(context.and_then(|ctx| ctx.get(key).cloned()))
    }

    /// Set a value in the current context by key
    pub fn set_context_value(key: ContextKey, value: ContextValue) -> Result<()> {
        let mut context = Self::context_get()?.unwrap_or_default();
        context.set(key, value)?;
        Self::context_set(context)
    }

    /// Remove a value from the current context by key
    pub fn remove_context_value(key: &ContextKey) -> Result<Option<ContextValue>> {
        if let Some(mut context) = Self::context_get()? {
            let removed = context.remove(key);
            Self::context_set(context)?;
            Ok(removed)
        } else {
            Ok(None)
        }
    }

    /// Clear all values from the current context
    pub fn clear_context() -> Result<()> {
        if let Some(mut context) = Self::context_get()? {
            context.clear();
            Self::context_set(context)?;
        }
        Ok(())
    }
}

/// Built-in function implementations for the canonical ABI
pub mod canonical_builtins {
    use super::*;

    /// `context.get` canonical built-in
    /// Returns the current async context as a component value
    pub fn canon_context_get() -> Result<ComponentValue> {
        let context = AsyncContextManager::context_get()?;
        match context {
            Some(ctx) => {
                // Serialize context to component value
                // For now, return a simple boolean indicating presence
                Ok(ComponentValue::Bool(true))
            }
            None => Ok(ComponentValue::Bool(false))
        }
    }

    /// `context.set` canonical built-in  
    /// Sets the current async context from a component value
    pub fn canon_context_set(value: ComponentValue) -> Result<()> {
        match value {
            ComponentValue::Bool(true) => {
                // Create a new empty context
                let context = AsyncContext::new();
                AsyncContextManager::context_set(context)
            }
            ComponentValue::Bool(false) => {
                // Clear the current context
                AsyncContextManager::context_pop()?;
                Ok(())
            }
            _ => Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "))
        }
    }

    /// Helper function to get a typed value from context
    pub fn get_typed_context_value<T>(key: &str, value_type: ValueType) -> Result<Option<T>>
    where
        T: TryFrom<ComponentValue>,
        T::Error: Into<Error>,
    {
        #[cfg(feature = "std")]
        let context_key = ContextKey::new(key.to_string());
        #[cfg(not(any(feature = "std", )))]
        let context_key = ContextKey::new(key)?;

        if let Some(context_value) = AsyncContextManager::get_context_value(&context_key)? {
            if let Some(component_value) = context_value.as_component_value() {
                let typed_value = T::try_from(component_value.clone())
                    .map_err(|e| e.into())?;
                Ok(Some(typed_value))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Helper function to set a typed value in context
    pub fn set_typed_context_value<T>(key: &str, value: T) -> Result<()>
    where
        T: Into<ComponentValue>,
    {
        #[cfg(feature = "std")]
        let context_key = ContextKey::new(key.to_string());
        #[cfg(not(any(feature = "std", )))]
        let context_key = ContextKey::new(key)?;

        let component_value = value.into();
        let context_value = ContextValue::from_component_value(component_value);
        AsyncContextManager::set_context_value(context_key, context_value)
    }
}

/// Scope guard for automatic context management
pub struct AsyncContextScope {
    _marker: core::marker::PhantomData<()>,
}

impl AsyncContextScope {
    /// Enter a new async context scope
    pub fn enter(context: AsyncContext) -> Result<Self> {
        AsyncContextManager::context_push(context)?;
        Ok(Self {
            _marker: core::marker::PhantomData,
        })
    }

    /// Enter a new empty async context scope
    pub fn enter_empty() -> Result<Self> {
        Self::enter(AsyncContext::new())
    }
}

impl Drop for AsyncContextScope {
    fn drop(&mut self) {
        // Automatically pop context when scope ends
        let _ = AsyncContextManager::context_pop();
    }
}

/// Convenience macro for executing code within an async context scope
#[macro_export]
macro_rules! with_async_context {
    ($context:expr, $body:expr) => {{
        let _scope = $crate::async_context_builtins::AsyncContextScope::enter($context)?;
        $body
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_key_creation() {
        #[cfg(feature = "std")]
        {
            let key = ContextKey::new("test-key".to_string());
            assert_eq!(key.as_str(), "test-key");
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let key = ContextKey::new("test-key").unwrap();
            assert_eq!(key.as_str(), "test-key");
        }
    }

    #[test]
    fn test_context_value_creation() {
        let value = ContextValue::from_component_value(ComponentValue::Bool(true));
        assert!(value.as_component_value().is_some());
        assert_eq!(value.as_component_value().unwrap(), &ComponentValue::Bool(true));
    }

    #[test]
    fn test_async_context_operations() {
        let mut context = AsyncContext::new();
        assert!(context.is_empty());

        #[cfg(feature = "std")]
        let key = ContextKey::new("test".to_string());
        #[cfg(not(any(feature = "std", )))]
        let key = ContextKey::new("test").unwrap();

        let value = ContextValue::from_component_value(ComponentValue::I32(42));
        context.set(key.clone(), value).unwrap();

        assert!(!context.is_empty());
        assert_eq!(context.len(), 1);
        assert!(context.contains_key(&key));

        let retrieved = context.get(&key).unwrap();
        assert_eq!(
            retrieved.as_component_value().unwrap(),
            &ComponentValue::I32(42)
        );
    }

    #[test]
    fn test_context_manager_operations() {
        // Clear any existing context
        let _ = AsyncContextManager::context_pop();

        // Test getting empty context
        let context = AsyncContextManager::context_get().unwrap();
        assert!(context.is_none());

        // Test setting context
        let new_context = AsyncContext::new();
        AsyncContextManager::context_set(new_context).unwrap();

        // Test getting set context
        let retrieved = AsyncContextManager::context_get().unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_canonical_builtins() {
        // Clear any existing context
        let _ = AsyncContextManager::context_pop();

        // Test context.get when no context
        let result = canonical_builtins::canon_context_get().unwrap();
        assert_eq!(result, ComponentValue::Bool(false));

        // Test context.set with true
        canonical_builtins::canon_context_set(ComponentValue::Bool(true)).unwrap();

        // Test context.get when context exists
        let result = canonical_builtins::canon_context_get().unwrap();
        assert_eq!(result, ComponentValue::Bool(true));
    }

    #[test]
    fn test_async_context_scope() {
        // Clear any existing context
        let _ = AsyncContextManager::context_pop();

        {
            let context = AsyncContext::new();
            let _scope = AsyncContextScope::enter(context).unwrap();
            
            // Context should be available in scope
            let retrieved = AsyncContextManager::context_get().unwrap();
            assert!(retrieved.is_some());
        }

        // Context should be popped after scope ends
        let retrieved = AsyncContextManager::context_get().unwrap();
        assert!(retrieved.is_none());
    }
}