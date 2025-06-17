//! Integration between component binary parser and runtime
//!
//! This module bridges the component binary format parsing and the runtime
//! execution environment, handling component loading and instantiation.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(feature = "std")]
use std::{boxed::Box, string::String, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, component::ComponentType, component_value::ComponentValue, prelude::*,
};

use crate::{
    adapter::CoreModuleAdapter,
    canonical::CanonicalAbi,
    component::Component,
    execution_engine::ComponentExecutionEngine,
    instantiation::{ImportValues, InstantiationContext},
    types::{ComponentInstance, ValType, Value},
    WrtResult,
};

/// Maximum number of parsed sections in no_std environments
const MAX_PARSED_SECTIONS: usize = 64;

/// Component binary loader and parser integration
pub struct ComponentLoader {
    /// Canonical ABI processor
    canonical_abi: CanonicalAbi,
    /// Maximum component size to load
    max_component_size: usize,
    /// Validation level
    validation_level: ValidationLevel,
}

/// Validation level for component loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// No validation (unsafe, for testing only)
    None,
    /// Basic structural validation
    Basic,
    /// Full validation including type checking
    Full,
}

/// Parsed component sections
#[derive(Debug, Clone)]
pub struct ParsedComponent {
    /// Component type definitions
    #[cfg(feature = "std")]
    pub types: Vec<ComponentType,
    #[cfg(not(any(feature = "std", )))]
    pub types: BoundedVec<ComponentType, MAX_PARSED_SECTIONS, NoStdProvider<65536>>,

    /// Component imports
    #[cfg(feature = "std")]
    pub imports: Vec<ParsedImport>,
    #[cfg(not(any(feature = "std", )))]
    pub imports: BoundedVec<ParsedImport, MAX_PARSED_SECTIONS, NoStdProvider<65536>>,

    /// Component exports
    #[cfg(feature = "std")]
    pub exports: Vec<ParsedExport>,
    #[cfg(not(any(feature = "std", )))]
    pub exports: BoundedVec<ParsedExport, MAX_PARSED_SECTIONS, NoStdProvider<65536>>,

    /// Embedded core modules
    #[cfg(feature = "std")]
    pub modules: Vec<ParsedModule>,
    #[cfg(not(any(feature = "std", )))]
    pub modules: BoundedVec<ParsedModule, 16, NoStdProvider<65536>>,

    /// Component instances
    #[cfg(feature = "std")]
    pub instances: Vec<ParsedInstance>,
    #[cfg(not(any(feature = "std", )))]
    pub instances: BoundedVec<ParsedInstance, 16, NoStdProvider<65536>>,

    /// Canonical function adapters
    #[cfg(feature = "std")]
    pub canonicals: Vec<ParsedCanonical>,
    #[cfg(not(any(feature = "std", )))]
    pub canonicals: BoundedVec<ParsedCanonical, MAX_PARSED_SECTIONS, NoStdProvider<65536>>,
}

/// Parsed import declaration
#[derive(Debug, Clone)]
pub struct ParsedImport {
    /// Import name
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, NoStdProvider<65536>>,
    /// Import type
    pub import_type: ImportKind,
}

/// Import kind enumeration
#[derive(Debug, Clone)]
pub enum ImportKind {
    /// Function import
    Function { type_index: u32 },
    /// Value import
    Value { type_index: u32 },
    /// Instance import
    Instance { type_index: u32 },
    /// Type import
    Type { bounds: TypeBounds },
}

/// Type bounds for type imports
#[derive(Debug, Clone)]
pub struct TypeBounds {
    /// Lower bound type
    pub lower: Option<u32>,
    /// Upper bound type
    pub upper: Option<u32>,
}

/// Parsed export declaration
#[derive(Debug, Clone)]
pub struct ParsedExport {
    /// Export name
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, NoStdProvider<65536>>,
    /// Export kind
    pub export_kind: ExportKind,
}

/// Export kind enumeration
#[derive(Debug, Clone)]
pub enum ExportKind {
    /// Function export
    Function { function_index: u32 },
    /// Value export
    Value { value_index: u32 },
    /// Instance export
    Instance { instance_index: u32 },
    /// Type export
    Type { type_index: u32 },
}

/// Parsed core module
#[derive(Debug, Clone)]
pub struct ParsedModule {
    /// Module index
    pub index: u32,
    /// Module binary data (simplified - would contain actual WASM bytes)
    #[cfg(feature = "std")]
    pub data: Vec<u8>,
    #[cfg(not(any(feature = "std", )))]
    pub data: BoundedVec<u8, 65536, NoStdProvider<65536>>, // 64KB max for no_std
}

/// Parsed component instance
#[derive(Debug, Clone)]
pub struct ParsedInstance {
    /// Instance index
    pub index: u32,
    /// Instantiation arguments
    #[cfg(feature = "std")]
    pub args: Vec<InstantiationArg>,
    #[cfg(not(any(feature = "std", )))]
    pub args: BoundedVec<InstantiationArg, 32, NoStdProvider<65536>>,
}

/// Instantiation argument
#[derive(Debug, Clone)]
pub struct InstantiationArg {
    /// Argument name
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, NoStdProvider<65536>>,
    /// Argument index/value
    pub index: u32,
}

/// Parsed canonical function adapter
#[derive(Debug, Clone)]
pub struct ParsedCanonical {
    /// Canonical function index
    pub index: u32,
    /// Canonical operation
    pub operation: CanonicalOperation,
}

/// Canonical operation types
#[derive(Debug, Clone)]
pub enum CanonicalOperation {
    /// Lift operation (core to component)
    Lift { core_func_index: u32, type_index: u32, options: CanonicalOptions },
    /// Lower operation (component to core)
    Lower { func_index: u32, options: CanonicalOptions },
    /// Resource new operation
    ResourceNew { resource_type: u32 },
    /// Resource drop operation
    ResourceDrop { resource_type: u32 },
    /// Resource rep operation
    ResourceRep { resource_type: u32 },
}

/// Canonical ABI options
#[derive(Debug, Clone)]
pub struct CanonicalOptions {
    /// String encoding
    pub string_encoding: Option<StringEncoding>,
    /// Memory index
    pub memory: Option<u32>,
    /// Binary std/no_std choice
    pub realloc: Option<u32>,
    /// Post-return function
    pub post_return: Option<u32>,
}

/// String encoding options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringEncoding {
    /// UTF-8 encoding
    Utf8,
    /// UTF-16 little endian
    Utf16Le,
    /// UTF-16 big endian
    Utf16Be,
    /// Latin-1 encoding
    Latin1,
}

impl ComponentLoader {
    /// Create a new component loader
    pub fn new() -> Self {
        Self {
            canonical_abi: CanonicalAbi::new(),
            max_component_size: 16 * 1024 * 1024, // 16MB default
            validation_level: ValidationLevel::Full,
        }
    }

    /// Set maximum component size
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_component_size = size;
        self
    }

    /// Set validation level
    pub fn with_validation_level(mut self, level: ValidationLevel) -> Self {
        self.validation_level = level;
        self
    }

    /// Parse component binary data
    pub fn parse_component(&self, binary_data: &[u8]) -> WrtResult<ParsedComponent> {
        // Validate size
        if binary_data.len() > self.max_component_size {
            return Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ));
        }

        // Validate basic structure
        if binary_data.len() < 8 {
            return Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ));
        }

        // Check magic bytes (simplified - would check actual WASM component magic)
        if &binary_data[0..4] != b"\x00asm" {
            return Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ));
        }

        // Parse sections (simplified implementation)
        let mut parsed = ParsedComponent::new();

        // In a real implementation, this would parse the actual binary format
        // For now, create a minimal valid component
        self.parse_sections(binary_data, &mut parsed)?;

        // Validate if required
        if self.validation_level != ValidationLevel::None {
            self.validate_component(&parsed)?;
        }

        Ok(parsed)
    }

    /// Parse component sections from binary data
    fn parse_sections(&self, _binary_data: &[u8], parsed: &mut ParsedComponent) -> WrtResult<()> {
        // Simplified section parsing - in reality would parse actual WASM component format

        // Add a default type
        parsed.add_type(ComponentType::Unit)?;

        // Add a default import
        #[cfg(feature = "std")]
        let import_name = "default".to_string();
        #[cfg(not(any(feature = "std", )))]
        let import_name = BoundedString::from_str("default")
            .map_err(|_| wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ))?;

        parsed.add_import(ParsedImport {
            name: import_name,
            import_type: ImportKind::Function { type_index: 0 },
        })?;

        // Add a default export
        #[cfg(feature = "std")]
        let export_name = "main".to_string();
        #[cfg(not(any(feature = "std", )))]
        let export_name = BoundedString::from_str("main")
            .map_err(|_| wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ))?;

        parsed.add_export(ParsedExport {
            name: export_name,
            export_kind: ExportKind::Function { function_index: 0 },
        })?;

        Ok(())
    }

    /// Validate parsed component
    fn validate_component(&self, parsed: &ParsedComponent) -> WrtResult<()> {
        if self.validation_level == ValidationLevel::Basic {
            // Basic validation - check we have at least some content
            if parsed.types.len() == 0 {
                return Err(wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Validation,
                    wrt_error::codes::VALIDATION_ERROR,
                    "Component must have at least one type"
                ));
            }
        } else if self.validation_level == ValidationLevel::Full {
            // Full validation - check type consistency
            self.validate_type_consistency(parsed)?;
            self.validate_import_export_consistency(parsed)?;
        }

        Ok(())
    }

    /// Validate type consistency
    fn validate_type_consistency(&self, _parsed: &ParsedComponent) -> WrtResult<()> {
        // In a full implementation, would validate:
        // - All type references are valid
        // - Function signatures are consistent
        // - Resource types are properly defined
        Ok(())
    }

    /// Validate import/export consistency
    fn validate_import_export_consistency(&self, _parsed: &ParsedComponent) -> WrtResult<()> {
        // In a full implementation, would validate:
        // - All import types are resolvable
        // - Export types match internal definitions
        // - No circular dependencies
        Ok(())
    }

    /// Convert parsed component to runtime component
    pub fn to_runtime_component(&self, parsed: &ParsedComponent) -> WrtResult<Component> {
        let mut component = Component::new(WrtComponentType::default());

        // Convert types
        for component_type in &parsed.types {
            component.add_type(component_type.clone())?;
        }

        // Convert imports
        for import in &parsed.imports {
            self.convert_import(&mut component, import)?;
        }

        // Convert exports
        for export in &parsed.exports {
            self.convert_export(&mut component, export)?;
        }

        // Convert modules to adapters
        for module in &parsed.modules {
            let adapter = self.create_module_adapter(module)?;
            component.add_module_adapter(adapter)?;
        }

        Ok(component)
    }

    /// Convert parsed import to runtime import
    fn convert_import(&self, component: &mut Component, import: &ParsedImport) -> WrtResult<()> {
        match &import.import_type {
            ImportKind::Function { type_index } => {
                component.add_function_import(&import.name, *type_index)?;
            }
            ImportKind::Value { type_index } => {
                component.add_value_import(&import.name, *type_index)?;
            }
            ImportKind::Instance { type_index } => {
                component.add_instance_import(&import.name, *type_index)?;
            }
            ImportKind::Type { bounds: _ } => {
                component.add_type_import(&import.name)?;
            }
        }
        Ok(())
    }

    /// Convert parsed export to runtime export
    fn convert_export(&self, component: &mut Component, export: &ParsedExport) -> WrtResult<()> {
        match &export.export_kind {
            ExportKind::Function { function_index } => {
                component.add_function_export(&export.name, *function_index)?;
            }
            ExportKind::Value { value_index } => {
                component.add_value_export(&export.name, *value_index)?;
            }
            ExportKind::Instance { instance_index } => {
                component.add_instance_export(&export.name, *instance_index)?;
            }
            ExportKind::Type { type_index } => {
                component.add_type_export(&export.name, *type_index)?;
            }
        }
        Ok(())
    }

    /// Create module adapter from parsed module
    fn create_module_adapter(&self, module: &ParsedModule) -> WrtResult<CoreModuleAdapter> {
        #[cfg(feature = "std")]
        let name = "Component not found";
        #[cfg(not(any(feature = "std", )))]
        let name = BoundedString::from_str("module")
            .map_err(|_| wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ))?;

        let adapter = CoreModuleAdapter::new(name);

        // In a real implementation, would parse the module binary
        // and create appropriate function/memory/table/global adapters

        Ok(adapter)
    }

    /// Load and instantiate component from binary
    pub fn load_and_instantiate(
        &self,
        binary_data: &[u8],
        imports: &ImportValues,
        context: &mut InstantiationContext,
    ) -> WrtResult<ComponentInstance> {
        // Parse the component
        let parsed = self.parse_component(binary_data)?;

        // Convert to runtime component
        let component = self.to_runtime_component(&parsed)?;

        // Instantiate the component
        component.instantiate(imports, context)
    }
}

impl ParsedComponent {
    /// Create a new empty parsed component
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            types: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            types: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            imports: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            exports: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            exports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            modules: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            modules: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            instances: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            instances: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            canonicals: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            canonicals: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        }
    }

    /// Add a type to the component
    pub fn add_type(&mut self, component_type: ComponentType) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.types.push(component_type);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.types
                .push(component_type)
                .map_err(|_| wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many types"
                ))
        }
    }

    /// Add an import to the component
    pub fn add_import(&mut self, import: ParsedImport) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.imports.push(import);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.imports
                .push(import)
                .map_err(|_| wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many imports"
                ))
        }
    }

    /// Add an export to the component
    pub fn add_export(&mut self, export: ParsedExport) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.exports.push(export);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.exports
                .push(export)
                .map_err(|_| wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many exports"
                ))
        }
    }
}

impl Default for ComponentLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ParsedComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CanonicalOptions {
    fn default() -> Self {
        Self {
            string_encoding: Some(StringEncoding::Utf8),
            memory: None,
            realloc: None,
            post_return: None,
        }
    }
}

impl fmt::Display for ValidationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationLevel::None => write!(f, "none"),
            ValidationLevel::Basic => write!(f, "basic"),
            ValidationLevel::Full => write!(f, "full"),
        }
    }
}

impl fmt::Display for StringEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringEncoding::Utf8 => write!(f, "utf8"),
            StringEncoding::Utf16Le => write!(f, "utf16le"),
            StringEncoding::Utf16Be => write!(f, "utf16be"),
            StringEncoding::Latin1 => write!(f, "latin1"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_loader_creation() {
        let loader = ComponentLoader::new();
        assert_eq!(loader.validation_level, ValidationLevel::Full);
        assert_eq!(loader.max_component_size, 16 * 1024 * 1024);
    }

    #[test]
    fn test_component_loader_configuration() {
        let loader = ComponentLoader::new()
            .with_max_size(1024)
            .with_validation_level(ValidationLevel::Basic);

        assert_eq!(loader.max_component_size, 1024);
        assert_eq!(loader.validation_level, ValidationLevel::Basic);
    }

    #[test]
    fn test_parsed_component_creation() {
        let mut component = ParsedComponent::new();
        assert_eq!(component.types.len(), 0);
        assert_eq!(component.imports.len(), 0);
        assert_eq!(component.exports.len(), 0);

        // Test adding components
        assert!(component.add_type(ComponentType::Unit).is_ok());
        assert_eq!(component.types.len(), 1);
    }

    #[test]
    fn test_validation_level_display() {
        assert_eq!(ValidationLevel::None.to_string(), "none");
        assert_eq!(ValidationLevel::Basic.to_string(), "basic");
        assert_eq!(ValidationLevel::Full.to_string(), "full");
    }

    #[test]
    fn test_string_encoding_display() {
        assert_eq!(StringEncoding::Utf8.to_string(), "utf8");
        assert_eq!(StringEncoding::Utf16Le.to_string(), "utf16le");
        assert_eq!(StringEncoding::Latin1.to_string(), "latin1");
    }

    #[test]
    fn test_canonical_options_default() {
        let options = CanonicalOptions::default();
        assert_eq!(options.string_encoding, Some(StringEncoding::Utf8));
        assert_eq!(options.memory, None);
        assert_eq!(options.realloc, None);
        assert_eq!(options.post_return, None);
    }

    #[test]
    fn test_parse_invalid_component() {
        let loader = ComponentLoader::new();

        // Test empty binary
        let result = loader.parse_component(&[]);
        assert!(result.is_err());

        // Test invalid magic
        let result = loader.parse_component(b"invalid_magic_bytes");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_minimal_component() {
        let loader = ComponentLoader::new();

        // Create minimal valid component binary (simplified)
        let binary = b"\x00asm\x0d\x00\x01\x00"; // Magic + version
        let result = loader.parse_component(binary);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert!(parsed.types.len() > 0);
    }
}
