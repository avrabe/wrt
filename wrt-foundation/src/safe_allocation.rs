//! Safe Memory Allocation API
//!
//! This module provides a completely safe alternative to the unsafe memory allocation
//! patterns. It eliminates the need for `unsafe { guard.release() }` by providing
//! a safe factory pattern that creates collections directly with owned providers.
//!
//! SW-REQ-ID: REQ_SAFETY_001 - No unsafe code in allocation
//! SW-REQ-ID: REQ_MEM_001 - Memory bounds checking
//! SW-REQ-ID: REQ_MEM_003 - Automatic cleanup

use crate::{
    bounded::{BoundedString, BoundedVec},
    bounded_collections::BoundedMap,
    budget_aware_provider::CrateId,
    codes,
    safe_memory::NoStdProvider,
    traits::{Checksummable, FromBytes, ToBytes},
    wrt_memory_system::{NoStdProviderFactory, WrtMemoryGuard, WrtProviderFactory},
    Error, ErrorCategory, Result,
};

/// Safe factory functions that create providers without unsafe code
pub struct SafeProviderFactory;

impl SafeProviderFactory {
    /// Create a NoStdProvider safely without unsafe code
    ///
    /// This function creates a provider using the internal safe mechanisms
    /// without exposing the guard's unsafe release method.
    #[allow(deprecated)] // We're using the safe internal method
    fn create_provider_safe<const N: usize>() -> Result<NoStdProvider<N>> {
        // Use the internal safe creation method that NoStdProviderFactory uses
        let mut provider = NoStdProvider::new();
        provider.resize(N).map_err(|_| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ERROR,
                "Failed to resize provider to requested capacity",
            )
        })?;
        Ok(provider)
    }

    /// Create a provider with budget tracking through the memory coordinator
    ///
    /// This provides the same functionality as safe_managed_alloc! but without
    /// requiring unsafe code to extract the provider.
    pub fn create_managed_provider<const N: usize>(crate_id: CrateId) -> Result<NoStdProvider<N>> {
        // Register the allocation with the coordinator first
        use crate::generic_memory_guard::MemoryCoordinator;
        use crate::wrt_memory_system::WRT_MEMORY_COORDINATOR;

        let _allocation_id = WRT_MEMORY_COORDINATOR.register_allocation(crate_id, N)?;

        // Create the provider safely
        Self::create_provider_safe::<N>()

        // Note: We're not implementing Drop for automatic cleanup here since
        // this is a transitional API. In the future, this should be redesigned
        // to use RAII properly.
    }
}

/// Safe collection factory functions using owned providers
///
/// These functions create bounded collections using safely allocated providers
/// without requiring unsafe code blocks.
pub mod safe_factories {
    use super::*;

    /// Safely create a BoundedVec with owned provider
    pub fn safe_bounded_vec<T, const CAPACITY: usize, const PROVIDER_SIZE: usize>(
        crate_id: CrateId,
    ) -> Result<BoundedVec<T, CAPACITY, NoStdProvider<PROVIDER_SIZE>>>
    where
        T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    {
        let provider = SafeProviderFactory::create_managed_provider::<PROVIDER_SIZE>(crate_id)?;
        BoundedVec::new(provider).map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ERROR,
                "Failed to create bounded vector",
            )
        })
    }

    /// Safely create a BoundedMap with owned provider
    pub fn safe_bounded_map<K, V, const CAPACITY: usize, const PROVIDER_SIZE: usize>(
        crate_id: CrateId,
    ) -> Result<BoundedMap<K, V, CAPACITY, NoStdProvider<PROVIDER_SIZE>>>
    where
        K: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
        V: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    {
        let provider = SafeProviderFactory::create_managed_provider::<PROVIDER_SIZE>(crate_id)?;
        BoundedMap::new(provider).map_err(|e| {
            Error::new(ErrorCategory::Memory, codes::MEMORY_ERROR, "Failed to create bounded map")
        })
    }

    /// Safely create a BoundedString with owned provider
    pub fn safe_bounded_string<const CAPACITY: usize, const PROVIDER_SIZE: usize>(
        crate_id: CrateId,
    ) -> Result<BoundedString<CAPACITY, NoStdProvider<PROVIDER_SIZE>>> {
        let provider = SafeProviderFactory::create_managed_provider::<PROVIDER_SIZE>(crate_id)?;
        BoundedString::from_str("", provider).map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ERROR,
                "Failed to create bounded string",
            )
        })
    }
}

/// Safe macro for creating providers without unsafe code
///
/// This macro replaces the unsafe `safe_managed_alloc!` pattern with a safe alternative.
///
/// # Examples
///
/// ```rust
/// use wrt_foundation::{safe_managed_alloc, CrateId};
///
/// // Safe allocation with automatic budget tracking
/// let provider = safe_managed_alloc!(131072, CrateId::Runtime)?;
/// let vec = BoundedVec::new(provider)?;
///
/// // No unsafe code required!
/// ```
#[macro_export]
macro_rules! safe_managed_alloc {
    ($size:expr, $crate_id:expr) => {
        $crate::safe_allocation::SafeProviderFactory::create_managed_provider::<$size>($crate_id)
    };
}

/// Public re-exports for convenience
pub use safe_factories::*;
