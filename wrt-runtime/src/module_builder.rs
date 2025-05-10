//! Module builder for WebAssembly runtime
//!
//! This module provides an implementation of the RuntimeModuleBuilder trait
//! from wrt-decoder, allowing the conversion of decoder modules to runtime modules.

use crate::module::Module;
use crate::prelude::*;
use wrt_decoder::module::CodeSection;
use wrt_decoder::runtime_adapter::RuntimeModuleBuilder;
use wrt_types::types::{
    FuncType, GlobalType as WrtGlobalType, Limits as WrtLimits, ValueType as WrtValueType,
    TableType as WrtTableType, MemoryType as WrtMemoryType, Import as WrtImport,
    Export as WrtExport, ElementSegment as WrtElementSegment, DataSegment as WrtDataSegment,
    CustomSection as WrtCustomSection, ImportDesc as WrtImportDesc, Value as WrtValue,
};

/// Builder for runtime modules
pub struct ModuleBuilder {
    /// Module being built
    module: Module,
    /// Keep track of imported functions to correctly index defined functions
    imported_func_count: u32,
}

impl RuntimeModuleBuilder for ModuleBuilder {
    type Module = Module;
    
    /// Create a new module builder
    fn new() -> Result<Self> {
        Ok(Self {
            module: Module::new()?,
            imported_func_count: 0,
        })
    }
    
    /// Set the module name
    fn set_name(&mut self, name: String) -> Result<()> {
        self.module.set_name(name)
    }
    
    /// Set the start function
    fn set_start(&mut self, start: u32) -> Result<()> {
        self.module.set_start(start)
    }
    
    /// Add a function type
    fn add_type(&mut self, ty: FuncType) -> Result<()> {
        self.module.add_type(ty)
    }
    
    /// Add an import
    fn add_import(&mut self, import: WrtImport) -> Result<()> {
        match import.desc {
            WrtImportDesc::Function(type_idx) => {
                self.module
                    .add_import_func(&import.module, &import.name, type_idx)?;
                self.imported_func_count += 1;
            }
            WrtImportDesc::Table(table_type) => {
                self.module
                    .add_import_table(&import.module, &import.name, table_type)?;
            }
            WrtImportDesc::Memory(memory_type) => {
                self.module
                    .add_import_memory(&import.module, &import.name, memory_type)?;
            }
            WrtImportDesc::Global(global_import_type) => {
                self.module.add_import_runtime_global(&import.module, &import.name, global_import_type)?;
            }
        }
        Ok(())
    }
    
    /// Add a function
    fn add_function(&mut self, _type_idx: u32) -> Result<()> {
        Ok(())
    }
    
    /// Add a table
    fn add_table(&mut self, table_type: WrtTableType) -> Result<()> {
        self.module.add_table(table_type)
    }
    
    /// Add a memory
    fn add_memory(&mut self, memory_type: WrtMemoryType) -> Result<()> {
        self.module.add_memory(memory_type)
    }
    
    /// Add a global
    fn add_global(&mut self, global: WrtGlobalType) -> Result<()> {
        self.module.add_runtime_global(global.value_type, global.mutable, global.initial_value)?;
        Ok(())
    }
    
    /// Add an export
    fn add_export(&mut self, export: WrtExport) -> Result<()> {
        self.module.add_runtime_export(export)?;
        Ok(())
    }
    
    /// Add an element segment
    fn add_element(&mut self, element: WrtElementSegment) -> Result<()> {
        self.module.add_runtime_element(element)?;
        Ok(())
    }
    
    /// Add a function body
    fn add_function_body(&mut self, func_idx: u32, type_idx: u32, body: CodeSection) -> Result<()> {
        let runtime_func_idx = self.imported_func_count + func_idx;

        let (parsed_locals, _locals_bytes_len) =
            wrt_decoder::instructions::parse_locals(&body.body).map_err(|e| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    format!("Failed to parse locals for func_idx {}: {}", func_idx, e),
                )
            })?;
        
        let instruction_bytes = &body.body[_locals_bytes_len..];

        let (instructions_vec, _instr_len) = 
            wrt_decoder::instructions::parse_instructions(instruction_bytes).map_err(|e| {
                 Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    format!("Failed to parse instructions for func_idx {}: {}", func_idx, e),
                )
            })?;

        self.module.set_function_body(runtime_func_idx, type_idx, parsed_locals, instructions_vec)?;
        Ok(())
    }
    
    /// Add a data segment
    fn add_data(&mut self, data: WrtDataSegment) -> Result<()> {
        self.module.add_runtime_data(data)?;
        Ok(())
    }
    
    /// Add a custom section
    fn add_custom_section(&mut self, section: WrtCustomSection) -> Result<()> {
        self.module.add_custom_section(section)
    }
    
    /// Build the final module
    fn build(mut self) -> Result<Self::Module> {
        Ok(self.module)
    }
}

impl ModuleBuilder {
    /// Create a new module builder with an existing binary
    pub fn with_binary(_binary: Vec<u8>) -> Result<Self> {
        Ok(Self {
            module: Module::new()?,
            imported_func_count: 0,
        })
    }
    
    /// Set the binary representation of the module
    pub fn set_binary(&mut self, _binary: Vec<u8>) -> Result<()> {
        Ok(())
    }
}

/// Load a module from binary data using the module builder
pub fn load_module_from_binary(binary: &[u8]) -> Result<Module> {
    let decoder_module = wrt_decoder::decode_module(binary)?;
    let types_module = wrt_decoder::decode_module(binary)?;
    Module::from_wrt_module(&types_module)
} 