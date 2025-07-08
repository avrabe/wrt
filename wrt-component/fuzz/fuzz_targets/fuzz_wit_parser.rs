#![no_main]

use libfuzzer_sys::fuzz_target;
use wrt_format::wit_parser::WitParser;

fuzz_target!(|data: &[u8]| {
    // Convert raw bytes to string
    if let Ok(input) = std::str::from_utf8(data) {
        // Create parser
        let mut parser = WitParser::new();
        
        // Try to parse as WIT - we expect this to either succeed or fail gracefully
        let _ = parser.parse(input);
        
        // Also try parsing individual WIT constructs
        let _ = parser.parse_world(input);
        let _ = parser.parse_interface(input);
        
        // Test edge cases with specific patterns
        if input.contains("interfaceMissing message") {
            let _ = parser.parse_interface(input);
        }
        
        if input.contains("worldMissing message") {
            let _ = parser.parse_world(input);
        }
        
        if input.contains("recordMissing message") || input.contains("variantMissing message") || input.contains("enumMissing message") {
            let _ = parser.parse_type_def(input);
        }
        
        if input.contains("funcMissing message") {
            let _ = parser.parse_function(input);
        }
    }
});