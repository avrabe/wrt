// WRT - wrt-types
// Module: WebAssembly Component Model Built-in Types
// SW-REQ-ID: REQ_WASM_COMPONENT_001 (Example: Relates to component model built-ins)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly Component Model built-in type definitions
//!
//! This module defines the built-in types and operations available in the
//! WebAssembly Component Model, as well as utility methods for working with them.

use core::fmt;
use core::str::FromStr;

#[cfg(feature = "std")]
use std::{vec, vec::Vec};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{vec, vec::Vec};

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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
    pub fn parse(s: &str) -> Option<Self> {
        Self::from_str(s).ok()
    }

    /// Check if this built-in is available in the current configuration
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
    pub fn all_available() -> Vec<Self> {
        // Create a vector with the resource built-ins which are always available
        #[allow(unused_mut)]
        let mut result =
            vec![Self::ResourceCreate, Self::ResourceDrop, Self::ResourceRep, Self::ResourceGet];

        // Async built-ins
        #[cfg(feature = "component-model-async")]
        {
            result.push(Self::AsyncNew);
            result.push(Self::AsyncGet);
            result.push(Self::AsyncPoll);
            result.push(Self::AsyncWait);
        }

        // Error Context built-ins
        #[cfg(feature = "component-model-error-context")]
        {
            result.push(Self::ErrorNew);
            result.push(Self::ErrorTrace);
        }

        // Threading built-ins
        #[cfg(feature = "component-model-threading")]
        {
            result.push(Self::ThreadingSpawn);
            result.push(Self::ThreadingJoin);
            result.push(Self::ThreadingSync);
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
        assert_eq!(BuiltinType::ResourceCreate.name(), "resource.create");
        assert_eq!(BuiltinType::ResourceDrop.name(), "resource.drop");
        assert_eq!(BuiltinType::ResourceRep.name(), "resource.rep");
        assert_eq!(BuiltinType::ResourceGet.name(), "resource.get");
    }

    #[test]
    fn test_builtin_from_str() {
        assert_eq!(BuiltinType::from_str("resource.create"), Ok(BuiltinType::ResourceCreate));
        assert_eq!(BuiltinType::from_str("resource.drop"), Ok(BuiltinType::ResourceDrop));
        assert_eq!(BuiltinType::from_str("resource.rep"), Ok(BuiltinType::ResourceRep));
        assert_eq!(BuiltinType::from_str("resource.get"), Ok(BuiltinType::ResourceGet));
        assert_eq!(BuiltinType::from_str("unknown.builtin"), Err(ParseBuiltinError));

        // Also test the parse convenience method
        assert_eq!(BuiltinType::parse("resource.create"), Some(BuiltinType::ResourceCreate));
        assert_eq!(BuiltinType::parse("unknown.builtin"), None);
    }

    #[test]
    fn test_builtin_is_available() {
        // Resource built-ins should always be available
        assert!(BuiltinType::ResourceCreate.is_available());
        assert!(BuiltinType::ResourceDrop.is_available());
        assert!(BuiltinType::ResourceRep.is_available());
        assert!(BuiltinType::ResourceGet.is_available());
    }

    #[test]
    fn test_all_available() {
        // Should at least contain the resource built-ins
        let available = BuiltinType::all_available();
        assert!(available.contains(&BuiltinType::ResourceCreate));
        assert!(available.contains(&BuiltinType::ResourceDrop));
        assert!(available.contains(&BuiltinType::ResourceRep));
        assert!(available.contains(&BuiltinType::ResourceGet));
    }
}
