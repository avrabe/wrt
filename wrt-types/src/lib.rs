//! Core type definitions, traits, and utilities for the WebAssembly Runtime
//! (WRT).
//!
//! This crate provides foundational data structures and functionalities used
//! across the WRT ecosystem, ensuring type safety, memory safety, and
//! consistent error handling. It supports both `std` and `no_std` environments,
//! with optional features for the WebAssembly Component Model and formal
//! verification.

// WRT - wrt-types
// SW-REQ-ID: REQ_MEM_SAFETY_001
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// #![deny(pointer_cast)] // Removed, as it's not a standard lint
// #![deny(alloc_instead_of_core)] // TODO: Verify this lint or implement if
// custom

// Import std when available
#[cfg(feature = "std")]
extern crate std;

// Import alloc for no_std with allocation
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Prelude module for consistent imports across std and no_std environments
pub mod prelude;

// Re-export common types from prelude
pub use prelude::*;
// Re-export error related types for convenience
pub use wrt_error::{kinds, Error};

/// Result type alias for WRT operations using `wrt_error::Error`
pub type WrtResult<T> = core::result::Result<T, Error>;

// Core modules
/// Bounded collections for memory safety
pub mod bounded;
/// WebAssembly Component Model built-in types
pub mod builtin;
/// WebAssembly Component Model types
pub mod component;
/// WebAssembly Component Model value types
pub mod component_value;
pub mod component_value_store;
/// Type conversion utilities
pub mod conversion;
/// Operation tracking and fuel metering
pub mod operations;
/// Resource management
pub mod resource;
/// Safe memory access primitives
pub mod safe_memory;
/// WebAssembly section definitions
pub mod sections;
/// Common traits for type conversions
pub mod traits;
/// Core WebAssembly types
pub mod types;
/// Validation utilities
pub mod validation;
/// WebAssembly value representations
pub mod values;
/// Verification and integrity checking
pub mod verification;

pub mod math_ops;

// Re-export the most important types
pub use bounded::{BoundedStack, BoundedVec, CapacityError};
pub use builtin::BuiltinType;
pub use component::{
    ComponentType, ExternType, GlobalType, InstanceType, Limits, MemoryType, Namespace,
    ResourceType, TableType,
};
pub use component_value::ComponentValue;
// Re-export conversion utilities
pub use component_value_store::ComponentValueStore;
pub use conversion::{ref_type_to_val_type, val_type_to_ref_type};
pub use operations::{
    global_fuel_consumed, global_operation_summary, record_global_operation,
    reset_global_operations, Summary as OperationSummary, Tracking as OperationTracking, Type as OperationType,
};
#[cfg(not(feature = "std"))]
pub use safe_memory::NoStdProvider;
#[cfg(feature = "std")]
pub use safe_memory::StdProvider;
pub use safe_memory::{Handler as SafeMemoryHandler, Provider as MemoryProvider, Slice as SafeSlice, SliceMut as SafeSliceMut, Stats as MemoryStats};
pub use traits::{FromFormat, ToFormat};
pub use types::{BlockType, FuncType, RefType, ValueType};
pub use validation::{BoundedCapacity, Checksummed, Validatable};
pub use values::Value;
pub use verification::{Checksum, VerificationLevel};

/// The WebAssembly binary format magic number: \0asm
pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// The WebAssembly Core Specification version this runtime aims to support.
/// Version 2.0 includes features like multi-value returns, reference types,
/// etc.
pub const WASM_VERSION: u32 = 2;

// Core feature re-exports
// Note: These are feature-gated re-exports that shouldn't conflict with the
// main ones #[cfg(feature = "component-model-core")]
// pub use component::ComponentType;

#[cfg(feature = "component-model-values")]
pub use component_value::ValType;
// Re-export key types
pub use values::{FloatBits32, FloatBits64};

#[cfg(test)]
mod tests {
    // TODO: Add comprehensive tests for all public functionality in wrt-types,
    // ensuring coverage for different VerificationLevels, std/no_std features,
    // and edge cases for component model types, value conversions, etc.
    // Specific modules like bounded.rs, safe_memory.rs, math_ops.rs, etc.,
    // should have their own detailed test suites as well.
}
