//! Component Model Integration Tests
//!
//! This module contains comprehensive integration tests for the WebAssembly
//! Component Model implementation across all WRT crates.

use wrt_test_registry::prelude::*;

mod canonical_abi_tests;
mod import_export_tests;
mod instantiation_tests;
mod resource_lifecycle_tests;
mod string_encoding_tests;

/// Test suite for component model functionality
pub fn run_component_model_tests() -> TestResult {
    let mut runner = TestRunner::new("Component Model Integration");

    runner.add_test_suite("Instantiation", instantiation_tests::run_tests);
    runner.add_test_suite("Import/Export", import_export_tests::run_tests);
    runner.add_test_suite("Resource Lifecycle", resource_lifecycle_tests::run_tests);
    runner.add_test_suite("Canonical ABI", canonical_abi_tests::run_tests);
    runner.add_test_suite("String Encoding", string_encoding_tests::run_tests);

    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_model_integration() {
        let result = run_component_model_tests();
        assert!(
            result.is_success(),
            "Component model tests failed: {:?}",
            result
        );
    }
}
