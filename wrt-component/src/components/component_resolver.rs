//! Component import/export resolution
//!
//! This module provides functionality for resolving imports and exports
//! during component instantiation and linking.

#[cfg(not(feature = "std"))]
use alloc::{
    collections::BTreeMap,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    collections::BTreeMap,
    vec::Vec,
};

use wrt_foundation::{
    bounded::{
        BoundedString,
        BoundedVec,
        MAX_GENERATIVE_TYPES,
    },
    budget_aware_provider::CrateId,
    prelude::*,
    safe_managed_alloc,
    safe_memory::NoStdProvider,
};

use crate::{
    generative_types::GenerativeTypeRegistry,
    type_bounds::{
        RelationKind,
        TypeBoundsChecker,
    },
    types::{
        ComponentError,
        ComponentInstanceId,
        TypeId,
        ValType,
    },
};

/// Import resolution result
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// Import name
    pub name:     BoundedString<64, NoStdProvider<65536>>,
    /// Resolved value
    pub value:    ImportValue,
    /// Type information
    pub val_type: Option<ValType>,
}

/// Export resolution result
#[derive(Debug, Clone)]
pub struct ResolvedExport {
    /// Export name
    pub name:     BoundedString<64, NoStdProvider<65536>>,
    /// Resolved value
    pub value:    ExportValue,
    /// Type information
    pub val_type: Option<ValType>,
}

/// Import value types
#[derive(Debug, Clone)]
pub enum ImportValue {
    /// Function import
    Function { type_id: TypeId, func_ref: u32 },
    /// Global import
    Global { type_id: TypeId, global_ref: u32 },
    /// Memory import
    Memory {
        min_pages: u32,
        max_pages: Option<u32>,
    },
    /// Table import
    Table {
        min_size: u32,
        max_size: Option<u32>,
    },
    /// Instance import
    Instance { instance_id: ComponentInstanceId },
    /// Value import
    Value {
        val_type: ValType,
        value:    ComponentValue,
    },
}

/// Export value types
#[derive(Debug, Clone)]
pub enum ExportValue {
    /// Function export
    Function { type_id: TypeId, func_ref: u32 },
    /// Global export
    Global { type_id: TypeId, global_ref: u32 },
    /// Memory export
    Memory { memory_ref: u32 },
    /// Table export
    Table { table_ref: u32 },
    /// Instance export
    Instance { instance_id: ComponentInstanceId },
    /// Value export
    Value {
        val_type: ValType,
        value:    ComponentValue,
    },
}

/// Component value for imports/exports
#[derive(Debug, Clone)]
pub enum ComponentValue {
    /// Boolean value
    Bool(bool),
    /// Integer values
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    S8(i8),
    S16(i16),
    S32(i32),
    S64(i64),
    /// Float values
    F32(f32),
    F64(f64),
    /// String value
    String(BoundedString<256, NoStdProvider<65536>>),
    /// List value
    List(BoundedVec<ComponentValue, 64, NoStdProvider<65536>>),
    /// Record value
    Record(BTreeMap<BoundedString<32, NoStdProvider<65536>>, ComponentValue>),
    /// Variant value
    Variant {
        discriminant: u32,
        value:        Option<Box<ComponentValue>>,
    },
    /// Option value
    Option(Option<Box<ComponentValue>>),
    /// Result value
    Result(core::result::Result<Box<ComponentValue>, Box<ComponentValue>>),
}

/// Component resolver for import/export resolution
#[derive(Debug)]
pub struct ComponentResolver {
    /// Type registry
    type_registry:  GenerativeTypeRegistry,
    /// Type bounds checker
    bounds_checker: TypeBoundsChecker,
    /// Import resolution cache
    import_cache:
        BTreeMap<(ComponentInstanceId, BoundedString<64, NoStdProvider<65536>>), ResolvedImport>,
    /// Export resolution cache
    export_cache:
        BTreeMap<(ComponentInstanceId, BoundedString<64, NoStdProvider<65536>>), ResolvedExport>,
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self {
            type_registry:  GenerativeTypeRegistry::new(),
            bounds_checker: TypeBoundsChecker::new(),
            import_cache:   BTreeMap::new(),
            export_cache:   BTreeMap::new(),
        }
    }

    /// Resolve an import for a component instance
    pub fn resolve_import(
        &mut self,
        instance_id: ComponentInstanceId,
        import_name: BoundedString<64, NoStdProvider<65536>>,
        provided_value: ImportValue,
    ) -> core::result::Result<ResolvedImport, ComponentError> {
        // Check cache first
        let cache_key = (instance_id, import_name.clone());
        if let Some(cached) = self.import_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Validate import type compatibility
        let val_type = self.get_import_type(&provided_value)?;

        let resolved = ResolvedImport {
            name: import_name,
            value: provided_value,
            val_type,
        };

        self.import_cache.insert(cache_key, resolved.clone());
        Ok(resolved)
    }

    /// Resolve an export from a component instance
    pub fn resolve_export(
        &mut self,
        instance_id: ComponentInstanceId,
        export_name: BoundedString<64, NoStdProvider<65536>>,
        export_value: ExportValue,
    ) -> core::result::Result<ResolvedExport, ComponentError> {
        // Check cache first
        let cache_key = (instance_id, export_name.clone());
        if let Some(cached) = self.export_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Validate export type
        let val_type = self.get_export_type(&export_value)?;

        let resolved = ResolvedExport {
            name: export_name,
            value: export_value,
            val_type,
        };

        self.export_cache.insert(cache_key, resolved.clone());
        Ok(resolved)
    }

    /// Check if an import can be satisfied by an export
    pub fn can_satisfy_import(
        &mut self,
        import: &ResolvedImport,
        export: &ResolvedExport,
    ) -> core::result::Result<bool, ComponentError> {
        match (&import.value, &export.value) {
            (
                ImportValue::Function {
                    type_id: import_type,
                    ..
                },
                ExportValue::Function {
                    type_id: export_type,
                    ..
                },
            ) => self.check_type_compatibility(*import_type, *export_type),
            (
                ImportValue::Global {
                    type_id: import_type,
                    ..
                },
                ExportValue::Global {
                    type_id: export_type,
                    ..
                },
            ) => self.check_type_compatibility(*import_type, *export_type),
            (
                ImportValue::Memory {
                    min_pages: import_min,
                    max_pages: import_max,
                },
                ExportValue::Memory { .. },
            ) => {
                // Memory compatibility checks would go here
                Ok(true)
            },
            (
                ImportValue::Table {
                    min_size: import_min,
                    max_size: import_max,
                },
                ExportValue::Table { .. },
            ) => {
                // Table compatibility checks would go here
                Ok(true)
            },
            (ImportValue::Instance { .. }, ExportValue::Instance { .. }) => {
                // Instance compatibility would check all nested imports/exports
                Ok(true)
            },
            (
                ImportValue::Value {
                    val_type: import_type,
                    ..
                },
                ExportValue::Value {
                    val_type: export_type,
                    ..
                },
            ) => Ok(self.are_types_compatible(import_type, export_type)),
            _ => Ok(false), // Different kinds of imports/exports
        }
    }

    /// Check type compatibility using the bounds checker
    fn check_type_compatibility(
        &mut self,
        import_type: TypeId,
        export_type: TypeId,
    ) -> core::result::Result<bool, ComponentError> {
        // Check if export type is a subtype of import type
        let result = self.bounds_checker.check_type_bound(
            export_type,
            import_type,
            crate::generative_types::BoundKind::Sub,
        );

        Ok(result == crate::type_bounds::RelationResult::Satisfied)
    }

    /// Check if two ValTypes are compatible
    fn are_types_compatible(&self, import_type: &ValType, export_type: &ValType) -> bool {
        match (import_type, export_type) {
            // Exact matches
            (ValType::Bool, ValType::Bool) => true,
            (ValType::U8, ValType::U8) => true,
            (ValType::U16, ValType::U16) => true,
            (ValType::U32, ValType::U32) => true,
            (ValType::U64, ValType::U64) => true,
            (ValType::S8, ValType::S8) => true,
            (ValType::S16, ValType::S16) => true,
            (ValType::S32, ValType::S32) => true,
            (ValType::S64, ValType::S64) => true,
            (ValType::F32, ValType::F32) => true,
            (ValType::F64, ValType::F64) => true,
            (ValType::Char, ValType::Char) => true,
            (ValType::String, ValType::String) => true,

            // Structural types need deep comparison
            (ValType::List(t1), ValType::List(t2)) => self.are_types_compatible(t1, t2),
            (ValType::Option(t1), ValType::Option(t2)) => self.are_types_compatible(t1, t2),

            // TODO: Add more structural type comparisons
            _ => false,
        }
    }

    /// Get the type of an import value
    fn get_import_type(
        &self,
        import: &ImportValue,
    ) -> core::result::Result<Option<ValType>, ComponentError> {
        match import {
            ImportValue::Function { .. } => Ok(None), // Function types are handled separately
            ImportValue::Global { .. } => Ok(None),   // Global types are handled separately
            ImportValue::Memory { .. } => Ok(None),
            ImportValue::Table { .. } => Ok(None),
            ImportValue::Instance { .. } => Ok(None),
            ImportValue::Value { val_type, .. } => Ok(Some(val_type.clone())),
        }
    }

    /// Get the type of an export value
    fn get_export_type(
        &self,
        export: &ExportValue,
    ) -> core::result::Result<Option<ValType>, ComponentError> {
        match export {
            ExportValue::Function { .. } => Ok(None), // Function types are handled separately
            ExportValue::Global { .. } => Ok(None),   // Global types are handled separately
            ExportValue::Memory { .. } => Ok(None),
            ExportValue::Table { .. } => Ok(None),
            ExportValue::Instance { .. } => Ok(None),
            ExportValue::Value { val_type, .. } => Ok(Some(val_type.clone())),
        }
    }

    /// Clear resolution caches
    pub fn clear_caches(&mut self) {
        self.import_cache.clear();
        self.export_cache.clear();
    }

    /// Get type registry reference
    pub fn type_registry(&self) -> &GenerativeTypeRegistry {
        &self.type_registry
    }

    /// Get type registry mutable reference
    pub fn type_registry_mut(&mut self) -> &mut GenerativeTypeRegistry {
        &mut self.type_registry
    }

    /// Get bounds checker reference
    pub fn bounds_checker(&self) -> &TypeBoundsChecker {
        &self.bounds_checker
    }

    /// Get bounds checker mutable reference
    pub fn bounds_checker_mut(&mut self) -> &mut TypeBoundsChecker {
        &mut self.bounds_checker
    }
}

impl Default for ComponentResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Import resolution helper
#[derive(Debug, Clone)]
pub struct ImportResolution {
    /// Import name
    pub name:           BoundedString<64, NoStdProvider<65536>>,
    /// Instance ID
    pub instance_id:    ComponentInstanceId,
    /// Resolved value
    pub resolved_value: ComponentValue,
}

/// Export resolution helper
#[derive(Debug, Clone)]
pub struct ExportResolution {
    /// Export name
    pub name:           BoundedString<64, NoStdProvider<65536>>,
    /// Instance ID
    pub instance_id:    ComponentInstanceId,
    /// Exported value
    pub exported_value: ComponentValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = ComponentResolver::new();
        assert_eq!(resolver.import_cache.len(), 0);
        assert_eq!(resolver.export_cache.len(), 0);
    }

    #[test]
    fn test_import_resolution() {
        let mut resolver = ComponentResolver::new();
        let instance_id = ComponentInstanceId(1);
        let import_name = BoundedString::from_str("test_import").unwrap();

        let import_value = ImportValue::Value {
            val_type: ValType::U32,
            value:    ComponentValue::U32(42),
        };

        let result = resolver.resolve_import(instance_id, import_name.clone(), import_value);
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved.name, import_name);
    }

    #[test]
    fn test_export_resolution() {
        let mut resolver = ComponentResolver::new();
        let instance_id = ComponentInstanceId(1);
        let export_name = BoundedString::from_str("test_export").unwrap();

        let export_value = ExportValue::Value {
            val_type: ValType::String,
            value:    ComponentValue::String(BoundedString::from_str("hello").unwrap()),
        };

        let result = resolver.resolve_export(instance_id, export_name.clone(), export_value);
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved.name, export_name);
    }

    #[test]
    fn test_import_export_compatibility() {
        let mut resolver = ComponentResolver::new();

        // Create matching import and export
        let import = ResolvedImport {
            name:     BoundedString::from_str("test").unwrap(),
            value:    ImportValue::Value {
                val_type: ValType::U32,
                value:    ComponentValue::U32(0),
            },
            val_type: Some(ValType::U32),
        };

        let export = ResolvedExport {
            name:     BoundedString::from_str("test").unwrap(),
            value:    ExportValue::Value {
                val_type: ValType::U32,
                value:    ComponentValue::U32(42),
            },
            val_type: Some(ValType::U32),
        };

        let result = resolver.can_satisfy_import(&import, &export);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_type_compatibility() {
        let resolver = ComponentResolver::new();

        // Test primitive type compatibility
        assert!(resolver.are_types_compatible(&ValType::Bool, &ValType::Bool));
        assert!(resolver.are_types_compatible(&ValType::U32, &ValType::U32));
        assert!(!resolver.are_types_compatible(&ValType::U32, &ValType::U64));

        // Test structural type compatibility
        let list_u32 = ValType::List(Box::new(ValType::U32));
        let list_u64 = ValType::List(Box::new(ValType::U64));
        assert!(resolver.are_types_compatible(&list_u32, &list_u32));
        assert!(!resolver.are_types_compatible(&list_u32, &list_u64));
    }
}

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{
    Checksummable,
    FromBytes,
    ReadStream,
    ToBytes,
    WriteStream,
};

// Macro to implement basic traits for complex types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                0u32.update_checksum(checksum);
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                _writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
                Ok(())
            }
        }

        impl FromBytes for $type {
            fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                _reader: &mut ReadStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<Self> {
                Ok($default_val)
            }
        }
    };
}

// Default implementations for ComponentValue in this module
impl Default for ComponentValue {
    fn default() -> Self {
        Self::Bool(false)
    }
}

impl ImportResolution {
    pub fn new() -> Result<Self, ComponentError> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)
            .map_err(|_| ComponentError::AllocationFailed)?;
        let name = BoundedString::new(provider).map_err(|_| ComponentError::AllocationFailed)?;

        Ok(Self {
            name,
            instance_id: ComponentInstanceId(0),
            resolved_value: ComponentValue::default(),
        })
    }
}

impl Default for ImportResolution {
    fn default() -> Self {
        Self::new().expect("ImportResolution allocation should not fail in default construction")
    }
}

impl ExportResolution {
    pub fn new() -> Result<Self, ComponentError> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)
            .map_err(|_| ComponentError::AllocationFailed)?;
        let name = BoundedString::new(provider).map_err(|_| ComponentError::AllocationFailed)?;

        Ok(Self {
            name,
            instance_id: ComponentInstanceId(0),
            exported_value: ComponentValue::default(),
        })
    }
}

impl Default for ExportResolution {
    fn default() -> Self {
        Self::new().expect("ExportResolution allocation should not fail in default construction")
    }
}

// Apply macro to types that need traits
// Note: ComponentValue traits are implemented in wrt-foundation
