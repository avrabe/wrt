// WRT - wrt
// SW-REQ-ID: [SW-REQ-ID-wrt]
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! WebAssembly Runtime (WRT)
//!
//! A pure Rust implementation of the WebAssembly runtime, supporting the WebAssembly Core
//! and Component Model specifications.
//!
//! WRT is designed to be compatible with both std and no_std environments, making it
//! suitable for a wide range of applications, from server-side WebAssembly execution
//! to embedded systems and bare-metal environments.
//!
//! ## Features
//!
//! - Full WebAssembly Core specification support
//! - Component Model implementation
//! - Stackless execution engine for environments with limited stack space
//! - no_std compatibility
//! - Comprehensive error handling
//! - Safe memory implementation with ASIL-B compliance features
//!
//! ## Organization
//!
//! WRT follows a modular design with specialized crates:
//!
//! - `wrt-error`: Error handling foundation
//! - `wrt-types`: Core and runtime type definitions
//! - `wrt-format`: Format specifications
//! - `wrt-decoder`: Binary parsing
//! - `wrt-sync`: Synchronization primitives
//! - `wrt-instructions`: Instruction encoding/decoding
//! - `wrt-intercept`: Function interception
//! - `wrt-host`: Host interface
//! - `wrt-component`: Component model
//! - `wrt-runtime`: Runtime execution
//! - `wrt`: Main library integration (this crate)

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]
#![warn(clippy::pedantic)]
#![warn(clippy::missing_panics_doc)]
#![warn(missing_docs)]
// Disable because it's unstable
// #![warn(rustdoc::missing_doc_code_examples)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Define debug_println macro for conditional debug printing
#[cfg(feature = "std")]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {{
        // No-op in no_std environments unless we implement a different printing mechanism
    }};
}

// Include prelude module for consistent imports across crates
pub mod prelude;

// Module adapters for integration between specialized crates
pub mod decoder_integration;
pub mod instructions_adapter;
pub mod memory_adapter;

// Re-export all public types and functionality through the prelude
pub use crate::prelude::*;

/// Version of the WebAssembly Core specification implemented
pub const CORE_VERSION: &str = "1.0";

/// Version of the WebAssembly Component Model specification implemented
pub const COMPONENT_VERSION: &str = "0.1.0";

/// Create a new stackless execution engine for WebAssembly modules.
///
/// This function creates a new stackless execution engine that can be used to run
/// WebAssembly modules in environments with limited stack space.
///
/// # Returns
///
/// A new stackless execution engine.
pub fn new_stackless_engine() -> StacklessEngine {
    StacklessEngine::new()
}

/// Create a new, empty WebAssembly module.
///
/// # Returns
///
/// A `Result` containing the new module, or an error if the module
/// could not be created.
pub fn new_module() -> Result<Module> {
    Module::new()
}

/// Create a new WebAssembly memory with the given type.
///
/// # Arguments
///
/// * `mem_type` - The type of memory to create.
///
/// # Returns
///
/// A new memory instance.
pub fn new_memory(mem_type: ComponentMemoryType) -> Memory {
    Memory::new(mem_type).unwrap()
}

/// Create a new WebAssembly memory adapter with the given type.
///
/// # Arguments
///
/// * `mem_type` - The type of memory to create.
///
/// # Returns
///
/// A new memory adapter instance.
pub fn new_memory_adapter(mem_type: ComponentMemoryType) -> Memory {
    memory_adapter::new_memory_adapter(mem_type).unwrap()
}

/// Create a new WebAssembly table with the given type.
///
/// # Arguments
///
/// * `table_type` - The type of table to create.
///
/// # Returns
///
/// A new table instance.
pub fn new_table(table_type: ComponentTableType) -> Table {
    // Create a default value based on the element type
    let default_value = Value::default_for_type(&table_type.element_type);

    Table::new(table_type, default_value).unwrap()
}

/// Load a module from a WebAssembly binary.
///
/// This is a convenience function that loads a WebAssembly module
/// from a binary buffer, handling validation and instantiation.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to load
///
/// # Returns
///
/// A Result containing the runtime module or an error
pub fn load_module_from_binary(binary: &[u8]) -> Result<Module> {
    // Directly use the function re-exported by the prelude from wrt_runtime
    // The types `Result` and `Module` are also from the prelude (originating in wrt_error and wrt_runtime)
    prelude::load_module_from_binary(binary)
}
