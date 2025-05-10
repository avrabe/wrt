//! Prelude module for wrt-format
//!
//! This module provides a unified set of imports for both std and no_std environments.
//! It re-exports commonly used types and traits to ensure consistency across all crates
//! in the WRT project and simplify imports in individual modules.

use crate::binary;

// Core imports for both std and no_std environments
pub use core::{
    any::Any,
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    convert::{TryFrom, TryInto},
    fmt,
    fmt::Debug,
    fmt::Display,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    slice, str,
};

// Re-export from std when the std feature is enabled
#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    format,
    string::{String, ToString},
    sync::{Arc, Mutex, RwLock},
    vec,
    vec::Vec,
};

// Re-export from alloc when no_std but alloc is available
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use alloc::{
    boxed::Box,
    collections::{BTreeMap as HashMap, BTreeSet as HashSet},
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};

// Import synchronization primitives for no_std
// For a no_std implementation, we should use appropriate no_std compatible sync primitives
// since wrt-sync is not implemented yet, we use spin-based locks as a placeholder
#[cfg(not(feature = "std"))]
pub use core::cell::{Cell, RefCell};

// Re-export from wrt-error
pub use wrt_error::{codes, kinds, Error, ErrorCategory, FromError, Result, ToErrorCategory};

// Re-export from wrt-types
pub use wrt_types::{
    // Component model types
    component_value::{ComponentValue, ValType},
    // Verification types
    verification::VerificationLevel,
    // SafeMemory types
    SafeMemoryHandler,
    SafeSlice,
    SafeStack,
};

// Conditional imports for safety features
#[cfg(feature = "safety")]
pub use wrt_types::safe_memory::{MemoryProvider, StdMemoryProvider};

// No-std memory provider
#[cfg(all(feature = "safety", not(feature = "std")))]
pub use wrt_types::safe_memory::NoStdMemoryProvider;

// Re-export from this crate's modules
pub use crate::{
    // Binary module constants and functions
    binary::{
        read_leb128_u32, read_string, write_leb128_u32, write_string, WASM_MAGIC, WASM_VERSION,
    },
    // Conversion utilities
    conversion::{
        block_type_to_format_block_type, convert, format_block_type_to_block_type,
        format_limits_to_wrt_limits, format_value_type, format_value_type as value_type_to_byte,
        parse_value_type, validate, validate_format, validate_option, wrt_limits_to_format_limits,
    },
    // Error conversion utilities
    error::{
        parse_error, runtime_error, to_wrt_error, type_error, validation_error, wrt_runtime_error,
        wrt_type_error, wrt_validation_error, IntoError,
    },
    // Section constants
    section::{
        CODE_ID, CUSTOM_ID, DATA_COUNT_ID, DATA_ID, ELEMENT_ID, EXPORT_ID, FUNCTION_ID, GLOBAL_ID,
        IMPORT_ID, MEMORY_ID, START_ID, TABLE_ID, TYPE_ID,
    },
};

// Helper functions for memory safety

/// Create a SafeSlice from a byte slice
#[cfg(feature = "safety")]
pub fn safe_slice(data: &[u8]) -> wrt_types::safe_memory::SafeSlice<'_> {
    wrt_types::safe_memory::SafeSlice::new(data)
}

/// Create a SafeSlice with specific verification level
#[cfg(feature = "safety")]
pub fn safe_slice_with_verification(
    data: &[u8],
    level: wrt_types::verification::VerificationLevel,
) -> wrt_types::safe_memory::SafeSlice<'_> {
    wrt_types::safe_memory::SafeSlice::with_verification_level(data, level)
}

/// Create a memory provider from a byte vector
#[cfg(feature = "safety")]
pub fn memory_provider(data: Vec<u8>) -> wrt_types::safe_memory::StdMemoryProvider {
    wrt_types::safe_memory::StdMemoryProvider::new(data)
}

/// Create a memory provider with specific capacity
#[cfg(feature = "safety")]
pub fn memory_provider_with_capacity(capacity: usize) -> wrt_types::safe_memory::StdMemoryProvider {
    wrt_types::safe_memory::StdMemoryProvider::with_capacity(capacity)
}

/// The prelude trait
pub trait Prelude {}

/// Standard prelude for the format library
pub mod std_prelude {
    // External crate imports
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    pub use alloc::{
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    #[cfg(feature = "std")]
    pub use std::{
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    #[cfg(all(not(feature = "std"), not(feature = "alloc")))]
    pub use wrt_types::{
        format, string::String, string::ToString, unsafe_vec::UnsafeVec as Vec, Box,
    };

    // Base types from wrt_types - fix incorrect paths
    pub use wrt_types::{
        // These types appear to be from the component module
        component::ComponentType,
        // Import valtype from component_value
        component_value::ValType,
        // SafeMemory types
        safe_memory::{SafeMemoryHandler, SafeSlice, SafeStack},
        // Import correctly from types module
        types::{
            BlockType, FuncType, GlobalType, Limits, MemoryType, RefType, TableType, ValueType,
        },
        // Verification
        verification::VerificationLevel,
    };

    // Format types - fix incorrect modules
    pub use crate::{
        binary, component::Component, module::Module, types::FormatBlockType,
        validation::Validatable,
    };

    // Explicitly re-export conversion utilities
    pub use crate::conversion::{
        block_type_to_format_block_type, convert, format_block_type_to_block_type,
        format_limits_to_wrt_limits, format_value_type, format_value_type as value_type_to_byte,
        parse_value_type, validate, validate_format, validate_option, wrt_limits_to_format_limits,
    };

    // Error handling
    pub use crate::error::{
        parse_error, runtime_error, to_wrt_error, type_error, validation_error, wrt_runtime_error,
        wrt_type_error, wrt_validation_error, IntoError,
    };

    // Result type
    pub use wrt_error::Result;
}

/// Implementation of the Prelude trait for the standard prelude
impl Prelude for std_prelude::String {}

/// Implementation of the Prelude trait for the standard prelude
impl Prelude for std_prelude::Vec<u8> {}

// Fix implementations for non-existent types
impl Prelude for binary::BinaryFormat {}

impl Prelude for crate::component::Component {}

impl Prelude for crate::module::Module {}

// Remove implementations for non-existent modules
// impl Prelude for crate::format::Format {}
// impl Prelude for crate::custom::Custom {}
// impl Prelude for crate::validation::Validation {}
// impl Prelude for crate::features::Features {}

/// No-std prelude for the format library
pub mod no_std_prelude {
    // External crate imports
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    pub use alloc::{
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    #[cfg(all(not(feature = "std"), not(feature = "alloc")))]
    pub use wrt_types::{
        format, string::String, string::ToString, unsafe_vec::UnsafeVec as Vec, Box,
    };

    // Base types from wrt_types - fix incorrect paths
    pub use wrt_types::{
        // These types appear to be from the component module
        component::ComponentType,
        // Import valtype from component_value
        component_value::ValType,
        // SafeMemory types
        safe_memory::{SafeMemoryHandler, SafeSlice, SafeStack},
        // Import correctly from types module
        types::{
            BlockType, FuncType, GlobalType, Limits, MemoryType, RefType, TableType, ValueType,
        },
        // Verification
        verification::VerificationLevel,
    };

    // Format types - fix incorrect modules
    pub use crate::{
        binary, component::Component, module::Module, types::FormatBlockType,
        validation::Validatable,
    };

    // Explicitly re-export conversion utilities
    pub use crate::conversion::{
        block_type_to_format_block_type, convert, format_block_type_to_block_type,
        format_limits_to_wrt_limits, format_value_type, format_value_type as value_type_to_byte,
        parse_value_type, validate, validate_format, validate_option, wrt_limits_to_format_limits,
    };

    // Error handling
    pub use crate::error::{
        parse_error, runtime_error, to_wrt_error, type_error, validation_error, wrt_runtime_error,
        wrt_type_error, wrt_validation_error, IntoError,
    };

    // Base error types from wrt_error
    pub use wrt_error::{codes, kinds, Error, ErrorCategory, FromError, Result, ToErrorCategory};
}
