//! Compile-Time Memory Enforcement System
//!
//! This module provides compile-time enforcement mechanisms that make it
//! impossible to bypass the memory budget system.
//!
//! SW-REQ-ID: REQ_MEM_002 - Budget enforcement

use crate::safe_managed_alloc;
use crate::{
    budget_aware_provider::CrateId,
    memory_coordinator::CrateIdentifier,
    Error, Result,
};

use crate::{
    capabilities::{MemoryCapabilityContext, CapabilityGuardedProvider},
};

/// Sealed trait to prevent external implementation
mod sealed {
    pub trait Sealed {}
}

/// Trait that can only be implemented by approved types
pub trait MemoryManaged: sealed::Sealed {
    type Guard<const N: usize>;

    /// Create a managed allocation with capability context
    fn allocate<const N: usize>(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Self::Guard<N>>;
}

/// Only MemoryCapabilityContext can implement MemoryManaged
impl sealed::Sealed for MemoryCapabilityContext {}

impl MemoryManaged for MemoryCapabilityContext {
    type Guard<const N: usize> = CapabilityGuardedProvider<N>;

    fn allocate<const N: usize>(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Self::Guard<N>> {
        #[allow(deprecated)]
        context.create_provider::<N>(crate_id)
    }
}

/// Compile-time enforcement macro
// managed_alloc macro is now defined in macros.rs for better organization

/// Type-level enforcement using phantom types
pub struct EnforcedAllocation<const SIZE: usize, const CRATE: usize> {
    _phantom: core::marker::PhantomData<[u8; SIZE]>,
}

impl<const SIZE: usize, const CRATE: usize> EnforcedAllocation<SIZE, CRATE> {
    /// Create an enforced allocation
    pub const fn new() -> Self {
        Self { _phantom: core::marker::PhantomData }
    }

    /// Materialize the allocation (only possible through managed system)
    pub fn materialize(
        self, 
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<CapabilityGuardedProvider<SIZE>> {
        // Verify crate ID matches compile-time constant
        if crate_id.as_index() != CRATE {
            return Err(Error::new(
                crate::ErrorCategory::Safety,
                crate::codes::EXECUTION_ERROR,
                "Crate ID mismatch in enforced allocation",
            ));
        }

        #[allow(deprecated)]
        context.create_provider::<SIZE>(crate_id)
    }
}

/// Capability-based allocation token
pub struct AllocationToken<const SIZE: usize> {
    crate_id: CrateId,
    _phantom: core::marker::PhantomData<[u8; SIZE]>,
}

impl<const SIZE: usize> AllocationToken<SIZE> {
    /// Create a new allocation token (capability)
    pub const fn new(crate_id: CrateId) -> Self {
        Self { crate_id, _phantom: core::marker::PhantomData }
    }

    /// Use the token to allocate memory
    pub fn allocate(self, context: &MemoryCapabilityContext) -> Result<CapabilityGuardedProvider<SIZE>> {
        #[allow(deprecated)]
        context.create_provider::<SIZE>(self.crate_id)
    }
}

/// Memory region with compile-time bounds checking
pub struct MemoryRegion<const START: usize, const SIZE: usize> {
    _phantom: core::marker::PhantomData<[u8; SIZE]>,
}

impl<const START: usize, const SIZE: usize> MemoryRegion<START, SIZE> {
    /// Create a memory region with compile-time bounds
    pub const fn new() -> Self {
        // Basic validation - more complex checks moved to where type is instantiated
        assert!(SIZE > 0);

        Self { _phantom: core::marker::PhantomData }
    }

    /// Get the size of this region
    pub const fn size(&self) -> usize {
        SIZE
    }

    /// Get the start offset
    pub const fn start(&self) -> usize {
        START
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_managed_allocation() {
        crate::memory_init::MemoryInitializer::initialize().unwrap();

        let guard = safe_managed_alloc!(1024, CrateId::Component).unwrap();
        assert_eq!(guard.size(), 1024);
    }

    #[test]
    fn test_token_allocation() {
        crate::memory_init::MemoryInitializer::initialize().unwrap();
        
        // Create a capability context for testing
        let mut context = MemoryCapabilityContext::default();
        context.register_dynamic_capability(CrateId::Foundation, 1024).unwrap();

        let token = AllocationToken::<512>::new(CrateId::Foundation);
        let guard = token.allocate(&context).unwrap();
        assert_eq!(guard.size(), 512);
    }

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion::<0, 1024>::new();
        assert_eq!(region.size(), 1024);
        assert_eq!(region.start(), 0);
    }
}
