//! Instance implementation for the WebAssembly Component Model.
//!
//! This module provides the instance types for component instances.

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use wrt_format::component::ComponentTypeDefinition;

use crate::export::Export;

/// Represents an instance value in a component
#[derive(Debug)]
pub struct InstanceValue {
    /// The name of the instance
    pub name: String,
    /// Instance type
    pub ty: ComponentTypeDefinition,
    /// Instance exports
    pub exports: Vec<Export>,
}

impl InstanceValue {
    /// Creates a new instance value
    pub fn new(name: String, ty: ComponentTypeDefinition, exports: Vec<Export>) -> Self {
        Self { name, ty, exports }
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Option<&Export> {
        self.exports.iter().find(|export| export.name == name)
    }

    /// Gets a mutable export by name
    pub fn get_export_mut(&mut self, name: &str) -> Option<&mut Export> {
        self.exports.iter_mut().find(|export| export.name == name)
    }
}

#[cfg(test)]
mod tests {
    use wrt::types::FuncType;
    use wrt_format::component::{ExternType, ValType};

    use super::*;
    use crate::component::{ExternValue, FunctionValue};

    #[test]
    fn test_instance_value() {
        let func_type = FuncType {
            params: vec![wrt::types::ValueType::I32, wrt::types::ValueType::I32],
            results: vec![wrt::types::ValueType::I32],
        };

        let func_value = FunctionValue { ty: func_type.clone(), export_name: "add".to_string() };

        let component_func_type = ExternType::Function {
            params: vec![("a".to_string(), ValType::S32), ("b".to_string(), ValType::S32)],
            results: vec![ValType::S32],
        };

        let export = Export {
            name: "add".to_string(),
            ty: component_func_type.clone(),
            value: ExternValue::Function(func_value),
        };

        let instance_type = ComponentTypeDefinition::Instance {
            exports: vec![("add".to_string(), component_func_type)],
        };

        let instance = InstanceValue::new("math".to_string(), instance_type, vec![export];

        assert_eq!(instance.name, "math";
        assert_eq!(instance.exports.len(), 1);

        let export = instance.get_export("add";
        assert!(export.is_some();
        assert_eq!(export.unwrap().name, "add";

        let not_found = instance.get_export("non_existent";
        assert!(not_found.is_none();
    }
}
