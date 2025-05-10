//! WebAssembly Core Module Parser
//!
//! Functions for parsing WebAssembly core modules from binary format.

use crate::Result;
use crate::prelude::*;
use crate::utils;
use wrt_error::{codes, Error, ErrorCategory};
use wrt_format::{
    binary::BinaryFormat,
    module::{Function, Global, Memory, Table, Export, ExportKind, Import, ImportDesc},
    CustomSection, Module, 
    types::{Limits, ValueType, FuncType, CoreWasmVersion},
};

// All collection types are now imported from the prelude

// Section ID constants
const CUSTOM_SECTION_ID: u8 = 0;
const TYPE_SECTION_ID: u8 = 1;
const IMPORT_SECTION_ID: u8 = 2;
const FUNCTION_SECTION_ID: u8 = 3;
const TABLE_SECTION_ID: u8 = 4;
const MEMORY_SECTION_ID: u8 = 5;
const GLOBAL_SECTION_ID: u8 = 6;
const EXPORT_SECTION_ID: u8 = 7;
const START_SECTION_ID: u8 = 8;
const ELEMENT_SECTION_ID: u8 = 9;
const CODE_SECTION_ID: u8 = 10;
const DATA_SECTION_ID: u8 = 11;
const DATA_COUNT_SECTION_ID: u8 = 12;
const TYPE_INFORMATION_SECTION_ID: u8 = 15;

// Custom error codes
const ERROR_INVALID_LENGTH: u16 = codes::PARSE_ERROR;
const ERROR_INVALID_MAGIC: u16 = codes::PARSE_ERROR;
const ERROR_INVALID_VERSION: u16 = codes::PARSE_ERROR;
const ERROR_INVALID_SECTION: u16 = codes::PARSE_ERROR;
const ERROR_INVALID_UTF8: u16 = codes::PARSE_ERROR;
const ERROR_CAPACITY_EXCEEDED: u16 = codes::PARSE_ERROR;

/// Parse a WebAssembly binary module
///
/// This function takes a WebAssembly binary and parses it into a structured
/// Module representation.
///
/// # Arguments
///
/// * `data` - The WebAssembly binary data
///
/// # Returns
///
/// * `Result<Module>` - The parsed module or an error
///
/// # Errors
///
/// Returns an error if the binary cannot be parsed.
pub fn parse_module(data: &[u8]) -> Result<Module> {
    // Verify the WebAssembly binary header
    utils::verify_binary_header(data)?;

    // Store the entire binary data
    let mut module = Module::new();
    // Make a copy of the entire binary
    module.binary = Some(data.to_vec());

    // Explicitly parse and set CoreWasmVersion based on hypothetical F1
    if data.len() >= 8 {
        let version_bytes = [data[4], data[5], data[6], data[7]];
        match CoreWasmVersion::from_bytes(version_bytes) {
            Some(version) => {
                module.core_version = version;
            }
            None => {
                // If CoreWasmVersion::from_bytes returns None, it's an unknown/unsupported version
                // according to our defined CoreWasmVersion enum.
                // We might allow parsing to continue with a default (e.g. V2_0) and warn,
                // or error out. For now, let's error out for strictness.
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_VERSION, // Use existing error code if suitable
                    format!("Unsupported WebAssembly version bytes: {:02X?}", version_bytes),
                ));
            }
        }
    } else {
        return Err(Error::new(
            ErrorCategory::Parse,
            ERROR_INVALID_LENGTH, // Use existing error code if suitable
            "Data too short for WebAssembly header.".to_string(),
        ));
    }

    // Parse the binary contents (skip the magic number and version)
    parse_binary_into_module(&data[8..], &mut module)?;

    Ok(module)
}

/// Parse the WebAssembly binary content after the magic number and version
/// into an existing module
///
/// # Arguments
///
/// * `data` - The WebAssembly binary data after the magic number and version
/// * `module` - The module to fill with parsed data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
///
/// # Errors
///
/// Returns an error if the binary cannot be parsed.
pub fn parse_binary_into_module(data: &[u8], module: &mut Module) -> Result<()> {
    let mut offset = 0;

    // Parse each section
    while offset < data.len() {
        if offset >= data.len() {
            break;
        }
        
        let section_id = data[offset];
        offset += 1;

        // Parse the section size
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading section size",
            ));
        }
        
        let (section_size, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;

        if offset + section_size as usize > data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                format!("Section size {} exceeds remaining data size {}", section_size, data.len() - offset),
            ));
        }

        let section_start = offset;
        let section_end = offset + section_size as usize;
        let section_data = &data[section_start..section_end];

        // Parse the section based on its ID
        match section_id {
            CUSTOM_SECTION_ID => {
                parse_custom_section(module, section_data)?;
            },
            TYPE_SECTION_ID => {
                parse_type_section(module, section_data)?;
            },
            IMPORT_SECTION_ID => {
                parse_import_section(module, section_data)?;
            },
            FUNCTION_SECTION_ID => {
                parse_function_section(module, section_data)?;
            },
            TABLE_SECTION_ID => {
                parse_table_section(module, section_data)?;
            },
            MEMORY_SECTION_ID => {
                parse_memory_section(module, section_data)?;
            },
            GLOBAL_SECTION_ID => {
                parse_global_section(module, section_data)?;
            },
            EXPORT_SECTION_ID => {
                parse_export_section(module, section_data)?;
            },
            START_SECTION_ID => {
                parse_start_section(module, section_data)?;
            },
            ELEMENT_SECTION_ID => {
                parse_element_section(module, section_data)?;
            },
            CODE_SECTION_ID => {
                parse_code_section(module, section_data)?;
            },
            DATA_SECTION_ID => {
                parse_data_section(module, section_data)?;
            },
            DATA_COUNT_SECTION_ID => {
                parse_data_count_section(module, section_data)?;
            },
            TYPE_INFORMATION_SECTION_ID => {
                if module.core_version == CoreWasmVersion::V3_0 {
                    parse_type_information_section(module, section_data)?;
                } else {
                    // Section ID 15 is unknown for Wasm 2.0, could warn or treat as custom/skip
                    // For now, let's be strict and error if it's not a V3_0 module,
                    // or simply skip if we want to be more lenient with unknown sections.
                    // Current loop structure implies skipping unknown sections silently.
                    // If strictness is desired, an error should be returned:
                    // return Err(Error::new(...));
                    // To match existing behavior of skipping unknown sections:
                    // log_warning!("Encountered TypeInformation section (ID 15) in a non-Wasm3.0 module. Skipping.");
                }
            },
            _ => {
                // Unknown section - just skip it
                // We could log a warning, but for now we'll just ignore it
            }
        }
        
        // Move to the next section
        offset = section_end;
    }

    Ok(())
}

/// Parse a custom section and add it to the module
///
/// # Arguments
///
/// * `module` - The module to add the custom section to
/// * `data` - The custom section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_custom_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    if offset >= data.len() {
        return Ok(());
    }
    
    // Parse name
    let (name, bytes_read) = utils::read_name_as_string(&data[offset..], 0)?;
    offset += bytes_read;

    // Extract the section data
    let custom_data = &data[offset..];

    // Create a CustomSection object and add it to the module
    let custom_section = CustomSection::new(name, custom_data.to_vec());
    module.add_custom_section(custom_section);
    
    Ok(())
}

/// Parse the type section
///
/// # Arguments
///
/// * `module` - The module to add the types to
/// * `data` - The type section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_type_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of types
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut types = Vec::with_capacity(count as usize);
    
    // Parse each function type
    for _ in 0..count {
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading type",
            ));
        }
        
        // Type form must be the function type (0x60)
        if data[offset] != 0x60 {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                format!("Invalid type form: 0x{:02x}", data[offset]),
            ));
        }
        offset += 1;
        
        // Read parameter count
        let (param_count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;
        
        // Read parameters
        let mut params = Vec::with_capacity(param_count as usize);
        for _ in 0..param_count {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_SECTION,
                    "Unexpected end of data while reading parameter type",
                ));
            }
            
            let type_byte = data[offset];
            let value_type = match type_byte {
                0x7F => ValueType::I32,
                0x7E => ValueType::I64,
                0x7D => ValueType::F32,
                0x7C => ValueType::F64,
                0x7B => ValueType::V128,
                0x70 => ValueType::FuncRef,
                0x6F => ValueType::ExternRef,
                // Hypothetical Finding F2: Allow I16x8 for Wasm 3.0
                0x79 if module.core_version == CoreWasmVersion::V3_0 => ValueType::I16x8,
                _ => {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION, // Consider a more specific error like wrt_error_kinds::unknown_value_type_for_version
                        format!("Invalid or unsupported value type byte for Wasm version {:?}: 0x{:02x}", module.core_version, type_byte),
                    ));
                }
            };
            
            params.push(value_type);
            offset += 1;
        }
        
        // Read result count
        let (result_count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;
        
        // Read results
        let mut results = Vec::with_capacity(result_count as usize);
        for _ in 0..result_count {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_SECTION,
                    "Unexpected end of data while reading result type",
                ));
            }
            
            let type_byte = data[offset];
            let value_type = match type_byte {
                0x7F => ValueType::I32,
                0x7E => ValueType::I64,
                0x7D => ValueType::F32,
                0x7C => ValueType::F64,
                0x7B => ValueType::V128,
                0x70 => ValueType::FuncRef,
                0x6F => ValueType::ExternRef,
                // Hypothetical Finding F2: Allow I16x8 for Wasm 3.0
                0x79 if module.core_version == CoreWasmVersion::V3_0 => ValueType::I16x8,
                _ => {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION, // Consider a more specific error
                        format!("Invalid or unsupported value type byte for Wasm version {:?}: 0x{:02x}", module.core_version, type_byte),
                    ));
                }
            };
            
            results.push(value_type);
            offset += 1;
        }
        
        // Create the function type
        let func_type = FuncType::new(params, results)?;
        types.push(func_type);
    }
    
    // Store the types in the module
    module.types = types;
    
    Ok(())
}

/// Parse the import section
///
/// # Arguments
///
/// * `module` - The module to add the imports to
/// * `data` - The import section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_import_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of imports
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut imports = Vec::with_capacity(count as usize);
    
    // Parse each import
    for _ in 0..count {
        // Read the module name
        let (module_name, bytes_read) = utils::read_name_as_string(&data[offset..], 0)?;
        offset += bytes_read;
        
        // Read the field name
        let (field_name, bytes_read) = utils::read_name_as_string(&data[offset..], 0)?;
        offset += bytes_read;
        
        // Read the import kind
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading import kind",
            ));
        }
        
        let kind = data[offset];
        offset += 1;
        
        // Parse the import descriptor based on kind
        let desc = match kind {
            0x00 => {
                // Function import
                let (type_idx, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
                offset += bytes_read;
                ImportDesc::Function(type_idx)
            },
            0x01 => {
                // Table import
                if offset >= data.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION,
                        "Unexpected end of data while reading table type",
                    ));
                }
                
                // Read element type
                let elem_type = match data[offset] {
                    0x70 => ValueType::FuncRef,
                    0x6F => ValueType::ExternRef,
                    _ => {
                        return Err(Error::new(
                            ErrorCategory::Parse,
                            ERROR_INVALID_SECTION,
                            format!("Invalid element type: 0x{:02x}", data[offset]),
                        ));
                    }
                };
                offset += 1;
                
                // Read limits
                if offset >= data.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION,
                        "Unexpected end of data while reading limits",
                    ));
                }
                
                let flags = data[offset];
                offset += 1;
                
                let (min, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
                offset += bytes_read;
                
                let max = if flags & 0x01 != 0 {
                    let (max_val, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
                    offset += bytes_read;
                    Some(max_val as u64)
                } else {
                    None
                };
                
                let table = Table {
                    element_type: elem_type,
                    limits: Limits {
                        min: min as u64,
                        max,
                        shared: (flags & 0x02) != 0,
                        memory64: (flags & 0x04) != 0,
                    },
                };
                
                ImportDesc::Table(table)
            },
            0x02 => {
                // Memory import
                if offset >= data.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION,
                        "Unexpected end of data while reading memory type",
                    ));
                }
                
                let flags = data[offset];
                offset += 1;
                
                let (min, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
                offset += bytes_read;
                
                let max = if flags & 0x01 != 0 {
                    let (max_val, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
                    offset += bytes_read;
                    Some(max_val as u64)
                } else {
                    None
                };
                
                let memory = Memory {
                    limits: Limits {
                        min: min as u64,
                        max,
                        shared: (flags & 0x02) != 0,
                        memory64: (flags & 0x04) != 0,
                    },
                    shared: (flags & 0x02) != 0,
                };
                
                ImportDesc::Memory(memory)
            },
            0x03 => {
                // Global import
                if offset >= data.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION,
                        "Unexpected end of data while reading global type",
                    ));
                }
                
                let type_byte = data[offset];
                let value_type = match type_byte {
                    0x7F => ValueType::I32,
                    0x7E => ValueType::I64,
                    0x7D => ValueType::F32,
                    0x7C => ValueType::F64,
                    0x7B => ValueType::V128,
                    0x70 => ValueType::FuncRef,
                    0x6F => ValueType::ExternRef,
                    // Hypothetical Finding F2: Allow I16x8 for Wasm 3.0 globals
                    0x79 if module.core_version == CoreWasmVersion::V3_0 => ValueType::I16x8,
                    _ => {
                        return Err(Error::new(
                            ErrorCategory::Parse,
                            ERROR_INVALID_SECTION, // Consider a more specific error
                            format!("Invalid or unsupported value type byte for Wasm version {:?} in global import: 0x{:02x}", module.core_version, type_byte),
                        ));
                    }
                };
                offset += 1;
                
                if offset >= data.len() { // For mutability byte
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        ERROR_INVALID_SECTION,
                        "Unexpected end of data while reading global mutability",
                    ));
                }
                let mutable = data[offset] != 0;
                offset += 1;
                
                // The `Global` struct in `wrt-format` currently takes `FormatGlobalType`,
                // which itself takes a `ValueType`.
                // The `Global` in `ImportDesc` in `wrt-format` should probably take `FormatGlobalType` too.
                // For now, assuming ImportDesc::Global needs wrt_types::types::GlobalType or similar that we can construct.
                // Let's ensure the structure in wrt-format::module::ImportDesc::Global is compatible or adjust here.
                // From wrt-format/src/module.rs: pub enum ImportDesc { ..., Global(FormatGlobalType), ... }
                // From wrt-format/src/types.rs: pub struct FormatGlobalType { pub value_type: ValueType, pub mutable: bool }
                ImportDesc::Global(wrt_format::types::FormatGlobalType {
                    value_type,
                    mutable,
                })
            },
            // Hypothetical Finding F6: New import kind for Wasm 3.0 Tag proposal
            0x04 if module.core_version == CoreWasmVersion::V3_0 => {
                // Tag import (represents an exception tag)
                // The proposal typically has a type index associated with a tag, pointing to a function type.
                let (type_idx, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
                offset += bytes_read;
                ImportDesc::Tag(type_idx) // Assumes ImportDesc::Tag(u32) was added in wrt-format
            },
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_SECTION, // Consider wrt_error_kinds::invalid_import_export_kind_for_version
                    format!("Invalid or unsupported import kind for Wasm version {:?}: 0x{:02x}", module.core_version, kind),
                ));
            }
        };
        
        // Create the import
        let import = Import {
            module: module_name,
            name: field_name,
            desc,
        };
        
        imports.push(import);
    }
    
    // Store the imports in the module
    module.imports = imports;
    
    Ok(())
}

/// Parse the function section
///
/// # Arguments
///
/// * `module` - The module to add the functions to
/// * `data` - The function section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_function_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of functions
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut function_type_indices = Vec::with_capacity(count as usize);
    
    // Parse each function type index
    for _ in 0..count {
        let (type_idx, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;
        
        function_type_indices.push(type_idx);
    }
    
    // Store the function type indices in the module
    module.function_type_indices = function_type_indices;
    
    Ok(())
}

/// Parse the table section
///
/// # Arguments
///
/// * `module` - The module to add the tables to
/// * `data` - The table section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_table_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of tables
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut tables = Vec::with_capacity(count as usize);
    
    // Parse each table
    for _ in 0..count {
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading table type",
            ));
        }
        
        // Read element type
        let elem_type = match data[offset] {
            0x70 => ValueType::FuncRef,
            0x6F => ValueType::ExternRef,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_SECTION,
                    format!("Invalid element type: 0x{:02x}", data[offset]),
                ));
            }
        };
        offset += 1;
        
        // Read limits
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading limits",
            ));
        }
        
        let flags = data[offset];
        offset += 1;
        
        let (min, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;
        
        let max = if flags & 0x01 != 0 {
            let (max_val, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
            offset += bytes_read;
            Some(max_val as u64)
        } else {
            None
        };
        
        let table = Table {
            element_type: elem_type,
            limits: Limits {
                min: min as u64,
                max,
                shared: (flags & 0x02) != 0,
                memory64: (flags & 0x04) != 0,
            },
        };
        
        tables.push(table);
    }
    
    // Store the tables in the module
    module.tables = tables;
    
    Ok(())
}

/// Parse the memory section
///
/// # Arguments
///
/// * `module` - The module to add the memories to
/// * `data` - The memory section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_memory_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of memories
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut memories = Vec::with_capacity(count as usize);
    
    // Parse each memory
    for _ in 0..count {
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading memory type",
            ));
        }
        
        let flags = data[offset];
        offset += 1;
        
        let (min, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;
        
        let max = if flags & 0x01 != 0 {
            let (max_val, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
            offset += bytes_read;
            Some(max_val as u64)
        } else {
            None
        };
        
        let memory = Memory {
            limits: Limits {
                min: min as u64,
                max,
                shared: (flags & 0x02) != 0,
                memory64: (flags & 0x04) != 0,
            },
            shared: (flags & 0x02) != 0,
        };
        
        memories.push(memory);
    }
    
    // Store the memories in the module
    module.memories = memories;
    
    Ok(())
}

/// Parse the global section
///
/// # Arguments
///
/// * `module` - The module to add the globals to
/// * `data` - The global section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_global_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of globals
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut globals = Vec::with_capacity(count as usize);
    
    // Parse each global
    for _ in 0..count {
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading global type",
            ));
        }
        
        let type_byte = data[offset];
        let value_type = match type_byte {
            0x7F => ValueType::I32,
            0x7E => ValueType::I64,
            0x7D => ValueType::F32,
            0x7C => ValueType::F64,
            0x7B => ValueType::V128,
            0x70 => ValueType::FuncRef,
            0x6F => ValueType::ExternRef,
            // Hypothetical Finding F2: Allow I16x8 for Wasm 3.0 globals
            0x79 if module.core_version == CoreWasmVersion::V3_0 => ValueType::I16x8,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_SECTION, // Consider a more specific error
                    format!("Invalid or unsupported value type byte for Wasm version {:?} in global section: 0x{:02x}", module.core_version, type_byte),
                ));
            }
        };
        offset += 1;
        
        // Read mutability
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading global mutability",
            ));
        }
        
        let mutable = data[offset] != 0;
        offset += 1;
        
        // Parse initialization expression
        // For simplicity, we'll just find the end of the expression (0x0B)
        let expr_start = offset;
        while offset < data.len() && data[offset] != 0x0B {
            offset += 1;
        }
        
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading global initialization expression",
            ));
        }
        
        // Include the end opcode
        offset += 1;
        
        let init = data[expr_start..offset].to_vec();
        
        let global = Global {
            global_type: wrt_types::types::GlobalType {
                value_type,
                mutable,
            },
            init,
        };
        
        globals.push(global);
    }
    
    // Store the globals in the module
    module.globals = globals;
    
    Ok(())
}

/// Parse the export section
///
/// # Arguments
///
/// * `module` - The module to add the exports to
/// * `data` - The export section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_export_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the count of exports
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    offset += bytes_read;
    
    let mut exports = Vec::with_capacity(count as usize);
    
    // Parse each export
    for _ in 0..count {
        // Read the export name
        let (name, bytes_read) = utils::read_name_as_string(&data[offset..], 0)?;
        offset += bytes_read;
        
        // Read the export kind
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                ERROR_INVALID_SECTION,
                "Unexpected end of data while reading export kind",
            ));
        }
        
        let kind_byte = data[offset];
        offset += 1;
        
        let kind = match kind_byte {
            0x00 => ExportKind::Function,
            0x01 => ExportKind::Table,
            0x02 => ExportKind::Memory,
            0x03 => ExportKind::Global,
            // Hypothetical Finding F6: New export kind for Wasm 3.0 Tag proposal
            0x04 if module.core_version == CoreWasmVersion::V3_0 => ExportKind::Tag, // Assumes ExportKind::Tag was added in wrt-format
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    ERROR_INVALID_SECTION, // Consider wrt_error_kinds::invalid_import_export_kind_for_version
                    format!("Invalid or unsupported export kind for Wasm version {:?}: 0x{:02x}", module.core_version, kind_byte),
                ));
            }
        };
        
        // Read the export index
        let (index, bytes_read) = BinaryFormat::decode_leb_u32(&data[offset..])?;
        offset += bytes_read;
        
        // Create the export
        let export = Export {
            name,
            kind,
            index,
        };
        
        exports.push(export);
    }
    
    // Store the exports in the module
    module.exports = exports;
    
    Ok(())
}

/// Parse the start section
///
/// # Arguments
///
/// * `module` - The module to add the start function to
/// * `data` - The start section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_start_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the start function index
    let (start_index, _) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    
    // Store the start function index in the module
    module.start = Some(start_index);
    
    Ok(())
}

/// Parse the element section
///
/// # Arguments
///
/// * `module` - The module to add the elements to
/// * `data` - The element section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_element_section(module: &mut Module, data: &[u8]) -> Result<()> {
    // For simplicity, we'll just store the raw element section data for now
    module.elements = data.to_vec();
    
    Ok(())
}

/// Parse the code section
///
/// # Arguments
///
/// * `module` - The module to add the code to
/// * `data` - The code section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_code_section(module: &mut Module, data: &[u8]) -> Result<()> {
    // For simplicity, we'll just store the raw code section data for now
    module.code = data.to_vec();
    
    Ok(())
}

/// Parse the data section
///
/// # Arguments
///
/// * `module` - The module to add the data to
/// * `data` - The data section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_data_section(module: &mut Module, data: &[u8]) -> Result<()> {
    // For simplicity, we'll just store the raw data section data for now
    module.data = data.to_vec();
    
    Ok(())
}

/// Parse the data count section
///
/// # Arguments
///
/// * `module` - The module to add the data count to
/// * `data` - The data count section data
///
/// # Returns
///
/// * `Result<()>` - Success or an error
fn parse_data_count_section(module: &mut Module, data: &[u8]) -> Result<()> {
    let mut offset = 0;
    
    // Read the data count
    let (data_count, _) = BinaryFormat::decode_leb_u32(&data[offset..])?;
    
    // Store the data count in the module
    module.data_count = Some(data_count);
    
    Ok(())
}

/// Parse the WebAssembly binary content after the magic number and version
///
/// # Arguments
///
/// * `data` - The WebAssembly binary data after the magic number and version
///
/// # Returns
///
/// * `Result<Module>` - The parsed module or an error
///
/// # Errors
///
/// Returns an error if the binary cannot be parsed.
pub fn parse_binary(data: &[u8]) -> Result<Module> {
    let mut module = Module::new();
    module.binary = Some(data.to_vec());

    parse_binary_into_module(data, &mut module)?;
    
    Ok(module)
}

/// Hypothetical Finding F5: Placeholder for parsing the TypeInformation section
fn parse_type_information_section(module: &mut Module, data: &[u8]) -> Result<()> {
    // Ensure module.type_info_section is initialized if not already
    // (it defaults to None, but if this section can appear multiple times,
    // the behavior would need clarification - Wasm sections usually appear at most once)
    let type_info_section = module.type_info_section.get_or_insert_with(Default::default);
    
    let mut current_offset = 0;
    let (count, bytes_read) = BinaryFormat::decode_leb_u32(&data[current_offset..])?;
    current_offset += bytes_read;

    for _ in 0..count {
        // Parse type_index (varuint32)
        let (type_idx, bytes_read) = BinaryFormat::decode_leb_u32(&data[current_offset..])?;
        current_offset += bytes_read;

        // Parse name (string)
        let (name, bytes_read) = utils::read_name_as_string(&data[current_offset..], current_offset)?; // Assuming read_name_as_string is suitable
        current_offset += bytes_read;
        
        // TODO: Add proper capacity checks for vector and data length checks.
        // TODO: Validate type_idx against module.types.len() in the validation phase.

        type_info_section.entries.push(wrt_format::module::TypeInformationEntry {
            type_index: type_idx,
            name,
        });
    }
    
    if current_offset != data.len() {
        return Err(Error::new(
            ErrorCategory::Parse,
            ERROR_INVALID_LENGTH, // Or a more specific error
            "Extra data at end of TypeInformation section".to_string(),
        ));
    }

    Ok(())
}
