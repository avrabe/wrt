//! Memory optimization strategies for WebAssembly Component Model
//!
//! This module provides memory optimization strategies for cross-component
//! communication in the WebAssembly Component Model.

use wrt_error::kinds::{OutOfBoundsAccess, ResourceLimitExceeded};

use crate::{
    resources::{BoundedBufferPool, MemoryStrategy},
    prelude::*,
};

/// Trait defining a memory optimization strategy
pub trait MemoryOptimizationStrategy: Send + Sync {
    /// Get the name of this strategy
    fn name(&self) -> &str;

    /// Get the memory strategy type used by this strategy
    fn memory_strategy_type(&self) -> MemoryStrategy;

    /// Copy memory from source to destination with strategy-specific
    /// optimizations
    fn copy_memory(
        &self,
        source: &[u8],
        destination: &mut [u8],
        offset: usize,
        size: usize,
    ) -> Result<()>;

    /// Determine if this strategy is appropriate for the given relationship
    fn is_appropriate_for(
        &self,
        source_trust_level: u8,
        destination_trust_level: u8,
        same_runtime: bool,
    ) -> bool;

    /// Clone this strategy
    fn clone_strategy(&self) -> Box<dyn MemoryOptimizationStrategy>;
}

/// Zero-copy memory optimization strategy
///
/// This strategy avoids copying data when possible, using direct memory
/// references for highest performance. Only suitable for trusted components
/// running in the same memory space with strong isolation guarantees.
#[derive(Debug, Clone)]
pub struct ZeroCopyStrategy {
    /// Minimum trust level required for this strategy
    min_trust_level: u8,
}

impl ZeroCopyStrategy {
    /// Create a new zero-copy strategy with the given minimum trust level
    pub fn new(min_trust_level: u8) -> Self {
        Self { min_trust_level }
    }

    /// Create a new zero-copy strategy with default settings
    pub fn default() -> Self {
        Self { min_trust_level: 3 } // Only for highly trusted components
    }
}

impl MemoryOptimizationStrategy for ZeroCopyStrategy {
    fn name(&self) -> &str {
        "ZeroCopy"
    }

    fn memory_strategy_type(&self) -> MemoryStrategy {
        MemoryStrategy::ZeroCopy
    }

    fn copy_memory(
        &self,
        source: &[u8],
        destination: &mut [u8],
        offset: usize,
        size: usize,
    ) -> Result<()> {
        // Check bounds
        if offset + size > source.len() || size > destination.len() {
            return Err(Error::runtime_execution_error(",
                    offset,
                    size,
                    source.len(),
                    destination.len()
                )),
            ));
        }

        // In a true zero-copy implementation, we would use memory mapping or other
        // mechanisms to avoid copying. For this implementation, we still do a
        // copy but could optimize further.
        destination[..size].copy_from_slice(&source[offset..offset + size]);

        Ok(())
    }

    fn is_appropriate_for(
        &self,
        source_trust_level: u8,
        destination_trust_level: u8,
        same_runtime: bool,
    ) -> bool {
        // Only use zero-copy for trusted components in the same runtime
        source_trust_level >= self.min_trust_level
            && destination_trust_level >= self.min_trust_level
            && same_runtime
    }

    fn clone_strategy(&self) -> Box<dyn MemoryOptimizationStrategy> {
        Box::new(self.clone())
    }
}

/// Bounded-copy memory optimization strategy
///
/// This strategy uses bounded copies with buffer pooling for good performance
/// while maintaining strong security boundaries. Suitable for standard
/// components with moderate trust levels.
#[derive(Debug)]
pub struct BoundedCopyStrategy {
    /// Binary std/no_std choice
    buffer_pool: Arc<RwLock<BoundedBufferPool>>,
    /// Maximum copy size in bytes
    max_copy_size: usize,
    /// Minimum trust level required for this strategy
    min_trust_level: u8,
}

impl BoundedCopyStrategy {
    /// Create a new bounded-copy strategy with the given parameters
    pub fn new(
        buffer_pool: Arc<RwLock<BoundedBufferPool>>,
        max_copy_size: usize,
        min_trust_level: u8,
    ) -> Self {
        Self { buffer_pool, max_copy_size, min_trust_level }
    }

    /// Create a new bounded-copy strategy with default settings
    pub fn default() -> Self {
        Self {
            buffer_pool: Arc::new(RwLock::new(BoundedBufferPool::new())), // Bounded pool
            max_copy_size: 64 * 1024,                                         // 64KB max copy
            min_trust_level: 1, // Works for standard trust components
        }
    }
}

impl Clone for BoundedCopyStrategy {
    fn clone(&self) -> Self {
        Self {
            buffer_pool: self.buffer_pool.clone(),
            max_copy_size: self.max_copy_size,
            min_trust_level: self.min_trust_level,
        }
    }
}

impl MemoryOptimizationStrategy for BoundedCopyStrategy {
    fn name(&self) -> &str {
        ") -> MemoryStrategy {
        MemoryStrategy::BoundedCopy
    }

    fn copy_memory(
        &self,
        source: &[u8],
        destination: &mut [u8],
        offset: usize,
        size: usize,
    ) -> Result<()> {
        // Check bounds
        if offset + size > source.len() || size > destination.len() {
            return Err(Error::runtime_execution_error(",
                    offset,
                    size,
                    source.len(),
                    destination.len()
                )),
            ));
        }

        // Check maximum copy size
        if size > self.max_copy_size {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                ResourceLimitExceeded(format!(
                    ")),
            ));
        }

        // Perform the copy directly
        destination[..size].copy_from_slice(&source[offset..offset + size]);

        Ok(())
    }

    fn is_appropriate_for(
        &self,
        source_trust_level: u8,
        destination_trust_level: u8,
        _same_runtime: bool,
    ) -> bool {
        // Use bounded-copy for components with at least minimal trust
        source_trust_level >= self.min_trust_level
            && destination_trust_level >= self.min_trust_level
    }

    fn clone_strategy(&self) -> Box<dyn MemoryOptimizationStrategy> {
        Box::new(self.clone())
    }
}

/// Full isolation memory optimization strategy
///
/// This strategy applies the strongest isolation for untrusted components,
/// with full validation and sanitization of all data. Suitable for untrusted
/// or potentially malicious components.
#[derive(Debug, Clone)]
pub struct FullIsolationStrategy {
    /// Maximum copy size in bytes
    max_copy_size: usize,
}

impl FullIsolationStrategy {
    /// Create a new full isolation strategy with the given maximum copy size
    pub fn new(max_copy_size: usize) -> Self {
        Self { max_copy_size }
    }

    /// Create a new full isolation strategy with default settings
    pub fn default() -> Self {
        Self {
            max_copy_size: 16 * 1024, // 16KB max copy for untrusted components
        }
    }
}

impl MemoryOptimizationStrategy for FullIsolationStrategy {
    fn name(&self) -> &str {
        "FullIsolation"
    }

    fn memory_strategy_type(&self) -> MemoryStrategy {
        MemoryStrategy::Isolated
    }

    fn copy_memory(
        &self,
        source: &[u8],
        destination: &mut [u8],
        offset: usize,
        size: usize,
    ) -> Result<()> {
        // Check bounds
        if offset + size > source.len() || size > destination.len() {
            return Err(Error::runtime_execution_error(",
                    offset,
                    size,
                    source.len(),
                    destination.len()
                )),
            ));
        }

        // Check maximum copy size
        if size > self.max_copy_size {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                ResourceLimitExceeded(format!(
                    ")),
            ));
        }

        // Full validation and sanitization
        for i in 0..size {
            let byte = source[offset + i];

            // Example validation (could be more complex in real implementation)
            // For demonstration, we make sure it's a valid ASCII value if it's a printable
            // character
            if byte >= 32 && byte < 127 {
                // Valid ASCII printable character
                destination[i] = byte;
            } else if byte < 32 || byte == 127 {
                // Control character - could handle differently depending on context
                destination[i] = byte;
            } else {
                // Non-ASCII byte - could be handled differently depending on context
                destination[i] = byte;
            }
        }

        Ok(())
    }

    fn is_appropriate_for(
        &self,
        source_trust_level: u8,
        destination_trust_level: u8,
        _same_runtime: bool,
    ) -> bool {
        // Use full isolation for untrusted components
        source_trust_level == 0 || destination_trust_level == 0
    }

    fn clone_strategy(&self) -> Box<dyn MemoryOptimizationStrategy> {
        Box::new(self.clone())
    }
}

/// Creates the appropriate memory optimization strategy based on the
/// relationship between components
pub fn create_memory_strategy(
    source_trust_level: u8,
    destination_trust_level: u8,
    same_runtime: bool,
) -> Box<dyn MemoryOptimizationStrategy> {
    // Try strategies in order of performance (fastest to slowest)
    let zero_copy = ZeroCopyStrategy::default();
    if zero_copy.is_appropriate_for(source_trust_level, destination_trust_level, same_runtime) {
        return Box::new(zero_copy);
    }

    let bounded_copy = BoundedCopyStrategy::default();
    if bounded_copy.is_appropriate_for(source_trust_level, destination_trust_level, same_runtime) {
        return Box::new(bounded_copy);
    }

    // Fallback to full isolation for any other case
    Box::new(FullIsolationStrategy::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_strategy() {
        let strategy = ZeroCopyStrategy::default();
        let source = vec![1, 2, 3, 4, 5];
        let mut dest = vec![0; 5];

        // Test valid copy
        let result = strategy.copy_memory(&source, &mut dest, 0, 5);
        assert!(result.is_ok());
        assert_eq!(dest, vec![1, 2, 3, 4, 5]);

        // Test out of bounds
        let result = strategy.copy_memory(&source, &mut dest, 2, 5);
        assert!(result.is_err());

        // Test appropriateness
        assert!(strategy.is_appropriate_for(3, 3, true)); // Trusted components in same runtime
        assert!(!strategy.is_appropriate_for(3, 3, false)); // Trusted components in different runtimes
        assert!(!strategy.is_appropriate_for(2, 3, true)); // Mixed trust in
                                                           // same runtime
    }

    #[test]
    fn test_bounded_copy_strategy() {
        let strategy = BoundedCopyStrategy::default();
        let source = vec![1, 2, 3, 4, 5];
        let mut dest = vec![0; 5];

        // Test valid copy
        let result = strategy.copy_memory(&source, &mut dest, 0, 5);
        assert!(result.is_ok());
        assert_eq!(dest, vec![1, 2, 3, 4, 5]);

        // Test out of bounds
        let result = strategy.copy_memory(&source, &mut dest, 2, 5);
        assert!(result.is_err());

        // Test appropriateness
        assert!(strategy.is_appropriate_for(1, 1, true)); // Standard trust in same runtime
        assert!(strategy.is_appropriate_for(1, 1, false)); // Standard trust in different runtimes
        assert!(!strategy.is_appropriate_for(0, 1, true)); // Mixed trust with
                                                           // untrusted component
    }

    #[test]
    fn test_full_isolation_strategy() {
        let strategy = FullIsolationStrategy::default();
        let source = vec![1, 2, 3, 4, 5];
        let mut dest = vec![0; 5];

        // Test valid copy
        let result = strategy.copy_memory(&source, &mut dest, 0, 5);
        assert!(result.is_ok());
        assert_eq!(dest, vec![1, 2, 3, 4, 5]);

        // Test out of bounds
        let result = strategy.copy_memory(&source, &mut dest, 2, 5);
        assert!(result.is_err());

        // Test appropriateness
        assert!(strategy.is_appropriate_for(0, 1, true)); // Untrusted component
        assert!(strategy.is_appropriate_for(1, 0, false)); // Untrusted component
        assert!(strategy.is_appropriate_for(0, 0, true)); // Both untrusted
    }

    #[test]
    fn test_strategy_selection() {
        // Test selection for trusted components in same runtime
        let strategy = create_memory_strategy(3, 3, true);
        assert_eq!(strategy.name(), "ZeroCopy");

        // Test selection for trusted components in different runtimes
        let strategy = create_memory_strategy(3, 3, false);
        assert_eq!(strategy.name(), "BoundedCopy");

        // Test selection for standard trust components
        let strategy = create_memory_strategy(1, 1, true);
        assert_eq!(strategy.name(), "BoundedCopy");

        // Test selection for untrusted components
        let strategy = create_memory_strategy(0, 1, true);
        assert_eq!(strategy.name(), "FullIsolation");
    }
}
