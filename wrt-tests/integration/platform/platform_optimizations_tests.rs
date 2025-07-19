#![cfg(test)]
#![cfg(any(target_os = "macos", target_os = "linux"))]
#![cfg(feature = "platform-memory")]
#![deny(warnings)]

//! Integration tests for platform-specific memory optimizations
//! These tests verify that the platform-specific optimizations work correctly
//! with bounded collections and other components.

use wrt_foundation::{
    BoundedQueue, BoundedMap, BoundedSet, BoundedDeque,
    VerificationLevel, NoStdProvider, bounded::{BoundedVec, BoundedString},
    safe_managed_alloc, budget_aware_provider::CrateId,
};

// Import platform-specific items that actually exist
use wrt_platform::{
    PlatformMemoryOptimizer, MemoryOptimization,
    PlatformOptimizedProviderBuilder,
};

use std::time::{Instant, Duration};
use std::string::String;
use std::vec::Vec;

// Helper to benchmark collection operations
fn benchmark_operations<F>(name: &str, operations: F) -> Duration
where
    F: FnOnce(),
{
    let start = Instant::now);
    operations);
    let duration = start.elapsed);
    println!("{} took: {:?}", name, duration;
    duration
}

#[test]
fn test_memory_optimization_configurations() {
    // Test creating optimization builder with various configurations
    let builder = PlatformOptimizedProviderBuilder::default()
        .with_size(4096)
        .with_verification_level(VerificationLevel::Critical)
        .with_optimization(MemoryOptimization::HardwareAcceleration)
        .with_optimization(MemoryOptimization::SecureZeroing;
    
    // The builder is created successfully
    assert_eq!(builder.size(), 4096;
    assert_eq!(builder.verification_level(), VerificationLevel::Critical;
}

#[test]
fn test_bounded_collections_with_standard_provider() {
    // Create a standard provider using safe_managed_alloc
    let provider = safe_managed_alloc!(4096, CrateId::Test).unwrap();
    
    // Test BoundedQueue
    let mut queue = BoundedQueue::<u32, 100, _>::new(provider.clone()).unwrap();
    
    // Benchmark adding 50 items
    let enqueue_duration = benchmark_operations("BoundedQueue enqueue", || {
        for i in 0..50 {
            queue.enqueue(i).unwrap();
        }
    };
    
    // Benchmark removing 25 items
    let dequeue_duration = benchmark_operations("BoundedQueue dequeue", || {
        for _ in 0..25 {
            queue.dequeue().unwrap();
        }
    };
    
    // Test BoundedMap
    let provider2 = safe_managed_alloc!(8192, CrateId::Test).unwrap();
    let mut map = BoundedMap::<u32, u32, 100, _>::new(provider2).unwrap();
    
    // Benchmark adding 50 items
    let insert_duration = benchmark_operations("BoundedMap insert", || {
        for i in 0..50 {
            map.insert(i, i * 2).unwrap();
        }
    };
    
    // Benchmark looking up 50 items
    let lookup_duration = benchmark_operations("BoundedMap lookup", || {
        for i in 0..50 {
            let _ = map.get(&i;
        }
    };
    
    // Print results
    println!("Performance results:";
    println!("  Queue enqueue: {:?}", enqueue_duration;
    println!("  Queue dequeue: {:?}", dequeue_duration;
    println!("  Map insert: {:?}", insert_duration;
    println!("  Map lookup: {:?}", lookup_duration;
}

#[test]
fn test_memory_optimizer() {
    // Test the PlatformMemoryOptimizer
    let optimizer = PlatformMemoryOptimizer::new);
    
    // Check available optimizations
    let available = optimizer.available_optimizations);
    println!("Available optimizations: {:?}", available;
    
    // The optimizer should support at least basic optimizations
    assert!(!available.is_empty();
}

#[test]
fn test_bounded_vec_with_different_sizes() {
    // Test various sized allocations
    let sizes = [1024, 2048, 4096, 8192];
    
    for size in &sizes {
        let provider = safe_managed_alloc!(*size, CrateId::Test).unwrap();
        let mut vec = BoundedVec::<u8, 1024, _>::new(provider).unwrap();
        
        // Fill with test data
        let fill_duration = benchmark_operations(&format!("BoundedVec fill (size {})", size), || {
            for i in 0..100 {
                vec.push(i as u8).unwrap();
            }
        };
        
        println!("Fill duration for size {}: {:?}", size, fill_duration;
    }
}

#[test]
fn test_verification_levels() {
    // Test different verification levels
    let levels = [
        VerificationLevel::Off,
        VerificationLevel::Minimal,
        VerificationLevel::Standard,
        VerificationLevel::Full,
        VerificationLevel::Critical,
    ];
    
    for level in &levels {
        let builder = PlatformOptimizedProviderBuilder::default()
            .with_size(2048)
            .with_verification_level(*level;
        
        // Verify the builder accepts the level
        assert_eq!(builder.verification_level(), *level;
        println!("Successfully configured verification level: {:?}", level;
    }
}

#[test]
fn test_memory_optimization_flags() {
    // Test individual optimization flags
    let optimizations = [
        MemoryOptimization::HardwareAcceleration,
        MemoryOptimization::SecureZeroing,
        MemoryOptimization::CachePrefetch,
        MemoryOptimization::AlignmentOptimization,
    ];
    
    for opt in &optimizations {
        let builder = PlatformOptimizedProviderBuilder::default()
            .with_optimization(*opt;
        
        println!("Successfully configured optimization: {:?}", opt;
    }
}