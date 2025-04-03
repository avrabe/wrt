//! WebAssembly arithmetic instructions
//!
//! This module contains implementations for all WebAssembly arithmetic instructions,
//! including addition, subtraction, multiplication, division, and more.

use crate::{
    behavior::{
        ControlFlowBehavior, FrameBehavior, /* MemoryBehavior, */ NullBehavior, StackBehavior,
    },
    error::{Error, Result},
    memory::MemoryBehavior,
    stack::Stack,
    values::Value,
    StacklessEngine,
};

/// Execute an i32 addition instruction
///
/// Pops two i32 values from the stack, adds them, and pushes the result.
pub fn i32_add(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    println!("DEBUG: i32_add - Starting execution");
    println!("DEBUG: i32_add - Stack before: {:?}", stack.values());

    // Check for stack underflow
    if stack.len() < 2 {
        println!(
            "DEBUG: i32_add - Error: Stack underflow. Stack has {} values, need at least 2",
            stack.len()
        );
        return Err(Error::StackUnderflow);
    }

    // Pop the values
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;

    println!("DEBUG: i32_add - Popped values: a={a:?}, b={b:?}");

    // Perform the addition
    let result = a.wrapping_add(b);
    println!("DEBUG: i32_add - Result: {a} + {b} = {result}");

    // Push the result back to the stack
    stack.push(Value::I32(result))?;
    println!("DEBUG: i32_add - Stack after: {:?}", stack.values());

    Ok(())
}

/// Execute an i32 subtraction instruction
///
/// Pops two i32 values from the stack, subtracts the second from the first, and pushes the result.
pub fn i32_sub(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a.wrapping_sub(b)))?;
    Ok(())
}

/// Execute an i32 multiplication instruction
///
/// Pops two i32 values from the stack, multiplies them, and pushes the result.
pub fn i32_mul(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a.wrapping_mul(b)))?;
    Ok(())
}

/// Execute an i32 signed division instruction
///
/// Pops two i32 values from the stack, divides the first by the second (signed),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i32_div_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    if a == i32::MIN && b == -1 {
        return Err(Error::IntegerOverflow);
    }
    stack.push(Value::I32(a.wrapping_div(b)))?;
    Ok(())
}

/// Execute an i32 unsigned division instruction
///
/// Pops two i32 values from the stack, divides the first by the second (unsigned),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i32_div_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_u32()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    stack.push(Value::I32(a.wrapping_div(b) as i32))?;
    Ok(())
}

/// Execute an i32 signed remainder instruction
///
/// Pops two i32 values from the stack, computes the remainder of the first divided
/// by the second (signed), and pushes the result. Returns an error if dividing by zero.
pub fn i32_rem_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    stack.push(Value::I32(a.wrapping_rem(b)))?;
    Ok(())
}

/// Execute an i32 unsigned remainder instruction
///
/// Pops two i32 values from the stack, computes the remainder of the first divided
/// by the second (unsigned), and pushes the result. Returns an error if dividing by zero.
pub fn i32_rem_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_u32()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    stack.push(Value::I32(a.wrapping_rem(b) as i32))?;
    Ok(())
}

/// Execute an i32 bitwise AND instruction
///
/// Pops two i32 values from the stack, computes their bitwise AND, and pushes the result.
pub fn i32_and(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    println!("DEBUG: i32_and - Stack BEFORE pop: {:?}", stack.values());
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    println!("DEBUG: i32_and - Popped a={:?}, b={:?}", a, b);
    stack.push(Value::I32(a & b))?;
    println!("DEBUG: i32_and - Stack BEFORE push: {:?}", stack.values());
    Ok(())
}

/// Execute an i32 bitwise OR instruction
///
/// Pops two i32 values from the stack, computes their bitwise OR, and pushes the result.
pub fn i32_or(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a | b))?;
    Ok(())
}

/// Execute an i32 bitwise XOR instruction
///
/// Pops two i32 values from the stack, computes their bitwise XOR, and pushes the result.
pub fn i32_xor(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a ^ b))?;
    Ok(())
}

/// Execute an i32 shift left instruction
///
/// Pops two i32 values from the stack, shifts the first left by the lower 5 bits
/// of the second value, and pushes the result.
pub fn i32_shl(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a.wrapping_shl(b % 32)))
}

/// Execute an i32 signed shift right instruction
///
/// Pops two i32 values from the stack, shifts the first right by the lower 5 bits
/// of the second value (signed, preserving sign bit), and pushes the result.
pub fn i32_shr_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a.wrapping_shr(b % 32)))
}

/// Execute an i32 unsigned shift right instruction
///
/// Pops two i32 values from the stack, shifts the first right by the lower 5 bits
/// of the second value (unsigned, filling with zeros), and pushes the result.
pub fn i32_shr_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_u32()?;
    stack.push(Value::I32(a.wrapping_shr(b % 32) as i32))
}

/// Execute an i32 rotate left instruction
///
/// Pops two i32 values from the stack, rotates the bits of the first left by the
/// lower 5 bits of the second value, and pushes the result.
pub fn i32_rotl(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a.rotate_left(b % 32)))
}

/// Execute an i32 rotate right instruction
///
/// Pops two i32 values from the stack, rotates the bits of the first right by the
/// lower 5 bits of the second value, and pushes the result.
pub fn i32_rotr(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u32()?;
    let a = stack.pop()?.as_i32()?;
    stack.push(Value::I32(a.rotate_right(b % 32)))
}

/// Execute an i64 addition instruction
///
/// Pops two i64 values from the stack, adds them, and pushes the result.
pub fn i64_add(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.wrapping_add(b)))?;
    Ok(())
}

/// Execute an i64 subtraction instruction
///
/// Pops two i64 values from the stack, subtracts the second from the first, and pushes the result.
pub fn i64_sub(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.wrapping_sub(b)))?;
    Ok(())
}

/// Execute an i64 multiplication instruction
///
/// Pops two i64 values from the stack, multiplies them, and pushes the result.
pub fn i64_mul(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.wrapping_mul(b)))?;
    Ok(())
}

/// Execute an i64 signed division instruction
///
/// Pops two i64 values from the stack, divides the first by the second (signed),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i64_div_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    if a == i64::MIN && b == -1 {
        return Err(Error::IntegerOverflow);
    }
    stack.push(Value::I64(a.wrapping_div(b)))?;
    Ok(())
}

/// Execute an i64 unsigned division instruction
///
/// Pops two i64 values from the stack, divides the first by the second (unsigned),
/// and pushes the result. Returns an error if dividing by zero.
pub fn i64_div_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_u64()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    stack.push(Value::I64(a.wrapping_div(b) as i64))?;
    Ok(())
}

/// Execute an i64 signed remainder instruction
///
/// Pops two i64 values from the stack, computes the remainder of the first divided
/// by the second (signed), and pushes the result. Returns an error if dividing by zero.
pub fn i64_rem_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    stack.push(Value::I64(a.wrapping_rem(b)))?;
    Ok(())
}

/// Execute an i64 unsigned remainder instruction
///
/// Pops two i64 values from the stack, computes the remainder of the first divided
/// by the second (unsigned), and pushes the result. Returns an error if dividing by zero.
pub fn i64_rem_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_u64()?;
    if b == 0 {
        return Err(Error::DivisionByZero);
    }
    stack.push(Value::I64(a.wrapping_rem(b) as i64))?;
    Ok(())
}

/// Execute an i64 bitwise AND instruction
///
/// Pops two i64 values from the stack, computes their bitwise AND, and pushes the result.
pub fn i64_and(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a & b))?;
    Ok(())
}

/// Execute an i64 bitwise OR instruction
///
/// Pops two i64 values from the stack, computes their bitwise OR, and pushes the result.
pub fn i64_or(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a | b))?;
    Ok(())
}

/// Execute an i64 bitwise XOR instruction
///
/// Pops two i64 values from the stack, computes their bitwise XOR, and pushes the result.
pub fn i64_xor(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_i64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a ^ b))?;
    Ok(())
}

/// Execute an i64 shift left instruction
///
/// Pops two i64 values from the stack, shifts the first left by the lower 6 bits
/// of the second value, and pushes the result.
pub fn i64_shl(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.wrapping_shl((b % 64) as u32)))
}

/// Execute an i64 signed shift right instruction
///
/// Pops two i64 values from the stack, shifts the first right by the lower 6 bits
/// of the second value (signed, preserving sign bit), and pushes the result.
pub fn i64_shr_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.wrapping_shr((b % 64) as u32)))
}

/// Execute an i64 unsigned shift right instruction
///
/// Pops two i64 values from the stack, shifts the first right by the lower 6 bits
/// of the second value (unsigned, filling with zeros), and pushes the result.
pub fn i64_shr_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_u64()?;
    stack.push(Value::I64(a.wrapping_shr((b % 64) as u32) as i64))
}

/// Execute an i64 rotate left instruction
///
/// Pops two i64 values from the stack, rotates the bits of the first left by the
/// lower 6 bits of the second value, and pushes the result.
pub fn i64_rotl(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.rotate_left((b % 64) as u32)))
}

/// Execute an i64 rotate right instruction
///
/// Pops two i64 values from the stack, rotates the bits of the first right by the
/// lower 6 bits of the second value, and pushes the result.
pub fn i64_rotr(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_u64()?;
    let a = stack.pop()?.as_i64()?;
    stack.push(Value::I64(a.rotate_right((b % 64) as u32)))
}

/// Execute an f32 ceiling instruction
///
/// Pops an f32 value from the stack, rounds it up to the nearest integer, and pushes the result.
pub fn f32_ceil(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f32()?;
    stack.push(Value::F32(a.ceil()))?;
    Ok(())
}

/// Execute an f32 floor instruction
///
/// Pops an f32 value from the stack, rounds it down to the nearest integer, and pushes the result.
pub fn f32_floor(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f32()?;
    stack.push(Value::F32(a.floor()))?;
    Ok(())
}

/// Execute an f32 truncate instruction
///
/// Pops an f32 value from the stack, truncates it toward zero to the nearest integer, and pushes the result.
pub fn f32_trunc(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f32()?;
    stack.push(Value::F32(a.trunc()))?;
    Ok(())
}

/// Execute an f32 nearest instruction
///
/// Pops an f32 value from the stack, rounds it to the nearest integer, and pushes the result.
pub fn f32_nearest(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f32()?;
    match a {
        // Special cases
        _ if a.is_nan() || a.is_infinite() || a == 0.0 => {
            stack.push(Value::F32(a))?;
            return Ok(());
        }
        _ => {
            // Banker's rounding (round to nearest, ties to even)
            let floor = a.floor();
            let fract = a.fract().abs(); // Use fract to get the fractional part

            let result = if fract < 0.5 {
                floor
            } else if fract > 0.5 {
                floor + 1.0
            } else {
                // If exactly 0.5, round to the nearest even integer
                // Floor will be the next lower integer
                let floor_int = floor as i32;
                if floor_int % 2 == 0 {
                    // floor is even
                    floor
                } else {
                    // floor is odd
                    floor + 1.0
                }
            };

            // Debug print to verify rounding
            println!("f32.nearest: nearest({}) = {}", a, result);

            stack.push(Value::F32(result))?;
            Ok(())
        }
    }
}

/// Execute an f32 add instruction
///
/// Pops two f32 values from the stack, adds them, and pushes the result.
pub fn f32_add(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f32()?;
    let a = stack.pop()?.as_f32()?;

    println!("f32_add called with a={:?}, b={:?}", a, b);

    match (a, b) {
        (Value::F32(a), Value::F32(b)) => {
            // Debug exact values
            println!(
                "f32_add exact values: a={} ({}), b={} ({})",
                a,
                a.to_bits(),
                b,
                b.to_bits()
            );

            // Always calculate the sum
            let result = a + b;
            println!("Regular case: a + b = {}", result);
            stack.push(Value::F32(result))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f32".to_string())),
    }
}

/// Execute an f32 subtract instruction
///
/// Pops two f32 values from the stack, subtracts the second from the first, and pushes the result.
pub fn f32_sub(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f32()?;
    let a = stack.pop()?.as_f32()?;
    match (a, b) {
        (Value::F32(a), Value::F32(b)) => {
            stack.push(Value::F32(a - b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f32".to_string())),
    }
}

/// Execute an f32 multiply instruction
///
/// Pops two f32 values from the stack, multiplies them, and pushes the result.
pub fn f32_mul(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f32()?;
    let a = stack.pop()?.as_f32()?;
    match (a, b) {
        (Value::F32(a), Value::F32(b)) => {
            stack.push(Value::F32(a * b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f32".to_string())),
    }
}

/// Execute an f32 divide instruction
///
/// Pops two f32 values from the stack, divides the first by the second, and pushes the result.
pub fn f32_div(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f32()?;
    let a = stack.pop()?.as_f32()?;
    match (a, b) {
        (Value::F32(a), Value::F32(b)) => {
            stack.push(Value::F32(a / b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f32".to_string())),
    }
}

/// Execute an f64 addition instruction
///
/// Pops two f64 values from the stack, adds them, and pushes the result.
pub fn f64_add(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    match (a, b) {
        (Value::F64(a), Value::F64(b)) => {
            stack.push(Value::F64(a + b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 subtraction instruction
///
/// Pops two f64 values from the stack, subtracts the second from the first, and pushes the result.
pub fn f64_sub(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    match (a, b) {
        (Value::F64(a), Value::F64(b)) => {
            stack.push(Value::F64(a - b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 multiplication instruction
///
/// Pops two f64 values from the stack, multiplies them, and pushes the result.
pub fn f64_mul(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    match (a, b) {
        (Value::F64(a), Value::F64(b)) => {
            stack.push(Value::F64(a * b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 division instruction
///
/// Pops two f64 values from the stack, divides the first by the second, and pushes the result.
pub fn f64_div(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    match (a, b) {
        (Value::F64(a), Value::F64(b)) => {
            if b == 0.0 {
                return Err(Error::DivisionByZero);
            }
            stack.push(Value::F64(a / b))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 maximum instruction
///
/// Pops two f64 values from the stack, pushes the maximum of the two values.
pub fn f64_max(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    match (a, b) {
        (Value::F64(a), Value::F64(b)) => {
            stack.push(Value::F64(a.max(b)))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 minimum instruction
///
/// Pops two f64 values from the stack, pushes the minimum of the two values.
pub fn f64_min(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    match (a, b) {
        (Value::F64(a), Value::F64(b)) => {
            stack.push(Value::F64(a.min(b)))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 square root instruction
///
/// Pops an f64 value from the stack, computes its square root, and pushes the result.
pub fn f64_sqrt(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    match a {
        Value::F64(a) => {
            stack.push(Value::F64(a.sqrt()))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 nearest instruction
///
/// Pops an f64 value from the stack, rounds it to the nearest integer, and pushes the result.
/// Round-to-nearest rounds to the nearest integral value; if two integral values are equally near,
/// rounds to the even value (Banker's rounding).
pub fn f64_nearest(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    match a {
        // Special cases
        _ if a.is_nan() || a.is_infinite() || a == 0.0 => {
            stack.push(Value::F64(a))?;
            return Ok(());
        }
        _ => {
            // Banker's rounding (round to nearest, ties to even)
            let floor = a.floor();
            let fract = a.fract().abs(); // Use fract to get the fractional part

            let result = if fract < 0.5 {
                floor
            } else if fract > 0.5 {
                floor + 1.0
            } else {
                // If exactly 0.5, round to the nearest even integer
                // Convert to integer to check even/odd
                let floor_int = floor as i64;
                if floor_int % 2 == 0 {
                    // floor is even
                    floor
                } else {
                    // floor is odd
                    floor + 1.0
                }
            };

            stack.push(Value::F64(result))?;
            Ok(())
        }
    }
}

/// Execute an f64 truncate instruction
///
/// Pops an f64 value from the stack, truncates it toward zero to the nearest integer, and pushes the result.
/// Truncation removes the fractional part, rounding toward zero.
pub fn f64_trunc(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    match a {
        Value::F64(a) => {
            stack.push(Value::F64(a.trunc()))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 floor instruction
///
/// Pops an f64 value from the stack, rounds it down to the nearest integer, and pushes the result.
/// Floor rounding always rounds towards negative infinity.
pub fn f64_floor(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    match a {
        Value::F64(a) => {
            stack.push(Value::F64(a.floor()))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 ceil instruction
///
/// Pops an f64 value from the stack, rounds it up to the nearest integer, and pushes the result.
/// Ceiling rounding always rounds towards positive infinity.
pub fn f64_ceil(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    stack.push(Value::F64(a.ceil()))?;
    Ok(())
}

/// Execute an f64 copysign instruction
///
/// Pops two f64 values from the stack, copies the sign of the second to the first, and pushes the result.
pub fn f64_copysign(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let b = stack.pop()?.as_f64()?;
    let a = stack.pop()?.as_f64()?;
    stack.push(Value::F64(a.copysign(b)))?;
    Ok(())
}

/// Execute an f64 absolute value instruction
///
/// Pops an f64 value from the stack, computes its absolute value, and pushes the result.
pub fn f64_abs(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    match a {
        Value::F64(a) => {
            stack.push(Value::F64(a.abs()))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Execute an f64 negate instruction
///
/// Pops an f64 value from the stack, negates it, and pushes the result.
pub fn f64_neg(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_f64()?;
    match a {
        Value::F64(a) => {
            stack.push(Value::F64(-a))?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected f64".to_string())),
    }
}

/// Add the missing sections back

/// Conversion ops
pub fn i32_wrap_i64(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i64()?;
    stack.push(Value::I32(val as i32))
}

pub fn i64_extend_i32_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    stack.push(Value::I64(val as i64))
}

pub fn i64_extend_i32_u(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_u32()?;
    stack.push(Value::I64(val as i64))
}

macro_rules! trunc_impl {
    ($name:ident, $from_ty:ty, $to_ty:ty, $intermediate_ty:ty, $signedness:ident, $check_nan:expr, $check_bounds:expr) => {
        pub fn $name(
            stack: &mut dyn Stack,
            _frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let val = stack.pop()?.as_::<$from_ty>()?;
            if $check_nan && val.is_nan() {
                return Err(Error::InvalidConversionToInt);
            }
            if !$check_bounds(val) {
                return Err(Error::IntegerOverflow);
            }
            stack.push(Value::$intermediate_ty(val.trunc() as $to_ty))
        }
    };
}

#[inline]
fn check_bounds_i32_trunc_f32_s(val: f32) -> bool {
    val > (i32::MIN as f32 - 1.0) && val < (i32::MAX as f32 + 1.0)
}
#[inline]
fn check_bounds_i32_trunc_f32_u(val: f32) -> bool {
    val > -1.0 && val < (u32::MAX as f32 + 1.0)
}
#[inline]
fn check_bounds_i32_trunc_f64_s(val: f64) -> bool {
    val > (i32::MIN as f64 - 1.0) && val < (i32::MAX as f64 + 1.0)
}
#[inline]
fn check_bounds_i32_trunc_f64_u(val: f64) -> bool {
    val > -1.0 && val < (u32::MAX as f64 + 1.0)
}
#[inline]
fn check_bounds_i64_trunc_f32_s(val: f32) -> bool {
    val > (i64::MIN as f32 - 1.0) && val < (i64::MAX as f32 + 1.0)
}
#[inline]
fn check_bounds_i64_trunc_f32_u(val: f32) -> bool {
    val > -1.0 && val < (u64::MAX as f32 + 1.0)
}
#[inline]
fn check_bounds_i64_trunc_f64_s(val: f64) -> bool {
    val > (i64::MIN as f64 - 1.0) && val < (i64::MAX as f64 + 1.0)
}
#[inline]
fn check_bounds_i64_trunc_f64_u(val: f64) -> bool {
    val > -1.0 && val < (u64::MAX as f64 + 1.0)
}

trunc_impl!(
    i32_trunc_f32_s,
    f32,
    i32,
    I32,
    signed,
    true,
    check_bounds_i32_trunc_f32_s
);
trunc_impl!(
    i32_trunc_f32_u,
    f32,
    u32,
    I32,
    unsigned,
    true,
    check_bounds_i32_trunc_f32_u
);
trunc_impl!(
    i32_trunc_f64_s,
    f64,
    i32,
    I32,
    signed,
    true,
    check_bounds_i32_trunc_f64_s
);
trunc_impl!(
    i32_trunc_f64_u,
    f64,
    u32,
    I32,
    unsigned,
    true,
    check_bounds_i32_trunc_f64_u
);
trunc_impl!(
    i64_trunc_f32_s,
    f32,
    i64,
    I64,
    signed,
    true,
    check_bounds_i64_trunc_f32_s
);
trunc_impl!(
    i64_trunc_f32_u,
    f32,
    u64,
    I64,
    unsigned,
    true,
    check_bounds_i64_trunc_f32_u
);
trunc_impl!(
    i64_trunc_f64_s,
    f64,
    i64,
    I64,
    signed,
    true,
    check_bounds_i64_trunc_f64_s
);
trunc_impl!(
    i64_trunc_f64_u,
    f64,
    u64,
    I64,
    unsigned,
    true,
    check_bounds_i64_trunc_f64_u
);

macro_rules! convert_impl {
    ($name:ident, $from_ty:ty, $to_ty:ty, $intermediate_ty:ty, $signedness:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            _frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let val = stack.pop()?.as_::<$from_ty>()?;
            stack.push(Value::$intermediate_ty(val as $to_ty))
        }
    };
}

convert_impl!(f32_convert_i32_s, i32, f32, F32, signed);
convert_impl!(f32_convert_i32_u, u32, f32, F32, unsigned);
convert_impl!(f32_convert_i64_s, i64, f32, F32, signed);
convert_impl!(f32_convert_i64_u, u64, f32, F32, unsigned);
convert_impl!(f64_convert_i32_s, i32, f64, F64, signed);
convert_impl!(f64_convert_i32_u, u32, f64, F64, unsigned);
convert_impl!(f64_convert_i64_s, i64, f64, F64, signed);
convert_impl!(f64_convert_i64_u, u64, f64, F64, unsigned);

pub fn f32_demote_f64(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_f64()?;
    stack.push(Value::F32(val as f32))
}

pub fn f64_promote_f32(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_f32()?;
    stack.push(Value::F64(val as f64))
}

pub fn i32_reinterpret_f32(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_f32()?;
    stack.push(Value::I32(i32::from_ne_bytes(val.to_ne_bytes())))
}

pub fn i64_reinterpret_f64(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_f64()?;
    stack.push(Value::I64(i64::from_ne_bytes(val.to_ne_bytes())))
}

pub fn f32_reinterpret_i32(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    stack.push(Value::F32(f32::from_ne_bytes(val.to_ne_bytes())))
}

pub fn f64_reinterpret_i64(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i64()?;
    stack.push(Value::F64(f64::from_ne_bytes(val.to_ne_bytes())))
}

pub fn i32_extend8_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    stack.push(Value::I32((val as i8) as i32))
}

pub fn i32_extend16_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    stack.push(Value::I32((val as i16) as i32))
}

pub fn i64_extend8_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i64()?;
    stack.push(Value::I64((val as i8) as i64))
}

pub fn i64_extend16_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i64()?;
    stack.push(Value::I64((val as i16) as i64))
}

pub fn i64_extend32_s(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let val = stack.pop()?.as_i64()?;
    stack.push(Value::I64((val as i32) as i64))
}

// Macro to implement saturating truncation operations
macro_rules! trunc_sat_impl {
    ($name:ident, $float_ty:ty, $int_ty:ty, $intermediate_ty:ident, $signedness:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            _frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let val = stack.pop()?.as_float::<$float_ty>()?;
            let result: $int_ty = if val.is_nan() {
                0
            } else if val > (<$int_ty>::MAX as $float_ty) {
                <$int_ty>::MAX
            } else if val < (<$int_ty>::MIN as $float_ty) {
                <$int_ty>::MIN
            } else {
                val as $int_ty // Truncates towards zero (safe after checks)
            };
            stack.push(Value::$intermediate_ty(result as _))
        }
    };
}

trunc_sat_impl!(i32_trunc_sat_f32_s, f32, i32, I32, signed);
trunc_sat_impl!(i32_trunc_sat_f32_u, f32, u32, I32, unsigned);
trunc_sat_impl!(i32_trunc_sat_f64_s, f64, i32, I32, signed);
trunc_sat_impl!(i32_trunc_sat_f64_u, f64, u32, I32, unsigned);
trunc_sat_impl!(i64_trunc_sat_f32_s, f32, i64, I64, signed);
trunc_sat_impl!(i64_trunc_sat_f32_u, f32, u64, I64, unsigned);
trunc_sat_impl!(i64_trunc_sat_f64_s, f64, i64, I64, signed);
trunc_sat_impl!(i64_trunc_sat_f64_u, f64, u64, I64, unsigned);

// SIMD operations
// ... rest of file

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        behavior::{ControlFlowBehavior, FrameBehavior, Label, NullBehavior, StackBehavior},
        error::{Error, Result},
        global::Global,
        stack::Stack,
        table::Table,
        types::{BlockType, FuncType},
        values::Value,
        StacklessEngine,
    };
    use std::any::Any;
    use std::sync::{Arc, MutexGuard};

    #[derive(Debug, Default)]
    struct TestFrame;

    // Filled FrameBehavior implementation
    impl FrameBehavior for TestFrame {
        // FrameBehavior specific methods
        fn locals(&mut self) -> &mut Vec<Value> {
            todo!()
        }
        fn get_local(&self, idx: usize) -> Result<Value> {
            todo!()
        }
        fn set_local(&mut self, idx: usize, value: Value) -> Result<()> {
            todo!()
        }
        fn get_global(&self, idx: usize) -> Result<Arc<Global>> {
            todo!()
        }
        fn set_global(&mut self, idx: usize, value: Value) -> Result<()> {
            todo!()
        }
        fn get_memory(&self, idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
            todo!()
        }
        fn get_memory_mut(&mut self, idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
            todo!()
        }
        fn get_table(&self, idx: usize) -> Result<Arc<Table>> {
            todo!()
        }
        fn get_table_mut(&mut self, idx: usize) -> Result<Arc<Table>> {
            todo!()
        }
        fn get_function_type(&self, func_idx: u32) -> Result<FuncType> {
            todo!()
        }
        fn pc(&self) -> usize {
            todo!()
        }
        fn set_pc(&mut self, pc: usize) {
            todo!()
        }
        fn func_idx(&self) -> u32 {
            todo!()
        }
        fn instance_idx(&self) -> u32 {
            0
        }
        fn locals_len(&self) -> usize {
            todo!()
        }
        fn label_stack(&mut self) -> &mut Vec<Label> {
            todo!()
        }
        fn arity(&self) -> usize {
            todo!()
        }
        fn set_arity(&mut self, arity: usize) {
            todo!()
        }
        fn label_arity(&self) -> usize {
            todo!()
        }
        fn return_pc(&self) -> usize {
            todo!()
        }
        fn set_return_pc(&mut self, pc: usize) {
            todo!()
        }
        fn as_any(&mut self) -> &mut dyn Any {
            todo!()
        }
        fn load_i32(&self, addr: usize, align: u32) -> Result<i32> {
            todo!()
        }
        fn load_i64(&self, addr: usize, align: u32) -> Result<i64> {
            todo!()
        }
        fn load_f32(&self, addr: usize, align: u32) -> Result<f32> {
            todo!()
        }
        fn load_f64(&self, addr: usize, align: u32) -> Result<f64> {
            todo!()
        }
        fn load_i8(&self, addr: usize, align: u32) -> Result<i8> {
            todo!()
        }
        fn load_u8(&self, addr: usize, align: u32) -> Result<u8> {
            todo!()
        }
        fn load_i16(&self, addr: usize, align: u32) -> Result<i16> {
            todo!()
        }
        fn load_u16(&self, addr: usize, align: u32) -> Result<u16> {
            todo!()
        }
        fn load_v128(&self, addr: usize, align: u32) -> Result<[u8; 16]> {
            todo!()
        }
        fn store_i32(&mut self, addr: usize, align: u32, value: i32) -> Result<()> {
            todo!()
        }
        fn store_i64(&mut self, addr: usize, align: u32, value: i64) -> Result<()> {
            todo!()
        }
        fn store_f32(&mut self, addr: usize, align: u32, value: f32) -> Result<()> {
            todo!()
        }
        fn store_f64(&mut self, addr: usize, align: u32, value: f64) -> Result<()> {
            todo!()
        }
        fn store_i8(&mut self, addr: usize, align: u32, value: i8) -> Result<()> {
            todo!()
        }
        fn store_u8(&mut self, addr: usize, align: u32, value: u8) -> Result<()> {
            todo!()
        }
        fn store_i16(&mut self, addr: usize, align: u32, value: i16) -> Result<()> {
            todo!()
        }
        fn store_u16(&mut self, addr: usize, align: u32, value: u16) -> Result<()> {
            todo!()
        }
        fn store_v128(&mut self, addr: usize, align: u32, value: [u8; 16]) -> Result<()> {
            todo!()
        }
        fn memory_size(&self) -> Result<u32> {
            todo!()
        }
        fn memory_grow(&mut self, pages: u32) -> Result<u32> {
            todo!()
        }
        fn table_get(&self, table_idx: u32, idx: u32) -> Result<Value> {
            todo!()
        }
        fn table_set(&mut self, table_idx: u32, idx: u32, value: Value) -> Result<()> {
            todo!()
        }
        fn table_size(&self, table_idx: u32) -> Result<u32> {
            todo!()
        }
        fn table_grow(&mut self, table_idx: u32, delta: u32, value: Value) -> Result<u32> {
            todo!()
        }
        fn table_init(
            &mut self,
            table_idx: u32,
            elem_idx: u32,
            dst: u32,
            src: u32,
            n: u32,
        ) -> Result<()> {
            todo!()
        }
        fn table_copy(
            &mut self,
            dst_table: u32,
            src_table: u32,
            dst: u32,
            src: u32,
            n: u32,
        ) -> Result<()> {
            todo!()
        }
        fn elem_drop(&mut self, elem_idx: u32) -> Result<()> {
            todo!()
        }
        fn table_fill(&mut self, table_idx: u32, dst: u32, val: Value, n: u32) -> Result<()> {
            todo!()
        }
        fn pop_bool(&mut self, stack: &mut dyn Stack) -> Result<bool> {
            todo!()
        }
        fn pop_i32(&mut self, stack: &mut dyn Stack) -> Result<i32> {
            todo!()
        }
        fn get_two_tables_mut(
            &mut self,
            idx1: u32,
            idx2: u32,
        ) -> Result<(MutexGuard<Table>, MutexGuard<Table>)> {
            todo!()
        }
    }

    // Added placeholder implementation
    impl ControlFlowBehavior for TestFrame {
        fn enter_block(&mut self, _ty: BlockType, _stack_len: usize) -> Result<()> {
            Ok(())
        }
        fn enter_loop(&mut self, _ty: BlockType, _stack_len: usize) -> Result<()> {
            Ok(())
        }
        fn enter_if(&mut self, _ty: BlockType, _stack_len: usize, _condition: bool) -> Result<()> {
            Ok(())
        }
        fn enter_else(&mut self, _stack_len: usize) -> Result<()> {
            Ok(())
        }
        fn exit_block(&mut self, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn branch(&mut self, _label_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn return_(&mut self, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn call(&mut self, _func_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn call_indirect(
            &mut self,
            _type_idx: u32,
            _table_idx: u32,
            _entry: u32,
            _stack: &mut dyn Stack,
        ) -> Result<()> {
            Ok(())
        }
        fn set_label_arity(&mut self, _arity: usize) {}
    }

    #[derive(Debug)]
    struct TestStack {
        values: Vec<Value>,
    }

    impl TestStack {
        fn new() -> Self {
            Self { values: Vec::new() }
        }
    }

    impl StackBehavior for TestStack {
        fn push(&mut self, value: Value) -> crate::error::Result<()> {
            self.values.push(value);
            Ok(())
        }

        fn pop(&mut self) -> crate::error::Result<Value> {
            self.values.pop().ok_or(crate::error::Error::StackUnderflow)
        }

        fn peek(&self) -> crate::error::Result<&Value> {
            self.values
                .last()
                .ok_or(crate::error::Error::StackUnderflow)
        }

        fn peek_mut(&mut self) -> crate::error::Result<&mut Value> {
            self.values
                .last_mut()
                .ok_or(crate::error::Error::StackUnderflow)
        }

        fn values(&self) -> &[Value] {
            &self.values
        }

        fn values_mut(&mut self) -> &mut [Value] {
            &mut self.values
        }

        fn len(&self) -> usize {
            self.values.len()
        }

        fn is_empty(&self) -> bool {
            self.values.is_empty()
        }

        fn push_label(&mut self, _arity: usize, _pc: usize) {
            // Not needed for this test
        }

        fn pop_label(&mut self) -> crate::error::Result<crate::behavior::Label> {
            unimplemented!("Not needed for this test")
        }

        fn get_label(&self, _index: usize) -> Option<&crate::behavior::Label> {
            None
        }
    }

    #[test]
    fn test_f32_nearest() {
        let test_cases = [
            // (input, expected output)
            (1.5, 2.0),                             // nearest even
            (2.5, 2.0),                             // nearest even
            (3.5, 4.0),                             // nearest even
            (4.5, 4.0),                             // nearest even
            (-1.5, -2.0),                           // negative nearest even
            (-2.5, -2.0),                           // negative nearest even
            (-3.5, -4.0),                           // negative nearest even
            (1.4, 1.0),                             // regular rounding
            (1.6, 2.0),                             // regular rounding
            (0.0, 0.0),                             // zero stays zero
            (f32::NAN, f32::NAN),                   // NaN stays NaN
            (f32::INFINITY, f32::INFINITY),         // Infinity stays infinity
            (f32::NEG_INFINITY, f32::NEG_INFINITY), // Negative infinity stays negative infinity
        ];

        for (input, expected) in test_cases {
            let mut stack = TestStack::new();
            let mut frame = TestFrame;
            let engine = StacklessEngine::default();

            stack.push(Value::F32(input)).unwrap();
            f32_nearest(&mut stack, &mut frame, &engine).unwrap();

            match stack.pop().unwrap() {
                Value::F32(result) => {
                    if result.is_nan() && expected.is_nan() {
                        // Both are NaN, consider them equal
                    } else {
                        assert_eq!(
                            result, expected,
                            "f32_nearest({}) should return {}",
                            input, expected
                        );
                    }
                }
                _ => panic!("Expected F32 result"),
            }
        }
    }

    #[test]
    fn test_f64_nearest() {
        let test_cases = [
            // (input, expected output)
            (1.5, 2.0),                             // nearest even
            (2.5, 2.0),                             // nearest even
            (3.5, 4.0),                             // nearest even
            (4.5, 4.0),                             // nearest even
            (-1.5, -2.0),                           // negative nearest even
            (-2.5, -2.0),                           // negative nearest even
            (-3.5, -4.0),                           // negative nearest even
            (1.4, 1.0),                             // regular rounding
            (1.6, 2.0),                             // regular rounding
            (0.0, 0.0),                             // zero stays zero
            (f64::NAN, f64::NAN),                   // NaN stays NaN
            (f64::INFINITY, f64::INFINITY),         // Infinity stays infinity
            (f64::NEG_INFINITY, f64::NEG_INFINITY), // Negative infinity stays negative infinity
        ];

        for (input, expected) in test_cases {
            let mut stack = TestStack::new();
            let mut frame = TestFrame;
            let engine = StacklessEngine::default();

            stack.push(Value::F64(input)).unwrap();
            f64_nearest(&mut stack, &mut frame, &engine).unwrap();

            match stack.pop().unwrap() {
                Value::F64(result) => {
                    if result.is_nan() && expected.is_nan() {
                        // Both are NaN, consider them equal
                    } else {
                        assert_eq!(
                            result, expected,
                            "f64_nearest({}) should return {}",
                            input, expected
                        );
                    }
                }
                _ => panic!("Expected F64 result"),
            }
        }
    }
}
