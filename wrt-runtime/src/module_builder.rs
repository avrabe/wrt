//! Module builder for WebAssembly runtime
//!
//! This module provides an implementation of the RuntimeModuleBuilder trait
//! from wrt-decoder, allowing the conversion of decoder modules to runtime
//! modules.

// Decoder imports are optional during development
// use wrt_decoder::{module::CodeSection,
// runtime_adapter::RuntimeModuleBuilder}; alloc is imported in lib.rs with
// proper feature gates

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::format;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
// Import format! macro for string formatting
#[cfg(feature = "std")]
use std::format;
#[cfg(feature = "std")]
use std::vec::Vec;

use wrt_format::{
    DataSegment as WrtDataSegment,
    ElementSegment as WrtElementSegment,
};
use wrt_foundation::{
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    types::{
        CustomSection as WrtCustomSection,
        FuncType,
        GlobalType as WrtGlobalType,
        Limits as WrtLimits,
        MemoryType as WrtMemoryType,
        TableType as WrtTableType,
        ValueType as WrtValueType,
    },
    values::Value as WrtValue,
};

// Add placeholder aliases for missing types
use crate::module::{
    Export as WrtExport,
    Function,
    Import as WrtImport,
    LocalEntry,
    WrtExpr,
};
use crate::{
    bounded_runtime_infra::create_runtime_provider,
    memory_adapter::StdMemoryProvider,
    module::Module,
    prelude::*,
};

// String type for runtime - use std::string::String or BoundedString
#[cfg(feature = "std")]
type String = std::string::String;
#[cfg(not(feature = "std"))]
type String =
    wrt_foundation::bounded::BoundedString<256, crate::bounded_runtime_infra::RuntimeProvider>;

// Define trait locally if not available from wrt_decoder
pub trait RuntimeModuleBuilder {
    type Module;

    fn new() -> Self;
    fn set_name(&mut self, name: String);
    fn set_start(&mut self, start_func: u32);
    fn add_type(&mut self, func_type: FuncType<StdMemoryProvider>) -> Result<u32>;
    fn add_function_type(&mut self, func_type: FuncType<StdMemoryProvider>) -> Result<u32>;
    fn add_import(&mut self, import: WrtImport) -> Result<u32>;
    fn add_function(&mut self, type_idx: u32) -> Result<u32>;
    fn add_function_body(
        &mut self,
        func_idx: u32,
        type_idx: u32,
        body: wrt_foundation::bounded::BoundedVec<
            u8,
            4096,
            crate::bounded_runtime_infra::RuntimeProvider,
        >,
    ) -> Result<()>;
    fn add_memory(&mut self, memory_type: WrtMemoryType) -> Result<u32>;
    fn add_table(&mut self, table_type: WrtTableType) -> Result<u32>;
    fn add_global(&mut self, global_type: WrtGlobalType) -> Result<u32>;
    fn add_export(&mut self, export: WrtExport) -> Result<()>;
    fn add_element(&mut self, element: WrtElementSegment) -> Result<u32>;
    fn add_data(&mut self, data: WrtDataSegment) -> Result<u32>;
    fn add_custom_section(
        &mut self,
        section: WrtCustomSection<crate::bounded_runtime_infra::RuntimeProvider>,
    ) -> Result<()>;
    fn build(self) -> Result<Self::Module>;
}

/// Builder for runtime modules
pub struct ModuleBuilder {
    /// Module being built
    module:              Module,
    /// Keep track of imported functions to correctly index defined functions
    imported_func_count: u32,
}

impl RuntimeModuleBuilder for ModuleBuilder {
    type Module = Module;

    /// Create a new module builder
    fn new() -> Self {
        Self {
            module:              Module::new().unwrap_or_else(|e| {
                // Log the error and panic or handle gracefully
                panic!("Failed to create new module: {:?}", e)
            }),
            imported_func_count: 0,
        }
    }

    fn set_name(&mut self, _name: String) {
        // Name setting not implemented in current Module struct
    }

    fn set_start(&mut self, _start_func: u32) {
        // Start function setting not implemented in current Module struct
    }

    fn add_type(&mut self, func_type: FuncType<StdMemoryProvider>) -> Result<u32> {
        self.add_function_type(func_type)
    }

    fn add_import(&mut self, _import: WrtImport) -> Result<u32> {
        // Import handling not implemented
        self.imported_func_count += 1;
        Ok(self.imported_func_count - 1)
    }

    fn add_function(&mut self, _type_idx: u32) -> Result<u32> {
        // Function addition without body
        Ok(0)
    }

    fn add_export(&mut self, _export: WrtExport) -> Result<()> {
        // Export handling not implemented
        Ok(())
    }

    fn add_element(&mut self, _element: WrtElementSegment) -> Result<u32> {
        // Element segment handling not implemented
        Ok(0)
    }

    fn add_data(&mut self, _data: WrtDataSegment) -> Result<u32> {
        // Data segment handling not implemented
        Ok(0)
    }

    fn add_custom_section(
        &mut self,
        _section: WrtCustomSection<crate::bounded_runtime_infra::RuntimeProvider>,
    ) -> Result<()> {
        // Custom section handling not implemented
        Ok(())
    }

    fn add_function_type(&mut self, _func_type: FuncType<StdMemoryProvider>) -> Result<u32> {
        // Function type addition not implemented
        Ok(0)
    }

    fn add_function_body(
        &mut self,
        func_idx: u32,
        type_idx: u32,
        body: wrt_foundation::bounded::BoundedVec<
            u8,
            4096,
            crate::bounded_runtime_infra::RuntimeProvider,
        >,
    ) -> Result<()> {
        use crate::{
            instruction_parser::parse_instructions,
            module::{
                Function,
                WrtExpr,
            },
        };

        // Convert BoundedVec to slice for parsing
        let bytecode_slice = body.as_slice();

        // For now, create empty function with proper types
        // TODO: Implement proper bytecode parsing with compatible types
        let provider1 = create_runtime_provider()?;
        let provider2 = create_runtime_provider()?;
        let instructions = wrt_foundation::bounded::BoundedVec::new(provider1)?;
        let locals = wrt_foundation::bounded::BoundedVec::new(provider2)?;

        // Create the function with proper types
        let function = Function {
            type_idx,
            locals,
            body: WrtExpr { instructions },
        };

        // Add to module's functions
        // Note: This assumes functions are stored somewhere in the module
        // The actual implementation depends on how Module stores functions

        Ok(())
    }

    fn add_memory(&mut self, _memory_type: WrtMemoryType) -> Result<u32> {
        // Memory addition not implemented
        Ok(0)
    }

    fn add_table(&mut self, _table_type: WrtTableType) -> Result<u32> {
        // Table addition not implemented
        Ok(0)
    }

    fn add_global(&mut self, _global_type: WrtGlobalType) -> Result<u32> {
        // Global addition not implemented
        Ok(0)
    }

    fn build(self) -> Result<Self::Module> {
        // Return the built module
        Ok(self.module)
    }

    // All trait methods implemented above with stub implementations
}

/// Parse local variable declarations from function body bytecode
fn parse_locals_from_body(
    bytecode: &[u8],
) -> Result<
    wrt_foundation::bounded::BoundedVec<
        wrt_foundation::types::LocalEntry,
        64,
        crate::bounded_runtime_infra::RuntimeProvider,
    >,
> {
    use wrt_foundation::{
        bounded::BoundedVec,
        types::LocalEntry,
    };

    let provider = create_runtime_provider()?;
    let mut locals = BoundedVec::new(provider)?;

    if bytecode.is_empty() {
        return Ok(locals);
    }

    let mut offset = 0;

    // Read local count (LEB128)
    let (local_count, consumed) = read_leb128_u32(bytecode, offset)?;
    offset += consumed;

    // Parse each local entry
    for _ in 0..local_count {
        if offset >= bytecode.len() {
            return Err(Error::parse_error(
                "Unexpected end of bytecode while parsing locals",
            ));
        }

        // Read count of this local type
        let (count, consumed) = read_leb128_u32(bytecode, offset)?;
        offset += consumed;

        if offset >= bytecode.len() {
            return Err(Error::parse_error(
                "Unexpected end of bytecode while parsing local type",
            ));
        }

        // Read value type
        let value_type = match bytecode[offset] {
            0x7F => wrt_foundation::types::ValueType::I32,
            0x7E => wrt_foundation::types::ValueType::I64,
            0x7D => wrt_foundation::types::ValueType::F32,
            0x7C => wrt_foundation::types::ValueType::F64,
            _ => return Err(Error::parse_error("Invalid value type for local variable")),
        };
        offset += 1;

        let local_entry = LocalEntry { count, value_type };
        locals.push(local_entry)?;
    }

    Ok(locals)
}

/// Read LEB128 u32 from bytecode
fn read_leb128_u32(bytecode: &[u8], offset: usize) -> Result<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut consumed = 0;

    loop {
        if offset + consumed >= bytecode.len() {
            return Err(Error::parse_error(
                "Unexpected end of bytecode while reading LEB128",
            ));
        }

        let byte = bytecode[offset + consumed];
        consumed += 1;

        result |= ((byte & 0x7F) as u32) << shift;

        if (byte & 0x80) == 0 {
            break;
        }

        shift += 7;
        if shift >= 32 {
            return Err(Error::parse_error("LEB128 value too large"));
        }
    }

    Ok((result, consumed))
}

impl ModuleBuilder {
    /// Create a new module builder with an existing binary
    pub fn with_binary(_binary: Vec<u8>) -> Result<Self> {
        Ok(Self {
            module:              Module::new()?,
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
    #[cfg(all(feature = "decoder"))]
    {
        let decoder_module = wrt_decoder::decode_module(binary)?;
        Module::from_wrt_module(&decoder_module)
    }
    #[cfg(all(not(feature = "decoder"), feature = "std"))]
    {
        // Decoder not available - create an empty module
        Err(Error::runtime_execution_error("Decoder not available"))
    }
    #[cfg(not(feature = "std"))]
    {
        // Basic fallback for no_std - create an empty module
        Err(Error::parse_invalid_binary(
            "Module loading from binary not supported in no_std mode",
        ))
    }
}
