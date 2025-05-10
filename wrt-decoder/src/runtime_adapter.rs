//! Adapter for runtime integration with the WRT decoder.
//!
//! This module provides traits and structures to facilitate the integration of the
//! WRT decoder with various runtime environments. It defines interfaces for
//! accessing decoded module information and for converting decoder-specific types
//! into runtime-compatible representations.

use crate::prelude::*; // Ensure prelude is used
// use alloc::string::String; // Should come from prelude
// use alloc::vec::Vec; // Should come from prelude
// use alloc::sync::Arc; // Should come from prelude

use crate::module::{CodeSection, Module as DecoderModule};
use wrt_error::{codes, Error, ErrorCategory, Result};
// Remove direct imports from wrt_format if builder now takes wrt_types
// use wrt_format::module::{Data, Element, Export, Global, Import};
// use wrt_format::section::CustomSection;

// These are already wrt_types::types due to the `use` below
use wrt_types::types::{
    FuncType, // Already wrt_types::types::FuncType
    MemoryType, // Already wrt_types::types::MemoryType
    TableType, // Already wrt_types::types::TableType
    Import as WrtImport, // Alias for clarity
    GlobalType as WrtGlobalType, // Alias for clarity
    Export as WrtExport, // Alias for clarity
    ElementSegment as WrtElementSegment, // Alias for clarity
    DataSegment as WrtDataSegment, // Alias for clarity
    CustomSection as WrtCustomSection, // Alias for clarity
};

/// Convert a decoder module to a runtime module structure
///
/// This function converts a module decoded by wrt-decoder to a structure
/// that can be used by the runtime system, handling all the necessary type
/// conversions and safety checks.
///
/// # Arguments
///
/// * `decoder_module` - The decoded module to convert
///
/// # Returns
///
/// A Result containing the runtime module structure
pub fn convert_to_runtime_module<B>(decoder_module: &DecoderModule) -> Result<B::Module>
where
    B: RuntimeModuleBuilder,
{
    let mut builder = B::new()?;
    
    // Set module name if available
    if let Some(name) = &decoder_module.name {
        builder.set_name(name.clone())?;
    }
    
    // Set start function if available
    if let Some(start) = decoder_module.start {
        builder.set_start(start)?;
    }
    
    // Add types
    for ty in &decoder_module.types {
        builder.add_type(ty.clone())?;
    }
    
    // Add imports
    for import in &decoder_module.imports {
        builder.add_import(import.clone())?;
    }
    
    // Add functions
    for func_idx in &decoder_module.functions {
        builder.add_function(*func_idx)?;
    }
    
    // Add tables - get TableType for each table
    for table in &decoder_module.tables {
        builder.add_table(table.clone())?;
    }
    
    // Add memories - get MemoryType for each memory
    for memory in &decoder_module.memories {
        builder.add_memory(memory.clone())?;
    }
    
    // Add globals
    for global in &decoder_module.globals {
        builder.add_global(global.clone())?;
    }
    
    // Add exports
    for export in &decoder_module.exports {
        builder.add_export(export.clone())?;
    }
    
    // Add elements
    for element in &decoder_module.elements {
        builder.add_element(element.clone())?;
    }
    
    // Add function bodies
    for (i, body) in decoder_module.code.iter().enumerate() {
        // Make sure we have a corresponding function type
        if i >= decoder_module.functions.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Function body without corresponding type: {}", i),
            ));
        }
        
        let type_idx = decoder_module.functions[i];
        builder.add_function_body(i as u32, type_idx, body.clone())?;
    }
    
    // Add data segments
    for data in &decoder_module.data {
        builder.add_data(data.clone())?;
    }
    
    // Add custom sections
    for section in &decoder_module.custom_sections {
        builder.add_custom_section(section.clone())?;
    }
    
    // Build the final module
    builder.build()
}

/// Trait for building runtime modules from decoder modules
///
/// This trait defines the interface for building runtime modules from
/// decoder modules, allowing different runtime implementations to provide
/// their own conversion logic.
pub trait RuntimeModuleBuilder {
    /// The type of module being built
    type Module;
    
    /// Create a new module builder
    fn new() -> Result<Self> where Self: Sized;
    
    /// Set the module name
    fn set_name(&mut self, name: String) -> Result<()>;
    
    /// Set the start function
    fn set_start(&mut self, start: u32) -> Result<()>;
    
    /// Add a function type
    fn add_type(&mut self, ty: FuncType) -> Result<()>;
    
    /// Add an import
    fn add_import(&mut self, import: WrtImport) -> Result<()>;
    
    /// Add a function
    fn add_function(&mut self, type_idx: u32) -> Result<()>;
    
    /// Add a table
    fn add_table(&mut self, table: TableType) -> Result<()>;
    
    /// Add a memory
    fn add_memory(&mut self, memory: MemoryType) -> Result<()>;
    
    /// Add a global
    fn add_global(&mut self, global: WrtGlobalType) -> Result<()>;
    
    /// Add an export
    fn add_export(&mut self, export: WrtExport) -> Result<()>;
    
    /// Add an element segment
    fn add_element(&mut self, element: WrtElementSegment) -> Result<()>;
    
    /// Add a function body
    fn add_function_body(&mut self, func_idx: u32, type_idx: u32, body: CodeSection) -> Result<()>;
    
    /// Add a data segment
    fn add_data(&mut self, data: WrtDataSegment) -> Result<()>;
    
    /// Add a custom section
    fn add_custom_section(&mut self, section: WrtCustomSection) -> Result<()>;
    
    /// Build the final module
    fn build(self) -> Result<Self::Module>;
}

/// Decodes a WebAssembly module from bytes and uses a `RuntimeAdapter` to build it.
// ... existing code ... 