//! Memory adapter for wrt
//!
//! This module re-exports and adapts the memory adapter functionality
//! from wrt-runtime, ensuring consistent memory safety features across
//! all WRT components.

// Use our prelude for consistent imports
use crate::prelude::*;

// Re-export the memory adapter types from wrt-runtime
pub use wrt_runtime::memory_adapter::{MemoryAdapter, SafeMemoryAdapter, StdMemoryProvider};

/// Create a new memory adapter with the given memory type
///
/// This is a convenience function that creates a new SafeMemoryAdapter
/// with the specified memory type.
///
/// # Arguments
///
/// * `memory_type` - The memory type to use for the adapter
///
/// # Returns
///
/// A result containing the new memory adapter or an error
pub fn new_memory_adapter(memory_type: ComponentMemoryType) -> Result<Memory> {
    let adapter = SafeMemoryAdapter::new(memory_type.into())?;
    Ok(Memory::with_adapter(adapter))
}

// Extension trait for Memory to support adapter operations
trait MemoryAdapterExt {
    /// Create a new memory with an adapter
    fn with_adapter(adapter: Arc<dyn MemoryAdapter>) -> Self;
}

// Implementation of the extension trait for Memory
impl MemoryAdapterExt for Memory {
    fn with_adapter(adapter: Arc<dyn MemoryAdapter>) -> Self {
        // Use the underlying memory from the adapter, cloning it
        (*adapter.memory()).clone()
    }
}
