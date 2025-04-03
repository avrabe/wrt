//! SIMD i8x16 operations
//!
//! This module contains implementations for WebAssembly SIMD instructions
//! that operate on 128-bit vectors as sixteen 8-bit integer lanes.

#[cfg(feature = "std")]
use std::cmp;

#[cfg(not(feature = "std"))]
use core::cmp;

use crate::behavior::FrameBehavior;
use crate::error::{Error, Result};
use crate::stack::Stack;
use crate::values::Value;
use crate::StacklessEngine;

use super::common::{pop_v128, push_v128 /*, V128 */};

/// Replicate an i8 value to all lanes of a v128
pub fn i8x16_splat(stack: &mut impl Stack, _frame: &mut impl FrameBehavior) -> Result<()> {
    // Pop the i32 value from the stack and extract the low 8 bits
    let value = match stack.pop()? {
        Value::I32(v) => v as i8,
        _ => return Err(Error::InvalidType("Expected i32 for i8x16.splat".into())),
    };

    // Create the result v128 with the i8 value replicated to all 16 lanes
    let mut result = [0u8; 16];

    // Replicate the byte to all 16 lanes
    let byte = value as u8;
    for i in 0..16 {
        result[i] = byte;
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Extract a lane as a signed i8 from a v128
pub fn i8x16_extract_lane_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 16 {
        return Err(Error::InvalidLaneIndex(16));
    }

    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Extract the i8 value from the specified lane and sign-extend to i32
    let value = v128[lane as usize] as i8 as i32;

    // Push the extracted and sign-extended i32 value to the stack
    stack.push(Value::I32(value))?;
    Ok(())
}

/// Extract a lane as an unsigned i8 from a v128
pub fn i8x16_extract_lane_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 16 {
        return Err(Error::InvalidLaneIndex(16));
    }

    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Extract the i8 value from the specified lane and zero-extend to i32
    let value = v128[lane as usize] as u8 as i32;

    // Push the extracted and zero-extended i32 value to the stack
    stack.push(Value::I32(value))?;
    Ok(())
}

/// Replace a lane in a v128 with an i8 value
pub fn i8x16_replace_lane(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
    lane: u32,
) -> Result<()> {
    // Ensure lane index is in range
    if lane >= 16 {
        return Err(Error::InvalidLaneIndex(16));
    }

    // Pop the replacement i32 value from the stack and truncate to i8
    let replacement = match stack.pop()? {
        Value::I32(v) => v as u8,
        _ => {
            return Err(Error::InvalidType(
                "Expected i32 for replacement value".into(),
            ))
        }
    };

    // Pop the v128 value from the stack
    let mut v128 = pop_v128(stack)?;

    // Replace the specified lane with the i8 value
    v128[lane as usize] = replacement;

    // Push the modified v128 value back to the stack
    push_v128(stack, v128)
}

/// Compare lanes for equality
pub fn i8x16_eq(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i8 lanes for equality
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if a[i] == b[i] { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for inequality
pub fn i8x16_ne(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i8 lanes for inequality
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if a[i] != b[i] { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Add corresponding lanes of two v128 values
pub fn i8x16_add(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Add corresponding i8 lanes
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = a[i].wrapping_add(b[i]);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Subtract corresponding lanes of two v128 values
pub fn i8x16_sub(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Subtract corresponding i8 lanes
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = a[i].wrapping_sub(b[i]);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Negate each lane of a v128 value
pub fn i8x16_neg(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Negate each i8 lane
    let mut result = [0u8; 16];

    for i in 0..16 {
        // Properly handle two's complement negation
        let val = v128[i] as i8;
        result[i] = val.wrapping_neg() as u8;
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for less than (signed)
pub fn i8x16_lt_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i8 lanes for less than (signed)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if (a[i] as i8) < (b[i] as i8) { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for less than (unsigned)
pub fn i8x16_lt_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding u8 lanes for less than (unsigned)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if a[i] < b[i] { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Find minimum of corresponding lanes (signed)
pub fn i8x16_min_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Find minimum of corresponding i8 lanes (signed)
    let mut result = [0u8; 16];

    for i in 0..16 {
        let a_val = a[i] as i8;
        let b_val = b[i] as i8;
        result[i] = cmp::min(a_val, b_val) as u8;
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Find minimum of corresponding lanes (unsigned)
pub fn i8x16_min_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Find minimum of corresponding u8 lanes
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = cmp::min(a[i], b[i]);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Find maximum of corresponding lanes (signed)
pub fn i8x16_max_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Find maximum of corresponding i8 lanes (signed)
    let mut result = [0u8; 16];

    for i in 0..16 {
        let a_val = a[i] as i8;
        let b_val = b[i] as i8;
        result[i] = cmp::max(a_val, b_val) as u8;
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Find maximum of corresponding lanes (unsigned)
pub fn i8x16_max_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Find maximum of corresponding u8 lanes
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = cmp::max(a[i], b[i]);
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for greater than (signed)
///
/// Pops two v128 values (a, b), compares corresponding i8 lanes for a > b,
/// pushes a v128 mask (0xFF if true, 0x00 if false).
pub fn i8x16_gt_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i8 lanes for greater than (signed)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if (a[i] as i8) > (b[i] as i8) { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for greater than (unsigned)
///
/// Pops two v128 values (a, b), compares corresponding u8 lanes for a > b,
/// pushes a v128 mask (0xFF if true, 0x00 if false).
pub fn i8x16_gt_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding u8 lanes for greater than (unsigned)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if a[i] > b[i] { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for less than or equal (signed)
///
/// Pops two v128 values (a, b), compares corresponding i8 lanes for a <= b,
/// pushes a v128 mask (0xFF if true, 0x00 if false).
pub fn i8x16_le_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i8 lanes for less than or equal (signed)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if (a[i] as i8) <= (b[i] as i8) {
            0xFF
        } else {
            0
        };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for less than or equal (unsigned)
///
/// Pops two v128 values (a, b), compares corresponding u8 lanes for a <= b,
/// pushes a v128 mask (0xFF if true, 0x00 if false).
pub fn i8x16_le_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding u8 lanes for less than or equal (unsigned)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if a[i] <= b[i] { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for greater than or equal (signed)
///
/// Pops two v128 values (a, b), compares corresponding i8 lanes for a >= b,
/// pushes a v128 mask (0xFF if true, 0x00 if false).
pub fn i8x16_ge_s(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding i8 lanes for greater than or equal (signed)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if (a[i] as i8) >= (b[i] as i8) {
            0xFF
        } else {
            0
        };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Compare lanes for greater than or equal (unsigned)
///
/// Pops two v128 values (a, b), compares corresponding u8 lanes for a >= b,
/// pushes a v128 mask (0xFF if true, 0x00 if false).
pub fn i8x16_ge_u(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop two v128 values from the stack
    let b = pop_v128(stack)?;
    let a = pop_v128(stack)?;

    // Compare corresponding u8 lanes for greater than or equal (unsigned)
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if a[i] >= b[i] { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Calculate absolute value for each lane
///
/// Pops a v128 value, calculates the absolute value of each signed i8 lane,
/// and pushes the resulting v128.
pub fn i8x16_abs(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Calculate absolute value for each i8 lane
    let mut result = [0u8; 16];

    for i in 0..16 {
        let val = v128[i] as i8;
        result[i] = val.abs() as u8;
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Create bitmask from lanes
///
/// Pops a v128 value, sets the corresponding bit in the result i32
/// if the most significant bit of the lane is set.
pub fn i8x16_bitmask(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Calculate bitmask for each i8 lane
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if v128[i] != 0 { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Check if all lanes are true
///
/// Pops a v128 value, checks if all i8 lanes are non-zero (true),
/// pushes 1 if all are true, 0 otherwise.
pub fn i8x16_all_true(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Check if all i8 lanes are true
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = if v128[i] != 0 { 0xFF } else { 0 };
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

/// Calculate population count for each lane
///
/// Pops a v128 value, calculates the population count (number of set bits)
/// for each i8 lane, and pushes the resulting v128.
pub fn i8x16_popcnt(
    stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // Pop the v128 value from the stack
    let v128 = pop_v128(stack)?;

    // Calculate population count for each i8 lane
    let mut result = [0u8; 16];

    for i in 0..16 {
        result[i] = v128[i].count_ones() as u8;
    }

    // Push the result v128 to the stack
    push_v128(stack, result)
}

// Add stubs for missing functions

pub fn i8x16_shr_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i8x16_shr_s")
}

pub fn i8x16_shr_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i8x16_shr_u")
}

pub fn i8x16_narrow_i16x8_s(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i8x16_narrow_i16x8_s")
}

pub fn i8x16_narrow_i16x8_u(
    _stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    todo!("Implement i8x16_narrow_i16x8_u")
}
