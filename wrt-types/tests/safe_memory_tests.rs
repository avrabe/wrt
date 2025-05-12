//! Tests for SafeMemory implementations
//!
//! This file contains comprehensive tests for the SafeMemory module
//! and its providers (StdMemoryProvider and NoStdMemoryProvider).

#![cfg(test)]

#[cfg(feature = "std")]
extern crate std;

extern crate wrt_types;

#[cfg(feature = "std")]
use wrt_types::safe_memory::StdMemoryProvider;
// Common imports
use wrt_types::{
    prelude::*,
    safe_memory::{MemoryProvider, SafeSlice, SafeSliceMut},
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
    use wrt_types::safe_memory::NoStdMemoryProvider;

    use super::*;

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
        assert_eq!(provider.verification_level(), VerificationLevel::default());

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
    let slice_none = SafeSlice::with_verification_level(data, VerificationLevel::Off).unwrap();
    let slice_sampling =
        SafeSlice::with_verification_level(data, VerificationLevel::default()).unwrap();
    let slice_basic = SafeSlice::with_verification_level(data, VerificationLevel::Basic).unwrap();
    let slice_full = SafeSlice::with_verification_level(data, VerificationLevel::Full).unwrap();

    // Access data with different verification levels - ensure integrity checks are
    // potentially run
    assert_eq!(slice_none.data().unwrap(), data);
    assert_eq!(slice_sampling.data().unwrap(), data);
    assert_eq!(slice_basic.data().unwrap(), data);
    assert_eq!(slice_full.data().unwrap(), data);

    // Test creating sub-slices
    let sub_slice_none = slice_none.slice(1, 3).unwrap();
    assert_eq!(sub_slice_none.data().unwrap(), &[2, 3, 4]);

    let sub_slice_full = slice_full.slice(1, 3).unwrap();
    assert_eq!(sub_slice_full.data().unwrap(), &[2, 3, 4]);
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
    assert_eq!(provider.verification_level(), VerificationLevel::default());

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

#[cfg(feature = "std")]
#[test]
fn test_safe_slice_sub_slicing() {
    let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let provider = StdMemoryProvider::new(data);
    let handler = SafeMemoryHandler::new(provider, VerificationLevel::Off);

    // Get a slice from the handler
    let slice_res = handler.get_slice(0, 10);
    assert!(slice_res.is_ok());
    let slice = slice_res.unwrap();

    // Now operate on 'slice' which is SafeSlice
    let sub_slice = slice.slice(2, 6).unwrap();
    assert_eq!(sub_slice.data().unwrap(), &[2, 3, 4, 5, 6, 7]);

    // Create another level of sub-slicing
    let sub_sub_slice = sub_slice.slice(0, 2).unwrap();
    assert_eq!(sub_sub_slice.data().unwrap(), &[2, 3]);

    // Test out of bounds sub-slicing
    let result_oob_1 = slice.slice(5, 8);
    assert!(result_oob_1.is_err());

    let result_oob_2 = slice.slice(5, 9);
    assert!(result_oob_2.is_err());
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
    use wrt_types::{
        safe_memory::{MemoryProvider, SafeSlice, StdMemoryProvider},
        verification::VerificationLevel,
    };

    use super::*;

    #[test]
    fn test_safe_slice_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let slice = SafeSlice::new(&data).unwrap();

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
            VerificationLevel::Off,
            VerificationLevel::Sampling,
            VerificationLevel::Basic,
            VerificationLevel::Full,
        ];

        for level in &levels {
            let slice = SafeSlice::with_verification_level(&data, *level).unwrap();

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
        let slice = SafeSlice::new(&data).unwrap();

        // slice.slice(start, len)
        let sub_slice = slice.slice(2, 6).unwrap(); // start=2, len=6. Data: data[2..8] -> &[3,4,5,6,7,8]
        assert_eq!(sub_slice.data().unwrap(), &[3, 4, 5, 6, 7, 8]);

        // Create a nested sub-slice from sub_slice (data: &[3,4,5,6,7,8], len: 6)
        // sub_slice.slice(start, len)
        let nested_slice = sub_slice.slice(0, 2).unwrap(); // start=0, len=2. Data: sub_slice.data[0..2] -> &[3,4]
        assert_eq!(nested_slice.data().unwrap(), &[3, 4]);

        // Create a nested sub-slice with boundaries of the entire sub-slice
        // sub_slice.slice(start, len)
        let boundary_slice = sub_slice.slice(0, 4).unwrap(); // start=0, len=4. Data: sub_slice.data[0..4] -> &[3,4,5,6]
        assert_eq!(boundary_slice.data().unwrap(), &[3, 4, 5, 6]);

        // Test out of bounds for sub_slice (len 6)
        assert!(sub_slice.slice(0, 7).is_err()); // CORRECTED: 0+7 > 6
        assert!(sub_slice.slice(5, 2).is_err()); // CORRECTED: 5+2 > 6
        assert!(sub_slice.slice(6, 1).is_err()); // CORRECTED: 6+1 > 6 (start 6
                                                 // is also OOB for len 1 if
                                                 // original len is 6)
    }

    #[test]
    fn test_safe_slice_integrity_verification() {
        let data = vec![1, 2, 3, 4, 5];
        let slice = SafeSlice::with_verification_level(&data, VerificationLevel::Full).unwrap();

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

#[test]
fn test_safe_slice_mut_operations() {
    // Each SafeSliceMut needs its own scope or its own data to avoid borrow errors.
    // Here, we test modifying the same underlying data sequentially.
    {
        let mut data = vec![1, 2, 3, 4, 5];
        let mut slice_none =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Off).unwrap();
        slice_none.data_mut().expect("data_mut on slice_none failed")[0] = 10;
        slice_none.update_checksum(); // Important after modification
        assert!(slice_none.verify_integrity().is_ok());
        assert_eq!(data[0], 10);
    }

    {
        let mut data = vec![1, 2, 3, 4, 5];
        let mut slice_sampling =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::default()).unwrap();
        slice_sampling.data_mut().expect("data_mut on slice_sampling failed")[1] = 20;
        slice_sampling.update_checksum();
        assert!(slice_sampling.verify_integrity().is_ok());
        assert_eq!(data[1], 20);
    }

    {
        let mut data = vec![1, 2, 3, 4, 5];
        let mut slice_basic =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Basic).unwrap();
        slice_basic.data_mut().expect("data_mut on slice_basic failed")[2] = 30;
        slice_basic.update_checksum();
        assert!(slice_basic.verify_integrity().is_ok());
        assert_eq!(data[2], 30);
    }

    {
        let mut data = vec![1, 2, 3, 4, 5];
        let mut slice_full =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Full).unwrap();
        slice_full.data_mut().expect("data_mut on slice_full failed")[3] = 40;
        slice_full.update_checksum();
        assert!(slice_full.verify_integrity().is_ok());
        assert_eq!(data[3], 40);
    }
}

#[test]
fn test_safe_slice_sub_slicing_and_errors() {
    let data = &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let slice = SafeSlice::with_verification_level(data, VerificationLevel::Full).unwrap();

    // Valid sub-slice
    let sub_slice = slice.slice(2, 6).unwrap();
    assert_eq!(sub_slice.data().unwrap(), &[2, 3, 4, 5, 6, 7]);
    assert_eq!(sub_slice.len(), 6);

    // Valid sub-slice that was previously misinterpreted as an error case
    let result_valid = slice.slice(5, 2).unwrap();
    assert_eq!(result_valid.data().unwrap(), &[5, 6]);

    // Invalid: start+len > original_length
    let result_oob_end = slice.slice(5, 11);
    assert!(result_oob_end.is_err());

    // Invalid: start >= original_length (if len > 0) or start+len > original_length
    let result_oob_start = slice.slice(10, 1);
    assert!(result_oob_start.is_err());

    // Zero length slice at end should be ok
    let zero_len_at_end = slice.slice(10, 0).unwrap();
    assert!(zero_len_at_end.is_empty());
}

// Test for SafeSliceMut integrity checks and operations at different levels
// This is where the E0499 errors likely occurred.
// The original test name is unknown, using a descriptive one.
#[test]
fn test_safe_slice_mut_integrity_checks_levels() {
    let mut data_orig = [0u8; 32];
    for i in 0..data_orig.len() {
        data_orig[i] = i as u8;
    }

    {
        let mut data = data_orig.clone();
        let mut slice_none =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Off).unwrap();
        slice_none.data_mut().unwrap()[0] = 100;
        slice_none.update_checksum();
        assert!(slice_none.verify_integrity().is_ok());
        assert_eq!(slice_none.data().unwrap()[0], 100);
    }

    {
        let mut data = data_orig.clone();
        let mut slice_sampling =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Sampling).unwrap();
        slice_sampling.data_mut().unwrap()[1] = 101;
        slice_sampling.update_checksum();
        assert!(slice_sampling.verify_integrity().is_ok());
        assert_eq!(slice_sampling.data().unwrap()[1], 101);
    }

    {
        let mut data = data_orig.clone();
        let mut slice_basic =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Basic).unwrap();
        slice_basic.data_mut().unwrap()[2] = 102;
        slice_basic.update_checksum();
        assert!(slice_basic.verify_integrity().is_ok());
        assert_eq!(slice_basic.data().unwrap()[2], 102);
    }

    {
        let mut data = data_orig.clone();
        let mut slice_full =
            SafeSliceMut::with_verification_level(&mut data, VerificationLevel::Full).unwrap();
        slice_full.data_mut().unwrap()[3] = 103;
        slice_full.update_checksum();
        assert!(slice_full.verify_integrity().is_ok());
        assert_eq!(slice_full.data().unwrap()[3], 103);
    }
}
