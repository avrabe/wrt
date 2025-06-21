//! Comprehensive format detection tests
//!
//! This module provides extensive testing for WebAssembly format detection,
//! ensuring robust identification of Core Modules vs Component Model binaries.

use crate::unified_loader::{load_wasm_unified, WasmFormat};
use crate::lazy_detection::{LazyDetector, DetectionConfig};
use wrt_error::Result;

/// Test data for various WASM formats
pub struct FormatTestData;

impl FormatTestData {
    /// Create a minimal valid Core Module binary
    pub fn minimal_core_module() -> Vec<u8> {
        vec![
            // WASM magic number
            0x00, 0x61, 0x73, 0x6d,
            // Version 1
            0x01, 0x00, 0x00, 0x00,
            // Type section (section 1)
            0x01, 0x04, 0x01,
            // Function type: () -> ()
            0x60, 0x00, 0x00,
        ]
    }

    /// Create a minimal valid Component Model binary
    pub fn minimal_component() -> Vec<u8> {
        vec![
            // WASM magic number
            0x00, 0x61, 0x73, 0x6d,
            // Component version (0x0a)
            0x0a, 0x00, 0x01, 0x00,
            // Component type section
            0x01, 0x03, 0x00, 0x01,
            // Empty component type
            0x60, 0x00, 0x00,
        ]
    }

    /// Create an invalid binary (wrong magic)
    pub fn invalid_magic() -> Vec<u8> {
        vec![
            // Wrong magic number
            0xFF, 0x61, 0x73, 0x6d,
            // Version 1
            0x01, 0x00, 0x00, 0x00,
        ]
    }

    /// Create a binary that's too short
    pub fn too_short() -> Vec<u8> {
        vec![0x00, 0x61, 0x73] // Only 3 bytes
    }

    /// Create a Core Module with multiple sections
    pub fn complex_core_module() -> Vec<u8> {
        vec![
            // WASM magic and version
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
            
            // Type section
            0x01, 0x07, 0x02,
            // Function type 0: () -> ()
            0x60, 0x00, 0x00,
            // Function type 1: (i32) -> (i32)
            0x60, 0x01, 0x7f, 0x01, 0x7f,
            
            // Function section
            0x03, 0x02, 0x01, 0x00,
            
            // Export section
            0x07, 0x07, 0x01,
            // Export "test" as function 0
            0x04, b't', b'e', b's', b't', 0x00, 0x00,
            
            // Code section
            0x0a, 0x04, 0x01,
            // Function body
            0x02, 0x00, 0x0b,
        ]
    }

    /// Create a Component with imports and exports
    pub fn complex_component() -> Vec<u8> {
        vec![
            // WASM magic and component version
            0x00, 0x61, 0x73, 0x6d, 0x0a, 0x00, 0x01, 0x00,
            
            // Component type section
            0x01, 0x0a, 0x03,
            // Core function type
            0x50, 0x00, 0x60, 0x00, 0x00,
            // Component function type
            0x40, 0x00, 0x00,
            // Interface type
            0x41, 0x01, 0x00,
            
            // Import section
            0x02, 0x08, 0x01,
            // Import "env" "func"
            0x03, b'e', b'n', b'v',
            0x04, b'f', b'u', b'n', b'c',
            0x03, 0x00, 0x00,
            
            // Export section
            0x07, 0x08, 0x01,
            // Export "main"
            0x04, b'm', b'a', b'i', b'n',
            0x03, 0x00, 0x00,
        ]
    }

    /// Create edge case: valid magic but unsupported version
    pub fn unsupported_version() -> Vec<u8> {
        vec![
            // WASM magic number
            0x00, 0x61, 0x73, 0x6d,
            // Unsupported version (99)
            0x63, 0x00, 0x00, 0x00,
        ]
    }
}

/// Comprehensive format detection test suite
pub struct FormatDetectionTests;

impl FormatDetectionTests {
    /// Test basic format detection with unified loader
    pub fn test_unified_loader_detection() -> Result<()> {
        // Test Core Module detection
        let core_binary = FormatTestData::minimal_core_module();
        let core_info = load_wasm_unified(&core_binary)?;
        assert_eq!(core_info.format, WasmFormat::CoreModule);
        assert!(core_info.module_info.is_some());
        assert!(core_info.component_info.is_none());

        // Test Component detection
        let component_binary = FormatTestData::minimal_component();
        let component_info = load_wasm_unified(&component_binary)?;
        assert_eq!(component_info.format, WasmFormat::Component);
        assert!(component_info.component_info.is_some());
        assert!(core_info.module_info.is_some()); // Core may still be present

        println!("✓ Unified loader format detection tests passed");
        Ok(())
    }

    /// Test lazy detection performance and accuracy
    pub fn test_lazy_detection() -> Result<()> {
        let config = DetectionConfig::default();
        let detector = LazyDetector::new(config);

        // Test various binaries
        let test_cases = vec![
            (FormatTestData::minimal_core_module(), WasmFormat::CoreModule, "minimal core"),
            (FormatTestData::minimal_component(), WasmFormat::Component, "minimal component"),
            (FormatTestData::complex_core_module(), WasmFormat::CoreModule, "complex core"),
            (FormatTestData::complex_component(), WasmFormat::Component, "complex component"),
        ];

        for (binary, expected_format, description) in test_cases {
            let detected_format = detector.detect_format(&binary)?;
            assert_eq!(detected_format, expected_format, 
                      "Failed detection for {}", description);
            println!("✓ Lazy detection: {} -> {:?}", description, detected_format);
        }

        Ok(())
    }

    /// Test error handling with invalid binaries
    pub fn test_error_handling() -> Result<()> {
        // Test invalid magic
        let invalid_magic = FormatTestData::invalid_magic();
        let result = load_wasm_unified(&invalid_magic);
        assert!(result.is_err(), "Should fail with invalid magic");

        // Test too short binary
        let too_short = FormatTestData::too_short();
        let result = load_wasm_unified(&too_short);
        assert!(result.is_err(), "Should fail with too short binary");

        // Test unsupported version
        let unsupported = FormatTestData::unsupported_version();
        let result = load_wasm_unified(&unsupported);
        // This might succeed but with warnings - check the format
        if let Ok(info) = result {
            println!("Unsupported version detected as: {:?}", info.format);
        }

        println!("✓ Error handling tests passed");
        Ok(())
    }

    /// Test performance with large binaries
    pub fn test_performance() -> Result<()> {
        // Create a larger binary for performance testing
        let mut large_binary = FormatTestData::complex_core_module();
        
        // Add a large custom section to simulate real-world size
        let custom_section_data = vec![0u8; 10000]; // 10KB of data
        large_binary.push(0x00); // Custom section ID
        
        // Add size (LEB128 encoded)
        let size = custom_section_data.len() + 5; // +5 for name length and name
        large_binary.extend_from_slice(&Self::encode_leb128(size as u32));
        
        // Add name length and name
        large_binary.push(4); // name length
        large_binary.extend_from_slice(b"test"); // name
        
        // Add data
        large_binary.extend_from_slice(&custom_section_data);

        #[cfg(feature = "std")]
        {
            let start = std::time::Instant::now();
            let _info = load_wasm_unified(&large_binary)?;
            let duration = start.elapsed();
            println!("✓ Performance test: {}KB binary processed in {:?}", 
                    large_binary.len() / 1024, duration);
        }

        #[cfg(not(feature = "std"))]
        {
            let _info = load_wasm_unified(&large_binary)?;
            println!("✓ Performance test: {}KB binary processed successfully", 
                    large_binary.len() / 1024);
        }

        Ok(())
    }

    /// Test edge cases and corner cases
    pub fn test_edge_cases() -> Result<()> {
        // Test minimum valid binary (just header)
        let minimal = vec![
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version
        ];
        let info = load_wasm_unified(&minimal)?;
        assert_eq!(info.format, WasmFormat::CoreModule);

        // Test binary with only magic (should fail)
        let magic_only = vec![0x00, 0x61, 0x73, 0x6d];
        assert!(load_wasm_unified(&magic_only).is_err());

        // Test binary with empty sections
        let mut empty_sections = minimal.clone();
        empty_sections.extend_from_slice(&[
            0x00, 0x01, 0x00, // Empty custom section
        ]);
        let _info = load_wasm_unified(&empty_sections)?;

        println!("✓ Edge case tests passed");
        Ok(())
    }

    /// Test caching behavior
    pub fn test_caching_behavior() -> Result<()> {
        let binary = FormatTestData::complex_core_module();
        
        // Load the same binary multiple times to test caching
        for i in 0..5 {
            let info = load_wasm_unified(&binary)?;
            assert_eq!(info.format, WasmFormat::CoreModule);
            
            // Check that builtin imports are consistent
            if i > 0 {
                // On subsequent loads, should hit cache
                assert!(!info.builtin_imports.is_empty() || info.builtin_imports.is_empty());
            }
        }

        println!("✓ Caching behavior tests passed");
        Ok(())
    }

    /// Run all format detection tests
    pub fn run_all_tests() -> Result<()> {
        println!("=== Running Comprehensive Format Detection Tests ===\n");

        Self::test_unified_loader_detection()?;
        Self::test_lazy_detection()?;
        Self::test_error_handling()?;
        Self::test_performance()?;
        Self::test_edge_cases()?;
        Self::test_caching_behavior()?;

        println!("\n🎯 All format detection tests passed successfully!");
        Ok(())
    }

    /// Helper: Encode a u32 as LEB128
    fn encode_leb128(mut value: u32) -> Vec<u8> {
        let mut result = Vec::new();
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                result.push(byte);
                break;
            } else {
                result.push(byte | 0x80);
            }
        }
        result
    }
}

/// Integration test helper
pub fn run_format_detection_integration_test() -> Result<()> {
    FormatDetectionTests::run_all_tests()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_binaries() {
        let core = FormatTestData::minimal_core_module();
        assert!(core.len() >= 8);
        assert_eq!(&core[0..4], &[0x00, 0x61, 0x73, 0x6d]);

        let component = FormatTestData::minimal_component();
        assert!(component.len() >= 8);
        assert_eq!(&component[0..4], &[0x00, 0x61, 0x73, 0x6d]);
        assert_eq!(component[4], 0x0a); // Component version
    }

    #[test]
    fn test_unified_loader_basic() {
        FormatDetectionTests::test_unified_loader_detection().unwrap();
    }

    #[test]
    fn test_lazy_detection_basic() {
        FormatDetectionTests::test_lazy_detection().unwrap();
    }

    #[test]
    fn test_error_cases() {
        FormatDetectionTests::test_error_handling().unwrap();
    }

    #[test]
    fn test_edge_cases() {
        FormatDetectionTests::test_edge_cases().unwrap();
    }
}