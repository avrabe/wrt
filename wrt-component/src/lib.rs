// WRT - wrt-component
// SW-REQ-ID: [SW-REQ-ID-wrt-component]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! Component Model implementation for the WebAssembly Runtime (WRT).
//!
//! This crate provides an implementation of the WebAssembly Component Model,
//! enabling composition and interoperability between WebAssembly modules
//! with shared-nothing linking.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "kani", feature(kani))]
#![warn(clippy::missing_panics_doc)]

// When no_std but alloc is available
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Verify that we have alloc when using no_std
#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!("The 'alloc' feature must be enabled when using no_std");

// Export our prelude module for consistent imports
pub mod prelude;

// Export modules
pub mod builtins;
pub mod canonical;
pub mod component;
pub mod component_registry;
pub mod execution;
pub mod export;
pub mod export_map;
pub mod factory;
pub mod host;
pub mod import;
pub mod import_map;
pub mod instance;
pub mod modules;
pub mod namespace;
pub mod parser;
pub mod resources;
pub mod runtime;
pub mod strategies;
pub mod type_conversion;
pub mod types;
pub mod values;

// Include verification module when the kani feature is enabled
#[cfg(feature = "kani")]
pub mod verify;

// Re-export core types and functionality for convenience
pub use builtins::{BuiltinHandler, BuiltinRegistry};
pub use canonical::CanonicalABI;
pub use component::{Component, ExternValue, FunctionValue, GlobalValue, MemoryValue, TableValue};
pub use component_registry::ComponentRegistry;
pub use export::Export;
pub use factory::ComponentFactory;
pub use host::Host;
pub use import::Import;
pub use instance::InstanceValue;
pub use namespace::Namespace;
pub use parser::get_required_builtins;
pub use resources::{BufferPool, MemoryStrategy, Resource, ResourceTable, VerificationLevel};
pub use strategies::memory::{
    BoundedCopyStrategy, FullIsolationStrategy, MemoryOptimizationStrategy, ZeroCopyStrategy,
};
pub use type_conversion::{
    common_to_format_val_type, core_value_to_types_componentvalue, extern_type_to_func_type,
    format_constvalue_to_types_componentvalue, format_to_common_val_type,
    format_to_types_extern_type, format_val_type_to_value_type, format_valtype_to_types_valtype,
    types_componentvalue_to_core_value, types_componentvalue_to_format_constvalue,
    types_to_format_extern_type, types_valtype_to_format_valtype, value_type_to_format_val_type,
    value_type_to_types_valtype, IntoFormatType, IntoRuntimeType,
};
pub use type_conversion::{
    FormatComponentType, FormatInstanceType, IntoFormatComponentType, IntoFormatInstanceType,
    IntoRuntimeComponentType, IntoRuntimeInstanceType, RuntimeComponentType, RuntimeInstanceType,
};
pub use types::ComponentInstance;
pub use values::{
    component_to_core_value,
    core_to_component_value,
    deserialize_component_value,
    // Use what's actually available in the values module
};
pub use wrt_error::{codes, Error, ErrorCategory, Result};
pub use wrt_host::CallbackRegistry;
pub use wrt_types::builtin::BuiltinType;
pub use wrt_types::component::ComponentType;
pub use wrt_types::types::ValueType;
pub use wrt_types::values::Value;

/// Debug logging macro - conditionally compiled
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "debug-log", feature = "std"))]
        {
            println!($($arg)*);
        }
        #[cfg(not(all(feature = "debug-log", feature = "std")))]
        {
            // Do nothing when debug logging is disabled or in no_std
        }
    };
}
