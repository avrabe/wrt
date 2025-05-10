//! Import implementation for the WebAssembly Component Model.
//!
//! This module provides the Import type for component imports.

use crate::component::ExternValue;
use crate::namespace::Namespace;
use crate::prelude::*;
use crate::type_conversion::bidirectional;
use wrt_format::component::ExternType;
use wrt_types::ExternType as RuntimeExternType;

/// Import to a component
#[derive(Debug, Clone)]
pub struct Import {
    /// Import namespace
    pub namespace: Namespace,
    /// Import name
    pub name: String,
    /// Import type
    pub ty: ExternType,
    /// Import value (runtime representation)
    pub value: ExternValue,
}

impl Import {
    /// Creates a new import
    pub fn new(namespace: Namespace, name: String, ty: ExternType, value: ExternValue) -> Self {
        Self { namespace, name, ty, value }
    }

    /// Creates an import identifier by combining namespace and name
    pub fn identifier(&self) -> String {
        let ns_str = self.namespace.to_string();
        if ns_str.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", ns_str, self.name)
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
    use super::*;
    use crate::component::{ExternValue, FunctionValue};
    use wrt_format::component::{ExternType, ValType};
    use wrt_runtime::func::FuncType;

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
                    params: vec![wrt_types::types::ValueType::I32],
                    results: vec![wrt_types::types::ValueType::I32],
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
                ty: FuncType { params: vec![wrt_types::types::ValueType::I32], results: vec![] },
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
                    params: vec![wrt_types::types::ValueType::I32],
                    results: vec![wrt_types::types::ValueType::I32],
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
                    params: vec![wrt_types::types::ValueType::I32],
                    results: vec![wrt_types::types::ValueType::I32],
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
