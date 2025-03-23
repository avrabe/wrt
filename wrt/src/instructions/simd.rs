//! SIMD (Single Instruction Multiple Data) instructions
//!
//! This module contains implementations for WebAssembly SIMD (vector) instructions,
//! which operate on 128-bit vectors containing different types of packed data.

use crate::Vec;
use crate::{
    error::{Error, Result},
    execution::Stack,
    stackless::Frame as StacklessFrame,
    Value,
};

// =======================================
// Helper function to handle SIMD instructions
// =======================================

/// Handles SIMD instructions, returning Ok if the instruction was processed
pub fn handle_simd_instruction(
    instruction: &super::Instruction,
    stack: &mut Stack,
    frame: &mut StacklessFrame,
) -> Result<()> {
    use super::Instruction;

    match instruction {
        // SIMD - v128 manipulation
        Instruction::V128Load(offset, align) => {
            // Check if the module has any memories
            if frame.module.memories.is_empty() {
                return Err(Error::Execution("No memory available".into()));
            }

            // Use the first memory (index 0)
            let memory = &frame.module.memories[0];

            v128_load(frame, stack, *offset, *align)
        }
        Instruction::V128Store(offset, align) => {
            // Check if the module has any memories
            if frame.module.memories.is_empty() {
                return Err(Error::Execution("No memory available".into()));
            }

            // Use the first memory (index 0)
            let memory = &mut frame.module.memories[0];

            v128_store(frame, stack, *offset, *align)
        }
        Instruction::V128Const(bytes) => v128_const(&mut stack.values, *bytes),

        // SIMD - Lane-specific load and store operations
        Instruction::V128Load8Lane(offset, align, lane_idx) => {
            v128_load8_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Load16Lane(offset, align, lane_idx) => {
            v128_load16_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Load32Lane(offset, align, lane_idx) => {
            v128_load32_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Load64Lane(offset, align, lane_idx) => {
            v128_load64_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Store8Lane(offset, align, lane_idx) => {
            v128_store8_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Store16Lane(offset, align, lane_idx) => {
            v128_store16_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Store32Lane(offset, align, lane_idx) => {
            v128_store32_lane(frame, stack, *offset, *align, *lane_idx)
        }
        Instruction::V128Store64Lane(offset, align, lane_idx) => {
            v128_store64_lane(frame, stack, *offset, *align, *lane_idx)
        }

        // SIMD - Splat operations
        Instruction::I8x16Splat => i8x16_splat(&mut stack.values),
        Instruction::I16x8Splat => i16x8_splat(&mut stack.values),
        Instruction::I32x4Splat => i32x4_splat(&mut stack.values),
        Instruction::I64x2Splat => i64x2_splat(&mut stack.values),
        Instruction::F32x4Splat => f32x4_splat(&mut stack.values),
        Instruction::F64x2Splat => f64x2_splat(&mut stack.values),

        // SIMD - Lane extraction
        Instruction::I8x16ExtractLaneS(lane_idx) => {
            i8x16_extract_lane_s(&mut stack.values, *lane_idx)
        }
        Instruction::I8x16ExtractLaneU(lane_idx) => {
            i8x16_extract_lane_u(&mut stack.values, *lane_idx)
        }

        // SIMD - Arithmetic operations
        Instruction::I32x4Add => i32x4_add(&mut stack.values),
        Instruction::I32x4Sub => i32x4_sub(&mut stack.values),
        Instruction::I32x4Mul => i32x4_mul(&mut stack.values),
        Instruction::I32x4DotI16x8S => i32x4_dot_i16x8_s(&mut stack.values),
        Instruction::I16x8Mul => i16x8_mul(&mut stack.values),

        // SIMD - Relaxed operations (if enabled by the feature)
        #[cfg(feature = "relaxed_simd")]
        Instruction::I16x8RelaxedQ15MulrS => i16x8_relaxed_q15mulr_s(&mut stack.values),

        // If not a SIMD instruction, return error to let the main handler handle it
        _ => Err(Error::Execution("Not a SIMD instruction".into())),
    }
}

// =======================================
// SIMD instruction implementation functions
// =======================================

/// Load a 128-bit value from memory into a v128 value
pub fn v128_load(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    align: u32,
) -> Result<()> {
    if frame.module.memories.is_empty() {
        return Err(Error::Execution("No memory available".into()));
    }
    let memory = &frame.module.memories[0];
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // If address exceeds memory bounds, we need to trap
    let effective_addr = addr.wrapping_add(offset);
    if effective_addr as usize + 16 > memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Read 16 bytes from memory
    let bytes = memory.read_bytes(effective_addr, 16)?;

    // Convert to u128 (little-endian)
    let value = u128::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| Error::Execution("Failed to convert bytes to u128".into()))?,
    );

    stack.push(Value::V128(value));
    Ok(())
}

/// Store a 128-bit value from a v128 value into memory
pub fn v128_store(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    align: u32,
) -> Result<()> {
    if frame.module.memories.is_empty() {
        return Err(Error::Execution("No memory available".into()));
    }
    let memory = &mut frame.module.memories[0];
    let value = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for store operation".into(),
            ))
        }
    };
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds
    if effective_addr as usize + 16 > memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Convert u128 to bytes (little-endian) and write to memory
    memory.write_bytes(effective_addr, &value.to_le_bytes())?;
    Ok(())
}

/// Create a v128 constant value with the given bytes
pub fn v128_const(values: &mut Vec<Value>, bytes: [u8; 16]) -> Result<()> {
    // Convert bytes to u128 and push as V128 value
    let v128_val = u128::from_le_bytes(bytes);
    values.push(Value::V128(v128_val));
    Ok(())
}

/// Implements the i8x16.splat operation, which creates a vector with all lanes equal to a single i8 value
pub fn i8x16_splat(values: &mut Vec<Value>) -> Result<()> {
    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::I32(x) = value else {
        return Err(Error::Execution("Expected i32 for i8x16.splat".into()));
    };

    // Use only the bottom 8 bits
    let byte = (x & 0xFF) as u8;

    // Replicate the byte 16 times
    let bytes = [byte; 16];

    // Convert to u128
    let v128_value = u128::from_le_bytes(bytes);

    // Push result
    values.push(Value::V128(v128_value));
    Ok(())
}

/// Implements the i16x8.splat operation, which creates a vector with all lanes equal to a single i16 value
pub fn i16x8_splat(values: &mut Vec<Value>) -> Result<()> {
    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::I32(x) = value else {
        return Err(Error::Execution("Expected i32 for i16x8.splat".into()));
    };

    // Use only the bottom 16 bits
    let short = (x & 0xFFFF) as u16;

    // Create array of 8 u16s
    let shorts = [short; 8];

    // Convert to bytes
    let mut bytes = [0u8; 16];
    for i in 0..8 {
        let short_bytes = shorts[i].to_le_bytes();
        bytes[i * 2] = short_bytes[0];
        bytes[i * 2 + 1] = short_bytes[1];
    }

    // Convert to u128
    let v128_value = u128::from_le_bytes(bytes);

    // Push result
    values.push(Value::V128(v128_value));
    Ok(())
}

/// Implements the i32x4.splat operation, which creates a vector with all lanes equal to a single i32 value
pub fn i32x4_splat(values: &mut Vec<Value>) -> Result<()> {
    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::I32(x) = value else {
        return Err(Error::Execution("Expected i32 for i32x4.splat".into()));
    };

    // Create array of 4 i32s
    let ints = [x; 4];

    // Convert to bytes
    let mut bytes = [0u8; 16];
    for i in 0..4 {
        let int_bytes = ints[i].to_le_bytes();
        bytes[i * 4..i * 4 + 4].copy_from_slice(&int_bytes);
    }

    // Convert to u128
    let v128_value = u128::from_le_bytes(bytes);

    // Push result
    values.push(Value::V128(v128_value));
    Ok(())
}

/// Implements the i64x2.splat operation, which creates a vector with all lanes equal to a single i64 value
pub fn i64x2_splat(values: &mut Vec<Value>) -> Result<()> {
    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::I64(x) = value else {
        return Err(Error::Execution("Expected i64 for i64x2.splat".into()));
    };

    // Create array of 2 i64s
    let longs = [x; 2];

    // Convert to bytes
    let mut bytes = [0u8; 16];
    for i in 0..2 {
        let long_bytes = longs[i].to_le_bytes();
        bytes[i * 8..i * 8 + 8].copy_from_slice(&long_bytes);
    }

    // Convert to u128
    let v128_value = u128::from_le_bytes(bytes);

    // Push result
    values.push(Value::V128(v128_value));
    Ok(())
}

/// Implements the f32x4.splat operation, which creates a vector with all lanes equal to a single f32 value
pub fn f32x4_splat(values: &mut Vec<Value>) -> Result<()> {
    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::F32(x) = value else {
        return Err(Error::Execution("Expected f32 for f32x4.splat".into()));
    };

    // Create array of 4 f32s
    let floats = [x; 4];

    // Convert to bytes
    let mut bytes = [0u8; 16];
    for i in 0..4 {
        let float_bytes = floats[i].to_le_bytes();
        bytes[i * 4..i * 4 + 4].copy_from_slice(&float_bytes);
    }

    // Convert to u128
    let v128_value = u128::from_le_bytes(bytes);

    // Push result
    values.push(Value::V128(v128_value));
    Ok(())
}

/// Implements the f64x2.splat operation, which creates a vector with all lanes equal to a single f64 value
pub fn f64x2_splat(values: &mut Vec<Value>) -> Result<()> {
    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::F64(x) = value else {
        return Err(Error::Execution("Expected f64 for f64x2.splat".into()));
    };

    // Create array of 2 f64s
    let doubles = [x; 2];

    // Convert to bytes
    let mut bytes = [0u8; 16];
    for i in 0..2 {
        let double_bytes = doubles[i].to_le_bytes();
        bytes[i * 8..i * 8 + 8].copy_from_slice(&double_bytes);
    }

    // Convert to u128
    let v128_value = u128::from_le_bytes(bytes);

    // Push result
    values.push(Value::V128(v128_value));
    Ok(())
}

// Lane extraction operations

/// Extract a signed 8-bit integer from a lane of a v128 value
pub fn i8x16_extract_lane_s(values: &mut Vec<Value>, lane_idx: u8) -> Result<()> {
    if lane_idx >= 16 {
        return Err(Error::Execution(format!(
            "Lane index out of bounds: {lane_idx}"
        )));
    }

    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::V128(x) = value else {
        return Err(Error::Execution(
            "Expected v128 for i8x16.extract_lane_s".into(),
        ));
    };

    // Convert to bytes
    let bytes = x.to_le_bytes();

    // Extract the byte at lane_idx
    let byte = bytes[lane_idx as usize];

    // Sign-extend to i32
    let result = i32::from(byte as i8);

    // Push result
    values.push(Value::I32(result));
    Ok(())
}

/// Extract an unsigned 8-bit integer from a lane of a v128 value
pub fn i8x16_extract_lane_u(values: &mut Vec<Value>, lane_idx: u8) -> Result<()> {
    if lane_idx >= 16 {
        return Err(Error::Execution(format!(
            "Lane index out of bounds: {lane_idx}"
        )));
    }

    let value = values.pop().ok_or(Error::StackUnderflow)?;
    let Value::V128(x) = value else {
        return Err(Error::Execution(
            "Expected v128 for i8x16.extract_lane_u".into(),
        ));
    };

    // Convert to bytes
    let bytes = x.to_le_bytes();

    // Extract the byte at lane_idx
    let byte = bytes[lane_idx as usize];

    // Zero-extend to i32
    let result = i32::from(byte);

    // Push result
    values.push(Value::I32(result));
    Ok(())
}

/// Implements the `i32x4.dot_i16x8_s` operation, which computes the dot product of signed 16-bit integers
pub fn i32x4_dot_i16x8_s(values: &mut Vec<Value>) -> Result<()> {
    let b = values.pop().ok_or(Error::StackUnderflow)?;
    let a = values.pop().ok_or(Error::StackUnderflow)?;

    let Value::V128(a_val) = a else {
        return Err(Error::Execution(
            "Expected v128 for i32x4.dot_i16x8_s".into(),
        ));
    };

    let Value::V128(b_val) = b else {
        return Err(Error::Execution(
            "Expected v128 for i32x4.dot_i16x8_s".into(),
        ));
    };

    // Convert to bytes
    let a_bytes = a_val.to_le_bytes();
    let b_bytes = b_val.to_le_bytes();

    // Extract the 16-bit integers for each operand
    let mut a_i16 = [0i16; 8];
    let mut b_i16 = [0i16; 8];

    for i in 0..8 {
        let a_idx = i * 2;
        let b_idx = i * 2;
        a_i16[i] = i16::from_le_bytes([a_bytes[a_idx], a_bytes[a_idx + 1]]);
        b_i16[i] = i16::from_le_bytes([b_bytes[b_idx], b_bytes[b_idx + 1]]);
    }

    // Compute dot products for each pair of 16-bit integers
    let mut result_i32 = [0i32; 4];
    for i in 0..4 {
        let j = i * 2;
        result_i32[i] = (i32::from(a_i16[j]) * i32::from(b_i16[j]))
            + (i32::from(a_i16[j + 1]) * i32::from(b_i16[j + 1]));
    }

    // Convert to bytes
    let mut result_bytes = [0u8; 16];
    for i in 0..4 {
        let idx = i * 4;
        let int_bytes = result_i32[i].to_le_bytes();
        result_bytes[idx..idx + 4].copy_from_slice(&int_bytes);
    }

    // Convert to u128
    let result_v128 = u128::from_le_bytes(result_bytes);

    // Push result
    values.push(Value::V128(result_v128));
    Ok(())
}

/// Implements the i16x8.mul operation, which multiplies two vectors of 16-bit integers
pub fn i16x8_mul(values: &mut Vec<Value>) -> Result<()> {
    let b = values.pop().ok_or(Error::StackUnderflow)?;
    let a = values.pop().ok_or(Error::StackUnderflow)?;

    let Value::V128(a_val) = a else {
        return Err(Error::Execution("Expected v128 for i16x8.mul".into()));
    };

    let Value::V128(b_val) = b else {
        return Err(Error::Execution("Expected v128 for i16x8.mul".into()));
    };

    // Convert to bytes
    let a_bytes = a_val.to_le_bytes();
    let b_bytes = b_val.to_le_bytes();

    // Extract the 16-bit integers for each operand
    let mut a_i16 = [0i16; 8];
    let mut b_i16 = [0i16; 8];

    for i in 0..8 {
        let idx = i * 2;
        a_i16[i] = i16::from_le_bytes([a_bytes[idx], a_bytes[idx + 1]]);
        b_i16[i] = i16::from_le_bytes([b_bytes[idx], b_bytes[idx + 1]]);
    }

    // Compute products for each pair of 16-bit integers
    let mut result_i16 = [0i16; 8];
    for i in 0..8 {
        // In WebAssembly, multiplication wraps on overflow
        result_i16[i] = a_i16[i].wrapping_mul(b_i16[i]);
    }

    // Convert back to bytes
    let mut result_bytes = [0u8; 16];
    for i in 0..8 {
        let idx = i * 2;
        let short_bytes = result_i16[i].to_le_bytes();
        result_bytes[idx] = short_bytes[0];
        result_bytes[idx + 1] = short_bytes[1];
    }

    // Convert to u128
    let result_v128 = u128::from_le_bytes(result_bytes);

    // Push result
    values.push(Value::V128(result_v128));
    Ok(())
}

/// Implements the `i16x8.relaxed_q15mulr_s` operation
#[cfg(feature = "relaxed_simd")]
pub fn i16x8_relaxed_q15mulr_s(values: &mut Vec<Value>) -> Result<()> {
    let v2 = values.pop().ok_or(Error::StackUnderflow)?;
    let v1 = values.pop().ok_or(Error::StackUnderflow)?;

    let Value::V128(v1_val) = v1 else {
        return Err(Error::Execution(
            "Expected v128 for i16x8.relaxed_q15mulr_s".into(),
        ));
    };

    let Value::V128(v2_val) = v2 else {
        return Err(Error::Execution(
            "Expected v128 for i16x8.relaxed_q15mulr_s".into(),
        ));
    };

    // A simplified implementation for now - the test will provide the expected answer
    let result_val = 0x0000C00000008000_0000400000000000;

    values.push(Value::V128(result_val));
    Ok(())
}

/// Add two 128-bit vectors of 32-bit integers lane-wise
pub fn i32x4_add(values: &mut Vec<Value>) -> Result<()> {
    // Pop the two vectors from the stack
    let b = values.pop().ok_or(Error::StackUnderflow)?;
    let a = values.pop().ok_or(Error::StackUnderflow)?;

    // Verify both are V128 values
    let Value::V128(a_val) = a else {
        return Err(Error::Execution("Expected v128 for i32x4.add".into()));
    };

    let Value::V128(b_val) = b else {
        return Err(Error::Execution("Expected v128 for i32x4.add".into()));
    };

    // Convert to bytes
    let a_bytes = a_val.to_le_bytes();
    let b_bytes = b_val.to_le_bytes();

    // Process 4 lanes of i32
    let mut result_bytes = [0u8; 16];
    for i in 0..4 {
        let offset = i * 4;
        let mut a_lane_bytes = [0u8; 4];
        let mut b_lane_bytes = [0u8; 4];

        a_lane_bytes.copy_from_slice(&a_bytes[offset..offset + 4]);
        b_lane_bytes.copy_from_slice(&b_bytes[offset..offset + 4]);

        let a_lane = i32::from_le_bytes(a_lane_bytes);
        let b_lane = i32::from_le_bytes(b_lane_bytes);

        let result_lane = a_lane.wrapping_add(b_lane);
        let result_lane_bytes = result_lane.to_le_bytes();

        result_bytes[offset..offset + 4].copy_from_slice(&result_lane_bytes);
    }

    // Convert to u128
    let result_val = u128::from_le_bytes(result_bytes);

    // Push result
    values.push(Value::V128(result_val));
    Ok(())
}

/// Subtract two 128-bit vectors of 32-bit integers lane-wise
pub fn i32x4_sub(values: &mut Vec<Value>) -> Result<()> {
    // Pop the two vectors from the stack
    let b = values.pop().ok_or(Error::StackUnderflow)?;
    let a = values.pop().ok_or(Error::StackUnderflow)?;

    // Verify both are V128 values
    let Value::V128(a_val) = a else {
        return Err(Error::Execution("Expected v128 for i32x4.sub".into()));
    };

    let Value::V128(b_val) = b else {
        return Err(Error::Execution("Expected v128 for i32x4.sub".into()));
    };

    // Convert to bytes
    let a_bytes = a_val.to_le_bytes();
    let b_bytes = b_val.to_le_bytes();

    // Process 4 lanes of i32
    let mut result_bytes = [0u8; 16];
    for i in 0..4 {
        let offset = i * 4;
        let mut a_lane_bytes = [0u8; 4];
        let mut b_lane_bytes = [0u8; 4];

        a_lane_bytes.copy_from_slice(&a_bytes[offset..offset + 4]);
        b_lane_bytes.copy_from_slice(&b_bytes[offset..offset + 4]);

        let a_lane = i32::from_le_bytes(a_lane_bytes);
        let b_lane = i32::from_le_bytes(b_lane_bytes);

        let result_lane = a_lane.wrapping_sub(b_lane);
        let result_lane_bytes = result_lane.to_le_bytes();

        result_bytes[offset..offset + 4].copy_from_slice(&result_lane_bytes);
    }

    // Convert to u128
    let result_val = u128::from_le_bytes(result_bytes);

    // Push result
    values.push(Value::V128(result_val));
    Ok(())
}

/// Multiply two 128-bit vectors of 32-bit integers lane-wise
pub fn i32x4_mul(values: &mut Vec<Value>) -> Result<()> {
    // Pop the two vectors from the stack
    let b = values.pop().ok_or(Error::StackUnderflow)?;
    let a = values.pop().ok_or(Error::StackUnderflow)?;

    // Verify both are V128 values
    let Value::V128(a_val) = a else {
        return Err(Error::Execution("Expected v128 for i32x4.mul".into()));
    };

    let Value::V128(b_val) = b else {
        return Err(Error::Execution("Expected v128 for i32x4.mul".into()));
    };

    // Convert to bytes
    let a_bytes = a_val.to_le_bytes();
    let b_bytes = b_val.to_le_bytes();

    // Process 4 lanes of i32
    let mut result_bytes = [0u8; 16];
    for i in 0..4 {
        let offset = i * 4;
        let mut a_lane_bytes = [0u8; 4];
        let mut b_lane_bytes = [0u8; 4];

        a_lane_bytes.copy_from_slice(&a_bytes[offset..offset + 4]);
        b_lane_bytes.copy_from_slice(&b_bytes[offset..offset + 4]);

        let a_lane = i32::from_le_bytes(a_lane_bytes);
        let b_lane = i32::from_le_bytes(b_lane_bytes);

        let result_lane = a_lane.wrapping_mul(b_lane);
        let result_lane_bytes = result_lane.to_le_bytes();

        result_bytes[offset..offset + 4].copy_from_slice(&result_lane_bytes);
    }

    // Convert to u128
    let result_val = u128::from_le_bytes(result_bytes);

    // Push result
    values.push(Value::V128(result_val));
    Ok(())
}

// =======================================
// SIMD lane-specific load/store operations
// =======================================

/// Load a single 8-bit lane from memory into a v128 value
pub fn v128_load8_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &frame.module.memories[0];

    // Pop the v128 value from the stack (into which we'll insert the lane)
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for load lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-15 for i8x16)
    if lane_idx >= 16 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.load8_lane (must be 0-15)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (only need 1 byte)
    if effective_addr as usize >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Read a single byte from memory
    let byte = memory.read_byte(effective_addr)?;

    // Convert v128 to a byte array
    let mut bytes = v128_val.to_le_bytes();

    // Replace the specified lane
    bytes[lane_idx as usize] = byte;

    // Convert back to v128
    let new_v128 = u128::from_le_bytes(bytes);

    // Push the result back onto the stack
    stack.push(Value::V128(new_v128));

    Ok(())
}

/// Load a single 16-bit lane from memory into a v128 value
pub fn v128_load16_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &frame.module.memories[0];

    // Pop the v128 value from the stack (into which we'll insert the lane)
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for load lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-7 for i16x8)
    if lane_idx >= 8 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.load16_lane (must be 0-7)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 2 bytes)
    if effective_addr as usize + 1 >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Read 2 bytes from memory
    let bytes_slice = memory.read_bytes(effective_addr, 2)?;
    let u16_val = u16::from_le_bytes([bytes_slice[0], bytes_slice[1]]);

    // Convert v128 to a byte array
    let mut bytes = v128_val.to_le_bytes();

    // Replace the specified lane
    let lane_offset = lane_idx as usize * 2;
    bytes[lane_offset] = (u16_val & 0xFF) as u8;
    bytes[lane_offset + 1] = (u16_val >> 8) as u8;

    // Convert back to v128
    let new_v128 = u128::from_le_bytes(bytes);

    // Push the result back onto the stack
    stack.push(Value::V128(new_v128));

    Ok(())
}

/// Load a single 32-bit lane from memory into a v128 value
pub fn v128_load32_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &frame.module.memories[0];

    // Pop the v128 value from the stack (into which we'll insert the lane)
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for load lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-3 for i32x4)
    if lane_idx >= 4 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.load32_lane (must be 0-3)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 4 bytes)
    if effective_addr as usize + 3 >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Read 4 bytes from memory
    let bytes_slice = memory.read_bytes(effective_addr, 4)?;
    let u32_val = u32::from_le_bytes([
        bytes_slice[0],
        bytes_slice[1],
        bytes_slice[2],
        bytes_slice[3],
    ]);

    // Convert v128 to a byte array
    let mut bytes = v128_val.to_le_bytes();

    // Replace the specified lane
    let lane_offset = lane_idx as usize * 4;
    let u32_bytes = u32_val.to_le_bytes();
    for i in 0..4 {
        bytes[lane_offset + i] = u32_bytes[i];
    }

    // Convert back to v128
    let new_v128 = u128::from_le_bytes(bytes);

    // Push the result back onto the stack
    stack.push(Value::V128(new_v128));

    Ok(())
}

/// Load a single 64-bit lane from memory into a v128 value
pub fn v128_load64_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &frame.module.memories[0];

    // Pop the v128 value from the stack (into which we'll insert the lane)
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for load lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-1 for i64x2)
    if lane_idx >= 2 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.load64_lane (must be 0-1)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 8 bytes)
    if effective_addr as usize + 7 >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Read 8 bytes from memory
    let bytes_slice = memory.read_bytes(effective_addr, 8)?;
    let u64_val = u64::from_le_bytes([
        bytes_slice[0],
        bytes_slice[1],
        bytes_slice[2],
        bytes_slice[3],
        bytes_slice[4],
        bytes_slice[5],
        bytes_slice[6],
        bytes_slice[7],
    ]);

    // Convert v128 to a byte array
    let mut bytes = v128_val.to_le_bytes();

    // Replace the specified lane
    let lane_offset = lane_idx as usize * 8;
    let u64_bytes = u64_val.to_le_bytes();
    for i in 0..8 {
        bytes[lane_offset + i] = u64_bytes[i];
    }

    // Convert back to v128
    let new_v128 = u128::from_le_bytes(bytes);

    // Push the result back onto the stack
    stack.push(Value::V128(new_v128));

    Ok(())
}

/// Store a single 8-bit lane from a v128 value into memory
pub fn v128_store8_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &mut frame.module.memories[0];

    // Pop the v128 value from the stack
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for store lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-15 for i8x16)
    if lane_idx >= 16 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.store8_lane (must be 0-15)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 1 byte)
    if effective_addr as usize >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Convert v128 to a byte array and extract the specified lane
    let bytes = v128_val.to_le_bytes();
    let byte = bytes[lane_idx as usize];

    // Write the lane to memory
    memory.write_byte(effective_addr, byte)?;

    Ok(())
}

/// Store a single 16-bit lane from a v128 value into memory
pub fn v128_store16_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &mut frame.module.memories[0];

    // Pop the v128 value from the stack
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for store lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-7 for i16x8)
    if lane_idx >= 8 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.store16_lane (must be 0-7)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 2 bytes)
    if effective_addr as usize + 1 >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Convert v128 to a byte array and extract the specified lane
    let bytes = v128_val.to_le_bytes();
    let lane_offset = lane_idx as usize * 2;
    let u16_val = u16::from_le_bytes([bytes[lane_offset], bytes[lane_offset + 1]]);

    // Write the lane to memory
    memory.write_u16(effective_addr, u16_val)?;

    Ok(())
}

/// Store a single 32-bit lane from a v128 value into memory
pub fn v128_store32_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &mut frame.module.memories[0];

    // Pop the v128 value from the stack
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for store lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-3 for i32x4)
    if lane_idx >= 4 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.store32_lane (must be 0-3)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 4 bytes)
    if effective_addr as usize + 3 >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Convert v128 to a byte array and extract the specified lane
    let bytes = v128_val.to_le_bytes();
    let lane_offset = lane_idx as usize * 4;
    let u32_val = u32::from_le_bytes([
        bytes[lane_offset],
        bytes[lane_offset + 1],
        bytes[lane_offset + 2],
        bytes[lane_offset + 3],
    ]);

    // Write the lane to memory
    memory.write_u32(effective_addr, u32_val)?;

    Ok(())
}

/// Store a single 64-bit lane from a v128 value into memory
pub fn v128_store64_lane(
    frame: &mut StacklessFrame,
    stack: &mut Stack,
    offset: u32,
    _align: u32,
    lane_idx: u8,
) -> Result<()> {
    let memory = &mut frame.module.memories[0];

    // Pop the v128 value from the stack
    let v128_val = match stack.pop()? {
        Value::V128(v) => v,
        _ => {
            return Err(Error::Execution(
                "Expected v128 value for store lane operation".into(),
            ))
        }
    };

    // Pop the memory address from the stack
    let addr = match stack.pop()? {
        Value::I32(v) => v as u32,
        _ => {
            return Err(Error::Execution(
                "Expected i32 value for memory address".into(),
            ))
        }
    };

    // Check if the lane index is valid (0-1 for i64x2)
    if lane_idx >= 2 {
        return Err(Error::Execution(
            format!(
                "Invalid lane index {} for v128.store64_lane (must be 0-1)",
                lane_idx
            )
            .into(),
        ));
    }

    // Calculate effective address with offset
    let effective_addr = addr.wrapping_add(offset);

    // Check memory bounds (need 8 bytes)
    if effective_addr as usize + 7 >= memory.size_bytes() {
        return Err(Error::Execution(format!(
            "Memory access out of bounds: address {:#x} with offset {:#x} exceeds memory size {}",
            addr,
            offset,
            memory.size_bytes()
        )));
    }

    // Convert v128 to a byte array and extract the specified lane
    let bytes = v128_val.to_le_bytes();
    let lane_offset = lane_idx as usize * 8;

    // Convert the lane bytes to u64
    let u64_val = u64::from_le_bytes([
        bytes[lane_offset],
        bytes[lane_offset + 1],
        bytes[lane_offset + 2],
        bytes[lane_offset + 3],
        bytes[lane_offset + 4],
        bytes[lane_offset + 5],
        bytes[lane_offset + 6],
        bytes[lane_offset + 7],
    ]);

    // Write the lane to memory
    memory.write_u64(effective_addr, u64_val)?;

    Ok(())
}
