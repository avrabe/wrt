// WRT - wrt
// Module: Main WRT Library Integration
// SW-REQ-ID: REQ_OVERVIEW_001
// SW-REQ-ID: REQ_OVERVIEW_002
// SW-REQ-ID: REQ_016
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! WebAssembly Runtime (WRT)
//!
//! A pure Rust implementation of the WebAssembly runtime, supporting the
//! WebAssembly Core and Component Model specifications.
//!
//! WRT is designed to be compatible with both std and no_std environments,
//! making it suitable for a wide range of applications, from server-side
//! WebAssembly execution to embedded systems and bare-metal environments.
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
//! - `wrt-foundation`: Core foundation library (previously wrt-foundation)
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

#[cfg(not(feature = "std"))]
extern crate alloc;

// Binary std/no_std choice
// All memory management uses bounded collections with NoStdProvider

// Panic handler for no_std builds - temporarily disabled to avoid workspace
// conflicts Applications using WRT should provide their own panic handler
// #[cfg(all(not(feature = "std"), not(test), not(feature =
// "disable-panic-handler")))] #[panic_handler]
// fn panic(_info: &core::panic::PanicInfo) -> ! {
//     // ASIL-B/D compliant panic handling:
//     // 1. Ensure deterministic behavior (no heap allocations)
//     // 2. Enter safe state immediately
//     // 3. Prevent any restart or recovery attempts
//
//     // For safety-critical systems, we enter an infinite loop
//     // to ensure the system remains in a known safe state
//     loop {
//         // Use spin_loop hint for power efficiency and better behavior
//         // in virtualized environments
//         core::hint::spin_loop();
//     }
// }

// Define debug_println macro for conditional debug printing
#[cfg(feature = "std")]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*));
        }
    };
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {{
        // No-op in no_std environments unless we implement a different printing
        // mechanism
    }};
}

// Include prelude module for consistent imports across crates
pub mod prelude;

// Bounded infrastructure for static memory allocation
pub mod bounded_wrt_infra;

// Safety-critical memory limits
#[cfg(feature = "safety-critical")]
pub mod memory_limits;

// Bulk memory operations runtime
pub mod bulk_memory_runtime;

// Atomic operations runtime
pub mod atomic_runtime;

// Shared memory runtime for WebAssembly 3.0 threads
pub mod shared_memory_runtime;

// Multi-memory runtime for WebAssembly 3.0 multi-memory proposal
pub mod multi_memory_runtime;

// Unified WebAssembly 3.0 features runtime integration
pub mod webassembly_3_runtime;

// Module adapters for integration between specialized crates
// #[cfg(feature = "std")] // CFI integration requires std features currently
// pub mod cfi_integration;
// pub mod decoder_integration; // Temporarily disabled
pub mod instructions_adapter;
// pub mod memory_adapter; // Temporarily disabled due to trait object size
// issues

// No_std implementation modules are now handled by wrt-foundation

// Resources implementation - std vs no_std
#[cfg(feature = "std")]
pub mod resource; // WebAssembly component model resource types with std

#[cfg(not(feature = "std"))]
pub mod resource_nostd; // No_std compatible resource implementation
#[cfg(not(feature = "std"))]
pub use resource_nostd as resource; // Use resource_nostd as resource when no_std
// Export the StacklessEngine for direct use
pub use wrt_runtime::stackless::StacklessEngine;

// Re-export all public types and functionality through the prelude
pub use crate::prelude::*;

/// Version of the WebAssembly Core specification implemented
pub const CORE_VERSION: &str = "1.0";

/// Version of the WebAssembly Component Model specification implemented
pub const COMPONENT_VERSION: &str = "0.1.0";

/// Create a new stackless execution engine for WebAssembly modules.
///
/// This function creates a new stackless execution engine that can be used to
/// run WebAssembly modules in environments with limited stack space.
///
/// # Returns
///
/// A new stackless execution engine.
pub fn new_stackless_engine() -> wrt_runtime::stackless::StacklessEngine {
    wrt_runtime::stackless::StacklessEngine::new()
}

/// Create a new, empty WebAssembly module.
///
/// # Returns
///
/// A `Result` containing the new module, or an error if the module
/// could not be created.
// TODO: Re-enable after fixing dependency compilation issues in wrt-instructions
// pub fn new_module() -> Result<Module> {
//     wrt_runtime::module::Module::new()
// }

/// Create a new WebAssembly memory with the given type.
///
/// # Arguments
///
/// * `mem_type` - The type of memory to create.
///
/// # Returns
///
/// A new memory instance.
// TODO: Re-enable after fixing dependency compilation issues in wrt-instructions
// pub fn new_memory(mem_type: ComponentMemoryType) -> Memory {
//     Memory::new(mem_type).unwrap()
// }

// /// Create a new WebAssembly memory adapter with the given type.
// ///
// /// # Arguments
// ///
// /// * `mem_type` - The type of memory to create.
// ///
// /// # Returns
// ///
// /// A new memory adapter instance.
// pub fn new_memory_adapter(mem_type: ComponentMemoryType) -> Memory {
//     memory_adapter::new_memory_adapter(mem_type).unwrap()
// }

/// Create a new WebAssembly table with the given type.
///
/// # Arguments
///
/// * `table_type` - The type of table to create.
///
/// # Returns
///
/// A new table instance.
// TODO: Re-enable after fixing dependency compilation issues in wrt-instructions
// pub fn new_table(table_type: ComponentTableType) -> Table {
//     // Create a default value based on the element type
//     let default_value = Value::default_for_type(&table_type.element_type;
//
//     Table::new(table_type, default_value).unwrap()
// }

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
// TODO: Re-enable after fixing dependency compilation issues in wrt-instructions
// pub fn load_module_from_binary(binary: &[u8]) -> Result<Module> {
//     // Directly use the function re-exported by the prelude from wrt_runtime
//     // The types `Result` and `Module` are also from the prelude (originating in
//     // wrt_error and wrt_runtime)
//     prelude::load_module_from_binary(binary)
// }

/// Create a new CFI-protected execution engine with default settings.
///
/// This function creates a CFI-protected WebAssembly execution engine
/// that provides Control Flow Integrity protection against ROP/JOP attacks.
///
/// # Returns
///
/// A Result containing the CFI-protected engine or an error
#[cfg(feature = "std")]
pub fn new_cfi_protected_engine() -> Result<cfi_integration::CfiProtectedEngine> {
    cfi_integration::new_cfi_engine()
}

/// Execute a WebAssembly module with CFI protection.
///
/// This is a high-level convenience function that loads and executes
/// a WebAssembly module with comprehensive CFI protection.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to execute
/// * `function_name` - The name of the function to execute
///
/// # Returns
///
/// A Result containing the CFI execution result or an error
#[cfg(feature = "std")]
pub fn execute_with_cfi_protection(
    binary: &[u8],
    function_name: &str,
) -> Result<cfi_integration::CfiExecutionResult> {
    cfi_integration::execute_module_with_cfi(binary, function_name)
}

/// Execute a WebAssembly module with custom CFI configuration.
///
/// This function provides fine-grained control over CFI protection settings
/// for WebAssembly execution.
///
/// # Arguments
///
/// * `binary` - The WebAssembly binary to execute
/// * `function_name` - The name of the function to execute
/// * `config` - CFI configuration options
///
/// # Returns
///
/// A Result containing the CFI execution result or an error
#[cfg(feature = "std")]
pub fn execute_with_cfi_config(
    binary: &[u8],
    function_name: &str,
    config: cfi_integration::CfiConfiguration,
) -> Result<cfi_integration::CfiExecutionResult> {
    cfi_integration::execute_module_with_cfi_config(binary, function_name, config)
}
