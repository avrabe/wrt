// Module implementation for runtime execution
//
// This module provides the core runtime implementation of WebAssembly modules
// used by the runtime execution engine.

use crate::prelude::*;
use crate::global::Global;
use crate::memory::Memory;
use crate::table::Table;
use wrt_types::types::{
    FuncType as WrtFuncType,
    ValueType as WrtValueType,
    LocalEntry as WrtLocalEntry,
    Expr as WrtExpr,
    Instruction as WrtInstruction,
    ImportDesc as WrtImportDesc,
    ExportDesc as WrtExportDesc,
    ElementSegment as WrtElementSegment,
    ElementMode as WrtElementMode,
    DataSegment as WrtDataSegment,
    DataMode as WrtDataMode,
    CustomSection as WrtCustomSection,
    TableType as WrtTableType,
    MemoryType as WrtMemoryType,
    GlobalType as WrtGlobalType,
    Limits as WrtLimits,
    RefType as WrtRefType,
    ImportGlobalType as WrtImportGlobalType,
};
use wrt_types::values::Value as WrtValue;

/// Represents a WebAssembly export kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    /// Function export
    Function,
    /// Table export
    Table,
    /// Memory export
    Memory,
    /// Global export
    Global,
}

/// Represents an export in a WebAssembly module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Export index
    pub index: u32,
}

impl Export {
    /// Creates a new export
    pub fn new(name: String, kind: ExportKind, index: u32) -> Self {
        Self { name, kind, index }
    }
}

/// Represents an import in a WebAssembly module
#[derive(Debug, Clone)]
pub struct Import {
    /// Module name
    pub module: String, 
    /// Import name
    pub name: String,
    /// Import type
    pub ty: ExternType,
}

impl Import {
    /// Creates a new import
    pub fn new(module: String, name: String, ty: ExternType) -> Self {
        Self { module, name, ty }
    }
}

/// Represents a WebAssembly function in the runtime
#[derive(Debug, Clone)]
pub struct Function {
    /// The type index of the function (referring to Module.types)
    pub type_idx: u32,
    /// The parsed local variable declarations
    pub locals: Vec<WrtLocalEntry>,
    /// The parsed instructions that make up the function body
    pub body: WrtExpr,
}

/// Represents the value of an export
#[derive(Debug, Clone)]
pub enum ExportItem {
    /// A function with the specified index
    Function(u32),
    /// A table with the specified index
    Table(Arc<Table>),
    /// A memory with the specified index
    Memory(Arc<Memory>),
    /// A global with the specified index
    Global(Arc<Global>),
}

/// Represents an element segment for tables in the runtime
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub mode: WrtElementMode,
    pub table_idx: Option<u32>,
    pub offset_expr: Option<WrtExpr>,
    pub element_type: WrtRefType,
    pub items: Vec<u32>,
}

/// Represents a data segment for memories in the runtime
#[derive(Debug, Clone)]
pub struct Data {
    pub mode: WrtDataMode,
    pub memory_idx: Option<u32>,
    pub offset_expr: Option<WrtExpr>,
    pub init: Vec<u8>,
}

impl Data {
    /// Returns a reference to the data in this segment
    pub fn data(&self) -> &[u8] {
        &self.init
    }
}

/// Represents a WebAssembly module in the runtime
#[derive(Debug, Clone)]
pub struct Module {
    /// Module types (function signatures)
    pub types: Vec<WrtFuncType>,
    /// Imported functions, tables, memories, and globals
    pub imports: HashMap<String, HashMap<String, Import>>,
    /// Function definitions
    pub functions: Vec<Function>,
    /// Table instances
    pub tables: Vec<Arc<Table>>,
    /// Memory instances
    pub memories: Vec<Arc<Memory>>,
    /// Global variable instances
    pub globals: Vec<Arc<Global>>,
    /// Element segments for tables
    pub elements: Vec<Element>,
    /// Data segments for memories
    pub data: Vec<Data>,
    /// Start function index
    pub start: Option<u32>,
    /// Custom sections
    pub custom_sections: HashMap<String, Vec<u8>>,
    /// Exports (functions, tables, memories, and globals)
    pub exports: HashMap<String, Export>,
    /// Optional name for the module
    pub name: Option<String>,
    /// Original binary (if available)
    pub binary: Option<Vec<u8>>,
    /// Execution validation flag
    pub validated: bool,
}

impl Module {
    /// Creates a new empty module
    pub fn new() -> Result<Self> {
        Ok(Self {
            types: Vec::new(),
            imports: HashMap::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            elements: Vec::new(),
            data: Vec::new(),
            start: None,
            custom_sections: HashMap::new(),
            exports: HashMap::new(),
            name: None,
            binary: None,
            validated: false,
        })
    }

    /// Creates a runtime Module from a wrt_types::types::Module.
    /// This is the primary constructor after decoding.
    pub fn from_wrt_module(wrt_module: &wrt_types::types::Module) -> Result<Self> {
        let mut runtime_module = Self::new()?;

        if let Some(name) = &wrt_module.name { // Assuming Module in wrt_types has an optional name
            runtime_module.name = Some(name.clone());
        }
        runtime_module.start = wrt_module.start;

        for type_def in &wrt_module.types {
            runtime_module.types.push(type_def.clone());
        }

        for import_def in &wrt_module.imports {
            let extern_ty = match &import_def.desc {
                WrtImportDesc::Function(type_idx) => {
                    let ft = runtime_module.types.get(*type_idx as usize).ok_or_else(|| Error::new(ErrorCategory::Validation, codes::TYPE_MISMATCH, "Imported function type index out of bounds"))?.clone();
                    ExternType::Function(ft)
                }
                WrtImportDesc::Table(tt) => ExternType::Table(wrt_types::component::TableType::from_core(tt)),
                WrtImportDesc::Memory(mt) => ExternType::Memory(wrt_types::component::MemoryType::from_core(mt)),
                WrtImportDesc::Global(gt) => ExternType::Global(wrt_types::component::GlobalType{ value_type: gt.value_type, mutable: gt.mutable }),
            };
            runtime_module.imports
                .entry(import_def.module.clone())
                .or_default()
                .insert(import_def.name.clone(), crate::module::Import::new(import_def.module.clone(), import_def.name.clone(), extern_ty));
        }
        
        // Pre-allocate functions vector based on type indices in wrt_module.funcs
        // The actual bodies are filled by wrt_module.code_entries
        runtime_module.functions = Vec::with_capacity(wrt_module.code_entries.len());
        for code_entry in &wrt_module.code_entries {
             // Find the corresponding type_idx from wrt_module.funcs.
             // This assumes wrt_module.funcs has the type indices for functions defined in this module,
             // and wrt_module.code_entries aligns with this.
             // A direct link or combined struct in wrt_types::Module would be better.
             // For now, we assume that the i-th code_entry corresponds to the i-th func type index in wrt_module.funcs
             // (after accounting for imported functions). This needs clarification in wrt_types::Module structure.
             // Let's assume wrt_module.funcs contains type indices for *defined* functions and code_entries matches this.
            let func_idx_in_defined_funcs = runtime_module.functions.len(); // 0-indexed among defined functions
            if func_idx_in_defined_funcs >= wrt_module.funcs.len() {
                return Err(Error::new(ErrorCategory::Validation, codes::VALIDATION_ERROR, "Mismatch between code entries and function type declarations"));
            }
            let type_idx = wrt_module.funcs[func_idx_in_defined_funcs];

            runtime_module.functions.push(Function {
                type_idx,
                locals: code_entry.locals.clone(),
                body: code_entry.body.clone(),
            });
        }


        for table_def in &wrt_module.tables {
            // For now, runtime tables are created empty and populated by element segments or host.
            // This assumes runtime::table::Table::new can take WrtTableType.
            runtime_module.tables.push(Arc::new(Table::new(table_def.clone())?));
        }

        for memory_def in &wrt_module.memories {
            runtime_module.memories.push(Arc::new(Memory::new(memory_def.clone())?));
        }

        for global_def in &wrt_module.globals {
            runtime_module.globals.push(Arc::new(Global::new(global_def.value_type, global_def.mutable, global_def.initial_value.clone())?));
        }

        for export_def in &wrt_module.exports {
            let kind = match export_def.desc {
                WrtExportDesc::Func(_) => ExportKind::Function,
                WrtExportDesc::Table(_) => ExportKind::Table,
                WrtExportDesc::Memory(_) => ExportKind::Memory,
                WrtExportDesc::Global(_) => ExportKind::Global,
                WrtExportDesc::Tag(_) => return Err(Error::new(ErrorCategory::NotSupported, codes::UNSUPPORTED_FEATURE, "Tag exports not supported")),
            };
            runtime_module.exports.insert(export_def.name.clone(), crate::module::Export::new(export_def.name.clone(), kind, export_def.desc.index()));
        }

        for element_def in &wrt_module.elements {
            // This requires significant processing to evaluate offset_expr and items expressions
            // For now, store a simplified version or one that requires instantiation-time evaluation.
            // This is a placeholder and needs robust implementation.
            let items_resolved = match &element_def.items {
                wrt_types::types::ElementItems::Functions(indices) => indices.iter().filter_map(|&opt_idx| opt_idx).collect(),
                wrt_types::types::ElementItems::Expressions(exprs) => {
                    // TODO: Evaluate expressions to get function indices. Placeholder:
                    vec![] // This is incorrect, expressions need evaluation.
                }
            };
            runtime_module.elements.push(crate::module::Element {
                mode: element_def.mode.clone(),
                table_idx: element_def.table_idx,
                offset_expr: element_def.offset_expr.clone(), // Store expression for later evaluation
                element_type: element_def.element_type,
                items: items_resolved, // Store resolved/placeholder items
            });
        }

        for data_def in &wrt_module.data_segments {
            runtime_module.data.push(crate::module::Data {
                mode: data_def.mode.clone(),
                memory_idx: data_def.memory_idx,
                offset_expr: data_def.offset_expr.clone(), // Store expression for later evaluation
                init: data_def.data.clone(),
            });
        }
        
        for custom_def in &wrt_module.custom_sections {
            runtime_module.custom_sections.insert(custom_def.name.clone(), custom_def.data.clone());
        }

        Ok(runtime_module)
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Option<&Export> {
        self.exports.get(name)
    }

    /// Gets a function by index
    pub fn get_function(&self, idx: u32) -> Option<&Function> {
        if idx as usize >= self.functions.len() {
            return None;
        }
        Some(&self.functions[idx as usize])
    }

    /// Gets a function type by index
    pub fn get_function_type(&self, idx: u32) -> Option<&WrtFuncType> {
        if idx as usize >= self.types.len() {
            return None;
        }
        Some(&self.types[idx as usize])
    }

    /// Gets a global by index
    pub fn get_global(&self, idx: usize) -> Result<Arc<Global>> {
        self.globals.get(idx).cloned().ok_or_else(|| {
            Error::new(
                ErrorCategory::Runtime,
                codes::GLOBAL_NOT_FOUND,
                format!("Global at index {} not found", idx)
            )
        })
    }

    /// Gets a memory by index
    pub fn get_memory(&self, idx: usize) -> Result<Arc<Memory>> {
        self.memories.get(idx).cloned().ok_or_else(|| {
            Error::new(
                ErrorCategory::Runtime,
                codes::MEMORY_NOT_FOUND,
                format!("Memory at index {} not found", idx)
            )
        })
    }

    /// Gets a table by index
    pub fn get_table(&self, idx: usize) -> Result<Arc<Table>> {
        self.tables.get(idx).cloned().ok_or_else(|| {
            Error::new(
                ErrorCategory::Runtime,
                codes::TABLE_NOT_FOUND,
                format!("Table at index {} not found", idx)
            )
        })
    }

    /// Adds a function export
    pub fn add_function_export(&mut self, name: String, index: u32) {
        self.exports.insert(
            name.clone(),
            Export::new(name, ExportKind::Function, index),
        );
    }

    /// Adds a table export
    pub fn add_table_export(&mut self, name: String, index: u32) {
        self.exports.insert(
            name.clone(),
            Export::new(name, ExportKind::Table, index),
        );
    }

    /// Adds a memory export
    pub fn add_memory_export(&mut self, name: String, index: u32) {
        self.exports.insert(
            name.clone(),
            Export::new(name, ExportKind::Memory, index),
        );
    }

    /// Adds a global export
    pub fn add_global_export(&mut self, name: String, index: u32) {
        self.exports.insert(
            name.clone(),
            Export::new(name, ExportKind::Global, index),
        );
    }

    /// Adds an export to the module from a wrt_format::module::Export
    pub fn add_export(&mut self, format_export: wrt_format::module::Export) -> Result<()> {
        let runtime_export_kind = match format_export.kind {
            wrt_format::module::ExportKind::Function => ExportKind::Function,
            wrt_format::module::ExportKind::Table => ExportKind::Table,
            wrt_format::module::ExportKind::Memory => ExportKind::Memory,
            wrt_format::module::ExportKind::Global => ExportKind::Global,
        };
        let runtime_export = Export {
            name: format_export.name,
            kind: runtime_export_kind,
            index: format_export.index,
        };
        self.exports.insert(runtime_export.name.clone(), runtime_export);
        Ok(())
    }

    /// Set the name of the module
    pub fn set_name(&mut self, name: String) -> Result<()> {
        self.name = Some(name);
        Ok(())
    }
    
    /// Set the start function index
    pub fn set_start(&mut self, start: u32) -> Result<()> {
        self.start = Some(start);
        Ok(())
    }
    
    /// Add a function type to the module
    pub fn add_type(&mut self, ty: WrtFuncType) -> Result<()> {
        self.types.push(ty);
        Ok(())
    }
    
    /// Add a function import to the module
    pub fn add_import_func(&mut self, module_name: &str, item_name: &str, type_idx: u32) -> Result<()> {
        let func_type = self.types.get(type_idx as usize)
            .ok_or_else(|| Error::new(ErrorCategory::Validation, codes::TYPE_MISMATCH, "Type index out of bounds for import func"))?
            .clone();
        
        let import_struct = crate::module::Import::new(
            module_name.to_string(),
            item_name.to_string(),
            ExternType::Function(func_type)
        );
        self.imports.entry(module_name.to_string()).or_default().insert(item_name.to_string(), import_struct);
        Ok(())
    }
    
    /// Adds an imported table to the module
    pub fn add_import_table(&mut self, module_name: &str, item_name: &str, table_type: WrtTableType) -> Result<()> {
        let component_table_type = wrt_types::component::TableType::from_core(&table_type);
        let import_struct = crate::module::Import::new(
            module_name.to_string(),
            item_name.to_string(),
            ExternType::Table(component_table_type)
        );
        self.imports.entry(module_name.to_string()).or_default().insert(item_name.to_string(), import_struct);
        Ok(())
    }
    
    /// Adds an imported memory to the module
    pub fn add_import_memory(&mut self, module_name: &str, item_name: &str, memory_type: WrtMemoryType) -> Result<()> {
        let component_memory_type = wrt_types::component::MemoryType::from_core(&memory_type);
        let import_struct = crate::module::Import::new(
            module_name.to_string(),
            item_name.to_string(),
            ExternType::Memory(component_memory_type)
        );
        self.imports.entry(module_name.to_string()).or_default().insert(item_name.to_string(), import_struct);
        Ok(())
    }
    
    /// Adds an imported global to the module
    pub fn add_import_global(&mut self, module_name: &str, item_name: &str, format_global: wrt_format::module::Global) -> Result<()> {
        let component_global_type = wrt_types::component::GlobalType {
            value_type: format_global.global_type.value_type,
            mutable: format_global.global_type.mutable,
        };

        let import = Import::new(
            module_name.to_string(),
            item_name.to_string(),
            ExternType::Global(component_global_type),
        );

        self.imports
            .entry(module_name.to_string())
            .or_default()
            .insert(item_name.to_string(), import);
        Ok(())
    }
    
    /// Add a function to the module
    pub fn add_function_type(&mut self, type_idx: u32) -> Result<()> {
        if type_idx as usize >= self.types.len() {
            return Err(Error::from(kinds::ValidationError(
                format!("Function type index {} out of bounds (max {})", type_idx, self.types.len())
            )));
        }
        
        let function = Function {
            type_idx,
            locals: Vec::new(),
            body: WrtExpr::default(),
        };
        
        self.functions.push(function);
        Ok(())
    }
    
    /// Add a table to the module
    pub fn add_table(&mut self, table_type: WrtTableType) -> Result<()> {
        self.tables.push(Arc::new(Table::new(table_type)?));
        Ok(())
    }
    
    /// Add a memory to the module
    pub fn add_memory(&mut self, memory_type: WrtMemoryType) -> Result<()> {
        self.memories.push(Arc::new(Memory::new(memory_type)?));
        Ok(())
    }
    
    /// Add a global to the module
    pub fn add_global(&mut self, global_type: WrtGlobalType, init: WrtValue) -> Result<()> {
        let global = Global::new(global_type, init);
        self.globals.push(Arc::new(global));
        Ok(())
    }
    
    /// Add a function export to the module
    pub fn add_export_func(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.functions.len() {
            return Err(Error::validation_error(format!("Export function index {} out of bounds", index)));
        }
        
        let export = Export {
            name: name.to_string(),
            kind: ExportKind::Function,
            index,
        };
        
        self.exports.insert(name.to_string(), export);
        Ok(())
    }
    
    /// Add a table export to the module
    pub fn add_export_table(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.tables.len() {
            return Err(Error::validation_error(format!("Export table index {} out of bounds", index)));
        }
        
        let export = Export {
            name: name.to_string(),
            kind: ExportKind::Table,
            index,
        };
        
        self.exports.insert(name.to_string(), export);
        Ok(())
    }
    
    /// Add a memory export to the module
    pub fn add_export_memory(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.memories.len() {
            return Err(Error::validation_error(format!("Export memory index {} out of bounds", index)));
        }
        
        let export = Export {
            name: name.to_string(),
            kind: ExportKind::Memory,
            index,
        };
        
        self.exports.insert(name.to_string(), export);
        Ok(())
    }
    
    /// Add a global export to the module
    pub fn add_export_global(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.globals.len() {
            return Err(Error::validation_error(format!("Export global index {} out of bounds", index)));
        }
        
        let export = Export {
            name: name.to_string(),
            kind: ExportKind::Global,
            index,
        };
        
        self.exports.insert(name.to_string(), export);
        Ok(())
    }
    
    /// Add an element segment to the module
    pub fn add_element(&mut self, element: wrt_format::module::Element) -> Result<()> {
        // Convert format element to runtime element
        let runtime_element = crate::module::Element {
            table_idx: element.table_idx,
            offset: element.offset.clone(), // wrt_format::module::Element.offset is Vec<u8>
            items: element.init.clone(),    // wrt_format::module::Element.init is Vec<u32>, maps to items
        };
        
        self.elements.push(runtime_element);
        Ok(())
    }
    
    /// Set a function body
    pub fn set_function_body(&mut self, func_idx: u32, type_idx: u32, locals: Vec<WrtLocalEntry>, body: WrtExpr) -> Result<()> {
        if func_idx as usize > self.functions.len() { // Allow appending
            return Err(Error::new(ErrorCategory::Runtime, codes::FUNCTION_NOT_FOUND, "Function index out of bounds for set_function_body"));
        }
        let func_entry = Function { type_idx, locals, body };
        if func_idx as usize == self.functions.len() {
            self.functions.push(func_entry);
        } else {
            self.functions[func_idx as usize] = func_entry;
        }
        Ok(())
    }
    
    /// Add a data segment to the module
    pub fn add_data(&mut self, data: wrt_format::module::Data) -> Result<()> {
        // Convert format data to runtime data
        // wrt_runtime::module::Data has fields: memory_idx: u32, offset: Vec<u8>, init: Vec<u8>
        let runtime_data = crate::module::Data {
            memory_idx: data.memory_idx,     // from wrt_format::module::Data
            offset: data.offset.clone(),       // from wrt_format::module::Data (Vec<u8>)
            init: data.init.clone(),         // from wrt_format::module::Data (Vec<u8>), maps to init
        };
        
        self.data.push(runtime_data);
        Ok(())
    }
    
    /// Add a custom section to the module
    pub fn add_custom_section(&mut self, name: &str, data: Vec<u8>) -> Result<()> {
        self.custom_sections.insert(name.to_string(), data);
        Ok(())
    }
    
    /// Set the binary representation of the module
    pub fn set_binary(&mut self, binary: Vec<u8>) -> Result<()> {
        self.binary = Some(binary);
        Ok(())
    }
    
    /// Validate the module
    pub fn validate(&self) -> Result<()> {
        // TODO: Implement comprehensive validation of the runtime module structure.
        // - Check type indices are valid.
        // - Check function indices in start/exports/elements are valid.
        // - Check table/memory/global indices.
        // - Validate instruction sequences in function bodies (optional, decoder should do most of this).
        Ok(())
    }
}

/// Additional exports that are not part of the standard WebAssembly exports
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherExport {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Export index
    pub index: u32,
}

/// Represents an imported item in a WebAssembly module
#[derive(Debug, Clone)]
pub enum ImportedItem {
    /// An imported function
    Function {
        /// The module name
        module: String,
        /// The function name
        name: String,
        /// The function type
        ty: FuncType,
    },
    /// An imported table
    Table {
        /// The module name
        module: String,
        /// The table name
        name: String,
        /// The table type
        ty: TableType,
    },
    /// An imported memory
    Memory {
        /// The module name
        module: String,
        /// The memory name
        name: String,
        /// The memory type
        ty: MemoryType,
    },
    /// An imported global
    Global {
        /// The module name
        module: String,
        /// The global name
        name: String,
        /// The global type
        ty: GlobalType,
    },
}

// Default trait for WrtExpr if not already present (for Function struct)
impl Default for WrtExpr {
    fn default() -> Self {
        WrtExpr { instructions: Vec::new() }
    }
}

// Ensure ExternType is available
use wrt_types::component::ExternType;
use std::sync::Arc; // For Arc<Table/Memory/Global>
use std::collections::HashMap; // For HashMaps in Module struct
use wrt_error::{Error, ErrorCategory, Result, codes}; // For error handling

// Ensure local `crate::module::Import` struct is defined
// Ensure local `crate::module::Export` struct is defined
// Ensure local `crate::global::Global`, `crate::table::Table`, `crate::memory::Memory` are defined and their `new` methods are compatible.

// New method for ModuleBuilder
pub fn add_import_runtime_global(&mut self, module_name: &str, item_name: &str, global_type: WrtImportGlobalType) -> Result<()> {
    let component_global_type = wrt_types::component::GlobalType {
        value_type: global_type.value_type,
        mutable: global_type.mutable,
    };
    let import_struct = crate::module::Import::new(
        module_name.to_string(),
        item_name.to_string(),
        ExternType::Global(component_global_type)
    );
    self.imports.entry(module_name.to_string()).or_default().insert(item_name.to_string(), import_struct);
    Ok(())
}

// New method for ModuleBuilder
pub fn add_runtime_export(&mut self, export: wrt_types::types::Export) -> Result<()> {
    let kind = match export.desc {
        WrtExportDesc::Func(_) => ExportKind::Function,
        WrtExportDesc::Table(_) => ExportKind::Table,
        WrtExportDesc::Memory(_) => ExportKind::Memory,
        WrtExportDesc::Global(_) => ExportKind::Global,
        WrtExportDesc::Tag(_) => return Err(Error::new(ErrorCategory::NotSupported, codes::UNSUPPORTED_FEATURE, "Tag exports not supported")),
    };
    let runtime_export = crate::module::Export::new(export.name.clone(), kind, export.desc.index());
    self.exports.insert(export.name, runtime_export);
    Ok(())
}

// New method for ModuleBuilder
pub fn add_runtime_element(&mut self, element_segment: WrtElementSegment) -> Result<()> {
    // TODO: Resolve element_segment.items expressions if they are not direct indices.
    // This is a placeholder and assumes items can be derived or handled during instantiation.
    let items_resolved = match &element_segment.items {
        wrt_types::types::ElementItems::Functions(indices) => indices.iter().filter_map(|&opt_idx| opt_idx).collect(),
        wrt_types::types::ElementItems::Expressions(_exprs) => {
            // This requires evaluation context (e.g., globals) which is not available here.
            // Instantiation phase should handle this. For now, maybe store expressions or error.
             return Err(Error::new(ErrorCategory::NotSupported, codes::NOT_IMPLEMENTED, "Element items with expressions require instantiation-time evaluation"));
        }
    };

    self.elements.push(crate::module::Element {
        mode: element_segment.mode,
        table_idx: element_segment.table_idx,
        offset_expr: element_segment.offset_expr,
        element_type: element_segment.element_type,
        items: items_resolved,
    });
    Ok(())
}

// New method for ModuleBuilder
pub fn add_runtime_data(&mut self, data_segment: WrtDataSegment) -> Result<()> {
    self.data.push(crate::module::Data {
        mode: data_segment.mode,
        memory_idx: data_segment.memory_idx,
        offset_expr: data_segment.offset_expr,
        init: data_segment.data,
    });
    Ok(())
}

// Signature updated for ModuleBuilder
pub fn add_custom_section(&mut self, section: WrtCustomSection) -> Result<()> {
    self.custom_sections.insert(section.name, section.data);
    Ok(())
}

pub fn set_binary(&mut self, binary: Vec<u8>) -> Result<()> {
    self.binary = Some(binary);
    Ok(())
}

pub fn validate(&self) -> Result<()> {
    // TODO: Implement comprehensive validation of the runtime module structure.
    // - Check type indices are valid.
    // - Check function indices in start/exports/elements are valid.
    // - Check table/memory/global indices.
    // - Validate instruction sequences in function bodies (optional, decoder should do most of this).
    Ok(())
} 