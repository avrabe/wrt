//! Tests for safe memory adapter and memory integrity verification

use std::sync::Arc;

use wrt::{
    memory_adapter::{MemoryAdapter, SafeMemoryAdapter},
    stackless::StacklessEngine,
    Module,
};
use wrt_runtime::{Memory, MemoryType};
use wrt_types::{types::Limits, verification::VerificationLevel};

#[test]
fn test_safe_memory_adapter() {
    // Create a memory instance
    let memory_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let memory = Arc::new(Memory::new(memory_type).unwrap());

    // Create a SafeMemoryAdapter
    let adapter = SafeMemoryAdapter::new(memory.clone()).unwrap();

    // Test basic operations
    let data = [1, 2, 3, 4, 5];
    adapter.store(0, &data).unwrap();

    let loaded = adapter.load(0, 5).unwrap();
    assert_eq!(&*loaded, &data);

    // Test memory size
    assert_eq!(adapter.size().unwrap(), 1);
    assert_eq!(adapter.byte_size().unwrap(), 65536);

    // Test verification
    assert!(adapter.verify_integrity().is_ok());
}

#[test]
fn test_safe_memory_adapter_verification_levels() {
    // Create a memory instance
    let memory_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
    let memory = Arc::new(Memory::new(memory_type).unwrap());

    // Create adapters with different verification levels
    let adapter_none =
        SafeMemoryAdapter::with_verification_level(memory.clone(), VerificationLevel::None)
            .unwrap();
    let adapter_full =
        SafeMemoryAdapter::with_verification_level(memory.clone(), VerificationLevel::Full)
            .unwrap();

    // Both should work for basic operations
    let data = [1, 2, 3, 4, 5];
    adapter_none.store(0, &data).unwrap();
    let loaded = adapter_full.load(0, 5).unwrap();
    assert_eq!(&*loaded, &data);
}

#[test]
fn test_stackless_engine_memory_validation() {
    // Create a minimal module with memory
    let module = Module::default();

    // Create an engine with full verification
    let mut engine = StacklessEngine::with_verification_level(VerificationLevel::Full);

    // Instantiate the module
    let instance_idx = engine.instantiate(module).unwrap();

    // Validate the engine state
    assert!(engine.validate().is_ok());

    // Access memory with bounds checking
    assert!(engine.check_memory_bounds(instance_idx, 0, 0, 100).is_ok());

    // Out of bounds access should fail
    assert!(engine.check_memory_bounds(instance_idx, 0, 65536, 100).is_err());
}

#[test]
fn test_run_with_memory_safety() {
    // Create a minimal module with memory
    let module = Module::default();

    // Create an engine with full verification
    let mut engine = StacklessEngine::with_verification_level(VerificationLevel::Full);

    // Instantiate the module
    engine.instantiate(module).unwrap();

    // Run with memory safety
    let result = engine.run_with_memory_safety();

    // Should complete successfully
    assert!(result.is_ok());
}
