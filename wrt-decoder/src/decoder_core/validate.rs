//! WebAssembly module validation.
//!
//! This module provides functionality for validating WebAssembly modules
//! according to the WebAssembly specification.

use crate::module::Module;
use crate::prelude::*;
// Use the proper imports from wrt_format instead of local sections
use wrt_error::{codes, kinds, Error, ErrorCategory, Result};
use wrt_format::types::CoreWasmVersion;
// REMOVED: use wrt_format::module::{DataMode, ExportKind, Global, ImportDesc, Memory, Table};
// REMOVED: use wrt_format::types::{FuncType, Limits};

// Explicitly use types from wrt_types for clarity in this validation context
use wrt_types::types::{
    ImportDesc as TypesImportDesc,
    ExportDesc as TypesExportDesc,
    TableType as TypesTableType,
    MemoryType as TypesMemoryType,
    GlobalType as TypesGlobalType,
    Limits as TypesLimits,
    FuncType as TypesFuncType,
    ValueType as TypesValueType, // Already in prelude, but good for explicitness if needed below
    DataMode as TypesDataMode,    // For DataSegment validation later
    ElementMode as TypesElementMode, // Added for validate_elements
    RefType as TypesRefType,         // Added for validate_elements
    // Add other wrt_types::types as needed for other validation functions
};
// For types that are only defined in wrt_format and are used as arguments to validation helpers
// that specifically operate on format-level details (if any, most should operate on wrt_types).
// For now, let's assume most validation helpers will be adapted to wrt_types.
// If a validation function *must* take a wrt_format type, it should be explicitly imported here or qualified.
// Example: use wrt_format::module::Global as FormatGlobal;

/// Validation configuration options
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum allowed function count
    pub max_function_count: usize,
    /// Maximum allowed import count
    pub max_import_count: usize,
    /// Maximum allowed export count
    pub max_export_count: usize,
    /// Maximum allowed memory size (in pages)
    pub max_memory_size: u32,
    /// Maximum allowed table size
    pub max_table_size: u32,
    /// Whether to verify function bodies
    pub verify_function_bodies: bool,
    /// Whether to verify memory limits
    pub verify_memory_limits: bool,
    /// Whether to verify table limits
    pub verify_table_limits: bool,
    /// Whether to perform strict validation (true) or relaxed validation (false)
    pub strict: bool,
    /// Maximum number of locals in a function
    pub max_locals: u32,
    /// Maximum number of globals
    pub max_globals: u32,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_function_count: 10000,
            max_import_count: 1000,
            max_export_count: 1000,
            max_memory_size: 65536, // 4GB
            max_table_size: 10000000,
            verify_function_bodies: true,
            verify_memory_limits: true,
            verify_table_limits: true,
            strict: true,
            max_locals: 50000,
            max_globals: 1000,
        }
    }
}

impl ValidationConfig {
    /// Create a new validation configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a validation configuration with relaxed settings
    pub fn relaxed() -> Self {
        Self {
            strict: false,
            verify_function_bodies: false,
            verify_memory_limits: false,
            verify_table_limits: false,
            ..Self::default()
        }
    }
}

/// Basic validation of a WebAssembly module
pub fn validate_module(module: &Module) -> Result<()> {
    validate_module_with_config(module, &ValidationConfig::default())
}

/// Validate a WebAssembly module with custom configuration
pub fn validate_module_with_config(module: &Module, config: &ValidationConfig) -> Result<()> {
    // Check for unique export names
    let mut export_names = Vec::new();
    for export in &module.exports {
        if export_names.contains(&export.name) {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Duplicate export name: {}", export.name),
            ));
        }
        export_names.push(export.name.clone());
    }

    // Skip further validation if we're in relaxed mode
    if !config.strict {
        return Ok(());
    }

    // Apply limits based on configuration
    if module.functions.len() > config.max_function_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Module has too many functions: {} (max: {})",
                module.functions.len(),
                config.max_function_count
            ),
        ));
    }

    if module.imports.len() > config.max_import_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Module has too many imports: {} (max: {})",
                module.imports.len(),
                config.max_import_count
            ),
        ));
    }

    if module.exports.len() > config.max_export_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Module has too many exports: {} (max: {})",
                module.exports.len(),
                config.max_export_count
            ),
        ));
    }

    // Validate basic structure, which we always do
    validate_basic_structure(module)?;
    validate_types(module)?;
    validate_imports(module)?;
    validate_functions(module)?;
    validate_tables(module)?;
    validate_memories(module)?;
    validate_globals(module)?;
    validate_exports(module)?;
    validate_start(module)?;
    validate_elements(module)?;
    validate_data(module)?;

    // Hypothetical Finding F5: Validate TypeInformation section for Wasm 3.0
    if module.core_version == CoreWasmVersion::V3_0 {
        if module.type_info_section.is_some() {
            validate_type_information_section(module)?;
        }
    } else {
        // If it's not Wasm 3.0, the type_info_section should not have been parsed.
        // The parser should ideally prevent this section from being populated in Module for V2_0.
        // If it still gets populated due to lenient parsing of unknown sections, this is a validation error.
        if module.type_info_section.is_some() {
            return Err(kinds::validation_error(
                "TypeInformation section (ID 15) is invalid for non-Wasm3.0 modules",
            ));
        }
    }

    // Validate code if configured to do so
    if config.verify_function_bodies {
        validate_code(module)?;
    }

    Ok(())
}

/// Validate the basic structure of a WebAssembly module
fn validate_basic_structure(module: &Module) -> Result<()> {
    // Check if we have a function section but no code section
    if !module.functions.is_empty() && module.code.is_empty() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            kinds::ValidationError("Module has function section but no code section".to_string()),
        ));
    }

    // Check if we have a code section but no function section
    if module.functions.is_empty() && !module.code.is_empty() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            kinds::ValidationError("Module has code section but no function section".to_string()),
        ));
    }

    // Check that function and code sections match in size
    if module.functions.len() != module.code.len() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            #[cfg(feature = "std")]
            kinds::ValidationError(format!(
                "Function and code sections have mismatched lengths: {} vs {}",
                module.functions.len(),
                module.code.len()
            )),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            kinds::ValidationError(alloc::format!(
                "Function and code sections have mismatched lengths: {} vs {}",
                module.functions.len(),
                module.code.len()
            )),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            kinds::ValidationError(
                "Function and code sections have mismatched lengths".to_string(),
            ),
        ));
    }

    Ok(())
}

/// Validate the types section of a WebAssembly module
fn validate_types(module: &Module) -> Result<()> {
    for (i, func_type) in module.types.iter().enumerate() {
        // Validate function type
        validate_func_type(func_type, i)?;
    }
    Ok(())
}

/// Validate a value type
fn validate_value_type(value_type: &TypesValueType, context: &str) -> Result<()> {
    // In MVPv1, only i32, i64, f32, and f64 are valid
    match value_type {
        TypesValueType::I32 | TypesValueType::I64 | TypesValueType::F32 | TypesValueType::F64 => Ok(()),
        TypesValueType::FuncRef | TypesValueType::ExternRef | TypesValueType::V128 => {
            // Reference types and V128 are part of later specifications
            Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!(
                    "Підтримка \"{}\": type {:?} not supported in MVPv1",
                    context,
                    value_type
                ),
            ))
        }
    }
}

/// Validate function type
fn validate_func_type(func_type: &TypesFuncType, type_idx: usize) -> Result<()> {
    // Validate parameter types
    for (i, param) in func_type.params.iter().enumerate() {
        let context = format!("parameter {} of type {}", i, type_idx);
        validate_value_type(param, &context)?;
    }

    // Validate result types
    for (i, result) in func_type.results.iter().enumerate() {
        let context = format!("result {} of type {}", i, type_idx);
        validate_value_type(result, &context)?;
    }

    // In MVP, functions can have at most one result
    if func_type.results.len() > 1 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Function type {} has {} results (max: 1 in MVP)",
                type_idx,
                func_type.results.len()
            ),
        ));
    }

    Ok(())
}

/// Validate the imports section of a WebAssembly module
fn validate_imports(module: &Module) -> Result<()> {
    for (idx, import) in module.imports.iter().enumerate() {
        // Existing UTF-8 validation for import.name and import.module should remain if present.
        // Example of what might exist or should be added:
        // if validate_utf8(import.name.as_bytes()).is_err() { /* ... error ... */ }
        // if validate_utf8(import.module.as_bytes()).is_err() { /* ... error ... */ }

        match &import.desc {
            wrt_format::module::ImportDesc::Function(type_idx) => {
                if *type_idx as usize >= module.types.len() {
                    return Err(validation_error_with_context(
                        &format!("Imported function uses out-of-bounds type index: {}", type_idx),
                        &format!("import {}", idx),
                    ));
                }
                // Further validation: module.types[*type_idx] should represent a FuncType.
                // This depends on how module.types is populated (e.g. if it stores wrt_types::FuncType or similar).
            }
            wrt_format::module::ImportDesc::Table(table_type) => {
                // table_type is wrt_format::module::Table
                validate_value_type(&table_type.element_type, "imported table element type")?;
                if !matches!(table_type.element_type, TypesValueType::FuncRef | TypesValueType::ExternRef) {
                    return Err(validation_error_with_context(
                        "Imported table has invalid element type (must be funcref or externref)",
                        &format!("import {}", idx),
                    ));
                }
                // TODO: Validate table_type.limits (e.g., using a version of validate_limits)
                // validate_format_limits(&table_type.limits, config.max_table_size)?; 
            }
            wrt_format::module::ImportDesc::Memory(memory_type) => {
                // memory_type is wrt_format::module::Memory
                // TODO: Validate memory_type.limits (e.g., using a version of validate_limits)
                // validate_format_limits(&memory_type.limits, config.max_memory_size)?; 
                // TODO: Validate memory_type.shared (e.g. if shared, max must be present)
            }
            wrt_format::module::ImportDesc::Global(global_type) => {
                // global_type is wrt_format::types::FormatGlobalType
                // validate_value_type is already version-aware for I16x8 due to previous changes.
                validate_value_type(&global_type.value_type, "imported global type")?;
                // MVP disallows mutable imported globals, but Wasm spec evolved. For now, allow as per struct.
            }
            // Hypothetical Finding F6: Validate Tag import
            wrt_format::module::ImportDesc::Tag(type_idx) => {
                if module.core_version != CoreWasmVersion::V3_0 {
                    return Err(validation_error_with_context(
                        "Tag import kind is only valid for Wasm 3.0 modules.",
                        &format!("import {} ('{}' from '{}')", idx, import.name, import.module),
                    ));
                }
                if *type_idx as usize >= module.types.len() {
                    return Err(validation_error_with_context(
                        &format!("Imported tag uses out-of-bounds type index: {} (max types {}).", type_idx, module.types.len()),
                        &format!("import {} ('{}' from '{}')", idx, import.name, import.module),
                    ));
                }
                // TODO: Ensure module.types[*type_idx] is a function type, if module.types stores FuncType objects.
            }
        }
    }
    Ok(())
}

/// Validate the memories section of a WebAssembly module
pub fn validate_memories(module: &Module) -> Result<()> {
    // In MVP, only one memory is allowed
    if module.memories.len() > 1 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Multiple memories are not allowed in MVP".to_string(),
        ));
    }

    // Count imported memories
    let imported_memories = module
        .imports
        .iter()
        .filter(|import| matches!(import.desc, TypesImportDesc::Memory(_)))
        .count();

    // Total memory count is defined memories + imported memories
    let total_memories = module.memories.len() + imported_memories;

    if total_memories > 1 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Too many memories: {} defined + {} imported = {} (max 1)",
                module.memories.len(),
                imported_memories,
                total_memories
            ),
        ));
    }

    // Validate each memory
    for memory in &module.memories {
        validate_memory_type(memory)?;
    }

    Ok(())
}

/// Validate the globals section of a WebAssembly module
fn validate_globals(module: &Module) -> Result<()> {
    for global in &module.globals {
        validate_global_type(global)?;
    }

    Ok(())
}

/// Validate the exports section of a WebAssembly module
fn validate_exports(module: &Module) -> Result<()> {
    for (idx, export) in module.exports.iter().enumerate() {
        // Existing UTF-8 validation for export.name should remain if present.

        match export.kind {
            wrt_format::module::ExportKind::Function => {
                validate_func_idx(module, export.index, idx)?;
            }
            wrt_format::module::ExportKind::Table => {
                validate_table_idx(module, export.index, idx)?;
            }
            wrt_format::module::ExportKind::Memory => {
                validate_memory_idx(module, export.index, idx)?;
            }
            wrt_format::module::ExportKind::Global => {
                validate_global_idx(module, export.index, idx)?;
                // Additionally, exported globals must not be mutable in Wasm MVP.
                // This rule might have changed. Check current spec if strict validation is needed.
                // If module.globals[export.index].global_type.mutable { ... error ... }
            }
            // Hypothetical Finding F6: Validate Tag export
            wrt_format::module::ExportKind::Tag => {
                if module.core_version != CoreWasmVersion::V3_0 {
                    return Err(validation_error_with_context(
                        "Tag export kind is only valid for Wasm 3.0 modules.",
                        &format!("export {} ('{}')", idx, export.name),
                    ));
                }
                // In the Wasm Tag proposal, exported tags refer to a tag definition index.
                // The current `wrt-format::module::Export` struct has `index: u32` which would be this tag_idx.
                // We need to validate this `export.index` against a (yet undefined) list of tags in the module.
                // For now, let's assume there's a `module.tags` (Vec<TagDefinition>) or similar.
                // This part of validation is incomplete without knowing how tags are defined in the module structure.
                // For example: if export.index as usize >= module.defined_tags.len() { ... error ... }

                // A common pattern for tags is that they also have an associated function type signature.
                // If the `export.index` for a Tag export actually refers to a type_index (for its signature)
                // rather than a separate tag definition index, then the validation would be:
                // if export.index as usize >= module.types.len() {
                //     return Err(validation_error_with_context(...));
                // }
                // And ensure module.types[export.index] is a function type.
                // Given ExportKind::Tag was added to wrt-format without changing Export struct, export.index is used.
                // Let's assume for now `export.index` for a Tag refers to a type index (its function signature).
                if export.index as usize >= module.types.len() {
                     return Err(validation_error_with_context(
                        &format!(
                            "Exported tag '{}' (idx {}) refers to an out-of-bounds type index: {} (max types {}).", 
                            export.name, idx, export.index, module.types.len()
                        ),
                        &format!("export {} ('{}')", idx, export.name),
                    ));
                }
                // TODO: Ensure module.types[export.index] is a function type.
            }
        }
    }
    Ok(())
}

/// Validate a function index (used for exports)
fn validate_func_idx(module: &Module, idx: u32, _export_idx: usize) -> Result<()> {
    let func_count = module.functions.len() as u32;
    let imported_func_count = module
        .imports
        .iter()
        .filter(|import| matches!(import.desc, TypesImportDesc::Function(_)))
        .count() as u32;

    if idx >= func_count + imported_func_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Invalid function index {} (max: {})",
                idx,
                func_count + imported_func_count - 1
            ),
        ));
    }

    Ok(())
}

/// Validate a table index (used for exports)
fn validate_table_idx(module: &Module, idx: u32, _export_idx: usize) -> Result<()> {
    let table_count = module.tables.len() as u32;
    let imported_table_count = module
        .imports
        .iter()
        .filter(|import| matches!(import.desc, TypesImportDesc::Table(_)))
        .count() as u32;

    if idx >= table_count + imported_table_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Invalid table index {} (max: {})",
                idx,
                table_count + imported_table_count - 1
            ),
        ));
    }

    Ok(())
}

/// Validate a memory index (used for exports)
fn validate_memory_idx(module: &Module, idx: u32, _export_idx: usize) -> Result<()> {
    let memory_count = module.memories.len() as u32;
    let imported_memory_count = module
        .imports
        .iter()
        .filter(|import| matches!(import.desc, TypesImportDesc::Memory(_)))
        .count() as u32;

    if idx >= memory_count + imported_memory_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Invalid memory index {} (max: {})",
                idx,
                memory_count + imported_memory_count - 1
            ),
        ));
    }

    Ok(())
}

/// Validate a global index (used for exports)
fn validate_global_idx(module: &Module, idx: u32, _export_idx: usize) -> Result<()> {
    let global_count = module.globals.len() as u32;
    let imported_global_count = module
        .imports
        .iter()
        .filter(|import| matches!(import.desc, TypesImportDesc::Global(_)))
        .count() as u32;

    if idx >= global_count + imported_global_count {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Invalid global index {} (max: {})",
                idx,
                global_count + imported_global_count - 1
            ),
        ));
    }

    Ok(())
}

/// Validate the start function of a WebAssembly module
fn validate_start(module: &Module) -> Result<()> {
    if let Some(start_func) = module.start {
        // Validate function index
        validate_func_idx(module, start_func, 0)?;

        // In MVP, the start function must have type [] -> []
        let _func_count = module.functions.len() as u32;
        let imported_func_count = module
            .imports
            .iter()
            .filter(|import| matches!(import.desc, TypesImportDesc::Function(_)))
            .count() as u32;

        let mut type_idx = None;

        if start_func < imported_func_count {
            // Get type index from import
            let import_idx = start_func as usize;
            let mut count = 0;
            for import in &module.imports {
                if let TypesImportDesc::Function(idx) = import.desc {
                    if count == import_idx {
                        type_idx = Some(idx);
                        break;
                    }
                    count += 1;
                }
            }
        } else {
            // Get type index from function section
            let func_idx = (start_func - imported_func_count) as usize;
            if func_idx < module.functions.len() {
                type_idx = Some(module.functions[func_idx]);
            }
        }

        if let Some(type_idx) = type_idx {
            if type_idx as usize >= module.types.len() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!("Invalid type index {} for start function", type_idx),
                ));
            }

            let func_type = &module.types[type_idx as usize];
            if !func_type.params.is_empty() || !func_type.results.is_empty() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    "Start function must have type [] -> []".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Validate the elements section of a WebAssembly module
fn validate_elements(module: &Module) -> Result<()> {
    for (i, elem) in module.elements.iter().enumerate() {
        match &elem.mode {
            TypesElementMode::Active { table_index, offset } => {
                // In MVP, only table 0 is allowed for active segments implicitly defined with prefix 0x00.
                // The ElementSegment in wrt_types directly stores table_index and offset from the parsed init_expr.
                // If elem.table_idx was > 0 for an MVP-style segment, it would be an issue, but
                // our wrt_format::binary::parse_element for 0x00 prefix hardcodes table_idx to 0.
                // More complex element segments (types 0x01-0x07) would require different checks.
                if *table_index != 0 {
                    // This case should ideally not be hit if parsing only MVP 0x00 prefix from format layer
                    // or if conversion logic correctly maps other format segment types.
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        format!(
                            "Element segment {} targets non-zero table index {} (MVP only supports table 0 for this form)",
                            i, table_index
                        ),
                    ));
                }
                validate_table_idx(module, *table_index, i)?;
                // Validate offset (must be a const expression resulting in i32)
                // elem.offset is already a wrt_types::values::Value, so its const_expr nature was checked at conversion.
                if offset.value_type() != TypesValueType::I32 {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::TYPE_MISMATCH_ERROR,
                        format!(
                            "Element segment {} offset expression must be I32, got {:?}",
                            i, offset.value_type()
                        ),
                    ));
                }
            }
            TypesElementMode::Passive => {
                // Passive segments are fine.
            }
            TypesElementMode::Declared => {
                // Declarative segments are fine.
            }
        }

        // Validate element type (must be funcref or externref for now)
        match elem.element_type {
            TypesRefType::Funcref | TypesRefType::Externref => (),
            // Other RefTypes might be part of future proposals.
        }

        // Validate function indices in items
        for (j, func_idx) in elem.items.iter().enumerate() { // Use elem.items
            validate_func_idx(module, *func_idx, j)?;
        }
    }
    Ok(())
}

/// Validate the data section of a WebAssembly module
fn validate_data(module: &Module) -> Result<()> {
    for (i, data_segment) in module.data.iter().enumerate() {
        match &data_segment.mode {
            TypesDataMode::Active { memory_index, offset } => {
                if *memory_index != 0 {
                    // MVP allows only memory index 0
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        format!(
                            "Data segment {} targets non-zero memory index {} (MVP only supports memory 0)",
                            i, memory_index
                        ),
                    ));
                }
                validate_memory_idx(module, *memory_index, i)?;
                // Validate offset (must be a const expression resulting in i32)
                if offset.value_type() != TypesValueType::I32 {
                     return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::TYPE_MISMATCH_ERROR,
                        format!(
                            "Data segment {} offset expression must be I32, got {:?}",
                            i, offset.value_type()
                        ),
                    ));
                }
            }
            TypesDataMode::Passive => {
                // Passive segments are fine, no memory index or offset to validate here directly.
            }
        }
        // data_segment.init is Vec<u8>, no specific validation here other than it exists.
        // Max data segment size checks could be added if needed, based on config.
    }
    Ok(())
}

/// Validate constant expression (used in globals, elem, and data segments)
fn validate_const_expr(expr: &[u8], _expected_type: TypesValueType) -> Result<()> {
    // In the MVP, constant expressions are limited to:
    // - i32.const
    // - i64.const
    // - f32.const
    // - f64.const
    // - global.get of an immutable imported global

    // For now, we just do a basic check that the expression isn't empty
    if expr.is_empty() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Constant expression cannot be empty".to_string(),
        ));
    }

    // TODO: Add more comprehensive validation of constant expressions

    Ok(())
}

/// Validate code section of a WebAssembly module
fn validate_code(module: &Module) -> Result<()> {
    // Validate each function body
    for (i, code) in module.code.iter().enumerate() {
        // For MVP, we do basic validation that the code isn't empty
        if code.body.is_empty() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Function body cannot be empty for function {}", i),
            ));
        }

        // TODO: Add more comprehensive code validation in the future
    }

    Ok(())
}

/// Validate the functions section of a WebAssembly module
fn validate_functions(module: &Module) -> Result<()> {
    for (i, type_idx) in module.functions.iter().enumerate() {
        if *type_idx as usize >= module.types.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Invalid type index {} in function {}", type_idx, i),
            ));
        }
    }

    Ok(())
}

/// Validate the tables section of a WebAssembly module
fn validate_tables(module: &Module) -> Result<()> {
    // In MVP, only one table is allowed
    if module.tables.len() > 1 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Multiple tables are not allowed in MVP".to_string(),
        ));
    }

    // Validate each table
    for table in &module.tables {
        validate_table_type(table)?;
    }

    Ok(())
}

/// Validate global type
fn validate_global_type(global: &TypesGlobalType) -> Result<()> {
    validate_value_type(&global.value_type, "global")?;
    // Check that the initial_value's type matches the declared global value_type
    if global.initial_value.value_type() != global.value_type {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::TYPE_MISMATCH_ERROR, // Specific error code
            format!(
                "Global init_expr type mismatch: global declared as {:?}, but init_expr evaluated to {:?}",
                global.value_type,
                global.initial_value.value_type()
            ),
        ));
    }

    // The const_expr validation for global.initial_value itself (i.e., ensuring it *was* derived from a const expr)
    // is tricky to do here because `global.initial_value` is already a `wrt_types::values::Value`.
    // This validation step is typically performed during the parsing and conversion phase
    // (e.g., in `wrt-decoder/src/conversion.rs` when converting `wrt_format::module::Global` to `wrt_types::types::GlobalType`).
    // For now, we trust that the conversion layer has ensured `initial_value` is valid per const expr rules.
    // If deeper validation of the Value itself against const expr rules is needed here,
    // it would require inspecting the Value and knowing its origin or having more context.
    Ok(())
}

/// Validate memory.copy instruction
pub fn validate_memory_copy(
    module: &Module,
    dst_memory_idx: u32,
    src_memory_idx: u32,
) -> Result<()> {
    // Validate destination memory index
    validate_memory_idx(module, dst_memory_idx, 0)?;

    // Validate source memory index
    validate_memory_idx(module, src_memory_idx, 0)?;

    // In WasmMVP, both indices should be 0 as only one memory is allowed
    if dst_memory_idx != 0 || src_memory_idx != 0 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Invalid memory indices for memory.copy: dst={}, src={} (only 0 is valid in MVP)",
                dst_memory_idx, src_memory_idx
            ),
        ));
    }

    Ok(())
}

/// Validate memory.fill instruction
pub fn validate_memory_fill(module: &Module, memory_idx: u32) -> Result<()> {
    // Validate memory index
    validate_memory_idx(module, memory_idx, 0)?;

    // In WasmMVP, memory index should be 0
    if memory_idx != 0 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            format!(
                "Invalid memory index for memory.fill: {} (only 0 is valid in MVP)",
                memory_idx
            ),
        ));
    }

    Ok(())
}

/// Helper function to create validation errors
pub fn validation_error(message: &str) -> Error {
    Error::new(
        ErrorCategory::Validation,
        codes::VALIDATION_ERROR,
        kinds::ValidationError(message.to_string()),
    )
}

/// Helper function to create validation errors with context
pub fn validation_error_with_context(message: &str, context: &str) -> Error {
    Error::new(
        ErrorCategory::Validation,
        codes::VALIDATION_ERROR,
        kinds::ValidationError(format!("{}: {}", context, message)),
    )
}

/// Helper function to create validation errors with type information
pub fn validation_error_with_type(message: &str, type_name: &str) -> Error {
    Error::new(
        ErrorCategory::Validation,
        codes::VALIDATION_ERROR,
        kinds::ValidationError(format!("{} (type: {})", message, type_name)),
    )
}

/// New helper for wrt_types::types::ImportGlobalType
fn validate_import_global_type(global_type: &wrt_types::types::ImportGlobalType) -> Result<()> {
    validate_value_type(&global_type.value_type, "imported global")?;
    // Mutability of imported globals is allowed by spec, though MVP had restrictions.
    // wrt_types::types::ImportGlobalType allows mutable.
    Ok(())
}

/// Hypothetical Finding F5: New function to validate the TypeInformation section
fn validate_type_information_section(module: &Module) -> Result<()> {
    if let Some(section) = &module.type_info_section {
        for entry in &section.entries {
            if entry.type_index as usize >= module.types.len() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::INVALID_TYPE_INDEX, // Using a more specific code
                    format!(
                        "TypeInformationSection: entry refers to type_index {} which is out of bounds (max types {}).",
                        entry.type_index,
                        module.types.len()
                    ),
                ));
            }
            // TODO: Add validation for entry.name (e.g., UTF-8 validity, length) if Wasm 3.0 spec requires.
        }
    }
    Ok(())
}
