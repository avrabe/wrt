use wrt::{Error, ExecutionEngine, Module, Value};

#[test]
fn test_simple_add() -> Result<(), Error> {
    // Create a very simple WebAssembly module with just an add function
    let wat_code = r#"
    (module
      (func (export "add") (param $x i32) (param $y i32) (result i32)
        local.get $x
        local.get $y
        i32.add)
    )
    "#;

    // Parse the WebAssembly text format to binary
    let wasm_binary = wat::parse_str(wat_code).expect("Failed to parse WAT");

    println!("DEBUG: WASM binary: {:?}", wasm_binary);

    // Create and load the module
    let mut module = Module::new()?;
    let module = module.load_from_binary(&wasm_binary)?;

    println!("DEBUG: Module exports: {:?}", module.exports);

    // Create the engine and instantiate the module
    let mut engine = ExecutionEngine::new(module.clone());
    let instance_idx = engine.instantiate(module.clone())?;

    println!("DEBUG: Instantiated module at index: {}", instance_idx);

    // Test values to add: 1 + 1 = 2
    let args = vec![Value::I32(1), Value::I32(1)];

    println!("DEBUG: Executing add function with args: {:?}", args);

    // Call the add function
    let result = engine.invoke_export("add", &args)?;

    println!("DEBUG: Result from add function: {:?}", result);

    // Verify the result
    assert_eq!(
        result,
        vec![Value::I32(2)],
        "Add function should return [I32(2)]"
    );

    println!("DEBUG: Test passed!");
    Ok(())
}
