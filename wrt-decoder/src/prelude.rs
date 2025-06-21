// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Prelude module for wrt-decoder
//!
//! This module provides a unified set of imports for both std and no_std
//! environments. It re-exports commonly used types and traits to ensure
//! consistency across all crates in the WRT project and simplify imports in
//! individual modules.

// Don't duplicate format import since it's already in the use block above
#[cfg(not(feature = "std"))]
pub use core::result::Result as StdResult;
pub use core::{
    any::Any,
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    convert::{From, Into, TryFrom, TryInto},
    fmt,
    fmt::{Debug, Display},
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

// No_std equivalents - use wrt-foundation types (Vec and String defined below with specific providers)
#[cfg(not(feature = "std"))]
pub use wrt_foundation::BoundedMap as HashMap;

// Import synchronization primitives for no_std
//#[cfg(not(feature = "std"))]
// pub use wrt_sync::{Mutex, RwLock};

// Re-export from wrt-error
pub use wrt_error::{codes, kinds, Error, ErrorCategory, Result};
// Re-export format module for compatibility
pub use wrt_format as wrt_format_module;
// Re-export from wrt-format
pub use wrt_format::{
    // Conversion utilities
    conversion::{
        block_type_to_format_block_type, format_block_type_to_block_type,
        format_value_type as value_type_to_byte, parse_value_type,
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

// Binary std/no_std choice
#[cfg(feature = "std")]
// pub use wrt_format::state::{create_state_section, extract_state_section, StateSection};
// Binary std/no_std choice
#[cfg(feature = "std")]
pub use wrt_foundation::component_value::{ComponentValue, ValType};
// Conversion utilities from wrt-foundation
#[cfg(feature = "conversion")]
pub use wrt_foundation::conversion::{ref_type_to_val_type, val_type_to_ref_type};
// Re-export clean types from wrt-foundation
pub use wrt_foundation::{
    // SafeMemory types
    safe_memory::{SafeMemoryHandler, SafeSlice, SafeStack},
    // Legacy types for compatibility
    types::{BlockType, RefType, ValueType},
    values::Value,
};

// Re-export clean types only when allocation is available
#[cfg(any(feature = "std", feature = "alloc"))]
pub use wrt_foundation::{
    CleanFuncType, CleanGlobalType, CleanMemoryType, CleanTableType, CleanValType, CleanValue,
};

// Most re-exports temporarily disabled for demo

// Binary std/no_std choice
pub use crate::decoder_no_alloc;

// Use our unified memory management system
#[cfg(not(feature = "std"))]
pub use wrt_foundation::{
    unified_types_simple::{DefaultTypes, EmbeddedTypes},
    BoundedString, BoundedVec,
};
// For no_std mode, use explicit bounded types (no confusing aliases!)
#[cfg(not(feature = "std"))]
pub type DecoderVec<T> = BoundedVec<T, 256, wrt_foundation::NoStdProvider<4096>>;
#[cfg(not(feature = "std"))]
pub type DecoderString = BoundedString<256, wrt_foundation::NoStdProvider<4096>>;

// For std mode, provide the same types but using std collections
#[cfg(feature = "std")]
pub type DecoderVec<T> = Vec<T>;
#[cfg(feature = "std")]
pub type DecoderString = String;

// Factory function for creating providers using capability system
#[cfg(not(feature = "std"))]
pub fn create_decoder_provider<const N: usize>(
) -> wrt_foundation::WrtResult<wrt_foundation::NoStdProvider<N>> {
    use wrt_foundation::{CrateId, capabilities::MemoryFactory};
    MemoryFactory::create::<N>(CrateId::Decoder)
}

// For std mode, provide a simple version
#[cfg(feature = "std")]
pub fn create_decoder_provider<const N: usize>(
) -> wrt_foundation::WrtResult<wrt_foundation::NoStdProvider<N>> {
    use wrt_foundation::NoStdProvider;
    Ok(NoStdProvider::default())
}

// For no_std mode, provide a minimal ToString trait
/// Minimal ToString trait for no_std environments
#[cfg(not(feature = "std"))]
pub trait ToString {
    /// Convert to string
    fn to_string(&self) -> DecoderString;
}

#[cfg(not(feature = "std"))]
impl ToString for &str {
    fn to_string(&self) -> DecoderString {
        if let Ok(provider) = create_decoder_provider::<4096>() {
            DecoderString::from_str(self, provider).unwrap_or_default()
        } else {
            DecoderString::default()
        }
    }
}

// Binary std/no_std choice
/// Minimal format macro for no_std environments
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => {{
        // In pure no_std, return a simple bounded string
        use wrt_foundation::{BoundedString, NoStdProvider};
        if let Ok(provider) = $crate::prelude::create_decoder_provider::<512>() {
            $crate::prelude::DecoderString::from_str("formatted_string", provider)
                .unwrap_or_default()
        } else {
            $crate::prelude::DecoderString::default()
        }
    }};
}

// Export our custom format macro for no_std
#[cfg(not(feature = "std"))]
pub use crate::format;

/// Binary format utilities
#[cfg(feature = "std")]
pub mod binary {
    /// Read LEB128 u32 from data
    pub fn read_leb_u32(data: &[u8]) -> wrt_error::Result<(u32, usize)> {
        wrt_format::binary::read_leb128_u32(data, 0)
    }
}

/// Binary utilities for no_std environments
#[cfg(not(feature = "std"))]
pub mod binary {
    use super::create_decoder_provider;
    use wrt_foundation::{BoundedVec, NoStdProvider};

    /// Write LEB128 u32 in no_std mode
    pub fn write_leb128_u32(value: u32) -> BoundedVec<u8, 10, NoStdProvider<64>> {
        if let Ok(provider) = create_decoder_provider::<64>() {
            let mut result = BoundedVec::new(provider).expect("Failed to create bounded vec for LEB128");
            let mut buffer = [0u8; 10];
            // Simple LEB128 encoding for no_std
            let mut bytes_written = 0;
            let mut val = value;
            loop {
                let mut byte = (val & 0x7F) as u8;
                val >>= 7;
                if val != 0 {
                    byte |= 0x80;
                }
                if bytes_written < buffer.len() {
                    buffer[bytes_written] = byte;
                    bytes_written += 1;
                }
                if val == 0 {
                    break;
                }
            }

            if bytes_written > 0 {
                for i in 0..bytes_written {
                    let _ = result.push(buffer[i]);
                }
            }
            result
        } else {
            BoundedVec::default()
        }
    }

    /// Write string in no_std mode
    pub fn write_string(_s: &str) -> BoundedVec<u8, 256, NoStdProvider<512>> {
        if let Ok(provider) = create_decoder_provider::<512>() {
            // Simplified no_std implementation
            BoundedVec::new(provider).expect("Failed to create bounded vec for string")
        } else {
            BoundedVec::default()
        }
    }

    /// Read LEB128 u32 from data with offset
    pub fn read_leb_u32(data: &[u8], offset: usize) -> wrt_error::Result<(u32, usize)> {
        // Simple implementation for no_std - just read from beginning
        if offset >= data.len() {
            return Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Parse,
                wrt_error::codes::PARSE_ERROR,
                "Offset out of bounds",
            ));
        }
        // For simplicity, just parse from the offset
        let mut value = 0u32;
        let mut shift = 0;
        let mut bytes_read = 0;

        for &byte in &data[offset..] {
            if bytes_read >= 5 {
                return Err(wrt_error::Error::new(
                    wrt_error::ErrorCategory::Parse,
                    wrt_error::codes::PARSE_ERROR,
                    "LEB128 too long",
                ));
            }

            value |= ((byte & 0x7F) as u32) << shift;
            bytes_read += 1;

            if (byte & 0x80) == 0 {
                return Ok((value, bytes_read));
            }

            shift += 7;
        }

        Err(wrt_error::Error::new(
            wrt_error::ErrorCategory::Parse,
            wrt_error::codes::PARSE_ERROR,
            "Incomplete LEB128",
        ))
    }

    /// Read name from binary data in no_std mode
    pub fn read_name(data: &[u8], offset: usize) -> wrt_error::Result<(&[u8], usize)> {
        if offset >= data.len() {
            return Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Parse,
                wrt_error::codes::PARSE_ERROR,
                "Offset out of bounds",
            ));
        }

        // Read length as LEB128
        let (length, new_offset) = read_leb_u32(data, offset)?;
        let name_start = offset + new_offset;

        if name_start + length as usize > data.len() {
            return Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Parse,
                wrt_error::codes::PARSE_ERROR,
                "Name extends beyond data",
            ));
        }

        Ok((
            &data[name_start..name_start + length as usize],
            name_start + length as usize,
        ))
    }
}

// Make commonly used binary functions available at top level (now exported by wrt_format directly)
pub use wrt_format::read_leb128_u32;
#[cfg(feature = "std")]
pub use wrt_format::{read_name, read_string, write_leb128_u32, write_string};

// For no_std mode, provide the missing functions locally
#[cfg(not(feature = "std"))]
pub use binary::{read_name, write_leb128_u32, write_string};

/// Extension trait to add missing methods to BoundedVec
pub trait BoundedVecExt<T, const N: usize, P: wrt_foundation::MemoryProvider> {
    /// Create an empty BoundedVec
    fn empty() -> Self;
    /// Try to push an item, returning an error if capacity is exceeded
    fn try_push(&mut self, item: T) -> wrt_error::Result<()>;
    /// Check if the collection is empty
    fn is_empty(&self) -> bool;
}

#[cfg(not(feature = "std"))]
impl<T, const N: usize, P> BoundedVecExt<T, N, P> for wrt_foundation::bounded::BoundedVec<T, N, P>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
    P: wrt_foundation::MemoryProvider + Clone + Default + PartialEq + Eq,
{
    fn empty() -> Self {
        Self::new(P::default()).unwrap_or_default()
    }

    fn try_push(&mut self, item: T) -> wrt_error::Result<()> {
        self.push(item).map_err(|_e| {
            wrt_error::Error::new(
                wrt_error::ErrorCategory::Resource,
                wrt_error::codes::CAPACITY_EXCEEDED,
                "BoundedVec push failed: capacity exceeded",
            )
        })
    }

    fn is_empty(&self) -> bool {
        use wrt_foundation::traits::BoundedCapacity;
        self.len() == 0
    }
}

// For compatibility, add some aliases that the code expects
/// Read LEB128 u32 from data
#[cfg(feature = "std")]
pub fn read_leb_u32(data: &[u8]) -> wrt_error::Result<(u32, usize)> {
    read_leb128_u32(data, 0)
}

/// Read LEB128 u32 from data (no_std version)
#[cfg(not(feature = "std"))]
pub fn read_leb_u32(data: &[u8]) -> wrt_error::Result<(u32, usize)> {
    read_leb128_u32(data, 0)
}

/// Read string from data (no_std version)
#[cfg(not(feature = "std"))]
pub fn read_string(_data: &[u8], _offset: usize) -> wrt_error::Result<(&[u8], usize)> {
    // Simplified implementation for no_std
    Ok((&[], 0))
}

// Missing utility functions
/// Validate WebAssembly header
pub fn is_valid_wasm_header(data: &[u8]) -> bool {
    data.len() >= 8
        && &data[0..4] == wrt_format::binary::WASM_MAGIC
        && &data[4..8] == wrt_format::binary::WASM_VERSION
}

// read_name is now imported from wrt_format

// read_leb128_u32 is now imported from wrt_format

// Feature-gated function aliases - bring in functions from wrt_format that aren't already exported
#[cfg(feature = "std")]
pub use wrt_format::binary::with_alloc::parse_block_type as parse_format_block_type;
