// WRT - wrt-types
// Module: WebAssembly Component Model Core Types
// SW-REQ-ID: REQ_WASM_COMPONENT_001 (Example: Relates to core component structures)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Defines the core types for the WebAssembly Component Model such as
//! ComponentType, InstanceType, ExternType, etc.

use crate::types::FuncType;
use crate::types::ValueType;
use crate::{String, Vec};
#[cfg(not(feature = "std"))]
use alloc::string::ToString;

/// Represents a component type
#[derive(Debug, Clone)]
pub struct ComponentType {
    /// Component imports
    pub imports: Vec<(String, String, ExternType)>,
    /// Component exports
    pub exports: Vec<(String, ExternType)>,
    /// Component instances
    pub instances: Vec<InstanceType>,
}

impl ComponentType {
    /// Creates a new component type with the specified imports and exports
    pub fn new(
        imports: Vec<(String, String, ExternType)>,
        exports: Vec<(String, ExternType)>,
    ) -> Self {
        Self { imports, exports, instances: Vec::new() }
    }

    /// Creates an empty component type
    pub fn empty() -> Self {
        Self { imports: Vec::new(), exports: Vec::new(), instances: Vec::new() }
    }
}

/// Represents an instance type
#[derive(Debug, Clone)]
pub struct InstanceType {
    /// Instance exports
    pub exports: Vec<(String, ExternType)>,
}

/// Represents an external type
#[derive(Debug, Clone)]
pub enum ExternType {
    /// Function type
    Function(FuncType),
    /// Table type
    Table(TableType),
    /// Memory type
    Memory(MemoryType),
    /// Global type
    Global(GlobalType),
    /// Resource type
    Resource(ResourceType),
    /// Instance type
    Instance(InstanceType),
    /// Component type
    Component(ComponentType),
}

/// Represents a table type
#[derive(Debug, Clone, PartialEq)]
pub struct TableType {
    /// Table element type
    pub element_type: ValueType,
    /// Table limits
    pub limits: Limits,
}

/// Represents a memory type
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryType {
    /// Memory limits
    pub limits: Limits,
    /// Whether the memory is shared
    pub shared: bool,
}

/// Represents a global type
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalType {
    /// Global value type
    pub value_type: ValueType,
    /// Whether the global is mutable
    pub mutable: bool,
}

/// Represents a resource type
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceType {
    /// Resource name
    pub name: String,
    /// Resource representation type
    pub rep_type: ValueType,
}

/// Represents a limit on a numeric range
#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    /// Minimum value
    pub min: u32,
    /// Maximum value (if any)
    pub max: Option<u32>,
}

/// Represents a namespace for component imports and exports
#[derive(Debug, Clone)]
pub struct Namespace {
    /// Namespace elements (e.g., "wasi", "http", "client")
    pub elements: Vec<String>,
}

impl Namespace {
    /// Creates a namespace from a string
    #[must_use]
    pub fn from_string(s: &str) -> Self {
        let elements =
            s.split('.').filter(|part| !part.is_empty()).map(ToString::to_string).collect();
        Self { elements }
    }

    /// Checks if this namespace matches another namespace
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        if self.elements.len() != other.elements.len() {
            return false;
        }

        self.elements.iter().zip(other.elements.iter()).all(|(a, b)| a == b)
    }

    /// Checks if this namespace is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

#[cfg(feature = "std")]
impl core::fmt::Display for Namespace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.elements.join("."))
    }
}

#[cfg(all(not(feature = "std"), feature = "alloc"))]
impl core::fmt::Display for Namespace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.elements.is_empty() {
            return Ok(());
        }

        write!(f, "{}", self.elements[0])?;
        for elem in &self.elements[1..] {
            write!(f, ".{}", elem)?;
        }
        Ok(())
    }
}

/// Represents a component import with namespace
#[derive(Debug, Clone)]
pub struct ImportDefinition {
    /// Import name
    pub name: String,
    /// Import namespace
    pub namespace: Namespace,
    /// Import type
    pub ty: ExternType,
}

/// Type compatibility checking
pub fn types_are_compatible(a: &ExternType, b: &ExternType) -> bool {
    match (a, b) {
        (ExternType::Function(a_ty), ExternType::Function(b_ty)) => {
            func_types_compatible(a_ty, b_ty)
        }
        (ExternType::Table(a_ty), ExternType::Table(b_ty)) => a_ty == b_ty,
        (ExternType::Memory(a_ty), ExternType::Memory(b_ty)) => a_ty == b_ty,
        (ExternType::Global(a_ty), ExternType::Global(b_ty)) => a_ty == b_ty,
        (ExternType::Resource(_), ExternType::Resource(_)) => true, // Basic compatibility for now
        (ExternType::Instance(a_ty), ExternType::Instance(b_ty)) => {
            instance_types_match(a_ty, b_ty)
        }
        (ExternType::Component(_), ExternType::Component(_)) => true, // Basic compatibility for now
        _ => false,
    }
}

/// Check if two function types are compatible
pub fn func_types_compatible(a: &FuncType, b: &FuncType) -> bool {
    if a.params.len() != b.params.len() || a.results.len() != b.results.len() {
        return false;
    }

    for (a_param, b_param) in a.params.iter().zip(b.params.iter()) {
        if a_param != b_param {
            return false;
        }
    }

    for (a_result, b_result) in a.results.iter().zip(b.results.iter()) {
        if a_result != b_result {
            return false;
        }
    }

    true
}

/// Check if two instance types match for linking
pub fn instance_types_match(a: &InstanceType, b: &InstanceType) -> bool {
    if a.exports.len() != b.exports.len() {
        return false;
    }

    for ((a_name, a_ty), (b_name, b_ty)) in a.exports.iter().zip(b.exports.iter()) {
        if a_name != b_name || !types_are_compatible(a_ty, b_ty) {
            return false;
        }
    }

    true
}
