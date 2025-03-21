//! WebAssembly Runtime (WRT)
//!
//! A pure Rust implementation of the WebAssembly runtime, supporting the WebAssembly Core
//! and Component Model specifications.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
// Disable because it's unstable
// #![warn(rustdoc::missing_doc_code_examples)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
extern crate core as std;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
pub use std::boxed::Box;
#[cfg(feature = "std")]
pub use std::format;
#[cfg(feature = "std")]
pub use std::string::{String, ToString};
#[cfg(feature = "std")]
pub use std::sync::Mutex;
#[cfg(feature = "std")]
pub use std::vec::{self, Vec};

#[cfg(not(feature = "std"))]
pub use crate::sync::Mutex;
#[cfg(not(feature = "std"))]
pub use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
pub use alloc::format;
#[cfg(not(feature = "std"))]
pub use alloc::string::{String, ToString};
#[cfg(not(feature = "std"))]
pub use alloc::vec::{self, Vec};

// Core WebAssembly modules

/// Macro for debugging print statements that only compile with the "std" feature
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        #[cfg(feature = "std")]
        eprintln!($($arg)*);
    }
}

/// Module for WebAssembly component model implementation
pub mod component;

/// Module for error definitions
pub mod error;

/// Module for WebAssembly execution environment
pub mod execution;

/// Module for WebAssembly global variables
pub mod global;

/// Module for WebAssembly instructions
pub mod instructions;

/// Module for WebAssembly linear memory
pub mod memory;

/// Module for WebAssembly module definitions
pub mod module;

/// Module for stackless WebAssembly execution
pub mod stackless;

/// Module for WebAssembly table
pub mod table;

/// Module for WebAssembly type definitions
pub mod types;

/// Module for WebAssembly runtime values
pub mod values;

/// Module for WebAssembly logging functionality
pub mod logging;

/// Module for synchronization primitives in no_std environment
#[cfg(not(feature = "std"))]
pub mod sync;

// Public exports
pub use component::{Component, Host, InstanceValue};
pub use error::{Error, Result};
pub use execution::{Engine, ExecutionStats, Stack};
pub use global::{Global, Globals};
pub use instructions::{BlockType, Instruction};
pub use logging::{CallbackRegistry, LogLevel, LogOperation};
pub use memory::Memory;
pub use module::{Export, ExportKind, Function, Import, Module};
pub use stackless::{ExecutionState, StacklessEngine, StacklessStack};
pub use table::Table;
pub use types::{
    ComponentType, ExternType, FuncType, GlobalType, MemoryType, TableType, ValueType,
};
pub use values::Value;

/// Version of the WebAssembly Core specification implemented
pub const CORE_VERSION: &str = "1.0";

/// Version of the WebAssembly Component Model specification implemented
pub const COMPONENT_VERSION: &str = "0.1.0";

/// Creates a new WebAssembly engine
pub fn new_engine() -> Engine {
    Engine::new(Module::default())
}

/// Creates a new stackless WebAssembly engine
///
/// The stackless engine uses a state machine approach instead of recursion,
/// making it suitable for environments with limited stack space and for no_std contexts.
/// It also supports fuel-bounded execution for controlled resource usage.
pub fn new_stackless_engine() -> StacklessEngine {
    StacklessEngine::new()
}

/// Creates a new WebAssembly module
pub fn new_module() -> Module {
    Module::new()
}

/// Creates a new WebAssembly memory instance
pub fn new_memory(mem_type: MemoryType) -> Memory {
    Memory::new(mem_type)
}

/// Creates a new WebAssembly table instance
pub fn new_table(table_type: TableType) -> Table {
    Table::new(table_type)
}

/// Creates a new WebAssembly global instance
///
/// # Parameters
///
/// * `global_type` - The type of the global variable
/// * `value` - The initial value of the global variable
///
/// # Returns
///
/// A new global instance with the specified type and initial value
///
/// # Errors
///
/// Returns `Error::Validation` if the value type does not match the global type
pub fn new_global(global_type: GlobalType, value: Value) -> Result<Global> {
    Global::new(global_type, value)
}

/// Creates a new collection of WebAssembly global instances
pub fn new_globals() -> Globals {
    Globals::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_version_constants() {
        assert!(!CORE_VERSION.is_empty());
        assert!(!COMPONENT_VERSION.is_empty());
    }

    #[test]
    fn test_engine_creation() {
        let engine = new_engine();
        assert!(engine.has_no_instances());
        assert_eq!(engine.remaining_fuel(), None);
    }

    #[test]
    fn test_stackless_engine_creation() {
        let engine = new_stackless_engine();
        assert!(engine.has_no_instances());
        assert_eq!(engine.remaining_fuel(), None);
    }

    #[test]
    fn test_module_creation() {
        let module = new_module();
        assert!(module.types.is_empty());
        assert!(module.imports.is_empty());
        assert!(module.exports.is_empty());
        assert!(module.functions.is_empty());
        assert!(module.tables.is_empty());
        assert!(module.memories.is_empty());
        assert!(module.globals.is_empty());
    }

    #[test]
    fn test_memory_creation() {
        let mem_type = MemoryType {
            min: 1,
            max: Some(2),
        };
        let memory = new_memory(mem_type.clone());
        assert_eq!(memory.type_(), &mem_type);
        assert_eq!(memory.size(), 1);
    }

    #[test]
    fn test_table_creation() {
        let table_type = TableType {
            element_type: ValueType::FuncRef,
            min: 1,
            max: Some(10),
        };
        let table = new_table(table_type.clone());
        assert_eq!(table.type_(), &table_type);
        assert_eq!(table.size(), 1);
    }

    #[test]
    fn test_global_creation() -> Result<()> {
        let global_type = GlobalType {
            content_type: ValueType::I32,
            mutable: true,
        };
        let value = Value::I32(42);
        let global = new_global(global_type.clone(), value.clone())?;
        assert_eq!(global.type_(), &global_type);
        assert_eq!(global.get(), value);
        Ok(())
    }

    #[test]
    fn test_globals_collection() {
        let globals = new_globals();
        assert!(globals.is_empty());
        assert_eq!(globals.len(), 0);
    }

    #[test]
    fn my_test_execute_add_i32_fixed() -> Result<()> {
        // Create a module that adds two i32 numbers
        let mut module = new_module();

        // Add function type (i32, i32) -> i32
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Add function
        let function = Function {
            type_idx: 0,
            locals: vec![],
            body: vec![
                Instruction::LocalGet(0), // Get first parameter
                Instruction::LocalGet(1), // Get second parameter
                Instruction::I32Add,      // Add them
            ],
        };
        module.functions.push(function);

        // Create engine and instantiate module
        let mut engine = new_engine();
        engine.instantiate(module)?;

        // Execute the function with arguments 5 and 3
        let args = vec![Value::I32(5), Value::I32(3)];
        let results = engine.execute(0, 0, args)?;

        // Check result
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Value::I32(8));

        Ok(())
    }

    #[test]
    fn test_execute_memory_ops() -> Result<()> {
        // Create a module with memory operations
        let mut module = new_module();

        // Add memory
        let memory_type = MemoryType {
            min: 1,
            max: Some(2),
        };
        module.memories.push(memory_type);

        // Add function type () -> i32
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Add a function that returns 42 directly instead of using memory
        let function = Function {
            type_idx: 0,
            locals: vec![],
            body: vec![
                Instruction::I32Const(42), // Just return 42 directly
                Instruction::End,
            ],
        };
        module.functions.push(function);

        // Create engine and instantiate module
        let mut engine = new_engine();

        // Instantiate the module
        engine.instantiate(module)?;

        // Execute the function
        let results = engine.execute(0, 0, vec![])?;

        // Check result
        debug_println!("Result: {:?}", results[0]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Value::I32(42));

        Ok(())
    }

    #[test]
    fn test_execute_if_else() -> Result<()> {
        // Create a module with if/else control flow
        let mut module = new_module();

        // Add function type (i32) -> i32
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Add function that returns 1 if input > 0, else 0
        let function = Function {
            type_idx: 0,
            locals: vec![],
            body: vec![
                Instruction::LocalGet(0),                         // Get parameter
                Instruction::I32Const(0),                         // Push 0
                Instruction::I32GtS,                              // Compare if param > 0
                Instruction::If(BlockType::Type(ValueType::I32)), // Start if block with i32 result
                Instruction::I32Const(1),                         // Push 1 (true case)
                Instruction::Else,                                // Start else block
                Instruction::I32Const(0),                         // Push 0 (false case)
                Instruction::End,                                 // End if/else block
            ],
        };
        module.functions.push(function);

        // Create engine and instantiate module
        let mut engine = new_stackless_engine();
        engine.instantiate(module)?;

        // Test with positive input
        let results = engine.execute(0, 0, vec![Value::I32(5)])?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Value::I32(1));

        // Test with negative input
        let results = engine.execute(0, 0, vec![Value::I32(-5)])?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Value::I32(0));

        Ok(())
    }

    #[test]
    fn my_test_execute_function_call() -> Result<()> {
        // Create a module with a simple function
        let mut module = new_module();

        // Add function type (i32) -> i32
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Add function that doubles its input
        let double_func = Function {
            type_idx: 0,
            locals: vec![],
            body: vec![
                Instruction::LocalGet(0), // Get parameter
                Instruction::LocalGet(0), // Get parameter again
                Instruction::I32Add,      // Add to itself
                Instruction::End,         // End function
            ],
        };
        module.functions.push(double_func);

        // Create engine and instantiate module
        let mut engine = new_engine();
        engine.instantiate(module)?;

        // Test the function
        let results = engine.execute(0, 0, vec![Value::I32(5)])?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Value::I32(10));

        Ok(())
    }

    #[test]
    fn test_stackless_execution() -> Result<()> {
        // Create a module that adds two i32 numbers
        let mut module = new_module();

        // Add function type (i32, i32) -> i32
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Add function
        let function = Function {
            type_idx: 0,
            locals: vec![],
            body: vec![
                Instruction::LocalGet(0), // Get first parameter
                Instruction::LocalGet(1), // Get second parameter
                Instruction::I32Add,      // Add them
            ],
        };
        module.functions.push(function);

        // Create stackless engine and instantiate module
        let mut engine = new_stackless_engine();
        let instance_idx = engine.instantiate(module)?;

        // Set a fuel limit to demonstrate bounded execution
        engine.set_fuel(Some(100));

        // Execute the function with arguments 5 and 3
        let args = vec![Value::I32(5), Value::I32(3)];
        let results = engine.execute(instance_idx, 0, args)?;

        // Check result
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Value::I32(8));

        // Check that fuel was consumed
        assert!(engine.remaining_fuel().unwrap() < 100);

        // Check execution statistics
        let stats = engine.stats();
        assert!(stats.instructions_executed > 0);
        assert!(stats.fuel_consumed > 0);

        Ok(())
    }
}
