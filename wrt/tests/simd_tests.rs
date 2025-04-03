use std::{
    // path::{Path, PathBuf}, // Remove unused
    sync::{Arc, Mutex},
};
use wrt::{Error as WrtError, Module, Result, StacklessEngine, Value};

#[test]
fn test_v128_load_store() -> Result<()> {
    // Create a WebAssembly module with SIMD instructions that tests loading and storing a v128 value
    let wat = r#"
    (module
      (memory 1)
      (export "memory" (memory 0))
      
      ;; Store a v128 constant in memory
      (func $store_v128 (export "store_v128") 
        (i32.const 0)  ;; address
        (v128.const i32x4 1 2 3 4) ;; Using [1, 2, 3, 4] as i32 lanes
        (v128.store)
      )
      
      ;; Load a v128 value from memory
      (func $load_v128 (export "load_v128") (result v128)
        (i32.const 0)  ;; address
        (v128.load)
      )
    )
    "#;

    // Parse WAT and create module
    let wasm = wat::parse_str(wat).map_err(|e| wrt::Error::Parse(e.to_string()))?;
    let module = Module::new()?.load_from_binary(&wasm)?;
    let mut engine = StacklessEngine::new(module.clone());

    // Instantiate the module
    let instance_idx = engine.instantiate(module.clone())?;

    // Execute the store function defined in WAT
    let store_func_idx = module.get_export("store_v128").unwrap().index;
    engine.execute(instance_idx, store_func_idx, vec![])?;

    // Get the V128 load function
    let load_func_idx = module.get_export("load_v128").unwrap().index;

    // Invoke the load function
    let result = engine.execute(instance_idx, load_func_idx, vec![])?;

    // Check the result
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v; // v is already [u8; 16]
        let expected_bytes: [u8; 16] = [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]; // Little-endian representation of i32x4 [1, 2, 3, 4]
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "v128.load/store returned incorrect value"
        );
    } else {
        panic!("Expected V128 result");
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

    // Parse WAT and create module
    let wasm = wat::parse_str(wat).map_err(|e| wrt::Error::Parse(e.to_string()))?;
    let module = Module::new()?.load_from_binary(&wasm)?;
    let mut engine = StacklessEngine::new(module.clone());

    // Instantiate the module
    let instance_idx = engine.instantiate(module.clone())?;

    println!("Running v128 splat tests");

    // Test i8x16.splat
    let func_idx = module.get_export("i8x16_splat").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![Value::I32(10)])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [10; 16];
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i8x16.splat returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i8x16.splat");
    }

    // Test i16x8.splat
    let func_idx = module.get_export("i16x8_splat").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![Value::I32(2000)])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [
            208, 7, 208, 7, 208, 7, 208, 7, 208, 7, 208, 7, 208, 7, 208, 7,
        ]; // 2000 in little-endian i16
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i16x8.splat returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i16x8.splat");
    }

    // Test i32x4.splat
    let func_idx = module.get_export("i32x4_splat").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![Value::I32(300000)])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [
            224, 147, 4, 0, 224, 147, 4, 0, 224, 147, 4, 0, 224, 147, 4, 0,
        ];
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i32x4.splat returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i32x4.splat");
    }

    // Test i64x2.splat
    let func_idx = module.get_export("i64x2_splat").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![Value::I64(4000000000)])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [0, 40, 107, 238, 0, 0, 0, 0, 0, 40, 107, 238, 0, 0, 0, 0]; // 4000000000 in little-endian i64
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i64x2.splat returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i64x2.splat");
    }

    // Test f32x4.splat
    let func_idx = module.get_export("f32x4_splat").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![Value::F32(5.5)])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [0, 0, 176, 64, 0, 0, 176, 64, 0, 0, 176, 64, 0, 0, 176, 64]; // 5.5 in little-endian f32
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "f32x4.splat returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for f32x4.splat");
    }

    // Test f64x2.splat
    let func_idx = module.get_export("f64x2_splat").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![Value::F64(6.25)])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 25, 64, 0, 0, 0, 0, 0, 0, 25, 64]; // 6.25 in little-endian f64
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "f64x2.splat returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for f64x2.splat");
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

    // Parse WAT and create module
    let wasm = wat::parse_str(wat).map_err(|e| wrt::Error::Parse(e.to_string()))?;
    let module = Module::new()?.load_from_binary(&wasm)?;
    let mut engine = StacklessEngine::new(module.clone());

    // Instantiate the module
    let instance_idx = engine.instantiate(module.clone())?;

    // Test i8x16.shuffle
    let func_idx = module.get_export("shuffle").unwrap().index;
    let result = engine.execute(instance_idx, func_idx, vec![])?;
    if let Some(Value::V128(v)) = result.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [
            31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16,
        ]; // Corrected expected shuffled bytes
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i8x16.shuffle returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i8x16.shuffle");
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

    // Parse WAT and create module
    let wasm = wat::parse_str(wat).map_err(|e| wrt::Error::Parse(e.to_string()))?;
    let module = Module::new()?.load_from_binary(&wasm)?;
    let mut engine = StacklessEngine::new(module.clone());

    // Instantiate the module
    let instance_idx = engine.instantiate(module.clone())?;

    // Test i32x4.add
    let func_idx_add = module.get_export("i32x4_add").unwrap().index;
    let result_add = engine.execute(instance_idx, func_idx_add, vec![])?;
    if let Some(Value::V128(v)) = result_add.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [6, 0, 0, 0, 8, 0, 0, 0, 10, 0, 0, 0, 12, 0, 0, 0]; // Expected result of adding [1,2,3,4] and [5,6,7,8]
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i32x4.add returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i32x4.add");
    }

    // Test i32x4.sub
    let func_idx_sub = module.get_export("i32x4_sub").unwrap().index;
    let result_sub = engine.execute(instance_idx, func_idx_sub, vec![])?;
    if let Some(Value::V128(v)) = result_sub.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [9, 0, 0, 0, 18, 0, 0, 0, 27, 0, 0, 0, 36, 0, 0, 0];
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i32x4.sub returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i32x4.sub");
    }

    // Test i32x4.mul
    let func_idx_mul = module.get_export("i32x4_mul").unwrap().index;
    let result_mul = engine.execute(instance_idx, func_idx_mul, vec![])?;
    if let Some(Value::V128(v)) = result_mul.first() {
        let actual_bytes = v;
        let expected_bytes: [u8; 16] = [5, 0, 0, 0, 12, 0, 0, 0, 21, 0, 0, 0, 32, 0, 0, 0]; // Expected result of multiplying [1,2,3,4] and [5,6,7,8]
        assert_eq!(
            &actual_bytes[..],
            &expected_bytes[..],
            "i32x4.mul returned incorrect value"
        );
    } else {
        panic!("Expected V128 result for i32x4.mul");
    }

    Ok(())
}
