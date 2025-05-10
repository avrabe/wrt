// WRT - wrt-host
// SW-REQ-ID: [SW-REQ-ID-wrt-host]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! Host function infrastructure for the WebAssembly Runtime (WRT).
//!
//! This crate provides the core infrastructure for registering and managing host functions
//! that can be called from WebAssembly components. It follows the Component Model specification
//! for host functions and the Canonical ABI.
//!
//! ## Features
//!
//! - Host function registration and callback management
//! - Built-in function handling for the WebAssembly Component Model
//! - Interception and introspection of function calls
//! - No-std compatible with the `no_std` and `alloc` features
//!
//! ## Usage
//!
//! ```rust,no_run
//! # use wrt_host::prelude::*;
//! # use wrt_types::Value;
//!
//! // Create a host builder
//! let builder = HostBuilder::new()
//!     .with_host_function("my_module", "my_function",
//!         HostFunctionHandler::new(|_| Ok(vec![Value::I32(42)])))
//!     .with_component_name("my_component")
//!     .with_host_id("my_host");
//!
//! // Build the host
//! let registry = builder.build().expect("Failed to build host");
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(clippy::missing_panics_doc)]

// When no_std but alloc is available
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Verify required features when using no_std
#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!("The 'alloc' feature must be enabled when using no_std");

// Export modules
pub mod builder;
pub mod callback;
pub mod function;
pub mod host;
pub mod prelude;

// Include verification module conditionally, but exclude during coverage builds
#[cfg(all(not(coverage), any(doc, kani)))]
pub mod verify;

// Reexport types for convenience (use fully qualified paths)
pub use builder::HostBuilder;
pub use callback::{CallbackRegistry, CallbackType};
pub use function::{CloneableFn, HostFunctionHandler};
pub use host::BuiltinHost;

// Re-export prelude for convenience
pub use prelude::*;
