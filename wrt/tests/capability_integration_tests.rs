//! Integration tests for capability-based WebAssembly execution
//!
//! These tests verify end-to-end functionality of the capability engine
//! across different safety levels.

#[cfg(all(test, any(feature = "std", feature = "alloc")))]
mod tests {
    use wrt::prelude::*;

    #[test]
    fn test_end_to_end_wasm_execution_qm() {
        // Initialize memory system
        wrt_foundation::memory_init::MemoryInitializer::initialize().unwrap();

        // Simple add function
        let wat = r#"
            (module
                (func $add (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add))
        "#;

        let wasm = wat::parse_str(wat).unwrap();

        let mut engine = CapabilityAwareEngine::with_preset(EnginePreset::QM).unwrap();
        let module = engine.load_module(&wasm).unwrap();
        let instance = engine.instantiate(module).unwrap();

        let result = engine.execute(instance, "add", &[Value::I32(2), Value::I32(3)]).unwrap();
        assert_eq!(result.len(), 1;
        assert_eq!(result[0], Value::I32(5;
    }

    #[test]
    fn test_simple_function_execution() {
        // Initialize memory system
        wrt_foundation::memory_init::MemoryInitializer::initialize().unwrap();

        // Module with a simple function that returns 42
        let wat = r#"
            (module
                (func $get_answer (export "get_answer") (result i32)
                    i32.const 42))
        "#;

        let wasm = wat::parse_str(wat).unwrap();

        let mut engine = CapabilityAwareEngine::with_preset(EnginePreset::QM).unwrap();
        let module = engine.load_module(&wasm).unwrap();
        let instance = engine.instantiate(module).unwrap();

        let result = engine.execute(instance, "get_answer", &[]).unwrap();
        assert_eq!(result.len(), 1;
        assert_eq!(result[0], Value::I32(42;
    }

    #[test]
    fn test_module_with_start_function() {
        // Initialize memory system
        wrt_foundation::memory_init::MemoryInitializer::initialize().unwrap();

        // Module with a start function that sets a global
        let wat = r#"
            (module
                (global $initialized (mut i32) (i32.const 0))
                (func $init
                    i32.const 1
                    global.set $initialized)
                (start $init)
                (func $is_initialized (export "is_initialized") (result i32)
                    global.get $initialized))
        "#;

        let wasm = wat::parse_str(wat).unwrap();

        let mut engine = CapabilityAwareEngine::with_preset(EnginePreset::QM).unwrap();
        let module = engine.load_module(&wasm).unwrap();
        let instance = engine.instantiate(module).unwrap();

        // Start function should have run during instantiation
        let result = engine.execute(instance, "is_initialized", &[]).unwrap();
        assert_eq!(result[0], Value::I32(1;
    }

    #[test]
    #[cfg(feature = "asil-b")]
    fn test_capability_constraints_asil_b() {
        // Initialize memory system
        wrt_foundation::memory_init::MemoryInitializer::initialize().unwrap();

        let mut engine = CapabilityAwareEngine::with_preset(EnginePreset::AsilB).unwrap();

        // Try to allocate beyond ASIL-B limits
        // Create a very large module that exceeds capability limits
        let large_size = 10_000_000; // 10MB
        let mut large_module = vec![
            0x00, 0x61, 0x73, 0x6D, // WASM magic
            0x01, 0x00, 0x00, 0x00, // Version
        ];
        // Pad with zeros to make it large
        large_module.resize(large_size, 0;

        let result = engine.load_module(&large_module;

        // Should fail due to capability constraints
        assert!(result.is_err();
        let err = result.unwrap_err);
        assert!(
            err.to_string().contains("capability")
                || err.to_string().contains("Allocate")
                || err.to_string().contains("size")
        ;
    }

    #[test]
    fn test_multiple_instances() {
        // Initialize memory system
        wrt_foundation::memory_init::MemoryInitializer::initialize().unwrap();

        let wat = r#"
            (module
                (func $identity (export "identity") (param i32) (result i32)
                    local.get 0))
        "#;

        let wasm = wat::parse_str(wat).unwrap();

        let mut engine = CapabilityAwareEngine::with_preset(EnginePreset::QM).unwrap();

        // Load module once
        let module = engine.load_module(&wasm).unwrap();

        // Create multiple instances
        let instance1 = engine.instantiate(module).unwrap();
        let instance2 = engine.instantiate(module).unwrap();

        // Both instances should work
        let result1 = engine.execute(instance1, "identity", &[Value::I32(10)]).unwrap();
        let result2 = engine.execute(instance2, "identity", &[Value::I32(20)]).unwrap();

        assert_eq!(result1[0], Value::I32(10;
        assert_eq!(result2[0], Value::I32(20;
    }

    #[test]
    fn test_invalid_function_name() {
        // Initialize memory system
        wrt_foundation::memory_init::MemoryInitializer::initialize().unwrap();

        let wat = r#"
            (module
                (func $foo (export "foo") (result i32)
                    i32.const 42))
        "#;

        let wasm = wat::parse_str(wat).unwrap();

        let mut engine = CapabilityAwareEngine::with_preset(EnginePreset::QM).unwrap();
        let module = engine.load_module(&wasm).unwrap();
        let instance = engine.instantiate(module).unwrap();

        // Try to call non-existent function
        let result = engine.execute(instance, "bar", &[];
        assert!(result.is_err();
        assert!(result.unwrap_err().to_string().contains("not found");
    }
}
