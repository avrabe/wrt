use crate::types::ValueType;
use crate::{Box, String, Vec};
use std::fmt;

/// Represents a WebAssembly value that can be used in the runtime.
///
/// This enum encompasses both core WebAssembly value types (i32, i64, f32, f64,
/// funcref, externref) and component model value types (record, tuple, list, etc.)
/// for use with the WebAssembly Component Model.
///
/// # Examples
///
/// ```
/// use wrt::{Value, ValueType};
///
/// // Create a simple i32 value
/// let value = Value::I32(42);
/// assert_eq!(value.type_(), ValueType::I32);
///
/// // Create a more complex component model value
/// let list_value = Value::List(vec![Box::new(Value::I32(1)), Box::new(Value::I32(2))]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 32-bit signed integer value
    I32(i32),

    /// 64-bit signed integer value
    I64(i64),

    /// 32-bit floating point value
    F32(f32),

    /// 64-bit floating point value
    F64(f64),

    /// Function reference value, containing an optional function index
    FuncRef(Option<u32>),

    /// External reference value, containing an optional reference index
    ExternRef(Option<u32>),

    /// Any reference value, containing an optional reference index
    AnyRef(Option<u32>),

    /// 128-bit SIMD vector value
    V128(u128),

    /// Record value from the component model, containing named fields
    /// with their corresponding values
    Record(Vec<(String, Box<Value>)>),

    /// Tuple value from the component model, containing a sequence of
    /// unnamed values
    Tuple(Vec<Box<Value>>),

    /// List value from the component model, containing a sequence of
    /// values of the same type
    List(Vec<Box<Value>>),

    /// Flags value from the component model, representing a set of
    /// boolean flags as their names when true
    Flags(Vec<String>),

    /// Variant value from the component model, containing a discriminant
    /// and an optional payload value
    Variant(String, Option<Box<Value>>),

    /// Enum value from the component model, containing just the discriminant
    Enum(String),

    /// Union value from the component model, representing a value that
    /// can be one of multiple possible types
    Union(Box<Value>),

    /// Option value from the component model, representing an optional value
    Option(Option<Box<Value>>),

    /// Result value from the component model, representing either a success
    /// value or an error value
    Result(std::result::Result<Box<Value>, Box<Value>>),

    /// Future value from the component model, representing a value that
    /// will be available asynchronously
    Future(Box<Value>),

    /// Stream value from the component model, representing a sequence of
    /// values that will be available asynchronously
    Stream {
        /// The element type of the stream
        element: Box<Value>,

        /// An optional end value that may be present when the stream completes
        end: Option<Box<Value>>,
    },
}

impl Value {
    /// Creates a default value for the given WebAssembly value type.
    ///
    /// This function returns a zero value for numeric types and None for reference types.
    ///
    /// # Parameters
    ///
    /// * `ty` - The WebAssembly value type for which to create a default value
    ///
    /// # Returns
    ///
    /// A Value that represents the default for the given type
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, ValueType};
    ///
    /// let i32_default = Value::default_for_type(&ValueType::I32);
    /// assert_eq!(i32_default.as_i32(), Some(0));
    ///
    /// let func_ref_default = Value::default_for_type(&ValueType::FuncRef);
    /// assert_eq!(func_ref_default.as_func_ref(), Some(None));
    /// ```
    pub fn default_for_type(ty: &ValueType) -> Self {
        match ty {
            ValueType::I32 => Value::I32(0),
            ValueType::I64 => Value::I64(0),
            ValueType::F32 => Value::F32(0.0),
            ValueType::F64 => Value::F64(0.0),
            ValueType::FuncRef => Value::FuncRef(None),
            ValueType::ExternRef => Value::ExternRef(None),
            ValueType::V128 => Value::V128(0),
            ValueType::AnyRef => Value::AnyRef(None),
        }
    }

    /// Returns the WebAssembly value type of this value.
    ///
    /// Note that for component model values (Record, Tuple, etc.), this currently
    /// returns I32 as a placeholder since they don't directly map to core WebAssembly types.
    ///
    /// # Returns
    ///
    /// The ValueType that describes this value
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, ValueType};
    ///
    /// let value = Value::F64(3.14);
    /// assert_eq!(value.type_(), ValueType::F64);
    /// ```
    pub fn type_(&self) -> ValueType {
        match self {
            Value::I32(_) => ValueType::I32,
            Value::I64(_) => ValueType::I64,
            Value::F32(_) => ValueType::F32,
            Value::F64(_) => ValueType::F64,
            Value::FuncRef(_) => ValueType::FuncRef,
            Value::ExternRef(_) => ValueType::ExternRef,
            Value::V128(_) => ValueType::V128,
            Value::AnyRef(_) => ValueType::AnyRef,
            Value::Record(_) => ValueType::I32,
            Value::Tuple(_) => ValueType::I32,
            Value::List(_) => ValueType::I32,
            Value::Flags(_) => ValueType::I32,
            Value::Variant(_, _) => ValueType::I32,
            Value::Enum(_) => ValueType::I32,
            Value::Union(_) => ValueType::I32,
            Value::Option(_) => ValueType::I32,
            Value::Result(_) => ValueType::I32,
            Value::Future(_) => ValueType::I32,
            Value::Stream { .. } => ValueType::I32,
        }
    }

    /// Checks if this value matches the given WebAssembly value type.
    ///
    /// # Parameters
    ///
    /// * `ty` - The ValueType to check against
    ///
    /// # Returns
    ///
    /// true if the value matches the given type, false otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, ValueType};
    ///
    /// let value = Value::I32(42);
    /// assert!(value.matches_type(&ValueType::I32));
    /// assert!(!value.matches_type(&ValueType::I64));
    /// ```
    pub fn matches_type(&self, ty: &ValueType) -> bool {
        match (self, ty) {
            (Value::I32(_), ValueType::I32) => true,
            (Value::I64(_), ValueType::I64) => true,
            (Value::F32(_), ValueType::F32) => true,
            (Value::F64(_), ValueType::F64) => true,
            (Value::FuncRef(_), ValueType::FuncRef) => true,
            (Value::ExternRef(_), ValueType::ExternRef) => true,
            (Value::V128(_), ValueType::V128) => true,
            (Value::AnyRef(_), ValueType::AnyRef) => true,
            (Value::Record(_), ValueType::I32) => true,
            (Value::Tuple(_), ValueType::I32) => true,
            (Value::List(_), ValueType::I32) => true,
            (Value::Flags(_), ValueType::I32) => true,
            (Value::Variant(_, _), ValueType::I32) => true,
            (Value::Enum(_), ValueType::I32) => true,
            (Value::Union(_), ValueType::I32) => true,
            (Value::Option(_), ValueType::I32) => true,
            (Value::Result(_), ValueType::I32) => true,
            (Value::Future(_), ValueType::I32) => true,
            (Value::Stream { .. }, ValueType::I32) => true,
            _ => false,
        }
    }

    /// Attempts to extract an i32 value if this Value is an I32.
    ///
    /// # Returns
    ///
    /// Some(i32) if this is an I32 value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::Value;
    ///
    /// let value = Value::I32(42);
    /// assert_eq!(value.as_i32(), Some(42));
    ///
    /// let value = Value::F32(3.14);
    /// assert_eq!(value.as_i32(), None);
    /// ```
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I32(x) => Some(*x),
            _ => None,
        }
    }

    /// Attempts to extract an i64 value if this Value is an I64.
    ///
    /// # Returns
    ///
    /// Some(i64) if this is an I64 value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::Value;
    ///
    /// let value = Value::I64(42);
    /// assert_eq!(value.as_i64(), Some(42));
    ///
    /// let value = Value::I32(42);
    /// assert_eq!(value.as_i64(), None);
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(x) => Some(*x),
            _ => None,
        }
    }

    /// Attempts to extract an f32 value if this Value is an F32.
    ///
    /// # Returns
    ///
    /// Some(f32) if this is an F32 value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::Value;
    ///
    /// let value = Value::F32(3.14);
    /// assert_eq!(value.as_f32(), Some(3.14));
    ///
    /// let value = Value::I32(42);
    /// assert_eq!(value.as_f32(), None);
    /// ```
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(x) => Some(*x),
            _ => None,
        }
    }

    /// Attempts to extract an f64 value if this Value is an F64.
    ///
    /// # Returns
    ///
    /// Some(f64) if this is an F64 value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::Value;
    ///
    /// let value = Value::F64(3.14159);
    /// assert_eq!(value.as_f64(), Some(3.14159));
    ///
    /// let value = Value::F32(3.14);
    /// assert_eq!(value.as_f64(), None);
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(x) => Some(*x),
            _ => None,
        }
    }

    /// Attempts to extract a function reference if this Value is a FuncRef.
    ///
    /// # Returns
    ///
    /// Some(Option<u32>) if this is a FuncRef value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::Value;
    ///
    /// let value = Value::FuncRef(Some(5));
    /// assert_eq!(value.as_func_ref(), Some(Some(5)));
    ///
    /// let value = Value::FuncRef(None);
    /// assert_eq!(value.as_func_ref(), Some(None));
    ///
    /// let value = Value::I32(42);
    /// assert_eq!(value.as_func_ref(), None);
    /// ```
    pub fn as_func_ref(&self) -> Option<Option<u32>> {
        match self {
            Value::FuncRef(x) => Some(*x),
            _ => None,
        }
    }

    /// Attempts to extract an external reference if this Value is an ExternRef.
    ///
    /// # Returns
    ///
    /// Some(Option<u32>) if this is an ExternRef value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::Value;
    ///
    /// let value = Value::ExternRef(Some(5));
    /// assert_eq!(value.as_extern_ref(), Some(Some(5)));
    ///
    /// let value = Value::ExternRef(None);
    /// assert_eq!(value.as_extern_ref(), Some(None));
    ///
    /// let value = Value::I32(42);
    /// assert_eq!(value.as_extern_ref(), None);
    /// ```
    pub fn as_extern_ref(&self) -> Option<Option<u32>> {
        match self {
            Value::ExternRef(x) => Some(*x),
            _ => None,
        }
    }

    /// Attempts to extract a record value if this Value is a Record.
    ///
    /// # Returns
    ///
    /// Some reference to the record fields if this is a Record value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, String, Box};
    ///
    /// let record = Value::Record(vec![
    ///     (String::from("field1"), Box::new(Value::I32(42)))
    /// ]);
    /// assert!(record.as_record().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_record().is_none());
    /// ```
    pub fn as_record(&self) -> Option<&Vec<(String, Box<Value>)>> {
        match self {
            Value::Record(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a tuple value if this Value is a Tuple.
    ///
    /// # Returns
    ///
    /// Some reference to the tuple elements if this is a Tuple value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    ///
    /// let tuple = Value::Tuple(vec![Box::new(Value::I32(1)), Box::new(Value::I32(2))]);
    /// assert!(tuple.as_tuple().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_tuple().is_none());
    /// ```
    pub fn as_tuple(&self) -> Option<&Vec<Box<Value>>> {
        match self {
            Value::Tuple(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a list value if this Value is a List.
    ///
    /// # Returns
    ///
    /// Some reference to the list elements if this is a List value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    ///
    /// let list = Value::List(vec![Box::new(Value::I32(1)), Box::new(Value::I32(2))]);
    /// assert!(list.as_list().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_list().is_none());
    /// ```
    pub fn as_list(&self) -> Option<&Vec<Box<Value>>> {
        match self {
            Value::List(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a flags value if this Value is a Flags.
    ///
    /// # Returns
    ///
    /// Some reference to the flag names if this is a Flags value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, String};
    ///
    /// let flags = Value::Flags(vec![String::from("enabled"), String::from("visible")]);
    /// assert!(flags.as_flags().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_flags().is_none());
    /// ```
    pub fn as_flags(&self) -> Option<&Vec<String>> {
        match self {
            Value::Flags(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a variant value if this Value is a Variant.
    ///
    /// # Returns
    ///
    /// Some tuple with the discriminant and optional payload if this is a Variant value,
    /// None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, String, Box};
    ///
    /// let variant = Value::Variant(String::from("some"), Some(Box::new(Value::I32(42))));
    /// assert!(variant.as_variant().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_variant().is_none());
    /// ```
    pub fn as_variant(&self) -> Option<(&String, &Option<Box<Value>>)> {
        match self {
            Value::Variant(x, y) => Some((x, y)),
            _ => None,
        }
    }

    /// Attempts to extract an enum value if this Value is an Enum.
    ///
    /// # Returns
    ///
    /// Some reference to the enum discriminant if this is an Enum value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, String};
    ///
    /// let enum_val = Value::Enum(String::from("RED"));
    /// assert!(enum_val.as_enum().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_enum().is_none());
    /// ```
    pub fn as_enum(&self) -> Option<&String> {
        match self {
            Value::Enum(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a union value if this Value is a Union.
    ///
    /// # Returns
    ///
    /// Some reference to the contained value if this is a Union value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    ///
    /// let union_val = Value::Union(Box::new(Value::I32(42)));
    /// assert!(union_val.as_union().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_union().is_none());
    /// ```
    pub fn as_union(&self) -> Option<&Value> {
        match self {
            Value::Union(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract an option value if this Value is an Option.
    ///
    /// # Returns
    ///
    /// Some reference to the option if this is an Option value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    ///
    /// let some_val = Value::Option(Some(Box::new(Value::I32(42))));
    /// assert!(some_val.as_option().is_some());
    ///
    /// let none_val = Value::Option(None);
    /// assert!(none_val.as_option().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_option().is_none());
    /// ```
    pub fn as_option(&self) -> Option<&Option<Box<Value>>> {
        match self {
            Value::Option(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a result value if this Value is a Result.
    ///
    /// # Returns
    ///
    /// Some reference to the result if this is a Result value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    /// use std::result::Result as StdResult;
    ///
    /// let ok_val = Value::Result(Ok(Box::new(Value::I32(42))));
    /// assert!(ok_val.as_result().is_some());
    ///
    /// let err_val = Value::Result(Err(Box::new(Value::I32(404))));
    /// assert!(err_val.as_result().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_result().is_none());
    /// ```
    pub fn as_result(&self) -> Option<&std::result::Result<Box<Value>, Box<Value>>> {
        match self {
            Value::Result(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a future value if this Value is a Future.
    ///
    /// # Returns
    ///
    /// Some reference to the future's value if this is a Future value, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    ///
    /// let future_val = Value::Future(Box::new(Value::I32(42)));
    /// assert!(future_val.as_future().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_future().is_none());
    /// ```
    pub fn as_future(&self) -> Option<&Value> {
        match self {
            Value::Future(x) => Some(x),
            _ => None,
        }
    }

    /// Attempts to extract a stream value if this Value is a Stream.
    ///
    /// # Returns
    ///
    /// Some tuple with references to the element type and end value if this is a Stream value,
    /// None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, Box};
    ///
    /// let stream_val = Value::Stream {
    ///     element: Box::new(Value::I32(42)),
    ///     end: Some(Box::new(Value::I32(0))),
    /// };
    /// assert!(stream_val.as_stream().is_some());
    ///
    /// let value = Value::I32(42);
    /// assert!(value.as_stream().is_none());
    /// ```
    pub fn as_stream(&self) -> Option<(&Value, &Option<Box<Value>>)> {
        match self {
            Value::Stream { element, end } => Some((element, end)),
            _ => None,
        }
    }

    /// Get the SIMD v128 value, if this is a V128 value
    pub fn as_v128(&self) -> Option<u128> {
        match self {
            Value::V128(val) => Some(*val),
            _ => None,
        }
    }

    /// Attempts to extract an anyref value if this Value is an AnyRef.
    ///
    /// # Returns
    ///
    /// Some reference to the anyref value if this is an AnyRef value, None otherwise
    pub fn as_any_ref(&self) -> Option<Option<u32>> {
        match self {
            Value::AnyRef(ref_idx) => Some(*ref_idx),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I32(v) => write!(f, "i32: {}", v),
            Value::I64(v) => write!(f, "i64: {}", v),
            Value::F32(v) => write!(f, "f32: {}", v),
            Value::F64(v) => write!(f, "f64: {}", v),
            Value::FuncRef(Some(v)) => write!(f, "funcref: {}", v),
            Value::FuncRef(None) => write!(f, "funcref: null"),
            Value::ExternRef(Some(v)) => write!(f, "externref: {}", v),
            Value::ExternRef(None) => write!(f, "externref: null"),
            Value::AnyRef(Some(v)) => write!(f, "anyref: {}", v),
            Value::AnyRef(None) => write!(f, "anyref: null"),
            Value::V128(v) => write!(f, "v128: 0x{:032x}", v),
            Value::Record(fields) => write!(f, "record: {:?}", fields),
            Value::Tuple(v) => write!(f, "tuple: {:?}", v),
            Value::List(v) => write!(f, "list: {:?}", v),
            Value::Flags(v) => write!(f, "flags: {:?}", v),
            Value::Variant(x, y) => write!(f, "variant: ({}, {:?})", x, y),
            Value::Enum(x) => write!(f, "enum: {}", x),
            Value::Union(x) => write!(f, "union: {:?}", x),
            Value::Option(x) => write!(f, "option: {:?}", x),
            Value::Result(x) => write!(f, "result: {:?}", x),
            Value::Future(x) => write!(f, "future: {:?}", x),
            Value::Stream { element, end } => write!(f, "stream: ({}, {:?})", element, end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::format;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_value_creation_and_type() {
        // Test numeric types
        let i32_val = Value::I32(42);
        assert_eq!(i32_val.type_(), ValueType::I32);

        let i64_val = Value::I64(42);
        assert_eq!(i64_val.type_(), ValueType::I64);

        let f32_val = Value::F32(3.14);
        assert_eq!(f32_val.type_(), ValueType::F32);

        let f64_val = Value::F64(3.14159);
        assert_eq!(f64_val.type_(), ValueType::F64);

        // Test reference types
        let func_ref = Value::FuncRef(Some(1));
        assert_eq!(func_ref.type_(), ValueType::FuncRef);

        let extern_ref = Value::ExternRef(Some(1));
        assert_eq!(extern_ref.type_(), ValueType::ExternRef);
    }

    #[test]
    fn test_value_default_creation() {
        // Test default values for each type
        assert_eq!(Value::default_for_type(&ValueType::I32), Value::I32(0));
        assert_eq!(Value::default_for_type(&ValueType::I64), Value::I64(0));
        assert_eq!(Value::default_for_type(&ValueType::F32), Value::F32(0.0));
        assert_eq!(Value::default_for_type(&ValueType::F64), Value::F64(0.0));
        assert_eq!(
            Value::default_for_type(&ValueType::FuncRef),
            Value::FuncRef(None)
        );
        assert_eq!(
            Value::default_for_type(&ValueType::ExternRef),
            Value::ExternRef(None)
        );
    }

    #[test]
    fn test_value_type_matching() {
        let i32_val = Value::I32(42);
        assert!(i32_val.matches_type(&ValueType::I32));
        assert!(!i32_val.matches_type(&ValueType::I64));

        let func_ref = Value::FuncRef(Some(1));
        assert!(func_ref.matches_type(&ValueType::FuncRef));
        assert!(!func_ref.matches_type(&ValueType::ExternRef));
    }

    #[test]
    fn test_numeric_value_extraction() {
        // Test i32
        let i32_val = Value::I32(42);
        assert_eq!(i32_val.as_i32(), Some(42));
        assert_eq!(i32_val.as_i64(), None);
        assert_eq!(i32_val.as_f32(), None);
        assert_eq!(i32_val.as_f64(), None);

        // Test i64
        let i64_val = Value::I64(42);
        assert_eq!(i64_val.as_i64(), Some(42));
        assert_eq!(i64_val.as_i32(), None);

        // Test f32
        let f32_val = Value::F32(3.14);
        assert_eq!(f32_val.as_f32(), Some(3.14));
        assert_eq!(f32_val.as_f64(), None);

        // Test f64
        let f64_val = Value::F64(3.14159);
        assert_eq!(f64_val.as_f64(), Some(3.14159));
        assert_eq!(f64_val.as_f32(), None);
    }

    #[test]
    fn test_reference_value_extraction() {
        // Test FuncRef
        let func_ref = Value::FuncRef(Some(42));
        assert_eq!(func_ref.as_func_ref(), Some(Some(42)));
        assert_eq!(func_ref.as_extern_ref(), None);

        let null_func_ref = Value::FuncRef(None);
        assert_eq!(null_func_ref.as_func_ref(), Some(None));

        // Test ExternRef
        let extern_ref = Value::ExternRef(Some(42));
        assert_eq!(extern_ref.as_extern_ref(), Some(Some(42)));
        assert_eq!(extern_ref.as_func_ref(), None);

        let null_extern_ref = Value::ExternRef(None);
        assert_eq!(null_extern_ref.as_extern_ref(), Some(None));
    }

    #[test]
    fn test_component_model_values() {
        // Test Record
        let record = Value::Record(vec![
            ("field1".to_string(), Box::new(Value::I32(1))),
            ("field2".to_string(), Box::new(Value::I64(2))),
        ]);
        assert!(record.as_record().is_some());
        assert_eq!(record.as_record().unwrap().len(), 2);

        // Test Tuple
        let tuple = Value::Tuple(vec![Box::new(Value::I32(1)), Box::new(Value::I64(2))]);
        assert!(tuple.as_tuple().is_some());
        assert_eq!(tuple.as_tuple().unwrap().len(), 2);

        // Test List
        let list = Value::List(vec![Box::new(Value::I32(1)), Box::new(Value::I32(2))]);
        assert!(list.as_list().is_some());
        assert_eq!(list.as_list().unwrap().len(), 2);

        // Test Flags
        let flags = Value::Flags(vec!["flag1".to_string(), "flag2".to_string()]);
        assert!(flags.as_flags().is_some());
        assert_eq!(flags.as_flags().unwrap().len(), 2);

        // Test Variant
        let variant = Value::Variant("some".to_string(), Some(Box::new(Value::I32(42))));
        assert!(variant.as_variant().is_some());
        assert_eq!(variant.as_variant().unwrap().0, "some");

        // Test Enum
        let enum_val = Value::Enum("RED".to_string());
        assert!(enum_val.as_enum().is_some());
        assert_eq!(enum_val.as_enum().unwrap(), "RED");

        // Test Union
        let union = Value::Union(Box::new(Value::I32(42)));
        assert!(union.as_union().is_some());
        assert_eq!(union.as_union().unwrap().as_i32(), Some(42));

        // Test Option
        let some_val = Value::Option(Some(Box::new(Value::I32(42))));
        assert!(some_val.as_option().is_some());
        let none_val = Value::Option(None);
        assert!(none_val.as_option().is_some());
        assert!(none_val.as_option().unwrap().is_none());

        // Test Result
        let ok_val = Value::Result(Ok(Box::new(Value::I32(42))));
        assert!(ok_val.as_result().is_some());
        let err_val = Value::Result(Err(Box::new(Value::I32(404))));
        assert!(err_val.as_result().is_some());

        // Test Future
        let future = Value::Future(Box::new(Value::I32(42)));
        assert!(future.as_future().is_some());
        assert_eq!(future.as_future().unwrap().as_i32(), Some(42));

        // Test Stream
        let stream = Value::Stream {
            element: Box::new(Value::I32(42)),
            end: Some(Box::new(Value::I32(0))),
        };
        assert!(stream.as_stream().is_some());
        let (element, end) = stream.as_stream().unwrap();
        assert_eq!(element.as_i32(), Some(42));
        assert!(end.is_some());
    }

    #[test]
    fn test_value_display() {
        // Test numeric display
        assert_eq!(Value::I32(42).to_string(), "i32: 42");
        assert_eq!(Value::I64(42).to_string(), "i64: 42");
        assert_eq!(Value::F32(3.14).to_string(), "f32: 3.14");
        assert_eq!(Value::F64(3.14159).to_string(), "f64: 3.14159");

        // Test reference display
        assert_eq!(Value::FuncRef(Some(1)).to_string(), "funcref: 1");
        assert_eq!(Value::ExternRef(None).to_string(), "externref: null");

        // Test component model value display
        let record = Value::Record(vec![("field1".to_string(), Box::new(Value::I32(1)))]);
        assert!(record.to_string().starts_with("record:"));

        let enum_val = Value::Enum("RED".to_string());
        assert_eq!(enum_val.to_string(), "enum: RED");
    }
}
