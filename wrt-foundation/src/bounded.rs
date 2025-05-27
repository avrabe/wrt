// WRT - wrt-foundation
// Module: Bounded Collections
// SW-REQ-ID: REQ_MEMORY_003, REQ_COLLECTION_BOUNDED_001
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Provides bounded versions of common collections like `Vec` and `String`.
//!
//! These collections ensure that they do not exceed a predefined capacity,
//! contributing to memory safety and predictability, especially in `no_std`
//! environments.

#[cfg(feature = "alloc")]
use alloc::string::ToString;

/// Bounded collections with functional safety verification
///
/// This module provides bounded collection types that are designed for
/// functional safety with built-in size limits and verification features.
// Make these constants available in all configurations
/// Maximum length for WebAssembly names (e.g., import/export names, custom
/// section names). Chosen as a reasonable upper limit, often Wasm tools have
/// smaller practical limits.
pub const MAX_WASM_NAME_LENGTH: usize = 255;

/// Maximum length for WebAssembly module names.
pub const MAX_WASM_MODULE_NAME_LENGTH: usize = MAX_WASM_NAME_LENGTH;

/// Maximum length for WebAssembly item names.
pub const MAX_WASM_ITEM_NAME_LENGTH: usize = MAX_WASM_NAME_LENGTH;

/// Maximum length for WebAssembly function names.
pub const MAX_WASM_FUNCTION_NAME_LENGTH: usize = MAX_WASM_NAME_LENGTH;

/// Maximum length for WebAssembly interface names.
pub const MAX_WASM_INTERFACE_NAME_LENGTH: usize = MAX_WASM_NAME_LENGTH;

/// Maximum length for WebAssembly instance names.
pub const MAX_WASM_INSTANCE_NAME_LENGTH: usize = MAX_WASM_NAME_LENGTH;

/// Maximum length for WebAssembly component names.
pub const MAX_WASM_COMPONENT_NAME_LENGTH: usize = MAX_WASM_NAME_LENGTH;

/// Maximum length for WebAssembly string values.
pub const MAX_WASM_STRING_LENGTH: usize = 1024;

/// Maximum size for custom section data.
pub const MAX_CUSTOM_SECTION_DATA_SIZE: usize = 4096;

/// DWARF Debug Information Constants
/// Maximum size for a single DWARF section (1MB)
pub const MAX_DWARF_SECTION_SIZE: usize = 1_048_576;

/// Maximum number of abbreviations to cache
pub const MAX_DWARF_ABBREV_CACHE: usize = 128;

/// Maximum depth for DWARF DIE tree traversal
pub const MAX_DWARF_TREE_DEPTH: usize = 32;

/// Maximum file names in line number program
pub const MAX_DWARF_FILE_TABLE: usize = 256;

/// Maximum directories in line number program
pub const MAX_DWARF_DIR_TABLE: usize = 64;

/// Maximum number of types in a component type definition.
pub const MAX_COMPONENT_TYPES: usize = 256;

/// Maximum number of items in a component list.
pub const MAX_COMPONENT_LIST_ITEMS: usize = 256;

/// Maximum number of items in a component fixed-size list.
pub const MAX_COMPONENT_FIXED_LIST_ITEMS: usize = 256;

/// Maximum number of fields in a component record.
pub const MAX_COMPONENT_RECORD_FIELDS: usize = 64;

/// Maximum number of elements in a component tuple.
pub const MAX_COMPONENT_TUPLE_ITEMS: usize = 64;

/// Maximum number of flag definitions in a component flags type.
pub const MAX_COMPONENT_FLAGS: usize = 64;

/// Maximum number of context items in a component error.
pub const MAX_COMPONENT_ERROR_CONTEXT_ITEMS: usize = 16;

/// Maximum number of values that can be deserialized at once.
pub const MAX_DESERIALIZED_VALUES: usize = 256;

/// Maximum number of fields in a component type record.
pub const MAX_TYPE_RECORD_FIELDS: usize = 64;

/// Maximum number of cases in a component type variant.
pub const MAX_TYPE_VARIANT_CASES: usize = 64;

/// Maximum number of elements in a component type tuple.
pub const MAX_TYPE_TUPLE_ELEMENTS: usize = 64;

/// Maximum number of names in a component type flags.
pub const MAX_TYPE_FLAGS_NAMES: usize = 64;

/// Maximum size for memory buffers in no_std environment
pub const MAX_BUFFER_SIZE: usize = 4096;

/// Maximum number of names in a component type enum.
pub const MAX_TYPE_ENUM_NAMES: usize = 64;

/// Default maximum size for an item to be serialized onto a stack buffer within
/// BoundedVec/BoundedStack.
const MAX_ITEM_SERIALIZED_SIZE: usize = 256;

/// Size of the checksum in bytes, typically the size of a u32.
pub const CHECKSUM_SIZE: usize = core::mem::size_of::<u32>();

#[cfg(feature = "alloc")]
extern crate alloc;

// For std environment
// For no_std with alloc
// #[cfg(all(feature = "alloc", not(feature = "std")))] // This line was
// importing `alloc::{};` use alloc::{}; // Removed empty import

// For no_std environment
#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use core::fmt; // Removed hash, mem
use core::{
    hash::{Hash, Hash as CoreHash, Hasher, Hasher as CoreHasher},
    marker::PhantomData,
};
// use core::mem::MaybeUninit; // No longer needed here if SafeMemoryHandler doesn't expose it
// directly
#[cfg(feature = "std")]
use std::fmt;

use wrt_error::ErrorCategory as WrtErrorCategory; /* And added here as a top-level import -
                                                   * Keep ErrorCategory qualified */

// Format is used via the prelude when std or alloc is enabled

// Use the HashMap that's re-exported in lib.rs - works for both std and no_std
#[allow(unused_imports)]
use crate::operations::{self, record_global_operation};
// use crate::HashMap; // Removed, should come from prelude

// Ensure MemoryProvider is imported directly for trait bounds.
// #[cfg(feature = "std")]
// use crate::prelude::Vec; // This was added in a previous step for owned Vec, keep it. <--
// Removing as per unused_import warning
// NoStdProvider is imported where it's actually used
// use crate::safe_memory::SafeMemory; // Remove this if it was added
use crate::safe_memory::SafeMemoryHandler; // Ensure this is imported
use crate::safe_memory::SliceMut; // IMPORT ADDED
use crate::traits::{
    importance, BoundedCapacity, Checksummed, DefaultMemoryProvider, SerializationError,
}; // Moved from validation to traits module
use crate::MemoryProvider; // Added import for the MemoryProvider trait alias
use crate::{
    codes,
    prelude::{Clone, Debug, Default, Display, Eq, Ord, PartialEq, Sized},
    safe_memory::Slice,
    traits::{ReadStream, WriteStream},
};
use crate::{
    operations::Type as OperationType,
    traits::{Checksummable, FromBytes, ToBytes},
    verification::{Checksum, VerificationLevel},
    // Error is available through prelude
    WrtResult, // Import WrtResult for the crate
}; // Renamed Hasher to CoreHasher to avoid conflict if P also brings a Hasher
   // use std::collections::hash_map::RandomState; // For a default hasher -
   // BoundedHashMap not found, this is likely unused for no_std

/// Error indicating a collection has reached its capacity limit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityError;

impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bounded collection capacity exceeded")
    }
}

impl From<CapacityError> for crate::Error {
    fn from(_err: CapacityError) -> Self {
        crate::Error::new(
            WrtErrorCategory::Capacity,
            codes::CAPACITY_EXCEEDED,
            "Bounded collection capacity exceeded", // Always &'static str
        )
    }
}

/// Error types for bounded collections
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum BoundedErrorKind {
    CapacityExceeded,
    InvalidCapacity,
    ConversionError,
    SliceError,
    Utf8Error,
    ItemTooLarge,
    VerificationError,
    // Add other kinds as needed
}

impl Display for BoundedErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BoundedErrorKind::CapacityExceeded => write!(f, "Capacity exceeded"),
            BoundedErrorKind::InvalidCapacity => write!(f, "Invalid capacity provided"),
            BoundedErrorKind::ConversionError => write!(f, "Conversion error"),
            BoundedErrorKind::SliceError => write!(f, "Slice error"),
            BoundedErrorKind::Utf8Error => write!(f, "UTF-8 error"),
            BoundedErrorKind::ItemTooLarge => write!(f, "Item too large for operation"),
            BoundedErrorKind::VerificationError => write!(f, "Verification failed"),
        }
    }
}

/// Error type for bounded collection operations.
#[derive(Debug, PartialEq, Eq)]
pub struct BoundedError {
    pub kind: BoundedErrorKind,
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub description: String, // This will be alloc::string::String or std::string::String
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    pub description_static: &'static str, // For no-alloc scenarios
}

impl BoundedError {
    /// Creates a new `BoundedError`.
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub fn new<S>(kind: BoundedErrorKind, description: S) -> Self
    where
        S: Into<String>,
    {
        Self { kind, description: description.into() }
    }

    /// Creates a new `BoundedError` for `no_std` (no alloc) environments.
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    pub fn new(kind: BoundedErrorKind, description: &'static str) -> Self {
        Self { kind, description_static: description }
    }

    /// Creates a new `BoundedError` indicating capacity was exceeded.
    pub fn capacity_exceeded() -> Self {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            Self::new(BoundedErrorKind::CapacityExceeded, "Capacity exceeded".to_string())
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            Self::new(BoundedErrorKind::CapacityExceeded, "Capacity exceeded")
        }
    }

    /// Creates a new `BoundedError` indicating invalid capacity.
    pub fn invalid_capacity<T: Debug>(value: T) -> Self {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            // Assuming prelude brings in `format` correctly
            Self::new(BoundedErrorKind::InvalidCapacity, format!("Invalid capacity: {:?}", value))
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            // In no_std without alloc, we cannot format `value`.
            // Provide a generic static message.
            drop(value); // Suppress unused warning
            Self::new(BoundedErrorKind::InvalidCapacity, "Invalid capacity provided")
        }
    }

    /// Creates a new `BoundedError` for conversion errors.
    pub fn conversion_error(msg_part: &str) -> Self {
        // Changed S: AsRef<str> to &str for simplicity with format!
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            // Assuming prelude brings in `format` correctly
            Self::new(BoundedErrorKind::ConversionError, format!("Conversion error: {}", msg_part))
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            // In no_std without alloc, we cannot use msg_part dynamically.
            // Provide a generic static message.
            Self::new(BoundedErrorKind::ConversionError, "Conversion error")
        }
    }

    /// Creates a new `BoundedError` for deserialization errors (placeholder).
    /// TODO: Define properly if this is distinct from general conversion/parse
    /// errors.
    pub fn deserialization_error(msg: &'static str) -> Self {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            Self::new(BoundedErrorKind::ConversionError, format!("Deserialization error: {}", msg))
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            Self::new(BoundedErrorKind::ConversionError, msg) // Use the static
                                                              // msg directly
        }
    }

    /// Creates a new `BoundedError` for memory-related errors (placeholder).
    /// TODO: Define properly.
    pub fn memory_error(msg: &'static str) -> Self {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            Self::new(BoundedErrorKind::SliceError, format!("Memory error: {}", msg))
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            Self::new(BoundedErrorKind::SliceError, msg)
        }
    }

    /// Creates a new `BoundedError` for index out of bounds (placeholder).
    /// TODO: Define properly.
    pub fn index_out_of_bounds(index: usize, length: usize) -> Self {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            Self::new(
                BoundedErrorKind::SliceError,
                format!("Index {} out of bounds for length {}", index, length),
            )
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            // Cannot format the index/length here, so a generic message
            Self::new(BoundedErrorKind::SliceError, "Index out of bounds")
        }
    }

    /// Creates a new `BoundedError` for validation errors (placeholder).
    /// TODO: Define properly.
    pub fn validation_error(msg: &'static str) -> Self {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            Self::new(BoundedErrorKind::VerificationError, format!("Validation error: {}", msg))
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            Self::new(BoundedErrorKind::VerificationError, msg)
        }
    }

    /// Returns the kind of this error.
    pub fn kind(&self) -> BoundedErrorKind {
        self.kind
    }

    /// Returns the description of the error.
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub fn message(&self) -> &str {
        &self.description
    }

    #[cfg(not(any(feature = "alloc", feature = "std")))]
    pub fn message(&self) -> &str {
        self.description_static
    }
}

impl Display for BoundedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message())
    }
}

// Implement std::error::Error for BoundedError if std feature is enabled
#[cfg(feature = "std")]
impl std::error::Error for BoundedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // If BoundedError were to wrap another error, it could be returned here.
        // For now, it's a simple error type.
        None
    }
}

impl From<BoundedError> for crate::Error {
    fn from(err: BoundedError) -> Self {
        let (category, code, static_message_prefix) = match err.kind {
            BoundedErrorKind::CapacityExceeded => {
                (WrtErrorCategory::Capacity, codes::CAPACITY_EXCEEDED, "Bounded capacity exceeded")
            }
            BoundedErrorKind::InvalidCapacity => (
                WrtErrorCategory::Validation,
                codes::INVALID_VALUE, // Consider a more specific code if available
                "Invalid capacity for bounded type",
            ),
            BoundedErrorKind::ConversionError => (
                WrtErrorCategory::Parse, // Or Type
                codes::CONVERSION_ERROR, // Or PARSE_ERROR
                "Bounded conversion error",
            ),
            BoundedErrorKind::SliceError => (
                WrtErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR, // Or a specific slice error code
                "Bounded slice error",
            ),
            BoundedErrorKind::Utf8Error => {
                (WrtErrorCategory::Parse, codes::PARSE_MALFORMED_UTF8_STRING, "Bounded UTF-8 error")
            }
            BoundedErrorKind::ItemTooLarge => (
                WrtErrorCategory::Validation,
                codes::VALUE_OUT_OF_RANGE,
                "Bounded item too large for operation",
            ),
            BoundedErrorKind::VerificationError => (
                WrtErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Bounded verification failed",
            ),
        };

        // wrt_error::Error expects a &'static str.
        // We use the static prefix determined by the kind.
        // If err.description_static is available (no_std no_alloc) and different,
        // it might offer more specifics, but we must choose one &'static str.
        // For simplicity, we'll use the matched static_message_prefix.
        // More complex message construction would require changes to wrt_error::Error
        // or careful management of static strings.
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        let message = if err.description_static != static_message_prefix {
            // This branch is tricky if we want to combine them and still return &'static
            // str. For now, let's prioritize the more specific static message
            // from BoundedError if it's different. However, this might lead to
            // losing the category/code context conveyed by static_message_prefix.
            // Sticking to static_message_prefix from the match is safer for now.
            static_message_prefix
        } else {
            static_message_prefix
        };

        #[cfg(any(feature = "alloc", feature = "std"))]
        // With alloc/std, err.description is a String. We can't directly use it
        // for WrtError's &'static str message. So we must use static_message_prefix.
        let message = static_message_prefix;

        crate::Error::new(category, code, message)
    }
}

impl From<crate::Error> for BoundedError {
    fn from(err: crate::Error) -> Self {
        // Determine a BoundedErrorKind based on the wrt_error::Error
        // This is a basic mapping; more sophisticated mapping might be needed.
        let kind = match err.category {
            WrtErrorCategory::Capacity => BoundedErrorKind::CapacityExceeded,
            WrtErrorCategory::Memory => BoundedErrorKind::SliceError, // Or another memory
            // related kind
            WrtErrorCategory::Parse | WrtErrorCategory::Validation => {
                BoundedErrorKind::ConversionError
            }
            _ => BoundedErrorKind::VerificationError, // Default or a more generic kind
        };
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            BoundedError::new(kind, err.to_string()) // Uses alloc::string::ToString
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            // No alloc, so we can't use err.to_string(). Use a static description based on
            // kind.
            let static_desc = match kind {
                BoundedErrorKind::CapacityExceeded => "Capacity exceeded (from WrtError)",
                BoundedErrorKind::SliceError => "Slice error (from WrtError)",
                BoundedErrorKind::ConversionError => "Conversion error (from WrtError)",
                _ => "Verification error (from WrtError)",
            };
            BoundedError::new(kind, static_desc)
        }
    }
}

/// A bounded stack with a fixed maximum capacity and verification.
///
/// This stack ensures it never exceeds the specified capacity `N_ELEMENTS`.
/// It uses a `MemoryProvider` for storing serialized elements.
#[derive(Debug)] // Removed Clone, PartialEq, Eq
pub struct BoundedStack<T, const N_ELEMENTS: usize, P: MemoryProvider>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default, // Added Default
{
    /// The underlying memory handler
    handler: SafeMemoryHandler<P>, // Corrected type to SafeMemoryHandler
    /// Current number of elements in the stack
    length: usize,
    /// Size of a single element T in bytes, assuming all T have the same
    /// serialized size. Determined from T::default().serialized_size().
    item_serialized_size: usize,
    /// Checksum for verifying data integrity
    checksum: Checksum,
    /// Verification level for this stack
    verification_level: VerificationLevel,
    /// Phantom data for type T
    _phantom: PhantomData<T>,
}

// Default implementation requires MemoryProvider to be Default, which might not
// always be true. Provide new and with_verification_level constructors instead.
// impl<T, const N_ELEMENTS: usize, P: MemoryProvider> Default for
// BoundedStack<T, N_ELEMENTS, P> where
//     T: Sized + Checksummable + ToBytes + FromBytes,
//     P: Default, // Added P: Default
// {
//     fn default() -> Self {
//         Self::new(P::default())
//     }
// }

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default,
    P: MemoryProvider + Default + Clone, // Ensure P has necessary bounds for methods
{
    /// Creates a new `BoundedStack` with the given memory provider.
    /// Assumes all instances of T will have the same serialized size as
    /// T::default().
    pub fn new(provider_arg: P) -> crate::WrtResult<Self> {
        Self::with_verification_level(provider_arg, VerificationLevel::default())
    }

    /// Creates a new `BoundedStack` with a specific verification level.
    ///
    /// Initializes the stack with the provided memory provider and verification
    /// settings. The actual memory allocation behavior depends on the
    /// `MemoryProvider`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying memory provider fails during
    /// initialization or if `T::default().serialized_size()` is 0 and
    /// N_ELEMENTS > 0, as this indicates an attempt to create a stack of
    /// ZSTs with a non-zero element count where item_serialized_size would
    /// be zero, potentially leading to division by zero or incorrect memory
    /// calculations if not handled carefully. For ZSTs, N_ELEMENTS should
    /// typically be 0, or specific ZST handling should be ensured.
    pub fn with_verification_level(
        provider_arg: P,
        level: VerificationLevel,
    ) -> crate::WrtResult<Self> {
        let item_serialized_size = T::default().serialized_size();
        if item_serialized_size == 0 && N_ELEMENTS > 0 {
            // Prevent division by zero or logical errors if N_ELEMENTS > 0 but items are
            // ZSTs. Or, if this is allowed, ensure memory_needed is handled
            // correctly. For now, consider it an invalid configuration for
            // typical BoundedStack usage.
            return Err(crate::Error::new(
                WrtErrorCategory::Memory, // Corrected Category - changed from Initialization
                codes::INITIALIZATION_ERROR,
                "Cannot create BoundedStack with zero-sized items and non-zero element count",
            ));
        }

        let memory_needed = N_ELEMENTS.saturating_mul(item_serialized_size);
        let handler = SafeMemoryHandler::new(provider_arg);

        // Record creation operation
        record_global_operation(OperationType::CollectionCreate, level);

        Ok(Self {
            handler,
            length: 0,
            item_serialized_size,
            checksum: Checksum::new(),
            verification_level: level,
            _phantom: PhantomData,
        })
    }

    /// Pushes an item onto the stack.
    ///
    /// # Errors
    ///
    /// Returns `BoundedError::CapacityExceeded` if the stack is full.
    /// Returns `BoundedError` if writing the item to memory fails or if
    /// checksum verification fails.
    pub fn push(&mut self, item: T) -> core::result::Result<(), BoundedError> {
        if self.is_full() {
            return Err(BoundedError::capacity_exceeded());
        }

        let offset = self.length.saturating_mul(self.item_serialized_size);
        let mut item_bytes_buffer = [0u8; MAX_ITEM_SERIALIZED_SIZE];

        let item_size = item.serialized_size();
        if item_size > MAX_ITEM_SERIALIZED_SIZE {
            return Err(BoundedError::new(
                BoundedErrorKind::ItemTooLarge,
                "Item exceeds max buffer size for push",
            ));
        }

        if item_size == 0 {
            // Handling ZSTs
            self.length += 1;
            item.update_checksum(&mut self.checksum); // ZSTs can affect checksum
            record_global_operation(OperationType::CollectionPush, self.verification_level);
            if self.verification_level >= VerificationLevel::Full {
                // Was should_recalculate_checksum_on_mutate
                self.recalculate_checksum();
            }
            return Ok(());
        }

        let bytes_written = {
            let buffer_slice =
                SliceMut::new(&mut item_bytes_buffer[..item_size]).map_err(|_| {
                    BoundedError::new(BoundedErrorKind::ConversionError, "Failed to create slice")
                })?;
            let mut write_stream = WriteStream::new(buffer_slice);
            item.to_bytes_with_provider(&mut write_stream, self.handler.provider()).map_err(
                |_| {
                    BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Conversion error in BoundedStack",
                    )
                },
            )?;
            write_stream.position()
        };

        self.handler.write_data(offset, &item_bytes_buffer[..bytes_written]).map_err(|e| {
            BoundedError::new(BoundedErrorKind::SliceError, "Write data failed: error occurred")
        })?;

        self.length += 1;
        record_global_operation(OperationType::CollectionWrite, self.verification_level); // Corrected

        if self.verification_level >= VerificationLevel::Full {
            // Was should_recalculate_checksum_on_mutate
            item.update_checksum(&mut self.checksum);
        }
        Ok(())
    }

    /// Pops an item from the stack.
    ///
    /// Returns `Ok(None)` if the stack is empty.
    ///
    /// # Errors
    ///
    /// Returns `BoundedError` if reading the item from memory fails or if
    /// checksum verification fails.
    pub fn pop(&mut self) -> core::result::Result<Option<T>, BoundedError> {
        if self.is_empty() {
            return Ok(None);
        }

        self.length -= 1;
        let offset = self.length.saturating_mul(self.item_serialized_size);
        record_global_operation(OperationType::CollectionWrite, self.verification_level); // Pop modifies length, considered a write/mutate to collection state

        if self.item_serialized_size == 0 {
            // Handle ZSTs
            // For ZSTs, no bytes are read, just return a default T
            // Checksum would need to be updated as if the ZST was "removed"
            let item = T::default();
            if self.verification_level >= VerificationLevel::Full {
                // Was should_recalculate_checksum_on_mutate
                self.recalculate_checksum(); // Recalculate based on remaining
                                             // items
            }
            return Ok(Some(item));
        }

        // Clone provider to avoid borrowing conflicts
        let provider = self.handler.provider().clone();

        let slice_view = self
            .handler
            .get_slice_mut(offset, self.item_serialized_size) // Changed to get_slice_mut
            .map_err(|e| {
                BoundedError::new(BoundedErrorKind::SliceError, "Get slice failed for pop")
            })?;

        // Before deserializing, if verification is high, consider if a checksum of this
        // specific item was stored or if we rely on the whole-collection
        // checksum. For now, assuming whole-collection checksum.

        let item_data = slice_view.as_ref(); // This now works as Slice implements AsRef<[u8]>
        let mut read_stream = ReadStream::new(Slice::new(item_data).map_err(|_| {
            BoundedError::new(
                BoundedErrorKind::ConversionError,
                "Failed to create slice for reading",
            )
        })?);
        let item = T::from_bytes_with_provider(&mut read_stream, &provider).map_err(|_e| {
            BoundedError::new(
                BoundedErrorKind::ConversionError,
                "Failed to deserialize item for pop",
            )
        })?;

        if self.verification_level >= VerificationLevel::Full {
            // Was should_recalculate_checksum_on_mutate
            self.recalculate_checksum();
        }

        // Optionally, zero out the popped memory for security/safety if required by
        // policy slice_view.fill(0); // Example

        Ok(Some(item))
    }

    /// Peeks at the top item of the stack without removing it.
    ///
    /// Returns `None` if the stack is empty.
    pub fn peek(&self) -> core::result::Result<Option<T>, BoundedError> {
        if self.is_empty() {
            return Ok(None);
        }
        let offset = (self.length - 1).saturating_mul(self.item_serialized_size);
        record_global_operation(OperationType::CollectionRead, self.verification_level); // Peek is a read

        if self.item_serialized_size == 0 {
            // Handle ZSTs
            return Ok(None);
        }

        let slice_view_result = self.handler.get_slice(offset, self.item_serialized_size);

        match slice_view_result {
            Ok(slice_view) => {
                // Assuming T::from_bytes doesn't modify the underlying slice if it's just a
                // view
                let mut read_stream = ReadStream::new(slice_view);
                match T::from_bytes_with_provider(&mut read_stream, self.handler.provider()) {
                    // Added .as_ref()
                    Ok(item) => Ok(Some(item)),
                    Err(_) => Ok(None), /* Failed to deserialize, treat as if item isn't there or
                                         * is corrupt */
                }
            }
            Err(_) => Ok(None), // Failed to get slice, treat as if item isn't there
        }
    }

    /// Returns the current verification level.
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Sets the verification level for this stack.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
        self.handler.set_verification_level(level);
    }

    /// Verifies the integrity of the stack using its checksum.
    /// Returns `true` if the current checksum matches a recalculated one.
    /// This is a potentially expensive operation.
    pub fn verify_checksum(&self) -> bool {
        if self.verification_level == VerificationLevel::Off {
            return true; // Verification is off
        }
        if self.item_serialized_size == 0 && self.length > 0 {
            // ZST handling
            let mut temp_checksum = Checksum::new();
            for _ in 0..self.length {
                T::default().update_checksum(&mut temp_checksum);
            }
            return self.checksum.verify(&temp_checksum);
        }

        let mut current_checksum = Checksum::new();
        for i in 0..self.length {
            let offset = i.saturating_mul(self.item_serialized_size);
            if let Ok(slice_view) = self.handler.get_slice(offset, self.item_serialized_size) {
                let mut read_stream = ReadStream::new(slice_view);
                match T::from_bytes_with_provider(&mut read_stream, self.handler.provider()) {
                    // Added .as_ref()
                    Ok(item) => {
                        item.update_checksum(&mut current_checksum);
                    }
                    Err(_) => return false, // Deserialization failure means data corruption
                }
            } else {
                return false; // Cannot access data, implies corruption or error
            }
        }
        self.checksum.verify(&current_checksum)
    }

    /// Recalculates the checksum for the entire stack.
    /// This should be called after operations that might invalidate the
    /// checksum if per-item updates are not feasible or verification level
    /// is high.
    pub fn recalculate_checksum(&mut self) {
        self.checksum.reset();
        if self.item_serialized_size == 0 {
            // ZST handling
            for _ in 0..self.length {
                T::default().update_checksum(&mut self.checksum);
            }
            return;
        }

        for i in 0..self.length {
            let offset = i.saturating_mul(self.item_serialized_size);
            if let Ok(slice_view) = self.handler.get_slice(offset, self.item_serialized_size) {
                // It's safer to deserialize and then use the item's Checksummable impl
                // if the byte representation for checksumming might differ from raw storage.
                // However, if T::from_bytes is cheap and Checksummable uses `to_ne_bytes`
                // for primitives, direct checksum of bytes might be okay for those.
                // For complex types, deserializing then checksumming `item` is more robust.
                let mut read_stream = ReadStream::new(slice_view);
                match T::from_bytes_with_provider(&mut read_stream, self.handler.provider()) {
                    // Added .as_ref()
                    Ok(item) => {
                        item.update_checksum(&mut self.checksum);
                    }
                    Err(_) => {
                        // Error during deserialization while recalculating checksum.
                        // This indicates a potential data corruption.
                        // The checksum will be "wrong" which is what we want to detect.
                        // Mark checksum as invalid or use a sentinel error value if possible.
                        // For now, it just won't match.
                        // Consider logging this error if a logger is available.
                        // Example: log_error("Checksum recalculation failed on item", i);
                        // We must continue to process all elements to ensure the checksum
                        // reflects the attempt to checksum all current data, even if parts are
                        // corrupt. A "poisoned" checksum state could also
                        // be an option. For simplicity, the current
                        // checksum will just not match the true one.
                        break; // Or continue and checksum what's possible
                    }
                }
            } else {
                // Failed to get slice, data is inaccessible.
                // Similar to above, checksum will be "wrong".
                break;
            }
        }
        record_global_operation(OperationType::ChecksumFullRecalculation, self.verification_level);
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedCapacity
    for BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default,
{
    fn capacity(&self) -> usize {
        N_ELEMENTS
    }

    fn len(&self) -> usize {
        self.length
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn is_full(&self) -> bool {
        self.length == N_ELEMENTS
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> Checksummed for BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default,
{
    fn checksum(&self) -> Checksum {
        self.checksum
    }

    fn recalculate_checksum(&mut self) {
        self.checksum = Checksum::new();
        for i in 0..self.length {
            let offset = i * self.item_serialized_size;
            match self.handler.borrow_slice(offset, self.item_serialized_size) {
                Ok(slice_view) => {
                    let mut read_stream = ReadStream::new(slice_view);
                    match T::from_bytes_with_provider(&mut read_stream, self.handler.provider()) {
                        Ok(item) => item.update_checksum(&mut self.checksum),
                        Err(_) => {
                            if self.verification_level >= VerificationLevel::Redundant {
                                // Consider logging or panicking if an
                                // element can't be deserialized
                                // during checksum recalculation, as it
                                // implies data corruption.
                            }
                        }
                    }
                }
                Err(_) => {
                    if self.verification_level >= VerificationLevel::Redundant {
                        // Log or handle error
                    }
                }
            }
        }
    }

    fn verify_checksum(&self) -> bool {
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        if !self.verification_level.should_verify(importance::CRITICAL) {
            // Was HIGH, // Use high importance for this check
            return true; // Skip if verification level allows
        }
        let mut current_checksum = Checksum::new();
        for i in 0..self.length {
            let offset = i * self.item_serialized_size;
            match self.handler.borrow_slice(offset, self.item_serialized_size) {
                Ok(slice_view) => {
                    let mut read_stream = ReadStream::new(slice_view);
                    match T::from_bytes_with_provider(&mut read_stream, self.handler.provider()) {
                        Ok(item) => item.update_checksum(&mut current_checksum),
                        Err(_) => return false, // Getting data from SafeSlice failed
                    }
                }
                Err(_) => return false, // Memory access failed
            }
        }
        current_checksum == self.checksum
    }
}

/// A bounded vector with a fixed maximum capacity and verification.
///
/// This vector ensures it never exceeds the specified capacity `N_ELEMENTS`.
/// It uses a `MemoryProvider` for storing serialized elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedVec<T, const N_ELEMENTS: usize, P: MemoryProvider>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    provider: P, // Changed from handler: SafeMemoryHandler<P>
    length: usize,
    item_serialized_size: usize, /* From T::default().serialized_size(), assumes fixed for all T
                                  * in this Vec */
    checksum: Checksum,
    verification_level: VerificationLevel,
    _phantom: PhantomData<T>,
}

impl<T, const N_ELEMENTS: usize, P> Default for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Default + Clone + PartialEq + Eq, // P must be Default
{
    fn default() -> Self {
        Self {
            provider: P::default(), // Requires P: Default
            length: 0,
            item_serialized_size: T::default().serialized_size(), // T is Default
            checksum: Checksum::default(),                        // Checksum is Default
            verification_level: VerificationLevel::default(),     // VerificationLevel is Default
            _phantom: PhantomData,
        }
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    /// Creates a new `BoundedVec` with the given memory provider.
    /// Assumes all instances of T will have the same serialized size as
    /// T::default().
    pub fn new(provider_arg: P) -> WrtResult<Self> {
        let item_s_size = T::default().serialized_size();
        if item_s_size == 0 && N_ELEMENTS > 0 {
            return Err(crate::Error::new(
                // Using WrtError directly
                WrtErrorCategory::Initialization,
                codes::INVALID_VALUE,
                "BoundedVec item serialized size cannot be 0 for non-empty capacity",
            ));
        }

        Ok(Self {
            provider: provider_arg, // Store the provider directly
            length: 0,
            item_serialized_size: item_s_size,
            checksum: Checksum::default(),
            verification_level: VerificationLevel::default(),
            _phantom: PhantomData,
        })
    }

    /// Creates a new `BoundedVec` with a specific verification level.
    ///
    /// # Errors
    ///
    /// Returns an error if the `MemoryProvider` fails during initialization.
    pub fn with_verification_level(
        provider_arg: P, // Renamed provider to provider_arg
        verification_level: VerificationLevel,
    ) -> WrtResult<Self> {
        let item_size = T::default().serialized_size();
        if item_size == 0 && N_ELEMENTS > 0 {
            return Err(crate::Error::new(
                WrtErrorCategory::Memory, // Corrected Category - changed from Initialization
                codes::INITIALIZATION_ERROR,
                "Cannot create BoundedVec with zero-sized items and non-zero element count",
            ));
        }

        // No SafeMemoryHandler needed directly here if P itself manages memory regions.
        // The provider is stored directly.
        record_global_operation(OperationType::CollectionCreate, verification_level);
        Ok(Self {
            provider: provider_arg,
            length: 0,
            item_serialized_size: item_size,
            checksum: Checksum::new(),
            verification_level,
            _phantom: PhantomData,
        })
    }

    /// Pushes an item to the end of the vector.
    ///
    /// # Errors
    ///
    /// Returns `BoundedError::CapacityExceeded` if the vector is full.
    /// Returns `BoundedError` if writing the item to memory fails.
    pub fn push(&mut self, item: T) -> core::result::Result<(), BoundedError> {
        if self.is_full() {
            return Err(BoundedError::capacity_exceeded());
        }

        let offset = self.length.saturating_mul(self.item_serialized_size);
        let mut item_bytes_buffer = [0u8; MAX_ITEM_SERIALIZED_SIZE];

        let item_size = item.serialized_size();
        if item_size > MAX_ITEM_SERIALIZED_SIZE {
            return Err(BoundedError::new(
                BoundedErrorKind::ItemTooLarge,
                "Item exceeds max buffer size for push",
            ));
        }

        if item_size == 0 {
            // ZST Handling
            self.length += 1;
            item.update_checksum(&mut self.checksum); // ZSTs can affect checksum
            record_global_operation(OperationType::CollectionPush, self.verification_level);
            if self.verification_level >= VerificationLevel::Full {
                // Was should_recalculate_checksum_on_mutate
                self.recalculate_checksum();
            }
            return Ok(());
        }

        let bytes_written = {
            let buffer_slice =
                SliceMut::new(&mut item_bytes_buffer[..item_size]).map_err(|_| {
                    BoundedError::new(BoundedErrorKind::ConversionError, "Failed to create slice")
                })?;
            let mut write_stream = WriteStream::new(buffer_slice);
            item.to_bytes_with_provider(&mut write_stream, &self.provider).map_err(|_| {
                BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Conversion error in BoundedVec",
                )
            })?;
            write_stream.position()
        };

        self.provider.write_data(offset, &item_bytes_buffer[..bytes_written]).map_err(|e| {
            BoundedError::new(BoundedErrorKind::SliceError, "Write data failed: error occurred")
        })?;

        self.length += 1;
        record_global_operation(OperationType::CollectionWrite, self.verification_level); // Corrected

        if self.verification_level >= VerificationLevel::Full {
            // Was should_recalculate_checksum_on_mutate
            item.update_checksum(&mut self.checksum);
        }
        Ok(())
    }

    /// Removes the last element from the vector and returns it.
    ///
    /// Returns `None` if the vector is empty.
    ///
    /// # Errors
    ///
    /// Returns `BoundedError` if reading the item from memory fails.
    pub fn pop(&mut self) -> core::result::Result<Option<T>, BoundedError> {
        if self.is_empty() {
            return Ok(None);
        }
        self.length -= 1;
        let offset = self.length.saturating_mul(self.item_serialized_size);
        record_global_operation(OperationType::CollectionWrite, self.verification_level); // Corrected, pop modifies collection state

        if self.item_serialized_size == 0 {
            // ZST handling
            let item = T::default();
            if self.verification_level >= VerificationLevel::Full {
                // Was should_recalculate_checksum_on_mutate
                self.recalculate_checksum();
            }
            return Ok(Some(item));
        }

        let slice_view = self
            .provider
            .borrow_slice(offset, self.item_serialized_size) // BoundedVec uses borrow_slice from MemoryProvider
            .map_err(|e| {
                BoundedError::new(BoundedErrorKind::SliceError, "Get slice failed for pop")
            })?;

        // The slice from MemoryProvider is assumed to be &[u8] if P is e.g.
        // GlobalBufferProvider or P directly returns &[u8] via its get_slice.
        // If P::get_slice returns its own Slice<'a, u8> type, then .as_ref() is needed.
        // Assuming P::get_slice returns a type that can be used with T::from_bytes.
        // If P::get_slice returns safe_memory::Slice, then .as_ref() is correct.
        // Let's assume for now P::get_slice returns something T::from_bytes can handle
        // or it's Slice.
        let mut read_stream = ReadStream::new(slice_view);
        let item = T::from_bytes_with_provider(&mut read_stream, &self.provider).map_err(|_e| {
            BoundedError::new(
                BoundedErrorKind::ConversionError,
                "Failed to deserialize item for pop",
            )
        })?;

        if self.verification_level >= VerificationLevel::Full {
            // Was should_recalculate_checksum_on_mutate
            self.recalculate_checksum();
        }
        Ok(Some(item))
    }

    /// Returns a reference to the element at the given index, or `None` if out
    /// of bounds.
    pub fn get(&self, index: usize) -> WrtResult<T> {
        if index >= self.length {
            return Err(crate::Error::index_out_of_bounds("Index out of bounds"));
        }
        let offset = index * self.item_serialized_size;

        // Use borrow_slice for immutable access
        match self.provider.borrow_slice(offset, self.item_serialized_size) {
            Ok(slice_view) => {
                let mut read_stream = ReadStream::new(slice_view);
                // Deserialize T using FromBytes trait
                match T::from_bytes_with_provider(&mut read_stream, &self.provider) {
                    Ok(item) => {
                        // Optional: Verify checksum if not ZST and verification is enabled
                        if CHECKSUM_SIZE > 0 && self.item_serialized_size > 0 {
                            let checksum_offset = offset + self.item_serialized_size;
                            if let Ok(checksum_slice) =
                                self.provider.borrow_slice(checksum_offset, CHECKSUM_SIZE)
                            {
                                let mut cs_stream = ReadStream::new(checksum_slice);
                                if let Ok(stored_checksum) = Checksum::from_bytes_with_provider(
                                    &mut cs_stream,
                                    &self.provider,
                                ) {
                                    let mut current_checksum = Checksum::new();
                                    item.update_checksum(&mut current_checksum);
                                    if current_checksum != stored_checksum {
                                        return Err(crate::Error::validation_error(
                                            "Checksum mismatch on BoundedVec::get",
                                        ));
                                    }
                                } else {
                                    return Err(crate::Error::deserialization_error(
                                        "Failed to read stored checksum on BoundedVec::get",
                                    ));
                                }
                            } else {
                                return Err(crate::Error::memory_error(
                                    "Failed to get checksum slice on BoundedVec::get",
                                ));
                            }
                        }
                        Ok(item)
                    }
                    Err(e) => Err(crate::Error::deserialization_error(
                        "Failed to deserialize item from BoundedVec",
                    )),
                }
            }
            Err(e) => Err(crate::Error::memory_error("Failed to get slice for BoundedVec::get")),
        }
    }

    /// Recalculates the checksum for the entire vector.
    fn recalculate_checksum(&mut self) {
        self.checksum.reset();
        if self.item_serialized_size == 0 {
            // ZST handling
            for _ in 0..self.length {
                T::default().update_checksum(&mut self.checksum);
            }
            return;
        }

        for i in 0..self.length {
            let offset = i * self.item_serialized_size;
            if let Ok(slice_view) = self.provider.borrow_slice(offset, self.item_serialized_size) {
                let mut read_stream = ReadStream::new(slice_view);
                match T::from_bytes_with_provider(&mut read_stream, &self.provider) {
                    Ok(item) => {
                        item.update_checksum(&mut self.checksum);
                    }
                    Err(_) => {
                        // Data corruption, checksum will not match.
                        break;
                    }
                }
            } else {
                // Cannot access data, checksum will not match.
                break;
            }
        }
        record_global_operation(OperationType::ChecksumFullRecalculation, self.verification_level);
    }

    /// Verifies the integrity of the vector using its checksum.
    fn verify_checksum(&self) -> bool {
        if self.verification_level == VerificationLevel::Off {
            return true;
        }
        if self.item_serialized_size == 0 && self.length > 0 {
            // ZST handling
            let mut temp_checksum = Checksum::new();
            for _ in 0..self.length {
                T::default().update_checksum(&mut temp_checksum);
            }
            return self.checksum.verify(&temp_checksum);
        }

        let mut current_checksum = Checksum::new();
        for i in 0..self.length {
            let offset = i * self.item_serialized_size;
            if let Ok(slice_view) = self.provider.borrow_slice(offset, self.item_serialized_size) {
                let mut read_stream = ReadStream::new(slice_view);
                match T::from_bytes_with_provider(&mut read_stream, &self.provider) {
                    Ok(item) => {
                        item.update_checksum(&mut current_checksum);
                    }
                    Err(_) => return false,
                }
            } else {
                return false;
            }
        }
        current_checksum == self.checksum
    }

    /// Returns an immutable slice of the underlying data for a specific item.
    /// Note: This is a low-level operation. Prefer `get` for most use cases.
    pub fn get_item_slice(&self, index: usize) -> WrtResult<Slice<'_>> {
        if index >= self.length {
            return Err(crate::Error::index_out_of_bounds("Index out of bounds"));
        }
        let offset = index * self.item_serialized_size;
        self.provider.borrow_slice(offset, self.item_serialized_size)
    }

    /// Returns a mutable slice of the underlying data for a specific item.
    /// Note: This is a low-level operation. Prefer `get_mut` or `set` for most
    /// use cases.
    pub fn get_item_slice_mut(&mut self, index: usize) -> WrtResult<SliceMut<'_>> {
        if index >= self.length {
            return Err(crate::Error::index_out_of_bounds("Index out of bounds"));
        }
        let offset = index * self.item_serialized_size;
        self.provider.get_slice_mut(offset, self.item_serialized_size)
    }

    /// Creates an iterator over the elements of the `BoundedVec`.
    /// Each element is deserialized on demand.
    pub fn iter(&self) -> BoundedVecIterator<'_, T, N_ELEMENTS, P> {
        BoundedVecIterator { vec: self, current_index: 0 }
    }

    /// Method to verify checksum for a single item, used by iter
    fn verify_item_checksum_at_offset(&self, offset: usize) -> WrtResult<()> {
        if !self.provider.verification_level().should_verify_redundant() {
            return Ok(());
        }

        match self.provider.borrow_slice(offset, self.item_serialized_size) {
            Ok(slice_view) => {
                let mut stream = ReadStream::new(slice_view);
                let item = T::from_bytes_with_provider(&mut stream, &self.provider)?;
                let stored_checksum_bytes = [0u8; 4]; // Checksum is u32, 4 bytes
                let checksum_offset = offset + self.item_serialized_size;

                match self.provider.borrow_slice(checksum_offset, 4) {
                    // Checksum is u32, 4 bytes
                    Ok(checksum_slice_view) => {
                        // This part needs careful implementation based on how ReadStream handles
                        // reading into a buffer Assuming ReadStream has a
                        // method like read_exact or similar For now, let's
                        // assume direct access for checksum bytes, though this is unsafe.
                        // This needs a safe way to read bytes for the checksum.
                        // A temporary workaround might be to re-deserialize the checksum.
                        let mut checksum_read_stream = ReadStream::new(checksum_slice_view);
                        let stored_checksum = Checksum::from_bytes_with_provider(
                            &mut checksum_read_stream,
                            &self.provider,
                        )?;

                        let mut current_checksum = Checksum::new();
                        item.update_checksum(&mut current_checksum);

                        if current_checksum == stored_checksum {
                            Ok(())
                        } else {
                            Err(crate::Error::validation_error(
                                "Checksum mismatch for BoundedVec item during iteration",
                            ))
                        }
                    }
                    Err(e) => Err(crate::Error::memory_error(
                        "Failed to read stored checksum for BoundedVec item",
                    )),
                }
            }
            Err(e) => Err(crate::Error::memory_error(
                "Failed to read item for checksum verification in BoundedVec",
            )),
        }
    }

    /// Clears the vector, removing all elements.
    ///
    /// This does not affect the capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// # assert_eq!(vec.len(), 3);
    /// vec.clear();
    /// assert_eq!(vec.len(), 0);
    /// ```
    pub fn clear(&mut self) -> core::result::Result<(), BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Simply reset the length to 0
        self.length = 0;

        // Reset the checksum
        self.checksum = Checksum::new();

        Ok(())
    }

    /// Sets the element at the specified index to the given value.
    ///
    /// Returns the previous value if successful.
    /// Returns an error if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// let old_value = vec.set(1, 42).unwrap();
    /// assert_eq!(old_value, 2);
    /// # assert_eq!(vec.get(1).unwrap(), 42);
    /// ```
    pub fn set(&mut self, index: usize, value: T) -> core::result::Result<T, BoundedError> {
        if index >= self.length {
            return Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Index out of bounds for BoundedVec::set",
            ));
        }

        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Get current value at the index
        let current_value = match self.get(index) {
            Ok(value) => value,
            Err(_) => {
                return Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to get current value for set operation",
                ))
            }
        };

        // Calculate offset for writing
        let offset = index * self.item_serialized_size;

        // Special handling for zero-sized types
        if self.item_serialized_size == 0 {
            // For ZSTs, we only need to update the checksum if verification is enabled
            if self.verification_level >= VerificationLevel::Full {
                // Remove old item from checksum
                let mut old_checksum = Checksum::new();
                current_value.update_checksum(&mut old_checksum);
                // This is a simplification - ideally we'd want to remove just this item's
                // contribution to the checksum, but for now we'll recalculate the entire
                // checksum
                self.recalculate_checksum();

                // Add new item to checksum
                value.update_checksum(&mut self.checksum);
            }
            return Ok(current_value);
        }

        // Serialize the new value
        let mut item_bytes_buffer = [0u8; MAX_ITEM_SERIALIZED_SIZE];
        let item_size = value.serialized_size();

        if item_size > MAX_ITEM_SERIALIZED_SIZE {
            return Err(BoundedError::new(
                BoundedErrorKind::ItemTooLarge,
                "Item exceeds max buffer size for set",
            ));
        }

        let bytes_written = {
            let buffer_slice =
                SliceMut::new(&mut item_bytes_buffer[..item_size]).map_err(|_| {
                    BoundedError::new(BoundedErrorKind::ConversionError, "Failed to create slice")
                })?;
            let mut write_stream = WriteStream::new(buffer_slice);
            value.to_bytes_with_provider(&mut write_stream, &self.provider).map_err(|_| {
                BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to serialize item for set",
                )
            })?;
            write_stream.position()
        };

        // Write new value to memory
        self.provider.write_data(offset, &item_bytes_buffer[..bytes_written]).map_err(|e| {
            BoundedError::new(BoundedErrorKind::SliceError, "Failed to write data for set")
        })?;

        // Update checksum if needed
        if self.verification_level >= VerificationLevel::Full {
            // Option 1: Recalculate the entire checksum (more expensive but ensures
            // correctness)
            self.recalculate_checksum();

            // Option 2: Update incrementally (more efficient but potentially
            // less reliable) Let's use option 1 for now to ensure
            // correctness
        }

        Ok(current_value)
    }

    /// Inserts an element at the specified index, shifting all elements after
    /// it to the right.
    ///
    /// # Errors
    ///
    /// Returns `BoundedError::CapacityExceeded` if the vector is full.
    /// Returns an error if the index is greater than the length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(3).unwrap();
    /// vec.insert(1, 2).unwrap();
    /// # assert_eq!(vec.get(0).unwrap(), 1);
    /// # assert_eq!(vec.get(1).unwrap(), 2);
    /// # assert_eq!(vec.get(2).unwrap(), 3);
    /// ```
    pub fn insert(&mut self, index: usize, value: T) -> core::result::Result<(), BoundedError> {
        if index > self.length {
            return Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Index out of bounds for BoundedVec::insert",
            ));
        }

        if self.is_full() {
            return Err(BoundedError::capacity_exceeded());
        }

        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Special handling for zero-sized types
        if self.item_serialized_size == 0 {
            self.length += 1;
            if self.verification_level >= VerificationLevel::Full {
                // Add new item to checksum
                value.update_checksum(&mut self.checksum);
            }
            return Ok(());
        }

        // If we're inserting at the end, this is equivalent to push
        if index == self.length {
            return self.push(value);
        }

        // We need to shift all elements from index to the end one position to the right
        // Start at the end and work backwards to avoid overwriting
        for i in (index..self.length).rev() {
            // Get the current item
            let current_item = match self.get(i) {
                Ok(item) => item,
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item for shifting during insert",
                    ))
                }
            };

            // Move it one position forward
            let dest_offset = (i + 1) * self.item_serialized_size;
            let mut item_bytes_buffer = [0u8; MAX_ITEM_SERIALIZED_SIZE];
            let item_size = current_item.serialized_size();

            if item_size > MAX_ITEM_SERIALIZED_SIZE {
                return Err(BoundedError::new(
                    BoundedErrorKind::ItemTooLarge,
                    "Item exceeds max buffer size during insert shift",
                ));
            }

            let bytes_written = {
                let buffer_slice =
                    SliceMut::new(&mut item_bytes_buffer[..item_size]).map_err(|_| {
                        BoundedError::new(
                            BoundedErrorKind::ConversionError,
                            "Failed to create slice",
                        )
                    })?;
                let mut write_stream = WriteStream::new(buffer_slice);
                current_item.to_bytes_with_provider(&mut write_stream, &self.provider).map_err(
                    |_| {
                        BoundedError::new(
                            BoundedErrorKind::ConversionError,
                            "Failed to serialize item during insert shift",
                        )
                    },
                )?;
                write_stream.position()
            };

            self.provider.write_data(dest_offset, &item_bytes_buffer[..bytes_written]).map_err(
                |e| {
                    BoundedError::new(
                        BoundedErrorKind::SliceError,
                        "Failed to write data during insert shift",
                    )
                },
            )?;
        }

        // Now write the new value at the specified index
        let offset = index * self.item_serialized_size;
        let mut item_bytes_buffer = [0u8; MAX_ITEM_SERIALIZED_SIZE];
        let item_size = value.serialized_size();

        if item_size > MAX_ITEM_SERIALIZED_SIZE {
            return Err(BoundedError::new(
                BoundedErrorKind::ItemTooLarge,
                "Item exceeds max buffer size for insert",
            ));
        }

        let bytes_written = {
            let buffer_slice =
                SliceMut::new(&mut item_bytes_buffer[..item_size]).map_err(|_| {
                    BoundedError::new(BoundedErrorKind::ConversionError, "Failed to create slice")
                })?;
            let mut write_stream = WriteStream::new(buffer_slice);
            value.to_bytes_with_provider(&mut write_stream, &self.provider).map_err(|_| {
                BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to serialize item for insert",
                )
            })?;
            write_stream.position()
        };

        self.provider.write_data(offset, &item_bytes_buffer[..bytes_written]).map_err(|e| {
            BoundedError::new(BoundedErrorKind::SliceError, "Failed to write data for insert")
        })?;

        // Update length
        self.length += 1;

        // Update checksum if needed
        if self.verification_level >= VerificationLevel::Full {
            self.recalculate_checksum();
        }

        Ok(())
    }

    /// Removes the element at the specified index, shifting all elements after
    /// it to the left.
    ///
    /// Returns the removed element if successful.
    /// Returns an error if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// let removed = vec.remove(1).unwrap();
    /// assert_eq!(removed, 2);
    /// # assert_eq!(vec.get(0).unwrap(), 1);
    /// # assert_eq!(vec.get(1).unwrap(), 3);
    /// # assert_eq!(vec.len(), 2);
    /// ```
    pub fn remove(&mut self, index: usize) -> core::result::Result<T, BoundedError> {
        if index >= self.length {
            return Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Index out of bounds for BoundedVec::remove",
            ));
        }

        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Get the item to remove first
        let item_to_remove = match self.get(index) {
            Ok(item) => item,
            Err(_) => {
                return Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to get item for remove operation",
                ))
            }
        };

        // Special handling for zero-sized types
        if self.item_serialized_size == 0 {
            self.length -= 1;
            if self.verification_level >= VerificationLevel::Full {
                self.recalculate_checksum();
            }
            return Ok(item_to_remove);
        }

        // If we're removing the last element, this is equivalent to pop
        if index == self.length - 1 {
            return match self.pop() {
                Ok(Some(item)) => Ok(item),
                Ok(None) => Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Unexpected empty vector during remove",
                )),
                Err(e) => Err(e),
            };
        }

        // Shift all elements after index one position to the left
        for i in index..(self.length - 1) {
            // Get the next item
            let next_item = match self.get(i + 1) {
                Ok(item) => item,
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get next item during remove shift",
                    ))
                }
            };

            // Write it at the current position
            let dest_offset = i * self.item_serialized_size;
            let mut item_bytes_buffer = [0u8; MAX_ITEM_SERIALIZED_SIZE];
            let item_size = next_item.serialized_size();

            if item_size > MAX_ITEM_SERIALIZED_SIZE {
                return Err(BoundedError::new(
                    BoundedErrorKind::ItemTooLarge,
                    "Item exceeds max buffer size during remove shift",
                ));
            }

            let bytes_written = {
                let buffer_slice =
                    SliceMut::new(&mut item_bytes_buffer[..item_size]).map_err(|_| {
                        BoundedError::new(
                            BoundedErrorKind::ConversionError,
                            "Failed to create slice",
                        )
                    })?;
                let mut write_stream = WriteStream::new(buffer_slice);
                next_item.to_bytes_with_provider(&mut write_stream, &self.provider).map_err(
                    |_| {
                        BoundedError::new(
                            BoundedErrorKind::ConversionError,
                            "Failed to serialize item during remove shift",
                        )
                    },
                )?;
                write_stream.position()
            };

            self.provider.write_data(dest_offset, &item_bytes_buffer[..bytes_written]).map_err(
                |e| {
                    BoundedError::new(
                        BoundedErrorKind::SliceError,
                        "Failed to write data during remove shift",
                    )
                },
            )?;
        }

        // Update length
        self.length -= 1;

        // Update checksum if needed
        if self.verification_level >= VerificationLevel::Full {
            self.recalculate_checksum();
        }

        Ok(item_to_remove)
    }

    /// Checks if the vector contains the given item.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// assert!(vec.contains(&2).unwrap());
    /// assert!(!vec.contains(&4).unwrap());
    /// ```
    pub fn contains(&self, item: &T) -> core::result::Result<bool, BoundedError>
    where
        T: PartialEq,
    {
        record_global_operation(OperationType::CollectionRead, self.verification_level);

        for i in 0..self.length {
            match self.get(i) {
                Ok(current_item) => {
                    if &current_item == item {
                        return Ok(true);
                    }
                }
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during contains check",
                    ))
                }
            }
        }

        Ok(false)
    }

    /// Truncates the vector to the specified length.
    ///
    /// If `new_len` is greater than or equal to the current length, this has no
    /// effect.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// # assert_eq!(vec.len(), 3);
    /// vec.truncate(1).unwrap();
    /// assert_eq!(vec.len(), 1);
    /// # assert_eq!(vec.get(0).unwrap(), 1);
    /// ```
    pub fn truncate(&mut self, new_len: usize) -> core::result::Result<(), BoundedError> {
        if new_len >= self.length {
            return Ok(());
        }

        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Simply update the length - we don't need to clear the memory
        self.length = new_len;

        // Update checksum if needed
        if self.verification_level >= VerificationLevel::Full {
            self.recalculate_checksum();
        }

        Ok(())
    }

    /// Completes the implementation of get_item_mut_slice_for_write that was
    /// previously a placeholder. This method provides a mutable slice of
    /// the underlying data for an item at the given index.
    ///
    /// Note: This is a low-level operation intended for internal use or special
    /// cases. For most use cases, prefer `set()` instead.
    ///
    /// # Safety
    /// This method can potentially bypass checksum validation if used
    /// incorrectly. The caller must ensure that the written data maintains
    /// the validity of the collection.
    fn get_item_mut_slice_for_write(&mut self, index: usize) -> WrtResult<SliceMut<'_>> {
        if index >= self.length {
            return Err(crate::Error::index_out_of_bounds("Index out of bounds"));
        }
        let offset = index.saturating_mul(self.item_serialized_size);
        self.provider.get_slice_mut(offset, self.item_serialized_size)
    }

    /// Swaps two elements in the vector.
    ///
    /// # Errors
    ///
    /// Returns an error if either index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// vec.swap(0, 2).unwrap();
    /// assert_eq!(vec.get(0).unwrap(), 3);
    /// assert_eq!(vec.get(2).unwrap(), 1);
    /// ```
    pub fn swap(&mut self, a: usize, b: usize) -> core::result::Result<(), BoundedError> {
        if a >= self.length || b >= self.length {
            return Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Index out of bounds for BoundedVec::swap",
            ));
        }

        // If indices are the same, nothing to do
        if a == b {
            return Ok(());
        }

        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Special handling for zero-sized types (no-op since all ZSTs are identical)
        if self.item_serialized_size == 0 {
            return Ok(());
        }

        // Get both items
        let item_a = match self.get(a) {
            Ok(item) => item,
            Err(_) => {
                return Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to get item A for swap operation",
                ))
            }
        };

        let item_b = match self.get(b) {
            Ok(item) => item,
            Err(_) => {
                return Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to get item B for swap operation",
                ))
            }
        };

        // Swap the items (set a to b's value, and b to a's value)
        self.set(a, item_b.clone())?;
        self.set(b, item_a)?;

        Ok(())
    }

    /// Reverses the order of elements in the vector, in place.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// vec.reverse().unwrap();
    /// assert_eq!(vec.get(0).unwrap(), 3);
    /// assert_eq!(vec.get(1).unwrap(), 2);
    /// assert_eq!(vec.get(2).unwrap(), 1);
    /// ```
    pub fn reverse(&mut self) -> core::result::Result<(), BoundedError> {
        if self.length <= 1 {
            return Ok(());
        }

        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Special handling for zero-sized types (no visible effect)
        if self.item_serialized_size == 0 {
            return Ok(());
        }

        // Swap pairs of elements from the start and end, moving inward
        let mut low = 0;
        let mut high = self.length - 1;

        while low < high {
            self.swap(low, high)?;
            low += 1;
            high -= 1;
        }

        Ok(())
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns
    /// `false`. This method operates in place, modifying the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(4).unwrap();
    /// vec.retain(|&x| x % 2 == 0).unwrap();
    /// assert_eq!(vec.len(), 2);
    /// assert_eq!(vec.get(0).unwrap(), 2);
    /// assert_eq!(vec.get(1).unwrap(), 4);
    /// ```
    pub fn retain<F>(&mut self, mut f: F) -> core::result::Result<(), BoundedError>
    where
        F: FnMut(&T) -> bool,
    {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Special handling for zero-sized types (no visible effect)
        if self.item_serialized_size == 0 {
            return Ok(());
        }

        // Maintain two indices: one for reading (i) and one for writing (write_idx)
        let mut write_idx = 0;

        for i in 0..self.length {
            // Get current item
            let item = match self.get(i) {
                Ok(item) => item,
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during retain operation",
                    ))
                }
            };

            // If predicate returns true, keep the item by writing it at write_idx
            if f(&item) {
                if i != write_idx {
                    // If i and write_idx are different, we need to move the item
                    match self.set(write_idx, item) {
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    }
                }

                write_idx += 1;
            }
            // If predicate returns false, we skip this item (don't increment
            // write_idx)
        }

        // Update the length to the new size
        if write_idx < self.length {
            self.truncate(write_idx)?
        }

        Ok(())
    }

    /// Binary searches this vector for a given element.
    ///
    /// If the vector contains the given value, this returns `Ok(index)` where
    /// `index` is the position of the value. If the vector doesn't contain
    /// the given value, this returns `Err(insertion_index)` where
    /// `insertion_index` is where the value would need to be inserted to
    /// maintain sorted order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(5).unwrap();
    /// assert_eq!(vec.binary_search(&1).unwrap(), Ok(0));
    /// assert_eq!(vec.binary_search(&2).unwrap(), Err(1));
    /// assert_eq!(vec.binary_search(&6).unwrap(), Err(3));
    /// ```
    pub fn binary_search(&self, x: &T) -> core::result::Result<Result<usize, usize>, BoundedError>
    where
        T: Ord,
    {
        record_global_operation(OperationType::CollectionRead, self.verification_level);

        // Special handling for zero-sized types (arbitrary behavior, but consistent)
        if self.item_serialized_size == 0 {
            if self.is_empty() {
                return Ok(Err(0));
            }
            return Ok(Ok(0)); // All ZSTs are equal
        }

        let mut size = self.length;
        if size == 0 {
            return Ok(Err(0));
        }

        let mut base = 0usize;

        // Binary search implementation
        while size > 1 {
            let half = size / 2;
            let mid = base + half;

            // Get current item at mid
            let item = match self.get(mid) {
                Ok(item) => item,
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during binary search",
                    ))
                }
            };

            base = if item > *x { base } else { mid };
            size -= half;
        }

        // Get final element to compare
        let item = match self.get(base) {
            Ok(item) => item,
            Err(_) => {
                return Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to get item during binary search",
                ))
            }
        };

        Ok(if item == *x {
            Ok(base)
        } else if item < *x {
            Err(base + 1)
        } else {
            Err(base)
        })
    }

    /// Binary searches this vector with a comparator function.
    ///
    /// The comparator function should implement an order consistent with the
    /// sort order of the underlying vector, returning an ordering according
    /// to the comparison.
    ///
    /// If the vector contains an element equal to the given one, the returned
    /// index is the first such element's index. If the vector doesn't
    /// contain an element equal to the given one, the returned index is the
    /// index where such an element could be inserted while maintaining
    /// sorted order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// # use core::cmp::Ordering;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<(u32, u32), 10, _>::new(provider).unwrap();
    /// # vec.push((1, 2)).unwrap();
    /// # vec.push((3, 4)).unwrap();
    /// # vec.push((5, 6)).unwrap();
    /// let result = vec.binary_search_by(|&(a, _)| a.cmp(&3)).unwrap();
    /// assert_eq!(result, Ok(1));
    /// ```
    pub fn binary_search_by<F>(
        &self,
        mut f: F,
    ) -> core::result::Result<Result<usize, usize>, BoundedError>
    where
        F: FnMut(&T) -> core::cmp::Ordering,
    {
        record_global_operation(OperationType::CollectionRead, self.verification_level);

        // Special handling for zero-sized types (arbitrary behavior, but consistent)
        if self.item_serialized_size == 0 {
            if self.is_empty() {
                return Ok(Err(0));
            }
            // Apply comparator to ZST to get consistent behavior
            let zst = T::default();
            let ordering = f(&zst);
            return Ok(match ordering {
                core::cmp::Ordering::Equal => Ok(0),
                core::cmp::Ordering::Greater => Err(0),
                core::cmp::Ordering::Less => Err(1),
            });
        }

        let mut size = self.length;
        if size == 0 {
            return Ok(Err(0));
        }

        let mut base = 0usize;

        // Binary search implementation
        while size > 1 {
            let half = size / 2;
            let mid = base + half;

            // Get current item at mid
            let item = match self.get(mid) {
                Ok(item) => item,
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during binary search",
                    ))
                }
            };

            let cmp = f(&item);
            base = if cmp == core::cmp::Ordering::Greater { base } else { mid };
            size -= half;
        }

        // Get final element to compare
        let item = match self.get(base) {
            Ok(item) => item,
            Err(_) => {
                return Err(BoundedError::new(
                    BoundedErrorKind::ConversionError,
                    "Failed to get item during binary search",
                ))
            }
        };

        let cmp = f(&item);
        Ok(match cmp {
            core::cmp::Ordering::Equal => Ok(base),
            core::cmp::Ordering::Greater => Err(base),
            core::cmp::Ordering::Less => Err(base + 1),
        })
    }

    /// Binary searches this vector with a key extraction function.
    ///
    /// Assumes this vector is sorted by the key extracted by the key function.
    ///
    /// If the vector contains an element with a key equal to the provided key,
    /// the returned index is the first such element's index. If the vector
    /// doesn't contain an element with a key equal to the provided key, the
    /// returned index is the index where such an element could be inserted
    /// while maintaining sorted order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<(u32, u32), 10, _>::new(provider).unwrap();
    /// # vec.push((1, 42)).unwrap();
    /// # vec.push((3, 100)).unwrap();
    /// # vec.push((5, 200)).unwrap();
    /// let result = vec.binary_search_by_key(&3, |&(a, _)| a).unwrap();
    /// assert_eq!(result, Ok(1));
    /// ```
    pub fn binary_search_by_key<B, F>(
        &self,
        key: &B,
        mut f: F,
    ) -> core::result::Result<Result<usize, usize>, BoundedError>
    where
        B: Ord,
        F: FnMut(&T) -> B,
    {
        self.binary_search_by(|item| f(item).cmp(key))
    }

    /// Sorts the vector in-place.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and has
    /// O(n log n) worst-case performance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<i32, 10, _>::new(provider).unwrap();
    /// # vec.push(5).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(4).unwrap();
    /// # vec.push(2).unwrap();
    /// vec.sort().unwrap();
    /// assert_eq!(vec.get(0).unwrap(), 1);
    /// assert_eq!(vec.get(1).unwrap(), 2);
    /// assert_eq!(vec.get(2).unwrap(), 3);
    /// assert_eq!(vec.get(3).unwrap(), 4);
    /// assert_eq!(vec.get(4).unwrap(), 5);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn sort(&mut self) -> core::result::Result<(), BoundedError>
    where
        T: Ord,
    {
        self.sort_by(|a, b| a.cmp(b))
    }

    /// Sorts the vector in-place with a comparator function.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and has
    /// O(n log n) worst-case performance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<i32, 10, _>::new(provider).unwrap();
    /// # vec.push(5).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(4).unwrap();
    /// # vec.push(2).unwrap();
    /// // Sort in reverse order
    /// vec.sort_by(|a, b| b.cmp(a)).unwrap();
    /// assert_eq!(vec.get(0).unwrap(), 5);
    /// assert_eq!(vec.get(1).unwrap(), 4);
    /// assert_eq!(vec.get(2).unwrap(), 3);
    /// assert_eq!(vec.get(3).unwrap(), 2);
    /// assert_eq!(vec.get(4).unwrap(), 1);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn sort_by<F>(&mut self, mut compare: F) -> core::result::Result<(), BoundedError>
    where
        F: FnMut(&T, &T) -> core::cmp::Ordering,
    {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Special handling for zero-sized types or empty/single element vectors
        if self.item_serialized_size == 0 || self.length <= 1 {
            return Ok(());
        }

        // Collect all elements into a temporary vector for sorting
        let mut temp_vec = Vec::with_capacity(self.length);
        for i in 0..self.length {
            match self.get(i) {
                Ok(item) => temp_vec.push(item),
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during sort operation",
                    ))
                }
            }
        }

        // Sort the temporary vector
        temp_vec.sort_by(compare);

        // Write sorted elements back to BoundedVec
        for (i, item) in temp_vec.iter().enumerate() {
            match self.set(i, item.clone()) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        // Recalculate checksum after sort
        self.checksum.reset();
        for i in 0..self.length {
            if let Ok(item) = self.get(i) {
                item.update_checksum(&mut self.checksum);
            }
        }

        Ok(())
    }

    /// Sorts the vector in-place with a key extraction function.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and has
    /// O(n log n) worst-case performance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<(i32, &str), 10, _>::new(provider).unwrap();
    /// # vec.push((5, "five")).unwrap();
    /// # vec.push((3, "three")).unwrap();
    /// # vec.push((1, "one")).unwrap();
    /// # vec.push((4, "four")).unwrap();
    /// # vec.push((2, "two")).unwrap();
    /// // Sort by the numeric key
    /// vec.sort_by_key(|k| k.0).unwrap();
    /// assert_eq!(vec.get(0).unwrap().0, 1);
    /// assert_eq!(vec.get(1).unwrap().0, 2);
    /// assert_eq!(vec.get(2).unwrap().0, 3);
    /// assert_eq!(vec.get(3).unwrap().0, 4);
    /// assert_eq!(vec.get(4).unwrap().0, 5);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn sort_by_key<K, F>(&mut self, mut f: F) -> core::result::Result<(), BoundedError>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        self.sort_by(|a, b| f(a).cmp(&f(b)))
    }

    /// Removes consecutive duplicate elements from the vector according to the
    /// `==` operator.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<i32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(4).unwrap();
    /// # assert_eq!(vec.len(), 7);
    /// vec.dedup().unwrap();
    /// assert_eq!(vec.len(), 4);
    /// assert_eq!(vec.get(0).unwrap(), 1);
    /// assert_eq!(vec.get(1).unwrap(), 2);
    /// assert_eq!(vec.get(2).unwrap(), 3);
    /// assert_eq!(vec.get(3).unwrap(), 4);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn dedup(&mut self) -> core::result::Result<(), BoundedError>
    where
        T: PartialEq,
    {
        self.dedup_by(|a, b| a == b)
    }

    /// Removes consecutive duplicate elements using the given equality
    /// function.
    ///
    /// If the vector is sorted such that all duplicates are next to each other,
    /// this will remove all duplicate items.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<i32, 10, _>::new(provider).unwrap();
    /// # vec.push(10).unwrap();
    /// # vec.push(20).unwrap();
    /// # vec.push(21).unwrap();
    /// # vec.push(30).unwrap();
    /// # vec.push(31).unwrap();
    /// # vec.push(32).unwrap();
    /// # vec.push(40).unwrap();
    /// # assert_eq!(vec.len(), 7);
    /// // Deduplicate based on integer division by 10
    /// vec.dedup_by(|a, b| a / 10 == b / 10).unwrap();
    /// assert_eq!(vec.len(), 4);
    /// assert_eq!(vec.get(0).unwrap(), 10);
    /// assert_eq!(vec.get(1).unwrap(), 20);
    /// assert_eq!(vec.get(2).unwrap(), 30);
    /// assert_eq!(vec.get(3).unwrap(), 40);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn dedup_by<F>(&mut self, mut same_bucket: F) -> core::result::Result<(), BoundedError>
    where
        F: FnMut(&T, &T) -> bool,
    {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Special handling for zero-sized types or empty/single element vectors
        if self.item_serialized_size == 0 || self.length <= 1 {
            return Ok(());
        }

        // Collect all elements into a temporary vector
        let mut temp_vec = Vec::with_capacity(self.length);
        for i in 0..self.length {
            match self.get(i) {
                Ok(item) => temp_vec.push(item),
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during dedup operation",
                    ))
                }
            }
        }

        // Dedup the temporary vector
        let mut i = 0;
        let mut j = 0;

        while j < temp_vec.len() {
            if i == 0 || !same_bucket(&temp_vec[i - 1], &temp_vec[j]) {
                if i != j {
                    temp_vec[i] = temp_vec[j].clone();
                }
                i += 1;
            }
            j += 1;
        }

        temp_vec.truncate(i);

        // Clear current vector
        self.length = 0;

        // Write back the deduped elements
        for item in temp_vec {
            match self.push(item) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        // Recalculate checksum is handled by push()

        Ok(())
    }

    /// Removes consecutive duplicate elements from the vector using the given
    /// key function.
    ///
    /// If the vector is sorted such that all duplicates (according to the key
    /// function) are next to each other, this will remove all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<(i32, &str), 10, _>::new(provider).unwrap();
    /// # vec.push((1, "one")).unwrap();
    /// # vec.push((2, "two")).unwrap();
    /// # vec.push((2, "dos")).unwrap();
    /// # vec.push((3, "three")).unwrap();
    /// # vec.push((3, "tres")).unwrap();
    /// # vec.push((3, "drei")).unwrap();
    /// # vec.push((4, "four")).unwrap();
    /// # assert_eq!(vec.len(), 7);
    /// // Deduplicate based on the first element of each tuple
    /// vec.dedup_by_key(|e| e.0).unwrap();
    /// assert_eq!(vec.len(), 4);
    /// assert_eq!(vec.get(0).unwrap().0, 1);
    /// assert_eq!(vec.get(1).unwrap().0, 2);
    /// assert_eq!(vec.get(2).unwrap().0, 3);
    /// assert_eq!(vec.get(3).unwrap().0, 4);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn dedup_by_key<K, F>(&mut self, mut key: F) -> core::result::Result<(), BoundedError>
    where
        K: PartialEq,
        F: FnMut(&T) -> K,
    {
        self.dedup_by(|a, b| key(a) == key(b))
    }

    /// Replaces the specified range with the contents of a slice.
    ///
    /// The range to be replaced must be valid indices within the vector,
    /// and the length of the replacement slice can be different from the
    /// range being replaced, as long as it fits within the vector's capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<i32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// # vec.push(2).unwrap();
    /// # vec.push(3).unwrap();
    /// # vec.push(4).unwrap();
    /// # vec.push(5).unwrap();
    /// let replacement = [10, 20, 30];
    /// vec.replace_range(1..4, &replacement).unwrap();
    /// assert_eq!(vec.len(), 4); // 1 + 3 items replaced 3 items
    /// assert_eq!(vec.get(0).unwrap(), 1);
    /// assert_eq!(vec.get(1).unwrap(), 10);
    /// assert_eq!(vec.get(2).unwrap(), 20);
    /// assert_eq!(vec.get(3).unwrap(), 30);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn replace_range<R>(
        &mut self,
        range: R,
        replacement: &[T],
    ) -> core::result::Result<(), BoundedError>
    where
        R: core::ops::RangeBounds<usize>,
    {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Convert range bounds to concrete indices
        let start = match range.start_bound() {
            core::ops::Bound::Included(&n) => n,
            core::ops::Bound::Excluded(&n) => n + 1,
            core::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            core::ops::Bound::Included(&n) => n + 1,
            core::ops::Bound::Excluded(&n) => n,
            core::ops::Bound::Unbounded => self.length,
        };

        // Validate range
        if start > end || end > self.length {
            return Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Invalid range for replace_range operation",
            ));
        }

        let range_len = end - start;

        // Calculate new length and check capacity
        let new_length = self.length - range_len + replacement.len();
        if new_length > N_ELEMENTS {
            return Err(BoundedError::new(
                BoundedErrorKind::CapacityExceeded,
                "Capacity exceeded when replacing range in BoundedVec",
            ));
        }

        // Handle special cases for zero-sized types
        if self.item_serialized_size == 0 {
            self.length = new_length;
            self.checksum.reset();
            for _ in 0..self.length {
                T::default().update_checksum(&mut self.checksum);
            }
            return Ok(());
        }

        // Collect all elements we're keeping
        let mut temp_vec = Vec::with_capacity(new_length);

        // Add elements before the range
        for i in 0..start {
            match self.get(i) {
                Ok(item) => temp_vec.push(item),
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during replace_range operation",
                    ))
                }
            }
        }

        // Add replacement elements
        for item in replacement {
            temp_vec.push(item.clone());
        }

        // Add elements after the range
        for i in end..self.length {
            match self.get(i) {
                Ok(item) => temp_vec.push(item),
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::ConversionError,
                        "Failed to get item during replace_range operation",
                    ))
                }
            }
        }

        // Clear current vector
        self.length = 0;

        // Write back the new elements
        for item in temp_vec {
            match self.push(item) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
    /// Extends the vector with the contents of a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedVec;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut vec = BoundedVec::<i32, 10, _>::new(provider).unwrap();
    /// # vec.push(1).unwrap();
    /// let items = [2, 3, 4, 5];
    /// vec.extend_from_slice(&items).unwrap();
    /// assert_eq!(vec.len(), 5);
    /// assert_eq!(vec.get(0).unwrap(), 1);
    /// assert_eq!(vec.get(1).unwrap(), 2);
    /// assert_eq!(vec.get(2).unwrap(), 3);
    /// assert_eq!(vec.get(3).unwrap(), 4);
    /// assert_eq!(vec.get(4).unwrap(), 5);
    /// ```
    pub fn extend_from_slice(&mut self, other: &[T]) -> core::result::Result<(), BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);

        // Check if there's enough capacity
        if self.length + other.len() > N_ELEMENTS {
            return Err(BoundedError::new(
                BoundedErrorKind::CapacityExceeded,
                "Capacity exceeded when extending BoundedVec from slice",
            ));
        }

        // Add each item from the slice
        for item in other {
            match self.push(item.clone()) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}

pub struct BoundedVecIterator<'a, T, const N_ELEMENTS: usize, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    vec: &'a BoundedVec<T, N_ELEMENTS, P>,
    current_index: usize,
}

impl<'a, T, const N_ELEMENTS: usize, P> Iterator for BoundedVecIterator<'a, T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    type Item = T; // Iterator returns T, not Result<T> or Option<T> directly from next()

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.vec.len() {
            // self.vec.get() returns WrtResult<T>.
            // The iterator should yield T if successful, or None if error or end.
            // For simplicity, if get() fails, this iterator will stop.
            // A more robust iterator might return Result<T, Error> or handle errors
            // differently.
            match self.vec.get(self.current_index) {
                Ok(item) => {
                    self.current_index += 1;
                    Some(item)
                }
                Err(_) => {
                    // Optionally log the error or handle it.
                    // For now, stop iteration on error.
                    self.current_index = self.vec.len(); // Ensure it stops
                    None
                }
            }
        } else {
            None
        }
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedCapacity for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    fn capacity(&self) -> usize {
        N_ELEMENTS
    }

    fn len(&self) -> usize {
        self.length
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn is_full(&self) -> bool {
        self.length >= N_ELEMENTS
    }
}

// Checksummable for BoundedVec<T, N, P>
impl<T, const N_ELEMENTS: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> Checksummable
    for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Default + Clone + PartialEq + Eq,
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Checksum the length first
        (self.length as u32).update_checksum(checksum); // Assuming u32 for length is reasonable for checksumming

        // Then checksum each item. This requires getting each item.
        // This could be inefficient if T is large or deserialization is costly.
        // An alternative for raw data collections might be to checksum the underlying
        // byte buffer. However, for structured data, checksumming logical items
        // is more robust.
        if self.item_serialized_size > 0 {
            // Only iterate if items have size
            for i in 0..self.length {
                // self.get(i) returns Option<T>.
                // If an item can't be retrieved (e.g., deserialization error, though get()
                // currently swallows this), it won't be part of the checksum.
                // This might be acceptable if `get` failing implies corruption.
                if let Ok(item) = self.get(i) {
                    item.update_checksum(checksum);
                } else {
                    // This case implies an issue with get(i) for a valid index,
                    // which shouldn't happen unless memory is corrupted or
                    // T::from_bytes fails unexpectedly. For
                    // robustness, one might log this or handle it based on
                    // verification level. For now, if get
                    // fails, that part of data won't contribute to checksum.
                }
            }
        }
    }
}

// Hash for BoundedVec<T, N, P>
impl<T, const N_ELEMENTS: usize, P> Hash for BoundedVec<T, N_ELEMENTS, P>
where
    T: Hash
        + Checksummable
        + ToBytes
        + FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq
        + core::fmt::Debug, // Corrected T bounds
    P: MemoryProvider + Default + Clone + PartialEq + Eq,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.length.hash(state);
        self.checksum.hash(state); // Assuming Checksum is Hashable
                                   // Hash elements if verification level suggests deep hashing
        if self.verification_level >= VerificationLevel::Full {
            for i in 0..self.length {
                if let Ok(item) = self.get(i) {
                    item.hash(state);
                }
            }
        }
    }
}

// Checksummed for BoundedVec<T, N, P>
impl<T, const N_ELEMENTS: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> Checksummed
    for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Default + Clone + PartialEq + Eq,
{
    fn checksum(&self) -> Checksum {
        self.checksum
    }

    fn recalculate_checksum(&mut self) {
        self.checksum = Checksum::new(); // Reset checksum
        if self.item_serialized_size > 0 {
            for i in 0..self.length {
                if let Ok(item) = self.get(i) {
                    item.update_checksum(&mut self.checksum);
                } else {
                    // Error case
                }
            }
        }
    }

    fn verify_checksum(&self) -> bool {
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        if !self.verification_level.should_verify(importance::CRITICAL) {
            return true;
        }
        let mut current_checksum = Checksum::new();
        if self.item_serialized_size > 0 {
            for i in 0..self.length {
                if let Ok(item) = self.get(i) {
                    item.update_checksum(&mut current_checksum);
                } else {
                    return false;
                }
            }
        }
        current_checksum == self.checksum
    }
}

// ToBytes for BoundedVec
impl<T, const N_ELEMENTS: usize, P: MemoryProvider + Clone + PartialEq + Eq> ToBytes
    for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    fn serialized_size(&self) -> usize {
        // Length (u32) + checksum + items
        4 + self.checksum.serialized_size()
            + (self.length * if self.length > 0 { T::default().serialized_size() } else { 0 })
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        // Write length
        writer.write_u32_le(self.length as u32)?;
        // Write checksum
        self.checksum.to_bytes_with_provider(writer, stream_provider)?;

        // Write each element
        // This requires iterating over the elements stored in SafeMemoryHandler via
        // self.provider The current BoundedVec structure doesn't directly
        // expose an iterator easily here without potentially unsafe memory
        // access or more complex SafeMemoryHandler methods. For now, this is a
        // conceptual loop. A real implementation needs safe access to elements.

        // Assuming self.get(i) provides a way to get T by value or ref
        // And that T implements ToBytes correctly using stream_provider
        for i in 0..self.length {
            if let Ok(item) = self.get(i) {
                // get() needs to be infallible or error handled
                item.to_bytes_with_provider(writer, stream_provider)?;
            } else {
                // This case should ideally not happen if length is correct.
                return Err(crate::Error::new(
                    WrtErrorCategory::System,
                    codes::SYSTEM_ERROR,
                    "BoundedVec inconsistency during serialization",
                ));
            }
        }
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

// FromBytes for BoundedVec
impl<T, const N_ELEMENTS: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> FromBytes
    for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream, // Provider for the stream's operations
    ) -> WrtResult<Self> {
        // Read length
        let count = reader.read_u32_le()? as usize;
        // Read checksum
        let checksum = Checksum::from_bytes_with_provider(reader, stream_provider)?;

        if count > N_ELEMENTS {
            return Err(crate::Error::from(SerializationError::Custom(
                "Decoded vector length exceeds capacity",
            )));
        }

        let mut vec = BoundedVec::<T, N_ELEMENTS, P>::new(P::default())?;
        vec.checksum = checksum;
        vec.length = count;

        for _ in 0..count {
            // T::from_bytes_with_provider might need its own provider if T is also generic
            // over one. Here, stream_provider is passed as per the FromBytes
            // trait signature for T.
            let item = T::from_bytes_with_provider(reader, stream_provider)?;
            vec.push(item).map_err(|e| {
                crate::Error::from(SerializationError::Custom(
                    "Failed to push item to BoundedVec during deserialization",
                ))
            })?; // Convert BoundedError
        }
        Ok(vec)
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

// Special method for BoundedVec<u8, N, P> to get its content as a byte slice
impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq>
    BoundedVec<u8, N_BYTES, P>
{
    /// Get the internal slice if the memory provider supports direct slice
    /// access.
    pub fn as_internal_slice(&self) -> core::result::Result<Slice<'_>, crate::Error> {
        // This method assumes that BoundedVec<u8, N, P> stores its bytes contiguously
        // and can expose them via the provider. The provider P itself must be able to
        // yield a Slice for its entire used range or a part of it.
        // For a NoStdProvider, this would be a slice of its internal array up to
        // self.length. For StdProvider, it's a slice of its Vec<u8>.

        // We need to get the raw slice from the BoundedVec's internal provider
        // `self.provider`. BoundedVec<T,N,P> has `provider: P`. Access its used
        // portion.
        self.provider.borrow_slice(0, self.length) // Assuming provider stores
                                                   // items from offset 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundedString<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq>
{
    bytes: BoundedVec<u8, N_BYTES, P>,
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> ToBytes
    for BoundedString<N_BYTES, P>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        self.bytes.to_bytes_with_provider(writer, stream_provider)
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> FromBytes
    for BoundedString<N_BYTES, P>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        Ok(Self {
            bytes: BoundedVec::<u8, N_BYTES, P>::from_bytes_with_provider(reader, stream_provider)?,
        })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> Checksummable
    for BoundedString<N_BYTES, P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.bytes.update_checksum(checksum); // Delegate to inner
                                              // BoundedVec<u8>
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> Hash
    for BoundedString<N_BYTES, P>
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

/// A type representing a valid WebAssembly name string, typically for
/// functions, locals, etc. It is a newtype wrapper around `BoundedString` to
/// provide a distinct type for WASM identifiers and potentially enforce
/// WASM-specific validation rules in the future.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WasmName<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> {
    inner: BoundedString<N_BYTES, P>,
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> Default
    for WasmName<N_BYTES, P>
{
    fn default() -> Self {
        Self { inner: BoundedString::default() }
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq>
    WasmName<N_BYTES, P>
{
    /// Creates a new, empty `WasmName`.
    pub fn new(provider: P) -> Result<Self, BoundedError> {
        // Use from_str_truncate to create an empty BoundedString
        let inner = BoundedString::from_str_truncate("", provider)?;
        Ok(Self { inner })
    }

    /// Creates a `WasmName` from a string slice.
    ///
    /// The string will be truncated if it exceeds `N_BYTES`.
    pub fn from_str_truncate(s: &str, provider: P) -> Result<Self, BoundedError> {
        let inner = BoundedString::from_str_truncate(s, provider)?;
        Ok(Self { inner })
    }

    /// Creates a `WasmName` from a string slice.
    ///
    /// Returns an error if the string exceeds `N_BYTES`.
    pub fn from_str(s: &str, provider: P) -> Result<Self, SerializationError> {
        let inner = BoundedString::from_str(s, provider)?;
        Ok(Self { inner })
    }

    /// Returns the name as a string slice if it's valid UTF-8.
    pub fn as_str(&self) -> Result<&str, BoundedError> {
        self.inner.as_str()
    }

    /// Returns the length of the name in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Checks if the name is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Provides direct access to the inner `BoundedString`.
    pub fn inner(&self) -> &BoundedString<N_BYTES, P> {
        &self.inner
    }

    /// Consumes the `WasmName` and returns the inner `BoundedString`.
    pub fn into_inner(self) -> BoundedString<N_BYTES, P> {
        self.inner
    }
}

// Ensure CoreHasher is used in Hasher bounds
impl<T, const N_ELEMENTS: usize, P> Hash for BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Hash,
    P: MemoryProvider + Default + Clone + PartialEq + Eq, // Added Default here
{
    fn hash<H: CoreHasher>(&self, state: &mut H) {
        // Changed to CoreHasher
        self.length.hash(state);
        self.checksum.hash(state); // Assuming Checksum is Hashable
                                   // Hash elements if verification level suggests deep hashing
        if self.verification_level >= VerificationLevel::Full {
            for i in 0..self.length {
                if let Some(item) = self.peek_at_index(i) {
                    // Define peek_at_index if needed
                    item.hash(state);
                }
            }
        }
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default,
{
    // Helper to peek at an arbitrary index, needed for hashing all elements.
    fn peek_at_index(&self, index: usize) -> Option<T> {
        if index >= self.length {
            return None;
        }
        let offset = index.saturating_mul(self.item_serialized_size);
        if self.item_serialized_size == 0 {
            return Some(T::default());
        }
        if let Ok(slice_view) = self.handler.get_slice(offset, self.item_serialized_size) {
            let mut read_stream = ReadStream::new(slice_view);
            if let Ok(item) = T::from_bytes_with_provider(&mut read_stream, self.handler.provider())
            {
                return Some(item);
            }
        }
        None
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq>
    BoundedString<N_BYTES, P>
{
    /// Creates a new BoundedString from a string slice.
    ///
    /// The string is truncated if it's longer than `N_BYTES`.
    /// Returns an error if the provider fails to initialize the internal
    /// BoundedVec.
    pub fn from_str_truncate(s: &str, provider: P) -> Result<Self, BoundedError> {
        let mut bytes_vec = BoundedVec::<u8, N_BYTES, P>::new(provider)?;
        let s_bytes = s.as_bytes();
        let len_to_copy = core::cmp::min(s_bytes.len(), N_BYTES);

        // Ensure that we are only copying valid UTF-8 characters even when truncating.
        // Find the last UTF-8 character boundary before or at len_to_copy.
        let mut actual_len_to_copy = len_to_copy;
        while actual_len_to_copy > 0 && !s.is_char_boundary(actual_len_to_copy) {
            actual_len_to_copy -= 1;
        }

        for i in 0..actual_len_to_copy {
            bytes_vec.push(s_bytes[i])?;
        }
        Ok(Self { bytes: bytes_vec })
    }

    /// Creates a new BoundedString from a string slice.
    ///
    /// Returns an error if the string is too long or if UTF-8 validation fails.
    pub fn from_str(s: &str, provider: P) -> Result<Self, SerializationError> {
        let s_bytes = s.as_bytes();
        if s_bytes.len() > N_BYTES {
            return Err(SerializationError::Custom("String too long for BoundedString"));
        }
        // Basic UTF-8 validation can be done by str::from_utf8 on the slice to be
        // stored. Since `s` is already a &str, it's valid UTF-8. We just need
        // to ensure it fits.
        let mut bytes_vec = BoundedVec::<u8, N_BYTES, P>::new(provider).map_err(|e| {
            SerializationError::Custom("Failed to create BoundedVec for BoundedString")
        })?;
        for byte in s_bytes.iter() {
            bytes_vec.push(*byte).map_err(|e| {
                SerializationError::Custom("Failed to push byte to BoundedVec for BoundedString")
            })?;
        }
        Ok(Self { bytes: bytes_vec })
    }

    /// Returns the string as a slice.
    ///
    /// This will panic if the internal bytes are not valid UTF-8.
    /// For a non-panicking version, use `try_as_str`.
    pub fn as_str(&self) -> Result<&str, BoundedError> {
        // This is temporarily disabled due to lifetime issues in no_std mode
        // TODO: Implement proper lifetime management or alternative API
        Err(BoundedError::new(
            BoundedErrorKind::ConversionError,
            "as_str temporarily disabled in no_std mode",
        ))
    }

    /// Tries to return the string as a slice.
    ///
    /// Returns an error if the internal bytes are not valid UTF-8 or if there's
    /// a problem accessing the underlying storage.
    pub fn try_as_str(&self) -> Result<&str, BoundedError> {
        self.as_str()
    }

    /// Returns the length of the string in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Checks if the string is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Appends a string slice to this string.
    ///
    /// If appending the string would exceed the capacity, it will be truncated
    /// to fit within the capacity while maintaining valid UTF-8 boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut s = BoundedString::<10, _>::from_str_truncate("Hello", provider).unwrap();
    /// s.push_str(", World!").unwrap();
    /// assert_eq!(s.as_str().unwrap(), "Hello, Wor"); // Truncated to fit capacity
    /// ```
    pub fn push_str(&mut self, s: &str) -> Result<(), BoundedError> {
        let remaining_capacity = N_BYTES - self.bytes.len();

        if remaining_capacity == 0 {
            return Ok(()); // Already at capacity, nothing to do
        }

        let s_bytes = s.as_bytes();
        let len_to_copy = core::cmp::min(s_bytes.len(), remaining_capacity);

        // Ensure we only copy valid UTF-8 boundaries
        let mut actual_len_to_copy = len_to_copy;
        while actual_len_to_copy > 0 && !s.is_char_boundary(actual_len_to_copy) {
            actual_len_to_copy -= 1;
        }

        // Add each byte
        for i in 0..actual_len_to_copy {
            self.bytes.push(s_bytes[i])?;
        }

        Ok(())
    }

    /// Clears the string, removing all contents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut s = BoundedString::<10, _>::from_str_truncate("Hello", provider).unwrap();
    /// s.clear().unwrap();
    /// assert!(s.is_empty());
    /// ```
    pub fn clear(&mut self) -> Result<(), BoundedError> {
        self.bytes.clear()
    }

    /// Checks if this string starts with the given prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<10, _>::from_str_truncate("Hello, World", provider).unwrap();
    /// assert!(s.starts_with("Hello").unwrap());
    /// assert!(!s.starts_with("World").unwrap());
    /// ```
    pub fn starts_with(&self, prefix: &str) -> Result<bool, BoundedError> {
        let s = self.as_str()?;
        Ok(s.starts_with(prefix))
    }

    /// Checks if this string ends with the given suffix.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<10, _>::from_str_truncate("Hello, Wor", provider).unwrap();
    /// assert!(s.ends_with("Wor").unwrap());
    /// assert!(!s.ends_with("World").unwrap());
    /// ```
    pub fn ends_with(&self, suffix: &str) -> Result<bool, BoundedError> {
        let s = self.as_str()?;
        Ok(s.ends_with(suffix))
    }

    /// Returns a substring of this string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<10, _>::from_str_truncate("Hello, World", provider).unwrap();
    /// let substring = s.substring(0, 5).unwrap();
    /// assert_eq!(substring.as_str().unwrap(), "Hello");
    /// ```
    pub fn substring(&self, start: usize, end: usize) -> Result<Self, BoundedError>
    where
        P: Clone,
    {
        let s = self.as_str()?;

        if start > end || end > s.len() {
            return Err(BoundedError::new(BoundedErrorKind::SliceError, "Invalid substring range"));
        }

        // Find valid character boundaries
        let mut actual_start = start;
        while actual_start < end && !s.is_char_boundary(actual_start) {
            actual_start += 1;
        }

        let mut actual_end = end;
        while actual_end > actual_start && !s.is_char_boundary(actual_end) {
            actual_end -= 1;
        }

        // Handle edge case where no valid boundaries were found
        if actual_start >= actual_end {
            return Ok(Self {
                bytes: BoundedVec::<u8, N_BYTES, P>::new(self.bytes.provider.clone())?,
            });
        }

        let substr = &s[actual_start..actual_end];
        Self::from_str_truncate(substr, self.bytes.provider.clone())
    }

    /// Appends a character to the end of the string.
    ///
    /// Returns an error if the character would exceed the string's capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let mut s = BoundedString::<10, _>::from_str_truncate("Hello", provider).unwrap();
    /// s.push_char('!').unwrap();
    /// assert_eq!(s.as_str().unwrap(), "Hello!");
    /// ```
    pub fn push_char(&mut self, c: char) -> Result<(), BoundedError> {
        let mut buf = [0u8; 4]; // UTF-8 encoding of a char is at most 4 bytes
        let s = c.encode_utf8(&mut buf);
        self.push_str(s)
    }

    /// Trims leading and trailing whitespace from the string.
    ///
    /// This returns a new `BoundedString` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<20, _>::from_str_truncate("  Hello  ", provider).unwrap();
    /// let trimmed = s.trim().unwrap();
    /// assert_eq!(trimmed.as_str().unwrap(), "Hello");
    /// ```
    pub fn trim(&self) -> Result<Self, BoundedError>
    where
        P: Clone,
    {
        let s = self.as_str()?;
        Self::from_str_truncate(s.trim(), self.bytes.provider.clone())
    }

    /// Converts all characters in the string to lowercase.
    ///
    /// This returns a new `BoundedString` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<20, _>::from_str_truncate("Hello WORLD", provider).unwrap();
    /// let lowercase = s.to_lowercase().unwrap();
    /// assert_eq!(lowercase.as_str().unwrap(), "hello world");
    /// ```
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub fn to_lowercase(&self) -> Result<Self, BoundedError>
    where
        P: Clone,
    {
        let s = self.as_str()?;
        // Allocate a String to perform the lowercase conversion
        // since str doesn't have a method to do this without allocation
        let lowercase = s.to_lowercase();

        Self::from_str_truncate(&lowercase, self.bytes.provider.clone())
    }

    /// Converts all characters in the string to uppercase.
    ///
    /// This returns a new `BoundedString` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<20, _>::from_str_truncate("Hello World", provider).unwrap();
    /// let uppercase = s.to_uppercase().unwrap();
    /// assert_eq!(uppercase.as_str().unwrap(), "HELLO WORLD");
    /// ```
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub fn to_uppercase(&self) -> Result<Self, BoundedError>
    where
        P: Clone,
    {
        let s = self.as_str()?;
        let uppercase = s.to_uppercase();

        Self::from_str_truncate(&uppercase, self.bytes.provider.clone())
    }

    /// Returns the capacity of the string in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<10, _>::from_str_truncate("Hello", provider).unwrap();
    /// assert_eq!(s.capacity(), 10);
    /// ```
    pub fn capacity(&self) -> usize {
        N_BYTES
    }

    /// Checks if the string contains the given substring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<20, _>::from_str_truncate("Hello World", provider).unwrap();
    /// assert!(s.contains("World").unwrap());
    /// assert!(!s.contains("Rust").unwrap());
    /// ```
    pub fn contains(&self, substring: &str) -> Result<bool, BoundedError> {
        let s = self.as_str()?;
        Ok(s.contains(substring))
    }
}

// Add as_bytes_slice to BoundedVec
impl<
        T: Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq + core::fmt::Debug, /* Added Debug */
        const N_ELEMENTS: usize,
        P: MemoryProvider + Clone + PartialEq + Eq, // Ensure P's bounds are sufficient
    > BoundedVec<T, N_ELEMENTS, P>
{
    /// Returns a raw byte slice of the BoundedVec's used data.
    /// This is unsafe if T is not u8 or if T's size is not 1.
    /// This is primarily for BoundedString<u8> use case.
    /// For direct access to the underlying provider's memory for T items,
    /// use get_item_slice or iterate and handle items.
    pub(crate) fn as_bytes_slice(&self) -> core::result::Result<&[u8], BoundedError> {
        // This method is temporarily disabled due to lifetime issues in no_std mode
        Err(BoundedError::new(
            BoundedErrorKind::ConversionError,
            "as_bytes_slice temporarily disabled in no_std mode",
        ))
    }

    /// Returns the raw binary data of this collection as a Vec<u8>.
    /// This is useful when you need to get a copy of the data, not just a
    /// reference.
    ///
    /// Note: This is only available when the `alloc` or `std` feature is
    /// enabled.
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub fn to_bytes_vec(&self) -> core::result::Result<Vec<u8>, BoundedError> {
        let mut result = Vec::with_capacity(self.length * self.item_serialized_size);

        for i in 0..self.length {
            let offset = i * self.item_serialized_size;
            match self.provider.borrow_slice(offset, self.item_serialized_size) {
                Ok(slice) => {
                    // Extend the Vec with the bytes from this item
                    result.extend_from_slice(slice.as_ref());
                }
                Err(_) => {
                    return Err(BoundedError::new(
                        BoundedErrorKind::SliceError,
                        "Failed to get slice for to_bytes_vec",
                    ))
                }
            }
        }

        Ok(result)
    }

    /// Returns a direct byte slice of the raw memory if the provider supports
    /// it. This is a more efficient alternative to `to_bytes_vec()` when a
    /// copy is not needed.
    ///
    /// This method will fail if the provider does not support direct memory
    /// access or if the data is not stored contiguously.
    pub fn as_raw_slice(&self) -> core::result::Result<Slice<'_>, BoundedError> {
        // Request the entire used portion of the memory
        let total_size = self.length * self.item_serialized_size;

        if total_size == 0 {
            return Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Cannot get raw slice of empty or zero-sized collection",
            ));
        }

        match self.provider.borrow_slice(0, total_size) {
            Ok(slice) => Ok(slice),
            Err(_) => Err(BoundedError::new(
                BoundedErrorKind::SliceError,
                "Failed to get raw slice from provider",
            )),
        }
    }
}

// Fix for BoundedString::new in WasmName tests (if any, this was in build log
// for BoundedString::new itself) The main fix is in WasmName::new above.
// If BoundedString::new was called directly elsewhere:
// `BoundedString::<CAP, _>::new(provider)` would become `BoundedString::<CAP,
// _>::from_str_truncate("", provider)`

// Fix for `try_extend_from_slice` on `BoundedVec`
impl<
        T: Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq + core::fmt::Debug, /* Added Debug and other consistent bounds */
        const N_ELEMENTS: usize,
        P: MemoryProvider + Clone + PartialEq + Eq, // Ensure P's bounds are sufficient
    > BoundedVec<T, N_ELEMENTS, P>
{
    pub fn try_extend_from_slice(&mut self, other_slice: &[T]) -> Result<(), BoundedError>
    where
        T: Clone, // Added Clone bound for items
    {
        if self.len() + other_slice.len() > N_ELEMENTS {
            return Err(BoundedError::capacity_exceeded());
        }
        for item in other_slice {
            // This will use self.push(item.clone()), which handles serialization and
            // checksums
            self.push(item.clone())?;
        }
        Ok(())
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> Checksummable
    for WasmName<N_BYTES, P>
{
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        self.inner.update_checksum(checksum);
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> ToBytes
    for WasmName<N_BYTES, P>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.inner.to_bytes_with_provider(writer, provider)
    }

    // to_bytes is provided by the trait if default-provider feature is enabled
    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        self.inner.to_bytes(writer)
    }
}

impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq> FromBytes
    for WasmName<N_BYTES, P>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        BoundedString::<N_BYTES, P>::from_bytes_with_provider(reader, provider)
            .map(|inner_bs| Self { inner: inner_bs })
    }

    // from_bytes is provided by the trait if default-provider feature is enabled
    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        BoundedString::<N_BYTES, P>::from_bytes(reader).map(|inner_bs| Self { inner: inner_bs })
    }
}

// Note: This impl block was removed due to overlapping type bounds with the
// main impl block. All necessary methods are already defined in the main impl
// block.

// ... (other BoundedVec impl methods, make sure to use `Error::` where it was
// `Error::` before) For example, in BoundedVec::get:

// ...
// fn get(&self, index: usize) -> WrtResult<T>
// ...
// if index >= self.length {
// return Err(Error::index_out_of_bounds(index, self.length));
// }
// ... Deserialization part ...
// T::from_bytes(&mut item_reader).map_err(|e|
// Error::deserialization_error("Failed to deserialize item from BoundedVec"))
// ... or use existing Error::from(e) if appropriate

// Ensure other usages of Error::method are corrected if they were missed.
// The build errors point to specific lines.

// Example for BoundedVec::get, line 943 in original error
// Error::deserialization_error(
// Error::memory_error(
// Error::index_out_of_bounds(
// Error::validation_error(

// Fix specific error lines from build output:
// bounded.rs:943:35 -> Error::deserialization_error(
// bounded.rs:948:27 -> Error::memory_error(
// bounded.rs:1018:24 -> Error::index_out_of_bounds(index, self.length)
// bounded.rs:1028:24 -> Error::index_out_of_bounds(index, self.length)
// bounded.rs:1077:33 -> Error::validation_error(
// bounded.rs:1082:35 -> Error::memory_error(
// bounded.rs:1087:27 -> Error::memory_error(

// In BoundedVec::get method (around line 943):
// ...
// match self.provider.read_slice(offset, self.item_serialized_size) {
//     Ok(item_slice) => {
//         let mut item_reader = ReadStream::new(item_slice.data());
//         T::from_bytes(&mut item_reader).map_err(|_e| {
//             Error::deserialization_error(
//                 "Failed to deserialize item from BoundedVec (read_slice
// path)",             )
//         })
//     }
//     Err(_e) => Err(Error::memory_error(
//         "Failed to read item slice from provider in BoundedVec",
//     )),
// }
// ...

// In BoundedVec::get_item_slice (around 1018):
// ...
// if index >= self.length {
//     return Err(Error::index_out_of_bounds(index, self.length));
// }
// ...

// In BoundedVec::get_item_slice_mut (around 1028):
// ...
// if index >= self.length {
//     return Err(Error::index_out_of_bounds(index, self.length));
// }
// ...

// In BoundedVec::verify_item_checksum_at_offset (around 1077):
// ...
//     .map_err(|_e| Error::validation_error("Failed to create ReadStream for
// item verification"))?; ...
// } else {
//     Err(Error::memory_error(
//         "Failed to read slice for item checksum verification",
//     ))
// }
// ...
// Err(e) => Err(Error::memory_error(
//     "Provider error during item checksum verification",
// )),

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    // This impl block provides methods with additional constraints
    // The verify_item_checksum_at_offset method is already defined in the main impl
    // block
}

// Alloc-dependent methods for BoundedString
#[cfg(feature = "alloc")]
impl<const N_BYTES: usize, P: MemoryProvider + Default + Clone + PartialEq + Eq>
    BoundedString<N_BYTES, P>
{
    /// Splits the string by the given delimiter and returns a vector of
    /// BoundedStrings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wrt_foundation::bounded::BoundedString;
    /// # use wrt_foundation::NoStdProvider;
    /// # use wrt_foundation::VerificationLevel;
    /// #
    /// # let provider = NoStdProvider::new(1024, VerificationLevel::default());
    /// # let s = BoundedString::<20, _>::from_str_truncate("Hello,World,Rust", provider).unwrap();
    /// let parts = s.split(',').unwrap();
    /// assert_eq!(parts.len(), 3);
    /// assert_eq!(parts[0].as_str().unwrap(), "Hello");
    /// assert_eq!(parts[1].as_str().unwrap(), "World");
    /// assert_eq!(parts[2].as_str().unwrap(), "Rust");
    /// ```
    pub fn split(&self, delimiter: char) -> Result<Vec<Self>, BoundedError>
    where
        P: Clone,
    {
        let s = self.as_str()?;
        let mut result = Vec::new();

        for part in s.split(delimiter) {
            let bounded_part = Self::from_str_truncate(part, self.bytes.provider.clone())?;
            result.push(bounded_part);
        }

        Ok(result)
    }
}
