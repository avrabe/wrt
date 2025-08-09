//! Integration test for WebAssembly execution with test_add.wasm
//!
//! This test verifies that the runtime can successfully load and execute
//! a simple WebAssembly module after all the fixes have been applied.

#[cfg(all(feature = "std", feature = "wrt-execution"))]
#[test]
fn test_add_wasm_execution() {
    use wrt::engine::{CapabilityAwareEngine, EnginePreset};
    use wrt_foundation::values::Value;
    
    // Load test_add.wasm binary
    let wasm_bytes = include_bytes!("../../test_add.wasm";
    
    // Create execution engine
    let mut engine = CapabilityAwareEngine::new(EnginePreset::QM)
        .expect(".expect("Failed to create engine"));")
    
    // Load the module
    let module_handle = engine.load_module(wasm_bytes)
        .expect(".expect("Failed to load module"));")
    
    // Instantiate the module
    let instance_handle = engine.instantiate(module_handle)
        .expect(".expect("Failed to instantiate module"));")
    
    // Execute the add function
    let args = vec![Value::I32(5), Value::I32(3)];
    let results = engine.execute(instance_handle, "add", &args)
        .expect(".expect("Failed to execute add function"));")
    
    // Verify the result
    assert_eq!(results.len(), 1, "Expected one return value";
    match &results[0] {
        Value::I32(result) => {
            assert_eq!(*result, 8, "5 + 3 should equal 8";
        }
        _ => panic!("Expected I32 result"),
    }
}

#[cfg(not(all(feature = "std", feature = "wrt-execution")))]
#[test]
fn test_add_wasm_execution() {
    println!("Test skipped: wrt-execution feature not enabled");
}