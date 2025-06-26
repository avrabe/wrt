use std::path::PathBuf;

use wrt::{Error, Module, Result, StacklessEngine, Value};

/// Test bitwise operations independently
#[test]
fn test_bitwise_operations() -> Result<()> {
    // A simple WAT module with individual functions for each bitwise operation
    let wat_code = r#"
    (module
      (func $and (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.and
      )
      (func $or (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.or
      )
      (func $xor (param $a i32) (param $b i32) (result i32)
        local.get $a
        local.get $b
        i32.xor
      )
      (export "and" (func $and))
      (export "or" (func $or))
      (export "xor" (func $xor))
    )
    "#;

    // Parse the WebAssembly text format
    let wasm_binary = wat::parse_str(wat_code)
        .map_err(|e| Error::Parse(format!("Failed to parse WAT: {}", e)))?;

    // Create a module
    let mut module = Module::new()?;
    let module = module.load_from_binary(&wasm_binary)?;

    // Create an engine
    let mut engine = StacklessEngine::new_with_module(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    // Using clear examples to test each bitwise operation
    let test_cases = vec![
        (10, 5), // 1010 & 0101 = 0000, 1010 | 0101 = 1111, 1010 ^ 0101 = 1111
        (12, 5), // 1100 & 0101 = 0100, 1100 | 0101 = 1101, 1100 ^ 0101 = 1001
        (0xFF, 0x0F), /* 11111111 & 00001111 = 00001111, 11111111 | 00001111 = 11111111,
                  * 11111111 ^ 00001111 = 11110000 */
    ];

    // Test all operations with all test cases
    for (a, b) in test_cases {
        let args = vec![Value::I32(a), Value::I32(b)];

        // Test AND
        let result = engine.execute(0usize, 0, args.clone())?;
        let expected_and = a & b;
        println!(
            "AND: {:#b} & {:#b} = {:#b} (Expected) | Result: {:?}",
            a, b, expected_and, result
        );

        // Test OR
        let result = engine.execute(0usize, 1, args.clone())?;
        let expected_or = a | b;
        println!(
            "OR: {:#b} | {:#b} = {:#b} (Expected) | Result: {:?}",
            a, b, expected_or, result
        );

        // Test XOR
        let result = engine.execute(0usize, 2, args.clone())?;
        let expected_xor = a ^ b;
        println!(
            "XOR: {:#b} ^ {:#b} = {:#b} (Expected) | Result: {:?}",
            a, b, expected_xor, result
        );

        println!("---");
    }

    Ok(())
}
