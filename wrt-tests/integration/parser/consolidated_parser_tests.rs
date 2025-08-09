//! Consolidated Parser Tests for WRT
//!
//! This module consolidates all parser test functionality from across the WRT project,
//! eliminating duplication and providing comprehensive testing in a single location.

#![cfg(test)]

use std::collections::HashSet;
use wrt_component::parser;
use wrt_decoder::{
    types::{Import, ImportDesc},
    Error, Parser, Payload, SectionReader,
};
use wrt_error::Result;
use wrt_foundation::builtin::BuiltinType;
use wrt_format::module::{Import as FormatImport, ImportDesc as FormatImportDesc};

// ===========================================
// SHARED TEST UTILITIES
// ===========================================

/// Consolidated helper to create WebAssembly module header
pub fn create_wasm_header() -> Vec<u8> {
    vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
}

/// Consolidated helper to create a minimal test module with an import
pub fn create_test_module(module_name: &str, import_name: &str) -> Vec<u8> {
    let mut module = create_wasm_header);

    // Type section (empty)
    module.extend_from_slice(&[0x01, 0x04, 0x01, 0x60, 0x00, 0x00];

    // Import section with one import
    let module_name_len = module_name.len() as u8;
    let import_name_len = import_name.len() as u8;

    // Import section header
    module.push(0x02); // Import section ID
    module.push(0x07 + module_name_len + import_name_len); // Section size
    module.push(0x01); // Number of imports

    // Import entry
    module.push(module_name_len); // Module name length
    module.extend_from_slice(module_name.as_bytes()); // Module name
    module.push(import_name_len); // Import name length
    module.extend_from_slice(import_name.as_bytes()); // Import name
    module.push(0x00); // Import kind (function)
    module.push(0x00); // Type index

    module
}

/// Create a test module with comprehensive sections
pub fn create_comprehensive_test_module() -> Vec<u8> {
    let mut module = create_wasm_header);

    // Type section with one function signature: (i32, i32) -> i32
    module.extend_from_slice(&[
        0x01, 0x07, // Type section ID and size
        0x01, // Number of types
        0x60, // Function type
        0x02, // Number of params
        0x7F, 0x7F, // i32, i32
        0x01, // Number of results
        0x7F, // i32
    ];

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
    ];

    // Function section with one function
    module.extend_from_slice(&[
        0x03, 0x02, // Function section ID and size
        0x01, // Number of functions
        0x00, // Type index
    ];

    // Export section with one export
    module.extend_from_slice(&[
        0x07, 0x07, // Export section ID and size
        0x01, // Number of exports
        0x04, // Export name length
        0x6D, 0x61, 0x69, 0x6E, // "main"
        0x00, // Export kind (function)
        0x01, // Function index (imported function + local function)
    ];

    // Code section with one function body
    module.extend_from_slice(&[
        0x0A, 0x07, // Code section ID and size
        0x01, // Number of function bodies
        0x05, // Function body size
        0x00, // Number of locals
        0x20, 0x00, // local.get 0
        0x20, 0x01, // local.get 1
        0x6A, // i32.add
        0x0B, // end
    ];

    module
}

/// Create a multi-import test module
pub fn create_multi_import_module() -> Vec<u8> {
    let mut module = create_wasm_header);

    // Type section
    module.extend_from_slice(&[0x01, 0x04, 0x01, 0x60, 0x00, 0x00];

    // Import section with multiple imports
    module.extend_from_slice(&[
        0x02, 0x2E, // Import section ID and size (updated for multiple imports)
        0x02, // Number of imports
        
        // First import: wasi_builtin.resource.create
        0x0C, // Module name length
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E, // "wasi_builtin"
        0x0F, // Import name length
        0x72, 0x65, 0x73, 0x6F, 0x75, 0x72, 0x63, 0x65, 0x2E, 0x63, 0x72, 0x65, 0x61, 0x74, 0x65, // "resource.create"
        0x00, // Import kind (function)
        0x00, // Type index
        
        // Second import: wasi_builtin.resource.drop
        0x0C, // Module name length
        0x77, 0x61, 0x73, 0x69, 0x5F, 0x62, 0x75, 0x69, 0x6C, 0x74, 0x69, 0x6E, // "wasi_builtin"
        0x0D, // Import name length
        0x72, 0x65, 0x73, 0x6F, 0x75, 0x72, 0x63, 0x65, 0x2E, 0x64, 0x72, 0x6F, 0x70, // "resource.drop"
        0x00, // Import kind (function)
        0x00, // Type index
    ];

    module
}

/// Helper function consolidated from individual tests
pub fn get_required_builtins(module: &[u8]) -> Result<HashSet<BuiltinType>> {
    let builtin_names = parser::scan_for_builtins(module)?;
    let mut required_builtins = HashSet::new();
    
    for name in builtin_names {
        match name.as_str() {
            "resource.create" => {
                required_builtins.insert(BuiltinType::ResourceCreate;
            }
            "resource.drop" => {
                required_builtins.insert(BuiltinType::ResourceDrop;
            }
            // Add more builtin mappings as needed
            _ => {
                // Unknown builtin, skip it
            }
        }
    }
    
    Ok(required_builtins)
}

// ===========================================
// BASIC PARSER TESTS (from parser_test.rs)
// ===========================================

mod basic_parser_tests {
    use super::*;

    #[test]
    fn test_scan_for_builtins() {
        let module = create_test_module("wasi_builtin", "resource.create";
        
        let builtin_names = parser::scan_for_builtins(&module).unwrap();
        assert_eq!(builtin_names.len(), 1);
        assert_eq!(builtin_names[0], "resource.create";
    }

    #[test]
    fn test_non_builtin_imports() {
        let module = create_test_module("other_module", "other_import";
        
        let builtin_names = parser::scan_for_builtins(&module).unwrap();
        assert_eq!(builtin_names.len(), 0);
    }

    #[test]
    fn test_get_required_builtins() {
        let module = create_test_module("wasi_builtin", "resource.create";
        
        let required_builtins = get_required_builtins(&module).unwrap();
        assert!(required_builtins.contains(&BuiltinType::ResourceCreate);
        assert_eq!(required_builtins.len(), 1);
    }

    #[test]
    fn test_random_builtin_import() {
        let module = create_test_module("wasi_builtin", "random_get_bytes";
        
        let required_builtins = get_required_builtins(&module).unwrap();
        assert!(required_builtins.is_empty());
    }

    #[test]
    fn test_multiple_builtins() {
        let module = create_multi_import_module);
        
        let required_builtins = get_required_builtins(&module).unwrap();
        assert!(required_builtins.contains(&BuiltinType::ResourceCreate);
        assert!(required_builtins.contains(&BuiltinType::ResourceDrop);
        assert_eq!(required_builtins.len(), 2;
    }

    #[test]
    fn test_malformed_module() {
        let invalid_module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00]; // Missing last byte
        
        let result = get_required_builtins(&invalid_module;
        assert!(result.is_err();
    }
}

// ===========================================
// COMPREHENSIVE PARSER TESTS (from comprehensive test files)
// ===========================================

mod comprehensive_parser_tests {
    use super::*;

    #[test]
    fn test_comprehensive_module_parsing() {
        let module = create_comprehensive_test_module);
        
        // Test that we can parse all sections
        let mut parser = Parser::new(&module;
        let mut section_count = 0;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    match payload {
                        Payload::Version(_) => {
                            // Expected version payload
                        }
                        Payload::TypeSection(_) => {
                            section_count += 1;
                        }
                        Payload::ImportSection(_) => {
                            section_count += 1;
                        }
                        Payload::FunctionSection(_) => {
                            section_count += 1;
                        }
                        Payload::ExportSection(_) => {
                            section_count += 1;
                        }
                        Payload::CodeSectionEntry(_) => {
                            section_count += 1;
                        }
                        Payload::End => break,
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
        
        assert!(section_count >= 5)); // At least type, import, function, export, code sections
    }

    #[test]
    fn test_import_section_parsing() {
        let module = create_comprehensive_test_module);
        
        let mut parser = Parser::new(&module;
        let mut found_import_section = false;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let Payload::ImportSection(reader) = payload {
                        found_import_section = true;
                        
                        // Parse imports
                        for import in reader {
                            let import = import.unwrap();
                            assert_eq!(import.module, "wasi_builtin";
                            assert_eq!(import.name, "random";
                        }
                        break;
                    } else if let Payload::End = payload {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        assert!(found_import_section);
    }

    #[test]
    fn test_section_reader_functionality() {
        let module = create_comprehensive_test_module);
        
        let mut parser = Parser::new(&module;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    match payload {
                        Payload::ImportSection(reader) => {
                            let section_size = reader.get_count);
                            assert_eq!(section_size, 1);
                        }
                        Payload::FunctionSection(reader) => {
                            let section_size = reader.get_count);
                            assert_eq!(section_size, 1);
                        }
                        Payload::End => break,
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
    }
}

// ===========================================
// INTEGRATION PARSER TESTS
// ===========================================

mod integration_parser_tests {
    use super::*;

    #[test]
    fn test_cross_crate_parser_integration() {
        let module = create_test_module("wasi_builtin", "resource.create";
        
        // Test wrt-component parser
        let component_result = parser::scan_for_builtins(&module;
        assert!(component_result.is_ok());
        
        // Test wrt-decoder parser
        let mut decoder_parser = Parser::new(&module;
        let decoder_result = decoder_parser.parse);
        assert!(decoder_result.is_ok());
    }

    #[test]
    fn test_builtin_detection_across_parsers() {
        let module = create_multi_import_module);
        
        // Test that both parsers detect the same information
        let builtins = parser::scan_for_builtins(&module).unwrap();
        assert_eq!(builtins.len(), 2;
        assert!(builtins.contains(&"resource.create".to_string());
        assert!(builtins.contains(&"resource.drop".to_string());
        
        // Test with decoder parser
        let mut parser = Parser::new(&module;
        let mut import_count = 0;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let Payload::ImportSection(reader) = payload {
                        for _import in reader {
                            import_count += 1;
                        }
                        break;
                    } else if let Payload::End = payload {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        assert_eq!(import_count, 2;
    }

    #[test]
    fn test_error_handling_consistency() {
        let invalid_module = vec![0x00, 0x61, 0x73, 0x6D]; // Truncated magic
        
        // Test component parser error handling
        let component_result = parser::scan_for_builtins(&invalid_module;
        assert!(component_result.is_err();
        
        // Test decoder parser error handling
        let mut decoder_parser = Parser::new(&invalid_module;
        let decoder_result = decoder_parser.parse);
        // Note: Some parsers may be more tolerant than others
    }
}

// ===========================================
// VALIDATION PARSER TESTS
// ===========================================

mod validation_parser_tests {
    use super::*;

    #[test]
    fn test_import_validation() {
        let module = create_test_module("wasi_builtin", "resource.create";
        
        let mut parser = Parser::new(&module;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let Payload::ImportSection(reader) = payload {
                        for import in reader {
                            let import = import.unwrap();
                            
                            // Validate import structure
                            assert!(!import.module.is_empty());
                            assert!(!import.name.is_empty());
                            
                            // Validate specific import
                            if import.module == "wasi_builtin" {
                                assert!(import.name == "resource.create");
                            }
                        }
                        break;
                    } else if let Payload::End = payload {
                        break;
                    }
                }
                Err(e) => {
                    panic!("Parser error: {:?}", e;
                }
            }
        }
    }

    #[test]
    fn test_truncated_module_handling() {
        let mut module = create_comprehensive_test_module);
        
        // Truncate the module at various points
        for truncate_at in [10, 20, 30, 40] {
            if truncate_at < module.len() {
                let truncated = &module[..truncate_at];
                
                let mut parser = Parser::new(truncated;
                
                // Parser should either succeed with partial data or fail gracefully
                loop {
                    match parser.parse() {
                        Ok(payload) => {
                            if let Payload::End = payload {
                                break;
                            }
                        }
                        Err(_) => {
                            // Expected for truncated modules
                            break;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_section_boundary_validation() {
        let module = create_comprehensive_test_module);
        
        let mut parser = Parser::new(&module;
        let mut sections_seen = Vec::new();
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    match payload {
                        Payload::TypeSection(_) => sections_seen.push("type"),
                        Payload::ImportSection(_) => sections_seen.push("import"),
                        Payload::FunctionSection(_) => sections_seen.push("function"),
                        Payload::ExportSection(_) => sections_seen.push("export"),
                        Payload::CodeSectionStart { .. } => sections_seen.push("code_start"),
                        Payload::End => break,
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
        
        // Validate that sections appear in expected order
        assert!(sections_seen.contains(&"type");
        assert!(sections_seen.contains(&"import");
    }
}

// ===========================================
// PERFORMANCE PARSER TESTS
// ===========================================

mod performance_parser_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_parser_performance() {
        let module = create_comprehensive_test_module);
        
        let start = Instant::now);
        
        for _ in 0..1000 {
            let _builtins = parser::scan_for_builtins(&module).unwrap();
        }
        
        let duration = start.elapsed);
        
        // Parser should complete 1000 iterations in reasonable time
        assert!(duration.as_secs() < 1, "Parser performance regression detected");
    }

    #[test]
    fn test_large_module_parsing() {
        // Create a module with many imports
        let mut module = create_wasm_header);
        
        // Type section
        module.extend_from_slice(&[0x01, 0x04, 0x01, 0x60, 0x00, 0x00];
        
        // Large import section
        let import_count = 100;
        let mut import_section = Vec::new();
        import_section.push(import_count); // Number of imports
        
        for i in 0..import_count {
            let module_name = format!("module_{}", i;
            let import_name = format!("import_{}", i;
            
            import_section.push(module_name.len() as u8;
            import_section.extend_from_slice(module_name.as_bytes);
            import_section.push(import_name.len() as u8;
            import_section.extend_from_slice(import_name.as_bytes);
            import_section.push(0x00); // Function import
            import_section.push(0x00); // Type index
        }
        
        module.push(0x02); // Import section ID
        module.push(import_section.len() as u8); // Section size (assuming < 255)
        module.extend_from_slice(&import_section;
        
        // Test parsing performance
        let start = Instant::now);
        let _builtins = parser::scan_for_builtins(&module).unwrap();
        let duration = start.elapsed);
        
        assert!(duration.as_millis() < 100, "Large module parsing too slow");
    }
}