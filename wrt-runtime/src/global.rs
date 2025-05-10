//! WebAssembly global value implementation
//!
//! This module provides the implementation for WebAssembly globals.

use crate::prelude::*;
// Use WrtGlobalType directly from wrt_types, and WrtValueType, WrtValue
use wrt_types::types::{GlobalType as WrtGlobalType, ValueType as WrtValueType};
use wrt_types::values::Value as WrtValue;

/// Represents a WebAssembly global variable in the runtime
#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    /// The global type (value_type and mutability).
    /// The initial_value from WrtGlobalType is used to set the runtime `value` field upon creation.
    ty: WrtGlobalType, 
    /// The current runtime value of the global variable.
    value: WrtValue,
}

impl Global {
    /// Create a new runtime Global instance.
    /// The `initial_value` is used to set the initial runtime `value`.
    pub fn new(value_type: WrtValueType, mutable: bool, initial_value: WrtValue) -> Result<Self> {
        // Construct the WrtGlobalType for storage.
        // The initial_value in WrtGlobalType might seem redundant here if we only use it for the `value` field,
        // but it keeps the `ty` field complete as per its definition.
        let global_ty_descriptor = WrtGlobalType {
            value_type,
            mutable,
            initial_value: initial_value.clone(), // Store the original initial value as part of the type descriptor.
        };

        // The runtime `value` starts as the provided `initial_value`.
        Ok(Self { ty: global_ty_descriptor, value: initial_value })
    }

    /// Get the current runtime value of the global.
    pub fn get(&self) -> &WrtValue {
        &self.value
    }

    /// Set the runtime value of the global.
    /// Returns an error if the global is immutable or if the value type mismatches.
    pub fn set(&mut self, new_value: &WrtValue) -> Result<()> {
        if !self.ty.mutable {
            return Err(Error::new(
                ErrorCategory::Validation, 
                codes::IMMUTABLE_GLOBAL, // More specific error code
                "Cannot modify immutable global",
            ));
        }

        if !new_value.matches_type(&self.ty.value_type) {
            return Err(Error::new(
                ErrorCategory::Type, 
                codes::TYPE_MISMATCH, 
                format!(
                    "Value type {:?} doesn't match global type {:?}",
                    new_value.value_type(), self.ty.value_type
                ),
            ));
        }

        self.value = new_value.clone();
        Ok(())
    }

    /// Get the WrtGlobalType descriptor (value_type, mutability, and original initial_value).
    pub fn global_type_descriptor(&self) -> &WrtGlobalType {
        &self.ty
    }
}

// The local `GlobalType` struct is no longer needed as we use WrtGlobalType from wrt_types directly.
// /// Represents a WebAssembly global type
// #[derive(Debug, Clone, PartialEq)]
// pub struct GlobalType { ... } // REMOVED
// impl GlobalType { ... } // REMOVED
