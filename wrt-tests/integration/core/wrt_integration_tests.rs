//! Integration tests for the WebAssembly Runtime Toolkit
//!
//! This file contains comprehensive integration tests for the WRT ecosystem,
//! testing functionality in both std and no_std environments.

#![cfg(test)]

#[cfg(feature = "std")]
use std::println;

use wrt_test_registry::{TestRegistry, TestResult, assert_test, assert_eq_test};

/// Create an integration test registry with comprehensive tests
fn create_integration_test_registry() -> TestRegistry {
    let registry = TestRegistry::new);
    
    // Register all integration tests
    registry.register(Box::new(CoreFeaturesTest;
    registry.register(Box::new(ComponentModelTest;
    registry.register(Box::new(StacklessEngineTest;
    registry.register(Box::new(RuntimeFeaturesTest;
    registry.register(Box::new(ErrorHandlingTest;
    registry.register(Box::new(DecoderTest;
    
    registry
}

/// Test core WebAssembly features
struct CoreFeaturesTest;

impl wrt_test_registry::TestCase for CoreFeaturesTest {
    fn name(&self) -> &'static str {
        "core_features"
    }
    
    fn category(&self) -> &'static str {
        "integration"
    }
    
    fn requires_std(&self) -> bool {
        false // Works in both environments
    }
    
    fn run(&self) -> TestResult {
        // Create a new module
        let module = wrt::new_module().map_err(|e| e.to_string())?;
        
        // Test basic memory functionality
        let mem_type = wrt::MemoryType::new(1, Some(2), false;
        let memory = wrt::new_memory(mem_type;
        
        // Verify memory operations
        let test_data = [1, 2, 3, 4];
        memory.write(100, &test_data).map_err(|e| e.to_string())?;
        
        let mut read_buffer = [0; 4];
        memory.read(100, &mut read_buffer).map_err(|e| e.to_string())?;
        
        assert_eq_test!(read_buffer, test_data, "Memory read/write should work correctly";
        
        // Test table functionality
        let table_type = wrt::TableType::new(wrt_foundation::reference::RefType::FuncRef, 10, Some(20;
        let table = wrt::new_table(table_type;
        
        assert_eq_test!(table.size(), 10, "Initial table size should be 10";
        
        Ok(())
    }
}

/// Test the Component Model functionality
struct ComponentModelTest;

impl wrt_test_registry::TestCase for ComponentModelTest {
    fn name(&self) -> &'static str {
        "component_model"
    }
    
    fn category(&self) -> &'static str {
        "integration"
    }
    
    fn requires_std(&self) -> bool {
        true // Component model typically needs std
    }
    
    fn run(&self) -> TestResult {
        #[cfg(feature = "std")]
        {
            // Create a basic component
            let component = wrt::component::Component::new);
            
            // Verify component properties
            assert_test!(!component.is_initialized(), "New component should not be initialized";
            
            Ok(())
        }
        
        #[cfg(not(feature = "std"))]
        {
            Ok(()) // Skip test in no_std environment
        }
    }
}

/// Test the Stackless Engine functionality
struct StacklessEngineTest;

impl wrt_test_registry::TestCase for StacklessEngineTest {
    fn name(&self) -> &'static str {
        "stackless_engine"
    }
    
    fn category(&self) -> &'static str {
        "integration"
    }
    
    fn requires_std(&self) -> bool {
        false // Stackless engine is designed for no_std
    }
    
    fn run(&self) -> TestResult {
        // Create a stackless engine
        let engine = wrt::new_stackless_engine);
        
        // Verify engine is initialized properly
        assert_test!(!engine.is_executing(), "New engine should not be executing";
        
        Ok(())
    }
}

/// Test runtime features
struct RuntimeFeaturesTest;

impl wrt_test_registry::TestCase for RuntimeFeaturesTest {
    fn name(&self) -> &'static str {
        "runtime_features"
    }
    
    fn category(&self) -> &'static str {
        "integration"
    }
    
    fn requires_std(&self) -> bool {
        false // Works in both environments
    }
    
    fn run(&self) -> TestResult {
        // Create an execution engine
        let engine = wrt::new_engine);
        
        // Verify engine properties
        assert_test!(!engine.has_started(), "New engine should not have started";
        
        Ok(())
    }
}

/// Test error handling across the ecosystem
struct ErrorHandlingTest;

impl wrt_test_registry::TestCase for ErrorHandlingTest {
    fn name(&self) -> &'static str {
        "error_handling"
    }
    
    fn category(&self) -> &'static str {
        "integration"
    }
    
    fn requires_std(&self) -> bool {
        false // Error handling works in both environments
    }
    
    fn run(&self) -> TestResult {
        use wrt_error::{Error, kinds};
        
        // Test error creation and properties
        let validation_error = Error::runtime_execution_error(";
        let memory_error = Error::new(kinds::ErrorKind::Memory, 2, Some(";
        
        assert_eq_test!(validation_error.kind(), kinds::ErrorKind::Validation, "Error kind should be Validation";
        assert_eq_test!(memory_error.kind(), kinds::ErrorKind::Memory, "Error kind should be Memory";
        
        Ok(())
    }
}

/// Test decoder functionality
struct DecoderTest;

impl wrt_test_registry::TestCase for DecoderTest {
    fn name(&self) -> &'static str {
        "decoder"
    }
    
    fn category(&self) -> &'static str {
        "integration" 
    }
    
    fn requires_std(&self) -> bool {
        false // Decoder works in both environments
    }
    
    fn run(&self) -> TestResult {
        // Simple WAT module (addition function)
        let wasm_bytes = wat::parse_str(
            r#"(module
                (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )
            )"#,
        ).unwrap();
        
        // Test decoding
        let result = wrt_decoder::decode_module(&wasm_bytes;
        assert_test!(result.is_ok(), "Should successfully decode valid WASM module";
        
        // Get decoded module
        let decoded_module = result.unwrap();
        
        // Verify structure
        assert_test!(!decoded_module.functions.is_empty(), "Decoded module should have functions";
        assert_test!(!decoded_module.exports.is_empty(), "Decoded module should have exports";
        
        // Verify exports
        let add_export = decoded_module.exports.iter().find(|e| e.name == "add";
        assert_test!(add_export.is_some(), "Module should export 'add' function";
        
        Ok(())
    }
}

#[test]
fn run_integration_tests() {
    // Create the test registry
    let registry = create_integration_test_registry);
    
    // Run all tests
    #[cfg(feature = "std")]
    {
        let failed_count = registry.run_all_tests);
        assert_eq!(failed_count, 0, "All integration tests should pass";
    }
    
    // In no_std, we just statically register the tests but can't run them all automatically
    #[cfg(not(feature = "std"))]
    {
        // Just register tests, individual tests are run separately
    }
} 