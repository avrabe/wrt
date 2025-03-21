//! Shared implementations of WebAssembly instructions
//!
//! This module contains implementations of WebAssembly instructions
//! that can be shared between different engine implementations.

use crate::error::{Error, Result};
use crate::values::Value;

/// Handle the LocalGet instruction by getting a local variable's value
pub fn local_get(locals: &[Value], idx: u32) -> Result<Value> {
    locals
        .get(idx as usize)
        .cloned()
        .ok_or_else(|| Error::Execution(format!("Invalid local index: {}", idx)))
}

/// Handle the LocalSet instruction by setting a local variable's value
pub fn local_set(locals: &mut [Value], idx: u32, value: Value) -> Result<()> {
    if (idx as usize) < locals.len() {
        // Create a clone for debugging
        let value_clone = value.clone();

        // Update the local variable
        locals[idx as usize] = value;

        #[cfg(feature = "std")]
        if let Ok(debug_instr) = std::env::var("WRT_DEBUG_INSTRUCTIONS") {
            if debug_instr == "1" || debug_instr.to_lowercase() == "true" {
                println!(
                    "[LOCAL_SET_DEBUG] Setting local {} to {:?}",
                    idx, value_clone
                );
            }
        }

        Ok(())
    } else {
        Err(Error::Execution(format!("Invalid local index: {}", idx)))
    }
}

/// Handle the LocalTee instruction by setting a local variable while keeping the value on the stack
pub fn local_tee(locals: &mut [Value], idx: u32, value: Value) -> Result<Value> {
    // Update the local variable
    local_set(locals, idx, value.clone())?;

    // Return the value to be kept on the stack
    Ok(value)
}

/// Execute I32Add operation
pub fn i32_add(a: Value, b: Value) -> Result<Value> {
    let b_val = b
        .as_i32()
        .ok_or_else(|| Error::Execution("Expected i32 for second operand".into()))?;
    let a_val = a
        .as_i32()
        .ok_or_else(|| Error::Execution("Expected i32 for first operand".into()))?;

    Ok(Value::I32(a_val + b_val))
}

/// Execute I64Add operation
pub fn i64_add(a: Value, b: Value) -> Result<Value> {
    let b_val = b
        .as_i64()
        .ok_or_else(|| Error::Execution("Expected i64 for second operand".into()))?;
    let a_val = a
        .as_i64()
        .ok_or_else(|| Error::Execution("Expected i64 for first operand".into()))?;

    Ok(Value::I64(a_val + b_val))
}

/// Execute I32Sub operation
pub fn i32_sub(a: Value, b: Value) -> Result<Value> {
    let b_val = b
        .as_i32()
        .ok_or_else(|| Error::Execution("Expected i32 for second operand".into()))?;
    let a_val = a
        .as_i32()
        .ok_or_else(|| Error::Execution("Expected i32 for first operand".into()))?;

    Ok(Value::I32(a_val - b_val))
}

/// Execute I64Sub operation
pub fn i64_sub(a: Value, b: Value) -> Result<Value> {
    let b_val = b
        .as_i64()
        .ok_or_else(|| Error::Execution("Expected i64 for second operand".into()))?;
    let a_val = a
        .as_i64()
        .ok_or_else(|| Error::Execution("Expected i64 for first operand".into()))?;

    Ok(Value::I64(a_val - b_val))
}

/// Execute I32Mul operation
pub fn i32_mul(a: Value, b: Value) -> Result<Value> {
    let b_val = b
        .as_i32()
        .ok_or_else(|| Error::Execution("Expected i32 for second operand".into()))?;
    let a_val = a
        .as_i32()
        .ok_or_else(|| Error::Execution("Expected i32 for first operand".into()))?;

    Ok(Value::I32(a_val * b_val))
}

/// Execute I64Mul operation
pub fn i64_mul(a: Value, b: Value) -> Result<Value> {
    let b_val = b
        .as_i64()
        .ok_or_else(|| Error::Execution("Expected i64 for second operand".into()))?;
    let a_val = a
        .as_i64()
        .ok_or_else(|| Error::Execution("Expected i64 for first operand".into()))?;

    Ok(Value::I64(a_val * b_val))
}

/// Handle I32Const instruction
pub fn i32_const(value: i32) -> Value {
    Value::I32(value)
}

/// Handle I64Const instruction
pub fn i64_const(value: i64) -> Value {
    Value::I64(value)
}

/// Handle F32Const instruction
pub fn f32_const(value: f32) -> Value {
    Value::F32(value)
}

/// Handle F64Const instruction
pub fn f64_const(value: f64) -> Value {
    Value::F64(value)
}
