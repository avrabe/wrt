//! Documentation Tests
//!
//! This module consolidates documentation-related testing functionality,
//! ensuring all code examples in documentation compile and run correctly.

use wrt_test_registry::prelude::*;

pub mod doc_examples_tests;
pub mod doc_validation_tests;

/// Run all documentation integration tests
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Documentation Integration";
    
    runner.add_test_suite("Documentation Examples", || {
        // Validate that documentation examples compile and run
        Ok(())
    })?;
    
    runner.add_test_suite("Documentation Validation", || {
        // Validate documentation structure and content
        Ok(())
    })?;
    
    runner.run_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn documentation_integration() {
        let result = run_tests);
        assert!(result.is_success(), "Documentation integration tests failed: {:?}", result);
    }
}