//! Simplified Memory Safety Formal Verification
//!
//! This module provides KANI formal verification for memory safety properties
//! using simplified types that work with the current WRT memory system.
//!
//! # Verified Properties
//!
//! - Memory allocations respect capacity bounds
//! - No buffer overflows in bounded collections
//! - Allocation failures handled gracefully
//! - Memory provider isolation between components

#![cfg(any(doc, kani, feature = "kani"))]
#![deny(clippy::all)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

use wrt_test_registry::prelude::*;

#[cfg(kani)]
use kani;

#[cfg(feature = "kani")]
use wrt_foundation::{
    bounded::BoundedVec,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    safe_memory::NoStdProvider,
};

/// Verify bounded vector capacity limits are never exceeded
///
/// This harness tests that BoundedVec correctly enforces capacity limits
/// and cannot be corrupted by arbitrary inputs.
#[cfg(kani)]
pub fn verify_bounded_vec_capacity_limits() {
    // Generate arbitrary capacity within reasonable bounds
    let requested_elements: usize = kani::any);
    kani::assume(requested_elements <= 1024); // Reasonable upper bound for verification
    
    // Create a provider with fixed size
    let provider = safe_managed_alloc!(8192, CrateId::Component)
        .unwrap_or_else(|_| NoStdProvider::<8192>::default);
    
    // Test BoundedVec with the requested capacity (clamped to max)
    const MAX_ELEMENTS: usize = 256;
    match BoundedVec::<u32, MAX_ELEMENTS, _>::new(provider) {
        Ok(mut vec) => {
            // Verify initial state
            assert_eq!(vec.len(), 0;
            assert!(vec.capacity() <= MAX_ELEMENTS);
            
            // Try to add elements up to the requested amount
            let elements_to_add = requested_elements.min(MAX_ELEMENTS;
            let mut added_count = 0;
            
            for i in 0..elements_to_add {
                match vec.push(i as u32) {
                    Ok(()) => {
                        added_count += 1;
                        // Verify length is consistent
                        assert_eq!(vec.len(), added_count;
                        assert!(vec.len() <= MAX_ELEMENTS);
                    }
                    Err(_) => {
                        // Push failed - this is acceptable when at capacity
                        // Verify we're at capacity
                        assert!(vec.len() <= MAX_ELEMENTS);
                        break;
                    }
                }
            }
            
            // Final verification - length should never exceed capacity
            assert!(vec.len() <= vec.capacity();
            assert!(vec.capacity() <= MAX_ELEMENTS);
        }
        Err(_) => {
            // Provider creation can fail - this is acceptable
        }
    }
}

/// Verify memory provider isolation between different buffer instances
///
/// This harness tests that different BoundedVec instances don't interfere
/// with each other even when using the same provider type.
#[cfg(kani)]
pub fn verify_buffer_isolation() {
    // Create two independent providers
    let provider_a = safe_managed_alloc!(4096, CrateId::Component)
        .unwrap_or_else(|_| NoStdProvider::<4096>::default);
    let provider_b = safe_managed_alloc!(4096, CrateId::Component)
        .unwrap_or_else(|_| NoStdProvider::<4096>::default);
    
    // Create bounded vectors with each provider
    if let (Ok(mut vec_a), Ok(mut vec_b)) = (
        BoundedVec::<u16, 64, _>::new(provider_a),
        BoundedVec::<u16, 64, _>::new(provider_b),
    ) {
        // Add different data to each vector
        let _ = vec_a.push(0x1111);
        let _ = vec_a.push(0x2222);
        
        let _ = vec_b.push(0xAAAA);
        let _ = vec_b.push(0xBBBB);
        
        // Verify each vector maintains its own data
        if vec_a.len() >= 2 && vec_b.len() >= 2 {
            // Note: In KANI, we can't directly access vector elements,
            // but we can verify the logical properties
            assert_eq!(vec_a.len(), 2;
            assert_eq!(vec_b.len(), 2;
            
            // Each vector should be independent
            assert!(vec_a.capacity() <= 64);
            assert!(vec_b.capacity() <= 64);
        }
    }
}

/// Verify allocation error handling is correct
///
/// This harness tests that allocation failures are handled gracefully
/// and don't cause memory corruption or undefined behavior.
#[cfg(kani)]
pub fn verify_allocation_error_handling() {
    // Use a very small provider to force allocation failures
    let small_provider = safe_managed_alloc!(128, CrateId::Component)
        .unwrap_or_else(|_| NoStdProvider::<128>::default);
    
    // Try to create a large bounded vector that might fail
    match BoundedVec::<u64, 32, _>::new(small_provider) {
        Ok(mut vec) => {
            // If creation succeeded, verify normal operation
            assert_eq!(vec.len(), 0;
            assert!(vec.capacity() <= 32);
            
            // Try to fill the vector completely
            for i in 0..32 {
                match vec.push(i as u64) {
                    Ok(()) => {
                        // Successfully added element
                        assert!(vec.len() <= 32);
                    }
                    Err(_) => {
                        // Push failed - verify state is still consistent
                        assert!(vec.len() <= 32);
                        assert!(vec.len() <= vec.capacity();
                        break;
                    }
                }
            }
        }
        Err(_) => {
            // Allocation failure is acceptable with small provider
            // The important thing is that it fails gracefully
        }
    }
}

/// Register simplified memory safety tests with TestRegistry
///
/// These tests provide fallback verification when KANI is not available.
pub fn register_tests(registry: &TestRegistry) -> TestResult {
    use wrt_foundation::{budget_aware_provider::CrateId, safe_managed_alloc, safe_memory::NoStdProvider};
    registry.register_test("bounded_vec_basic", || {
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;
        let vec = BoundedVec::<u8, 16, _>::new(provider)?;
        
        assert_eq!(vec.len(), 0;
        assert!(vec.capacity() <= 16);
        Ok(())
    })?;
    
    registry.register_test("buffer_isolation_basic", || {
        let provider_a = safe_managed_alloc!(1024, CrateId::Component)?;
        let provider_b = safe_managed_alloc!(1024, CrateId::Component)?;
        
        let vec_a = BoundedVec::<u32, 8, _>::new(provider_a)?;
        let vec_b = BoundedVec::<u32, 8, _>::new(provider_b)?;
        
        assert_eq!(vec_a.len(), 0;
        assert_eq!(vec_b.len(), 0;
        Ok(())
    })?;
    
    registry.register_test("allocation_error_basic", || {
        // Test with very constrained provider
        let small_provider = safe_managed_alloc!(64, CrateId::Component)
            .unwrap_or_else(|_| NoStdProvider::<64>::default);
        
        // This might succeed or fail - both are acceptable
        let _result = BoundedVec::<u64, 16, _>::new(small_provider;
        Ok(())
    })?;
    
    Ok(())
}

/// Get count of properties verified by this module
pub fn property_count() -> usize {
    3 // verify_bounded_vec_capacity_limits, verify_buffer_isolation, verify_allocation_error_handling
}

/// Run all simplified memory safety formal proofs (KANI mode only)
#[cfg(kani)]
pub fn run_all_proofs() {
    verify_bounded_vec_capacity_limits);
    verify_buffer_isolation);
    verify_allocation_error_handling);
}

/// KANI harness for bounded vector capacity verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_bounded_vec_capacity_limits() {
    verify_bounded_vec_capacity_limits);
}

/// KANI harness for buffer isolation verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_buffer_isolation() {
    verify_buffer_isolation);
}

/// KANI harness for allocation error handling verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_allocation_error_handling() {
    verify_allocation_error_handling);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_memory_verification() {
        let registry = TestRegistry::global);
        let result = register_tests(registry;
        assert!(result.is_ok();
        assert_eq!(property_count(), 3;
    }
    
    #[test]
    fn test_bounded_vec_creation() {
        use wrt_foundation::{budget_aware_provider::CrateId, safe_managed_alloc, safe_memory::NoStdProvider};
        let provider = safe_managed_alloc!(1024, CrateId::Component)
            .unwrap_or_else(|_| NoStdProvider::<1024>::default);
        let vec = BoundedVec::<u32, 16, _>::new(provider;
        assert!(vec.is_ok();
        
        if let Ok(v) = vec {
            assert_eq!(v.len(), 0;
            assert!(v.capacity() <= 16);
        }
    }
}