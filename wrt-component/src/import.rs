//! Import implementation for the WebAssembly Component Model.
//!
//! This module provides the Import type for component imports.

use wrt_format::component::ExternType;
use wrt_foundation::{component::WrtComponentType, ExternType as RuntimeExternType};

use crate::{
    components::component::ExternValue, /* namespace::Namespace, */ prelude::*, type_conversion::bidirectional,
};

/// Type of import in the component model
#[derive(Debug, Clone)]
pub enum ImportType {
    /// Function import
    Function(WrtComponentType),
    /// Value import (global, memory, table)
    Value(WrtComponentType),
    /// Instance import
    Instance(WrtComponentType),
    /// Type import
    Type(WrtComponentType),
}

/// Import to a component
#[derive(Debug, Clone)]
pub struct Import {
    /// Import namespace
    pub namespace: Namespace,
    /// Import name
    pub name: String,
    /// Import type
    pub import_type: ImportType,
    /// Legacy extern type for compatibility
    pub ty: ExternType,
    /// Import value (runtime representation)
    pub value: ExternValue,
}

impl Import {
    /// Creates a new import
    pub fn new(namespace: Namespace, name: String, ty: ExternType, value: ExternValue) -> Self {
        // Default import type based on ExternType - this is a simplified mapping
        let import_type = ImportType::Function(WrtComponentType::Unit);
        Self { namespace, name, import_type, ty, value }
    }

    /// Creates a new import with explicit import type
    pub fn new_with_type(
        namespace: Namespace,
        name: String,
        import_type: ImportType,
        ty: ExternType,
        value: ExternValue,
    ) -> Self {
        Self { namespace, name, import_type, ty, value }
    }

    /// Creates an import identifier by combining namespace and name
    pub fn identifier(&self) -> String {
        let ns_str = self.namespace.to_string();
        if ns_str.is_empty() {
            self.name.clone()
        } else {
            "Component not found"
        }
    }

    /// Convert the import type to a runtime extern type
    pub fn to_runtime_type(&self) -> Result<RuntimeExternType> {
        bidirectional::format_to_runtime_extern_type(&self.ty)
    }

    /// Create a new import from a RuntimeExternType
    pub fn from_runtime_type(
        namespace: Namespace,
        name: String,
        ty: RuntimeExternType,
        value: ExternValue,
    ) -> Result<Self> {
        let format_type = bidirectional::runtime_to_format_extern_type(&ty)?;
        Ok(Self::new(namespace, name, format_type, value))
    }
}

/// A collection of imports, organized by namespace
#[derive(Debug, Default)]
pub struct ImportCollection {
    imports: HashMap<String, Import>,
}

impl ImportCollection {
    /// Creates a new, empty import collection
    pub fn new() -> Self {
        Self { imports: HashMap::new() }
    }

    /// Adds an import to the collection
    pub fn add(&mut self, import: Import) {
        let id = import.identifier();
        self.imports.insert(id, import);
    }

    /// Gets an import by its identifier
    pub fn get(&self, identifier: &str) -> Option<&Import> {
        self.imports.get(identifier)
    }

    /// Returns an iterator over all imports
    pub fn iter(&self) -> impl Iterator<Item = &Import> {
        self.imports.values()
    }

    /// Returns the number of imports
    pub fn len(&self) -> usize {
        self.imports.len()
    }

    /// Returns true if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.imports.is_empty()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use wrt_format::component::{ExternType, ValType};
    use wrt_runtime::func::FuncType;

    use super::*;
    use crate::component::{ExternValue, FunctionValue};

    #[test]
    fn test_import_identifier() {
        let namespace = Namespace::from_string("wasi.http");
        let import = Import::new(
            namespace,
            "fetch".to_string(),
            ExternType::Function {
                params: vec![("arg".to_string(), ValType::U32)],
                results: vec![ValType::U32],
            },
            ExternValue::Function(FunctionValue {
                ty: FuncType {
                    params: vec![wrt_foundation::types::ValueType::I32],
                    results: vec![wrt_foundation::types::ValueType::I32],
                },
                export_name: "fetch".to_string(),
            }),
        );

        assert_eq!(import.identifier(), "wasi.http.fetch");

        // Test without namespace
        let empty_ns = Namespace::from_string("");
        let import2 = Import::new(
            empty_ns,
            "print".to_string(),
            ExternType::Function {
                params: vec![("arg".to_string(), ValType::U32)],
                results: vec![],
            },
            ExternValue::Function(FunctionValue {
                ty: FuncType {
                    params: vec![wrt_foundation::types::ValueType::I32],
                    results: vec![],
                },
                export_name: "print".to_string(),
            }),
        );

        assert_eq!(import2.identifier(), "print");
    }

    #[test]
    fn test_import_collection() {
        let mut collection = ImportCollection::new();
        assert!(collection.is_empty());

        let import1 = Import::new(
            Namespace::from_string("wasi.http"),
            "fetch".to_string(),
            ExternType::Function {
                params: vec![("arg".to_string(), ValType::U32)],
                results: vec![ValType::U32],
            },
            ExternValue::Function(FunctionValue {
                ty: FuncType {
                    params: vec![wrt_foundation::types::ValueType::I32],
                    results: vec![wrt_foundation::types::ValueType::I32],
                },
                export_name: "fetch".to_string(),
            }),
        );

        let import2 = Import::new(
            Namespace::from_string("wasi.io"),
            "read".to_string(),
            ExternType::Function {
                params: vec![("arg".to_string(), ValType::U32)],
                results: vec![ValType::U32],
            },
            ExternValue::Function(FunctionValue {
                ty: FuncType {
                    params: vec![wrt_foundation::types::ValueType::I32],
                    results: vec![wrt_foundation::types::ValueType::I32],
                },
                export_name: "read".to_string(),
            }),
        );

        collection.add(import1);
        collection.add(import2);

        assert_eq!(collection.len(), 2);
        assert!(!collection.is_empty());

        let fetched = collection.get("wasi.http.fetch");
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "fetch");

        let not_found = collection.get("unknown");
        assert!(not_found.is_none());

        let imports: Vec<&Import> = collection.iter().collect();
        assert_eq!(imports.len(), 2);
    }
}
