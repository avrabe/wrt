// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

/// Statistics about a buffer pool
pub struct BufferPoolStats {
    /// Total number of buffers in the pool
    pub total_buffers: usize,
    /// Total capacity of all buffers in bytes
    pub total_capacity: usize,
    /// Number of different buffer sizes
    pub size_count: usize,
}

/// Binary std/no_std choice
///
/// This implementation uses power-of-two size classes for better memory reuse
/// and reduced fragmentation compared to the basic buffer pool.
pub struct SizeClassBufferPool {
    /// Power-of-two size classes from 16B to 16KB
    size_classes: [Vec<Vec<u8>>; 11], // 16, 32, 64, 128, 256, 512, 1K, 2K, 4K, 8K, 16K
    /// Pools for sizes larger than size classes
    overflow_pools: BTreeMap<usize, Vec<Vec<u8>>>,
    /// Maximum buffers per size class
    max_buffers_per_class: usize,
    /// Maximum buffer size to keep in the pool
    max_buffer_size: usize,
}

impl SizeClassBufferPool {
    /// Create a new size class buffer pool with default settings
    pub fn new() -> Self {
        Self::new_with_config(10, 1024 * 1024) // 10 buffers per class, 1MB max
                                               // size
    }

    /// Create a new buffer pool with specific max buffers per class
    pub fn new_with_config(max_buffers_per_class: usize, max_buffer_size: usize) -> Self {
        Self {
            // Initialize 11 empty vectors for each size class
            size_classes: Default::default(),
            overflow_pools: BTreeMap::new(),
            max_buffers_per_class,
            max_buffer_size,
        }
    }

    /// Create a buffer pool with a specified size
    pub fn new_with_size(size: usize) -> Self {
        Self::new_with_config(
            // Adjust max buffers per class based on size
            // More classes for larger size pools
            if size > 1024 * 1024 { 20 } else { 8 },
            // Max size depends on the pool size
            size,
        )
    }

    /// Allocate a buffer of at least the specified size
    pub fn allocate(&mut self, size: usize) -> Vec<u8> {
        // Find the appropriate size class (next power of 2)
        let class_size = size.next_power_of_two();

        // If the size is too small, use minimum 16 bytes
        let class_size = std::cmp::max(class_size, 16);

        if class_size <= 16384 {
            // Calculate the size class index: log2(size) - 4
            // 16 => 0, 32 => 1, 64 => 2, etc.
            let class_idx = (class_size.trailing_zeros() as usize).saturating_sub(4);

            // Try to get from size class pool
            if class_idx < self.size_classes.len() {
                if let Some(buffer) = self.size_classes[class_idx].pop() {
                    return buffer;
                }
            }

            // Create new buffer with exact class size
            Vec::with_capacity(class_size)
        } else {
            // For larger buffers, use the overflow pool
            self.allocate_overflow(size)
        }
    }

    /// Allocate a buffer from the overflow pool
    fn allocate_overflow(&mut self, size: usize) -> Vec<u8> {
        let size = size.next_power_of_two();

        // Try to get a buffer from the overflow pool
        if let Some(buffers) = self.overflow_pools.get_mut(&size) {
            if let Some(buffer) = buffers.pop() {
                return buffer;
            }
        }

        // Create new buffer
        Vec::with_capacity(size)
    }

    /// Return a buffer to the pool
    pub fn return_buffer(&mut self, mut buffer: Vec<u8>) {
        let capacity = buffer.capacity();

        // Don't keep oversized buffers
        if capacity > self.max_buffer_size {
            return;
        }

        // Clear the buffer before returning it
        buffer.clear();

        if capacity <= 16384 {
            // Find the right size class
            match capacity {
                16 => self.return_to_class(0, buffer),
                32 => self.return_to_class(1, buffer),
                64 => self.return_to_class(2, buffer),
                128 => self.return_to_class(3, buffer),
                256 => self.return_to_class(4, buffer),
                512 => self.return_to_class(5, buffer),
                1024 => self.return_to_class(6, buffer),
                2048 => self.return_to_class(7, buffer),
                4096 => self.return_to_class(8, buffer),
                8192 => self.return_to_class(9, buffer),
                16384 => self.return_to_class(10, buffer),
                // Unusual size (not power of 2)
                _ => self.return_to_overflow(capacity, buffer),
            }
        } else {
            // Return to overflow pool
            self.return_to_overflow(capacity, buffer);
        }
    }

    /// Return a buffer to a specific size class
    fn return_to_class(&mut self, class_idx: usize, buffer: Vec<u8>) {
        if class_idx < self.size_classes.len() {
            let class_buffers = &mut self.size_classes[class_idx];
            if class_buffers.len() < self.max_buffers_per_class {
                class_buffers.push(buffer);
            }
        }
    }

    /// Return a buffer to the overflow pool
    fn return_to_overflow(&mut self, capacity: usize, buffer: Vec<u8>) {
        let buffers = self.overflow_pools.entry(capacity).or_insert_with(Vec::new);
        if buffers.len() < self.max_buffers_per_class {
            buffers.push(buffer);
        }
    }

    /// Reset the buffer pool, clearing all pooled buffers
    pub fn reset(&mut self) {
        for class in &mut self.size_classes {
            class.clear();
        }
        self.overflow_pools.clear();
    }

    /// Get statistics about the buffer pool
    pub fn stats(&self) -> BufferPoolStats {
        let mut total_buffers = 0;
        let mut total_capacity = 0;

        // Count buffers in size classes
        for (i, class) in self.size_classes.iter().enumerate() {
            let class_size = 16 << i; // 16, 32, 64, ...
            total_buffers += class.len();
            total_capacity += class_size * class.len();
        }

        // Count buffers in overflow pools
        for (size, buffers) in &self.overflow_pools {
            total_buffers += buffers.len();
            total_capacity += size * buffers.len();
        }

        BufferPoolStats {
            total_buffers,
            total_capacity,
            size_count: 11 + self.overflow_pools.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_from_size_class() {
        let mut pool = SizeClassBufferPool::new();

        // Allocate a buffer
        let buffer = pool.allocate(100);
        // Should round up to 128 bytes (next power of 2)
        assert_eq!(buffer.capacity(), 128);

        // Return to pool
        pool.return_buffer(buffer);

        // Allocate again
        let buffer2 = pool.allocate(100);
        // Should get the same buffer back
        assert_eq!(buffer2.capacity(), 128);
    }

    #[test]
    fn test_size_classes() {
        let mut pool = SizeClassBufferPool::new();

        // Test a few size classes
        let sizes = [15, 32, 33, 500, 1025, 4000, 16385];
        let expected = [16, 32, 64, 512, 2048, 4096, 32768];

        for (i, &size) in sizes.iter().enumerate() {
            let buffer = pool.allocate(size);
            assert_eq!(
                buffer.capacity(),
                expected[i],
                "Expected size {} for request {}",
                expected[i],
                size
            );
        }
    }

    #[test]
    fn test_return_buffer_limit() {
        let mut pool = SizeClassBufferPool::new_with_config(2, 1024);

        // Allocate and return 3 buffers of the same size
        let buffer1 = pool.allocate(64);
        let buffer2 = pool.allocate(64);
        let buffer3 = pool.allocate(64);

        pool.return_buffer(buffer1);
        pool.return_buffer(buffer2);
        pool.return_buffer(buffer3);

        // Pool should only have kept 2
        let stats = pool.stats();
        assert_eq!(stats.total_buffers, 2);
    }

    #[test]
    fn test_max_buffer_size() {
        let mut pool = SizeClassBufferPool::new_with_config(10, 1024);

        // Allocate a large buffer
        let large_buffer = pool.allocate(2048);

        // Return it to the pool
        pool.return_buffer(large_buffer);

        // Should not be kept because it's too large
        let stats = pool.stats();
        assert_eq!(stats.total_buffers, 0);
    }

    #[test]
    fn test_reset() {
        let mut pool = SizeClassBufferPool::new();

        // Allocate and return some buffers
        pool.return_buffer(pool.allocate(64);
        pool.return_buffer(pool.allocate(128);
        pool.return_buffer(pool.allocate(4096);

        // Check stats before reset
        let stats_before = pool.stats();
        assert_eq!(stats_before.total_buffers, 3);

        // Reset the pool
        pool.reset();

        // Check stats after reset
        let stats_after = pool.stats();
        assert_eq!(stats_after.total_buffers, 0);
    }
}
