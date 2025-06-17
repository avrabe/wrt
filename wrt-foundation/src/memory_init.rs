//! Modern Memory Initialization System
//!
//! This module provides automatic memory system initialization that works
//! across all crates in the WRT ecosystem with zero boilerplate.
//!
//! SW-REQ-ID: REQ_MEM_001 - Memory bounds checking
//! SW-REQ-ID: REQ_MEM_002 - Budget enforcement

use crate::{
    budget_aware_provider::CrateId,
    budget_verification::{CRATE_BUDGETS, TOTAL_MEMORY_BUDGET},
    wrt_memory_system::{WrtMemoryCoordinator, WRT_MEMORY_COORDINATOR},
    Error, Result,
};
use core::sync::atomic::{AtomicBool, Ordering};

/// Global initialization state
static MEMORY_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialization error storage
static mut INIT_ERROR: Option<&'static str> = None;

/// Modern memory system initializer
pub struct MemoryInitializer;

impl MemoryInitializer {
    /// Initialize the entire WRT memory system
    ///
    /// This is called once at startup and configures all crate budgets
    pub fn initialize() -> Result<()> {
        // Check if already initialized
        if MEMORY_INITIALIZED.swap(true, Ordering::AcqRel) {
            return Ok(()); // Already initialized
        }

        // Create budget configuration
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
        ];

        // Initialize coordinator
        match WRT_MEMORY_COORDINATOR.initialize(budgets.iter().copied(), TOTAL_MEMORY_BUDGET) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Store error for debugging
                #[allow(unsafe_code)] // Safe: single-threaded write to static during initialization
                unsafe {
                    INIT_ERROR = Some("Memory coordinator initialization failed");
                }
                MEMORY_INITIALIZED.store(false, Ordering::Release);
                Err(e)
            }
        }
    }

    /// Check if memory system is initialized
    pub fn is_initialized() -> bool {
        MEMORY_INITIALIZED.load(Ordering::Acquire)
    }

    /// Get initialization error if any
    pub fn get_error() -> Option<&'static str> {
        #[allow(unsafe_code)] // Safe: read-only access to static
        unsafe {
            INIT_ERROR
        }
    }

    /// Force reinitialization (for testing only)
    #[cfg(test)]
    pub fn reset() {
        MEMORY_INITIALIZED.store(false, Ordering::Release);
        unsafe {
            INIT_ERROR = None;
        }
    }
}

/// Macro to ensure memory system is initialized
///
/// This macro can be placed at the beginning of any function that uses
/// memory allocation to ensure the system is ready.
#[macro_export]
macro_rules! ensure_memory_init {
    () => {
        if !$crate::memory_init::MemoryInitializer::is_initialized() {
            $crate::memory_init::MemoryInitializer::initialize()?;
        }
    };
}

/// Attribute macro for automatic memory initialization
///
/// Place this on functions that need memory allocation
#[macro_export]
macro_rules! memory_safe {
    ($fn:item) => {
        $fn
        // Note: In a real implementation, this would be a proc macro
        // that injects the initialization check
    };
}

/// Static initializer using ctor if available
#[cfg(feature = "ctor")]
#[ctor::ctor]
fn init_wrt_memory() {
    drop(MemoryInitializer::initialize());
}

/// Manual initialization function for platforms without ctor
#[cfg(not(feature = "ctor"))]
pub fn init_wrt_memory() -> Result<()> {
    MemoryInitializer::initialize()
}

/// Convenience function for crate-specific initialization
pub fn init_crate_memory(crate_id: CrateId) -> Result<()> {
    // Ensure global system is initialized
    MemoryInitializer::initialize()?;

    // Verify crate is properly configured
    let budget = WRT_MEMORY_COORDINATOR.get_crate_budget(crate_id);
    if budget == 0 {
        return Err(Error::new(
            crate::ErrorCategory::Initialization,
            crate::codes::INITIALIZATION_ERROR,
            "Crate has zero budget",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_initialization() {
        MemoryInitializer::reset();

        assert!(!MemoryInitializer::is_initialized());

        MemoryInitializer::initialize().unwrap();

        assert!(MemoryInitializer::is_initialized());
    }

    #[test]
    fn test_crate_initialization() {
        MemoryInitializer::reset();

        init_crate_memory(CrateId::Component).unwrap();

        assert!(MemoryInitializer::is_initialized());
        assert!(WRT_MEMORY_COORDINATOR.get_crate_budget(CrateId::Component) > 0);
    }
}
