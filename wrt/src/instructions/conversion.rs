//! WebAssembly conversion instructions
//!
//! This module contains implementations for all WebAssembly conversion instructions,
//! including type conversion operations between different numeric types.

use crate::error::Error;
use crate::values::Value;
use crate::Vec;

/// Execute an i32.wrap_i64 instruction
///
/// Wraps an i64 value to i32 by truncating to the lower 32 bits.
pub fn i32_wrap_i64(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i64 for i32.wrap_i64".into()));
    };

    stack.push(Value::I32(value as i32));
    Ok(())
}

/// Execute an i64.extend_i32_s instruction
///
/// Extends an i32 value to i64 with sign extension.
pub fn i64_extend_i32_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for i64.extend_i32_s".into()));
    };

    stack.push(Value::I64(value as i64));
    Ok(())
}

/// Execute an i64.extend_i32_u instruction
///
/// Extends an i32 value to i64 with zero extension.
pub fn i64_extend_i32_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for i64.extend_i32_u".into()));
    };

    stack.push(Value::I64(value as u32 as i64));
    Ok(())
}

/// Execute an f32.convert_i32_s instruction
///
/// Converts an i32 value to f32 with signed conversion.
pub fn f32_convert_i32_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for f32.convert_i32_s".into(),
        ));
    };

    stack.push(Value::F32(value as f32));
    Ok(())
}

/// Execute an f32.convert_i32_u instruction
///
/// Converts an i32 value to f32 with unsigned conversion.
pub fn f32_convert_i32_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for f32.convert_i32_u".into(),
        ));
    };

    stack.push(Value::F32(value as u32 as f32));
    Ok(())
}

/// Execute an f32.convert_i64_s instruction
///
/// Converts an i64 value to f32 with signed conversion.
pub fn f32_convert_i64_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i64 for f32.convert_i64_s".into(),
        ));
    };

    stack.push(Value::F32(value as f32));
    Ok(())
}

/// Execute an f32.convert_i64_u instruction
///
/// Converts an i64 value to f32 with unsigned conversion.
pub fn f32_convert_i64_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i64 for f32.convert_i64_u".into(),
        ));
    };

    stack.push(Value::F32(value as u64 as f32));
    Ok(())
}

/// Execute an f64.convert_i32_s instruction
///
/// Converts an i32 value to f64 with signed conversion.
pub fn f64_convert_i32_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for f64.convert_i32_s".into(),
        ));
    };

    stack.push(Value::F64(value as f64));
    Ok(())
}

/// Execute an f64.convert_i32_u instruction
///
/// Converts an i32 value to f64 with unsigned conversion.
pub fn f64_convert_i32_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for f64.convert_i32_u".into(),
        ));
    };

    stack.push(Value::F64(value as u32 as f64));
    Ok(())
}

/// Execute an f64.convert_i64_s instruction
///
/// Converts an i64 value to f64 with signed conversion.
pub fn f64_convert_i64_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i64 for f64.convert_i64_s".into(),
        ));
    };

    stack.push(Value::F64(value as f64));
    Ok(())
}

/// Execute an f64.convert_i64_u instruction
///
/// Converts an i64 value to f64 with unsigned conversion.
pub fn f64_convert_i64_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i64 for f64.convert_i64_u".into(),
        ));
    };

    stack.push(Value::F64(value as u64 as f64));
    Ok(())
}

/// Execute an i32.trunc_f32_s instruction
///
/// Truncates an f32 value to i32 with signed conversion.
pub fn i32_trunc_f32_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f32 for i32.trunc_f32_s".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i32".into()));
    }

    // Check if value is outside the range of i32
    if value >= 2_147_483_648.0 || value < -2_147_483_648.0 {
        return Err(Error::Execution("Value out of range for i32".into()));
    }

    stack.push(Value::I32(value as i32));
    Ok(())
}

/// Execute an i32.trunc_f32_u instruction
///
/// Truncates an f32 value to i32 with unsigned conversion.
pub fn i32_trunc_f32_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f32 for i32.trunc_f32_u".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i32".into()));
    }
    if value >= (1i64 << 32) as f32 || value < 0.0 {
        return Err(Error::Execution("Value outside range of u32".into()));
    }

    stack.push(Value::I32(value as u32 as i32));
    Ok(())
}

/// Execute an i32.trunc_f64_s instruction
///
/// Truncates an f64 value to i32 with signed conversion.
pub fn i32_trunc_f64_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f64 for i32.trunc_f64_s".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i32".into()));
    }
    if value >= (1i64 << 31) as f64 || value < -(1i64 << 31) as f64 {
        return Err(Error::Execution("Value outside range of i32".into()));
    }

    stack.push(Value::I32(value as i32));
    Ok(())
}

/// Execute an i32.trunc_f64_u instruction
///
/// Truncates an f64 value to i32 with unsigned conversion.
pub fn i32_trunc_f64_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f64 for i32.trunc_f64_u".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i32".into()));
    }
    if value >= (1i64 << 32) as f64 || value < 0.0 {
        return Err(Error::Execution("Value outside range of u32".into()));
    }

    stack.push(Value::I32(value as u32 as i32));
    Ok(())
}

/// Execute an i64.trunc_f32_s instruction
///
/// Truncates an f32 value to i64 with signed conversion.
pub fn i64_trunc_f32_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f32 for i64.trunc_f32_s".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i64".into()));
    }

    // Check if value is outside the range of i64
    if value >= 9_223_372_036_854_775_808.0 || value <= -9_223_372_036_854_775_808.0 {
        return Err(Error::Execution("Value out of range for i64".into()));
    }

    stack.push(Value::I64(value as i64));
    Ok(())
}

/// Execute an i64.trunc_f32_u instruction
///
/// Truncates an f32 value to i64 with unsigned conversion.
pub fn i64_trunc_f32_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f32 for i64.trunc_f32_u".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i64".into()));
    }

    // Check if value is outside the range of u64
    if value >= 18_446_744_073_709_551_616.0 || value < 0.0 {
        return Err(Error::Execution("Value out of range for u64".into()));
    }

    stack.push(Value::I64(value as i64));
    Ok(())
}

/// Execute an i64.trunc_f64_s instruction
///
/// Truncates an f64 value to i64 with signed conversion.
pub fn i64_trunc_f64_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f64 for i64.trunc_f64_s".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i64".into()));
    }

    // Check if value is outside the range of i64
    if value >= 9_223_372_036_854_775_808.0 || value <= -9_223_372_036_854_775_808.0 {
        return Err(Error::Execution("Value out of range for i64".into()));
    }

    stack.push(Value::I64(value as i64));
    Ok(())
}

/// Execute an i64.trunc_f64_u instruction
///
/// Truncates an f64 value to i64 with unsigned conversion.
pub fn i64_trunc_f64_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f64 for i64.trunc_f64_u".into()));
    };

    if value.is_nan() {
        return Err(Error::Execution("NaN cannot be converted to i64".into()));
    }

    // Check if value is outside the range of u64
    if value >= 18_446_744_073_709_551_616.0 || value < 0.0 {
        return Err(Error::Execution("Value out of range for u64".into()));
    }

    stack.push(Value::I64(value as i64));
    Ok(())
}

/// Execute an f32.demote_f64 instruction
///
/// Demotes an f64 value to f32.
pub fn f32_demote_f64(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f64 for f32.demote_f64".into()));
    };

    stack.push(Value::F32(value as f32));
    Ok(())
}

/// Execute an f64.promote_f32 instruction
///
/// Promotes an f32 value to f64.
pub fn f64_promote_f32(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected f32 for f64.promote_f32".into()));
    };

    stack.push(Value::F64(value as f64));
    Ok(())
}

/// Execute an i32.reinterpret_f32 instruction
///
/// Reinterprets the bits of an f32 value as an i32 value.
pub fn i32_reinterpret_f32(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected f32 for i32.reinterpret_f32".into(),
        ));
    };

    stack.push(Value::I32(value.to_bits() as i32));
    Ok(())
}

/// Execute an i64.reinterpret_f64 instruction
///
/// Reinterprets the bits of an f64 value as an i64 value.
pub fn i64_reinterpret_f64(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::F64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected f64 for i64.reinterpret_f64".into(),
        ));
    };

    stack.push(Value::I64(value.to_bits() as i64));
    Ok(())
}

/// Execute an f32.reinterpret_i32 instruction
///
/// Reinterprets the bits of an i32 value as an f32 value.
pub fn f32_reinterpret_i32(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for f32.reinterpret_i32".into(),
        ));
    };

    stack.push(Value::F32(f32::from_bits(value as u32)));
    Ok(())
}

/// Execute an f64.reinterpret_i64 instruction
///
/// Reinterprets the bits of an i64 value as an f64 value.
pub fn f64_reinterpret_i64(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i64 for f64.reinterpret_i64".into(),
        ));
    };

    stack.push(Value::F64(f64::from_bits(value as u64)));
    Ok(())
}
