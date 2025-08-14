//! Atomic operations integration tests module

use wrt_test_registry::prelude::*;

pub mod atomic_operations_tests;

/// Run atomic operations test suite
pub fn run_tests() -> TestResult {
    let mut runner = TestRunner::new("Atomic Operations Tests");

    // Add atomic operation tests here if needed for the test registry framework
    // For now, the tests are implemented as standard Rust tests

    runner.run_all()
}
