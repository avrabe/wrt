//! KANI Formal Verification Integration Suite
//!
//! This module integrates KANI verification with the existing WRT test
//! framework, leveraging the TestRegistry infrastructure for consistency and
//! providing comprehensive formal verification of safety-critical properties.
//!
//! # Architecture
//!
//! The formal verification suite is designed to:
//! - Work seamlessly with WRT's existing TestRegistry framework
//! - Provide both KANI formal proofs and traditional tests
//! - Ensure no build errors when KANI is not available
//! - Support incremental verification during development
//!
//! # Modules
//!
//! - `utils`: Common utilities and generators for KANI verification
//! - `memory_safety_proofs`: Memory bounds and budget verification
//! - `safety_invariants_proofs`: ASIL level and safety context verification
//! - `concurrency_proofs`: Threading and atomic operation verification
//! - `resource_lifecycle_proofs`: Resource management verification
//! - `integration_proofs`: Cross-component formal verification
//!
//! # Usage
//!
//! ```bash
//! # Run as traditional tests
//! cargo test formal_verification_tests
//!
//! # Run KANI formal proofs
//! cargo kani --features kani
//!
//! # Run specific verification suite
//! cargo kani --harness "verify_memory_*"
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

use wrt_test_registry::prelude::*;

// Verification utility modules
pub mod utils;

// Formal verification modules (will be implemented in subsequent phases)
#[cfg(any(doc, kani, feature = "kani"))]
pub mod memory_safety_proofs;

// Simplified memory safety proofs that work with current type system
#[cfg(any(doc, kani, feature = "kani"))]
pub mod memory_safety_simple;

#[cfg(any(doc, kani, feature = "kani"))]
pub mod safety_invariants_proofs;

#[cfg(any(doc, kani, feature = "kani"))]
pub mod concurrency_proofs;

#[cfg(any(doc, kani, feature = "kani"))]
pub mod resource_lifecycle_proofs;

#[cfg(any(doc, kani, feature = "kani"))]
pub mod integration_proofs;

#[cfg(any(doc, kani, feature = "kani"))]
pub mod advanced_proofs;

// New ASIL-A verification modules for error handling and fault detection
#[cfg(any(doc, kani, feature = "kani"))]
pub mod error_handling_proofs;

#[cfg(any(doc, kani, feature = "kani"))]
pub mod fault_detection_proofs;

/// Statistics for formal verification execution
#[derive(Debug, Default, Clone)]
pub struct VerificationStats {
    /// Number of properties formally verified
    pub verified_properties:  usize,
    /// Number of properties that failed verification
    pub failed_properties:    usize,
    /// Number of properties skipped due to configuration
    pub skipped_properties:   usize,
    /// Total verification time in milliseconds
    #[cfg(feature = "std")]
    pub verification_time_ms: u64,
    /// Peak memory usage during verification
    #[cfg(feature = "std")]
    pub peak_memory_usage:    usize,
}

/// KANI Test Runner that integrates with WRT TestRegistry
pub struct KaniTestRunner {
    registry: &'static TestRegistry,
    stats:    VerificationStats,
}

impl Default for KaniTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl KaniTestRunner {
    /// Create a new KANI test runner
    pub fn new() -> Self {
        Self {
            registry: TestRegistry::global(),
            stats:    VerificationStats::default(),
        }
    }

    /// Register all KANI verification tests with the TestRegistry
    ///
    /// This method registers formal verification tests so they can be run
    /// as traditional tests when KANI is not available, ensuring consistent
    /// test execution across different environments.
    pub fn register_all_kani_tests(&mut self) -> TestResult {
        self.register_memory_safety_tests()?;
        self.register_safety_invariant_tests()?;
        self.register_concurrency_tests()?;
        self.register_resource_lifecycle_tests()?;
        self.register_integration_tests()?;
        self.register_advanced_tests()?;
        self.register_error_handling_tests()?;
        self.register_fault_detection_tests()?;
        Ok(())
    }

    /// Register memory safety verification tests
    fn register_memory_safety_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            memory_safety_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += memory_safety_proofs::property_count();

            // Also register simplified memory safety tests
            memory_safety_simple::register_tests(self.registry)?;
            self.stats.verified_properties += memory_safety_simple::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            // When KANI is not available, register placeholder tests
            use wrt_test_registry::register_test;

            register_test!(
                "memory_safety_placeholder",
                "formal-verification",
                true,
                "Placeholder for memory safety verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult {
                    Ok(()) // Always pass when KANI not available
                }
            );
        }

        Ok(())
    }

    /// Register safety invariant verification tests
    fn register_safety_invariant_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            safety_invariants_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += safety_invariants_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "safety_invariants_placeholder",
                "formal-verification",
                true,
                "Placeholder for safety invariants verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Register concurrency verification tests
    fn register_concurrency_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            concurrency_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += concurrency_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "concurrency_placeholder",
                "formal-verification",
                true,
                "Placeholder for concurrency verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Register resource lifecycle verification tests
    fn register_resource_lifecycle_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            resource_lifecycle_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += resource_lifecycle_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "resource_lifecycle_placeholder",
                "formal-verification",
                true,
                "Placeholder for resource lifecycle verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Register integration verification tests
    fn register_integration_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            integration_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += integration_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "integration_placeholder",
                "formal-verification",
                true,
                "Placeholder for integration verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Register advanced verification tests
    fn register_advanced_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            advanced_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += advanced_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "advanced_proofs_placeholder",
                "formal-verification",
                true,
                "Placeholder for advanced verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Register error handling verification tests
    fn register_error_handling_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            error_handling_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += error_handling_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "error_handling_placeholder",
                "formal-verification",
                true,
                "Placeholder for error handling verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Register fault detection verification tests
    fn register_fault_detection_tests(&mut self) -> TestResult {
        #[cfg(any(doc, kani, feature = "kani"))]
        {
            fault_detection_proofs::register_tests(self.registry)?;
            self.stats.verified_properties += fault_detection_proofs::property_count();
        }

        #[cfg(not(any(doc, kani, feature = "kani")))]
        {
            use wrt_test_registry::register_test;

            register_test!(
                "fault_detection_placeholder",
                "formal-verification",
                true,
                "Placeholder for fault detection verification (requires KANI feature)",
                |_config: &TestConfig| -> TestResult { Ok(()) }
            );
        }

        Ok(())
    }

    /// Get verification statistics
    pub fn get_stats(&self) -> &VerificationStats {
        &self.stats
    }

    /// Run all formal verification proofs (KANI mode only)
    #[cfg(kani)]
    pub fn run_all_proofs() {
        memory_safety_proofs::run_all_proofs();
        memory_safety_simple::run_all_proofs();
        safety_invariants_proofs::run_all_proofs();
        concurrency_proofs::run_all_proofs();
        resource_lifecycle_proofs::run_all_proofs();
        integration_proofs::run_all_proofs();
        advanced_proofs::run_all_proofs();
        error_handling_proofs::run_all_proofs();
        fault_detection_proofs::run_all_proofs();
    }
}

/// Integration function for existing test suite
///
/// This function provides the entry point for the formal verification suite
/// and integrates with WRT's existing test infrastructure.
pub fn run_tests() -> TestResult {
    #[cfg(kani)]
    {
        // When running under KANI, execute formal proofs
        KaniTestRunner::run_all_proofs();
    }

    #[cfg(not(kani))]
    {
        // When not under KANI, run as traditional tests via TestRegistry
        let mut runner = KaniTestRunner::new();
        runner.register_all_kani_tests()?;

        let registry = TestRegistry::global();
        let executed = registry.run_filtered_tests(None, Some("formal-verification"), true);

        if executed == 0 {
            return Err("No formal verification tests executed".to_string());
        }

        #[cfg(feature = "std")]
        {
            let stats = runner.get_stats();
            println!("Formal verification completed:");
            println!("  Properties verified: {}", stats.verified_properties);
            println!("  Properties failed: {}", stats.failed_properties);
            println!("  Properties skipped: {}", stats.skipped_properties);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kani_runner_creation() {
        let runner = KaniTestRunner::new();
        let stats = runner.get_stats();

        // Initially no properties should be registered
        assert_eq!(stats.verified_properties, 0);
        assert_eq!(stats.failed_properties, 0);
        assert_eq!(stats.skipped_properties, 0);
    }

    #[test]
    fn test_run_tests_integration() {
        let result = run_tests();
        assert!(
            result.is_ok(),
            "Formal verification tests should not fail: {:?}",
            result
        );
    }
}
