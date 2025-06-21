//! Capability-driven Memory Provider Factory
//!
//! This module provides capability-driven memory allocation with explicit
//! capability verification and authorization for all memory operations.
//!
//! NOTE: This factory requires alloc feature for proper operation.

#[cfg(any(feature = "std", feature = "alloc"))]
use core::marker::PhantomData;

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;

#[cfg(any(feature = "std", feature = "alloc"))]
use crate::{
    budget_aware_provider::CrateId,
    codes,
    safe_memory::{NoStdProvider, Provider},
    Error, ErrorCategory, Result,
};

#[cfg(any(feature = "std", feature = "alloc"))]
use super::{context::MemoryCapabilityContext, MemoryCapability, MemoryOperation};

/// Capability-driven memory provider factory
///
/// This factory provides explicit capability injection for memory allocation,
/// ensuring all memory operations are properly verified and authorized.
/// 
/// **DEPRECATED**: Use `MemoryFactory` instead for a simpler API.
#[deprecated(note = "Use MemoryFactory for simpler memory provider creation")]
#[cfg(any(feature = "std", feature = "alloc"))]
pub struct CapabilityMemoryFactory {
    context: MemoryCapabilityContext,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl CapabilityMemoryFactory {
    /// Create a new factory with the given capability context
    pub fn new(context: MemoryCapabilityContext) -> Self {
        Self { context }
    }

    /// Create a capability-verified memory provider
    ///
    /// This method verifies that the requesting crate has the necessary
    /// capability to allocate the requested size before creating the provider.
    pub fn create_provider<const N: usize>(
        &self,
        crate_id: CrateId,
    ) -> Result<CapabilityGuardedProvider<N>> {
        // Verify the crate has capability to allocate this size
        let capability = self.context.get_capability(crate_id)?;

        // Verify allocation operation
        let operation = MemoryOperation::Allocate { size: N };
        capability.verify_access(&operation)?;

        // Create the underlying provider through the capability
        CapabilityGuardedProvider::new(capability.clone_capability())
    }

    /// Create a capability-aware wrapped provider (compatible with Provider trait)
    ///
    /// This method creates a CapabilityAwareProvider that wraps a NoStdProvider
    /// and can be used anywhere a Provider trait is expected.
    pub fn create_capability_aware_provider<const N: usize>(
        &self,
        crate_id: CrateId,
    ) -> Result<super::provider_bridge::CapabilityAwareProvider<NoStdProvider<N>>> {
        // Verify the crate has capability to allocate this size
        let capability = self.context.get_capability(crate_id)?;

        // Verify allocation operation
        let operation = MemoryOperation::Allocate { size: N };
        capability.verify_access(&operation)?;

        // Create the underlying NoStdProvider
        let provider = NoStdProvider::<N>::default();

        // Wrap with capability verification
        Ok(super::provider_bridge::CapabilityAwareProvider::new(
            provider,
            capability.clone_capability(),
            crate_id,
        ))
    }

    /// Create a provider with explicit capability verification
    pub fn create_verified_provider<const N: usize>(
        &self,
        crate_id: CrateId,
        required_verification_level: crate::verification::VerificationLevel,
    ) -> Result<CapabilityGuardedProvider<N>> {
        let capability = self.context.get_capability(crate_id)?;

        // Check if capability meets required verification level
        if capability.verification_level() < required_verification_level {
            return Err(Error::new(
                ErrorCategory::Security,
                codes::VERIFICATION_REQUIRED,
                "Capability verification level insufficient for request",
            ));
        }

        // Verify allocation operation
        let operation = MemoryOperation::Allocate { size: N };
        capability.verify_access(&operation)?;

        CapabilityGuardedProvider::new(capability.clone_capability())
    }

    /// Get the capability context for advanced operations
    pub fn context(&self) -> &MemoryCapabilityContext {
        &self.context
    }

    /// Register a new capability for a crate
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn register_capability(
        &mut self,
        crate_id: CrateId,
        capability: Box<dyn super::AnyMemoryCapability>,
    ) -> Result<()> {
        self.context.register_capability(crate_id, capability)
    }
}

/// A memory provider that is protected by capability verification
///
/// This provides capability-gated access to memory allocation.
#[cfg(any(feature = "std", feature = "alloc"))]
pub struct CapabilityGuardedProvider<const N: usize> {
    capability: Box<dyn super::AnyMemoryCapability>,
    provider: Option<NoStdProvider<N>>,
    _phantom: PhantomData<[u8; N]>,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<const N: usize> CapabilityGuardedProvider<N> {
    /// Create a new capability-guarded provider
    pub(crate) fn new(capability: Box<dyn super::AnyMemoryCapability>) -> Result<Self> {
        // Verify the capability allows allocation of this size
        let operation = MemoryOperation::Allocate { size: N };
        capability.verify_access(&operation)?;

        if capability.max_allocation_size() < N {
            return Err(Error::new(
                ErrorCategory::Security,
                codes::ACCESS_DENIED,
                "Provider size exceeds capability limit",
            ));
        }

        Ok(Self { capability, provider: None, _phantom: PhantomData })
    }

    /// Initialize the underlying provider (lazy initialization)
    fn ensure_provider(&mut self) -> Result<&mut NoStdProvider<N>> {
        if self.provider.is_none() {
            // Create the underlying provider
            #[allow(deprecated)]
            let mut provider = NoStdProvider::default();
            let _ = provider.resize(N);
            self.provider = Some(provider);
        }

        self.provider.as_mut().ok_or_else(|| Error::new(
            ErrorCategory::Memory,
            codes::MEMORY_ERROR,
            "Provider not initialized",
        ))
    }

    /// Read data with capability verification
    pub fn read_bytes(&mut self, offset: usize, len: usize) -> Result<&[u8]> {
        let operation = MemoryOperation::Read { offset, len };
        self.capability.verify_access(&operation)?;

        let provider = self.ensure_provider()?;

        // Delegate to the provider's slice implementation
        if offset + len > N {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::OUT_OF_BOUNDS,
                "Read operation exceeds provider bounds",
            ));
        }

        // Return a slice from the provider's buffer
        let slice = provider.borrow_slice(offset, len)?;
        slice.data()
    }

    /// Write data with capability verification
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        let operation = MemoryOperation::Write { offset, len: data.len() };
        self.capability.verify_access(&operation)?;

        let provider = self.ensure_provider()?;

        if offset + data.len() > N {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::OUT_OF_BOUNDS,
                "Write operation exceeds provider bounds",
            ));
        }

        // Write to the provider's buffer
        let mut slice = provider.get_slice_mut(offset, data.len())?;
        slice.data_mut()?.copy_from_slice(data);
        Ok(())
    }

    /// Get the size of this provider
    pub const fn size(&self) -> usize {
        N
    }

    /// Get the capability that guards this provider
    pub fn capability(&self) -> &dyn super::AnyMemoryCapability {
        self.capability.as_ref()
    }

    /// Get a read-only slice of the entire buffer
    pub fn as_slice(&mut self) -> Result<&[u8]> {
        self.read_bytes(0, N)
    }

    /// Get a mutable slice of the entire buffer
    pub fn as_slice_mut(&mut self) -> Result<crate::safe_memory::SliceMut<'_>> {
        let operation = MemoryOperation::Write { offset: 0, len: N };
        self.capability.verify_access(&operation)?;

        let provider = self.ensure_provider()?;
        provider.get_slice_mut(0, N)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<const N: usize> core::fmt::Debug for CapabilityGuardedProvider<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CapabilityGuardedProvider")
            .field("size", &N)
            .field("capability", &self.capability)
            .field("initialized", &self.provider.is_some())
            .finish()
    }
}

// Safety: CapabilityGuardedProvider can be sent between threads safely
// The capability system ensures thread-safe access
#[cfg(any(feature = "std", feature = "alloc"))]
unsafe impl<const N: usize> Send for CapabilityGuardedProvider<N> {}
#[cfg(any(feature = "std", feature = "alloc"))]
unsafe impl<const N: usize> Sync for CapabilityGuardedProvider<N> {}

/// Builder for creating capability-driven memory factories
///
/// **DEPRECATED**: Use `MemoryFactory` instead for direct provider creation.
#[deprecated(note = "Use MemoryFactory for simpler memory provider creation")]
#[cfg(any(feature = "std", feature = "alloc"))]
pub struct CapabilityFactoryBuilder {
    context: MemoryCapabilityContext,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl CapabilityFactoryBuilder {
    /// Create a new builder with default capability context
    pub fn new() -> Self {
        Self { context: MemoryCapabilityContext::default() }
    }

    /// Create builder with custom capability context
    pub fn with_context(context: MemoryCapabilityContext) -> Self {
        Self { context }
    }

    /// Register a dynamic capability for a crate
    pub fn with_dynamic_capability(
        mut self,
        crate_id: CrateId,
        max_allocation: usize,
    ) -> Result<Self> {
        self.context.register_dynamic_capability(crate_id, max_allocation)?;
        Ok(self)
    }

    /// Register a static capability for a crate
    pub fn with_static_capability<const N: usize>(mut self, crate_id: CrateId) -> Result<Self> {
        self.context.register_static_capability::<N>(crate_id)?;
        Ok(self)
    }

    /// Register a verified capability for a crate (ASIL-D)
    pub fn with_verified_capability<const N: usize>(
        mut self,
        crate_id: CrateId,
        proofs: super::verified::VerificationProofs,
    ) -> Result<Self> {
        self.context.register_verified_capability::<N>(crate_id, proofs)?;
        Ok(self)
    }

    /// Build the capability factory
    pub fn build(self) -> CapabilityMemoryFactory {
        CapabilityMemoryFactory::new(self.context)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl Default for CapabilityFactoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, any(feature = "std", feature = "alloc")))]
mod tests {
    use super::*;
    use crate::verification::VerificationLevel;

    #[test]
    fn test_capability_factory_creation() {
        let factory = CapabilityFactoryBuilder::new()
            .with_dynamic_capability(CrateId::Foundation, 1024)
            .unwrap()
            .build();

        assert!(factory.context().has_capability(CrateId::Foundation));
    }

    #[test]
    fn test_capability_guarded_provider() {
        let mut factory = CapabilityFactoryBuilder::new()
            .with_dynamic_capability(CrateId::Foundation, 4096)
            .unwrap()
            .build();

        let mut provider = factory.create_provider::<1024>(CrateId::Foundation).unwrap();
        assert_eq!(provider.size(), 1024);

        // Test read/write operations
        let test_data = b"Hello, World!";
        provider.write_bytes(0, test_data).unwrap();
        let read_data = provider.read_bytes(0, test_data.len()).unwrap();
        assert_eq!(read_data, test_data);
    }

    #[test]
    fn test_capability_verification_failure() {
        let factory = CapabilityFactoryBuilder::new()
            .with_dynamic_capability(CrateId::Foundation, 512) // Small limit
            .unwrap()
            .build();

        // Try to create provider larger than capability allows
        let result = factory.create_provider::<1024>(CrateId::Foundation);
        assert!(result.is_err());
    }

    #[test]
    fn test_verification_level_checking() {
        let factory = CapabilityFactoryBuilder::new()
            .with_dynamic_capability(CrateId::Foundation, 1024)
            .unwrap()
            .build();

        // Request higher verification than capability provides
        let result = factory
            .create_verified_provider::<512>(CrateId::Foundation, VerificationLevel::Redundant);
        assert!(result.is_err());
    }
}
