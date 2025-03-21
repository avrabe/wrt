//! WebAssembly arithmetic instructions
//!
//! This module contains implementations for all WebAssembly arithmetic instructions,
//! including addition, subtraction, multiplication, division, and more.

use crate::error::Error;
use crate::Value;
use crate::Vec;

/// Execute an i32 addition instruction
///
/// Pops two i32 values from the stack, adds them, and pushes the result.
pub fn i32_add(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1.wrapping_add(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 subtraction instruction
///
/// Pops two i32 values from the stack, subtracts the second from the first, and pushes the result.
pub fn i32_sub(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1.wrapping_sub(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 multiplication instruction
///
/// Pops two i32 values from the stack, multiplies them, and pushes the result.
pub fn i32_mul(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1.wrapping_mul(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 signed division instruction
///
/// Pops two i32 values from the stack, divides the first by the second (signed),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i32_div_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        if v2 == 0 {
            return Err(Error::Execution("Division by zero".into()));
        }
        // The WebAssembly spec requires a trap when dividing INT_MIN by -1
        if v1 == i32::MIN && v2 == -1 {
            return Err(Error::Execution("Integer overflow".into()));
        }
        stack.push(Value::I32(v1.wrapping_div(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 unsigned division instruction
///
/// Pops two i32 values from the stack, divides the first by the second (unsigned),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i32_div_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        if v2 == 0 {
            return Err(Error::Execution("Division by zero".into()));
        }
        stack.push(Value::I32(((v1 as u32).wrapping_div(v2 as u32)) as i32));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 signed remainder instruction
///
/// Pops two i32 values from the stack, computes the remainder of the first divided
/// by the second (signed), and pushes the result. Returns an error if dividing by zero.
pub fn i32_rem_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        if v2 == 0 {
            return Err(Error::Execution("Division by zero".into()));
        }
        stack.push(Value::I32(v1.wrapping_rem(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 unsigned remainder instruction
///
/// Pops two i32 values from the stack, computes the remainder of the first divided
/// by the second (unsigned), and pushes the result. Returns an error if dividing by zero.
pub fn i32_rem_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        if v2 == 0 {
            return Err(Error::Execution("Division by zero".into()));
        }
        stack.push(Value::I32(((v1 as u32).wrapping_rem(v2 as u32)) as i32));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 bitwise AND instruction
///
/// Pops two i32 values from the stack, computes their bitwise AND, and pushes the result.
pub fn i32_and(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1 & v2));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 bitwise OR instruction
///
/// Pops two i32 values from the stack, computes their bitwise OR, and pushes the result.
pub fn i32_or(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1 | v2));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 bitwise XOR instruction
///
/// Pops two i32 values from the stack, computes their bitwise XOR, and pushes the result.
pub fn i32_xor(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1 ^ v2));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 shift left instruction
///
/// Pops two i32 values from the stack, shifts the first left by the lower 5 bits
/// of the second value, and pushes the result.
pub fn i32_shl(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1.wrapping_shl(v2 as u32 % 32)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 signed shift right instruction
///
/// Pops two i32 values from the stack, shifts the first right by the lower 5 bits
/// of the second value (signed, preserving sign bit), and pushes the result.
pub fn i32_shr_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(v1.wrapping_shr(v2 as u32 % 32)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 unsigned shift right instruction
///
/// Pops two i32 values from the stack, shifts the first right by the lower 5 bits
/// of the second value (unsigned, filling with zeros), and pushes the result.
pub fn i32_shr_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        stack.push(Value::I32(
            ((v1 as u32).wrapping_shr(v2 as u32 % 32)) as i32,
        ));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 rotate left instruction
///
/// Pops two i32 values from the stack, rotates the bits of the first left by the
/// lower 5 bits of the second value, and pushes the result.
pub fn i32_rotl(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        let shift = (v2 as u32) % 32;
        let value = v1 as u32;
        let result = value.rotate_left(shift);
        stack.push(Value::I32(result as i32));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i32 rotate right instruction
///
/// Pops two i32 values from the stack, rotates the bits of the first right by the
/// lower 5 bits of the second value, and pushes the result.
pub fn i32_rotr(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I32(v1), Value::I32(v2)) = (val1, val2) {
        let shift = (v2 as u32) % 32;
        let value = v1 as u32;
        let result = value.rotate_right(shift);
        stack.push(Value::I32(result as i32));
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 values".into()))
    }
}

/// Execute an i64 addition instruction
///
/// Pops two i64 values from the stack, adds them, and pushes the result.
pub fn i64_add(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I64(v1), Value::I64(v2)) = (val1, val2) {
        stack.push(Value::I64(v1.wrapping_add(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i64 values".into()))
    }
}

/// Execute an i64 subtraction instruction
///
/// Pops two i64 values from the stack, subtracts the second from the first, and pushes the result.
pub fn i64_sub(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I64(v1), Value::I64(v2)) = (val1, val2) {
        stack.push(Value::I64(v1.wrapping_sub(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i64 values".into()))
    }
}

/// Execute an i64 multiplication instruction
///
/// Pops two i64 values from the stack, multiplies them, and pushes the result.
pub fn i64_mul(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I64(v1), Value::I64(v2)) = (val1, val2) {
        stack.push(Value::I64(v1.wrapping_mul(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i64 values".into()))
    }
}

/// Execute an i64 signed division instruction
///
/// Pops two i64 values from the stack, divides the first by the second (signed),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i64_div_s(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I64(v1), Value::I64(v2)) = (val1, val2) {
        if v2 == 0 {
            return Err(Error::Execution("Division by zero".into()));
        }
        // The WebAssembly spec requires a trap when dividing INT_MIN by -1
        if v1 == i64::MIN && v2 == -1 {
            return Err(Error::Execution("Integer overflow".into()));
        }
        stack.push(Value::I64(v1.wrapping_div(v2)));
        Ok(())
    } else {
        Err(Error::Execution("Expected i64 values".into()))
    }
}

/// Execute an i64 unsigned division instruction
///
/// Pops two i64 values from the stack, divides the first by the second (unsigned),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i64_div_u(stack: &mut Vec<Value>) -> Result<(), Error> {
    let val2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let val1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let (Value::I64(v1), Value::I64(v2)) = (val1, val2) {
        if v2 == 0 {
            return Err(Error::Execution("Division by zero".into()));
        }
        stack.push(Value::I64(((v1 as u64).wrapping_div(v2 as u64)) as i64));
        Ok(())
    } else {
        Err(Error::Execution("Expected i64 values".into()))
    }
}

// More i64 arithmetic operations would follow a similar pattern to the i32 operations
// TODO: Implement the remaining i64 arithmetic operations
// TODO: Implement f32/f64 arithmetic operations
