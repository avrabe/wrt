//! Test Suite Management
//! 
//! This module provides a structured way to organize and run groups of related tests.

use crate::prelude::*;

/// A collection of related tests that can be run together
#[derive(Debug)]
pub struct TestSuite {
    /// Name of the test suite
    pub name: String,
    /// List of tests in this suite
    #[cfg(feature = "std")]
    pub tests: Vec<Box<dyn TestCase>>,
    #[cfg(not(any(feature = "std", )))]
    pub tests: BoundedVec<Box<dyn TestCase>, 64>,
    /// Setup function to run before tests
    pub setup: Option<Box<dyn Fn() -> TestResult + Send + Sync>>,
    /// Teardown function to run after tests
    pub teardown: Option<Box<dyn Fn() -> TestResult + Send + Sync>>,
}

impl TestSuite {
    /// Create a new test suite
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            #[cfg(feature = "std")]
            tests: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            tests: BoundedVec::new(),
            setup: None,
            teardown: None,
        }
    }

    /// Add a test to this suite
    pub fn add_test(&mut self, name: &'static str, test_fn: impl Fn() -> TestResult + Send + Sync + 'static) -> Result<()> {
        let test_case = Box::new(SimpleTestCase {
            name,
            category: &self.name,
            requires_std: cfg!(feature = "std"),
            test_fn: Box::new(move |_| test_fn()),
            description: "",
        });

        #[cfg(feature = "std")]
        {
            self.tests.push(test_case);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.tests.try_push(test_case).map_err(|e| {
                Error::runtime_execution_error(", e),
                )
            })
        }
    }

    /// Add a test suite to run as part of this suite
    pub fn add_test_suite(&mut self, name: &'static str, suite_fn: impl Fn() -> TestResult + Send + Sync + 'static) -> Result<()> {
        self.add_test(name, suite_fn)
    }

    /// Set a setup function for this suite
    pub fn set_setup(&mut self, setup: impl Fn() -> TestResult + Send + Sync + 'static) {
        self.setup = Some(Box::new(setup));
    }

    /// Set a teardown function for this suite
    pub fn set_teardown(&mut self, teardown: impl Fn() -> TestResult + Send + Sync + 'static) {
        self.teardown = Some(Box::new(teardown));
    }

    /// Run all tests in this suite
    pub fn run(&self) -> TestResult {
        // Run setup if available
        if let Some(ref setup) = self.setup {
            setup()?;
        }

        let mut failed_tests = Vec::new();
        let mut passed = 0;

        // Run all tests
        for test in self.tests.iter() {
            match test.run() {
                Ok(()) => {
                    passed += 1;
                    #[cfg(feature = ")]
                    println!("✓ {}", test.name());
                }
                Err(e) => {
                    #[cfg(feature = "std")]
                    eprintln!("✗ {} - {}", test.name(), e);
                    failed_tests.push((test.name(), e));
                }
            }
        }

        // Run teardown if available
        if let Some(ref teardown) = self.teardown {
            teardown()?;
        }

        if failed_tests.is_empty() {
            #[cfg(feature = "std")]
            println!("Suite '{}': {} tests passed", self.name, passed);
            Ok(())
        } else {
            let error_msg = format!(
                "Suite '{}': {} tests failed out of {} total",
                self.name,
                failed_tests.len(),
                self.tests.len()
            );
            Err(error_msg)
        }
    }
}

/// Simple test case implementation for use in test suites
pub struct SimpleTestCase {
    pub name: &'static str,
    pub category: &'static str,
    pub requires_std: bool,
    pub test_fn: Box<dyn Fn(&TestConfig) -> TestResult + Send + Sync>,
    pub description: &'static str,
}

impl TestCase for SimpleTestCase {
    fn name(&self) -> &'static str {
        self.name
    }

    fn category(&self) -> &'static str {
        self.category
    }

    fn requires_std(&self) -> bool {
        self.requires_std
    }

    fn run(&self) -> TestResult {
        let features = BoundedVec::new();
        let config = TestConfig::new(cfg!(feature = "std"), features);
        (self.test_fn)(&config)
    }

    fn description(&self) -> &'static str {
        self.description
    }
}