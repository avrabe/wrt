use wrt_component::error::Error as ComponentError;
use wrt_component::parser;
use wrt_component::Error;
use wrt_decoder::section_reader::SectionReader;
use wrt_decoder::Error as DecoderError;
use wrt_decoder::{ModuleParser, Parser, Payload};
use wrt_types::values::Value;

/// Helper to create a WebAssembly module with various import types
fn create_module_with_imports() -> Vec<u8> {
    // WebAssembly module header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Type section
    module.extend_from_slice(&[
        0x01, 0x13, // Type section ID and size
        0x03, // Number of types
        // Function type 1: (i32, i32) -> i32
        0x60, // Function type
        0x02, // Number of params
        0x7F, 0x7F, // i32, i32
        0x01, // Number of results
        0x7F, // i32
        // Function type 2: () -> i32
        0x60, // Function type
        0x00, // No params
        0x01, // Number of results
        0x7F, // i32
        // Function type 3: (i64) -> f64
        0x60, // Function type
        0x01, // Number of params
        0x7E, // i64
        0x01, // Number of results
        0x7C, // f64
    ]);

    // Import section with various imports from wasi_builtin
    module.extend_from_slice(&[
        0x02, 0x37, // Import section ID and size
        0x03, // Number of imports
        // Import 1: wasi_builtin.random (function)
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x06, // Field name length
        // "random"
        0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x00, // Import kind (function)
        0x00, // Type index
        // Import 2: wasi_builtin.resource.create (function)
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x0F, // Field name length
        // "resource.create"
        0x72, 0x65, 0x73, 0x6F, 0x75, 0x72, 0x63, 0x65, 0x2E, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65,
        0x00, // Import kind (function)
        0x01, // Type index
        // Import 3: env.memory (memory)
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

    // Code section
    module.extend_from_slice(&[
        0x0A, 0x06, // Code section ID and size
        0x01, // Number of functions
        0x04, // Function body size
        0x00, // Local variable count
        0x42, 0x01, // i64.const 1
        0x0B, // end
    ]);

    module
}

/// Helper to create a module with custom section
fn create_module_with_custom_section() -> Vec<u8> {
    // WebAssembly module header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Custom section
    module.extend_from_slice(&[
        0x00, 0x10, // Custom section ID and size
        0x04, // Name length
        // "test"
        0x74, 0x65, 0x73, 0x74, // Custom data (11 bytes)
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ]);

    // Type section (minimal)
    module.extend_from_slice(&[
        0x01, 0x04, // Type section ID and size
        0x01, // Number of types
        0x60, // Function type
        0x00, // No params
        0x00, // No results
    ]);

    module
}

#[test]
fn test_builtin_scanning_comprehensive() {
    // Test with a module containing multiple wasi_builtin imports
    let module = create_module_with_imports();

    // Test scanning for built-ins
    let builtin_names = parser::scan_for_builtins(&module).unwrap();

    // Verify we found the expected built-ins
    assert_eq!(builtin_names.len(), 2);
    assert!(builtin_names.contains(&"random".to_string()));
    assert!(builtin_names.contains(&"resource.create".to_string()));

    // Verify we didn't get any imports from other modules
    assert!(!builtin_names.contains(&"memory".to_string()));
}

#[test]
fn test_import_section_iteration() {
    let module = create_module_with_imports();

    // Parse the module with the streaming parser
    let parser = Parser::new(&module);
    let mut found_import_section = false;

    for payload_result in parser {
        let payload = payload_result.unwrap();

        // Check if we found the import section
        if let Payload::ImportSection(data, size) = payload {
            found_import_section = true;

            // Process the import section using wrt_decoder's import section reader
            let reader = wrt_decoder::ImportSectionReader::new(data, size).unwrap();
            let imports: Vec<_> = reader.collect::<Result<_, _>>().unwrap();

            // Verify the imports
            assert_eq!(imports.len(), 3);

            // Check import details (first import)
            assert_eq!(imports[0].module, "wasi_builtin");
            assert_eq!(imports[0].name, "random");

            // Check second import
            assert_eq!(imports[1].module, "wasi_builtin");
            assert_eq!(imports[1].name, "resource.create");

            // Check third import
            assert_eq!(imports[2].module, "env");
            assert_eq!(imports[2].name, "memory");
        }
    }

    assert!(found_import_section, "Import section not found in the module");
}

#[test]
fn test_section_reader_integration() {
    let module = create_module_with_imports();

    // Test finding a specific section using the section reader
    let section_id = 2; // Import section
    let section_data = wrt_decoder::find_section(&module, section_id).unwrap();

    assert!(section_data.is_some(), "Import section not found");

    let (offset, size) = section_data.unwrap();
    let section_slice = &module[offset..offset + size];

    // Verify the section size matches what's expected
    assert_eq!(size, 0x37);

    // Verify the section contains the expected data
    assert_eq!(section_slice[0], 0x03); // Number of imports
}

#[test]
fn test_custom_section_handling() {
    let module = create_module_with_custom_section();

    // Test parsing the module with custom sections
    let parser = Parser::new(&module);
    let mut found_custom_section = false;

    for payload_result in parser {
        let payload = payload_result.unwrap();

        // Check if we found the custom section
        if let Payload::CustomSection(name, data, _) = payload {
            found_custom_section = true;

            // Verify the custom section name
            assert_eq!(name, "test");

            // Verify the custom section data
            assert_eq!(data.len(), 11);
            assert_eq!(data[0], 0x01);
            assert_eq!(data[10], 0x0B);
        }
    }

    assert!(found_custom_section, "Custom section not found in the module");
}

#[test]
fn test_module_parser() {
    let module = create_module_with_imports();

    // Test the module parser
    let parser = ModuleParser::new();
    let result = parser.parse(&module);

    // Verify module parsing succeeded
    assert!(result.is_ok(), "Module parsing failed: {:?}", result.err());

    // Check the parsed module contents
    let parsed_module = result.unwrap();

    // Verify the imports were parsed correctly
    assert_eq!(parsed_module.imports.len(), 3);

    // Verify the function types were parsed correctly
    assert_eq!(parsed_module.types.len(), 3);

    // Verify the functions were parsed correctly
    assert_eq!(parsed_module.functions.len(), 1);
}

#[test]
fn test_error_handling_integration() {
    // Create a malformed module (truncated)
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    module.extend_from_slice(&[0x01, 0x08, 0x01]); // Incomplete type section

    // Test error handling in builtin scanning
    let result = parser::scan_for_builtins(&module);
    assert!(result.is_err(), "Expected error for malformed module");
}

#[test]
fn test_performance() {
    // Create a larger module for performance testing
    let mut large_module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Add a type section with many types
    let mut type_section = vec![0x01]; // Section ID
    let mut type_data = vec![100]; // 100 types

    // Add 100 identical function types
    for _ in 0..100 {
        type_data.extend_from_slice(&[
            0x60, // Function type
            0x02, // Number of params
            0x7F, 0x7F, // i32, i32
            0x01, // Number of results
            0x7F, // i32
        ]);
    }

    // Set section size
    type_section.push(type_data.len() as u8);

    // Add the type section
    large_module.extend_from_slice(&type_section);
    large_module.extend_from_slice(&type_data);

    // Add a minimal import section
    large_module.extend_from_slice(&[
        0x02, 0x15, // Import section ID and size
        0x01, // Number of imports
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x03, // Field name length
        // "foo"
        0x66, 0x6F, 0x6F, 0x00, // Import kind (function)
        0x00, // Type index
    ]);

    // Time the parsing operation
    let start = std::time::Instant::now();
    let _ = parser::scan_for_builtins(&large_module).unwrap();
    let duration = start.elapsed();

    // Just verify that parsing completes without timing out
    // This is a very simple performance test, but it ensures
    // the operation doesn't take an unreasonable amount of time
    assert!(duration.as_millis() < 500, "Parsing took too long: {:?}", duration);
}

// Helper function to create a minimal valid WebAssembly module with imports
fn create_test_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Type section (id = 1)
    // One type: [] -> []
    let type_section = vec![0x01, 0x04, 0x01, 0x60, 0x00, 0x00];
    module.extend_from_slice(&type_section);

    // Import section (id = 2)
    // One import: wasi_builtin.random (func)
    let import_section = vec![
        0x02, 0x13, 0x01, 0x0C, 0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69,
        0x6E, 0x06, 0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x00, 0x00,
    ];
    module.extend_from_slice(&import_section);

    // Custom section (id = 0)
    let custom_section =
        vec![0x00, 0x0A, 0x04, 0x74, 0x65, 0x73, 0x74, 0x01, 0x02, 0x03, 0x04, 0x05];
    module.extend_from_slice(&custom_section);

    module
}

// Helper function to create a malformed WebAssembly module
fn create_malformed_module() -> Vec<u8> {
    // Start with valid header
    let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    // Add malformed import section (wrong length)
    let bad_section = vec![
        0x02, 0x20, // Incorrect section size
        0x01, 0x0C, 0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E, 0x06,
        0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x00, 0x00,
    ];
    module.extend_from_slice(&bad_section);

    module
}

#[test]
fn test_scan_for_builtins() {
    let module = create_test_module();

    // Scan for builtins
    let builtins = parser::scan_for_builtins(&module).expect("Failed to scan for builtins");

    // Verify result
    assert_eq!(builtins.len(), 1);
    assert_eq!(builtins[0], "random");
}

#[test]
fn test_parser_traversal() {
    let module = create_test_module();

    // Parse the module
    let parser = Parser::new(&module);
    let payloads: Result<Vec<_>, _> = parser.collect();
    let payloads = payloads.expect("Failed to parse module");

    // Verify correct number of sections (version + type + import + custom + end)
    assert_eq!(payloads.len(), 5);

    // Verify version
    match &payloads[0] {
        Payload::Version(v) => assert_eq!(*v, 1),
        _ => panic!("Expected Version payload"),
    }

    // Verify type section
    match &payloads[1] {
        Payload::TypeSection(_, _) => {}
        _ => panic!("Expected TypeSection payload"),
    }

    // Verify import section
    match &payloads[2] {
        Payload::ImportSection(_, _) => {}
        _ => panic!("Expected ImportSection payload"),
    }

    // Verify custom section
    match &payloads[3] {
        Payload::CustomSection { name, .. } => assert_eq!(name, "test"),
        _ => panic!("Expected CustomSection payload"),
    }

    // Verify end
    match &payloads[4] {
        Payload::End => {}
        _ => panic!("Expected End payload"),
    }
}

#[test]
fn test_import_section_reader() {
    let module = create_test_module();

    // Parse the module to find import section
    let parser = Parser::new(&module);

    for payload in parser {
        let payload = payload.expect("Failed to parse payload");

        // When we find the import section, test the reader
        if let Payload::ImportSection(data, size) = payload {
            let reader = Parser::create_import_section_reader(&Payload::ImportSection(data, size))
                .expect("Failed to create import section reader");

            let imports: Result<Vec<_>, _> = reader.collect();
            let imports = imports.expect("Failed to read imports");

            // Verify imports
            assert_eq!(imports.len(), 1);
            assert_eq!(imports[0].module(), "wasi_builtin");
            assert_eq!(imports[0].name(), "random");

            return; // Test passed
        }
    }

    panic!("Import section not found");
}

#[test]
fn test_error_handling() {
    let module = create_malformed_module();

    // Attempt to scan for builtins in malformed module
    let result = parser::scan_for_builtins(&module);

    // Verify that an error is returned
    assert!(result.is_err());
}

#[test]
fn test_parser_performance() {
    let module = create_test_module();

    // Measure time for 1000 iterations
    let start = std::time::Instant::now();

    for _ in 0..1000 {
        let _ = parser::scan_for_builtins(&module).expect("Failed to scan for builtins");
    }

    let duration = start.elapsed();

    // Print performance info
    println!("Parser performance test: Scanned 1000 modules in {:?}", duration);
    // Ensure the test doesn't take too long (adjust based on hardware)
    assert!(duration.as_millis() < 1000, "Parser is too slow");
}
