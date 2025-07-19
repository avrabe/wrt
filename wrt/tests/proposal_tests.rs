/// This file provides a simple test that checks if any proposal features are
/// enabled. The real proposal tests are in wast_tests.rs.

/// Run all available tests for all enabled proposal features
/// This is a convenience function to run everything with detailed reporting
#[test]
fn run_all_enabled_proposal_tests() {
    println!("=================================================";
    println!("Running all enabled proposal tests";
    println!("=================================================";

    #[cfg(feature = "relaxed_simd")]
    println!("✅ relaxed_simd feature is enabled";

    #[cfg(feature = "gc")]
    println!("✅ gc feature is enabled";

    #[cfg(feature = "function_references")]
    println!("✅ function_references feature is enabled";

    #[cfg(feature = "multi_memory")]
    println!("✅ multi_memory feature is enabled";

    #[cfg(feature = "exception_handling")]
    println!("✅ exception_handling feature is enabled";

    #[cfg(feature = "threads")]
    println!("✅ threads feature is enabled";

    #[cfg(feature = "extended_const")]
    println!("✅ extended_const feature is enabled";

    #[cfg(feature = "tail_call")]
    println!("✅ tail_call feature is enabled";

    #[cfg(feature = "wasm_3_0")]
    println!("✅ wasm_3_0 feature is enabled";

    #[cfg(feature = "wide_arithmetic")]
    println!("✅ wide_arithmetic feature is enabled";

    #[cfg(feature = "custom_page_sizes")]
    println!("✅ custom_page_sizes feature is enabled";

    #[cfg(feature = "annotations")]
    println!("✅ annotations feature is enabled";

    #[cfg(not(any(
        feature = "relaxed_simd",
        feature = "gc",
        feature = "function_references",
        feature = "multi_memory",
        feature = "exception_handling",
        feature = "threads",
        feature = "extended_const",
        feature = "tail_call",
        feature = "wasm_3_0",
        feature = "wide_arithmetic",
        feature = "custom_page_sizes",
        feature = "annotations"
    )))]
    println!("⚠️ No proposal features are enabled";

    println!("=================================================";

    #[cfg(feature = "std")]
    println!("Running in std environment";

    #[cfg(not(feature = "std"))]
    println!("Running in no_std environment";

    println!("=================================================";
}

/// Report on environment for execution
#[test]
fn report_test_environment() {
    use std::env;

    // Check if the WASM_TESTSUITE environment variable is set
    if let Ok(testsuite_path) = env::var("WASM_TESTSUITE") {
        println!("WASM testsuite path: {}", testsuite_path;
        if let Ok(commit) = env::var("WASM_TESTSUITE_COMMIT") {
            println!("WASM testsuite commit: {}", commit;
        }
    } else {
        println!("WASM_TESTSUITE environment variable not set";
    }
}
