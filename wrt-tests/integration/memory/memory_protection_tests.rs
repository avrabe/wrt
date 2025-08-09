//! Memory Protection Tests
//!
//! This module consolidates memory protection testing functionality, including bounds checking,
//! overflow prevention, and memory isolation tests from across the WRT project.

#![cfg(test)]

use std::sync::Arc;
use wrt_error::Result;
use wrt_runtime::memory::Memory;
use wrt_runtime::types::MemoryType;
use wrt_foundation::safe_memory::{SafeMemoryHandler, SafeSlice, MemoryProvider};
use wrt_foundation::verification::VerificationLevel;
use wrt_foundation::types::Limits;

#[cfg(not(feature = "std"))]
use wrt_foundation::safe_memory::NoStdMemoryProvider;
#[cfg(feature = "std")]
use wrt_foundation::safe_memory::StdMemoryProvider;

// ===========================================
// BOUNDS CHECKING TESTS
// ===========================================

mod bounds_checking_tests {
    use super::*;

    #[test]
    fn test_memory_bounds_enforcement() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 1, max: Some(2) },
        };
        let memory = Memory::new(mem_type)?;
        
        let test_data = vec![1, 2, 3, 4, 5];
        let page_size = 65536; // 64KB
        
        // Test valid writes at different positions
        assert!(memory.write(0, &test_data).is_ok());
        assert!(memory.write(100, &test_data).is_ok());
        assert!(memory.write(page_size - test_data.len(), &test_data).is_ok());
        
        // Test boundary condition - exactly at page boundary
        let boundary_write = memory.write(page_size - 1, &[42];
        assert!(boundary_write.is_ok());
        
        // Test out-of-bounds writes
        assert!(memory.write(page_size, &test_data).is_err();
        assert!(memory.write(page_size + 1, &test_data).is_err();
        assert!(memory.write(usize::MAX - 100, &test_data).is_err();
        
        Ok(())
    }

    #[test]
    fn test_memory_bounds_after_growth() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 1, max: Some(4) },
        };
        let mut memory = Memory::new(mem_type)?;
        
        let test_data = vec![1, 2, 3, 4, 5];
        let page_size = 65536;
        
        // Test initial bounds
        assert!(memory.write(page_size - test_data.len(), &test_data).is_ok());
        assert!(memory.write(page_size, &test_data).is_err();
        
        // Grow memory by 1 page
        memory.grow(1)?;
        
        // Test new bounds
        assert!(memory.write(page_size, &test_data).is_ok())); // Now valid
        assert!(memory.write(page_size * 2 - test_data.len(), &test_data).is_ok());
        assert!(memory.write(page_size * 2, &test_data).is_err())); // Still out of bounds
        
        Ok(())
    }

    #[test]
    fn test_safe_slice_bounds_protection() -> Result<()> {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let slice = SafeSlice::new(&data)?;
        
        // Test valid subslice operations
        assert!(slice.subslice(0, 5).is_ok());
        assert!(slice.subslice(5, 5).is_ok());
        assert!(slice.subslice(9, 1).is_ok());
        
        // Test boundary conditions
        assert!(slice.subslice(0, 10).is_ok())); // Exact size
        assert!(slice.subslice(10, 0).is_ok())); // Zero length at end
        
        // Test out-of-bounds operations
        assert!(slice.subslice(0, 11).is_err())); // Too long
        assert!(slice.subslice(11, 1).is_err())); // Start beyond end
        assert!(slice.subslice(5, 10).is_err())); // Extends beyond end
        
        Ok(())
    }

    #[test]
    fn test_bounds_checking_with_verification_levels() -> Result<()> {
        let data = vec![0u8; 1024];
        
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            let slice = SafeSlice::with_verification_level(&data, *level)?;
            
            // Valid operations should work at all levels
            assert!(slice.subslice(0, 100).is_ok());
            assert!(slice.subslice(500, 200).is_ok());
            
            // Invalid operations should fail at all levels
            assert!(slice.subslice(0, 2000).is_err();
            assert!(slice.subslice(2000, 100).is_err();
        }
        
        Ok(())
    }
}

// ===========================================
// OVERFLOW PREVENTION TESTS
// ===========================================

mod overflow_prevention_tests {
    use super::*;

    #[test]
    fn test_arithmetic_overflow_protection() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 1, max: Some(2) },
        };
        let memory = Memory::new(mem_type)?;
        
        let test_data = vec![1, 2, 3, 4, 5];
        
        // Test operations that could cause arithmetic overflow
        let large_offset = usize::MAX - 10;
        
        // These should be caught as out-of-bounds, not overflow
        assert!(memory.write(large_offset, &test_data).is_err();
        
        let mut buffer = vec![0; test_data.len()];
        assert!(memory.read(large_offset, &mut buffer).is_err();
        
        Ok(())
    }

    #[test]
    fn test_size_calculation_overflow_protection() -> Result<()> {
        let data = vec![1, 2, 3, 4, 5];
        let slice = SafeSlice::new(&data)?;
        
        // Test subslice operations that could overflow
        assert!(slice.subslice(usize::MAX, 1).is_err();
        assert!(slice.subslice(1, usize::MAX).is_err();
        assert!(slice.subslice(usize::MAX, usize::MAX).is_err();
        
        Ok(())
    }

    #[test]
    fn test_memory_provider_overflow_protection() -> Result<()> {
        #[cfg(feature = "std")]
        {
            let data = vec![0u8; 1024];
            let provider = StdMemoryProvider::new(data;
            
            // Test access operations that could overflow
            assert!(provider.verify_access(usize::MAX, 1).is_err();
            assert!(provider.verify_access(1, usize::MAX).is_err();
            assert!(provider.verify_access(usize::MAX, usize::MAX).is_err();
            
            // Test borrow_slice with overflow potential
            assert!(provider.borrow_slice(usize::MAX, 1).is_err();
            assert!(provider.borrow_slice(1, usize::MAX).is_err();
        }
        
        #[cfg(not(feature = "std"))]
        {
            let provider = NoStdMemoryProvider::<1024>::new();
            
            // Test access operations that could overflow
            assert!(provider.verify_access(usize::MAX, 1).is_err();
            assert!(provider.verify_access(1, usize::MAX).is_err();
            assert!(provider.verify_access(usize::MAX, usize::MAX).is_err();
            
            // Test borrow_slice with overflow potential
            assert!(provider.borrow_slice(usize::MAX, 1).is_err();
            assert!(provider.borrow_slice(1, usize::MAX).is_err();
        }
        
        Ok(())
    }

    #[test]
    fn test_memory_handler_overflow_protection() -> Result<()> {
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Full)?;
        
        // Binary std/no_std choice
        let large_alloc_result = handler.allocate(usize::MAX;
        assert!(large_alloc_result.is_err();
        
        // Allocate normal memory for further testing
        let memory_id = handler.allocate(1024)?;
        
        // Test read/write operations with overflow potential
        let test_data = vec![42u8; 10];
        
        assert!(handler.write(memory_id, usize::MAX, &test_data).is_err();
        
        let mut buffer = vec![0u8; 10];
        assert!(handler.read(memory_id, usize::MAX, &mut buffer).is_err();
        
        handler.deallocate(memory_id)?;
        
        Ok(())
    }
}

// ===========================================
// MEMORY ISOLATION TESTS
// ===========================================

mod memory_isolation_tests {
    use super::*;

    #[test]
    fn test_memory_instance_isolation() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 1, max: Some(2) },
        };
        
        let memory1 = Memory::new(mem_type.clone())?;
        let memory2 = Memory::new(mem_type)?;
        
        let test_data1 = vec![1, 2, 3, 4, 5];
        let test_data2 = vec![10, 20, 30, 40, 50];
        
        // Write different data to each memory
        memory1.write(0, &test_data1)?;
        memory2.write(0, &test_data2)?;
        
        // Verify isolation - each memory should contain its own data
        let mut buffer1 = vec![0; test_data1.len()];
        let mut buffer2 = vec![0; test_data2.len()];
        
        memory1.read(0, &mut buffer1)?;
        memory2.read(0, &mut buffer2)?;
        
        assert_eq!(buffer1, test_data1;
        assert_eq!(buffer2, test_data2;
        assert_ne!(buffer1, buffer2;
        
        Ok(())
    }

    #[test]
    fn test_memory_handler_isolation() -> Result<()> {
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Full)?;
        
        let test_data1 = vec![1, 2, 3, 4, 5];
        let test_data2 = vec![10, 20, 30, 40, 50];
        
        // Allocate two separate memory regions
        let memory_id1 = handler.allocate(test_data1.len())?;
        let memory_id2 = handler.allocate(test_data2.len())?;
        
        // Write different data to each region
        handler.write(memory_id1, 0, &test_data1)?;
        handler.write(memory_id2, 0, &test_data2)?;
        
        // Verify isolation
        let mut buffer1 = vec![0; test_data1.len()];
        let mut buffer2 = vec![0; test_data2.len()];
        
        handler.read(memory_id1, 0, &mut buffer1)?;
        handler.read(memory_id2, 0, &mut buffer2)?;
        
        assert_eq!(buffer1, test_data1;
        assert_eq!(buffer2, test_data2;
        assert_ne!(buffer1, buffer2;
        
        // Clean up
        handler.deallocate(memory_id1)?;
        handler.deallocate(memory_id2)?;
        
        Ok(())
    }

    #[test]
    fn test_slice_isolation() -> Result<()> {
        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![10, 20, 30, 40, 50];
        
        let slice1 = SafeSlice::new(&data1)?;
        let slice2 = SafeSlice::new(&data2)?;
        
        // Verify each slice contains its own data
        assert_eq!(slice1.data()?, &data1;
        assert_eq!(slice2.data()?, &data2;
        assert_ne!(slice1.data()?, slice2.data()?;
        
        // Verify subslices maintain isolation
        let subslice1 = slice1.subslice(1, 3)?;
        let subslice2 = slice2.subslice(1, 3)?;
        
        assert_eq!(subslice1.data()?, &data1[1..4];
        assert_eq!(subslice2.data()?, &data2[1..4];
        assert_ne!(subslice1.data()?, subslice2.data()?;
        
        Ok(())
    }

    #[test]
    fn test_cross_thread_memory_isolation() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 1, max: Some(2) },
        };
        let memory = Arc::new(Memory::new(mem_type)?;
        
        let test_data1 = vec![1, 2, 3, 4, 5];
        let test_data2 = vec![10, 20, 30, 40, 50];
        
        // Write initial data from main thread
        memory.write(0, &test_data1)?;
        
        // Spawn thread to write different data at different offset
        let memory_clone = Arc::clone(&memory);
        let test_data2_clone = test_data2.clone();
        
        let handle = std::thread::spawn(move || -> Result<()> {
            memory_clone.write(100, &test_data2_clone)?;
            Ok(())
        };
        
        handle.join().unwrap()?;
        
        // Verify both data regions are intact and isolated
        let mut buffer1 = vec![0; test_data1.len()];
        let mut buffer2 = vec![0; test_data2.len()];
        
        memory.read(0, &mut buffer1)?;
        memory.read(100, &mut buffer2)?;
        
        assert_eq!(buffer1, test_data1;
        assert_eq!(buffer2, test_data2;
        
        Ok(())
    }
}

// ===========================================
// ACCESS CONTROL TESTS
// ===========================================

mod access_control_tests {
    use super::*;

    #[test]
    fn test_verification_level_access_control() -> Result<()> {
        let data = vec![0u8; 1024];
        
        // Test with different verification levels
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            let slice = SafeSlice::with_verification_level(&data, *level)?;
            
            // All levels should enforce basic bounds checking
            assert!(slice.subslice(0, 100).is_ok());
            assert!(slice.subslice(0, 2000).is_err();
            
            // Verify the verification level is set correctly
            assert_eq!(slice.verification_level(), *level;
        }
        
        Ok(())
    }

    #[test]
    fn test_memory_provider_access_verification() -> Result<()> {
        #[cfg(feature = "std")]
        {
            let data = vec![0u8; 1024];
            let provider = StdMemoryProvider::new(data;
            
            // Test valid access patterns
            assert!(provider.verify_access(0, 100).is_ok());
            assert!(provider.verify_access(500, 200).is_ok());
            assert!(provider.verify_access(1023, 1).is_ok());
            
            // Test invalid access patterns
            assert!(provider.verify_access(0, 2000).is_err();
            assert!(provider.verify_access(1024, 1).is_err();
            assert!(provider.verify_access(2000, 100).is_err();
        }
        
        #[cfg(not(feature = "std"))]
        {
            let provider = NoStdMemoryProvider::<1024>::new();
            
            // Test valid access patterns
            assert!(provider.verify_access(0, 100).is_ok());
            assert!(provider.verify_access(500, 200).is_ok());
            assert!(provider.verify_access(1023, 1).is_ok());
            
            // Test invalid access patterns
            assert!(provider.verify_access(0, 2000).is_err();
            assert!(provider.verify_access(1024, 1).is_err();
            assert!(provider.verify_access(2000, 100).is_err();
        }
        
        Ok(())
    }

    #[test]
    fn test_memory_handler_access_control() -> Result<()> {
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Full)?;
        
        let memory_id = handler.allocate(1024)?;
        let test_data = vec![42u8; 100];
        
        // Test valid operations
        assert!(handler.write(memory_id, 0, &test_data).is_ok());
        assert!(handler.write(memory_id, 500, &test_data).is_ok());
        assert!(handler.write(memory_id, 924, &test_data).is_ok())); // Exactly fits
        
        // Test invalid operations
        assert!(handler.write(memory_id, 925, &test_data).is_err())); // Overflows
        assert!(handler.write(memory_id, 1024, &test_data).is_err())); // Out of bounds
        
        let mut buffer = vec![0u8; 100];
        
        // Test valid reads
        assert!(handler.read(memory_id, 0, &mut buffer).is_ok());
        assert!(handler.read(memory_id, 500, &mut buffer).is_ok());
        assert!(handler.read(memory_id, 924, &mut buffer).is_ok());
        
        // Test invalid reads
        assert!(handler.read(memory_id, 925, &mut buffer).is_err();
        assert!(handler.read(memory_id, 1024, &mut buffer).is_err();
        
        handler.deallocate(memory_id)?;
        
        Ok(())
    }
}

// ===========================================
// PROTECTION INTEGRATION TESTS
// ===========================================

mod protection_integration_tests {
    use super::*;

    #[test]
    fn test_comprehensive_protection_stack() -> Result<()> {
        // Test that multiple protection layers work together
        let mut handler = SafeMemoryHandler::new(VerificationLevel::Critical)?;
        
        let memory_id = handler.allocate(1024)?;
        let test_data = vec![1, 2, 3, 4, 5];
        
        // Write data through the handler (multiple protection layers)
        handler.write(memory_id, 100, &test_data)?;
        
        // Verify the data through multiple mechanisms
        let mut buffer = vec![0; test_data.len()];
        handler.read(memory_id, 100, &mut buffer)?;
        assert_eq!(buffer, test_data;
        
        // Verify integrity at handler level
        handler.verify_all()?;
        
        // Test that protections catch various types of violations
        assert!(handler.write(memory_id, 1020, &test_data).is_err())); // Bounds
        assert!(handler.write(memory_id, usize::MAX, &test_data).is_err())); // Overflow
        
        handler.deallocate(memory_id)?;
        
        Ok(())
    }

    #[test]
    fn test_protection_under_concurrent_access() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 2, max: Some(4) },
        };
        let memory = Arc::new(Memory::new(mem_type)?;
        
        let test_data = vec![42u8; 100];
        
        // Spawn multiple threads trying to access different regions
        let handles: Vec<_> = (0..4).map(|i| {
            let memory_clone = Arc::clone(&memory);
            let test_data_clone = test_data.clone();
            
            std::thread::spawn(move || -> Result<()> {
                let offset = i * 1000; // Spread accesses across memory
                
                // All should succeed as they're in different regions
                memory_clone.write(offset, &test_data_clone)?;
                
                let mut buffer = vec![0; test_data_clone.len()];
                memory_clone.read(offset, &mut buffer)?;
                assert_eq!(buffer, test_data_clone;
                
                // All should fail for out-of-bounds access
                let oob_result = memory_clone.write(200000, &test_data_clone;
                assert!(oob_result.is_err();
                
                Ok(())
            })
        }).collect();
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap()?;
        }
        
        Ok(())
    }

    #[test]
    fn test_protection_preservation_across_operations() -> Result<()> {
        let mem_type = MemoryType {
            limits: Limits { min: 1, max: Some(4) },
        };
        let mut memory = Memory::new(mem_type)?;
        
        let test_data = vec![1, 2, 3, 4, 5];
        
        // Set high verification level
        memory.set_verification_level(VerificationLevel::Critical;
        
        // Test that protections are maintained through growth
        memory.write(0, &test_data)?;
        memory.grow(1)?; // Grow memory
        
        // Protections should still work
        assert!(memory.write(200000, &test_data).is_err())); // Still out of bounds
        assert!(memory.write(70000, &test_data).is_ok())); // Now in bounds
        
        // Verification level should be preserved
        assert_eq!(memory.verification_level(), VerificationLevel::Critical;
        
        // Integrity should still be verifiable
        memory.verify_integrity()?;
        
        Ok(())
    }
}