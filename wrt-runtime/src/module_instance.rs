//! Module instance implementation for WebAssembly runtime
//!
//! This module provides the implementation of a WebAssembly module instance,
//! which represents a runtime instance of a WebAssembly module with its own
//! memory, tables, globals, and functions.

extern crate alloc;

#[cfg(feature = "debug-full")]
use wrt_debug::FunctionInfo;
#[cfg(feature = "debug")]
use wrt_debug::{DwarfDebugInfo, LineInfo};

use crate::{global::Global, memory::Memory, module::{Module, MemoryWrapper, TableWrapper, GlobalWrapper}, prelude::{Debug, DefaultProvider, Error, ErrorCategory, FuncType, Result, codes}, table::Table};
use wrt_foundation::traits::BoundedCapacity;
use wrt_instructions::reference_ops::ReferenceOperations;

// Type alias for FuncType to make signatures more readable - matches PlatformProvider from module.rs
type WrtFuncType = wrt_foundation::types::FuncType<wrt_foundation::safe_memory::NoStdProvider<8192>>;

// Platform sync primitives - use prelude imports for consistency
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};
#[cfg(not(feature = "std"))]
use crate::prelude::{Arc, Mutex};

// Import format! macro for string formatting
#[cfg(feature = "std")]
use std::format;
#[cfg(not(feature = "std"))]
use alloc::format;

/// Represents a runtime instance of a WebAssembly module
#[derive(Debug)]
pub struct ModuleInstance {
    /// The module this instance was instantiated from
    module: Arc<Module>,
    /// The instance's memory (using safety-critical wrapper types)
    memories: Arc<Mutex<wrt_foundation::bounded::BoundedVec<MemoryWrapper, 64, wrt_foundation::safe_memory::NoStdProvider<1024>>>>,
    /// The instance's tables (using safety-critical wrapper types)
    tables: Arc<Mutex<wrt_foundation::bounded::BoundedVec<TableWrapper, 64, wrt_foundation::safe_memory::NoStdProvider<1024>>>>,
    /// The instance's globals (using safety-critical wrapper types)
    globals: Arc<Mutex<wrt_foundation::bounded::BoundedVec<GlobalWrapper, 256, wrt_foundation::safe_memory::NoStdProvider<1024>>>>,
    /// Instance ID for debugging
    instance_id: usize,
    /// Imported instance indices to resolve imports
    imports: wrt_format::HashMap<wrt_foundation::bounded::BoundedString<128, wrt_foundation::safe_memory::NoStdProvider<1024>>, wrt_format::HashMap<wrt_foundation::bounded::BoundedString<128, wrt_foundation::safe_memory::NoStdProvider<1024>>, (usize, usize)>>,
    /// Debug information (optional)
    #[cfg(feature = "debug")]
    debug_info: Option<DwarfDebugInfo<'static>>,
}

impl ModuleInstance {
    /// Create a new module instance from a module
    pub fn new(module: Module, instance_id: usize) -> Result<Self> {
        // Allocate memory for memories collection
        let memories_provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
        let memories_vec = wrt_foundation::bounded::BoundedVec::new(memories_provider)?;
        
        // Allocate memory for tables collection
        let tables_provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
        let tables_vec = wrt_foundation::bounded::BoundedVec::new(tables_provider)?;
        
        // Allocate memory for globals collection
        let globals_provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
        let globals_vec = wrt_foundation::bounded::BoundedVec::new(globals_provider)?;
        
        Ok(Self {
            module: Arc::new(module),
            memories: Arc::new(Mutex::new(memories_vec)),
            tables: Arc::new(Mutex::new(tables_vec)),
            globals: Arc::new(Mutex::new(globals_vec)),
            instance_id,
            imports: Default::default(),
            #[cfg(feature = "debug")]
            debug_info: None,
        })
    }

    /// Get the module associated with this instance
    #[must_use] pub fn module(&self) -> &Arc<Module> {
        &self.module
    }

    /// Get a memory from this instance
    pub fn memory(&self, idx: u32) -> Result<MemoryWrapper> {
        #[cfg(feature = "std")]
        let memories = self
            .memories
            .lock()
            .map_err(|_| Error::runtime_error("Failed to lock memories"))?;
        
        #[cfg(not(feature = "std"))]
        let memories = self.memories.lock();

        let memory = memories
            .get(idx as usize)
            .map_err(|_| Error::runtime_execution_error("))?;
        Ok(memory.clone())
    }

    /// Get a table from this instance
    pub fn table(&self, idx: u32) -> Result<TableWrapper> {
        #[cfg(feature = ")]
        let tables = self
            .tables
            .lock()
            .map_err(|_| Error::runtime_error("Failed to lock tables"))?;
        
        #[cfg(not(feature = "std"))]
        let tables = self.tables.lock();

        let table = tables
            .get(idx as usize)
            .map_err(|_| Error::resource_table_not_found("Runtime operation error"))?;
        Ok(table.clone())
    }

    /// Get a global from this instance
    pub fn global(&self, idx: u32) -> Result<GlobalWrapper> {
        #[cfg(feature = "std")]
        let globals = self
            .globals
            .lock()
            .map_err(|_| Error::runtime_error("Failed to lock globals"))?;
        
        #[cfg(not(feature = "std"))]
        let globals = self.globals.lock();

        let global = globals
            .get(idx as usize)
            .map_err(|_| Error::resource_global_not_found("Runtime operation error"))?;
        Ok(global.clone())
    }

    /// Get the function type for a function
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn function_type(&self, idx: u32) -> Result<crate::prelude::CoreFuncType> {
        let function = self.module.functions.get(idx as usize).map_err(|_| {
            Error::runtime_function_not_found("Function index not found")
        })?;

        let ty = self.module.types.get(function.type_idx as usize).map_err(|_| {
            Error::validation_type_mismatch("Type index not found")
        })?;

        // Convert from provider-aware FuncType to clean CoreFuncType
        // Create BoundedVecs manually since FromIterator isn't implemented
        let params_slice = ty.params.as_slice().map_err(|_| Error::runtime_error("Failed to access params"))?;
        let results_slice = ty.results.as_slice().map_err(|_| Error::runtime_error("Failed to access results"))?;
        
        let mut params = wrt_foundation::bounded::BoundedVec::<wrt_foundation::ValueType, 128, crate::memory_adapter::StdMemoryProvider>::new(
            crate::memory_adapter::StdMemoryProvider::default()
        ).map_err(|_| Error::memory_error("Failed to create params vec"))?;
        
        let mut results = wrt_foundation::bounded::BoundedVec::<wrt_foundation::ValueType, 128, crate::memory_adapter::StdMemoryProvider>::new(
            crate::memory_adapter::StdMemoryProvider::default()
        ).map_err(|_| Error::memory_error("Failed to create results vec"))?;
        
        for param in params_slice {
            params.push(param.clone()).map_err(|_| Error::capacity_exceeded("Too many params"))?;
        }
        
        for result in results_slice {
            results.push(result.clone()).map_err(|_| Error::capacity_exceeded("Too many results"))?;
        }
        
        Ok(crate::prelude::CoreFuncType {
            params,
            results,
        })
    }

    /// Get the function type for a function (no_std version)
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    pub fn function_type(&self, idx: u32) -> Result<WrtFuncType> {
        let function = self.module.functions.get(idx as usize).map_err(|_| {
            Error::runtime_function_not_found("Function index not found")
        })?;

        let ty = self.module.types.get(function.type_idx as usize).map_err(|_| {
            Error::validation_type_mismatch("Type index not found")
        })?;

        Ok(ty.clone())
    }

    /// Add a memory to this instance
    pub fn add_memory(&self, memory: Memory) -> Result<()> {
        #[cfg(feature = "std")]
        let mut memories = self
            .memories
            .lock()
            .map_err(|_| Error::runtime_error("Failed to lock memories"))?;
        
        #[cfg(not(feature = "std"))]
        let mut memories = self.memories.lock();

        memories.push(MemoryWrapper::new(memory))
            .map_err(|_| Error::capacity_exceeded("Memory capacity exceeded"))?;
        Ok(())
    }

    /// Add a table to this instance
    pub fn add_table(&self, table: Table) -> Result<()> {
        #[cfg(feature = "std")]
        let mut tables = self
            .tables
            .lock()
            .map_err(|_| Error::runtime_error("Failed to lock tables"))?;
        
        #[cfg(not(feature = "std"))]
        let mut tables = self.tables.lock();

        tables.push(TableWrapper::new(table))
            .map_err(|_| Error::capacity_exceeded("Table capacity exceeded"))?;
        Ok(())
    }

    /// Add a global to this instance
    pub fn add_global(&self, global: Global) -> Result<()> {
        #[cfg(feature = "std")]
        let mut globals = self
            .globals
            .lock()
            .map_err(|_| Error::runtime_error("Failed to lock globals"))?;
        
        #[cfg(not(feature = "std"))]
        let mut globals = self.globals.lock();

        globals.push(GlobalWrapper::new(global))
            .map_err(|_| Error::capacity_exceeded("Global capacity exceeded"))?;
        Ok(())
    }

    /// Initialize debug information for this instance
    #[cfg(feature = "debug")]
    pub fn init_debug_info(&mut self, module_bytes: &'static [u8]) -> Result<()> {
        let mut debug_info = DwarfDebugInfo::new(module_bytes);

        // TODO: Extract debug section offsets from the module
        // For now, this is a placeholder that would need module parsing integration

        self.debug_info = Some(debug_info);
        Ok(())
    }

    /// Get line information for a given program counter
    #[cfg(feature = "debug")]
    pub fn get_line_info(&mut self, pc: u32) -> Result<Option<LineInfo>> {
        if let Some(ref mut debug_info) = self.debug_info {
            debug_info
                .find_line_info(pc)
                .map_err(|e| Error::runtime_debug_info_error("Runtime operation error"))
        } else {
            Ok(None)
        }
    }

    /// Get function information for a given program counter
    #[cfg(feature = "debug-full")]
    pub fn get_function_info(&self, pc: u32) -> Option<&FunctionInfo> {
        self.debug_info.as_ref()?.find_function_info(pc)
    }

    /// Check if debug information is available
    #[cfg(feature = "debug")]
    pub fn has_debug_info(&self) -> bool {
        self.debug_info.as_ref().map_or(false, |di| di.has_debug_info())
    }

    /// Get a function by index - alias for compatibility with tail_call.rs
    pub fn get_function(&self, idx: usize) -> Result<crate::module::Function> {
        self.module.functions.get(idx).map_err(|_| {
            Error::runtime_function_not_found("Function index not found")
        })
    }

    /// Get function type by index - alias for compatibility with tail_call.rs  
    pub fn get_function_type(&self, idx: usize) -> Result<WrtFuncType> {
        let function = self.module.functions.get(idx).map_err(|_| {
            Error::runtime_function_not_found("Function index not found")
        })?;

        self.module.types.get(function.type_idx as usize).map_err(|_| {
            Error::validation_type_mismatch("Type index not found")
        })
    }

    /// Get a table by index - alias for compatibility with tail_call.rs
    pub fn get_table(&self, idx: usize) -> Result<TableWrapper> {
        self.table(idx as u32)
    }

    /// Get a type by index - alias for compatibility with tail_call.rs
    pub fn get_type(&self, idx: usize) -> Result<WrtFuncType> {
        self.module.types.get(idx).map_err(|_| {
            Error::validation_type_mismatch("Type index not found")
        })
    }
}

/// Implementation of ReferenceOperations trait for ModuleInstance
impl ReferenceOperations for ModuleInstance {
    fn get_function(&self, function_index: u32) -> Result<Option<u32>> {
        // Check if function exists in module
        if (function_index as usize) < self.module.functions.len() {
            Ok(Some(function_index))
        } else {
            Ok(None)
        }
    }

    fn validate_function_index(&self, function_index: u32) -> Result<()> {
        if (function_index as usize) < self.module.functions.len() {
            Ok(())
        } else {
            Err(Error::runtime_function_not_found("Function index out of bounds"))
        }
    }
}

// Implement the ModuleInstance trait for module_instance - temporarily disabled
// impl crate::stackless::extensions::ModuleInstance for ModuleInstance {
    // fn module(&self) -> &Module {
    //     &self.module
    // }

    // fn memory(&self, idx: u32) -> Result<MemoryWrapper> {
    //     self.memory(idx)
    // }

    // fn table(&self, idx: u32) -> Result<TableWrapper> {
    //     self.table(idx)
    // }

    // fn global(&self, idx: u32) -> Result<GlobalWrapper> {
    //     self.global(idx)
    // }

    // fn function_type(&self, idx: u32) -> Result<FuncType> {
    //     self.function_type(idx)
    // }
// } // End of commented impl block
