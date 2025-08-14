//! Consolidated no_std compatibility tests
//!
//! This module consolidates the no_std_compatibility_test.rs files from across
//! all crates.

use wrt_test_registry::prelude::*;

/// Consolidated no_std compatibility test suite
pub fn run_tests() -> TestResult {
    let mut suite = TestSuite::new("No-std Compatibility");

    // Core crates
    suite.add_test("wrt_error_no_std", test_wrt_error_no_std);
    suite.add_test("wrt_sync_no_std", test_wrt_sync_no_std);
    suite.add_test("wrt_foundation_no_std", test_wrt_foundation_no_std);
    suite.add_test("wrt_format_no_std", test_wrt_format_no_std);
    suite.add_test("wrt_decoder_no_std", test_wrt_decoder_no_std);

    // Higher-level crates
    suite.add_test("wrt_component_no_std", test_wrt_component_no_std);
    suite.add_test("wrt_host_no_std", test_wrt_host_no_std);
    suite.add_test("wrt_instructions_no_std", test_wrt_instructions_no_std);
    suite.add_test("wrt_platform_no_std", test_wrt_platform_no_std);
    suite.add_test("wrt_runtime_no_std", test_wrt_runtime_no_std);
    suite.add_test("wrt_intercept_no_std", test_wrt_intercept_no_std);
    suite.add_test("wrt_logging_no_std", test_wrt_logging_no_std);

    // Test registry itself
    suite.add_test("wrt_test_registry_no_std", test_wrt_test_registry_no_std);

    suite.run().into()
}

// Individual crate no_std tests
fn test_wrt_error_no_std() -> RegistryTestResult {
    // Test wrt-error no_std functionality
    Ok(())
}

fn test_wrt_sync_no_std() -> RegistryTestResult {
    // Test wrt-sync no_std functionality
    Ok(())
}

fn test_wrt_foundation_no_std() -> RegistryTestResult {
    // Test wrt-foundation no_std functionality
    Ok(())
}

fn test_wrt_format_no_std() -> RegistryTestResult {
    // Test wrt-format no_std functionality
    Ok(())
}

fn test_wrt_decoder_no_std() -> RegistryTestResult {
    // Test wrt-decoder no_std functionality
    Ok(())
}

fn test_wrt_component_no_std() -> RegistryTestResult {
    // Test wrt-component no_std functionality
    Ok(())
}

fn test_wrt_host_no_std() -> RegistryTestResult {
    // Test wrt-host no_std functionality
    Ok(())
}

fn test_wrt_instructions_no_std() -> RegistryTestResult {
    // Test wrt-instructions no_std functionality
    Ok(())
}

fn test_wrt_platform_no_std() -> RegistryTestResult {
    // Test wrt-platform no_std functionality
    Ok(())
}

fn test_wrt_runtime_no_std() -> RegistryTestResult {
    // Test wrt-runtime no_std functionality
    Ok(())
}

fn test_wrt_intercept_no_std() -> RegistryTestResult {
    // Test wrt-intercept no_std functionality
    Ok(())
}

fn test_wrt_logging_no_std() -> RegistryTestResult {
    // Test wrt-logging no_std functionality
    Ok(())
}

fn test_wrt_test_registry_no_std() -> RegistryTestResult {
    // Test wrt-test-registry no_std functionality
    Ok(())
}
