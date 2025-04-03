//! SIMD f32x4 operations
//!
//! This module contains implementations for WebAssembly SIMD instructions
//! that operate on 128-bit vectors as four 32-bit floating point lanes.

#[cfg(not(feature = "std"))]
use core::cmp;

use crate::behavior::FrameBehavior;
use crate::error::{Error, Result};
use crate::stack::Stack;
use crate::values::Value;
use crate::StacklessEngine;

use super::common::{pop_v128, push_v128, V128};

/// Helper to get an f32 value from a lane in a v128
#[inline]
const fn get_f32_lane(v: &V128, lane: usize) -> f32 {
    let offset = lane * 4;
    f32::from_le_bytes([v[offset], v[offset + 1], v[offset + 2], v[offset + 3]])
}

/// Helper to set an f32 value to a lane in a v128
#[inline]
fn set_f32_lane(v: &mut V128, lane: usize, value: f32) {
    let offset = lane * 4;
    let bytes = value.to_le_bytes();
    v[offset] = bytes[0];
    v[offset + 1] = bytes[1];
    v[offset + 2] = bytes[2];
    v[offset + 3] = bytes[3];
}

/// Replicate an f32 value to all lanes of a v128
pub fn f32x4_splat(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop the f32 value from the stack
    let value = match stack.pop()? {
        Value::F32(v) => v,
        _ => return Err(Error::InvalidType("Expected f32 for f32x4.splat".into())),
    };

    // Create the result v128 with the f32 value replicated to all 4 lanes
    let mut result = [0u8; 16];
    let bytes = value.to_le_bytes();

    // Replicate the 4 bytes to each of the 4 lanes
    for i in 0..4 {
        let offset = i * 4;
        result[offset] = bytes[0];
        result[offset + 1] = bytes[1];
        result[offset + 2] = bytes[2];
        result[offset + 3] = bytes[3];
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Extract a lane as an f32 from a v128
pub fn f32x4_extract_lane(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 4 {
        return Err(Error::InvalidLaneIndex(4));
    }

    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Extract the f32 value from the specified lane
    let value = get_f32_lane(&v128, lane as usize);

    // Push the extracted f32 value to the stack
    stack.push(Value::F32(value))?;
    Ok(())
}

/// Replace a lane in a v128 with an f32 value
pub fn f32x4_replace_lane(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 4 {
        return Err(Error::InvalidLaneIndex(4));
    }

    // Pop the replacement f32 value from the stack
    let replacement = match stack.pop()? {
        Value::F32(v) => v,
        _ => {
            return Err(Error::InvalidType(
                "Expected f32 for replacement value".into(),
            ))
        }
    };

    // Pop the v128 value from the stack
    let mut v128 = pop_v128(stack)?;

    // Replace the specified lane with the f32 value
    set_f32_lane(&mut v128, lane as usize, replacement);

    // Push the modified v128 value back to the stack
    push_v128(stack, v128)
}

/// Add corresponding lanes of two v128 values
pub fn f32x4_add(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Add corresponding f32 lanes
    let mut result = [0u8; 16];

    for i in 0..4 {
        let a_val = get_f32_lane(&a, i);
        let b_val = get_f32_lane(&b, i);
        let sum = a_val + b_val;
        set_f32_lane(&mut result, i, sum);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Subtract corresponding lanes of two v128 values
pub fn f32x4_sub(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Subtract corresponding f32 lanes
    let mut result = [0u8; 16];

    for i in 0..4 {
        let a_val = get_f32_lane(&a, i);
        let b_val = get_f32_lane(&b, i);
        let diff = a_val - b_val;
        set_f32_lane(&mut result, i, diff);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Multiply corresponding lanes of two v128 values
pub fn f32x4_mul(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Multiply corresponding f32 lanes
    let mut result = [0u8; 16];

    for i in 0..4 {
        let a_val = get_f32_lane(&a, i);
        let b_val = get_f32_lane(&b, i);
        let product = a_val * b_val;
        set_f32_lane(&mut result, i, product);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Divide corresponding lanes of two v128 values
pub fn f32x4_div(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Divide corresponding f32 lanes
    let mut result = [0u8; 16];

    for i in 0..4 {
        let a_val = get_f32_lane(&a, i);
        let b_val = get_f32_lane(&b, i);
        let quotient = a_val / b_val;
        set_f32_lane(&mut result, i, quotient);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Negate each lane of a v128 value
pub fn f32x4_neg(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Negate each f32 lane
    let mut result = [0u8; 16];

    for i in 0..4 {
        let val = get_f32_lane(&v128, i);
        let neg = -val;
        set_f32_lane(&mut result, i, neg);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

// Add stubs for missing functions

pub fn f32x4_min(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_min")
}

pub fn f32x4_max(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_max")
}

pub fn f32x4_pmin(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_pmin")
}

pub fn f32x4_pmax(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_pmax")
}

pub fn f32x4_convert_i32x4_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_convert_i32x4_s")
}

pub fn f32x4_convert_i32x4_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_convert_i32x4_u")
}

pub fn f32x4_demote_f64x2_zero(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement f32x4_demote_f64x2_zero")
}
