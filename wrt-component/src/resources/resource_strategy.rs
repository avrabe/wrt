// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use wrt_error::Result;
use wrt_foundation::bounded::{BoundedVec, MAX_BUFFER_SIZE};

#[cfg(not(feature = "std"))]
use wrt_foundation::safe_memory::NoStdProvider;

use crate::resources::MemoryStrategy;
use wrt_foundation::resource::ResourceOperation;

/// Trait for resource access strategies
pub trait ResourceStrategy: Send + Sync {
    /// Get the type of memory strategy this implements
    fn memory_strategy_type(&self) -> MemoryStrategy;

    /// Process memory with this strategy
    #[cfg(feature = "std")]
    fn process_memory(&self, data: &[u8], operation: ResourceOperation) -> Result<Vec<u8>>;

    /// Process memory with this strategy (no_std version)
    #[cfg(not(feature = "std"))]
    fn process_memory(
        &self,
        data: &[u8],
        operation: ResourceOperation,
    ) -> core::result::Result<BoundedVec<u8, MAX_BUFFER_SIZE, NoStdProvider<65536>>, NoStdProvider<65536>>;

    /// Check if the strategy allows a certain operation
    fn allows_operation(&self, operation: ResourceOperation) -> bool {
        true // Default implementation allows all operations
    }

    /// Reset any internal state or buffers
    fn reset(&mut self) {
        // Default is no-op
    }
}
