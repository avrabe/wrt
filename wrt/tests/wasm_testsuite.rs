use std::path::Path;
use wrt::{Engine, Error as WrtError, Module, Result, Value};

/// Utility function to get the test suite path from environment variables
fn get_testsuite_path() -> Option<String> {
    std::env::var("WASM_TESTSUITE").ok()
}

/// Basic test to verify the WebAssembly test suite is accessible
#[test]
fn verify_wasm_testsuite_access() {
    let testsuite_path = match get_testsuite_path() {
        Some(path) => path,
        None => {
            println!("Skipping test: WASM_TESTSUITE environment variable not set");
            return;
        }
    };

    // Check directory exists
    let testsuite_dir = Path::new(&testsuite_path);
    if !testsuite_dir.exists() {
        println!(
            "Warning: WebAssembly test suite directory not found at {:?}",
            testsuite_dir
        );
        return;
    }

    // Check for specific SIMD WAST files directly in testsuite directory
    let wast_files = vec![
        "simd_splat.wast",
        "simd_load.wast",
        "simd_store.wast",
        "simd_bitwise.wast",
        "simd_i8x16_arith.wast",
    ];

    println!("Found WASM test suite at: {}", testsuite_path);
    println!("Checking for SIMD WAST files...");

    let mut found_files = 0;
    for file in wast_files {
        let file_path = testsuite_dir.join(file);
        if file_path.exists() {
            println!("✅ Found {}", file);
            found_files += 1;
        } else {
            println!("❌ Missing {}", file);
        }
    }

    // Get the commit hash if available
    if let Ok(commit) = std::env::var("WASM_TESTSUITE_COMMIT") {
        println!("Test suite commit: {}", commit);
    }

    // This test passes as long as we find at least one SIMD file
    assert!(found_files > 0, "No SIMD test files found");
}

/// Test that runs a simple SIMD module with basic operations
#[test]
fn test_basic_simd_operations() -> Result<()> {
    println!("Running basic SIMD operations test");

    // WAT code with simple SIMD operations that only use splatting
    let wat_code = r#"
    (module
      (memory (export "memory") 1)
      (func (export "f32x4_splat_test") (result v128)
        f32.const 3.14
        f32x4.splat
      )
      (func (export "f64x2_splat_test") (result v128)
        f64.const 6.28
        f64x2.splat
      )
      (func (export "i32x4_splat_test") (result v128)
        i32.const 42
        i32x4.splat
      )
    )
    "#;

    // Parse the WebAssembly text format to a binary module
    let wasm_binary = wat::parse_str(wat_code).expect("Failed to parse WAT");

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm_binary)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    // Test f32x4.splat
    let result = engine.execute(0, 0, vec![])?;
    if let Some(Value::V128(_)) = result.get(0) {
        println!("✅ f32x4_splat_test passed: {:?}", result[0]);
    } else {
        println!(
            "❌ f32x4_splat_test failed: expected V128, got {:?}",
            result
        );
        return Err(WrtError::Custom("f32x4_splat_test failed".to_string()));
    }

    // Test f64x2.splat
    let result = engine.execute(0, 1, vec![])?;
    if let Some(Value::V128(_)) = result.get(0) {
        println!("✅ f64x2_splat_test passed: {:?}", result[0]);
    } else {
        println!(
            "❌ f64x2_splat_test failed: expected V128, got {:?}",
            result
        );
        return Err(WrtError::Custom("f64x2_splat_test failed".to_string()));
    }

    // Test i32x4.splat
    let result = engine.execute(0, 2, vec![])?;
    if let Some(Value::V128(_v)) = result.get(0) {
        println!("✅ i32x4_splat_test passed: {:?}", result[0]);
    } else {
        println!(
            "❌ i32x4_splat_test failed: expected V128, got {:?}",
            result
        );
        return Err(WrtError::Custom("i32x4_splat_test failed".to_string()));
    }

    println!("All SIMD operations tests passed!");
    Ok(())
}

#[test]
fn test_simd_dot_product() -> Result<()> {
    println!("Running SIMD dot product test");

    // WAT code to test i32x4.dot_i16x8_s
    let wat_code = r#"
    (module
      (func (export "dot_product_test") (result v128)
        ;; Create two vectors with i16x8.splat
        i32.const 2
        i16x8.splat  ;; [2, 2, 2, 2, 2, 2, 2, 2]
        i32.const 3
        i16x8.splat  ;; [3, 3, 3, 3, 3, 3, 3, 3]
        
        ;; Compute the dot product
        ;; Each lane should be: (2*3) + (2*3) = 12
        i32x4.dot_i16x8_s
      )
    )
    "#;

    // Parse the WebAssembly text format to a binary module
    let wasm_binary = wat::parse_str(wat_code).expect("Failed to parse WAT");

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm_binary)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    // Execute the dot product function
    let result = engine.execute(0, 0, vec![])?;
    if let Some(Value::V128(v)) = result.get(0) {
        println!("✅ dot_product_test passed: {:?}", result[0]);

        // Convert to bytes to inspect the values
        let bytes = v.to_le_bytes();

        // Read 4 i32 values out of the bytes
        let mut i32_values = [0i32; 4];
        for i in 0..4 {
            let start = i * 4;
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[start..start + 4]);
            i32_values[i] = i32::from_le_bytes(value_bytes);
        }

        // Check if each i32 value is 12 (2*3 + 2*3)
        let all_correct = i32_values.iter().all(|&x| x == 12);
        if all_correct {
            println!("✅ All dot product values are correct: {:?}", i32_values);
        } else {
            println!("❌ Incorrect dot product values: {:?}", i32_values);
            return Err(WrtError::Custom(
                "Dot product values are incorrect".to_string(),
            ));
        }
    } else {
        println!(
            "❌ dot_product_test failed: expected V128, got {:?}",
            result
        );
        return Err(WrtError::Custom("Dot product test failed".to_string()));
    }

    println!("SIMD dot product test passed!");
    Ok(())
}
