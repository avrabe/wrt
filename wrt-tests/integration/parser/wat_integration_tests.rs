//! WAT (WebAssembly Text) Integration Tests
//!
//! This module consolidates WAT parsing and integration tests from across the WRT project.

#![cfg(test)]

use wrt_decoder::Parser;
use wrt_error::Result;

// ===========================================
// WAT PARSING UTILITIES
// ===========================================

/// Convert WAT text to WASM binary for testing
pub fn wat_to_wasm(wat: &str) -> Result<Vec<u8>> {
    wat::parse_str(wat).map_err(|e| {
        wrt_error::Error::runtime_execution_error(", e),
        )
    })
}

/// Create a simple WAT module for testing
pub fn create_simple_wat_module() -> &'static str {
    r#") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add
        )
    )"#
}

/// Create a WAT module with imports
pub fn create_wat_module_with_imports() -> &'static str {
    r#"(module
        (import "wasi_builtin" "resource.create" (func $resource_create (param i32) (result i32)))
        (import "wasi_builtin" "resource.drop" (func $resource_drop (param i32)))
        
        (func (export "test_resource") (param i32) (result i32)
            local.get 0
            call $resource_create
        )
    )"#
}

/// Create a WAT module with memory operations
pub fn create_wat_module_with_memory() -> &'static str {
    r#"(module
        (memory 1)
        
        (func (export "store_value") (param i32 i32)
            local.get 0
            local.get 1
            i32.store
        )
        
        (func (export "load_value") (param i32) (result i32)
            local.get 0
            i32.load
        )
    )"#
}

/// Create a complex WAT module for comprehensive testing
pub fn create_complex_wat_module() -> &'static str {
    r#"(module
        (import "wasi_builtin" "random" (func $random (result i32)))
        
        (memory 1)
        (table 10 funcref)
        
        (global $counter (mut i32) (i32.const 0))
        
        (func $internal_helper (param i32) (result i32)
            local.get 0
            i32.const 1
            i32.add
        )
        
        (func (export "main") (result i32)
            call $random
            call $internal_helper
            
            global.get $counter
            i32.const 1
            i32.add
            global.set $counter
        )
        
        (func (export "get_counter") (result i32)
            global.get $counter
        )
        
        (start $internal_helper)
    )"#
}

// ===========================================
// BASIC WAT TESTS
// ===========================================

mod basic_wat_tests {
    use super::*;
    use wrt_component::parser;

    #[test]
    fn test_simple_wat_parsing() {
        let wat = create_simple_wat_module);
        let wasm = wat_to_wasm(wat).unwrap();
        
        // Test that we can parse the generated WASM
        let mut parser = Parser::new(&wasm;
        let result = parser.parse);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wat_with_imports_parsing() {
        let wat = create_wat_module_with_imports);
        let wasm = wat_to_wasm(wat).unwrap();
        
        // Test that builtin scanning works with WAT-generated modules
        let builtins = parser::scan_for_builtins(&wasm).unwrap();
        assert_eq!(builtins.len(), 2;
        assert!(builtins.contains(&"resource.create".to_string());
        assert!(builtins.contains(&"resource.drop".to_string());
    }

    #[test]
    fn test_wat_with_memory_parsing() {
        let wat = create_wat_module_with_memory);
        let wasm = wat_to_wasm(wat).unwrap();
        
        // Test that memory sections are parsed correctly
        let mut parser = Parser::new(&wasm;
        let mut found_memory = false;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let wrt_decoder::Payload::MemorySection(_) = payload {
                        found_memory = true;
                    } else if let wrt_decoder::Payload::End = payload {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        assert!(found_memory);
    }

    #[test]
    fn test_complex_wat_parsing() {
        let wat = create_complex_wat_module);
        let wasm = wat_to_wasm(wat).unwrap();
        
        // Test that all sections are parsed correctly
        let mut parser = Parser::new(&wasm;
        let mut sections_found = std::collections::HashSet::new();
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    match payload {
                        wrt_decoder::Payload::ImportSection(_) => {
                            sections_found.insert("import";
                        }
                        wrt_decoder::Payload::MemorySection(_) => {
                            sections_found.insert("memory";
                        }
                        wrt_decoder::Payload::TableSection(_) => {
                            sections_found.insert("table";
                        }
                        wrt_decoder::Payload::GlobalSection(_) => {
                            sections_found.insert("global";
                        }
                        wrt_decoder::Payload::ExportSection(_) => {
                            sections_found.insert("export";
                        }
                        wrt_decoder::Payload::StartSection { .. } => {
                            sections_found.insert("start";
                        }
                        wrt_decoder::Payload::End => break,
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
        
        // Verify that all expected sections were found
        assert!(sections_found.contains("import");
        assert!(sections_found.contains("memory");
        assert!(sections_found.contains("table");
        assert!(sections_found.contains("global");
        assert!(sections_found.contains("export");
    }
}

// ===========================================
// WAT ERROR HANDLING TESTS
// ===========================================

mod wat_error_tests {
    use super::*;

    #[test]
    fn test_invalid_wat_syntax() {
        let invalid_wat = r#"(module
            (func (export "bad_syntax" (param i32)
                local.get 0
                i32.invalid_instruction
            )
        )"#;
        
        let result = wat_to_wasm(invalid_wat;
        assert!(result.is_err();
    }

    #[test]
    fn test_wat_type_mismatch() {
        let invalid_wat = r#"(module
            (func (export "type_error") (result i32)
                i64.const 42
            )
        )"#;
        
        let result = wat_to_wasm(invalid_wat;
        assert!(result.is_err();
    }

    #[test]
    fn test_wat_undefined_import() {
        let wat_with_undefined = r#"(module
            (import "undefined_module" "undefined_function" (func))
            (func (export "test")
                call 0
            )
        )"#;
        
        // WAT parsing should succeed, but the module should indicate the import
        let wasm = wat_to_wasm(wat_with_undefined).unwrap();
        
        let mut parser = Parser::new(&wasm;
        let mut found_import = false;
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let wrt_decoder::Payload::ImportSection(reader) = payload {
                        for import in reader {
                            let import = import.unwrap();
                            if import.module == "undefined_module" {
                                found_import = true;
                            }
                        }
                    } else if let wrt_decoder::Payload::End = payload {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        assert!(found_import);
    }
}

// ===========================================
// WAT INTEGRATION TESTS
// ===========================================

mod wat_integration_tests {
    use super::*;
    use wrt_component::parser;

    #[test]
    fn test_wat_to_builtin_detection() {
        // Test that WAT modules with WASI builtins are detected correctly
        let wat = r#"(module
            (import "wasi_builtin" "resource.create" (func $create (param i32) (result i32)))
            (import "wasi_builtin" "resource.drop" (func $drop (param i32)))
            (import "other_module" "other_func" (func $other))
            
            (func (export "test") (param i32) (result i32)
                local.get 0
                call $create
            )
        )"#;
        
        let wasm = wat_to_wasm(wat).unwrap();
        let builtins = parser::scan_for_builtins(&wasm).unwrap();
        
        // Should only detect WASI builtin imports, not other imports
        assert_eq!(builtins.len(), 2;
        assert!(builtins.contains(&"resource.create".to_string());
        assert!(builtins.contains(&"resource.drop".to_string());
    }

    #[test]
    fn test_wat_export_parsing() {
        let wat = r#"(module
            (func (export "add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
            
            (func (export "sub") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.sub
            )
            
            (memory (export "mem") 1)
            (global (export "counter") (mut i32) (i32.const 0))
        )"#;
        
        let wasm = wat_to_wasm(wat).unwrap();
        
        let mut parser = Parser::new(&wasm;
        let mut exports_found = Vec::new();
        
        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let wrt_decoder::Payload::ExportSection(reader) = payload {
                        for export in reader {
                            let export = export.unwrap();
                            exports_found.push(export.name.to_string());
                        }
                    } else if let wrt_decoder::Payload::End = payload {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        assert_eq!(exports_found.len(), 4;
        assert!(exports_found.contains(&"add".to_string());
        assert!(exports_found.contains(&"sub".to_string());
        assert!(exports_found.contains(&"mem".to_string());
        assert!(exports_found.contains(&"counter".to_string());
    }

    #[test]
    fn test_wat_cross_crate_compatibility() {
        // Test that WAT modules work consistently across different parser implementations
        let wat = create_wat_module_with_imports);
        let wasm = wat_to_wasm(wat).unwrap();
        
        // Test with wrt-component parser
        let component_builtins = parser::scan_for_builtins(&wasm).unwrap();
        
        // Test with wrt-decoder parser
        let mut decoder_parser = Parser::new(&wasm;
        let mut decoder_import_count = 0;
        
        loop {
            match decoder_parser.parse() {
                Ok(payload) => {
                    if let wrt_decoder::Payload::ImportSection(reader) = payload {
                        for _import in reader {
                            decoder_import_count += 1;
                        }
                    } else if let wrt_decoder::Payload::End = payload {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        // Both should detect the same number of imports
        assert_eq!(component_builtins.len(), 2;
        assert_eq!(decoder_import_count, 2;
    }
}

// ===========================================
// WAT PERFORMANCE TESTS
// ===========================================

mod wat_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_wat_parsing_performance() {
        let wat = create_complex_wat_module);
        
        let start = Instant::now);
        
        for _ in 0..100 {
            let _wasm = wat_to_wasm(wat).unwrap();
        }
        
        let duration = start.elapsed);
        
        // WAT parsing should be reasonable fast
        assert!(duration.as_secs() < 1, "WAT parsing performance regression");
    }

    #[test]
    fn test_large_wat_module_parsing() {
        // Generate a large WAT module
        let mut wat = String::from("(module\n";
        
        // Add many functions
        for i in 0..100 {
            wat.push_str(&format!(
                "  (func (export \"func_{}\") (param i32) (result i32)\n    local.get 0\n    i32.const {}\n    i32.add\n  )\n",
                i, i
            ;
        }
        
        wat.push_str(")";
        
        let start = Instant::now);
        let wasm = wat_to_wasm(&wat).unwrap();
        let wat_duration = start.elapsed);
        
        let start = Instant::now);
        let mut parser = Parser::new(&wasm;
        loop {
            match parser.parse() {
                Ok(wrt_decoder::Payload::End) => break,
                Err(_) => break,
                _ => {}
            }
        }
        let parse_duration = start.elapsed);
        
        assert!(wat_duration.as_millis() < 500, "Large WAT parsing too slow");
        assert!(parse_duration.as_millis() < 100, "Large WASM parsing too slow");
    }
}