#![cfg(test)]
#![cfg(any(target_os = "macos", target_os = "linux"))]
#![cfg(feature = "platform-memory")]
#![deny(warnings)]

//! Integration tests for platform-specific memory optimizations
//! These tests verify that the platform-specific optimizations work correctly
//! with bounded collections and other components.

use wrt_foundation::{
    BoundedQueue, BoundedMap, BoundedSet, BoundedDeque, BoundedBitSet,
    BoundedBuilder, StringBuilder, ResourceBuilder, MemoryBuilder,
    VerificationLevel, NoStdProvider, bounded::{BoundedVec, BoundedString, WasmName},
};

// Import platform-specific items
#[cfg(target_os = "macos")]
use wrt_platform::{
    MacOSOptimizedProvider, PlatformOptimizedProviderBuilder,
    PlatformMemoryOptimizer, MemoryOptimization,
    OptimizedQueue, OptimizedMap, OptimizedSet, OptimizedDeque, OptimizedVec
};

#[cfg(target_os = "linux")]
use wrt_platform::{
    LinuxOptimizedProvider, PlatformOptimizedProviderBuilder,
    PlatformMemoryOptimizer, MemoryOptimization,
    OptimizedQueue, OptimizedMap, OptimizedSet, OptimizedDeque, OptimizedVec
};

use std::time::{Instant, Duration};
use std::string::String;
use std::vec::Vec;

// Test helper to create platform-specific providers
#[cfg(target_os = "macos")]
fn create_platform_provider() -> MacOSOptimizedProvider {
    PlatformOptimizedProviderBuilder::new()
        .with_size(4096)
        .with_verification_level(VerificationLevel::Critical)
        .with_optimization(MemoryOptimization::HardwareAcceleration)
        .with_optimization(MemoryOptimization::SecureZeroing)
        .build()
}

#[cfg(target_os = "linux")]
fn create_platform_provider() -> LinuxOptimizedProvider {
    PlatformOptimizedProviderBuilder::new()
        .with_size(4096)
        .with_verification_level(VerificationLevel::Critical)
        .with_optimization(MemoryOptimization::HardwareAcceleration)
        .with_optimization(MemoryOptimization::SecureZeroing)
        .build()
}

// Helper to benchmark collection operations
fn benchmark_operations<F>(name: &str, operations: F) -> Duration
where
    F: FnOnce(),
{
    let start = Instant::now();
    operations();
    let duration = start.elapsed();
    println!("{} took: {:?}", name, duration);
    duration
}

#[test]
fn test_optimized_collections() {
    // Create a platform-specific provider
    let provider = create_platform_provider();
    
    // Test OptimizedQueue
    let mut queue = OptimizedQueue::<u32, 100>::new(provider.clone()).unwrap();
    
    // Benchmark adding 50 items
    let enqueue_duration = benchmark_operations("OptimizedQueue enqueue", || {
        for i in 0..50 {
            queue.enqueue(i).unwrap();
        }
    });
    
    // Benchmark removing 25 items
    let dequeue_duration = benchmark_operations("OptimizedQueue dequeue", || {
        for _ in 0..25 {
            queue.dequeue().unwrap();
        }
    });
    
    // Test OptimizedMap
    let mut map = OptimizedMap::<u32, String, 100>::new(provider.clone()).unwrap();
    
    // Benchmark adding 50 items
    let insert_duration = benchmark_operations("OptimizedMap insert", || {
        for i in 0..50 {
            map.insert(i, format!("value-{}", i)).unwrap();
        }
    });
    
    // Benchmark looking up 50 items
    let lookup_duration = benchmark_operations("OptimizedMap lookup", || {
        for i in 0..50 {
            map.get(&i).unwrap();
        }
    });
    
    // Test OptimizedSet
    let mut set = OptimizedSet::<u32, 100>::new(provider.clone()).unwrap();
    
    // Benchmark adding 50 items
    let set_insert_duration = benchmark_operations("OptimizedSet insert", || {
        for i in 0..50 {
            set.insert(i).unwrap();
        }
    });
    
    // Benchmark checking 50 items
    let set_contains_duration = benchmark_operations("OptimizedSet contains", || {
        for i in 0..100 {
            set.contains(&i).unwrap(); // Only first 50 will be true
        }
    });
    
    // Test OptimizedDeque
    let mut deque = OptimizedDeque::<u32, 100>::new(provider).unwrap();
    
    // Benchmark mixed operations
    let deque_operations_duration = benchmark_operations("OptimizedDeque operations", || {
        for i in 0..25 {
            deque.push_back(i).unwrap();
        }
        
        for i in 25..50 {
            deque.push_front(i).unwrap();
        }
        
        for _ in 0..10 {
            deque.pop_front().unwrap();
        }
        
        for _ in 0..10 {
            deque.pop_back().unwrap();
        }
    });
    
    // These assertions just make sure the operations complete in a reasonable time
    // Actual performance will vary by machine
    assert!(enqueue_duration < Duration::from_millis(100));
    assert!(dequeue_duration < Duration::from_millis(100));
    assert!(insert_duration < Duration::from_millis(100));
    assert!(lookup_duration < Duration::from_millis(100));
    assert!(set_insert_duration < Duration::from_millis(100));
    assert!(set_contains_duration < Duration::from_millis(100));
    assert!(deque_operations_duration < Duration::from_millis(100));
}

#[test]
fn test_memory_optimizer_operations() {
    // Create a platform-specific provider
    let provider = create_platform_provider();
    
    // Test zero-copy read (which may fall back to accelerated copy)
    let source = [1, 2, 3, 4, 5];
    let mut dest = [0; 5];
    
    let result = provider.zero_copy_read(&source, &mut dest);
    assert!(result.is_ok());
    assert_eq!(dest, source);
    
    // Test accelerated copy
    let source = [10, 20, 30, 40, 50];
    let mut dest = [0; 5];
    
    let result = provider.accelerated_copy(&source, &mut dest);
    assert!(result.is_ok());
    assert_eq!(dest, source);
    
    // Test memory alignment
    let ptr = dest.as_mut_ptr();
    let result = provider.align_memory(ptr, 8);
    assert!(result.is_ok());
    
    // Test secure zeroing
    let mut sensitive_data = [0xAA; 32];
    let result = provider.secure_zero(&mut sensitive_data);
    assert!(result.is_ok());
    assert_eq!(sensitive_data, [0; 32]);
}

#[test]
fn test_performance_comparison() {
    // Create both standard and optimized providers
    let std_provider = NoStdProvider::new(4096, VerificationLevel::Critical);
    let opt_provider = create_platform_provider();
    
    // Create standard and optimized collections
    let mut std_vec = BoundedVec::<u32, 1000, NoStdProvider<1024>>::new(std_provider.clone()).unwrap();
    let mut opt_vec = OptimizedVec::<u32, 1000>::new(opt_provider.clone()).unwrap();
    
    // Benchmark standard collection
    let std_duration = benchmark_operations("Standard BoundedVec operations", || {
        for i in 0..500 {
            std_vec.push(i).unwrap();
        }
        
        for i in 0..500 {
            std_vec.get(i).unwrap();
        }
    });
    
    // Benchmark optimized collection
    let opt_duration = benchmark_operations("Optimized BoundedVec operations", || {
        for i in 0..500 {
            opt_vec.push(i).unwrap();
        }
        
        for i in 0..500 {
            opt_vec.get(i).unwrap();
        }
    });
    
    // Print performance comparison
    println!("Performance ratio: {:.2}x", std_duration.as_micros() as f64 / opt_duration.as_micros() as f64);
    
    // The optimized version should generally be no slower than the standard version
    // In many cases it should be faster, but we can't guarantee by how much
    // So we just ensure it's not significantly slower
    assert!(opt_duration.as_micros() <= std_duration.as_micros() * 12 / 10); // Within 20% of standard
    
    // Additional test with larger data
    let data_size = 100000;
    
    // Create large test data
    let mut large_data = Vec::with_capacity(data_size);
    for i in 0..data_size {
        large_data.push((i % 256) as u8);
    }
    
    // Standard provider write and read
    let mut std_buffer = [0u8; 1000];
    let std_write_read = benchmark_operations("Standard provider write/read", || {
        let mut provider = NoStdProvider::new(data_size, VerificationLevel::Critical);
        provider.write_data(0, &large_data[..data_size]).unwrap();
        provider.read_data(0, &mut std_buffer).unwrap();
    });
    
    // Optimized provider write and read
    let mut opt_buffer = [0u8; 1000];
    let opt_write_read = benchmark_operations("Optimized provider write/read", || {
        let mut provider = create_platform_provider();
        provider.write_data(0, &large_data[..data_size]).unwrap();
        provider.read_data(0, &mut opt_buffer).unwrap();
    });
    
    println!("Bulk memory performance ratio: {:.2}x", 
        std_write_read.as_micros() as f64 / opt_write_read.as_micros() as f64);
        
    // Again, we're just ensuring the optimized version is not significantly slower
    assert!(opt_write_read.as_micros() <= std_write_read.as_micros() * 12 / 10); // Within 20% of standard
}

// Only run secure memory test if we're in release mode, as debug mode might not optimize away
#[cfg(not(debug_assertions))]
#[test]
fn test_secure_memory_operations() {
    let provider = create_platform_provider();
    
    // Create sensitive data
    let mut sensitive_data = [0xAA; 1024];
    
    // Benchmark secure zeroing
    let secure_zero_duration = benchmark_operations("Secure zeroing", || {
        provider.secure_zero(&mut sensitive_data).unwrap();
    });
    
    // Verify data is zeroed
    assert_eq!(sensitive_data, [0; 1024]);
    
    // Compare with standard zeroing
    let std_zero_duration = benchmark_operations("Standard zeroing", || {
        sensitive_data.fill(0);
    });
    
    println!("Zeroing comparison - Secure: {:?}, Standard: {:?}", 
        secure_zero_duration, std_zero_duration);
    
    // The secure zeroing should not be orders of magnitude slower
    // Just ensure it's within reasonable bounds (10x slower would be acceptable)
    assert!(secure_zero_duration < std_zero_duration * 10);
}