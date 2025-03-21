use wrt::{Engine, Error as WrtError, Module, Result, Value};

#[test]
fn test_i32_add() -> Result<()> {
    // Create a very simple Wasm module with just a function
    // that adds two numbers (i32.add)
    let wasm_binary = [
        // Magic header + version
        0x00, 0x61, 0x73, 0x6D, // Magic (\0asm)
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Type section (1 function type)
        0x01, 0x07, // Section code and size
        0x01, // Number of types
        0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F, // (func (param i32 i32) (result i32))
        // Function section
        0x03, 0x02, // Section code and size
        0x01, // Number of functions
        0x00, // Function 0 has type 0
        // Export section
        0x07, 0x07, // Section code and size
        0x01, // Number of exports
        // Export 0: "add"
        0x03, // String length
        0x61, 0x64, 0x64, // "add"
        0x00, // Export kind: function
        0x00, // Export index
        // Code section
        0x0A, 0x09, // Section code and size
        0x01, // Number of function bodies
        // Function 0: add
        0x07, // Function body size
        0x00, // Number of locals
        0x20, 0x00, // local.get 0
        0x20, 0x01, // local.get 1
        0x6A, // i32.add
        0x0B, // end
    ];

    // Load the module from binary
    let empty_module = Module::new();
    let module = empty_module.load_from_binary(&wasm_binary)?;

    // Create an engine with the loaded module
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;

    // Test with a few inputs
    let test_cases = [
        (vec![Value::I32(1), Value::I32(2)], vec![Value::I32(3)]),
        (vec![Value::I32(-1), Value::I32(1)], vec![Value::I32(0)]),
    ];

    println!("Running simple add test");

    for (idx, (inputs, expected)) in test_cases.iter().enumerate() {
        let result = engine.execute(0, 0, inputs.clone())?;

        if result == *expected {
            println!(
                "✅ Test case {}: {} + {} = {}",
                idx,
                inputs[0].as_i32().unwrap(),
                inputs[1].as_i32().unwrap(),
                expected[0].as_i32().unwrap()
            );
        } else {
            println!(
                "❌ Test case {}: {} + {} expected {}, got {:?}",
                idx,
                inputs[0].as_i32().unwrap(),
                inputs[1].as_i32().unwrap(),
                expected[0].as_i32().unwrap(),
                result
            );
            return Err(WrtError::Custom(format!("Test case {} failed", idx)));
        }
    }

    println!("All tests passed!");
    Ok(())
}

#[test]
fn test_simple_memory() -> Result<()> {
    // Define a simple WebAssembly module with memory operations in WAT format
    let wat = r#"
    (module
      (memory 1)
      (export "memory" (memory 0))
      
      (func $store (export "store") (param i32)
        i32.const 0  ;; address
        local.get 0  ;; value
        i32.store)   ;; store at address 0
      
      (func $load (export "load") (result i32)
        i32.const 0  ;; address
        i32.load))   ;; load from address 0
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

    println!("Running simple memory test");

    // Test store/load with a few different values
    let test_values = [42, -10, 0x12345678, -1i32];

    for (idx, value) in test_values.iter().enumerate() {
        // Store the value
        engine.execute(0, 0, vec![Value::I32(*value)])?;

        // Load the value back
        let result = engine.execute(0, 1, vec![])?;

        if result == vec![Value::I32(*value)] {
            println!("✅ Test case {}: Store and load {}", idx, value);
        } else {
            println!("❌ Test case {}: Store {}, got {:?}", idx, value, result);
            return Err(WrtError::Custom(format!("Test case {} failed", idx)));
        }
    }

    println!("All memory tests passed!");
    Ok(())
}
