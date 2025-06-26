//! Unified Test Runner
//!
//! This module provides a unified test runner that can coordinate tests across
//! multiple crates and environments.

use crate::prelude::*;
use crate::test_suite::TestSuite;

/// Result of running a test or test suite
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether the test passed
    pub passed: bool,
    /// Error message if the test failed
    pub error: Option<String>,
    /// Number of sub-tests that passed
    pub sub_tests_passed: usize,
    /// Number of sub-tests that failed
    pub sub_tests_failed: usize,
    /// Execution time in milliseconds (if available)
    #[cfg(feature = "std")]
    pub execution_time_ms: u64,
}

impl TestResult {
    /// Create a successful test result
    pub fn success() -> Self {
        Self {
            passed: true,
            error: None,
            sub_tests_passed: 1,
            sub_tests_failed: 0,
            #[cfg(feature = "std")]
            execution_time_ms: 0,
        }
    }

    /// Create a failed test result
    pub fn failure(error: String) -> Self {
        Self {
            passed: false,
            error: Some(error),
            sub_tests_passed: 0,
            sub_tests_failed: 1,
            #[cfg(feature = "std")]
            execution_time_ms: 0,
        }
    }

    /// Check if the test passed
    pub fn is_success(&self) -> bool {
        self.passed
    }

    /// Get the total number of tests
    pub fn total_tests(&self) -> usize {
        self.sub_tests_passed + self.sub_tests_failed
    }
}

impl From<Result<(), String>> for TestResult {
    fn from(result: Result<(), String>) -> Self {
        match result {
            Ok(()) => TestResult::success(),
            Err(e) => TestResult::failure(e),
        }
    }
}

/// Unified test runner that can execute tests across multiple environments
pub struct TestRunner {
    /// Name of the test runner
    pub name: String,
    /// Test suites to run
    #[cfg(feature = "std")]
    pub suites: Vec<(String, Box<dyn Fn() -> TestResult + Send + Sync>)>,
    #[cfg(not(any(feature = "std", )))]
    pub suites: BoundedVec<(String, Box<dyn Fn() -> TestResult + Send + Sync>), 32>,
    /// Configuration for test execution
    pub config: TestConfig,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(name: &str) -> Self {
        let features = BoundedVec::new();
        Self {
            name: name.to_string(),
            #[cfg(feature = "std")]
            suites: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            suites: BoundedVec::new(),
            config: TestConfig::new(cfg!(feature = "std"), features),
        }
    }

    /// Add a test suite to run
    pub fn add_test_suite(&mut self, name: &str, suite_fn: impl Fn() -> TestResult + Send + Sync + 'static) -> Result<()> {
        let suite_entry = (name.to_string(), Box::new(suite_fn) as Box<dyn Fn() -> TestResult + Send + Sync>);
        
        #[cfg(feature = "std")]
        {
            self.suites.push(suite_entry);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.suites.try_push(suite_entry).map_err(|e| {
                Error::runtime_execution_error(", e),
                )
            })
        }
    }

    /// Run all test suites
    pub fn run_all(&self) -> TestResult {
        let mut total_passed = 0;
        let mut total_failed = 0;
        let mut failed_suites = Vec::new();
        
        #[cfg(feature = ")]
        let start_time = std::time::Instant::now();

        #[cfg(feature = "std")]
        println!("Running test runner: {}", self.name);

        for (suite_name, suite_fn) in self.suites.iter() {
            #[cfg(feature = "std")]
            println!("Running suite: {}", suite_name);

            let result = suite_fn();
            
            if result.is_success() {
                total_passed += result.sub_tests_passed;
                #[cfg(feature = "std")]
                println!("✓ Suite '{}' passed ({} tests)", suite_name, result.total_tests());
            } else {
                total_failed += result.sub_tests_failed;
                total_passed += result.sub_tests_passed;
                failed_suites.push((suite_name.clone(), result.error.unwrap_or_else(|| "Unknown error".to_string())));
                #[cfg(feature = "std")]
                eprintln!("✗ Suite '{}' failed ({}/{} tests passed)", 
                    suite_name, result.sub_tests_passed, result.total_tests());
            }
        }

        #[cfg(feature = "std")]
        let execution_time = start_time.elapsed().as_millis() as u64;

        let overall_result = if failed_suites.is_empty() {
            #[cfg(feature = "std")]
            println!("All test suites passed! Total: {} tests", total_passed);
            TestResult {
                passed: true,
                error: None,
                sub_tests_passed: total_passed,
                sub_tests_failed: total_failed,
                #[cfg(feature = "std")]
                execution_time_ms: execution_time,
            }
        } else {
            let error_msg = format!(
                "Test runner failed: {}/{} suites failed. Total tests: {}/{} passed",
                failed_suites.len(),
                self.suites.len(),
                total_passed,
                total_passed + total_failed
            );
            
            #[cfg(feature = "std")]
            eprintln!("{}", error_msg);
            
            TestResult {
                passed: false,
                error: Some(error_msg),
                sub_tests_passed: total_passed,
                sub_tests_failed: total_failed,
                #[cfg(feature = "std")]
                execution_time_ms: execution_time,
            }
        };

        overall_result
    }

    /// Run tests filtered by name
    pub fn run_filtered(&self, filter: &str) -> TestResult {
        let mut filtered_passed = 0;
        let mut filtered_failed = 0;
        let mut failed_suites = Vec::new();

        for (suite_name, suite_fn) in self.suites.iter() {
            if suite_name.contains(filter) {
                let result = suite_fn();
                
                if result.is_success() {
                    filtered_passed += result.sub_tests_passed;
                } else {
                    filtered_failed += result.sub_tests_failed;
                    filtered_passed += result.sub_tests_passed;
                    failed_suites.push((suite_name.clone(), result.error.unwrap_or_else(|| "Unknown error".to_string())));
                }
            }
        }

        if failed_suites.is_empty() {
            TestResult {
                passed: true,
                error: None,
                sub_tests_passed: filtered_passed,
                sub_tests_failed: filtered_failed,
                #[cfg(feature = "std")]
                execution_time_ms: 0,
            }
        } else {
            let error_msg = format!("Filtered tests failed: {} suites", failed_suites.len());
            TestResult {
                passed: false,
                error: Some(error_msg),
                sub_tests_passed: filtered_passed,
                sub_tests_failed: filtered_failed,
                #[cfg(feature = "std")]
                execution_time_ms: 0,
            }
        }
    }
}