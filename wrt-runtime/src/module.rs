// Module implementation for runtime execution
//
// This module provides the core runtime implementation of WebAssembly modules
// used by the runtime execution engine.

// Use alloc when available through lib.rs
#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::format;

use wrt_foundation::{
    types::{
        CustomSection as WrtCustomSection, DataMode as WrtDataMode,
        ElementMode as WrtElementMode,
        ExportDesc as WrtExportDesc, FuncType as WrtFuncType,
        GlobalType as WrtGlobalType, ImportDesc as WrtImportDesc,
        Limits as WrtLimits, LocalEntry as WrtLocalEntry, MemoryType as WrtMemoryType,
        RefType as WrtRefType, TableType as WrtTableType, ValueType as WrtValueType,
        ValueType, // Also import without alias
    },
    values::{Value as WrtValue, Value}, // Also import without alias
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

// Re-export for module_builder
pub use wrt_foundation::types::LocalEntry;
use crate::prelude::CoreMemoryType;
use wrt_format::{
    DataSegment as WrtDataSegment,
    ElementSegment as WrtElementSegment,
    module::{ImportDesc as FormatImportDesc, ExportKind as FormatExportKind},
};

// Type alias for the runtime ImportDesc
type RuntimeImportDesc = WrtImportDesc<RuntimeProvider>;

use crate::{global::Global, memory::Memory, table::Table};
use crate::prelude::{ToString, RuntimeString};
// Use clean types for collections instead of provider-embedded ones
#[cfg(feature = "std")]
use std::{vec::Vec, collections::HashMap, string::String, sync::Arc};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{vec::Vec, string::String, sync::Arc};
// HashMap is not needed with clean architecture using BoundedMap
use wrt_foundation::bounded_collections::BoundedMap;
use wrt_foundation::traits::{BoundedCapacity, Checksummable, ToBytes, FromBytes};

// Unified memory allocation using safe_managed_alloc! - NO hardcoded providers
// All memory allocation goes through safe_managed_alloc!(size, crate_id) as per CLAUDE.md

// Use the unified RuntimeProvider from bounded_runtime_infra
use crate::bounded_runtime_infra::{RuntimeProvider, create_runtime_provider};
type ImportMap = BoundedMap<wrt_foundation::bounded::BoundedString<256, RuntimeProvider>, Import, 32, RuntimeProvider>;
type ModuleImports = BoundedMap<wrt_foundation::bounded::BoundedString<256, RuntimeProvider>, ImportMap, 32, RuntimeProvider>;
type CustomSections = BoundedMap<wrt_foundation::bounded::BoundedString<256, RuntimeProvider>, wrt_foundation::bounded::BoundedVec<u8, 4096, RuntimeProvider>, 16, RuntimeProvider>;
type ExportMap = BoundedMap<wrt_foundation::bounded::BoundedString<256, RuntimeProvider>, Export, 64, RuntimeProvider>;

// Additional type aliases for struct fields to use unified RuntimeProvider
type BoundedExportName = wrt_foundation::bounded::BoundedString<128, RuntimeProvider>;
type BoundedImportName = wrt_foundation::bounded::BoundedString<128, RuntimeProvider>;
type BoundedModuleName = wrt_foundation::bounded::BoundedString<128, RuntimeProvider>;
type BoundedLocalsVec = wrt_foundation::bounded::BoundedVec<WrtLocalEntry, 64, RuntimeProvider>;
type BoundedElementItems = wrt_foundation::bounded::BoundedVec<u32, 1024, RuntimeProvider>;
type BoundedDataInit = wrt_foundation::bounded::BoundedVec<u8, 4096, RuntimeProvider>;
type BoundedModuleTypes = wrt_foundation::bounded::BoundedVec<WrtFuncType<RuntimeProvider>, 256, RuntimeProvider>;
type BoundedFunctionVec = wrt_foundation::bounded::BoundedVec<Function, 1024, RuntimeProvider>;
type BoundedTableVec = wrt_foundation::bounded::BoundedVec<TableWrapper, 64, RuntimeProvider>;
type BoundedMemoryVec = wrt_foundation::bounded::BoundedVec<MemoryWrapper, 64, RuntimeProvider>;
type BoundedGlobalVec = wrt_foundation::bounded::BoundedVec<GlobalWrapper, 256, RuntimeProvider>;
type BoundedElementVec = wrt_foundation::bounded::BoundedVec<Element, 256, RuntimeProvider>;
type BoundedDataVec = wrt_foundation::bounded::BoundedVec<Data, 256, RuntimeProvider>;
type BoundedBinary = wrt_foundation::bounded::BoundedVec<u8, 65536, RuntimeProvider>;

/// Convert MemoryType to CoreMemoryType
fn to_core_memory_type(memory_type: WrtMemoryType) -> CoreMemoryType {
    CoreMemoryType {
        limits: memory_type.limits,
        shared: memory_type.shared,
    }
}

/// A WebAssembly expression (sequence of instructions)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WrtExpr {
    /// Parsed instructions (simplified representation)
    pub instructions: wrt_foundation::bounded::BoundedVec<wrt_foundation::types::Instruction<RuntimeProvider>, 1024, RuntimeProvider>, // Parsed instructions
}

impl WrtExpr {
    /// Returns the length of the instruction sequence
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Returns true if the expression is empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

/// Represents a WebAssembly export kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportKind {
    /// Function export
    #[default]
    Function,
    /// Table export
    Table,
    /// Memory export
    Memory,
    /// Global export
    Global,
}

/// Represents an export in a WebAssembly module
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Export {
    /// Export name
    pub name: BoundedExportName,
    /// Export kind
    pub kind: ExportKind,
    /// Export index
    pub index: u32,
}

impl Export {
    /// Creates a new export
    pub fn new(name: String, kind: ExportKind, index: u32) -> Result<Self> {
        let provider = create_runtime_provider()?;
        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(&name, provider)?;
        Ok(Self { name: bounded_name, kind, index })
    }
}

impl wrt_foundation::traits::Checksummable for Export {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        self.name.update_checksum(checksum;
        checksum.update_slice(&(self.kind as u8).to_le_bytes);
        checksum.update_slice(&self.index.to_le_bytes);
    }
}

impl wrt_foundation::traits::ToBytes for Export {
    fn serialized_size(&self) -> usize {
        self.name.serialized_size() + 1 + 4 // name + kind (1 byte) + index (4 bytes)
    }

    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'_>,
        provider: &P,
    ) -> wrt_foundation::Result<()> {
        self.name.to_bytes_with_provider(writer, provider)?;
        writer.write_all(&(self.kind as u8).to_le_bytes())?;
        writer.write_all(&self.index.to_le_bytes())
    }
}

impl wrt_foundation::traits::FromBytes for Export {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'_>,
        provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let name = wrt_foundation::bounded::BoundedString::from_bytes_with_provider(reader, provider)?;
        
        let mut kind_bytes = [0u8; 1];
        reader.read_exact(&mut kind_bytes)?;
        let kind = match kind_bytes[0] {
            0 => ExportKind::Function,
            1 => ExportKind::Table,
            2 => ExportKind::Memory,
            3 => ExportKind::Global,
            _ => ExportKind::Function, // Default fallback
        };
        
        let mut index_bytes = [0u8; 4];
        reader.read_exact(&mut index_bytes)?;
        let index = u32::from_le_bytes(index_bytes;
        
        Ok(Self { name, kind, index })
    }
}

/// Represents an import in a WebAssembly module
#[derive(Debug, Clone)]
pub struct Import {
    /// Module name
    pub module: BoundedImportName,
    /// Import name
    pub name: BoundedImportName,
    /// Import type
    pub ty: ExternType<RuntimeProvider>,
    /// Import description
    pub desc: RuntimeImportDesc,
}

impl Import {
    /// Creates a new import
    pub fn new(module: String, name: String, ty: ExternType<RuntimeProvider>, desc: RuntimeImportDesc) -> Result<Self> {
        let provider1 = create_runtime_provider()?;
        let provider2 = create_runtime_provider()?;
        let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(&module, provider1)?;
        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(&name, provider2)?;
        Ok(Self { module: bounded_module, name: bounded_name, ty, desc })
    }
}

impl Default for Import {
    fn default() -> Self {
        let provider1 = create_runtime_provider().unwrap();
        let provider2 = create_runtime_provider().unwrap();
        Self {
            module: wrt_foundation::bounded::BoundedString::from_str_truncate("", provider1).unwrap(),
            name: wrt_foundation::bounded::BoundedString::from_str_truncate("", provider2).unwrap(),
            ty: ExternType::default(),
            desc: RuntimeImportDesc::Function(0),
        }
    }
}

impl PartialEq for Import {
    fn eq(&self, other: &Self) -> bool {
        self.module == other.module && self.name == other.name
    }
}

impl Eq for Import {}

impl wrt_foundation::traits::Checksummable for Import {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        self.module.update_checksum(checksum;
        self.name.update_checksum(checksum;
    }
}

impl wrt_foundation::traits::ToBytes for Import {
    fn serialized_size(&self) -> usize {
        self.module.serialized_size() + self.name.serialized_size() + 4 // simplified
    }

    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'_>,
        provider: &P,
    ) -> wrt_foundation::Result<()> {
        self.module.to_bytes_with_provider(writer, provider)?;
        self.name.to_bytes_with_provider(writer, provider)
    }
}

impl wrt_foundation::traits::FromBytes for Import {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'_>,
        provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let module = wrt_foundation::bounded::BoundedString::from_bytes_with_provider(reader, provider)?;
        let name = wrt_foundation::bounded::BoundedString::from_bytes_with_provider(reader, provider)?;
        Ok(Self {
            module,
            name,
            ty: ExternType::default(), // simplified
            desc: RuntimeImportDesc::Function(0),
        })
    }
}

/// Represents a WebAssembly function in the runtime
#[derive(Debug, Clone)]
pub struct Function {
    /// The type index of the function (referring to Module.types)
    pub type_idx: u32,
    /// The parsed local variable declarations
    pub locals: BoundedLocalsVec,
    /// The parsed instructions that make up the function body
    pub body: WrtExpr,
}

impl Default for Function {
    fn default() -> Self {
        let provider = create_runtime_provider().unwrap();
        Self {
            type_idx: 0,
            locals: BoundedLocalsVec::new(provider).unwrap(),
            body: WrtExpr::default(),
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.type_idx == other.type_idx
    }
}

impl Eq for Function {}

impl wrt_foundation::traits::Checksummable for Function {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&self.type_idx.to_le_bytes);
    }
}

impl wrt_foundation::traits::ToBytes for Function {
    fn serialized_size(&self) -> usize {
        8 // simplified
    }

    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        writer.write_all(&self.type_idx.to_le_bytes())
    }
}

impl wrt_foundation::traits::FromBytes for Function {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        let type_idx = u32::from_le_bytes(bytes;
        let provider = create_runtime_provider().map_err(|_| wrt_error::Error::memory_error("Failed to allocate provider for function locals"))?;
        Ok(Self {
            type_idx,
            locals: BoundedLocalsVec::new(provider).unwrap(),
            body: WrtExpr::default(),
        })
    }
}

/// Represents the value of an export
#[derive(Debug, Clone)]
pub enum ExportItem {
    /// A function with the specified index
    Function(u32),
    /// A table with the specified index
    Table(TableWrapper),
    /// A memory with the specified index
    Memory(MemoryWrapper),
    /// A global with the specified index
    Global(GlobalWrapper),
}

/// Represents an element segment for tables in the runtime
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Element {
    /// Element segment mode (active, passive, or declarative)
    pub mode: WrtElementMode,
    /// Index of the target table (for active elements)
    pub table_idx: Option<u32>,
    /// Offset expression for element placement
    pub offset_expr: Option<WrtExpr>,
    /// Type of elements in this segment
    pub element_type: WrtRefType,
    /// Element items (function indices or expressions)
    pub items: BoundedElementItems,
}

impl wrt_foundation::traits::Checksummable for Element {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        let mode_byte = match &self.mode {
            WrtElementMode::Active { .. } => 0u8,
            WrtElementMode::Passive => 1u8,
            WrtElementMode::Declarative => 2u8,
        };
        checksum.update_slice(&mode_byte.to_le_bytes);
        if let Some(table_idx) = self.table_idx {
            checksum.update_slice(&table_idx.to_le_bytes);
        }
    }
}

impl wrt_foundation::traits::ToBytes for Element {
    fn serialized_size(&self) -> usize {
        16 // simplified
    }

    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        let mode_byte = match &self.mode {
            WrtElementMode::Active { .. } => 0u8,
            WrtElementMode::Passive => 1u8,
            WrtElementMode::Declarative => 2u8,
        };
        writer.write_all(&mode_byte.to_le_bytes())?;
        writer.write_all(&self.table_idx.unwrap_or(0).to_le_bytes())
    }
}

impl wrt_foundation::traits::FromBytes for Element {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 1];
        reader.read_exact(&mut bytes)?;
        let mode = match bytes[0] {
            0 => WrtElementMode::Active { table_index: 0, offset: 0 },
            1 => WrtElementMode::Passive,
            _ => WrtElementMode::Declarative,
        };
        
        let mut idx_bytes = [0u8; 4];
        reader.read_exact(&mut idx_bytes)?;
        let table_idx = Some(u32::from_le_bytes(idx_bytes;
        
        Ok(Self {
            mode,
            table_idx,
            offset_expr: None,
            element_type: WrtRefType::Funcref,
            items: BoundedElementItems::new(create_runtime_provider().unwrap()).unwrap(),
        })
    }
}

/// Represents a data segment for memories in the runtime
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Data {
    /// Data segment mode (active or passive)
    pub mode: WrtDataMode,
    /// Index of the target memory (for active data)
    pub memory_idx: Option<u32>,
    /// Offset expression for data placement
    pub offset_expr: Option<WrtExpr>,
    /// Initialization data bytes
    pub init: BoundedDataInit,
}

impl wrt_foundation::traits::Checksummable for Data {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        let mode_byte = match &self.mode {
            WrtDataMode::Active { .. } => 0u8,
            WrtDataMode::Passive => 1u8,
        };
        checksum.update_slice(&mode_byte.to_le_bytes);
        if let Some(memory_idx) = self.memory_idx {
            checksum.update_slice(&memory_idx.to_le_bytes);
        }
        checksum.update_slice(&(self.init.len() as u32).to_le_bytes);
    }
}

impl wrt_foundation::traits::ToBytes for Data {
    fn serialized_size(&self) -> usize {
        16 + self.init.len() // simplified
    }

    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        let mode_byte = match &self.mode {
            WrtDataMode::Active { .. } => 0u8,
            WrtDataMode::Passive => 1u8,
        };
        writer.write_all(&mode_byte.to_le_bytes())?;
        writer.write_all(&self.memory_idx.unwrap_or(0).to_le_bytes())?;
        writer.write_all(&(self.init.len() as u32).to_le_bytes())
    }
}

impl wrt_foundation::traits::FromBytes for Data {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 1];
        reader.read_exact(&mut bytes)?;
        let mode = match bytes[0] {
            0 => WrtDataMode::Active { memory_index: 0, offset: 0 },
            _ => WrtDataMode::Passive,
        };
        
        let mut idx_bytes = [0u8; 4];
        reader.read_exact(&mut idx_bytes)?;
        let memory_idx = Some(u32::from_le_bytes(idx_bytes;
        
        reader.read_exact(&mut idx_bytes)?;
        let _len = u32::from_le_bytes(idx_bytes;
        
        Ok(Self {
            mode,
            memory_idx,
            offset_expr: None,
            init: BoundedDataInit::new(create_runtime_provider().map_err(|_| wrt_error::Error::memory_error("Failed to allocate provider for data init"))?)?,
        })
    }
}

impl Data {
    /// Returns a reference to the data in this segment
    pub fn data(&self) -> Result<&[u8]> {
        self.init.as_slice()
    }
}

/// Represents a WebAssembly module in the runtime
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Module {
    /// Module types (function signatures)
    pub types: BoundedModuleTypes,
    /// Imported functions, tables, memories, and globals
    pub imports: ModuleImports,
    /// Function definitions
    pub functions: BoundedFunctionVec,
    /// Table instances
    pub tables: BoundedTableVec,
    /// Memory instances
    pub memories: BoundedMemoryVec,
    /// Global variable instances
    pub globals: BoundedGlobalVec,
    /// Element segments for tables
    pub elements: BoundedElementVec,
    /// Data segments for memories
    pub data: BoundedDataVec,
    /// Start function index
    pub start: Option<u32>,
    /// Custom sections
    pub custom_sections: CustomSections,
    /// Exports (functions, tables, memories, and globals)
    pub exports: ExportMap,
    /// Optional name for the module
    pub name: Option<BoundedModuleName>,
    /// Original binary (if available)
    pub binary: Option<BoundedBinary>,
    /// Execution validation flag
    pub validated: bool,
}

impl Module {
    /// Creates a truly empty module without any allocations
    /// This is used to avoid circular dependencies during engine initialization
    pub fn empty() -> Self {
        // Use the derived Default implementation which creates empty collections
        Self::default()
    }
    
    /// Creates a new empty module
    pub fn new() -> Result<Self> {
        let provider = create_runtime_provider()?;
        let runtime_provider1 = create_runtime_provider()?;
        let runtime_provider2 = create_runtime_provider()?;
        let runtime_provider3 = create_runtime_provider()?;
        Ok(Self {
            types: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            imports: BoundedMap::new(runtime_provider1)?,
            functions: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            tables: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            memories: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            globals: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            elements: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            data: wrt_foundation::bounded::BoundedVec::new(provider.clone())?,
            start: None,
            custom_sections: BoundedMap::new(runtime_provider2)?,
            exports: BoundedMap::new(runtime_provider3)?,
            name: None,
            binary: None,
            validated: false,
        })
    }


    /// Creates a runtime Module from a `wrt_format::module::Module`.
    /// This is the primary constructor after decoding.
    #[cfg(feature = "std")]
    pub fn from_wrt_module(wrt_module: &wrt_format::module::Module) -> Result<Self> {
        // Ensure memory system is initialized before creating providers
        wrt_foundation::memory_init::MemoryInitializer::ensure_initialized()?;
        
        // Use empty() instead of new() to avoid memory allocation during initialization
        // This prevents stack overflow when the memory system isn't fully initialized
        let mut runtime_module = Self::empty);
        
        // Map start function if present
        runtime_module.start = wrt_module.start;
        
        // Create a single shared provider for the entire module to avoid stack overflow
        // from creating multiple providers in tight loops
        let shared_provider = create_runtime_provider()?;
        
        // Convert types
        for func_type in &wrt_module.types {
            let mut params = wrt_foundation::bounded::BoundedVec::new(shared_provider.clone())?;
            let mut results = wrt_foundation::bounded::BoundedVec::new(shared_provider.clone())?;
            
            for param in &func_type.params {
                params.push(*param)?;
            }
            for result in &func_type.results {
                results.push(*result)?;
            }
            
            let wrt_func_type = WrtFuncType {
                params,
                results,
            };
            runtime_module.types.push(wrt_func_type)?;
        }
        
        // Convert functions
        for func in &wrt_module.functions {
            // Convert locals using the locals conversion function
            let locals = crate::type_conversion::convert_locals_to_bounded(&func.locals)?;
            
            // Parse the function body bytecode into instructions
            let instructions = crate::instruction_parser::parse_instructions(&func.code)?;
            let body = WrtExpr { instructions };
            
            let runtime_func = Function {
                type_idx: func.type_idx,
                locals,
                body,
            };
            runtime_module.functions.push(runtime_func)?;
        }
        
        // Convert exports
        for export in &wrt_module.exports {
            // Create the export name with correct provider size (8192)
            let name = wrt_foundation::bounded::BoundedString::from_str_truncate(&export.name, shared_provider.clone())?;
            
            // Create key with correct type for ExportMap (BoundedString<256, RuntimeProvider>)
            let map_key = wrt_foundation::bounded::BoundedString::from_str_truncate(&export.name, shared_provider.clone())?;
            
            let kind = match export.kind {
                FormatExportKind::Function => ExportKind::Function,
                FormatExportKind::Table => ExportKind::Table,
                FormatExportKind::Memory => ExportKind::Memory,
                FormatExportKind::Global => ExportKind::Global,
                FormatExportKind::Tag => {
                    // Skip Tag exports for now as they're not supported in the runtime
                    continue;
                }
            };
            
            let runtime_export = Export {
                name,
                kind,
                index: export.index,
            };
            
            runtime_module.exports.insert(map_key, runtime_export)?;
        }
        
        Ok(runtime_module)
    }
    
    /// Creates a runtime Module from a `wrt_format::module::Module` in no_std environments.
    /// This handles the generic provider type from the decoder.
    #[cfg(not(feature = "std"))]
    pub fn from_wrt_module_nostd(
        wrt_module: &wrt_format::module::Module
    ) -> Result<Self> {
        // Ensure memory system is initialized before creating providers
        wrt_foundation::memory_init::MemoryInitializer::ensure_initialized()?;
        
        // Use empty() instead of new() to avoid memory allocation during initialization
        let mut runtime_module = Self::empty);
        
        // Map start function if present
        runtime_module.start = wrt_module.start;
        
        // Convert types
        for func_type in &wrt_module.types {
            let provider = create_runtime_provider()?;
            let mut params = wrt_foundation::bounded::BoundedVec::new(provider.clone())?;
            let mut results = wrt_foundation::bounded::BoundedVec::new(provider.clone())?;
            
            for param in &func_type.params {
                params.push(*param)?;
            }
            for result in &func_type.results {
                results.push(*result)?;
            }
            
            let wrt_func_type = WrtFuncType {
                params,
                results,
            };
            runtime_module.types.push(wrt_func_type)?;
        }
        
        // Convert imports
        for import in &wrt_module.imports {
            let desc = match &import.desc {
                FormatImportDesc::Function(type_idx) => {
                    RuntimeImportDesc::Function(*type_idx)
                }
                FormatImportDesc::Table(tt) => {
                    RuntimeImportDesc::Table(tt.clone())
                }
                FormatImportDesc::Memory(mt) => {
                    RuntimeImportDesc::Memory(*mt)
                }
                FormatImportDesc::Global(gt) => {
                    RuntimeImportDesc::Global(wrt_foundation::types::GlobalType {
                        value_type: gt.value_type,
                        mutable: gt.mutable,
                    })
                }
                FormatImportDesc::Tag(tag_idx) => {
                    // Handle Tag import - convert to appropriate runtime representation
                    return Err(Error::parse_error("Tag imports not yet supported";
                }
            };
            
            // Convert string to BoundedString - need different sizes for different use cases
            let provider1 = create_runtime_provider()?;
            let provider2 = create_runtime_provider()?;
            let provider3 = create_runtime_provider()?;
            let provider4 = create_runtime_provider()?;
            
            // 128-char strings for Import struct fields
            let bounded_module_128 = wrt_foundation::bounded::BoundedString::from_str_truncate(&import.module, provider1)?;
            let bounded_name_128 = wrt_foundation::bounded::BoundedString::from_str_truncate(&import.name, provider2)?;
            
            // 256-char strings for map keys
            let bounded_module_256 = wrt_foundation::bounded::BoundedString::from_str_truncate(&import.module, provider3)?;
            let bounded_name_256 = wrt_foundation::bounded::BoundedString::from_str_truncate(&import.name, provider4)?;
            
            let import_entry = Import {
                module: bounded_module_128,
                name: bounded_name_128,
                ty: ExternType::default(),
                desc,
            };
            
            // Get or create inner map for this module
            let mut inner_map = match runtime_module.imports.get(&bounded_module_256)? {
                Some(existing) => existing,
                None => ImportMap::new(create_runtime_provider()?)?
            };
            
            // Insert the import into the inner map
            inner_map.insert(bounded_name_256, import_entry)?;
            
            // Update the outer map
            runtime_module.imports.insert(bounded_module_256, inner_map)?;
        }
        
        // Convert functions
        for function in &wrt_module.functions {
            runtime_module.functions.push(Function {
                type_idx: function.type_idx,
                locals: crate::type_conversion::convert_locals_to_bounded(&function.locals)?,
                // Body conversion would happen here
                body: WrtExpr::default(),
            })?;
        }
        
        // Convert tables
        for table in &wrt_module.tables {
            runtime_module.tables.push(TableWrapper::new(Table::new(table.clone())?))?;
        }
        
        // Convert memories
        for memory in &wrt_module.memories {
            runtime_module.memories.push(MemoryWrapper::new(Memory::new(to_core_memory_type(*memory))?))?;
        }
        
        // Convert globals
        for global in &wrt_module.globals {
            // For now, use a default initial value based on type
            let initial_value = match global.global_type.value_type {
                wrt_foundation::types::ValueType::I32 => wrt_foundation::values::Value::I32(0),
                wrt_foundation::types::ValueType::I64 => wrt_foundation::values::Value::I64(0),
                wrt_foundation::types::ValueType::F32 => wrt_foundation::values::Value::F32(wrt_foundation::values::FloatBits32::from_bits(0)),
                wrt_foundation::types::ValueType::F64 => wrt_foundation::values::Value::F64(wrt_foundation::values::FloatBits64::from_bits(0)),
                _ => return Err(Error::not_supported_unsupported_operation("Unsupported global type")),
            };
            
            let new_global = Global::new(global.global_type.value_type, global.global_type.mutable, initial_value)?;
            runtime_module.globals.push(GlobalWrapper(Arc::new(new_global)))?;
        }
        
        // Convert exports
        for export in &wrt_module.exports {
            let kind = match export.kind {
                FormatExportKind::Function => ExportKind::Function,
                FormatExportKind::Table => ExportKind::Table,
                FormatExportKind::Memory => ExportKind::Memory,
                FormatExportKind::Global => ExportKind::Global,
                FormatExportKind::Tag => {
                    // Skip Tag exports for now as they're not supported in the runtime
                    continue;
                }
            };
            
            // Create the export name with runtime provider
            let provider = create_runtime_provider()?;
            let export_name = wrt_foundation::bounded::BoundedString::from_str_truncate(&export.name, provider)?;
            
            let export_obj = Export {
                name: export_name.clone(),
                kind,
                index: export.index,
            };
            
            // Insert into the exports map using the export name as key
            let provider2 = create_runtime_provider()?;
            let map_key = wrt_foundation::bounded::BoundedString::from_str_truncate(&export.name, provider2)?;
            runtime_module.exports.insert(map_key, export_obj)?;
        }
        
        Ok(runtime_module)
    }
    
    /// Creates a runtime Module from a `wrt_foundation::types::Module`.
    /// This is the primary constructor after decoding for no_std.
    #[cfg(not(feature = "std"))]
    pub fn from_wrt_foundation_module(wrt_module: &wrt_foundation::types::Module<RuntimeProvider>) -> Result<Self> {
        let mut runtime_module = Self::new()?;

        // TODO: wrt_module doesn't have a name field currently
        // if let Some(name) = &wrt_module.name {
        //     runtime_module.name = Some(name.clone();
        // }
        // Map start function if present
        runtime_module.start = wrt_module.start_func;

        for type_def in &wrt_module.types {
            runtime_module.types.push(type_def.clone())?;
        }

        for import_def in &wrt_module.imports {
            let extern_ty = match &import_def.desc {
                WrtImportDesc::Function(type_idx) => {
                    let ft = runtime_module
                        .types
                        .get(*type_idx as usize)
                        .map_err(|_| {
                            Error::validation_type_mismatch("Imported function type index out of bounds")
                        })?
                        .clone();
                    ExternType::Func(ft)
                }
                WrtImportDesc::Table(tt) => {
                    ExternType::Table(tt.clone())
                }
                WrtImportDesc::Memory(mt) => {
                    ExternType::Memory(*mt)
                }
                WrtImportDesc::Global(gt) => {
                    ExternType::Global(wrt_foundation::types::GlobalType {
                        value_type: gt.value_type,
                        mutable: gt.mutable,
                    })
                }
                WrtImportDesc::Extern(_) => {
                    return Err(Error::not_supported_unsupported_operation("Extern imports not supported"))
                }
                WrtImportDesc::Resource(_) => {
                    return Err(Error::not_supported_unsupported_operation("Resource imports not supported"))
                }
                _ => {
                    return Err(Error::not_supported_unsupported_operation("Unsupported import type"))
                }
            };
            // Create bounded strings for the import - avoid as_str() which is broken in no_std
            // For now, use empty strings as placeholders since as_str() is broken
            let provider = create_runtime_provider()?;
            let module_key_256: wrt_foundation::bounded::BoundedString<256, RuntimeProvider> = wrt_foundation::bounded::BoundedString::from_str_truncate(
                "", // TODO: copy from import_def.module_name when as_str() is fixed
                provider
            )?;
            let provider = create_runtime_provider()?;
            let module_key_128: wrt_foundation::bounded::BoundedString<128, RuntimeProvider> = wrt_foundation::bounded::BoundedString::from_str_truncate(
                "", // TODO: copy from import_def.module_name when as_str() is fixed
                provider
            )?;
            let provider = create_runtime_provider()?;
            let name_key_256: wrt_foundation::bounded::BoundedString<256, RuntimeProvider> = wrt_foundation::bounded::BoundedString::from_str_truncate(
                "", // TODO: copy from import_def.item_name when as_str() is fixed
                provider
            )?;
            let provider = create_runtime_provider()?;
            let name_key_128: wrt_foundation::bounded::BoundedString<128, RuntimeProvider> = wrt_foundation::bounded::BoundedString::from_str_truncate(
                "", // TODO: copy from import_def.item_name when as_str() is fixed
                provider
            )?;
            
            // Create import directly to avoid as_str() conversion issues
            let import = crate::module::Import {
                module: module_key_128,
                name: name_key_128,
                ty: extern_ty.clone(),
                desc: match &extern_ty {
                    ExternType::Func(_) => RuntimeImportDesc::Function(0),
                    ExternType::Table(table_type) => RuntimeImportDesc::Table(table_type.clone()),
                    ExternType::Memory(memory_type) => RuntimeImportDesc::Memory(memory_type.clone()),
                    ExternType::Global(global_type) => RuntimeImportDesc::Global(global_type.clone()),
                    ExternType::Tag(_) => RuntimeImportDesc::Function(0),
                    ExternType::Component(_) => RuntimeImportDesc::Function(0),
                    ExternType::Instance(_) => RuntimeImportDesc::Function(0),
                    ExternType::CoreModule(_) => RuntimeImportDesc::Function(0),
                    ExternType::TypeDef(_) => RuntimeImportDesc::Function(0),
                    ExternType::Resource(_) => RuntimeImportDesc::Function(0),
                },
            };
            let provider = create_runtime_provider()?;
        let mut inner_map = BoundedMap::new(provider)?;
            inner_map.insert(name_key_256, import)?;
            runtime_module.imports.insert(module_key_256, inner_map)?;
        }

        // Binary std/no_std choice
        // The actual bodies are filled by wrt_module.code_entries
        // Clear existing functions and prepare for new ones
        for code_entry in &wrt_module.func_bodies {
            // Find the corresponding type_idx from wrt_module.functions.
            // This assumes wrt_module.functions has the type indices for functions defined in
            // this module, and wrt_module.code_entries aligns with this.
            // A direct link or combined struct in wrt_foundation::Module would be better.
            // For now, we assume that the i-th code_entry corresponds to the i-th func type
            // index in wrt_module.functions (after accounting for imported
            // functions). This needs clarification in wrt_foundation::Module structure.
            // Let's assume wrt_module.functions contains type indices for *defined* functions
            // and code_entries matches this.
            let func_idx_in_defined_funcs = runtime_module.functions.len(); // 0-indexed among defined functions
            if func_idx_in_defined_funcs >= wrt_module.functions.len() {
                return Err(Error::validation_error("Mismatch between code entries and function type declarations";
            }
            let type_idx = wrt_module.functions.get(func_idx_in_defined_funcs).map_err(|_| Error::validation_function_not_found("Function index out of bounds"))?;

            // Convert locals from foundation format to runtime format
            let provider = create_runtime_provider()?;
            let mut runtime_locals = wrt_foundation::bounded::BoundedVec::<WrtLocalEntry, 64, RuntimeProvider>::new(provider)?;
            for local in &code_entry.locals {
                if runtime_locals.push(local).is_err() {
                    return Err(Error::runtime_execution_error("Runtime execution error: locals capacity exceeded";
                }
            }
            
            // Convert body to WrtExpr
            // For now, just use the default empty expression
            // TODO: Properly convert the instruction sequence
            let runtime_body = WrtExpr::default());
            
            runtime_module.functions.push(Function {
                type_idx,
                locals: runtime_locals,
                body: runtime_body,
            })?;
        }

        for table_def in &wrt_module.tables {
            // For now, runtime tables are created empty and populated by element segments
            // or host. This assumes runtime::table::Table::new can take
            // WrtTableType.
            runtime_module.tables.push(TableWrapper::new(Table::new(table_def.clone())?))?;
        }

        for memory_def in &wrt_module.memories {
            runtime_module.memories.push(MemoryWrapper::new(Memory::new(to_core_memory_type(memory_def))?))?;
        }

        for global_def in &wrt_module.globals {
            // GlobalType only has value_type and mutable, no initial_value
            // For now, create a default initial value based on the type
            let default_value = match global_def.value_type {
                ValueType::I32 => Value::I32(0),
                ValueType::I64 => Value::I64(0),
                ValueType::F32 => Value::F32(wrt_foundation::FloatBits32::from_float(0.0)),
                ValueType::F64 => Value::F64(wrt_foundation::FloatBits64::from_float(0.0)),
                ValueType::FuncRef => Value::FuncRef(None),
                ValueType::ExternRef => Value::ExternRef(None),
                ValueType::V128 => {
                    return Err(Error::not_supported_unsupported_operation("V128 globals not supported"))
                }
                ValueType::I16x8 => {
                    return Err(Error::not_supported_unsupported_operation("I16x8 globals not supported"))
                }
                ValueType::StructRef(_) => {
                    return Err(Error::not_supported_unsupported_operation("StructRef globals not supported"))
                }
                _ => {
                    return Err(Error::not_supported_unsupported_operation("Unsupported global value type"))
                }
            };
            
            runtime_module.globals.push(GlobalWrapper::new(Global::new(
                global_def.value_type,
                global_def.mutable,
                default_value,
            )?))?;
        }

        for export_def in &wrt_module.exports {
            let (kind, index) = match &export_def.ty {
                wrt_foundation::component::ExternType::Func(_) => {
                    // For functions, we need to find the index in the function list
                    // This is a simplified approach - in practice we'd need proper index tracking
                    (ExportKind::Function, 0) // TODO: proper function index tracking
                },
                wrt_foundation::component::ExternType::Table(_) => {
                    (ExportKind::Table, 0) // TODO: proper table index tracking
                },
                wrt_foundation::component::ExternType::Memory(_) => {
                    (ExportKind::Memory, 0) // TODO: proper memory index tracking
                },
                wrt_foundation::component::ExternType::Global(_) => {
                    (ExportKind::Global, 0) // TODO: proper global index tracking
                },
                wrt_foundation::component::ExternType::Tag(_) => {
                    return Err(Error::not_supported_unsupported_operation("Tag exports not supported"))
                }
                _ => {
                    return Err(Error::not_supported_unsupported_operation("Unsupported export type"))
                }
            };
            let provider = create_runtime_provider()?;
            let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
                export_def.name.as_str()?,
                provider
            )?;
            let export = crate::module::Export::new(String::from(name_key.as_str()?), kind, index)?;
            runtime_module.exports.insert(name_key, export)?;
        }

        // TODO: Element segments are not yet available in wrt_foundation Module
        // This will need to be implemented once element segments are added to the Module struct

        // TODO: Data segments are not yet available in wrt_foundation Module
        // This will need to be implemented once data segments are added to the Module struct

        for custom_def in &wrt_module.custom_sections {
            let provider = create_runtime_provider()?;
            let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
                custom_def.name.as_str()?,
                provider
            )?;
            runtime_module.custom_sections.insert(name_key, custom_def.data.clone())?;
        }

        Ok(runtime_module)
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Option<Export> {
        // TODO: BoundedMap doesn't support iteration, so we'll use get with a RuntimeString key
        let provider = create_runtime_provider().ok()?;
        let runtime_key: wrt_foundation::bounded::BoundedString<256, RuntimeProvider> = wrt_foundation::bounded::BoundedString::from_str_truncate(name, provider).ok()?;
        self.exports.get(&runtime_key).ok().flatten()
    }

    /// Gets a function by index
    pub fn get_function(&self, idx: u32) -> Option<Function> {
        if idx as usize >= self.functions.len() {
            return None;
        }
        self.functions.get(idx as usize).ok()
    }

    /// Gets a function type by index
    pub fn get_function_type(&self, idx: u32) -> Option<WrtFuncType<RuntimeProvider>> {
        if idx as usize >= self.types.len() {
            return None;
        }
        self.types.get(idx as usize).ok()
    }

    /// Gets a global by index
    pub fn get_global(&self, idx: usize) -> Result<GlobalWrapper> {
        self.globals.get(idx).map_err(|_| {
            Error::runtime_execution_error("Global index out of bounds")
        })
    }

    /// Gets a memory by index
    pub fn get_memory(&self, idx: usize) -> Result<MemoryWrapper> {
        self.memories.get(idx).map_err(|_| {
            Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::MEMORY_NOT_FOUND,
                "Memory index out of bounds")
        })
    }

    /// Gets a table by index
    pub fn get_table(&self, idx: usize) -> Result<TableWrapper> {
        self.tables.get(idx).map_err(|_| {
            Error::runtime_execution_error("Table index out of bounds")
        })
    }

    /// Adds a function export
    pub fn add_function_export(&mut self, name: String, index: u32) -> Result<()> {
        let export = Export::new(name.clone(), ExportKind::Function, index)?;
        #[cfg(feature = "std")]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        Ok(())
    }

    /// Adds a table export
    pub fn add_table_export(&mut self, name: String, index: u32) -> Result<()> {
        let export = Export::new(name.clone(), ExportKind::Table, index)?;
        #[cfg(feature = "std")]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        Ok(())
    }

    /// Adds a memory export
    pub fn add_memory_export(&mut self, name: String, index: u32) -> Result<()> {
        let export = Export::new(name.clone(), ExportKind::Memory, index)?;
        #[cfg(feature = "std")]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        Ok(())
    }

    /// Adds a global export
    pub fn add_global_export(&mut self, name: String, index: u32) -> Result<()> {
        let export = Export::new(name.clone(), ExportKind::Global, index)?;
        #[cfg(feature = "std")]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
                name.as_str(),
                create_runtime_provider()?
            )?;
            self.exports.insert(bounded_name, export)?;
        }
        Ok(())
    }

    /// Adds an export to the module from a `wrt_format::module::Export`
    pub fn add_export(&mut self, format_export: wrt_format::module::Export) -> Result<()> {
        let runtime_export_kind = match format_export.kind {
            wrt_format::module::ExportKind::Function => ExportKind::Function,
            wrt_format::module::ExportKind::Table => ExportKind::Table,
            wrt_format::module::ExportKind::Memory => ExportKind::Memory,
            wrt_format::module::ExportKind::Global => ExportKind::Global,
            wrt_format::module::ExportKind::Tag => {
                return Err(Error::not_supported_unsupported_operation("Tag exports not supported"))
            }
        };
        // Convert BoundedString to String - use default empty string if conversion fails
        let export_name_string = String::from("export"); // Use a placeholder name
        let runtime_export = Export::new(export_name_string, runtime_export_kind, format_export.index)?;
        let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
            runtime_export.name.as_str().map_err(|_| Error::runtime_error("Invalid export name"))?,
            create_runtime_provider()?
        )?;
        self.exports.insert(name_key, runtime_export)?;
        Ok(())
    }

    /// Set the name of the module
    pub fn set_name(&mut self, name: String) -> Result<()> {
        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
            &name,
            create_runtime_provider()?
        )?;
        self.name = Some(bounded_name;
        Ok(())
    }

    /// Set the start function index
    pub fn set_start(&mut self, start: u32) -> Result<()> {
        self.start = Some(start;
        Ok(())
    }

    /// Add a function type to the module
    pub fn add_type(&mut self, ty: WrtFuncType<RuntimeProvider>) -> Result<()> {
        self.types.push(ty)?;
        Ok(())
    }

    /// Add a function import to the module
    pub fn add_import_func(
        &mut self,
        module_name: &str,
        item_name: &str,
        type_idx: u32,
    ) -> Result<()> {
        let func_type = self
            .types
            .get(type_idx as usize)
            .map_err(|_| {
                Error::validation_type_mismatch("Type index out of bounds for import func")
            })?
            .clone();

        let import_struct = crate::module::Import::new(
            String::from(module_name),
            String::from(item_name),
            ExternType::Func(func_type),
            RuntimeImportDesc::Function(0), // Function index would need to be determined properly
        )?;
        #[cfg(feature = "std")]
        {
            // Convert to bounded strings
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            
            // For BoundedMap, we need to handle the nested map differently
            // First check if module exists
            let mut inner_map = match self.imports.get(&bounded_module)? {
                Some(existing) => existing,
                None => ImportMap::new(create_runtime_provider()?)?
            };
            
            // Insert the import into the inner map
            let _ = inner_map.insert(bounded_item, import_struct)?;
            
            // Update the outer map
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            // BoundedMap doesn't support get_mut, so we'll use a simpler approach
            let provider = create_runtime_provider()?;
        let mut inner_map = BoundedMap::new(provider)?;
            let _ = inner_map.insert(bounded_item, import_struct)?;
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        Ok(())
    }

    /// Adds an imported table to the module
    pub fn add_import_table(
        &mut self,
        module_name: &str,
        item_name: &str,
        table_type: WrtTableType,
    ) -> Result<()> {
        let import_struct = crate::module::Import::new(
            String::from(module_name),
            String::from(item_name),
            ExternType::Table(table_type.clone()),
            RuntimeImportDesc::Table(table_type),
        )?;
        #[cfg(feature = "std")]
        {
            // Convert to bounded strings
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            
            // For BoundedMap, we need to handle the nested map differently
            // First check if module exists
            let mut inner_map = match self.imports.get(&bounded_module)? {
                Some(existing) => existing,
                None => ImportMap::new(create_runtime_provider()?)?
            };
            
            // Insert the import into the inner map
            let _ = inner_map.insert(bounded_item, import_struct)?;
            
            // Update the outer map
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            // BoundedMap doesn't support get_mut, so we'll use a simpler approach
            let provider = create_runtime_provider()?;
        let mut inner_map = BoundedMap::new(provider)?;
            let _ = inner_map.insert(bounded_item, import_struct)?;
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        Ok(())
    }

    /// Adds an imported memory to the module
    pub fn add_import_memory(
        &mut self,
        module_name: &str,
        item_name: &str,
        memory_type: WrtMemoryType,
    ) -> Result<()> {
        let import_struct = crate::module::Import::new(
            String::from(module_name),
            String::from(item_name),
            ExternType::Memory(memory_type.clone()),
            RuntimeImportDesc::Memory(memory_type),
        )?;
        #[cfg(feature = "std")]
        {
            // Convert to bounded strings
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            
            // For BoundedMap, we need to handle the nested map differently
            // First check if module exists
            let mut inner_map = match self.imports.get(&bounded_module)? {
                Some(existing) => existing,
                None => ImportMap::new(create_runtime_provider()?)?
            };
            
            // Insert the import into the inner map
            let _ = inner_map.insert(bounded_item, import_struct)?;
            
            // Update the outer map
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            // BoundedMap doesn't support get_mut, so we'll use a simpler approach
            let provider = create_runtime_provider()?;
        let mut inner_map = BoundedMap::new(provider)?;
            let _ = inner_map.insert(bounded_item, import_struct)?;
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        Ok(())
    }

    /// Adds an imported global to the module
    pub fn add_import_global(
        &mut self,
        module_name: &str,
        item_name: &str,
        format_global: wrt_format::module::Global,
    ) -> Result<()> {
        let component_global_type = wrt_foundation::types::GlobalType {
            value_type: format_global.global_type.value_type,
            mutable: format_global.global_type.mutable,
        };

        let import = Import::new(
            String::from(module_name),
            String::from(item_name),
            ExternType::Global(component_global_type),
            RuntimeImportDesc::Global(component_global_type),
        )?;

        let module_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
            module_name,
            create_runtime_provider()?
        )?;
        let item_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
            item_name,
            create_runtime_provider()?
        )?;
        let provider = create_runtime_provider()?;
        let mut inner_map = BoundedMap::new(provider)?;
        inner_map.insert(item_key, import)?;
        self.imports.insert(module_key, inner_map)?;
        Ok(())
    }

    /// Add a function to the module
    pub fn add_function_type(&mut self, type_idx: u32) -> Result<()> {
        if type_idx as usize >= self.types.len() {
            return Err(Error::validation_type_mismatch("Function type index out of bounds";
        }

        let provider = create_runtime_provider()?;
        let function = Function { 
            type_idx, 
            locals: wrt_foundation::bounded::BoundedVec::new(provider)?, 
            body: WrtExpr::default() 
        };

        self.functions.push(function)?;
        Ok(())
    }

    /// Add a table to the module
    pub fn add_table(&mut self, table_type: WrtTableType) -> Result<()> {
        self.tables.push(TableWrapper::new(Table::new(table_type)?))?;
        Ok(())
    }

    /// Add a memory to the module
    pub fn add_memory(&mut self, memory_type: WrtMemoryType) -> Result<()> {
        self.memories.push(MemoryWrapper::new(Memory::new(to_core_memory_type(memory_type))?))?;
        Ok(())
    }

    /// Add a global to the module
    pub fn add_global(&mut self, global_type: WrtGlobalType, init: WrtValue) -> Result<()> {
        let global = Global::new(global_type.value_type, global_type.mutable, init)?;
        self.globals.push(GlobalWrapper::new(global))?;
        Ok(())
    }

    /// Add a function export to the module
    pub fn add_export_func(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.functions.len() {
            return Err(Error::validation_error(
                "Export function index out of bounds"
            ;
        }

        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
            name,
            create_runtime_provider()?
        )?;
        let export = Export::new(String::from(name), ExportKind::Function, index)?;
        self.exports.insert(bounded_name, export)?;
        Ok(())
    }

    /// Add a table export to the module
    pub fn add_export_table(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.tables.len() {
            return Err(Error::validation_error(
                "Export table index out of bounds"
            ;
        }

        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
            name,
            create_runtime_provider()?
        )?;
        let export = Export::new(String::from(bounded_name.as_str()?), ExportKind::Table, index)?;
        self.exports.insert(bounded_name, export)?;
        Ok(())
    }

    /// Add a memory export to the module
    pub fn add_export_memory(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.memories.len() {
            return Err(Error::validation_error(
                "Export memory index out of bounds"
            ;
        }

        let export = Export::new(String::from(name), ExportKind::Memory, index)?;

        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
            name,
            create_runtime_provider()?
        )?;
        self.exports.insert(bounded_name, export)?;
        Ok(())
    }

    /// Add a global export to the module
    pub fn add_export_global(&mut self, name: &str, index: u32) -> Result<()> {
        if index as usize >= self.globals.len() {
            return Err(Error::validation_error(
                "Export global index out of bounds"
            ;
        }

        let export = Export::new(String::from(name), ExportKind::Global, index)?;

        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
            name,
            create_runtime_provider()?
        )?;
        self.exports.insert(bounded_name, export)?;
        Ok(())
    }

    /// Add an element segment to the module
    pub fn add_element(&mut self, element: wrt_format::module::Element) -> Result<()> {
        // Convert format element to runtime element
        let items = match &element.init {
            wrt_format::module::ElementInit::FuncIndices(func_indices) => {
                // For function indices, copy them
                let provider = create_runtime_provider()?;
                let mut bounded_items = wrt_foundation::bounded::BoundedVec::new(provider)?;
                for idx in func_indices.iter() {
                    bounded_items.push(*idx)?;
                }
                bounded_items
            }
            wrt_format::module::ElementInit::Expressions(_expressions) => {
                // For expressions, create empty items list for now (TODO: process expressions)
                let provider = create_runtime_provider()?;
                wrt_foundation::bounded::BoundedVec::new(provider)?
            }
        };
        
        // Extract table index from mode if available
        let table_idx = match &element.mode {
            wrt_format::module::ElementMode::Active { table_index, .. } => Some(*table_index),
            _ => None,
        };
        
        let runtime_element = crate::module::Element {
            mode: WrtElementMode::Active { table_index: 0, offset: 0 }, // Default mode, should be determined from element.mode
            table_idx,
            offset_expr: None, // Would need to convert from element.mode offset_expr
            element_type: element.element_type,
            items,
        };

        self.elements.push(runtime_element)?;
        Ok(())
    }

    /// Set a function body
    pub fn set_function_body(
        &mut self,
        func_idx: u32,
        type_idx: u32,
        locals: Vec<WrtLocalEntry>,
        body: WrtExpr,
    ) -> Result<()> {
        if func_idx as usize > self.functions.len() {
            // Allow appending
            return Err(Error::runtime_function_not_found("Function index out of bounds for set_function_body";
        }
        
        // Convert Vec<WrtLocalEntry> to BoundedVec
        let provider = create_runtime_provider()?;
        let mut bounded_locals = wrt_foundation::bounded::BoundedVec::<WrtLocalEntry, 64, RuntimeProvider>::new(provider)?;
        for local in locals {
            bounded_locals.push(local)?;
        }
        
        let func_entry = Function { type_idx, locals: bounded_locals, body };
        if func_idx as usize == self.functions.len() {
            self.functions.push(func_entry)?;
        } else {
            let _ = self.functions.set(func_idx as usize, func_entry).map_err(|_| Error::runtime_component_limit_exceeded("Failed to set function entry"))?;
        }
        Ok(())
    }

    /// Add a data segment to the module
    pub fn add_data(&mut self, data: wrt_format::module::Data) -> Result<()> {
        // Convert format data to runtime data
        let provider = create_runtime_provider()?;
        let mut init_4096 = wrt_foundation::bounded::BoundedVec::new(provider)?;
        
        // Copy data from the format's init (Vec<u8> in std mode)
        for byte in &data.init {
            init_4096.push(*byte)?;
        }
        
        let runtime_data = crate::module::Data {
            mode: WrtDataMode::Active { memory_index: 0, offset: 0 }, // Default mode
            memory_idx: Some(0), // Default memory index - field is deprecated
            offset_expr: None, // Would need to convert from data.offset
            init: init_4096,
        };

        self.data.push(runtime_data)?;
        Ok(())
    }

    /// Add a custom section to the module
    pub fn add_custom_section(&mut self, name: &str, data: Vec<u8>) -> Result<()> {
        let provider_key = create_runtime_provider()?;
        let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(name, provider_key)?;
        let provider_data = create_runtime_provider()?;
        let mut bounded_data = wrt_foundation::bounded::BoundedVec::<u8, 4096, RuntimeProvider>::new(provider_data)?;
        for byte in data {
            bounded_data.push(byte)?;
        }
        self.custom_sections.insert(name_key, bounded_data)?;
        Ok(())
    }

    /// Set the binary representation of the module
    pub fn set_binary(&mut self, binary: Vec<u8>) -> Result<()> {
        let provider = create_runtime_provider()?;
        let mut bounded_binary = wrt_foundation::bounded::BoundedVec::<u8, 65536, RuntimeProvider>::new(provider)?;
        for byte in binary {
            bounded_binary.push(byte)?;
        }
        self.binary = Some(bounded_binary;
        Ok(())
    }

    /// Validate the module
    pub fn validate(&self) -> Result<()> {
        // TODO: Implement comprehensive validation of the runtime module structure.
        // - Check type indices are valid.
        // - Check function indices in start/exports/elements are valid.
        // - Check table/memory/global indices.
        // - Validate instruction sequences in function bodies (optional, decoder should
        //   do most of this).
        Ok(())
    }

    /// Add an import runtime global to the module
    pub fn add_import_runtime_global(
        &mut self,
        module_name: &str,
        item_name: &str,
        global_type: WrtGlobalType,
    ) -> Result<()> {
        let component_global_type = wrt_foundation::types::GlobalType {
            value_type: global_type.value_type,
            mutable: global_type.mutable,
        };
        let import_struct = crate::module::Import::new(
            String::from(module_name),
            String::from(item_name),
            ExternType::Global(component_global_type),
            RuntimeImportDesc::Global(component_global_type),
        )?;
        #[cfg(feature = "std")]
        {
            // Convert to bounded strings
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            
            // For BoundedMap, we need to handle the nested map differently
            // First check if module exists
            let mut inner_map = match self.imports.get(&bounded_module)? {
                Some(existing) => existing,
                None => ImportMap::new(create_runtime_provider()?)?
            };
            
            // Insert the import into the inner map
            let _ = inner_map.insert(bounded_item, import_struct)?;
            
            // Update the outer map
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        #[cfg(not(feature = "std"))]
        {
            let bounded_module = wrt_foundation::bounded::BoundedString::from_str_truncate(
                module_name,
                create_runtime_provider()?
            )?;
            let bounded_item = wrt_foundation::bounded::BoundedString::from_str_truncate(
                item_name,
                create_runtime_provider()?
            )?;
            // BoundedMap doesn't support get_mut, so we'll use a simpler approach
            let provider = create_runtime_provider()?;
        let mut inner_map = BoundedMap::new(provider)?;
            let _ = inner_map.insert(bounded_item, import_struct)?;
            let _ = self.imports.insert(bounded_module, inner_map)?;
        }
        Ok(())
    }

    /// Add a runtime export to the module
    pub fn add_runtime_export(&mut self, name: String, export_desc: WrtExportDesc) -> Result<()> {
        let (kind, index) = match export_desc {
            WrtExportDesc::Func(idx) => (ExportKind::Function, idx),
            WrtExportDesc::Table(idx) => (ExportKind::Table, idx),
            WrtExportDesc::Mem(idx) => (ExportKind::Memory, idx),
            WrtExportDesc::Global(idx) => (ExportKind::Global, idx),
            WrtExportDesc::Tag(_) => {
                return Err(Error::not_supported_unsupported_operation("Tag exports not supported"))
            }
        };
        let runtime_export = crate::module::Export::new(name.clone(), kind, index)?;
        let provider = create_runtime_provider()?;
        let provider = create_runtime_provider()?;
        let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(&name, provider)?;
        self.exports.insert(name_key, runtime_export)?;
        Ok(())
    }

    /// Add a runtime element to the module
    pub fn add_runtime_element(&mut self, element_segment: WrtElementSegment) -> Result<()> {
        // TODO: Resolve element_segment.items expressions if they are not direct
        // indices. This is a placeholder and assumes items can be derived or
        // handled during instantiation.
        // TODO: ElementItems type not available yet, using empty items for now
        let provider = create_runtime_provider()?;
        let items_resolved = wrt_foundation::bounded::BoundedVec::new(provider)?;

        // Convert element mode from PureElementMode to WrtElementMode
        let runtime_mode = match &element_segment.mode {
            wrt_format::pure_format_types::PureElementMode::Active { table_index, .. } => {
                WrtElementMode::Active { 
                    table_index: *table_index, 
                    offset: 0 // Simplified - would need to evaluate offset_expr_bytes
                }
            },
            wrt_format::pure_format_types::PureElementMode::Passive => WrtElementMode::Passive,
            wrt_format::pure_format_types::PureElementMode::Declared => WrtElementMode::Declarative,
        };

        self.elements.push(crate::module::Element {
            mode: runtime_mode,
            table_idx: None, // Simplified for now
            offset_expr: None, // Element segment doesn't have direct offset_expr field
            element_type: element_segment.element_type,
            items: items_resolved,
        })?;
        Ok(())
    }

    /// Add a runtime data segment to the module  
    pub fn add_runtime_data(&mut self, data_segment: WrtDataSegment) -> Result<()> {
        // WrtDataSegment is actually PureDataSegment
        // Convert data mode from PureDataMode to WrtDataMode
        let (runtime_mode, memory_idx) = match &data_segment.mode {
            wrt_format::pure_format_types::PureDataMode::Active { memory_index, .. } => {
                (WrtDataMode::Active { 
                    memory_index: *memory_index, 
                    offset: 0 // Simplified - would need to evaluate offset_expr_bytes
                }, Some(*memory_index))
            },
            wrt_format::pure_format_types::PureDataMode::Passive => {
                (WrtDataMode::Passive, None)
            },
        };

        // Convert data_segment.data_bytes to larger capacity
        let provider = create_runtime_provider()?;
        let mut runtime_init = wrt_foundation::bounded::BoundedVec::<u8, 4096, RuntimeProvider>::new(provider)?;
        for byte in data_segment.data_bytes.iter() {
            runtime_init.push(*byte)?;
        }
        
        self.data.push(crate::module::Data {
            mode: runtime_mode,
            memory_idx,
            offset_expr: None, // Simplified for now
            init: runtime_init,
        })?;
        Ok(())
    }

    /// Add a custom section to the module
    pub fn add_custom_section_runtime(&mut self, section: WrtCustomSection<RuntimeProvider>) -> Result<()> {
        let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
            section.name.as_str()?,
            create_runtime_provider()?
        )?;
        // Convert section.data to the expected type
        let provider_data = create_runtime_provider()?;
        let mut bounded_data = wrt_foundation::bounded::BoundedVec::<u8, 4096, RuntimeProvider>::new(provider_data)?;
        for i in 0..section.data.len() {
            bounded_data.push(section.data.get(i)?)?;
        }
        self.custom_sections.insert(name_key, bounded_data)?;
        Ok(())
    }

    /// Set the binary representation of the module (alternative method)
    pub fn set_binary_runtime(&mut self, binary: Vec<u8>) -> Result<()> {
        let provider = create_runtime_provider()?;
        let mut bounded_binary = wrt_foundation::bounded::BoundedVec::<u8, 65536, RuntimeProvider>::new(provider)?;
        for byte in binary {
            bounded_binary.push(byte)?;
        }
        self.binary = Some(bounded_binary;
        Ok(())
    }

    /// Load a module from WebAssembly binary
    ///
    /// This method uses streaming decoding to minimize memory usage.
    /// The binary is processed section by section without loading
    /// the entire module into intermediate data structures.
    pub fn load_from_binary(&mut self, binary: &[u8]) -> Result<Self> {
        // Use wrt-decoder's unified loader for efficient parsing
        use wrt_decoder::{load_wasm_unified, WasmFormat};
        
        // Load using unified API to get both module info and cached data
        let wasm_info = load_wasm_unified(binary)?;
        
        // Ensure this is a core module
        if !wasm_info.is_core_module() {
            return Err(Error::validation_type_mismatch("Binary is not a WebAssembly core module";
        }
        
        let module_info = wasm_info.require_module_info()?;
        
        // Create runtime module from unified API data
        let runtime_module = Self::from_module_info(module_info, binary)?;
        
        // Store the binary for later use
        // Note: This is the only place where we keep the full binary in memory
        // Consider using a streaming approach here too if binary size is a concern
        let provider = create_runtime_provider()?;
        let mut bounded_binary = wrt_foundation::bounded::BoundedVec::<u8, 65536, RuntimeProvider>::new(provider)?;
        for byte in binary {
            bounded_binary.push(*byte)?;
        }
        
        Ok(Self {
            binary: Some(bounded_binary),
            validated: true,
            ..runtime_module
        })
    }

    /// Create runtime Module from unified API ModuleInfo
    fn from_module_info(module_info: &wrt_decoder::ModuleInfo, binary: &[u8]) -> Result<Self> {
        let mut runtime_module = Self::new()?;

        // Set start function if present
        runtime_module.start = module_info.start_function;

        // Process imports
        for import in &module_info.imports {
            let extern_type = match &import.import_type {
                wrt_decoder::ImportType::Function(type_idx) => {
                    // For now, create a simple function type
                    // In a full implementation, we'd look up the actual type
                    let func_type = WrtFuncType::new(
                        create_runtime_provider()?,
                        core::iter::empty::<WrtValueType>(), // empty params
                        core::iter::empty::<WrtValueType>()  // empty results
                    )?;
                    ExternType::Func(func_type)
                }
                wrt_decoder::ImportType::Table => {
                    // Create default table type
                    let table_type = WrtTableType {
                        element_type: WrtRefType::Funcref,
                        limits: WrtLimits { min: 0, max: None },
                    };
                    ExternType::Table(table_type)
                }
                wrt_decoder::ImportType::Memory => {
                    // Create default memory type
                    let memory_type = WrtMemoryType {
                        limits: WrtLimits { min: 1, max: None },
                        shared: false,
                    };
                    ExternType::Memory(memory_type)
                }
                wrt_decoder::ImportType::Global => {
                    // Create default global type
                    let global_type = wrt_foundation::types::GlobalType {
                        value_type: WrtValueType::I32,
                        mutable: false,
                    };
                    ExternType::Global(global_type)
                }
            };

            // Create the import
            let import_struct = crate::module::Import::new(
                String::from(import.module.as_str()),
                String::from(import.name.as_str()),
                extern_type.clone(),
                match &extern_type {
                    ExternType::Func(_func_type) => RuntimeImportDesc::Function(0), // TODO: proper type index lookup
                    ExternType::Table(table_type) => RuntimeImportDesc::Table(table_type.clone()),
                    ExternType::Memory(memory_type) => RuntimeImportDesc::Memory(memory_type.clone()),
                    ExternType::Global(global_type) => RuntimeImportDesc::Global(global_type.clone()),
                    ExternType::Tag(_) => RuntimeImportDesc::Function(0), // Handle tag as function placeholder
                    ExternType::Component(_) => RuntimeImportDesc::Function(0), // Component imports not supported yet
                    ExternType::Instance(_) => RuntimeImportDesc::Function(0), // Instance imports not supported yet
                    ExternType::CoreModule(_) => RuntimeImportDesc::Function(0), // Core module imports not supported yet
                    ExternType::TypeDef(_) => RuntimeImportDesc::Function(0), // Type definition imports not supported yet
                    ExternType::Resource(_) => RuntimeImportDesc::Function(0), // Resource imports not supported yet
                },
            )?;

            // Add to imports map
            let module_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
                &import.module,
                create_runtime_provider()?
            )?;
            let item_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
                &import.name,
                create_runtime_provider()?
            )?;

            // Get or create inner map
            let mut inner_map = match runtime_module.imports.get(&module_key)? {
                Some(existing) => existing,
                None => ImportMap::new(create_runtime_provider()?)?
            };

            // Insert the import
            inner_map.insert(item_key, import_struct)?;
            runtime_module.imports.insert(module_key, inner_map)?;
        }

        // Process exports
        for export in &module_info.exports {
            let export_kind = match export.export_type {
                wrt_decoder::ExportType::Function => ExportKind::Function,
                wrt_decoder::ExportType::Table => ExportKind::Table,
                wrt_decoder::ExportType::Memory => ExportKind::Memory,
                wrt_decoder::ExportType::Global => ExportKind::Global,
            };

            let runtime_export = Export::new(export.name.clone(), export_kind, export.index)?;
            let name_key = wrt_foundation::bounded::BoundedString::from_str_truncate(
                &export.name,
                create_runtime_provider()?
            )?;
            runtime_module.exports.insert(name_key, runtime_export)?;
        }

        // Set memory info if present
        if let Some((min_pages, max_pages)) = module_info.memory_pages {
            let memory_type = WrtMemoryType {
                limits: WrtLimits { min: min_pages, max: max_pages },
                shared: false,
            };
            runtime_module.memories.push(MemoryWrapper::new(Memory::new(to_core_memory_type(memory_type))?))?;
        }

        // For now, we'll use the fallback decoder for full section parsing if needed
        // This ensures compatibility while leveraging the unified API for basic info
        if module_info.function_types.len() > 0 {
            // Fall back to full parsing for complex cases
            use wrt_decoder::decoder;
            let decoded_module = decoder::decode_module(binary)?;
            
            // decoded_module is wrt_format::Module, so we need the format-compatible method
            #[cfg(feature = "std")]
            let full_runtime_module = Module::from_wrt_module(&decoded_module)?;
            #[cfg(not(feature = "std"))]
            let full_runtime_module = Module::from_wrt_module_nostd(&decoded_module)?;
            
            return Ok(full_runtime_module;
        }

        Ok(runtime_module)
    }

    /// Find a function export by name
    pub fn find_function_by_name(&self, name: &str) -> Option<u32> {
        let provider = create_runtime_provider().ok()?;
        let bounded_name = wrt_foundation::bounded::BoundedString::from_str_truncate(
            name, 
            provider
        ).ok()?;
        
        if let Ok(Some(export)) = self.exports.get(&bounded_name) {
            if export.kind == ExportKind::Function {
                return Some(export.index;
            }
        }
        None
    }

    /// Get function signature by function index
    pub fn get_function_signature(&self, func_idx: u32) -> Option<WrtFuncType<RuntimeProvider>> {
        let function = self.get_function(func_idx)?;
        self.get_function_type(function.type_idx)
    }

    /// Validate that a function exists and can be called
    pub fn validate_function_call(&self, name: &str) -> Result<u32> {
        match self.find_function_by_name(name) {
            Some(func_idx) => {
                // Verify function exists
                if self.get_function(func_idx).is_some() {
                    Ok(func_idx)
                } else {
                    Err(Error::runtime_function_not_found("Function index is invalid"))
                }
            }
            None => Err(Error::runtime_function_not_found("Function not found in exports"))
        }
    }
}

/// Additional exports that are not part of the standard WebAssembly exports
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherExport {
    /// Export name
    pub name: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
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
        module: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The function name
        name: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The function type
        ty: WrtFuncType<RuntimeProvider>,
    },
    /// An imported table
    Table {
        /// The module name
        module: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The table name
        name: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The table type
        ty: WrtTableType,
    },
    /// An imported memory
    Memory {
        /// The module name
        module: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The memory name
        name: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The memory type
        ty: WrtMemoryType,
    },
    /// An imported global
    Global {
        /// The module name
        module: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The global name
        name: wrt_foundation::bounded::BoundedString<128, RuntimeProvider>,
        /// The global type
        ty: WrtGlobalType,
    },
}

// Trait implementations for Module
impl wrt_foundation::traits::Checksummable for Module {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        // Use module name (if available) and validation status for checksum
        if let Some(ref name) = self.name {
            if let Ok(name_str) = name.as_str() {
                checksum.update_slice(name_str.as_bytes);
            }
        } else {
            // Use a default identifier if no name is available
            checksum.update_slice(b"unnamed_module";
        }
        checksum.update_slice(&[if self.validated { 1 } else { 0 }];
        checksum.update_slice(&(self.types.len() as u32).to_le_bytes);
        checksum.update_slice(&(self.functions.len() as u32).to_le_bytes);
    }
}

impl wrt_foundation::traits::ToBytes for Module {
    fn serialized_size(&self) -> usize {
        // Simple size calculation for module metadata
        let name_size = self.name.as_ref().map_or(0, |n| n.len);
        8 + name_size + 1 + 4 + 4 // magic(4) + name_len(4) + name + validated(1) + types_len(4) + functions_len(4)
    }
    
    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        // Write a magic number to identify this as a module
        writer.write_all(&0x6D6F6475u32.to_le_bytes())?; // "modu" in little endian
        
        // Write module name length and name
        if let Some(ref name) = self.name {
            if let Ok(name_str) = name.as_str() {
                writer.write_all(&(name_str.len() as u32).to_le_bytes())?;
                writer.write_all(name_str.as_bytes())?;
            } else {
                writer.write_all(&0u32.to_le_bytes())?; // Error getting name
            }
        } else {
            writer.write_all(&0u32.to_le_bytes())?; // No name
        }
        
        // Write validation status
        writer.write_all(&[if self.validated { 1 } else { 0 }])?;
        
        // Write type and function counts
        writer.write_all(&(self.types.len() as u32).to_le_bytes())?;
        writer.write_all(&(self.functions.len() as u32).to_le_bytes())?;
        
        Ok(())
    }
}

impl wrt_foundation::traits::FromBytes for Module {
    fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        // Read and verify magic number
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if u32::from_le_bytes(magic) != 0x6D6F6475 {
            return Err(wrt_error::Error::runtime_error("Invalid module magic number";
        }
        
        // Read module name
        let mut name_len_bytes = [0u8; 4];
        reader.read_exact(&mut name_len_bytes)?;
        let name_len = u32::from_le_bytes(name_len_bytes;
        
        let name = if name_len > 0 && name_len <= 128 {
            // Use a fixed-size buffer for reading the name
            let mut name_bytes = [0u8; 128];
            reader.read_exact(&mut name_bytes[..name_len as usize])?;
            let name_str = core::str::from_utf8(&name_bytes[..name_len as usize])
                .map_err(|_| wrt_error::Error::runtime_error("Invalid module name UTF-8"))?;
            let provider = create_runtime_provider()?;
            Some(wrt_foundation::bounded::BoundedString::from_str(name_str, provider)?)
        } else {
            None
        };
        
        // Read validation status
        let mut validated_byte = [0u8; 1];
        reader.read_exact(&mut validated_byte)?;
        let validated = validated_byte[0] != 0;
        
        // Read type and function counts (for validation)
        let mut counts = [0u8; 8];
        reader.read_exact(&mut counts)?;
        
        // Create a new empty module with the restored name and validation status
        let mut module = Module::new()?;
        module.name = name;
        module.validated = validated;
        
        Ok(module)
    }
}


// HashMap is already imported above, no need to re-import

use wrt_error::{Error, ErrorCategory, Result};
use wrt_foundation::component::ExternType; // For error handling

// Newtype wrappers to solve orphan rules issue
// These allow us to implement external traits on types containing Arc<T>

/// Wrapper for Arc<Table> to enable trait implementations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableWrapper(pub Arc<Table>;

impl Default for TableWrapper {
    fn default() -> Self {
        use wrt_foundation::types::{Limits, TableType, RefType};
        let table_type = TableType {
            element_type: RefType::Funcref,
            limits: Limits { min: 0, max: Some(1) },
        };
        Self::new(Table::new(table_type).unwrap())
    }
}

impl TableWrapper {
    /// Create a new table wrapper
    pub fn new(table: Table) -> Self {
        Self(Arc::new(table))
    }
    
    /// Get a reference to the inner table
    #[must_use] pub fn inner(&self) -> &Arc<Table> {
        &self.0
    }
    
    /// Unwrap to get the Arc<Table>
    #[must_use] pub fn into_inner(self) -> Arc<Table> {
        self.0
    }
    
    /// Get table size
    #[must_use] pub fn size(&self) -> u32 {
        self.0.size()
    }
    
    /// Get table element
    pub fn get(&self, idx: u32) -> Result<Option<WrtValue>> {
        self.0.get(idx)
    }
    
    /// Set table element (requires mutable access)
    pub fn set(&self, idx: u32, value: Option<WrtValue>) -> Result<()> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Table>
        // For now, we'll return an error
        Err(Error::runtime_execution_error("Runtime execution error: Cannot set table value through Arc<Table>"))
    }
    
    /// Grow table (requires mutable access)
    pub fn grow(&self, delta: u32, init_value: WrtValue) -> Result<u32> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Table>
        // For now, we'll return an error
        Err(Error::new(
            ErrorCategory::Runtime,
            wrt_error::codes::TABLE_ACCESS_DENIED,
            "Cannot grow table through Arc<Table>"))
    }
    
    /// Initialize table (requires mutable access)
    pub fn init(&self, offset: u32, init_data: &[Option<WrtValue>]) -> Result<()> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Table>
        // For now, we'll return an error
        Err(Error::runtime_execution_error("Runtime execution error: Cannot initialize table through Arc<Table>"))
    }
}

/// Wrapper for Arc<Memory> to enable trait implementations  
/// Memory guard for atomic operations
#[derive(Debug)]
pub struct MemoryGuard {
    memory: Arc<Memory>,
}

impl MemoryGuard {
    /// Read from memory
    pub fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<()> {
        self.memory.read(offset as u32, buffer)
    }
    
    /// Write to memory (atomic operations may need this)
    pub fn write(&self, offset: usize, buffer: &[u8]) -> Result<()> {
        // For atomic operations, we need to allow writes even through Arc
        // This is safe because atomic operations are inherently thread-safe
        unsafe {
            let memory_ptr = Arc::as_ptr(&self.memory) as *mut Memory;
            (*memory_ptr).write(offset as u32, buffer)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWrapper(pub Arc<Memory>;

impl Default for MemoryWrapper {
    fn default() -> Self {
        use wrt_foundation::types::{Limits, MemoryType};
        let memory_type = MemoryType {
            limits: Limits { min: 1, max: Some(1) },
            shared: false,
        };
        Self::new(Memory::new(to_core_memory_type(memory_type)).unwrap())
    }
}

impl MemoryWrapper {
    /// Create a new memory wrapper
    pub fn new(memory: Memory) -> Self {
        Self(Arc::new(memory))
    }
    
    /// Get a reference to the inner memory
    #[must_use] pub fn inner(&self) -> &Arc<Memory> {
        &self.0
    }
    
    /// Unwrap to get the Arc<Memory>
    #[must_use] pub fn into_inner(self) -> Arc<Memory> {
        self.0
    }
    
    /// Get memory size in bytes
    #[must_use] pub fn size_in_bytes(&self) -> usize {
        self.0.size_in_bytes()
    }
    
    /// Get memory size in pages
    #[must_use] pub fn size(&self) -> u32 {
        self.0.size()
    }
    
    /// Get memory size in pages (alias for compatibility)
    #[must_use] pub fn size_pages(&self) -> u32 {
        self.0.size()
    }
    
    /// Get memory size in bytes (alias for compatibility)
    #[must_use] pub fn size_bytes(&self) -> usize {
        self.0.size_in_bytes()
    }
    
    /// Read from memory
    pub fn read(&self, offset: u32, buffer: &mut [u8]) -> Result<()> {
        self.0.read(offset, buffer)
    }
    
    /// Write to memory (requires mutable access to Arc<Memory>)
    pub fn write(&self, offset: u32, buffer: &[u8]) -> Result<()> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Memory>
        // For now, we'll return an error
        Err(Error::new(
            ErrorCategory::Runtime,
            wrt_error::codes::MEMORY_ACCESS_DENIED,
            "Cannot write to memory through Arc<Memory>"))
    }
    
    /// Grow memory (requires mutable access)
    pub fn grow(&self, pages: u32) -> Result<u32> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Memory>
        // For now, we'll return an error
        Err(Error::runtime_execution_error("Runtime execution error: Cannot grow memory through Arc<Memory>"))
    }
    
    /// Write i32 to memory
    pub fn write_i32(&self, offset: u32, value: i32) -> Result<()> {
        use crate::memory_helpers::ArcMemoryExt;
        self.0.write_i32(offset, value)
    }
    
    /// Write i64 to memory
    pub fn write_i64(&self, offset: u32, value: i64) -> Result<()> {
        use crate::memory_helpers::ArcMemoryExt;
        self.0.write_i64(offset, value)
    }
    
    /// Write f32 to memory
    pub fn write_f32(&self, offset: u32, value: f32) -> Result<()> {
        use crate::memory_helpers::ArcMemoryExt;
        self.0.write_f32(offset, value)
    }
    
    /// Write f64 to memory
    pub fn write_f64(&self, offset: u32, value: f64) -> Result<()> {
        use crate::memory_helpers::ArcMemoryExt;
        self.0.write_f64(offset, value)
    }
    
    /// Fill memory (requires mutable access)
    pub fn fill(&self, offset: u32, len: u32, value: u8) -> Result<()> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Memory>
        // For now, we'll return an error
        Err(Error::new(
            ErrorCategory::Runtime,
            wrt_error::codes::MEMORY_ACCESS_DENIED,
            "Cannot fill memory through Arc<Memory>"))
    }
    
    /// Get a memory guard for atomic operations
    pub fn lock(&self) -> MemoryGuard {
        MemoryGuard {
            memory: self.0.clone()
        }
    }
}

/// Wrapper for Arc<Global> to enable trait implementations
#[derive(Debug, Clone, PartialEq, Eq)]  
pub struct GlobalWrapper(pub Arc<Global>;

impl Default for GlobalWrapper {
    fn default() -> Self {
        use wrt_foundation::types::ValueType;
        use wrt_foundation::values::Value;
        Self::new(Global::new(ValueType::I32, false, Value::I32(0)).unwrap())
    }
}

impl GlobalWrapper {
    /// Create a new global wrapper
    pub fn new(global: Global) -> Self {
        Self(Arc::new(global))
    }
    
    /// Get a reference to the inner global
    #[must_use] pub fn inner(&self) -> &Arc<Global> {
        &self.0
    }
    
    /// Get the global value
    pub fn get(&self) -> Result<WrtValue> {
        Ok(self.0.get().clone())
    }
    
    /// Set the global value
    pub fn set(&self, value: WrtValue) -> Result<()> {
        // Since Global is behind Arc, we can't mutate it directly
        // This is a design limitation - for now return an error
        Err(crate::Error::runtime_execution_error("Runtime execution error: Cannot set global value through Arc<Global>"))
    }
    
    /// Unwrap to get the Arc<Global>
    #[must_use] pub fn into_inner(self) -> Arc<Global> {
        self.0
    }
    
    /// Get global value
    #[must_use] pub fn get_value(&self) -> &WrtValue {
        self.0.get()
    }
    
    /// Set global value (requires mutable access)
    pub fn set_value(&self, new_value: &WrtValue) -> Result<()> {
        // Note: This requires unsafe because we can't get mutable access to Arc<Global>
        // For now, we'll return an error
        Err(Error::new(
            ErrorCategory::Runtime,
            wrt_error::codes::GLOBAL_ACCESS_DENIED,
            "Cannot set global value through Arc<Global>"))
    }
    
    /// Get global value type
    #[must_use] pub fn value_type(&self) -> WrtValueType {
        self.0.global_type_descriptor().value_type
    }
    
    /// Check if global is mutable
    #[must_use] pub fn is_mutable(&self) -> bool {
        self.0.global_type_descriptor().mutable
    }
}

// Implement foundation traits for wrapper types
use wrt_foundation::traits::{ReadStream, WriteStream};
use wrt_foundation::verification::Checksum;

// TableWrapper trait implementations
impl Checksummable for TableWrapper {
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Use table size and element type for checksum
        checksum.update_slice(&self.0.size().to_le_bytes);
        checksum.update_slice(&(self.0.ty.element_type as u8).to_le_bytes);
    }
}

impl ToBytes for TableWrapper {
    fn serialized_size(&self) -> usize {
        12 // table type (4) + size (4) + limits (4)
    }
    
    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut WriteStream,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        writer.write_all(&self.0.size().to_le_bytes())?;
        writer.write_all(&(self.0.ty.element_type as u8).to_le_bytes())?;
        writer.write_all(&self.0.ty.limits.min.to_le_bytes())?;
        Ok(())
    }
}

impl FromBytes for TableWrapper {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut ReadStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 12];
        reader.read_exact(&mut bytes)?;
        
        // Create a default table (simplified implementation)
        use wrt_foundation::types::{Limits, TableType, RefType};
        let table_type = TableType {
            element_type: RefType::Funcref,
            limits: Limits { min: 0, max: Some(1) },
        };
        
        let table = Table::new(table_type).map_err(|_| {
            wrt_error::Error::runtime_execution_error("Runtime execution error: Failed to create table from bytes")
        })?;
        
        Ok(TableWrapper::new(table))
    }
}

// MemoryWrapper trait implementations  
impl Checksummable for MemoryWrapper {
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Use memory size for checksum
        checksum.update_slice(&self.0.size().to_le_bytes);
        checksum.update_slice(&self.0.size_in_bytes().to_le_bytes);
    }
}

impl ToBytes for MemoryWrapper {
    fn serialized_size(&self) -> usize {
        12 // size (4) + limits min (4) + limits max (4)
    }
    
    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut WriteStream,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        writer.write_all(&self.0.size().to_le_bytes())?;
        writer.write_all(&self.0.ty.limits.min.to_le_bytes())?;
        let max = self.0.ty.limits.max.unwrap_or(u32::MAX;
        writer.write_all(&max.to_le_bytes())?;
        Ok(())
    }
}

impl FromBytes for MemoryWrapper {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut ReadStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 12];
        reader.read_exact(&mut bytes)?;
        
        // Create a default memory (simplified implementation)
        use wrt_foundation::types::{Limits, MemoryType};
        let memory_type = MemoryType {
            limits: Limits { min: 1, max: Some(1) },
            shared: false,
        };
        
        let memory = Memory::new(to_core_memory_type(memory_type)).map_err(|_| {
            wrt_error::Error::new(wrt_error::ErrorCategory::Memory,
                wrt_error::codes::INVALID_VALUE,
                "Failed to create memory from bytes")
        })?;
        
        Ok(MemoryWrapper::new(memory))
    }
}

// Helper function to convert ValueType to u8
fn value_type_to_u8(vt: WrtValueType) -> u8 {
    match vt {
        WrtValueType::I32 => 0,
        WrtValueType::I64 => 1,
        WrtValueType::F32 => 2,
        WrtValueType::F64 => 3,
        WrtValueType::FuncRef => 4,
        WrtValueType::ExternRef => 5,
        WrtValueType::V128 => 6,
        WrtValueType::I16x8 => 7,
        WrtValueType::StructRef(_) => 8,
        _ => 255, // fallback for other types
    }
}

// GlobalWrapper trait implementations
impl Checksummable for GlobalWrapper {
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Use global value type for checksum
        checksum.update_slice(&value_type_to_u8(self.0.global_type_descriptor().value_type).to_le_bytes);
        checksum.update_slice(&u8::from(self.0.global_type_descriptor().mutable).to_le_bytes);
    }
}

impl ToBytes for GlobalWrapper {
    fn serialized_size(&self) -> usize {
        12 // value type (4) + mutable flag (4) + value (4)
    }
    
    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut WriteStream,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        writer.write_all(&value_type_to_u8(self.0.global_type_descriptor().value_type).to_le_bytes())?;
        writer.write_all(&u8::from(self.0.global_type_descriptor().mutable).to_le_bytes())?;
        // Simplified value serialization
        writer.write_all(&0u32.to_le_bytes())?;
        Ok(())
    }
}

impl FromBytes for GlobalWrapper {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut ReadStream<'_>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 12];
        reader.read_exact(&mut bytes)?;
        
        // Create a default global (simplified implementation)
        use wrt_foundation::types::ValueType;
        use wrt_foundation::values::Value;
        
        let global = Global::new(ValueType::I32, false, Value::I32(0)).map_err(|_| {
            wrt_error::Error::runtime_execution_error("Failed to create global from bytes") 
        })?;
        
        Ok(GlobalWrapper::new(global))
    }
}

// Arc<Table> trait implementations removed due to orphan rule violations.
// Use TableWrapper instead which implements these traits properly.

// Trait implementations for Arc<Memory>

// Default for Arc<Memory> removed due to orphan rules - use explicit creation instead
/*
*/


// Arc<Memory> trait implementations removed due to orphan rule violations.
// Use MemoryWrapper instead which implements these traits properly.

// Trait implementations for Arc<Global>

// Default for Arc<Global> removed due to orphan rules - use explicit creation instead


// Arc<Global> trait implementations removed due to orphan rule violations.
// Use GlobalWrapper instead which implements these traits properly.

// Ensure local `crate::module::Import` struct is defined
// Ensure local `crate::module::Export` struct is defined
// Ensure local `crate::global::Global`, `crate::table::Table`,
// `crate::memory::Memory` are defined and their `new` methods are compatible.

