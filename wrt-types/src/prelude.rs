// WRT - wrt-types
// Module: Prelude for Common Imports
// SW-REQ-ID: N/A (Prelude, not directly implementing a specific requirement)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Prelude module for wrt-types
//!
//! This module provides a unified set of imports for both std and no_std environments.
//! It re-exports commonly used types and traits to ensure consistency across all crates
//! in the WRT project and simplify imports in individual modules.

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

// Re-export from wrt-error
pub use wrt_error::{codes, kinds, Error, ErrorCategory, Result};

// Re-export from this crate
pub use crate::{
    // Bounded collections
    bounded::{BoundedStack, BoundedVec, CapacityError},
    // Builtin types
    builtin::BuiltinType,
    // Component model types
    component::{ComponentType, ExternType, InstanceType, Limits, Namespace, ResourceType},
    component_value::{ComponentValue, ValType},
    // Conversion utilities
    conversion::{ref_type_to_val_type, val_type_to_ref_type},
    // Resource types
    resource::ResourceOperation,
    // Safe memory types (SafeMemoryHandler, SafeSlice, SafeStack are already here from direct re-exports)
    // Sections (SectionId, SectionType, Section are usually handled by decoder)
    // Traits
    traits::{FromFormat, ToFormat},
    // Common types (BlockType, FuncType, GlobalType, MemoryType, TableType, ValueType are already here)
    types::{BlockType, FuncType, GlobalType, MemoryType, RefType, TableType, ValueType},
    // Validation traits
    validation::{BoundedCapacity, Checksummed, Validatable},
    // Value representations
    values::{FloatBits32, FloatBits64, Value},
    // Verification types
    verification::{Checksum, VerificationLevel},
    // Direct re-exports for convenience (original list)
    // ResourceType, // Already covered by component::* above
    SafeMemoryHandler,
    SafeSlice,
};
