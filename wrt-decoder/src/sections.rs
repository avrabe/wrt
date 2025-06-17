// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Section parsers for WebAssembly binary format
//!
//! This module contains parsers for various sections in WebAssembly modules.

use wrt_error::{errors::codes, Error, ErrorCategory, ErrorSource, Result};
use wrt_format::{
    binary::{self},
    types::ValueType as FormatValueType,
};

// Note: These functions should be available if they're exported by wrt_format
// If not, we'll need to implement alternatives or define them locally
use wrt_foundation::types::{
    FuncType, GlobalType as WrtGlobalType, Import as WrtImport, MemoryType as WrtMemoryType,
    TableType as WrtTableType,
};
use wrt_foundation::NoStdProvider;

// Type aliases with specific provider
type WrtFuncType = FuncType<NoStdProvider<1024>>;
type WrtFoundationImport = WrtImport<NoStdProvider<1024>>;

// Import segment types from wrt-format
use wrt_format::{
    module::Export as WrtExport, DataSegment as WrtDataSegment, ElementSegment as WrtElementSegment,
};

use crate::memory_optimized::{check_bounds_u32, safe_usize_conversion};
use crate::optimized_string::parse_utf8_string_inplace;
use crate::prelude::Vec;

// Helper functions for missing imports
fn parse_element_segment(
    _bytes: &[u8],
    _offset: usize,
) -> Result<(wrt_format::module::Element, usize)> {
    // Simplified element segment parsing - would need full implementation
    Err(Error::new(
        ErrorCategory::Parse,
        codes::PARSE_ERROR,
        "Element segment parsing not implemented",
    ))
}

fn parse_data(_bytes: &[u8], _offset: usize) -> Result<(wrt_format::module::Data, usize)> {
    // Simplified data segment parsing - would need full implementation
    Err(Error::new(
        ErrorCategory::Parse,
        codes::PARSE_ERROR,
        "Data segment parsing not implemented",
    ))
}

fn parse_limits(bytes: &[u8], offset: usize) -> Result<(wrt_format::types::Limits, usize)> {
    if offset >= bytes.len() {
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::PARSE_ERROR,
            "Unexpected end while parsing limits",
        ));
    }

    let flags = bytes[offset];
    let mut new_offset = offset + 1;

    // Read minimum
    let (min, min_offset) = binary::read_leb128_u32(bytes, new_offset)?;
    new_offset = min_offset;

    // Check if maximum is present (flag bit 0)
    let max = if flags & 0x01 != 0 {
        let (max_val, max_offset) = binary::read_leb128_u32(bytes, new_offset)?;
        new_offset = max_offset;
        Some(max_val as u64)
    } else {
        None
    };

    // Check shared flag (flag bit 1)
    let shared = flags & 0x02 != 0;

    Ok((wrt_format::types::Limits { min: min as u64, max, shared, memory64: false }, new_offset))
}

/// Parsers implementation
pub mod parsers {
    use super::*;

    /// Parse a type section with memory optimization
    pub fn parse_type_section(bytes: &[u8]) -> Result<Vec<WrtFuncType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;

        // Binary std/no_std choice
        check_bounds_u32(count, 10000, "type count")?;
        let count_usize = safe_usize_conversion(count, "type count")?;

        let mut format_func_types = Vec::new();
        format_func_types.reserve(count_usize.min(1024)); // Reserve conservatively

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

            // Bounds check param count
            check_bounds_u32(param_count, 1000, "param count")?;
            let param_count_usize = safe_usize_conversion(param_count, "param count")?;

            let mut params = Vec::new();
            params.reserve(param_count_usize.min(256)); // Conservative reservation
            for _ in 0..param_count {
                if offset >= bytes.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Unexpected end of param types",
                    ));
                }

                let val_type_byte = bytes[offset];
                let format_val_type = wrt_format::conversion::parse_value_type(val_type_byte)
                    .map_err(|_e: wrt_error::Error| {
                        Error::new(
                            ErrorCategory::Parse,
                            codes::INVALID_TYPE,
                            "Invalid param value type byte",
                        )
                    })?;
                params.push(format_val_type);
                offset += 1;
            }

            // Parse result types
            let (result_count, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;

            // Bounds check result count
            check_bounds_u32(result_count, 1000, "result count")?;
            let result_count_usize = safe_usize_conversion(result_count, "result count")?;

            let mut results = Vec::new();
            results.reserve(result_count_usize.min(256)); // Conservative reservation
            for _ in 0..result_count {
                if offset >= bytes.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Unexpected end of result types",
                    ));
                }

                let val_type_byte = bytes[offset];
                let format_val_type = wrt_format::conversion::parse_value_type(val_type_byte)
                    .map_err(|_e: wrt_error::Error| {
                        Error::new(
                            ErrorCategory::Parse,
                            codes::INVALID_TYPE,
                            "Invalid result value type byte",
                        )
                    })?;
                results.push(format_val_type);
                offset += 1;
            }

            format_func_types.push(wrt_format::types::FuncType::new(
                wrt_foundation::NoStdProvider::<1024>::default(),
                params,
                results,
            )?);
        }

        // Since wrt_format::types::FuncType is re-exported from wrt_foundation,
        // we can return the vector directly
        Ok(format_func_types)
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

    /// Parse an import section with memory optimization
    pub fn parse_import_section(bytes: &[u8]) -> Result<Vec<WrtFoundationImport>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;

        // Binary std/no_std choice
        check_bounds_u32(count, 10000, "import count")?;
        let count_usize = safe_usize_conversion(count, "import count")?;

        let mut format_imports = Vec::new();
        format_imports.reserve(count_usize.min(1024)); // Conservative reservation

        for _ in 0..count {
            // Parse module name using optimized string processing
            let (module_string, new_offset) = parse_utf8_string_inplace(bytes, offset)?;
            offset = new_offset;

            // Parse field name using optimized string processing
            let (field_string, new_offset) = parse_utf8_string_inplace(bytes, offset)?;
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
                0x00 => {
                    // Function import
                    let (type_idx, new_offset) = binary::read_leb128_u32(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Function(type_idx)
                }
                0x01 => {
                    // Table import
                    let (format_table, new_offset) = parse_format_module_table(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Table(format_table)
                }
                0x02 => {
                    // Memory import
                    let (format_memory, new_offset) = parse_format_module_memory(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Memory(format_memory)
                }
                0x03 => {
                    // Global import
                    let (format_global_type, new_offset) = parse_format_global_type(bytes, offset)?;
                    offset = new_offset;
                    wrt_format::module::ImportDesc::Global(format_global_type)
                }
                // TODO: Handle 0x04 Tag import if/when supported by wrt_format
                _ => {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Invalid import description kind",
                    ));
                }
            };

            format_imports.push(wrt_format::module::Import {
                module: module_string,
                name: field_string,
                desc: format_desc,
            });
        }

        // Convert wrt_format::Import to wrt_foundation::Import
        // Since Table and Memory are now type aliases to foundation types, this should work directly
        let mut wrt_imports = Vec::with_capacity(format_imports.len());
        let provider = wrt_foundation::NoStdProvider::<1024>::default();

        for format_import in format_imports {
            let module_name = wrt_foundation::bounded::WasmName::from_str(
                &format_import.module,
                provider.clone(),
            )
            .map_err(|_| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Module name too long for bounded string",
                )
            })?;

            let item_name =
                wrt_foundation::bounded::WasmName::from_str(&format_import.name, provider.clone())
                    .map_err(|_| {
                        Error::new(
                            ErrorCategory::Parse,
                            codes::PARSE_ERROR,
                            "Item name too long for bounded string",
                        )
                    })?;

            let wrt_desc = match format_import.desc {
                wrt_format::module::ImportDesc::Function(type_idx) => {
                    wrt_foundation::types::ImportDesc::Function(type_idx)
                }
                wrt_format::module::ImportDesc::Table(table) => {
                    wrt_foundation::types::ImportDesc::Table(table)
                }
                wrt_format::module::ImportDesc::Memory(memory) => {
                    wrt_foundation::types::ImportDesc::Memory(memory)
                }
                wrt_format::module::ImportDesc::Global(format_global) => {
                    // Convert FormatGlobalType to wrt_foundation::GlobalType
                    let global_type = wrt_foundation::GlobalType::new(
                        format_global.value_type,
                        format_global.mutable,
                    );
                    wrt_foundation::types::ImportDesc::Global(global_type)
                }
                wrt_format::module::ImportDesc::Tag(type_idx) => {
                    // Tag is not available in ImportDesc, map to Function for now
                    wrt_foundation::types::ImportDesc::Function(type_idx)
                }
            };

            wrt_imports.push(WrtFoundationImport { module_name, item_name, desc: wrt_desc });
        }

        Ok(wrt_imports)
    }

    /// Parse a table section
    pub fn parse_table_section(bytes: &[u8]) -> Result<Vec<WrtTableType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_tables = Vec::with_capacity(count as usize);

        // Parse format tables and convert directly to foundation types
        for _ in 0..count {
            let (format_table, new_offset) = parse_format_module_table(bytes, offset)?;
            offset = new_offset;

            // Since wrt_format::module::Table is now a type alias to wrt_foundation::TableType,
            // we can use it directly
            wrt_tables.push(format_table);
        }

        Ok(wrt_tables)
    }

    fn parse_format_module_table(
        bytes: &[u8],
        mut offset: usize,
    ) -> Result<(wrt_foundation::TableType, usize)> {
        if offset >= bytes.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Unexpected end of table entry (element type byte)",
            ));
        }
        let element_type_byte = bytes[offset];
        offset += 1;

        let element_type =
            wrt_format::conversion::parse_value_type(element_type_byte).map_err(|_e| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_TYPE,
                    "Invalid element type byte for table",
                )
            })?;

        if element_type != FormatValueType::FuncRef && element_type != FormatValueType::ExternRef {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::INVALID_TYPE,
                "Table element type must be funcref or externref",
            ));
        }

        let (limits, new_offset) = parse_limits(bytes, offset)?;
        offset = new_offset;

        // Convert ValueType to RefType for table element_type
        let ref_type = match element_type {
            FormatValueType::FuncRef => wrt_foundation::RefType::Funcref,
            FormatValueType::ExternRef => wrt_foundation::RefType::Externref,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_TYPE,
                    "Table element type must be funcref or externref",
                ));
            }
        };

        // Convert wrt_format::Limits to wrt_foundation::Limits
        let foundation_limits = wrt_foundation::Limits::new(
            limits.min as u32, // Convert u64 to u32
            limits.max.map(|m| m as u32),
        );

        Ok((
            wrt_foundation::TableType { element_type: ref_type, limits: foundation_limits },
            offset,
        ))
    }

    /// Parse a memory section
    pub fn parse_memory_section(bytes: &[u8]) -> Result<Vec<WrtMemoryType>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_memories = Vec::with_capacity(count as usize);

        // Parse format memories and convert directly to foundation types
        for _ in 0..count {
            let (format_memory, new_offset) = parse_format_module_memory(bytes, offset)?;
            offset = new_offset;

            // Since wrt_format::module::Memory is now a type alias to wrt_foundation::MemoryType,
            // we can use it directly
            wrt_memories.push(format_memory);
        }

        Ok(wrt_memories)
    }

    fn parse_format_module_memory(
        bytes: &[u8],
        offset: usize,
    ) -> Result<(wrt_foundation::MemoryType, usize)> {
        let (limits, new_offset) = parse_limits(bytes, offset)?;

        // Convert wrt_format::Limits to wrt_foundation::Limits
        let foundation_limits = wrt_foundation::Limits::new(
            limits.min as u32, // Convert u64 to u32
            limits.max.map(|m| m as u32),
        );

        Ok((wrt_foundation::MemoryType::new(foundation_limits, limits.shared), new_offset))
    }

    fn parse_format_global_type(
        bytes: &[u8],
        mut offset: usize,
    ) -> Result<(wrt_format::types::FormatGlobalType, usize)> {
        if offset + 1 >= bytes.len() {
            // Need valtype + mutability byte
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Unexpected end for global type (need 2 bytes)",
            ));
        }
        let val_type_byte = bytes[offset];
        offset += 1;
        let mutability_byte = bytes[offset];
        offset += 1;

        let value_type = wrt_format::conversion::parse_value_type(val_type_byte).map_err(|_e| {
            Error::new(
                ErrorCategory::Parse,
                codes::INVALID_TYPE,
                "Invalid value type byte for global",
            )
        })?;

        let mutable = match mutability_byte {
            0x00 => false,
            0x01 => true,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Invalid mutability byte for global",
                ))
            }
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
            #[allow(unused_assignments)]
            let mut end_opcode_idx = None;
            // Find the END opcode for the init_expr
            // Correctly iterate through the slice from the current offset
            let mut temp_offset = init_expr_start;
            loop {
                if temp_offset >= bytes.len() {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Global init expression unterminated or extends beyond section bounds",
                    ));
                }
                // A simple way to find END is to try parsing instructions one by one,
                // but that requires a full instruction parser here or assumptions.
                // A more robust way is to scan for END, but must be careful about nested blocks
                // if allowed in init_expr (they are not). MVP: init_expr is a
                // single const instruction, or global.get, followed by END. So,
                // we can scan for the END opcode.
                if bytes[temp_offset] == binary::END {
                    // binary::END should be 0x0B
                    end_opcode_idx = Some(temp_offset);
                    break;
                }
                // To avoid infinite loop on malformed input without END, check reasonable
                // length or instruction. For now, simple scan for END. Max
                // init_expr length is small.
                if temp_offset > init_expr_start + 20 {
                    // Heuristic limit to prevent runaway scan
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Global init expression too long or END opcode not found within \
                         reasonable limit",
                    ));
                }
                temp_offset += 1;
            }

            let end_idx = end_opcode_idx.ok_or_else(|| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Global init expression missing END opcode",
                )
            })?;

            let init_expr_bytes = &bytes[init_expr_start..end_idx + 1]; // Slice includes the END opcode
            offset = end_idx + 1; // Update main offset to after the init_expr

            let _format_global = wrt_format::module::Global {
                global_type: format_global_type,
                init: init_expr_bytes.to_vec(),
            };

            // Convert FormatGlobalType to wrt_foundation::GlobalType
            // Both types have the same structure (value_type and mutable)
            let wrt_global = WrtGlobalType {
                value_type: format_global_type.value_type,
                mutable: format_global_type.mutable,
            };

            wrt_globals.push(wrt_global);
        }
        Ok(wrt_globals)
    }

    /// Parse an export section with memory optimization
    pub fn parse_export_section(bytes: &[u8]) -> Result<Vec<WrtExport>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;

        // Binary std/no_std choice
        check_bounds_u32(count, 10000, "export count")?;
        let count_usize = safe_usize_conversion(count, "export count")?;

        let mut format_exports = Vec::new();
        format_exports.reserve(count_usize.min(1024)); // Conservative reservation

        for _ in 0..count {
            // Parse export name using optimized string processing
            let (export_name, new_offset) = parse_utf8_string_inplace(bytes, offset)?;
            offset = new_offset;

            if offset >= bytes.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Unexpected end of export kind",
                ));
            }
            let kind_byte = bytes[offset];
            offset += 1;

            let format_kind = match kind_byte {
                0x00 => wrt_format::module::ExportKind::Function,
                0x01 => wrt_format::module::ExportKind::Table,
                0x02 => wrt_format::module::ExportKind::Memory,
                0x03 => wrt_format::module::ExportKind::Global,
                // TODO: Handle 0x04 Tag if/when supported by wrt_format
                _ => {
                    return Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Invalid export kind byte",
                    ))
                }
            };

            let (index, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;

            format_exports.push(wrt_format::module::Export {
                name: export_name,
                kind: format_kind,
                index,
            });
        }
        // Return the format_exports since wrt_format::module::Export is what's expected
        Ok(format_exports)
    }

    /// Parse an element section
    pub fn parse_element_section(bytes: &[u8]) -> Result<Vec<WrtElementSegment>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut wrt_elements = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // binary::parse_element is expected to parse a wrt_format::module::Element
            let (format_element, new_offset) = parse_element_segment(bytes, offset)
                .map_err(|e| Error::new(e.category(), e.code(), "Failed to parse element entry"))?;
            offset = new_offset;

            // Since we're expecting wrt_format::ElementSegment, use the parsed element directly
            wrt_elements.push(format_element);
        }
        Ok(wrt_elements)
    }

    /// Parse a code section with memory optimization
    pub fn parse_code_section(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;

        // Binary std/no_std choice
        check_bounds_u32(count, 100000, "function count")?;
        let count_usize = safe_usize_conversion(count, "function count")?;

        let mut bodies = Vec::new();
        bodies.reserve(count_usize.min(10000)); // Conservative reservation

        for _ in 0..count {
            // Get body size
            let (body_size, new_offset) = binary::read_leb128_u32(bytes, offset)?;
            offset = new_offset;

            // Bounds check body size
            check_bounds_u32(body_size, 1_000_000, "function body size")?;
            let body_size_usize = safe_usize_conversion(body_size, "function body size")?;

            if offset + body_size_usize > bytes.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Unexpected end of code body",
                ));
            }

            // Binary std/no_std choice
            let mut body = Vec::new();
            body.reserve_exact(body_size_usize);
            body.extend_from_slice(&bytes[offset..offset + body_size_usize]);
            offset += body_size_usize;

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
            // Note: The name in wrt_format::binary might be parse_data, not
            // parse_data_segment
            let (format_data_segment, new_offset) = parse_data(bytes, offset).map_err(|e| {
                Error::new(e.category(), e.code(), "Failed to parse data segment entry")
            })?;
            offset = new_offset;

            // Since we're expecting wrt_format::DataSegment, use the parsed data directly
            wrt_data_segments.push(format_data_segment);
        }
        Ok(wrt_data_segments)
    }
}
