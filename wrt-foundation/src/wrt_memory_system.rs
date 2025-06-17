//! WRT-Specific Memory System Implementation
//!
//! This module provides the WRT-specific implementation of the generic memory
//! system, using the generic components to create a complete solution.
//!
//! SW-REQ-ID: REQ_MEM_001 - Memory bounds checking
//! SW-REQ-ID: REQ_MEM_002 - Budget enforcement
//! SW-REQ-ID: REQ_MEM_003 - Automatic cleanup

use crate::{
    budget_aware_provider::CrateId,
    codes,
    generic_memory_guard::{GenericMemoryGuard, ManagedMemoryProvider, MemoryCoordinator},
    generic_provider_factory::{GenericBudgetAwareFactory, ProviderFactory},
    memory_coordinator::{AllocationId, CrateIdentifier, GenericMemoryCoordinator},
    safe_memory::NoStdProvider,
    Error, ErrorCategory, Result,
};

/// Maximum number of crates in the WRT system
pub const WRT_MAX_CRATES: usize = 32;

/// WRT-specific memory coordinator
pub type WrtMemoryCoordinator = GenericMemoryCoordinator<CrateId, WRT_MAX_CRATES>;

/// WRT-specific memory guard for NoStdProvider
pub type WrtMemoryGuard<const N: usize> =
    GenericMemoryGuard<NoStdProvider<N>, WrtMemoryCoordinator, CrateId>;

/// Global WRT memory coordinator instance (DEPRECATED - use capability injection)
#[deprecated(since = "0.3.0", note = "Use MemoryCapabilityContext for capability-driven design")]
pub static WRT_MEMORY_COORDINATOR: WrtMemoryCoordinator = WrtMemoryCoordinator::new();

// CrateIdentifier implementation is in budget_aware_provider.rs

// Implement ManagedMemoryProvider for NoStdProvider
impl<const N: usize> ManagedMemoryProvider for NoStdProvider<N> {
    fn allocation_size(&self) -> usize {
        N
    }
}

// Implement MemoryCoordinator trait for WrtMemoryCoordinator
impl MemoryCoordinator<CrateId> for WrtMemoryCoordinator {
    type AllocationId = AllocationId;

    fn register_allocation(&self, crate_id: CrateId, size: usize) -> Result<Self::AllocationId> {
        GenericMemoryCoordinator::register_allocation(self, crate_id, size)
    }

    fn return_allocation(
        &self,
        crate_id: CrateId,
        id: Self::AllocationId,
        size: usize,
    ) -> Result<()> {
        GenericMemoryCoordinator::return_allocation(self, crate_id, id, size)
    }
}

/// Factory for creating NoStdProviders
pub struct NoStdProviderFactory;

impl NoStdProviderFactory {
    /// Create a provider with deprecated constructor (temporary)
    #[allow(deprecated)]
    fn create_provider_internal<const N: usize>() -> NoStdProvider<N> {
        let mut provider = NoStdProvider::new();
        // Initialize provider to full capacity
        let _ = provider.resize(N);
        provider
    }
}

/// Generic implementation for any size
pub struct SizedNoStdProviderFactory<const N: usize>;

impl<const N: usize> ProviderFactory for SizedNoStdProviderFactory<N> {
    type Provider = NoStdProvider<N>;

    fn create_provider(&self, size: usize) -> Result<Self::Provider> {
        if size > N {
            return Err(Error::new(
                ErrorCategory::Capacity,
                codes::CAPACITY_EXCEEDED,
                "Requested size exceeds provider capacity",
            ));
        }

        #[allow(deprecated)]
        Ok(NoStdProviderFactory::create_provider_internal::<N>())
    }
}

/// WRT-specific budget-aware factory
pub type WrtBudgetAwareFactory<const N: usize> =
    GenericBudgetAwareFactory<SizedNoStdProviderFactory<N>, WrtMemoryCoordinator, CrateId>;

/// Main factory for WRT memory providers (DEPRECATED - use MemoryCapabilityContext)
#[deprecated(
    since = "0.3.0",
    note = "Use MemoryCapabilityContext::create_provider() for capability-driven design"
)]
pub struct WrtProviderFactory;

impl WrtProviderFactory {
    /// Create a budget-aware provider with specified size (DEPRECATED)
    #[deprecated(
        since = "0.3.0",
        note = "Use MemoryCapabilityContext::create_provider() for capability-driven design"
    )]
    #[allow(deprecated)]
    pub fn create_provider<const N: usize>(crate_id: CrateId) -> Result<WrtMemoryGuard<N>> {
        let factory = SizedNoStdProviderFactory::<N>;
        let budget_factory = GenericBudgetAwareFactory::new(factory, &WRT_MEMORY_COORDINATOR);
        budget_factory.create_provider(N, crate_id)
    }

    /// Create a typed provider (for macro support)
    pub fn create_typed_provider<T>(crate_id: CrateId) -> Result<WrtMemoryGuard<4096>>
    where
        T: crate::safe_memory::Provider,
    {
        // For typed providers, we use a standard size and let the type system handle the rest
        Self::create_provider::<4096>(crate_id)
    }

    /// Initialize the WRT memory system with default budgets
    pub fn initialize_default() -> Result<()> {
        use crate::budget_verification::CRATE_BUDGETS;

        let budgets = [
            (CrateId::Foundation, CRATE_BUDGETS[0]),
            (CrateId::Decoder, CRATE_BUDGETS[1]),
            (CrateId::Runtime, CRATE_BUDGETS[2]),
            (CrateId::Component, CRATE_BUDGETS[3]),
            (CrateId::Host, CRATE_BUDGETS[4]),
            (CrateId::Debug, CRATE_BUDGETS[5]),
            (CrateId::Platform, CRATE_BUDGETS[6]),
            (CrateId::Instructions, CRATE_BUDGETS[7]),
            (CrateId::Format, CRATE_BUDGETS[8]),
            (CrateId::Intercept, CRATE_BUDGETS[9]),
            (CrateId::Sync, CRATE_BUDGETS[10]),
            (CrateId::Math, CRATE_BUDGETS[11]),
            (CrateId::Logging, CRATE_BUDGETS[12]),
            (CrateId::Panic, CRATE_BUDGETS[13]),
            (CrateId::TestRegistry, CRATE_BUDGETS[14]),
            (CrateId::VerificationTool, CRATE_BUDGETS[15]),
            (CrateId::Unknown, CRATE_BUDGETS[16]),
            (CrateId::Wasi, CRATE_BUDGETS[17]),
            (CrateId::WasiComponents, CRATE_BUDGETS[18]),
        ];

        let total = CRATE_BUDGETS.iter().sum();
        WRT_MEMORY_COORDINATOR.initialize(budgets.iter().copied(), total)
    }
}

/// Convenience macro for creating WRT providers
#[macro_export]
macro_rules! wrt_provider {
    ($size:expr, $crate_id:expr) => {
        $crate::wrt_memory_system::WrtProviderFactory::create_provider::<$size>($crate_id)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrt_memory_system() {
        // Initialize the system
        WrtProviderFactory::initialize_default().unwrap();

        // Create a provider
        let guard = wrt_provider!(1024, CrateId::Component).unwrap();
        assert_eq!(guard.size(), 1024);

        // Check allocation
        let allocated = WRT_MEMORY_COORDINATOR.get_crate_allocation(CrateId::Component);
        assert_eq!(allocated, 1024);

        // Guard drops here, should return allocation
    }
}
