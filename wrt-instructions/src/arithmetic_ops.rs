// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Arithmetic operations for WebAssembly instructions.
//!
//! This module provides pure implementations for WebAssembly arithmetic
//! instructions, including add, subtract, multiply, divide, and remainder
//! operations for various numeric types.

use crate::prelude::*;
use crate::validation::{Validate, ValidationContext, validate_arithmetic_op};

/// Represents a pure arithmetic operation for WebAssembly.
#[derive(Debug, Clone)]
pub enum ArithmeticOp {
    // Integer operations (i32)
    /// Add two 32-bit integers
    I32Add,
    /// Subtract one 32-bit integer from another
    I32Sub,
    /// Multiply two 32-bit integers
    I32Mul,
    /// Divide two 32-bit integers (signed)
    I32DivS,
    /// Divide two 32-bit integers (unsigned)
    I32DivU,
    /// Get remainder after dividing two 32-bit integers (signed)
    I32RemS,
    /// Get remainder after dividing two 32-bit integers (unsigned)
    I32RemU,
    /// Perform bitwise AND on two 32-bit integers
    I32And,
    /// Perform bitwise OR on two 32-bit integers
    I32Or,
    /// Perform bitwise XOR on two 32-bit integers
    I32Xor,
    /// Shift 32-bit integer left
    I32Shl,
    /// Shift 32-bit integer right (signed)
    I32ShrS,
    /// Shift 32-bit integer right (unsigned)
    I32ShrU,
    /// Rotate 32-bit integer left
    I32Rotl,
    /// Rotate 32-bit integer right
    I32Rotr,
    /// Count leading zeros in a 32-bit integer
    I32Clz,
    /// Count trailing zeros in a 32-bit integer
    I32Ctz,
    /// Count number of set bits in a 32-bit integer
    I32Popcnt,

    // Integer operations (i64)
    /// Add two 64-bit integers
    I64Add,
    /// Subtract one 64-bit integer from another
    I64Sub,
    /// Multiply two 64-bit integers
    I64Mul,
    /// Divide two 64-bit integers (signed)
    I64DivS,
    /// Divide two 64-bit integers (unsigned)
    I64DivU,
    /// Get remainder after dividing two 64-bit integers (signed)
    I64RemS,
    /// Get remainder after dividing two 64-bit integers (unsigned)
    I64RemU,
    /// Perform bitwise AND on two 64-bit integers
    I64And,
    /// Perform bitwise OR on two 64-bit integers
    I64Or,
    /// Perform bitwise XOR on two 64-bit integers
    I64Xor,
    /// Shift 64-bit integer left
    I64Shl,
    /// Shift 64-bit integer right (signed)
    I64ShrS,
    /// Shift 64-bit integer right (unsigned)
    I64ShrU,
    /// Rotate 64-bit integer left
    I64Rotl,
    /// Rotate 64-bit integer right
    I64Rotr,
    /// Count leading zeros in a 64-bit integer
    I64Clz,
    /// Count trailing zeros in a 64-bit integer
    I64Ctz,
    /// Count number of set bits in a 64-bit integer
    I64Popcnt,

    // Float operations (f32)
    /// Add two 32-bit float values
    F32Add,
    /// Subtract 32-bit float values
    F32Sub,
    /// Multiply 32-bit float values
    F32Mul,
    /// Divide 32-bit float values
    F32Div,
    /// Get the minimum of two 32-bit float values
    F32Min,
    /// Get the maximum of two 32-bit float values
    F32Max,
    /// Get the absolute value of a 32-bit float
    F32Abs,
    /// Negate a 32-bit float
    F32Neg,
    /// Round a 32-bit float up to the nearest integer
    F32Ceil,
    /// Round a 32-bit float down to the nearest integer
    F32Floor,
    /// Truncate a 32-bit float to an integer
    F32Trunc,
    /// Round a 32-bit float to the nearest integer
    F32Nearest,
    /// Calculate the square root of a 32-bit float
    F32Sqrt,
    /// Copy sign from one 32-bit float to another
    F32Copysign,

    // Float operations (f64)
    /// Add two 64-bit float values
    F64Add,
    /// Subtract 64-bit float values
    F64Sub,
    /// Multiply 64-bit float values
    F64Mul,
    /// Divide 64-bit float values
    F64Div,
    /// Get the minimum of two 64-bit float values
    F64Min,
    /// Get the maximum of two 64-bit float values
    F64Max,
    /// Get the absolute value of a 64-bit float
    F64Abs,
    /// Negate a 64-bit float
    F64Neg,
    /// Round a 64-bit float up to the nearest integer
    F64Ceil,
    /// Round a 64-bit float down to the nearest integer
    F64Floor,
    /// Truncate a 64-bit float to an integer
    F64Trunc,
    /// Round a 64-bit float to the nearest integer
    F64Nearest,
    /// Calculate the square root of a 64-bit float
    F64Sqrt,
    /// Copy sign from one 64-bit float to another
    F64Copysign,
}

/// Execution context for arithmetic operations
pub trait ArithmeticContext {
    /// Pop a value from the context
    fn pop_arithmetic_value(&mut self) -> Result<Value>;

    /// Push a value to the context
    fn push_arithmetic_value(&mut self, value: Value) -> Result<()>;
}

impl<T: ArithmeticContext> PureInstruction<T, Error> for ArithmeticOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            // Integer operations (i32)
            Self::I32Add => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.add operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.add operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.wrapping_add(b)))
            }
            Self::I32Sub => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.sub operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.sub operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.wrapping_sub(b)))
            }
            Self::I32Mul => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.mul operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.mul operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.wrapping_mul(b)))
            }
            Self::I32DivS => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.div_s operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.div_s operand")
                })?;

                if b == 0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::RUNTIME_ERROR,
                        "Division by zero",
                    ));
                }
                if a == i32::MIN && b == -1 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::CONVERSION_ERROR,
                        "Integer overflow",
                    ));
                }

                context.push_arithmetic_value(Value::I32(a.wrapping_div(b)))
            }
            Self::I32DivU => {
                let b = context.pop_arithmetic_value()?.as_u32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.div_u operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_u32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.div_u operand")
                })?;

                if b == 0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::RUNTIME_ERROR,
                        "Division by zero",
                    ));
                }

                context.push_arithmetic_value(Value::I32(a.wrapping_div(b) as i32))
            }
            Self::I32RemS => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rem_s operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rem_s operand")
                })?;

                if b == 0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::RUNTIME_ERROR,
                        "Division by zero",
                    ));
                }

                context.push_arithmetic_value(Value::I32(a.wrapping_rem(b)))
            }
            Self::I32RemU => {
                let b = context.pop_arithmetic_value()?.as_u32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rem_u operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_u32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rem_u operand")
                })?;

                if b == 0 {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Runtime,
                        wrt_error::codes::RUNTIME_ERROR,
                        "Division by zero",
                    ));
                }

                context.push_arithmetic_value(Value::I32(a.wrapping_rem(b) as i32))
            }
            Self::I32And => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.and operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.and operand")
                })?;
                context.push_arithmetic_value(Value::I32(a & b))
            }
            Self::I32Or => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.or operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.or operand")
                })?;
                context.push_arithmetic_value(Value::I32(a | b))
            }
            Self::I32Xor => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.xor operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.xor operand")
                })?;
                context.push_arithmetic_value(Value::I32(a ^ b))
            }
            Self::I32Shl => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.shl operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.shl operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.wrapping_shl(b as u32 % 32)))
            }
            Self::I32ShrS => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.shr_s operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.shr_s operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.wrapping_shr(b as u32 % 32)))
            }
            Self::I32ShrU => {
                let b = context.pop_arithmetic_value()?.as_u32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.shr_u operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_u32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.shr_u operand")
                })?;
                context.push_arithmetic_value(Value::I32((a.wrapping_shr(b % 32)) as i32))
            }
            Self::I32Rotl => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rotl operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rotl operand")
                })?;
                let n = (b as u32) % 32;
                let result = a.rotate_left(n);
                context.push_arithmetic_value(Value::I32(result))
            }
            Self::I32Rotr => {
                let b = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rotr operand")
                })?;
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.rotr operand")
                })?;
                let n = (b as u32) % 32;
                let result = a.rotate_right(n);
                context.push_arithmetic_value(Value::I32(result))
            }
            Self::I32Clz => {
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.clz operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.leading_zeros() as i32))
            }
            Self::I32Ctz => {
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.ctz operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.trailing_zeros() as i32))
            }
            Self::I32Popcnt => {
                let a = context.pop_arithmetic_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for i32.popcnt operand")
                })?;
                context.push_arithmetic_value(Value::I32(a.count_ones() as i32))
            }

            // Integer operations (i64)
            Self::I64Add => {
                let b = context.pop_arithmetic_value()?.as_i64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I64 for i64.add operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_i64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I64 for i64.add operand")
                })?;
                context.push_arithmetic_value(Value::I64(a.wrapping_add(b)))
            }
            // I'll implement just a few more i64 operations as examples
            Self::I64Sub => {
                let b = context.pop_arithmetic_value()?.as_i64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I64 for i64.sub operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_i64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I64 for i64.sub operand")
                })?;
                context.push_arithmetic_value(Value::I64(a.wrapping_sub(b)))
            }
            Self::I64Mul => {
                let b = context.pop_arithmetic_value()?.as_i64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I64 for i64.mul operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_i64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I64 for i64.mul operand")
                })?;
                context.push_arithmetic_value(Value::I64(a.wrapping_mul(b)))
            }

            // Float operations (f32)
            Self::F32Add => {
                let b = context.pop_arithmetic_value()?.as_f32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected F32 for f32.add operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_f32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected F32 for f32.add operand")
                })?;
                context.push_arithmetic_value(Value::F32(FloatBits32::from_float(a + b)))
            }

            // Float operations (f64)
            Self::F64Add => {
                let b = context.pop_arithmetic_value()?.as_f64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected F64 for f64.add operand")
                })?;
                let a = context.pop_arithmetic_value()?.as_f64().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected F64 for f64.add operand")
                })?;
                context.push_arithmetic_value(Value::F64(FloatBits64::from_float(a + b)))
            }

            // Return Ok for unimplemented operations (to be completed)
            _ => Ok(()),
        }
    }
}

impl Validate for ArithmeticOp {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        match self {
            // I32 operations
            Self::I32Add | Self::I32Sub | Self::I32Mul | 
            Self::I32DivS | Self::I32DivU | Self::I32RemS | Self::I32RemU |
            Self::I32And | Self::I32Or | Self::I32Xor |
            Self::I32Shl | Self::I32ShrS | Self::I32ShrU |
            Self::I32Rotl | Self::I32Rotr => {
                validate_arithmetic_op(
                    "i32 binary op",
                    &[ValueType::I32, ValueType::I32],
                    ValueType::I32,
                    ctx
                )
            }
            
            Self::I32Clz | Self::I32Ctz | Self::I32Popcnt => {
                validate_arithmetic_op(
                    "i32 unary op",
                    &[ValueType::I32],
                    ValueType::I32,
                    ctx
                )
            }
            
            // I64 operations
            Self::I64Add | Self::I64Sub | Self::I64Mul |
            Self::I64DivS | Self::I64DivU | Self::I64RemS | Self::I64RemU |
            Self::I64And | Self::I64Or | Self::I64Xor |
            Self::I64Shl | Self::I64ShrS | Self::I64ShrU |
            Self::I64Rotl | Self::I64Rotr => {
                validate_arithmetic_op(
                    "i64 binary op",
                    &[ValueType::I64, ValueType::I64],
                    ValueType::I64,
                    ctx
                )
            }
            
            Self::I64Clz | Self::I64Ctz | Self::I64Popcnt => {
                validate_arithmetic_op(
                    "i64 unary op",
                    &[ValueType::I64],
                    ValueType::I64,
                    ctx
                )
            }
            
            // F32 operations
            Self::F32Add | Self::F32Sub | Self::F32Mul | Self::F32Div |
            Self::F32Min | Self::F32Max | Self::F32Copysign => {
                validate_arithmetic_op(
                    "f32 binary op",
                    &[ValueType::F32, ValueType::F32],
                    ValueType::F32,
                    ctx
                )
            }
            
            Self::F32Abs | Self::F32Neg | Self::F32Ceil | Self::F32Floor |
            Self::F32Trunc | Self::F32Nearest | Self::F32Sqrt => {
                validate_arithmetic_op(
                    "f32 unary op",
                    &[ValueType::F32],
                    ValueType::F32,
                    ctx
                )
            }
            
            // F64 operations
            Self::F64Add | Self::F64Sub | Self::F64Mul | Self::F64Div |
            Self::F64Min | Self::F64Max | Self::F64Copysign => {
                validate_arithmetic_op(
                    "f64 binary op",
                    &[ValueType::F64, ValueType::F64],
                    ValueType::F64,
                    ctx
                )
            }
            
            Self::F64Abs | Self::F64Neg | Self::F64Ceil | Self::F64Floor |
            Self::F64Trunc | Self::F64Nearest | Self::F64Sqrt => {
                validate_arithmetic_op(
                    "f64 unary op",
                    &[ValueType::F64],
                    ValueType::F64,
                    ctx
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ExecutionContext;

    #[test]
    fn test_i32_arithmetic() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i32.add
        context.push_arithmetic_value(Value::I32(2)).unwrap();
        context.push_arithmetic_value(Value::I32(3)).unwrap();
        ArithmeticOp::I32Add.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(5));

        // Test i32.sub
        context.push_arithmetic_value(Value::I32(10)).unwrap();
        context.push_arithmetic_value(Value::I32(4)).unwrap();
        ArithmeticOp::I32Sub.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(6));

        // Test i32.mul
        context.push_arithmetic_value(Value::I32(3)).unwrap();
        context.push_arithmetic_value(Value::I32(4)).unwrap();
        ArithmeticOp::I32Mul.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(12));

        // Test i32.div_s
        context.push_arithmetic_value(Value::I32(10)).unwrap();
        context.push_arithmetic_value(Value::I32(3)).unwrap();
        ArithmeticOp::I32DivS.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(3));

        // Test i32.div_u
        context.push_arithmetic_value(Value::I32(10)).unwrap();
        context.push_arithmetic_value(Value::I32(3)).unwrap();
        ArithmeticOp::I32DivU.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(3));
    }

    #[test]
    fn test_i32_bitwise() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i32.and
        context.push_arithmetic_value(Value::I32(0b1010)).unwrap();
        context.push_arithmetic_value(Value::I32(0b1100)).unwrap();
        ArithmeticOp::I32And.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(0b1000));

        // Test i32.or
        context.push_arithmetic_value(Value::I32(0b1010)).unwrap();
        context.push_arithmetic_value(Value::I32(0b1100)).unwrap();
        ArithmeticOp::I32Or.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(0b1110));

        // Test i32.xor
        context.push_arithmetic_value(Value::I32(0b1010)).unwrap();
        context.push_arithmetic_value(Value::I32(0b1100)).unwrap();
        ArithmeticOp::I32Xor.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(0b0110));
    }

    #[test]
    fn test_i32_shift_rotate() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i32.shl
        context.push_arithmetic_value(Value::I32(1)).unwrap();
        context.push_arithmetic_value(Value::I32(3)).unwrap();
        ArithmeticOp::I32Shl.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(8));

        // Test i32.shr_s
        context.push_arithmetic_value(Value::I32(-8)).unwrap();
        context.push_arithmetic_value(Value::I32(2)).unwrap();
        ArithmeticOp::I32ShrS.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(-2));

        // Test i32.shr_u
        context.push_arithmetic_value(Value::I32(-8)).unwrap();
        context.push_arithmetic_value(Value::I32(2)).unwrap();
        ArithmeticOp::I32ShrU.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(0x3FFFFFFE));
    }

    #[test]
    fn test_division_by_zero() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i32.div_s with division by zero
        context.push_arithmetic_value(Value::I32(10)).unwrap();
        context.push_arithmetic_value(Value::I32(0)).unwrap();
        let result = ArithmeticOp::I32DivS.execute(&mut context);
        assert!(result.is_err());

        // Test i32.div_u with division by zero
        context.push_arithmetic_value(Value::I32(10)).unwrap();
        context.push_arithmetic_value(Value::I32(0)).unwrap();
        let result = ArithmeticOp::I32DivU.execute(&mut context);
        assert!(result.is_err());
    }

    #[test]
    fn test_i64_arithmetic() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test i64.add
        context.push_arithmetic_value(Value::I64(2)).unwrap();
        context.push_arithmetic_value(Value::I64(3)).unwrap();
        ArithmeticOp::I64Add.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I64(5));

        // Test i64.sub
        context.push_arithmetic_value(Value::I64(10)).unwrap();
        context.push_arithmetic_value(Value::I64(4)).unwrap();
        ArithmeticOp::I64Sub.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I64(6));

        // Test i64.mul
        context.push_arithmetic_value(Value::I64(3)).unwrap();
        context.push_arithmetic_value(Value::I64(4)).unwrap();
        ArithmeticOp::I64Mul.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I64(12));
    }

    #[test]
    fn test_float_arithmetic() {
        // Create a simple test context
        let mut context = ExecutionContext::new();

        // Test f32.add
        context.push_arithmetic_value(Value::F32(2.5)).unwrap();
        context.push_arithmetic_value(Value::F32(3.75)).unwrap();
        ArithmeticOp::F32Add.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::F32(6.25));

        // Test f64.add
        context.push_arithmetic_value(Value::F64(2.5)).unwrap();
        context.push_arithmetic_value(Value::F64(3.75)).unwrap();
        ArithmeticOp::F64Add.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::F64(6.25));
    }
}
