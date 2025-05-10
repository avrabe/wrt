use wrt_component::parser;
use wrt_decoder::types::{Import, ImportDesc};
use wrt_decoder::{Error, Parser, Payload, SectionReader};

/// Helper to create a WebAssembly module header
fn create_wasm_header() -> Vec<u8> {
    vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
}

/// Helper to create a test module with various section types
fn create_test_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Type section with one function signature: (i32, i32) -> i32
    module.extend_from_slice(&[
        0x01, 0x07, // Type section ID and size
        0x01, // Number of types
        0x60, // Function type
        0x02, // Number of params
        0x7F, 0x7F, // i32, i32
        0x01, // Number of results
        0x7F, // i32
    ]);

    // Import section with one import from wasi_builtin
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

    // Function section with one function
    module.extend_from_slice(&[
        0x03, 0x02, // Function section ID and size
        0x01, // Number of functions
        0x00, // Type index
    ]);

    // Export section with one export
    module.extend_from_slice(&[
        0x07, 0x07, // Export section ID and size
        0x01, // Number of exports
        0x03, // Export name length
        // "add"
        0x61, 0x64, 0x64, 0x00, // Export kind (function)
        0x01, // Function index
    ]);

    // Code section with one function body
    module.extend_from_slice(&[
        0x0A, 0x09, // Code section ID and size
        0x01, // Number of functions
        0x07, // Function body size
        0x00, // Local variable count
        0x20, 0x00, // get_local 0
        0x20, 0x01, // get_local 1
        0x6A, // i32.add
        0x0B, // end
    ]);

    module
}

/// Helper to create a module with multiple imports of different types
fn create_multi_import_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Type section with one function signature
    module.extend_from_slice(&[
        0x01, 0x04, // Type section ID and size
        0x01, // Number of types
        0x60, // Function type
        0x00, // No params
        0x00, // No results
    ]);

    // Import section with multiple imports
    module.extend_from_slice(&[
        0x02, 0x39, // Import section ID and size
        0x03, // Number of imports
        // Import 1: wasi_builtin.memory (memory)
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x06, // Field name length
        // "memory"
        0x6D, 0x65, 0x6D, 0x6F, 0x72, 0x79, 0x02, // Import kind (memory)
        0x00, 0x01, // Memory limits (min: 0, max: 1)
        // Import 2: wasi_builtin.table (table)
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x05, // Field name length
        // "table"
        0x74, 0x61, 0x62, 0x6C, 0x65, 0x01, // Import kind (table)
        0x70, // Table element type (funcref)
        0x00, 0x10, // Table limits (min: 0, max: 16)
        // Import 3: wasi_builtin.global (global)
        0x0C, // Module name length
        // "wasi_builtin"
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E,
        0x06, // Field name length
        // "global"
        0x67, 0x6C, 0x6F, 0x62, 0x61, 0x6C, 0x03, // Import kind (global)
        0x7F, 0x00, // Global type (i32, const)
    ]);

    module
}

/// Helper to create a module with invalid import section (truncated)
fn create_invalid_import_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Import section with truncated data
    module.extend_from_slice(&[
        0x02, 0x10, // Import section ID and size
        0x01, // Number of imports
        0x0C, // Module name length
        // "wasi_builtin" (truncated)
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69,
        0x6E,
        // Missing field name and import kind
    ]);

    module
}

/// Helper to create a module with invalid section order
fn create_invalid_order_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Code section before function section (invalid order)
    module.extend_from_slice(&[
        0x0A, 0x04, // Code section ID and size
        0x01, // Number of functions
        0x02, // Function body size
        0x00, // Local variable count
        0x0B, // end
        // Function section after code section (invalid)
        0x03, 0x02, // Function section ID and size
        0x01, // Number of functions
        0x00, // Type index
    ]);

    module
}

#[test]
fn test_parser_basic_module() {
    let module = create_test_module();

    // Parse the module using the Parser
    let parser = Parser::new(&module);
    let payloads: Result<Vec<_>, _> = parser.collect();

    // Check that parsing succeeded
    assert!(payloads.is_ok());
    let payloads = payloads.unwrap();

    // We expect 6 payloads: Version + 5 sections
    assert_eq!(payloads.len(), 6);

    // Verify each section type is present
    let section_ids: Vec<u8> = payloads
        .iter()
        .filter_map(|p| match p {
            Payload::Section { id, .. } => Some(*id),
            _ => None,
        })
        .collect();

    // Check section order
    assert_eq!(section_ids, vec![1, 2, 3, 7, 10]); // Type, Import, Function, Export, Code
}

#[test]
fn test_import_section_parsing() {
    let module = create_test_module();

    // Parse the module using the Parser
    let parser = Parser::new(&module);

    // Find the import section
    let import_section = parser.into_iter().find_map(|payload_result| match payload_result {
        Ok(Payload::Section { id: 2, data, size }) => Some((data, size)),
        _ => None,
    });

    assert!(import_section.is_some());
    let (data, size) = import_section.unwrap();

    // Parse the import section
    let imports = wrt_decoder::parse_import_section(data, 0, size).unwrap();

    // Verify imports
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].module, "wasi_builtin");
    assert_eq!(imports[0].name, "random");

    match imports[0].desc {
        ImportDesc::Function(type_idx) => assert_eq!(type_idx, 0),
        _ => panic!("Expected function import"),
    }
}

#[test]
fn test_multi_import_parsing() {
    let module = create_multi_import_module();

    // Parse the module and find the import section
    let parser = Parser::new(&module);
    let import_section = parser.into_iter().find_map(|payload_result| match payload_result {
        Ok(Payload::Section { id: 2, data, size }) => Some((data, size)),
        _ => None,
    });

    assert!(import_section.is_some());
    let (data, size) = import_section.unwrap();

    // Parse the import section
    let imports = wrt_decoder::parse_import_section(data, 0, size).unwrap();

    // Verify we have 3 imports
    assert_eq!(imports.len(), 3);

    // Check the types of imports
    match &imports[0].desc {
        ImportDesc::Memory(_) => {}
        _ => panic!("Expected memory import"),
    }

    match &imports[1].desc {
        ImportDesc::Table(_) => {}
        _ => panic!("Expected table import"),
    }

    match &imports[2].desc {
        ImportDesc::Global(_) => {}
        _ => panic!("Expected global import"),
    }

    // Verify all are from wasi_builtin
    for import in &imports {
        assert_eq!(import.module, "wasi_builtin");
    }
}

#[test]
fn test_scan_for_wasi_builtins() {
    let module = create_multi_import_module();

    // Use the scan_for_builtins function
    let builtins = parser::scan_for_builtins(&module).unwrap();

    // Should find 3 builtins
    assert_eq!(builtins.len(), 3);
    assert!(builtins.contains(&"memory".to_string()));
    assert!(builtins.contains(&"table".to_string()));
    assert!(builtins.contains(&"global".to_string()));
}

#[test]
fn test_invalid_import_section() {
    let module = create_invalid_import_module();

    // Parse the module
    let parser = Parser::new(&module);
    let result: Result<Vec<_>, _> = parser.collect();

    // Parsing should fail because of the truncated import section
    assert!(result.is_err());
}

#[test]
fn test_invalid_section_order() {
    let module = create_invalid_order_module();

    // Parse the module
    let parser = Parser::new(&module);
    let result: Result<Vec<_>, _> = parser.collect();

    // The parser should at least be able to read both sections
    assert!(result.is_ok());

    // However, validation might fail if implemented
    // This test is primarily to ensure that the parser can still read
    // sections even if they're in an invalid order
}

#[test]
fn test_section_reader_random_access() {
    let module = create_test_module();

    // Create a section reader
    let mut reader = SectionReader::new(&module).unwrap();

    // Find the import section directly
    let import_section = reader.find_section(2).unwrap().unwrap();

    // Find the export section directly
    let export_section = reader.find_section(7).unwrap().unwrap();

    // Verify both were found
    assert!(import_section.0 > 0);
    assert!(export_section.0 > 0);

    // Verify they're in the right order
    assert!(import_section.0 < export_section.0);
}

#[test]
fn test_non_existent_section() {
    let module = create_test_module();

    // Create a section reader
    let mut reader = SectionReader::new(&module).unwrap();

    // Try to find a section that doesn't exist (e.g., data section)
    let data_section = reader.find_section(11).unwrap(); // 11 is data section ID

    // Should return None as the section doesn't exist
    assert!(data_section.is_none());
}

#[test]
fn test_empty_import_section() {
    // Create a module with an empty import section
    let mut module = create_wasm_header();

    // Import section with zero imports
    module.extend_from_slice(&[
        0x02, 0x01, // Import section ID and size
        0x00, // Number of imports (0)
    ]);

    // Parse the module and find the import section
    let parser = Parser::new(&module);
    let import_section = parser.into_iter().find_map(|payload_result| match payload_result {
        Ok(Payload::Section { id: 2, data, size }) => Some((data, size)),
        _ => None,
    });

    assert!(import_section.is_some());
    let (data, size) = import_section.unwrap();

    // Parse the import section
    let imports = wrt_decoder::parse_import_section(data, 0, size).unwrap();

    // Verify there are no imports
    assert_eq!(imports.len(), 0);
}
