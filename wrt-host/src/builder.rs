//! Builder pattern for constructing and configuring WebAssembly hosts.
//!
//! This module provides a builder pattern for creating and configuring instances
//! of the `CallbackRegistry` with the appropriate built-in functions, interceptors,
//! and other configuration options.

// Use the prelude for consistent imports
use crate::prelude::*;

/// Builder for configuring and creating instances of `CallbackRegistry` with built-in support.
///
/// This builder pattern allows for fluent configuration of a WebAssembly host environment,
/// including built-in functions, interceptors, and validation of required capabilities.
#[derive(Default)]
pub struct HostBuilder {
    /// The callback registry being built
    registry: CallbackRegistry,

    /// Built-in types that are required by the component
    required_builtins: HashSet<BuiltinType>,

    /// Built-in interceptor
    builtin_interceptor: Option<Arc<dyn BuiltinInterceptor>>,

    /// Link interceptor
    link_interceptor: Option<Arc<LinkInterceptor>>,

    /// Whether strict validation is enabled
    strict_validation: bool,

    /// Component name for the built-in host
    component_name: String,

    /// Host ID for the built-in host
    host_id: String,

    /// Fallback handlers for critical built-ins
    fallback_handlers: Vec<(BuiltinType, HostFunctionHandler)>,
}

impl HostBuilder {
    /// Create a new host builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: CallbackRegistry::new(),
            required_builtins: HashSet::new(),
            builtin_interceptor: None,
            link_interceptor: None,
            strict_validation: false,
            #[cfg(feature = "std")]
            component_name: String::from("default"),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            component_name: "default".into(),
            #[cfg(feature = "std")]
            host_id: String::from("default"),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            host_id: "default".into(),
            fallback_handlers: Vec::new(),
        }
    }

    /// Require a built-in type.
    ///
    /// This method marks a built-in type as required for the component.
    /// During validation, the builder will ensure that all required built-ins
    /// are properly implemented.
    pub fn require_builtin(mut self, builtin_type: BuiltinType) -> Self {
        self.required_builtins.insert(builtin_type);
        self
    }

    /// Register a host function.
    ///
    /// This method registers a host function with the specified module and function name.
    pub fn with_host_function(
        mut self,
        module_name: &str,
        function_name: &str,
        handler: HostFunctionHandler,
    ) -> Self {
        self.registry.register_host_function(module_name, function_name, handler);
        self
    }

    /// Register a callback.
    ///
    /// This method registers a callback of the specified type.
    pub fn with_callback<T: 'static + Send + Sync>(
        mut self,
        callback_type: CallbackType,
        callback: T,
    ) -> Self {
        self.registry.register_callback(callback_type, callback);
        self
    }

    /// Set the built-in interceptor.
    ///
    /// This method sets an interceptor for built-in functions.
    pub fn with_builtin_interceptor(mut self, interceptor: Arc<dyn BuiltinInterceptor>) -> Self {
        self.builtin_interceptor = Some(interceptor);
        self
    }

    /// Set the link interceptor.
    ///
    /// This method sets an interceptor for link-time function resolution.
    pub fn with_link_interceptor(mut self, interceptor: Arc<LinkInterceptor>) -> Self {
        self.link_interceptor = Some(interceptor.clone());
        self.registry = self.registry.with_interceptor(interceptor);
        self
    }

    /// Enable or disable strict validation.
    ///
    /// When strict validation is enabled, the builder will validate that all
    /// required built-in functions are properly implemented before building
    /// the callback registry.
    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.strict_validation = strict;
        self
    }

    /// Register a built-in handler.
    ///
    /// This method registers a handler for a specific built-in function.
    pub fn with_builtin_handler<F>(self, builtin_type: BuiltinType, handler: F) -> Self
    where
        F: Fn(&mut dyn Any, Vec<Value>) -> Result<Vec<Value>> + Send + Sync + Clone + 'static,
    {
        let handler_fn = HostFunctionHandler::new(move |target| {
            let args = Vec::new(); // Default empty args, will be replaced in actual call
            handler(target, args)
        });

        // Register the handler with the special "wasi_builtin" module name
        // and the built-in type name as the function name
        self.with_host_function("wasi_builtin", builtin_type.name(), handler_fn)
    }

    /// Manually specify that a built-in is implemented.
    ///
    /// This method is used to mark a built-in as implemented even if it's
    /// not directly registered through this builder. This is useful when
    /// built-ins are registered through other mechanisms.
    pub fn builtin_implemented(mut self, builtin_type: BuiltinType) -> Self {
        // Remove from required if it's there
        self.required_builtins.remove(&builtin_type);
        self
    }

    /// Check if a built-in type is required.
    #[must_use]
    pub fn is_builtin_required(&self, builtin_type: BuiltinType) -> bool {
        self.required_builtins.contains(&builtin_type)
    }

    /// Check if a built-in type is implemented.
    #[must_use]
    pub fn is_builtin_implemented(&self, builtin_type: BuiltinType) -> bool {
        // For now we just check if the registry has a handler for the built-in
        self.registry.has_host_function("wasi_builtin", builtin_type.name())
    }

    /// Validate the configuration.
    ///
    /// This method checks that all required built-ins are implemented.
    ///
    /// # Errors
    ///
    /// Returns an error if strict validation is enabled and any required built-in
    /// is not implemented.
    pub fn validate(&self) -> Result<()> {
        if self.strict_validation {
            for &builtin_type in &self.required_builtins {
                if !self.is_builtin_implemented(builtin_type) {
                    #[cfg(feature = "std")]
                    return Err(Error::runtime_error(format!(
                        "Required built-in {} is not implemented",
                        builtin_type.name()
                    )));

                    #[cfg(all(feature = "alloc", not(feature = "std")))]
                    return Err(Error::runtime_error(alloc::format!(
                        "Required built-in {} is not implemented",
                        builtin_type.name()
                    )));

                    #[cfg(not(any(feature = "std", feature = "alloc")))]
                    return Err(Error::runtime_error("Required built-in is not implemented"));
                }
            }
        }

        Ok(())
    }

    /// Build the callback registry.
    ///
    /// This method creates a `CallbackRegistry` instance based on the current configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn build(self) -> Result<CallbackRegistry> {
        self.validate()?;
        Ok(self.registry)
    }

    /// Set the component name
    ///
    /// This is used for context in built-in interception
    pub fn with_component_name(mut self, name: &str) -> Self {
        #[cfg(feature = "std")]
        {
            self.component_name = String::from(name);
        }
        #[cfg(all(feature = "alloc", not(feature = "std")))]
        {
            self.component_name = name.into();
        }
        self
    }

    /// Set the host ID
    ///
    /// This is used for context in built-in interception
    pub fn with_host_id(mut self, id: &str) -> Self {
        #[cfg(feature = "std")]
        {
            self.host_id = String::from(id);
        }
        #[cfg(all(feature = "alloc", not(feature = "std")))]
        {
            self.host_id = id.into();
        }
        self
    }

    /// Register a fallback handler for a critical built-in
    ///
    /// Fallbacks are used when a built-in is required but not explicitly
    /// implemented through a regular handler.
    pub fn with_fallback_handler<F>(mut self, builtin_type: BuiltinType, handler: F) -> Self
    where
        F: Fn(&mut dyn Any, Vec<Value>) -> Result<Vec<Value>> + Send + Sync + Clone + 'static,
    {
        let handler_fn = HostFunctionHandler::new(move |target| {
            let args = Vec::new(); // Default empty args, will be replaced in actual call
            handler(target, args)
        });

        self.fallback_handlers.push((builtin_type, handler_fn));
        self
    }

    /// Build a BuiltinHost instance from this builder
    ///
    /// This creates a BuiltinHost with all the configured handlers, fallbacks,
    /// and interceptors.
    ///
    /// # Returns
    ///
    /// A `BuiltinHost` instance ready for use
    pub fn build_builtin_host(&self) -> BuiltinHost {
        let mut host = BuiltinHost::new(&self.component_name, &self.host_id);

        // Set interceptor if available
        if let Some(interceptor) = &self.builtin_interceptor {
            host.set_interceptor(interceptor.clone());
        }

        // Register all built-in handlers from the registry
        // Since BuiltinType doesn't have an all() method, we'll check for each known type
        let builtin_types = [
            BuiltinType::ResourceCreate,
            BuiltinType::ResourceDrop,
            BuiltinType::ResourceRep,
            BuiltinType::ResourceGet,
            // Add other built-in types as needed
        ];

        for builtin_type in &builtin_types {
            let builtin_name = builtin_type.name();
            if self.registry.has_host_function("wasi_builtin", builtin_name) {
                // We need a way to extract the handler function from the registry
                // For now, we'll create a new function that calls through the registry
                let registry_clone = self.registry.clone();
                host.register_handler(*builtin_type, move |engine, args| {
                    registry_clone.call_host_function(engine, "wasi_builtin", builtin_name, args)
                });
            }
        }

        // Register fallbacks
        for (builtin_type, handler) in &self.fallback_handlers {
            let handler_clone = handler.clone();
            host.register_fallback(*builtin_type, move |engine, args| {
                handler_clone.call(engine, args)
            });
        }

        host
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_types::values::Value;

    #[test]
    fn test_builder_basics() {
        let builder = HostBuilder::new();
        let registry = builder.build().expect("Failed to build registry");

        assert!(!registry.has_host_function("test_module", "test_function"));
    }

    #[test]
    fn test_host_function_registration() {
        let handler = HostFunctionHandler::new(|_| Ok(vec![Value::I32(42)]));

        let builder =
            HostBuilder::new().with_host_function("test_module", "test_function", handler);

        let registry = builder.build().expect("Failed to build registry");

        assert!(registry.has_host_function("test_module", "test_function"));
    }

    #[test]
    fn test_builtin_registration() {
        let builder = HostBuilder::new()
            .with_builtin_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I64(123)]));

        let registry = builder.build().expect("Failed to build registry");

        assert!(registry.has_host_function("wasi_builtin", "resource.create"));
    }

    #[test]
    fn test_required_builtin() {
        let builder = HostBuilder::new()
            .require_builtin(BuiltinType::ResourceCreate)
            .with_strict_validation(true);

        // Should fail because ResourceCreate is required but not implemented
        let result = builder.build();
        assert!(result.is_err());

        // Now implement the required built-in
        let builder = HostBuilder::new()
            .require_builtin(BuiltinType::ResourceCreate)
            .with_strict_validation(true)
            .with_builtin_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I64(123)]));

        // Should succeed now
        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_manually_implemented_builtin() {
        let builder = HostBuilder::new()
            .require_builtin(BuiltinType::ResourceCreate)
            .with_strict_validation(true)
            .builtin_implemented(BuiltinType::ResourceCreate);

        // Should succeed because we manually marked ResourceCreate as implemented
        let result = builder.build();
        assert!(result.is_ok());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_link_interceptor() {
        // Creating a simple mock interceptor for testing
        use std::sync::Arc;
        use wrt_intercept::{LinkInterceptor, LinkInterceptorStrategy};
        use wrt_types::values::Value;

        #[derive(Clone)]
        struct MockStrategy;

        impl LinkInterceptorStrategy for MockStrategy {
            fn before_call(
                &self,
                _source: &str,
                _target: &str,
                _function: &str,
                args: &[Value],
            ) -> Result<Vec<Value>> {
                Ok(args.to_vec())
            }

            fn after_call(
                &self,
                _source: &str,
                _target: &str,
                _function: &str,
                _args: &[Value],
                result: Result<Vec<Value>>,
            ) -> Result<Vec<Value>> {
                result
            }

            fn clone_strategy(&self) -> Arc<dyn LinkInterceptorStrategy> {
                Arc::new(self.clone())
            }
        }

        // Create a LinkInterceptor with our mock strategy
        let mut interceptor = LinkInterceptor::new("test-interceptor");
        interceptor.add_strategy(Arc::new(MockStrategy));
        let interceptor = Arc::new(interceptor);

        let builder = HostBuilder::new().with_link_interceptor(interceptor);

        let registry = builder.build().expect("Failed to build registry");

        assert!(registry.get_interceptor().is_some());
    }

    #[test]
    fn test_builtin_host_creation() {
        let builder = HostBuilder::new()
            .with_component_name("test-component")
            .with_host_id("test-host")
            .with_builtin_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I32(42)]));

        let builtin_host = builder.build_builtin_host();

        // Check that the handler was registered
        assert!(builtin_host.is_implemented(BuiltinType::ResourceCreate));

        // Test calling the built-in
        let mut engine = ();
        let result = builtin_host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(42)]);
    }

    #[test]
    fn test_fallback_registration() {
        let builder = HostBuilder::new()
            .with_fallback_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I32(99)]));

        let builtin_host = builder.build_builtin_host();

        // Check that the fallback was registered
        assert!(builtin_host.has_fallback(BuiltinType::ResourceCreate));

        // Test calling the built-in (should use fallback)
        let mut engine = ();
        let result = builtin_host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(99)]);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_builder_with_interceptor() {
        use std::sync::Arc;
        use wrt_intercept::{BeforeBuiltinResult, BuiltinInterceptor, InterceptContext};
        use wrt_types::component_value::ComponentValue;

        struct TestInterceptor;

        impl BuiltinInterceptor for TestInterceptor {
            fn before_builtin(
                &self,
                _context: &InterceptContext,
                _args: &[ComponentValue],
            ) -> Result<BeforeBuiltinResult> {
                // Bypass normal execution and return our own result
                Ok(BeforeBuiltinResult::Bypass(vec![ComponentValue::s32(777)]))
            }

            fn after_builtin(
                &self,
                _context: &InterceptContext,
                _args: &[ComponentValue],
                result: Result<Vec<ComponentValue>>,
            ) -> Result<Vec<ComponentValue>> {
                // Just pass through the result
                result
            }

            fn clone_interceptor(&self) -> Arc<dyn BuiltinInterceptor> {
                Arc::new(TestInterceptor)
            }
        }

        let builder = HostBuilder::new()
            .with_builtin_interceptor(Arc::new(TestInterceptor))
            .with_builtin_handler(BuiltinType::ResourceCreate, |_, _| {
                // This should never be called because the interceptor bypasses it
                Ok(vec![Value::I32(42)])
            });

        let builtin_host = builder.build_builtin_host();

        // Test calling the built-in
        let mut engine = ();
        let result = builtin_host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);

        // The interceptor should have bypassed the handler and returned 777
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(777)]);
    }
}

/// Create a runtime error with the specified message
///
/// This function properly handles both std and no_std environments
pub fn runtime_error(message: &str) -> Error {
    #[cfg(feature = "std")]
    return Error::runtime_error(message.to_string());

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    return Error::runtime_error(alloc::string::ToString::to_string(message));

    #[cfg(not(any(feature = "std", feature = "alloc")))]
    return Error::runtime_error(message);
}

/// Create a runtime error with a context string
///
/// This function properly handles both std and no_std environments
pub fn runtime_error_with_context(message: &str, context: &str) -> Error {
    #[cfg(feature = "std")]
    return Error::runtime_error(format!("{}: {}", message, context));

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    return Error::runtime_error(alloc::format!("{}: {}", message, context));

    #[cfg(not(any(feature = "std", feature = "alloc")))]
    return Error::runtime_error(message);
}
