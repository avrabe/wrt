//! Simplified Type System for WRT Runtime - COMPILATION FIX
//!
//! This module provides a simplified unified type system to resolve compilation
//! errors. It focuses on concrete types rather than generic type aliases.

use wrt_foundation::{
    safe_memory::NoStdProvider,
    bounded::{BoundedVec, BoundedString},
    traits::{Checksummable, ToBytes, FromBytes},
    prelude::*,
};

// =============================================================================
// CONCRETE RUNTIME TYPES
// =============================================================================

/// Default memory provider for runtime operations
pub type RuntimeProvider = NoStdProvider<1048576>; // 1MB

/// Vector for local variables in function execution
pub type LocalsVec = BoundedVec<Value, 64, RuntimeProvider>;

/// Stack for WebAssembly values during execution
pub type ValueStackVec = BoundedVec<Value, 1024, RuntimeProvider>;

/// Buffer for linear memory content  
pub type MemoryBuffer = BoundedVec<u8, 65536, RuntimeProvider>;

/// String for runtime identifiers and names
pub type RuntimeString = BoundedString<256, RuntimeProvider>;

/// String for component and module names
pub type ComponentName = BoundedString<64, RuntimeProvider>;

/// Vector for function parameters
pub type ParameterVec = BoundedVec<Value, 16, RuntimeProvider>;

/// Vector for function results
pub type ResultVec = BoundedVec<Value, 16, RuntimeProvider>;

// =============================================================================
// PLATFORM CONFIGURATION
// =============================================================================

/// Platform capacity configuration
#[derive(Debug, Clone, Copy)]
pub struct PlatformCapacities {
    pub small_capacity: usize,
    pub medium_capacity: usize, 
    pub large_capacity: usize,
    pub memory_provider_size: usize,
}

impl PlatformCapacities {
    pub const fn default() -> Self {
        Self {
            small_capacity: 64,
            medium_capacity: 1024,
            large_capacity: 65536,
            memory_provider_size: 1048576,
        }
    }
    
    pub const fn embedded() -> Self {
        Self {
            small_capacity: 16,
            medium_capacity: 256,
            large_capacity: 8192,
            memory_provider_size: 32768,
        }
    }
}

// =============================================================================
// COMPATIBILITY LAYER
// =============================================================================

/// Compatibility types for gradual migration
pub mod compat {
    use super::*;
    
    /// Small vector for limited collections (T must implement all required traits)
    pub type SmallVec<T> = BoundedVec<T, 64, RuntimeProvider>;
    
    /// Medium vector for standard collections (T must implement all required traits)
    pub type MediumVec<T> = BoundedVec<T, 1024, RuntimeProvider>;
    
    /// Large vector for big collections (T must implement all required traits)
    pub type LargeVec<T> = BoundedVec<T, 65536, RuntimeProvider>;
        
    /// Compatibility string type
    pub type String = BoundedString<256, RuntimeProvider>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_platform_capacities() {
        let default_caps = PlatformCapacities::default();
        assert_eq!(default_caps.small_capacity, 64);
        
        let embedded_caps = PlatformCapacities::embedded();
        assert!(embedded_caps.small_capacity < default_caps.small_capacity);
    }
}