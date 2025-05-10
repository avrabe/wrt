//! WebAssembly module format.
//!
//! This module provides types and utilities for working with WebAssembly modules.

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, vec::Vec};

use crate::section::CustomSection;
use crate::types::CoreWasmVersion;
use crate::types::FormatGlobalType;
use crate::types::Limits;
use wrt_error::{codes, Error, ErrorCategory, Result};
use wrt_types::{types::GlobalType, RefType, ValueType};

#[cfg(not(feature = "std"))]
use alloc::string::ToString;

use crate::validation::Validatable;

/// WebAssembly function definition
#[derive(Debug, Clone)]
pub struct Function {
    /// Type index
    pub type_idx: u32,
    /// Local variables
    pub locals: Vec<ValueType>,
    /// Function body (instructions)
    pub code: Vec<u8>,
}

/// WebAssembly memory definition
///
/// A memory instance as defined in the WebAssembly Core Specification.
/// The memory section consists of a vector of memory definitions, each
/// defining a memory with limits, and optional shared flag for threading.
///
/// WebAssembly 1.0 allows at most one memory per module.
/// Memory64 extension allows memories with 64-bit addressing.
#[derive(Debug, Clone)]
pub struct Memory {
    /// Memory limits (minimum and optional maximum size in pages)
    /// Each page is 64KiB (65536 bytes)
    pub limits: Limits,
    /// Whether this memory is shared between threads
    /// Shared memory must have a maximum size specified
    pub shared: bool,
}

/// WebAssembly table definition
#[derive(Debug, Clone)]
pub struct Table {
    /// Element type
    pub element_type: ValueType,
    /// Table limits
    pub limits: Limits,
}

/// WebAssembly global definition
#[derive(Debug, Clone)]
pub struct Global {
    /// Global type
    pub global_type: FormatGlobalType,
    /// Initialization expression
    pub init: Vec<u8>,
}

/// WebAssembly data segment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataMode {
    /// Active data segment (explicitly placed into a memory)
    Active,
    /// Passive data segment (used with memory.init)
    Passive,
}

/// WebAssembly data segment
#[derive(Debug, Clone)]
pub struct Data {
    /// Data mode (active or passive)
    pub mode: DataMode,
    /// Memory index (for active data segments)
    pub memory_idx: u32,
    /// Offset expression (for active data segments)
    pub offset: Vec<u8>,
    /// Initial data
    pub init: Vec<u8>,
}

/// Represents the initialization items for an element segment.
#[derive(Debug, Clone)]
pub enum ElementInit {
    /// A vector of function indices (for funcref element type when expressions are not used).
    FuncIndices(Vec<u32>),
    /// A vector of initialization expressions (for externref, or funcref with expressions).
    /// Each expression is a raw byte vector, representing a const expr.
    Expressions(Vec<Vec<u8>>),
}

/// Mode for an element segment, determining how it's initialized.
#[derive(Debug, Clone)]
pub enum ElementMode {
    /// Active segment: associated with a table and an offset.
    Active {
        /// Index of the table to initialize.
        table_index: u32,
        /// Offset expression (raw bytes of a const expr).
        offset_expr: Vec<u8>,
    },
    /// Passive segment: elements are not actively placed in a table at instantiation.
    Passive,
    /// Declared segment: elements are declared but not available at runtime
    /// until explicitly instantiated. Useful for some linking scenarios.
    Declared,
}

/// WebAssembly element segment (Wasm 2.0 compatible)
#[derive(Debug, Clone)]
pub struct Element {
    /// The type of elements in this segment (funcref or externref).
    pub element_type: RefType,
    /// Initialization items for the segment.
    pub init: ElementInit,
    /// The mode of the element segment.
    pub mode: ElementMode,
}

/// WebAssembly export
#[derive(Debug, Clone)]
pub struct Export {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Export index
    pub index: u32,
}

/// WebAssembly export kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// Function export
    Function,
    /// Table export
    Table,
    /// Memory export
    Memory,
    /// Global export
    Global,
    /// Tag export
    Tag,
}

/// WebAssembly import
#[derive(Debug, Clone)]
pub struct Import {
    /// Module name
    pub module: String,
    /// Import name
    pub name: String,
    /// Import description
    pub desc: ImportDesc,
}

/// WebAssembly import description
#[derive(Debug, Clone)]
pub enum ImportDesc {
    /// Function import
    Function(u32),
    /// Table import
    Table(Table),
    /// Memory import
    Memory(Memory),
    /// Global import
    Global(FormatGlobalType),
    /// Tag import
    Tag(u32),
}

/// Hypothetical Finding F5: Represents an entry in the TypeInformation section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInformationEntry {
    pub type_index: u32, // Assuming TypeIdx is u32
    pub name: String,
}

/// Hypothetical Finding F5: Represents the custom TypeInformation section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeInformationSection {
    pub entries: Vec<TypeInformationEntry>,
}

/// Represents a WebAssembly module.
#[derive(Debug, Clone, Default)]
pub struct Module {
    /// Function types
    pub types: Vec<ValueType>,
    /// Function definitions
    pub functions: Vec<Function>,
    /// Table definitions
    pub tables: Vec<Table>,
    /// Memory definitions
    pub memories: Vec<Memory>,
    /// Global definitions
    pub globals: Vec<Global>,
    /// Element segments
    pub elements: Vec<Element>,
    /// Data segments
    pub data: Vec<Data>,
    /// Exports
    pub exports: Vec<Export>,
    /// Imports
    pub imports: Vec<Import>,
    /// Start function
    pub start: Option<u32>,
    /// Custom sections
    pub custom_sections: Vec<CustomSection>,
    /// Original binary (if available)
    pub binary: Option<Vec<u8>>,
    pub core_version: CoreWasmVersion, // Added for Wasm 3.0 support
    pub type_info_section: Option<TypeInformationSection>,
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

impl Module {
    /// Create a new empty module
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            elements: Vec::new(),
            data: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            start: None,
            custom_sections: Vec::new(),
            binary: None,
            core_version: CoreWasmVersion::default(),
            type_info_section: None,
        }
    }

    /// Convert a WebAssembly binary to a Module.
    ///
    /// This is a convenience method that wraps Binary::from_bytes + Module::from_binary
    pub fn from_bytes(_wasm_bytes: &[u8]) -> Result<Self> {
        Err(Error::new(
            ErrorCategory::Validation,
            codes::PARSE_ERROR,
            "Module::from_bytes not yet implemented".to_string(),
        ))
    }

    /// Convert a Module to a WebAssembly binary.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Err(Error::new(
            ErrorCategory::Validation,
            codes::PARSE_ERROR,
            "Module::to_bytes not yet implemented".to_string(),
        ))
    }

    /// Find a custom section by name
    pub fn find_custom_section(&self, name: &str) -> Option<&CustomSection> {
        self.custom_sections.iter().find(|section| section.name == name)
    }

    /// Add a custom section
    pub fn add_custom_section(&mut self, section: CustomSection) {
        self.custom_sections.push(section);
    }

    /// Check if this module contains state sections
    pub fn has_state_sections(&self) -> bool {
        crate::state::has_state_sections(&self.custom_sections)
    }
}

impl Validatable for Module {
    fn validate(&self) -> Result<()> {
        // Basic validation checks

        // Check for reasonable number of types
        if self.types.len() > 10000 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Module has too many types",
            ));
        }

        // Check for reasonable number of functions
        if self.functions.len() > 10000 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Module has too many functions",
            ));
        }

        // Check for empty exports
        for export in &self.exports {
            if export.name.is_empty() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    "Export name cannot be empty",
                ));
            }
        }

        // Check for empty imports
        for import in &self.imports {
            if import.module.is_empty() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    "Import module name cannot be empty",
                ));
            }

            if import.name.is_empty() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    "Import name cannot be empty",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    // ... existing test code ...
}
