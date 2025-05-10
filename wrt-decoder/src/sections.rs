//! Section parsers for WebAssembly binary format
//!
//! This module contains parsers for various sections in WebAssembly modules.

use crate::prelude::{format, String, Vec};
use wrt_error::errors::codes;
use wrt_error::{Error, ErrorCategory, Result, ErrorSource};
use wrt_format::binary;
use wrt_format::{
    module::{
        Data, DataMode, Element, Export, ExportKind, Global, Import, ImportDesc, Memory, Table,
    },
    types::ValueType as FormatValueType,
};
use wrt_types::types::FuncType as WrtFuncType;
use wrt_types::types::Import as WrtImport;
use wrt_types::types::TableType as WrtTableType;
use wrt_types::types::MemoryType as WrtMemoryType;
use wrt_types::types::GlobalType as WrtGlobalType;
use wrt_types::types::Export as WrtExport;
use wrt_types::types::ElementSegment as WrtElementSegment;
use wrt_types::types::DataSegment as WrtDataSegment;

/// Parsers implementation
pub mod parsers {
    use super::*;

    /// Parse a type section
    pub fn parse_type_section(bytes: &[u8]) -> Result<Vec<WrtFuncType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut format_func_types = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Function type indicator (0x60)
            if offset >= bytes.len() || bytes[offset] != 0x60 {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Expected function type indicator (0x60)",
                ));
            }
            offset += 1;

            // Parse param types
            let (param_count, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;

            let mut params = Vec::with_capacity(param_count as usize);
            for _ in 0..param_count {
                if offset >= bytes.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Unexpected end of param types",
                    ));
                }

                let val_type_byte = bytes[offset];
                let format_val_type = wrt_format::conversion::parse_value_type(val_type_byte).map_err(|e: wrt_error::Error|
                    Error::new(
                        ErrorCategory::Parse,
                        codes::INVALID_TYPE,
                        format!("Invalid param value type byte: 0x{:x} - {}", val_type_byte, e.message()),
                    )
                )?;
                params.push(format_val_type);
                offset += 1;
            }

            // Parse result types
            let (result_count, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;

            let mut results = Vec::with_capacity(result_count as usize);
            for _ in 0..result_count {
                if offset >= bytes.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Unexpected end of result types",
                    ));
                }

                let val_type_byte = bytes[offset];
                let format_val_type = wrt_format::conversion::parse_value_type(val_type_byte).map_err(|e: wrt_error::Error|
                    Error::new(
                        ErrorCategory::Parse,
                        codes::INVALID_TYPE,
                        format!("Invalid result value type byte: 0x{:x} - {}", val_type_byte, e.message()),
                    )
                )?;
                results.push(format_val_type);
                offset += 1;
            }

            format_func_types.push(wrt_format::FuncType::new(params, results)?);
        }

        format_func_types.into_iter()
            .map(|ft_format| crate::conversion::format_func_type_to_types_func_type(&ft_format))
            .collect()
    }

    /// Parse a function section
    pub fn parse_function_section(bytes: &[u8]) -> Result<Vec<u32>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut indices = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (index, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;
            indices.push(index);
        }

        Ok(indices)
    }

    /// Parse an import section
    pub fn parse_import_section(bytes: &[u8]) -> Result<Vec<WrtImport>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut format_imports = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Parse module name
            let (module_bytes, new_offset) = binary::read_name(bytes, offset)?;
            offset = new_offset;

            // Parse field name
            let (name_bytes, new_offset) = binary::read_name(bytes, offset)?;
            offset = new_offset;

            if offset >= bytes.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Unexpected end of import description",
                ));
            }

            // Parse import description kind byte
            let kind_byte = bytes[offset];
            offset += 1;

            // Parse into wrt_format::module::ImportDesc first
            let format_desc = match kind_byte {
                0x00 => { // Function import
                    let (type_idx, new_offset) = binary::read_leb128_u32(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Function(type_idx)
                }
                0x01 => { // Table import
                    let (format_table, new_offset) = parse_format_module_table(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Table(format_table)
                }
                0x02 => { // Memory import
                    let (format_memory, new_offset) = parse_format_module_memory(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Memory(format_memory)
                }
                0x03 => { // Global import
                    let (format_global_type, new_offset) = parse_format_global_type(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Global(format_global_type)
                }
                // TODO: Handle 0x04 Tag import if/when supported by wrt_format
                _ => {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        format!("Invalid import description kind: 0x{:x}", kind_byte),
                    ));
                }
            };

            format_imports.push(wrt_format::module::Import {
                module: String::from_utf8(module_bytes.to_vec()).map_err(|e|
                    Error::new(
                        ErrorCategory::Parse,
                        codes::INVALID_UTF8_ENCODING,
                        format!("Invalid UTF-8 in import module name: {}", e),
                    )
                )?,
                name: String::from_utf8(name_bytes.to_vec()).map_err(|e|
                    Error::new(
                        ErrorCategory::Parse,
                        codes::INVALID_UTF8_ENCODING,
                        format!("Invalid UTF-8 in import field name: {}", e),
                    )
                )?,
                desc: format_desc,
            });
        }

        // Convert all wrt_format::module::Import to wrt_types::types::Import
        format_imports.into_iter()
            .map(|fi| crate::conversion::format_import_to_types_import(&fi))
            .collect()
    }

    /// Parse a table section
    pub fn parse_table_section(bytes: &[u8]) -> Result<Vec<WrtTableType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_tables = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (format_table, new_offset) = parse_format_module_table(bytes, offset)?;
            offset = new_offset;
            
            let types_table = crate::conversion::format_table_type_to_types_table_type(&format_table);
            wrt_tables.push(types_table);
        }
        Ok(wrt_tables)
    }

    fn parse_format_module_table(bytes: &[u8], mut offset: usize) -> Result<(wrt_format::module::Table, usize)> {
        if offset >= bytes.len() {
            return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Unexpected end of table entry (element type byte)"));
        }
        let element_type_byte = bytes[offset];
        offset += 1;

        let element_type = wrt_format::conversion::parse_value_type(element_type_byte)
            .map_err(|e| Error::new(ErrorCategory::Parse, codes::INVALID_TYPE,
                format!("Invalid element type byte for table: 0x{:x}. Error: {}", element_type_byte, e.message())))?;
        
        if element_type != FormatValueType::FuncRef && element_type != FormatValueType::ExternRef {
            return Err(Error::new(ErrorCategory::Parse, codes::INVALID_TYPE,
                format!("Table element type must be funcref or externref, found {:?}", element_type)));
        }

        let (limits, new_offset) = binary::parse_limits(bytes, offset)?;
        offset = new_offset;

        Ok((wrt_format::module::Table { element_type, limits }, offset))
    }

    /// Parse a memory section
    pub fn parse_memory_section(bytes: &[u8]) -> Result<Vec<WrtMemoryType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_memories = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (format_memory, new_offset) = parse_format_module_memory(bytes, offset)?;
            offset = new_offset;
            
            let types_memory = crate::conversion::format_memory_type_to_types_memory_type(&format_memory);
            wrt_memories.push(types_memory);
        }
        Ok(wrt_memories)
    }

    fn parse_format_module_memory(bytes: &[u8], offset: usize) -> Result<(wrt_format::module::Memory, usize)> {
        let (limits, new_offset) = binary::parse_limits(bytes, offset)?;
        Ok((
            wrt_format::module::Memory {
                limits: limits.clone(),
                shared: limits.shared, // The shared flag from the parsed limits
            },
            new_offset,
        ))
    }

    fn parse_format_global_type(bytes: &[u8], mut offset: usize) -> Result<(wrt_format::types::FormatGlobalType, usize)> {
        if offset + 1 >= bytes.len() { // Need valtype + mutability byte
            return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Unexpected end for global type (need 2 bytes)"));
        }
        let val_type_byte = bytes[offset];
        offset += 1;
        let mutability_byte = bytes[offset];
        offset += 1;

        let value_type = wrt_format::conversion::parse_value_type(val_type_byte)
            .map_err(|e| Error::new(ErrorCategory::Parse, codes::INVALID_TYPE,
                format!("Invalid value type byte for global: 0x{:x}. Error: {}", val_type_byte, e.message())))?;

        let mutable = match mutability_byte {
            0x00 => false,
            0x01 => true,
            _ => return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, 
                format!("Invalid mutability byte for global: 0x{:x}", mutability_byte))),
        };

        Ok((wrt_format::types::FormatGlobalType { value_type, mutable }, offset))
    }

    /// Parse a global section
    pub fn parse_global_section(bytes: &[u8]) -> Result<Vec<WrtGlobalType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_globals = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (format_global_type, new_offset) = parse_format_global_type(bytes, offset)?;
            offset = new_offset;

            let init_expr_start = offset;
            let mut end_opcode_idx = None;
            // Find the END opcode for the init_expr
            // Correctly iterate through the slice from the current offset
            let mut temp_offset = init_expr_start;
            loop {
                if temp_offset >= bytes.len() {
                    return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Global init expression unterminated or extends beyond section bounds"));
                }
                // A simple way to find END is to try parsing instructions one by one,
                // but that requires a full instruction parser here or assumptions.
                // A more robust way is to scan for END, but must be careful about nested blocks if allowed in init_expr (they are not).
                // MVP: init_expr is a single const instruction, or global.get, followed by END.
                // So, we can scan for the END opcode.
                if bytes[temp_offset] == binary::END { // binary::END should be 0x0B
                    end_opcode_idx = Some(temp_offset);
                    break;
                }
                // To avoid infinite loop on malformed input without END, check reasonable length or instruction.
                // For now, simple scan for END. Max init_expr length is small.
                if temp_offset > init_expr_start + 20 { // Heuristic limit to prevent runaway scan
                     return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Global init expression too long or END opcode not found within reasonable limit"));
                }
                temp_offset += 1;
            }

            let end_idx = end_opcode_idx.ok_or_else(|| Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Global init expression missing END opcode"))?;
            
            let init_expr_bytes = &bytes[init_expr_start..end_idx + 1]; // Slice includes the END opcode
            offset = end_idx + 1; // Update main offset to after the init_expr
            
            let format_global = wrt_format::module::Global {
                global_type: format_global_type,
                init: init_expr_bytes.to_vec(),
            };

            let types_global = crate::conversion::format_global_to_types_global(&format_global)?;
            wrt_globals.push(types_global);
        }
        Ok(wrt_globals)
    }

    /// Parse an export section
    pub fn parse_export_section(bytes: &[u8]) -> Result<Vec<WrtExport>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut format_exports = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (name_bytes, new_offset) = binary::read_name(bytes, offset)?;
            offset = new_offset;

            if offset >= bytes.len() {
                return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Unexpected end of export kind"));
            }
            let kind_byte = bytes[offset];
            offset += 1;

            let format_kind = match kind_byte {
                0x00 => wrt_format::module::ExportKind::Function,
                0x01 => wrt_format::module::ExportKind::Table,
                0x02 => wrt_format::module::ExportKind::Memory,
                0x03 => wrt_format::module::ExportKind::Global,
                // TODO: Handle 0x04 Tag if/when supported by wrt_format
                _ => return Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, format!("Invalid export kind byte: 0x{:x}", kind_byte))),
            };

            let (index, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;
            
            format_exports.push(wrt_format::module::Export {
                name: String::from_utf8(name_bytes.to_vec()).map_err(|e| Error::new(ErrorCategory::Parse, codes::INVALID_UTF8_ENCODING, format!("Invalid UTF-8 in export name: {}",e)))?,
                kind: format_kind,
                index,
            });
        }
        format_exports.into_iter()
            .map(|fe| crate::conversion::format_export_to_types_export(&fe))
            .collect()
    }

    /// Parse an element section
    pub fn parse_element_section(bytes: &[u8]) -> Result<Vec<WrtElementSegment>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_elements = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // binary::parse_element is expected to parse a wrt_format::module::Element
            let (format_element, new_offset) = binary::parse_element(bytes, offset).map_err(|e| {
                Error::new(
                    e.category(),
                    e.code(),
                    format!("Failed to parse element entry: {}", e.message()),
                )
            })?;
            offset = new_offset;

            let types_element = crate::conversion::format_element_to_types_element_segment(&format_element)?;
            wrt_elements.push(types_element);
        }
        Ok(wrt_elements)
    }

    /// Parse a code section
    pub fn parse_code_section(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut bodies = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Get body size
            let (body_size, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;

            if offset + body_size as usize > bytes.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Unexpected end of code body",
                ));
            }

            // Extract body bytes
            let body = bytes[offset..offset + body_size as usize].to_vec();
            offset += body_size as usize;

            bodies.push(body);
        }

        Ok(bodies)
    }

    /// Parse a data section
    pub fn parse_data_section(bytes: &[u8]) -> Result<Vec<WrtDataSegment>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_data_segments = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // binary::parse_data_segment is expected to parse a wrt_format::module::Data
            // Note: The name in wrt_format::binary might be parse_data, not parse_data_segment
            let (format_data_segment, new_offset) = binary::parse_data(bytes, offset).map_err(|e| {
                Error::new(
                    e.category(),
                    e.code(),
                    format!("Failed to parse data segment entry: {}", e.message()),
                )
            })?;
            offset = new_offset;

            let types_data_segment = crate::conversion::format_data_to_types_data_segment(&format_data_segment)?;
            wrt_data_segments.push(types_data_segment);
        }
        Ok(wrt_data_segments)
    }
}
