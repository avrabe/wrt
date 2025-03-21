//! WebAssembly memory instructions
//!
//! This module contains implementations for all WebAssembly memory instructions,
//! including loads, stores, and memory management operations.

use crate::error::Error;
use crate::memory::Memory;
use crate::Value;
use crate::Vec;

/// Execute an i32 load instruction
///
/// Loads a 32-bit integer from memory at the specified address.
pub fn i32_load(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 4 bytes from the effective address
    if effective_addr as usize + 4 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read bytes and convert to little-endian i32
    let bytes = &memory.data[effective_addr as usize..effective_addr as usize + 4];
    let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

    stack.push(Value::I32(value));
    Ok(())
}

/// Execute an i64 load instruction
///
/// Loads a 64-bit integer from memory at the specified address.
pub fn i64_load(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 8 bytes from the effective address
    if effective_addr as usize + 8 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read bytes and convert to little-endian i64
    let bytes = &memory.data[effective_addr as usize..effective_addr as usize + 8];
    let value = i64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);

    stack.push(Value::I64(value));
    Ok(())
}

/// Execute an f32 load instruction
///
/// Loads a 32-bit float from memory at the specified address.
pub fn f32_load(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 4 bytes from the effective address
    if effective_addr as usize + 4 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read bytes and convert to little-endian f32
    let bytes = &memory.data[effective_addr as usize..effective_addr as usize + 4];
    let bits = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let value = f32::from_bits(bits);

    stack.push(Value::F32(value));
    Ok(())
}

/// Execute an f64 load instruction
///
/// Loads a 64-bit float from memory at the specified address.
pub fn f64_load(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 8 bytes from the effective address
    if effective_addr as usize + 8 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read bytes and convert to little-endian f64
    let bytes = &memory.data[effective_addr as usize..effective_addr as usize + 8];
    let bits = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);
    let value = f64::from_bits(bits);

    stack.push(Value::F64(value));
    Ok(())
}

/// Execute an i32 load8_s instruction
///
/// Loads an 8-bit signed integer from memory and sign-extends it to 32 bits.
pub fn i32_load8_s(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 1 byte from the effective address
    if effective_addr as usize >= memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read byte and sign-extend to i32
    let byte = memory.data[effective_addr as usize] as i8;
    let value = byte as i32;

    stack.push(Value::I32(value));
    Ok(())
}

/// Execute an i32 load8_u instruction
///
/// Loads an 8-bit unsigned integer from memory and zero-extends it to 32 bits.
pub fn i32_load8_u(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 1 byte from the effective address
    if effective_addr as usize >= memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read byte and zero-extend to i32
    let byte = memory.data[effective_addr as usize];
    let value = byte as i32;

    stack.push(Value::I32(value));
    Ok(())
}

/// Execute an i32 load16_s instruction
///
/// Loads a 16-bit signed integer from memory and sign-extends it to 32 bits.
pub fn i32_load16_s(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 2 bytes from the effective address
    if effective_addr as usize + 2 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read bytes and convert to little-endian i16, then sign-extend to i32
    let bytes = &memory.data[effective_addr as usize..effective_addr as usize + 2];
    let value = i16::from_le_bytes([bytes[0], bytes[1]]) as i32;

    stack.push(Value::I32(value));
    Ok(())
}

/// Execute an i32 load16_u instruction
///
/// Loads a 16-bit unsigned integer from memory and zero-extends it to 32 bits.
pub fn i32_load16_u(
    stack: &mut Vec<Value>,
    memory: &Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we can read 2 bytes from the effective address
    if effective_addr as usize + 2 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Read bytes and convert to little-endian u16, then zero-extend to i32
    let bytes = &memory.data[effective_addr as usize..effective_addr as usize + 2];
    let value = u16::from_le_bytes([bytes[0], bytes[1]]) as i32;

    stack.push(Value::I32(value));
    Ok(())
}

/// Execute an i32 store instruction
///
/// Stores a 32-bit integer to memory at the specified address.
pub fn i32_store(
    stack: &mut Vec<Value>,
    memory: &mut Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    // Pop the value and address from the stack
    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we have a valid i32 value
    let i32_value = match value {
        Value::I32(v) => v,
        _ => return Err(Error::Execution("Expected i32 value".into())),
    };

    // Check if we can write 4 bytes to the effective address
    if effective_addr as usize + 4 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Convert i32 to little-endian bytes and write to memory
    let bytes = i32_value.to_le_bytes();
    memory.data[effective_addr as usize..effective_addr as usize + 4].copy_from_slice(&bytes);

    Ok(())
}

/// Execute an i64 store instruction
///
/// Stores a 64-bit integer to memory at the specified address.
pub fn i64_store(
    stack: &mut Vec<Value>,
    memory: &mut Memory,
    offset: u32,
    _align: u32,
) -> Result<(), Error> {
    // Pop the value and address from the stack
    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let addr = pop_memory_address(stack)?;
    let effective_addr = addr.wrapping_add(offset);

    // Check if we have a valid i64 value
    let i64_value = match value {
        Value::I64(v) => v,
        _ => return Err(Error::Execution("Expected i64 value".into())),
    };

    // Check if we can write 8 bytes to the effective address
    if effective_addr as usize + 8 > memory.data.len() {
        return Err(Error::Execution("Memory access out of bounds".into()));
    }

    // Convert i64 to little-endian bytes and write to memory
    let bytes = i64_value.to_le_bytes();
    memory.data[effective_addr as usize..effective_addr as usize + 8].copy_from_slice(&bytes);

    Ok(())
}

/// Execute memory size instruction
///
/// Returns the current size of memory in pages (64KB per page).
pub fn memory_size(stack: &mut Vec<Value>, memory: &Memory) -> Result<(), Error> {
    // Memory size in pages (64KB per page)
    let size_in_pages = memory.data.len() / 65536;
    stack.push(Value::I32(size_in_pages as i32));
    Ok(())
}

/// Execute memory grow instruction
///
/// Grows memory by the specified number of pages, returns previous size or -1 on failure.
pub fn memory_grow(stack: &mut Vec<Value>, memory: &mut Memory) -> Result<(), Error> {
    // Pop the number of pages to grow
    let pages = match stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    {
        Value::I32(pages) => pages as usize,
        _ => return Err(Error::Execution("Expected i32 value".into())),
    };

    // Calculate current size in pages
    let current_pages = memory.data.len() / 65536;

    // Check if growing would exceed max memory size
    if let Some(max_pages) = memory.type_().max {
        if current_pages + pages > max_pages as usize {
            // Growth failed, return -1
            stack.push(Value::I32(-1));
            return Ok(());
        }
    }

    // Grow the memory
    let additional_bytes = pages * 65536;
    memory.data.resize(memory.data.len() + additional_bytes, 0);

    // Return the previous size in pages
    stack.push(Value::I32(current_pages as i32));
    Ok(())
}

/// Pop a memory address from the stack
///
/// Helper function to get a memory address from the stack.
fn pop_memory_address(stack: &mut Vec<Value>) -> Result<u32, Error> {
    match stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    {
        Value::I32(addr) => Ok(addr as u32),
        _ => Err(Error::Execution("Expected i32 address".into())),
    }
}
