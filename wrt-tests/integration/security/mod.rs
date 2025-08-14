//! Security Integration Tests
//!
//! This module contains security-focused integration tests.

use wrt_test_registry::prelude::*;

mod cfi_hardening_tests;
mod memory_protection_tests;
mod validation_tests;

/// Run all security integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Security Integration");

    runner.add_test_suite("CFI Hardening", cfi_hardening_tests::run_tests)?;
    runner.add_test_suite("Memory Protection", memory_protection_tests::run_tests)?;
    runner.add_test_suite("Validation", validation_tests::run_tests)?;

    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_integration() {
        let result = run_tests();
        assert!(
            result.is_success(),
            "Security integration tests failed: {:?}",
            result
        );
    }
}
