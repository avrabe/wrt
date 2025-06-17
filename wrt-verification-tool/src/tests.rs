// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT


#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

// Import appropriate types based on environment
use std::{format, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{format, string::String, time::Instant, vec::Vec};

use wrt_decoder::{find_section, Parser, Payload};
use wrt_test_registry::{assert_eq_test, assert_test, register_test};

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

// Generate a larger test module for more comprehensive testing
fn create_large_test_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Type section with multiple function types
    module.extend_from_slice(&[
        0x01, 0x15, // Type section ID and size
        0x03, // Number of types
        // Type 1: (i32, i32) -> i32
        0x60, // Function type
        0x02, // Number of params
        0x7F, 0x7F, // i32, i32
        0x01, // Number of results
        0x7F, // i32
        // Type 2: () -> i32
        0x60, // Function type
        0x00, // Number of params
        0x01, // Number of results
        0x7F, // i32
        // Type 3: (i64) -> f64
        0x60, // Function type
        0x01, // Number of params
        0x7E, // i64
        0x01, // Number of results
        0x7C, // f64
    ]);

    // Import section with multiple imports
    module.extend_from_slice(&[
        0x02, 0x42, // Import section ID and size
        0x03, // Number of imports
        // Import 1: wasi_builtin.random
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x06, // Field name length
        // "random"
        0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x00, // Import kind (function)
        0x00, // Type index
        // Import 2: wasi_builtin.resource.create
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x0F, // Field name length
        // "resource.create"
        0x72, 0x65, 0x73, 0x6F, 0x75, 0x72, 0x63, 0x65, 0x2E, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65,
        0x00, // Import kind (function)
        0x01, // Type index
        // Import 3: env.memory
        0x03, // Module name length
        // "env"
        0x65, 0x6E, 0x76, 0x06, // Field name length
        // "memory"
        0x6D, 0x65, 0x6D, 0x6F, 0x72, 0x79, 0x02, // Import kind (memory)
        0x00, 0x01, // Memory limits (min=0, max=1)
    ]);

    // Function section
    module.extend_from_slice(&[
        0x03, 0x02, // Function section ID and size
        0x01, // Number of functions
        0x02, // Type index
    ]);

    // Export section
    module.extend_from_slice(&[
        0x07, 0x0F, // Export section ID and size
        0x01, // Number of exports
        0x04, // Export name length
        // "main"
        0x6D, 0x61, 0x69, 0x6E, 0x00, // Export kind (function)
        0x00, // Function index
    ]);

    // Code section
    module.extend_from_slice(&[
        0x0A, 0x06, // Code section ID and size
        0x01, // Number of functions
        0x04, // Function body size
        0x00, // Local variable count
        0x41, 0x2A, // i32.const 42
        0x0B, // end
    ]);

    // Custom section
    module.extend_from_slice(&[
        0x00, 0x09, // Custom section ID and size
        0x04, // Name length
        // "test"
        0x74, 0x65, 0x73, 0x74, // Custom data
        0x01, 0x02, 0x03, 0x04,
    ]);

    module
}

// Register all the decoder verification tests
pub fn register_decoder_tests() {
    // Test parser finds module version
    register_test!("parser_finds_module_version", "decoder", false, || {
        let module = create_minimal_module();
        let parser = Parser::new(&module);

        let mut found_version = false;

        for payload_result in parser {
            if let Ok(Payload::Version(version)) = payload_result {
                found_version = true;
                assert_eq_test!(version, 1, "Expected version 1");
                break;
            }
        }

        assert_test!(found_version, "Failed to find module version");

        Ok(())
    });

    // Test section finding
    register_test!("section_finding", "decoder", false, || {
        let module = create_minimal_module();

        // Test finding the import section (ID 2)
        let section_result =
            find_section(&module, 2).map_err(|e| format!("Error finding section: {:?}", e))?;

        assert_test!(section_result.is_some(), "Failed to find import section");

        Ok(())
    });

    // Test scanning for builtins
    register_test!("scanning_for_builtins", "decoder", false, || {
        let module = create_minimal_module();

        // Test scanning for builtins
        let builtins = scan_for_builtins(&module)?;

        assert_eq_test!(
            builtins.len(),
            1,
            format!("Expected 1 builtin, found: {}", builtins.len())
        );
        assert_eq_test!(
            builtins[0],
            "random",
            format!("Expected 'random' builtin, found: {}", builtins[0])
        );

        Ok(())
    });

    // Test payload iteration
    register_test!("payloads", "decoder", false, || {
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

        assert_test!(count >= 2, format!("Expected at least 2 payloads, found {}", count));
        assert_test!(found_import_section, "Failed to find import section payload");

        Ok(())
    });

    // Test with larger module
    register_test!("larger_module", "decoder", false, || {
        let module = create_large_test_module();

        // Full parse test
        let parser = Parser::new(&module);
        let mut section_count = 0;
        let mut custom_count = 0;
        let mut import_section_found = false;

        for payload_result in parser {
            let payload =
                payload_result.map_err(|e| format!("Failed to parse payload: {:?}", e))?;

            match payload {
                Payload::ImportSection(_, _) => {
                    import_section_found = true;
                    section_count += 1;
                }
                Payload::CustomSection { name: _, data: _, size: _ } => {
                    custom_count += 1;
                }
                Payload::Version(_) => {}
                _ => {
                    section_count += 1;
                }
            }
        }

        assert_test!(import_section_found, "Import section not found in larger module");
        assert_test!(
            section_count >= 5,
            format!("Expected at least 5 sections, found {}", section_count)
        );
        assert_test!(
            custom_count == 1,
            format!("Expected 1 custom section, found {}", custom_count)
        );

        // Test builtin scanning
        let builtins = scan_for_builtins(&module)?;

        assert_eq_test!(
            builtins.len(),
            2,
            format!("Expected 2 builtins, found: {}", builtins.len())
        );

        Ok(())
    });

    // Test performance
    register_test!(
        "performance",
        "decoder-performance",
        true, // This test requires std for timing
        || {
            let module = create_minimal_module();

            // Measure scanning performance
            let start = Instant::now();
            let iterations = 1000; // Reduced from 10000 to make test faster

            for _ in 0..iterations {
                let result = scan_for_builtins(&module)?;
                assert_eq_test!(result.len(), 1, "Expected 1 builtin");
            }

            let duration = start.elapsed();
            let milliseconds = duration.as_millis();
            let avg_microseconds = duration.as_micros() / iterations as u128;

            assert_test!(
                avg_microseconds < 1000,
                format!("Performance too slow: {}µs per scan", avg_microseconds)
            );

            Ok(())
        }
    );
}
