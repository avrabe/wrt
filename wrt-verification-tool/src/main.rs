// WRT - wrt-verification-tool
// Module: WRT Verification Tool
// SW-REQ-ID: REQ_QUAL_005
// SW-REQ-ID: REQ_DEV_003
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

// Tests module
mod tests;
mod platform_verification;

// Import appropriate types based on environment
use std::{format, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{format, process, string::String, time::Instant, vec::Vec};

use wrt_decoder::{find_section, Parser, Payload};
use platform_verification::{
    PlatformVerificationEngine, PlatformVerificationConfigBuilder, 
    ContainerRuntime, ExternalLimitSources
};

// Display implementation for no_std environments
#[cfg(not(feature = "std"))]
macro_rules! println {
    ($($arg:tt)*) => {{
        // In no_std, we don't print anything, but we could integrate with
        // embedded logging frameworks if needed
    }};
}

// Create a minimal WebAssembly module
fn create_minimal_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Type section (empty)
    module.extend_from_slice(&[0x01, 0x04, 0x01, 0x60, 0x00, 0x00]);

    // Import section with wasi_builtin.random
    module.extend_from_slice(&[
        0x02, 0x16, // Import section ID and size
        0x01, // Number of imports
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x06, // Field name length
        // "random"
        0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x00, // Import kind (function)
        0x00, // Type index
    ]);

    module
}

// Implementation of a simplified scan_for_builtins function
fn scan_for_builtins(binary: &[u8]) -> Result<Vec<String>, String> {
    let parser = Parser::new(binary);
    let mut builtin_imports = Vec::new();

    for payload_result in parser {
        match payload_result {
            Ok(Payload::ImportSection(data, size)) => {
                let reader =
                    match Parser::create_import_section_reader(&Payload::ImportSection(data, size))
                    {
                        Ok(reader) => reader,
                        Err(err) => {
                            return Err(format!("Failed to create import section reader: {}", err));
                        }
                    };

                for import_result in reader {
                    match import_result {
                        Ok(import) => {
                            if import.module == "wasi_builtin" {
                                builtin_imports.push(import.name.to_string());
                            }
                        }
                        Err(err) => {
                            return Err(format!("Failed to parse import: {}", err));
                        }
                    }
                }

                // Import section found and processed, we can stop parsing
                break;
            }
            Err(err) => {
                return Err(format!("Failed to parse module: {}", err));
            }
            _ => {} // Skip other payload types
        }
    }

    Ok(builtin_imports)
}

// Verify parser finds module version
fn test_parser_finds_module_version() -> Result<(), String> {
    println!("Testing parser finds module version...");
    let module = create_minimal_module();
    let parser = Parser::new(&module);

    let mut found_version = false;

    for payload_result in parser {
        if let Ok(Payload::Version(version)) = payload_result {
            found_version = true;
            if version != 1 {
                return Err(format!("Expected version 1, got {}", version));
            }
            break;
        }
    }

    if !found_version {
        return Err("Failed to find module version".into());
    }

    println!("✅ Parser correctly identifies module version");
    Ok(())
}

// Test section finding
fn test_section_finding() -> Result<(), String> {
    println!("Testing section finding...");
    let module = create_minimal_module();

    // Test finding the import section (ID 2)
    let section_result =
        find_section(&module, 2).map_err(|e| format!("Error finding section: {:?}", e))?;

    if section_result.is_none() {
        return Err("Failed to find import section".into());
    }

    println!("✅ Section finding works correctly");
    Ok(())
}

// Test scanning for builtins
fn test_scanning_for_builtins() -> Result<(), String> {
    println!("Testing scanning for builtins...");
    let module = create_minimal_module();

    // Test scanning for builtins
    let builtins = scan_for_builtins(&module)?;

    if builtins.len() != 1 {
        return Err(format!("Expected 1 builtin, found: {}", builtins.len()));
    }

    if builtins[0] != "random" {
        return Err(format!("Expected 'random' builtin, found: {}", builtins[0]));
    }

    println!("✅ Builtin scanning works correctly");
    Ok(())
}

// Test payload iteration
fn test_payloads() -> Result<(), String> {
    println!("Testing payload iteration...");
    let module = create_minimal_module();
    let parser = Parser::new(&module);

    // Test iterating through all payloads
    let mut count = 0;
    let mut found_import_section = false;

    for payload_result in parser {
        let payload = payload_result.map_err(|e| format!("Payload error: {:?}", e))?;
        count += 1;

        match payload {
            Payload::ImportSection(_, _) => {
                found_import_section = true;
            }
            _ => {}
        }
    }

    if count < 2 {
        return Err(format!("Expected at least 2 payloads, found {}", count));
    }

    if !found_import_section {
        return Err("Failed to find import section payload".into());
    }

    println!("✅ Payload iteration works correctly");
    Ok(())
}

// Test section reader
fn test_section_reader() -> Result<(), String> {
    println!("Testing section reader...");
    let module = create_minimal_module();

    // Find the import section
    let section_result =
        find_section(&module, 2).map_err(|e| format!("Error finding section: {:?}", e))?;

    let (offset, size) = section_result.ok_or("Failed to find import section")?;

    // Use the section reader to parse the import section
    let import_data = &module[offset..offset + size];

    if import_data[0] != 0x01 {
        return Err(format!("Expected 1 import, found {}", import_data[0]));
    }

    println!("✅ Section reader works correctly");
    Ok(())
}

// Test performance - only available with std
#[cfg(feature = "std")]
fn test_performance() -> Result<(), String> {
    println!("Testing performance...");
    let module = create_minimal_module();

    // Measure scanning performance
    let start = Instant::now();
    let iterations = 10000;

    for _ in 0..iterations {
        let result = scan_for_builtins(&module)?;
        if result.len() != 1 {
            return Err(format!("Expected 1 builtin, found {}", result.len()));
        }
    }

    let duration = start.elapsed();
    let avg_micros = duration.as_micros() / iterations as u128;

    println!(
        "✅ Performance test complete: {} iterations in {:?} ({} µs/scan)",
        iterations, duration, avg_micros
    );
    Ok(())
}

// No-op performance test for no_std
#[cfg(not(feature = "std"))]
fn test_performance() -> Result<(), String> {
    // Skip performance testing in no_std environments
    Ok(())
}

// Create a larger test module
fn create_large_test_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Add a large type section
    let mut type_section = vec![0x01]; // Section ID
    let mut types = Vec::new();

    // Add 100 function types
    for _ in 0..100 {
        types.extend_from_slice(&[0x60, 0x01, 0x7F, 0x01, 0x7F]); // (param i32)
                                                                  // (result i32)
    }

    // Add length of section content
    let type_count = 100u32;
    let mut type_count_bytes = Vec::new();
    let mut n = type_count;
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        type_count_bytes.push(byte);
        if n == 0 {
            break;
        }
    }

    let content_size = type_count_bytes.len() + types.len();
    let mut content_size_bytes = Vec::new();
    let mut n = content_size as u32;
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        content_size_bytes.push(byte);
        if n == 0 {
            break;
        }
    }

    type_section.extend_from_slice(&content_size_bytes);
    type_section.extend_from_slice(&type_count_bytes);
    type_section.extend_from_slice(&types);

    module.extend_from_slice(&type_section);

    // Add a function section with 100 functions
    let mut function_section = vec![0x03]; // Section ID
    let function_count = 100u32;
    let mut functions = Vec::new();

    for i in 0..100 {
        functions.push(i as u8); // Function i uses type i
    }

    let mut function_count_bytes = Vec::new();
    let mut n = function_count;
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        function_count_bytes.push(byte);
        if n == 0 {
            break;
        }
    }

    let content_size = function_count_bytes.len() + functions.len();
    let mut content_size_bytes = Vec::new();
    let mut n = content_size as u32;
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        content_size_bytes.push(byte);
        if n == 0 {
            break;
        }
    }

    function_section.extend_from_slice(&content_size_bytes);
    function_section.extend_from_slice(&function_count_bytes);
    function_section.extend_from_slice(&functions);

    module.extend_from_slice(&function_section);

    module
}

// Test with a larger module
fn test_larger_module() -> Result<(), String> {
    println!("Testing with larger module...");
    let large_module = create_large_test_module();

    let parser = Parser::new(&large_module);
    let mut section_count = 0;

    for payload_result in parser {
        let _ = payload_result.map_err(|e| format!("Payload error: {:?}", e))?;
        section_count += 1;
    }

    if section_count < 3 {
        return Err(format!("Expected at least 3 sections, found {}", section_count));
    }

    println!("✅ Parser handles larger modules correctly");
    Ok(())
}

// Test platform verification with external limits
#[cfg(feature = "std")]
fn test_platform_verification() -> Result<(), String> {
    println!("Testing platform verification with external limits...");
    
    // Create configuration with CLI args and environment overrides
    let cli_args = vec![
        "--max-memory=512MB".to_string(),
        "--max-components=128".to_string(),
    ];
    
    let config = PlatformVerificationConfigBuilder::new()
        .with_cli_args(cli_args)
        .with_strict_validation(false)
        .build();
    
    let mut engine = PlatformVerificationEngine::with_config(config);
    
    // Discover limits with external overrides
    let limits = engine.discover_limits()
        .map_err(|e| format!("Failed to discover limits: {:?}", e))?;
    
    // Verify that CLI overrides were applied
    if limits.max_total_memory != 512 * 1024 * 1024 {
        return Err(format!(
            "Expected CLI memory override (512MB), got {} bytes", 
            limits.max_total_memory
        ));
    }
    
    if limits.max_components != 128 {
        return Err(format!(
            "Expected CLI components override (128), got {}", 
            limits.max_components
        ));
    }
    
    // Verify basic constraints
    if limits.max_wasm_linear_memory > limits.max_total_memory {
        return Err("WASM memory exceeds total memory".to_string());
    }
    
    if limits.max_stack_bytes == 0 {
        return Err("Stack memory cannot be zero".to_string());
    }
    
    println!("✅ Platform verification with external limits works correctly");
    Ok(())
}

// No-op platform verification test for no_std
#[cfg(not(feature = "std"))]
fn test_platform_verification() -> Result<(), String> {
    // Skip platform verification testing in no_std environments
    Ok(())
}

// Test container runtime detection
#[cfg(feature = "std")]
fn test_container_detection() -> Result<(), String> {
    println!("Testing container runtime detection...");
    
    let config = PlatformVerificationConfigBuilder::new()
        .build(); // This will auto-detect container runtime
    
    // Just verify that detection doesn't crash
    let container_runtime = config.sources.container_runtime;
    println!("Detected container runtime: {:?}", container_runtime);
    
    // Test with explicit Docker configuration
    let docker_config = PlatformVerificationConfigBuilder::new()
        .with_container_runtime(ContainerRuntime::Docker)
        .build();
    
    assert_eq!(docker_config.sources.container_runtime, ContainerRuntime::Docker);
    
    println!("✅ Container runtime detection works correctly");
    Ok(())
}

// No-op container detection test for no_std
#[cfg(not(feature = "std"))]
fn test_container_detection() -> Result<(), String> {
    // Skip container detection testing in no_std environments
    Ok(())
}

// Main function - only available with std
#[cfg(feature = "std")]
fn main() {
    println!("Running wrt-decoder verification tests...");

    // Initialize global memory system first
    if let Err(e) = wrt_foundation::memory_system_initializer::presets::development() {
        eprintln!("Failed to initialize memory system: {}", e);
        process::exit(1);
    }

    // Register all tests with the global registry
    tests::register_decoder_tests();

    // Run all tests
    let registry = wrt_test_registry::TestRegistry::global();
    let failed_count = registry.run_all_tests();

    if failed_count == 0 {
        println!("\n✅ All tests PASSED!");
        println!("Verification completed successfully!");
    } else {
        println!("\n❌ Some tests FAILED!");
        process::exit(1);
    }

    // Complete memory system cleanup
    if let Err(e) = wrt_foundation::memory_system_initializer::complete_global_memory_initialization() {
        eprintln!("Warning: Failed to complete memory system: {}", e);
    }
}

// Entry point for no_std environments
#[cfg(not(feature = "std"))]
fn main() -> ! {
    // Initialize global memory system for embedded environment
    if let Err(_) = wrt_foundation::memory_system_initializer::presets::embedded(32) { // 32KB budget
        // In no_std, we can't easily print errors, so we enter an error loop
        loop {
            core::hint::spin_loop();
        }
    }

    // Register all tests with the global registry
    tests::register_decoder_tests();

    // In a real no_std environment, we would need a custom way to report results
    // Here we just enter an idle loop
    loop {}
}
