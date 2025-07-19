//! Decoder integration for wrt
//!
//! This module provides the integration layer between wrt-decoder and
//! wrt-runtime, handling the safe and efficient decoding of WebAssembly
//! modules.
//!
//! The implementation delegates to the specialized crates (wrt-decoder for
//! decoding, wrt-runtime for module building) to avoid code duplication and
//! maintain clean separation of concerns.

// Re-export useful decoder functions for convenience
pub use wrt_decoder::{
    from_binary, parse,
    runtime_adapter::{convert_to_runtime_module, RuntimeModuleBuilder},
    validate,
};
// Re-export the module loading functionality from wrt-runtime
// pub use wrt_runtime::module_builder::load_module_from_binary; // Temporarily disabled

use crate::prelude::*;

/// Load a module from a binary buffer
///
/// This function now uses the unified loader for efficient parsing and
/// automatic format detection.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to load
///
/// # Returns
///
/// A Result containing the runtime module or an error
pub fn load_module(binary: &[u8]) -> Result<Module> {
    use wrt_decoder::{load_wasm_unified, WasmFormat};
    
    // Use unified API to load and detect format
    let wasm_info = load_wasm_unified(binary)?;
    
    // Ensure this is a core module
    if !wasm_info.is_core_module() {
        return Err(Error::validation_type_mismatch("Binary is not a WebAssembly core module";
    }
    
    // Create module using runtime's load_from_binary which now uses unified API
    let mut dummy_module = Module::new()?;
    dummy_module.load_from_binary(binary)?
}

/// Decode and validate a WebAssembly binary module
///
/// This function uses the unified loader to efficiently decode and validate
/// a WebAssembly binary module according to the WebAssembly specification.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to decode and validate
///
/// # Returns
///
/// A Result containing whether the validation was successful
pub fn decode_and_validate(binary: &[u8]) -> Result<()> {
    use wrt_decoder::{load_wasm_unified, LazyDetector};
    
    // Use unified API for efficient loading and validation
    let wasm_info = load_wasm_unified(binary)?;
    
    // Basic validation is done by the unified loader
    // Additional validation can be performed here if needed
    match wasm_info.format_type {
        wrt_decoder::WasmFormat::CoreModule => {
            // Module-specific validation
            let module_info = wasm_info.require_module_info()?;
            
            // Validate memory constraints
            if let Some((min, max)) = module_info.memory_pages {
                if let Some(max_pages) = max {
                    if min > max_pages {
                        return Err(Error::validation_error("Memory minimum exceeds maximum";
                    }
                }
            }
            
            // Additional module validation can be added here
            Ok(())
        }
        wrt_decoder::WasmFormat::Component => {
            // Component-specific validation
            let _component_info = wasm_info.require_component_info()?;
            
            // Component validation can be added here
            Ok(())
        }
        wrt_decoder::WasmFormat::Unknown => {
            Err(Error::validation_error("Unknown or invalid WASM format"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn test_load_empty_module() {
        // Empty module as a binary
        let binary = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        // Load and validate the module
        let result = decode_and_validate(&binary;
        assert!(result.is_ok();

        // Load the module into the runtime
        let result = load_module(&binary;
        assert!(result.is_ok();
    }
}
