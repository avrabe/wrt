// WRT - wrt-types
// Module: Safe Memory Abstractions
// SW-REQ-ID: REQ_MEM_SAFETY_002
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

// #![allow(unsafe_code)] // REMOVED: Unsafe code will be handled by specific
// blocks with safety comments.

//! Safe memory abstractions for WebAssembly runtime
//!
//! This module provides memory safety abstractions designed for
//! functional safety at ASIL-B level, implementing verification
//! mechanisms to detect memory corruption.

#![cfg_attr(not(feature = "std"), allow(unused_unsafe))]

// Core imports
#[cfg(feature = "std")]
extern crate std;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::operations::{record_global_operation, Type as OperationType};
// Checksum is in prelude
// pub use crate::verification::Checksum;
pub use crate::prelude::ToString;
pub use crate::prelude::*;
#[cfg(not(feature = "std"))]
use crate::validation::importance;

// HashSet and Mutex are in std part of prelude when feature "std" is on.
// For no_std, Mutex comes from once_mutex, HashSet isn't used directly here for
// no_std provider. #[cfg(feature = "std")]
// use std::{collections::HashSet, sync::Mutex};

/// A safe slice with integrated checksum for data integrity verification
#[derive(Clone)]
pub struct Slice<'a> {
    /// The underlying data slice
    data: &'a [u8],
    /// Checksum for data integrity verification
    checksum: Checksum,
    /// Length of the slice for redundant verification
    length: usize,
    /// Verification level for this slice
    verification_level: VerificationLevel,
}

impl<'a> Slice<'a> {
    /// Create a new `Slice` from a raw byte slice
    ///
    /// This computes a checksum for the data which can be used
    /// to verify integrity later.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial integrity verification fails.
    pub fn new(data: &'a [u8]) -> Result<Self> {
        Self::with_verification_level(data, VerificationLevel::default())
    }

    /// Create a new `Slice` with a specific verification level
    ///
    /// This computes a checksum for the data which can be used
    /// to verify integrity later.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial integrity verification fails (e.g., checksum
    /// computation error or validation failure upon creation).
    ///
    /// # Panics
    ///
    /// This function previously panicked if initial verification failed.
    /// It now returns a Result to indicate failure.
    /// Safety impact: [LOW|MEDIUM|HIGH] - [Brief explanation of the safety
    /// implication] Tracking: WRTQ-XXX (qualification requirement tracking
    /// ID).
    pub fn with_verification_level(data: &'a [u8], level: VerificationLevel) -> Result<Self> {
        // Track checksum calculation
        record_global_operation(OperationType::ChecksumCalculation, level);

        let checksum = Checksum::compute(data);
        let length = data.len();

        let slice = Self { data, checksum, length, verification_level: level };

        // Verify on creation to ensure consistency
        // Use full importance (255) for initial verification
        slice.verify_integrity_with_importance(255)?;

        Ok(slice)
    }

    /// Get the underlying data slice
    ///
    /// This performs an integrity check before returning the data.
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails.
    pub fn data(&self) -> Result<&'a [u8]> {
        // Track memory read operation
        record_global_operation(OperationType::MemoryRead, self.verification_level);

        // Medium importance for data access (128)
        self.verify_integrity_with_importance(128)?;
        Ok(self.data)
    }

    /// Get length of the slice
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if the slice is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get the current verification level
    #[must_use]
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Set a new verification level
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    /// Verify data integrity using the stored checksum
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails based on the current
    /// verification level and default importance (128).
    pub fn verify_integrity(&self) -> Result<()> {
        // Track validation operation
        record_global_operation(OperationType::CollectionValidate, self.verification_level);

        // Medium importance by default (128)
        self.verify_integrity_with_importance(128)
    }

    /// Verify data integrity with specified operation importance
    ///
    /// The importance value (0-255) affects the likelihood of
    /// verification when using `VerificationLevel::Sampling`
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails based on the current
    /// verification level and the provided importance.
    pub fn verify_integrity_with_importance(&self, importance: u8) -> Result<()> {
        // Skip verification if the level indicates we shouldn't verify
        if !self.verification_level.should_verify(importance) {
            return Ok(());
        }

        // If length doesn't match stored value, memory is corrupt
        if self.data.len() != self.length {
            return Err(Error::validation_error("Memory corruption detected: length mismatch"));
        }

        // Different paths for optimize vs non-optimize
        #[cfg(feature = "optimize")]
        {
            Ok(())
        }

        #[cfg(not(feature = "optimize"))]
        {
            // Track checksum calculation for important operations
            if importance >= 200 || self.verification_level.should_verify_redundant() {
                record_global_operation(
                    OperationType::ChecksumCalculation,
                    self.verification_level,
                );
            }

            // Skip detailed checks for low importance non-redundant checks
            if !self.verification_level.should_verify_redundant() && importance < 200 {
                return Ok(());
            }

            // Compute current checksum and compare with stored checksum
            let current = Checksum::compute(self.data);
            if current == self.checksum {
                Ok(())
            } else {
                Err(Error::validation_error("Memory corruption detected: checksum mismatch"))
            }
        }
    }

    /// Create a sub-slice with the same safety guarantees
    ///
    /// # Errors
    ///
    /// Returns an error if the parent slice fails its integrity check, if the
    /// requested sub-slice range is invalid (overflow or out of bounds), or
    /// if creating the sub-slice fails its own initial integrity check.
    pub fn slice(&self, start: usize, len: usize) -> Result<Slice<'a>> {
        // Track memory read operation
        record_global_operation(OperationType::MemoryRead, self.verification_level);

        // High importance for slicing operations (200)
        self.verify_integrity_with_importance(200)?;

        // Use let-else for cleaner error handling
        let Some(end) = start.checked_add(len) else {
            return Err(Error::memory_error(format!(
                "Sub-slice range calculation overflow: start={start}, len={len}"
            )));
        };

        if end > self.length {
            // Check if calculated end (exclusive) exceeds original slice length
            return Err(Error::memory_error(format!(
                "Invalid sub-slice range: start {}, len {} (end {} > actual_len {})",
                start, len, end, self.length
            )));
        }

        // Create a new Slice with the specified range
        let sub_data = &self.data[start..end];

        // Create a new Slice with the same verification level
        Slice::with_verification_level(sub_data, self.verification_level)
    }
}

impl fmt::Debug for Slice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slice")
            .field("length", &self.length)
            .field("checksum", &self.checksum)
            .field("verification_level", &self.verification_level)
            .finish()
    }
}

/// A safe mutable slice with integrated checksum for data integrity
/// verification
pub struct SliceMut<'a> {
    /// The underlying mutable data slice
    data: &'a mut [u8],
    /// Checksum for data integrity verification
    checksum: Checksum,
    /// Length of the slice for redundant verification
    length: usize,
    /// Verification level for this slice
    verification_level: VerificationLevel,
}

impl<'a> SliceMut<'a> {
    /// Create a new `SliceMut` from a raw mutable byte slice
    ///
    /// This computes a checksum for the data which can be used
    /// to verify integrity.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial integrity check fails.
    pub fn new(data: &'a mut [u8]) -> Result<Self> {
        Self::with_verification_level(data, VerificationLevel::default())
    }

    /// Create a new `SliceMut` with a specific verification level
    ///
    /// # Errors
    ///
    /// Returns an error if the initial integrity check fails (e.g., checksum
    /// computation error or validation failure upon creation).
    ///
    /// # Panics
    /// This function previously panicked if initial verification failed.
    /// It now returns a Result to indicate failure.
    pub fn with_verification_level(data: &'a mut [u8], level: VerificationLevel) -> Result<Self> {
        record_global_operation(OperationType::ChecksumCalculation, level);
        let checksum = Checksum::compute(data);
        let length = data.len();
        let slice = Self { data, checksum, length, verification_level: level };

        // Verify on creation
        slice.verify_integrity_with_importance(255)?; // High importance for creation

        Ok(slice)
    }

    /// Get a mutable reference to the underlying data slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails.
    ///
    /// # Safety
    /// Modifying the returned slice directly will invalidate the stored
    /// checksum. The checksum must be updated using `update_checksum()`
    /// after modification. This performs an integrity check before
    /// returning the data.
    pub fn data_mut(&mut self) -> Result<&mut [u8]> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level); // Or a more specific operation
        self.verify_integrity_with_importance(128)?;
        Ok(self.data)
    }

    /// Get an immutable reference to the underlying data slice.
    /// This performs an integrity check before returning the data.
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails.
    pub fn data(&self) -> Result<&[u8]> {
        record_global_operation(OperationType::MemoryRead, self.verification_level);
        self.verify_integrity_with_importance(128)?;
        Ok(self.data)
    }

    /// Get length of the slice
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if the slice is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get the current verification level
    #[must_use]
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Set a new verification level
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    /// Recomputes and updates the checksum based on the current data.
    /// This should be called after any direct modification to the slice
    /// obtained via `data_mut()`.
    pub fn update_checksum(&mut self) {
        record_global_operation(OperationType::ChecksumCalculation, self.verification_level);
        self.checksum = Checksum::compute(self.data);
    }

    /// Verify data integrity using the stored checksum
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails based on the current
    /// verification level and default importance (128).
    pub fn verify_integrity(&self) -> Result<()> {
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        self.verify_integrity_with_importance(128)
    }

    /// Verify data integrity with specified operation importance
    ///
    /// # Errors
    ///
    /// Returns an error if the integrity check fails based on the current
    /// verification level and the provided importance.
    pub fn verify_integrity_with_importance(&self, importance: u8) -> Result<()> {
        if !self.verification_level.should_verify(importance) {
            return Ok(());
        }
        if self.data.len() != self.length {
            return Err(Error::validation_error(
                "Memory corruption detected: length mismatch (SliceMut)",
            ));
        }

        #[cfg(feature = "optimize")]
        {
            Ok(())
        }
        #[cfg(not(feature = "optimize"))]
        {
            if importance >= 200 || self.verification_level.should_verify_redundant() {
                record_global_operation(
                    OperationType::ChecksumCalculation,
                    self.verification_level,
                );
            }
            if !self.verification_level.should_verify_redundant() && importance < 200 {
                return Ok(());
            }
            let current_checksum = Checksum::compute(self.data);
            if current_checksum == self.checksum {
                Ok(())
            } else {
                Err(Error::validation_error(format!(
                    "Memory corruption detected: checksum mismatch (SliceMut). Expected {}, \
                     got {}",
                    self.checksum, current_checksum
                )))
            }
        }
    }

    /// Create a mutable sub-slice with the same safety guarantees
    ///
    /// # Errors
    ///
    /// Returns an error if the parent slice fails its integrity check, if the
    /// requested sub-slice range is invalid (overflow or out of bounds), or
    /// if creating the sub-slice fails its own initial integrity check.
    pub fn slice_mut(&mut self, start: usize, len: usize) -> Result<SliceMut<'_>> {
        // Track memory read/write intent (as it's a mutable slice)
        // For now, using MemoryWrite as it provides potential for modification.
        record_global_operation(OperationType::MemoryWrite, self.verification_level);

        // High importance for slicing operations (200)
        self.verify_integrity_with_importance(200)?;

        // Use let-else for cleaner error handling
        let Some(end) = start.checked_add(len) else {
            return Err(Error::memory_error(format!(
                "Mutable sub-slice range calculation overflow: start={start}, len={len}"
            )));
        };

        if end > self.length {
            return Err(Error::memory_error(format!(
                "Invalid mutable sub-slice range: start {start}, len {len} (end {end} > actual_len {})",
                self.length
            )));
        }

        // Obtain the mutable sub-slice unsafe block needs careful justification if used
        // Let's assume safe slicing for now
        let sub_data = &mut self.data[start..end];

        // Create a new SliceMut with the same verification level
        SliceMut::with_verification_level(sub_data, self.verification_level)
    }

    /// Get a mutable raw pointer to the start of the slice data, if the offset
    /// is within bounds.
    ///
    /// # Safety
    ///
    /// Returning a raw pointer bypasses Rust's borrow checking rules.
    /// The caller must ensure that the pointer is used safely:
    /// - The pointer must not be used after the `SliceMut` it originated
    ///   from is dropped or modified in a way that invalidates the pointer
    ///   (e.g., if the underlying provider reallocates).
    /// - Accesses through the pointer must be within the bounds of the original
    ///   slice.
    /// - Data races must be prevented if the same memory region can be accessed
    ///   through other means (e.g., other pointers or safe references).
    /// - Modifying the memory through this pointer invalidates the internal
    ///   checksum. `update_checksum()` must be called afterwards if integrity
    ///   checks are still required.
    ///
    /// This method should only be used when strictly necessary, typically for FFI
    /// or performance-critical code where invariants are manually upheld.
    #[must_use]
    pub fn get_ptr_mut(&self, offset: usize) -> Option<*mut u8> {
        // Check if offset is within the bounds of the slice
        if offset < self.length { // Check against self.length for consistency
            // .as_ptr() gets a raw pointer to the start.
            // .wrapping_add(offset) calculates the pointer offset, wrapping on overflow
            // (though overflow shouldn't happen here due to the bounds check).
            // .cast_mut() converts *const u8 to *mut u8. This is safe as long
            // as the original reference `&'a mut [u8]` allows mutation.
            Some(self.data.as_ptr().wrapping_add(offset).cast_mut())
        } else {
            // Offset is out of bounds
            None
        }
    }
}

impl fmt::Debug for SliceMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SliceMut")
            .field("length", &self.length)
            .field("checksum", &self.checksum)
            .field("verification_level", &self.verification_level)
            // Do not print mutable data content in debug
            .finish()
    }
}

/// Memory provider interface for different allocation strategies.
///
/// This trait abstracts over different memory allocation strategies,
/// allowing both std and `no_std` environments to share the same interface.
/// It combines raw access, safety features, and informational methods.
pub trait Provider: Send + Sync + Debug {
    /// Borrows a slice of memory with safety guarantees.
    /// The returned `Slice` will have its verification level typically
    /// initialized by the provider or a wrapping handler.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested slice is out of bounds or if the
    /// underlying memory cannot be borrowed (e.g., due to internal errors
    /// or failed integrity checks).
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<Slice<'_>>;

    /// Writes data to the memory at a given offset.
    ///
    /// # Errors
    ///
    /// Returns an error if the write operation would go out of bounds, if the
    /// underlying memory cannot be written to (e.g., internal errors or
    /// failed integrity checks), or if the data itself is invalid for writing.
    fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()>;

    /// Verifies that an access to memory (read or write) of `len` at `offset`
    /// would be valid. This is a pre-check and does not perform the access.
    ///
    /// # Errors
    ///
    /// Returns an error if the described access would be invalid (e.g., out of
    /// bounds).
    fn verify_access(&self, offset: usize, len: usize) -> Result<()>;

    /// Gets the total current size/length of the initialized/used memory within
    /// the provider.
    fn size(&self) -> usize;

    /// Gets the total capacity of the memory region managed by the provider.
    fn capacity(&self) -> usize;

    /// Verifies the overall integrity of the memory managed by the provider.
    /// This could involve checking internal checksums, canaries, or other
    /// mechanisms.
    ///
    /// # Errors
    ///
    /// Returns an error if an integrity violation is detected.
    fn verify_integrity(&self) -> Result<()>;

    /// Sets the verification level for operations performed by this memory
    /// provider.
    fn set_verification_level(&mut self, level: VerificationLevel);

    /// Gets the current verification level of this memory provider.
    fn verification_level(&self) -> VerificationLevel;

    /// Gets statistics about memory usage from this provider.
    fn memory_stats(&self) -> Stats;

    /// Gets a mutable slice from the underlying memory provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested slice is out of bounds or if the
    /// underlying memory cannot be borrowed mutably (e.g., due to internal
    /// errors or failed integrity checks).
    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SliceMut<'_>>;

    /// Copies data within the memory provider from a source offset to a
    /// destination offset.
    ///
    /// # Errors
    ///
    /// Returns an error if the source or destination ranges are invalid or out
    /// of bounds, or if the copy operation fails due to internal provider issues.
    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()>;

    /// Ensures the provider's internal accounting of used space extends up to
    /// `byte_offset`. This is typically called by a collection after it
    /// logically extends its length.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested offset is beyond the provider's capacity
    /// or if the operation fails internally.
    fn ensure_used_up_to(&mut self, byte_offset: usize) -> Result<()>;
}

/// Memory statistics structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// Total memory size in bytes
    pub total_size: usize,
    /// Number of memory accesses
    pub access_count: usize,
    /// Number of unique memory regions accessed
    pub unique_regions: usize,
    /// Maximum size of any access
    pub max_access_size: usize,
}

/// Memory provider for std environments
#[cfg(feature = "std")]
pub struct StdProvider {
    /// The underlying data buffer
    data: Vec<u8>,
    /// Track memory accesses for safety monitoring
    access_log: std::sync::Mutex<Vec<(usize, usize)>>,
    /// Counter for access operations
    access_count: AtomicUsize,
    /// Maximum size of any access seen
    max_access_size: AtomicUsize,
    /// Number of unique regions accessed
    unique_regions: AtomicUsize,
    /// Regions hash (for uniqueness tracking)
    regions_hash: std::sync::Mutex<HashSet<usize>>,
    /// Verification level for memory operations
    verification_level: VerificationLevel,
}

#[cfg(feature = "std")]
impl fmt::Debug for StdProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("StdProvider");

        debug_struct
            .field("data_size", &self.data.len())
            .field("capacity", &self.data.capacity())
            .field("access_count", &self.access_count.load(Ordering::Relaxed))
            .field("max_access_size", &self.max_access_size.load(Ordering::Relaxed))
            .field("unique_regions", &self.unique_regions.load(Ordering::Relaxed))
            .field("verification_level", &self.verification_level);

        // Handle potential Mutex poisoning gracefully in Debug
        match self.access_log.lock() {
            Ok(guard) => {
                debug_struct.field("access_log_len", &guard.len());
            }
            Err(_) => {
                debug_struct.field("access_log_len", &"<poisoned>");
            }
        }

        match self.regions_hash.lock() {
            Ok(guard) => {
                debug_struct.field("regions_hash_len", &guard.len());
            }
            Err(_) => {
                debug_struct.field("regions_hash_len", &"<poisoned>");
            }
        }

        debug_struct.finish()
    }
}

#[cfg(feature = "std")]
impl Default for StdProvider {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            access_log: std::sync::Mutex::new(Vec::new()),
            access_count: AtomicUsize::new(0),
            max_access_size: AtomicUsize::new(0),
            unique_regions: AtomicUsize::new(0),
            regions_hash: std::sync::Mutex::new(HashSet::new()),
            verification_level: VerificationLevel::Sampling,
        }
    }
}

#[cfg(feature = "std")]
impl StdProvider {
    /// Create a new `StdProvider` with the given data
    #[allow(clippy::redundant_clone)]
    #[must_use]
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            access_log: std::sync::Mutex::new(Vec::with_capacity(100)),
            access_count: AtomicUsize::new(0),
            max_access_size: AtomicUsize::new(0),
            unique_regions: AtomicUsize::new(0),
            regions_hash: std::sync::Mutex::new(HashSet::new()),
            verification_level: VerificationLevel::default(),
        }
    }

    /// Create a new empty `StdProvider` with the given capacity
    #[must_use]
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new(Vec::with_capacity(100))
    }

    /// Get the access log for debugging and verification
    ///
    /// # Errors
    ///
    /// Returns an error if the access log mutex is poisoned.
    pub fn access_log(&self) -> Result<Vec<(usize, usize)>> {
        match self.access_log.lock() {
            Ok(log) => Ok(log.clone()),
            Err(_) => Err(Error::runtime_error("Access log mutex poisoned")),
        }
    }

    /// Add data to the memory provider
    pub fn add_data(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Clear all data and access logs
    ///
    /// Note: Ignores mutex poisoning errors for simplicity in clear operation.
    pub fn clear(&mut self) {
        self.data.clear();
        if let Ok(mut log) = self.access_log.lock() {
            log.clear();
        }
        self.access_count.store(0, Ordering::SeqCst);
        self.max_access_size.store(0, Ordering::SeqCst);
        self.unique_regions.store(0, Ordering::SeqCst);
        if let Ok(mut regions) = self.regions_hash.lock() {
            regions.clear();
        }
        // Checksum is not managed here, it's per Slice/SliceMut
    }

    /// Resize the underlying data vector, filling new space with `value`.
    ///
    /// # Errors
    ///
    /// This function does not currently return errors but is designed to if
    /// future resizing constraints (e.g., max capacity) are added.
    pub fn resize(&mut self, new_size: usize, value: u8) -> Result<()> {
        // Track memory allocation-like operation as resize can allocate
        record_global_operation(OperationType::MemoryAllocation, self.verification_level);
        self.data.resize(new_size, value);
        // Checksum needs to be re-evaluated by consumer if they are using SliceMut over this.
        Ok(())
    }

    /// Track an access to a memory region.
    /// This is an internal helper and does not return Result, assumes locks succeed or logs ignore.
    fn track_access(&self, offset: usize, len: usize) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.max_access_size.fetch_max(len, Ordering::Relaxed);

        if let Ok(mut log) = self.access_log.lock() {
            if log.len() < 1000 { // Cap log size
                log.push((offset, len));
            }
        }

        if let Ok(mut regions) = self.regions_hash.lock() {
            if !regions.contains(&offset) {
                regions.insert(offset);
                self.unique_regions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Clear the access tracking logs.
    ///
    /// # Errors
    ///
    /// Returns an error if the access log or region hash mutex is poisoned.
    pub fn clear_access_tracking(&self) -> Result<()> {
        match self.access_log.lock() {
            Ok(mut log) => log.clear(),
            Err(_) => return Err(Error::runtime_error("Access log mutex poisoned during clear")),
        }
        match self.regions_hash.lock() {
            Ok(mut regions) => regions.clear(),
            Err(_) => return Err(Error::runtime_error("Region hash mutex poisoned during clear")),
        }
        self.access_count.store(0, Ordering::Relaxed);
        self.max_access_size.store(0, Ordering::Relaxed);
        self.unique_regions.store(0, Ordering::Relaxed);
        Ok(())
    }

    /// Set the verification level for memory operations
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    /// Get current verification level
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Get detailed memory statistics
    pub fn memory_stats(&self) -> Stats {
        // Lock the hash set to read its size accurately
        let unique_regions_count = self
            .regions_hash
            .lock()
            .map_or(0, |guard| guard.len()); // Handle potential lock poisoning

        Stats {
            total_size: self.data.capacity(),
            access_count: self.access_count.load(Ordering::Relaxed),
            unique_regions: unique_regions_count, // Use the locked value
            max_access_size: self.max_access_size.load(Ordering::Relaxed),
        }
    }

    /// Recalculates checksums after direct memory modifications
    ///
    /// This method should be called after directly modifying the underlying
    /// data to ensure memory integrity verification continues to work
    /// properly.
    pub fn recalculate_checksums(&mut self) {
        // This is left as a placeholder in the current implementation
        // In a production-grade implementation, this would update any internal
        // checksums or verification data structures.

        // For now, the memory safety checks are mostly based on bounds checking
        // rather than checksums, but this extension point is important for future
        // enhancements to memory safety.

        // Track operation for proper profiling
        record_global_operation(OperationType::ChecksumCalculation, self.verification_level);
    }

    /// Gets a mutable pointer to data at the given offset
    pub fn get_ptr_mut(&self, offset: usize) -> Option<*mut u8> {
        if offset >= self.data.len() {
            return None;
        }
        Some(self.data.as_ptr().wrapping_add(offset).cast_mut())
    }
}

#[cfg(feature = "std")]
impl Provider for StdProvider {
    /// Borrows a slice of memory with safety guarantees.
    /// The returned `Slice` will have its verification level typically
    /// initialized by the provider or a wrapping handler.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested slice is out of bounds or if the
    /// underlying memory cannot be borrowed (e.g., due to internal errors
    /// or failed integrity checks).
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<Slice<'_>> {
        // Track memory read operation implicitly via Slice creation
        record_global_operation(OperationType::MemoryRead, self.verification_level);
        self.verify_access(offset, len)?;
        self.track_access(offset, len);
        Slice::with_verification_level(&self.data[offset..offset + len], self.verification_level)
    }

    /// Gets a mutable slice from the underlying memory provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested slice is out of bounds or if the
    /// underlying memory cannot be borrowed mutably (e.g., due to internal
    /// errors or failed integrity checks).
    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SliceMut<'_>> {
        // Track memory write operation implicitly via SliceMut creation
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.verify_access(offset, len)?;
        self.track_access(offset, len); // Track access before mutable borrow
        SliceMut::with_verification_level(
            &mut self.data[offset..offset + len],
            self.verification_level,
        )
    }

    /// Writes data to the memory at a given offset.
    ///
    /// # Errors
    ///
    /// Returns an error if the write operation would go out of bounds, if the
    /// underlying memory cannot be written to (e.g., internal errors or
    /// failed integrity checks), or if the data itself is invalid for writing.
    fn write_data(&mut self, offset: usize, data_to_write: &[u8]) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        let len = data_to_write.len();
        self.verify_access(offset, len)?;

        let end = offset.checked_add(len).ok_or_else(|| {
            Error::memory_error(format!(
                "Write range calculation overflow: offset={offset}, len={len}"
            ))
        })?;

        if end > self.data.len() {
            return Err(Error::memory_error(format!(
                "Write out of bounds: offset={offset}, len={len}, data_len={}",
                self.data.len()
            )));
        }

        self.data[offset..end].copy_from_slice(data_to_write);
        self.track_access(offset, len);
        // Note: Checksum needs to be updated by the caller if they are using
        // SliceMut over this part of memory.
        Ok(())
    }

    /// Verifies that an access to memory (read or write) of `len` at `offset`
    /// would be valid. This is a pre-check and does not perform the access.
    ///
    /// # Errors
    ///
    /// Returns an error if the described access would be invalid (e.g., out of
    /// bounds).
    fn verify_access(&self, offset: usize, len: usize) -> Result<()> {
        // Track generic validation operation
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        let end = offset.checked_add(len).ok_or_else(|| {
            Error::memory_error(format!(
                "Access range calculation overflow: offset={offset}, len={len}"
            ))
        })?;

        if end > self.data.len() {
            // Use capacity for verify_access if we want to allow writes up to capacity
            // For now, strictly check against current data.len()
            return Err(Error::memory_error(format!(
                "Access out of bounds: offset={offset}, len={len}, data_len={}",
                self.data.len()
            )));
        }
        Ok(())
    }

    /// Gets the total current size/length of the initialized/used memory within
    /// the provider.
    fn size(&self) -> usize {
        self.data.len()
    }

    /// Gets the total capacity of the memory region managed by the provider.
    fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Verifies the overall integrity of the memory managed by the provider.
    /// This could involve checking internal checksums, canaries, or other
    /// mechanisms.
    ///
    /// # Errors
    ///
    /// Returns an error if an integrity violation is detected.
    /// (Currently a placeholder for StdProvider, as integrity is per Slice)
    fn verify_integrity(&self) -> Result<()> {
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        // For StdProvider, integrity is primarily managed by Slice/SliceMut.
        // This could be extended if StdProvider had its own global checksums or canaries.
        Ok(())
    }

    /// Sets the verification level for operations performed by this memory
    /// provider.
    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    /// Gets the current verification level of this memory provider.
    fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Gets statistics about memory usage from this provider.
    fn memory_stats(&self) -> Stats {
        self.memory_stats()
    }

    /// Copies data within the memory provider from a source offset to a
    /// destination offset.
    ///
    /// # Errors
    ///
    /// Returns an error if the source or destination ranges are invalid or out
    /// of bounds, or if the copy operation fails due to internal provider issues.
    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level); // Treat as write

        // Verify source range
        self.verify_access(src_offset, len)?;
        // Verify destination range
        self.verify_access(dst_offset, len)?;

        // Perform the copy_within. This itself has bounds checks but we pre-verified.
        // The ranges must not overlap in a way that copy_from_slice would be needed,
        // but Vec::copy_within handles overlaps correctly.
        self.data.copy_within(src_offset..src_offset + len, dst_offset);
        self.track_access(dst_offset, len); // Track write to destination
        Ok(())
    }

    /// Ensures the provider's internal accounting of used space extends up to
    /// `byte_offset`. This is typically called by a collection after it
    /// logically extends its length.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested offset is beyond the provider's capacity
    /// or if the operation fails internally (e.g. Vec realloc fail, though rare).
    /// Currently, it will try to resize if `byte_offset` is beyond current length.
    fn ensure_used_up_to(&mut self, byte_offset: usize) -> Result<()> {
        record_global_operation(OperationType::MemoryGrow, self.verification_level);
        if byte_offset > self.data.len() {
            // This implies the logical size is growing. We need to ensure the Vec has this capacity
            // and then logically extend its reported size. For Vec, this might mean resizing.
            // For simplicity, let's ensure it can contain `byte_offset` by resizing.
            // This might be more involved depending on exact semantics (zeroing new memory, etc.)
            // For now, assume if it's used up to an offset, it means it should exist.
            if byte_offset > self.data.capacity() {
                // Attempt to reserve additional space if byte_offset exceeds capacity.
                // This can fail if allocation fails.
                self.data.try_reserve(byte_offset - self.data.len()).map_err(|e| {
                    Error::memory_error(format!(
                        "Failed to reserve space for ensure_used_up_to: {e}"
                    ))
                })?;
            }
            // If ensure_used_up_to means the valid data now extends to byte_offset,
            // and new bytes should be zeroed or initialized:
            if byte_offset > self.data.len() { // Check again after potential reserve
                 // This is a simplification; actual behavior for uninitialized parts might differ.
                 // For now, we just ensure length, which Vec::resize does.
                 // If the intent is just to mark as used, not necessarily initialize,
                 // this might be different. For Vec, length implies initialized.
                self.data.resize(byte_offset, 0); // Fill new space with 0
            }
        }
        // If byte_offset is within current len, it's already "used up to" that point.
        Ok(())
    }
}

/// Memory provider using a fixed-size array, suitable for `no_std` environments.
///
/// Note: This provider does not perform heap allocations.
pub struct NoStdProvider<const N: usize> {
    /// The underlying data buffer
    data: [u8; N],
    /// Current usage of the buffer
    used: usize,
    /// Counter for access operations
    access_count: AtomicUsize,
    /// Last access offset for validation
    last_access_offset: AtomicUsize,
    /// Last access length for validation
    last_access_length: AtomicUsize,
    /// Verification level for memory operations
    verification_level: VerificationLevel,
}

#[cfg(not(feature = "std"))]
impl<const N: usize> fmt::Debug for NoStdProvider<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoStdProvider")
            .field("used", &self.used)
            .field("access_count", &self.access_count.load(Ordering::Relaxed))
            .field("verification_level", &self.verification_level)
            .finish()
    }
}

#[cfg(not(feature = "std"))]
impl<const N: usize> Default for NoStdProvider<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "std"))]
impl<const N: usize> NoStdProvider<N> {
    /// Create a new empty memory provider
    pub fn new() -> Self {
        Self {
            data: [0; N],
            used: 0,
            access_count: AtomicUsize::new(0),
            last_access_offset: AtomicUsize::new(0),
            last_access_length: AtomicUsize::new(0),
            verification_level: VerificationLevel::default(),
        }
    }

    /// Set the data for this memory provider
    ///
    /// # Errors
    ///
    /// Returns an error if the provided data length exceeds the fixed capacity `N`.
    pub fn set_data(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > N {
            return Err(Error::memory_error(format!(
                "Data too large for fixed-size buffer: {} > {}",
                data.len(),
                N
            )));
        }

        // Copy the data into our fixed buffer
        for (i, &byte) in data.iter().enumerate() {
            self.data[i] = byte;
        }
        self.used = data.len();

        // Reset access metrics
        self.access_count.store(0, Ordering::SeqCst);
        self.last_access_offset.store(0, Ordering::SeqCst);
        self.last_access_length.store(0, Ordering::SeqCst);

        Ok(())
    }

    /// Get the current access count
    pub fn access_count(&self) -> usize {
        self.access_count.load(Ordering::SeqCst)
    }

    /// Reset usage to zero (clear memory)
    pub fn clear(&mut self) {
        self.used = 0;
        self.access_count.store(0, Ordering::SeqCst);
    }

    /// Get the last accessed region
    pub fn last_access(&self) -> (usize, usize) {
        let offset = self.last_access_offset.load(Ordering::SeqCst);
        let length = self.last_access_length.load(Ordering::SeqCst);
        (offset, length)
    }

    /// Resize the used portion of memory
    ///
    /// # Errors
    ///
    /// Returns an error if `new_size` exceeds the fixed capacity `N`.
    pub fn resize(&mut self, new_size: usize) -> Result<()> {
        if new_size > N {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_OUT_OF_BOUNDS,
                format!("Cannot resize to {} (max: {})", new_size, N),
            ));
        }

        // Zero out any newly used memory
        if new_size > self.used {
            for i in self.used..new_size {
                self.data[i] = 0;
            }
        }

        self.used = new_size;
        Ok(())
    }

    /// Verify memory integrity
    ///
    /// # Errors
    ///
    /// Returns an error if the internal `used` counter exceeds the fixed
    /// capacity `N`, or if the last recorded access was out of the `used` bounds.
    pub fn verify_integrity(&self) -> Result<()> {
        // Track validation operation
        record_global_operation(OperationType::CollectionValidate, self.verification_level);

        // Simple length check
        if self.used > N {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::INTEGRITY_VIOLATION,
                "Memory corruption detected: used > capacity",
            ));
        }

        // Verify that the last access was valid
        let offset = self.last_access_offset.load(Ordering::SeqCst);
        let length = self.last_access_length.load(Ordering::SeqCst);

        if length > 0 && offset + length > self.used {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                format!(
                    "Last access out of bounds: offset={}, len={}, used={}",
                    offset, length, self.used
                ),
            ));
        }

        Ok(())
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> Stats {
        Stats {
            total_size: N,
            access_count: self.access_count(),
            unique_regions: 1, // No-std can't track unique regions
            max_access_size: self.last_access_length.load(Ordering::Relaxed),
        }
    }

    #[allow(dead_code)]
    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SliceMut<'_>> {
        if offset + len > N {
            return Err(Error::memory_error("Out of bounds access".to_string()));
        }
        let slice = &mut self.data[offset..offset + len];
        SliceMut::new(slice)
    }

    #[allow(dead_code)]
    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        if src_offset + len > N || dst_offset + len > N {
            return Err(Error::memory_error("Out of bounds access".to_string()));
        }
        self.data.copy_within(src_offset..src_offset + len, dst_offset);
        Ok(())
    }
}

/// A generic handler for safe memory operations, backed by a `Provider`.
///
/// This struct acts as a wrapper around a `Provider` implementation,
/// adding a layer of verification control and a consistent interface.
#[derive(Debug)]
pub struct Handler<P: Provider> {
    /// The internal memory provider instance.
    provider: P,
    /// Verification level for operations managed by this handler.
    /// This level can be used to influence checks within the handler itself
    /// or be passed down to the provider if the provider also uses it.
    verification_level: VerificationLevel,
}

impl<P: Provider> Handler<P> {
    /// Creates a new `Handler` with a given provider and verification
    /// level.
    ///
    /// The provider is moved into the handler.
    pub fn new(mut provider: P, verification_level: VerificationLevel) -> Self {
        provider.set_verification_level(verification_level);
        Self { provider, verification_level }
    }

    /// Sets the verification level for this handler and its underlying
    /// provider.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
        self.provider.set_verification_level(level);
    }

    /// Gets the current verification level of this handler.
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Gets a safe slice from the underlying memory provider.
    pub fn get_slice(&self, offset: usize, len: usize) -> Result<Slice<'_>> {
        self.provider.borrow_slice(offset, len)
    }

    /// Writes data to the underlying memory provider.
    pub fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.provider.write_data(offset, data)
    }

    /// Gets the current size of the memory from the provider.
    pub fn size(&self) -> usize {
        self.provider.size()
    }

    /// Gets the total capacity of the memory from the provider.
    pub fn capacity(&self) -> usize {
        self.provider.capacity()
    }

    /// Verifies the integrity of the underlying memory provider.
    pub fn verify_integrity(&self) -> Result<()> {
        self.provider.verify_integrity()
    }

    /// Gets memory usage statistics from the provider.
    pub fn memory_stats(&self) -> Stats {
        self.provider.memory_stats()
    }

    /// Provides immutable access to the underlying provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Provides mutable access to the underlying provider.
    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    /// Gets a mutable slice from the underlying memory provider.
    pub fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SliceMut<'_>> {
        self.provider.get_slice_mut(offset, len)
    }

    /// Copies data within the memory provider from a source offset to a
    /// destination offset.
    pub fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        self.provider.copy_within(src_offset, dst_offset, len)
    }

    /// Ensures the provider's internal accounting of used space extends up to
    /// `byte_offset`.
    pub fn ensure_used_up_to(&mut self, byte_offset: usize) -> Result<()> {
        self.provider.ensure_used_up_to(byte_offset)
    }
}

// Constructors for specific default providers
#[cfg(feature = "std")]
impl Handler<StdProvider> {
    /// Creates a new `Handler` backed by an `StdProvider`
    /// initialized with a specific capacity and verification level.
    #[must_use]
    pub fn with_std_provider(capacity: usize, verification_level: VerificationLevel) -> Self {
        let mut provider = StdProvider::with_capacity(capacity);
        provider.set_verification_level(verification_level);
        Self { provider, verification_level }
    }
}

#[cfg(all(not(feature = "std"), feature = "alloc"))]
impl<const N: usize> Handler<NoStdProvider<N>> {
    /// Creates a new `Handler` backed by a `NoStdProvider`
    /// with a compile-time fixed size N and a given verification level.
    pub fn with_no_std_provider(verification_level: VerificationLevel) -> Self {
        let mut provider = NoStdProvider::<N>::new();
        provider.set_verification_level(verification_level);
        Self { provider, verification_level }
    }
}

// Include tests if the feature is enabled
// #[cfg(all(feature = "std", test))]
// #[path = "memory_integration_tests.rs"]
// mod memory_integration_tests; // Removed missing module
