use wrt::{
    Error,
    ExportKind,
    Module,
    Result,
};

#[test]
fn test_memory_persistence() -> Result<()> {
    let wat = r#"
        (module
          (memory (export "memory") 1)
          (func $store_int (export "store_int")
            i32.const 100      ;; address
            i32.const 42       ;; value 
            i32.store          ;; store 42 at address 100
          )
          
          (func $load_int (export "load_int") (result i32)
            i32.const 100      ;; address
            i32.load           ;; load value from address 100
          )
        )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module
    let module = Module::from_bytes(&wasm).expect("Failed to parse WASM");

    // Create a new engine with the StacklessEngine
    // This uses the correct implementation that initializes memory properly
    let mut engine = wrt::new_stackless_engine);
    let instance_idx = engine.instantiate(module.clone())?;

    // Find store function index
    let store_fn_idx = module
        .exports
        .iter()
        .find(|e| e.name == "store_int" && e.kind == ExportKind::Function)
        .map(|e| e.index)
        .ok_or(Error::ExportNotFound("store_int".to_string()))?;

    // Find load function index
    let load_fn_idx = module
        .exports
        .iter()
        .find(|e| e.name == "load_int" && e.kind == ExportKind::Function)
        .map(|e| e.index)
        .ok_or(Error::ExportNotFound("load_int".to_string()))?;

    // Store value first
    engine.execute(instance_idx, store_fn_idx.try_into().unwrap(), vec![])?;

    // Now load the value (should be 42)
    let results = engine.execute(instance_idx, load_fn_idx.try_into().unwrap(), vec![])?;
    println!("Load results: {:?}", results;

    assert_eq!(results[0].as_i32().unwrap_or(-1), 42;

    Ok(())
}

#[test]
fn test_memory_in_single_function() -> Result<()> {
    let wat = r#"
        (module
          (memory (export "memory") 1)
          (func $store_int (export "store_int")
            i32.const 100      ;; address
            i32.const 42       ;; value 
            i32.store          ;; store 42 at address 100
          )
          
          (func $load_int (export "load_int") (result i32)
            i32.const 100      ;; address
            i32.load           ;; load value from address 100
          )
        )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module
    let module = Module::from_bytes(&wasm).expect("Failed to parse WASM");

    // Create a new engine with the StacklessEngine
    // This uses the correct implementation that initializes memory properly
    let mut engine = wrt::new_stackless_engine);
    let instance_idx = engine.instantiate(module.clone())?;

    // Find store function index
    let store_fn_idx = module
        .exports
        .iter()
        .find(|e| e.name == "store_int" && e.kind == ExportKind::Function)
        .map(|e| e.index)
        .ok_or(Error::ExportNotFound("store_int".to_string()))?;

    // Find load function index
    let load_fn_idx = module
        .exports
        .iter()
        .find(|e| e.name == "load_int" && e.kind == ExportKind::Function)
        .map(|e| e.index)
        .ok_or(Error::ExportNotFound("load_int".to_string()))?;

    // Store 42 at address 100
    let store_results = engine.execute(instance_idx, store_fn_idx.try_into().unwrap(), vec![])?;
    println!("Store results: {:?}", store_results;

    // Load value from address 100 (should be 42)
    let load_results = engine.execute(instance_idx, load_fn_idx.try_into().unwrap(), vec![])?;
    println!("Load results: {:?}", load_results;

    assert_eq!(load_results[0].as_i32().unwrap_or(-1), 42;

    Ok(())
}

#[test]
fn test_memory_single_function_combined() -> Result<()> {
    let wat = r#"
        (module
          (memory (export "memory") 1)
          (func $store_and_load (export "store_and_load") (result i32)
            ;; Store value
            i32.const 100      ;; address
            i32.const 42       ;; value 
            i32.store          ;; store 42 at address 100
            
            ;; Load value (should be 42)
            i32.const 100      ;; address
            i32.load           ;; load value from address 100
          )
        )
    "#;

    // Convert WAT to binary WebAssembly
    let wasm = wat::parse_str(wat).expect("Failed to parse WAT");

    // Load the module
    let module = Module::from_bytes(&wasm).expect("Failed to parse WASM");

    // Create a new engine with the StacklessEngine
    let mut engine = wrt::new_stackless_engine);
    let instance_idx = engine.instantiate(module.clone())?;

    // Find function index
    let func_idx = module
        .exports
        .iter()
        .find(|e| e.name == "store_and_load" && e.kind == ExportKind::Function)
        .map(|e| e.index)
        .ok_or(Error::ExportNotFound("store_and_load".to_string()))?;

    // Call the combined function
    let results = engine.execute(instance_idx, func_idx.try_into().unwrap(), vec![])?;
    println!("Store and load results: {:?}", results;

    // This should work since it's within a single function
    assert_eq!(results[0].as_i32().unwrap_or(-1), 42;

    Ok(())
}
