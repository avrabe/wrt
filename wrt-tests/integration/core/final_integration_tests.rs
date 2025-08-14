//! Final integration test for the WRT reorganization plan
//!
//! This test verifies that all subcrates are properly integrated
//! in the main wrt crate and can be used in both std and no_std environments.

#![cfg(test)]

mod tests {
    use wrt::{
        // Format-related types from wrt-format
        binary,
        bounded::{
            BoundedStack,
            BoundedVec,
        },

        component as decoder_component,
        component::{
            Component,
            Host,
        },
        component::{
            FormatComponent,
            FormatComponentType,
        },
        compression::{
            rle_decode,
            rle_encode,
        },
        // Decoder-related functionality from wrt-decoder
        decoder_core,
        decoder_prelude,
        execution::ExecutionStats,
        // Instruction-related functionality from wrt-instructions
        instructions::{
            opcodes,
            Instruction,
        },

        module::{
            ExportKind,
            Module,
        },
        // Main WRT functionality
        new_engine,
        new_global,
        new_memory,
        new_module,
        new_table,
        parser::{
            Parser,
            Payload,
        },

        resource::{
            Resource,
            ResourceId,
        },
        section::{
            CustomSection,
            Section,
        },

        // Sync primitives from wrt-sync
        sync::{
            AtomicBool,
            Mutex,
            RwLock,
        },

        // Core types from wrt-foundation
        types::{
            FuncType,
            ValueType,
        },
        values::Value,
        // Error handling from wrt-error
        Error,
        ErrorCategory,
        Global,
        // Logging functionality from wrt-logging
        LogLevel,
        LogOperation,

        // Runtime-related functionality
        Memory,
        Result,

        Table,
        PAGE_SIZE,
    };

    #[test]
    fn test_core_type_integration() {
        // Test ValueType and FuncType from wrt-foundation
        let i32_type = ValueType::I32;
        let i64_type = ValueType::I64;

        let params = vec![i32_type, i64_type];
        let results = vec![i32_type];

        let func_type = FuncType::new(params, results);

        assert_eq!(func_type.params().len(), 2);
        assert_eq!(func_type.results().len(), 1);
    }

    #[test]
    fn test_value_integration() {
        // Test Value from wrt-foundation
        let i32_val = Value::I32(42);
        let f64_val = Value::F64(3.14159);

        assert_eq!(i32_val.get_type(), ValueType::I32);
        assert_eq!(f64_val.get_type(), ValueType::F64);
    }

    #[test]
    fn test_bounded_container_integration() {
        // Test BoundedVec from wrt-foundation
        let mut vec = BoundedVec::<u32, 5>::new();
        assert!(vec.push(1).is_ok());
        assert!(vec.push(2).is_ok());
        assert_eq!(vec.len(), 2);

        // Test BoundedStack from wrt-foundation
        let mut stack = BoundedStack::<u32, 5>::new();
        assert!(stack.push(1).is_ok());
        assert!(stack.push(2).is_ok());
        assert_eq!(stack.pop(), Some(2));
    }

    #[test]
    fn test_resource_integration() {
        // Test Resource from wrt-foundation
        let resource_id = ResourceId::new(42);
        assert_eq!(resource_id.get(), 42);
    }

    #[test]
    fn test_error_integration() {
        // Test Error from wrt-error
        let error = Error::runtime_execution_error("Test error".to_string());

        assert_eq!(error.category(), ErrorCategory::Core);
        assert_eq!(error.code(), 1);
    }

    #[test]
    fn test_memory_integration() {
        // Test Memory from wrt-runtime
        let memory = new_memory(wrt::MemoryType {
            limits: wrt::component::Limits {
                min: 1,
                max: Some(2),
            },
            shared: false,
        });

        // Write and read memory
        let data = [1, 2, 3, 4];
        assert!(memory.write(100, &data).is_ok());

        let mut buffer = [0; 4];
        assert!(memory.read(100, &mut buffer).is_ok());

        assert_eq!(buffer, data);
    }

    #[test]
    fn test_table_integration() {
        // Test Table from wrt-runtime
        let table = new_table(wrt::TableType {
            element_type: wrt::component::RefType::Funcref,
            limits:       wrt::component::Limits {
                min: 1,
                max: Some(10),
            },
        });

        assert_eq!(table.size(), 1);
    }

    #[test]
    fn test_global_integration() {
        // Test Global from wrt-runtime
        let global = new_global(
            wrt::GlobalType {
                value_type: ValueType::I32,
                mutable:    true,
            },
            Value::I32(42),
        )
        .unwrap();

        assert_eq!(global.get(), Value::I32(42));

        // Test mutability
        assert!(global.set(Value::I32(100)).is_ok());
        assert_eq!(global.get(), Value::I32(100));
    }

    #[test]
    fn test_module_integration() {
        // Test Module
        let module = new_module().unwrap();
        assert!(module.exports().is_empty());

        // Create a new engine
        let engine = new_engine();
        assert!(engine.validate_module(&module).is_ok());
    }

    #[test]
    fn test_sync_integration() {
        // Test Sync primitives from wrt-sync
        let atomic = AtomicBool::new(false);
        atomic.store(true, wrt::sync::atomic::Ordering::SeqCst);
        assert!(atomic.load(wrt::sync::atomic::Ordering::SeqCst));

        // Test Mutex
        let mutex = Mutex::new(42);
        {
            let mut guard = mutex.lock();
            *guard = 100;
        }
        assert_eq!(*mutex.lock(), 100);

        // Test RwLock
        let rwlock = RwLock::new(vec![1, 2, 3]);
        {
            let guard = rwlock.read();
            assert_eq!(*guard, vec![1, 2, 3]);
        }
        {
            let mut guard = rwlock.write();
            guard.push(4);
        }
        assert_eq!(*rwlock.read(), vec![1, 2, 3, 4]);
    }

    // This test ensures that all components of WRT can work together
    #[test]
    fn test_comprehensive_integration() {
        // Create module
        let module = new_module().unwrap();

        // Create memory
        let memory = new_memory(wrt::MemoryType {
            limits: wrt::component::Limits {
                min: 1,
                max: Some(2),
            },
            shared: false,
        });

        // Create globals
        let global = new_global(
            wrt::GlobalType {
                value_type: ValueType::I32,
                mutable:    true,
            },
            Value::I32(42),
        )
        .unwrap();

        // Create engine
        let engine = new_engine();

        // Validate the module
        assert!(engine.validate_module(&module).is_ok());

        // Test serialization if enabled
        #[cfg(feature = "serialization")]
        {
            use wrt::serialization::{
                deserialize_module,
                serialize_module,
            };

            // Serialize and deserialize the module
            let serialized = serialize_module(&module).unwrap();
            let deserialized = deserialize_module(&serialized).unwrap();

            // Validate the deserialized module
            assert!(engine.validate_module(&deserialized).is_ok());
        }
    }
}
