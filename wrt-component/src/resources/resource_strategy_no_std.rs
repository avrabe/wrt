// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use wrt_error::{codes, Error, ErrorCategory, Result};
use wrt_foundation::bounded::{BoundedVec, MAX_BUFFER_SIZE};
use wrt_foundation::safe_memory::NoStdProvider;

use super::MemoryStrategy;
use wrt_foundation::resource::ResourceOperation;
use super::resource_strategy::ResourceStrategy;

/// No-std version of ResourceStrategy implementation
/// This struct provides resource access strategies for no_std environments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceStrategyNoStd {
    /// The memory strategy to use for this resource
    strategy: MemoryStrategy,
}

impl ResourceStrategyNoStd {
    /// Create a new ResourceStrategyNoStd with the given memory strategy
    pub fn new(strategy: MemoryStrategy) -> Self {
        Self { strategy }
    }
}

impl ResourceStrategy for ResourceStrategyNoStd {
    fn memory_strategy_type(&self) -> MemoryStrategy {
        self.strategy
    }

    fn process_memory(
        &self,
        data: &[u8],
        operation: ResourceOperation,
    ) -> core::result::Result<BoundedVec<u8, MAX_BUFFER_SIZE, NoStdProvider<65536>>, NoStdProvider<65536>> {
        match self.strategy {
            // Zero-copy strategy - returns a view without copying for reads, a copy for writes
            MemoryStrategy::ZeroCopy => match operation {
                ResourceOperation::Read => {
                    let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to create bounded vec for zero-copyMissing message")
                    })?;

                    for &byte in data {
                        result.push(byte).map_err(|e| {
                            Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                        })?;
                    }
                    Ok(result)
                }
                ResourceOperation::Write => {
                    let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to create bounded vec for zero-copyMissing message")
                    })?;

                    for &byte in data {
                        result.push(byte).map_err(|e| {
                            Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                        })?;
                    }
                    Ok(result)
                }
                _ => Err(Error::not_supported("Error occurred"Unsupported operation for ZeroCopy strategyMissing messageMissing messageMissing message")),
            },

            // Bounded-copy strategy - always copies but reuses buffers
            MemoryStrategy::BoundedCopy => {
                let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                    Error::memory_error("Error occurred"Failed to create bounded vec for bounded-copyMissing message")
                })?;

                for &byte in data {
                    result.push(byte).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                    })?;
                }
                Ok(result)
            }

            // Isolated strategy - always copies and validates
            MemoryStrategy::Isolated => {
                let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                    Error::memory_error("Error occurred"Failed to create bounded vec for isolated strategyMissing message")
                })?;

                // In a real implementation this would include validation
                for &byte in data {
                    result.push(byte).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                    })?;
                }
                Ok(result)
            }

            // Copy strategy - always copies the data
            MemoryStrategy::Copy => {
                let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                    Error::memory_error("Error occurred"Failed to create bounded vec for copy strategyMissing message")
                })?;

                for &byte in data {
                    result.push(byte).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                    })?;
                }
                Ok(result)
            }

            // Reference strategy - returns a view without copying
            MemoryStrategy::Reference => {
                let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                    Error::memory_error("Error occurred"Failed to create bounded vec for reference strategyMissing message")
                })?;

                // In a real implementation, this would return a reference
                // For now, we'll still return a BoundedVec
                for &byte in data {
                    result.push(byte).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                    })?;
                }
                Ok(result)
            }

            // Full isolation strategy - copies and performs full validation
            MemoryStrategy::FullIsolation => {
                let mut result = BoundedVec::with_capacity(data.len()).map_err(|e| {
                    Error::memory_error("Error occurred"Failed to create bounded vec for full isolation strategyMissing message")
                })?;

                // In a real implementation this would include more extensive validation
                for &byte in data {
                    result.push(byte).map_err(|e| {
                        Error::memory_error("Error occurred"Failed to push to bounded vecMissing message")
                    })?;
                }
                Ok(result)
            }
        }
    }

    // We're using the default implementation for allows_operation
    // fn allows_operation(&self, operation: ResourceOperation) -> bool {
    //     true // Default implementation allows all operations
    // }

    // We're using the default implementation for reset
    // fn reset(&mut self) {
    //     // Default is no-op
    // }
}

// Implementation-specific constants
/// Maximum buffer size for bounded vectors in no_std environments
pub const MAX_RESOURCE_BUFFER_SIZE: usize = wrt_foundation::bounded::MAX_BUFFER_SIZE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_strategy_no_std_copy() {
        let strategy = ResourceStrategyNoStd::new(MemoryStrategy::Copy);
        let data = &[1, 2, 3, 4, 5];

        let result = strategy.process_memory(data, ResourceOperation::Read).unwrap();
        assert_eq!(result.as_slice(), data);

        // Modifying the copy shouldn't affect the original
        let mut result_clone = result.clone();
        if let Ok(()) = result_clone.set(0, 99) {
            assert_ne!(result_clone.as_slice()[0], data[0]);
        }
    }

    #[test]
    fn test_resource_strategy_no_std_reference() {
        let strategy = ResourceStrategyNoStd::new(MemoryStrategy::Reference);
        let data = &[1, 2, 3, 4, 5];

        let result = strategy.process_memory(data, ResourceOperation::Read).unwrap();
        assert_eq!(result.as_slice(), data);
    }

    #[test]
    fn test_memory_strategy_type() {
        let strategy = ResourceStrategyNoStd::new(MemoryStrategy::ZeroCopy);
        assert_eq!(strategy.memory_strategy_type(), MemoryStrategy::ZeroCopy);

        let strategy = ResourceStrategyNoStd::new(MemoryStrategy::BoundedCopy);
        assert_eq!(strategy.memory_strategy_type(), MemoryStrategy::BoundedCopy);
    }

    #[test]
    fn test_zero_copy_strategy_invalid_operation() {
        let strategy = ResourceStrategyNoStd::new(MemoryStrategy::ZeroCopy);
        let data = &[1, 2, 3, 4, 5];

        // ZeroCopy only supports Read and Write
        let result = strategy.process_memory(data, ResourceOperation::Execute);
        assert!(result.is_err();
    }

    #[test]
    fn test_capacity_limits() {
        let strategy = ResourceStrategyNoStd::new(MemoryStrategy::Copy);

        // Create data that exceeds MAX_BUFFER_SIZE
        let large_data = vec![0u8; MAX_BUFFER_SIZE + 1];

        // This should fail because the data is too large for BoundedVec
        let result = strategy.process_memory(&large_data, ResourceOperation::Read);
        assert!(result.is_err();
    }
}
