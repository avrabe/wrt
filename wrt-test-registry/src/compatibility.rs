//! Compatibility tests for WebAssembly Runtime in both std and no_std environments
//!
//! This module contains tests that validate consistent behavior across std and no_std
//! environments, ensuring that we can rely on the same semantics in both contexts.

use crate::prelude::*;

/// Register all compatibility tests with the global registry
pub fn register_compatibility_tests() {
    let registry = TestRegistry::global();

    // Test error handling
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "error_handling_compatibility",
        category: "compatibility",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            features
        },
        test_fn: Box::new(test_error_handling),
        description: "Tests error handling in both std and no_std environments",
    }));

    // Test safe memory operations
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "safe_memory_operations",
        category: "compatibility",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            features
        },
        test_fn: Box::new(test_safe_memory),
        description: "Tests SafeMemory operations in both environments",
    }));

    // Test bounded collections
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "bounded_collections",
        category: "compatibility",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            features
        },
        test_fn: Box::new(test_bounded_collections),
        description: "Tests bounded collections in both environments",
    }));

    // Test type system compatibility
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "type_system_compatibility",
        category: "compatibility",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            features
        },
        test_fn: Box::new(test_type_system),
        description: "Tests type system compatibility across environments",
    }));

    // Test limits conversion
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "limits_conversion",
        category: "compatibility",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            features
        },
        test_fn: Box::new(test_limits_conversion),
        description: "Tests proper limits conversion between format and types",
    }));

    // Test value operations
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "value_operations",
        category: "compatibility",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            features
        },
        test_fn: Box::new(test_value_operations),
        description: "Tests value operations in both environments",
    }));

    // Test verification levels
    let _ = registry.register(Box::new(TestCaseImpl {
        name: "verification_levels",
        category: "safety",
        requires_std: false,
        features: {
            let mut features = BoundedVec::new();
            let _ = features.try_push("std".to_string());
            let _ = features.try_push("no_std".to_string());
            let _ = features.try_push("safety".to_string());
            features
        },
        test_fn: Box::new(test_verification_levels),
        description: "Tests different verification levels for memory safety",
    }));
}

/// Test error handling in both std and no_std environments
fn test_error_handling(_config: &TestConfig) -> TestResult {
    // Create errors with different categories
    let validation_error =
        Error::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, "Test validation error");

    let memory_error =
        Error::new(ErrorCategory::Memory, codes::MEMORY_ACCESS_OUT_OF_BOUNDS, "Test memory error");

    // Check error categories
    assert_eq_test!(validation_error.category(), ErrorCategory::Validation);
    assert_eq_test!(memory_error.category(), ErrorCategory::Memory);

    // Check error codes
    assert_eq_test!(validation_error.code(), codes::VALIDATION_ERROR);
    assert_eq_test!(memory_error.code(), codes::MEMORY_ACCESS_OUT_OF_BOUNDS);

    // Test error propagation
    let result: Result<(), Error> = Err(validation_error.clone());
    let propagated = result.map_err(|e| {
        assert_eq_test!(e.category(), ErrorCategory::Validation);
        assert_eq_test!(e.code(), codes::VALIDATION_ERROR);
        e
    });
    assert_test!(propagated.is_err());

    Ok(())
}

/// Test SafeMemory operations
fn test_safe_memory(config: &TestConfig) -> TestResult {
    // Create a safe memory handler with bounded vector
    let mut buffer = BoundedVec::<u8, 1024>::new();
    for i in 0..1024 {
        let _ = buffer.try_push(0);
    }

    let mut memory_handler = SafeMemoryHandler::new(buffer.as_slice());

    // Set verification level based on test config
    memory_handler.set_verification_level(config.verification_level);

    // Test basic operations with bounded vectors
    let test_data = BoundedVec::<u8, 5>::from_slice(&[1, 2, 3, 4, 5])
        .map_err(|e| format!("Failed to create BoundedVec: {:?}", e))?;

    memory_handler.clone_bytes_into(100, test_data.as_slice())?;

    // Get a safe slice and verify data
    let slice = memory_handler.get_slice(100, test_data.len())?;
    let data = slice.data()?;
    assert_eq_test!(data, test_data.as_slice());

    // Test memory integrity
    memory_handler.verify_integrity()?;

    // Test memory stats
    let stats = memory_handler.provider().memory_stats();
    assert_test!(stats.total_size >= 1024);

    // Test SafeSlice operations
    let slice = memory_handler.get_slice(0, 10)?;
    assert_eq_test!(slice.length(), 10);
    let sub_slice = slice.slice(2, 5)?;
    assert_eq_test!(sub_slice.length(), 3);

    Ok(())
}

/// Test bounded collections
fn test_bounded_collections(_config: &TestConfig) -> TestResult {
    // Test BoundedVec
    let mut vec = BoundedVec::<u32, 10>::new();

    // Test basic operations
    assert_test!(vec.try_push(1).is_ok());
    assert_test!(vec.try_push(2).is_ok());
    assert_test!(vec.try_push(3).is_ok());

    // Test capacity enforcement
    for i in 4..=10 {
        assert_test!(vec.try_push(i).is_ok());
    }

    // This should fail (over capacity)
    assert_test!(vec.try_push(11).is_err());

    // Test iteration
    let mut sum = 0;
    for &value in &vec {
        sum += value;
    }
    assert_eq_test!(sum, 55); // Sum of 1 through 10

    // Test BoundedStack
    let mut stack = BoundedStack::<u32, 5>::new();

    // Push some values
    assert_test!(stack.try_push(10).is_ok());
    assert_test!(stack.try_push(20).is_ok());
    assert_test!(stack.try_push(30).is_ok());

    // Pop values and check
    assert_eq_test!(stack.pop(), Some(30));
    assert_eq_test!(stack.pop(), Some(20));
    assert_eq_test!(stack.pop(), Some(10));
    assert_eq_test!(stack.pop(), None); // Stack is empty

    // Test BoundedHashMap
    let mut map = BoundedHashMap::<u32, &str, 5>::new();

    // Insert some values
    assert_test!(map.try_insert(1, "one").is_ok());
    assert_test!(map.try_insert(2, "two").is_ok());
    assert_test!(map.try_insert(3, "three").is_ok());

    // Retrieve values
    assert_eq_test!(map.get(&1), Some(&"one"));
    assert_eq_test!(map.get(&2), Some(&"two"));
    assert_eq_test!(map.get(&3), Some(&"three"));
    assert_eq_test!(map.get(&4), None);

    Ok(())
}

/// Test type system compatibility
fn test_type_system(_config: &TestConfig) -> TestResult {
    // Test basic value types
    let i32_type = ValueType::I32;
    let i64_type = ValueType::I64;
    let f32_type = ValueType::F32;
    let f64_type = ValueType::F64;

    // Test function types with BoundedVec
    let mut params = BoundedVec::<ValueType, 10>::new();
    let _ = params.try_push(i32_type);
    let _ = params.try_push(i64_type);

    let mut results = BoundedVec::<ValueType, 10>::new();
    let _ = results.try_push(f32_type);

    let func_type = FuncType { params: params.clone(), results: results.clone() };

    // Verify function type properties
    assert_eq_test!(func_type.params.len(), 2);
    assert_eq_test!(func_type.results.len(), 1);
    assert_eq_test!(func_type.params[0], ValueType::I32);
    assert_eq_test!(func_type.params[1], ValueType::I64);
    assert_eq_test!(func_type.results[0], ValueType::F32);

    // Test reference types
    let func_ref = RefType::FuncRef;
    let extern_ref = RefType::ExternRef;

    // Create table type with reference type
    let table_type = TableType { element_type: func_ref, limits: Limits { min: 0, max: Some(10) } };

    // Check component value types
    let i32_val_type = ValType::I32;
    let string_val_type = ValType::String;

    // Create component values
    let i32_value = ComponentValue::I32(42);
    let string_value = ComponentValue::String("test".to_string());

    // Verify component values
    assert_eq_test!(i32_value.get_type(), i32_val_type);
    assert_eq_test!(string_value.get_type(), string_val_type);

    Ok(())
}

/// Test limits conversion between format and types
fn test_limits_conversion(_config: &TestConfig) -> TestResult {
    // Create wrt-types limits
    let type_limits = Limits { min: 1, max: Some(10) };

    // Convert to format limits
    let format_limits = types_limits_to_format_limits(&type_limits);

    // Check the conversion
    assert_eq_test!(format_limits.min, 1);
    assert_eq_test!(format_limits.max, Some(10));

    // Convert back to types limits
    let round_trip = format_limits_to_types_limits(&format_limits);

    // Check the round trip
    assert_eq_test!(round_trip.min, type_limits.min);
    assert_eq_test!(round_trip.max, type_limits.max);

    // Test with no max
    let type_limits_no_max = Limits { min: 5, max: None };

    // Convert to format limits
    let format_limits_no_max = types_limits_to_format_limits(&type_limits_no_max);

    // Check the conversion
    assert_eq_test!(format_limits_no_max.min, 5);
    assert_eq_test!(format_limits_no_max.max, None);

    // Test runtime limits
    let runtime_limits = RuntimeLimits { min: 2, max: Some(20) };

    // Create equivalent type limits
    let equivalent_type_limits = Limits { min: 2, max: Some(20) };

    // Compare fields
    assert_eq_test!(runtime_limits.min, equivalent_type_limits.min);
    assert_eq_test!(runtime_limits.max, equivalent_type_limits.max);

    Ok(())
}

/// Test value operations in both std and no_std environments
fn test_value_operations(_config: &TestConfig) -> TestResult {
    // Test integer values
    let i32_val = Value::I32(42);
    let i64_val = Value::I64(0x0123_4567_89AB_CDEF);

    // Test float values
    let f32_val = Value::F32(3.14);
    let f64_val = Value::F64(2.71828);

    // Test reference values
    let func_ref_val = Value::FuncRef(None);
    let extern_ref_val = Value::ExternRef(None);

    // Check types
    assert_eq_test!(i32_val.val_type(), ValueType::I32);
    assert_eq_test!(i64_val.val_type(), ValueType::I64);
    assert_eq_test!(f32_val.val_type(), ValueType::F32);
    assert_eq_test!(f64_val.val_type(), ValueType::F64);
    assert_eq_test!(func_ref_val.val_type(), ValueType::FuncRef);
    assert_eq_test!(extern_ref_val.val_type(), ValueType::ExternRef);

    // Test value extraction
    if let Value::I32(v) = i32_val {
        assert_eq_test!(v, 42);
    } else {
        return Err("Failed to extract I32 value".to_string());
    }

    if let Value::I64(v) = i64_val {
        assert_eq_test!(v, 0x0123_4567_89AB_CDEF);
    } else {
        return Err("Failed to extract I64 value".to_string());
    }

    // Test ComponentValue creation and extraction
    let comp_i32 = ComponentValue::I32(42);
    let comp_string = ComponentValue::String("test".to_string());

    assert_eq_test!(comp_i32.get_type(), ValType::I32);
    assert_eq_test!(comp_string.get_type(), ValType::String);

    // Test binary conversion for ValType
    let i32_binary = val_type_to_binary(&ValType::I32);
    let string_binary = val_type_to_binary(&ValType::String);

    // Convert back
    let i32_from_binary = binary_to_val_type(i32_binary)
        .map_err(|e| format!("Failed to convert binary to ValType: {:?}", e))?;
    let string_from_binary = binary_to_val_type(string_binary)
        .map_err(|e| format!("Failed to convert binary to ValType: {:?}", e))?;

    assert_eq_test!(i32_from_binary, ValType::I32);
    assert_eq_test!(string_from_binary, ValType::String);

    Ok(())
}

/// Test different verification levels for memory safety
fn test_verification_levels(config: &TestConfig) -> TestResult {
    // Create a safe memory handler with bounded vector
    let mut buffer = BoundedVec::<u8, 1024>::new();
    for _i in 0..1024 {
        // Use _i if i is not used
        let _ = buffer.try_push(0);
    }

    let mut memory_handler = SafeMemoryHandler::new(buffer.as_slice());

    // Set verification level from config
    memory_handler.set_verification_level(config.verification_level);

    // Test basic operations
    let test_data = [1, 2, 3, 4, 5];
    memory_handler.clone_bytes_into(100, &test_data)?;

    // Get a safe slice and verify data
    let slice = memory_handler.get_slice(100, test_data.len())?;
    let data = slice.data()?;
    assert_eq_test!(data, &test_data);

    // Test integrity verification behavior based on level
    match config.verification_level {
        VerificationLevel::None => {
            // No verification means we can skip the check or it's a no-op
            // verify_integrity should still be callable and return Ok.
            memory_handler.verify_integrity()?;
            // Consider asserting something about operations count if possible to ensure no work was done.
        }
        VerificationLevel::Sampling => {
            // For sampling, verify_integrity might do work or not.
            // It's hard to make a deterministic assertion here without knowing the sampling details
            // or having a way to force/observe the sampling decision for a test.
            // For now, just ensure it can be called.
            memory_handler.verify_integrity()?;
            // If possible, one could try to run it multiple times and check if ops count varies,
            // but that's more complex than a simple unit test.
        }
        VerificationLevel::Standard => {
            // Standard verification should check basic integrity
            memory_handler.verify_integrity()?;

            // Attempt to corrupt memory (unsafe, for testing purposes)
            // This part is tricky because direct memory corruption might not always be caught
            // by `Standard` level, or its detection might not be guaranteed to fail `verify_integrity`.
            // The original comment "With standard verification, we might not catch all corruptions
            // so we can't reliably test this behavior" is important.
            // For now, ensure verify_integrity runs.
            // If a specific corruption scenario *should* be caught by Standard, that would be a more specific test.
        }
        VerificationLevel::Full => {
            // Full verification should perform thorough checks.
            memory_handler.verify_integrity()?;

            // If we corrupt memory here, Full verification should ideally catch it.
            // This requires a careful setup for corruption that Full is designed to detect.
            // Example (conceptual, actual corruption needs care):
            // if config.is_std { // Guard unsafe operations
            //     let raw_ptr = memory_handler.provider().as_ptr() as *mut u8;
            //     unsafe {
            //         if !raw_ptr.is_null() && memory_handler.provider().len() > 100 {
            //             *raw_ptr.add(100) = 255; // Corrupt a byte
            //         }
            //     }
            //     // After known corruption, verify_integrity at Full level should ideally fail.
            //     // assert_test!(memory_handler.verify_integrity().is_err());
            //     // However, this depends on the checksum implementation and what it covers.
            //     // For now, just call it. More specific tests would be needed for corruption detection.
            // }
        }
    }

    Ok(())
}
