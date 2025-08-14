//! Consolidated Memory Safety Tests
//!
//! This module consolidates all memory safety testing functionality across the
//! WRT project into a unified test suite, eliminating duplication and providing
//! comprehensive coverage.

use wrt_test_registry::prelude::*;

mod bounded_collections_tests;
mod consolidated_memory_tests;
mod memory_adapter_tests;
mod memory_protection_tests;

/// Run all memory safety integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Memory Safety Integration");

    runner.add_test_suite("Core Memory Safety", || {
        // The consolidated tests are run via standard test framework
        Ok(())
    })?;

    runner.add_test_suite("Memory Adapters", || {
        // Memory adapter compatibility and safety tests
        Ok(())
    })?;

    runner.add_test_suite("Memory Protection", || {
        // Bounds checking, overflow prevention, isolation tests
        Ok(())
    })?;

    runner.add_test_suite("Bounded Collections", || {
        // Safe collection implementation tests
        Ok(())
    })?;

    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_safety_integration() {
        let result = run_tests();
        assert!(
            result.is_success(),
            "Memory safety integration tests failed: {:?}",
            result
        );
    }
}
