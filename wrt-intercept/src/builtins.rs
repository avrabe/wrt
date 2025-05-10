//! Built-in interception for WebAssembly Component Model
//!
//! This module provides facilities for intercepting built-in function calls
//! in the WebAssembly Component Model implementation.

use crate::prelude::*;

/// Context for built-in interception
///
/// This struct provides context for built-in interception, including
/// information about the caller, the built-in being called, and any
/// additional context needed for making interception decisions.
#[derive(Debug, Clone)]
pub struct InterceptContext {
    /// The name of the component making the built-in call
    pub component_name: String,
    /// The built-in function being called
    pub builtin_type: BuiltinType,
    /// The host environment's unique identifier
    pub host_id: String,
    /// Additional context data (if any)
    #[cfg(feature = "std")]
    pub context_data: std::collections::HashMap<String, Value>,
}

impl InterceptContext {
    /// Create a new interception context
    ///
    /// # Arguments
    ///
    /// * `component_name` - The name of the calling component
    /// * `builtin_type` - The built-in function being called
    /// * `host_id` - The host identifier
    ///
    /// # Returns
    ///
    /// A new `InterceptContext` instance
    pub fn new(component_name: &str, builtin_type: BuiltinType, host_id: &str) -> Self {
        Self {
            component_name: component_name.to_string(),
            builtin_type,
            host_id: host_id.to_string(),
            #[cfg(feature = "std")]
            context_data: std::collections::HashMap::new(),
        }
    }

    /// Add context data (only available with `std` feature)
    #[cfg(feature = "std")]
    pub fn add_data(&mut self, key: &str, value: Value) {
        self.context_data.insert(key.to_string(), value);
    }

    /// Get context data (only available with `std` feature)
    #[cfg(feature = "std")]
    pub fn get_data(&self, key: &str) -> Option<&Value> {
        self.context_data.get(key)
    }
}

/// Serialization helper for built-in arguments
///
/// This struct provides methods for serializing and deserializing
/// arguments and results for built-in function calls.
pub struct BuiltinSerialization;

impl BuiltinSerialization {
    /// Serialize component values to bytes
    ///
    /// # Arguments
    ///
    /// * `values` - The values to serialize
    ///
    /// # Returns
    ///
    /// A `Result` containing the serialized bytes or an error
    pub fn serialize(values: &[ComponentValue]) -> Result<Vec<u8>> {
        // Simple implementation for now - convert to bytes
        let mut result = Vec::new();
        for value in values {
            let bytes = match value {
                ComponentValue::S32(v) => v.to_le_bytes().to_vec(),
                ComponentValue::S64(v) => v.to_le_bytes().to_vec(),
                ComponentValue::F32(v) => v.to_le_bytes().to_vec(),
                ComponentValue::F64(v) => v.to_le_bytes().to_vec(),
                _ => {
                    return Err(Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::INVALID_TYPE,
                        "Unsupported value type for serialization",
                    ))
                }
            };
            result.extend(bytes);
        }
        Ok(result)
    }

    /// Deserialize bytes to component values
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to deserialize
    /// * `types` - The types of values to deserialize
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized values or an error
    pub fn deserialize(bytes: &[u8], types: &[ValType]) -> Result<Vec<ComponentValue>> {
        let mut result = Vec::new();
        let mut offset = 0;

        for ty in types {
            match ty {
                ValType::S32 => {
                    if offset + 4 > bytes.len() {
                        return Err(Error::new(
                            wrt_error::ErrorCategory::Parse,
                            wrt_error::codes::PARSE_ERROR,
                            "Insufficient bytes for i32",
                        ));
                    }
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(&bytes[offset..offset + 4]);
                    result.push(ComponentValue::S32(i32::from_le_bytes(buf)));
                    offset += 4;
                }
                ValType::S64 => {
                    if offset + 8 > bytes.len() {
                        return Err(Error::new(
                            wrt_error::ErrorCategory::Parse,
                            wrt_error::codes::PARSE_ERROR,
                            "Insufficient bytes for i64",
                        ));
                    }
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&bytes[offset..offset + 8]);
                    result.push(ComponentValue::S64(i64::from_le_bytes(buf)));
                    offset += 8;
                }
                ValType::F32 => {
                    if offset + 4 > bytes.len() {
                        return Err(Error::new(
                            wrt_error::ErrorCategory::Parse,
                            wrt_error::codes::PARSE_ERROR,
                            "Insufficient bytes for f32",
                        ));
                    }
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(&bytes[offset..offset + 4]);
                    result.push(ComponentValue::F32(f32::from_le_bytes(buf)));
                    offset += 4;
                }
                ValType::F64 => {
                    if offset + 8 > bytes.len() {
                        return Err(Error::new(
                            wrt_error::ErrorCategory::Parse,
                            wrt_error::codes::PARSE_ERROR,
                            "Insufficient bytes for f64",
                        ));
                    }
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&bytes[offset..offset + 8]);
                    result.push(ComponentValue::F64(f64::from_le_bytes(buf)));
                    offset += 8;
                }
                _ => {
                    return Err(Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::INVALID_TYPE,
                        "Unsupported value type for deserialization",
                    ))
                }
            }
        }

        Ok(result)
    }

    // NOTE: The following two functions (serialize_args, supported_serialization_types) were erroneously added
    // by a previous edit and should be removed if they are not part of the original DefaultBuiltinSerialization.
    // For now, I am commenting them out to ensure the build doesn't break, but they should be deleted if not needed.
    /*
    fn serialize_args(
        &self,
        context: &InterceptContext,
        args: &[ComponentValue],
        types: &[ValType],
    ) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        for (index, value) in args.iter().enumerate() {
            match types.get(index) {
                Some(ty) => match ty {
                    ValType::S32 => {
                        if let Some(ComponentValue::S32(val)) = args.get(index) {
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                    }
                    ValType::S64 => {
                        if let Some(ComponentValue::S64(val)) = args.get(index) {
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                    }
                    ValType::F32 => {
                        if let Some(ComponentValue::F32(val)) = args.get(index) {
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                    }
                    ValType::F64 => {
                        if let Some(ComponentValue::F64(val)) = args.get(index) {
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            wrt_error::ErrorCategory::Type,
                            wrt_error::codes::INVALID_TYPE,
                            "Unsupported value type for serialization",
                        ))
                    }
                },
                None => {
                    return Err(Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::INVALID_TYPE,
                        "Index out of bounds for serialization",
                    ))
                }
            }
        }
        Ok(bytes)
    }

    fn supported_serialization_types() -> Vec<ValType> {
        // Example: only basic numeric types are supported for now
        vec![
            ValType::S32,
            ValType::S64,
            ValType::F32,
            ValType::F64,
        ]
    }
    */
}

/// The BuiltinInterceptor trait defines methods for intercepting and
/// potentially modifying built-in function calls in the WebAssembly
/// Component Model implementation.
pub trait BuiltinInterceptor: Send + Sync {
    /// Called before a built-in function is invoked
    ///
    /// # Arguments
    ///
    /// * `context` - The interception context
    /// * `args` - The arguments to the built-in function
    ///
    /// # Returns
    ///
    /// A `Result` containing potentially modified arguments, or a complete result
    /// if the built-in execution should be bypassed
    fn before_builtin(
        &self,
        context: &InterceptContext,
        args: &[ComponentValue],
    ) -> Result<BeforeBuiltinResult>;

    /// Called after a built-in function has been invoked
    ///
    /// # Arguments
    ///
    /// * `context` - The interception context
    /// * `args` - The original arguments to the built-in function
    /// * `result` - The result of the built-in function call
    ///
    /// # Returns
    ///
    /// A `Result` containing potentially modified result values
    fn after_builtin(
        &self,
        context: &InterceptContext,
        args: &[ComponentValue],
        result: Result<Vec<ComponentValue>>,
    ) -> Result<Vec<ComponentValue>>;

    /// Clone this interceptor
    ///
    /// # Returns
    ///
    /// A cloned version of this interceptor
    fn clone_interceptor(&self) -> Arc<dyn BuiltinInterceptor>;
}

/// Result of the `before_builtin` method
pub enum BeforeBuiltinResult {
    /// Continue with the built-in execution using the provided arguments
    Continue(Vec<ComponentValue>),
    /// Skip the built-in execution and use these values as the result
    Bypass(Vec<ComponentValue>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercept_context() {
        let context =
            InterceptContext::new("test-component", BuiltinType::ResourceCreate, "test-host");

        assert_eq!(context.component_name, "test-component");
        assert_eq!(context.builtin_type, BuiltinType::ResourceCreate);
        assert_eq!(context.host_id, "test-host");

        #[cfg(feature = "std")]
        {
            let mut context = context;
            context.add_data("test-key", Value::I32(42));
            assert_eq!(context.get_data("test-key"), Some(&Value::I32(42)));
            assert_eq!(context.get_data("non-existent"), None);
        }
    }

    #[test]
    fn test_builtin_serialization() {
        let values = vec![
            ComponentValue::S32(123),
            ComponentValue::S64(456),
            ComponentValue::F32(1.23),
            ComponentValue::F64(4.56),
        ];

        let serialized_bytes = BuiltinSerialization::serialize(&values).unwrap();

        let types = vec![ValType::S32, ValType::S64, ValType::F32, ValType::F64];

        let deserialized_values =
            BuiltinSerialization::deserialize(&serialized_bytes, &types).unwrap();

        assert_eq!(deserialized_values.len(), values.len());
        assert_eq!(deserialized_values[0], values[0]);
        assert_eq!(deserialized_values[1], values[1]);
        // For floating point, we need to handle potential rounding issues
        if let (ComponentValue::F32(a), ComponentValue::F32(b)) =
            (&deserialized_values[2], &values[2])
        {
            assert!((a - b).abs() < f32::EPSILON);
        } else {
            panic!("Expected F32 values");
        }
        if let (ComponentValue::F64(a), ComponentValue::F64(b)) =
            (&deserialized_values[3], &values[3])
        {
            assert!((a - b).abs() < f64::EPSILON);
        } else {
            panic!("Expected F64 values");
        }
    }
}
