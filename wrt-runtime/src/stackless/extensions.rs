//! Extensions for the stackless WebAssembly execution engine.
//!
//! This module provides extensions and utility functions for the stackless
//! execution engine, supporting both the core WebAssembly specification and
//! the Component Model.

use crate::prelude::*;
use crate::stackless::engine::StacklessEngine;

/// Types that represent a Wasm module instance
pub trait ModuleInstance: Debug {
    /// Get the module associated with this instance
    fn module(&self) -> &RuntimeModule;
    
    /// Get a reference to a memory from this instance
    fn memory(&self, idx: u32) -> Result<Arc<Memory>>;
    
    /// Get a reference to a table from this instance
    fn table(&self, idx: u32) -> Result<Arc<Table>>;
    
    /// Get a reference to a global from this instance
    fn global(&self, idx: u32) -> Result<Arc<Global>>;
    
    /// Get the function type for a function in this instance
    fn function_type(&self, idx: u32) -> Result<FuncType>;
}

// Further implementations will be added in subsequent updates 