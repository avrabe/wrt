use std::{
    fs,
    path::Path,
    str::FromStr,
    sync::{
        Arc,
        Mutex,
    },
    thread,
};

use wrt::{
    behavior::{
        ControlFlowBehavior,
        FrameBehavior,
        NullBehavior,
        StackBehavior,
    },
    execution::Engine,
    logging::{
        LogLevel,
        LogOperation,
    },
    Error,
    Global,
    Module,
    StacklessEngine,
    *,
};

// Include our WebAssembly spec test modules
#[cfg(test)]
mod simple_spec_tests;

// Include comprehensive test suite with expected failures
#[cfg(test)]
mod comprehensive_test_suite;

// Include test statistics validation
#[cfg(test)]
mod test_statistics_validation;

// Include SIMD instruction tests
#[cfg(test)]
mod simd_tests;

// Include WebAssembly testsuite tests
#[cfg(test)]
mod wasm_testsuite;

#[cfg(test)]
mod tests {
    use super::*;

    // Value Tests
    #[test]
    fn test_value_creation_and_access() {
        let i32_val = Value::I32(42);
        assert_eq!(i32_val.as_i32(), Some(42));
        assert_eq!(i32_val.as_i64(), None); // Wrong type access

        let i64_val = Value::I64(123456789);
        assert_eq!(i64_val.as_i64(), Some(123456789));
        assert_eq!(i64_val.as_i32(), None); // Wrong type access

        let f32_val = Value::F32(2.5);
        assert_eq!(f32_val.as_f32(), Some(2.5));
        assert_eq!(f32_val.as_f64(), None); // Wrong type access

        let f64_val = Value::F64(1.23456);
        assert_eq!(f64_val.as_f64(), Some(1.23456));
        assert_eq!(f64_val.as_f32(), None); // Wrong type access

        let func_ref = Value::FuncRef(Some(5));
        assert_eq!(func_ref.as_func_ref(), Some(Some(5)));
        assert_eq!(func_ref.as_extern_ref(), None); // Wrong type access

        let extern_ref = Value::ExternRef(Some(10));
        assert_eq!(extern_ref.as_extern_ref(), Some(Some(10)));
        assert_eq!(extern_ref.as_func_ref(), None); // Wrong type access
    }

    #[test]
    fn test_value_default() {
        // Test default values for each type
        assert_eq!(Value::default_for_type(&ValueType::I32).as_i32(), Some(0));
        assert_eq!(Value::default_for_type(&ValueType::I64).as_i64(), Some(0));
        assert_eq!(Value::default_for_type(&ValueType::F32).as_f32(), Some(0.0));
        assert_eq!(Value::default_for_type(&ValueType::F64).as_f64(), Some(0.0));
        assert_eq!(
            Value::default_for_type(&ValueType::FuncRef).as_func_ref(),
            Some(None)
        );
        assert_eq!(
            Value::default_for_type(&ValueType::ExternRef).as_extern_ref(),
            Some(None)
        );
    }

    // Memory Tests
    #[test]
    fn test_memory_operations() {
        let mem_type = MemoryType {
            min: 1,
            max: Some(10),
        };
        let mut memory = new_memory(mem_type);

        // Test initial state
        assert_eq!(memory.size(), 1);
        assert_eq!(memory.type_().limits.min, 1);
        assert_eq!(memory.type_().limits.max, Some(10));

        // Test growth
        let result = memory.grow(2);
        assert!(result.is_ok());
        assert_eq!(memory.size(), 3);

        // Test read/write operations
        memory.write_byte(0, 42).unwrap();
        assert_eq!(memory.read_byte(0).unwrap(), 42);

        memory.write_u32(4, 0x12345678).unwrap();
        assert_eq!(memory.read_u32(4).unwrap(), 0x12345678);
    }

    // #[test]
    // fn test_memory_bounds() {
    //     let mem_type = MemoryType {
    //         min: 1,
    //         max: Some(2),
    //     };
    //     let mut memory = new_memory(mem_type);

    //     // Test out of bounds access
    //     assert!(memory.read_byte(65536).is_err())); // Out of bounds read
    //     assert!(memory.write_byte(65536, 42).is_err())); // Out of bounds write

    //     // Test growth limits
    //     assert!(memory.grow(1).is_ok())); // Within max
    //     assert!(memory.grow(1).is_err())); // Exceeds max
    // }

    // Table Tests
    #[test]
    fn test_table_operations() {
        let table_type = TableType {
            element_type: ValueType::FuncRef,
            min:          1,
            max:          Some(10),
        };
        let mut table = new_table(table_type);

        // Test initial state
        assert_eq!(table.size(), 1);
        assert_eq!(table.type_().limits.min, 1);
        assert_eq!(table.type_().limits.max, Some(10));

        // Test growth
        assert!(table.grow(2).is_ok());
        assert_eq!(table.size(), 3);

        // Test element access
        let func_ref = Value::FuncRef(Some(42));
        assert!(table.set(0, Some(func_ref.clone())).is_ok());
        assert_eq!(table.get(0).unwrap(), Some(func_ref));

        // Test bounds checking
        assert!(table.get(100).is_err()); // Out of bounds
        assert!(table.set(100, None).is_err()); // Out of bounds
    }

    // Global Tests
    #[test]
    fn test_global_operations() {
        let global_type = GlobalType {
            content_type: ValueType::I32,
            mutable:      true,
        };
        let mut global = new_global(global_type, Value::I32(42)).unwrap();

        // Test initial value
        assert_eq!(global.get().as_i32(), Some(42));

        // Test mutability
        assert!(global.type_().mutable);
        global.set(Value::I32(100)).unwrap();
        assert_eq!(global.get().as_i32(), Some(100));
    }

    #[test]
    fn test_global_type_checking() {
        // Test immutable global
        let immutable_type = GlobalType {
            content_type: ValueType::I32,
            mutable:      false,
        };
        let immutable = new_global(immutable_type, Value::I32(42)).unwrap();
        assert!(!immutable.type_().mutable);

        // Test type mismatch
        let global_type = GlobalType {
            content_type: ValueType::I32,
            mutable:      true,
        };
        assert!(new_global(global_type, Value::F64(2.5)).is_err());
    }

    // Engine Tests
    #[test]
    fn test_engine_lifecycle() {
        let module = Module::new().expect("Module creation failed");
        let mut engine = StacklessEngine::new_with_module(module);
        // Access fields directly
        assert_eq!(engine.execution_stats.instructions_executed, 0);

        engine.fuel = Some(100);
        // ... execution ...
        engine.fuel = None;

        // Reset stats using Default
        engine.execution_stats = ExecutionStats::default();
        assert_eq!(engine.execution_stats.instructions_executed, 0);
        assert_eq!(engine.execution_stats.current_memory_bytes, 0); // Example access
        assert_eq!(engine.execution_stats.peak_memory_bytes, 0); // Example access
    }

    // Module Tests
    #[test]
    fn test_module_new() {
        let module_result = Module::new();
        assert!(module_result.is_ok());
        let module = module_result.unwrap(); // Unwrap here
        assert!(module.types.is_empty());
        assert!(module.imports.is_empty());
        assert!(module.exports.is_empty());
        assert!(module.start.is_none());
        // Validate should be called on the Module instance
        assert!(module.validate().is_ok());
    }

    #[test]
    fn test_module_structure() {
        let mut module_result = Module::new();
        assert!(module_result.is_ok());
        let mut module = module_result.unwrap(); // Unwrap here

        // Add a type
        module.types.push(FuncType {
            params:  vec![ValueType::I32],
            results: vec![],
        });

        // Add imports
        module.imports.push(Import {
            module: "env".to_string(),
            name:   "func".to_string(),
            // Use unwrapped module to access types
            ty:     ExternType::Function(module.types[0].clone()),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name:   "memory".to_string(),
            ty:     ExternType::Memory(MemoryType { min: 1, max: None }),
        });

        // Check imports
        assert_eq!(module.imports.len(), 2);

        // Check function import details
        let func_import = &module.imports[0];
        assert_eq!(func_import.module, "env");
        assert_eq!(func_import.name, "func");
        match &func_import.ty {
            ExternType::Function(func_type) => {
                assert_eq!(func_type.params, vec![ValueType::I32]);
                assert!(func_type.results.is_empty());
            },
            _ => panic!("Expected function import type"),
        }

        // Check memory import details
        let mem_import = &module.imports[1];
        assert_eq!(mem_import.module, "env");
        assert_eq!(mem_import.name, "memory");
        match &mem_import.ty {
            ExternType::Memory(mem_type) => {
                assert_eq!(mem_type.limits.min, 1);
                assert!(mem_type.limits.max.is_none());
            },
            _ => panic!("Expected memory import type"),
        }

        // Validate the module
        assert!(module.validate().is_ok());
    }

    // Logging Tests
    #[test]
    fn test_logging_system() {
        // Test log level parsing
        assert_eq!(LogLevel::from_str("trace").unwrap(), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("debug").unwrap(), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);
        assert_eq!(LogLevel::from_str("critical").unwrap(), LogLevel::Critical);

        // Test invalid log level
        assert!(LogLevel::from_str("invalid").is_err());

        // Test log operation creation
        let operation = LogOperation::new(LogLevel::Info, "Test message".to_string());
        assert_eq!(operation.level, LogLevel::Info);
        assert_eq!(operation.message, "Test message");
        assert!(operation.component_id.is_none());

        // Test log operation with component ID
        let operation_with_id = LogOperation::with_component(
            LogLevel::Warn,
            "Component message".to_string(),
            "test-component".to_string(),
        );
        assert_eq!(operation_with_id.level, LogLevel::Warn);
        assert_eq!(operation_with_id.message, "Component message");
        assert_eq!(
            operation_with_id.component_id,
            Some("test-component".to_string())
        );
    }

    // Error Tests
    #[test]
    fn test_error_handling() {
        // Test basic error types
        let validation_error = Error::Validation("test error".to_string());
        assert_eq!(validation_error.to_string(), "Validation error: test error");

        let execution_error = Error::Execution("test error".to_string());
        assert_eq!(execution_error.to_string(), "Execution error: test error");

        let fuel_error = Error::FuelExhausted;
        assert_eq!(fuel_error.to_string(), "Fuel exhausted");

        // Test additional error types
        let io_error = Error::IO("file not found".to_string());
        assert_eq!(io_error.to_string(), "I/O error: file not found");

        let parse_error = Error::Parse("invalid syntax".to_string());
        assert_eq!(parse_error.to_string(), "Parse error: invalid syntax");

        let component_error = Error::Component("invalid interface".to_string());
        assert_eq!(
            component_error.to_string(),
            "Component error: invalid interface"
        );

        let custom_error = Error::Custom("custom error message".to_string());
        assert_eq!(
            custom_error.to_string(),
            "Custom error: custom error message"
        );
    }

    #[test]
    fn test_memory_type_checking() {
        let module = Module::new().expect("Module creation failed");
        let mem = module.get_memory(0)?;
        let mem_type = mem.type_();

        // Check memory type
        assert_eq!(mem_type.limits.min, 1);
        assert!(mem_type.limits.max.is_none());
    }
}
