//! Comparison operations for WebAssembly instructions.
//!
//! This module provides pure implementations for WebAssembly comparison instructions,
//! including equality, inequality, and relational operations for various numeric types.

use crate::prelude::*;

/// Represents a pure comparison operation for WebAssembly.
#[derive(Debug, Clone)]
pub enum ComparisonOp {
    // Integer equality operations (i32)
    /// Equals comparison for i32 values
    I32Eq,
    /// Not equals comparison for i32 values
    I32Ne,
    /// Less than comparison for i32 values (signed)
    I32LtS,
    /// Less than comparison for i32 values (unsigned)
    I32LtU,
    /// Greater than comparison for i32 values (signed)
    I32GtS,
    /// Greater than comparison for i32 values (unsigned)
    I32GtU,
    /// Less than or equal comparison for i32 values (signed)
    I32LeS,
    /// Less than or equal comparison for i32 values (unsigned)
    I32LeU,
    /// Greater than or equal comparison for i32 values (signed)
    I32GeS,
    /// Greater than or equal comparison for i32 values (unsigned)
    I32GeU,

    // Integer equality operations (i64)
    /// Equals comparison for i64 values
    I64Eq,
    /// Not equals comparison for i64 values
    I64Ne,
    /// Less than comparison for i64 values (signed)
    I64LtS,
    /// Less than comparison for i64 values (unsigned)
    I64LtU,
    /// Greater than comparison for i64 values (signed)
    I64GtS,
    /// Greater than comparison for i64 values (unsigned)
    I64GtU,
    /// Less than or equal comparison for i64 values (signed)
    I64LeS,
    /// Less than or equal comparison for i64 values (unsigned)
    I64LeU,
    /// Greater than or equal comparison for i64 values (signed)
    I64GeS,
    /// Greater than or equal comparison for i64 values (unsigned)
    I64GeU,

    // Float comparison operations (f32)
    /// Equals comparison for f32 values
    F32Eq,
    /// Not equals comparison for f32 values
    F32Ne,
    /// Less than comparison for f32 values
    F32Lt,
    /// Greater than comparison for f32 values
    F32Gt,
    /// Less than or equal comparison for f32 values
    F32Le,
    /// Greater than or equal comparison for f32 values
    F32Ge,

    // Float comparison operations (f64)
    /// Equals comparison for f64 values
    F64Eq,
    /// Not equals comparison for f64 values
    F64Ne,
    /// Less than comparison for f64 values
    F64Lt,
    /// Greater than comparison for f64 values
    F64Gt,
    /// Less than or equal comparison for f64 values
    F64Le,
    /// Greater than or equal comparison for f64 values
    F64Ge,
}

/// Execution context for comparison operations
pub trait ComparisonContext {
    /// Pop a value from the context
    fn pop_comparison_value(&mut self) -> Result<Value>;

    /// Push a value to the context
    fn push_comparison_value(&mut self, value: Value) -> Result<()>;
}

impl<T: ComparisonContext> PureInstruction<T, Error> for ComparisonOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            // i32 equality operations
            Self::I32Eq => {
                let b = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.eq operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.eq operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a == b { 1 } else { 0 }))
            }
            Self::I32Ne => {
                let b = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.ne operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.ne operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a != b { 1 } else { 0 }))
            }
            Self::I32LtS => {
                let b = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.lt_s operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.lt_s operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a < b { 1 } else { 0 }))
            }
            Self::I32LtU => {
                let b = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.lt_u operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.lt_u operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a < b { 1 } else { 0 }))
            }
            Self::I32GtS => {
                let b = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.gt_s operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.gt_s operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a > b { 1 } else { 0 }))
            }
            Self::I32GtU => {
                let b = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.gt_u operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.gt_u operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a > b { 1 } else { 0 }))
            }
            Self::I32LeS => {
                let b = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.le_s operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.le_s operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a <= b { 1 } else { 0 }))
            }
            Self::I32LeU => {
                let b = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.le_u operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.le_u operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a <= b { 1 } else { 0 }))
            }
            Self::I32GeS => {
                let b = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.ge_s operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.ge_s operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a >= b { 1 } else { 0 }))
            }
            Self::I32GeU => {
                let b = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.ge_u operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_u32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for i32.ge_u operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a >= b { 1 } else { 0 }))
            }

            // i64 equality operations
            Self::I64Eq => {
                let b = context.pop_comparison_value()?.as_i64().ok_or_else(|| {
                    Error::invalid_type("Expected I64 for i64.eq operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i64().ok_or_else(|| {
                    Error::invalid_type("Expected I64 for i64.eq operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a == b { 1 } else { 0 }))
            }
            Self::I64Ne => {
                let b = context.pop_comparison_value()?.as_i64().ok_or_else(|| {
                    Error::invalid_type("Expected I64 for i64.ne operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_i64().ok_or_else(|| {
                    Error::invalid_type("Expected I64 for i64.ne operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a != b { 1 } else { 0 }))
            }

            // f32 comparison operations
            Self::F32Eq => {
                let b = context.pop_comparison_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f32.eq operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f32.eq operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a == b { 1 } else { 0 }))
            }
            Self::F32Ne => {
                let b = context.pop_comparison_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f32.ne operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f32.ne operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a != b { 1 } else { 0 }))
            }
            Self::F32Lt => {
                let b = context.pop_comparison_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f32.lt operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_f32().ok_or_else(|| {
                    Error::invalid_type("Expected F32 for f32.lt operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a < b { 1 } else { 0 }))
            }

            // f64 comparison operations
            Self::F64Eq => {
                let b = context.pop_comparison_value()?.as_f64().ok_or_else(|| {
                    Error::invalid_type("Expected F64 for f64.eq operand".to_string())
                })?;
                let a = context.pop_comparison_value()?.as_f64().ok_or_else(|| {
                    Error::invalid_type("Expected F64 for f64.eq operand".to_string())
                })?;
                context.push_comparison_value(Value::I32(if a == b { 1 } else { 0 }))
            }

            // Return Ok for unimplemented operations (to be completed)
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ExecutionContext;

    #[test]
    fn test_i32_equality() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i32.eq (equal)
        context.push_comparison_value(Value::I32(5)).unwrap();
        context.push_comparison_value(Value::I32(5)).unwrap();
        ComparisonOp::I32Eq.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test i32.eq (not equal)
        context.push_comparison_value(Value::I32(5)).unwrap();
        context.push_comparison_value(Value::I32(7)).unwrap();
        ComparisonOp::I32Eq.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(0));

        // Test i32.ne (not equal)
        context.push_comparison_value(Value::I32(5)).unwrap();
        context.push_comparison_value(Value::I32(7)).unwrap();
        ComparisonOp::I32Ne.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test i32.ne (equal)
        context.push_comparison_value(Value::I32(5)).unwrap();
        context.push_comparison_value(Value::I32(5)).unwrap();
        ComparisonOp::I32Ne.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(0));
    }

    #[test]
    fn test_i32_relational() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i32.lt_s (less than, signed)
        context.push_comparison_value(Value::I32(-5)).unwrap();
        context.push_comparison_value(Value::I32(7)).unwrap();
        ComparisonOp::I32LtS.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test i32.lt_u (less than, unsigned)
        // Note: -5 as unsigned is a large positive number
        context.push_comparison_value(Value::I32(-5)).unwrap();
        context.push_comparison_value(Value::I32(7)).unwrap();
        ComparisonOp::I32LtU.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(0));

        // Test i32.gt_s (greater than, signed)
        context.push_comparison_value(Value::I32(10)).unwrap();
        context.push_comparison_value(Value::I32(7)).unwrap();
        ComparisonOp::I32GtS.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));
    }

    #[test]
    fn test_i64_equality() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i64.eq (equal)
        context.push_comparison_value(Value::I64(5)).unwrap();
        context.push_comparison_value(Value::I64(5)).unwrap();
        ComparisonOp::I64Eq.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test i64.ne (not equal)
        context.push_comparison_value(Value::I64(5)).unwrap();
        context.push_comparison_value(Value::I64(7)).unwrap();
        ComparisonOp::I64Ne.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));
    }

    #[test]
    fn test_float_equality() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test f32.eq (equal)
        context.push_comparison_value(Value::F32(5.0)).unwrap();
        context.push_comparison_value(Value::F32(5.0)).unwrap();
        ComparisonOp::F32Eq.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test f32.ne (not equal)
        context.push_comparison_value(Value::F32(5.0)).unwrap();
        context.push_comparison_value(Value::F32(7.0)).unwrap();
        ComparisonOp::F32Ne.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test f32.lt (less than)
        context.push_comparison_value(Value::F32(5.0)).unwrap();
        context.push_comparison_value(Value::F32(7.0)).unwrap();
        ComparisonOp::F32Lt.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));

        // Test f64.eq (equal)
        context.push_comparison_value(Value::F64(5.0)).unwrap();
        context.push_comparison_value(Value::F64(5.0)).unwrap();
        ComparisonOp::F64Eq.execute(&mut context).unwrap();
        assert_eq!(context.pop_comparison_value().unwrap(), Value::I32(1));
    }
}
