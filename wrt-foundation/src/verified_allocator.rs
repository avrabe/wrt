//! Verified Memory Allocator with Formal Proofs
//!
//! This module provides a formally verified allocator that maintains
//! strong invariants about memory safety and budget compliance.

#![allow(unused_attributes)] // For formal verification attributes

use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use crate::{
    WrtResult, Error, ErrorCategory, codes,
    budget_aware_provider::CrateId,
    formal_verification::{requires, ensures, invariant},
};

/// Formally verified allocator state
pub struct VerifiedAllocator {
    /// Total memory budget
    total_budget: usize,
    /// Currently allocated memory
    allocated: AtomicUsize,
    /// Allocation enabled flag
    enabled: AtomicBool,
    /// Invariant checker
    #[cfg(debug_assertions)]
    invariant_checker: InvariantChecker,
}

#[cfg(debug_assertions)]
struct InvariantChecker {
    check_frequency: usize,
    check_counter: AtomicUsize,
}

impl VerifiedAllocator {
    /// Create a new verified allocator
    /// 
    /// # Formal Specification
    /// ```text
    /// requires: budget > 0
    /// ensures: self.allocated() == 0
    /// ensures: self.total_budget == budget
    /// ensures: self.enabled == true
    /// ```
    #[requires!(budget > 0)]
    #[ensures!(ret.allocated.load(Ordering::Acquire) == 0)]
    #[ensures!(ret.total_budget == budget)]
    pub const fn new(budget: usize) -> Self {
        Self {
            total_budget: budget,
            allocated: AtomicUsize::new(0),
            enabled: AtomicBool::new(true),
            #[cfg(debug_assertions)]
            invariant_checker: InvariantChecker {
                check_frequency: 100,
                check_counter: AtomicUsize::new(0),
            },
        }
    }
    
    /// Allocate memory with verification
    /// 
    /// # Formal Specification
    /// ```text
    /// requires: self.enabled
    /// requires: size > 0
    /// requires: self.allocated() + size <= self.total_budget
    /// ensures: result.is_ok() => self.allocated() == old(self.allocated()) + size
    /// ensures: result.is_err() => self.allocated() == old(self.allocated())
    /// ```
    #[requires!(self.enabled.load(Ordering::Acquire))]
    #[requires!(size > 0)]
    #[ensures!(ret.is_ok() => self.allocated.load(Ordering::Acquire) == old_allocated + size)]
    pub fn allocate(&self, size: usize) -> WrtResult<VerifiedAllocation> {
        // Check preconditions
        if !self.enabled.load(Ordering::Acquire) {
            return Err(Error::new(
                ErrorCategory::Runtime,
                codes::RUNTIME_ERROR,
                "Allocator is disabled"
            ));
        }
        
        if size == 0 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_PARAMETER,
                "Cannot allocate zero bytes"
            ));
        }
        
        // Atomic allocation with overflow checking
        let mut current = self.allocated.load(Ordering::Acquire);
        loop {
            // Check budget constraint
            let new_total = current.checked_add(size).ok_or_else(|| {
                Error::new(
                    ErrorCategory::Memory,
                    codes::INTEGER_OVERFLOW,
                    "Allocation would overflow"
                )
            })?;
            
            if new_total > self.total_budget {
                return Err(Error::new(
                    ErrorCategory::Memory,
                    codes::CAPACITY_EXCEEDED,
                    "Allocation would exceed budget"
                ));
            }
            
            // Try to update atomically
            match self.allocated.compare_exchange_weak(
                current,
                new_total,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Success - check invariants in debug mode
                    #[cfg(debug_assertions)]
                    self.check_invariants();
                    
                    return Ok(VerifiedAllocation {
                        allocator: self,
                        size,
                        #[cfg(feature = "formal-verification")]
                        allocation_id: crate::formal_verification::ghost::record_allocation(size),
                    });
                }
                Err(actual) => current = actual,
            }
        }
    }
    
    /// Deallocate memory with verification
    /// 
    /// # Formal Specification
    /// ```text
    /// requires: self.allocated() >= size
    /// ensures: self.allocated() == old(self.allocated()) - size
    /// ```
    #[requires!(self.allocated.load(Ordering::Acquire) >= size)]
    #[ensures!(self.allocated.load(Ordering::Acquire) == old_allocated - size)]
    fn deallocate(&self, size: usize) {
        let result = self.allocated.fetch_sub(size, Ordering::AcqRel);
        
        // Check for underflow in debug mode
        debug_assert!(
            result >= size,
            "Deallocation underflow: allocated={}, deallocating={}",
            result, size
        );
        
        #[cfg(debug_assertions)]
        self.check_invariants();
    }
    
    /// Check all invariants
    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        let count = self.invariant_checker.check_counter.fetch_add(1, Ordering::Relaxed);
        if count % self.invariant_checker.check_frequency == 0 {
            invariant!(self.allocated.load(Ordering::Acquire) <= self.total_budget);
            invariant!(self.enabled.load(Ordering::Acquire) || self.allocated.load(Ordering::Acquire) == 0);
        }
    }
    
    /// Disable the allocator (for shutdown)
    /// 
    /// # Formal Specification
    /// ```text
    /// ensures: !self.enabled
    /// ensures: self.allocated() == old(self.allocated())
    /// ```
    #[ensures!(!self.enabled.load(Ordering::Acquire))]
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }
}

/// A verified allocation handle
pub struct VerifiedAllocation<'a> {
    allocator: &'a VerifiedAllocator,
    size: usize,
    #[cfg(feature = "formal-verification")]
    allocation_id: u64,
}

impl<'a> Drop for VerifiedAllocation<'a> {
    /// Automatically deallocate on drop
    /// 
    /// # Formal Specification
    /// ```text
    /// ensures: allocator.allocated() == old(allocator.allocated()) - self.size
    /// ```
    fn drop(&mut self) {
        self.allocator.deallocate(self.size);
        
        #[cfg(feature = "formal-verification")]
        crate::formal_verification::ghost::record_deallocation(self.allocation_id);
    }
}

/// Global verified allocator instances
pub mod global_allocators {
    use super::*;
    
    /// System-wide verified allocator
    pub static SYSTEM_ALLOCATOR: VerifiedAllocator = VerifiedAllocator::new(16_777_216); // 16MB
    
    /// Per-crate verified allocators
    pub static CRATE_ALLOCATORS: [VerifiedAllocator; 16] = [
        VerifiedAllocator::new(1_048_576),  // Foundation - 1MB
        VerifiedAllocator::new(2_097_152),  // Component - 2MB
        VerifiedAllocator::new(4_194_304),  // Runtime - 4MB
        VerifiedAllocator::new(1_048_576),  // Decoder - 1MB
        VerifiedAllocator::new(2_097_152),  // Host - 2MB
        VerifiedAllocator::new(524_288),    // Debug - 512KB
        VerifiedAllocator::new(1_048_576),  // Platform - 1MB
        VerifiedAllocator::new(524_288),    // Instructions - 512KB
        VerifiedAllocator::new(524_288),    // Format - 512KB
        VerifiedAllocator::new(524_288),    // Intercept - 512KB
        VerifiedAllocator::new(262_144),    // Sync - 256KB
        VerifiedAllocator::new(262_144),    // Math - 256KB
        VerifiedAllocator::new(262_144),    // Logging - 256KB
        VerifiedAllocator::new(131_072),    // Panic - 128KB
        VerifiedAllocator::new(262_144),    // TestRegistry - 256KB
        VerifiedAllocator::new(262_144),    // VerificationTool - 256KB
    ];
    
    /// Get allocator for a specific crate
    pub fn get_crate_allocator(crate_id: CrateId) -> &'static VerifiedAllocator {
        &CRATE_ALLOCATORS[crate_id as usize]
    }
}

/// Verification properties that can be checked
#[cfg(feature = "formal-verification")]
pub mod properties {
    use super::*;
    
    /// Property: No allocation can exceed the total budget
    #[cfg(kani)]
    #[kani::proof]
    fn verify_budget_safety() {
        let budget = kani::any_where(|b: &usize| *b > 0 && *b < 1_000_000);
        let allocator = VerifiedAllocator::new(budget);
        
        let size = kani::any_where(|s: &usize| *s > 0);
        
        let result = allocator.allocate(size);
        
        // If allocation succeeds, we must be within budget
        if result.is_ok() {
            assert!(allocator.allocated.load(Ordering::Acquire) <= budget);
        }
    }
    
    /// Property: Deallocation always reduces allocated amount
    #[cfg(kani)]
    #[kani::proof]
    fn verify_deallocation_reduces() {
        let allocator = VerifiedAllocator::new(1_000_000);
        let size = kani::any_where(|s: &usize| *s > 0 && *s < 1_000_000);
        
        if let Ok(allocation) = allocator.allocate(size) {
            let before = allocator.allocated.load(Ordering::Acquire);
            drop(allocation);
            let after = allocator.allocated.load(Ordering::Acquire);
            assert!(after < before);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_allocation() {
        let allocator = VerifiedAllocator::new(1024);
        
        let alloc1 = allocator.allocate(512).unwrap();
        assert_eq!(allocator.allocated.load(Ordering::Acquire), 512);
        
        let alloc2 = allocator.allocate(256).unwrap();
        assert_eq!(allocator.allocated.load(Ordering::Acquire), 768);
        
        drop(alloc1);
        assert_eq!(allocator.allocated.load(Ordering::Acquire), 256);
        
        drop(alloc2);
        assert_eq!(allocator.allocated.load(Ordering::Acquire), 0);
    }
    
    #[test]
    fn test_budget_enforcement() {
        let allocator = VerifiedAllocator::new(1024);
        
        let _alloc1 = allocator.allocate(1024).unwrap();
        let result = allocator.allocate(1);
        assert!(result.is_err());
    }
}