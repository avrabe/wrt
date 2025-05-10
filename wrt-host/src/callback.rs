//! Callback registry for host functions.
//!
//! This module provides a registry for callbacks that can be invoked from
//! WebAssembly components, including host functions and interceptors.

// Use the prelude for consistent imports
use crate::prelude::*;

/// Types of callbacks that can be registered
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallbackType {
    /// Callback for setup before execution
    Setup,
    /// Callback for cleanup after execution
    Cleanup,
    /// Callback for memory allocation
    Allocate,
    /// Callback for memory deallocation
    Deallocate,
    /// Callback for custom interceptors
    Intercept,
    /// Callback for logging
    Logging,
}

/// A callback registry for handling WebAssembly component operations
#[derive(Default)]
pub struct CallbackRegistry {
    /// Generic callback storage for different types of callbacks
    callbacks: HashMap<CallbackType, Box<dyn Any + Send + Sync>>,

    /// Host functions registry (module name -> function name -> handler)
    host_functions: HashMap<String, HashMap<String, HostFunctionHandler>>,

    /// Optional interceptor for monitoring and modifying function calls
    interceptor: Option<Arc<LinkInterceptor>>,
}

#[cfg(feature = "std")]
impl fmt::Debug for CallbackRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackRegistry")
            .field("registered_callbacks", &self.callbacks.keys())
            .field("registered_modules", &self.host_functions.keys())
            .finish()
    }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Debug for CallbackRegistry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CallbackRegistry")
            .field("registered_callbacks", &self.callbacks.keys())
            .field("registered_modules", &self.host_functions.keys())
            .finish()
    }
}

impl CallbackRegistry {
    /// Create a new callback registry
    #[must_use]
    pub fn new() -> Self {
        Self { callbacks: HashMap::new(), interceptor: None, host_functions: HashMap::new() }
    }

    /// Sets an interceptor for this registry
    pub fn with_interceptor(mut self, interceptor: Arc<LinkInterceptor>) -> Self {
        self.interceptor = Some(interceptor);
        self
    }

    /// Get the interceptor if one is set
    pub fn get_interceptor(&self) -> Option<&LinkInterceptor> {
        self.interceptor.as_ref().map(|arc| arc.as_ref())
    }

    /// Register a callback
    pub fn register_callback<T: 'static + Send + Sync>(
        &mut self,
        callback_type: CallbackType,
        callback: T,
    ) {
        self.callbacks.insert(callback_type, Box::new(callback));
    }

    /// Get a callback
    pub fn get_callback<T: 'static + Send + Sync>(
        &self,
        callback_type: &CallbackType,
    ) -> Option<&T> {
        self.callbacks.get(callback_type).and_then(|cb| cb.downcast_ref())
    }

    /// Get a mutable callback
    pub fn get_callback_mut<T: 'static + Send + Sync>(
        &mut self,
        callback_type: &CallbackType,
    ) -> Option<&mut T> {
        self.callbacks.get_mut(callback_type).and_then(|cb| cb.downcast_mut())
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

        let module_functions = self.host_functions.entry(module_name).or_default();
        module_functions.insert(function_name, handler);
    }

    /// Check if a host function is registered
    #[must_use]
    pub fn has_host_function(&self, module_name: &str, function_name: &str) -> bool {
        self.host_functions.get(module_name).and_then(|funcs| funcs.get(function_name)).is_some()
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
    ) -> Result<Vec<Value>> {
        // If we have an interceptor, use it to intercept the call
        if let Some(interceptor) = self.get_interceptor() {
            interceptor.intercept_call(
                "host",
                &function_key(module_name, function_name),
                args,
                |modified_args| {
                    self.call_host_function_internal(
                        engine,
                        module_name,
                        function_name,
                        modified_args,
                    )
                },
            )
        } else {
            self.call_host_function_internal(engine, module_name, function_name, args)
        }
    }

    /// Internal implementation of call_host_function without interception
    fn call_host_function_internal(
        &self,
        engine: &mut dyn Any,
        module_name: &str,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        if let Some(module_functions) = self.host_functions.get(module_name) {
            if let Some(handler) = module_functions.get(function_name) {
                return handler.call(engine, args);
            }
        }

        // Return error if the function is not found
        Err(Error::new(
            ErrorCategory::Runtime,
            codes::RUNTIME_ERROR,
            #[cfg(feature = "std")]
            format!("Host function {module_name}.{function_name} not found"),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            alloc::format!("Host function {module_name}.{function_name} not found"),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            "Host function not found",
        ))
    }

    /// Get all registered module names
    #[must_use]
    pub fn get_registered_modules(&self) -> Vec<&String> {
        self.host_functions.keys().collect()
    }

    /// Get all registered function names for a module
    #[must_use]
    pub fn get_registered_functions(&self, module_name: &str) -> Vec<&String> {
        if let Some(module_functions) = self.host_functions.get(module_name) {
            module_functions.keys().collect()
        } else {
            Vec::new()
        }
    }

    /// Get all available built-in types provided by this registry
    ///
    /// This method returns a set of all built-in types that are available
    /// through this registry's host functions.
    #[must_use]
    pub fn get_available_builtins(&self) -> crate::HashSet<BuiltinType> {
        use crate::HashSet;

        let mut builtins = HashSet::new();

        // Check for built-ins in the wasi_builtin module
        if let Some(builtin_funcs) = self.host_functions.get("wasi_builtin") {
            for func_name in builtin_funcs.keys() {
                if let Ok(builtin_type) = func_name.parse::<BuiltinType>() {
                    builtins.insert(builtin_type);
                }
            }
        }

        builtins
    }

    /// Call a built-in function
    ///
    /// # Arguments
    ///
    /// * `engine` - The engine context
    /// * `builtin_host` - The built-in host to use
    /// * `builtin_type` - The built-in type to call
    /// * `args` - The arguments to the function
    ///
    /// # Returns
    ///
    /// A `Result` containing the function results or an error
    ///
    /// # Errors
    ///
    /// Returns an error if the built-in is not implemented or fails during execution
    pub fn call_builtin_function(
        &self,
        engine: &mut dyn Any,
        builtin_host: &BuiltinHost,
        builtin_type: BuiltinType,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        // First check if we have a direct host function registered
        let builtin_name = builtin_type.name();
        if self.has_host_function("wasi_builtin", builtin_name) {
            return self.call_host_function(engine, "wasi_builtin", builtin_name, args);
        }

        // If not, delegate to the built-in host
        builtin_host.call_builtin(engine, builtin_type, args)
    }
}

impl Clone for CallbackRegistry {
    fn clone(&self) -> Self {
        // Create a new registry
        let mut new_registry = Self::new();

        // Clone the interceptor if present
        if let Some(interceptor) = &self.interceptor {
            new_registry.interceptor = Some(interceptor.clone());
        }

        // Clone host functions by creating new mappings with cloned handlers
        for (module_name, function_map) in &self.host_functions {
            for (function_name, handler) in function_map {
                new_registry.register_host_function(module_name, function_name, handler.clone());
            }
        }

        // Note: We can't easily clone the callbacks since they're Any type
        // In a real implementation, you would need to find a way to clone these as well

        new_registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_types::builtin::BuiltinType;
    use wrt_types::values::Value;

    #[test]
    fn test_callback_registry() {
        let mut registry = CallbackRegistry::new();

        // Register a host function
        let handler = HostFunctionHandler::new(|_| Ok(vec![Value::I32(42)]));
        registry.register_host_function("test_module", "test_function", handler);

        // Verify it can be found
        assert!(registry.has_host_function("test_module", "test_function"));
        assert!(!registry.has_host_function("nonexistent", "function"));

        // Call the function
        let mut engine = ();
        let result =
            registry.call_host_function(&mut engine, "test_module", "test_function", vec![]);

        // Fix the assertion to not rely on PartialEq for Error type
        match result {
            Ok(values) => {
                assert_eq!(values.len(), 1);
                assert!(matches!(values[0], Value::I32(42)));
            }
            Err(_) => panic!("Expected successful function call"),
        }

        // Test calling a nonexistent function
        let err = registry.call_host_function(&mut engine, "nonexistent", "function", vec![]);
        assert!(err.is_err());
    }

    #[test]
    fn test_callback_registry_callback() {
        let mut registry = CallbackRegistry::new();

        // Register a callback
        registry.register_callback(CallbackType::Intercept, 42);

        // Get the callback
        let callback = registry.get_callback::<i32>(&CallbackType::Intercept);
        assert!(callback.is_some());
        assert_eq!(*callback.unwrap(), 42);

        // Modify the callback
        if let Some(callback) = registry.get_callback_mut::<i32>(&CallbackType::Intercept) {
            *callback = 24;
        }

        // Verify it was modified
        let callback = registry.get_callback::<i32>(&CallbackType::Intercept);
        assert!(callback.is_some());
        assert_eq!(*callback.unwrap(), 24);
    }

    #[test]
    fn test_call_builtin_function() {
        // Create a registry with a host function for resource.create
        let mut registry = CallbackRegistry::new();
        let handler = HostFunctionHandler::new(|_| Ok(vec![Value::I32(42)]));
        registry.register_host_function("wasi_builtin", "resource.create", handler);

        // Create a built-in host with a different implementation
        let mut builtin_host = BuiltinHost::new("test-component", "test-host");
        builtin_host.register_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I32(99)]));

        // Test calling via registry - should use the registry's implementation
        let mut engine = ();
        let result = registry.call_builtin_function(
            &mut engine,
            &builtin_host,
            BuiltinType::ResourceCreate,
            vec![],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(42)]);

        // Now test with a built-in that's only in the host
        let result = registry.call_builtin_function(
            &mut engine,
            &builtin_host,
            BuiltinType::ResourceDrop,
            vec![],
        );

        // Should fail because neither registry nor host implements it
        assert!(result.is_err());

        // Now add it to the host
        builtin_host.register_handler(BuiltinType::ResourceDrop, |_, _| Ok(vec![Value::I32(55)]));

        // Try again
        let result = registry.call_builtin_function(
            &mut engine,
            &builtin_host,
            BuiltinType::ResourceDrop,
            vec![],
        );

        // Should work now
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(55)]);
    }
}

/// Generate a unique function key from module and function names
///
/// This function creates a unique identifier for a function by combining
/// the module name and function name. It's used to lookup functions in
/// registries and for interception.
///
/// # Arguments
///
/// * `module_name` - The name of the module containing the function
/// * `function_name` - The name of the function
///
/// # Returns
///
/// A string in the format `module_name::function_name`
pub fn function_key(module_name: &str, function_name: &str) -> String {
    #[cfg(feature = "std")]
    return format!("{}::{}", module_name, function_name);

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    return alloc::format!("{}::{}", module_name, function_name);

    #[cfg(not(any(feature = "std", feature = "alloc")))]
    {
        // Fallback for environments without allocation
        // This is a simplified version that won't work for all cases
        let mut result = String::from(module_name);
        result.push_str("::");
        result.push_str(function_name);
        result
    }
}
