// WRT - wrt-foundation
// Module: Linear Memory using Platform Abstraction Layer
// SW-REQ-ID: REQ_MEMORY_001 REQ_PLATFORM_001
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![allow(unsafe_code)] // This module needs unsafe for memory operations

//! Provides a Wasm linear memory implementation backed by a `PageAllocator`
//! from the `wrt-platform` crate.

// Test comment to try and force recompile / cache clear
use core::{
    fmt::Debug,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use wrt_platform::memory::{PageAllocator, WASM_PAGE_SIZE};

use crate::{
    prelude::*,
    safe_memory::{Allocator, Provider, SafeMemoryHandler, Slice, SliceMut, Stats},
    verification::VerificationLevel,
    WrtResult,
};

/// Adapter to convert `PageAllocator` to `Allocator` interface
#[derive(Debug, Clone)]
pub struct PageAllocatorAdapter<A> {
    /// Binary std/no_std choice
    allocator: A,
}

impl<A: PageAllocator + Send + Sync> PageAllocatorAdapter<A> {
    /// Binary std/no_std choice
    pub fn new(allocator: A) -> Self {
        Self { allocator }
    }
}

impl<A: PageAllocator + Send + Sync + Clone + 'static> Allocator for PageAllocatorAdapter<A> {
    fn allocate(&self, layout: core::alloc::Layout) -> WrtResult<*mut u8> {
        // Convert the layout to pages (rounded up)
        let size_pages = (layout.size() + WASM_PAGE_SIZE - 1) / WASM_PAGE_SIZE;
        let mut allocator = self.allocator.clone();
        let (ptr, _size) = allocator.allocate(size_pages as u32, None).map_err(|_| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_OUT_OF_BOUNDS,
                "Failed to allocate memory",
            )
        })?;
        Ok(ptr.as_ptr())
    }

    fn deallocate(&self, ptr: *mut u8, _layout: core::alloc::Layout) -> WrtResult<()> {
        // Binary std/no_std choice
        // as they typically manage entire memory regions
        Ok(())
    }
}

/// A WebAssembly linear memory implementation using a `PageAllocator`.
///
/// Binary std/no_std choice
/// a platform-specific `PageAllocator`.
#[derive(Debug)]
pub struct PalMemoryProvider<A: PageAllocator + Send + Sync + Clone + 'static> {
    allocator: A,
    adapter: PageAllocatorAdapter<A>,
    base_ptr: Option<NonNull<u8>>,
    current_pages: u32,
    maximum_pages: Option<u32>,
    initial_allocation_size: usize, // Binary std/no_std choice
    verification_level: VerificationLevel,
    // Binary std/no_std choice
    access_count: AtomicUsize,
    max_access_size: AtomicUsize,
}

// SAFETY: The PalMemoryProvider is Send if the PageAllocator A is Send.
// The NonNull<u8> itself is not Send/Sync, but we are managing its lifecycle
// Binary std/no_std choice
// methods are used externally (e.g., if &mut self methods are correctly
// serialized). The raw pointer is only ever accessed through methods that take
// &self or &mut self, and the underlying memory operations via the
// PageAllocator are assumed to be safe or synchronized if A is Send + Sync.
unsafe impl<A: PageAllocator + Send + Sync + Clone + 'static> Send for PalMemoryProvider<A> {}

// SAFETY: Similar to Send, PalMemoryProvider is Sync if A is Sync.
// Accesses to shared state like AtomicUsize are atomic.
// Accesses to the memory region via &self methods (like borrow_slice) provide
// immutable slices, which is safe. Mutable access is through &mut self.
unsafe impl<A: PageAllocator + Send + Sync + Clone + 'static> Sync for PalMemoryProvider<A> {}

impl<A: PageAllocator + Send + Sync + Clone + 'static> Clone for PalMemoryProvider<A> {
    fn clone(&self) -> Self {
        Self {
            allocator: self.allocator.clone(),
            adapter: self.adapter.clone(),
            base_ptr: self.base_ptr,
            current_pages: self.current_pages,
            maximum_pages: self.maximum_pages,
            initial_allocation_size: self.initial_allocation_size,
            verification_level: self.verification_level,
            access_count: AtomicUsize::new(self.access_count.load(Ordering::Relaxed)),
            max_access_size: AtomicUsize::new(self.max_access_size.load(Ordering::Relaxed)),
        }
    }
}

impl<A: PageAllocator + Send + Sync + Clone + 'static> PalMemoryProvider<A> {
    /// Creates a new `PalMemoryProvider`.
    ///
    /// # Arguments
    ///
    /// Binary std/no_std choice
    ///   operations.
    /// Binary std/no_std choice
    /// * `maximum_pages`: An optional maximum number of Wasm pages the memory
    ///   can grow to.
    /// * `verification_level`: The verification level for memory operations.
    ///
    /// # Errors
    ///
    /// Binary std/no_std choice
    pub fn new(
        mut allocator: A,
        initial_pages: u32,
        maximum_pages: Option<u32>,
        verification_level: VerificationLevel,
    ) -> Result<Self> {
        if initial_pages == 0 && maximum_pages.unwrap_or(0) == 0 {
            // Allow zero initial if max is also zero, effectively an empty
            // Binary std/no_std choice
            // initial_pages = 0. For now, let's assume
            // Binary std/no_std choice
            // Wasm spec: min size is required, max is optional.
            // Binary std/no_std choice
            // spec.
        }

        let (ptr, allocated_size) = allocator.allocate(initial_pages, maximum_pages)?;

        let adapter = PageAllocatorAdapter::new(allocator.clone());

        Ok(Self {
            allocator,
            adapter,
            base_ptr: Some(ptr),
            current_pages: initial_pages,
            maximum_pages,
            initial_allocation_size: allocated_size, // Binary std/no_std choice
            verification_level,
            access_count: AtomicUsize::new(0),
            max_access_size: AtomicUsize::new(0),
        })
    }

    /// Grows the memory by `additional_pages`.
    ///
    /// Returns the previous number of pages on success, as per Wasm
    /// `memory.grow`.
    ///
    /// # Errors
    ///
    /// Binary std/no_std choice
    /// error).
    pub fn grow(&mut self, additional_pages: u32) -> Result<u32> {
        if additional_pages == 0 {
            return Ok(self.current_pages);
        }
        let Some(_base_ptr) = self.base_ptr else {
            return Err(Error::new(
                ErrorCategory::Core,
                codes::INITIALIZATION_ERROR,
                "Memory not allocated, cannot grow.",
            ));
        };

        let old_pages = self.current_pages;
        let new_total_pages = old_pages.checked_add(additional_pages).ok_or_else(|| {
            Error::new(
                ErrorCategory::Memory,
                codes::CAPACITY_EXCEEDED,
                "Page count overflow during grow attempt.",
            )
        })?;

        if let Some(max) = self.maximum_pages {
            if new_total_pages > max {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::CAPACITY_EXCEEDED,
                    "Grow attempt exceeds maximum pages.",
                ));
            }
        }

        self.allocator.grow(old_pages, additional_pages)?;
        self.current_pages = new_total_pages;
        Ok(old_pages)
    }

    fn track_access(&self, _offset: usize, len: usize) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.max_access_size.fetch_max(len, Ordering::Relaxed);
        // More sophisticated tracking (like unique regions) could be added if
        // needed.
    }

    /// Binary std/no_std choice
    pub fn pages(&self) -> u32 {
        self.current_pages
    }

    /// Returns the maximum number of WebAssembly pages this memory can grow to.
    pub fn max_pages(&self) -> Option<u32> {
        self.maximum_pages
    }
}

impl<A: PageAllocator + Send + Sync + Clone + 'static> Provider for PalMemoryProvider<A> {
    fn borrow_slice(&self, offset: usize, len: usize) -> Result<Slice<'_>> {
        self.verify_access(offset, len)?;
        let Some(base_ptr) = self.base_ptr else {
            return Err(Error::new(
                ErrorCategory::Core,
                codes::INITIALIZATION_ERROR,
                "Memory not allocated, cannot borrow slice.",
            ));
        };
        self.track_access(offset, len);
        // SAFETY: `verify_access` ensures that `offset + len` is within the
        // Binary std/no_std choice
        // WASM_PAGE_SIZE). `base_ptr` is guaranteed to be non-null and valid if
        // Binary std/no_std choice
        // Binary std/no_std choice
        // tied to `&self`, ensuring the data remains valid as long as the
        // `PalMemoryProvider` is borrowed. The underlying memory pointed to by
        // `base_ptr.as_ptr().add(offset)` is valid for reads of `len` bytes
        // because `verify_access` has passed.
        let data_slice = unsafe { core::slice::from_raw_parts(base_ptr.as_ptr().add(offset), len) };
        Slice::with_verification_level(data_slice, self.verification_level)
    }

    fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.verify_access(offset, data.len())?;
        let Some(base_ptr) = self.base_ptr else {
            return Err(Error::new(
                ErrorCategory::Core,
                codes::INITIALIZATION_ERROR,
                "Memory not allocated, cannot write data.",
            ));
        };
        self.track_access(offset, data.len());
        // SAFETY: `verify_access` ensures that `offset + data.len()` is within the
        // Binary std/no_std choice
        // `base_ptr` is guaranteed to be non-null and valid if Some.
        // The method takes `&mut self`, ensuring exclusive access to the
        // `PalMemoryProvider`, and thus to the underlying memory region for the
        // duration of this call. This prevents data races.
        // The memory at `base_ptr.as_ptr().add(offset)` is valid for writes of
        // `data.len()` bytes because `verify_access` has passed.
        let dest_slice =
            unsafe { core::slice::from_raw_parts_mut(base_ptr.as_ptr().add(offset), data.len()) };
        dest_slice.copy_from_slice(data);
        Ok(())
    }

    fn verify_access(&self, offset: usize, len: usize) -> Result<()> {
        let current_byte_size = self.current_pages as usize * WASM_PAGE_SIZE;
        let end_offset = offset.checked_add(len).ok_or_else(|| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS, // Or overflow error code
                "Access range calculation overflow.",
            )
        })?;

        if end_offset > current_byte_size {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                "Access out of bounds.",
            ));
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.current_pages as usize * WASM_PAGE_SIZE
    }

    fn capacity(&self) -> usize {
        self.maximum_pages.map_or_else(
            || self.size(), // Binary std/no_std choice
            |max_pages| max_pages as usize * WASM_PAGE_SIZE,
        )
    }

    fn verify_integrity(&self) -> Result<()> {
        // Binary std/no_std choice
        // and our view (pages, ptr) is consistent. Deeper integrity (checksums)
        // is handled by Slice/SliceMut.
        if self.base_ptr.is_none() && self.current_pages > 0 {
            return Err(Error::new(
                ErrorCategory::Core,
                codes::INTEGRITY_VIOLATION,
                "Memory pointer is None but current_pages > 0",
            ));
        }
        // Binary std/no_std choice
        // checks.
        Ok(())
    }

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }

    fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    fn memory_stats(&self) -> Stats {
        Stats {
            total_size: self.size(),
            access_count: self.access_count.load(Ordering::Relaxed),
            unique_regions: 0, // Not tracked by this basic provider yet
            max_access_size: self.max_access_size.load(Ordering::Relaxed),
        }
    }

    fn get_slice_mut(&mut self, offset: usize, len: usize) -> Result<SliceMut<'_>> {
        self.verify_access(offset, len)?;
        let Some(base_ptr) = self.base_ptr else {
            return Err(Error::new(
                ErrorCategory::Core,
                codes::INITIALIZATION_ERROR,
                "Memory not allocated, cannot get mutable slice.",
            ));
        };
        self.track_access(offset, len);
        // SAFETY: `verify_access` ensures that `offset + len` is within the
        // Binary std/no_std choice
        // non-null and valid. `&mut self` ensures exclusive access. The
        // memory region is valid for mutable access.
        let data_slice =
            unsafe { core::slice::from_raw_parts_mut(base_ptr.as_ptr().add(offset), len) };
        SliceMut::with_verification_level(data_slice, self.verification_level)
    }

    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> Result<()> {
        if len == 0 {
            return Ok(());
        }

        // Verify source and destination ranges independently first.
        self.verify_access(src_offset, len).map_err(|_| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                "Source range out of bounds for copy_within.",
            )
        })?;
        self.verify_access(dst_offset, len).map_err(|_| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_OUT_OF_BOUNDS,
                "Destination range out of bounds for copy_within.",
            )
        })?;

        let Some(base_ptr) = self.base_ptr else {
            return Err(Error::new(
                ErrorCategory::Core,
                codes::INITIALIZATION_ERROR,
                "Memory not allocated, cannot copy_within.",
            ));
        };

        // Track access before performing the copy
        // We could track two accesses (read from src, write to dst) or one logical
        // copy operation.
        self.track_access(src_offset, len); // Track the read part
        self.track_access(dst_offset, len); // Track the write part

        // SAFETY: `verify_access` for both source and destination ensures that
        // `src_offset + len` and `dst_offset + len` are within bounds.
        // `base_ptr` is non-null and valid. `&mut self` ensures exclusive access
        // suitable for `copy_within` which might have overlapping regions.
        // `as_ptr()` is safe as `base_ptr` is `NonNull`.
        // `add(offset)` is pointer arithmetic, safe here because bounds are checked.
        // `copy_to_nonoverlapping` or `copy` needs to be chosen based on overlap.
        // `core::ptr::copy` handles overlapping regions correctly.
        unsafe {
            let src_ptr = base_ptr.as_ptr().add(src_offset);
            let dst_ptr = base_ptr.as_ptr().add(dst_offset);
            core::ptr::copy(src_ptr, dst_ptr, len);
        }
        Ok(())
    }

    fn ensure_used_up_to(&mut self, byte_offset: usize) -> Result<()> {
        let current_size_bytes = self.size();
        if byte_offset > current_size_bytes {
            if byte_offset > self.capacity() {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::CAPACITY_EXCEEDED,
                    "ensure_used_up_to exceeds capacity",
                ));
            }
            // Calculate additional pages needed. Ceiling division.
            let additional_bytes_needed = byte_offset - current_size_bytes;
            let additional_pages_needed =
                (additional_bytes_needed + WASM_PAGE_SIZE - 1) / WASM_PAGE_SIZE;

            self.grow(additional_pages_needed as u32)?;
            // grow updates current_pages, size() will reflect the new size.
            // We need to ensure the new size is at least byte_offset.
            if self.size() < byte_offset {
                // This case should ideally not be hit if grow succeeded and calculations are
                // correct. It might happen if grow couldn't satisfy the full
                // request but didn't error, or if there are rounding issues.
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::INVALID_STATE,
                    "Memory growth did not reach requested byte_offset",
                ));
            }
        }
        Ok(())
    }

    type Allocator = PageAllocatorAdapter<A>;

    fn acquire_memory(&self, layout: core::alloc::Layout) -> WrtResult<*mut u8> {
        self.get_allocator().allocate(layout)
    }

    fn release_memory(&self, ptr: *mut u8, layout: core::alloc::Layout) -> WrtResult<()> {
        self.get_allocator().deallocate(ptr, layout)
    }

    fn get_allocator(&self) -> &Self::Allocator {
        &self.adapter
    }

    fn new_handler(&self) -> Result<SafeMemoryHandler<Self>>
    where
        Self: Sized + Clone,
    {
        Ok(SafeMemoryHandler::new(self.clone()))
    }
}

impl<A: PageAllocator + Send + Sync + Clone + 'static> Drop for PalMemoryProvider<A> {
    fn drop(&mut self) {
        if let Some(ptr) = self.base_ptr.take() {
            // Binary std/no_std choice
            // Binary std/no_std choice
            // Binary std/no_std choice
            let size_to_deallocate = self.initial_allocation_size;

            if size_to_deallocate > 0 {
                // Binary std/no_std choice
                // Binary std/no_std choice
                // Binary std/no_std choice
                // `PalMemoryProvider` goes out of scope, ensuring exclusive access
                // Binary std/no_std choice
                unsafe {
                    if let Err(_e) = self.allocator.deallocate(ptr, size_to_deallocate) {
                        // In a no_std environment, error reporting in drop is
                        // complex. Panicking in drop is
                        // highly discouraged.
                        // Logging might be done via a specific facade if
                        // available. For now, we
                        // Binary std/no_std choice
                        // The error `_e` could potentially be logged if a
                        // mechanism exists.
                    }
                }
            }
        }
    }
}

// Conditionally compile tests module only when std is enabled
#[cfg(all(test, feature = "std"))]
mod tests {
    // ... existing code ...
}
