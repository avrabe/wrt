//! Component Model resource type implementation
//!
//! This module provides resource type handling for the WebAssembly Component
//! Model, including resource lifetime management, memory optimization, and
//! interception support.

#[cfg(feature = "std")]
use std::sync::Weak;

use crate::prelude::*;

// Submodules
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod bounded_buffer_pool;
#[cfg(feature = "std")]
pub mod buffer_pool;
#[cfg(feature = "std")]
pub mod memory_access;
pub mod memory_strategy;
#[cfg(feature = "std")]
pub mod resource_arena;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod resource_arena_no_std;
pub mod resource_builder;
pub mod resource_interceptor;
#[cfg(feature = "std")]
pub mod resource_manager;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod resource_manager_no_std;
#[cfg(feature = "std")]
pub mod resource_operation;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod resource_operation_no_std;
pub mod resource_strategy;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod resource_strategy_no_std;
#[cfg(feature = "std")]
pub mod resource_table;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod resource_table_no_std;
#[cfg(feature = "std")]
pub mod size_class_buffer_pool;

#[cfg(test)]
mod tests;

// Re-export for no_std feature
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use bounded_buffer_pool::{BoundedBufferPool, BoundedBufferStats as BufferPoolStats};
// Re-export for std feature
#[cfg(feature = "std")]
pub use buffer_pool::BufferPool;
#[cfg(feature = "std")]
pub use memory_access::MemoryAccessMode;
// Common re-exports for both std and no_std
pub use memory_strategy::MemoryStrategy as MemoryStrategyTrait;
// Export ResourceArena based on feature flags
#[cfg(feature = "std")]
pub use resource_arena::ResourceArena;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use resource_arena_no_std::ResourceArena;
// Export Builder types
pub use resource_builder::{ResourceBuilder, ResourceManagerBuilder, ResourceTableBuilder};
// Export ResourceInterceptor
pub use resource_interceptor::ResourceInterceptor;
// Export ResourceId and ResourceManager based on feature flags
#[cfg(feature = "std")]
pub use resource_manager::{ResourceId, ResourceManager};
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use resource_manager_no_std::{ResourceId, ResourceManager};
// Export resource_operation based on feature flags
#[cfg(feature = "std")]
pub use resource_operation::{from_format_resource_operation, to_format_resource_operation};
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use resource_operation_no_std::{from_format_resource_operation, to_format_resource_operation};
// Export ResourceStrategy
pub use resource_strategy::ResourceStrategy;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use resource_strategy_no_std::{ResourceStrategyNoStd, MAX_BUFFER_SIZE};
// Export ResourceTable components based on feature flags
#[cfg(feature = "std")]
pub use resource_table::{
    BufferPoolTrait, MemoryStrategy, Resource, ResourceTable, VerificationLevel,
};
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use resource_table_no_std::{
    BufferPoolTrait, MemoryStrategy, Resource, ResourceTable, VerificationLevel,
};
// Export size class buffer pool for std environment
#[cfg(feature = "std")]
pub use size_class_buffer_pool::{BufferPoolStats, SizeClassBufferPool};

/// Timestamp implementation for no_std
#[cfg(all(not(feature = "std"), feature = "alloc"))]
#[derive(Debug, Clone, Copy)]
pub struct Instant {
    // Store a monotonic counter for elapsed time simulation
    dummy: u64,
}

#[cfg(all(not(feature = "std"), feature = "alloc"))]
impl Instant {
    // Create a new instant at the current monotonic time
    pub fn now() -> Self {
        // In a real implementation, we might read from a hardware timer
        // Here we just use a placeholder value for no_std compatibility
        Self { dummy: 0 }
    }

    // Get the elapsed time since this instant was created
    pub fn elapsed(&self) -> Duration {
        // In a real implementation, we'd compare with the current monotonic time
        // Here we just return zero for no_std compatibility
        Duration::from_secs(0)
    }

    // Calculate the duration between two instants
    pub fn duration_since(&self, earlier: &Self) -> Duration {
        // Just a placeholder implementation for no_std
        Duration::from_secs(0)
    }
}
