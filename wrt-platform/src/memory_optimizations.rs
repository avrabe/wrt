
// WRT - wrt-platform
// Module: Memory Optimizations
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Platform-specific memory optimizations for bounded collections and resource
//! types.
//!
//! This module provides optimized implementations of memory operations for
//! different platforms, focusing on efficiency, security, and reliability in
//! no_std/no_alloc environments.

use core::marker::PhantomData;

// For Vec when std is available
#[cfg(feature = "std")]
use std::vec::Vec;

use wrt_error::Error;

use crate::memory::{MemoryProvider, NoStdProvider, VerificationLevel};
type WrtResult<T> = Result<T, Error>;

/// A checksum implementation for data verification.
///
/// This simple checksum stores a 64-bit value and is used for verifying
/// the integrity of memory operations in optimized providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checksum(u64);

impl Checksum {
    /// Creates a new checksum with an initial value of 0.
    pub fn new() -> Self {
        Self(0)
    }

    /// Resets the checksum to its initial value.
    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

impl Default for Checksum {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for platform-optimized memory operations.
pub trait PlatformMemoryOptimizer {
    /// Perform zero-copy read if the platform supports it.
    fn zero_copy_read(&self, source: &[u8], dest: &mut [u8]) -> WrtResult<usize>;

    /// Perform hardware-accelerated copy if available.
    fn accelerated_copy(&self, source: &[u8], dest: &mut [u8]) -> WrtResult<usize>;

    /// Use platform-specific alignment optimization.
    fn align_memory(&self, ptr: *mut u8, alignment: usize) -> WrtResult<*mut u8>;

    /// Perform secure memory zeroing (preventing compiler optimization
    /// removals).
    fn secure_zero(&self, buffer: &mut [u8]) -> WrtResult<()>;

    /// Check if the platform supports a specific memory optimization.
    fn supports_optimization(&self, optimization: MemoryOptimization) -> bool;
}

/// Memory optimization types available on different platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOptimization {
    /// Zero-copy operations (e.g., DMA on embedded platforms).
    ZeroCopy,
    /// Hardware-accelerated copies (e.g., SIMD instructions).
    HardwareAcceleration,
    /// Special alignment requirements available.
    AlignmentOptimization,
    /// Secure zeroing (e.g., explicit flush to memory).
    SecureZeroing,
    /// Memory protection (e.g., guard pages, no-execute regions).
    MemoryProtection,
}

/// Optimized provider for macOS platforms.
#[cfg(target_os = "macos")]
pub struct MacOSOptimizedProvider {
    /// The underlying provider
    inner: NoStdProvider,
    /// Platform-specific features detected
    features: MacOSFeatures,
}

/// macOS-specific features for memory optimization.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct MacOSFeatures {
    /// Whether AVX/AVX2 is available for vectorized operations
    has_avx: bool,
    /// Whether memory protection features can be used
    can_use_memory_protection: bool,
    /// Optimal page size on this system
    #[allow(dead_code)] // Used for future optimizations
    page_size: usize,
}

#[cfg(target_os = "macos")]
impl Default for MacOSFeatures {
    fn default() -> Self {
        Self {
            // Detect features at runtime or use conservative defaults
            has_avx: cfg!(target_feature = "avx"),
            can_use_memory_protection: true, // macOS typically supports this
            page_size: 4096,                 // Standard page size, could be detected at runtime
        }
    }
}

#[cfg(target_os = "macos")]
impl MacOSOptimizedProvider {
    /// Create a new macOS-optimized provider with the specified size and
    /// verification level.
    pub fn new(size: usize, verification_level: VerificationLevel) -> Self {
        Self {
            inner: NoStdProvider::new(size, verification_level),
            features: MacOSFeatures::default(),
        }
    }

    /// Create a new macOS-optimized provider with custom features.
    pub fn with_features(
        size: usize,
        verification_level: VerificationLevel,
        features: MacOSFeatures,
    ) -> Self {
        Self { inner: NoStdProvider::new(size, verification_level), features }
    }
}

#[cfg(target_os = "macos")]
impl MemoryProvider for MacOSOptimizedProvider {
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn verification_level(&self) -> VerificationLevel {
        self.inner.verification_level()
    }

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.inner.set_verification_level(level);
    }

    // These methods are no longer needed as we've removed the dependency on
    // wrt_foundation::safe_memory Instead, we directly use read_data and write_data

    fn write_data(&mut self, offset: usize, data: &[u8]) -> WrtResult<usize> {
        // Use optimized copy if possible
        if self.features.has_avx && data.len() >= 32 {
            // For large copies, use AVX instructions via intrinsics
            // This is a placeholder for actual AVX implementation
            // In real implementation, this would use arch-specific intrinsics
            self.inner.write_data(offset, data)
        } else {
            // Fall back to standard implementation
            self.inner.write_data(offset, data)
        }
    }

    fn read_data(&self, offset: usize, buffer: &mut [u8]) -> WrtResult<usize> {
        // Use optimized read if possible
        if self.features.has_avx && buffer.len() >= 32 {
            // For large reads, use AVX instructions via intrinsics
            // This is a placeholder for actual AVX implementation
            self.inner.read_data(offset, buffer)
        } else {
            // Fall back to standard implementation
            self.inner.read_data(offset, buffer)
        }
    }
}

#[cfg(target_os = "macos")]
impl PlatformMemoryOptimizer for MacOSOptimizedProvider {
    fn zero_copy_read(&self, source: &[u8], dest: &mut [u8]) -> WrtResult<usize> {
        // macOS doesn't generally support true zero-copy for user space
        // Fall back to optimized copy
        self.accelerated_copy(source, dest)
    }

    fn accelerated_copy(&self, source: &[u8], dest: &mut [u8]) -> WrtResult<usize> {
        // Use AVX if available for large copies
        if self.features.has_avx && source.len() >= 32 && dest.len() >= source.len() {
            // This is a placeholder - in reality we'd use AVX intrinsics here
            // For now just use the standard copy
            let copy_size = core::cmp::min(source.len(), dest.len());
            dest[..copy_size].copy_from_slice(&source[..copy_size]);
            Ok(copy_size)
        } else {
            // Fall back to standard copy
            let copy_size = core::cmp::min(source.len(), dest.len());
            dest[..copy_size].copy_from_slice(&source[..copy_size]);
            Ok(copy_size)
        }
    }

    fn align_memory(&self, ptr: *mut u8, alignment: usize) -> WrtResult<*mut u8> {
        // Ensure alignment is a power of 2
        if !alignment.is_power_of_two() {
            return Err(Error::new(
                wrt_error::ErrorCategory::Memory, 1,
                "Alignment must be a power of 2",
            ));
        }

        // Calculate aligned pointer
        let addr = ptr as usize;
        let aligned_addr = (addr + alignment - 1) & !(alignment - 1);
        Ok(aligned_addr as *mut u8)
    }

    fn secure_zero(&self, buffer: &mut [u8]) -> WrtResult<()> {
        // On macOS, we can use mlock/munlock to ensure memory is not swapped
        // For this demonstration, we'll just use volatile writes

        // This uses a simple volatile write pattern that the compiler won't optimize
        // away
        for byte in buffer.iter_mut() {
            unsafe {
                core::ptr::write_volatile(byte, 0);
            }
        }

        // Optionally add memory fence to ensure writes are completed
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    fn supports_optimization(&self, optimization: MemoryOptimization) -> bool {
        match optimization {
            MemoryOptimization::ZeroCopy => false, // macOS doesn't support true zero-copy in
            // userspace
            MemoryOptimization::HardwareAcceleration => self.features.has_avx,
            MemoryOptimization::AlignmentOptimization => true,
            MemoryOptimization::SecureZeroing => true,
            MemoryOptimization::MemoryProtection => self.features.can_use_memory_protection,
        }
    }
}

/// Optimized provider for Linux platforms.
#[cfg(target_os = "linux")]
pub struct LinuxOptimizedProvider {
    /// The underlying provider
    inner: NoStdProvider,
    /// Platform-specific features detected
    features: LinuxFeatures,
}

/// Linux-specific features for memory optimization.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct LinuxFeatures {
    /// Whether AVX/AVX2 is available for vectorized operations
    has_avx: bool,
    /// Whether memory protection features can be used
    can_use_memory_protection: bool,
    /// Optimal page size on this system
    page_size: usize,
    /// Whether transparent huge pages are available
    has_huge_pages: bool,
}

#[cfg(target_os = "linux")]
impl Default for LinuxFeatures {
    fn default() -> Self {
        Self {
            // Detect features at runtime or use conservative defaults
            has_avx: cfg!(target_feature = "avx"),
            can_use_memory_protection: true,
            page_size: 4096,       // Could be detected at runtime
            has_huge_pages: false, // Could be detected at runtime
        }
    }
}

#[cfg(target_os = "linux")]
impl LinuxOptimizedProvider {
    /// Create a new Linux-optimized provider with the specified size and
    /// verification level.
    pub fn new(size: usize, verification_level: VerificationLevel) -> Self {
        Self {
            inner: NoStdProvider::new(size, verification_level),
            features: LinuxFeatures::default(),
        }
    }

    /// Create a new Linux-optimized provider with custom features.
    pub fn with_features(
        size: usize,
        verification_level: VerificationLevel,
        features: LinuxFeatures,
    ) -> Self {
        Self { inner: NoStdProvider::new(size, verification_level), features }
    }
}

#[cfg(target_os = "linux")]
impl MemoryProvider for LinuxOptimizedProvider {
    // Implementation similar to MacOSOptimizedProvider
    // Linux-specific optimizations would go here

    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn verification_level(&self) -> VerificationLevel {
        self.inner.verification_level()
    }

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.inner.set_verification_level(level);
    }

    // These methods are no longer needed as we've removed the dependency on
    // wrt_foundation::safe_memory Instead, we directly use read_data and write_data

    fn write_data(&mut self, offset: usize, data: &[u8]) -> WrtResult<usize> {
        if self.features.has_avx && data.len() >= 32 {
            // Linux AVX optimization would go here
            self.inner.write_data(offset, data)
        } else {
            self.inner.write_data(offset, data)
        }
    }

    fn read_data(&self, offset: usize, buffer: &mut [u8]) -> WrtResult<usize> {
        if self.features.has_avx && buffer.len() >= 32 {
            // Linux AVX optimization would go here
            self.inner.read_data(offset, buffer)
        } else {
            self.inner.read_data(offset, buffer)
        }
    }
}

#[cfg(target_os = "linux")]
impl PlatformMemoryOptimizer for LinuxOptimizedProvider {
    // Linux-specific implementation
    // Would include mmap, madvise, and other Linux-specific optimizations

    fn zero_copy_read(&self, source: &[u8], dest: &mut [u8]) -> WrtResult<usize> {
        // Linux might support some zero-copy with the right setup
        // For now, fall back to accelerated copy
        self.accelerated_copy(source, dest)
    }

    fn accelerated_copy(&self, source: &[u8], dest: &mut [u8]) -> WrtResult<usize> {
        // Similar to macOS implementation
        let copy_size = core::cmp::min(source.len(), dest.len());
        dest[..copy_size].copy_from_slice(&source[..copy_size]);
        Ok(copy_size)
    }

    fn align_memory(&self, ptr: *mut u8, alignment: usize) -> WrtResult<*mut u8> {
        // Similar to macOS implementation
        if !alignment.is_power_of_two() {
            return Err(Error::new(
                wrt_error::ErrorCategory::Memory, 1,
                "Alignment must be a power of 2",
            ));
        }

        let addr = ptr as usize;
        let aligned_addr = (addr + alignment - 1) & !(alignment - 1);
        Ok(aligned_addr as *mut u8)
    }

    fn secure_zero(&self, buffer: &mut [u8]) -> WrtResult<()> {
        // On Linux, we might use explicit_bzero or similar
        // For this demonstration, we'll just use volatile writes as in macOS
        for byte in buffer.iter_mut() {
            unsafe {
                core::ptr::write_volatile(byte, 0);
            }
        }

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    fn supports_optimization(&self, optimization: MemoryOptimization) -> bool {
        match optimization {
            MemoryOptimization::ZeroCopy => false, // Might be true with the right setup
            MemoryOptimization::HardwareAcceleration => self.features.has_avx,
            MemoryOptimization::AlignmentOptimization => true,
            MemoryOptimization::SecureZeroing => true,
            MemoryOptimization::MemoryProtection => self.features.can_use_memory_protection,
        }
    }
}

/// Builder for creating platform-optimized memory providers.
///
/// This builder provides a fluent interface for configuring and creating
/// platform-specific memory providers with various optimizations. The builder
/// follows the standard pattern used throughout the WRT crates for consistency.
///
/// # Examples
///
/// ```
/// use wrt_platform::memory_optimizations::{PlatformOptimizedProviderBuilder, MacOSOptimizedProvider, MemoryOptimization};
/// use wrt_platform::memory::VerificationLevel;
///
/// #[cfg(all(target_os = "macos", feature = "std"))]
/// let provider = PlatformOptimizedProviderBuilder::<MacOSOptimizedProvider>::new()
///     .with_size(8192)
///     .with_verification_level(VerificationLevel::Critical)
///     .with_optimization(MemoryOptimization::HardwareAcceleration)
///     .with_optimization(MemoryOptimization::SecureZeroing)
///     .build();
/// ```
#[cfg(feature = "std")]
pub struct PlatformOptimizedProviderBuilder<P: PlatformMemoryOptimizer> {
    /// The buffer size in bytes
    size: usize,
    /// The verification level for safety checks
    verification_level: VerificationLevel,
    /// The set of optimizations to enable
    optimizations: Vec<MemoryOptimization>,
    /// PhantomData for the provider type
    features: PhantomData<P>,
}

#[cfg(feature = "std")]
impl<P: PlatformMemoryOptimizer> Default for PlatformOptimizedProviderBuilder<P> {
    fn default() -> Self {
        Self {
            size: 4096, // Default reasonable size
            verification_level: VerificationLevel::default(),
            optimizations: Vec::new(),
            features: PhantomData,
        }
    }
}

// Simplified version for no-alloc environments
/// Builder for creating platform-optimized memory providers in no_alloc
/// environments.
///
/// This version of the builder uses fixed-size arrays instead of Vec for
/// storing optimizations, making it suitable for environments without dynamic
/// allocation.
#[cfg(not(feature = "std"))]
pub struct PlatformOptimizedProviderBuilder<P: PlatformMemoryOptimizer> {
    /// The buffer size in bytes
    size: usize,
    /// The verification level for safety checks
    verification_level: VerificationLevel,
    /// The current number of optimizations
    optimization_count: usize,
    /// Fixed array of optimizations (up to 8)
    optimizations: [MemoryOptimization; 8], // Support up to 8 optimizations
    /// PhantomData for the provider type
    features: PhantomData<P>,
}

#[cfg(not(feature = "std"))]
impl<P: PlatformMemoryOptimizer> Default for PlatformOptimizedProviderBuilder<P> {
    fn default() -> Self {
        Self {
            size: 4096, // Default reasonable size
            verification_level: VerificationLevel::default(),
            optimization_count: 0,
            optimizations: [
                MemoryOptimization::ZeroCopy, // Default values, will be ignored unless added
                MemoryOptimization::ZeroCopy,
                MemoryOptimization::ZeroCopy,
                MemoryOptimization::ZeroCopy,
                MemoryOptimization::ZeroCopy,
                MemoryOptimization::ZeroCopy,
                MemoryOptimization::ZeroCopy,
                MemoryOptimization::ZeroCopy,
            ],
            features: PhantomData,
        }
    }
}

#[cfg(all(target_os = "macos", feature = "std"))]
impl PlatformOptimizedProviderBuilder<MacOSOptimizedProvider> {
    /// Create a new builder for macOS platforms.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the buffer size.
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set the verification level.
    pub fn with_verification_level(mut self, level: VerificationLevel) -> Self {
        self.verification_level = level;
        self
    }

    /// Enable a specific optimization.
    pub fn with_optimization(mut self, opt: MemoryOptimization) -> Self {
        if !self.optimizations.contains(&opt) {
            self.optimizations.push(opt);
        }
        self
    }

    /// Build a macOS-optimized provider.
    pub fn build(self) -> MacOSOptimizedProvider {
        let mut features = MacOSFeatures::default();

        // Configure features based on requested optimizations
        if self.optimizations.contains(&MemoryOptimization::HardwareAcceleration) {
            features.has_avx = cfg!(target_feature = "avx");
        }

        if self.optimizations.contains(&MemoryOptimization::MemoryProtection) {
            features.can_use_memory_protection = true;
        }

        MacOSOptimizedProvider::with_features(self.size, self.verification_level, features)
    }
}

// No-alloc version for macOS
#[cfg(all(target_os = "macos", not(feature = "std")))]
impl PlatformOptimizedProviderBuilder<MacOSOptimizedProvider> {
    /// Create a new builder for macOS platforms.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the buffer size.
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set the verification level.
    pub fn with_verification_level(mut self, level: VerificationLevel) -> Self {
        self.verification_level = level;
        self
    }

    /// Enable a specific optimization.
    pub fn with_optimization(mut self, opt: MemoryOptimization) -> Self {
        // Only add if not already present and space available
        if self.optimization_count < self.optimizations.len() {
            let mut found = false;
            for i in 0..self.optimization_count {
                if self.optimizations[i] == opt {
                    found = true;
                    break;
                }
            }

            if !found {
                self.optimizations[self.optimization_count] = opt;
                self.optimization_count += 1;
            }
        }
        self
    }

    /// Build a macOS-optimized provider.
    pub fn build(self) -> MacOSOptimizedProvider {
        let mut features = MacOSFeatures::default();

        // Configure features based on requested optimizations
        for i in 0..self.optimization_count {
            match self.optimizations[i] {
                MemoryOptimization::HardwareAcceleration => {
                    features.has_avx = cfg!(target_feature = "avx");
                }
                MemoryOptimization::MemoryProtection => {
                    features.can_use_memory_protection = true;
                }
                _ => {}
            }
        }

        MacOSOptimizedProvider::with_features(self.size, self.verification_level, features)
    }
}

#[cfg(all(target_os = "linux", feature = "std"))]
impl PlatformOptimizedProviderBuilder<LinuxOptimizedProvider> {
    /// Create a new builder for Linux platforms.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the buffer size.
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set the verification level.
    pub fn with_verification_level(mut self, level: VerificationLevel) -> Self {
        self.verification_level = level;
        self
    }

    /// Enable a specific optimization.
    pub fn with_optimization(mut self, opt: MemoryOptimization) -> Self {
        if !self.optimizations.contains(&opt) {
            self.optimizations.push(opt);
        }
        self
    }

    /// Build a Linux-optimized provider.
    pub fn build(self) -> LinuxOptimizedProvider {
        let mut features = LinuxFeatures::default();

        // Configure features based on requested optimizations
        if self.optimizations.contains(&MemoryOptimization::HardwareAcceleration) {
            features.has_avx = cfg!(target_feature = "avx");
        }

        if self.optimizations.contains(&MemoryOptimization::MemoryProtection) {
            features.can_use_memory_protection = true;
        }

        LinuxOptimizedProvider::with_features(self.size, self.verification_level, features)
    }
}

// No-alloc version for Linux
#[cfg(all(target_os = "linux", not(feature = "std")))]
impl PlatformOptimizedProviderBuilder<LinuxOptimizedProvider> {
    /// Create a new builder for Linux platforms.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the buffer size.
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set the verification level.
    pub fn with_verification_level(mut self, level: VerificationLevel) -> Self {
        self.verification_level = level;
        self
    }

    /// Enable a specific optimization.
    pub fn with_optimization(mut self, opt: MemoryOptimization) -> Self {
        // Only add if not already present and space available
        if self.optimization_count < self.optimizations.len() {
            let mut found = false;
            for i in 0..self.optimization_count {
                if self.optimizations[i] == opt {
                    found = true;
                    break;
                }
            }

            if !found {
                self.optimizations[self.optimization_count] = opt;
                self.optimization_count += 1;
            }
        }
        self
    }

    /// Build a Linux-optimized provider.
    pub fn build(self) -> LinuxOptimizedProvider {
        let mut features = LinuxFeatures::default();

        // Configure features based on requested optimizations
        for i in 0..self.optimization_count {
            match self.optimizations[i] {
                MemoryOptimization::HardwareAcceleration => {
                    features.has_avx = cfg!(target_feature = "avx");
                }
                MemoryOptimization::MemoryProtection => {
                    features.can_use_memory_protection = true;
                }
                _ => {}
            }
        }

        LinuxOptimizedProvider::with_features(self.size, self.verification_level, features)
    }
}

/// Module for optimized collection types that use platform-specific memory
/// optimizations.
///
/// This module will eventually contain optimized implementations of various
/// collection types (Vector, Map, Set, etc.) that leverage platform-specific
/// memory optimizations. In the current version, this is a placeholder for
/// future development.
pub mod optimized_collections {
    // TODO: Add integration with wrt-foundation BoundedVec, BoundedMap, etc.
    // This will be implemented in a future version that properly handles
    // the dependency structure between crates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_memory_optimization_enum() {
        // Basic test that just checks the enum variants
        let opt1 = MemoryOptimization::ZeroCopy;
        let opt2 = MemoryOptimization::HardwareAcceleration;

        assert_ne!(opt1, opt2);
    }

    #[cfg(all(target_os = "macos", feature = "std"))]
    #[test]
    fn test_platform_builder() {
        let builder = PlatformOptimizedProviderBuilder::<MacOSOptimizedProvider>::new()
            .with_size(2048)
            .with_verification_level(VerificationLevel::Full)
            .with_optimization(MemoryOptimization::HardwareAcceleration)
            .with_optimization(MemoryOptimization::SecureZeroing);

        let provider = builder.build();
        assert_eq!(provider.capacity(), 2048);
        assert_eq!(provider.verification_level(), VerificationLevel::Full);
        assert!(provider.supports_optimization(MemoryOptimization::SecureZeroing));
    }
}
