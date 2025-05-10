//! Test no_std compatibility for wrt-format
//!
//! This file validates that the wrt-format crate works correctly in no_std environments.

// For testing in a no_std environment
#![cfg_attr(not(feature = "std"), no_std)]

// External crate imports
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(test)]
mod tests {
    // Import necessary types for no_std environment
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::{format, string::String, vec, vec::Vec};

    #[cfg(feature = "std")]
    use std::{vec, vec::Vec};

    // Import from wrt-format
    use wrt_format::{
        binary::{
            read_leb128_u32, read_string, write_leb128_u32, write_string, WASM_MAGIC, WASM_VERSION,
        },
        section::{CustomSection, CUSTOM_ID, FUNCTION_ID, IMPORT_ID, TYPE_ID},
        types::{FormatBlockType, Limits},
    };

    // Import from wrt-types for ValueType and ValType
    use wrt_types::{component_value::ValType, ValueType};

    #[test]
    fn test_binary_constants() {
        assert_eq!(WASM_MAGIC, [0x00, 0x61, 0x73, 0x6D]);
        assert_eq!(WASM_VERSION, [0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_leb128_encoding() {
        // Test encoding u32
        let encoded = write_leb128_u32(624485);

        // Read u32 from position 0
        let (value, read) = read_leb128_u32(&encoded, 0).unwrap();

        // Verify
        assert_eq!(value, 624485);
        assert_eq!(read, encoded.len());
    }

    #[test]
    fn test_string_encoding() {
        // Test encoding string
        let test_string = "test_string";
        let encoded = write_string(test_string);

        // Read string from position 0
        let (string, read) = read_string(&encoded, 0).unwrap();

        // Verify
        assert_eq!(string, test_string);
        assert_eq!(read, encoded.len());
    }

    #[test]
    fn test_section_ids() {
        // Test section ID constants
        assert_eq!(CUSTOM_ID, 0);
        assert_eq!(TYPE_ID, 1);
        assert_eq!(IMPORT_ID, 2);
        assert_eq!(FUNCTION_ID, 3);
    }

    #[test]
    fn test_custom_section() {
        // Test custom section
        let name = "test_section";
        let data = vec![1, 2, 3, 4];
        let section = CustomSection { name: name.to_string(), data: data.clone() };

        assert_eq!(section.name, name);
        assert_eq!(section.data, data);
    }

    #[test]
    fn test_limits() {
        // Test limits
        let limits = Limits { min: 1, max: Some(2), memory64: false, shared: false };

        assert_eq!(limits.min, 1);
        assert_eq!(limits.max, Some(2));
        assert_eq!(limits.memory64, false);
        assert_eq!(limits.shared, false);
    }

    #[test]
    fn test_value_types() {
        // Test ValueType enum from wrt-types
        assert_ne!(ValueType::I32, ValueType::I64);
        assert_ne!(ValueType::F32, ValueType::F64);

        // Test component ValType enum
        assert_ne!(ValType::S32, ValType::S64);
        assert_ne!(ValType::F32, ValType::F64);
    }

    #[test]
    fn test_format_block_type() {
        // Test block types
        let block_empty = FormatBlockType::Empty;
        let block_value = FormatBlockType::ValueType(ValueType::I32);

        assert_ne!(block_empty, block_value);
    }

    #[cfg(feature = "safety")]
    #[test]
    fn test_safe_memory_operations() {
        // Test safe memory operations
        use wrt_format::prelude::{memory_provider, safe_slice};

        // Create a sample buffer
        let buffer = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        // Create safe slice
        let safe_buffer = safe_slice(&buffer);

        // Verify first 4 bytes (WASM_MAGIC)
        assert_eq!(&buffer[0..4], &WASM_MAGIC);
        assert_eq!(safe_buffer.range(0, 4).unwrap(), &WASM_MAGIC);

        // Test memory provider
        let provider = memory_provider(buffer.clone());
        let provider_slice = wrt_types::safe_memory::MemoryProvider::borrow_slice(
            &provider,
            0,
            wrt_types::safe_memory::MemoryProvider::size(&provider),
        )
        .unwrap();

        // Verify first 4 bytes (WASM_MAGIC)
        assert_eq!(provider_slice.range(0, 4).unwrap(), &WASM_MAGIC);
    }
}
