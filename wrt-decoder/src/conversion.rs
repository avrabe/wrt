//! Conversion utilities for WASM types
//!
//! This module contains functions to convert between format types and runtime types.

use wrt_error::errors::codes;
use wrt_error::{Error, ErrorCategory, Result};
use wrt_format::{section::CustomSection, Error as WrtFormatError, ValueType as FormatValueType};

// Import RefType directly from wrt-format
use wrt_format::RefType as FormatRefType;

// Import common types from prelude
use crate::prelude::*;

// Import types from wrt-types
use wrt_types::{
    types::{FuncType, GlobalType, Limits, MemoryType, RefType, TableType},
    ValueType,
};

/// Convert a format binary value type to runtime value type
///
/// This function maps the binary format value types (from wrt-format)
/// to the runtime value types (from wrt-types).
pub fn byte_to_value_type(byte: u8) -> Result<ValueType> {
    match byte {
        0x7F => Ok(ValueType::I32),
        0x7E => Ok(ValueType::I64),
        0x7D => Ok(ValueType::F32),
        0x7C => Ok(ValueType::F64),
        0x70 => Ok(ValueType::FuncRef),
        0x6F => Ok(ValueType::ExternRef),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::INVALID_TYPE,
            "Invalid WebAssembly value type.",
        )),
    }
}

/// Convert a runtime value type to format binary value type
///
/// This function maps the runtime value types (from wrt-types)
/// to the binary format value types (from wrt-format).
pub fn value_type_to_byte(val_type: &ValueType) -> u8 {
    match val_type {
        ValueType::I32 => 0x7F,
        ValueType::I64 => 0x7E,
        ValueType::F32 => 0x7D,
        ValueType::F64 => 0x7C,
        ValueType::V128 => unimplemented!("V128 to byte mapping is not yet defined"),
        ValueType::FuncRef => 0x70,
        ValueType::ExternRef => 0x6F,
    }
}

/// Convert a format error to a wrt error
pub fn format_error_to_wrt_error<E: Debug>(error: E) -> Error {
    let code = codes::PARSE_ERROR; // Default to generic parse error

    Error::new(
        ErrorCategory::Parse,
        code,
        format!("Format error: {error:?}"),
    )
}

/// Convert a format error into a wrt error
pub fn convert_to_wrt_error(error: WrtFormatError) -> Error {
    format_error_to_wrt_error(error)
}

/// Convert a section code into a section type
pub fn section_code_to_section_type(section_code: u8) -> wrt_format::section::Section {
    // Simple conversion to section enum
    match section_code {
        0 => wrt_format::section::Section::Custom(CustomSection {
            name: String::new(),
            data: Vec::new(),
        }),
        1 => wrt_format::section::Section::Type(Vec::new()),
        2 => wrt_format::section::Section::Import(Vec::new()),
        3 => wrt_format::section::Section::Function(Vec::new()),
        4 => wrt_format::section::Section::Table(Vec::new()),
        5 => wrt_format::section::Section::Memory(Vec::new()),
        6 => wrt_format::section::Section::Global(Vec::new()),
        7 => wrt_format::section::Section::Export(Vec::new()),
        8 => wrt_format::section::Section::Start(Vec::new()),
        9 => wrt_format::section::Section::Element(Vec::new()),
        10 => wrt_format::section::Section::Code(Vec::new()),
        11 => wrt_format::section::Section::Data(Vec::new()),
        12 => wrt_format::section::Section::DataCount(Vec::new()),
        _ => wrt_format::section::Section::Custom(CustomSection {
            name: format!("Unknown_{}", section_code),
            data: Vec::new(),
        }),
    }
}

/// Convert a section type into a section code
pub fn section_type_to_section_code(section_type: wrt_format::section::Section) -> u8 {
    // Simple conversion from section enum
    match section_type {
        wrt_format::section::Section::Custom(_) => 0,
        wrt_format::section::Section::Type(_) => 1,
        wrt_format::section::Section::Import(_) => 2,
        wrt_format::section::Section::Function(_) => 3,
        wrt_format::section::Section::Table(_) => 4,
        wrt_format::section::Section::Memory(_) => 5,
        wrt_format::section::Section::Global(_) => 6,
        wrt_format::section::Section::Export(_) => 7,
        wrt_format::section::Section::Start(_) => 8,
        wrt_format::section::Section::Element(_) => 9,
        wrt_format::section::Section::Code(_) => 10,
        wrt_format::section::Section::Data(_) => 11,
        wrt_format::section::Section::DataCount(_) => 12,
    }
}

/// Convert a format value type to a runtime value type
pub fn format_value_type_to_value_type(format_type: &FormatValueType) -> ValueType {
    match format_type {
        FormatValueType::I32 => ValueType::I32,
        FormatValueType::I64 => ValueType::I64,
        FormatValueType::F32 => ValueType::F32,
        FormatValueType::F64 => ValueType::F64,
        FormatValueType::V128 => unimplemented!("V128 to ValueType (format) mapping is not yet defined"),
        FormatValueType::FuncRef => ValueType::FuncRef,
        FormatValueType::ExternRef => ValueType::ExternRef,
    }
}

/// Convert a runtime value type to a format value type
pub fn value_type_to_format_value_type(value_type: &ValueType) -> FormatValueType {
    match value_type {
        ValueType::I32 => FormatValueType::I32,
        ValueType::I64 => FormatValueType::I64,
        ValueType::F32 => FormatValueType::F32,
        ValueType::F64 => FormatValueType::F64,
        ValueType::V128 => unimplemented!("V128 to FormatValueType mapping is not yet defined"),
        ValueType::FuncRef => FormatValueType::FuncRef,
        ValueType::ExternRef => FormatValueType::ExternRef,
    }
}

/// Convert a sequence of format value types to runtime value types
pub fn format_value_types_to_value_types(format_types: &[FormatValueType]) -> Vec<ValueType> {
    format_types
        .iter()
        .map(format_value_type_to_value_type)
        .collect()
}

/// Convert format limits to runtime limits
pub fn format_limits_to_types_limits(format_limits: &wrt_format::types::Limits) -> Limits {
    Limits {
        min: format_limits.min as u32,
        max: format_limits.max.map(|m| m as u32),
    }
}

/// Convert runtime limits to format limits
pub fn types_limits_to_format_limits(types_limits: &Limits) -> wrt_format::types::Limits {
    wrt_format::types::Limits {
        min: types_limits.min as u64,
        max: types_limits.max.map(|m| m as u64),
        memory64: false,
        shared: false,
    }
}

/// Convert format limits to component limits
pub fn format_limits_to_component_limits(
    format_limits: &wrt_format::types::Limits,
) -> wrt_types::component::Limits {
    wrt_types::component::Limits {
        min: format_limits.min as u32,
        max: format_limits.max.map(|m| m as u32),
    }
}

/// Convert component limits to format limits
pub fn component_limits_to_format_limits(
    comp_limits: &wrt_types::component::Limits,
) -> wrt_format::types::Limits {
    wrt_format::types::Limits {
        min: comp_limits.min as u64,
        max: comp_limits.max.map(|m| m as u64),
        memory64: false,
        shared: false,
    }
}

/// Convert format ref type to runtime ref type
pub fn format_ref_type_to_types_ref_type(format_type: &FormatRefType) -> RefType {
    match format_type {
        FormatRefType::Funcref => RefType::Funcref,
        FormatRefType::Externref => RefType::Externref,
    }
}

/// Convert runtime ref type to format ref type
pub fn types_ref_type_to_format_ref_type(types_type: &RefType) -> FormatRefType {
    match types_type {
        RefType::Funcref => FormatRefType::Funcref,
        RefType::Externref => FormatRefType::Externref,
    }
}

/// Convert a format function type to a runtime function type
pub fn format_func_type_to_types_func_type(format_type: &wrt_format::FuncType) -> Result<FuncType> {
    FuncType::new(
        format_value_types_to_value_types(&format_type.params),
        format_value_types_to_value_types(&format_type.results),
    )
}

/// Convert a format global type to a runtime global type
pub fn format_global_to_types_global(
    format_global: &wrt_format::module::Global,
) -> Result<GlobalType> {
    let initial_value = parse_and_evaluate_const_expr(&format_global.init)?;

    // format_global.global_type is wrt_format::types::FormatGlobalType
    // which has value_type: wrt_types::ValueType and mutable: bool
    let declared_value_type = format_global.global_type.value_type;

    if initial_value.value_type() != declared_value_type {
        return Err(Error::new(
            ErrorCategory::Type,
            codes::TYPE_MISMATCH_ERROR,
            format!(
                "Constant expression evaluated to type {:?} but global declared as {:?}",
                initial_value.value_type(),
                declared_value_type
            ),
        ));
    }

    Ok(GlobalType {
        value_type: declared_value_type,
        mutable: format_global.global_type.mutable,
        initial_value,
    })
}

/// Convert a format memory type to a runtime memory type
pub fn format_memory_type_to_types_memory_type(
    format_type: &wrt_format::module::Memory,
) -> MemoryType {
    MemoryType {
        limits: format_limits_to_types_limits(&format_type.limits),
        shared: format_type.shared,
    }
}

/// Convert a format table type to a runtime table type
pub fn format_table_type_to_types_table_type(format_type: &wrt_format::module::Table) -> TableType {
    TableType {
        element_type: format_value_type_to_value_type(&format_type.element_type),
        limits: format_limits_to_types_limits(&format_type.limits),
    }
}

// --- Import Conversion ---

pub fn format_import_desc_to_types_import_desc(
    format_desc: &wrt_format::module::ImportDesc,
) -> Result<wrt_types::types::ImportDesc> {
    match format_desc {
        wrt_format::module::ImportDesc::Function(type_idx) => {
            Ok(wrt_types::types::ImportDesc::Function(*type_idx))
        }
        wrt_format::module::ImportDesc::Table(format_table) => {
            let types_table_type = format_table_type_to_types_table_type(format_table);
            Ok(wrt_types::types::ImportDesc::Table(types_table_type))
        }
        wrt_format::module::ImportDesc::Memory(format_memory) => {
            let types_memory_type = format_memory_type_to_types_memory_type(format_memory);
            Ok(wrt_types::types::ImportDesc::Memory(types_memory_type))
        }
        wrt_format::module::ImportDesc::Global(format_global) => {
            let types_import_global_type = wrt_types::types::ImportGlobalType {
                value_type: format_global.value_type,
                mutable: format_global.mutable,
            };
            Ok(wrt_types::types::ImportDesc::Global(types_import_global_type))
        }
        // wrt_format::module::ImportDesc::Tag is not yet in wrt_types::types::ImportDesc
        // Add if/when Tag support is complete in wrt-types
    }
}

pub fn format_import_to_types_import(
    format_import: &wrt_format::module::Import,
) -> Result<wrt_types::types::Import> {
    let types_desc = format_import_desc_to_types_import_desc(&format_import.desc)?;
    Ok(wrt_types::types::Import {
        module: format_import.module.clone(),
        name: format_import.name.clone(),
        desc: types_desc,
    })
}

// --- Export Conversion ---

pub fn format_export_to_types_export(
    format_export: &wrt_format::module::Export,
) -> Result<wrt_types::types::Export> {
    let types_export_desc = match format_export.kind {
        wrt_format::module::ExportKind::Function => {
            wrt_types::types::ExportDesc::Function(format_export.index)
        }
        wrt_format::module::ExportKind::Table => {
            wrt_types::types::ExportDesc::Table(format_export.index)
        }
        wrt_format::module::ExportKind::Memory => {
            wrt_types::types::ExportDesc::Memory(format_export.index)
        }
        wrt_format::module::ExportKind::Global => {
            wrt_types::types::ExportDesc::Global(format_export.index)
        }
        // wrt_format::module::ExportKind::Tag not yet in wrt_types::types::ExportDesc
    };
    Ok(wrt_types::types::Export {
        name: format_export.name.clone(),
        desc: types_export_desc,
    })
}

// --- Const Expression Parsing ---
// This is a simplified version focusing on *.const instructions.
// It assumes the input `expr_bytes` is the raw init expression (opcodes + end).
pub(crate) fn parse_and_evaluate_const_expr(
    expr_bytes: &[u8],
) -> Result<wrt_types::values::Value> {
    // Ensure there's at least one byte for instruction and one for END.
    if expr_bytes.len() < 2 {
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::PARSE_ERROR,
            "Constant expression too short",
        ));
    }

    // Check for END opcode at the end of the expression
    // Global init expressions are `expr END` where expr is a single instruction.
    // Data/Element offsets are also `expr END`.
    // The parse_instructions function in instructions.rs already handles the END opcode if present within its input.
    // So we can pass expr_bytes directly to it.

    // Let's assume expr_bytes is just the sequence of instructions *without* the final END
    // if the section parser already consumes the END. Or, if parse_instructions expects it.
    // The spec for init_expr says "expr must be a constant expression".
    // A constant expression is an instruction sequence that produces a single value of the required type
    // and consists of a single `i*.const`, `f*.const`, `ref.null`, `ref.func`, or `global.get` instruction.
    // The `code` section parsing for function bodies already uses parse_instructions which expects an END.
    // Global init_expr, data offset, element offset are `expr`, and this `expr` is further defined as sequence of instructions terminated by `end`.
    // So, parse_instructions should be suitable here.

    let (instructions, _bytes_read) = crate::instructions::parse_instructions(expr_bytes)?;

    if instructions.is_empty() {
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::PARSE_ERROR,
            "Constant expression cannot be empty",
        ));
    }

    if instructions.len() > 1 {
        // Technically, Wasm allows multiple instructions if they resolve to one value on stack (e.g. drop; i32.const 1)
        // But for MVP constant expressions, it's usually a single producing instruction.
        // For simplicity and strictness for now, let's expect one main producer instruction.
        // Or, we'd need a mini-evaluator here.
        // The spec says "a single X.const instruction, a global.get instruction, or a ref.null instruction".
        // So, a single instruction is the correct expectation for MVP constant expressions.
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::PARSE_ERROR,
            format!("Constant expression must be a single instruction, found {}", instructions.len()),
        ));
    }

    match instructions.first().unwrap() { // Safe due to len checks
        crate::instructions::Instruction::I32Const(val) => Ok(wrt_types::values::Value::I32(*val)),
        crate::instructions::Instruction::I64Const(val) => Ok(wrt_types::values::Value::I64(*val)),
        crate::instructions::Instruction::F32Const(val) => Ok(wrt_types::values::Value::F32(*val)), // Assuming Instruction enum stores f32 directly
        crate::instructions::Instruction::F64Const(val) => Ok(wrt_types::values::Value::F64(*val)), // Assuming Instruction enum stores f64 directly
        // TODO: Handle ref.null <type> -> Value::RefNull( соответствующий RefType из wrt_types)
        // TODO: Handle ref.func <idx> -> Value::FuncRef(FuncRefValue::Actual(idx)) or similar
        // TODO: Handle global.get <imported_global_idx> (this requires context of imported globals)
        ref instr => Err(Error::new(
            ErrorCategory::Parse, 
            codes::UNSUPPORTED_CONST_EXPR_OPERATION, 
            format!("Unsupported instruction in constant expression: {:?}", instr)
        )),
    }
}

// --- Data Segment Conversion ---
pub fn format_data_to_types_data_segment(
    format_data: &wrt_format::module::Data,
) -> Result<wrt_types::types::DataSegment> {
    let types_data_mode = match format_data.mode {
        wrt_format::module::DataMode::Active => {
            let offset_value = parse_and_evaluate_const_expr(&format_data.offset)?;
            wrt_types::types::DataMode::Active {
                memory_index: format_data.memory_idx, // Use from format_data directly
                offset: offset_value,
            }
        }
        wrt_format::module::DataMode::Passive => wrt_types::types::DataMode::Passive,
    };

    Ok(wrt_types::types::DataSegment {
        mode: types_data_mode,
        init: format_data.init.clone(), // Directly clone the byte vector
    })
}

// --- Element Segment Conversion ---
pub fn format_element_to_types_element_segment(
    format_element: &wrt_format::module::Element,
) -> Result<wrt_types::types::ElementSegment> {
    // Assuming wrt_format::module::Element always represents an active, funcref element segment
    // as per its current structure: { table_idx: u32, offset: Vec<u8>, init: Vec<u32> }

    let offset_value = parse_and_evaluate_const_expr(&format_element.offset)?;
    
    let types_element_mode = wrt_types::types::ElementMode::Active {
        table_index: format_element.table_idx,
        offset: offset_value,
    };

    // For MVP, elements are funcrefs. wrt_format::Element implicitly means funcref.
    let types_element_type = wrt_types::types::RefType::Funcref;

    // items are directly from format_element.init (which is Vec<u32> of func indices)
    let types_items: Vec<u32> = format_element.init.clone();

    Ok(wrt_types::types::ElementSegment {
        mode: types_element_mode,
        element_type: types_element_type,
        items: types_items,
    })
}
