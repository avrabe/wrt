//! SIMD i64x2 operations
//!
//! This module contains implementations for WebAssembly SIMD instructions
//! that operate on 128-bit vectors as two 64-bit integer lanes.

#[cfg(not(feature = "std"))]
use core::cmp;

use crate::behavior::FrameBehavior;
use crate::error::{Error, Result};
use crate::stack::Stack;
use crate::values::Value;
use crate::StacklessEngine;

use super::common::{pop_v128, push_v128, V128};

/// Helper to get an i64 value from a lane in a v128
#[inline]
fn get_i64_lane(v: &V128, lane: usize) -> i64 {
    let offset = lane * 8;
    i64::from_le_bytes([
        v[offset],
        v[offset + 1],
        v[offset + 2],
        v[offset + 3],
        v[offset + 4],
        v[offset + 5],
        v[offset + 6],
        v[offset + 7],
    ])
}

/// Helper to set an i64 value to a lane in a v128
#[inline]
fn set_i64_lane(v: &mut V128, lane: usize, value: i64) {
    let offset = lane * 8;
    let bytes = value.to_le_bytes();
    for i in 0..8 {
        v[offset + i] = bytes[i];
    }
}

/// Replicate an i64 value to all lanes of a v128
pub fn i64x2_splat(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop the i64 value from the stack
    let value = match stack.pop()? {
        Value::I64(v) => v,
        _ => return Err(Error::InvalidType("Expected i64 for i64x2.splat".into())),
    };

    // Create the result v128 with the i64 value replicated to both lanes
    let mut result = [0u8; 16];
    let bytes = value.to_le_bytes();

    // Replicate the 8 bytes to each of the 2 lanes
    for lane in 0..2 {
        let offset = lane * 8;
        for i in 0..8 {
            result[offset + i] = bytes[i];
        }
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Extract a lane as an i64 from a v128
pub fn i64x2_extract_lane(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 2 {
        return Err(Error::InvalidLaneIndex(2));
    }

    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Extract the i64 value from the specified lane
    let value = get_i64_lane(&v128, lane as usize);

    // Push the extracted i64 value to the stack
    stack.push(Value::I64(value))?;
    Ok(())
}

/// Replace a lane in a v128 with an i64 value
pub fn i64x2_replace_lane(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 2 {
        return Err(Error::InvalidLaneIndex(2));
    }

    // Pop the replacement i64 value from the stack
    let replacement = match stack.pop()? {
        Value::I64(v) => v,
        _ => {
            return Err(Error::InvalidType(
                "Expected i64 for replacement value".into(),
            ))
        }
    };

    // Pop the v128 value from the stack
    let mut v128 = pop_v128(stack)?;

    // Replace the specified lane with the i64 value
    set_i64_lane(&mut v128, lane as usize, replacement);

    // Push the modified v128 value back to the stack
    push_v128(stack, v128)
}

/// Add corresponding lanes of two v128 values
pub fn i64x2_add(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Add corresponding i64 lanes
    let mut result = [0u8; 16];

    for i in 0..2 {
        let a_val = get_i64_lane(&a, i);
        let b_val = get_i64_lane(&b, i);
        let sum = a_val.wrapping_add(b_val);
        set_i64_lane(&mut result, i, sum);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Subtract corresponding lanes of two v128 values
pub fn i64x2_sub(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Subtract corresponding i64 lanes
    let mut result = [0u8; 16];

    for i in 0..2 {
        let a_val = get_i64_lane(&a, i);
        let b_val = get_i64_lane(&b, i);
        let diff = a_val.wrapping_sub(b_val);
        set_i64_lane(&mut result, i, diff);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Multiply corresponding lanes of two v128 values
pub fn i64x2_mul(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Multiply corresponding i64 lanes
    let mut result = [0u8; 16];

    for i in 0..2 {
        let a_val = get_i64_lane(&a, i);
        let b_val = get_i64_lane(&b, i);
        let product = a_val.wrapping_mul(b_val);
        set_i64_lane(&mut result, i, product);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Negate each lane of a v128 value
pub fn i64x2_neg(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Negate each i64 lane
    let mut result = [0u8; 16];

    for i in 0..2 {
        let val = get_i64_lane(&v128, i);
        let neg = val.wrapping_neg();
        set_i64_lane(&mut result, i, neg);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

// Add stubs for missing functions

pub fn i64x2_abs(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_abs")
}

pub fn i64x2_shl(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_shl")
}

pub fn i64x2_shr_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_shr_s")
}

pub fn i64x2_shr_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_shr_u")
}

pub fn i64x2_extmul_low_i32x4_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extmul_low_i32x4_s")
}

pub fn i64x2_extmul_high_i32x4_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extmul_high_i32x4_s")
}

pub fn i64x2_extmul_low_i32x4_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extmul_low_i32x4_u")
}

pub fn i64x2_extmul_high_i32x4_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extmul_high_i32x4_u")
}

pub fn i64x2_extend_low_i32x4_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extend_low_i32x4_s")
}

pub fn i64x2_extend_high_i32x4_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extend_high_i32x4_s")
}

pub fn i64x2_extend_low_i32x4_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extend_low_i32x4_u")
}

pub fn i64x2_extend_high_i32x4_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i64x2_extend_high_i32x4_u")
}
