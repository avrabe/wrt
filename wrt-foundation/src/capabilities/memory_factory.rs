//! Memory Factory - Unified Memory Provider Creation
//!
//! This module provides a single, simple factory for creating memory providers
//! with capability verification. Replaces all previous factory patterns.

use crate::{
    budget_aware_provider::CrateId,
    memory_init::get_global_capability_context,
    safe_memory::NoStdProvider,
    Result,
};

use super::{
    context::MemoryCapabilityContext,
    MemoryOperation,
};

#[cfg(any(feature = "std", feature = "alloc"))]
use super::provider_bridge::CapabilityAwareProvider;

/// Unified memory provider factory
///
/// This factory provides a simple API for creating memory providers with
/// capability verification. It replaces SafeProviderFactory and CapabilityMemoryFactory.
pub struct MemoryFactory;

impl MemoryFactory {
    /// Create a memory provider with capability verification
    ///
    /// This is the primary method for creating providers. It uses the global
    /// capability context and returns a standard NoStdProvider.
    ///
    /// # Arguments
    /// * `crate_id` - The crate requesting the provider
    ///
    /// # Returns
    /// * `Ok(NoStdProvider<N>)` - A verified provider ready to use
    /// * `Err(Error)` - If capability verification fails
    pub fn create<const N: usize>(crate_id: CrateId) -> Result<NoStdProvider<N>> {
        let context = get_global_capability_context()?;
        Self::create_with_context(context, crate_id)
    }

    /// Create a memory provider with explicit context
    ///
    /// Use this when you have a specific capability context to use.
    ///
    /// # Arguments
    /// * `context` - The capability context to use for verification
    /// * `crate_id` - The crate requesting the provider
    ///
    /// # Returns
    /// * `Ok(NoStdProvider<N>)` - A verified provider ready to use
    /// * `Err(Error)` - If capability verification fails
    pub fn create_with_context<const N: usize>(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<NoStdProvider<N>> {
        // Verify allocation capability
        let operation = MemoryOperation::Allocate { size: N };
        context.verify_operation(crate_id, &operation)?;

        // Create the provider
        Ok(NoStdProvider::<N>::default())
    }

    /// Create a capability-aware provider wrapper
    ///
    /// Use this when you need a provider that implements the Provider trait
    /// with built-in capability verification for all operations.
    ///
    /// # Arguments
    /// * `crate_id` - The crate requesting the provider
    ///
    /// # Returns
    /// * `Ok(CapabilityAwareProvider<NoStdProvider<N>>)` - A wrapped provider
    /// * `Err(Error)` - If capability verification fails
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn create_wrapped<const N: usize>(
        crate_id: CrateId,
    ) -> Result<CapabilityAwareProvider<NoStdProvider<N>>> {
        let context = get_global_capability_context()?;
        Self::create_wrapped_with_context(context, crate_id)
    }

    /// Create a capability-aware provider wrapper with explicit context
    ///
    /// # Arguments
    /// * `context` - The capability context to use for verification
    /// * `crate_id` - The crate requesting the provider
    ///
    /// # Returns
    /// * `Ok(CapabilityAwareProvider<NoStdProvider<N>>)` - A wrapped provider
    /// * `Err(Error)` - If capability verification fails
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn create_wrapped_with_context<const N: usize>(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<CapabilityAwareProvider<NoStdProvider<N>>> {
        // Get the capability for this crate
        let capability = context.get_capability(crate_id)?;

        // Verify allocation operation
        let operation = MemoryOperation::Allocate { size: N };
        capability.verify_access(&operation)?;

        // Create the underlying provider
        let provider = NoStdProvider::<N>::default();

        // Wrap with capability verification
        Ok(CapabilityAwareProvider::new(
            provider,
            capability.clone_capability(),
            crate_id,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_factory_api() {
        // Test that the API compiles correctly
        let _test_fn = || -> Result<NoStdProvider<1024>> {
            MemoryFactory::create(CrateId::Foundation)
        };

        let _test_fn2 = |context: &MemoryCapabilityContext| -> Result<NoStdProvider<1024>> {
            MemoryFactory::create_with_context(context, CrateId::Foundation)
        };

        #[cfg(any(feature = "std", feature = "alloc"))]
        let _test_fn3 = || -> Result<CapabilityAwareProvider<NoStdProvider<1024>>> {
            MemoryFactory::create_wrapped(CrateId::Foundation)
        };
    }
}