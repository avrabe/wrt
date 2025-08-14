// WRT - wrt
// Example: CFI-Protected WebAssembly Execution
// SW-REQ-ID: REQ_CFI_EXAMPLE_001
//
// Copyright (c) 2025 The WRT Project Developers
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Example demonstrating CFI-protected WebAssembly execution
//!
//! This example shows how to use WRT's Control Flow Integrity features
//! to protect WebAssembly execution against ROP/JOP attacks.

use wrt::{
    execute_with_cfi_protection,
    new_cfi_protected_engine,
    CfiConfiguration,
    CfiHardwareFeatures,
    CfiProtectionLevel,
    CfiViolationPolicy,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("WRT CFI-Protected WebAssembly Execution Example");
    println!("=============================================");

    // Example 1: Simple CFI execution with default settings
    println!("\n1. Executing with default CFI protection...");

    let simple_wasm = create_simple_wasm_module();
    match execute_with_cfi_protection(&simple_wasm, "main") {
        Ok(result) => {
            println!("✓ CFI execution successful!");
            println!("  Function executed: {}", result.function_index);
            println!(
                "  Instructions protected: {}",
                result.instruction_results.len()
            );
            println!("  CFI violations detected: {}", result.violations_detected);
        },
        Err(e) => {
            println!("✗ CFI execution failed: {}", e);
        },
    }

    // Example 2: Custom CFI configuration with hardware features
    println!("\n2. Executing with custom CFI configuration...");

    let custom_config = CfiConfiguration {
        protection_level:           CfiProtectionLevel::Hardware,
        max_shadow_stack_depth:     2048,
        landing_pad_timeout_ns:     Some(500_000), // 0.5ms
        violation_policy:           CfiViolationPolicy::LogAndContinue,
        enable_temporal_validation: true,
        hardware_features:          CfiHardwareFeatures {
            arm_bti:     true,
            riscv_cfi:   true,
            x86_cet:     true,
            auto_detect: false, // Use explicit settings
        },
    };

    let complex_wasm = create_complex_wasm_module();
    match wrt::execute_with_cfi_config(&complex_wasm, "fibonacci", custom_config) {
        Ok(result) => {
            println!("✓ Custom CFI execution successful!");
            println!("  Function executed: {}", result.function_index);
            println!(
                "  Instructions protected: {}",
                result.instruction_results.len()
            );
            println!("  CFI violations detected: {}", result.violations_detected);
        },
        Err(e) => {
            println!("✗ Custom CFI execution failed: {}", e);
        },
    }

    // Example 3: CFI engine with persistent state
    println!("\n3. Using persistent CFI engine...");

    match new_cfi_protected_engine() {
        Ok(mut engine) => {
            println!("✓ CFI engine created successfully!");

            // Load module with CFI metadata generation
            match engine.load_module_with_cfi(&simple_wasm) {
                Ok(protected_module) => {
                    println!("✓ Module loaded with CFI protection!");
                    println!(
                        "  Functions with CFI metadata: {}",
                        protected_module.cfi_metadata.functions.len()
                    );

                    // Execute the module
                    match engine.execute_module(&protected_module, "main") {
                        Ok(result) => {
                            println!("✓ Module executed with CFI protection!");
                            println!("  CFI violations: {}", result.violations_detected);
                        },
                        Err(e) => {
                            println!("✗ Module execution failed: {}", e);
                        },
                    }

                    // Print CFI statistics
                    let stats = engine.statistics();
                    println!("\nCFI Execution Statistics:");
                    println!(
                        "  Modules executed: {}",
                        stats.execution_metrics.modules_executed
                    );
                    println!(
                        "  Functions analyzed: {}",
                        stats.metadata_stats.functions_analyzed
                    );
                    println!(
                        "  Instructions protected: {}",
                        stats.runtime_stats.instructions_protected
                    );
                    println!(
                        "  Total violations: {}",
                        stats.execution_metrics.total_violations
                    );
                    println!(
                        "  Total validations: {}",
                        stats.execution_metrics.total_validations
                    );
                    println!(
                        "  Average CFI overhead: {:.2}%",
                        stats.execution_metrics.avg_cfi_overhead_percent
                    );
                },
                Err(e) => {
                    println!("✗ Module loading failed: {}", e);
                },
            }
        },
        Err(e) => {
            println!("✗ CFI engine creation failed: {}", e);
        },
    }

    // Example 4: Demonstrate CFI violation detection
    println!("\n4. Demonstrating CFI violation detection...");

    let malicious_config = CfiConfiguration {
        violation_policy: CfiViolationPolicy::ReturnError,
        ..Default::default()
    };

    let malicious_wasm = create_malicious_wasm_module();
    match wrt::execute_with_cfi_config(&malicious_wasm, "exploit_attempt", malicious_config) {
        Ok(result) => {
            if result.violations_detected > 0 {
                println!(
                    "✓ CFI successfully detected {} violations!",
                    result.violations_detected
                );
            } else {
                println!("⚠ No violations detected (module may be benign)");
            }
        },
        Err(e) => {
            println!("✓ CFI successfully blocked execution: {}", e);
        },
    }

    println!("\nCFI-Protected WebAssembly execution examples completed!");
    Ok(())
}

/// Create a simple WebAssembly module for testing
fn create_simple_wasm_module() -> Vec<u8> {
    // This would contain a real WASM binary in a production example
    // For this example, we create a minimal valid WASM module
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Type section
        0x01, 0x07, 0x01, 0x60, 0x00, 0x01, 0x7f, // Function section
        0x03, 0x02, 0x01, 0x00, // Export section
        0x07, 0x08, 0x01, 0x04, 0x6d, 0x61, 0x69, 0x6e, 0x00, 0x00, // Code section
        0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b,
    ]
}

/// Create a more complex WebAssembly module with function calls
fn create_complex_wasm_module() -> Vec<u8> {
    // This would contain a real WASM binary with recursive function calls
    // For this example, we create a module with multiple functions
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Type section (two function types)
        0x01, 0x0d, 0x02, 0x60, 0x01, 0x7f, 0x01, 0x7f, // (i32) -> i32
        0x60, 0x00, 0x01, 0x7f, // () -> i32
        // Function section
        0x03, 0x03, 0x02, 0x00, 0x01, // Export section
        0x07, 0x0d, 0x01, 0x09, 0x66, 0x69, 0x62, 0x6f, 0x6e, 0x61, 0x63, 0x63, 0x69, 0x00, 0x01,
        // Code section with recursive calls
        0x0a, 0x20, 0x02, 0x0e, 0x00, 0x20, 0x00, 0x41, 0x02, 0x49, 0x04, 0x40, 0x20, 0x00, 0x0f,
        0x0b, 0x20, 0x00, 0x41, 0x01, 0x6b, 0x10, 0x00, 0x20, 0x00, 0x41, 0x02, 0x6b, 0x10, 0x00,
        0x6a, 0x0b, 0x07, 0x00, 0x41, 0x0a, 0x10, 0x00, 0x0b,
    ]
}

/// Create a WebAssembly module that might trigger CFI violations
fn create_malicious_wasm_module() -> Vec<u8> {
    // This would contain WASM that attempts control flow manipulation
    // For this example, we create a module with indirect calls
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Type section
        0x01, 0x07, 0x01, 0x60, 0x00, 0x01, 0x7f, // Function section
        0x03, 0x02, 0x01, 0x00, // Table section (for indirect calls)
        0x04, 0x04, 0x01, 0x70, 0x00, 0x01, // Export section
        0x07, 0x11, 0x01, 0x0e, 0x65, 0x78, 0x70, 0x6c, 0x6f, 0x69, 0x74, 0x5f, 0x61, 0x74, 0x74,
        0x65, 0x6d, 0x70, 0x74, 0x00, 0x00, // Code section with indirect call
        0x0a, 0x0a, 0x01, 0x08, 0x00, 0x41, 0x00, 0x41, 0x00, 0x11, 0x00, 0x0b,
    ]
}
