//! Export implementation for the WebAssembly Component Model.
//!
//! This module provides the Export type for component exports.

use wrt_format::component::ExternType;
use wrt_foundation::ExternType as RuntimeExternType;

use crate::{components::component::ExternValue, prelude::*, type_conversion::bidirectional};

/// Export from a component
#[derive(Debug, Clone)]
pub struct Export {
    /// Export name
    pub name: String,
    /// Export type
    pub ty: ExternType,
    /// Export value
    pub value: ExternValue,
    /// Export attributes
    pub attributes: HashMap<String, String>,
    /// Integrity hash for the export (if available)
    pub integrity_hash: Option<String>,
}

impl Export {
    /// Create a new export
    pub fn new(name: String, ty: ExternType, value: ExternValue) -> Self {
        Self { name, ty, value, attributes: HashMap::new(), integrity_hash: None }
    }

    /// Create a new export with attributes
    pub fn new_with_attributes(
        name: String,
        ty: ExternType,
        value: ExternValue,
        attributes: HashMap<String, String>,
    ) -> Self {
        Self { name, ty, value, attributes, integrity_hash: None }
    }

    /// Create a new export with integrity hash
    pub fn new_with_integrity(
        name: String,
        ty: ExternType,
        value: ExternValue,
        integrity_hash: String,
    ) -> Self {
        Self { name, ty, value, attributes: HashMap::new(), integrity_hash: Some(integrity_hash) }
    }

    /// Check if the export has a specific attribute
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    /// Get the value of an attribute
    pub fn get_attribute(&self, name: &str) -> Option<&String> {
        self.attributes.get(name)
    }

    /// Set an attribute value
    pub fn set_attribute(&mut self, name: String, value: String) {
        self.attributes.insert(name, value;
    }

    /// Convert the export type to a runtime extern type
    pub fn to_runtime_type(&self) -> Result<RuntimeExternType> {
        bidirectional::format_to_runtime_extern_type(&self.ty)
    }

    /// Create a new export from a RuntimeExternType
    pub fn from_runtime_type(
        name: String,
        ty: RuntimeExternType,
        value: ExternValue,
    ) -> Result<Self> {
        let format_type = bidirectional::runtime_to_format_extern_type(&ty)?;
        Ok(Self::new(name, format_type, value))
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use wrt_format::component::{ExternType, ValType};

    use super::*;
    use crate::component::{ExternValue, FunctionValue};

    #[test]
    fn test_export_creation() {
        let func_type = ExternType::Function {
            params: vec![("a".to_string(), ValType::S32), ("b".to_string(), ValType::S32)],
            results: vec![ValType::S32],
        };

        let func_value = FunctionValue { ty: func_type.clone(), export_name: "add".to_string() };

        let export = Export {
            name: "add".to_string(),
            ty: func_type,
            value: ExternValue::Function(func_value),
            attributes: HashMap::new(),
            integrity_hash: None,
        };

        assert_eq!(export.name, "add";
        match &export.ty {
            ExternType::Function { params, results } => {
                assert_eq!(params.len(), 2;
                assert_eq!(results.len(), 1;
            }
            _ => panic!("Expected function type"),
        }
    }
}
