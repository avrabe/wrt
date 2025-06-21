// WebAssembly Component Model Built-ins Implementation
//
// This module provides the infrastructure and implementations for WebAssembly
// Component Model built-ins, including resource handling, async operations,
// error contexts, and threading.

#[cfg(feature = "std")]
use std::{
    boxed::Box,
    sync::{Arc, Mutex},
    vec::Vec,
};

#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box,
    sync::Arc,
    vec::Vec,
};

#[cfg(not(feature = "std"))]
use wrt_sync::Mutex;

#[cfg(not(feature = "std"))]
use wrt_foundation::{bounded::{BoundedVec, BoundedString}, safe_memory::NoStdProvider};

use wrt_error::{Error, Result};
use crate::prelude::*;
#[cfg(feature = "std")]
use wrt_foundation::builtin::BuiltinType;
use crate::prelude::WrtComponentValue;

#[cfg(not(feature = "std"))]
use crate::types::Value;

// Define a unified BuiltinType for no_std
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinType {
    // Error built-ins
    ErrorNew,
    ErrorTrace,
    // Resource built-ins
    ResourceCreate,
    ResourceDrop,
    ResourceRep,
    ResourceGet,
    // Threading built-ins
    ThreadingSpawn,
    ThreadingJoin,
    ThreadingSync,
    // Async built-ins
    AsyncNew,
    AsyncGet,
    AsyncPoll,
    AsyncWait,
}
// Commented out until wrt_intercept is properly available
// use wrt_intercept::{BeforeBuiltinResult, BuiltinInterceptor, InterceptContext};

// No_std stubs for interception (simplified)
#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub struct InterceptContext {
    component_name: BoundedString<128, NoStdProvider<65536>>,
    builtin_type: BuiltinType,
    host_id: BoundedString<128, NoStdProvider<65536>>,
}

#[cfg(not(feature = "std"))]
impl InterceptContext {
    pub fn new(component_name: &str, builtin_type: BuiltinType, host_id: &str) -> Self {
        Self {
            component_name: BoundedString::from_str(component_name).unwrap_or_default(),
            builtin_type,
            host_id: BoundedString::from_str(host_id).unwrap_or_default(),
        }
    }
}

#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub enum BeforeBuiltinResult {
    Continue(BoundedVec<WrtComponentValue, 16, NoStdProvider<65536>>),
    Override(BoundedVec<WrtComponentValue, 16, NoStdProvider<65536>>),
    Deny,
}

#[cfg(not(feature = "std"))]
pub trait BuiltinInterceptor {
    fn before_builtin(&self, context: &InterceptContext, args: &[WrtComponentValue]) -> Result<BeforeBuiltinResult>;
}

// Import the real types for std
#[cfg(feature = "std")]
use wrt_intercept::{BeforeBuiltinResult, BuiltinInterceptor, InterceptContext};

use crate::resources::ResourceManager;

/// Resource built-ins implementation
pub mod resource;

/// Async built-ins implementation
#[cfg(feature = "component-model-async")]
pub mod async_ops;

/// Error context built-ins implementation
#[cfg(feature = "component-model-error-context")]
pub mod error;

/// Threading built-ins implementation
#[cfg(feature = "component-model-threading")]
pub mod threading;

/// Safe threading built-ins implementation using platform-aware architecture
#[cfg(feature = "component-model-threading")]
pub mod safe_threading;

/// Trait for built-in function handlers
///
/// This trait defines the interface for handlers that implement built-in
/// functions for the WebAssembly Component Model.
#[cfg(feature = "std")]
pub trait BuiltinHandler: Send + Sync {
    /// Get the type of built-in this handler manages
    fn builtin_type(&self) -> BuiltinType;

    /// Execute the built-in function with the given arguments
    ///
    /// # Arguments
    ///
    /// * `args` - The arguments to the built-in function
    ///
    /// # Returns
    ///
    /// A `Result` containing the function results or an error
    fn execute(&self, args: &[WrtComponentValue]) -> Result<Vec<WrtComponentValue>>;

    /// Clone this handler
    ///
    /// # Returns
    ///
    /// A boxed clone of this handler
    fn clone_handler(&self) -> Box<dyn BuiltinHandler>;
}

#[cfg(not(feature = "std"))]
pub trait BuiltinHandler {
    /// Get the type of built-in this handler manages
    fn builtin_type(&self) -> BuiltinType;

    /// Execute the built-in function with the given arguments (no_std version)
    fn execute(&self, args: &[WrtComponentValue]) -> core::result::Result<BoundedVec<WrtComponentValue, 16, NoStdProvider<65536>>>;

    /// Clone this handler
    fn clone_handler(&self) -> Box<dyn BuiltinHandler>;
}

/// Function executor type for threading built-ins
#[cfg(feature = "component-model-threading")]
pub type FunctionExecutor =
    Arc<dyn Fn(u32, Vec<WrtComponentValue>) -> Result<Vec<WrtComponentValue>> + Send + Sync>;

/// Registry of built-in handlers
///
/// This struct manages the available built-in handlers and routes calls
/// to the appropriate implementation.
pub struct BuiltinRegistry {
    /// Registered handlers for built-in functions
    handlers: Vec<Box<dyn BuiltinHandler>>,
    /// Optional interceptor for built-in calls
    // interceptor: Option<Arc<dyn BuiltinInterceptor>>,
    /// Component name for context
    component_name: String,
    /// Host ID for context
    host_id: String,
    /// Store for async values
    #[cfg(feature = "component-model-async")]
    async_store: Arc<Mutex<async_ops::AsyncValueStore>>,
    /// Store for error contexts
    #[cfg(feature = "component-model-error-context")]
    error_store: Arc<Mutex<error::ErrorContextStore>>,
    /// Function executor for threading handlers
    #[cfg(feature = "component-model-threading")]
    function_executor: FunctionExecutor,
}

impl BuiltinRegistry {
    /// Create a new built-in registry
    ///
    /// # Arguments
    ///
    /// * `component_name` - The name of the component
    /// * `host_id` - The host identifier
    /// * `resource_manager` - The resource manager to use
    ///
    /// # Returns
    ///
    /// A new `BuiltinRegistry` instance with default handlers
    pub fn new(
        component_name: &str,
        host_id: &str,
        resource_manager: Arc<Mutex<ResourceManager>>,
    ) -> Self {
        #[cfg(feature = "component-model-async")]
        let async_store = Arc::new(Mutex::new(async_ops::AsyncValueStore::new()));

        #[cfg(feature = "component-model-error-context")]
        let error_store = Arc::new(Mutex::new(error::ErrorContextStore::new()));

        // Define a default function executor for threading that just returns an error
        #[cfg(feature = "component-model-threading")]
        let function_executor: FunctionExecutor = Arc::new(|function_id, _args| {
            Err(Error::new(
                wrt_error::ErrorCategory::Runtime,
                wrt_error::codes::NOT_IMPLEMENTED,
                "Function not implemented"
            ))
        });

        let mut registry = Self {
            handlers: Vec::new(),
            // interceptor: None,
            component_name: component_name.to_string(),
            host_id: host_id.to_string(),
            #[cfg(feature = "component-model-async")]
            async_store,
            #[cfg(feature = "component-model-error-context")]
            error_store,
            #[cfg(feature = "component-model-threading")]
            function_executor,
        };

        // Register default resource handlers
        let resource_handlers = resource::create_resource_handlers(resource_manager);
        for handler in resource_handlers {
            registry.register_handler(handler);
        }

        // Register async handlers if the feature is enabled
        #[cfg(feature = "component-model-async")]
        {
            let async_handlers = async_ops::create_async_handlers(registry.async_store.clone());
            for handler in async_handlers {
                registry.register_handler(handler);
            }
        }

        // Register error context handlers if the feature is enabled
        #[cfg(feature = "component-model-error-context")]
        {
            let error_handlers = error::create_error_handlers();
            for handler in error_handlers {
                registry.register_handler(handler);
            }
        }

        // Register threading handlers if the feature is enabled
        #[cfg(feature = "component-model-threading")]
        {
            let threading_handlers =
                threading::create_threading_handlers(registry.function_executor.clone());
            for handler in threading_handlers {
                registry.register_handler(handler);
            }
        }

        registry
    }

    /// Register a built-in handler
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    pub fn register_handler(&mut self, handler: Box<dyn BuiltinHandler>) {
        // Check if we already have a handler for this built-in type
        let builtin_type = handler.builtin_type();
        if self.handlers.iter().any(|h| h.builtin_type() == builtin_type) {
            // Replace the existing handler
            let idx = self.handlers.iter().position(|h| h.builtin_type() == builtin_type).unwrap();
            self.handlers[idx] = handler;
        } else {
            // Add a new handler
            self.handlers.push(handler);
        }
    }

    /// Set the interceptor for built-in calls
    ///
    /// # Arguments
    ///
    /// * `interceptor` - The interceptor to use
    // pub fn set_interceptor(&mut self, interceptor: Arc<dyn BuiltinInterceptor>) {
    //     self.interceptor = Some(interceptor);
    // }

    /// Check if a built-in type is supported
    ///
    /// # Arguments
    ///
    /// * `builtin_type` - The built-in type to check
    ///
    /// # Returns
    ///
    /// `true` if the built-in is supported, `false` otherwise
    pub fn supports_builtin(&self, builtin_type: BuiltinType) -> bool {
        self.handlers.iter().any(|h| h.builtin_type() == builtin_type)
    }

    /// Call a built-in function
    ///
    /// # Arguments
    ///
    /// * `builtin_type` - The type of built-in to call
    /// * `args` - The arguments to the function
    ///
    /// # Returns
    ///
    /// A `Result` containing the function results or an error
    pub fn call(
        &self,
        builtin_type: BuiltinType,
        args: &[WrtComponentValue],
    ) -> Result<Vec<WrtComponentValue>> {
        // Find the handler for this built-in
        let handler = self
            .handlers
            .iter()
            .find(|h| h.builtin_type() == builtin_type)
            .ok_or_else(|| Error::new("Component not found"))?;

        // Create interception context
        let context = InterceptContext::new(&self.component_name, builtin_type, &self.host_id);

        // Apply interception if available
        if let Some(interceptor) = &self.interceptor {
            // Before interceptor
            match interceptor.before_builtin(&context, args)? {
                #[cfg(feature = "std")]
                BeforeBuiltinResult::Continue(modified_args) => {
                    // Execute with potentially modified args
                    let result = handler.execute(&modified_args);

                    // After interceptor
                    interceptor.after_builtin(&context, args, result)
                }
                #[cfg(feature = "std")]
                BeforeBuiltinResult::Bypass(result) => {
                    // Skip execution and use provided result
                    Ok(result)
                }
                #[cfg(not(feature = "std"))]
                BeforeBuiltinResult::Continue => {
                    // Execute with original args
                    let result = handler.execute(args);

                    // After interceptor (simplified for no_std)
                    result
                }
                #[cfg(not(feature = "std"))]
                BeforeBuiltinResult::Bypass => {
                    // Skip execution and return empty result
                    Ok(Vec::new())
                }
            }
        } else {
            // No interceptor, just execute
            handler.execute(args)
        }
    }

    /// Get the async store
    #[cfg(feature = "component-model-async")]
    pub fn async_store(&self) -> Arc<Mutex<async_ops::AsyncValueStore>> {
        self.async_store.clone()
    }

    /// Set the function executor for threading built-ins
    ///
    /// # Arguments
    ///
    /// * `executor` - The function executor to use
    #[cfg(feature = "component-model-threading")]
    pub fn set_function_executor(&mut self, executor: FunctionExecutor) {
        self.function_executor = executor;

        // Re-register threading handlers with the new executor
        let threading_handlers =
            threading::create_threading_handlers(self.function_executor.clone());
        for handler in threading_handlers {
            self.register_handler(handler);
        }
    }
}

impl Clone for BuiltinRegistry {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.iter().map(|h| h.clone_handler()).collect(),
            interceptor: self.interceptor.clone(),
            component_name: self.component_name.clone(),
            host_id: self.host_id.clone(),
            #[cfg(feature = "component-model-async")]
            async_store: self.async_store.clone(),
            #[cfg(feature = "component-model-error-context")]
            error_store: self.error_store.clone(),
            #[cfg(feature = "component-model-threading")]
            function_executor: self.function_executor.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use wrt_foundation::component_value::WrtComponentValue;

    use super::*;
    use crate::resources::ResourceManager;

    // Simple test handler implementation
    struct TestHandler {
        builtin_type: BuiltinType,
    }

    impl BuiltinHandler for TestHandler {
        fn builtin_type(&self) -> BuiltinType {
            self.builtin_type
        }

        fn execute(&self, args: &[WrtComponentValue]) -> Result<Vec<WrtComponentValue>> {
            // Simple echo implementation for testing
            Ok(args.to_vec())
        }

        fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
            Box::new(TestHandler { builtin_type: self.builtin_type })
        }
    }

    #[test]
    fn test_registry_supports_builtin() {
        let mut registry = BuiltinRegistry::new(
            "test-component",
            "test-host",
            Arc::new(Mutex::new(ResourceManager::new())),
        );

        // Initially no built-ins are supported
        assert!(!registry.supports_builtin(BuiltinType::ResourceCreate));

        // Register a handler
        registry
            .register_handler(Box::new(TestHandler { builtin_type: BuiltinType::ResourceCreate }));

        // Now it should be supported
        assert!(registry.supports_builtin(BuiltinType::ResourceCreate));
        assert!(!registry.supports_builtin(BuiltinType::ResourceDrop));
    }

    #[test]
    fn test_registry_call() {
        let mut registry = BuiltinRegistry::new(
            "test-component",
            "test-host",
            Arc::new(Mutex::new(ResourceManager::new())),
        );

        // Register handlers
        registry
            .register_handler(Box::new(TestHandler { builtin_type: BuiltinType::ResourceCreate }));

        // Call the built-in
        let args = vec![WrtComponentValue::S32(42)];
        let result = registry.call(BuiltinType::ResourceCreate, &args);

        // Verify result
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), args);

        // Call an unsupported built-in
        let result = registry.call(BuiltinType::ResourceDrop, &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_clone() {
        let mut registry = BuiltinRegistry::new(
            "test-component",
            "test-host",
            Arc::new(Mutex::new(ResourceManager::new())),
        );

        // Register a handler
        registry
            .register_handler(Box::new(TestHandler { builtin_type: BuiltinType::ResourceCreate }));

        // Clone the registry
        let cloned = registry.clone();

        // Check that the clone works correctly
        assert!(cloned.supports_builtin(BuiltinType::ResourceCreate));

        // Call a built-in on the clone
        let args = vec![WrtComponentValue::S32(42)];
        let result = cloned.call(BuiltinType::ResourceCreate, &args);

        // Verify result
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), args);
    }

    #[cfg(feature = "component-model-async")]
    #[test]
    fn test_async_builtin_integration() {
        use wrt_foundation::builtin::BuiltinType;

        let registry = BuiltinRegistry::new(
            "test-component",
            "test-host",
            Arc::new(Mutex::new(ResourceManager::new())),
        );

        // Test the automatic registration of async handlers
        assert!(registry.supports_builtin(BuiltinType::AsyncNew));
        assert!(registry.supports_builtin(BuiltinType::AsyncGet));
        assert!(registry.supports_builtin(BuiltinType::AsyncPoll));

        #[cfg(feature = "std")]
        assert!(registry.supports_builtin(BuiltinType::AsyncWait));

        // Test creating an async value
        let result = registry.call(BuiltinType::AsyncNew, &[]).unwrap();
        assert_eq!(result.len(), 1);

        match &result[0] {
            WrtComponentValue::U32(id) => {
                // Test polling it (should be pending)
                let poll_result =
                    registry.call(BuiltinType::AsyncPoll, &[WrtComponentValue::U32(*id)]).unwrap();
                assert_eq!(poll_result, vec![WrtComponentValue::U32(0)]);

                // Complete the async value
                let store = registry.async_store();
                let mut async_store = store.lock().unwrap();
                async_store.set_result(*id, vec![WrtComponentValue::U32(42)]).unwrap();

                // Test polling again (should be ready)
                let poll_result =
                    registry.call(BuiltinType::AsyncPoll, &[WrtComponentValue::U32(*id)]).unwrap();
                assert_eq!(poll_result, vec![WrtComponentValue::U32(1)]);

                // Test getting the result
                let get_result =
                    registry.call(BuiltinType::AsyncGet, &[WrtComponentValue::U32(*id)]).unwrap();
                assert_eq!(get_result, vec![WrtComponentValue::U32(42)]);
            }
            _ => panic!("Expected U32 result"),
        }
    }
}
