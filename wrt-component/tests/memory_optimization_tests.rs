#![deny(warnings)]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};
use wrt_component::resources::{BufferPool, MemoryStrategy, VerificationLevel};
use wrt_component::strategies::{
    BoundedCopyStrategy, FullIsolationStrategy, MemoryOptimizationStrategy, ZeroCopyStrategy,
};
use wrt_error::Result;

/// Test basic buffer pool functionality
#[test]
fn test_buffer_pool_basics() {
    let mut pool = BufferPool::new(4096); // 4KB max size

    // Allocate a buffer and check its size
    let buffer = pool.get_buffer(100);
    assert_eq!(buffer.len(), 100);

    // Write to the buffer
    buffer.fill(42);
    assert_eq!(buffer[0], 42);
    assert_eq!(buffer[99], 42);

    // Allocate another buffer
    let buffer2 = pool.get_buffer(200);
    assert_eq!(buffer2.len(), 200);

    // Return buffers to the pool
    pool.return_buffer(buffer);
    pool.return_buffer(buffer2);

    // Get stats
    let stats = pool.stats();
    assert!(stats.total_buffers > 0);
}

/// Test buffer pool reuse efficiency
#[test]
fn test_buffer_pool_reuse() {
    let mut pool = BufferPool::new(4096);

    // Allocate and release a few buffers of different sizes
    let buffer_sizes = [10, 20, 30, 40, 50];

    for size in buffer_sizes.iter() {
        let buffer = pool.get_buffer(*size);
        assert_eq!(buffer.len(), *size);
        pool.return_buffer(buffer);
    }

    // Now allocate the same sizes again - should reuse from pool
    for size in buffer_sizes.iter() {
        let buffer = pool.get_buffer(*size);
        assert_eq!(buffer.len(), *size);
        pool.return_buffer(buffer);
    }

    // Allocate a buffer larger than max size
    let large_buffer = pool.get_buffer(8192); // Larger than our 4KB max
    assert_eq!(large_buffer.len(), 8192);

    // Clear the pool
    pool.clear();

    // Stats should show empty pool
    let stats = pool.stats();
    assert_eq!(stats.total_buffers, 0);
}

/// Test memory strategy types
#[test]
fn test_memory_strategy_types() {
    // Test different memory strategy types
    let zero_copy = MemoryStrategy::ZeroCopy;
    let bounded_copy = MemoryStrategy::BoundedCopy;
    let isolated = MemoryStrategy::Isolated;

    // Check they're different
    assert_ne!(zero_copy, bounded_copy);
    assert_ne!(zero_copy, isolated);
    assert_ne!(bounded_copy, isolated);

    // Check each type can be converted to its string representation
    assert_eq!(format!("{:?}", zero_copy), "ZeroCopy");
    assert_eq!(format!("{:?}", bounded_copy), "BoundedCopy");
    assert_eq!(format!("{:?}", isolated), "Isolated");
}

/// Test ZeroCopy optimization strategy
#[test]
fn test_zero_copy_strategy() {
    let strategy = ZeroCopyStrategy::default();
    assert_eq!(strategy.name(), "ZeroCopy");
    assert_eq!(strategy.memory_strategy_type(), MemoryStrategy::ZeroCopy);

    // Test memory copy
    let source = vec![1, 2, 3, 4, 5];
    let mut destination = vec![0; 5];

    strategy.copy_memory(&source, &mut destination, 0, 5).unwrap();
    assert_eq!(destination, vec![1, 2, 3, 4, 5]);

    // Test with offset
    let mut destination = vec![0; 3];
    strategy.copy_memory(&source, &mut destination, 2, 3).unwrap();
    assert_eq!(destination, vec![3, 4, 5]);

    // Test is_appropriate_for
    // Should be appropriate for trusted components in same runtime
    assert!(strategy.is_appropriate_for(3, 3, true));
    // Should not be appropriate for untrusted components
    assert!(!strategy.is_appropriate_for(1, 3, true));
    // Should not be appropriate for different runtimes
    assert!(!strategy.is_appropriate_for(3, 3, false));
}

/// Test BoundedCopy optimization strategy
#[test]
fn test_bounded_copy_strategy() {
    let buffer_pool = Arc::new(RwLock::new(BufferPool::new(1024 * 1024)));
    let strategy = BoundedCopyStrategy::new(buffer_pool, 1024, 1);

    assert_eq!(strategy.name(), "BoundedCopy");
    assert_eq!(strategy.memory_strategy_type(), MemoryStrategy::BoundedCopy);

    // Test memory copy
    let source = vec![1, 2, 3, 4, 5];
    let mut destination = vec![0; 5];

    strategy.copy_memory(&source, &mut destination, 0, 5).unwrap();
    assert_eq!(destination, vec![1, 2, 3, 4, 5]);

    // Test with boundaries
    // This should work fine
    let large_source = vec![0; 1024];
    let mut large_dest = vec![0; 1024];
    assert!(strategy.copy_memory(&large_source, &mut large_dest, 0, 1024).is_ok());

    // This should fail (exceeds max_copy_size)
    let too_large_source = vec![0; 2048];
    let mut too_large_dest = vec![0; 2048];
    assert!(strategy.copy_memory(&too_large_source, &mut too_large_dest, 0, 2048).is_err());

    // Test is_appropriate_for
    // Should be appropriate for components with minimum trust level
    assert!(strategy.is_appropriate_for(1, 1, false));
    // Should not be appropriate for untrusted components
    assert!(!strategy.is_appropriate_for(0, 1, false));
}

/// Test FullIsolation optimization strategy
#[test]
fn test_full_isolation_strategy() {
    let strategy = FullIsolationStrategy::default();

    assert_eq!(strategy.name(), "FullIsolation");
    assert_eq!(strategy.memory_strategy_type(), MemoryStrategy::Isolated);

    // Test memory copy with sanitization
    let source = vec![1, 2, 3, 4, 5];
    let mut destination = vec![0; 5];

    strategy.copy_memory(&source, &mut destination, 0, 5).unwrap();
    assert_eq!(destination, vec![1, 2, 3, 4, 5]);

    // This should fail (exceeds max_copy_size of 16KB in default settings)
    let large_source = vec![0; 20 * 1024]; // 20KB
    let mut large_dest = vec![0; 20 * 1024];
    assert!(strategy.copy_memory(&large_source, &mut large_dest, 0, 20 * 1024).is_err());

    // Test is_appropriate_for - should work for any trust level
    assert!(strategy.is_appropriate_for(0, 0, false));
}

/// Test dynamic strategy selection
#[test]
fn test_strategy_selection() {
    use wrt_component::strategies::create_memory_strategy;

    // High trust + same runtime should get ZeroCopy
    let strategy = create_memory_strategy(3, 3, true);
    assert_eq!(strategy.name(), "ZeroCopy");

    // Medium trust should get BoundedCopy
    let strategy = create_memory_strategy(2, 2, false);
    assert_eq!(strategy.name(), "BoundedCopy");

    // Low trust should get FullIsolation
    let strategy = create_memory_strategy(0, 0, false);
    assert_eq!(strategy.name(), "FullIsolation");
}

/// Test strategy cloning
#[test]
fn test_strategy_cloning() {
    let zero_copy = ZeroCopyStrategy::default();
    let cloned = zero_copy.clone_strategy();
    assert_eq!(cloned.name(), "ZeroCopy");

    let bounded_copy = BoundedCopyStrategy::default();
    let cloned = bounded_copy.clone_strategy();
    assert_eq!(cloned.name(), "BoundedCopy");

    let full_isolation = FullIsolationStrategy::default();
    let cloned = full_isolation.clone_strategy();
    assert_eq!(cloned.name(), "FullIsolation");
}

/// Test memory bounds checking in strategies
#[test]
fn test_memory_bounds_checking() {
    let strategy = ZeroCopyStrategy::default();

    // Source buffer
    let source = vec![1, 2, 3, 4, 5];

    // Destination buffer too small
    let mut small_dest = vec![0; 3];

    // This should fail because we're trying to copy 5 bytes into a 3-byte buffer
    assert!(strategy.copy_memory(&source, &mut small_dest, 0, 5).is_err());

    // This should fail because the offset + size exceeds source length
    assert!(strategy.copy_memory(&source, &mut small_dest, 3, 3).is_err());

    // This should succeed
    assert!(strategy.copy_memory(&source, &mut small_dest, 0, 3).is_ok());
    assert_eq!(small_dest, vec![1, 2, 3]);
}

/// Integration test combining multiple strategies
#[test]
fn test_integration_with_multiple_strategies() {
    // Set up different strategies
    let zero_copy = ZeroCopyStrategy::default();
    let bounded_copy = BoundedCopyStrategy::default();
    let full_isolation = FullIsolationStrategy::default();

    // Test data
    let source_data = vec![10, 20, 30, 40, 50];
    let mut dest1 = vec![0; 5];
    let mut dest2 = vec![0; 5];
    let mut dest3 = vec![0; 5];

    // Apply each strategy
    zero_copy.copy_memory(&source_data, &mut dest1, 0, 5).unwrap();
    bounded_copy.copy_memory(&source_data, &mut dest2, 0, 5).unwrap();
    full_isolation.copy_memory(&source_data, &mut dest3, 0, 5).unwrap();

    // All should have the same result
    assert_eq!(dest1, vec![10, 20, 30, 40, 50]);
    assert_eq!(dest2, vec![10, 20, 30, 40, 50]);
    assert_eq!(dest3, vec![10, 20, 30, 40, 50]);

    // Modify source after copy
    let mut source_data = source_data;
    source_data[0] = 99;

    // Destinations should remain unchanged
    assert_eq!(dest1, vec![10, 20, 30, 40, 50]);
    assert_eq!(dest2, vec![10, 20, 30, 40, 50]);
    assert_eq!(dest3, vec![10, 20, 30, 40, 50]);
}
