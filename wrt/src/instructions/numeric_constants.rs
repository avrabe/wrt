//! WebAssembly numeric constant instructions
//!
//! This module contains implementations for all WebAssembly numeric constant instructions,
//! including operations for pushing constant values onto the stack.

use crate::error::Error;
use crate::values::Value;
use crate::Vec;

/// Execute an i32.const instruction
///
/// Pushes a constant i32 value onto the stack.
pub fn i32_const(stack: &mut Vec<Value>, value: i32) -> Result<(), Error> {
    stack.push(Value::I32(value));
    Ok(())
}

/// Execute an i64.const instruction
///
/// Pushes a constant i64 value onto the stack.
pub fn i64_const(stack: &mut Vec<Value>, value: i64) -> Result<(), Error> {
    stack.push(Value::I64(value));
    Ok(())
}

/// Execute an f32.const instruction
///
/// Pushes a constant f32 value onto the stack.
pub fn f32_const(stack: &mut Vec<Value>, value: f32) -> Result<(), Error> {
    stack.push(Value::F32(value));
    Ok(())
}

/// Execute an f64.const instruction
///
/// Pushes a constant f64 value onto the stack.
pub fn f64_const(stack: &mut Vec<Value>, value: f64) -> Result<(), Error> {
    stack.push(Value::F64(value));
    Ok(())
}
