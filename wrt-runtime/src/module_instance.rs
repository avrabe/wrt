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
    pub fn new(module: Module, instance_id: usize) -> Self {
        Self {
            module: Arc::new(module),
            memories: Arc::new(Mutex::new(wrt_foundation::bounded::BoundedVec::new(wrt_foundation::safe_memory::NoStdProvider::<1024>::default()).unwrap())),
            tables: Arc::new(Mutex::new(wrt_foundation::bounded::BoundedVec::new(wrt_foundation::safe_memory::NoStdProvider::<1024>::default()).unwrap())),
            globals: Arc::new(Mutex::new(wrt_foundation::bounded::BoundedVec::new(wrt_foundation::safe_memory::NoStdProvider::<1024>::default()).unwrap())),
            instance_id,
            imports: Default::default(),
            #[cfg(feature = "debug")]
            debug_info: None,
        }
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
            .map_err(|_| Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Failed to lock memories"))?;
        
        #[cfg(not(feature = "std"))]
        let memories = self.memories.lock();

        let memory = memories
            .get(idx as usize)
            .map_err(|_| Error::new(ErrorCategory::Resource, codes::MEMORY_NOT_FOUND, "Runtime operation error"))?;
        Ok(memory.clone())
    }

    /// Get a table from this instance
    pub fn table(&self, idx: u32) -> Result<TableWrapper> {
        #[cfg(feature = "std")]
        let tables = self
            .tables
            .lock()
            .map_err(|_| Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Failed to lock tables"))?;
        
        #[cfg(not(feature = "std"))]
        let tables = self.tables.lock();

        let table = tables
            .get(idx as usize)
            .map_err(|_| Error::new(ErrorCategory::Resource, codes::TABLE_NOT_FOUND, "Runtime operation error"))?;
        Ok(table.clone())
    }

    /// Get a global from this instance
    pub fn global(&self, idx: u32) -> Result<GlobalWrapper> {
        #[cfg(feature = "std")]
        let globals = self
            .globals
            .lock()
            .map_err(|_| Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Failed to lock globals"))?;
        
        #[cfg(not(feature = "std"))]
        let globals = self.globals.lock();

        let global = globals
            .get(idx as usize)
            .map_err(|_| Error::new(ErrorCategory::Resource, codes::GLOBAL_NOT_FOUND, "Runtime operation error"))?;
        Ok(global.clone())
    }

    /// Get the function type for a function
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn function_type(&self, idx: u32) -> Result<crate::prelude::CoreFuncType> {
        let function = self.module.functions.get(idx as usize).map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::FUNCTION_NOT_FOUND, "Function index not found")
        })?;

        let ty = self.module.types.get(function.type_idx as usize).map_err(|_| {
            Error::new(ErrorCategory::Validation, codes::TYPE_MISMATCH, "Type index not found")
        })?;

        // Convert from provider-aware FuncType to clean CoreFuncType
        Ok(crate::prelude::CoreFuncType {
            params: ty.params.iter().collect(),
            results: ty.results.iter().collect(),
        })
    }

    /// Get the function type for a function (no_std version)
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    pub fn function_type(&self, idx: u32) -> Result<wrt_foundation::types::FuncType> {
        let function = self.module.functions.get(idx as usize).map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::FUNCTION_NOT_FOUND, "Function index not found")
        })?;

        let ty = self.module.types.get(function.type_idx as usize).map_err(|_| {
            Error::new(ErrorCategory::Validation, codes::TYPE_MISMATCH, "Type index not found")
        })?;

        Ok(ty.clone())
    }

    /// Add a memory to this instance
    pub fn add_memory(&self, memory: Memory) -> Result<()> {
        #[cfg(feature = "std")]
        let mut memories = self
            .memories
            .lock()
            .map_err(|_| Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Failed to lock memories"))?;
        
        #[cfg(not(feature = "std"))]
        let mut memories = self.memories.lock();

        memories.push(MemoryWrapper::new(memory))
            .map_err(|_| Error::new(ErrorCategory::Memory, codes::CAPACITY_EXCEEDED, "Memory capacity exceeded"))?;
        Ok(())
    }

    /// Add a table to this instance
    pub fn add_table(&self, table: Table) -> Result<()> {
        #[cfg(feature = "std")]
        let mut tables = self
            .tables
            .lock()
            .map_err(|_| Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Failed to lock tables"))?;
        
        #[cfg(not(feature = "std"))]
        let mut tables = self.tables.lock();

        tables.push(TableWrapper::new(table))
            .map_err(|_| Error::new(ErrorCategory::Memory, codes::CAPACITY_EXCEEDED, "Table capacity exceeded"))?;
        Ok(())
    }

    /// Add a global to this instance
    pub fn add_global(&self, global: Global) -> Result<()> {
        #[cfg(feature = "std")]
        let mut globals = self
            .globals
            .lock()
            .map_err(|_| Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "Failed to lock globals"))?;
        
        #[cfg(not(feature = "std"))]
        let mut globals = self.globals.lock();

        globals.push(GlobalWrapper::new(global))
            .map_err(|_| Error::new(ErrorCategory::Memory, codes::CAPACITY_EXCEEDED, "Global capacity exceeded"))?;
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
                .map_err(|e| Error::new(ErrorCategory::Runtime, codes::DEBUG_INFO_ERROR, "Runtime operation error"))
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
