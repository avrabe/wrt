// WRT - wrt-test-registry
// SW-REQ-ID: [SW-REQ-ID-wrt-test-registry]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! Test registry for WebAssembly Runtime Testing
//!
//! This module provides a unified testing framework for the WebAssembly Runtime.
//! The framework is designed to work in both std and no_std environments,
//! allowing for consistent testing across all target platforms.
//!
//! ## Features
//!
//! - Support for both std and no_std environments
//! - Bounded collections for memory safety
//! - Test categorization and filtering
//! - Verification level configuration for safety-critical tests
//! - Compatibility test suite to ensure consistent behavior
//!
//! ## Organization
//!
//! The test registry follows the modular design of the WRT project:
//!
//! - Uses `wrt-error` for consistent error handling
//! - Uses `wrt-types` for bounded collections and safety-first primitives
//! - Tests each module independently through direct imports rather than wrt

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]
#![warn(clippy::pedantic)]
#![warn(clippy::missing_panics_doc)]
#![warn(missing_docs)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Create the prelude module for consistent imports
pub mod prelude;

// Re-export the compatibility module
pub mod compatibility;

// Import the runner module
#[cfg(feature = "runner")]
pub mod runner;

// Use prelude for all standard imports
use prelude::*;

/// Result type for test functions.
pub type TestResult = Result<(), String>;

/// Trait that all test cases must implement.
pub trait TestCase: Send + Sync {
    /// The name of the test case.
    fn name(&self) -> &'static str;

    /// The category of the test (e.g., "decoder", "runtime", etc.).
    fn category(&self) -> &'static str;

    /// Whether this test requires the standard library.
    fn requires_std(&self) -> bool;

    /// Optional features this test supports
    fn features(&self) -> &[String] {
        &[]
    }

    /// Run the test case.
    fn run(&self) -> TestResult;

    /// Description of the test case
    fn description(&self) -> &'static str {
        "No description provided"
    }
}

/// Statistics about test execution
#[derive(Debug, Default, Clone)]
pub struct TestStats {
    /// Number of tests passed
    pub passed: usize,
    /// Number of tests failed
    pub failed: usize,
    /// Number of tests skipped
    pub skipped: usize,
    /// Total execution time in milliseconds
    #[cfg(feature = "std")]
    pub execution_time_ms: u64,
    /// Memory usage peak in bytes
    #[cfg(feature = "std")]
    pub peak_memory_usage: usize,
}

/// The test registry that stores all registered tests.
pub struct TestRegistry {
    /// The list of all registered tests.
    #[cfg(feature = "std")]
    tests: Mutex<BoundedVec<Box<dyn TestCase>, 1024>>,

    /// The list of all registered tests for no_std environments.
    #[cfg(not(feature = "std"))]
    tests: OnceCell<BoundedVec<Box<dyn TestCase>, 1024>>,

    /// The number of registered tests.
    count: AtomicUsize,

    /// Test execution statistics
    #[cfg(feature = "std")]
    stats: Mutex<TestStats>,

    /// Global verification level for tests
    #[cfg(feature = "std")]
    verification_level: Mutex<VerificationLevel>,
    #[cfg(not(feature = "std"))]
    verification_level: OnceCell<VerificationLevel>,
}

impl Default for TestRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRegistry {
    /// Create a new, empty test registry.
    pub const fn new() -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                tests: Mutex::new(BoundedVec::new()),
                count: AtomicUsize::new(0),
                stats: Mutex::new(TestStats::default()),
                verification_level: Mutex::new(VerificationLevel::Standard),
            }
        }

        #[cfg(not(feature = "std"))]
        {
            Self {
                tests: OnceCell::new(),
                count: AtomicUsize::new(0),
                verification_level: OnceCell::new(),
            }
        }
    }

    /// Get the global test registry instance.
    pub fn global() -> &'static Self {
        static REGISTRY: OnceCell<TestRegistry> = OnceCell::new();
        REGISTRY.get_or_init(|| {
            let registry = TestRegistry::new();
            #[cfg(not(feature = "std"))]
            {
                let _ = registry.verification_level.set(VerificationLevel::Standard);
            }
            registry
        })
    }

    /// Register a new test case.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the test was registered successfully, or an `Error` with appropriate
    /// error code and category if registration failed.
    pub fn register(&self, test: Box<dyn TestCase>) -> Result<()> {
        #[cfg(feature = "std")]
        {
            let mut tests = self.tests.lock().map_err(|_| {
                Error::new(
                    ErrorCategory::Concurrency,
                    codes::CONCURRENCY_LOCK_FAILURE,
                    "Failed to acquire lock for test registration",
                )
            })?;

            tests.try_push(test).map_err(|e| {
                Error::new(
                    ErrorCategory::Capacity,
                    codes::CAPACITY_LIMIT_EXCEEDED,
                    format!("Test registry capacity exceeded: {}", e),
                )
            })?;

            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        {
            if self.tests.get().is_none() {
                let mut tests = BoundedVec::new();
                tests.try_push(test).map_err(|e| {
                    Error::new(
                        ErrorCategory::Capacity,
                        codes::CAPACITY_LIMIT_EXCEEDED,
                        format!("Test registry capacity exceeded: {}", e),
                    )
                })?;

                if self.tests.set(tests).is_ok() {
                    self.count.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                } else {
                    Err(Error::new(
                        ErrorCategory::Concurrency,
                        codes::CONCURRENCY_INITIALIZATION_FAILURE,
                        "Test registry already initialized",
                    ))
                }
            } else {
                Err(Error::new(
                    ErrorCategory::Concurrency,
                    codes::CONCURRENCY_INITIALIZATION_FAILURE,
                    "Test registry already initialized - cannot add tests after initialization in no_std mode",
                ))
            }
        }
    }

    /// Get the number of registered tests.
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Execute a function with all the registered tests.
    /// This avoids the need to clone the tests.
    #[cfg(feature = "std")]
    pub fn with_tests<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[Box<dyn TestCase>]) -> R,
    {
        let tests = self.tests.lock().expect("Failed to acquire lock for test registry");
        let test_slice = tests.as_slice();
        f(test_slice)
    }

    #[cfg(not(feature = "std"))]
    /// Execute a function with all the registered tests.
    /// This avoids the need to clone the tests.
    pub fn with_tests<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[Box<dyn TestCase>]) -> R,
    {
        let tests = self.tests.get().expect("Test registry not initialized");
        let test_slice = tests.as_slice();
        f(test_slice)
    }

    /// Run filtered tests based on name and category.
    ///
    /// # Arguments
    ///
    /// * `name_filter` - Optional name filter
    /// * `category_filter` - Optional category filter
    /// * `require_std` - Whether to require the standard library
    ///
    /// # Returns
    ///
    /// The number of tests that were run.
    pub fn run_filtered_tests(
        &self,
        name_filter: Option<&str>,
        category_filter: Option<&str>,
        require_std: bool,
    ) -> usize {
        #[cfg(feature = "std")]
        {
            let mut stats = self.stats.lock().unwrap_or_else(|_| {
                panic!("Failed to acquire lock for test statistics");
            });
            stats.passed = 0;
            stats.failed = 0;
            stats.skipped = 0;
            stats.execution_time_ms = 0;
            stats.peak_memory_usage = 0;
        }

        let mut run_count = 0;
        self.with_tests(|tests| {
            for test in tests {
                let should_run = match (name_filter, category_filter) {
                    (Some(name), Some(category)) => {
                        test.name().contains(name) && test.category() == category
                    }
                    (Some(name), None) => test.name().contains(name),
                    (None, Some(category)) => test.category() == category,
                    (None, None) => true,
                };

                if should_run {
                    if !require_std && test.requires_std() {
                        #[cfg(feature = "std")]
                        {
                            let mut stats = self.stats.lock().unwrap();
                            stats.skipped += 1;
                        }
                        continue;
                    }

                    #[cfg(feature = "std")]
                    {
                        use std::time::Instant;
                        println!("Running test: {}", test.name());
                        let start = Instant::now();
                        let result = test.run();
                        let duration = start.elapsed();
                        let mut stats = self.stats.lock().unwrap();
                        stats.execution_time_ms += duration.as_millis() as u64;

                        match result {
                            Ok(()) => {
                                println!("Test passed: {}", test.name());
                                stats.passed += 1;
                            }
                            Err(e) => {
                                println!("Test failed: {} - {}", test.name(), e);
                                stats.failed += 1;
                            }
                        }
                    }

                    #[cfg(not(feature = "std"))]
                    {
                        let result = test.run();
                        if result.is_err() {
                            return;
                        }
                    }

                    run_count += 1;
                }
            }
        });

        run_count
    }

    /// Run all tests.
    ///
    /// # Returns
    ///
    /// The number of tests that were run.
    pub fn run_all_tests(&self) -> usize {
        self.run_filtered_tests(None, None, cfg!(feature = "std"))
    }

    /// Get the current test statistics
    #[cfg(feature = "std")]
    pub fn get_stats(&self) -> TestStats {
        self.stats.lock().expect("Failed to acquire lock").clone()
    }

    /// Set the global verification level for tests.
    pub fn set_verification_level(&self, level: VerificationLevel) -> Result<()> {
        #[cfg(feature = "std")]
        {
            let mut verification_level = self.verification_level.lock().map_err(|_| {
                Error::new(
                    ErrorCategory::Concurrency,
                    codes::CONCURRENCY_LOCK_FAILURE,
                    "Failed to acquire lock for verification level",
                )
            })?;
            *verification_level = level;
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        {
            // Attempt to set the value. If `set` returns Ok, it was successfully set.
            // If `set` returns Err, it means the OnceCell was already initialized.
            if self.verification_level.set(level).is_ok() {
                Ok(())
            } else {
                // It was already set. Check if the existing value is different from the new one.
                match self.verification_level.get() {
                    Some(existing_level) if *existing_level != level => {
                        // Already set to a DIFFERENT value, this is an error.
                        Err(Error::new(
                            ErrorCategory::Configuration,
                            codes::CONFIGURATION_ERROR, // Assuming such a code exists
                            "Verification level already set to a different value in this no_std configuration",
                        ))
                    }
                    _ => {
                        // Either already set to the SAME value, or get() failed (should not happen if set() failed).
                        // If it's the same, it's fine. Consider this Ok.
                        Ok(())
                    }
                }
            }
        }
    }

    /// Get the current global verification level.
    pub fn get_verification_level(&self) -> VerificationLevel {
        #[cfg(feature = "std")]
        {
            *self.verification_level.lock().unwrap_or_else(|e| {
                panic!("Failed to acquire lock for verification level: {}", e);
            })
        }
        #[cfg(not(feature = "std"))]
        {
            *self.verification_level.get().unwrap_or(&VerificationLevel::Standard)
        }
    }
}

/// Configuration for test execution
pub struct TestConfig {
    /// Whether the test is running in std mode
    pub is_std: bool,
    /// Current feature set enabled
    pub features: BoundedVec<String, 32>,
    /// Optional test parameters
    #[cfg(feature = "std")]
    pub params: HashMap<String, String>,
    /// Verification level for safety-critical tests
    pub verification_level: VerificationLevel,
}

impl TestConfig {
    /// Create a new test configuration
    pub fn new(is_std: bool, features: BoundedVec<String, 32>) -> Self {
        Self {
            is_std,
            features,
            #[cfg(feature = "std")]
            params: HashMap::new(),
            verification_level: VerificationLevel::Standard,
        }
    }

    /// Check if a specific feature is enabled
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// Get a parameter value
    #[cfg(feature = "std")]
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }

    /// Set the verification level for safety-critical tests
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }
}

/// Implementation of the TestCase trait
pub struct TestCaseImpl {
    /// The name of the test case
    pub name: &'static str,
    /// The category of the test
    pub category: &'static str,
    /// Whether this test requires the standard library
    pub requires_std: bool,
    /// Which features this test supports
    pub features: BoundedVec<String, 32>,
    /// The test function to run
    pub test_fn: Box<dyn Fn(&TestConfig) -> TestResult + Send + Sync>,
    /// Description of the test
    pub description: &'static str,
}

impl TestCase for TestCaseImpl {
    fn name(&self) -> &'static str {
        self.name
    }

    fn category(&self) -> &'static str {
        self.category
    }

    fn requires_std(&self) -> bool {
        self.requires_std
    }

    fn features(&self) -> &[String] {
        self.features.as_slice()
    }

    fn run(&self) -> TestResult {
        let mut features = BoundedVec::new();
        for feature in self.features.iter() {
            features
                .try_push(feature.clone())
                .map_err(|e| format!("Failed to add feature to test config: {}", e))?;
        }

        let registry = TestRegistry::global();
        let verification_level = registry.get_verification_level();

        let mut config = TestConfig::new(cfg!(feature = "std"), features);
        config.verification_level = verification_level;

        (self.test_fn)(&config)
    }

    fn description(&self) -> &'static str {
        self.description
    }
}

/// Register a test function
#[macro_export]
macro_rules! register_test {
    ($name:expr, $category:expr, $requires_std:expr, $description:expr, $test_fn:expr) => {
        #[ctor::ctor]
        fn __register_test() {
            let features = $crate::prelude::BoundedVec::new();
            let test_case = Box::new($crate::TestCaseImpl {
                name: $name,
                category: $category,
                requires_std: $requires_std,
                features,
                test_fn: Box::new($test_fn),
                description: $description,
            });

            let registry = $crate::TestRegistry::global();
            if let Err(e) = registry.register(test_case) {
                #[cfg(feature = "std")]
                {
                    eprintln!("Failed to register test {}: {}", $name, e);
                }
                debug_assert!(false, "Failed to register test");
            }
        }
    };

    ($name:expr, $category:expr, $requires_std:expr, $description:expr, $features:expr, $test_fn:expr) => {
        #[ctor::ctor]
        fn __register_test() {
            let mut features = $crate::prelude::BoundedVec::new();
            for feature in $features {
                if let Err(_) = features.try_push(feature.to_string()) {
                    #[cfg(feature = "std")]
                    {
                        eprintln!("Too many features for test {}", $name);
                    }
                    debug_assert!(false, "Too many features for test");
                    return;
                }
            }

            let test_case = Box::new($crate::TestCaseImpl {
                name: $name,
                category: $category,
                requires_std: $requires_std,
                features,
                test_fn: Box::new($test_fn),
                description: $description,
            });

            let registry = $crate::TestRegistry::global();
            if let Err(e) = registry.register(test_case) {
                #[cfg(feature = "std")]
                {
                    eprintln!("Failed to register test {}: {}", $name, e);
                }
                debug_assert!(false, "Failed to register test");
            }
        }
    };
}
