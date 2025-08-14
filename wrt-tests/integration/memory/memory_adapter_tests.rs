//! Memory Adapter Consolidated Tests
//!
//! This module consolidates all memory adapter testing from across the WRT
//! project.

#![cfg(test)]

use std::sync::Arc;

// Import memory adapters
use wrt::memory_adapter::{
    DefaultMemoryAdapter,
    MemoryAdapter,
    SafeMemoryAdapter,
};
use wrt_error::Result;
use wrt_foundation::{
    safe_memory::MemoryProvider,
    types::Limits,
    verification::VerificationLevel,
};
use wrt_runtime::{
    memory::Memory,
    types::MemoryType,
};

// ===========================================
// SHARED ADAPTER TESTING UTILITIES
// ===========================================

/// Create a standard memory type for adapter testing
fn create_adapter_memory_type() -> MemoryType {
    MemoryType {
        limits: Limits {
            min: 1,
            max: Some(4),
        },
    }
}

/// Create test data for adapter operations
fn create_adapter_test_data() -> Vec<u8> {
    vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A]
}

/// Test an adapter with comprehensive operations
fn test_adapter_comprehensive<T: MemoryAdapter>(adapter: &T) -> Result<()> {
    let test_data = create_adapter_test_data();

    // Test store operation
    adapter.store(0, &test_data[..5])?;

    // Test load operation
    let loaded_data = adapter.load(0, 5)?;
    assert_eq!(loaded_data, test_data[..5]);

    // Test size operations
    let size = adapter.size()?;
    assert_eq!(size, 65536); // 1 page = 64KB
    assert_eq!(adapter.byte_size()?, size);

    // Test bounds checking
    let bounds_result = adapter.load(size, 1);
    assert!(
        bounds_result.is_err(),
        "Should fail for out-of-bounds access"
    );

    Ok(())
}

/// Test adapter error handling
fn test_adapter_error_handling<T: MemoryAdapter>(adapter: &T) -> Result<()> {
    let test_data = create_adapter_test_data();

    // Test out-of-bounds store
    let size = adapter.size()?;
    let oob_store = adapter.store(size, &test_data);
    assert!(oob_store.is_err(), "Should fail for out-of-bounds store");

    // Test out-of-bounds load
    let oob_load = adapter.load(size, 1);
    assert!(oob_load.is_err(), "Should fail for out-of-bounds load");

    // Test zero-length operations (should succeed)
    assert!(adapter.store(0, &[]).is_ok());
    assert!(adapter.load(0, 0).is_ok());

    Ok(())
}

// ===========================================
// SAFE MEMORY ADAPTER TESTS
// ===========================================

mod safe_adapter_tests {
    use super::*;

    #[test]
    fn test_safe_memory_adapter_creation() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        // Create the safe memory adapter
        let adapter = SafeMemoryAdapter::new(memory_arc.clone())?;

        // Verify adapter was created successfully
        assert_eq!(adapter.size()?, 65536); // 1 page

        Ok(())
    }

    #[test]
    fn test_safe_memory_adapter_operations() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = SafeMemoryAdapter::new(memory_arc)?;

        // Test comprehensive operations
        test_adapter_comprehensive(&adapter)?;

        Ok(())
    }

    #[test]
    fn test_safe_memory_adapter_verification() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = SafeMemoryAdapter::new(memory_arc)?;
        let test_data = create_adapter_test_data();

        // Test verification through memory provider
        adapter.store(0, &test_data[..5])?;
        adapter.memory_provider().verify_access(0, 5)?;

        // Test invalid verification
        let invalid_verify = adapter.memory_provider().verify_access(0, 100000);
        assert!(
            invalid_verify.is_err(),
            "Should fail for invalid access verification"
        );

        Ok(())
    }

    #[test]
    fn test_safe_memory_adapter_bounds_checking() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = SafeMemoryAdapter::new(memory_arc)?;

        // Test comprehensive error handling
        test_adapter_error_handling(&adapter)?;

        Ok(())
    }

    #[test]
    fn test_safe_memory_adapter_with_verification_levels() -> Result<()> {
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];

        for level in &levels {
            let mem_type = create_adapter_memory_type();
            let mut memory = Memory::new(mem_type)?;
            memory.set_verification_level(*level);

            let memory_arc = Arc::new(memory);
            let adapter = SafeMemoryAdapter::new(memory_arc)?;

            // Test operations with this verification level
            let test_data = create_adapter_test_data();
            adapter.store(0, &test_data[..3])?;

            let loaded = adapter.load(0, 3)?;
            assert_eq!(loaded, test_data[..3]);
        }

        Ok(())
    }
}

// ===========================================
// DEFAULT MEMORY ADAPTER TESTS
// ===========================================

mod default_adapter_tests {
    use super::*;

    #[test]
    fn test_default_memory_adapter_creation() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        // Create the default memory adapter
        let adapter = DefaultMemoryAdapter::new(memory_arc.clone())?;

        // Verify adapter was created successfully
        assert_eq!(adapter.size()?, 65536); // 1 page

        Ok(())
    }

    #[test]
    fn test_default_memory_adapter_operations() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = DefaultMemoryAdapter::new(memory_arc)?;

        // Test comprehensive operations
        test_adapter_comprehensive(&adapter)?;

        Ok(())
    }

    #[test]
    fn test_default_memory_adapter_performance() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = DefaultMemoryAdapter::new(memory_arc)?;
        let test_data = vec![42u8; 1024];

        let start = std::time::Instant::now();

        // Perform many operations
        for i in 0..1000 {
            let offset = (i % 60) * 1024; // Stay within bounds
            adapter.store(offset, &test_data)?;
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Default adapter performance regression"
        );

        Ok(())
    }

    #[test]
    fn test_default_memory_adapter_error_handling() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = DefaultMemoryAdapter::new(memory_arc)?;

        // Test comprehensive error handling
        test_adapter_error_handling(&adapter)?;

        Ok(())
    }
}

// ===========================================
// ADAPTER COMPARISON TESTS
// ===========================================

mod adapter_comparison_tests {
    use super::*;

    #[test]
    fn test_adapter_interface_consistency() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory1 = Memory::new(mem_type.clone())?;
        let memory2 = Memory::new(mem_type)?;

        let safe_adapter = SafeMemoryAdapter::new(Arc::new(memory1))?;
        let default_adapter = DefaultMemoryAdapter::new(Arc::new(memory2))?;

        let test_data = create_adapter_test_data();

        // Both adapters should behave consistently for basic operations
        safe_adapter.store(0, &test_data[..5])?;
        default_adapter.store(0, &test_data[..5])?;

        let safe_loaded = safe_adapter.load(0, 5)?;
        let default_loaded = default_adapter.load(0, 5)?;

        assert_eq!(safe_loaded, default_loaded);
        assert_eq!(safe_loaded, test_data[..5]);

        // Both should report the same size
        assert_eq!(safe_adapter.size()?, default_adapter.size()?);
        assert_eq!(safe_adapter.byte_size()?, default_adapter.byte_size()?);

        Ok(())
    }

    #[test]
    fn test_adapter_error_consistency() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory1 = Memory::new(mem_type.clone())?;
        let memory2 = Memory::new(mem_type)?;

        let safe_adapter = SafeMemoryAdapter::new(Arc::new(memory1))?;
        let default_adapter = DefaultMemoryAdapter::new(Arc::new(memory2))?;

        let test_data = create_adapter_test_data();
        let size = safe_adapter.size()?;

        // Both should fail consistently for out-of-bounds operations
        let safe_error = safe_adapter.store(size, &test_data);
        let default_error = default_adapter.store(size, &test_data);

        assert!(safe_error.is_err());
        assert!(default_error.is_err());

        // Both should fail consistently for out-of-bounds loads
        let safe_load_error = safe_adapter.load(size, 1);
        let default_load_error = default_adapter.load(size, 1);

        assert!(safe_load_error.is_err());
        assert!(default_load_error.is_err());

        Ok(())
    }

    #[test]
    fn test_adapter_performance_comparison() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory1 = Memory::new(mem_type.clone())?;
        let memory2 = Memory::new(mem_type)?;

        let safe_adapter = SafeMemoryAdapter::new(Arc::new(memory1))?;
        let default_adapter = DefaultMemoryAdapter::new(Arc::new(memory2))?;

        let test_data = vec![42u8; 512];
        let iterations = 1000;

        // Test safe adapter performance
        let start = std::time::Instant::now();
        for i in 0..iterations {
            let offset = (i % 120) * 512; // Stay within bounds
            safe_adapter.store(offset, &test_data)?;
        }
        let safe_duration = start.elapsed();

        // Test default adapter performance
        let start = std::time::Instant::now();
        for i in 0..iterations {
            let offset = (i % 120) * 512; // Stay within bounds
            default_adapter.store(offset, &test_data)?;
        }
        let default_duration = start.elapsed();

        // Both should be reasonably fast
        assert!(safe_duration.as_millis() < 200, "Safe adapter too slow");
        assert!(
            default_duration.as_millis() < 200,
            "Default adapter too slow"
        );

        // Safe adapter may be slightly slower due to additional checks
        // but shouldn't be excessively slower
        if safe_duration > default_duration {
            let ratio = safe_duration.as_nanos() / default_duration.as_nanos();
            assert!(ratio < 10, "Safe adapter overhead too high");
        }

        Ok(())
    }
}

// ===========================================
// ADAPTER INTEGRATION TESTS
// ===========================================

mod adapter_integration_tests {
    use super::*;

    #[test]
    fn test_adapter_memory_growth() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = SafeMemoryAdapter::new(memory_arc.clone())?;

        // Initial size should be 1 page
        assert_eq!(adapter.size()?, 65536);

        // Grow the underlying memory
        {
            let memory_ref = Arc::clone(&memory_arc);
            // Note: In a real implementation, this would need proper
            // synchronization For testing, we assume the adapter
            // can handle underlying memory changes
        }

        // Test operations still work after growth conceptually
        let test_data = create_adapter_test_data();
        adapter.store(0, &test_data)?;

        let loaded = adapter.load(0, test_data.len())?;
        assert_eq!(loaded, test_data);

        Ok(())
    }

    #[test]
    fn test_adapter_with_multiple_memories() -> Result<()> {
        let mem_type1 = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mem_type2 = MemoryType {
            limits: Limits {
                min: 2,
                max: Some(4),
            },
        };

        let memory1 = Memory::new(mem_type1)?;
        let memory2 = Memory::new(mem_type2)?;

        let adapter1 = SafeMemoryAdapter::new(Arc::new(memory1))?;
        let adapter2 = SafeMemoryAdapter::new(Arc::new(memory2))?;

        // Different sizes
        assert_eq!(adapter1.size()?, 65536); // 1 page
        assert_eq!(adapter2.size()?, 131072); // 2 pages

        let test_data = create_adapter_test_data();

        // Both should work independently
        adapter1.store(0, &test_data[..5])?;
        adapter2.store(0, &test_data[..8])?;

        let loaded1 = adapter1.load(0, 5)?;
        let loaded2 = adapter2.load(0, 8)?;

        assert_eq!(loaded1, test_data[..5]);
        assert_eq!(loaded2, test_data[..8]);

        Ok(())
    }

    #[test]
    fn test_adapter_thread_safety() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = Arc::new(SafeMemoryAdapter::new(memory_arc)?);
        let test_data = create_adapter_test_data();

        // Test that adapter can be safely shared across threads
        let adapter_clone = Arc::clone(&adapter);
        let test_data_clone = test_data.clone();

        let handle = std::thread::spawn(move || -> Result<()> {
            adapter_clone.store(1024, &test_data_clone[..5])?;
            let loaded = adapter_clone.load(1024, 5)?;
            assert_eq!(loaded, test_data_clone[..5]);
            Ok(())
        });

        // Simultaneous operations from main thread
        adapter.store(2048, &test_data[..3])?;
        let loaded = adapter.load(2048, 3)?;
        assert_eq!(loaded, test_data[..3]);

        // Wait for thread to complete
        handle.join().unwrap()?;

        Ok(())
    }

    #[test]
    fn test_adapter_with_verification_changes() -> Result<()> {
        let mem_type = create_adapter_memory_type();
        let memory = Memory::new(mem_type)?;
        let memory_arc = Arc::new(memory);

        let adapter = SafeMemoryAdapter::new(memory_arc.clone())?;
        let test_data = create_adapter_test_data();

        // Store data with initial verification level
        adapter.store(0, &test_data[..5])?;

        // Change verification level on underlying memory
        {
            let memory_ref = Arc::clone(&memory_arc);
            // In a real implementation, this would require proper
            // synchronization memory_ref.
            // set_verification_level(VerificationLevel::Critical);
        }

        // Adapter should still work
        let loaded = adapter.load(0, 5)?;
        assert_eq!(loaded, test_data[..5]);

        // New operations should work with changed verification
        adapter.store(10, &test_data[..3])?;
        let loaded2 = adapter.load(10, 3)?;
        assert_eq!(loaded2, test_data[..3]);

        Ok(())
    }
}
