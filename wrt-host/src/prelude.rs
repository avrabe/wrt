// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Prelude module for wrt-host
//!
//! This module provides a unified set of imports for both std and no_std
//! environments. It re-exports commonly used types and traits to ensure
//! consistency across all crates in the WRT project and simplify imports in
//! individual modules.

// Binary std/no_std choice - conditional imports only

// Binary std/no_std choice
#[cfg(not(feature = "std"))]
pub use wrt_foundation::{
    bounded::{BoundedVec as Vec, BoundedString as String},
    BoundedMap as HashMap,
    BoundedSet as HashSet,
};

// Additional imports for pure no_std
#[cfg(not(feature = "std"))]
pub use core::fmt::Write as FmtWrite;

// Arc is not available in pure no_std, use a reference wrapper
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone)]
/// Arc-like wrapper for no_std environments
pub struct Arc<T> {
    inner: T,
}

#[cfg(not(feature = "std"))]
impl<T> Arc<T> {
    /// Create a new Arc-like wrapper
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }
}

#[cfg(not(feature = "std"))]
impl<T> core::ops::Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// In pure no_std mode, we need a minimal Box implementation for trait objects
// Simple Box implementation for no_std environments
#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub struct Box<T> {
    inner: T,
}

#[cfg(not(feature = "std"))]
impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }
}

#[cfg(not(feature = "std"))]
impl<T> core::ops::Deref for Box<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(not(feature = "std"))]
impl<T> core::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// Drop and Debug are automatically derived for our simple Box implementation
pub use core::{
    any::Any,
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    convert::{TryFrom, TryInto},
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
    boxed::Box,
    collections::{HashMap, HashSet},
    format,
    string::{String, ToString},
    sync::{Arc, Mutex, RwLock},
    vec,
    vec::Vec,
};

// Re-export from wrt-error
pub use wrt_error::{codes, kinds, Error, ErrorCategory, Result};
// Re-export from wrt-foundation
pub use wrt_foundation::{
    // Builtin types
    builtin::BuiltinType,
    // SafeMemory types
    safe_memory::{SafeMemoryHandler, SafeSlice, SafeStack},
    // Common types
    types::{BlockType, FuncType, GlobalType, MemoryType, TableType, ValueType},
    values::Value,
    // Verification types
    verification::VerificationLevel,
};

// Binary std/no_std choice
#[cfg(feature = "std")]
pub use wrt_foundation::component_value::ComponentValue;
// Binary std/no_std choice
#[cfg(feature = "std")]
pub use wrt_intercept::{
    BeforeBuiltinResult, BuiltinInterceptor, InterceptContext, LinkInterceptor,
    LinkInterceptorStrategy,
};
// Import synchronization primitives for no_std
#[cfg(not(feature = "std"))]
pub use wrt_sync::{Mutex, RwLock};

// Re-export from this crate
pub use crate::{
    builder::HostBuilder,
    callback::{CallbackRegistry, CallbackType},
    function::{CloneableFn, HostFunctionHandler},
    host::BuiltinHost,
};
