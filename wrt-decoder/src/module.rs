//! WebAssembly module representation
//!
//! This module provides a high-level representation of a WebAssembly module,
//! including all its sections, types, and functions.
//!
//! It serves as the bridge between the binary format (handled by wrt-format)
//! and the runtime representation (using wrt-types).

use wrt_error::{codes, Error, ErrorCategory, Result};
use wrt_format::binary::{WASM_MAGIC, WASM_VERSION};
use wrt_types::{
    safe_memory::SafeSlice,
    types::{
        // Import the canonical Module, Code, Expr, LocalEntry from wrt_types
        Module as WrtModule, Code as WrtCode, Expr as WrtExpr, LocalEntry as WrtLocalEntry,
        FuncType, TableType, MemoryType, GlobalType, ElementSegment, DataSegment, Import, Export,
        CustomSection as WrtCustomSection,
        ImportDesc as TypesImportDesc,
        DataMode as TypesDataMode,
        ElementMode as TypesElementMode,
        ExportDesc as TypesExportDesc,
        TypeIdx, // Added TypeIdx for funcs field
        ValueType, // For LocalEntry
    },
    values::Value,
};

use crate::prelude::*;
use crate::{Parser, instructions}; // Import instructions module

// Import DataMode directly to avoid reimport issues
// pub use wrt_format::module::DataMode as FormatDataMode; // This might be unused after refactor.

/// Module struct representing a parsed WebAssembly module.
/// This struct will now mirror wrt_types::types::Module's relevant fields for the output.
/// The internal parsing function `parse_module` will construct an instance of `WrtModule`.
/// The struct defined here is effectively a placeholder for the type `WrtModule`.
//
// Instead of redefining Module here, the functions that return `Module` will return `WrtModule`.
// The local `struct Module` will be removed.

// Functions like `decode_module` will now return `Result<WrtModule>`
// Functions like `encode_module` will take `&WrtModule`

// Default impl for WrtModule might be better in wrt-types or not needed if constructed by parser.
/*
impl Default for WrtModule { // This should be for WrtModule if needed
    fn default() -> Self {
        Self::new() // WrtModule would need a ::new()
    }
}
*/

// Methods previously on `crate::module::Module` might need to be adapted if they operate
// on fields that have changed structure (e.g., accessing function code).
// For now, focus on the parsing logic in `parse_module_internal_logic` (renamed from `parse_module`).

/// Decode a WebAssembly module from binary format
pub fn decode_module(bytes: &[u8]) -> Result<WrtModule> {
    let parser = Parser::new(Some(bytes), false);
    // The internal parse_module_internal_logic now returns WrtModule
    let (module, _remaining_bytes) = parse_module_internal_logic(parser)?;
    Ok(module)
}

/// Decode a WebAssembly module from binary format and store the original binary
/// (Storing original binary in WrtModule is not standard, might be a specific feature here)
pub fn decode_module_with_binary(binary: &[u8]) -> Result<WrtModule> {
    // This function would need to handle how `binary: Option<SafeSlice<'static>>` is populated
    // if that field is desired on `WrtModule`. `WrtModule` as defined in `wrt-types` does not have it.
    // For now, let's assume `WrtModule` is as defined in `wrt-types`.
    // If `SafeSlice` needs to be part of it, `wrt-types::Module` must be extended.
    // This function might be simplified to just call decode_module for now.
    decode_module(binary)
}

/// Encode a custom section to binary format
///
/// # Arguments
///
/// * `result` - Binary vector to append to
/// * `section` - Custom section to encode
///
/// # Returns
///
/// * `Result<()>` - Success or error
fn encode_custom_section(result: &mut Vec<u8>, section: &WrtCustomSection) -> Result<()> {
    // Write section ID
    result.push(wrt_format::binary::CUSTOM_SECTION_ID);

    // Write section size placeholder (will be filled in later)
    let size_offset = result.len();
    result.extend_from_slice(&[0, 0, 0, 0]); // Placeholder for section size

    // Write name length and name
    write_string(result, &section.name)?;

    // Write section data
    result.extend_from_slice(&section.data);

    // Go back and write the section size
    let section_size = result.len() - size_offset - 4;
    let size_bytes = section_size.to_le_bytes();
    result[size_offset..size_offset + 4].copy_from_slice(&size_bytes);

    Ok(())
}

/// Encode a WebAssembly module to binary format
///
/// # Arguments
///
/// * `module` - Module to encode
///
/// # Returns
///
/// * `Result<Vec<u8>>` - Binary representation of the module
pub fn encode_module(module: &WrtModule) -> Result<Vec<u8>> {
    // This would ideally use SafeMemory types, but for the final binary output
    // we need a Vec<u8> that can be returned as the serialized representation
    let mut result = Vec::new();

    // Write module header
    result.extend_from_slice(&WASM_MAGIC);
    result.extend_from_slice(&WASM_VERSION);

    // For a complete implementation, each section would be encoded here

    // Encode custom sections
    for section in &module.custom_sections {
        encode_custom_section(&mut result, section)?;
    }

    Ok(result)
}

/// Create a parse error with the given message
///
/// # Arguments
///
/// * `message` - Error message
///
/// # Returns
///
/// * `Error` - Parse error
pub fn parse_error(message: &str) -> Error {
    Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, message)
}

/// Create a parse error with the given message and context
///
/// # Arguments
///
/// * `message` - Error message
/// * `context` - Additional context
///
/// # Returns
///
/// * `Error` - Parse error with context
pub fn parse_error_with_context(message: &str, context: &str) -> Error {
    Error::new(
        ErrorCategory::Parse,
        codes::PARSE_ERROR,
        format!("{}: {}", message, context),
    )
}

/// Create a parse error with the given message and position
///
/// # Arguments
///
/// * `message` - Error message
/// * `position` - Position in the binary
///
/// # Returns
///
/// * `Error` - Parse error with position
pub fn parse_error_with_position(message: &str, position: usize) -> Error {
    Error::new(
        ErrorCategory::Parse,
        codes::PARSE_ERROR,
        format!("{} at position {}", message, position),
    )
}

/// Create a runtime error with the given message
///
/// # Arguments
///
/// * `message` - Error message
///
/// # Returns
///
/// * `Error` - Runtime error
pub fn runtime_error(message: &str) -> Error {
    Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, message)
}

/// Create a runtime error with the given message and context
///
/// # Arguments
///
/// * `message` - Error message
/// * `context` - Additional context
///
/// # Returns
///
/// * `Error` - Runtime error with context
pub fn runtime_error_with_context(message: &str, context: &str) -> Error {
    Error::new(
        ErrorCategory::Runtime,
        codes::RUNTIME_ERROR,
        format!("{}: {}", message, context),
    )
}

/// Wrapper for custom sections with additional functionality
#[derive(Debug, Clone)]
pub struct CustomSectionWrapper {
    /// Name of the custom section
    pub name: String,
    /// Data of the custom section
    pub data: Vec<u8>,
}

/// Internal parsing logic that consumes a `crate::parser::Parser`.
/// Renamed from `parse_module` to avoid conflict with the public one if struct Module is removed.
fn parse_module_internal_logic(mut parser: crate::parser::Parser<'_>) -> Result<(WrtModule, Vec<u8>)> {
    let mut mod_types = Vec::new();
    let mut mod_imports = Vec::new();
    let mut mod_funcs = Vec::new(); // Type indices for functions
    let mut mod_tables = Vec::new();
    let mut mod_memories = Vec::new();
    let mut mod_globals = Vec::new();
    let mut mod_exports = Vec::new();
    let mut mod_start = None;
    let mut mod_elements = Vec::new();
    let mut mod_code_entries = Vec::new(); // Will hold WrtCode
    let mut mod_data_segments = Vec::new();
    let mut mod_data_count = None;
    let mut mod_custom_sections = Vec::new();

    let mut remaining_bytes = Vec::new();

    loop {
        match parser.read() {
            Ok(Some(payload)) => {
                match payload {
                    crate::parser::Payload::Version(_version, _bytes) => {}
                    crate::parser::Payload::TypeSection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                            mod_types = crate::sections::parsers::parse_type_section(data)?;
                        } else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from TypeSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::ImportSection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                             mod_imports = crate::sections::parsers::parse_import_section(data)?;
                        }  else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from ImportSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::FunctionSection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                            // This parser returns Vec<u32> directly which is correct for mod_funcs
                            mod_funcs = crate::sections::parsers::parse_function_section(data)?;
                        } else {
                             return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from FunctionSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::TableSection(slice, _size) => {
                         if let Ok(data) = slice.data() {
                            mod_tables = crate::sections::parsers::parse_table_section(data)?;
                        } else {
                             return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from TableSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::MemorySection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                            mod_memories = crate::sections::parsers::parse_memory_section(data)?;
                        } else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from MemorySection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::GlobalSection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                            mod_globals = crate::sections::parsers::parse_global_section(data)?;
                        } else {
                             return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from GlobalSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::ExportSection(slice, _size) => {
                         if let Ok(data) = slice.data() {
                            mod_exports = crate::sections::parsers::parse_export_section(data)?;
                        } else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from ExportSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::StartSection(func_idx) => {
                        mod_start = Some(func_idx);
                    }
                    crate::parser::Payload::ElementSection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                            mod_elements = crate::sections::parsers::parse_element_section(data)?;
                        } else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from ElementSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::CodeSection(slice, _size) => {
                        if let Ok(mut code_section_data) = slice.data() {
                            let (num_functions, mut bytes_read_for_count) = wrt_format::binary::read_leb_u32(code_section_data)?;
                            code_section_data = &code_section_data[bytes_read_for_count..];

                            if num_functions as usize != mod_funcs.len() {
                                return Err(Error::new(ErrorCategory::Validation, codes::VALIDATION_ERROR,
                                    format!("Code section function count {} mismatches function section count {}", num_functions, mod_funcs.len())));
                            }

                            for _ in 0..num_functions {
                                let (func_size, size_len) = wrt_format::binary::read_leb_u32(code_section_data)?;
                                code_section_data = &code_section_data[size_len..];
                                // bytes_read_for_count += size_len; // This counter is not total for section, but per-func

                                if code_section_data.len() < func_size as usize {
                                     return Err(Error::new(ErrorCategory::Parse, codes::DECODE_UNEXPECTED_EOF, "EOF in code section entry"));
                                }
                                let mut func_data_slice = &code_section_data[..func_size as usize];

                                let (locals, locals_len) = instructions::parse_locals(func_data_slice)?;
                                func_data_slice = &func_data_slice[locals_len..];

                                let (instructions_vec, _instr_len) = instructions::parse_instructions(func_data_slice)?;

                                let expr = WrtExpr { instructions: instructions_vec };
                                mod_code_entries.push(WrtCode { locals, body: expr });

                                code_section_data = &code_section_data[func_size as usize..];
                                // bytes_read_for_count += func_size as usize; // This counter is not total for section, but per-func
                            }
                        } else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from CodeSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::DataSection(slice, _size) => {
                        if let Ok(data) = slice.data() {
                            mod_data_segments = crate::sections::parsers::parse_data_section(data)?;
                        } else {
                            return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from DataSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::DataCountSection { count } => {
                        mod_data_count = Some(count);
                    }
                    crate::parser::Payload::CustomSection{ name, data, size: _ } => {
                        if let Ok(data_bytes) = data.data() {
                             mod_custom_sections.push(WrtCustomSection { name, data: data_bytes.to_vec() });
                        } else {
                             return Err(Error::new(ErrorCategory::Parse, codes::DECODE_ERROR, "Failed to get data from CustomSection SafeSlice"));
                        }
                    }
                    crate::parser::Payload::ComponentSection { .. } => {
                         return Err(Error::new(ErrorCategory::Parse, codes::UNSUPPORTED_FEATURE, "Component sections not supported in core module parsing"));
                    }
                    crate::parser::Payload::End => {
                        break;
                    }
                }
            }
            Ok(None) => { break; }
            Err(e) => { return Err(e.add_context(codes::DECODE_ERROR, "Failed to read payload from parser")); }
        }
    }
    Ok((
        WrtModule {
            types: mod_types,
            imports: mod_imports,
            funcs: mod_funcs,
            tables: mod_tables,
            memories: mod_memories,
            globals: mod_globals,
            exports: mod_exports,
            start: mod_start,
            elements: mod_elements,
            code_entries: mod_code_entries,
            data_segments: mod_data_segments,
            data_count: mod_data_count,
            custom_sections: mod_custom_sections,
        },
        remaining_bytes,
    ))
}

/// Helper function to write a string to a binary vector
fn write_string(result: &mut Vec<u8>, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    let len = bytes.len();

    // Write the length as LEB128
    let len_bytes = wrt_format::binary::write_leb128_u32(len as u32);
    result.extend_from_slice(&len_bytes);

    // Write the string
    result.extend_from_slice(bytes);

    Ok(())
}

#[cfg(test)]
mod tests {
    // ... existing code ...
}
