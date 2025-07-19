//! Consolidated Memory Safety Tests for WRT
//!
//! This module consolidates all memory safety test functionality from across the WRT project,
//! eliminating duplication and providing comprehensive testing in a single location.

#![cfg(test)]

use std::sync::Arc;
use wrt_runtime::memory::Memory;
use wrt_runtime::table::Table;
use wrt_runtime::types::{MemoryType, TableType};
use wrt_foundation::safe_memory::{SafeMemoryHandler, SafeStack, SafeSlice, MemoryProvider};
use wrt_foundation::verification::VerificationLevel;
use wrt_foundation::types::{Limits, ValueType, RefType};
use wrt_foundation::values::Value;
use wrt_error::Result;

#[cfg(not(feature = "std"))]
use wrt_foundation::safe_memory::NoStdMemoryProvider;
#[cfg(feature = "std")]
use wrt_foundation::safe_memory::StdMemoryProvider;

// ===========================================
// SHARED MEMORY TESTING UTILITIES
// ===========================================

/// Create a standard memory type for testing
pub fn create_test_memory_type() -> MemoryType {
    MemoryType {
        limits: Limits { min: 1, max: Some(2) },
    }
}

/// Create a standard table type for testing
pub fn create_test_table_type() -> TableType {
    TableType {
        element_type: RefType::Funcref,
        limits: Limits { min: 10, max: Some(20) },
    }
}

/// Create test data for memory operations
pub fn create_test_data() -> Vec<u8> {
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
}

/// Test verification levels comprehensively
pub fn test_all_verification_levels<F>(mut test_fn: F) 
where 
    F: FnMut(VerificationLevel) -> Result<()>,
{
    let levels = [
        VerificationLevel::Off,
        VerificationLevel::Basic,
        VerificationLevel::Standard,
        VerificationLevel::Full,
        VerificationLevel::Critical,
    ];
    
    for level in &levels {
        test_fn(*level).expect(&format!("Test failed for verification level: {:?}", level);
    }
}

// ===========================================
// CORE MEMORY SAFETY TESTS (from safe_memory_test.rs)
// ===========================================

mod core_memory_tests {
    use super::*;

    #[test]
    fn test_memory_safe_operations() -> Result<()> {
        let mem_type = create_test_memory_type);
        let mut memory = Memory::new(mem_type)?;
        
        let test_data = create_test_data);
        
        // Write the data
        memory.write(0, &test_data[..5])?;
        
        // Read it back using safe slice
        let safe_slice = memory.get_safe_slice(0, 5)?;
        let slice_data = safe_slice.data()?;
        assert_eq!(slice_data, &test_data[..5];
        
        // Verify integrity
        memory.verify_integrity()?;
        
        // Test with different verification levels
        test_all_verification_levels(|level| {
            memory.set_verification_level(level;
            assert_eq!(memory.verification_level(), level;
            
            // Write with this verification level
            memory.write(10, &test_data[..5])?;
            
            // Read it back
            let mut buffer = [0; 5];
            memory.read(10, &mut buffer)?;
            assert_eq!(buffer, test_data[..5];
            
            Ok(())
        };

        Ok(())
    }

    #[test]
    fn test_table_safe_operations() -> Result<()> {
        let table_type = create_test_table_type);
        let mut table = Table::new(table_type)?;
        
        // Test setting and getting values
        let func_value = Value::FuncRef(42;
        table.set(0, func_value.clone())?;
        
        let retrieved = table.get(0)?;
        assert_eq!(retrieved, func_value;
        
        // Test bounds checking
        let result = table.get(100;
        assert!(result.is_err(), "Should fail for out-of-bounds access");
        
        Ok(())
    }

    #[test]
    fn test_safe_memory_handler() -> Result<()> {
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Full)?;
        
        let test_data = create_test_data);
        
        // Allocate memory
        let memory_id = handler.allocate(test_data.len())?;
        
        // Write data
        handler.write(memory_id, 0, &test_data)?;
        
        // Read data back
        let mut buffer = vec![0; test_data.len()];
        handler.read(memory_id, 0, &mut buffer)?;
        assert_eq!(buffer, test_data;
        
        // Test verification
        handler.verify_all()?;
        
        // Clean up
        handler.deallocate(memory_id)?;
        
        Ok(())
    }
}

// ===========================================
// FOUNDATION MEMORY TESTS (from foundation safe_memory tests)
// ===========================================

mod foundation_memory_tests {
    use super::*;

    #[test]
    fn test_safe_slice_creation() {
        let data = create_test_data);
        let slice_res = SafeSlice::new(&data;
        assert!(slice_res.is_ok(), "Slice creation failed");
        let slice = slice_res.unwrap();

        // Verify data access works
        assert_eq!(slice.data().unwrap(), &data;
        assert_eq!(slice.len(), data.len);
        assert!(!slice.is_empty();
    }

    #[test]
    fn test_safe_slice_verification_levels() {
        let data = create_test_data);

        test_all_verification_levels(|level| {
            let slice = SafeSlice::with_verification_level(&data, level)?;
            assert_eq!(slice.data()?, &data;
            assert_eq!(slice.verification_level(), level;
            Ok(())
        };
    }

    #[test]
    fn test_safe_slice_bounds_checking() {
        let data = create_test_data);
        let slice = SafeSlice::new(&data).unwrap();

        // Test valid access
        let subslice = slice.subslice(2, 5;
        assert!(subslice.is_ok();
        assert_eq!(subslice.unwrap().data().unwrap(), &data[2..7];

        // Test invalid access
        let invalid_subslice = slice.subslice(5, 20;
        assert!(invalid_subslice.is_err(), "Should fail for out-of-bounds access");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_std_memory_provider() {
        let data = create_test_data);
        let provider = StdMemoryProvider::new(data.clone();

        // Test borrow_slice
        let slice = provider.borrow_slice(0, 5).unwrap();
        assert_eq!(slice, &data[..5];

        // Test with different verification levels
        test_all_verification_levels(|level| {
            let provider_with_level = StdMemoryProvider::with_verification_level(data.clone(), level;
            let slice = provider_with_level.borrow_slice(0, 5)?;
            assert_eq!(slice, &data[..5];
            Ok(())
        };
    }

    #[cfg(not(feature = "std"))]
    #[test]
    fn test_nostd_memory_provider() {
        let provider = NoStdMemoryProvider::<64>::new);

        // Binary std/no_std choice
        let memory_id = provider.allocate(32).unwrap();
        
        // Test write/read
        let test_data = [1, 2, 3, 4, 5];
        provider.write(memory_id, 0, &test_data).unwrap();
        
        let mut buffer = [0; 5];
        provider.read(memory_id, 0, &mut buffer).unwrap();
        assert_eq!(buffer, test_data;
        
        // Clean up
        provider.deallocate(memory_id).unwrap();
    }

    #[test]
    fn test_safe_stack_operations() {
        let mut stack = SafeStack::new(VerificationLevel::Full;

        let test_values = [10, 20, 30, 40, 50];

        // Push values
        for &value in &test_values {
            stack.push(value).unwrap();
        }

        assert_eq!(stack.len(), test_values.len);
        assert!(!stack.is_empty();

        // Pop values (should be in reverse order)
        for &expected in test_values.iter().rev() {
            let value = stack.pop().unwrap();
            assert_eq!(value, Some(expected;
        }

        assert_eq!(stack.len(), 0;
        assert!(stack.is_empty();

        // Pop from empty stack
        let empty_pop = stack.pop().unwrap();
        assert_eq!(empty_pop, None;
    }

    #[test]
    fn test_safe_stack_verification_levels() {
        test_all_verification_levels(|level| {
            let mut stack = SafeStack::new(level;
            
            stack.push(42)?;
            assert_eq!(stack.verification_level(), level;
            
            let value = stack.pop()?.unwrap();
            assert_eq!(value, 42;
            
            Ok(())
        };
    }
}

// ===========================================
// RUNTIME MEMORY SAFETY TESTS
// ===========================================

mod runtime_memory_tests {
    use super::*;

    #[test]
    fn test_memory_growth_safety() -> Result<()> {
        let mem_type = create_test_memory_type);
        let mut memory = Memory::new(mem_type)?;
        
        // Test initial size
        assert_eq!(memory.size(), 1); // 1 page
        
        // Test growth within limits
        memory.grow(1)?; // Grow by 1 page
        assert_eq!(memory.size(), 2); // Now 2 pages
        
        // Test growth beyond limits (should fail)
        let result = memory.grow(1;
        assert!(result.is_err(), "Should fail when growing beyond max limit");
        
        Ok(())
    }

    #[test]
    fn test_memory_bounds_checking() -> Result<()> {
        let mem_type = create_test_memory_type);
        let memory = Memory::new(mem_type)?;
        
        let test_data = create_test_data);
        
        // Test valid write
        assert!(memory.write(0, &test_data[..5]).is_ok();
        
        // Test write at boundary
        let page_size = 65536; // 64KB
        let boundary_write = memory.write(page_size - 5, &test_data[..5];
        assert!(boundary_write.is_ok();
        
        // Test out-of-bounds write
        let oob_write = memory.write(page_size, &test_data;
        assert!(oob_write.is_err(), "Should fail for out-of-bounds write");
        
        Ok(())
    }

    #[test]
    fn test_memory_integrity_verification() -> Result<()> {
        let mem_type = create_test_memory_type);
        let mut memory = Memory::new(mem_type)?;
        
        // Test with different verification levels
        test_all_verification_levels(|level| {
            memory.set_verification_level(level;
            
            // Write some data
            let test_data = create_test_data);
            memory.write(0, &test_data[..5])?;
            
            // Verify integrity
            memory.verify_integrity()?;
            
            // Read back and verify
            let mut buffer = [0; 5];
            memory.read(0, &mut buffer)?;
            assert_eq!(buffer, test_data[..5];
            
            Ok(())
        };

        Ok(())
    }

    #[test]
    fn test_table_bounds_and_growth() -> Result<()> {
        let table_type = create_test_table_type);
        let mut table = Table::new(table_type)?;
        
        // Test initial size
        assert_eq!(table.size(), 10;
        
        // Test valid access
        let func_value = Value::FuncRef(42;
        table.set(5, func_value.clone())?;
        assert_eq!(table.get(5)?, func_value;
        
        // Test bounds checking
        assert!(table.get(10).is_err(), "Should fail for out-of-bounds get");
        assert!(table.set(10, func_value.clone()).is_err(), "Should fail for out-of-bounds set");
        
        // Test growth
        table.grow(5, Value::FuncRef(0))?;
        assert_eq!(table.size(), 15;
        
        // Test growth beyond limits
        let result = table.grow(10, Value::FuncRef(0;
        assert!(result.is_err(), "Should fail when growing beyond max limit");
        
        Ok(())
    }
}

// ===========================================
// INTEGRATION MEMORY TESTS
// ===========================================

mod memory_integration_tests {
    use super::*;

    #[test]
    fn test_memory_table_interaction() -> Result<()> {
        let mem_type = create_test_memory_type);
        let table_type = create_test_table_type);
        
        let memory = Memory::new(mem_type)?;
        let mut table = Table::new(table_type)?;
        
        // Write data to memory
        let test_data = create_test_data);
        memory.write(0, &test_data[..5])?;
        
        // Store function references in table
        for i in 0..5 {
            table.set(i, Value::FuncRef(i as u32))?;
        }
        
        // Read back and verify both work together
        let mut buffer = [0; 5];
        memory.read(0, &mut buffer)?;
        assert_eq!(buffer, test_data[..5];
        
        for i in 0..5 {
            let func_ref = table.get(i)?;
            assert_eq!(func_ref, Value::FuncRef(i as u32;
        }
        
        Ok(())
    }

    #[test]
    fn test_cross_component_memory_safety() -> Result<()> {
        // Test that different memory components maintain safety independently
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Full)?;
        let memory = Memory::new(create_test_memory_type())?;
        
        let test_data = create_test_data);
        
        // Use both memory systems
        let memory_id = handler.allocate(test_data.len())?;
        handler.write(memory_id, 0, &test_data)?;
        
        memory.write(0, &test_data[..5])?;
        
        // Verify both maintain integrity
        handler.verify_all()?;
        memory.verify_integrity()?;
        
        // Read from both
        let mut handler_buffer = vec![0; test_data.len()];
        handler.read(memory_id, 0, &mut handler_buffer)?;
        assert_eq!(handler_buffer, test_data;
        
        let mut memory_buffer = [0; 5];
        memory.read(0, &mut memory_buffer)?;
        assert_eq!(memory_buffer, test_data[..5];
        
        // Clean up
        handler.deallocate(memory_id)?;
        
        Ok(())
    }

    #[test]
    fn test_verification_level_consistency() -> Result<()> {
        // Test that verification levels work consistently across components
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            // Test SafeSlice
            let data = create_test_data);
            let slice = SafeSlice::with_verification_level(&data, *level)?;
            assert_eq!(slice.verification_level(), *level;
            
            // Test SafeStack
            let stack = SafeStack::new(*level;
            assert_eq!(stack.verification_level(), *level;
            
            // Test SafeMemoryHandler
            let handler = SafeMemoryHandler::new(*level)?;
            assert_eq!(handler.verification_level(), *level;
            
            // Test Memory
            let mut memory = Memory::new(create_test_memory_type())?;
            memory.set_verification_level(*level;
            assert_eq!(memory.verification_level(), *level;
        }
        
        Ok(())
    }
}

// ===========================================
// PERFORMANCE MEMORY TESTS
// ===========================================

mod memory_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_memory_operation_performance() -> Result<()> {
        let mem_type = create_test_memory_type);
        let memory = Memory::new(mem_type)?;
        
        let test_data = vec![0u8; 1024]; // 1KB test data
        
        let start = Instant::now);
        
        // Perform many memory operations
        for i in 0..1000 {
            let offset = (i % 60) * 1024; // Stay within bounds
            memory.write(offset, &test_data)?;
        }
        
        let duration = start.elapsed);
        assert!(duration.as_millis() < 100, "Memory write performance regression");
        
        let start = Instant::now);
        
        // Perform many read operations
        for i in 0..1000 {
            let offset = (i % 60) * 1024;
            let mut buffer = vec![0u8; 1024];
            memory.read(offset, &mut buffer)?;
        }
        
        let duration = start.elapsed);
        assert!(duration.as_millis() < 100, "Memory read performance regression");
        
        Ok(())
    }

    #[test]
    fn test_verification_performance_impact() -> Result<()> {
        let mem_type = create_test_memory_type);
        let mut memory = Memory::new(mem_type)?;
        
        let test_data = vec![42u8; 1024];
        
        // Test with verification off
        memory.set_verification_level(VerificationLevel::Off;
        let start = Instant::now);
        for _ in 0..1000 {
            memory.write(0, &test_data)?;
        }
        let off_duration = start.elapsed);
        
        // Test with full verification
        memory.set_verification_level(VerificationLevel::Full;
        let start = Instant::now);
        for _ in 0..1000 {
            memory.write(0, &test_data)?;
        }
        let full_duration = start.elapsed);
        
        // Full verification should be slower but not excessively so
        assert!(full_duration > off_duration, "Full verification should have some overhead");
        assert!(full_duration.as_millis() < off_duration.as_millis() * 10, "Verification overhead too high");
        
        Ok(())
    }

    #[test]
    fn test_safe_stack_performance() -> Result<()> {
        let mut stack = SafeStack::new(VerificationLevel::Standard;
        
        let start = Instant::now);
        
        // Push many values
        for i in 0..10000 {
            stack.push(i)?;
        }
        
        // Pop all values
        for _ in 0..10000 {
            stack.pop()?;
        }
        
        let duration = start.elapsed);
        assert!(duration.as_millis() < 100, "SafeStack performance regression");
        
        Ok(())
    }
}

// ===========================================
// ERROR HANDLING MEMORY TESTS
// ===========================================

mod memory_error_tests {
    use super::*;

    #[test]
    fn test_memory_error_recovery() -> Result<()> {
        let mem_type = create_test_memory_type);
        let memory = Memory::new(mem_type)?;
        
        let test_data = create_test_data);
        
        // Cause an error with out-of-bounds write
        let error_result = memory.write(100000, &test_data;
        assert!(error_result.is_err();
        
        // Verify memory is still usable after error
        assert!(memory.write(0, &test_data[..5]).is_ok();
        
        let mut buffer = [0; 5];
        assert!(memory.read(0, &mut buffer).is_ok();
        assert_eq!(buffer, test_data[..5];
        
        Ok(())
    }

    #[test]
    fn test_table_error_recovery() -> Result<()> {
        let table_type = create_test_table_type);
        let mut table = Table::new(table_type)?;
        
        let func_value = Value::FuncRef(42;
        
        // Cause an error with out-of-bounds access
        let error_result = table.set(100, func_value.clone();
        assert!(error_result.is_err();
        
        // Verify table is still usable after error
        assert!(table.set(5, func_value.clone()).is_ok();
        assert_eq!(table.get(5)?, func_value;
        
        Ok(())
    }

    #[test]
    fn test_safe_memory_handler_error_handling() -> Result<()> {
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Full)?;
        
        // Binary std/no_std choice
        let large_alloc_result = handler.allocate(usize::MAX;
        assert!(large_alloc_result.is_err();
        
        // Verify handler is still usable
        let memory_id = handler.allocate(1024)?;
        let test_data = vec![42u8; 1024];
        
        assert!(handler.write(memory_id, 0, &test_data).is_ok();
        
        let mut buffer = vec![0u8; 1024];
        assert!(handler.read(memory_id, 0, &mut buffer).is_ok();
        assert_eq!(buffer, test_data;
        
        handler.deallocate(memory_id)?;
        
        Ok(())
    }
}