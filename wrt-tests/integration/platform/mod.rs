//! Platform Integration Tests
//!
//! This module contains integration tests for platform-specific functionality.

use wrt_test_registry::prelude::*;

pub mod memory_platform_tests;
pub mod platform_optimizations_tests;
pub mod sync_platform_tests;
pub mod threading_tests;

/// Run all platform integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Platform Integration");

    runner.add_test_suite("Memory Platform", memory_platform_tests::run_tests)?;
    runner.add_test_suite("Sync Platform", sync_platform_tests::run_tests)?;

    #[cfg(feature = "threading")]
    runner.add_test_suite("Threading", threading_tests::run_tests)?;

    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_integration() {
        let result = run_tests();
        assert!(
            result.is_success(),
            "Platform integration tests failed: {:?}",
            result
        );
    }
}
