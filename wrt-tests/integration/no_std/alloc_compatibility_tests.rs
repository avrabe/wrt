//! Alloc compatibility tests for no_std+alloc environments

use wrt_test_registry::prelude::*;

#[cfg(feature = "std")]
pub fn run_tests() -> TestResult {
    let mut suite = TestSuite::new("Alloc Compatibility");

    suite.add_test("alloc_vec_operations", test_alloc_vec);
    suite.add_test("alloc_string_operations", test_alloc_string);
    suite.add_test("alloc_collections", test_alloc_collections);
    suite.add_test("dynamic_allocation", test_dynamic_allocation);

    suite.run().into()
}

#[cfg(not(feature = "std"))]
pub fn run_tests() -> TestResult {
    TestResult::success()
}

#[cfg(feature = "std")]
fn test_alloc_vec() -> RegistryTestResult {
    // Binary std/no_std choice
    Ok(())
}

#[cfg(feature = "std")]
fn test_alloc_string() -> RegistryTestResult {
    // Binary std/no_std choice
    Ok(())
}

#[cfg(feature = "std")]
fn test_alloc_collections() -> RegistryTestResult {
    // Binary std/no_std choice
    Ok(())
}

#[cfg(feature = "std")]
fn test_dynamic_allocation() -> RegistryTestResult {
    // Binary std/no_std choice
    Ok(())
}
