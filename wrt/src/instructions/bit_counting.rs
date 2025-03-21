//! WebAssembly bit counting instructions
//!
//! This module contains implementations for all WebAssembly bit counting instructions,
//! including operations for counting leading and trailing zeros and ones.

use crate::error::Error;
use crate::values::Value;
use crate::Vec;

/// Execute an i32.clz instruction
///
/// Counts the number of leading zeros in an i32 value.
pub fn i32_clz(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for i32.clz".into()));
    };

    stack.push(Value::I32(value.leading_zeros() as i32));
    Ok(())
}

/// Execute an i32.ctz instruction
///
/// Counts the number of trailing zeros in an i32 value.
pub fn i32_ctz(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for i32.ctz".into()));
    };

    stack.push(Value::I32(value.trailing_zeros() as i32));
    Ok(())
}

/// Execute an i32.popcnt instruction
///
/// Counts the number of 1 bits in an i32 value.
pub fn i32_popcnt(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I32(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for i32.popcnt".into()));
    };

    stack.push(Value::I32(value.count_ones() as i32));
    Ok(())
}

/// Execute an i64.clz instruction
///
/// Counts the number of leading zeros in an i64 value.
pub fn i64_clz(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i64 for i64.clz".into()));
    };

    stack.push(Value::I64(value.leading_zeros() as i64));
    Ok(())
}

/// Execute an i64.ctz instruction
///
/// Counts the number of trailing zeros in an i64 value.
pub fn i64_ctz(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i64 for i64.ctz".into()));
    };

    stack.push(Value::I64(value.trailing_zeros() as i64));
    Ok(())
}

/// Execute an i64.popcnt instruction
///
/// Counts the number of 1 bits in an i64 value.
pub fn i64_popcnt(stack: &mut Vec<Value>) -> Result<(), Error> {
    let Value::I64(value) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i64 for i64.popcnt".into()));
    };

    stack.push(Value::I64(value.count_ones() as i64));
    Ok(())
}
