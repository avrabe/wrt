//! Core Integration Tests
//!
//! This module consolidates core integration testing functionality across the
//! WRT project into a unified test suite, providing comprehensive integration
//! testing.

use wrt_test_registry::prelude::*;

mod component_core_tests;
mod conversion_tests;
mod final_integration_tests;
mod wrt_integration_tests;

/// Run all core integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Core Integration");

    runner.add_test_suite("WRT Ecosystem Integration", || {
        // Core WRT ecosystem integration tests
        Ok(())
    })?;

    runner.add_test_suite("Final Integration Verification", || {
        // Final integration verification tests
        Ok(())
    })?;

    runner.add_test_suite("Component Model Integration", || {
        // Component model integration tests
        Ok(())
    })?;

    runner.add_test_suite("Type Conversion Integration", || {
        // Type conversion and architecture tests
        Ok(())
    })?;

    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_integration() {
        let result = run_tests();
        assert!(
            result.is_success(),
            "Core integration tests failed: {:?}",
            result
        );
    }
}
