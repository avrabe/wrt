//! WebAssembly Runtime (WRT)
//!
//! A pure Rust implementation of the WebAssembly runtime, supporting the WebAssembly Core
//! and Component Model specifications.

#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
// Disable because it's unstable
// #![warn(rustdoc::missing_doc_code_examples)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
extern crate alloc;

// Import and re-export types from std when available
#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::HashMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

// Import and re-export types for no_std environment
#[cfg(not(feature = "std"))]
pub use alloc::{
    boxed::Box,
    collections::BTreeMap as HashMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Re-export needed traits and types at crate level
pub use error::{Error, Result};

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

/// Module for WebAssembly error handling
pub mod error;

/// Module for WebAssembly execution
pub mod execution;

/// Module for WebAssembly global variables
pub mod global;

/// Module for WebAssembly instructions
pub mod instructions;

/// Module for WebAssembly Component Model interface types
pub mod interface;

/// Module for WebAssembly linear memory
pub mod memory;

/// Module for WebAssembly module definitions
pub mod module;

/// Module for WebAssembly Component Model resource handling
pub mod resource;

/// Module for WebAssembly serialization (experimental)
#[cfg(feature = "serialization")]
pub mod serialization;

/// Module for stackless WebAssembly execution
pub mod stackless;

/// Extensions for the stackless WebAssembly execution engine
pub mod stackless_extensions;

/// Module for WebAssembly table
pub mod table;

/// Module for WebAssembly type definitions
pub mod types;

/// Module for WebAssembly runtime values
pub mod values;

/// Module for WebAssembly logging functionality
pub mod logging;

/// Module for WebAssembly synchronization primitives in no_std environment
#[cfg(not(feature = "std"))]
pub mod sync;

/// Shared instruction implementations for all engines
pub mod shared_instructions;

/// Module for WebAssembly stack operations
pub mod stack;

/// Module for WebAssembly behavior
pub mod behavior;

/// Module for WebAssembly module instance
pub mod module_instance;

/// Module for WebAssembly stackless frame
pub mod stackless_frame;

// Public exports
pub use crate::{stackless::StacklessEngine, stackless_frame::StacklessFrame};
pub use behavior::InstructionExecutor;
pub use component::{Component, Host, InstanceValue};
pub use execution::ExecutionStats;
pub use global::Global;
pub use instructions::{types::BlockType, Instruction};
pub use logging::HostFunctionHandler;
pub use memory::{DefaultMemory, MemoryBehavior};
pub use module::{ExportKind, Function, Import, Module, OtherExport};
pub use stack::Stack;
pub use table::Table;
pub use types::{
    ComponentType, ExternType, FuncType, GlobalType, MemoryType, TableType, ValueType,
};
pub use values::Value;

/// Version of the WebAssembly Core specification implemented
pub const CORE_VERSION: &str = "1.0";

/// Version of the WebAssembly Component Model specification implemented
pub const COMPONENT_VERSION: &str = "0.1.0";

/// Uses execution engine implementation instead of redefining
pub use execution::Engine as ExecutionEngine;

/// Creates a new WebAssembly engine
#[must_use]
pub fn new_engine() -> ExecutionEngine<'static> {
    let module = Module::new().expect("Failed to create new empty module");
    ExecutionEngine::new(module)
}

/// Creates a new stackless WebAssembly engine
///
/// The stackless engine uses a state machine approach instead of recursion,
/// making it suitable for environments with limited stack space and for `no_std` contexts.
/// It also supports fuel-bounded execution for controlled resource usage.
#[must_use]
pub fn new_stackless_engine() -> stackless::StacklessEngine {
    stackless::StacklessEngine::new()
}

/// Creates a new WebAssembly module
#[must_use]
pub fn new_module() -> Result<Module> {
    Module::new()
}

/// Creates a new WebAssembly memory instance
#[must_use]
pub fn new_memory(mem_type: MemoryType) -> DefaultMemory {
    DefaultMemory::new(mem_type)
}

/// Creates a new WebAssembly table instance
#[must_use]
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

/// Creates a new empty globals vector
#[must_use]
pub fn new_globals() -> Vec<Arc<Global>> {
    Vec::new()
}

/// Executes a test with the stackless engine.
///
/// # Errors
///
/// Returns an error if the test execution fails, such as when loading the module,
/// instantiating the module, or executing the test itself.
#[cfg(feature = "wat-parsing")]
pub fn execute_test_with_stackless(path: &str) -> Result<()> {
    // Parse the WAT to WASM
    let wasm = wat::parse_file(path).map_err(|e| Error::Parse(e.to_string()))?;

    // Create a new module
    let mut module = Module::new()?;
    let module = module.load_from_binary(&wasm)?;

    println!(
        "Successfully loaded module with {} memory definitions",
        module.memories_len()
    );
    println!("Memory types: {:?}", module.memories);
    println!(
        "Exports: {}",
        module
            .exports
            .iter()
            .map(|e| format!("{} (kind={:?}, idx={})", e.name, e.kind, e.index))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Initialize the StacklessVM
    let mut engine = new_stackless_engine();
    let instance_idx = engine.instantiate(module.clone())?;

    // Set a fuel limit to prevent infinite loops
    engine.set_fuel(Some(1000000));

    // Find the 'run' export in the module
    let export = module
        .exports
        .iter()
        .find(|e| e.name == "run" && e.kind == crate::module::ExportKind::Function)
        .ok_or_else(|| Error::Execution("No 'run' export found".into()))?;

    println!("Found 'run' export at index {}", export.index);

    // Set up the engine state for execution
    let instance_idx_usize = instance_idx
        .try_into()
        .map_err(|_| Error::InvalidInstanceIndex(instance_idx))?;
    engine
        .stack
        .execute(engine, instance_idx_usize, export.index as u32, Vec::new())?;

    // Check if we have a result
    if let Some(result) = engine.stack.values.last() {
        if *result == Value::I32(1) {
            Ok(())
        } else {
            Err(Error::Execution(format!(
                "Test failed: expected 1, got {result:?}"
            )))
        }
    } else {
        Err(Error::Execution("Expected I32 result".to_string()))
    }
}

/// Executes an exported function by name from a specific instance in the engine.
///
/// This function looks up the export by name and calls it using the engine.
///
/// # Errors
///
/// Returns an error if the export is not found, if the export is not a function,
/// or if there is an error during the function execution.
pub fn execute_export_by_name(
    instance_idx: usize,
    name: &str,
    engine: &mut ExecutionEngine,
) -> Result<Vec<Value>> {
    use std::vec::Vec;

    match find_export_by_name(instance_idx, name, engine) {
        Some(export) => {
            // Check if this is a function export, otherwise return an error
            if export.kind != ExportKind::Function {
                return Err(Error::ExportNotFound(format!(
                    "Export {name} is not a function"
                )));
            }

            // Execute the function with an empty arguments vector
            let result = engine.execute(instance_idx, export.index, Vec::new());

            // Check if execution was halted due to out of fuel
            match result {
                Ok(values) => Ok(values),
                Err(Error::FuelExhausted) => {
                    debug_println!("Execution halted due to out of fuel");
                    Err(Error::FuelExhausted)
                }
                Err(e) => Err(e),
            }
        }
        None => Err(Error::ExportNotFound(format!("Export {name} not found"))),
    }
}

/// Find an export by name in a module instance
fn find_export_by_name(
    instance_idx: usize,
    name: &str,
    engine: &ExecutionEngine,
) -> Option<OtherExport> {
    if instance_idx >= engine.instances.len() {
        return None;
    }

    let instance = &engine.instances[instance_idx];

    // Look for the export in the module's exports
    for export in &instance.module.exports {
        if export.name == name {
            return Some(export.clone());
        }
    }

    None
}

/// Gets the memory count of a module
pub fn memories_len(module: &Module) -> Result<usize> {
    Ok(module.memories_len())
}

/// Gets the number of tables in a module
pub fn tables_len(module: &Module) -> Result<usize> {
    Ok(module.tables_len())
}

/// Gets the number of globals in a module
pub fn globals_len(module: &Module) -> Result<usize> {
    Ok(module.globals_len())
}

pub fn new() -> Result<ExecutionEngine<'static>> {
    ExecutionEngine::new_from_result(Module::new())
}

impl StacklessEngine {
    pub fn execute(
        &mut self,
        instance_idx: usize,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        let instance = &self.instances[instance_idx];

        // Get the function type to determine number of results
        let func = instance
            .module
            .get_function(func_idx)
            .ok_or(Error::InvalidFunctionIndex(func_idx as usize))?;
        let func_type = instance
            .module
            .get_function_type(func.type_idx)
            .ok_or_else(|| {
                Error::InvalidFunctionType(format!(
                    "Function type not found for index {}",
                    func.type_idx
                ))
            })?;

        let result_count = func_type.results.len();

        println!(
            "DEBUG: Function type has {} parameters and {} results",
            func_type.params.len(),
            result_count
        );

        // Use the module from the ModuleInstance
        let mut frame = StacklessFrame::from_function(
            Arc::new(instance.module.clone()),
            func_idx,
            &args,
            instance_idx as u32,
        )?;

        // Get a concrete stack implementation for execution
        let mut stack = Vec::<Value>::new();

        // Execute the frame with our concrete stack
        // frame.execute(&mut stack)?; // << COMMENTED OUT - Method doesn't exist, needs revisit
        println!(
            "WARN: StacklessFrame::execute commented out in wrt/src/lib.rs - execution logic needs revisit"
        );

        println!(
            "DEBUG: After execution, stack has {} values: {:?}",
            stack.len(),
            stack
        );

        // Take the top 'result_count' values from the stack as our results
        let mut results = Vec::with_capacity(result_count);

        // Make sure we have enough values on the stack
        if stack.len() < result_count {
            return Err(Error::Execution(format!(
                "Function did not produce enough results. Expected {}, got {}",
                result_count,
                stack.len()
            )));
        }

        // Return the appropriate number of results
        if result_count > 0 {
            // Take values from the end of the stack (most recently pushed)
            let start_index = stack.len() - result_count;
            for i in 0..result_count {
                results.push(stack[start_index + i].clone());
            }
        }

        println!("DEBUG: Returning {} results: {:?}", results.len(), results);

        Ok(results)
    }
}
