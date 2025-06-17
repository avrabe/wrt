//! Dynamic Memory Capability Implementation
//!
//! This capability type allows runtime memory allocation suitable for QM and ASIL-A
//! safety levels where dynamic allocation is permitted.

use core::{
    fmt,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;

use crate::{
    budget_aware_provider::CrateId, codes, verification::VerificationLevel, BudgetProvider, Error,
    ErrorCategory, Result,
};

use super::{
    CapabilityMask, MemoryCapability, MemoryGuard, MemoryOperation, MemoryOperationType,
    MemoryRegion,
};

/// Dynamic memory capability that allows runtime allocation
///
/// This capability is suitable for QM and ASIL-A levels where runtime
/// allocation is acceptable. It enforces budget limits and tracks allocations.
#[derive(Debug)]
pub struct DynamicMemoryCapability {
    /// Maximum total allocation allowed
    max_allocation: usize,
    /// Currently allocated bytes (tracked atomically)
    current_allocated: AtomicUsize,
    /// Operations allowed by this capability
    allowed_operations: CapabilityMask,
    /// Verification level for memory operations
    verification_level: VerificationLevel,
    /// Crate that owns this capability
    owner_crate: CrateId,
}

impl DynamicMemoryCapability {
    /// Create a new dynamic memory capability
    ///
    /// # Arguments
    /// * `max_allocation` - Maximum bytes that can be allocated
    /// * `owner_crate` - Crate that owns this capability
    /// * `verification_level` - Level of verification required
    pub fn new(
        max_allocation: usize,
        owner_crate: CrateId,
        verification_level: VerificationLevel,
    ) -> Self {
        Self {
            max_allocation,
            current_allocated: AtomicUsize::new(0),
            allowed_operations: CapabilityMask::all(),
            verification_level,
            owner_crate,
        }
    }

    /// Create a capability with specific operation mask
    pub fn with_operations(
        max_allocation: usize,
        owner_crate: CrateId,
        verification_level: VerificationLevel,
        operations: CapabilityMask,
    ) -> Self {
        Self {
            max_allocation,
            current_allocated: AtomicUsize::new(0),
            allowed_operations: operations,
            verification_level,
            owner_crate,
        }
    }

    /// Get current allocation usage
    pub fn current_usage(&self) -> usize {
        self.current_allocated.load(Ordering::Acquire)
    }

    /// Get remaining allocation capacity
    pub fn remaining_capacity(&self) -> usize {
        self.max_allocation.saturating_sub(self.current_usage())
    }

    /// Check if allocation of given size would exceed limits
    fn would_exceed_limit(&self, size: usize) -> bool {
        self.current_usage().saturating_add(size) > self.max_allocation
    }

    /// Record an allocation (internal use)
    fn record_allocation(&self, size: usize) -> Result<()> {
        let old_value = self.current_allocated.fetch_add(size, Ordering::AcqRel);
        if old_value.saturating_add(size) > self.max_allocation {
            // Rollback the allocation
            self.current_allocated.fetch_sub(size, Ordering::AcqRel);
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::INSUFFICIENT_MEMORY,
                "Allocation would exceed capability limit",
            ));
        }
        Ok(())
    }

    /// Record a deallocation (internal use)
    fn record_deallocation(&self, size: usize) {
        self.current_allocated.fetch_sub(size, Ordering::AcqRel);
    }
}

impl MemoryCapability for DynamicMemoryCapability {
    type Region = DynamicMemoryRegion;
    type Guard = DynamicMemoryGuard;

    fn verify_access(&self, operation: &MemoryOperation) -> Result<()> {
        // Check if operation type is allowed
        if !operation.requires_capability(&self.allowed_operations) {
            return Err(super::capability_errors::capability_violation(
                "Operation not allowed by capability mask",
            ));
        }

        // Additional checks based on operation type
        match operation {
            MemoryOperation::Allocate { size } => {
                if self.would_exceed_limit(*size) {
                    return Err(super::capability_errors::capability_violation(
                        "Allocation would exceed capability limit",
                    ));
                }
            }
            MemoryOperation::Read { offset, len } => {
                // Basic bounds checking will be done by the guard
                if *len == 0 {
                    return Err(super::capability_errors::capability_violation(
                        "Zero-length read not allowed",
                    ));
                }
            }
            MemoryOperation::Write { offset, len } => {
                if *len == 0 {
                    return Err(super::capability_errors::capability_violation(
                        "Zero-length write not allowed",
                    ));
                }
            }
            MemoryOperation::Delegate { subset } => {
                // Check if delegation is allowed and subset is valid
                if !self.allowed_operations.delegate {
                    return Err(super::capability_errors::capability_violation(
                        "Capability delegation not allowed",
                    ));
                }
                if subset.max_size > self.max_allocation {
                    return Err(super::capability_errors::capability_violation(
                        "Delegated size exceeds capability limit",
                    ));
                }
            }
            MemoryOperation::Deallocate => {
                // Always allowed if deallocate permission exists
            }
        }

        Ok(())
    }

    fn allocate_region(&self, size: usize, crate_id: CrateId) -> Result<Self::Guard> {
        // Verify allocation operation
        let operation = MemoryOperation::Allocate { size };
        self.verify_access(&operation)?;

        // Check if this crate is allowed to use this capability
        if crate_id != self.owner_crate {
            return Err(super::capability_errors::capability_violation(
                "Crate cannot use capability owned by different crate",
            ));
        }

        // Record the allocation before creating the provider
        self.record_allocation(size)?;

        // Create the actual memory provider
        // Create raw provider directly for internal use within the capability
        let provider = if size <= 65536 {
            // Create raw provider without capability wrapping (we are the capability!)
            crate::safe_memory::NoStdProvider::<65536>::new()
        } else {
            // For larger allocations, we'd need a different strategy
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::CAPACITY_EXCEEDED,
                "Dynamic allocation size exceeds maximum provider capacity",
            ));
        };

        let region = DynamicMemoryRegion::new(size, provider)?;
        let guard = DynamicMemoryGuard::new(region, self, size);

        Ok(guard)
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn delegate(
        &self,
        subset: CapabilityMask,
    ) -> Result<Box<dyn MemoryCapability<Region = Self::Region, Guard = Self::Guard>>> {
        let operation = MemoryOperation::Delegate { subset };
        self.verify_access(&operation)?;

        // Create a delegated capability with restricted permissions
        let delegated_max = if subset.max_size == 0 {
            self.remaining_capacity()
        } else {
            core::cmp::min(subset.max_size, self.remaining_capacity())
        };

        let delegated = DynamicMemoryCapability {
            max_allocation: delegated_max,
            current_allocated: AtomicUsize::new(0),
            allowed_operations: self.allowed_operations.intersect(&subset),
            verification_level: self.verification_level,
            owner_crate: self.owner_crate,
        };

        Ok(Box::new(delegated))
    }

    fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    fn max_allocation_size(&self) -> usize {
        self.max_allocation
    }

    fn supports_operation(&self, op_type: MemoryOperationType) -> bool {
        match op_type {
            MemoryOperationType::Read => self.allowed_operations.read,
            MemoryOperationType::Write => self.allowed_operations.write,
            MemoryOperationType::Allocate => self.allowed_operations.allocate,
            MemoryOperationType::Deallocate => self.allowed_operations.deallocate,
            MemoryOperationType::Delegate => self.allowed_operations.delegate,
        }
    }
}

/// Dynamic memory region backed by NoStdProvider
#[derive(Debug)]
pub struct DynamicMemoryRegion {
    size: usize,
    provider: crate::safe_memory::NoStdProvider<65536>, // Fixed size for now
}

impl DynamicMemoryRegion {
    fn new(size: usize, provider: crate::safe_memory::NoStdProvider<65536>) -> Result<Self> {
        Ok(Self { size, provider })
    }
}

impl MemoryRegion for DynamicMemoryRegion {
    fn as_ptr(&self) -> NonNull<u8> {
        // For now, return a null pointer - this would need proper implementation
        // with actual memory allocation
        NonNull::dangling()
    }

    fn size(&self) -> usize {
        self.size
    }
}

/// Dynamic memory guard that protects access to dynamic memory regions
pub struct DynamicMemoryGuard {
    region: DynamicMemoryRegion,
    capability: *const DynamicMemoryCapability, // Raw pointer to avoid lifetime issues
    allocated_size: usize,
}

impl DynamicMemoryGuard {
    fn new(
        region: DynamicMemoryRegion,
        capability: &DynamicMemoryCapability,
        allocated_size: usize,
    ) -> Self {
        Self { region, capability: capability as *const _, allocated_size }
    }

    fn get_capability(&self) -> &DynamicMemoryCapability {
        // Safety: The capability pointer is valid for the lifetime of this guard
        // This is a temporary solution - proper lifetime management would be better
        unsafe { &*self.capability }
    }
}

impl fmt::Debug for DynamicMemoryGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicMemoryGuard")
            .field("region", &self.region)
            .field("allocated_size", &self.allocated_size)
            .finish()
    }
}

impl MemoryGuard for DynamicMemoryGuard {
    type Region = DynamicMemoryRegion;

    fn region(&self) -> &Self::Region {
        &self.region
    }

    fn read_bytes(&self, offset: usize, len: usize) -> Result<&[u8]> {
        let operation = MemoryOperation::Read { offset, len };
        self.get_capability().verify_access(&operation)?;

        if !self.region.contains_range(offset, len) {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::OUT_OF_BOUNDS,
                "Read operation exceeds region bounds",
            ));
        }

        // This would need actual implementation with the provider
        // For now, return an empty slice
        Ok(&[])
    }

    fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        let operation = MemoryOperation::Write { offset, len: data.len() };
        self.get_capability().verify_access(&operation)?;

        if !self.region.contains_range(offset, data.len()) {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::OUT_OF_BOUNDS,
                "Write operation exceeds region bounds",
            ));
        }

        // This would need actual implementation with the provider
        Ok(())
    }

    fn capability(&self) -> &dyn MemoryCapability<Region = Self::Region, Guard = Self> {
        self.get_capability()
    }
}

impl Drop for DynamicMemoryGuard {
    fn drop(&mut self) {
        // Record deallocation when guard is dropped
        self.get_capability().record_deallocation(self.allocated_size);
    }
}

// Safety: DynamicMemoryGuard can be sent between threads safely
unsafe impl Send for DynamicMemoryGuard {}
unsafe impl Sync for DynamicMemoryGuard {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_capability_creation() {
        let capability =
            DynamicMemoryCapability::new(1024, CrateId::Foundation, VerificationLevel::Standard);

        assert_eq!(capability.max_allocation_size(), 1024);
        assert_eq!(capability.current_usage(), 0);
        assert_eq!(capability.remaining_capacity(), 1024);
    }

    #[test]
    fn test_allocation_verification() {
        let capability =
            DynamicMemoryCapability::new(100, CrateId::Foundation, VerificationLevel::Standard);

        let valid_op = MemoryOperation::Allocate { size: 50 };
        assert!(capability.verify_access(&valid_op).is_ok());

        let invalid_op = MemoryOperation::Allocate { size: 200 };
        assert!(capability.verify_access(&invalid_op).is_err());
    }

    #[test]
    fn test_capability_delegation() {
        let capability =
            DynamicMemoryCapability::new(1000, CrateId::Foundation, VerificationLevel::Standard);

        let subset = CapabilityMask {
            read: true,
            write: false,
            allocate: true,
            deallocate: false,
            delegate: false,
            max_size: 500,
        };

        let delegated = capability.delegate(subset).unwrap();
        assert_eq!(delegated.max_allocation_size(), 500);
        assert!(!delegated.supports_operation(MemoryOperationType::Write));
    }
}
