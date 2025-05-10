//! Built-in function host implementation for WebAssembly components.
//!
//! This module provides the functionality for executing built-in functions
//! as defined in the WebAssembly Component Model.

// Use the prelude for consistent imports
use crate::prelude::*;

/// Converts wrt_types::values::Value to wrt_types::component_value::ComponentValue
///
/// This function converts WebAssembly core values to Component Model values
/// with support for both std and no_std environments.
fn convert_to_component_values(values: &[Value]) -> Vec<ComponentValue> {
    values
        .iter()
        .map(|v| match v {
            Value::I32(i) => ComponentValue::s32(*i),
            Value::I64(i) => ComponentValue::s64(*i),
            Value::F32(f) => ComponentValue::f32(*f),
            Value::F64(f) => ComponentValue::f64(*f),
            // Add other conversions as needed
            _ => ComponentValue::s32(0), // Default fallback
        })
        .collect()
}

/// Converts wrt_types::component_value::ComponentValue to wrt_types::values::Value
///
/// This function converts Component Model values to WebAssembly core values
/// with support for both std and no_std environments.
fn convert_from_component_values(values: &[ComponentValue]) -> Vec<Value> {
    values
        .iter()
        .map(|v| match v {
            ComponentValue::S8(i) => Value::I32(*i as i32),
            ComponentValue::S16(i) => Value::I32(*i as i32),
            ComponentValue::S32(i) => Value::I32(*i),
            ComponentValue::S64(i) => Value::I64(*i),
            ComponentValue::U8(i) => Value::I32(*i as i32),
            ComponentValue::U16(i) => Value::I32(*i as i32),
            ComponentValue::U32(i) => Value::I32(*i as i32),
            ComponentValue::U64(i) => Value::I64(*i as i64),
            ComponentValue::F32(f) => Value::F32(*f),
            ComponentValue::F64(f) => Value::F64(*f),
            // Add other conversions as needed
            _ => Value::I32(0), // Default fallback
        })
        .collect()
}

/// WebAssembly built-in function host implementation
#[derive(Default)]
pub struct BuiltinHost {
    /// Component name
    component_name: String,
    /// Host ID
    host_id: String,
    /// Interceptor for built-in calls
    interceptor: Option<Arc<dyn BuiltinInterceptor>>,
    /// Built-in handlers (builtin_type_name -> handler)
    handlers:
        HashMap<String, Box<dyn Fn(&mut dyn Any, Vec<Value>) -> Result<Vec<Value>> + Send + Sync>>,
    /// Critical built-ins that should have fallbacks
    critical_builtins: HashMap<
        BuiltinType,
        Box<dyn Fn(&mut dyn Any, Vec<Value>) -> Result<Vec<Value>> + Send + Sync>,
    >,
}

impl BuiltinHost {
    /// Create a new built-in host
    ///
    /// # Arguments
    ///
    /// * `component_name` - The name of the component
    /// * `host_id` - The host identifier
    ///
    /// # Returns
    ///
    /// A new `BuiltinHost` instance
    pub fn new(component_name: &str, host_id: &str) -> Self {
        Self {
            component_name: component_name.to_string(),
            host_id: host_id.to_string(),
            interceptor: None,
            handlers: HashMap::new(),
            critical_builtins: HashMap::new(),
        }
    }

    /// Set the interceptor for built-in calls
    ///
    /// # Arguments
    ///
    /// * `interceptor` - The interceptor to use
    pub fn set_interceptor(&mut self, interceptor: Arc<dyn BuiltinInterceptor>) {
        self.interceptor = Some(interceptor);
    }

    /// Register a handler for a built-in function
    ///
    /// # Arguments
    ///
    /// * `builtin_type` - The built-in type
    /// * `handler` - The handler function
    pub fn register_handler<F>(&mut self, builtin_type: BuiltinType, handler: F)
    where
        F: Fn(&mut dyn Any, Vec<Value>) -> Result<Vec<Value>> + Send + Sync + 'static,
    {
        self.handlers.insert(builtin_type.name().to_string(), Box::new(handler));
    }

    /// Register a fallback for a critical built-in function
    ///
    /// # Arguments
    ///
    /// * `builtin_type` - The built-in type
    /// * `handler` - The fallback handler function
    pub fn register_fallback<F>(&mut self, builtin_type: BuiltinType, handler: F)
    where
        F: Fn(&mut dyn Any, Vec<Value>) -> Result<Vec<Value>> + Send + Sync + 'static,
    {
        self.critical_builtins.insert(builtin_type, Box::new(handler));
    }

    /// Check if a built-in type is implemented
    ///
    /// # Arguments
    ///
    /// * `builtin_type` - The built-in type to check
    ///
    /// # Returns
    ///
    /// `true` if the built-in is implemented, `false` otherwise
    pub fn is_implemented(&self, builtin_type: BuiltinType) -> bool {
        self.handlers.contains_key(builtin_type.name())
    }

    /// Check if a built-in type has a fallback
    ///
    /// # Arguments
    ///
    /// * `builtin_type` - The built-in type to check
    ///
    /// # Returns
    ///
    /// `true` if the built-in has a fallback, `false` otherwise
    pub fn has_fallback(&self, builtin_type: BuiltinType) -> bool {
        self.critical_builtins.contains_key(&builtin_type)
    }

    /// Call a built-in function
    ///
    /// # Arguments
    ///
    /// * `engine` - The engine context
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
    pub fn call_builtin(
        &self,
        engine: &mut dyn Any,
        builtin_type: BuiltinType,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        // Apply interception if available
        if let Some(interceptor) = &self.interceptor {
            let context = InterceptContext::new(&self.component_name, builtin_type, &self.host_id);
            let component_args = convert_to_component_values(&args);

            // Before interceptor
            match interceptor.before_builtin(&context, &component_args)? {
                BeforeBuiltinResult::Continue(modified_args) => {
                    // Convert the modified args back to regular values
                    let modified_values = convert_from_component_values(&modified_args);

                    // Execute with potentially modified args
                    let result =
                        self.execute_builtin_internal(engine, builtin_type, modified_values);

                    // After interceptor - convert result to component values and back
                    let component_result = match &result {
                        Ok(values) => Ok(convert_to_component_values(values)),
                        Err(e) => Err(Error::new(
                            ErrorCategory::Runtime,
                            codes::RUNTIME_ERROR,
                            e.to_string(),
                        )),
                    };

                    let modified_result =
                        interceptor.after_builtin(&context, &component_args, component_result)?;
                    Ok(convert_from_component_values(&modified_result))
                }
                BeforeBuiltinResult::Bypass(result) => {
                    // Skip execution and use provided result
                    Ok(convert_from_component_values(&result))
                }
            }
        } else {
            // No interceptor, just execute
            self.execute_builtin_internal(engine, builtin_type, args)
        }
    }

    /// Internal implementation of execute_builtin without interception
    fn execute_builtin_internal(
        &self,
        engine: &mut dyn Any,
        builtin_type: BuiltinType,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        let builtin_name = builtin_type.name();

        // Try to find the handler
        if let Some(handler) = self.handlers.get(builtin_name) {
            return handler(engine, args);
        }

        // Try to use a fallback for critical built-ins
        if let Some(fallback) = self.critical_builtins.get(&builtin_type) {
            return fallback(engine, args);
        }

        // No handler or fallback found
        Err(Error::new(
            ErrorCategory::Runtime,
            codes::RUNTIME_ERROR,
            #[cfg(feature = "std")]
            format!("Built-in function {} not implemented", builtin_name),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            alloc::format!("Built-in function {} not implemented", builtin_name),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            "Built-in function not implemented",
        ))
    }
}

impl Clone for BuiltinHost {
    fn clone(&self) -> Self {
        // This is a simplified clone that doesn't actually clone the handlers
        // In a real implementation, you would need to properly clone all handlers
        Self {
            component_name: self.component_name.clone(),
            host_id: self.host_id.clone(),
            interceptor: self.interceptor.clone(),
            handlers: HashMap::new(),
            critical_builtins: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_types::values::Value;

    #[test]
    fn test_builtin_host_basics() {
        let host = BuiltinHost::new("test-component", "test-host");

        assert!(!host.is_implemented(BuiltinType::ResourceCreate));
        assert!(!host.has_fallback(BuiltinType::ResourceCreate));
    }

    #[test]
    fn test_register_handler() {
        let mut host = BuiltinHost::new("test-component", "test-host");

        host.register_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I32(42)]));

        assert!(host.is_implemented(BuiltinType::ResourceCreate));

        let mut engine = ();
        let result = host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(42)]);
    }

    #[test]
    fn test_fallback_mechanism() {
        let mut host = BuiltinHost::new("test-component", "test-host");

        // Register a fallback for ResourceCreate
        host.register_fallback(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I32(99)]));

        assert!(!host.is_implemented(BuiltinType::ResourceCreate));
        assert!(host.has_fallback(BuiltinType::ResourceCreate));

        let mut engine = ();
        let result = host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);

        // Should use the fallback
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(99)]);

        // Now register a regular handler
        host.register_handler(BuiltinType::ResourceCreate, |_, _| Ok(vec![Value::I32(42)]));

        let result = host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);

        // Should use the regular handler, not the fallback
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![Value::I32(42)]);
    }

    #[test]
    fn test_nonexistent_builtin() {
        let host = BuiltinHost::new("test-component", "test-host");

        let mut engine = ();
        let result = host.call_builtin(&mut engine, BuiltinType::ResourceCreate, vec![]);

        // Should fail because the built-in is not implemented
        assert!(result.is_err());
    }
}
