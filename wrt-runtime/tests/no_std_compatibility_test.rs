//! Test no_std compatibility for wrt-runtime
//!
//! This file validates that the wrt-runtime crate works correctly in no_std environments.

// For testing in a no_std environment
#![cfg_attr(not(feature = "std"), no_std)]

// External crate imports
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(test)]
mod tests {
    // Import necessary types for no_std environment
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::{boxed::Box, format, string::String, vec, vec::Vec};

    #[cfg(feature = "std")]
    use std::{boxed::Box, string::String, vec, vec::Vec};

    // Import from wrt-runtime
    use wrt_runtime::{
        component_impl::{ComponentRuntimeImpl, DefaultHostFunctionFactory},
        component_traits::{
            ComponentInstance, ComponentRuntime, HostFunction, HostFunctionFactory,
        },
        global::Global,
        memory::Memory,
        table::Table,
        types::{
            GlobalType as RuntimeGlobalType, MemoryType as RuntimeMemoryType,
            TableType as RuntimeTableType,
        },
    };

    // Import from wrt-types
    use wrt_types::{
        safe_memory::{SafeMemoryHandler, SafeSlice},
        types::{GlobalType as TypesGlobalType, Limits, ValueType},
        values::{FuncRef, Value},
        verification::VerificationLevel,
    };

    #[test]
    fn test_memory_operations() {
        // Create memory
        let memory_type = RuntimeMemoryType { limits: Limits { min: 1, max: Some(10) } };

        let mut memory = Memory::new(memory_type.clone()).unwrap();

        // Write memory
        let data = [1, 2, 3, 4];
        memory.write(0, &data).unwrap();

        // Read memory
        let mut buffer = [0; 4];
        memory.read(0, &mut buffer).unwrap();

        // Verify data
        assert_eq!(buffer, data);

        // Grow memory
        let old_pages = memory.grow(1).unwrap();
        assert_eq!(old_pages, 1); // Initial size was 1 page

        // Check new size
        assert_eq!(memory.size(), 2);
    }

    #[test]
    fn test_global_operations() {
        // Create global
        let global_type = TypesGlobalType { value_type: ValueType::I32, mutable: true };

        let mut global = Global::new(global_type, Value::I32(42));

        // Verify global value
        assert_eq!(global.get(), &Value::I32(42));

        // Modify global
        global.set(&Value::I32(100)).unwrap();

        // Verify new value
        assert_eq!(global.get(), &Value::I32(100));

        // Verify global type
        let global_type = global.global_type();
        assert_eq!(global_type.value_type, ValueType::I32);
        assert_eq!(global_type.mutable, true);
    }

    #[test]
    fn test_table_operations() {
        // Create table
        let table_type = RuntimeTableType {
            element_type: ValueType::FuncRef,
            limits: Limits { min: 10, max: Some(20) },
        };

        let mut table = Table::new(table_type.clone(), Value::FuncRef(None)).unwrap();

        // Get table size
        assert_eq!(table.size(), 10);

        // Set table element
        let func_ref = Value::FuncRef(Some(FuncRef::from_index(5)));
        table.set(0, Some(func_ref.clone())).unwrap();

        // Get table element
        let element = table.get(0).unwrap();

        // Verify element
        assert_eq!(element, Some(func_ref));

        // Grow table
        let old_size = table.grow(5, Value::FuncRef(None)).unwrap();
        assert_eq!(old_size, 10); // Initial size was 10

        // Check new size
        assert_eq!(table.size(), 15);
    }

    #[test]
    fn test_runtime_types() {
        // Test MemoryType
        let memory_type = RuntimeMemoryType { limits: Limits { min: 1, max: Some(2) } };

        // Test GlobalType
        let global_type = RuntimeGlobalType { value_type: ValueType::I32, mutable: true };

        // Test TableType
        let table_type = RuntimeTableType {
            element_type: ValueType::FuncRef,
            limits: Limits { min: 10, max: Some(20) },
        };

        // Verify different types
        assert_ne!(memory_type.limits.min, table_type.limits.min);
        assert_ne!(global_type.value_type, table_type.element_type);
    }
}
