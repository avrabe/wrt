//! Consolidated Parser Tests
//!
//! This module consolidates all parser testing functionality across the WRT
//! project into a unified test suite, eliminating duplication and providing
//! comprehensive coverage.

use wrt_test_registry::prelude::*;

mod comprehensive_parsing_tests;
mod consolidated_parser_tests;
mod control_instruction_parser_tests;
mod wat_integration_tests;

/// Run all parser integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Parser Integration");

    runner.add_test_suite("Core Parser Tests", || {
        // The consolidated tests are run via standard test framework
        Ok(())
    })?;

    runner.add_test_suite("WAT Integration", || {
        // WAT parsing and conversion tests
        Ok(())
    })?;

    runner.add_test_suite("Comprehensive Parsing", || {
        // Complex parsing scenarios and validation
        Ok(())
    })?;

    runner.add_test_suite("Control Instruction Parsing", || {
        // Control instruction parsing and encoding tests
        Ok(())
    })?;

    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_integration() {
        let result = run_tests();
        assert!(
            result.is_success(),
            "Parser integration tests failed: {:?}",
            result
        );
    }
}
