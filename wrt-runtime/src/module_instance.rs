//! Module instance implementation for WebAssembly runtime
//!
//! This module provides the implementation of a WebAssembly module instance,
//! which represents a runtime instance of a WebAssembly module with its own
//! memory, tables, globals, and functions.

use crate::global::Global;
use crate::memory::Memory;
use crate::module::Module;
use crate::prelude::*;
use crate::table::Table;

/// Represents a runtime instance of a WebAssembly module
#[derive(Debug)]
pub struct ModuleInstance {
    /// The module this instance was instantiated from
    module: Arc<Module>,
    /// The instance's memory
    memories: Arc<Mutex<Vec<Arc<Memory>>>>,
    /// The instance's tables
    tables: Arc<Mutex<Vec<Arc<Table>>>>,
    /// The instance's globals
    globals: Arc<Mutex<Vec<Arc<Global>>>>,
    /// Instance ID for debugging
    instance_id: usize,
    /// Imported instance indices to resolve imports
    imports: HashMap<String, HashMap<String, (usize, usize)>>,
}

impl ModuleInstance {
    /// Create a new module instance from a module
    pub fn new(module: Module, instance_id: usize) -> Self {
        Self {
            module: Arc::new(module),
            memories: Arc::new(Mutex::new(Vec::new())),
            tables: Arc::new(Mutex::new(Vec::new())),
            globals: Arc::new(Mutex::new(Vec::new())),
            instance_id,
            imports: HashMap::new(),
        }
    }

    /// Get the module associated with this instance
    pub fn module(&self) -> &Arc<Module> {
        &self.module
    }

    /// Get a memory from this instance
    pub fn memory(&self, idx: u32) -> Result<Arc<Memory>> {
        let memories = self
            .memories
            .lock()
            .map_err(|_| create_simple_runtime_error("Mutex poisoned when accessing memories"))?;
        
        memories
            .get(idx as usize)
            .cloned()
            .ok_or_else(|| create_simple_resource_error(format!("Memory index {} not found", idx)))
    }

    /// Get a table from this instance
    pub fn table(&self, idx: u32) -> Result<Arc<Table>> {
        let tables = self
            .tables
            .lock()
            .map_err(|_| create_simple_runtime_error("Mutex poisoned when accessing tables"))?;
        
        tables
            .get(idx as usize)
            .cloned()
            .ok_or_else(|| create_simple_resource_error(format!("Table index {} not found", idx)))
    }

    /// Get a global from this instance
    pub fn global(&self, idx: u32) -> Result<Arc<Global>> {
        let globals = self
            .globals
            .lock()
            .map_err(|_| create_simple_runtime_error("Mutex poisoned when accessing globals"))?;
        
        globals
            .get(idx as usize)
            .cloned()
            .ok_or_else(|| create_simple_resource_error(format!("Global index {} not found", idx)))
    }

    /// Get the function type for a function
    pub fn function_type(&self, idx: u32) -> Result<FuncType> {
        let function = self
            .module
            .functions
            .get(idx as usize)
            .ok_or_else(|| create_simple_runtime_error(format!("Function index {} not found", idx)))?;
        
        let ty = self
            .module
            .types
            .get(function.type_idx as usize)
            .cloned()
            .ok_or_else(|| {
                create_simple_validation_error(format!(
                    "Type index {} not found",
                    function.type_idx
                ))
            })?;
        
        Ok(ty)
    }

    /// Add a memory to this instance
    pub fn add_memory(&self, memory: Memory) -> Result<()> {
        let mut memories = self
            .memories
            .lock()
            .map_err(|_| create_simple_runtime_error("Mutex poisoned when adding memory"))?;
        
        memories.push(Arc::new(memory));
        Ok(())
    }

    /// Add a table to this instance
    pub fn add_table(&self, table: Table) -> Result<()> {
        let mut tables = self
            .tables
            .lock()
            .map_err(|_| create_simple_runtime_error("Mutex poisoned when adding table"))?;
        
        tables.push(Arc::new(table));
        Ok(())
    }

    /// Add a global to this instance
    pub fn add_global(&self, global: Global) -> Result<()> {
        let mut globals = self
            .globals
            .lock()
            .map_err(|_| create_simple_runtime_error("Mutex poisoned when adding global"))?;
        
        globals.push(Arc::new(global));
        Ok(())
    }
}

// Implement the ModuleInstance trait for module_instance
impl crate::stackless::extensions::ModuleInstance for ModuleInstance {
    fn module(&self) -> &Module {
        &self.module
    }
    
    fn memory(&self, idx: u32) -> Result<Arc<Memory>> {
        self.memory(idx)
    }
    
    fn table(&self, idx: u32) -> Result<Arc<Table>> {
        self.table(idx)
    }
    
    fn global(&self, idx: u32) -> Result<Arc<Global>> {
        self.global(idx)
    }
    
    fn function_type(&self, idx: u32) -> Result<FuncType> {
        self.function_type(idx)
    }
} 