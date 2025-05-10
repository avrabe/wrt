//! Decoder integration for wrt
//!
//! This module provides the integration layer between wrt-decoder and wrt-runtime,
//! handling the safe and efficient decoding of WebAssembly modules.
//!
//! The implementation delegates to the specialized crates (wrt-decoder for decoding,
//! wrt-runtime for module building) to avoid code duplication and maintain clean
//! separation of concerns.

use crate::prelude::*;

// Re-export the module loading functionality from wrt-runtime
pub use wrt_runtime::module_builder::load_module_from_binary;

// Re-export useful decoder functions for convenience
pub use wrt_decoder::{
    from_binary, parse,
    runtime_adapter::{convert_to_runtime_module, RuntimeModuleBuilder},
    validate,
};

/// Load a module from a binary buffer
///
/// This is a convenience function that delegates to wrt-runtime's module builder,
/// which implements the RuntimeModuleBuilder trait from wrt-decoder.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to load
///
/// # Returns
///
/// A Result containing the runtime module or an error
pub fn load_module(binary: &[u8]) -> Result<Module> {
    load_module_from_binary(binary)
}

/// Decode and validate a WebAssembly binary module
///
/// This function decodes a WebAssembly binary module and validates it
/// according to the WebAssembly specification, ensuring it can be safely
/// instantiated by the runtime.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to decode and validate
///
/// # Returns
///
/// A Result containing whether the validation was successful
pub fn decode_and_validate(binary: &[u8]) -> Result<()> {
    // First decode the module
    let module = from_binary(binary)?;

    // Then validate it
    validate(&module)
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
        let result = decode_and_validate(&binary);
        assert!(result.is_ok());

        // Load the module into the runtime
        let result = load_module(&binary);
        assert!(result.is_ok());
    }
}
