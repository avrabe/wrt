//! Tests for SafeMemory implementations
//!
//! This file contains comprehensive tests for the SafeMemory module
//! and its providers (StdMemoryProvider and NoStdMemoryProvider).

#![cfg(test)]

#[cfg(feature = "std")]
extern crate std;

extern crate wrt_types;

use std::hint::black_box;
// Common imports (assuming these are okay or handled internally by wrt-types for no_std)
use wrt_types::WrtResult as Result;
use wrt_types::{
    prelude::*,
    safe_memory::{MemoryProvider, SafeMemoryHandler, SafeSlice, StdMemoryProvider},
    verification::VerificationLevel,
};

#[cfg(feature = "std")]
#[test]
fn test_std_memory_provider() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let provider = StdMemoryProvider::new(data);

    // Test basic properties
    assert_eq!(provider.size(), 8);

    // Test slice borrowing
    let slice = provider.borrow_slice(2, 4).unwrap();
    assert_eq!(slice.len(), 4);
    assert_eq!(slice.data().unwrap(), &[3, 4, 5, 6]);

    // Test access tracking
    let stats = provider.memory_stats();
    assert_eq!(stats.access_count, 1);
}

#[cfg(feature = "std")]
#[test]
fn test_std_memory_provider_out_of_bounds() {
    let data = vec![1, 2, 3, 4, 5];
    let provider = StdMemoryProvider::new(data);

    // Test out of bounds
    let result = provider.borrow_slice(3, 3);
    assert!(result.is_err());

    // Verify error is correct type
    if let Err(e) = result {
        assert!(e.to_string().contains("out of bounds"));
    }
}

#[cfg(feature = "std")]
#[test]
fn test_std_memory_provider_operations() {
    let mut provider = StdMemoryProvider::with_capacity(10);

    // Add data
    provider.add_data(&[1, 2, 3, 4]);
    assert_eq!(provider.size(), 4);

    // Borrow slice
    let slice = provider.borrow_slice(1, 2).unwrap();
    assert_eq!(slice.data().unwrap(), &[2, 3]);

    // Resize with more data
    provider.resize(8, 0).unwrap();
    assert_eq!(provider.size(), 8);

    // Clear
    provider.clear();
    assert_eq!(provider.size(), 0);

    // After clearing, the access count should be reset
    let stats = provider.memory_stats();
    assert_eq!(stats.access_count, 0);
}

#[cfg(feature = "std")]
#[test]
fn test_std_memory_provider_integrity() {
    let mut provider = StdMemoryProvider::with_capacity(1024);
    provider.add_data(&[1, 2, 3, 4, 5, 6, 7, 8]);

    // Access a few slices
    provider.borrow_slice(0, 4).unwrap();
    provider.borrow_slice(2, 4).unwrap();
    provider.borrow_slice(4, 4).unwrap();

    // Check integrity
    assert!(provider.verify_integrity().is_ok());

    // Get stats
    let stats = provider.memory_stats();
    assert_eq!(stats.access_count, 3);
    assert!(stats.unique_regions > 0);
}

// NoStd memory provider tests are only included when not using std feature
#[cfg(not(feature = "std"))]
mod nostd_tests {
    use super::*;
    use wrt_types::safe_memory::NoStdMemoryProvider;

    #[test]
    fn test_nostd_memory_provider() {
        let mut provider = NoStdMemoryProvider::<16>::new();

        // Set data
        provider.set_data(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        assert_eq!(provider.size(), 8);

        // Borrow slice
        let slice = provider.borrow_slice(2, 4).unwrap();
        assert_eq!(slice.data().unwrap(), &[3, 4, 5, 6]);

        // Check access count
        assert_eq!(provider.access_count(), 1);

        // Check last access
        let (offset, length) = provider.last_access();
        assert_eq!(offset, 2);
        assert_eq!(length, 4);
    }

    #[test]
    fn test_nostd_memory_provider_out_of_bounds() {
        let mut provider = NoStdMemoryProvider::<8>::new();
        provider.set_data(&[1, 2, 3, 4, 5]).unwrap();

        // Test out of bounds
        let result = provider.borrow_slice(3, 3);
        assert!(result.is_err());

        // Verify error is correct type
        if let Err(e) = result {
            assert!(e.to_string().contains("out of bounds"));
        }
    }

    #[test]
    fn test_nostd_memory_provider_operations() {
        let mut provider = NoStdMemoryProvider::<16>::new();

        // Set data
        provider.set_data(&[1, 2, 3, 4]).unwrap();
        assert_eq!(provider.size(), 4);

        // Resize
        provider.resize(6).unwrap();
        assert_eq!(provider.size(), 6);

        // Check that new memory is zeroed
        let slice = provider.borrow_slice(4, 2).unwrap();
        assert_eq!(slice.data().unwrap(), &[0, 0]);

        // Clear
        provider.clear();
        assert_eq!(provider.size(), 0);
    }

    #[test]
    fn test_nostd_memory_provider_integrity() {
        let mut provider = NoStdMemoryProvider::<16>::new();
        provider.set_data(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        // Access a slice
        provider.borrow_slice(2, 4).unwrap();

        // Check integrity
        assert!(provider.verify_integrity().is_ok());
    }

    #[test]
    fn test_nostd_memory_safety_trait() {
        let mut provider = NoStdMemoryProvider::<16>::new();
        provider.set_data(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        // Get statistics
        let stats = provider.memory_stats();
        assert_eq!(stats.total_size, 8);
        assert_eq!(stats.access_count, 0); // No accesses yet

        // Check verification level
        assert_eq!(provider.verification_level(), VerificationLevel::Standard);

        // Verify integrity
        assert!(provider.verify_integrity().is_ok());

        // Make some accesses
        provider.borrow_slice(0, 2).unwrap();
        provider.borrow_slice(4, 4).unwrap();

        // Check stats again
        let stats = provider.memory_stats();
        assert_eq!(stats.access_count, 2);
        assert_eq!(stats.max_access_size, 4);
    }
}

#[test]
fn test_safe_slice_verification_levels() {
    let data = &[1, 2, 3, 4, 5, 6, 7, 8];

    // Create safe slices with different verification levels
    let slice_none = SafeSlice::with_verification_level(data, VerificationLevel::None);
    let slice_sampling = SafeSlice::with_verification_level(data, VerificationLevel::Sampling);
    let slice_standard = SafeSlice::with_verification_level(data, VerificationLevel::Standard);
    let slice_full = SafeSlice::with_verification_level(data, VerificationLevel::Full);

    // All should provide correct data access
    assert_eq!(slice_none.data().unwrap(), data);
    assert_eq!(slice_sampling.data().unwrap(), data);
    assert_eq!(slice_standard.data().unwrap(), data);
    assert_eq!(slice_full.data().unwrap(), data);

    // All should have correct length
    assert_eq!(slice_none.len(), 8);
    assert_eq!(slice_sampling.len(), 8);
    assert_eq!(slice_standard.len(), 8);
    assert_eq!(slice_full.len(), 8);
}

#[cfg(feature = "std")]
#[test]
fn test_memory_safety_trait() {
    // Create providers implementing MemorySafety
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let provider = StdMemoryProvider::new(data);

    // Get statistics
    let stats = provider.memory_stats();
    assert_eq!(stats.total_size, 8);
    assert_eq!(stats.access_count, 0); // No accesses yet

    // Check verification level
    assert_eq!(provider.verification_level(), VerificationLevel::Standard);

    // Verify integrity
    assert!(provider.verify_integrity().is_ok());

    // Make some accesses
    provider.borrow_slice(0, 4).unwrap();
    provider.borrow_slice(4, 4).unwrap();

    // Check stats again
    let stats = provider.memory_stats();
    assert_eq!(stats.access_count, 2);
    assert!(stats.unique_regions > 0);
    assert_eq!(stats.max_access_size, 4);
}

#[test]
fn test_safe_slice_sub_slicing() {
    let data = &[1, 2, 3, 4, 5, 6, 7, 8];
    let slice = SafeSlice::new(data);

    // Create a sub-slice from index 2 to 6 (exclusive)
    let sub_slice = slice.slice(2, 6).unwrap();
    assert_eq!(sub_slice.data().unwrap(), &[3, 4, 5, 6]);

    // Create another level of sub-slicing from index 0 to 2 (exclusive)
    let sub_sub_slice = sub_slice.slice(0, 2).unwrap();
    assert_eq!(sub_sub_slice.data().unwrap(), &[3, 4]);

    // Test out of bounds sub-slicing
    let result = slice.slice(5, 8);
    assert!(result.is_ok());
    let result = slice.slice(5, 9);
    assert!(result.is_err());
}

// Generic test functions to ensure traits are properly implemented

#[cfg(feature = "std")]
#[test]
fn test_generic_memory_provider() {
    // Test with StdMemoryProvider
    let std_provider = StdMemoryProvider::new(vec![1, 2, 3, 4, 5]);
    let data = access_memory(&std_provider).unwrap();
    assert_eq!(data, vec![1, 2, 3, 4, 5]);

    // Generic function to access memory
    fn access_memory<P: MemoryProvider>(provider: &P) -> Result<Vec<u8>> {
        let slice = provider.borrow_slice(0, provider.size())?;
        Ok(slice.data()?.to_vec())
    }
}

#[cfg(feature = "std")]
#[test]
fn test_generic_memory_safety() {
    let std_provider = StdMemoryProvider::new(vec![1, 2, 3, 4, 5]);
    verify_and_access(&std_provider).unwrap();

    // Generic function to verify and access memory
    fn verify_and_access<M: MemoryProvider>(provider: &M) -> Result<()> {
        provider.verify_integrity()?;
        let slice = provider.borrow_slice(0, provider.size())?;
        // Perform some operation with slice.data() if needed, e.g., black_box it
        let _ = black_box(slice.data()?);
        Ok(())
    }
}

// More comprehensive tests in a separate module
#[cfg(feature = "std")]
mod tests {
    use super::*;
    use wrt_types::{
        safe_memory::{MemoryProvider, SafeSlice, StdMemoryProvider},
        verification::VerificationLevel,
    };

    #[test]
    fn test_safe_slice_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let slice = SafeSlice::new(&data);

        assert_eq!(slice.data().unwrap(), &[1, 2, 3, 4, 5]);
        assert_eq!(slice.len(), 5);
        assert!(!slice.is_empty());
    }

    #[test]
    fn test_std_memory_provider() {
        let data = vec![1, 2, 3, 4, 5];
        let provider = StdMemoryProvider::new(data);

        let slice = provider.borrow_slice(1, 3).unwrap();
        assert_eq!(slice.data().unwrap(), &[2, 3, 4]);
    }

    #[test]
    fn test_safe_slice_with_verification_levels() {
        let data = vec![1, 2, 3, 4, 5];

        // Test with all verification levels
        let levels = [
            VerificationLevel::None,
            VerificationLevel::Sampling,
            VerificationLevel::Standard,
            VerificationLevel::Full,
        ];

        for level in &levels {
            let slice = SafeSlice::with_verification_level(&data, *level);

            // Basic properties should work
            assert_eq!(slice.data().unwrap(), &[1, 2, 3, 4, 5]);
            assert_eq!(slice.len(), 5);

            // Verification level should be retained
            assert_eq!(slice.verification_level(), *level);
        }
    }

    #[test]
    fn test_out_of_bounds_access() {
        let data = vec![1, 2, 3, 4, 5];
        let provider = StdMemoryProvider::new(data);

        // Valid access
        assert!(provider.borrow_slice(0, 5).is_ok());

        // Invalid accesses
        assert!(provider.borrow_slice(1, 5).is_err());
        assert!(provider.borrow_slice(5, 1).is_err());
        assert!(provider.borrow_slice(6, 0).is_err());
    }

    #[test]
    fn test_slice_sub_slicing() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let slice = SafeSlice::new(&data);

        // Create a sub-slice from indices 2 to 6 (exclusive of 2, inclusive of 5)
        let sub_slice = slice.slice(2, 6).unwrap();
        assert_eq!(sub_slice.data().unwrap(), &[3, 4, 5, 6]);

        // Create a nested sub-slice from indices 0 to 2 of the sub-slice
        let nested_slice = sub_slice.slice(0, 2).unwrap();
        // The nested slice should contain elements at positions 0 and 1 of the sub_slice
        assert_eq!(nested_slice.data().unwrap(), &[3, 4]);

        // Create a nested sub-slice with boundaries of the entire sub-slice
        let boundary_slice = sub_slice.slice(0, 4).unwrap();
        assert_eq!(boundary_slice.data().unwrap(), &[3, 4, 5, 6]);

        // Test out of bounds
        assert!(sub_slice.slice(0, 5).is_err());
        assert!(sub_slice.slice(4, 1).is_err());
    }

    #[test]
    fn test_safe_slice_integrity_verification() {
        let data = vec![1, 2, 3, 4, 5];
        let slice = SafeSlice::with_verification_level(&data, VerificationLevel::Full);

        // Basic integrity check
        assert!(slice.verify_integrity().is_ok());

        // Data access should work
        assert_eq!(slice.data().unwrap(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_memory_provider_safety() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut provider = StdMemoryProvider::new(data);

        // Set verification level
        provider.set_verification_level(VerificationLevel::Full);
        assert_eq!(provider.verification_level(), VerificationLevel::Full);

        // Get statistics
        let stats_before = provider.memory_stats();
        assert_eq!(stats_before.access_count, 0);

        // Make some accesses
        provider.borrow_slice(1, 3).unwrap();
        provider.borrow_slice(4, 3).unwrap();

        // Check statistics after access
        let stats_after = provider.memory_stats();
        assert_eq!(stats_after.access_count, 2);
        assert_eq!(stats_after.max_access_size, 3);
    }
}
