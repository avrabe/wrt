// WRT - wrt-decoder
// SW-REQ-ID: [SW-REQ-ID-wrt-decoder]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! WebAssembly module decoder for wrt runtime
//!
//! This crate provides a high-level API for decoding WebAssembly binary modules
//! into structured representations that can be used by the wrt runtime.
//!
//! The decoder sits between the low-level binary format handling in `wrt-format`
//! and the runtime execution in `wrt`. It properly converts between format types
//! and runtime types, ensuring type consistency across the system.
//!
//! # Features
//!
//! - Decoding WebAssembly modules from binary format
//! - Encoding modules back to binary format
//! - Validating module structure
//! - Memory-efficient handling of WASM modules
//! - Component model support
//! - No_std and std environment support
//!
//! ## Feature Flags
//!
//! - `std` (default): Enable standard library support
//! - `alloc`: Enable allocation support (required for no_std)
//! - `component`: Enable WebAssembly Component Model support
//! - `safe_memory`: Enable memory safety features

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(clippy::missing_panics_doc)]
//#![deny(missing_docs)] // Temporarily disabled for build

// Verify required features
#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!("The 'alloc' feature must be enabled when using no_std");

// Import core
extern crate core;

// Import std when available
#[cfg(feature = "std")]
extern crate std;

// Import alloc for no_std
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Module exports
pub mod component;
pub mod conversion;
pub mod decoder_core;
pub mod instructions;
pub mod module;
pub mod name_section;
pub mod parser;
pub mod prelude;
pub mod producers_section;
pub mod runtime_adapter;
pub mod section_error;
pub mod section_reader;
pub mod sections;
pub mod utils;
pub mod validation;
pub mod wasm;
pub mod custom_section_utils;

// Re-exports from error crate
pub use wrt_error::{codes, kinds, Error, Result};

// Re-export conversion utilities
pub use conversion::{
    byte_to_value_type, component_limits_to_format_limits, convert_to_wrt_error,
    format_error_to_wrt_error, format_func_type_to_types_func_type,
    format_global_to_types_global, format_limits_to_component_limits,
    format_limits_to_types_limits, format_memory_type_to_types_memory_type,
    format_table_type_to_types_table_type, format_value_type_to_value_type,
    format_value_types_to_value_types, section_code_to_section_type, section_type_to_section_code,
    types_limits_to_format_limits, value_type_to_byte, value_type_to_format_value_type,
};

// Re-export runtime adapter
pub use runtime_adapter::{convert_to_runtime_module, RuntimeModuleBuilder};

// Re-export safe_memory for backward compatibility
pub use wrt_types::safe_memory;

// Re-export the SafeSlice type and other memory safety types
pub use wrt_types::safe_memory::{MemoryProvider, SafeSlice, StdMemoryProvider};

// Re-export binary constants and functions from wrt-format
pub use wrt_format::binary::{
    read_leb128_i32, read_leb128_i64, read_leb128_u32, read_leb128_u64, write_leb128_i32,
    write_leb128_i64, write_leb128_u32, write_leb128_u64, WASM_MAGIC, WASM_VERSION,
};

// Re-export important module types and functions
pub use module::{
    decode_module_with_binary as decode_module, decode_module_with_binary, encode_module, Module,
};
pub use wrt_format::section::CustomSection;

// Re-export parser types and functions
pub use parser::{Parser, Payload};

// Re-export section types
pub use sections::parsers;
pub use wrt_format::section::Section;

// Re-export core types for easier access
pub use wrt_types::types::{FuncType, GlobalType, Limits, MemoryType, RefType, TableType};

// Re-export format types for easy access to section types
pub use wrt_format::module::{Data, DataMode, Element, Export, Import, ImportDesc};

// Additional re-exports from wrt_format
pub use wrt_format::module::{Function, Global, Memory, Table};

// Re-exports from wrt_types
pub use wrt_types::{
    component::{ComponentType, ExternType},
    resource::ResourceId,
    types::ValueType,
    values::Value,
};

// Re-export validation from validation module
pub use crate::decoder_core::validate::ValidationConfig;
pub use validation::{validate_module, validate_module_with_config};

// Re-export custom section utilities
pub use custom_section_utils::{create_engine_state_section, get_data_from_state_section};

/// Create a module from WebAssembly binary data
///
/// # Arguments
///
/// * `bytes` - WebAssembly binary data
///
/// # Returns
///
/// * `Result<Module>` - Parsed module or error
pub fn from_binary(bytes: &[u8]) -> Result<Module> {
    module::decode_module_with_binary(bytes)
}

/// Validate a WebAssembly module
///
/// # Arguments
///
/// * `module` - Module to validate
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub fn validate(module: &Module) -> Result<()> {
    validation::validate_module(module)
}

/// Encode a module to binary format
///
/// # Arguments
///
/// * `module` - Module to encode
///
/// # Returns
///
/// * `Result<Vec<u8>>` - Binary data or error
pub fn to_binary(module: &Module) -> Result<crate::prelude::Vec<u8>> {
    module::encode_module(module)
}

/// Parse a WebAssembly module from binary data
///
/// This is an alias for `from_binary` for backward compatibility.
///
/// # Arguments
///
/// * `binary` - WebAssembly binary data
///
/// # Returns
///
/// * `Result<Module>` - Parsed module or error
pub fn parse(binary: &[u8]) -> Result<Module> {
    from_binary(binary)
}
