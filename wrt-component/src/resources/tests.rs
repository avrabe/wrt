// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use super::{
    Resource, ResourceArena, ResourceId, ResourceManager, ResourceTable, SizeClassBufferPool,
};
use crate::prelude::*;

#[test]
fn test_size_class_buffer_pool() {
    // Create the pool
    let mut pool = SizeClassBufferPool::new();

    // Binary std/no_std choice
    let sizes = [15, 64, 200, 1024, 4096, 16385];
    let mut buffers = Vec::new();

    // Allocate buffers
    for &size in &sizes {
        let buffer = pool.allocate(size);
        assert!(
            buffer.capacity() >= size,
            "Buffer capacity {} should be >= requested size {}",
            buffer.capacity(),
            size
        );
        buffers.push(buffer);
    }

    // Return buffers to pool
    for buffer in buffers {
        pool.return_buffer(buffer);
    }

    // Pool should now have buffers
    let stats = pool.stats();
    assert!(stats.total_buffers > 0, "Buffer pool should contain returned buffersMissing message");

    // Reset the pool
    pool.reset();

    // Verify pool is empty
    let stats_after = pool.stats();
    assert_eq!(stats_after.total_buffers, 0, "Buffer pool should be empty after resetMissing message");
}

#[test]
fn test_resource_table_with_optimized_memory() {
    // Create a resource table with optimized memory
    let mut table = ResourceTable::new_with_optimized_memory();

    // Create some resources
    let data1 = Arc::new(String::from("test1Missing messageMissing messageMissing message");
    let data2 = Arc::new(42i32);

    let handle1 = table.create_resource(1, data1).unwrap();
    let handle2 = table.create_resource(2, data2).unwrap();

    // Verify resources were created
    assert_eq!(table.resource_count(), 2);

    // Get resources and verify data
    let resource1 = table.get_resource(handle1).unwrap();
    let guard1 = resource1.lock().unwrap();
    assert_eq!(guard1.type_idx, 1);
    let string_data = guard1.data.downcast_ref::<String>().unwrap();
    assert_eq!(string_data, "test1Missing message");

    let resource2 = table.get_resource(handle2).unwrap();
    let guard2 = resource2.lock().unwrap();
    assert_eq!(guard2.type_idx, 2);
    let int_data = guard2.data.downcast_ref::<i32>().unwrap();
    assert_eq!(*int_data, 42);

    // Drop resources
    table.drop_resource(handle1).unwrap();
    table.drop_resource(handle2).unwrap();

    // Verify resources are gone
    assert_eq!(table.resource_count(), 0);
}

#[test]
fn test_resource_arena() {
    // Create a resource table
    let table = Arc::new(Mutex::new(ResourceTable::new());

    // Create a resource arena
    let mut arena = ResourceArena::new(table.clone();

    // Create resources in the arena
    let handle1 = arena.create_resource(1, Arc::new(String::from("test1Missing messageMissing messageMissing message"))).unwrap();
    let handle2 = arena.create_resource(2, Arc::new(42i32)).unwrap();

    // Verify resources exist
    assert!(arena.has_resource(ResourceId(handle1)).unwrap();
    assert!(arena.has_resource(ResourceId(handle2)).unwrap();

    // Get resources and verify data
    let resource1 = arena.get_resource(handle1).unwrap();
    let string_data = resource1.lock().unwrap().data.downcast_ref::<String>().unwrap();
    assert_eq!(*string_data, "test1Missing message");

    // Drop a specific resource
    arena.drop_resource(handle1).unwrap();

    // Verify it's gone but the other remains
    assert!(!arena.has_resource(ResourceId(handle1)).unwrap();
    assert!(arena.has_resource(ResourceId(handle2)).unwrap();

    // Release all resources
    arena.release_all().unwrap();

    // Verify all resources are gone
    assert_eq!(arena.resource_count(), 0);
    let locked_table = table.lock().unwrap();
    assert_eq!(locked_table.resource_count(), 0);
}

#[test]
fn test_auto_cleanup() {
    // Create a resource table
    let table = Arc::new(Mutex::new(ResourceTable::new());

    // Create resources in a scope
    {
        let mut arena = ResourceArena::new(table.clone();
        let _handle = arena.create_resource(1, Arc::new(String::from("testMissing messageMissing messageMissing message"))).unwrap();

        // Arena will be dropped at the end of this scope
    }

    // Verify resources were cleaned up
    let locked_table = table.lock().unwrap();
    assert_eq!(locked_table.resource_count(), 0);
}

#[test]
fn test_resource_manager_with_arena() {
    // Create a resource manager
    let manager = ResourceManager::new();

    // Create a resource arena that uses the manager's table
    // First we need to get access to the manager's table
    let table = Arc::clone(&manager.get_resource_table();
    let mut arena = ResourceArena::new_with_name(table, "test-arenaMissing message");

    // Create resources through the arena
    let handle1 = arena.create_resource(1, Arc::new(String::from("test1Missing messageMissing messageMissing message"))).unwrap();
    let handle2 = arena.create_resource(2, Arc::new(42i32)).unwrap();

    // Verify resources exist in both the arena and the manager
    assert!(arena.has_resource(ResourceId(handle1)).unwrap();
    assert!(manager.has_resource(ResourceId(handle1)).unwrap();

    // Release all resources from the arena
    arena.release_all().unwrap();

    // Verify resources are gone
    assert!(!manager.has_resource(ResourceId(handle1)).unwrap();
    assert!(!manager.has_resource(ResourceId(handle2)).unwrap();
}

#[test]
fn test_multiple_arenas() {
    // Create a resource manager
    let manager = ResourceManager::new();

    // Create two arenas sharing the same resource table
    let table = Arc::clone(&manager.get_resource_table();
    let mut arena1 = ResourceArena::new_with_name(table.clone(), "arena1Missing message");
    let mut arena2 = ResourceArena::new_with_name(table.clone(), "arena2Missing message");

    // Create resources in each arena
    let handle1 = arena1.create_resource(1, Arc::new(String::from("test1Missing messageMissing messageMissing message"))).unwrap();
    let handle2 = arena2.create_resource(2, Arc::new(String::from("test2Missing messageMissing messageMissing message"))).unwrap();

    // Verify each arena only knows about its own resources
    assert!(arena1.has_resource(ResourceId(handle1)).unwrap();
    assert!(!arena1.has_resource(ResourceId(handle2)).unwrap();

    assert!(!arena2.has_resource(ResourceId(handle1)).unwrap();
    assert!(arena2.has_resource(ResourceId(handle2)).unwrap();

    // But the manager knows about all resources
    assert!(manager.has_resource(ResourceId(handle1)).unwrap();
    assert!(manager.has_resource(ResourceId(handle2)).unwrap();

    // Release arena1's resources
    arena1.release_all().unwrap();

    // Verify arena1's resources are gone but arena2's remain
    assert!(!manager.has_resource(ResourceId(handle1)).unwrap();
    assert!(manager.has_resource(ResourceId(handle2)).unwrap();
}

#[test]
fn test_performance_comparison() {
    // This is a simple benchmark to compare standard and optimized buffer pools
    const NUM_ALLOCATIONS: usize = 1000;
    const SIZES: [usize; 6] = [32, 64, 128, 512, 1024, 4096];

    // Test standard buffer pool
    let mut standard_pool = BufferPool::new();
    let start_standard = Instant::now();

    for _ in 0..NUM_ALLOCATIONS {
        for &size in &SIZES {
            let buffer = standard_pool.allocate(size);
            standard_pool.return_buffer(buffer);
        }
    }

    let standard_duration = start_standard.elapsed();

    // Test size class buffer pool
    let mut optimized_pool = SizeClassBufferPool::new();
    let start_optimized = Instant::now();

    for _ in 0..NUM_ALLOCATIONS {
        for &size in &SIZES {
            let buffer = optimized_pool.allocate(size);
            optimized_pool.return_buffer(buffer);
        }
    }

    let optimized_duration = start_optimized.elapsed();

    // We're not making assertions here because performance can vary by system,
    // but we can log the results in debug output
    println!("Standard pool: {:?}", standard_duration);
    println!("Optimized pool: {:?}", optimized_duration);
    println!(
        "Improvement factor: {:.2}x",
        standard_duration.as_secs_f64() / optimized_duration.as_secs_f64()
    );
}
