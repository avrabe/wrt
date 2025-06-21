// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! No-std compatible instance implementation for the WebAssembly Component
//! Model.
//!
//! This module provides the instance types for component instances in a no_std
//! environment.

use wrt_format::component::ComponentTypeDefinition;
use wrt_foundation::bounded::{BoundedString, BoundedVec, MAX_WASM_NAME_LENGTH};

use crate::{export::Export, prelude::*};

/// Maximum number of exports per instance
pub const MAX_INSTANCE_EXPORTS: usize = 64;

/// No-std compatible representation of an instance value in a component
#[derive(Debug)]
pub struct InstanceValue {
    /// The name of the instance
    pub name: BoundedString<MAX_WASM_NAME_LENGTH>,
    /// Instance type
    pub ty: ComponentTypeDefinition,
    /// Instance exports
    pub exports: BoundedVec<Export, MAX_INSTANCE_EXPORTS, NoStdProvider<65536>>,
}

impl InstanceValue {
    /// Creates a new instance value
    pub fn new(name: &str, ty: ComponentTypeDefinition, exports: &[Export]) -> Result<Self> {
        let bounded_name = BoundedString::from_str(name).map_err(|_| {
            Error::new(ErrorCategory::Parameter, codes::VALIDATION_ERROR, "Instance name too long")
        })?;

        let mut bounded_exports = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();
        for export in exports {
            bounded_exports.push(export.clone()).map_err(|_| {
                Error::new(
                    ErrorCategory::Capacity,
                    codes::CAPACITY_EXCEEDED,
                    "Maximum number of exports exceeded",
                )
            })?;
        }

        Ok(Self { name: bounded_name, ty, exports: bounded_exports })
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Option<&Export> {
        self.exports.iter().find(|export| export.name == name)
    }

    /// Gets a mutable export by name
    pub fn get_export_mut(&mut self, name: &str) -> Option<&mut Export> {
        self.exports.iter_mut().find(|export| export.name == name)
    }

    /// Creates a builder for this instance value
    pub fn builder() -> InstanceValueBuilder {
        InstanceValueBuilder::new()
    }
}

/// Builder for InstanceValue
pub struct InstanceValueBuilder {
    /// The name of the instance
    name: Option<String>,
    /// Instance type
    ty: Option<ComponentTypeDefinition>,
    /// Instance exports
    exports: Vec<Export>,
}

impl InstanceValueBuilder {
    /// Creates a new instance value builder
    pub fn new() -> Self {
        Self { name: None, ty: None, exports: Vec::new() }
    }

    /// Sets the instance name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Sets the instance type
    pub fn with_type(mut self, ty: ComponentTypeDefinition) -> Self {
        self.ty = Some(ty);
        self
    }

    /// Adds an export to the instance
    pub fn with_export(mut self, export: Export) -> Self {
        self.exports.push(export);
        self
    }

    /// Adds multiple exports to the instance
    pub fn with_exports(mut self, exports: Vec<Export>) -> Self {
        self.exports.extend(exports);
        self
    }

    /// Builds the instance value
    pub fn build(self) -> Result<InstanceValue> {
        let name = self.name.ok_or_else(|| {
            Error::new(
                ErrorCategory::Parameter,
                codes::VALIDATION_ERROR,
                "Instance name is required",
            )
        })?;

        let ty = self.ty.ok_or_else(|| {
            Error::new(
                ErrorCategory::Parameter,
                codes::VALIDATION_ERROR,
                "Instance type is required",
            )
        })?;

        InstanceValue::new(&name, ty, &self.exports)
    }
}

impl Default for InstanceValueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A collection of instances with bounded capacity
pub struct InstanceCollection {
    instances: BoundedVec<InstanceValue, MAX_INSTANCE_EXPORTS, NoStdProvider<65536>>,
}

impl InstanceCollection {
    /// Creates a new empty instance collection
    pub fn new() -> Self {
        Self { instances: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap() }
    }

    /// Adds an instance to the collection
    pub fn add_instance(&mut self, instance: InstanceValue) -> Result<()> {
        self.instances.push(instance).map_err(|_| {
            Error::new(
                ErrorCategory::Capacity,
                codes::CAPACITY_EXCEEDED,
                "Maximum number of instances exceeded",
            )
        })
    }

    /// Gets an instance by name
    pub fn get_instance(&self, name: &str) -> Option<&InstanceValue> {
        self.instances.iter().find(|instance| instance.name.as_str() == name)
    }

    /// Gets a mutable instance by name
    pub fn get_instance_mut(&mut self, name: &str) -> Option<&mut InstanceValue> {
        self.instances.iter_mut().find(|instance| instance.name.as_str() == name)
    }

    /// Returns the number of instances in the collection
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Checks if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Returns an iterator over the instances
    pub fn iter(&self) -> impl Iterator<Item = &InstanceValue> {
        self.instances.iter()
    }
}

impl Default for InstanceCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use wrt::types::{FuncType, ValueType};
    use wrt_format::component::{ExternType, ValType};

    use super::*;
    use crate::export::Export;

    #[test]
    fn test_instance_value_builder() {
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };

        // Since most of the types require ownership, we'll mock them for testing
        // In a real implementation, we would need to use the actual types
        let component_func_type = ExternType::Function {
            params: vec![("a".to_string(), ValType::S32), ("b".to_string(), ValType::S32)],
            results: vec![ValType::S32],
        };

        let export = Export {
            name: "add".to_string(),
            ty: component_func_type.clone(),
            // Mocking this as we can't create an actual ExternValue in tests
            value: None,
        };

        let instance_type = ComponentTypeDefinition::Instance {
            exports: vec![("add".to_string(), component_func_type)],
        };

        // Build the instance
        let instance = InstanceValue::builder()
            .with_name("math")
            .with_type(instance_type)
            .with_export(export)
            .build()
            .unwrap();

        // Check the instance
        assert_eq!(instance.name.as_str(), "math");
        assert_eq!(instance.exports.len(), 1);
        assert_eq!(instance.get_export("add").unwrap().name, "add");
        assert!(instance.get_export("non_existent").is_none());
    }

    #[test]
    fn test_instance_collection() {
        // Create a simple instance
        let instance_type = ComponentTypeDefinition::Instance { exports: vec![] };

        let instance1 = InstanceValue::builder()
            .with_name("instance1")
            .with_type(instance_type.clone())
            .build()
            .unwrap();

        let instance2 = InstanceValue::builder()
            .with_name("instance2")
            .with_type(instance_type)
            .build()
            .unwrap();

        // Create a collection and add the instances
        let mut collection = InstanceCollection::new();
        assert!(collection.is_empty());

        collection.add_instance(instance1).unwrap();
        collection.add_instance(instance2).unwrap();

        // Check the collection
        assert_eq!(collection.len(), 2);
        assert!(!collection.is_empty());
        assert!(collection.get_instance("instance1").is_some());
        assert!(collection.get_instance("instance2").is_some());
        assert!(collection.get_instance("non_existent").is_none());

        // Test iteration
        let names: Vec<&str> = collection.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["instance1", "instance2"]);
    }
}
