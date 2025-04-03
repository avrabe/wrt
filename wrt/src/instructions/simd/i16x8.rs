//! SIMD i16x8 operations
//!
//! This module contains implementations for WebAssembly SIMD instructions
//! that operate on 128-bit vectors as eight 16-bit integer lanes.

#[cfg(feature = "std")]
use std::cmp;

#[cfg(not(feature = "std"))]
use core::cmp;

use crate::behavior::FrameBehavior;
use crate::error::{Error, Result};
use crate::stack::Stack;
use crate::values::Value;
use crate::StacklessEngine;

use super::common::{pop_v128, push_v128, V128};

/// Helper to get an i16 value from a lane in a v128
#[inline]
pub fn get_i16_lane(v: &V128, lane: usize) -> i16 {
    let offset = lane * 2;
    i16::from_le_bytes([v[offset], v[offset + 1]])
}

/// Helper to get an u16 value from a lane in a v128
#[inline]
fn get_u16_lane(v: &V128, lane: usize) -> u16 {
    let offset = lane * 2;
    u16::from_le_bytes([v[offset], v[offset + 1]])
}

/// Helper to set an i16 value to a lane in a v128
#[inline]
fn set_i16_lane(v: &mut V128, lane: usize, value: i16) {
    let offset = lane * 2;
    let bytes = value.to_le_bytes();
    v[offset] = bytes[0];
    v[offset + 1] = bytes[1];
}

/// Replicate an i16 value to all lanes of a v128
pub fn i16x8_splat(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop the i32 value from the stack and extract the low 16 bits
    let value = match stack.pop()? {
        Value::I32(v) => v as i16,
        _ => return Err(Error::InvalidType("Expected i32 for i16x8.splat".into())),
    };

    // Create the result v128 with the i16 value replicated to all 8 lanes
    let mut result = [0u8; 16];
    let bytes = value.to_le_bytes();

    // Replicate the 2 bytes to each of the 8 lanes
    for i in 0..8 {
        let offset = i * 2;
        result[offset] = bytes[0];
        result[offset + 1] = bytes[1];
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Extract a lane as a signed i16 from a v128
pub fn i16x8_extract_lane_s(
    stack: &mut impl Stack,
    _frame: &mut impl FrameBehavior,
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 8 {
        return Err(Error::InvalidLaneIndex(8));
    }

    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Extract the i16 value from the specified lane and sign-extend to i32
    let value = get_i16_lane(&v128, lane as usize) as i32;

    // Push the extracted and sign-extended i32 value to the stack
    stack.push(Value::I32(value))?;
    Ok(())
}

/// Extract a lane as an unsigned i16 from a v128
pub fn i16x8_extract_lane_u(
    stack: &mut impl Stack,
    _frame: &mut impl FrameBehavior,
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 8 {
        return Err(Error::InvalidLaneIndex(8));
    }

    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Extract the i16 value from the specified lane and zero-extend to i32
    let value = get_u16_lane(&v128, lane as usize) as i32;

    // Push the extracted and zero-extended i32 value to the stack
    stack.push(Value::I32(value))?;
    Ok(())
}

/// Replace a lane in a v128 with an i16 value
pub fn i16x8_replace_lane(
    stack: &mut impl Stack,
    _frame: &mut impl FrameBehavior,
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 8 {
        return Err(Error::InvalidLaneIndex(8));
    }

    // Pop the replacement i32 value from the stack and truncate to i16
    let replacement = match stack.pop()? {
        Value::I32(v) => v as i16,
        _ => {
            return Err(Error::InvalidType(
                "Expected i32 for replacement value".into(),
            ))
        }
    };

    // Pop the v128 value from the stack
    let mut v128 = pop_v128(stack)?;

    // Replace the specified lane with the i16 value
    set_i16_lane(&mut v128, lane as usize, replacement);

    // Push the modified v128 value back to the stack
    push_v128(stack, v128)
}

/// Add corresponding lanes of two v128 values
pub fn i16x8_add(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Add corresponding i16 lanes
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let sum = a_val.wrapping_add(b_val);
        set_i16_lane(&mut result, i, sum);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Subtract corresponding lanes of two v128 values
pub fn i16x8_sub(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Subtract corresponding i16 lanes
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let diff = a_val.wrapping_sub(b_val);
        set_i16_lane(&mut result, i, diff);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Multiply corresponding lanes of two v128 values
pub fn i16x8_mul(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Multiply corresponding i16 lanes
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let product = a_val.wrapping_mul(b_val);
        set_i16_lane(&mut result, i, product);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Negate each lane of a v128 value
pub fn i16x8_neg(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Negate each i16 lane
    let mut result = [0u8; 16];

    for i in 0..8 {
        let val = get_i16_lane(&v128, i);
        let neg = val.wrapping_neg();
        set_i16_lane(&mut result, i, neg);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for equality
pub fn i16x8_eq(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i16 lanes for equality
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let mask = if a_val == b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for inequality
pub fn i16x8_ne(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i16 lanes for inequality
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let mask = if a_val != b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for less than (signed)
pub fn i16x8_lt_s(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let mask = if a_val < b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for less than (unsigned)
pub fn i16x8_lt_u(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_u16_lane(&a, i);
        let b_val = get_u16_lane(&b, i);
        let mask = if a_val < b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Find minimum of corresponding lanes (signed)
pub fn i16x8_min_s(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        set_i16_lane(&mut result, i, cmp::min(a_val, b_val));
    }

    push_v128(stack, result)
}

/// Find minimum of corresponding lanes (unsigned)
pub fn i16x8_min_u(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_u16_lane(&a, i);
        let b_val = get_u16_lane(&b, i);
        set_i16_lane(&mut result, i, cmp::min(a_val, b_val) as i16);
    }

    push_v128(stack, result)
}

/// Find maximum of corresponding lanes (signed)
pub fn i16x8_max_s(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        set_i16_lane(&mut result, i, cmp::max(a_val, b_val));
    }

    push_v128(stack, result)
}

/// Find maximum of corresponding lanes (unsigned)
pub fn i16x8_max_u(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_u16_lane(&a, i);
        let b_val = get_u16_lane(&b, i);
        set_i16_lane(&mut result, i, cmp::max(a_val, b_val) as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for greater than (signed)
pub fn i16x8_gt_s(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let mask = if a_val > b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for greater than (unsigned)
pub fn i16x8_gt_u(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_u16_lane(&a, i);
        let b_val = get_u16_lane(&b, i);
        let mask = if a_val > b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for less than or equal (signed)
pub fn i16x8_le_s(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let mask = if a_val <= b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for less than or equal (unsigned)
pub fn i16x8_le_u(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_u16_lane(&a, i);
        let b_val = get_u16_lane(&b, i);
        let mask = if a_val <= b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for greater than or equal (signed)
pub fn i16x8_ge_s(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_i16_lane(&a, i);
        let b_val = get_i16_lane(&b, i);
        let mask = if a_val >= b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

/// Compare lanes for greater than or equal (unsigned)
pub fn i16x8_ge_u(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;
    let mut result = [0u8; 16];

    for i in 0..8 {
        let a_val = get_u16_lane(&a, i);
        let b_val = get_u16_lane(&b, i);
        let mask = if a_val >= b_val { 0xFFFF } else { 0 };
        set_i16_lane(&mut result, i, mask as i16);
    }

    push_v128(stack, result)
}

// Add stubs for missing functions

pub fn i16x8_shr_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_shr_s")
}

pub fn i16x8_shr_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_shr_u")
}

pub fn i16x8_narrow_i32x4_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_narrow_i32x4_s")
}

pub fn i16x8_narrow_i32x4_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_narrow_i32x4_u")
}

pub fn i16x8_extend_low_i8x16_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_extend_low_i8x16_s")
}

pub fn i16x8_extend_high_i8x16_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_extend_high_i8x16_s")
}

pub fn i16x8_extend_low_i8x16_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_extend_low_i8x16_u")
}

pub fn i16x8_extend_high_i8x16_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i16x8_extend_high_i8x16_u")
}
