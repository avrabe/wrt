use crate::error::{Error, Result};
use crate::instructions::{BlockType, Instruction};
use crate::logging::{CallbackRegistry, LogLevel, LogOperation};
use crate::module::{Function, Module};
use crate::values::Value;
use crate::{format, Box, Vec};

#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "std"))]
use crate::Mutex;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;

#[cfg(feature = "serialization")]
use crate::serialization::{
    Serializable, SerializableExecutionState, SerializableFrame, 
    SerializableGlobal, SerializableMemory, SerializableModuleInstance, 
    SerializableStack, SerializableState, SerializableTable
};
#[cfg(feature = "serialization")]
use std::collections::HashSet;

/// The WebAssembly execution engine
#[derive(Debug)]
pub struct Engine {
    /// Execution stack
    stack: Stack,
    /// Module instances
    pub instances: Vec<ModuleInstance>,
    /// Remaining fuel for bounded execution
    fuel: Option<u64>,
    /// Current execution state
    state: ExecutionState,
    /// Execution statistics
    stats: ExecutionStats,
    /// Callback registry for host functions (logging, etc.)
    callbacks: Arc<Mutex<CallbackRegistry>>,
    /// Module
    module: Module,
    /// Memories
    memories: Vec<Memory>,
    /// Tables
    tables: Vec<Table>,
    /// Globals
    globals: Vec<Global>,
    /// Functions
    functions: Vec<Function>,
    /// Function imports
    function_imports: Vec<Function>,
    /// Maximum call depth
    max_call_depth: Option<usize>,
    /// Polling loop counter
    polling_loop_counter: usize,
    /// Dropped elements
    dropped_elems: HashSet<(u32, u32)>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Creates a new execution engine
    pub fn new(module: Module) -> Self {
        Self {
            stack: Stack::new(),
            instances: Vec::new(),
            fuel: None, // No fuel limit by default
            state: ExecutionState::Idle,
            stats: ExecutionStats::default(),
            callbacks: Arc::new(Mutex::new(CallbackRegistry::default())),
            module,
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            function_imports: Vec::new(),
            max_call_depth: None,
            polling_loop_counter: 0,
            dropped_elems: HashSet::new(),
        }
    }

    /// Get the callback registry
    pub fn callbacks(&self) -> Arc<Mutex<CallbackRegistry>> {
        self.callbacks.clone()
    }

    /// Register a log handler
    pub fn register_log_handler<F>(&self, handler: F)
    where
        F: Fn(LogOperation) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.register_log_handler(handler);
        }
    }

    /// Sets the fuel limit for bounded execution
    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
    }

    /// Returns the current amount of remaining fuel
    pub fn remaining_fuel(&self) -> Option<u64> {
        self.fuel
    }

    /// Returns the current execution state
    pub fn state(&self) -> &ExecutionState {
        &self.state
    }

    /// Returns the current execution statistics
    pub fn stats(&self) -> &ExecutionStats {
        &self.stats
    }
}

#[cfg(feature = "serialization")]
impl Serializable for Engine {
    fn to_serializable(&self) -> Result<SerializableState> {
        // Get the module binary
        let module_binary = self.module.binary().to_vec();
        
        // Convert execution state
        let state = SerializableExecutionState::from(self.state());
        
        // Convert stack
        let stack = SerializableStack::from(&self.stack);
        
        // Convert instances
        let instances = self.instances.iter()
            .map(SerializableModuleInstance::from)
            .collect();
        
        // Convert memories
        let memories = self.memories.iter()
            .map(SerializableMemory::from)
            .collect();
        
        // Convert tables
        let tables = self.tables.iter()
            .map(SerializableTable::from)
            .collect();
        
        // Convert globals
        let globals = self.globals.iter()
            .map(SerializableGlobal::from)
            .collect();
        
        // Convert dropped_elems
        let dropped_elems = self.dropped_elems.iter().cloned().collect();
        
        Ok(SerializableState {
            state,
            stack,
            instances,
            fuel: self.fuel,
            stats: self.stats.clone(),
            module_binary,
            memories,
            tables,
            globals,
            dropped_elems,
        })
    }
    
    fn from_serializable(state: SerializableState) -> Result<Self> {
        // First, load the module from binary
        let mut module = Module::new();
        module.load_from_binary(&state.module_binary)?;
        
        // Create a new engine with the module
        let mut engine = Engine::new(module);
        
        // Set the execution state
        engine.state = ExecutionState::from(state.state);
        
        // Set the fuel
        engine.fuel = state.fuel;
        
        // Set the stats
        engine.stats = state.stats;
        
        // Reconstruct the stack
        engine.stack = Stack::from(&state.stack);
        
        // Reconstruct module instances (structures only, not connections)
        engine.instances = state.instances.iter()
            .map(|inst| ModuleInstance {
                name: inst.name.clone(),
                memories: inst.memories.clone(),
                tables: inst.tables.clone(),
                globals: inst.globals.clone(),
                functions: inst.functions.clone(),
            })
            .collect();
        
        // Reconstruct memories
        engine.memories = state.memories.iter()
            .map(Memory::from)
            .collect();
        
        // Reconstruct tables
        engine.tables = state.tables.iter()
            .map(Table::from)
            .collect();
        
        // Reconstruct globals
        engine.globals = state.globals.iter()
            .map(Global::from)
            .collect();
        
        // Reconstruct dropped_elems
        engine.dropped_elems = state.dropped_elems.into_iter().collect();
        
        Ok(engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = Engine::new(Module::default());
        // Engine starts with empty stack and no instances
        assert!(engine.instances.is_empty());
    }

    #[test]
    fn test_engine_creation_and_fuel() {
        let mut engine = Engine::new(Module::default());

        // Test initial state
        assert!(matches!(engine.state(), ExecutionState::Idle));
        assert_eq!(engine.remaining_fuel(), None);

        // Test fuel management
        engine.set_fuel(Some(1000));
        assert_eq!(engine.remaining_fuel(), Some(1000));

        engine.set_fuel(None);
        assert_eq!(engine.remaining_fuel(), None);
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = Engine::new(Module::default());

        // Test initial stats
        let stats = engine.stats();
        assert_eq!(stats.instructions_executed, 0);
        assert_eq!(stats.fuel_consumed, 0);
        assert_eq!(stats.peak_memory_bytes, 0);
        assert_eq!(stats.current_memory_bytes, 0);
        assert_eq!(stats.function_calls, 0);
        assert_eq!(stats.memory_operations, 0);

        // Test stats reset
        engine.reset_stats();
        let stats = engine.stats();
        assert_eq!(stats.instructions_executed, 0);
    }

    #[test]
    fn test_engine_callbacks() {
        let engine = Engine::new(Module::default());
        let callbacks = engine.callbacks();

        // Test log handler registration with correct LogOperation type
        engine.register_log_handler(|_op: LogOperation| {
            // Do nothing, just verify we can register a handler
        });
    }
} 