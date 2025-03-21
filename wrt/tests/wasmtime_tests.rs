use wrt::{Engine, ExportKind, Module, Result, Value};

#[test]
fn test_memory_persistence() -> Result<()> {
    // Define a simple WebAssembly module with memory operations
    let wat = r#"
        (module
          (memory 1)
          (export "memory" (memory 0))
          (func $store_value (export "store_value")
            i32.const 100      ;; address
            i32.const 42       ;; value 
            i32.store)         ;; store 42 at address 100
          (func $load_value (export "load_value") (result i32)
            i32.const 100      ;; address
            i32.load))         ;; load value from address 100
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Initialize the WRT engine
    let empty_module = Module::new();
    let module = empty_module
        .load_from_binary(&wasm)
        .expect("Failed to parse WASM");
    let mut engine = Engine::new(module.clone());

    // Check memory initialization
    let instance_idx = 0;
    println!("Engine created. Instances: {}", engine.instance_count());

    // Instantiate the module
    engine.instantiate(module)?;
    println!(
        "Module instantiated. Instances: {}",
        engine.instance_count()
    );

    // Get the instance and check its memory
    let instance = engine
        .get_instance(instance_idx)
        .expect("No instance found");
    println!("Memory count in instance: {}", instance.memories.len());

    if instance.memories.len() > 0 {
        println!("Memory size: {}", instance.memories[0].size_bytes());

        // Check the byte at address 100 before store (should be 0)
        let initial_byte = match instance.memories[0].read_byte(100) {
            Ok(b) => b,
            Err(e) => {
                println!("Error reading memory before store: {:?}", e);
                0
            }
        };
        println!("Initial byte at address 100: {}", initial_byte);
    }

    // Execute the store function
    println!("Executing store_value function...");
    let func_idx = instance
        .module
        .exports
        .iter()
        .find(|export| export.name == "store_value")
        .map(|export| {
            if export.kind == ExportKind::Function {
                export.index
            } else {
                panic!("Expected function export");
            }
        })
        .expect("No store_value export found");

    let store_result = engine.execute(instance_idx, func_idx, vec![])?;
    println!("Store result: {:?}", store_result);

    // Check memory state after store
    let instance = engine
        .get_instance(instance_idx)
        .expect("No instance found");
    if instance.memories.len() > 0 {
        // Read the byte at address 100 directly from memory
        let stored_bytes = match instance.memories[0].read_bytes(100, 4) {
            Ok(bytes) => bytes.to_vec(),
            Err(e) => {
                println!("Error reading memory after store: {:?}", e);
                vec![0, 0, 0, 0]
            }
        };
        println!("Bytes at address 100 after store: {:?}", stored_bytes);

        // Convert bytes to i32 value
        let stored_value = if stored_bytes.len() >= 4 {
            i32::from_le_bytes([
                stored_bytes[0],
                stored_bytes[1],
                stored_bytes[2],
                stored_bytes[3],
            ])
        } else {
            0
        };
        println!("Value at address 100 after store: {}", stored_value);
    }

    // Execute the load function
    println!("Executing load_value function...");
    let func_idx = instance
        .module
        .exports
        .iter()
        .find(|export| export.name == "load_value")
        .map(|export| {
            if export.kind == ExportKind::Function {
                export.index
            } else {
                panic!("Expected function export");
            }
        })
        .expect("No load_value export found");

    let load_result = engine.execute(instance_idx, func_idx, vec![])?;
    println!("Load result: {:?}", load_result);

    // Check the final result
    if let Some(Value::I32(value)) = load_result.first() {
        println!("Loaded value: {}", value);
        assert_eq!(*value, 42, "Expected 42, got {}", value);
        println!("✅ Memory persistence test passed!");
    } else {
        println!("❌ Memory persistence test failed: unexpected result format");
        assert!(false, "Expected I32 value in result");
    }

    Ok(())
}

#[test]
fn test_memory_in_single_function() -> Result<()> {
    // Define a simple WebAssembly module with a single function that does both store and load
    let wat = r#"
        (module
          (memory 1)
          (export "memory" (memory 0))
          (func $store_and_load (export "store_and_load") (result i32)
            i32.const 100      ;; address
            i32.const 42       ;; value 
            i32.store          ;; store 42 at address 100
            
            i32.const 100      ;; address
            i32.load))         ;; load value from address 100
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Initialize the WRT engine
    let empty_module = Module::new();
    let module = empty_module
        .load_from_binary(&wasm)
        .expect("Failed to parse WASM");
    let mut engine = Engine::new(module.clone());

    // Instantiate the module
    engine.instantiate(module)?;
    println!("Module instantiated successfully");

    // Get the instance and find the function
    let instance_idx = 0;
    let instance = engine
        .get_instance(instance_idx)
        .expect("No instance found");

    let func_idx = instance
        .module
        .exports
        .iter()
        .find(|export| export.name == "store_and_load")
        .map(|export| {
            if export.kind == ExportKind::Function {
                export.index
            } else {
                panic!("Expected function export");
            }
        })
        .expect("No store_and_load export found");

    // Execute the combined function
    println!("Executing store_and_load function...");
    let result = engine.execute(instance_idx, func_idx, vec![])?;
    println!("Function result: {:?}", result);

    // Check the result
    if let Some(Value::I32(value)) = result.first() {
        println!("Loaded value: {}", value);
        assert_eq!(*value, 42, "Expected 42, got {}", value);
        println!("✅ Memory operations work correctly within a single function!");
    } else {
        println!("❌ Memory test failed: unexpected result format");
        assert!(false, "Expected I32 value in result");
    }

    Ok(())
}
