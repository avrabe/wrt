//! Comprehensive tests for WebAssembly atomic operations.
//!
//! These tests verify the integration between atomic operations, memory management,
//! and thread synchronization in the WRT runtime.

use core::time::Duration;
use std::{sync::Arc, thread};

use wrt_error::Result;
use wrt_foundation::types::Limits;
use wrt_runtime::{Memory, MemoryType};
use wrt_instructions::atomic_ops::AtomicOperations;

#[cfg(feature = "threading")]
use wrt_platform::{
    atomic_thread_manager::AtomicAwareThreadManager,
    threading::{ThreadPoolConfig, ThreadingLimits, ThreadPriority, ThreadSpawnRequest},
};

/// Test basic atomic load/store operations
#[test]
fn test_atomic_load_store() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Test 32-bit atomic operations
    memory.atomic_store_i32(0, 42)?;
    let value = memory.atomic_load_i32(0)?;
    assert_eq!(value, 42;
    
    // Test 64-bit atomic operations
    memory.atomic_store_i64(8, 0x123456789ABCDEF0)?;
    let value = memory.atomic_load_i64(8)?;
    assert_eq!(value, 0x123456789ABCDEF0u64 as i64;
    
    Ok(())
}

/// Test atomic read-modify-write operations
#[test]
fn test_atomic_rmw_operations() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Initialize memory
    memory.atomic_store_i32(0, 10)?;
    
    // Test atomic add
    let old_value = memory.atomic_rmw_add_i32(0, 5)?;
    assert_eq!(old_value, 10;
    assert_eq!(memory.atomic_load_i32(0)?, 15;
    
    // Test atomic sub
    let old_value = memory.atomic_rmw_sub_i32(0, 3)?;
    assert_eq!(old_value, 15;
    assert_eq!(memory.atomic_load_i32(0)?, 12;
    
    // Test atomic and
    memory.atomic_store_i32(0, 0xFF)?;
    let old_value = memory.atomic_rmw_and_i32(0, 0x0F)?;
    assert_eq!(old_value, 0xFF;
    assert_eq!(memory.atomic_load_i32(0)?, 0x0F;
    
    // Test atomic or
    let old_value = memory.atomic_rmw_or_i32(0, 0xF0)?;
    assert_eq!(old_value, 0x0F;
    assert_eq!(memory.atomic_load_i32(0)?, 0xFF;
    
    // Test atomic xor
    let old_value = memory.atomic_rmw_xor_i32(0, 0xFF)?;
    assert_eq!(old_value, 0xFF;
    assert_eq!(memory.atomic_load_i32(0)?, 0x00;
    
    // Test atomic exchange
    let old_value = memory.atomic_rmw_xchg_i32(0, 999)?;
    assert_eq!(old_value, 0x00;
    assert_eq!(memory.atomic_load_i32(0)?, 999;
    
    // Test atomic compare-exchange
    let old_value = memory.atomic_rmw_cmpxchg_i32(0, 999, 1000)?;
    assert_eq!(old_value, 999;
    assert_eq!(memory.atomic_load_i32(0)?, 1000;
    
    // Test failed compare-exchange
    let old_value = memory.atomic_rmw_cmpxchg_i32(0, 999, 2000)?;
    assert_eq!(old_value, 1000); // Should return current value, not expected
    assert_eq!(memory.atomic_load_i32(0)?, 1000); // Value should not change
    
    Ok(())
}

/// Test atomic alignment requirements
#[test]
fn test_atomic_alignment() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Test that aligned accesses work
    assert!(memory.atomic_load_i32(0).is_ok());
    assert!(memory.atomic_load_i32(4).is_ok());
    assert!(memory.atomic_load_i64(0).is_ok());
    assert!(memory.atomic_load_i64(8).is_ok());
    
    // Test that misaligned accesses fail
    assert!(memory.atomic_load_i32(1).is_err())); // 32-bit at non-4-byte boundary
    assert!(memory.atomic_load_i32(2).is_err();
    assert!(memory.atomic_load_i32(3).is_err();
    
    assert!(memory.atomic_load_i64(1).is_err())); // 64-bit at non-8-byte boundary
    assert!(memory.atomic_load_i64(4).is_err();
    assert!(memory.atomic_load_i64(7).is_err();
    
    Ok(())
}

/// Test atomic wait/notify basic functionality
#[test]
fn test_atomic_wait_notify_basic() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Set initial value
    memory.atomic_store_i32(0, 42)?;
    
    // Test wait with incorrect expected value (should return immediately)
    let result = memory.atomic_wait32(0, 99, Some(1_000_000))?; // 1ms timeout
    assert_eq!(result, 1); // Should return 1 (value mismatch)
    
    // Test notify when no waiters
    let result = memory.atomic_notify(0, 1)?;
    assert_eq!(result, 0); // Should return 0 (no waiters woken)
    
    Ok(())
}

/// Test atomic wait with timeout
#[test]
fn test_atomic_wait_timeout() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Set initial value
    memory.atomic_store_i32(0, 42)?;
    
    let start = std::time::Instant::now);
    
    // Test wait with correct expected value but timeout
    let result = memory.atomic_wait32(0, 42, Some(10_000_000))?; // 10ms timeout
    
    let elapsed = start.elapsed);
    
    // Should timeout (return 2) and take approximately the timeout duration
    assert_eq!(result, 2;
    assert!(elapsed >= Duration::from_millis(8))); // Allow some tolerance
    assert!(elapsed <= Duration::from_millis(50))); // But not too much
    
    Ok(())
}

/// Test 64-bit atomic operations
#[test]
fn test_atomic_64bit_operations() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Test 64-bit atomic operations
    let initial_value = 0x123456789ABCDEF0u64 as i64;
    memory.atomic_store_i64(0, initial_value)?;
    
    // Test 64-bit RMW operations
    let old_value = memory.atomic_rmw_add_i64(0, 0x10)?;
    assert_eq!(old_value, initial_value;
    assert_eq!(memory.atomic_load_i64(0)?, initial_value + 0x10;
    
    // Test 64-bit compare-exchange
    let current = memory.atomic_load_i64(0)?;
    let old_value = memory.atomic_rmw_cmpxchg_i64(0, current, 0x1111111111111111)?;
    assert_eq!(old_value, current;
    assert_eq!(memory.atomic_load_i64(0)?, 0x1111111111111111;
    
    // Test 64-bit wait
    let result = memory.atomic_wait64(0, 0x2222222222222222, Some(1_000_000))?; // 1ms timeout
    assert_eq!(result, 1); // Value mismatch
    
    Ok(())
}

/// Test memory bounds checking for atomic operations
#[test]
fn test_atomic_bounds_checking() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    let memory_size = memory.size_in_bytes);
    
    // Test access at the very end of memory (should fail)
    assert!(memory.atomic_load_i32((memory_size - 3) as u32).is_err();
    assert!(memory.atomic_load_i64((memory_size - 7) as u32).is_err();
    
    // Test access beyond memory (should fail)
    assert!(memory.atomic_load_i32(memory_size as u32).is_err();
    assert!(memory.atomic_load_i64(memory_size as u32).is_err();
    
    // Test valid access near the end
    assert!(memory.atomic_load_i32((memory_size - 4) as u32).is_ok());
    assert!(memory.atomic_load_i64((memory_size - 8) as u32).is_ok());
    
    Ok(())
}

/// Test atomic operations with thread manager integration
#[cfg(feature = "threading")]
#[test]
fn test_atomic_thread_manager_integration() -> Result<()> {
    use std::sync::Arc;
    
    let config = ThreadPoolConfig::default());
    let limits = ThreadingLimits::default());
    let executor = Arc::new(|_function_id: u32, args: Vec<u8>| -> Result<Vec<u8>> {
        Ok(args) // Echo the arguments back
    };
    
    let manager = AtomicAwareThreadManager::new(config, limits, executor)?;
    
    // Test atomic notify with no waiters
    let result = manager.execute_atomic_notify(0x1000, 1)?;
    assert_eq!(result, 0); // No waiters to wake
    
    // Test atomic wait with immediate mismatch
    let result = manager.execute_atomic_wait(0x1000, 42, Some(1_000_000))?; // 1ms timeout
    // Note: This might return different values depending on the implementation
    assert!(result == 0 || result == 1 || result == 2)); // Valid return codes
    
    let stats = manager.get_stats);
    println!("Atomic-aware thread manager stats: {:?}", stats);
    
    Ok(())
}

/// Test concurrent atomic operations (requires threading)
#[cfg(feature = "threading")]
#[test]
fn test_concurrent_atomic_operations() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let memory = Arc::new(std::sync::Mutex::new(Memory::new(mem_type)?;
    
    // Initialize counter
    {
        let mut mem = memory.lock().unwrap();
        mem.atomic_store_i32(0, 0)?;
    }
    
    const NUM_THREADS: usize = 4;
    const INCREMENTS_PER_THREAD: i32 = 1000;
    
    let mut handles = Vec::new());
    
    // Spawn threads that increment the counter
    for _i in 0..NUM_THREADS {
        let mem_clone = Arc::clone(&memory);
        let handle = thread::spawn(move || -> Result<()> {
            for _j in 0..INCREMENTS_PER_THREAD {
                let mut mem = mem_clone.lock().unwrap();
                mem.atomic_rmw_add_i32(0, 1)?;
            }
            Ok(())
        };
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap()?;
    }
    
    // Check final value
    let final_value = {
        let mem = memory.lock().unwrap();
        mem.atomic_load_i32(0)?
    };
    
    let expected = NUM_THREADS as i32 * INCREMENTS_PER_THREAD;
    assert_eq!(final_value, expected;
    
    Ok(())
}

/// Test atomic wait/notify with actual threading
#[cfg(feature = "threading")]
#[test]
fn test_atomic_wait_notify_threading() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let memory = Arc::new(std::sync::Mutex::new(Memory::new(mem_type)?;
    
    // Initialize value
    {
        let mut mem = memory.lock().unwrap();
        mem.atomic_store_i32(0, 0)?;
    }
    
    let mem_clone = Arc::clone(&memory);
    
    // Spawn a thread that will wait
    let waiter_handle = thread::spawn(move || -> Result<i32> {
        let mut mem = mem_clone.lock().unwrap();
        // Wait for value 0 with a long timeout
        mem.atomic_wait32(0, 0, Some(5_000_000_000)) // 5 second timeout
    };
    
    // Give the waiter thread time to start waiting
    thread::sleep(Duration::from_millis(100;
    
    // Change the value and notify
    {
        let mut mem = memory.lock().unwrap();
        mem.atomic_store_i32(0, 1)?; // Change the value
        mem.atomic_notify(0, 1)?; // Wake the waiter
    }
    
    // The waiter should wake up
    let result = waiter_handle.join().unwrap()?;
    
    // The result should be 0 (woken) or 1 (value changed), not 2 (timeout)
    assert!(result == 0 || result == 1);
    assert_ne!(result, 2); // Should not timeout
    
    Ok(())
}

/// Benchmark atomic operations performance
#[cfg(feature = "threading")]
#[test]
fn benchmark_atomic_operations() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    const NUM_OPERATIONS: usize = 10_000;
    
    // Benchmark atomic loads
    memory.atomic_store_i32(0, 42)?;
    let start = std::time::Instant::now);
    for _i in 0..NUM_OPERATIONS {
        let _value = memory.atomic_load_i32(0)?;
    }
    let load_duration = start.elapsed);
    
    // Benchmark atomic stores
    let start = std::time::Instant::now);
    for i in 0..NUM_OPERATIONS {
        memory.atomic_store_i32(0, i as i32)?;
    }
    let store_duration = start.elapsed);
    
    // Benchmark atomic RMW operations
    memory.atomic_store_i32(0, 0)?;
    let start = std::time::Instant::now);
    for _i in 0..NUM_OPERATIONS {
        memory.atomic_rmw_add_i32(0, 1)?;
    }
    let rmw_duration = start.elapsed);
    
    println!("Atomic operations benchmark:");
    println!("  Load:  {:?} ({:.2} ns/op)", load_duration, load_duration.as_nanos() as f64 / NUM_OPERATIONS as f64);
    println!("  Store: {:?} ({:.2} ns/op)", store_duration, store_duration.as_nanos() as f64 / NUM_OPERATIONS as f64);
    println!("  RMW:   {:?} ({:.2} ns/op)", rmw_duration, rmw_duration.as_nanos() as f64 / NUM_OPERATIONS as f64);
    
    // Verify RMW operations worked correctly
    let final_value = memory.atomic_load_i32(0)?;
    assert_eq!(final_value, NUM_OPERATIONS as i32;
    
    Ok(())
}

/// Test error handling in atomic operations
#[test]
fn test_atomic_error_handling() -> Result<()> {
    let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let mut memory = Memory::new(mem_type)?;
    
    // Test various error conditions
    
    // Misaligned access
    assert!(memory.atomic_load_i32(1).is_err();
    assert!(memory.atomic_load_i64(4).is_err();
    
    // Out of bounds access
    let memory_size = memory.size_in_bytes);
    assert!(memory.atomic_load_i32(memory_size as u32).is_err();
    assert!(memory.atomic_load_i64(memory_size as u32).is_err();
    
    // Wait operations with invalid addresses
    assert!(memory.atomic_wait32(memory_size as u32, 0, Some(1_000_000)).is_err();
    assert!(memory.atomic_wait64(memory_size as u32, 0, Some(1_000_000)).is_err();
    
    // Notify operations with invalid addresses
    assert!(memory.atomic_notify(memory_size as u32, 1).is_err();
    
    Ok(())
}