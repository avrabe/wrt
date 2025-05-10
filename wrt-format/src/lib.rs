// WRT - wrt-format
// SW-REQ-ID: [SW-REQ-ID-wrt-format]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! WebAssembly format handling for WRT
//!
//! This crate defines and handles the WebAssembly binary format specifications,
//! including type encodings, section layouts, and module structures.
//!
//! It is designed to work in both std and no_std environments when configured
//! with the appropriate feature flags.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Import std when available
#[cfg(feature = "std")]
extern crate std;

// Import alloc for no_std environments with allocation
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

// Import std/alloc collections based on feature flag
#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    fmt, format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use alloc::{
    boxed::Box,
    collections::{BTreeMap as HashMap, BTreeSet as HashSet},
    fmt, format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

// Re-export core types from wrt-types
pub use wrt_types::{BlockType, FuncType, RefType, ValueType};
// Re-export error types directly from wrt-error
pub use wrt_error::{Error, ErrorCategory};
// Re-export Result type from wrt-types
pub use wrt_types::Result;
// Re-export resource types from wrt-types
pub use wrt_types::resource::ResourceRepresentation;

/// WebAssembly binary format parsing and access
pub mod binary;
pub mod canonical;
/// WebAssembly component model format
pub mod component;
/// Conversion utilities for component model types
pub mod component_conversion;
/// Compression utilities for WebAssembly modules
pub mod compression;
/// Conversion utilities for type system standardization
pub mod conversion;
/// Error utilities for working with wrt-error types
pub mod error;
/// WebAssembly module format
pub mod module;
/// Common imports for convenience
pub mod prelude;
/// Safe memory operations
pub mod safe_memory;
pub mod section;
pub mod state;
pub mod types;
/// Validation utilities
pub mod validation;
pub mod verify;
pub mod version;

pub use component::Component;
pub use compression::{rle_decode, rle_encode, CompressionType};
// Re-export conversion utilities
pub use conversion::{
    block_type_to_format_block_type, format_block_type_to_block_type, format_limits_to_wrt_limits,
    wrt_limits_to_format_limits,
};
pub use error::{
    parse_error, wrt_runtime_error as runtime_error, wrt_type_error as type_error,
    wrt_validation_error as validation_error,
};
pub use module::Module;
pub use section::{CustomSection, Section};
pub use state::{create_state_section, extract_state_section, is_state_section_name, StateSection};
// Use the conversion module versions for consistency
pub use types::{FormatBlockType, Limits, MemoryIndexType};
pub use validation::Validatable;
pub use version::{
    ComponentModelFeature, ComponentModelVersion, FeatureStatus, VersionInfo, STATE_VERSION,
};
// Re-export binary constants
pub use binary::{
    read_leb128_u32, read_string, write_leb128_u32, write_string, COMPONENT_CORE_SORT_FUNC,
    COMPONENT_CORE_SORT_GLOBAL, COMPONENT_CORE_SORT_INSTANCE, COMPONENT_CORE_SORT_MEMORY,
    COMPONENT_CORE_SORT_MODULE, COMPONENT_CORE_SORT_TABLE, COMPONENT_CORE_SORT_TYPE,
    COMPONENT_MAGIC, COMPONENT_SORT_COMPONENT, COMPONENT_SORT_CORE, COMPONENT_SORT_FUNC,
    COMPONENT_SORT_INSTANCE, COMPONENT_SORT_TYPE, COMPONENT_SORT_VALUE, COMPONENT_VERSION,
};

// Re-export safe memory utilities
pub use safe_memory::safe_slice;

// Public functions for feature detection
/// Check if a component model feature is available in a binary
pub fn is_feature_available(info: &VersionInfo, feature: ComponentModelFeature) -> bool {
    info.is_feature_available(feature)
}

/// Get the status of a component model feature
pub fn get_feature_status(info: &VersionInfo, feature: ComponentModelFeature) -> FeatureStatus {
    info.get_feature_status(feature)
}

/// Check if a binary uses any experimental features
pub fn uses_experimental_features(binary: &[u8]) -> bool {
    let version_bytes = if binary.len() >= 8 {
        [binary[4], binary[5], binary[6], binary[7]]
    } else {
        return false;
    };

    let mut info = VersionInfo::from_version_bytes(version_bytes);
    info.detect_experimental_features(binary)
}

// Deprecated: use conversion utilities instead
#[deprecated(
    since = "0.2.0",
    note = "Use conversion::parse_value_type instead for better type conversion"
)]
pub use types::parse_value_type;
#[deprecated(
    since = "0.2.0",
    note = "Use conversion::value_type_to_byte instead for better type conversion"
)]
pub use types::value_type_to_byte;

// For formal verification when the 'kani' feature is enabled
#[cfg(feature = "kani")]
pub mod verification {
    use kani_verifier::*;

    /// Verify LEB128 encoding and decoding
    #[kani::proof]
    fn verify_leb128_roundtrip() {
        let value: u32 = kani::any();
        // Limit to reasonable values for test
        kani::assume(value <= 0xFFFF);

        let encoded = super::binary::write_leb128_u32(value);
        let (decoded, _) = super::binary::read_leb128_u32(&encoded, 0).unwrap();

        assert_eq!(value, decoded);
    }
}
