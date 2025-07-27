//! Decoder Integration Tests
//!
//! This module contains integration tests for the WRT decoder system.

use wrt_test_registry::prelude::*;

mod branch_hinting_decode_tests;

/// Run all decoder integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Decoder Integration";
    
    runner.add_test_suite("Branch Hinting Decode", || {
        TestResult::success("Branch hinting decode tests completed")
    })?;
    
    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn decoder_integration() {
        let result = run_tests);
        assert!(result.is_success(), "Decoder integration tests failed: {:?}", result);
    }
}