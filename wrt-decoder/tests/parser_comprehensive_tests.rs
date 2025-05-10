use wrt_decoder::parser::{Parser, Payload};
use wrt_decoder::prelude::*;
use wrt_decoder::section_reader::SectionReader;
use wrt_error::{Error, Result};
use wrt_format::module::Import;
use wrt_format::module::ImportDesc;

/// Create a WebAssembly module header with magic bytes and version
fn create_wasm_header() -> Vec<u8> {
    vec![
        // Magic bytes
        0x00, 0x61, 0x73, 0x6D, // Version
        0x01, 0x00, 0x00, 0x00,
    ]
}

/// Create a test module with basic sections
fn create_test_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Type section with one function signature
    let mut type_section = Vec::new();
    type_section.push(0x01); // Number of types
    type_section.push(0x60); // Function type
    type_section.push(0x01); // Number of parameters
    type_section.push(0x7F); // i32 parameter
    type_section.push(0x01); // Number of results
    type_section.push(0x7E); // i64 result

    // Add type section to module with correct size
    module.push(0x01); // Section ID (type)
    module.push(type_section.len() as u8); // Section size
    module.extend_from_slice(&type_section);

    // Import section with one function import
    let mut import_section = Vec::new();

    // Number of imports (1)
    import_section.push(0x01);

    // Module name "wasi_builtin"
    import_section.push(0x0C); // Module name length
    import_section.extend_from_slice(b"wasi_builtin");

    // Field name "random"
    import_section.push(0x06); // Field name length
    import_section.extend_from_slice(b"random");

    // Import kind and type index
    import_section.push(0x00); // Function import
    import_section.push(0x00); // Type index 0

    // Add import section to module with correct size
    module.push(0x02); // Section ID (import)
    module.push(import_section.len() as u8); // Section size
    module.extend_from_slice(&import_section);

    // Function section with one function
    let mut function_section = Vec::new();
    function_section.push(0x01); // Number of functions
    function_section.push(0x00); // Type index

    // Add function section to module with correct size
    module.push(0x03); // Section ID (function)
    module.push(function_section.len() as u8); // Section size
    module.extend_from_slice(&function_section);

    // Export section with one export
    let mut export_section = Vec::new();
    export_section.push(0x01); // Number of exports
    export_section.push(0x04); // Export name length
    export_section.extend_from_slice(b"main"); // "main"
    export_section.push(0x00); // Export kind (function)
    export_section.push(0x01); // Function index (0 = import, 1 = local function)

    // Add export section to module with correct size
    module.push(0x07); // Section ID (export)
    module.push(export_section.len() as u8); // Section size
    module.extend_from_slice(&export_section);

    // Code section with empty function
    let mut code_section = Vec::new();
    code_section.push(0x01); // Number of functions
    code_section.push(0x02); // Function body size
    code_section.push(0x00); // Local declarations count
    code_section.push(0x0B); // End opcode

    // Add code section to module with correct size
    module.push(0x0A); // Section ID (code)
    module.push(code_section.len() as u8); // Section size
    module.extend_from_slice(&code_section);

    module
}

/// Create a test module with multiple import types
fn create_multi_import_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Build import section
    let mut import_section = Vec::new();

    // Number of imports (3)
    import_section.push(0x03);

    // Import 1: memory import
    import_section.push(0x0C); // Module name length
    import_section.extend_from_slice(b"wasi_builtin");
    import_section.push(0x06); // Field name length
    import_section.extend_from_slice(b"memory");
    import_section.push(0x02); // Import kind (memory)
    import_section.push(0x00); // No max flag
    import_section.push(0x01); // Min pages

    // Import 2: table import
    import_section.push(0x0C); // Module name length
    import_section.extend_from_slice(b"wasi_builtin");
    import_section.push(0x05); // Field name length
    import_section.extend_from_slice(b"table");
    import_section.push(0x01); // Import kind (table)
    import_section.push(0x70); // Element type: funcref
    import_section.push(0x01); // Has max flag
    import_section.push(0x01); // Min size
    import_section.push(0x10); // Max size

    // Import 3: global import
    import_section.push(0x0C); // Module name length
    import_section.extend_from_slice(b"wasi_builtin");
    import_section.push(0x06); // Field name length
    import_section.extend_from_slice(b"global");
    import_section.push(0x03); // Import kind (global)
    import_section.push(0x7F); // Value type: i32
    import_section.push(0x01); // Mutable flag

    // Add import section with correct size
    module.push(0x02); // Section ID (import)
    module.push(import_section.len() as u8); // Section size
    module.extend_from_slice(&import_section);

    module
}

/// Helper to encode a u32 as a LEB128
fn varint_u32(mut value: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            break;
        }
    }
    bytes
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

/// Create a module with invalid section order
fn create_invalid_order_module() -> Vec<u8> {
    // WebAssembly module header
    let mut module = create_wasm_header();

    // Function section (should come after Type section)
    module.extend_from_slice(&[
        0x03, 0x02, // Function section ID and size
        0x01, // Number of functions
        0x00, // Type index
    ]);

    // Type section (out of order - should come before Function section)
    module.extend_from_slice(&[
        0x01, 0x06, // Type section ID and size
        0x01, // Number of types
        0x60, // Function type
        0x01, // Number of parameters
        0x7F, // i32 parameter
        0x01, // Number of results
        0x7F, // i32 result
    ]);

    // Code section
    module.extend_from_slice(&[
        0x0A, 0x04, // Code section ID and size
        0x01, // Number of functions
        0x02, // Function body size
        0x00, // Local declarations count
        0x0B, // End opcode
    ]);

    module
}

/// Parse an import section from raw data
fn parse_import_section(data: SafeSlice, offset: usize, size: usize) -> Result<Vec<Import>> {
    // Convert SafeSlice to &[u8]
    let bytes = data.data()?;

    // Read the number of imports
    let (count, mut offset) = wrt_format::binary::read_leb128_u32(bytes, 0)?;
    let mut imports = Vec::with_capacity(count as usize);

    for _ in 0..count {
        // Parse module name
        let (module, new_offset) = wrt_format::binary::read_name(bytes, offset)?;
        offset = new_offset;

        // Parse field name
        let (name, new_offset) = wrt_format::binary::read_name(bytes, offset)?;
        offset = new_offset;

        // Parse import kind
        let kind = bytes[offset];
        offset += 1;

        // Parse import description
        let desc = match kind {
            0x00 => {
                // Function import
                let (type_idx, new_offset) = wrt_format::binary::read_leb128_u32(bytes, offset)?;
                offset = new_offset;
                ImportDesc::Function(type_idx)
            }
            _ => {
                // For simplicity, assume only function imports in test
                return Err(Error::new(
                    wrt_error::ErrorCategory::Parse,
                    wrt_error::codes::PARSE_ERROR,
                    "Unsupported import kind in test",
                ));
            }
        };

        imports.push(Import {
            module: String::from_utf8_lossy(module).into_owned(),
            name: String::from_utf8_lossy(name).into_owned(),
            desc,
        });
    }

    Ok(imports)
}

#[test]
fn test_parser_basic_module() {
    let module_bytes = create_test_module();
    let mut parser = Parser::new(Some(&module_bytes), false);

    let mut sections_found = 0;
    while let Some(payload) = parser.read().unwrap() {
        match payload {
            Payload::TypeSection(_, _) => sections_found += 1,
            Payload::ImportSection(_, _) => sections_found += 1,
            Payload::FunctionSection(_, _) => sections_found += 1,
            Payload::ExportSection(_, _) => sections_found += 1,
            Payload::CodeSection(_, _) => sections_found += 1,
            Payload::End => break,
            _ => {}
        }
    }
    assert_eq!(sections_found, 5, "Should find all basic sections");
}

#[test]
fn test_import_section_parsing() {
    let module_bytes = create_test_module();
    let mut parser = Parser::new(Some(&module_bytes), false);
    let mut found_import_section = false;

    while let Some(payload) = parser.read().unwrap() {
        if let Payload::ImportSection(data, _) = payload {
            let imports = parse_import_section(data, 0, 0).unwrap(); // offset and size are dummy here
            assert_eq!(imports.len(), 1);
            assert_eq!(imports[0].module, "wasi_builtin");
            assert_eq!(imports[0].name, "random");
            if let ImportDesc::Function(idx) = imports[0].desc {
                assert_eq!(idx, 0);
            } else {
                panic!("Expected function import");
            }
            found_import_section = true;
            break;
        }
    }
    assert!(found_import_section, "Import section not found or parsed");
}

#[test]
fn test_multi_import_parsing() {
    let module_bytes = create_multi_import_module();
    let mut parser = Parser::new(Some(&module_bytes), false);
    let mut found_import_section = false;

    while let Some(payload) = parser.read().unwrap() {
        if let Payload::ImportSection(data, _) = payload {
            let imports = parse_import_section(data, 0, 0).unwrap();
            assert_eq!(imports.len(), 3);
            // TODO: Add detailed checks for each import type
            found_import_section = true;
            break;
        }
    }
    assert!(found_import_section, "Multi-type import section not found or parsed");
}

#[test]
fn test_invalid_import_section() {
    let module_bytes = create_invalid_import_module();
    let mut parser = Parser::new(Some(&module_bytes), true); // Enable error recovery for this test

    // Attempt to parse the module, expecting an error during import section processing
    let result = std::panic::catch_unwind(move || {
        // Collect results, allowing errors to be accumulated if error recovery is on
        parser.collect::<Result<Vec<_>>>() // MODIFIED
    });

    assert!(result.is_ok(), "Parsing should not panic with error recovery");
    let parse_result = result.unwrap();
    assert!(parse_result.is_err(), "Parsing invalid import section should result in an error");
}

#[test]
fn test_invalid_section_order() {
    let module_bytes = create_invalid_order_module();
    let mut parser = Parser::new(Some(&module_bytes), false);
    let result: Result<Vec<_>> = parser.collect(); // MODIFIED
    assert!(result.is_err());
    // Further checks can be added to ensure the error is due to section order
}

#[test]
fn test_section_reader_random_access() -> Result<()> {
    let module_bytes = create_test_module();
    let reader = SectionReader::new(&module_bytes)?;

    // Try to read type section (ID 1)
    let type_section_data = reader.get_section_data(1)?;
    assert!(type_section_data.is_some(), "Type section should be found");

    // Try to read a non-existent section (e.g., ID 15)
    let non_existent_section_data = reader.get_section_data(15)?;
    assert!(non_existent_section_data.is_none(), "Section 15 should not exist");

    Ok(())
}

#[test]
fn test_non_existent_section() {
    let module_bytes = create_wasm_header(); // Only header, no sections
    let reader = SectionReader::new(&module_bytes).unwrap();
    let section_data = reader.get_section_data(1).unwrap(); // Try to get Type section
    assert!(section_data.is_none());
}

#[test]
fn test_empty_import_section() {
    let mut module_bytes = create_wasm_header();
    // Import section ID (2), size 0
    module_bytes.extend_from_slice(&[0x02, 0x00]);
    let mut parser = Parser::new(Some(&module_bytes), false);
    let limited_parser = parser.take(1); // Limit to 1 payload (ImportSection)
    let result = limited_parser.collect::<Result<Vec<_>>>(); // MODIFIED
    assert!(result.is_ok());
    let payloads = result.unwrap();
    assert_eq!(payloads.len(), 1);
    match &payloads[0] {
        Payload::ImportSection(data, _) => {
            assert!(
                data.data().unwrap().is_empty(),
                "Import section data should be empty for size 0"
            );
            let imports = parse_import_section(data.clone(), 0, 0).unwrap();
            assert!(imports.is_empty(), "Parsed imports should be empty for empty section");
        }
        _ => panic!("Expected ImportSection payload"),
    }
}
