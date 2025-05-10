use std::collections::HashMap;

/// A buffer pool for reusing memory allocations
pub struct BufferPool {
    /// Map of buffer sizes to pools of buffers
    pools: HashMap<usize, Vec<Vec<u8>>>,
    /// Maximum buffer size to keep in the pool
    max_buffer_size: usize,
    /// Maximum number of buffers per size
    max_buffers_per_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool with default settings
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            max_buffer_size: 1024 * 1024, // 1MB default max size
            max_buffers_per_size: 10,
        }
    }

    /// Create a new buffer pool with custom max buffer size
    pub fn new_with_config(max_buffer_size: usize, max_buffers_per_size: usize) -> Self {
        Self { pools: HashMap::new(), max_buffer_size, max_buffers_per_size }
    }

    /// Allocate a buffer of at least the specified size
    pub fn allocate(&mut self, min_size: usize) -> Vec<u8> {
        // Try to find a buffer of the right size
        if let Some(buffers) = self.pools.get_mut(&min_size) {
            if let Some(buffer) = buffers.pop() {
                return buffer;
            }
        }

        // No buffer available, create a new one
        Vec::with_capacity(min_size)
    }

    /// Return a buffer to the pool
    pub fn return_buffer(&mut self, mut buffer: Vec<u8>) {
        let size = buffer.capacity();

        // Only keep reasonably sized buffers
        if size <= self.max_buffer_size {
            // Clear the buffer before returning it to the pool
            buffer.clear();

            // Add to the pool if we have space
            let buffers = self.pools.entry(size).or_insert_with(Vec::new);
            if buffers.len() < self.max_buffers_per_size {
                buffers.push(buffer);
            }
        }
    }

    /// Reset the buffer pool, clearing all pooled buffers
    pub fn reset(&mut self) {
        self.pools.clear();
    }

    /// Get stats about the buffer pool
    pub fn stats(&self) -> BufferPoolStats {
        let mut total_buffers = 0;
        let mut total_capacity = 0;

        for (size, buffers) in &self.pools {
            total_buffers += buffers.len();
            total_capacity += size * buffers.len();
        }

        BufferPoolStats { total_buffers, total_capacity, size_count: self.pools.len() }
    }
}

/// Statistics about a buffer pool
pub struct BufferPoolStats {
    /// Total number of buffers in the pool
    pub total_buffers: usize,
    /// Total capacity of all buffers in bytes
    pub total_capacity: usize,
    /// Number of different buffer sizes
    pub size_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_allocation() {
        let mut pool = BufferPool::new();

        // Allocate a buffer
        let buffer = pool.allocate(100);
        assert_eq!(buffer.capacity(), 100);

        // Return it to the pool
        pool.return_buffer(buffer);

        // Allocate again, should reuse
        let buffer2 = pool.allocate(100);
        assert_eq!(buffer2.capacity(), 100);
    }

    #[test]
    fn test_buffer_pool_reset() {
        let mut pool = BufferPool::new();

        // Allocate and return some buffers
        pool.return_buffer(pool.allocate(100));
        pool.return_buffer(pool.allocate(200));

        // Check stats
        let stats_before = pool.stats();
        assert_eq!(stats_before.total_buffers, 2);

        // Reset the pool
        pool.reset();

        // Check stats again
        let stats_after = pool.stats();
        assert_eq!(stats_after.total_buffers, 0);
    }
}
