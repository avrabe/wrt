// #![allow(unsafe_code)] // Allow unsafe for UnsafeCell, raw pointers, and Send/Sync impls

// WRT - wrt-sync
// Module: OnceCell Implementation
// SW-REQ-ID: N/A (Derived from foundational sync primitives)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! A cell that can be initialized at most once.
//!
//! This module provides a `WrtOnce<T>` type that allows for safe, one-time
//! initialization of a value, typically used for global statics.

use crate::mutex::WrtMutex; // Using the WrtMutex from this crate. Removed WrtMutexGuard as it's not used directly here.
use core::cell::UnsafeCell;
use core::fmt;
use core::mem::MaybeUninit;

/// A synchronization primitive which can be written to only once.
///
/// This type is analogous to `std::sync::OnceLock` or `once_cell::sync::OnceCell`.
///
/// # Safety
/// This implementation uses `UnsafeCell` to store the actual data `T` once initialized.
/// The `WrtMutex<bool>` (`initialized_lock`) is used to ensure that the initialization
/// logic runs at most once. After initialization, the data is considered immutable
/// and references can be safely given out.
pub struct WrtOnce<T> {
    mutex: WrtMutex<()>, // Used as a traditional lock for the initialization check
    data: UnsafeCell<MaybeUninit<T>>,
    initialized: AtomicBool, // Tracks if `data` is initialized
}

// Safety: `WrtOnce<T>` is Send if T is Send because the data is protected by the mutex
// during initialization and then accessed via shared references. The AtomicBool is Send.
// UnsafeCell<MaybeUninit<T>> is Send if T is Send.
unsafe impl<T: Send> Send for WrtOnce<T> {}

// Safety: `WrtOnce<T>` is Sync if T is Send and Sync.
// `Send` is required for `T` to be stored. `Sync` is required for `&T` to be `Send`.
// The mutex ensures exclusive access during initialization.
// Once initialized, `data` is accessed immutably. AtomicBool is Sync.
// UnsafeCell<MaybeUninit<T>> is Sync if T is Send + Sync (as &MaybeUninit<T> needs to be Sync).
unsafe impl<T: Send + Sync> Sync for WrtOnce<T> {}

impl<T> WrtOnce<T> {
    /// Creates a new, empty `WrtOnce`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mutex: WrtMutex::new(()),
            data: UnsafeCell::new(MaybeUninit::uninit()),
            initialized: AtomicBool::new(false),
        }
    }

    /// Gets the reference to the underlying value, initializing it if necessary.
    ///
    /// If `WrtOnce` is already initialized, this method returns immediately.
    /// Otherwise, it calls `f` to initialize the value, and then returns a reference.
    /// If multiple threads attempt to initialize the same `WrtOnce` concurrently,
    /// only one thread will execute `f()`, and others will block until initialization
    /// is complete.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        // Fast path: check if already initialized without locking.
        // Acquire ordering ensures that if we see `true`, the write to `data`
        // by the initializing thread is also visible.
        if self.initialized.load(Ordering::Acquire) {
            // Safety: `initialized` is true, so `data` has been initialized.
            // The Acquire load synchronizes with the Release store by the initializing thread.
            // It's safe to assume `data` contains a valid `T`.
            return unsafe { self.get_unchecked() };
        }

        // Slow path: acquire lock and attempt initialization.
        let _guard = self.mutex.lock(); // Lock to ensure only one thread initializes.

        // Double-check: another thread might have initialized while we were waiting for the lock.
        if self.initialized.load(Ordering::Relaxed) {
            // Relaxed is fine here, guarded by mutex.
            // Safety: Same as above fast path, but now under lock.
            return unsafe { self.get_unchecked() };
        }

        // We are the thread to initialize it.
        let value = f();
        unsafe {
            // Write the initialized value.
            (*self.data.get()).write(value);
        }

        // Mark as initialized. Release ordering ensures that the write to `data`
        // happens-before any subsequent Acquire load of `initialized` by other threads.
        self.initialized.store(true, Ordering::Release);

        // Safety: We just initialized it.
        unsafe { self.get_unchecked() }
    }

    /// Gets a reference to the underlying value if it is initialized.
    ///
    /// Returns `None` if the `WrtOnce` has not been initialized yet.
    pub fn get(&self) -> Option<&T> {
        if self.initialized.load(Ordering::Acquire) {
            // Safety: `initialized` is true, so `data` has been initialized.
            Some(unsafe { self.get_unchecked() })
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// Caller must ensure that the `WrtOnce<T>` has been initialized prior to calling this.
    /// This is typically ensured by checking `initialized.load(Ordering::Acquire)` first,
    /// or by virtue of program logic (e.g., after `get_or_init` has completed).
    #[inline]
    unsafe fn get_unchecked(&self) -> &T {
        // Safety: The caller guarantees that `data` is initialized.
        // `MaybeUninit::assume_init_ref` is the correct way to get a reference
        // from an initialized `MaybeUninit`.
        (*self.data.get()).assume_init_ref()
    }
}

impl<T> Default for WrtOnce<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for WrtOnce<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Some(v) => f.debug_struct("WrtOnce").field("data", v).finish(),
            None => f.debug_struct("WrtOnce").field("data", &"<uninitialized>").finish(),
        }
    }
}

// It's good practice to ensure Drop is handled, though for `MaybeUninit<T>`
// where T might need dropping, if WrtOnce itself is dropped, the T inside MaybeUninit
// should be dropped if it was initialized.
impl<T> Drop for WrtOnce<T> {
    fn drop(&mut self) {
        // If initialized, take the value and drop it.
        // Relaxed ordering is fine for the load, as `drop` has exclusive access.
        if *self.initialized.get_mut() {
            // get_mut provides safe access to the bool
            unsafe {
                // Safety: We have exclusive access (`&mut self`) and `initialized` is true,
                // so `data` contains an initialized `T`.
                (*self.data.get_mut()).assume_init_drop();
            }
        }
    }
}

// Re-import core items used by the module that might not be in scope otherwise in no_std
use core::sync::atomic::{AtomicBool, Ordering};
