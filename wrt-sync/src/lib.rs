// WRT - wrt-sync
// SW-REQ-ID: [SW-REQ-ID-wrt-sync]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::missing_panics_doc)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Allow `alloc` crate usage when no_std
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Conditionally use `std` for tests or specific features
#[cfg(feature = "std")]
extern crate std;

// Verify required features
#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!("The 'alloc' feature must be enabled when using no_std");

/// Synchronization primitives for Wasmtime, supporting both std and `no_std` environments.
///
/// This module provides essential synchronization tools like `WrtMutex` and `WrtRwLock`,
/// designed to work seamlessly in WebAssembly runtimes. They are optimized for
/// performance and safety, offering robust solutions for managing concurrent access
/// to shared resources.
///
/// ## Modules
///
/// - `mutex`: Provides `WrtMutex` and `WrtMutexGuard`.
/// - `rwlock`: Provides `WrtRwLock` and `WrtParkingRwLock`.
///
/// Each module is designed with an emphasis on ergonomics, performance, and safety,
/// offering robust solutions for managing concurrent access to shared resources.
/// The `WrtMutex` provides a mutual exclusion primitive.
/// It ensures that only one thread or task can access the protected data at any given time.
/// This implementation is designed for WebAssembly runtimes and
/// transparently adapts to both standard library and `no_std` environments based on
/// feature flags.
///
/// # Features
pub mod mutex;

/// OnceCell implementation for one-time initialization.
///
/// This module provides a synchronization primitive that allows for safe, one-time
/// initialization of a value.
///
/// # Examples
///
/// ```
/// use wrt_sync::prelude::*;
/// static SOME_STATIC: WrtOnce<Vec<i32>> = WrtOnce::new();
/// fn main() {
///     let data = SOME_STATIC.get_or_init(|| vec![1,2,3]);
///     assert_eq!(data, &vec![1,2,3]);
/// }
/// ```
pub mod once;

/// Prelude module for `wrt_sync`.
///
/// This module re-exports commonly used items for convenience.
pub mod prelude {
    #[cfg(not(feature = "std"))]
    pub use core::{
        cell::UnsafeCell,
        fmt,
        ops::{Deref, DerefMut},
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    pub use alloc::{sync::Arc, vec::Vec};

    #[cfg(feature = "std")]
    pub use std::{
        cell::UnsafeCell,
        fmt,
        ops::{Deref, DerefMut},
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
        thread::{self, park as park_thread, sleep, yield_now, JoinHandle},
        time::Duration,
        vec::Vec,
    };
}

/// `ReadWrite` lock implementation for both std and `no_std` environments.
///
/// The `WrtRwLock` allows multiple readers or a single writer at any point in time.
/// This is useful for data structures that are read frequently but modified infrequently.
/// Like `WrtMutex`, this implementation is tailored for WebAssembly and adjusts its
/// behavior for `std` and `no_std` contexts.
///
/// # Features
pub mod rwlock;

// Include verification module conditionally, but exclude during coverage builds
#[cfg(all(not(coverage), doc))]
#[cfg_attr(docsrs, doc(cfg(feature = "kani")))]
pub mod verify;

// Re-export types for convenience
pub use mutex::{WrtMutex, WrtMutexGuard};
pub use once::WrtOnce;
pub use rwlock::{WrtRwLock, WrtRwLockReadGuard, WrtRwLockWriteGuard};

/// Publicly exported synchronization primitives when the `std` feature is enabled.
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use rwlock::parking_impl::{
    PoisonError, WrtParkingRwLock, WrtParkingRwLockReadGuard, WrtParkingRwLockWriteGuard,
};

// Conditional re-export for the basic (spin-lock) RwLock and its guards
// These are always available as they don't depend on std for parking.
