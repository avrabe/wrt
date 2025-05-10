// WRT - wrt-types
// Module: Safe Memory Abstractions
// SW-REQ-ID: REQ_MEM_SAFETY_002 (Example: Relates to safe memory handling)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

// #![allow(unsafe_code)] // REMOVED: Unsafe code will be handled by specific blocks with safety comments.

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

use crate::operations::{record_global_operation, OperationType};
// Checksum is in prelude
// pub use crate::verification::Checksum;

pub use crate::prelude::ToString;
pub use crate::prelude::*;

// HashSet and Mutex are in std part of prelude when feature "std" is on.
// For no_std, Mutex comes from once_mutex, HashSet isn't used directly here for no_std provider.
// #[cfg(feature = "std")]
// use std::{collections::HashSet, sync::Mutex};

/// A safe slice with integrated checksum for data integrity verification
#[derive(Clone)]
pub struct SafeSlice<'a> {
    /// The underlying data slice
    data: &'a [u8],
    /// Checksum for data integrity verification
    checksum: Checksum,
    /// Length of the slice for redundant verification
    length: usize,
    /// Verification level for this slice
    verification_level: VerificationLevel,
}

impl<'a> SafeSlice<'a> {
    /// Create a new SafeSlice from a raw byte slice
    ///
    /// This computes a checksum for the data which can be used
    /// to verify integrity later.
    pub fn new(data: &'a [u8]) -> Result<Self> {
        Self::with_verification_level(data, VerificationLevel::default())
    }

    /// Create a new SafeSlice with a specific verification level
    ///
    /// This computes a checksum for the data which can be used
    /// to verify integrity later.
    ///
    /// # Panics
    ///
    /// This function previously panicked if initial verification failed.
    /// It now returns a Result to indicate failure.
    /// Safety impact: [LOW|MEDIUM|HIGH] - [Brief explanation of the safety implication]
    /// Tracking: WRTQ-XXX (qualification requirement tracking ID).
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
    pub fn data(&self) -> Result<&'a [u8]> {
        // Track memory read operation
        record_global_operation(OperationType::MemoryRead, self.verification_level);

        // Medium importance for data access (128)
        self.verify_integrity_with_importance(128)?;
        Ok(self.data)
    }

    /// Get length of the slice
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if the slice is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get the current verification level
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Set a new verification level
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    /// Verify data integrity using the stored checksum
    pub fn verify_integrity(&self) -> Result<()> {
        // Track validation operation
        record_global_operation(OperationType::CollectionValidate, self.verification_level);

        // Medium importance by default (128)
        self.verify_integrity_with_importance(128)
    }

    /// Verify data integrity with specified operation importance
    ///
    /// The importance value (0-255) affects the likelihood of
    /// verification when using VerificationLevel::Sampling
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
    pub fn slice(&self, start: usize, end: usize) -> Result<SafeSlice<'a>> {
        // Track memory read operation
        record_global_operation(OperationType::MemoryRead, self.verification_level);

        // High importance for slicing operations (200)
        self.verify_integrity_with_importance(200)?;

        if start > end || end > self.length {
            return Err(Error::memory_error(format!(
                "Invalid slice range: {}..{} (len: {})",
                start, end, self.length
            )));
        }

        // Create a new SafeSlice with the specified range
        // We need to use just the exact range requested
        let sub_data = &self.data[start..end];

        // Create a new SafeSlice with the same verification level
        SafeSlice::with_verification_level(sub_data, self.verification_level)
    }
}

impl fmt::Debug for SafeSlice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SafeSlice")
            .field("length", &self.length)
            .field("checksum", &self.checksum)
            .field("verification_level", &self.verification_level)
            .finish()
    }
}

/// A safe mutable slice with integrated checksum for data integrity verification
#[derive(Debug)]
pub struct SafeSliceMut<'a> {
    /// The underlying mutable data slice
    data: &'a mut [u8],
    /// Checksum for data integrity verification
    checksum: Checksum,
    /// Length of the slice for redundant verification
    length: usize,
    /// Verification level for this slice
    verification_level: VerificationLevel,
}

impl<'a> SafeSliceMut<'a> {
    /// Create a new SafeSliceMut from a raw mutable byte slice
    ///
    /// This computes a checksum for the data which can be used
    /// to verify integrity.
    pub fn new(data: &'a mut [u8]) -> Result<Self> {
        Self::with_verification_level(data, VerificationLevel::default())
    }

    /// Create a new SafeSliceMut with a specific verification level
    ///
    /// # Panics
    /// This function previously panicked if initial verification failed.
    /// It now returns a Result to indicate failure.
    pub fn with_verification_level(data: &'a mut [u8], level: VerificationLevel) -> Result<Self> {
        record_global_operation(OperationType::ChecksumCalculation, level);
        let checksum = Checksum::compute(data);
        let length = data.len();
        let slice_mut = Self { data, checksum, length, verification_level: level };
        slice_mut.verify_integrity_with_importance(255)?; // Verify on creation
        Ok(slice_mut)
    }

    /// Get a mutable reference to the underlying data slice.
    ///
    /// # Safety
    /// Modifying the returned slice directly will invalidate the stored checksum.
    /// The checksum must be updated using `update_checksum()` after modification.
    /// This performs an integrity check before returning the data.
    pub fn data_mut(&mut self) -> Result<&mut [u8]> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level); // Or a more specific operation
        self.verify_integrity_with_importance(128)?;
        Ok(self.data)
    }
    
    /// Get an immutable reference to the underlying data slice.
    /// This performs an integrity check before returning the data.
    pub fn data(&self) -> Result<&[u8]> {
        record_global_operation(OperationType::MemoryRead, self.verification_level);
        self.verify_integrity_with_importance(128)?;
        Ok(self.data)
    }

    /// Get length of the slice
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if the slice is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get the current verification level
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Set a new verification level
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    /// Recomputes and updates the checksum based on the current data.
    /// This should be called after any direct modification to the slice obtained via `data_mut()`.
    pub fn update_checksum(&mut self) {
        record_global_operation(OperationType::ChecksumCalculation, self.verification_level);
        self.checksum = Checksum::compute(self.data);
    }
    
    /// Verify data integrity using the stored checksum
    pub fn verify_integrity(&self) -> Result<()> {
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        self.verify_integrity_with_importance(128)
    }

    /// Verify data integrity with specified operation importance
    pub fn verify_integrity_with_importance(&self, importance: u8) -> Result<()> {
        if !self.verification_level.should_verify(importance) {
            return Ok(());
        }
        if self.data.len() != self.length {
            return Err(Error::validation_error("Memory corruption detected: length mismatch (SafeSliceMut)"));
        }

        #[cfg(feature = "optimize")]
        {
            Ok(())
        }
        #[cfg(not(feature = "optimize"))]
        {
            if importance >= 200 || self.verification_level.should_verify_redundant() {
                record_global_operation(OperationType::ChecksumCalculation, self.verification_level);
            }
            if !self.verification_level.should_verify_redundant() && importance < 200 {
                return Ok(());
            }
            let current_checksum = Checksum::compute(self.data);
            if current_checksum == self.checksum {
                Ok(())
            } else {
                Err(Error::validation_error(format!(
                    "Memory corruption detected: checksum mismatch (SafeSliceMut). Expected {}, got {}",
                    self.checksum, current_checksum
                )))
            }
        }
    }
    // Note: A `slice_mut` method similar to SafeSlice::slice would be complex due to lifetime issues with sub-slicing mutable references.
    // It's often easier to manage mutable sub-access via the MemoryProvider or SafeMemoryHandler.
}

/// Memory provider interface for different allocation strategies.
///
/// This trait abstracts over different memory allocation strategies,
/// allowing both std and no_std environments to share the same interface.
/// It combines raw access, safety features, and informational methods.
pub trait MemoryProvider: Send + Sync + Debug {
    /// Borrows a slice of memory with safety guarantees.
    /// The returned `SafeSlice` will have its verification level typically
    /// initialized by the provider or a wrapping handler.
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<SafeSlice<'_>>;

    /// Writes data to the memory at a given offset.
    fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()>;

    /// Verifies that an access to memory (read or write) of `len` at `offset` would be valid.
    /// This is a pre-check and does not perform the access.
    fn verify_access(&self, offset: usize, len: usize) -> Result<()>;

    /// Gets the total current size/length of the initialized/used memory within the provider.
    fn size(&self) -> usize;

    /// Gets the total capacity of the memory region managed by the provider.
    fn capacity(&self) -> usize;

    // Methods previously from MemorySafety trait
    /// Verifies the overall integrity of the memory managed by the provider.
    /// This could involve checking internal checksums, canaries, or other mechanisms.
    fn verify_integrity(&self) -> Result<()>;

    /// Sets the verification level for operations performed by this memory provider.
    fn set_verification_level(&mut self, level: VerificationLevel);

    /// Gets the current verification level of this memory provider.
    fn verification_level(&self) -> VerificationLevel;

    /// Gets statistics about memory usage from this provider.
    fn memory_stats(&self) -> MemoryStats;

    /// Gets a mutable slice from the underlying memory provider.
    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SafeSliceMut<'_>>;

    /// Copies data within the memory provider from a source offset to a destination offset.
    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()>;

    // TODO: Consider if methods like `resize`, `clear_data_in_range` are needed on this trait
    // if SafeMemoryHandler is to delegate them generally. For BoundedVec, these might not be
    // directly used from the handler if BoundedVec manages its own logical length within the handler's memory.
}

/// Memory statistics structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryStats {
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
pub struct StdMemoryProvider {
    /// The underlying data buffer
    data: Vec<u8>,
    /// Track memory accesses for safety monitoring
    access_log: Mutex<Vec<(usize, usize)>>,
    /// Counter for access operations
    access_count: AtomicUsize,
    /// Maximum size of any access seen
    max_access_size: AtomicUsize,
    /// Number of unique regions accessed
    unique_regions: AtomicUsize,
    /// Regions hash (for uniqueness tracking)
    regions_hash: Mutex<HashSet<usize>>,
    /// Verification level for memory operations
    verification_level: VerificationLevel,
}

#[cfg(feature = "std")]
impl fmt::Debug for StdMemoryProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdMemoryProvider")
            .field("data_size", &self.data.len())
            .field("access_count", &self.access_count.load(Ordering::Relaxed))
            .field("max_access_size", &self.max_access_size.load(Ordering::Relaxed))
            .field("unique_regions", &self.unique_regions.load(Ordering::Relaxed))
            .field("verification_level", &self.verification_level)
            .finish()
    }
}

#[cfg(feature = "std")]
impl Default for StdMemoryProvider {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            access_log: Mutex::new(Vec::new()),
            access_count: AtomicUsize::new(0),
            max_access_size: AtomicUsize::new(0),
            unique_regions: AtomicUsize::new(0),
            regions_hash: Mutex::new(HashSet::new()),
            verification_level: VerificationLevel::Sampling,
        }
    }
}

#[cfg(feature = "std")]
impl StdMemoryProvider {
    /// Create a new StdMemoryProvider with the given data
    #[allow(clippy::redundant_clone)]
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            access_log: Mutex::new(Vec::with_capacity(100)),
            access_count: AtomicUsize::new(0),
            max_access_size: AtomicUsize::new(0),
            unique_regions: AtomicUsize::new(0),
            regions_hash: Mutex::new(HashSet::new()),
            verification_level: VerificationLevel::default(),
        }
    }

    /// Create a new empty StdMemoryProvider with the given capacity
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new(Vec::with_capacity(100))
    }

    /// Get the access log for debugging and verification
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
    }

    /// Resize the memory to the given size
    pub fn resize(&mut self, new_size: usize, value: u8) -> Result<()> {
        self.data.resize(new_size, value);
        // After resizing, provider's internal checksums might be invalid if it has them.
        // For StdMemoryProvider, it relies on SafeSlice, which recomputes on demand.
        // However, if StdMemoryProvider itself maintained a checksum, it would need update.
        Ok(())
    }

    /// Track a memory access for safety monitoring
    fn track_access(&self, offset: usize, len: usize) {
        // Record memory read operation
        record_global_operation(OperationType::MemoryRead, self.verification_level);

        self.access_count.fetch_add(1, Ordering::Relaxed);

        // Update max access size if this is larger
        let current_max = self.max_access_size.load(Ordering::Relaxed);
        if len > current_max {
            self.max_access_size.store(len, Ordering::Relaxed);
        }

        // Track unique regions accessed (approximate with offset/1024)
        let region = offset >> 10; // Divide by 1024 to get region

        if let Ok(mut regions) = self.regions_hash.lock() {
            if regions.insert(region) {
                // New region, increment unique regions count
                self.unique_regions.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Log the access if logging is enabled
        if let Ok(mut log) = self.access_log.lock() {
            log.push((offset, len));
        }
    }

    /// Clear the access tracking data but preserve statistics
    pub fn clear_access_tracking(&self) -> Result<()> {
        match self.access_log.lock() {
            Ok(mut log) => {
                log.clear();
                Ok(())
            }
            Err(_) => Err(Error::runtime_error("Access log mutex poisoned")),
        }
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
    pub fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            total_size: self.data.len(),
            access_count: self.access_count.load(Ordering::Relaxed),
            unique_regions: self.unique_regions.load(Ordering::Relaxed),
            max_access_size: self.max_access_size.load(Ordering::Relaxed),
        }
    }

    /// Recalculates checksums after direct memory modifications
    ///
    /// This method should be called after directly modifying the underlying data
    /// to ensure memory integrity verification continues to work properly.
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
        Some(self.data.as_ptr().wrapping_add(offset) as *mut u8)
    }
}

#[cfg(feature = "std")]
impl MemoryProvider for StdMemoryProvider {
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<SafeSlice<'_>> {
        record_global_operation(OperationType::MemoryRead, self.verification_level);
        self.track_access(offset, len);

        if offset.saturating_add(len) > self.data.len() {
            return Err(Error::memory_error(format!(
                "Slice out of bounds: offset {} + len {} > total_len {}",
                offset,
                len,
                self.data.len()
            )));
        }

        // The bounds check above ensures that `get` will return Some.
        let slice = self.data.get(offset..offset + len).ok_or_else(|| {
            // This path should ideally be unreachable due to the check above.
            // If reached, it implies a logic error or an unexpected state.
            Error::new(
                ErrorCategory::Internal,
                codes::INTERNAL_ERROR, // Assuming a generic internal error code exists
                "Internal error: slice bounds check failed unexpectedly in StdMemoryProvider::borrow_slice".to_string(),
            )
        })?;
        SafeSlice::with_verification_level(slice, self.verification_level)
    }

    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SafeSliceMut<'_>> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.verify_access(offset, len)?;
        let slice = &mut self.data[offset..offset + len];
        SafeSliceMut::new(slice)
    }

    fn write_data(&mut self, offset: usize, data_to_write: &[u8]) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.track_access(offset, data_to_write.len());

        // Check bounds against current length of the initialized data
        if offset.saturating_add(data_to_write.len()) > self.data.len() {
            // If the write would exceed current length but is within capacity,
            // it might be an error or require resizing self.data.len().
            // For now, strictly adhere to writing within current self.data.len().
            // If writing to capacity is desired, self.data.resize() should be used first.
            return Err(Error::memory_error(format!(
                "Write out of bounds: offset {} + len {} > current_len {}. Provider capacity is {}.",
                offset,
                data_to_write.len(),
                self.data.len(),
                self.data.capacity()
            )));
        }

        // Safety: Bounds check ensures that `offset + data_to_write.len()` does not exceed `self.data.len()`,
        // which is inherently within `self.data.capacity()`.
        // Thus, `self.data[offset..offset + data_to_write.len()]` is a valid slice.
        self.data[offset..offset + data_to_write.len()].copy_from_slice(data_to_write);

        Ok(())
    }

    fn verify_access(&self, offset: usize, len: usize) -> Result<()> {
        // Track validation operation
        record_global_operation(OperationType::CollectionValidate, self.verification_level);

        // Calculate the end offset
        let end = offset
            .checked_add(len)
            .ok_or_else(|| Error::memory_error("Memory access overflow".to_string()))?;

        // Check if the access is within bounds
        if end > self.data.len() {
            return Err(Error::memory_error(format!(
                "Memory access out of bounds: offset={}, len={}, size={}",
                offset,
                len,
                self.data.len()
            )));
        }

        Ok(())
    }

    fn size(&self) -> usize {
        self.data.len()
    }

    fn capacity(&self) -> usize {
        self.data.capacity()
    }

    fn verify_integrity(&self) -> Result<()> {
        // Track validation operation
        record_global_operation(OperationType::CollectionValidate, self.verification_level);

        // For full verification, calculate a checksum of the entire memory
        if matches!(self.verification_level, VerificationLevel::Full) {
            record_global_operation(OperationType::ChecksumCalculation, self.verification_level);

            // Calculate checksum of all used memory
            let _checksum = Checksum::compute(&self.data);

            // In a real implementation, we would compare this against a stored
            // checksum, but for now we just calculate it for fuel accounting
        }

        Ok(())
    }

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    fn memory_stats(&self) -> MemoryStats {
        self.memory_stats()
    }

    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        // Verify both source and destination ranges
        self.verify_access(src_offset, len)?;
        self.verify_access(dst_offset, len)?;
        // Ensure total capacity is not exceeded by dst_offset + len
        if dst_offset.checked_add(len).map_or(true, |end| end > self.data.len()) {
            return Err(Error::memory_error(format!(
                "copy_within destination out of bounds: offset {}, len {}, capacity {}",
                dst_offset, len, self.data.len()
            )));
        }
        
        self.data.copy_within(src_offset..src_offset + len, dst_offset);
        // Checksums on individual SafeSlices are not managed here; BoundedVec recalculates its own.
        Ok(())
    }
}

/// Memory provider for no-std environments
///
/// This provider uses a fixed-size array and manages accesses with atomic operations.
/// It provides memory safety and verification similar to StdMemoryProvider but with
/// a simpler implementation suitable for environments without std.
#[cfg(not(feature = "std"))]
pub struct NoStdMemoryProvider<const N: usize> {
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
impl<const N: usize> fmt::Debug for NoStdMemoryProvider<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoStdMemoryProvider")
            .field("used", &self.used)
            .field("access_count", &self.access_count.load(Ordering::Relaxed))
            .field("verification_level", &self.verification_level)
            .finish()
    }
}

#[cfg(not(feature = "std"))]
impl<const N: usize> Default for NoStdMemoryProvider<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "std"))]
impl<const N: usize> NoStdMemoryProvider<N> {
    /// Create a new empty memory provider
    pub fn new() -> Self {
        Self {
            data: [0; N],
            used: 0,
            access_count: AtomicUsize::new(0),
            last_access_offset: AtomicUsize::new(0),
            last_access_length: AtomicUsize::new(0),
            verification_level: VerificationLevel::Standard,
        }
    }

    /// Set the data for this memory provider
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
    pub fn memory_stats(&self) -> MemoryStats {
        let access_count = self.access_count();
        let (_, length) = self.last_access();

        MemoryStats {
            total_size: self.size(),
            access_count,
            unique_regions: 1, // No-std can't track unique regions
            max_access_size: length,
        }
    }

    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SafeSliceMut<'_>> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.verify_access(offset, len)?;
        let slice = &mut self.data[offset..offset + len];
        SafeSliceMut::new(slice)
    }

    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.verify_access(src_offset, len)?;
        self.verify_access(dst_offset, len)?;

        if src_offset.checked_add(len).map_or(true, |end| end > self.used) ||
           dst_offset.checked_add(len).map_or(true, |end| end > N) {
            return Err(Error::memory_error("copy_within source or destination out of bounds"));
        }

        // Manual copy_within to handle potential overlaps correctly for [u8]
        if src_offset == dst_offset { return Ok(()); }

        if src_offset < dst_offset {
            // Copy backwards
            for i in (0..len).rev() {
                self.data[dst_offset + i] = self.data[src_offset + i];
            }
        } else {
            // Copy forwards
            for i in 0..len {
                self.data[dst_offset + i] = self.data[src_offset + i];
            }
        }
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl<const N: usize> MemoryProvider for NoStdMemoryProvider<N> {
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<SafeSlice<'_>> {
        // Track memory read operation
        record_global_operation(OperationType::MemoryRead, self.verification_level);

        self.verify_access(offset, len)?;
        let slice_data = self.data.get(offset..offset + len).ok_or_else(|| {
            Error::memory_error(format!(
                "Internal error: borrow_slice access granted but slice.get failed for {}..{}",
                offset,
                offset + len
            ))
        })?;
        SafeSlice::with_verification_level(slice_data, self.verification_level)
    }

    fn write_data(&mut self, offset: usize, data_to_write: &[u8]) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.verify_access(offset, data_to_write.len())?;

        let end_offset = offset.saturating_add(data_to_write.len());

        // Get mutable slice and write data
        if let Some(slice_to_write_in) = self.data.get_mut(offset..end_offset) {
            slice_to_write_in.copy_from_slice(data_to_write);
        } else {
            // This should ideally be caught by verify_access, but as a safeguard:
            return Err(Error::memory_error(format!(
                "Internal error: write_data access granted but slice.get_mut failed for {}..{}",
                offset, end_offset
            )));
        }

        // Update `used` if this write extends the known used area
        if end_offset > self.used {
            self.used = end_offset;
        }

        // Update access tracking (simplified)
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.last_access_offset.store(offset, Ordering::Relaxed);
        self.last_access_length.store(data_to_write.len(), Ordering::Relaxed);

        Ok(())
    }

    fn verify_access(&self, offset: usize, len: usize) -> Result<()> {
        // Range check to ensure the access is within bounds
        let end = offset.saturating_add(len);

        // Check if offset + len overflows
        if end < offset {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                format!("Memory access overflow: offset={}, len={}", offset, len),
            ));
        }

        // Check if the access is within the used portion of memory
        if end > self.used {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                format!(
                    "Memory access out of bounds: offset={}, len={}, used={}",
                    offset, len, self.used
                ),
            ));
        }

        // Track the access
        self.last_access_offset.store(offset, Ordering::SeqCst);
        self.last_access_length.store(len, Ordering::SeqCst);

        Ok(())
    }

    fn size(&self) -> usize {
        self.used
    }

    fn capacity(&self) -> usize {
        N
    }

    fn verify_integrity(&self) -> Result<()> {
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

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    fn memory_stats(&self) -> MemoryStats {
        self.memory_stats()
    }

    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.verify_access(src_offset, len)?;
        self.verify_access(dst_offset, len)?;

        if src_offset.checked_add(len).map_or(true, |end| end > self.used) ||
           dst_offset.checked_add(len).map_or(true, |end| end > N) {
            return Err(Error::memory_error("copy_within source or destination out of bounds"));
        }

        // Manual copy_within to handle potential overlaps correctly for [u8]
        if src_offset == dst_offset { return Ok(()); }

        if src_offset < dst_offset {
            // Copy backwards
            for i in (0..len).rev() {
                self.data[dst_offset + i] = self.data[src_offset + i];
            }
        } else {
            // Copy forwards
            for i in 0..len {
                self.data[dst_offset + i] = self.data[src_offset + i];
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::format;

    #[cfg(not(feature = "std"))]
    use super::NoStdMemoryProvider;
    #[cfg(feature = "std")]
    use super::StdMemoryProvider;

    #[test]
    #[cfg(feature = "std")]
    fn test_std_memory_provider() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let provider = StdMemoryProvider::new(data);

        // Test valid access
        let slice = provider.borrow_slice(1, 3).unwrap();
        assert_eq!(slice.data().unwrap(), &[2, 3, 4]);

        // Test out of bounds
        assert!(provider.borrow_slice(6, 3).is_err());
    }

    #[test]
    fn test_safe_slice() {
        let data = [1, 2, 3, 4, 5];
        let slice = SafeSlice::new(&data).unwrap();

        // Test data access
        assert_eq!(slice.data().unwrap(), &[1, 2, 3, 4, 5]);

        // Test sub-slicing
        let sub = slice.slice(1, 4).unwrap();
        assert_eq!(sub.data().unwrap(), &[2, 3, 4]);

        // Test invalid sub-slice
        assert!(slice.slice(3, 6).is_err());
    }

    #[test]
    #[cfg(not(feature = "std"))]
    fn test_no_std_memory_provider() {
        // Create a provider with fixed size 10
        let mut provider = NoStdMemoryProvider::<10>::new();

        // Set some data
        provider.set_data(&[1, 2, 3, 4, 5]).unwrap();

        // Test valid access
        let slice = provider.borrow_slice(1, 3).unwrap();
        assert_eq!(slice.data().unwrap(), &[2, 3, 4]);

        // Test out of bounds
        assert!(provider.borrow_slice(3, 3).is_err());

        // Test setting verification level
        provider.set_verification_level(VerificationLevel::None);
        assert_eq!(provider.verification_level(), VerificationLevel::None);

        // Resize within bounds
        assert!(provider.resize(8).is_ok());

        // Resize beyond bounds should fail
        assert!(provider.resize(11).is_err());

        // Verify integrity
        assert!(provider.verify_integrity().is_ok());

        // Test memory stats
        let stats = provider.memory_stats();
        assert_eq!(stats.total_size, 8);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_safe_memory_handler_modifications() {
        // Test with StdMemoryProvider
        let std_provider = StdMemoryProvider::with_capacity(1024);
        let mut std_handler = SafeMemoryHandler::new(std_provider, VerificationLevel::Standard);

        std_handler.set_verification_level(VerificationLevel::Full);
        assert_eq!(std_handler.verification_level(), VerificationLevel::Full);
        assert_eq!(std_handler.provider().verification_level(), VerificationLevel::Full);
    }
}

/// A generic handler for safe memory operations, backed by a MemoryProvider.
///
/// This struct acts as a wrapper around a `MemoryProvider` implementation,
/// adding a layer of verification control and a consistent interface.
#[derive(Debug)]
pub struct SafeMemoryHandler<P: MemoryProvider> {
    /// The internal memory provider instance.
    provider: P,
    /// Verification level for operations managed by this handler.
    /// This level can be used to influence checks within the handler itself
    /// or be passed down to the provider if the provider also uses it.
    verification_level: VerificationLevel,
}

impl<P: MemoryProvider> SafeMemoryHandler<P> {
    /// Creates a new `SafeMemoryHandler` with a given provider and verification level.
    ///
    /// The provider is moved into the handler.
    pub fn new(mut provider: P, verification_level: VerificationLevel) -> Self {
        // Synchronize the handler's verification level with the provider's initial level.
        // Or, the provider could be expected to be pre-configured.
        // For consistency, let's set the provider's level from the handler's level.
        provider.set_verification_level(verification_level);
        Self { provider, verification_level }
    }

    /// Sets the verification level for this handler and its underlying provider.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
        self.provider.set_verification_level(level);
    }

    /// Gets the current verification level of this handler.
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Gets a safe slice from the underlying memory provider.
    /// The verification level of the returned SafeSlice is determined by the provider
    /// or how SafeSlice::with_verification_level is called within the provider's borrow_slice.
    pub fn get_slice(&self, offset: usize, len: usize) -> Result<SafeSlice<'_>> {
        // The handler's verification level could be used here for additional checks
        // before or after calling the provider, if necessary.
        // For now, direct delegation for core operation.
        self.provider.borrow_slice(offset, len)
    }

    /// Writes data to the underlying memory provider.
    pub fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        // Pre-access verification using the provider itself.
        self.provider.verify_access(offset, data.len())?;
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
    pub fn memory_stats(&self) -> MemoryStats {
        self.provider.memory_stats()
    }

    /// Provides immutable access to the underlying provider (e.g., for provider-specific methods or inspection in tests).
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Provides mutable access to the underlying provider (e.g., for provider-specific methods or inspection in tests).
    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    /// Gets a mutable slice from the underlying memory provider.
    pub fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SafeSliceMut<'_>> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        self.provider.verify_access(offset, len)?;
        self.provider.get_slice_mut(offset, len)
    }

    /// Copies data within the memory provider from a source offset to a destination offset.
    pub fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        record_global_operation(OperationType::MemoryWrite, self.verification_level);
        // Provider should verify access internally
        self.provider.copy_within(src_offset, dst_offset, len)
    }

    // TODO: Review other methods that were on the concrete SafeMemoryHandler versions
    // (e.g., resize, clear, to_vec, add_data for StdMemoryProvider version) and decide if they
    // should be on the MemoryProvider trait and delegated here, or if they are provider-specific
    // and should be accessed via provider() or provider_mut().
    // For BoundedVec, the current set of delegated methods is likely sufficient.
}

// Constructors for specific default providers can be added using inherent impls with feature gates.
#[cfg(feature = "std")]
impl SafeMemoryHandler<StdMemoryProvider> {
    /// Creates a new `SafeMemoryHandler` backed by an `StdMemoryProvider`
    /// initialized with a specific capacity and verification level.
    pub fn with_std_provider(capacity: usize, verification_level: VerificationLevel) -> Self {
        let mut provider = StdMemoryProvider::with_capacity(capacity);
        provider.set_verification_level(verification_level); // Ensure provider's level is set
        Self { provider, verification_level }
    }
}

#[cfg(all(not(feature = "std"), feature = "alloc"))]
impl<const N: usize> SafeMemoryHandler<NoStdMemoryProvider<N>> {
    /// Creates a new `SafeMemoryHandler` backed by a `NoStdMemoryProvider`
    /// with a compile-time fixed size N and a given verification level.
    pub fn with_no_std_provider(verification_level: VerificationLevel) -> Self {
        let mut provider = NoStdMemoryProvider::<N>::default(); // or new()
        provider.set_verification_level(verification_level);
        Self { provider, verification_level }
    }
}

#[cfg(test)]
mod fault_injection {
    use super::*; // To get MemoryProvider, Result, SafeSlice, Error, VerificationLevel, MemoryStats etc.
    use core::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct FaultConfig {
        pub fail_on_borrow_slice: bool,
        pub fail_on_write_data: bool,
        pub fail_on_verify_access: bool,
        pub fail_on_verify_integrity: bool,
        // Add more specific controls if needed, e.g., counters, specific error types
    }

    #[derive(Debug)]
    pub struct FaultyMemoryProvider<P: MemoryProvider> {
        inner_provider: P,
        fault_config: RefCell<FaultConfig>, // RefCell for interior mutability of config
    }

    impl<P: MemoryProvider> FaultyMemoryProvider<P> {
        pub fn new(inner_provider: P) -> Self {
            Self { inner_provider, fault_config: RefCell::new(FaultConfig::default()) }
        }

        pub fn set_fault_config(&self, config: FaultConfig) {
            *self.fault_config.borrow_mut() = config;
        }

        pub fn reset_fault_config(&self) {
            *self.fault_config.borrow_mut() = FaultConfig::default();
        }

        pub fn configure_fail_on_next_borrow_slice(&self, fail: bool) {
            self.fault_config.borrow_mut().fail_on_borrow_slice = fail;
        }

        pub fn configure_fail_on_next_write_data(&self, fail: bool) {
            self.fault_config.borrow_mut().fail_on_write_data = fail;
        }

        pub fn configure_fail_on_next_verify_access(&self, fail: bool) {
            self.fault_config.borrow_mut().fail_on_verify_access = fail;
        }

        pub fn configure_fail_on_next_verify_integrity(&self, fail: bool) {
            self.fault_config.borrow_mut().fail_on_verify_integrity = fail;
        }
    }

    impl<P: MemoryProvider> MemoryProvider for FaultyMemoryProvider<P> {
        fn borrow_slice(&self, offset: usize, len: usize) -> Result<SafeSlice<'_>> {
            if self.fault_config.borrow().fail_on_borrow_slice {
                // Reset flag after firing, if desired (depends on testing strategy)
                // self.fault_config.borrow_mut().fail_on_borrow_slice = false;
                return Err(Error::memory_error(
                    "FaultyMemoryProvider: Forced borrow_slice failure".to_string(),
                ));
            }
            self.inner_provider.borrow_slice(offset, len)
        }

        fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
            // Note: fault_config is &RefCell, self is &mut. RefCell allows borrowing config mutably.
            if self.fault_config.borrow().fail_on_write_data {
                return Err(Error::memory_error(
                    "FaultyMemoryProvider: Forced write_data failure".to_string(),
                ));
            }
            self.inner_provider.write_data(offset, data)
        }

        fn verify_access(&self, offset: usize, len: usize) -> Result<()> {
            if self.fault_config.borrow().fail_on_verify_access {
                return Err(Error::validation_error(
                    "FaultyMemoryProvider: Forced verify_access failure".to_string(),
                ));
            }
            self.inner_provider.verify_access(offset, len)
        }

        fn size(&self) -> usize {
            self.inner_provider.size()
        }

        fn capacity(&self) -> usize {
            self.inner_provider.capacity()
        }

        fn verify_integrity(&self) -> Result<()> {
            if self.fault_config.borrow().fail_on_verify_integrity {
                return Err(Error::validation_error(
                    "FaultyMemoryProvider: Forced verify_integrity failure".to_string(),
                ));
            }
            self.inner_provider.verify_integrity()
        }

        fn set_verification_level(&mut self, level: VerificationLevel) {
            self.inner_provider.set_verification_level(level);
            // Propagate to inner provider. Faulty provider itself doesn't use verification_level directly.
        }

        fn verification_level(&self) -> VerificationLevel {
            self.inner_provider.verification_level()
        }

        fn memory_stats(&self) -> MemoryStats {
            self.inner_provider.memory_stats()
        }

        fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SafeSliceMut<'_>> {
            record_global_operation(OperationType::MemoryWrite, self.verification_level);
            self.verify_access(offset, len)?;
            self.inner_provider.get_slice_mut(offset, len)
        }

        fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
            record_global_operation(OperationType::MemoryWrite, self.verification_level);
            self.verify_access(src_offset, len)?;
            self.verify_access(dst_offset, len)?;

            if src_offset.checked_add(len).map_or(true, |end| end > self.inner_provider.size()) ||
               dst_offset.checked_add(len).map_or(true, |end| end > self.inner_provider.capacity()) {
                return Err(Error::memory_error("copy_within source or destination out of bounds"));
            }

            // Manual copy_within to handle potential overlaps correctly for [u8]
            if src_offset == dst_offset { return Ok(()); }

            if src_offset < dst_offset {
                // Copy backwards
                for i in (0..len).rev() {
                    self.inner_provider.data[dst_offset + i] = self.inner_provider.data[src_offset + i];
                }
            } else {
                // Copy forwards
                for i in 0..len {
                    self.inner_provider.data[dst_offset + i] = self.inner_provider.data[src_offset + i];
                }
            }
            Ok(())
        }
    }
}
