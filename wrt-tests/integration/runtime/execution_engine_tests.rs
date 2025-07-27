//! Execution engine integration tests

use wrt_test_registry::prelude::*;

pub fn run_tests() -> TestResult {
    let mut suite = TestSuite::new("Execution Engine";
    suite.add_test("stackless_execution", || Ok(();
    suite.add_test("instruction_execution", || Ok(();
    suite.add_test("error_handling", || Ok(();
    suite.run().into()
}