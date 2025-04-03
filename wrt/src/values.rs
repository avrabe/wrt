use crate::types::ValueType;
use crate::{Box, String, Vec};
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a WebAssembly runtime value
#[derive(Debug, Clone)]
pub enum Value {
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
    /// 128-bit vector
    V128([u8; 16]),
    /// Function reference
    FuncRef(Option<u32>), // Store function index
    /// External reference
    ExternRef(Option<u32>), // Store external reference index
    /// Any reference
    AnyRef(Option<u32>), // Store any reference index
}

// Manual PartialEq implementation for Value
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::I32(a), Value::I32(b)) => a == b,
            (Value::I64(a), Value::I64(b)) => a == b,
            // Handle NaN comparison for floats: NaN != NaN
            (Value::F32(a), Value::F32(b)) => (a.is_nan() && b.is_nan()) || (a == b),
            (Value::F64(a), Value::F64(b)) => (a.is_nan() && b.is_nan()) || (a == b),
            (Value::V128(a), Value::V128(b)) => a == b,
            (Value::FuncRef(a), Value::FuncRef(b)) => a == b,
            (Value::ExternRef(a), Value::ExternRef(b)) => a == b,
            (Value::AnyRef(a), Value::AnyRef(b)) => a == b,
            _ => false, // Different types are not equal
        }
    }
}

impl Eq for Value {}

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
    #[must_use]
    pub const fn default_for_type(ty: &ValueType) -> Self {
        match ty {
            ValueType::I32 => Value::I32(0),
            ValueType::I64 => Value::I64(0),
            ValueType::F32 => Value::F32(0.0),
            ValueType::F64 => Value::F64(0.0),
            ValueType::V128 => Value::V128([0; 16]),
            ValueType::FuncRef => Value::FuncRef(None), // Default for FuncRef is null
            ValueType::ExternRef => Value::ExternRef(None), // Default for ExternRef is null
            ValueType::AnyRef => Value::AnyRef(None),   // Default for AnyRef is null
        }
    }

    /// Returns the WebAssembly type of this value
    ///
    /// # Examples
    ///
    /// ```
    /// use wrt::{Value, ValueType};
    ///
    /// let value = Value::F64(3.14);
    /// assert_eq!(value.type_(), ValueType::F64);
    /// ```
    #[must_use]
    pub const fn type_(&self) -> ValueType {
        match self {
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::F32(_) => ValueType::F32,
            Self::F64(_) => ValueType::F64,
            Self::FuncRef(_) => ValueType::FuncRef,
            Self::ExternRef(_) => ValueType::ExternRef,
            Self::V128(_) => ValueType::V128,
            Self::AnyRef(_) => ValueType::AnyRef,
        }
    }

    /// Checks if this Value matches the specified `ValueType`
    ///
    /// # Returns
    ///
    /// `true` if the value matches the type, `false` otherwise
    #[must_use]
    pub const fn matches_type(&self, ty: &ValueType) -> bool {
        matches!(
            (self, ty),
            (Self::I32(_), ValueType::I32)
                | (Self::I64(_), ValueType::I64)
                | (Self::F32(_), ValueType::F32)
                | (Self::F64(_), ValueType::F64)
                | (Self::FuncRef(_), ValueType::FuncRef)
                | (Self::ExternRef(_), ValueType::ExternRef)
                | (Self::V128(_), ValueType::V128)
                | (Self::AnyRef(_), ValueType::AnyRef)
        )
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
    #[must_use]
    pub const fn as_i32(&self) -> Option<i32> {
        match self {
            Self::I32(v) => Some(*v),
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
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(v) => Some(*v),
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
    #[must_use]
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(v) => Some(*v),
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
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(v) => Some(*v),
            _ => None,
        }
    }

    /// Attempts to extract a function reference if this Value is a `FuncRef`.
    ///
    /// # Returns
    ///
    /// Some(Option<u32>) if this is a `FuncRef` value, None otherwise
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
    #[must_use]
    pub const fn as_func_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::FuncRef(v) => Some(*v),
            _ => None,
        }
    }

    /// Attempts to extract an external reference if this Value is an `ExternRef`.
    ///
    /// # Returns
    ///
    /// Some(Option<u32>) if this is an `ExternRef` value, None otherwise
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
    #[must_use]
    pub const fn as_extern_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::ExternRef(v) => Some(*v),
            _ => None,
        }
    }

    /// Attempts to extract an anyref value if this Value is an `AnyRef`.
    ///
    /// # Returns
    ///
    /// Some reference to the anyref value if this is an `AnyRef` value, None otherwise
    #[must_use]
    pub const fn as_any_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::AnyRef(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the `ValueType` of this Value
    ///
    /// This is used to determine the type of the value for type checking
    /// and validation.
    #[must_use]
    pub const fn get_type(&self) -> ValueType {
        match self {
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::F32(_) => ValueType::F32,
            Self::F64(_) => ValueType::F64,
            Self::V128(_) => ValueType::V128,
            Self::FuncRef(_) => ValueType::FuncRef,
            Self::ExternRef(_) => ValueType::ExternRef,
            Self::AnyRef(_) => ValueType::AnyRef,
        }
    }

    /// Attempts to get the value as a u64
    ///
    /// # Returns
    ///
    /// Some(u64) if the value is an I64 and can be converted to u64,
    /// None otherwise
    #[must_use]
    pub const fn as_u64(&self) -> Option<u64> {
        match self {
            Self::I64(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }

    /// Attempts to get the value as a u32
    ///
    /// # Returns
    ///
    /// Some(u32) if the value is an I32 and can be converted to u32,
    /// None otherwise
    #[must_use]
    pub const fn as_u32(&self) -> Option<u32> {
        match self {
            Self::I32(v) => Some(*v as u32),
            _ => None,
        }
    }

    /// Converts the value to an i32, returning an error if not possible
    pub fn into_i32(self) -> crate::error::Result<i32> {
        match self {
            Self::I32(v) => Ok(v),
            _ => Err(crate::error::Error::TypeMismatch(
                "Expected i32 value".to_string(),
            )),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32(v) => write!(f, "i32: {v}"),
            Self::I64(v) => write!(f, "i64: {v}"),
            Self::F32(v) => write!(f, "f32: {v}"),
            Self::F64(v) => write!(f, "f64: {v}"),
            Self::V128(v) => write!(f, "v128: {v:02x?}"),
            Self::FuncRef(v) => match v {
                Some(idx) => write!(f, "funcref: {idx}"),
                None => write!(f, "funcref: null"),
            },
            Self::ExternRef(v) => match v {
                Some(idx) => write!(f, "externref: {idx}"),
                None => write!(f, "externref: null"),
            },
            Self::AnyRef(v) => write!(f, "anyref: {v:?}"),
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
    fn test_value_display() {
        // Test numeric display
        assert_eq!(Value::I32(42).to_string(), "i32: 42");
        assert_eq!(Value::I64(42).to_string(), "i64: 42");
        assert_eq!(Value::F32(3.14).to_string(), "f32: 3.14");
        assert_eq!(Value::F64(3.14159).to_string(), "f64: 3.14159");

        // Test reference display
        assert_eq!(Value::FuncRef(Some(1)).to_string(), "funcref: 1");
        assert_eq!(Value::ExternRef(None).to_string(), "externref: null");
    }
}
