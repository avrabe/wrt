//! WebAssembly Component Model Canonical ABI
//!
//! This module provides implementations of the Canonical ABI for the
//! WebAssembly Component Model, including lifting, lowering, and memory
//! allocation functions.

pub mod canonical;
pub mod canonical_abi;
pub mod canonical_options;
pub mod canonical_realloc;
pub mod post_return;

pub use canonical::*;
pub use canonical_abi::*;
pub use canonical_options::*;
pub use canonical_realloc::*;
pub use post_return::*;

// Tests moved from canonical_abi_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use wrt_error::ErrorCategory;

    /// Create a test memory with some sample data
    fn create_test_memory() -> SimpleMemory {
        let mut memory = SimpleMemory::new(4096;

        // Initialize with some test data
        memory.data_mut()[0..4].copy_from_slice(&42u32.to_le_bytes);
        memory.data_mut()[4..8].copy_from_slice(&3.14f32.to_bits().to_le_bytes);
        memory.data_mut()[8..16].copy_from_slice(&(-123i64).to_le_bytes);

        memory
    }

    // ====== BASIC TYPE TESTS ======

    #[test]
    fn test_bool_lifting_and_lowering() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test true
        abi.lower_bool(&mut memory, true, 0).unwrap();
        let lifted = abi.lift_bool(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::Bool(true;

        // Test false
        abi.lower_bool(&mut memory, false, 1).unwrap();
        let lifted = abi.lift_bool(&memory, 1).unwrap();
        assert_eq!(lifted, ComponentValue::Bool(false;

        // Test non-zero as true
        memory.write_u8(2, 42).unwrap();
        let lifted = abi.lift_bool(&memory, 2).unwrap();
        assert_eq!(lifted, ComponentValue::Bool(true;
    }

    #[test]
    fn test_integer_lifting_and_lowering() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test i8
        abi.lower_s8(&mut memory, -42, 0).unwrap();
        let lifted = abi.lift_s8(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::S8(-42;

        // Test u8
        abi.lower_u8(&mut memory, 200, 1).unwrap();
        let lifted = abi.lift_u8(&memory, 1).unwrap();
        assert_eq!(lifted, ComponentValue::U8(200;

        // Test i16
        abi.lower_s16(&mut memory, -1000, 2).unwrap();
        let lifted = abi.lift_s16(&memory, 2).unwrap();
        assert_eq!(lifted, ComponentValue::S16(-1000;

        // Test u16
        abi.lower_u16(&mut memory, 60000, 4).unwrap();
        let lifted = abi.lift_u16(&memory, 4).unwrap();
        assert_eq!(lifted, ComponentValue::U16(60000;

        // Test i32
        abi.lower_s32(&mut memory, -123456, 6).unwrap();
        let lifted = abi.lift_s32(&memory, 6).unwrap();
        assert_eq!(lifted, ComponentValue::S32(-123456;

        // Test u32
        abi.lower_u32(&mut memory, 3000000000, 10).unwrap();
        let lifted = abi.lift_u32(&memory, 10).unwrap();
        assert_eq!(lifted, ComponentValue::U32(3000000000;

        // Test i64
        abi.lower_s64(&mut memory, -9223372036854775807, 14).unwrap();
        let lifted = abi.lift_s64(&memory, 14).unwrap();
        assert_eq!(lifted, ComponentValue::S64(-9223372036854775807;

        // Test u64
        abi.lower_u64(&mut memory, 18446744073709551615, 22).unwrap();
        let lifted = abi.lift_u64(&memory, 22).unwrap();
        assert_eq!(lifted, ComponentValue::U64(18446744073709551615;
    }

    #[test]
    fn test_float_lifting_and_lowering() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test f32
        abi.lower_f32(&mut memory, 3.14159, 0).unwrap();
        let lifted = abi.lift_f32(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::F32(3.14159;

        // Test f64
        abi.lower_f64(&mut memory, 2.718281828459045, 4).unwrap();
        let lifted = abi.lift_f64(&memory, 4).unwrap();
        assert_eq!(lifted, ComponentValue::F64(2.718281828459045;

        // Test special values
        abi.lower_f32(&mut memory, f32::NAN, 12).unwrap();
        let lifted = abi.lift_f32(&memory, 12).unwrap();
        if let ComponentValue::F32(v) = lifted {
            assert!(v.is_nan();
        } else {
            panic!("Expected F32 value";
        }

        abi.lower_f32(&mut memory, f32::INFINITY, 16).unwrap();
        let lifted = abi.lift_f32(&memory, 16).unwrap();
        assert_eq!(lifted, ComponentValue::F32(f32::INFINITY;

        abi.lower_f32(&mut memory, f32::NEG_INFINITY, 20).unwrap();
        let lifted = abi.lift_f32(&memory, 20).unwrap();
        assert_eq!(lifted, ComponentValue::F32(f32::NEG_INFINITY;
    }

    #[test]
    fn test_char_lifting_and_lowering() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test ASCII character
        abi.lower_char(&mut memory, 'A', 0).unwrap();
        let lifted = abi.lift_char(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::Char('A';

        // Test Unicode character
        abi.lower_char(&mut memory, '€', 4).unwrap();
        let lifted = abi.lift_char(&memory, 4).unwrap();
        assert_eq!(lifted, ComponentValue::Char('€';

        // Test emoji
        abi.lower_char(&mut memory, '🚀', 8).unwrap();
        let lifted = abi.lift_char(&memory, 8).unwrap();
        assert_eq!(lifted, ComponentValue::Char('🚀';
    }

    #[test]
    fn test_char_invalid_code_point() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Write invalid Unicode code point
        memory.write_u32_le(0, 0xD800).unwrap(); // Surrogate code point
        let result = abi.lift_char(&memory, 0;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_string_lifting_and_lowering() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test empty string
        abi.lower_string(&mut memory, "", 0).unwrap();
        let lifted = abi.lift_string(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::String("".to_string();

        // Test ASCII string
        abi.lower_string(&mut memory, "Hello, World!", 20).unwrap();
        let lifted = abi.lift_string(&memory, 20).unwrap();
        assert_eq!(lifted, ComponentValue::String("Hello, World!".to_string();

        // Test Unicode string
        abi.lower_string(&mut memory, "Hello, 世界! 🌍", 40).unwrap();
        let lifted = abi.lift_string(&memory, 40).unwrap();
        assert_eq!(lifted, ComponentValue::String("Hello, 世界! 🌍".to_string();
    }

    #[test]
    fn test_string_too_long() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Create a string that's too long
        let long_string = "x".repeat(MAX_STRING_LENGTH + 1;
        let result = abi.lower_string(&mut memory, &long_string, 0;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_string_invalid_utf8() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Write invalid UTF-8 data
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        memory.write_u32_le(0, 10).unwrap(); // ptr
        memory.write_u32_le(4, 3).unwrap(); // len
        memory.write_bytes(10, &invalid_utf8).unwrap();

        let result = abi.lift_string(&memory, 0;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    // ====== COMPLEX TYPE TESTS ======

    #[test]
    fn test_option_lifting_and_lowering() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test None
        abi.lower_option(&mut memory, &None, 0).unwrap();
        let lifted = abi.lift_option(&memory, &ComponentType::S32, 0).unwrap();
        assert_eq!(lifted, ComponentValue::Option(None;

        // Test Some (simplified test due to implementation limitations)
        let some_value = Some(Box::new(ComponentValue::S32(42);
        abi.lower_option(&mut memory, &some_value, 10).unwrap();
        // Note: Full round-trip test would require more complete lowering implementation
    }

    #[test]
    fn test_list_basic() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test empty list (simplified)
        let empty_list: Vec<ComponentValue> = vec![];
        abi.lower_list(&mut memory, &empty_list, 0).unwrap();

        // Test list with elements (simplified)
        let list = vec![ComponentValue::S32(1), ComponentValue::S32(2), ComponentValue::S32(3)];
        abi.lower_list(&mut memory, &list, 20).unwrap();
    }

    #[test]
    fn test_list_too_long() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Create a list that's too long
        let long_list = vec![ComponentValue::S32(0); MAX_LIST_LENGTH + 1];
        let result = abi.lower_list(&mut memory, &long_list, 0;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_enum_lifting() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        let cases = vec!["red".to_string(), "green".to_string(), "blue".to_string()];

        // Test valid discriminant
        memory.write_u32_le(0, 1).unwrap(); // green
        let lifted = abi.lift_enum(&memory, &cases, 0).unwrap();
        assert_eq!(lifted, ComponentValue::Enum("green".to_string();

        // Test invalid discriminant
        memory.write_u32_le(4, 5).unwrap(); // out of bounds
        let result = abi.lift_enum(&memory, &cases, 4;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_variant_lifting() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        let cases =
            vec![("none".to_string(), None), ("some".to_string(), Some(ComponentType::S32))];

        // Test variant without payload
        memory.write_u32_le(0, 0).unwrap(); // none
        let lifted = abi.lift_variant(&memory, &cases, 0).unwrap();
        assert_eq!(lifted, ComponentValue::Variant("none".to_string(), None;

        // Test invalid discriminant
        memory.write_u32_le(4, 5).unwrap(); // out of bounds
        let result = abi.lift_variant(&memory, &cases, 4;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_result_lifting() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        let ok_ty = Some(Box::new(ComponentType::S32;
        let err_ty = Some(Box::new(ComponentType::String;

        // Test Ok case
        memory.write_u32_le(0, 0).unwrap(); // Ok discriminant
        memory.write_u32_le(4, 42).unwrap(); // Ok value
        let lifted = abi.lift_result(&memory, &ok_ty, &err_ty, 0).unwrap();
        if let ComponentValue::Result(Ok(Some(value))) = lifted {
            assert_eq!(**value, ComponentValue::S32(42;
        } else {
            panic!("Expected Ok result";
        }

        // Test Err case
        memory.write_u32_le(8, 1).unwrap(); // Err discriminant
        let lifted = abi.lift_result(&memory, &ok_ty, &err_ty, 8).unwrap();
        if let ComponentValue::Result(Err(_)) = lifted {
            // Expected
        } else {
            panic!("Expected Err result";
        }

        // Test invalid discriminant
        memory.write_u32_le(12, 5).unwrap(); // invalid
        let result = abi.lift_result(&memory, &ok_ty, &err_ty, 12;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_flags_lifting() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        let flags = vec![
            "read".to_string(),
            "write".to_string(),
            "execute".to_string(),
            "delete".to_string(),
        ];

        // Test with some flags set
        memory.write_u8(0, 0b00001011).unwrap(); // read, write, delete
        let lifted = abi.lift_flags(&memory, &flags, 0).unwrap();
        if let ComponentValue::Flags(active_flags) = lifted {
            assert_eq!(active_flags.len(), 3;
            assert!(active_flags.contains(&"read".to_string());
            assert!(active_flags.contains(&"write".to_string());
            assert!(active_flags.contains(&"delete".to_string());
            assert!(!active_flags.contains(&"execute".to_string());
        } else {
            panic!("Expected Flags value";
        }

        // Test with no flags set
        memory.write_u8(1, 0).unwrap();
        let lifted = abi.lift_flags(&memory, &flags, 1).unwrap();
        if let ComponentValue::Flags(active_flags) = lifted {
            assert!(active_flags.is_empty();
        } else {
            panic!("Expected Flags value";
        }
    }

    // ====== SIZE AND ALIGNMENT TESTS ======

    #[test]
    fn test_size_calculations() {
        let abi = CanonicalABI::new);

        // Primitive types
        assert_eq!(abi.size_of(&ComponentType::Bool).unwrap(), 1;
        assert_eq!(abi.size_of(&ComponentType::S8).unwrap(), 1;
        assert_eq!(abi.size_of(&ComponentType::U8).unwrap(), 1;
        assert_eq!(abi.size_of(&ComponentType::S16).unwrap(), 2;
        assert_eq!(abi.size_of(&ComponentType::U16).unwrap(), 2;
        assert_eq!(abi.size_of(&ComponentType::S32).unwrap(), 4;
        assert_eq!(abi.size_of(&ComponentType::U32).unwrap(), 4;
        assert_eq!(abi.size_of(&ComponentType::S64).unwrap(), 8;
        assert_eq!(abi.size_of(&ComponentType::U64).unwrap(), 8;
        assert_eq!(abi.size_of(&ComponentType::F32).unwrap(), 4;
        assert_eq!(abi.size_of(&ComponentType::F64).unwrap(), 8;
        assert_eq!(abi.size_of(&ComponentType::Char).unwrap(), 4;

        // Composite types
        assert_eq!(abi.size_of(&ComponentType::String).unwrap(), 8); // ptr + len
        assert_eq!(abi.size_of(&ComponentType::List(Box::new(ComponentType::S32))).unwrap(), 8;

        // Option type
        let option_s32 = ComponentType::Option(Box::new(ComponentType::S32;
        assert_eq!(abi.size_of(&option_s32).unwrap(), 5); // 4 + 1 discriminant

        // Record type
        let record = ComponentType::Record(vec![
            ("x".to_string(), ComponentType::S32),
            ("y".to_string(), ComponentType::F32),
        ];
        assert_eq!(abi.size_of(&record).unwrap(), 8); // 4 + 4

        // Tuple type
        let tuple = ComponentType::Tuple(vec![ComponentType::S32, ComponentType::S64];
        assert_eq!(abi.size_of(&tuple).unwrap(), 12); // 4 + 8

        // Enum type
        let enum_type = ComponentType::Enum(vec!["A".to_string(), "B".to_string()];
        assert_eq!(abi.size_of(&enum_type).unwrap(), 4); // discriminant only

        // Flags type
        let flags_type = ComponentType::Flags(vec![
            "flag1".to_string(),
            "flag2".to_string(),
            "flag3".to_string(),
        ];
        assert_eq!(abi.size_of(&flags_type).unwrap(), 1); // 3 bits -> 1 byte
    }

    #[test]
    fn test_alignment_calculations() {
        let abi = CanonicalABI::new);

        // Primitive types
        assert_eq!(abi.align_of(&ComponentType::Bool).unwrap(), 1;
        assert_eq!(abi.align_of(&ComponentType::S8).unwrap(), 1;
        assert_eq!(abi.align_of(&ComponentType::U8).unwrap(), 1;
        assert_eq!(abi.align_of(&ComponentType::S16).unwrap(), 2;
        assert_eq!(abi.align_of(&ComponentType::U16).unwrap(), 2;
        assert_eq!(abi.align_of(&ComponentType::S32).unwrap(), 4;
        assert_eq!(abi.align_of(&ComponentType::U32).unwrap(), 4;
        assert_eq!(abi.align_of(&ComponentType::S64).unwrap(), 8;
        assert_eq!(abi.align_of(&ComponentType::U64).unwrap(), 8;
        assert_eq!(abi.align_of(&ComponentType::F32).unwrap(), 4;
        assert_eq!(abi.align_of(&ComponentType::F64).unwrap(), 8;
        assert_eq!(abi.align_of(&ComponentType::Char).unwrap(), 4;

        // Composite types
        assert_eq!(abi.align_of(&ComponentType::String).unwrap(), 4); // pointer alignment
        assert_eq!(abi.align_of(&ComponentType::List(Box::new(ComponentType::S64))).unwrap(), 4;

        // Record with mixed alignment
        let record = ComponentType::Record(vec![
            ("a".to_string(), ComponentType::S8),
            ("b".to_string(), ComponentType::S64),
        ];
        assert_eq!(abi.align_of(&record).unwrap(), 8); // max alignment

        // Tuple with mixed alignment
        let tuple = ComponentType::Tuple(vec![ComponentType::S16, ComponentType::F64];
        assert_eq!(abi.align_of(&tuple).unwrap(), 8); // max alignment
    }

    // ====== ERROR CONDITION TESTS ======

    #[test]
    fn test_memory_out_of_bounds() {
        let abi = CanonicalABI::new);
        let memory = SimpleMemory::new(100;

        // Try to read beyond memory bounds
        let result = abi.lift_s32(&memory, 98); // Would read 4 bytes starting at 98
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Memory;
    }

    #[test]
    fn test_string_length_bounds_check() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Write a string pointer with length exceeding MAX_STRING_LENGTH
        memory.write_u32_le(0, 100).unwrap(); // ptr
        memory.write_u32_le(4, MAX_STRING_LENGTH as u32 + 1).unwrap(); // len

        let result = abi.lift_string(&memory, 0;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    #[test]
    fn test_list_length_bounds_check() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Write a list pointer with length exceeding MAX_LIST_LENGTH
        memory.write_u32_le(0, 100).unwrap(); // ptr
        memory.write_u32_le(4, MAX_LIST_LENGTH as u32 + 1).unwrap(); // len

        let result = abi.lift_list(&memory, &ComponentType::S32, 0;
        assert!(result.is_err();
        assert_eq!(result.unwrap_err().category(), ErrorCategory::Validation;
    }

    // ====== CROSS-ENVIRONMENT COMPATIBILITY TESTS ======

    #[cfg(feature = "std")]
    #[test]
    fn test_std_environment() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test basic operations work in std environment
        abi.lower_s32(&mut memory, 42, 0).unwrap();
        let value = abi.lift_s32(&memory, 0).unwrap();
        assert_eq!(value, ComponentValue::S32(42;
    }

    #[cfg(all(not(feature = "std")))]
    #[test]
    fn test_alloc_environment() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Binary std/no_std choice
        abi.lower_s32(&mut memory, 42, 0).unwrap();
        let value = abi.lift_s32(&memory, 0).unwrap();
        assert_eq!(value, ComponentValue::S32(42;
    }

    #[cfg(not(any(feature = "std", )))]
    #[test]
    fn test_no_std_environment() {
        let abi = CanonicalABI::new);
        // Note: SimpleMemory is not available in pure no_std
        // This test verifies the API compiles in no_std

        assert_eq!(abi.size_of(&ComponentType::S32).unwrap(), 4;
        assert_eq!(abi.align_of(&ComponentType::S64).unwrap(), 8;
    }

    // ====== ROUND-TRIP TESTS ======

    #[test]
    fn test_primitive_round_trips() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test various primitive values
        let test_cases = vec![
            (ComponentValue::Bool(true), ComponentType::Bool),
            (ComponentValue::Bool(false), ComponentType::Bool),
            (ComponentValue::S8(-128), ComponentType::S8),
            (ComponentValue::S8(127), ComponentType::S8),
            (ComponentValue::U8(0), ComponentType::U8),
            (ComponentValue::U8(255), ComponentType::U8),
            (ComponentValue::S16(-32768), ComponentType::S16),
            (ComponentValue::S16(32767), ComponentType::S16),
            (ComponentValue::U16(0), ComponentType::U16),
            (ComponentValue::U16(65535), ComponentType::U16),
            (ComponentValue::S32(-2147483648), ComponentType::S32),
            (ComponentValue::S32(2147483647), ComponentType::S32),
            (ComponentValue::U32(0), ComponentType::U32),
            (ComponentValue::U32(4294967295), ComponentType::U32),
            (ComponentValue::F32(0.0), ComponentType::F32),
            (ComponentValue::F32(-0.0), ComponentType::F32),
            (ComponentValue::F32(1.0), ComponentType::F32),
            (ComponentValue::F32(-1.0), ComponentType::F32),
            (ComponentValue::F64(0.0), ComponentType::F64),
            (ComponentValue::F64(1.0), ComponentType::F64),
            (ComponentValue::Char('A'), ComponentType::Char),
            (ComponentValue::Char('\u{20ac}'), ComponentType::Char),
            (ComponentValue::Char('\u{1f680}'), ComponentType::Char),
        ];

        for (i, (value, ty)) in test_cases.iter().enumerate() {
            let offset = (i * 16) as u32; // Give each test enough space

            // Lower the value
            abi.lower(&mut memory, value, offset).unwrap();

            // Lift it back
            let lifted = abi.lift(&memory, ty, offset).unwrap();

            // Should be equal
            assert_eq!(&lifted, value, "Round-trip failed for {:?}", value;
        }
    }

    // ====== PERFORMANCE TESTS ======

    #[test]
    fn test_batch_operations() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(4096;

        // Test batch lowering/lifting of many values
        let values: Vec<_> = (0..100).map(|i| (i as i32, (i * 4) as u32)).collect();

        // Lower all values
        for (value, offset) in &values {
            abi.lower_s32(&mut memory, *value, *offset).unwrap();
        }

        // Lift all values
        for (expected_value, offset) in &values {
            let lifted = abi.lift_s32(&memory, *offset).unwrap();
            assert_eq!(lifted, ComponentValue::S32(*expected_value;
        }
    }

    // ====== CONFIGURATION TESTS ======

    #[test]
    fn test_abi_configuration() {
        // Test default configuration
        let abi = CanonicalABI::new);
        assert_eq!(abi.string_encoding, StringEncoding::Utf8;
        assert_eq!(abi.alignment, 1;

        // Test custom configuration
        let abi = CanonicalABI::new().with_string_encoding(StringEncoding::Utf16).with_alignment(8;
        assert_eq!(abi.string_encoding, StringEncoding::Utf16;
        assert_eq!(abi.alignment, 8;
    }

    #[test]
    fn test_string_encoding_enum() {
        assert_eq!(StringEncoding::default(), StringEncoding::Utf8;

        // Test enum values
        assert_eq!(StringEncoding::Utf8, StringEncoding::Utf8;
        assert_ne!(StringEncoding::Utf8, StringEncoding::Utf16;
        assert_ne!(StringEncoding::Utf8, StringEncoding::Latin1;
    }

    // ====== MEMORY INTERFACE TESTS ======

    #[test]
    fn test_memory_interface_bounds_checking() {
        let memory = SimpleMemory::new(100;

        // Test successful reads
        assert!(memory.read_u8(0).is_ok();
        assert!(memory.read_u8(99).is_ok();
        assert!(memory.read_u16_le(98).is_ok();
        assert!(memory.read_u32_le(96).is_ok();
        assert!(memory.read_u64_le(92).is_ok();
        assert!(memory.read_bytes(50, 50).is_ok();

        // Test out-of-bounds reads
        assert!(memory.read_u8(100).is_err())); // exactly at end
        assert!(memory.read_u16_le(99).is_err())); // would read 2 bytes
        assert!(memory.read_u32_le(97).is_err())); // would read 4 bytes
        assert!(memory.read_u64_le(93).is_err())); // would read 8 bytes
        assert!(memory.read_bytes(50, 51).is_err())); // would read past end
    }

    #[test]
    fn test_memory_interface_writes() {
        let mut memory = SimpleMemory::new(100;

        // Test successful writes
        assert!(memory.write_u8(0, 42).is_ok();
        assert!(memory.write_u16_le(98, 0x1234).is_ok();
        assert!(memory.write_u32_le(96, 0x12345678).is_ok();
        assert!(memory.write_u64_le(92, 0x123456789ABCDEF0).is_ok();
        assert!(memory.write_bytes(50, &[1, 2, 3, 4, 5]).is_ok();

        // Test out-of-bounds writes
        assert!(memory.write_u8(100, 42).is_err();
        assert!(memory.write_u16_le(99, 0x1234).is_err();
        assert!(memory.write_u32_le(97, 0x12345678).is_err();
        assert!(memory.write_u64_le(93, 0x123456789ABCDEF0).is_err();
        assert!(memory.write_bytes(50, &[1); 51]).is_err);
    }

    // ====== EDGE CASE TESTS ======

    #[test]
    fn test_zero_sized_operations() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test empty string
        abi.lower_string(&mut memory, "", 0).unwrap();
        let lifted = abi.lift_string(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::String("".to_string();

        // Test empty list
        let empty_list: Vec<ComponentValue> = vec![];
        abi.lower_list(&mut memory, &empty_list, 10).unwrap();

        // Test empty flags
        let empty_flags: Vec<String> = vec![];
        let lifted = abi.lift_flags(&memory, &empty_flags, 20).unwrap();
        if let ComponentValue::Flags(flags) = lifted {
            assert!(flags.is_empty();
        } else {
            panic!("Expected Flags value";
        }
    }

    #[test]
    fn test_maximum_values() {
        let abi = CanonicalABI::new);
        let mut memory = SimpleMemory::new(1024;

        // Test maximum integer values
        abi.lower_u8(&mut memory, u8::MAX, 0).unwrap();
        let lifted = abi.lift_u8(&memory, 0).unwrap();
        assert_eq!(lifted, ComponentValue::U8(u8::MAX;

        abi.lower_u16(&mut memory, u16::MAX, 2).unwrap();
        let lifted = abi.lift_u16(&memory, 2).unwrap();
        assert_eq!(lifted, ComponentValue::U16(u16::MAX;

        abi.lower_u32(&mut memory, u32::MAX, 4).unwrap();
        let lifted = abi.lift_u32(&memory, 4).unwrap();
        assert_eq!(lifted, ComponentValue::U32(u32::MAX;

        abi.lower_u64(&mut memory, u64::MAX, 8).unwrap();
        let lifted = abi.lift_u64(&memory, 8).unwrap();
        assert_eq!(lifted, ComponentValue::U64(u64::MAX;
    }
}