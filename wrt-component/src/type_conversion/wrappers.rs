//! Type wrapper implementations for Component Model types
//!
//! This module provides wrapper types for Component Model types to avoid
//! orphan rule violations when implementing conversions between different
//! representations.
//!
//! # Orphan Rules
//!
//! Rust's orphan rules prevent implementing traits for types from a different
//! crate unless the trait is defined in the current crate. This module defines
//! local wrapper types around external types to enable safe trait
//! implementations.
//!
//! # Examples
//!
//! ```
//! use wrt_component::type_conversion::wrappers::{
//!     RuntimeComponentType, FormatComponentType
//! };
//! use wrt_foundation::component::ComponentType as TypesComponentType;
//! use wrt_format::component::ComponentTypeDefinition;
//!
//! // Create a wrapper around a runtime type
//! let rt_type = TypesComponentType {
//!     imports: vec![],
//!     exports: vec![],
//!     instances: vec![],
//! };
//! let wrapper = RuntimeComponentType::new(rt_type);
//!
//! // Get the underlying type
//! let inner_type = wrapper.into_inner();
//! ```

// Additional imports
use wrt_format::component::{ComponentTypeDefinition, ExternType as FormatExternType};

use super::bidirectional::{
    format_to_runtime_extern_type, runtime_to_format_extern_type, IntoFormatType, IntoRuntimeType,
};

/// Wrapper around wrt_foundation::component::ComponentType
#[derive(Debug, Clone)]
pub struct RuntimeComponentType {
    /// The wrapped component type
    inner: ComponentType,
}

/// Wrapper around wrt_format::component::ComponentTypeDefinition::Component
#[derive(Debug, Clone)]
pub struct FormatComponentType {
    /// The component imports
    pub imports: Vec<(String, String, FormatExternType)>,
    /// The component exports
    pub exports: Vec<(String, FormatExternType)>,
}

/// Wrapper around wrt_foundation::component::InstanceType
#[derive(Debug, Clone)]
pub struct RuntimeInstanceType {
    /// The wrapped instance type
    inner: InstanceType,
}

/// Wrapper around wrt_format::component::ComponentTypeDefinition::Instance
#[derive(Debug, Clone)]
pub struct FormatInstanceType {
    /// The instance exports
    pub exports: Vec<(String, FormatExternType)>,
}

impl RuntimeComponentType {
    /// Create a new runtime component type wrapper
    pub fn new(component_type: ComponentType) -> Self {
        Self { inner: component_type }
    }

    /// Get the inner component type
    pub fn inner(&self) -> &ComponentType {
        &self.inner
    }

    /// Consume the wrapper and return the inner component type
    pub fn into_inner(self) -> ComponentType {
        self.inner
    }
}

impl From<ComponentType> for RuntimeComponentType {
    fn from(component_type: ComponentType) -> Self {
        Self::new(component_type)
    }
}

impl From<RuntimeComponentType> for ComponentType {
    fn from(wrapper: RuntimeComponentType) -> Self {
        wrapper.into_inner()
    }
}

impl FormatComponentType {
    /// Create a new format component type wrapper
    pub fn new(
        imports: Vec<(String, String, FormatExternType)>,
        exports: Vec<(String, FormatExternType)>,
    ) -> Self {
        Self { imports, exports }
    }

    /// Convert to ComponentTypeDefinition
    pub fn to_component_type_definition(&self) -> ComponentTypeDefinition {
        ComponentTypeDefinition::Component {
            imports: self.imports.clone(),
            exports: self.exports.clone(),
        }
    }
}

impl From<ComponentTypeDefinition> for FormatComponentType {
    fn from(type_def: ComponentTypeDefinition) -> Self {
        match type_def {
            ComponentTypeDefinition::Component { imports, exports } => Self::new(imports, exports),
            _ => panic!("Expected Component type definition"),
        }
    }
}

impl RuntimeInstanceType {
    /// Create a new runtime instance type wrapper
    pub fn new(instance_type: InstanceType) -> Self {
        Self { inner: instance_type }
    }

    /// Get the inner instance type
    pub fn inner(&self) -> &InstanceType {
        &self.inner
    }

    /// Consume the wrapper and return the inner instance type
    pub fn into_inner(self) -> InstanceType {
        self.inner
    }
}

impl From<InstanceType> for RuntimeInstanceType {
    fn from(instance_type: InstanceType) -> Self {
        Self::new(instance_type)
    }
}

impl From<RuntimeInstanceType> for InstanceType {
    fn from(wrapper: RuntimeInstanceType) -> Self {
        wrapper.into_inner()
    }
}

impl FormatInstanceType {
    /// Create a new format instance type wrapper
    pub fn new(exports: Vec<(String, FormatExternType)>) -> Self {
        Self { exports }
    }

    /// Convert to ComponentTypeDefinition
    pub fn to_component_type_definition(&self) -> ComponentTypeDefinition {
        ComponentTypeDefinition::Instance { exports: self.exports.clone() }
    }
}

impl From<ComponentTypeDefinition> for FormatInstanceType {
    fn from(type_def: ComponentTypeDefinition) -> Self {
        match type_def {
            ComponentTypeDefinition::Instance { exports } => Self::new(exports),
            _ => panic!("Expected Instance type definition"),
        }
    }
}

// Bidirectional conversion implementations

impl TryFrom<RuntimeComponentType> for FormatComponentType {
    type Error = Error;

    fn try_from(runtime_type: RuntimeComponentType) -> Result<Self> {
        let runtime_type = runtime_type.into_inner();

        // Convert imports
        let imports_result: core::result::Result<Vec<(String, String, FormatExternType)>> = runtime_type
            .imports
            .into_iter()
            .map(|(namespace, name, extern_type)| {
                runtime_to_format_extern_type(&extern_type)
                    .map(|format_type| (namespace, name, format_type))
            })
            .collect();

        // Convert exports
        let exports_result: core::result::Result<Vec<(String, FormatExternType)>> = runtime_type
            .exports
            .into_iter()
            .map(|(name, extern_type)| {
                runtime_to_format_extern_type(&extern_type).map(|format_type| (name, format_type))
            })
            .collect();

        Ok(Self { imports: imports_result?, exports: exports_result? })
    }
}

impl TryFrom<FormatComponentType> for RuntimeComponentType {
    type Error = Error;

    fn try_from(format_type: FormatComponentType) -> Result<Self> {
        // Convert imports
        let imports_result: core::result::Result<Vec<(String, String, TypesExternType)>> = format_type
            .imports
            .into_iter()
            .map(|(namespace, name, extern_type)| {
                format_to_runtime_extern_type(&extern_type)
                    .map(|runtime_type| (namespace, name, runtime_type))
            })
            .collect();

        // Convert exports
        let exports_result: core::result::Result<Vec<(String, TypesExternType)>> = format_type
            .exports
            .into_iter()
            .map(|(name, extern_type)| {
                format_to_runtime_extern_type(&extern_type).map(|runtime_type| (name, runtime_type))
            })
            .collect();

        // Create empty instances for now - can be enhanced in future
        let instances = Vec::new();

        Ok(Self::new(ComponentType {
            imports: imports_result?,
            exports: exports_result?,
            instances,
        }))
    }
}

impl TryFrom<RuntimeInstanceType> for FormatInstanceType {
    type Error = Error;

    fn try_from(runtime_type: RuntimeInstanceType) -> Result<Self> {
        let runtime_type = runtime_type.into_inner();

        // Convert exports
        let exports_result: core::result::Result<Vec<(String, FormatExternType)>> = runtime_type
            .exports
            .into_iter()
            .map(|(name, extern_type)| {
                runtime_to_format_extern_type(&extern_type).map(|format_type| (name, format_type))
            })
            .collect();

        Ok(Self { exports: exports_result? })
    }
}

impl TryFrom<FormatInstanceType> for RuntimeInstanceType {
    type Error = Error;

    fn try_from(format_type: FormatInstanceType) -> Result<Self> {
        // Convert exports
        let exports_result: core::result::Result<Vec<(String, TypesExternType)>> = format_type
            .exports
            .into_iter()
            .map(|(name, extern_type)| {
                format_to_runtime_extern_type(&extern_type).map(|runtime_type| (name, runtime_type))
            })
            .collect();

        Ok(Self::new(InstanceType { exports: exports_result? }))
    }
}

// Extension traits to make conversions easier to use

/// Trait for converting to RuntimeComponentType
pub trait IntoRuntimeComponentType {
    /// Convert to RuntimeComponentType
    fn into_runtime_component_type(self) -> Result<RuntimeComponentType>;
}

/// Trait for converting to FormatComponentType
pub trait IntoFormatComponentType {
    /// Convert to FormatComponentType
    fn into_format_component_type(self) -> Result<FormatComponentType>;
}

/// Trait for converting to RuntimeInstanceType
pub trait IntoRuntimeInstanceType {
    /// Convert to RuntimeInstanceType
    fn into_runtime_instance_type(self) -> Result<RuntimeInstanceType>;
}

/// Trait for converting to FormatInstanceType
pub trait IntoFormatInstanceType {
    /// Convert to FormatInstanceType
    fn into_format_instance_type(self) -> Result<FormatInstanceType>;
}

impl IntoRuntimeComponentType for FormatComponentType {
    fn into_runtime_component_type(self) -> Result<RuntimeComponentType> {
        self.try_into()
    }
}

impl IntoFormatComponentType for RuntimeComponentType {
    fn into_format_component_type(self) -> Result<FormatComponentType> {
        self.try_into()
    }
}

impl IntoRuntimeInstanceType for FormatInstanceType {
    fn into_runtime_instance_type(self) -> Result<RuntimeInstanceType> {
        self.try_into()
    }
}

impl IntoFormatInstanceType for RuntimeInstanceType {
    fn into_format_instance_type(self) -> Result<FormatInstanceType> {
        self.try_into()
    }
}

// Implementation for ComponentTypeDefinition conversion
impl TryFrom<ComponentTypeDefinition> for RuntimeComponentType {
    type Error = Error;

    fn try_from(type_def: ComponentTypeDefinition) -> Result<Self> {
        match type_def {
            ComponentTypeDefinition::Component { imports, exports } => {
                let format_type = FormatComponentType::new(imports, exports);
                format_type.try_into()
            }
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Expected Component type definition".to_string(),
            )),
        }
    }
}

impl TryFrom<ComponentTypeDefinition> for RuntimeInstanceType {
    type Error = Error;

    fn try_from(type_def: ComponentTypeDefinition) -> Result<Self> {
        match type_def {
            ComponentTypeDefinition::Instance { exports } => {
                let format_type = FormatInstanceType::new(exports);
                format_type.try_into()
            }
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Expected Instance type definition".to_string(),
            )),
        }
    }
}
