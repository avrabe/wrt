//! Shared implementations of WebAssembly instructions
//!
//! This module contains implementations of WebAssembly instructions
//! that can be shared between different engine implementations.

use crate::{
    behavior::{ControlFlow, FrameBehavior, StackBehavior},
    error::{kinds, Error, Result},
    prelude::TypesValue as Value,
};

#[cfg(feature = "std")]
use std::format;

#[cfg(not(feature = "std"))]
use alloc::format;

/// Handle the `LocalGet` instruction by getting a local variable's value
///
/// # Errors
///
/// Returns an error if the local index is out of bounds.
pub fn local_get(locals: &[Value], idx: u32) -> Result<Value> {
    locals
        .get(idx as usize)
        .cloned()
        .ok_or_else(|| kinds::ExecutionError(format!("Invalid local index: {idx}")).into())
}

/// Handle the `LocalSet` instruction by setting a local variable's value
///
/// # Errors
///
/// Returns an error if the local index is out of bounds.
pub fn local_set(locals: &mut [Value], idx: u32, value: Value) -> Result<()> {
    if (idx as usize) < locals.len() {
        // Create a clone for debugging
        let _value_clone = value.clone();

        // Update the local variable
        locals[idx as usize] = value;

        #[cfg(feature = "std")]
        if let Ok(debug_instr) = std::env::var("WRT_DEBUG_INSTRUCTIONS") {
            if debug_instr == "1" || debug_instr.to_lowercase() == "true" {
                println!("[LOCAL_SET_DEBUG] Setting local {idx} to {_value_clone:?}");
            }
        }

        Ok(())
    } else {
        Err(kinds::ExecutionError(format!("Invalid local index: {idx}")).into())
    }
}

/// Handle the `LocalTee` instruction by setting a local variable while keeping the value on the stack
///
/// # Errors
///
/// Returns an error if the local index is out of bounds.
pub fn local_tee(locals: &mut [Value], idx: u32, value: Value) -> Result<Value> {
    // Update the local variable
    local_set(locals, idx, value.clone())?;

    // Return the value to be kept on the stack
    Ok(value)
}

/// Execute `I32Add` operation
///
/// # Errors
///
/// Returns an error if the values are not both `i32` types.
pub fn i32_add(a: &Value, b: &Value) -> Result<Value> {
    let b_val = b.as_i32().ok_or_else(|| {
        kinds::ExecutionError("Expected i32 for second operand".into()).into()
    })?;
    let a_val = a.as_i32().ok_or_else(|| {
        kinds::ExecutionError("Expected i32 for first operand".into()).into()
    })?;

    Ok(Value::I32(a_val + b_val))
}

/// Execute `I64Add` operation
///
/// # Errors
///
/// Returns an error if the values are not both `i64` types.
pub fn i64_add(a: &Value, b: &Value) -> Result<Value> {
    let b_val = b.as_i64().ok_or_else(|| {
        kinds::ExecutionError("Expected i64 for second operand".into()).into()
    })?;
    let a_val = a.as_i64().ok_or_else(|| {
        kinds::ExecutionError("Expected i64 for first operand".into()).into()
    })?;

    Ok(Value::I64(a_val + b_val))
}

/// Execute `I32Sub` operation
///
/// # Errors
///
/// Returns an error if the values are not both `i32` types.
pub fn i32_sub(a: &Value, b: &Value) -> Result<Value> {
    let b_val = b.as_i32().ok_or_else(|| {
        kinds::ExecutionError("Expected i32 for second operand".into()).into()
    })?;
    let a_val = a.as_i32().ok_or_else(|| {
        kinds::ExecutionError("Expected i32 for first operand".into()).into()
    })?;

    Ok(Value::I32(a_val - b_val))
}

/// Execute `I64Sub` operation
///
/// # Errors
///
/// Returns an error if the values are not both `i64` types.
pub fn i64_sub(a: &Value, b: &Value) -> Result<Value> {
    let b_val = b.as_i64().ok_or_else(|| {
        kinds::ExecutionError("Expected i64 for second operand".into()).into()
    })?;
    let a_val = a.as_i64().ok_or_else(|| {
        kinds::ExecutionError("Expected i64 for first operand".into()).into()
    })?;

    Ok(Value::I64(a_val - b_val))
}

/// Execute `I32Mul` operation
///
/// # Errors
///
/// Returns an error if the values are not both `i32` types.
pub fn i32_mul(a: &Value, b: &Value) -> Result<Value> {
    let b_val = b.as_i32().ok_or_else(|| {
        kinds::ExecutionError("Expected i32 for second operand".into()).into()
    })?;
    let a_val = a.as_i32().ok_or_else(|| {
        kinds::ExecutionError("Expected i32 for first operand".into()).into()
    })?;

    Ok(Value::I32(a_val * b_val))
}

/// Execute `I64Mul` operation
///
/// # Errors
///
/// Returns an error if the values are not both `i64` types.
pub fn i64_mul(a: &Value, b: &Value) -> Result<Value> {
    let b_val = b.as_i64().ok_or_else(|| {
        kinds::ExecutionError("Expected i64 for second operand".into()).into()
    })?;
    let a_val = a.as_i64().ok_or_else(|| {
        kinds::ExecutionError("Expected i64 for first operand".into()).into()
    })?;

    Ok(Value::I64(a_val * b_val))
}

/// Handle `I32Const` instruction
#[must_use]
pub const fn i32_const(value: i32) -> Value {
    Value::I32(value)
}

/// Handle `I64Const` instruction
#[must_use]
pub const fn i64_const(value: i64) -> Value {
    Value::I64(value)
}

/// Handle `F32Const` instruction
#[must_use]
pub const fn f32_const(value: f32) -> Value {
    Value::F32(value)
}

/// Handle `F64Const` instruction
#[must_use]
pub const fn f64_const(value: f64) -> Value {
    Value::F64(value)
}

/// Gets local value by index
pub fn get_local<T>(frame: &mut dyn FrameBehavior, idx: u32) -> Result<Value>
where
    T: FrameBehavior,
{
    frame
        .get_local(idx.try_into().unwrap())
        .map_err(|e| kinds::ExecutionError(format!("Invalid local index: {idx}")).into())
}

/// Sets local value by index
pub fn set_local<T>(frame: &mut dyn FrameBehavior, idx: u32, value: Value) -> Result<()>
where
    T: FrameBehavior,
{
    match frame.set_local(idx.try_into().unwrap(), value) {
        Ok(()) => Ok(()),
        Err(e) => Err(kinds::ExecutionError(format!("Invalid local index: {idx}")).into()),
    }
}

/// Execute an i32 relop with two operands
pub fn i32_relop(
    stack: &mut impl StackBehavior,
    op: fn(i32, i32) -> bool,
) -> Result<ControlFlow> {
    // Get the second operand
    let v2 = stack.pop()?;
    if let Value::I32(i2) = v2 {
        // Get the first operand
        let v1 = stack.pop()?;
        if let Value::I32(i1) = v1 {
            // Compute the result
            let result = op(i1, i2);
            // Push the result
            stack.push(Value::I32(result as i32))?;
            Ok(ControlFlow::Continue)
        } else {
            Err(kinds::ExecutionError("Expected i32 for first operand".into()).into())
        }
    } else {
        Err(kinds::ExecutionError("Expected i32 for second operand".into()).into())
    }
}

/// Execute an i64 relop with two operands
pub fn i64_relop(
    stack: &mut impl StackBehavior,
    op: fn(i64, i64) -> bool,
) -> Result<ControlFlow> {
    // Get the second operand
    let v2 = stack.pop()?;
    if let Value::I64(i2) = v2 {
        // Get the first operand
        let v1 = stack.pop()?;
        if let Value::I64(i1) = v1 {
            // Compute the result
            let result = op(i1, i2);
            // Push the result
            stack.push(Value::I32(result as i32))?;
            Ok(ControlFlow::Continue)
        } else {
            Err(kinds::ExecutionError("Expected i64 for first operand".into()).into())
        }
    } else {
        Err(kinds::ExecutionError("Expected i64 for second operand".into()).into())
    }
}

/// Execute an i32 binary operation with two operands
pub fn i32_binop(
    stack: &mut impl StackBehavior,
    op: fn(i32, i32) -> i32,
) -> Result<ControlFlow> {
    // Get the second operand
    let v2 = stack.pop()?;
    if let Value::I32(i2) = v2 {
        // Get the first operand
        let v1 = stack.pop()?;
        if let Value::I32(i1) = v1 {
            // Compute the result
            let result = op(i1, i2);
            // Push the result
            stack.push(Value::I32(result))?;
            Ok(ControlFlow::Continue)
        } else {
            Err(kinds::ExecutionError("Expected i32 for first operand".into()).into())
        }
    } else {
        Err(kinds::ExecutionError("Expected i32 for second operand".into()).into())
    }
}

/// Execute an i64 binary operation with two operands
pub fn i64_binop(
    stack: &mut impl StackBehavior,
    op: fn(i64, i64) -> i64,
) -> Result<ControlFlow> {
    // Get the second operand
    let v2 = stack.pop()?;
    if let Value::I64(i2) = v2 {
        // Get the first operand
        let v1 = stack.pop()?;
        if let Value::I64(i1) = v1 {
            // Compute the result
            let result = op(i1, i2);
            // Push the result
            stack.push(Value::I64(result))?;
            Ok(ControlFlow::Continue)
        } else {
            Err(kinds::ExecutionError("Expected i64 for first operand".into()).into())
        }
    } else {
        Err(kinds::ExecutionError("Expected i64 for second operand".into()).into())
    }
}