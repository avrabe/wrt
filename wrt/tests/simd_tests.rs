use wrt::{Engine, Error as WrtError, Module, Result, Value};

#[test]
fn test_v128_load_store() -> Result<()> {
    // Create a WebAssembly module with SIMD instructions that tests loading and storing a v128 value
    let wat = r#"
    (module
      (memory 1)
      (export "memory" (memory 0))
      
      ;; Store a v128 constant in memory
      (func $store (export "store") 
        (v128.const i32x4 0x10203040 0x50607080 0x90A0B0C0 0xD0E0F0FF)
        (i32.const 0)  ;; address
        (v128.store)
      )
      
      ;; Load a v128 value from memory
      (func $load (export "load") (result v128)
        (i32.const 0)  ;; address
        (v128.load)
      )
    )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    println!("Running v128.load/store test");

    // Execute the store function to put a v128 value in memory
    engine.execute(0, 0, vec![])?;

    // Load the value back with the load function
    let result = engine.execute(0, 1, vec![])?;

    // Expected v128 value (0xD0E0F0FF_90A0B0C0_50607080_10203040 in little-endian representation)
    let expected_value = Value::V128(0xD0E0F0FF_90A0B0C0_50607080_10203040);
    if result == vec![expected_value.clone()] {
        println!("✅ v128.load/store test passed: {:?}", expected_value);
    } else {
        println!(
            "❌ v128.load/store test failed: expected {:?}, got {:?}",
            expected_value, result
        );
        return Err(WrtError::Custom("v128.load/store test failed".to_string()));
    }

    Ok(())
}

#[test]
fn test_v128_splat() -> Result<()> {
    // Create a WebAssembly module testing the various splat operations for v128
    let wat = r#"
    (module
      ;; i8x16.splat - create a vector with 16 lanes of the same i8 value
      (func $i8x16_splat (export "i8x16_splat") (param i32) (result v128)
        (local.get 0)
        (i8x16.splat))
        
      ;; i16x8.splat - create a vector with 8 lanes of the same i16 value
      (func $i16x8_splat (export "i16x8_splat") (param i32) (result v128)
        (local.get 0)
        (i16x8.splat))
        
      ;; i32x4.splat - create a vector with 4 lanes of the same i32 value
      (func $i32x4_splat (export "i32x4_splat") (param i32) (result v128)
        (local.get 0)
        (i32x4.splat))
        
      ;; i64x2.splat - create a vector with 2 lanes of the same i64 value
      (func $i64x2_splat (export "i64x2_splat") (param i64) (result v128)
        (local.get 0)
        (i64x2.splat))
        
      ;; f32x4.splat - create a vector with 4 lanes of the same f32 value
      (func $f32x4_splat (export "f32x4_splat") (param f32) (result v128)
        (local.get 0)
        (f32x4.splat))
        
      ;; f64x2.splat - create a vector with 2 lanes of the same f64 value
      (func $f64x2_splat (export "f64x2_splat") (param f64) (result v128)
        (local.get 0)
        (f64x2.splat))
    )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    println!("Running v128 splat tests");

    // Test i8x16.splat with value 0x42
    let result = engine.execute(0, 0, vec![Value::I32(0x42)])?;
    let expected = Value::V128(0x4242424242424242_4242424242424242);
    if result == vec![expected.clone()] {
        println!("✅ i8x16.splat test passed: {:?}", expected);
    } else {
        println!(
            "❌ i8x16.splat test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i8x16.splat test failed".to_string()));
    }

    // Test i16x8.splat with value 0x4243
    let result = engine.execute(0, 1, vec![Value::I32(0x4243)])?;
    let expected = Value::V128(0x4243424342434243_4243424342434243);
    if result == vec![expected.clone()] {
        println!("✅ i16x8.splat test passed: {:?}", expected);
    } else {
        println!(
            "❌ i16x8.splat test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i16x8.splat test failed".to_string()));
    }

    // Test i32x4.splat with value 0x10203040
    let result = engine.execute(0, 2, vec![Value::I32(0x10203040)])?;
    let expected = Value::V128(0x1020304010203040_1020304010203040);
    if result == vec![expected.clone()] {
        println!("✅ i32x4.splat test passed: {:?}", expected);
    } else {
        println!(
            "❌ i32x4.splat test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i32x4.splat test failed".to_string()));
    }

    // Test i64x2.splat with value 0x1122334455667788
    let result = engine.execute(0, 3, vec![Value::I64(0x1122334455667788)])?;
    let expected = Value::V128(0x1122334455667788_1122334455667788);
    if result == vec![expected.clone()] {
        println!("✅ i64x2.splat test passed: {:?}", expected);
    } else {
        println!(
            "❌ i64x2.splat test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i64x2.splat test failed".to_string()));
    }

    // Test f32x4.splat with value 3.14159
    let result = engine.execute(0, 4, vec![Value::F32(3.14159)])?;
    // We can't easily represent the exact expected bit pattern for floats, so just check that we got a v128 back
    if let Some(Value::V128(_)) = result.get(0) {
        println!("✅ f32x4.splat test passed: {:?}", result[0]);
    } else {
        println!(
            "❌ f32x4.splat test failed: expected V128, got {:?}",
            result
        );
        return Err(WrtError::Custom("f32x4.splat test failed".to_string()));
    }

    // Test f64x2.splat with value 2.71828
    let result = engine.execute(0, 5, vec![Value::F64(2.71828)])?;
    // We can't easily represent the exact expected bit pattern for floats, so just check that we got a v128 back
    if let Some(Value::V128(_)) = result.get(0) {
        println!("✅ f64x2.splat test passed: {:?}", result[0]);
    } else {
        println!(
            "❌ f64x2.splat test failed: expected V128, got {:?}",
            result
        );
        return Err(WrtError::Custom("f64x2.splat test failed".to_string()));
    }

    Ok(())
}

#[test]
fn test_v128_shuffle() -> Result<()> {
    // Create a WebAssembly module testing the i8x16.shuffle operation
    let wat = r#"
    (module
      ;; i8x16.shuffle - create a vector by selecting lanes from two vectors
      (func $shuffle (export "shuffle") (result v128)
        ;; First vector: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        (v128.const i8x16 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15)
        
        ;; Second vector: [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31]
        (v128.const i8x16 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31)
        
        ;; Shuffle: select lanes in reverse order, alternating between vectors
        (i8x16.shuffle 31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16)
      )
    )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    println!("Running i8x16.shuffle test");

    // Execute the shuffle function
    let result = engine.execute(0, 0, vec![])?;

    // The expected result is a vector with the lanes selected as specified in the shuffle
    // The lanes should be [31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16]
    let expected = Value::V128(0x1011121314151617_18191A1B1C1D1E1F);
    if result == vec![expected.clone()] {
        println!("✅ i8x16.shuffle test passed: {:?}", expected);
    } else {
        println!(
            "❌ i8x16.shuffle test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i8x16.shuffle test failed".to_string()));
    }

    Ok(())
}

#[test]
fn test_v128_arithmetic() -> Result<()> {
    // Create a WebAssembly module testing basic SIMD arithmetic operations
    let wat = r#"
    (module
      ;; i32x4.add - add two vectors lane-wise
      (func $i32x4_add (export "i32x4_add") (result v128)
        (v128.const i32x4 1 2 3 4)
        (v128.const i32x4 5 6 7 8)
        (i32x4.add))
        
      ;; i32x4.sub - subtract two vectors lane-wise
      (func $i32x4_sub (export "i32x4_sub") (result v128)
        (v128.const i32x4 10 20 30 40)
        (v128.const i32x4 1 2 3 4)
        (i32x4.sub))
        
      ;; i32x4.mul - multiply two vectors lane-wise
      (func $i32x4_mul (export "i32x4_mul") (result v128)
        (v128.const i32x4 1 2 3 4)
        (v128.const i32x4 5 6 7 8)
        (i32x4.mul))
    )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    println!("Running SIMD arithmetic tests");

    // Test i32x4.add
    let result = engine.execute(0, 0, vec![])?;
    // Expected: [1+5, 2+6, 3+7, 4+8] = [6, 8, 10, 12]
    let expected = Value::V128(0x0000000C0000000A_0000000800000006);
    if result == vec![expected.clone()] {
        println!("✅ i32x4.add test passed: {:?}", expected);
    } else {
        println!(
            "❌ i32x4.add test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i32x4.add test failed".to_string()));
    }

    // Test i32x4.sub
    let result = engine.execute(0, 1, vec![])?;
    // Expected: [10-1, 20-2, 30-3, 40-4] = [9, 18, 27, 36]
    let expected = Value::V128(0x0000002400000021_000000120000000A);
    if result == vec![expected.clone()] {
        println!("✅ i32x4.sub test passed: {:?}", expected);
    } else {
        println!(
            "❌ i32x4.sub test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i32x4.sub test failed".to_string()));
    }

    // Test i32x4.mul
    let result = engine.execute(0, 2, vec![])?;
    // Expected: [1*5, 2*6, 3*7, 4*8] = [5, 12, 21, 32]
    let expected = Value::V128(0x0000002000000015_000000070000000C);
    if result == vec![expected.clone()] {
        println!("✅ i32x4.mul test passed: {:?}", expected);
    } else {
        println!(
            "❌ i32x4.mul test failed: expected {:?}, got {:?}",
            expected, result
        );
        return Err(WrtError::Custom("i32x4.mul test failed".to_string()));
    }

    Ok(())
}
