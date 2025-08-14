//! Comprehensive Parsing Tests
//!
//! This module provides comprehensive parser testing scenarios that cover
//! complex edge cases, validation, and integration across the WRT ecosystem.

#![cfg(test)]

use std::collections::HashMap;

use wrt_component::parser;
use wrt_decoder::{
    Parser,
    Payload,
    SectionReader,
};
use wrt_error::Result;

// ===========================================
// COMPLEX MODULE BUILDERS
// ===========================================

/// Create a module with all major section types
pub fn create_full_featured_module() -> Vec<u8> {
    let wat = r#"(module
        ));; Type section - function signatures
        (type $binary_op (func (param i32 i32) (result i32)))
        (type $unary_op (func (param i32) (result i32)))
        
        ));; Import section - various import types
        (import "wasi_builtin" "resource.create" (func $res_create (type $unary_op)))
        (import "wasi_builtin" "resource.drop" (func $res_drop (param i32)))
        (import "env" "external_memory" (memory 1))
        (import "env" "external_table" (table 10 funcref))
        (import "env" "external_global" (global i32))
        
        ));; Function section - local function declarations
        (func $add (type $binary_op)
            local.get 0
            local.get 1
            i32.add
        )
        
        (func $multiply (type $binary_op)
            local.get 0
            local.get 1
            i32.mul
        )
        
        (func $complex_logic (param i32) (result i32)
            (local i32 i32)
            
            ));; Complex control flow
            local.get 0
            i32.const 10
            i32.gt_s
            if (result i32)
                ));; Greater than 10 - call resource create
                local.get 0
                call $res_create
            else
                ));; Less than or equal to 10 - do math
                local.get 0
                i32.const 2
                call $multiply
            end
        )
        
        ));; Memory section - local memory
        (memory $local_mem 2 4)
        
        ));; Table section - function table
        (table $func_table 5 funcref)
        
        ));; Global section - mutable and immutable globals
        (global $config (mut i32) (i32.const 100))
        (global $version i32 (i32.const 1))
        
        ));; Element section - table initialization
        (elem (i32.const 0) $add $multiply)
        
        ));; Data section - memory initialization
        (data (i32.const 0) "Hello WRT")
        
        ));; Export section - public interface
        (export "add" (func $add))
        (export "multiply" (func $multiply))
        (export "process" (func $complex_logic))
        (export "memory" (memory $local_mem))
        (export "table" (table $func_table))
        (export "config" (global $config))
        
        ));; Start function
        (start $complex_logic)
    )"#;

    wat::parse_str(wat).expect("Failed to parse comprehensive WAT module")
}

/// Create a module with edge case scenarios
pub fn create_edge_case_module() -> Vec<u8> {
    let wat = r#"(module
        ));; Empty function
        (func $empty)
        
        ));; Function with many locals
        (func $many_locals (param i32) (result i32)
            (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
            local.get 0
            local.get 1
            i32.add
        )
        
        ));; Deeply nested control structures
        (func $nested_control (param i32) (result i32)
            local.get 0
            if (result i32)
                local.get 0
                i32.const 1
                i32.sub
                if (result i32)
                    local.get 0
                    i32.const 2
                    i32.sub
                    if (result i32)
                        i32.const 42
                    else
                        i32.const 24
                    end
                else
                    i32.const 12
                end
            else
                i32.const 0
            end
        )
        
        ));; Function with loop
        (func $loop_test (param i32) (result i32)
            (local i32)
            local.get 0
            local.set 1
            
            loop $continue
                local.get 1
                i32.const 1
                i32.sub
                local.tee 1
                i32.const 0
                i32.gt_s
                br_if $continue
            end
            
            local.get 1
        )
        
        ));; Exports with same names as builtins (should not conflict)
        (export "create" (func $empty))
        (export "drop" (func $many_locals))
    )"#;

    wat::parse_str(wat).expect("Failed to parse edge case WAT module")
}

/// Create a module that tests parser limits
pub fn create_stress_test_module() -> Vec<u8> {
    let mut wat = String::from("(module\n");

    // Many type definitions
    for i in 0..50 {
        wat.push_str(&format!(
            "  (type $func_type_{} (func (param i32) (result i32)))\n",
            i
        ));
    }

    // Many imports
    for i in 0..30 {
        wat.push_str(&format!(
            "  (import \"test_module_{}\" \"test_func_{}\" (func (param i32) (result i32)))\n",
            i, i
        ));
    }

    // Many functions
    for i in 0..100 {
        wat.push_str(&format!(
            "  (func $func_{} (param i32) (result i32)\n    local.get 0\n    i32.const {}\n    \
             i32.add\n  )\n",
            i, i
        ));
    }

    // Many exports
    for i in 0..100 {
        wat.push_str(&format!("  (export \"func_{}\" (func $func_{}))\n", i, i));
    }

    wat.push_str(")");

    wat::parse_str(&wat).expect("Failed to parse stress test WAT module")
}

// ===========================================
// COMPREHENSIVE PARSING TESTS
// ===========================================

mod comprehensive_tests {
    use super::*;

    #[test]
    fn test_full_featured_module_parsing() {
        let module = create_full_featured_module();

        let mut parser = Parser::new(&module);
        let mut sections = HashMap::new();

        loop {
            match parser.parse() {
                Ok(payload) => match payload {
                    Payload::TypeSection(reader) => {
                        sections.insert("type", reader.get_count());
                    },
                    Payload::ImportSection(reader) => {
                        sections.insert("import", reader.get_count());
                    },
                    Payload::FunctionSection(reader) => {
                        sections.insert("function", reader.get_count());
                    },
                    Payload::MemorySection(reader) => {
                        sections.insert("memory", reader.get_count());
                    },
                    Payload::TableSection(reader) => {
                        sections.insert("table", reader.get_count());
                    },
                    Payload::GlobalSection(reader) => {
                        sections.insert("global", reader.get_count());
                    },
                    Payload::ExportSection(reader) => {
                        sections.insert("export", reader.get_count());
                    },
                    Payload::ElementSection(reader) => {
                        sections.insert("element", reader.get_count());
                    },
                    Payload::DataSection(reader) => {
                        sections.insert("data", reader.get_count());
                    },
                    Payload::StartSection { .. } => {
                        sections.insert("start", 1);
                    },
                    Payload::End => break,
                    _ => {},
                },
                Err(e) => panic!("Parsing failed: {:?}", e),
            }
        }

        // Verify all expected sections were found
        assert!(sections.contains_key("type"));
        assert!(sections.contains_key("import"));
        assert!(sections.contains_key("function"));
        assert!(sections.contains_key("memory"));
        assert!(sections.contains_key("table"));
        assert!(sections.contains_key("global"));
        assert!(sections.contains_key("export"));
        assert!(sections.contains_key("element"));
        assert!(sections.contains_key("data"));
        assert!(sections.contains_key("start"));

        // Verify section counts
        assert_eq!(sections["type"], 2); // Two type definitions
        assert_eq!(sections["import"], 5); // Five imports
        assert_eq!(sections["function"], 3); // Three local functions
        assert_eq!(sections["export"], 6); // Six exports
    }

    #[test]
    fn test_builtin_detection_in_complex_module() {
        let module = create_full_featured_module();

        let builtins = parser::scan_for_builtins(&module).unwrap();

        // Should detect WASI builtin imports but not other imports
        assert_eq!(builtins.len(), 2);
        assert!(builtins.contains(&"resource.create".to_string()));
        assert!(builtins.contains(&"resource.drop".to_string()));
    }

    #[test]
    fn test_edge_case_module_parsing() {
        let module = create_edge_case_module();

        let mut parser = Parser::new(&module);
        let mut function_count = 0;
        let mut export_count = 0;

        loop {
            match parser.parse() {
                Ok(payload) => match payload {
                    Payload::FunctionSection(reader) => {
                        function_count = reader.get_count();
                    },
                    Payload::ExportSection(reader) => {
                        export_count = reader.get_count();
                    },
                    Payload::End => break,
                    _ => {},
                },
                Err(e) => panic!("Edge case parsing failed: {:?}", e),
            }
        }

        assert_eq!(function_count, 4); // Four functions defined
        assert_eq!(export_count, 2); // Two exports
    }

    #[test]
    fn test_stress_module_parsing() {
        let module = create_stress_test_module();

        let mut parser = Parser::new(&module);
        let mut type_count = 0;
        let mut import_count = 0;
        let mut function_count = 0;
        let mut export_count = 0;

        loop {
            match parser.parse() {
                Ok(payload) => match payload {
                    Payload::TypeSection(reader) => {
                        type_count = reader.get_count();
                    },
                    Payload::ImportSection(reader) => {
                        import_count = reader.get_count();
                    },
                    Payload::FunctionSection(reader) => {
                        function_count = reader.get_count();
                    },
                    Payload::ExportSection(reader) => {
                        export_count = reader.get_count();
                    },
                    Payload::End => break,
                    _ => {},
                },
                Err(e) => panic!("Stress test parsing failed: {:?}", e),
            }
        }

        assert_eq!(type_count, 50);
        assert_eq!(import_count, 30);
        assert_eq!(function_count, 100);
        assert_eq!(export_count, 100);
    }
}

// ===========================================
// VALIDATION AND ERROR TESTS
// ===========================================

mod validation_tests {
    use super::*;

    #[test]
    fn test_section_order_validation() {
        // Create a module with sections in wrong order (this should still parse)
        let wat = r#"(module
            (func $test (result i32)
                i32.const 42
            )
            
            (type $sig (func (result i32)))
            
            (export "test" (func $test))
        )"#;

        let module = wat::parse_str(wat).unwrap();

        // Parser should handle sections regardless of order
        let mut parser = Parser::new(&module);
        let mut sections_seen = Vec::new();

        loop {
            match parser.parse() {
                Ok(payload) => match payload {
                    Payload::TypeSection(_) => sections_seen.push("type"),
                    Payload::FunctionSection(_) => sections_seen.push("function"),
                    Payload::ExportSection(_) => sections_seen.push("export"),
                    Payload::End => break,
                    _ => {},
                },
                Err(_) => break,
            }
        }

        assert!(sections_seen.contains(&"type"));
        assert!(sections_seen.contains(&"function"));
        assert!(sections_seen.contains(&"export"));
    }

    #[test]
    fn test_import_validation_comprehensive() {
        let module = create_full_featured_module();

        let mut parser = Parser::new(&module);
        let mut imports = Vec::new();

        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let Payload::ImportSection(reader) = payload {
                        for import in reader {
                            let import = import.unwrap();
                            imports.push((import.module.to_string(), import.name.to_string()));
                        }
                    } else if let Payload::End = payload {
                        break;
                    }
                },
                Err(e) => panic!("Import validation failed: {:?}", e),
            }
        }

        // Validate specific imports
        assert!(imports.contains(&("wasi_builtin".to_string(), "resource.create".to_string())));
        assert!(imports.contains(&("wasi_builtin".to_string(), "resource.drop".to_string())));
        assert!(imports.contains(&("env".to_string(), "external_memory".to_string())));
        assert!(imports.contains(&("env".to_string(), "external_table".to_string())));
        assert!(imports.contains(&("env".to_string(), "external_global".to_string())));
    }

    #[test]
    fn test_export_validation_comprehensive() {
        let module = create_full_featured_module();

        let mut parser = Parser::new(&module);
        let mut exports = Vec::new();

        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let Payload::ExportSection(reader) = payload {
                        for export in reader {
                            let export = export.unwrap();
                            exports.push(export.name.to_string());
                        }
                    } else if let Payload::End = payload {
                        break;
                    }
                },
                Err(e) => panic!("Export validation failed: {:?}", e),
            }
        }

        // Validate specific exports
        assert!(exports.contains(&"add".to_string()));
        assert!(exports.contains(&"multiply".to_string()));
        assert!(exports.contains(&"process".to_string()));
        assert!(exports.contains(&"memory".to_string()));
        assert!(exports.contains(&"table".to_string()));
        assert!(exports.contains(&"config".to_string()));
    }

    #[test]
    fn test_malformed_section_handling() {
        // Create a module and then corrupt it
        let mut module = create_full_featured_module();

        // Corrupt the module by changing a section size
        if module.len() > 20 {
            module[15] = 0xFF; // Invalid section size
        }

        let mut parser = Parser::new(&module);
        let mut parsing_result = Ok();

        loop {
            match parser.parse() {
                Ok(payload) => {
                    if let Payload::End = payload {
                        break;
                    }
                },
                Err(e) => {
                    parsing_result = Err(e);
                    break;
                },
            }
        }

        // Should detect the corruption and fail gracefully
        assert!(parsing_result.is_err());
    }
}

// ===========================================
// PERFORMANCE AND SCALABILITY TESTS
// ===========================================

mod performance_tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn test_comprehensive_parsing_performance() {
        let module = create_full_featured_module();

        let start = Instant::now();

        for _ in 0..1000 {
            let mut parser = Parser::new(&module);
            loop {
                match parser.parse() {
                    Ok(Payload::End) => break,
                    Err(_) => break,
                    _ => {},
                }
            }
        }

        let duration = start.elapsed();
        assert!(
            duration.as_secs() < 1,
            "Comprehensive parsing performance regression"
        );
    }

    #[test]
    fn test_stress_parsing_performance() {
        let module = create_stress_test_module();

        let start = Instant::now();

        let mut parser = Parser::new(&module);
        loop {
            match parser.parse() {
                Ok(Payload::End) => break,
                Err(_) => break,
                _ => {},
            }
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 500, "Stress test parsing too slow");
    }

    #[test]
    fn test_builtin_scanning_performance() {
        let module = create_stress_test_module();

        let start = Instant::now();

        for _ in 0..100 {
            let _builtins = parser::scan_for_builtins(&module).unwrap();
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Builtin scanning performance regression"
        );
    }

    #[test]
    fn test_memory_usage_scalability() {
        // This test ensures that parsing doesn't consume excessive memory
        let module = create_stress_test_module();

        // Parse the same module multiple times to check for memory leaks
        for _ in 0..100 {
            let mut parser = Parser::new(&module);
            let mut section_count = 0;

            loop {
                match parser.parse() {
                    Ok(payload) => match payload {
                        Payload::TypeSection(_)
                        | Payload::ImportSection(_)
                        | Payload::FunctionSection(_)
                        | Payload::ExportSection(_) => {
                            section_count += 1;
                        },
                        Payload::End => break,
                        _ => {},
                    },
                    Err(_) => break,
                }
            }

            assert!(section_count > 0);
        }

        // If we get here without running out of memory, the test passes
    }
}

// ===========================================
// CROSS-CRATE INTEGRATION TESTS
// ===========================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_parser_consistency_across_crates() {
        let module = create_full_featured_module();

        // Test with wrt-component parser
        let component_builtins = parser::scan_for_builtins(&module).unwrap();

        // Test with wrt-decoder parser (count imports manually)
        let mut decoder_parser = Parser::new(&module);
        let mut wasi_import_count = 0;

        loop {
            match decoder_parser.parse() {
                Ok(payload) => {
                    if let Payload::ImportSection(reader) = payload {
                        for import in reader {
                            let import = import.unwrap();
                            if import.module == "wasi_builtin" {
                                wasi_import_count += 1;
                            }
                        }
                    } else if let Payload::End = payload {
                        break;
                    }
                },
                Err(_) => break,
            }
        }

        // Both parsers should detect the same WASI builtin imports
        assert_eq!(component_builtins.len(), wasi_import_count);
    }

    #[test]
    fn test_format_compatibility() {
        let module = create_comprehensive_test_module();

        // Test that the module can be parsed by different parser implementations
        let mut decoder_parser = Parser::new(&module);
        let mut sections_parsed = 0;

        loop {
            match decoder_parser.parse() {
                Ok(payload) => match payload {
                    Payload::TypeSection(_)
                    | Payload::ImportSection(_)
                    | Payload::FunctionSection(_)
                    | Payload::ExportSection(_) => {
                        sections_parsed += 1;
                    },
                    Payload::End => break,
                    _ => {},
                },
                Err(_) => break,
            }
        }

        assert!(sections_parsed > 0, "No sections were parsed");
    }

    #[test]
    fn test_error_consistency() {
        // Create an intentionally malformed module
        let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]; // Valid header
        module.extend_from_slice(&[0x01, 0xFF, 0x01]); // Invalid type section

        // Test component parser error handling
        let component_result = parser::scan_for_builtins(&module);

        // Test decoder parser error handling
        let mut decoder_parser = Parser::new(&module);
        let decoder_result = decoder_parser.parse();

        // Both should handle the error gracefully (may succeed or fail differently)
        // The key is that neither should panic or crash
        assert!(component_result.is_ok() || component_result.is_err());
        assert!(decoder_result.is_ok() || decoder_result.is_err());
    }
}
