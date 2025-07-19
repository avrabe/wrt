//! Tests for bounded collections in the StacklessEngine implementation

use wrt::{
    stackless::{
        StacklessEngine,
        StacklessExecutionState,
    },
    Error as WrtError,
    Module,
    Result,
};
use wrt_foundation::VerificationLevel;

#[test]
fn test_stackless_engine_with_verification_levels() -> Result<()> {
    // WAT code for a simple test module
    let wat_code = r#"
    (module
      (memory (export "memory") 1)
      (func $add (export "add") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add)
    )
    "#;

    // Parse the WAT to WASM binary
    let wasm = wat::parse_str(wat_code).unwrap();

    // Test StacklessEngine with different verification levels
    let verification_levels = [
        VerificationLevel::None,
        VerificationLevel::Sampling,
        VerificationLevel::Standard,
        VerificationLevel::Full,
    ];

    for level in verification_levels.iter() {
        println!("Testing with verification level: {:?}", level;

        // Create a new module
        let module = Module::new()?.load_from_binary(&wasm)?;

        // Create an engine with the current verification level
        let mut engine = StacklessEngine::with_verification_level(*level;

        // Instantiate the module
        let instance_idx = engine.instantiate(module)?;

        // Call the add function with arguments
        let args = vec![wrt::values::Value::I32(40), wrt::values::Value::I32(2)];
        let result = engine.call_function(instance_idx as u32, 0, &args)?;

        // Verify the result
        assert_eq!(result.len(), 1;
        assert_eq!(result[0], wrt::values::Value::I32(42;

        // Validate the engine state
        engine.validate()?;
    }

    Ok(())
}

#[test]
fn test_large_local_count() -> Result<()> {
    // WAT code for a module with many locals
    let wat_code = r#"
    (module
      (func $many_locals (export "many_locals") (result i32)
        (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32
               i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
        i32.const 42)
    )
    "#;

    // Parse the WAT to WASM binary
    let wasm = wat::parse_str(wat_code).unwrap();

    // Create a new module
    let module = Module::new()?.load_from_binary(&wasm)?;

    // Create an engine with standard verification
    let mut engine = StacklessEngine::with_verification_level(VerificationLevel::Standard;

    // Instantiate the module
    let instance_idx = engine.instantiate(module)?;

    // Call the function with many locals
    let result = engine.call_function(instance_idx as u32, 0, &[])?;

    // Verify the result
    assert_eq!(result.len(), 1;
    assert_eq!(result[0], wrt::values::Value::I32(42;

    // Validate the engine state after execution
    engine.validate()?;

    Ok(())
}

#[test]
fn test_deep_call_stack() -> Result<()> {
    // WAT code for a module with a recursive function
    let wat_code = r#"
    (module
      (func $factorial (export "factorial") (param i32) (result i32)
        (local i32)
        ;; if n <= 1 return 1
        local.get 0
        i32.const 1
        i32.le_s
        if (result i32)
          i32.const 1
        else
          ;; else return n * factorial(n-1)
          local.get 0
          local.get 0
          i32.const 1
          i32.sub
          call $factorial
          i32.mul
        end)
    )
    "#;

    // Parse the WAT to WASM binary
    let wasm = wat::parse_str(wat_code).unwrap();

    // Create a new module
    let module = Module::new()?.load_from_binary(&wasm)?;

    // Create an engine with standard verification
    let mut engine = StacklessEngine::with_verification_level(VerificationLevel::Standard;

    // Instantiate the module
    let instance_idx = engine.instantiate(module)?;

    // Calculate factorial of 5 (should be 120)
    let args = vec![wrt::values::Value::I32(5)];
    let result = engine.call_function(instance_idx as u32, 0, &args)?;

    // Verify the result
    assert_eq!(result.len(), 1;
    assert_eq!(result[0], wrt::values::Value::I32(120;

    // Validate the engine state after execution
    engine.validate()?;

    Ok(())
}
