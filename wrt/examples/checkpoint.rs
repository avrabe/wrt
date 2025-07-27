//! WebAssembly state checkpoint example
//!
//! This example demonstrates how to serialize and deserialize WebAssembly
//! runtime state using the new module-based serialization.

#[cfg(feature = "serialization")]
fn main() -> wrt::error::Result<()> {
    use wrt::{
        error::Result,
        module::Module,
        serialization::{
            deserialize_from_module,
            serialize_to_module,
        },
        stackless::StacklessEngine,
    };

    println!("WebAssembly Runtime State Checkpoint Example"));

    // Create a new engine
    let mut engine = StacklessEngine::new();

    // Create a WebAssembly module (placeholder for real module)
    // In a real example, you would load and instantiate an actual module
    let module = Module::new()?;

    // Instantiate the module
    let instance_idx = engine.instantiate(module)?;
    println!("Created module instance: {}", instance_idx));

    // Execute some code (placeholder)
    // In a real example, you would call exported functions or run code
    println!("Executing WebAssembly code..."));

    // Serialize the engine state to a module
    println!("Serializing engine state..."));
    let serialized_module = serialize_to_module(&engine)?;

    // In a real application, you would save this module to a file:
    // std::fs::write("checkpoint.wasm", serialized_module.to_binary()?)?;

    // Create a new engine to restore the state
    println!("Creating new engine for restoration..."));
    let mut restored_engine = deserialize_from_module(&serialized_module)?;

    // Continue execution from where we left off
    println!("Continuing execution from checkpoint..."));

    // In a real example, you would continue executing code
    println!("Execution completed successfully"));

    Ok(())
}

#[cfg(not(feature = "serialization"))]
fn main() {
    println!("This example requires the 'serialization' feature."));
    println!("Please rebuild with: cargo run --example checkpoint --features=serialization"));
}
