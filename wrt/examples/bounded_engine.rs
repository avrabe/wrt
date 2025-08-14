//! Example demonstrating the bounded collections in the StacklessEngine
//!
//! This example shows how to use the StacklessEngine with bounded collections,
//! and how to configure the verification level for different safety/performance
//! tradeoffs.

use wrt::{
    stackless::StacklessEngine,
    values::Value,
    Module,
    Result,
};
use wrt_foundation::VerificationLevel;

fn main() -> Result<()> {
    // Initialize global memory system for examples
    wrt_foundation::memory_system_initializer::presets::development()
        .map_err(|e| wrt::Error::Instantiation(format!("Memory system init failed: {}", e)))?;

    println!("Bounded Collections Example");
    println!("==========================\n");

    // Create a simple WebAssembly module with a function that adds two numbers
    let wat_code = r#"
    (module
      (func $add (export "add") (param i32 i32) (result i32)
        local.get 0
        local.get 1
        i32.add)
    )
    "#;

    // Parse the WebAssembly text format to binary
    let wasm = wat::parse_str(wat_code).unwrap();

    // Create a new module
    let module = Module::new()?.load_from_binary(&wasm)?;

    // Create engines with different verification levels
    let verification_levels = [
        VerificationLevel::None,     // No verification (fastest)
        VerificationLevel::Sampling, // Occasional verification (fast)
        VerificationLevel::Standard, // Regular verification (balanced)
        VerificationLevel::Full,     // Continuous verification (safest)
    ];

    // Run the same module with each verification level
    for level in verification_levels.iter() {
        println!("Testing with verification level: {:?}", level);

        // Create an engine with the current verification level
        let mut engine = StacklessEngine::with_verification_level(*level);

        // Instantiate the module
        let instance_idx = engine.instantiate(module.clone())?;

        // Prepare arguments for the add function
        let args = vec![Value::I32(40), Value::I32(2)];

        // Call the function
        let result = engine.call_function(instance_idx as u32, 0, &args)?;

        // Print the result
        println!("Result: {:?}", result);

        // Validate the engine state
        if let Err(e) = engine.validate() {
            println!("Validation failed: {:?}", e);
        } else {
            println!("Validation passed");
        }

        println!();
    }

    // Example of how to modify the verification level after engine creation
    println!("Changing verification level on an existing engine");
    let mut engine = StacklessEngine::new();
    println!(
        "Default verification level: {:?}",
        engine.verification_level
    );

    // Change to Full verification for maximum safety
    engine.set_verification_level(VerificationLevel::Full);
    println!("New verification level: {:?}", engine.verification_level);

    // Instantiate the module
    let instance_idx = engine.instantiate(module)?;

    // Call the function with arguments
    let args = vec![Value::I32(40), Value::I32(2)];
    let result = engine.call_function(instance_idx as u32, 0, &args)?;

    // Print the result
    println!("Result with Full verification: {:?}", result);

    // The engine state should be valid
    assert!(engine.validate().is_ok());

    println!("\nAll tests completed successfully!");

    Ok(())
}
