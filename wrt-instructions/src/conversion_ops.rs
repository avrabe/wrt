//! Conversion operations for WebAssembly instructions.
//!
//! This module provides pure implementations for WebAssembly conversion instructions,
//! including type conversions between numeric types.

use crate::prelude::*;

/// Represents a pure conversion operation for WebAssembly.
#[derive(Debug, Clone)]
pub enum ConversionOp {
    // i32 conversions
    /// Convert i64 to i32 (truncate)
    I32WrapI64,
    /// Convert f32 to i32 (signed, truncate)
    I32TruncF32S,
    /// Convert f32 to i32 (unsigned, truncate)
    I32TruncF32U,
    /// Convert f64 to i32 (signed, truncate)
    I32TruncF64S,
    /// Convert f64 to i32 (unsigned, truncate)
    I32TruncF64U,
    /// Convert i32 to f32 (reinterpret bits)
    I32ReinterpretF32,
    // Wasm 2.0: Sign-extension operators for i32
    /// Sign-extend 8-bit integer to 32-bit integer
    I32Extend8S,
    /// Sign-extend 16-bit integer to 32-bit integer
    I32Extend16S,

    // i64 conversions
    /// Sign-extend i32 to i64
    I64ExtendI32S,
    /// Zero-extend i32 to i64
    I64ExtendI32U,
    /// Convert f32 to i64 (signed, truncate)
    I64TruncF32S,
    /// Convert f32 to i64 (unsigned, truncate)
    I64TruncF32U,
    /// Convert f64 to i64 (signed, truncate)
    I64TruncF64S,
    /// Convert f64 to i64 (unsigned, truncate)
    I64TruncF64U,
    /// Convert i64 to f64 (reinterpret bits)
    I64ReinterpretF64,
    // Wasm 2.0: Sign-extension operators for i64
    /// Sign-extend 8-bit integer to 64-bit integer
    I64Extend8S,
    /// Sign-extend 16-bit integer to 64-bit integer
    I64Extend16S,
    /// Sign-extend 32-bit integer to 64-bit integer
    I64Extend32S,

    // f32 conversions
    /// Convert i32 to f32 (signed)
    F32ConvertI32S,
    /// Convert i32 to f32 (unsigned)
    F32ConvertI32U,
    /// Convert i64 to f32 (signed)
    F32ConvertI64S,
    /// Convert i64 to f32 (unsigned)
    F32ConvertI64U,
    /// Demote f64 to f32
    F32DemoteF64,
    /// Reinterpret i32 bits as f32
    F32ReinterpretI32,

    // f64 conversions
    /// Convert i32 to f64 (signed)
    F64ConvertI32S,
    /// Convert i32 to f64 (unsigned)
    F64ConvertI32U,
    /// Convert i64 to f64 (signed)
    F64ConvertI64S,
    /// Convert i64 to f64 (unsigned)
    F64ConvertI64U,
    /// Promote f32 to f64
    F64PromoteF32,
    /// Reinterpret i64 bits as f64
    F64ReinterpretI64,

    // Wasm 2.0: Non-trapping float-to-int conversions
    /// Convert f32 to i32 (signed, saturate)
    I32TruncSatF32S,
    /// Convert f32 to i32 (unsigned, saturate)
    I32TruncSatF32U,
    /// Convert f64 to i32 (signed, saturate)
    I32TruncSatF64S,
    /// Convert f64 to i32 (unsigned, saturate)
    I32TruncSatF64U,
    /// Convert f32 to i64 (signed, saturate)
    I64TruncSatF32S,
    /// Convert f32 to i64 (unsigned, saturate)
    I64TruncSatF32U,
    /// Convert f64 to i64 (signed, saturate)
    I64TruncSatF64S,
    /// Convert f64 to i64 (unsigned, saturate)
    I64TruncSatF64U,
}

/// Execution context for conversion operations
pub trait ConversionContext {
    /// Pop a value from the context
    fn pop_conversion_value(&mut self) -> Result<Value>;

    /// Push a value to the context
    fn push_conversion_value(&mut self, value: Value) -> Result<()>;
}

impl<T: ConversionContext> PureInstruction<T, Error> for ConversionOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            // i32 conversions
            Self::I32WrapI64 => {
                let a = context.pop_conversion_value()?.as_i64().ok_or_else(|| {
                    Error::invalid_type("Expected I64 for i32.wrap_i64 operand".to_string())
                })?;
                context.push_conversion_value(Value::I32(a as i32))
            }
            Self::I32TruncF32S => {
                let a = context.pop_conversion_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for i32.trunc_f32_s operand".to_string())
                })?;

                if a.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }

                if a >= (i32::MAX as f32) + 1.0 || a < (i32::MIN as f32) {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }

                context.push_conversion_value(Value::I32(a as i32))
            }
            Self::I32TruncF32U => {
                let a = context.pop_conversion_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for i32.trunc_f32_u operand".to_string())
                })?;

                if a.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }

                if a >= (u32::MAX as f32) + 1.0 || a < 0.0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }

                context.push_conversion_value(Value::I32(a as u32 as i32))
            }
            Self::I32ReinterpretF32 => {
                let a = context.pop_conversion_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for i32.reinterpret_f32 operand".to_string())
                })?;

                let bits = a.to_bits() as i32;
                context.push_conversion_value(Value::I32(bits))
            }

            // i64 conversions
            Self::I64ExtendI32S => {
                let a = context.pop_conversion_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i64.extend_i32_s operand".to_string())
                })?;
                context.push_conversion_value(Value::I64(a as i64))
            }
            Self::I64ExtendI32U => {
                let a = context.pop_conversion_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i64.extend_i32_u operand".to_string())
                })?;
                context.push_conversion_value(Value::I64(a as i64))
            }

            // f32 conversions
            Self::F32ConvertI32S => {
                let a = context.pop_conversion_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for f32.convert_i32_s operand".to_string())
                })?;
                context.push_conversion_value(Value::F32(a as f32))
            }
            Self::F32ConvertI32U => {
                let a = context.pop_conversion_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for f32.convert_i32_u operand".to_string())
                })?;
                context.push_conversion_value(Value::F32(a as f32))
            }
            Self::F32ReinterpretI32 => {
                let a = context.pop_conversion_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for f32.reinterpret_i32 operand".to_string())
                })?;

                let float = f32::from_bits(a as u32);
                context.push_conversion_value(Value::F32(float))
            }

            // f64 conversions
            Self::F64ConvertI32S => {
                let a = context.pop_conversion_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for f64.convert_i32_s operand".to_string())
                })?;
                context.push_conversion_value(Value::F64(a as f64))
            }
            Self::F64ConvertI32U => {
                let a = context.pop_conversion_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for f64.convert_i32_u operand".to_string())
                })?;
                context.push_conversion_value(Value::F64(a as f64))
            }
            Self::F64PromoteF32 => {
                let a = context.pop_conversion_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f64.promote_f32 operand".to_string())
                })?;
                context.push_conversion_value(Value::F64(a as f64))
            }
            Self::I32TruncF64S => {
                let a = context.pop_conversion_value()?.as_f64().ok_or_else(|| {
                    Error::invalid_type("Expected F64 for i32.trunc_f64_s operand".to_string())
                })?;

                if a.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }

                if a >= (i32::MAX as f64) + 1.0 || a < (i32::MIN as f64) {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }

                context.push_conversion_value(Value::I32(a as i32))
            }

            // Return Ok for unimplemented operations (to be completed)
            _ => Ok(()),
        }
    }
}

/// I32WrapI64 conversion operation
pub struct I32WrapI64(pub Value);

/// I64ExtendI32S conversion operation
pub struct I64ExtendI32S(pub Value);

/// I64ExtendI32U conversion operation
pub struct I64ExtendI32U(pub Value);

/// I64TruncF32S conversion operation
pub struct I64TruncF32S(pub Value);

/// I64TruncF32U conversion operation
pub struct I64TruncF32U(pub Value);

/// I64TruncF64S conversion operation
pub struct I64TruncF64S(pub Value);

/// I64TruncF64U conversion operation
pub struct I64TruncF64U(pub Value);

/// I64ReinterpretF64 conversion operation
pub struct I64ReinterpretF64(pub Value);

impl TryInto<Value> for I32WrapI64 {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::I64(val) => Ok(Value::I32((val & 0xFFFFFFFF) as i32)),
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected I64, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64ExtendI32S {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::I32(val) => Ok(Value::I64(val as i64)),
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected I32, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64ExtendI32U {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::I32(val) => {
                // Convert to u32 to ensure proper unsigned semantics
                let val_u32 = val as u32;
                // Note: This is a direct conversion, no need to check for overflow
                // since u32::MAX cannot overflow u32
                Ok(Value::I64(val_u32 as i64))
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected I32, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64TruncF32S {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::F32(val) => {
                if val.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }
                if val >= (i64::MAX as f32) + 1.0 || val < (i64::MIN as f32) {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }
                Ok(Value::I64(val as i64))
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected F32, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64TruncF32U {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::F32(val) => {
                if val.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }
                if val >= (u64::MAX as f32) + 1.0 || val < 0.0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }
                Ok(Value::I64(val as u64 as i64))
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected F32, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64TruncF64S {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::F64(val) => {
                if val.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }
                if val >= (i64::MAX as f64) + 1.0 || val < (i64::MIN as f64) {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }
                Ok(Value::I64(val as i64))
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected F64, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64TruncF64U {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::F64(val) => {
                if val.is_nan() {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Type,
                        wrt_error::codes::CONVERSION_ERROR,
                        "NaN cannot be converted to integer",
                    ));
                }
                if val >= (u64::MAX as f64) + 1.0 || val < 0.0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }
                Ok(Value::I64(val as u64 as i64))
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected F64, got {:?}", self.0),
            )),
        }
    }
}

impl TryInto<Value> for I64ReinterpretF64 {
    type Error = Error;

    fn try_into(self) -> Result<Value> {
        match self.0 {
            Value::F64(val) => {
                let bits = val.to_bits();
                Ok(Value::I64(bits as i64))
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::CONVERSION_ERROR,
                format!("Expected F64, got {:?}", self.0),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Create a mock context for testing
    struct MockExecutionContext {
        stack: Vec<Value>,
    }

    impl MockExecutionContext {
        fn new() -> Self {
            Self { stack: Vec::new() }
        }
    }

    impl ConversionContext for MockExecutionContext {
        fn pop_conversion_value(&mut self) -> Result<Value> {
            self.stack.pop().ok_or_else(|| {
                Error::from(wrt_error::Error::new(
                    wrt_error::ErrorCategory::Core,
                    wrt_error::codes::STACK_UNDERFLOW,
                    "Stack underflow",
                ))
            })
        }

        fn push_conversion_value(&mut self, value: Value) -> Result<()> {
            self.stack.push(value);
            Ok(())
        }
    }

    #[test]
    fn test_i32_wrap_i64() {
        let mut context = MockExecutionContext::new();
        context.push_conversion_value(Value::I64(0x1_0000_0000)).unwrap();
        ConversionOp::I32WrapI64.execute(&mut context).unwrap();
        assert_eq!(context.pop_conversion_value().unwrap(), Value::I32(0));
    }

    #[test]
    fn test_i32_trunc_f32_s() {
        let mut context = MockExecutionContext::new();
        context.push_conversion_value(Value::F32(-123.45)).unwrap();
        ConversionOp::I32TruncF32S.execute(&mut context).unwrap();
        assert_eq!(context.pop_conversion_value().unwrap(), Value::I32(-123));
    }

    #[test]
    fn test_i32_trunc_f32_u() {
        let mut context = MockExecutionContext::new();
        context.push_conversion_value(Value::F32(123.45)).unwrap();
        ConversionOp::I32TruncF32U.execute(&mut context).unwrap();
        assert_eq!(context.pop_conversion_value().unwrap(), Value::I32(123));
    }

    #[test]
    fn test_i32_trunc_f64_s() {
        let mut context = MockExecutionContext::new();
        context.push_conversion_value(Value::F64(-123.45)).unwrap();
        ConversionOp::I32TruncF64S.execute(&mut context).unwrap();
        assert_eq!(context.pop_conversion_value().unwrap(), Value::I32(-123));
    }

    // More tests can be added as needed
}
