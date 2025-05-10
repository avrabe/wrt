//! Integration tests for memory safety implementations.
//!
//! These tests verify that our memory and table implementations properly use safe memory structures.

use wrt_error::Result;
use wrt_runtime::memory::Memory;
use wrt_runtime::table::Table;
use wrt_runtime::types::{MemoryType, TableType};
use wrt_types::safe_memory::{SafeMemoryHandler, SafeSlice};
use wrt_types::types::{Limits, ValueType};
use wrt_types::values::Value;
use wrt_types::verification::VerificationLevel;

#[test]
fn test_memory_with_safe_memory_handler() -> Result<()> {
    // Create a memory type with 1 page (64KB)
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };

    // Create a memory instance
    let mut memory = Memory::new(mem_type)?;

    // Set verification level
    memory.set_verification_level(VerificationLevel::Full);

    // Test data to write
    let test_data = [1, 2, 3, 4, 5];

    // Write the data
    memory.write(0, &test_data)?;

    // Read it back using safe slice
    let safe_slice = memory.get_safe_slice(0, test_data.len())?;
    let slice_data = safe_slice.data()?;
    assert_eq!(slice_data, test_data);

    // Write with full verification
    memory.write(10, &test_data)?;

    // Read it back
    let mut buffer = [0; 5];
    memory.read(10, &mut buffer)?;
    assert_eq!(buffer, test_data);

    // Verify memory integrity
    memory.verify_integrity()?;

    Ok(())
}

#[test]
fn test_table_safe_operations() -> Result<()> {
    // Create a table type
    let table_type =
        TableType { element_type: ValueType::FuncRef, limits: Limits { min: 5, max: Some(10) } };

    // Create a table
    let mut table = Table::new(table_type, Value::func_ref(None))?;

    // Set verification level
    table.set_verification_level(VerificationLevel::Full);

    // Set some values
    table.set(1, Some(Value::func_ref(Some(42))))?;
    table.set(2, Some(Value::func_ref(Some(43))))?;

    // Get them back
    let val1 = table.get(1)?;
    let val2 = table.get(2)?;

    // Verify values
    assert_eq!(val1, Some(Value::func_ref(Some(42))));
    assert_eq!(val2, Some(Value::func_ref(Some(43))));

    // Test fill operation
    table.fill_elements(3, Some(Value::func_ref(Some(99))), 2)?;
    assert_eq!(table.get(3)?, Some(Value::func_ref(Some(99))));
    assert_eq!(table.get(4)?, Some(Value::func_ref(Some(99))));

    // Test copy operation
    table.copy_elements(0, 3, 2)?;
    assert_eq!(table.get(0)?, Some(Value::func_ref(Some(99))));
    assert_eq!(table.get(1)?, Some(Value::func_ref(Some(99))));

    Ok(())
}

#[test]
fn test_memory_read_write_large_buffer() -> Result<()> {
    // Create a memory type with 2 pages (128KB)
    let mem_type = MemoryType { limits: Limits { min: 2, max: Some(4) } };

    // Create a memory instance
    let mut memory = Memory::new(mem_type)?;

    // Create a large buffer that will use the optimized write path
    let large_buffer = vec![42u8; 1024];

    // Write the large buffer
    memory.write(1000, &large_buffer)?;

    // Read it back
    let mut read_buffer = vec![0u8; large_buffer.len()];
    memory.read(1000, &mut read_buffer)?;

    // Verify the data
    assert_eq!(read_buffer, large_buffer);

    // Create an even larger buffer that spans page boundaries
    let huge_buffer = vec![99u8; 65000];

    // Write the huge buffer
    memory.write(1000, &huge_buffer)?;

    // Read it back
    let mut read_huge_buffer = vec![0u8; huge_buffer.len()];
    memory.read(1000, &mut read_huge_buffer)?;

    // Verify the data
    assert_eq!(read_huge_buffer, huge_buffer);

    Ok(())
}

#[test]
fn test_memory_copy_within() -> Result<()> {
    // Create a memory type with 1 page (64KB)
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };

    // Create a memory instance
    let mut memory = Memory::new(mem_type)?;

    // Initial data
    let initial_data = [10, 20, 30, 40, 50];
    memory.write(100, &initial_data)?;

    // Copy within the same memory
    memory.copy_within_or_between(
        std::sync::Arc::new(memory.clone()),
        100,
        200,
        initial_data.len(),
    )?;

    // Verify the copied data
    let mut buffer = [0; 5];
    memory.read(200, &mut buffer)?;
    assert_eq!(buffer, initial_data);

    // Test overlapping copy (should handle correctly)
    memory.copy_within_or_between(std::sync::Arc::new(memory.clone()), 200, 202, 3)?;

    // Verify the result of overlapping copy
    let mut overlap_buffer = [0; 3];
    memory.read(202, &mut overlap_buffer)?;
    assert_eq!(overlap_buffer, [10, 20, 30]);

    Ok(())
}

#[test]
fn test_memory_fill() -> Result<()> {
    // Create a memory type with 1 page (64KB)
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };

    // Create a memory instance
    let mut memory = Memory::new(mem_type)?;

    // Fill a region with a specific value
    let fill_value = 0x42;
    let fill_size = 1000;
    memory.fill(500, fill_value, fill_size)?;

    // Verify the fill worked correctly
    let mut buffer = vec![0; fill_size];
    memory.read(500, &mut buffer)?;

    for value in buffer {
        assert_eq!(value, fill_value);
    }

    Ok(())
}
