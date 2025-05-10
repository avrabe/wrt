use wrt_decoder::find_section;
use wrt_decoder::{Parser, Payload};

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

// Tests for parser verification
#[test]
fn test_parser_finds_module_version() {
    let module = create_minimal_module();
    let parser = Parser::new(&module);

    let mut found_version = false;

    for payload_result in parser {
        if let Ok(Payload::Version(version)) = payload_result {
            found_version = true;
            assert_eq!(version, 1);
            break;
        }
    }

    assert!(found_version, "Failed to find module version");
}

#[test]
fn test_section_finding() {
    let module = create_minimal_module();

    // Test finding the import section (ID 2)
    let section_result = find_section(&module, 2);
    assert!(section_result.is_ok(), "Error finding section: {:?}", section_result.err());
    let section = section_result.unwrap();

    assert!(section.is_some(), "Failed to find import section");
}

#[test]
fn test_scanning_for_builtins() {
    let module = create_minimal_module();

    // Test scanning for builtins
    let builtin_result = scan_for_builtins(&module);
    assert!(
        builtin_result.is_ok(),
        "Error scanning for builtins: {}",
        builtin_result.err().unwrap()
    );

    let builtins = builtin_result.unwrap();
    assert_eq!(builtins.len(), 1, "Expected 1 builtin, found: {}", builtins.len());
    assert_eq!(builtins[0], "random", "Expected 'random' builtin, found: {}", builtins[0]);
}

#[test]
fn test_payloads() {
    let module = create_minimal_module();
    let parser = Parser::new(&module);

    // Test iterating through all payloads
    let mut count = 0;
    let mut found_import_section = false;

    for payload_result in parser {
        let payload = payload_result.unwrap();
        count += 1;

        match payload {
            Payload::ImportSection(_, _) => {
                found_import_section = true;
            }
            _ => {}
        }
    }

    assert!(count >= 2, "Expected at least 2 payloads, found {}", count);
    assert!(found_import_section, "Failed to find import section payload");
}

#[test]
fn test_section_reader() {
    let module = create_minimal_module();

    // Find the import section
    let section_result = find_section(&module, 2);
    let (offset, size) = section_result.unwrap().unwrap();

    // Use the section reader to parse the import section
    let import_data = &module[offset..offset + size];
    assert_eq!(import_data[0], 0x01, "Expected 1 import, found {}", import_data[0]);
}

#[test]
fn test_performance() {
    let module = create_minimal_module();

    // Measure scanning performance
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let result = scan_for_builtins(&module);
        assert!(result.is_ok());
    }
    let duration = start.elapsed();

    // Check that scanning is reasonably fast
    assert!(duration.as_millis() < 1000, "Scanning took too long: {:?}", duration);
    println!("Scanning 1000 times took: {:?}", duration);
}
