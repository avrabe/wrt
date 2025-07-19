// WRT - wrt-foundation
// Module: WebAssembly Component Model Built-in Types
// SW-REQ-ID: REQ_019
// SW-REQ-ID: REQ_020
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use core::{fmt, str::FromStr};

// Error types are imported through crate root
use crate::{
    bounded::BoundedVec,
    prelude::*,
    safe_memory::NoStdProvider,
    traits::{Checksummable, FromBytes, ReadStream, SerializationError, ToBytes, WriteStream},
    verification::Checksum,
    WrtResult,
};

/// Maximum number of `BuiltinType` variants, used for `BoundedVec` capacity.
const MAX_BUILTIN_TYPES: usize = 13;

// Calculate a suitable capacity for the NoStdProvider.
// Each BuiltinType takes 1 byte (serialized_size).
// BoundedVec itself might have a small overhead, but provider capacity is for
// raw bytes.
const ALL_AVAILABLE_PROVIDER_CAPACITY: usize = MAX_BUILTIN_TYPES * 1; // BuiltinType::default().serialized_size() is 1

/// Enumeration of all supported WebAssembly Component Model built-in functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuiltinType {
    // Resource built-ins (always available)
    /// Create a new resource
    ResourceCreate,
    /// Drop (destroy) a resource
    ResourceDrop,
    /// Get the representation of a resource
    ResourceRep,
    /// Get a handle to a resource
    ResourceGet,

    // Async built-ins (feature-gated)
    /// Create a new async value
    #[cfg(feature = "component-model-async")]
    AsyncNew,
    /// Get the value from an async value once resolved
    #[cfg(feature = "component-model-async")]
    AsyncGet,
    /// Poll an async value for completion
    #[cfg(feature = "component-model-async")]
    AsyncPoll,
    /// Wait for an async value to complete
    #[cfg(feature = "component-model-async")]
    AsyncWait,

    // Error Context built-ins (feature-gated)
    /// Create a new error context
    #[cfg(feature = "component-model-error-context")]
    ErrorNew,
    /// Get the trace from an error context
    #[cfg(feature = "component-model-error-context")]
    ErrorTrace,

    // Threading built-ins (feature-gated)
    /// Spawn a new thread
    #[cfg(feature = "component-model-threading")]
    ThreadingSpawn,
    /// Join a thread (wait for its completion)
    #[cfg(feature = "component-model-threading")]
    ThreadingJoin,
    /// Create a synchronization primitive
    #[cfg(feature = "component-model-threading")]
    ThreadingSync,
}

impl Default for BuiltinType {
    fn default() -> Self {
        BuiltinType::ResourceCreate
    }
}

impl Checksummable for BuiltinType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        let discriminant_byte = match self {
            // Assign distinct, stable byte values for each variant
            BuiltinType::ResourceCreate => 0x01,
            BuiltinType::ResourceDrop => 0x02,
            BuiltinType::ResourceRep => 0x03,
            BuiltinType::ResourceGet => 0x04,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncNew => 0x05,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncGet => 0x06,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncPoll => 0x07,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncWait => 0x08,
            #[cfg(feature = "component-model-error-context")]
            BuiltinType::ErrorNew => 0x09,
            #[cfg(feature = "component-model-error-context")]
            BuiltinType::ErrorTrace => 0x0A,
            #[cfg(feature = "component-model-threading")]
            BuiltinType::ThreadingSpawn => 0x0B,
            #[cfg(feature = "component-model-threading")]
            BuiltinType::ThreadingJoin => 0x0C,
            #[cfg(feature = "component-model-threading")]
            BuiltinType::ThreadingSync => 0x0D,
        };
        checksum.update_slice(&[discriminant_byte]); // Use update_slice for
                                                     // &[u8]
    }
}

impl ToBytes for BuiltinType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        let byte_val = match self {
            BuiltinType::ResourceCreate => 0,
            BuiltinType::ResourceDrop => 1,
            BuiltinType::ResourceRep => 2,
            BuiltinType::ResourceGet => 3,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncNew => 4,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncGet => 5,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncPoll => 6,
            #[cfg(feature = "component-model-async")]
            BuiltinType::AsyncWait => 7,
            #[cfg(feature = "component-model-error-context")]
            BuiltinType::ErrorNew => 8,
            #[cfg(feature = "component-model-error-context")]
            BuiltinType::ErrorTrace => 9,
            #[cfg(feature = "component-model-threading")]
            BuiltinType::ThreadingSpawn => 10,
            #[cfg(feature = "component-model-threading")]
            BuiltinType::ThreadingJoin => 11,
            #[cfg(feature = "component-model-threading")]
            BuiltinType::ThreadingSync => 12,
        };
        writer.write_u8(byte_val).map_err(|e| e)
    }
}

impl FromBytes for BuiltinType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        let val = reader.read_u8()?;
        match val {
            0 => Ok(BuiltinType::ResourceCreate),
            1 => Ok(BuiltinType::ResourceDrop),
            2 => Ok(BuiltinType::ResourceRep),
            3 => Ok(BuiltinType::ResourceGet),
            #[cfg(feature = "component-model-async")]
            4 => Ok(BuiltinType::AsyncNew),
            #[cfg(feature = "component-model-async")]
            5 => Ok(BuiltinType::AsyncGet),
            #[cfg(feature = "component-model-async")]
            6 => Ok(BuiltinType::AsyncPoll),
            #[cfg(feature = "component-model-async")]
            7 => Ok(BuiltinType::AsyncWait),
            #[cfg(feature = "component-model-error-context")]
            8 => Ok(BuiltinType::ErrorNew),
            #[cfg(feature = "component-model-error-context")]
            9 => Ok(BuiltinType::ErrorTrace),
            #[cfg(feature = "component-model-threading")]
            10 => Ok(BuiltinType::ThreadingSpawn),
            #[cfg(feature = "component-model-threading")]
            11 => Ok(BuiltinType::ThreadingJoin),
            #[cfg(feature = "component-model-threading")]
            12 => Ok(BuiltinType::ThreadingSync),
            _ => Err(SerializationError::InvalidFormat.into()),
        }
    }
}

/// Error returned when parsing a built-in type fails
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBuiltinError;

impl fmt::Display for ParseBuiltinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid built-in type name")
    }
}

impl FromStr for BuiltinType {
    type Err = ParseBuiltinError;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            // Resource built-ins
            "resource.create" => Ok(Self::ResourceCreate),
            "resource.drop" => Ok(Self::ResourceDrop),
            "resource.rep" => Ok(Self::ResourceRep),
            "resource.get" => Ok(Self::ResourceGet),

            // Async built-ins
            #[cfg(feature = "component-model-async")]
            "async.new" => Ok(Self::AsyncNew),
            #[cfg(feature = "component-model-async")]
            "async.get" => Ok(Self::AsyncGet),
            #[cfg(feature = "component-model-async")]
            "async.poll" => Ok(Self::AsyncPoll),
            #[cfg(feature = "component-model-async")]
            "async.wait" => Ok(Self::AsyncWait),

            // Error Context built-ins
            #[cfg(feature = "component-model-error-context")]
            "error.new" => Ok(Self::ErrorNew),
            #[cfg(feature = "component-model-error-context")]
            "error.trace" => Ok(Self::ErrorTrace),

            // Threading built-ins
            #[cfg(feature = "component-model-threading")]
            "threading.spawn" => Ok(Self::ThreadingSpawn),
            #[cfg(feature = "component-model-threading")]
            "threading.join" => Ok(Self::ThreadingJoin),
            #[cfg(feature = "component-model-threading")]
            "threading.sync" => Ok(Self::ThreadingSync),

            // Unknown built-in
            _ => Err(ParseBuiltinError),
        }
    }
}

impl BuiltinType {
    /// Get the name of the built-in as a string
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            // Resource built-ins
            Self::ResourceCreate => "resource.create",
            Self::ResourceDrop => "resource.drop",
            Self::ResourceRep => "resource.rep",
            Self::ResourceGet => "resource.get",

            // Async built-ins
            #[cfg(feature = "component-model-async")]
            Self::AsyncNew => "async.new",
            #[cfg(feature = "component-model-async")]
            Self::AsyncGet => "async.get",
            #[cfg(feature = "component-model-async")]
            Self::AsyncPoll => "async.poll",
            #[cfg(feature = "component-model-async")]
            Self::AsyncWait => "async.wait",

            // Error Context built-ins
            #[cfg(feature = "component-model-error-context")]
            Self::ErrorNew => "error.new",
            #[cfg(feature = "component-model-error-context")]
            Self::ErrorTrace => "error.trace",

            // Threading built-ins
            #[cfg(feature = "component-model-threading")]
            Self::ThreadingSpawn => "threading.spawn",
            #[cfg(feature = "component-model-threading")]
            Self::ThreadingJoin => "threading.join",
            #[cfg(feature = "component-model-threading")]
            Self::ThreadingSync => "threading.sync",
        }
    }

    /// Parse a string into a built-in type (convenience method)
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::from_str(s).ok()
    }

    /// Check if this built-in is available in the current configuration
    #[must_use]
    pub fn is_available(&self) -> bool {
        match self {
            // Resource built-ins are always available
            Self::ResourceCreate | Self::ResourceDrop | Self::ResourceRep | Self::ResourceGet => {
                true
            }

            // Feature-gated built-ins
            #[cfg(feature = "component-model-async")]
            Self::AsyncNew | Self::AsyncGet | Self::AsyncPoll | Self::AsyncWait => true,

            #[cfg(feature = "component-model-error-context")]
            Self::ErrorNew | Self::ErrorTrace => true,

            #[cfg(feature = "component-model-threading")]
            Self::ThreadingSpawn | Self::ThreadingJoin | Self::ThreadingSync => true,

            // Built-ins that are conditionally compiled out are not available
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    /// Get all available built-in types in the current configuration
    #[must_use]
    pub fn all_available(
    ) -> BoundedVec<Self, MAX_BUILTIN_TYPES, NoStdProvider<ALL_AVAILABLE_PROVIDER_CAPACITY>> {
        let provider = NoStdProvider::<ALL_AVAILABLE_PROVIDER_CAPACITY>::default);
        let mut result: BoundedVec<Self, MAX_BUILTIN_TYPES, _> = BoundedVec::new(provider)
            .expect("Static BoundedVec init failed for BuiltinType::all_available");

        // These pushes are infallible as MAX_BUILTIN_TYPES is calculated to hold all
        // possible variants, and item_serialized_size (1) * MAX_BUILTIN_TYPES fits
        // provider capacity. BoundedVec::push returns Result<(), BoundedError>,
        // so use expect or proper handling.
        result.push(Self::ResourceCreate).expect("Static capacity push failed");
        result.push(Self::ResourceDrop).expect("Static capacity push failed");
        result.push(Self::ResourceRep).expect("Static capacity push failed");
        result.push(Self::ResourceGet).expect("Static capacity push failed");

        // Async built-ins
        #[cfg(feature = "component-model-async")]
        {
            result.push(Self::AsyncNew).expect("Static capacity push failed");
            result.push(Self::AsyncGet).expect("Static capacity push failed");
            result.push(Self::AsyncPoll).expect("Static capacity push failed");
            result.push(Self::AsyncWait).expect("Static capacity push failed");
        }

        // Error Context built-ins
        #[cfg(feature = "component-model-error-context")]
        {
            result.push(Self::ErrorNew).expect("Static capacity push failed");
            result.push(Self::ErrorTrace).expect("Static capacity push failed");
        }

        // Threading built-ins
        #[cfg(feature = "component-model-threading")]
        {
            result.push(Self::ThreadingSpawn).expect("Static capacity push failed");
            result.push(Self::ThreadingJoin).expect("Static capacity push failed");
            result.push(Self::ThreadingSync).expect("Static capacity push failed");
        }

        result
    }
}

impl fmt::Display for BuiltinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_name() {
        assert_eq!(BuiltinType::ResourceCreate.name(), "resource.create";
        assert_eq!(BuiltinType::ResourceDrop.name(), "resource.drop";
        assert_eq!(BuiltinType::ResourceRep.name(), "resource.rep";
        assert_eq!(BuiltinType::ResourceGet.name(), "resource.get";
    }

    #[test]
    fn test_builtin_from_str() {
        assert_eq!(BuiltinType::from_str("resource.create"), Ok(BuiltinType::ResourceCreate;
        assert_eq!(BuiltinType::from_str("resource.drop"), Ok(BuiltinType::ResourceDrop;
        assert_eq!(BuiltinType::from_str("resource.rep"), Ok(BuiltinType::ResourceRep;
        assert_eq!(BuiltinType::from_str("resource.get"), Ok(BuiltinType::ResourceGet;
        assert_eq!(BuiltinType::from_str("unknown.builtin"), Err(ParseBuiltinError;

        // Also test the parse convenience method
        assert_eq!(BuiltinType::parse("resource.create"), Some(BuiltinType::ResourceCreate;
        assert_eq!(BuiltinType::parse("unknown.builtin"), None;
    }

    #[test]
    fn test_builtin_is_available() {
        // Resource built-ins should always be available
        assert!(BuiltinType::ResourceCreate.is_available();
        assert!(BuiltinType::ResourceDrop.is_available();
        assert!(BuiltinType::ResourceRep.is_available();
        assert!(BuiltinType::ResourceGet.is_available();
    }

    #[test]
    fn test_all_available() {
        // Should at least contain the resource built-ins
        let available = BuiltinType::all_available);
        assert!(available.contains(&BuiltinType::ResourceCreate);
        assert!(available.contains(&BuiltinType::ResourceDrop);
        assert!(available.contains(&BuiltinType::ResourceRep);
        assert!(available.contains(&BuiltinType::ResourceGet);
    }
}
