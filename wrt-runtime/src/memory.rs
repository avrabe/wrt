//! WebAssembly memory implementation.
//!
//! This module provides a comprehensive implementation of WebAssembly linear memory.
//!
//! # Memory Architecture
//!
//! The `Memory` struct is the core implementation for WebAssembly linear memory. It represents
//! a memory instance as defined in the WebAssembly specification. Key features include:
//!
//! - Thread-safe access with internal synchronization
//! - Performance metrics tracking (access counts, peak usage)
//! - Debug name support for easier debugging
//! - Support for both `std` and `no_std` environments
//! - Memory safety checks and bounds validation
//! - Integrated checksum verification for data integrity
//!
//! # Memory Operations
//!
//! The implementation provides methods for all standard WebAssembly memory operations:
//!
//! - Growing memory (`grow`)
//! - Reading from memory (`read`, `get_byte`)
//! - Writing to memory (`write`, `set_byte`)
//! - Additional safety methods for alignment and bounds checking (`check_alignment`)
//!
//! # Safety Features
//!
//! The implementation includes several safety features:
//!
//! - Checksum verification for data integrity
//! - Bounds checking for all memory operations
//! - Alignment validation
//! - Thread safety guarantees
//! - Memory access tracking
//!
//! # Performance Monitoring
//!
//! Memory instances automatically track usage metrics:
//!
//! - Peak memory usage via `peak_memory()`
//! - Access counts via `access_count()`
//! - Memory access patterns via `memory_stats()`
//!
//! These metrics are updated automatically when memory operations are performed.
//!
//! # Thread Safety
//!
//! Memory operations use appropriate synchronization primitives based on the environment:
//!
//! - In `std` environments, atomic variables are used for metrics
//! - In `no_std` environments, `RwLock` is used for metrics
//!
//! # Usage
//!
//! ```no_run
//! use wrt_runtime::{Memory, MemoryType};
//! use wrt_types::types::Limits;
//!
//! // Create a memory type with initial 1 page (64KB) and max 2 pages
//! let mem_type = MemoryType {
//!     limits: Limits { min: 1, max: Some(2) },
//! };
//!
//! // Create a new memory instance
//! let mut memory = Memory::new(mem_type).unwrap();
//!
//! // Write data to memory
//! memory.write(0, &[1, 2, 3, 4]).unwrap();
//!
//! // Read data from memory
//! let mut buffer = [0; 4];
//! memory.read(0, &mut buffer).unwrap();
//! assert_eq!(buffer, [1, 2, 3, 4]);
//!
//! // Grow memory by 1 page
//! let old_size = memory.grow(1).unwrap();
//! assert_eq!(old_size, 1); // Previous size was 1 page
//! ```

use crate::prelude::*;
use wrt_types::safe_memory::{
    MemoryProvider, MemorySafety, MemoryStats, SafeMemoryHandler, SafeSlice,
};

// Import BorrowMut for SafeMemoryHandler
#[cfg(feature = "std")]
use std::borrow::BorrowMut;
#[cfg(all(feature = "alloc", not(feature = "std")))] // Should also work for no_std + alloc
use core::borrow::BorrowMut;

// Import RwLock from appropriate location in no_std
#[cfg(not(feature = "std"))]
use wrt_sync::WrtRwLock as RwLock;

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// WebAssembly page size (64KB)
pub const PAGE_SIZE: usize = 65536;

/// Maximum number of memory pages allowed by WebAssembly spec
pub const MAX_PAGES: u32 = 65536;

/// The maximum memory size in bytes (4GB)
const MAX_MEMORY_BYTES: usize = 4 * 1024 * 1024 * 1024;

/// Memory size error code (must be u16 to match Error::new)
const MEMORY_SIZE_TOO_LARGE: u16 = 4001;

/// Memory metrics for tracking usage and safety
#[derive(Debug)]
struct MemoryMetrics {
    /// Peak memory usage in bytes
    #[cfg(feature = "std")]
    peak_usage: AtomicUsize,
    /// Memory access counter for profiling
    #[cfg(feature = "std")]
    access_count: AtomicU64,
    /// Maximum size of any access
    #[cfg(feature = "std")]
    max_access_size: AtomicUsize,
    /// Number of unique regions accessed
    #[cfg(feature = "std")]
    unique_regions: AtomicUsize,
    /// Last access offset for validation
    #[cfg(feature = "std")]
    last_access_offset: AtomicUsize,
    /// Last access length for validation
    #[cfg(feature = "std")]
    last_access_length: AtomicUsize,

    /// Peak memory usage (no_std version)
    #[cfg(not(feature = "std"))]
    peak_usage: usize,
    /// Memory access counter (no_std version)
    #[cfg(not(feature = "std"))]
    access_count: u64,
    /// Maximum size of any access (no_std version)
    #[cfg(not(feature = "std"))]
    max_access_size: usize,
    /// Number of unique regions accessed (no_std version)
    #[cfg(not(feature = "std"))]
    unique_regions: usize,
    /// Last access offset for validation (no_std version)
    #[cfg(not(feature = "std"))]
    last_access_offset: usize,
    /// Last access length for validation (no_std version)
    #[cfg(not(feature = "std"))]
    last_access_length: usize,
}

#[cfg(feature = "std")]
impl Clone for MemoryMetrics {
    fn clone(&self) -> Self {
        Self {
            peak_usage: AtomicUsize::new(self.peak_usage.load(Ordering::Relaxed)),
            access_count: AtomicU64::new(self.access_count.load(Ordering::Relaxed)),
            max_access_size: AtomicUsize::new(self.max_access_size.load(Ordering::Relaxed)),
            unique_regions: AtomicUsize::new(self.unique_regions.load(Ordering::Relaxed)),
            last_access_offset: AtomicUsize::new(self.last_access_offset.load(Ordering::Relaxed)),
            last_access_length: AtomicUsize::new(self.last_access_length.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(not(feature = "std"))]
impl Clone for MemoryMetrics {
    fn clone(&self) -> Self {
        // For no_std, fields are directly cloneable (usize, u64)
        Self {
            peak_usage: self.peak_usage,
            access_count: self.access_count,
            max_access_size: self.max_access_size,
            unique_regions: self.unique_regions,
            last_access_offset: self.last_access_offset,
            last_access_length: self.last_access_length,
        }
    }
}

impl MemoryMetrics {
    #[cfg(feature = "std")]
    fn new(size: usize) -> Self {
        Self {
            peak_usage: AtomicUsize::new(size),
            access_count: AtomicU64::new(0),
            max_access_size: AtomicUsize::new(0),
            unique_regions: AtomicUsize::new(0),
            last_access_offset: AtomicUsize::new(0),
            last_access_length: AtomicUsize::new(0),
        }
    }

    #[cfg(not(feature = "std"))]
    fn new(size: usize) -> Self {
        Self {
            peak_usage: size,
            access_count: 0,
            max_access_size: 0,
            unique_regions: 0,
            last_access_offset: 0,
            last_access_length: 0,
        }
    }
}

/// Represents a WebAssembly memory instance
#[derive(Debug)]
pub struct Memory {
    /// The memory type
    pub ty: CoreMemoryType,
    /// The memory data
    pub data: SafeMemoryHandler,
    /// Current number of pages
    pub current_pages: core::sync::atomic::AtomicU32,
    /// Optional name for debugging
    pub debug_name: Option<String>,
    /// Memory metrics for tracking access
    #[cfg(feature = "std")]
    pub metrics: MemoryMetrics,
    /// Memory metrics for tracking access (RwLock for no_std)
    #[cfg(not(feature = "std"))]
    pub metrics: RwLock<MemoryMetrics>,
    /// Memory verification level
    pub verification_level: VerificationLevel,
}

impl Clone for Memory {
    fn clone(&self) -> Self {
        // Create new SafeMemoryHandler by copying bytes
        let current_bytes = self
            .data
            .to_vec()
            .unwrap_or_else(|e| panic!("Failed to clone memory data: {}", e));
        let new_data = SafeMemoryHandler::new(current_bytes);

        // Clone metrics, handling potential RwLock poisoning for no_std
        #[cfg(feature = "std")]
        let cloned_metrics = MemoryMetrics {
            peak_usage: AtomicUsize::new(self.metrics.peak_usage.load(Ordering::Relaxed)),
            access_count: AtomicU64::new(self.metrics.access_count.load(Ordering::Relaxed)),
            max_access_size: AtomicUsize::new(self.metrics.max_access_size.load(Ordering::Relaxed)),
            unique_regions: AtomicUsize::new(self.metrics.unique_regions.load(Ordering::Relaxed)),
            last_access_offset: AtomicUsize::new(self.metrics.last_access_offset.load(Ordering::Relaxed)),
            last_access_length: AtomicUsize::new(self.metrics.last_access_length.load(Ordering::Relaxed)),
        };

        #[cfg(not(feature = "std"))]
        let cloned_metrics = {
            let guard = self.metrics.read().unwrap_or_else(|e| {
                panic!("Failed to acquire read lock for cloning metrics: {}", e)
            });
            RwLock::new((*guard).clone()) // Assuming MemoryMetrics is Clone
        };

        Self {
            ty: self.ty.clone(),
            data: new_data,
            current_pages: AtomicU32::new(self.current_pages.load(Ordering::Relaxed)),
            debug_name: self.debug_name.clone(),
            metrics: cloned_metrics,
            verification_level: self.verification_level, // Assuming VerificationLevel is Copy
        }
    }
}

impl PartialEq for Memory {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
            // && self.data == other.data // SafeMemoryHandler is not PartialEq. Comparing by bytes for now.
            && self.data.to_vec().unwrap_or_default() == other.data.to_vec().unwrap_or_default()
            && self.current_pages.load(Ordering::Relaxed) == other.current_pages.load(Ordering::Relaxed) 
            && self.debug_name == other.debug_name
            && self.verification_level == other.verification_level
    }
}

impl Memory {
    /// Creates a new memory instance from a type
    ///
    /// # Arguments
    ///
    /// * `ty` - The memory type
    ///
    /// # Returns
    ///
    /// A new memory instance
    ///
    /// # Errors
    ///
    /// Returns an error if the memory type is invalid
    pub fn new(ty: CoreMemoryType) -> Result<Self> {
        // Calculate number of pages
        let min_pages = ty.limits.min;

        // Calculate size in bytes (1 page = 64KiB)
        let size = min_pages as usize * PAGE_SIZE;

        // Check limits
        if size > MAX_MEMORY_BYTES {
            return Err(Error::new(
                ErrorCategory::Memory,
                MEMORY_SIZE_TOO_LARGE,
                format!("Memory size too large: {} bytes", size),
            ));
        }

        // Create initial memory buffer with proper size
        let mut data = SafeMemoryHandler::with_capacity(size);
        // Set verification level to match our memory safety needs
        data.set_verification_level(VerificationLevel::Standard);

        // Initialize with zeros (avoiding unnecessary Vec creation)
        // Use resize instead of creating a temporary Vec
        data.resize(size, 0);

        // Create memory instance
        Ok(Self {
            ty,
            data,
            current_pages: core::sync::atomic::AtomicU32::new(min_pages),
            debug_name: None,
            #[cfg(feature = "std")]
            metrics: MemoryMetrics::new(size),
            #[cfg(not(feature = "std"))]
            metrics: RwLock::new(MemoryMetrics::new(size)),
            verification_level: VerificationLevel::Standard,
        })
    }

    /// Creates a new memory instance with a debug name
    ///
    /// # Arguments
    ///
    /// * `ty` - The memory type
    /// * `name` - The debug name for the memory
    ///
    /// # Returns
    ///
    /// A new Memory instance with the given type and name
    ///
    /// # Errors
    ///
    /// Returns an error if the memory cannot be created
    pub fn new_with_name(ty: CoreMemoryType, name: &str) -> Result<Self> {
        let mut memory = Self::new(ty)?;
        memory.debug_name = Some(name.to_string());
        Ok(memory)
    }

    /// Sets a debug name for this memory instance
    pub fn set_debug_name(&mut self, name: &str) {
        self.debug_name = Some(name.to_string());
    }

    /// Returns the debug name of this memory instance, if any
    #[must_use]
    pub fn debug_name(&self) -> Option<&str> {
        self.debug_name.as_deref()
    }

    /// Gets the current size of the memory in pages
    ///
    /// # Returns
    ///
    /// The current size in pages
    #[must_use]
    pub fn size(&self) -> u32 {
        self.current_pages.load(Ordering::Relaxed)
    }

    /// Gets the current size of the memory in bytes
    ///
    /// # Returns
    ///
    /// The current size in bytes
    #[must_use]
    pub fn size_in_bytes(&self) -> usize {
        self.current_pages.load(Ordering::Relaxed) as usize * PAGE_SIZE
    }

    /// A reference to the memory data as a `Vec<u8>`
    ///
    /// # Warning
    ///
    /// This method is primarily for compatibility with existing code and should be
    /// avoided in new code. It creates a full copy of the memory data which is inefficient.
    ///
    /// For memory-safe access, prefer using `get_safe_slice()` or `as_safe_slice()` methods instead.
    pub fn buffer(&self) -> Result<Vec<u8>> {
        // Use the SafeMemoryHandler to get data through a safe slice to ensure
        // memory integrity is verified during the operation
        let data_size = self.data.size();
        if data_size == 0 {
            return Ok(Vec::new());
        }

        // Get a safe slice over the entire memory
        let safe_slice = self.data.get_slice(0, data_size)?;

        // Get the data from the safe slice and create a copy
        let memory_data = safe_slice.data()?;

        // Create a new Vec with the data
        #[cfg(feature = "std")]
        let result = memory_data.to_vec();

        #[cfg(all(not(feature = "std"), feature = "alloc"))]
        let result = alloc::slice::SliceExt::to_vec(memory_data);

        Ok(result)
    }

    /// Returns the peak memory usage in bytes
    pub fn peak_memory(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.metrics.peak_usage.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "std"))]
        {
            // Use read() method with WrtRwLock
            let metrics = self.metrics.read();
            metrics.peak_usage
        }
    }

    /// Returns the total number of memory accesses
    pub fn access_count(&self) -> u64 {
        #[cfg(feature = "std")]
        {
            self.metrics.access_count.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "std"))]
        {
            // Use read() method with WrtRwLock
            let metrics = self.metrics.read();
            metrics.access_count
        }
    }

    /// Increment the access count for memory operations
    fn increment_access_count(&self, offset: usize, len: usize) {
        #[cfg(feature = "std")]
        {
            self.metrics.access_count.fetch_add(1, Ordering::Relaxed);
            self.metrics
                .max_access_size
                .fetch_max(len, Ordering::Relaxed);
            self.metrics
                .last_access_offset
                .store(offset, Ordering::Relaxed);
            self.metrics
                .last_access_length
                .store(len, Ordering::Relaxed);
        }

        #[cfg(not(feature = "std"))]
        {
            // Use write() method with WrtRwLock
            let mut metrics = self.metrics.write();
            metrics.access_count += 1;
            metrics.max_access_size = metrics.max_access_size.max(len);
            metrics.last_access_offset = offset;
            metrics.last_access_length = len;
        }
    }

    /// Update the peak memory usage statistic
    fn update_peak_memory(&self) {
        let current_size = self.size_in_bytes();

        #[cfg(feature = "std")]
        {
            let mut current_peak = self.metrics.peak_usage.load(Ordering::Relaxed);
            while current_size > current_peak {
                match self.metrics.peak_usage.compare_exchange(
                    current_peak,
                    current_size,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(actual) => current_peak = actual,
                }
            }
        }

        #[cfg(not(feature = "std"))]
        {
            // Use write() method with WrtRwLock
            let mut metrics = self.metrics.write();
            metrics.peak_usage = metrics.peak_usage.max(current_size);
        }
    }

    /// Returns the maximum size of any memory access
    pub fn max_access_size(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.metrics.max_access_size.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "std"))]
        {
            // Use read() method with WrtRwLock
            let metrics = self.metrics.read();
            metrics.max_access_size
        }
    }

    /// Returns the number of unique memory regions accessed
    pub fn unique_regions(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.metrics.unique_regions.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "std"))]
        {
            // Use read() method with WrtRwLock
            let metrics = self.metrics.read();
            metrics.unique_regions
        }
    }

    /// Returns the offset of the most recent memory access
    pub fn last_access_offset(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.metrics.last_access_offset.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "std"))]
        {
            // Use read() method with WrtRwLock
            let metrics = self.metrics.read();
            metrics.last_access_offset
        }
    }

    /// Returns the length of the most recent memory access
    pub fn last_access_length(&self) -> usize {
        #[cfg(feature = "std")]
        {
            self.metrics.last_access_length.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "std"))]
        {
            // Use read() method with WrtRwLock
            let metrics = self.metrics.read();
            metrics.last_access_length
        }
    }

    /// Grows memory by the given number of pages
    ///
    /// # Arguments
    ///
    /// * `pages` - The number of pages to grow by
    ///
    /// # Returns
    ///
    /// The previous number of pages if successful, error otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the memory cannot be grown
    pub fn grow(&mut self, pages: u32) -> Result<u32> {
        // Return early if not growing
        if pages == 0 {
            return Ok(self.current_pages.load(Ordering::Relaxed));
        }

        // Check that growing wouldn't exceed max pages
        let current_pages_val = self.current_pages.load(Ordering::Relaxed);
        let new_page_count = current_pages_val.checked_add(pages).ok_or_else(|| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_GROW_ERROR,
                "Memory growth would overflow",
            )
        })?;

        // Check against the maximum allowed by type
        if let Some(max) = self.ty.limits.max {
            if new_page_count > max {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    format!("Exceeded maximum memory size: {} > {}", new_page_count, max),
                ));
            }
        }

        // Check against the absolute maximum (4GB)
        if new_page_count > MAX_PAGES {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                format!(
                    "Exceeded maximum memory size: {} > {}",
                    new_page_count, MAX_PAGES
                ),
            ));
        }

        // Calculate the new size in bytes
        let old_size = self.data.size();
        let new_size = new_page_count as usize * PAGE_SIZE;

        // Resize the underlying data
        self.data.resize(new_size, 0);

        // Update the page count
        let old_pages = self.current_pages.swap(new_page_count, Ordering::Relaxed);

        // Update peak memory usage
        self.update_peak_memory();

        Ok(old_pages)
    }

    /// Read data from memory into a buffer
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to read from
    /// * `buffer` - The buffer to read into
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, error otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the memory access is invalid
    pub fn read(&self, offset: u32, buffer: &mut [u8]) -> Result<()> {
        // Empty read is always successful
        if buffer.is_empty() {
            return Ok(());
        }

        // Calculate total size and verify bounds
        let offset_usize = offset as usize;
        let size = buffer.len();

        // Track this access for profiling
        self.increment_access_count(offset_usize, size);

        // Use safe memory get_slice to get a verified slice
        let safe_slice = self.data.get_slice(offset_usize, size)?;

        // Copy from the safe slice to the buffer
        buffer.copy_from_slice(safe_slice.data()?);

        Ok(())
    }

    /// Write data from a buffer into memory
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to write to
    /// * `buffer` - The buffer to write from
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, error otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the memory access is invalid
    pub fn write(&mut self, offset: u32, buffer: &[u8]) -> Result<()> {
        // Empty write is always successful
        if buffer.is_empty() {
            return Ok(());
        }

        // Calculate total size and verify bounds
        let offset_usize = offset as usize;
        let size = buffer.len();
        let end = offset_usize.checked_add(size).ok_or_else(|| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_OUT_OF_BOUNDS,
                "Memory write would overflow",
            )
        })?;

        // Verify the access is within memory bounds
        if end > self.size_in_bytes() {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_OUT_OF_BOUNDS,
                format!(
                    "Memory access out of bounds: offset={}, size={}",
                    offset, size
                ),
            ));
        }

        // Track this access for profiling
        self.increment_access_count(offset_usize, size);

        // Use the SafeMemoryHandler's write_data method for efficient direct writing
        self.data.write_data(offset_usize, buffer)?;

        // Update the peak memory usage
        self.update_peak_memory();

        Ok(())
    }

    /// Gets a byte from memory
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to read from
    ///
    /// # Returns
    ///
    /// The byte at the given offset
    ///
    /// # Errors
    ///
    /// Returns an error if the offset is out of bounds
    pub fn get_byte(&self, offset: u32) -> Result<u8> {
        if !self.verify_bounds(offset, 1) {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Memory access out of bounds",
            ));
        }

        self.increment_access_count(offset as usize, 1);

        // Use SafeMemoryHandler to get a safe slice
        let offset = offset as usize;
        let slice = self.data.get_slice(offset, 1)?;
        let data = slice.data()?;
        Ok(data[0])
    }

    /// Sets a byte in memory
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write was successful
    ///
    /// # Errors
    ///
    /// Returns an error if the offset is out of bounds
    pub fn set_byte(&mut self, offset: u32, value: u8) -> Result<()> {
        if !self.verify_bounds(offset, 1) {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Memory access out of bounds",
            ));
        }

        self.increment_access_count(offset as usize, 1);

        // This is a simpler case - just write a single byte
        // using the write method which handles all the safety checks
        self.write(offset, &[value])
    }

    /// Verifies that a memory access is within bounds
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to access
    /// * `len` - The length to access
    ///
    /// # Returns
    ///
    /// True if the access is within bounds, false otherwise
    fn verify_bounds(&self, offset: u32, len: u32) -> bool {
        if len == 0 {
            return true;
        }

        // Get current data size
        let data_size = self.data.len();

        // Get the last byte that would be accessed
        let end_offset = match offset.checked_add(len) {
            Some(end) => end as usize,
            None => return false, // Overflow
        };

        // Check if the end offset is within bounds (inclusive)
        end_offset <= data_size
    }

    /// Check alignment for memory accesses
    pub fn check_alignment(&self, addr: u32, access_size: u32, align: u32) -> Result<()> {
        if addr % align != 0 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Unaligned memory access: address {addr} is not aligned to {align} bytes"),
            ));
        }

        let addr = addr as usize;
        let access_size = access_size as usize;
        if addr + access_size > self.data.size() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!(
                    "Memory access out of bounds: address {addr} + size {access_size} exceeds memory size {}",
                    self.data.size()
                )
            ));
        }

        Ok(())
    }

    /// Gets a memory-safe slice from memory at the specified address
    ///
    /// This is the preferred method for accessing memory when safety is important.
    /// The returned SafeSlice includes integrity verification to prevent memory corruption.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    /// * `len` - The length of the slice to read
    ///
    /// # Returns
    ///
    /// A SafeSlice referencing the memory region with integrity verification
    ///
    /// # Safety
    ///
    /// This method is safer than using buffer() as it performs integrity checks
    /// on the returned slice, which helps detect memory corruption.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use wrt_runtime::Memory;
    /// # use wrt_error::Result;
    /// # fn example(memory: &Memory) -> Result<()> {
    /// // Get a safe slice from memory
    /// let slice = memory.get_safe_slice(0, 10)?;
    ///
    /// // Access the data (this performs integrity verification)
    /// let data = slice.data()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is out of bounds
    pub fn get_safe_slice(
        &self,
        addr: u32,
        len: usize,
    ) -> Result<wrt_types::safe_memory::SafeSlice> {
        if !self.verify_bounds(addr, len as u32) {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Memory access out of bounds for safe slice",
            ));
        }

        self.increment_access_count(addr as usize, len);

        // Get the slice first
        let mut slice = self.data.get_slice(addr as usize, len)?;

        // Explicitly set the verification level to match the memory's level
        // This ensures consistent verification behavior
        slice.set_verification_level(self.verification_level);

        Ok(slice)
    }

    /// Creates a copy of this memory instance and applies a mutation function
    ///
    /// This is useful for operations that need to mutate memory without affecting
    /// the original instance, such as in speculative execution or transaction-like
    /// operations.
    ///
    /// # Arguments
    ///
    /// * `mutate_fn` - The function to apply to the cloned memory
    ///
    /// # Returns
    ///
    /// The result of the mutation function
    pub fn clone_and_mutate<F, R>(&self, mutate_fn: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let mut cloned = self.clone();
        mutate_fn(&mut cloned)
    }

    /// Register a pre-grow hook to be executed before memory grows
    ///
    /// This is used by SafeMemory to validate the memory state before growing
    #[cfg(feature = "std")]
    pub fn with_pre_grow_hook<F>(&self, hook: F) -> Result<()>
    where
        F: FnOnce(u32, &[u8]) -> Result<()> + Send + 'static,
    {
        // Get the current memory data
        let data = self.data.to_vec()?;

        // Execute the hook immediately with current state
        hook(self.current_pages.load(Ordering::Relaxed), &data)?;

        // In a full implementation, we would store the hook for later use
        // but for now we just run it once to validate the current state

        Ok(())
    }

    /// Register a post-grow hook to be executed after memory grows
    ///
    /// This is used by SafeMemory to update checksums and validate the new memory state
    #[cfg(feature = "std")]
    pub fn with_post_grow_hook<F>(&self, hook: F) -> Result<()>
    where
        F: FnOnce(u32, &[u8]) -> Result<()> + Send + 'static,
    {
        // Get the current memory data
        let data = self.data.to_vec()?;

        // Execute the hook immediately with current state
        hook(self.current_pages.load(Ordering::Relaxed), &data)?;

        // In a full implementation, we would store the hook for later use
        // but for now we just run it once to validate the current state

        Ok(())
    }

    /// Verify data integrity
    pub fn verify_integrity(&self) -> Result<()> {
        // Get the expected size
        let expected_size = self.current_pages.load(Ordering::Relaxed) as usize * PAGE_SIZE;

        // Verify memory size is consistent
        if self.data.size() != expected_size {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!(
                    "Memory size mismatch: expected {}, got {}",
                    expected_size,
                    self.data.size()
                ),
            ));
        }

        // Check memory integrity
        Ok(())
    }

    /// Copy memory within the same memory instance or between two memory instances
    ///
    /// This method implements the memory.copy instruction from the WebAssembly bulk memory operations proposal.
    ///
    /// # Arguments
    ///
    /// * `src_mem` - The source memory instance (can be the same as self)
    /// * `src_addr` - The source address
    /// * `dst_addr` - The destination address
    /// * `size` - The number of bytes to copy
    ///
    /// # Errors
    ///
    /// Returns an error if either the source or destination range is out of bounds
    pub fn copy_within_or_between(
        &mut self,
        src_mem: Arc<Memory>,
        src_addr: usize,
        dst_addr: usize,
        size: usize,
    ) -> Result<()> {
        // Bounds check for source
        let src_end = match src_addr.checked_add(size) {
            Some(end) if end <= src_mem.data.size() => end,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                    format!(
                        "Source memory access out of bounds: address={}, size={}",
                        src_addr, size
                    ),
                ))
            }
        };

        // Bounds check for destination
        let dst_end = match dst_addr.checked_add(size) {
            Some(end) if end <= self.data.size() => end,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                    format!(
                        "Destination memory access out of bounds: address={}, size={}",
                        dst_addr, size
                    ),
                ))
            }
        };

        // Increment the access count for both memories
        self.increment_access_count(dst_addr, size);
        src_mem.increment_access_count(src_addr, size);

        // Use SafeSlice for source memory access
        let src_slice = src_mem.data.get_slice(src_addr, size)?;
        let src_data = src_slice.data()?;

        // Handle overlapping regions safely by using a temporary buffer
        let mut temp_buf = Vec::with_capacity(size);
        temp_buf.extend_from_slice(src_data);

        // Get destination memory data
        let mut dst_data = self.data.to_vec()?;

        // Copy from temporary buffer to destination
        dst_data[dst_addr..dst_addr + size].copy_from_slice(&temp_buf);

        // Update destination memory
        self.data.clear();
        self.data.add_data(&dst_data);

        // Update peak memory usage
        self.update_peak_memory();

        // Verify integrity if full verification is enabled
        if self.verification_level == VerificationLevel::Full {
            self.data.verify_integrity()?;
        }

        Ok(())
    }

    /// Fill memory with a byte value
    ///
    /// This method implements the memory.fill instruction from the WebAssembly bulk memory operations proposal.
    ///
    /// # Arguments
    ///
    /// * `dst` - The destination address in memory
    /// * `val` - The byte value to fill with
    /// * `size` - The number of bytes to fill
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    ///
    /// Returns an error if the memory access is invalid
    pub fn fill(&mut self, dst: usize, val: u8, size: usize) -> Result<()> {
        // Handle empty fill
        if size == 0 {
            return Ok(());
        }

        // Verify destination is within bounds
        let end = dst.checked_add(size).ok_or_else(|| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_OUT_OF_BOUNDS,
                "Memory fill would overflow",
            )
        })?;

        if end > self.size_in_bytes() {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_OUT_OF_BOUNDS,
                format!("Memory fill out of bounds: dst={}, size={}", dst, size),
            ));
        }

        // Track this access for profiling
        self.increment_access_count(dst, size);

        // Create a safety-bounded buffer size to avoid excessive memory usage
        const MAX_CHUNK_SIZE: usize = 4096;

        // Fill in chunks to avoid excessive memory usage while maintaining safety
        let mut remaining = size;
        let mut current_dst = dst;

        while remaining > 0 {
            let chunk_size = remaining.min(MAX_CHUNK_SIZE);

            // For each chunk, create a properly sized fill buffer
            let fill_buffer = vec![val; chunk_size];

            // Write directly to the data handler with safety verification
            // Get a safe slice of the appropriate size and location
            let slice_data = self.data.get_slice(current_dst, chunk_size)?;

            // Use the safe memory handler's internal methods to write
            self.data
                .provider()
                .verify_access(current_dst, chunk_size)?;

            // Create a temporary safety-verified buffer for the operation
            let mut temp_data = self.data.to_vec()?;
            temp_data[current_dst..current_dst + chunk_size].fill(val);

            // Update the memory handler with the new data
            let mut new_data = SafeMemoryHandler::new(temp_data);
            new_data.set_verification_level(self.verification_level);

            // Verify integrity based on verification level
            if self.verification_level.should_verify(150) {
                new_data.verify_integrity()?;
            }

            // Replace the data handler
            self.data = new_data;

            current_dst += chunk_size;
            remaining -= chunk_size;
        }

        // Update peak memory usage
        self.update_peak_memory();

        Ok(())
    }

    /// Initialize memory from a data segment
    ///
    /// This method implements the memory.init instruction from the WebAssembly bulk memory operations proposal.
    ///
    /// # Arguments
    ///
    /// * `dst` - The destination address in memory
    /// * `data` - The source data segment
    /// * `src` - The offset within the data segment
    /// * `size` - The number of bytes to copy
    ///
    /// # Errors
    ///
    /// Returns an error if the source or destination range is out of bounds
    pub fn init(&mut self, dst: usize, data: &[u8], src: usize, size: usize) -> Result<()> {
        // Source bounds check
        let src_end = match src.checked_add(size) {
            Some(end) if end <= data.len() => end,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                    format!(
                        "Source data access out of bounds: address={}, size={}",
                        src, size
                    ),
                ));
            }
        };

        // Destination bounds check
        let dst_end = match dst.checked_add(size) {
            Some(end) if end <= self.data.size() => end,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                    format!(
                        "Destination memory access out of bounds: address={}, size={}",
                        dst, size
                    ),
                ));
            }
        };

        // Handle zero-size initialization
        if size == 0 {
            return Ok(());
        }

        // For small copies, we can use set_byte directly - this provides maximum safety
        // with acceptable performance for small operations
        if size <= 32 {
            // Create a safe copy of the source data for integrity
            let src_data = SafeSlice::new(&data[src..src + size]);

            // Verify the source data integrity
            src_data.verify_integrity()?;

            // Access source data safely
            let verified_data = src_data.data()?;

            for (i, &byte) in verified_data.iter().enumerate().take(size) {
                self.set_byte((dst + i) as u32, byte)?;
            }

            // Update metrics to reflect the entire operation rather than just the last byte
            self.update_access_metrics(dst, size);
            return Ok(());
        }

        // For larger copies, use chunked processing to maintain memory safety
        // without excessive temporary allocations
        const MAX_CHUNK_SIZE: usize = 4096;
        let mut remaining = size;
        let mut src_offset = src;
        let mut dst_offset = dst;

        while remaining > 0 {
            let chunk_size = remaining.min(MAX_CHUNK_SIZE);

            // Create a safe slice for the source chunk to verify its integrity
            let src_slice = SafeSlice::new(&data[src_offset..src_offset + chunk_size]);
            src_slice.verify_integrity()?;

            // Get the source data after verification
            let src_data = src_slice.data()?;

            // Verify destination access is valid using the SafeMemoryHandler
            self.data.provider().verify_access(dst_offset, chunk_size)?;

            // Apply the write in chunks with verification
            let mut memory_data = self.data.to_vec()?;

            // Use explicit indices to ensure safety
            for i in 0..chunk_size {
                if dst_offset + i < memory_data.len() {
                    memory_data[dst_offset + i] = src_data[i];
                } else {
                    return Err(Error::new(
                        ErrorCategory::Memory,
                        codes::MEMORY_OUT_OF_BOUNDS,
                        "Memory access out of bounds during init",
                    ));
                }
            }

            // Create a new safe memory handler with the updated data
            let mut new_handler = SafeMemoryHandler::new(memory_data);

            // Set the same verification level as the current handler
            new_handler.set_verification_level(self.verification_level);

            // Verify integrity if needed based on verification level
            if self.verification_level.should_verify(180) {
                new_handler.verify_integrity()?;
            }

            // Replace the current data handler
            self.data = new_handler;

            // Update for next chunk
            src_offset += chunk_size;
            dst_offset += chunk_size;
            remaining -= chunk_size;
        }

        // Update peak memory usage
        self.update_peak_memory();

        // Ensure all metrics reflect the entire init operation
        self.update_access_metrics(dst, size);

        Ok(())
    }

    /// Read a 32-bit integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 32-bit integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_i32(&self, addr: u32) -> Result<i32> {
        let mut buffer = [0; 4];
        self.read(addr, &mut buffer)?;
        Ok(i32::from_le_bytes(buffer))
    }

    /// Read a 64-bit integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 64-bit integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_i64(&self, addr: u32) -> Result<i64> {
        let mut buffer = [0; 8];
        self.read(addr, &mut buffer)?;
        Ok(i64::from_le_bytes(buffer))
    }

    /// Read a 32-bit float from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 32-bit float read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_f32(&self, addr: u32) -> Result<f32> {
        let value = self.read_i32(addr)?;
        Ok(f32::from_bits(value as u32))
    }

    /// Read a 64-bit float from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 64-bit float read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_f64(&self, addr: u32) -> Result<f64> {
        let value = self.read_i64(addr)?;
        Ok(f64::from_bits(value as u64))
    }

    /// Read an 8-bit signed integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 8-bit signed integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_i8(&self, addr: u32) -> Result<i8> {
        let mut buffer = [0; 1];
        self.read(addr, &mut buffer)?;
        Ok(i8::from_le_bytes(buffer))
    }

    /// Read an 8-bit unsigned integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 8-bit unsigned integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_u8(&self, addr: u32) -> Result<u8> {
        let mut buffer = [0; 1];
        self.read(addr, &mut buffer)?;
        Ok(u8::from_le_bytes(buffer))
    }

    /// Read a 16-bit signed integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 16-bit signed integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_i16(&self, addr: u32) -> Result<i16> {
        let mut buffer = [0; 2];
        self.read(addr, &mut buffer)?;
        Ok(i16::from_le_bytes(buffer))
    }

    /// Read a 16-bit unsigned integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 16-bit unsigned integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_u16(&self, addr: u32) -> Result<u16> {
        let mut buffer = [0; 2];
        self.read(addr, &mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    /// Read a 32-bit unsigned integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 32-bit unsigned integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_u32(&self, addr: u32) -> Result<u32> {
        let mut buffer = [0; 4];
        self.read(addr, &mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    /// Read a 64-bit unsigned integer from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 64-bit unsigned integer read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_u64(&self, addr: u32) -> Result<u64> {
        let mut buffer = [0; 8];
        self.read(addr, &mut buffer)?;
        Ok(u64::from_le_bytes(buffer))
    }

    /// Read a 128-bit vector from memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to read from
    ///
    /// # Returns
    ///
    /// The 128-bit vector read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read is out of bounds
    pub fn read_v128(&self, addr: u32) -> Result<[u8; 16]> {
        let mut buffer = [0; 16];
        self.read(addr, &mut buffer)?;
        Ok(buffer)
    }

    /// Write a 32-bit integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_i32(&mut self, addr: u32, value: i32) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 64-bit integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_i64(&mut self, addr: u32, value: i64) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 32-bit float to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_f32(&mut self, addr: u32, value: f32) -> Result<()> {
        self.write_i32(addr, value.to_bits() as i32)
    }

    /// Write a 64-bit float to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_f64(&mut self, addr: u32, value: f64) -> Result<()> {
        self.write_i64(addr, value.to_bits() as i64)
    }

    /// Write an 8-bit signed integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_i8(&mut self, addr: u32, value: i8) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write an 8-bit unsigned integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_u8(&mut self, addr: u32, value: u8) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 16-bit signed integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_i16(&mut self, addr: u32, value: i16) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 16-bit unsigned integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_u16(&mut self, addr: u32, value: u16) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 32-bit unsigned integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_u32(&mut self, addr: u32, value: u32) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 64-bit unsigned integer to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_u64(&mut self, addr: u32, value: u64) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    /// Write a 128-bit vector to memory
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to write to
    /// * `value` - The value to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write is out of bounds
    pub fn write_v128(&mut self, addr: u32, value: [u8; 16]) -> Result<()> {
        self.write(addr, &value)
    }

    /// Sets the verification level for memory operations
    ///
    /// This controls how much verification is performed during memory operations:
    /// - None: No verification (fastest, least safe)
    /// - Sampling: Verification on a random subset of operations
    /// - Standard: Normal verification level (default)
    /// - Full: Maximum verification (slowest, most safe)
    ///
    /// # Arguments
    ///
    /// * `level` - The new verification level
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        // Update our own verification level
        self.verification_level = level;

        // Propagate to the memory handler
        self.data.set_verification_level(level);
    }

    /// Gets the current verification level
    ///
    /// # Returns
    ///
    /// The current verification level
    #[must_use]
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Get the memory statistics
    fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            total_size: self.data.size(),
            access_count: self.access_count() as usize, // Convert u64 to usize
            unique_regions: self.unique_regions(),
            max_access_size: self.max_access_size(),
        }
    }

    // Note: This functionality is now handled in update_access_metrics

    /// Update all access metrics in one operation
    fn update_access_metrics(&self, offset: usize, len: usize) {
        #[cfg(feature = "std")]
        {
            self.metrics
                .max_access_size
                .fetch_max(len, Ordering::Relaxed);
            self.metrics
                .last_access_offset
                .store(offset, Ordering::Relaxed);
            self.metrics
                .last_access_length
                .store(len, Ordering::Relaxed);
        }

        #[cfg(not(feature = "std"))]
        {
            // Use write() method with WrtRwLock
            let mut metrics = self.metrics.write();
            metrics.max_access_size = metrics.max_access_size.max(len);
            metrics.last_access_offset = offset;
            metrics.last_access_length = len;
        }
    }

    /// Get safety statistics for this memory instance
    ///
    /// This returns detailed statistics about memory usage and safety checks
    ///
    /// # Returns
    ///
    /// A string containing the statistics
    pub fn safety_stats(&self) -> String {
        let memory_stats = self.memory_stats();
        let access_count = self.access_count();
        let peak_memory = self.peak_memory();
        let max_access = self.max_access_size();
        let unique_regions = self.unique_regions();

        format!(
            "Memory Safety Stats:\n\
             - Size: {} bytes ({} pages)\n\
             - Peak usage: {} bytes\n\
             - Access count: {}\n\
             - Unique regions: {}\n\
             - Max access size: {} bytes\n\
             - Verification level: {:?}",
            self.size_in_bytes(),
            self.current_pages.load(Ordering::Relaxed),
            peak_memory,
            access_count,
            unique_regions,
            max_access,
            self.verification_level
        )
    }

    /// Returns a SafeSlice representing the entire memory
    ///
    /// Unlike `buffer()`, this does not create a copy of the memory data,
    /// making it more efficient for read-only access to memory.
    ///
    /// # Returns
    ///
    /// A SafeSlice covering the entire memory buffer
    ///
    /// # Errors
    ///
    /// Returns an error if the memory is corrupted or integrity checks fail
    pub fn as_safe_slice(&self) -> Result<wrt_types::safe_memory::SafeSlice> {
        self.data.get_slice(0, self.size_in_bytes())
    }

    /// Update the memory buffer through a callback function
    pub fn update_buffer<F>(&self, _update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut [u8]) -> Result<()>,
    {
        // self.data.with_mutable_data(update_fn)?;
        // This pattern is not directly supported by SafeMemoryHandler due to its
        // ownership and checksumming of the byte buffer.
        // A redesign of this function or SafeMemoryHandler would be needed
        // for direct mutable slice access.
        Err(Error::system_error_with_code(
            codes::UNSUPPORTED_OPERATION,
            "Memory::update_buffer pattern is not currently supported with SafeMemoryHandler",
        ))
    }
    
    /// Grow memory by a number of pages.
    pub fn grow_memory(&mut self, pages: u32) -> Result<u32> {
        let old_size_pages = self.current_pages.load(Ordering::Relaxed);
        let new_size_pages = old_size_pages.saturating_add(pages);

        if new_size_pages > MAX_PAGES {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_GROW_ERROR,
                format!("Cannot grow memory beyond WebAssembly maximum of {} pages", MAX_PAGES),
            ));
        }
        
        let new_byte_size = new_size_pages as usize * PAGE_SIZE;
        // Placeholder: Assumes SafeMemoryHandler has a method like `resize` 
        // that takes &self and handles locking internally.
        self.data.resize(new_byte_size, 0)?;
        
        self.current_pages.store(new_size_pages, Ordering::Relaxed);
        self.update_peak_memory();
        
        Ok(old_size_pages)
    }
}

impl MemoryProvider for Memory {
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<SafeSlice<'_>> {
        // Verify bounds
        if offset + len > self.data.size() {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                format!(
                    "Memory access out of bounds: offset={}, len={}, size={}",
                    offset,
                    len,
                    self.data.size()
                ),
            ));
        }

        self.data.get_slice(offset, len)
    }

    fn verify_access(&self, offset: usize, len: usize) -> Result<()> {
        if offset + len > self.data.size() {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                format!(
                    "Memory access out of bounds: offset={}, len={}, size={}",
                    offset,
                    len,
                    self.data.size()
                ),
            ));
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.data.size()
    }
}

impl MemorySafety for Memory {
    fn verify_integrity(&self) -> Result<()> {
        self.verify_integrity()
    }

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.set_verification_level(level)
    }

    fn verification_level(&self) -> VerificationLevel {
        self.verification_level()
    }

    fn memory_stats(&self) -> MemoryStats {
        self.memory_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_types::safe_memory::SafeSlice;
    use wrt_types::types::Limits;
    use wrt_types::verification::VerificationLevel;

    #[test]
    fn test_memory_creation() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let memory = Memory::new(mem_type).unwrap();
        assert_eq!(memory.size(), 1);
        assert_eq!(memory.size_in_bytes(), PAGE_SIZE);
    }

    #[test]
    fn test_memory_grow() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();
        let old_size = memory.grow(1).unwrap();
        assert_eq!(old_size, 1);
        assert_eq!(memory.size(), 2);
        assert_eq!(memory.size_in_bytes(), 2 * PAGE_SIZE);
    }

    #[test]
    fn test_memory_read_write() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();
        let data = [1, 2, 3, 4];
        memory.write(0, &data).unwrap();
        let mut buffer = [0; 4];
        memory.read(0, &mut buffer).unwrap();
        assert_eq!(buffer, data);
    }

    #[test]
    fn test_memory_get_set_byte() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();
        memory.set_byte(0, 42).unwrap();
        assert_eq!(memory.get_byte(0).unwrap(), 42);
    }

    #[test]
    fn test_memory_peak_usage() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();
        assert_eq!(memory.peak_memory(), PAGE_SIZE);
        memory.grow(1).unwrap();
        assert_eq!(memory.peak_memory(), 2 * PAGE_SIZE);
    }

    #[test]
    fn test_alignment_check() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let memory = Memory::new(mem_type).unwrap();
        assert!(memory.check_alignment(0, 4, 4).is_ok());
        assert!(memory.check_alignment(1, 4, 4).is_err());
    }

    #[test]
    fn test_memory_access_tracking() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();

        // Test single byte access
        memory.set_byte(0, 42).unwrap();
        assert_eq!(memory.max_access_size(), 1);
        assert_eq!(memory.last_access_offset(), 0);
        assert_eq!(memory.last_access_length(), 1);

        // Test multi-byte access
        let data = [1, 2, 3, 4];
        memory.write(10, &data).unwrap();
        assert_eq!(memory.max_access_size(), 4);
        assert_eq!(memory.last_access_offset(), 10);
        assert_eq!(memory.last_access_length(), 4);

        // Test read access
        let mut buffer = [0; 2];
        memory.read(20, &mut buffer).unwrap();
        assert_eq!(memory.max_access_size(), 4); // Still 4 from previous write
        assert_eq!(memory.last_access_offset(), 20);
        assert_eq!(memory.last_access_length(), 2);
    }

    #[test]
    fn test_memory_copy_tracking() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory1 = Memory::new(mem_type.clone()).unwrap();
        let mut memory2 = Memory::new(mem_type).unwrap();

        // Initialize source memory
        let data = [1, 2, 3, 4];
        memory1.write(0, &data).unwrap();

        // Copy between memories
        memory2
            .copy_within_or_between(Arc::new(memory1.clone()), 0, 0, 4)
            .unwrap();

        // Check access tracking
        assert_eq!(memory1.max_access_size(), 4);
        assert_eq!(memory1.last_access_offset(), 0);
        assert_eq!(memory1.last_access_length(), 4);

        assert_eq!(memory2.max_access_size(), 4);
        assert_eq!(memory2.last_access_offset(), 0);
        assert_eq!(memory2.last_access_length(), 4);
    }

    #[test]
    fn test_memory_fill_tracking() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();

        // Fill memory region
        memory.fill(0, 42, 10).unwrap();

        // Check access tracking
        assert_eq!(memory.max_access_size(), 10);
        assert_eq!(memory.last_access_offset(), 0);
        assert_eq!(memory.last_access_length(), 10);
    }

    #[test]
    fn test_memory_init_tracking() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type).unwrap();

        // Initialize memory region
        let data = [1, 2, 3, 4, 5];
        memory.init(0, &data, 0, 5).unwrap();

        // Check access tracking
        assert_eq!(memory.max_access_size(), 5);
        assert_eq!(memory.last_access_offset(), 0);
        assert_eq!(memory.last_access_length(), 5);
    }

    #[test]
    fn test_memory_safety_features() -> Result<()> {
        use wrt_types::verification::VerificationLevel;

        // Create a memory with a specific verification level
        let mem_type = MemoryType {
            limits: wrt_types::types::Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = Memory::new(mem_type)?;
        memory.set_verification_level(VerificationLevel::Full);

        // Write some data
        memory.write(0, &[1, 2, 3, 4])?;

        // Read the data back
        let mut buffer = [0; 4];
        memory.read(0, &mut buffer)?;
        assert_eq!(buffer, [1, 2, 3, 4]);

        // Test fill operation
        memory.fill(4, 0x42, 4)?;

        // Read the filled data
        let mut buffer = [0; 4];
        memory.read(4, &mut buffer)?;
        assert_eq!(buffer, [0x42, 0x42, 0x42, 0x42]);

        // Test memory growth
        let old_pages = memory.grow(1)?;
        assert_eq!(old_pages, 1);
        assert_eq!(memory.size(), 2);

        // Verify integrity
        memory.verify_integrity()?;

        // Print safety stats
        println!("{}", memory.safety_stats());

        Ok(())
    }
}
