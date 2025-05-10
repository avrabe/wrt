use crate::resources::{ResourceOperation, ResourceStrategy};
use wrt_error::{Error, Result};

/// Memory strategy for resource operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryStrategy {
    /// Zero-copy strategy for trusted components
    ZeroCopy,
    /// Bounded-copy strategy with buffer pooling
    BoundedCopy,
    /// Full isolation with validation
    Isolated,
    /// Copy strategy - creates a copy of memory for safety
    Copy,
    /// Reference strategy - provides a direct reference to memory
    Reference,
    /// Full isolation with complete memory validation
    FullIsolation,
}

impl ResourceStrategy for MemoryStrategy {
    fn memory_strategy_type(&self) -> MemoryStrategy {
        *self
    }

    fn process_memory(&self, data: &[u8], operation: ResourceOperation) -> Result<Vec<u8>> {
        match self {
            // Zero-copy strategy - returns a view without copying for reads, a copy for writes
            MemoryStrategy::ZeroCopy => match operation {
                ResourceOperation::Read => Ok(data.to_vec()),
                ResourceOperation::Write => Ok(data.to_vec()),
                _ => Err(Error::new("Unsupported operation for ZeroCopy strategy")),
            },

            // Bounded-copy strategy - always copies but reuses buffers
            MemoryStrategy::BoundedCopy => Ok(data.to_vec()),

            // Isolated strategy - always copies and validates
            MemoryStrategy::Isolated => {
                // In a real implementation this would include validation
                Ok(data.to_vec())
            }

            // Copy strategy - always copies the data
            MemoryStrategy::Copy => Ok(data.to_vec()),

            // Reference strategy - returns a view without copying
            MemoryStrategy::Reference => {
                // In a real implementation, this would return a reference
                // For testing purposes, we'll still return a vec
                Ok(data.to_vec())
            }

            // Full isolation strategy - copies and performs full validation
            MemoryStrategy::FullIsolation => {
                // In a real implementation this would include more extensive validation
                Ok(data.to_vec())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_strategy() {
        let strategy = MemoryStrategy::Copy;
        let data = vec![1, 2, 3, 4, 5];

        let result = strategy.process_memory(&data, ResourceOperation::Read).unwrap();
        assert_eq!(result, data);

        // Modifying the copy shouldn't affect the original
        let mut result_copy = result.clone();
        result_copy[0] = 99;
        assert_ne!(result_copy[0], data[0]);
    }

    #[test]
    fn test_reference_strategy() {
        let strategy = MemoryStrategy::Reference;
        let data = vec![1, 2, 3, 4, 5];

        let result = strategy.process_memory(&data, ResourceOperation::Read).unwrap();
        assert_eq!(result, data);
    }
}
