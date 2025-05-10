//! Tests for the safe memory implementation

use wrt_types::{
    safe_memory::{MemoryProvider, SafeSlice},
    verification::VerificationLevel,
};

#[cfg(feature = "std")]
use wrt_types::safe_memory::StdMemoryProvider;

#[cfg(not(feature = "std"))]
use wrt_types::safe_memory::NoStdMemoryProvider;

#[test]
fn test_safe_slice_creation() {
    let data = vec![1, 2, 3, 4, 5];
    let slice = SafeSlice::new(&data);

    // Verify data access works
    assert_eq!(slice.as_ref().unwrap().data().unwrap(), &[1, 2, 3, 4, 5]);
    assert_eq!(slice.as_ref().unwrap().len(), 5);
    assert!(!slice.as_ref().unwrap().is_empty());
}

#[test]
fn test_safe_slice_verification_levels() {
    let data = vec![1, 2, 3, 4, 5];

    // Create with different verification levels
    let slice_none = SafeSlice::with_verification_level(&data, VerificationLevel::None);
    let slice_sampling = SafeSlice::with_verification_level(&data, VerificationLevel::Sampling);
    let slice_standard = SafeSlice::with_verification_level(&data, VerificationLevel::Standard);
    let slice_full = SafeSlice::with_verification_level(&data, VerificationLevel::Full);

    // All should return the same data
    assert_eq!(slice_none.as_ref().unwrap().data().unwrap(), &[1, 2, 3, 4, 5]);
    assert_eq!(slice_sampling.as_ref().unwrap().data().unwrap(), &[1, 2, 3, 4, 5]);
    assert_eq!(slice_standard.as_ref().unwrap().data().unwrap(), &[1, 2, 3, 4, 5]);
    assert_eq!(slice_full.as_ref().unwrap().data().unwrap(), &[1, 2, 3, 4, 5]);
}

#[cfg(feature = "std")]
#[test]
fn test_std_memory_provider() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let provider = StdMemoryProvider::new(data);

    // Test borrow_slice
    let slice = provider.borrow_slice(2, 3).unwrap();
    assert_eq!(slice.data().unwrap(), &[3, 4, 5]);

    // Test size
    assert_eq!(provider.size(), 10);

    // Test verify_access
    assert!(provider.verify_access(0, 10).is_ok());
    assert!(provider.verify_access(5, 5).is_ok());
    assert!(provider.verify_access(10, 1).is_err()); // Out of bounds
}

#[cfg(not(feature = "std"))]
#[test]
fn test_nostd_memory_provider() {
    let mut provider = NoStdMemoryProvider::<16>::new();
    provider.set_data(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap();

    // Test borrow_slice
    let slice = provider.borrow_slice(2, 3).unwrap();
    assert_eq!(slice.data().unwrap(), &[3, 4, 5]);

    // Test size
    assert_eq!(provider.size(), 10);

    // Test verify_access
    assert!(provider.verify_access(0, 10).is_ok());
    assert!(provider.verify_access(5, 5).is_ok());
    assert!(provider.verify_access(10, 1).is_err()); // Out of bounds
}

#[cfg(feature = "std")]
#[test]
fn test_memory_stats() {
    let data = vec![0; 1024];
    let provider = StdMemoryProvider::new(data);

    // Access different regions
    let _ = provider.borrow_slice(0, 100).unwrap();
    let _ = provider.borrow_slice(200, 100).unwrap();
    let _ = provider.borrow_slice(500, 200).unwrap();

    // Get stats
    let stats = provider.memory_stats();

    // Verify stats
    assert_eq!(stats.total_size, 1024);
    assert_eq!(stats.access_count, 3);
    assert!(stats.unique_regions > 0);
    assert_eq!(stats.max_access_size, 200);
}

#[cfg(not(feature = "std"))]
#[test]
fn test_memory_stats() {
    let mut provider = NoStdMemoryProvider::<1024>::new();
    provider.set_data(&[0; 1024]).unwrap();

    // Access different regions
    let _ = provider.borrow_slice(0, 100).unwrap();
    let _ = provider.borrow_slice(200, 100).unwrap();
    let _ = provider.borrow_slice(500, 200).unwrap();

    // Get stats
    let stats = provider.memory_stats();

    // Verify stats
    assert_eq!(stats.total_size, 1024);
    assert_eq!(stats.access_count, 3);
    assert!(stats.unique_regions > 0);
    assert_eq!(stats.max_access_size, 200);
}

#[cfg(feature = "std")]
#[test]
fn test_memory_safety_trait() {
    let data = vec![0; 1024];
    let mut provider = StdMemoryProvider::new(data);

    // Test MemorySafety trait methods
    assert!(provider.verify_integrity().is_ok());

    // Test changing verification level
    assert_eq!(provider.verification_level(), VerificationLevel::Standard);
    provider.set_verification_level(VerificationLevel::Full);
    assert_eq!(provider.verification_level(), VerificationLevel::Full);

    // Test memory stats
    let stats = provider.memory_stats();
    assert_eq!(stats.total_size, 1024);
}

#[cfg(not(feature = "std"))]
#[test]
fn test_memory_safety_trait() {
    let mut provider = NoStdMemoryProvider::<1024>::new();
    provider.set_data(&[0; 1024]).unwrap();

    // Test MemorySafety trait methods
    assert!(provider.verify_integrity().is_ok());

    // Test verification level (starts at Standard)
    assert_eq!(provider.verification_level(), VerificationLevel::Standard);

    // Change verification level
    provider.set_verification_level(VerificationLevel::Full);
    assert_eq!(provider.verification_level(), VerificationLevel::Full);

    // Test memory stats
    let stats = provider.memory_stats();
    assert_eq!(stats.total_size, 1024);
}
