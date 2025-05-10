//! Prelude module for wrt-decoder
//!
//! This module provides a unified set of imports for both std and no_std environments.
//! It re-exports commonly used types and traits to ensure consistency across all crates
//! in the WRT project and simplify imports in individual modules.

// Core imports for both std and no_std environments
pub use core::{
    any::Any,
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    convert::{From, Into, TryFrom, TryInto},
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
    borrow::Cow,
    boxed::Box,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    format, io,
    io::{Read, Write},
    rc::Rc,
    result::Result as StdResult,
    string::{String, ToString},
    sync::{Arc, Mutex, RwLock},
    vec,
    vec::Vec,
};

// Re-export from alloc when no_std but alloc is available
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use alloc::{
    borrow::Cow,
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    rc::Rc,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};

#[cfg(not(feature = "std"))]
pub use core::result::Result as StdResult;

// Import synchronization primitives for no_std
//#[cfg(not(feature = "std"))]
//pub use wrt_sync::{Mutex, RwLock};

// Re-export from wrt-error
pub use wrt_error::{codes, kinds, Error, ErrorCategory, Result};

// Re-export from wrt-types
pub use wrt_types::{
    // Component model types
    component_value::{ComponentValue, ValType},
    // SafeMemory types
    safe_memory::{SafeMemoryHandler, SafeSlice, SafeStack},
    // Types
    types::{BlockType, FuncType, GlobalType, MemoryType, RefType, TableType, ValueType},
    values::Value,
};

// Re-export from wrt-format
pub use wrt_format::{
    // Binary utilities
    binary::{
        is_valid_wasm_header, parse_block_type, read_leb128_i32, read_leb128_i64, read_leb128_u32,
        read_leb128_u64, read_name, read_string, validate_utf8, write_leb128_i32, write_leb128_i64,
        write_leb128_u32, write_leb128_u64, BinaryFormat, WASM_MAGIC, WASM_VERSION,
    },
    // Conversion utilities
    conversion::{
        block_type_to_format_block_type, format_block_type_to_block_type, parse_value_type,
        format_value_type as value_type_to_byte,
    },
    // Module types
    module::{
        Data, DataMode, Element, Export, ExportKind, Function, Global, Import, ImportDesc, Memory,
        Table,
    },
    // Section types
    section::{CustomSection, Section, SectionId},
    // Format-specific types
    types::{FormatBlockType, Limits, MemoryIndexType},
};

// Conversion utilities from wrt-types
#[cfg(feature = "conversion")]
pub use wrt_types::conversion::{ref_type_to_val_type, val_type_to_ref_type};

// Re-export from this crate
pub use crate::{
    // Decoder core
    decoder_core::validate,
    // Module types
    module::Module,
    // Utils
    utils,
};

// Re-export format module for compatibility
pub use wrt_format as format;
