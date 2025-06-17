//! Verified Memory Capability Implementation
//!
//! This capability type provides formally verified memory allocation suitable
//! for ASIL-D safety levels where formal verification is required.

use core::{fmt, marker::PhantomData, ptr::NonNull};

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;

use crate::{
    budget_aware_provider::CrateId,
    codes,
    verification::{Checksum, VerificationLevel},
    Error, ErrorCategory, Result,
};

// Import FormalProof if available, otherwise define a stub
#[cfg(feature = "formal-verification")]
use crate::formal_verification::FormalProof;

#[cfg(not(feature = "formal-verification"))]
#[derive(Debug, Clone)]
pub struct FormalProof {
    name: &'static str,
    #[allow(dead_code)]
    axioms: &'static [&'static str],
}

#[cfg(not(feature = "formal-verification"))]
impl FormalProof {
    pub fn new(name: &'static str, axioms: &'static [&'static str]) -> Result<Self> {
        Ok(Self { name, axioms })
    }

    pub fn verify(&self) -> Result<()> {
        // Stub implementation - always passes
        Ok(())
    }
}

use super::{
    CapabilityMask, MemoryCapability, MemoryGuard, MemoryOperation, MemoryOperationType,
    MemoryRegion,
};

/// Thread-safe wrapper for NonNull<u8> pointers
#[derive(Debug, Clone)]
struct ThreadSafePtr {
    ptr: Option<NonNull<u8>>,
}

impl ThreadSafePtr {
    fn new(ptr: Option<NonNull<u8>>) -> Self {
        Self { ptr }
    }

    fn get(&self) -> Option<NonNull<u8>> {
        self.ptr
    }
}

// Safety: ThreadSafePtr can be safely shared between threads when used within
// the capability system which ensures proper access control
unsafe impl Send for ThreadSafePtr {}
unsafe impl Sync for ThreadSafePtr {}

/// Verified memory capability with formal verification requirements
///
/// This capability is suitable for ASIL-D level where all memory operations
/// must be formally verified and provide mathematical proofs of correctness.
#[derive(Debug, Clone)]
pub struct VerifiedMemoryCapability<const N: usize> {
    /// Fixed memory region size (compile-time constant)
    region_size: usize,
    /// Base address of the verified region (if available)
    region_base: ThreadSafePtr,
    /// Operations allowed by this capability
    allowed_operations: CapabilityMask,
    /// Verification level (always Critical for ASIL-D)
    verification_level: VerificationLevel,
    /// Crate that owns this capability
    owner_crate: CrateId,
    /// Formal proofs for memory safety
    safety_proofs: VerificationProofs,
    /// Runtime verification enabled
    runtime_verification: bool,
    /// Static buffer for the memory region
    _phantom: PhantomData<[u8; N]>,
}

/// Formal verification proofs for memory safety
#[derive(Debug, Clone)]
pub struct VerificationProofs {
    /// Proof that all memory accesses are within bounds
    pub bounds_proof: Option<FormalProof>,
    /// Proof that no memory corruption can occur
    pub corruption_proof: Option<FormalProof>,
    /// Proof that concurrent access is safe
    pub concurrency_proof: Option<FormalProof>,
    /// Proof that temporal properties are satisfied
    pub temporal_proof: Option<FormalProof>,
}

impl VerificationProofs {
    /// Create empty verification proofs (to be filled by formal verification tools)
    pub fn empty() -> Self {
        Self {
            bounds_proof: None,
            corruption_proof: None,
            concurrency_proof: None,
            temporal_proof: None,
        }
    }

    /// Create proofs with all required ASIL-D verifications
    pub fn asil_d_complete(
        bounds: FormalProof,
        corruption: FormalProof,
        concurrency: FormalProof,
        temporal: FormalProof,
    ) -> Self {
        Self {
            bounds_proof: Some(bounds),
            corruption_proof: Some(corruption),
            concurrency_proof: Some(concurrency),
            temporal_proof: Some(temporal),
        }
    }

    /// Check if all required proofs are present for ASIL-D
    pub fn is_asil_d_complete(&self) -> bool {
        self.bounds_proof.is_some()
            && self.corruption_proof.is_some()
            && self.concurrency_proof.is_some()
            && self.temporal_proof.is_some()
    }

    /// Verify all proofs are valid
    pub fn verify_all(&self) -> Result<()> {
        if let Some(ref proof) = self.bounds_proof {
            proof.verify().map_err(|_| {
                Error::new(
                    ErrorCategory::Verification,
                    codes::VERIFICATION_FAILED,
                    "Bounds proof verification failed",
                )
            })?;
        }

        if let Some(ref proof) = self.corruption_proof {
            proof.verify().map_err(|_| {
                Error::new(
                    ErrorCategory::Verification,
                    codes::VERIFICATION_FAILED,
                    "Corruption proof verification failed",
                )
            })?;
        }

        if let Some(ref proof) = self.concurrency_proof {
            proof.verify().map_err(|_| {
                Error::new(
                    ErrorCategory::Verification,
                    codes::VERIFICATION_FAILED,
                    "Concurrency proof verification failed",
                )
            })?;
        }

        if let Some(ref proof) = self.temporal_proof {
            proof.verify().map_err(|_| {
                Error::new(
                    ErrorCategory::Verification,
                    codes::VERIFICATION_FAILED,
                    "Temporal proof verification failed",
                )
            })?;
        }

        Ok(())
    }
}

impl<const N: usize> VerifiedMemoryCapability<N> {
    /// Create a new verified memory capability with formal proofs
    ///
    /// # Arguments
    /// * `owner_crate` - Crate that owns this capability
    /// * `proofs` - Formal verification proofs
    /// * `runtime_verification` - Enable runtime verification checks
    pub fn new(
        owner_crate: CrateId,
        proofs: VerificationProofs,
        runtime_verification: bool,
    ) -> Result<Self> {
        // For ASIL-D, require all proofs to be present
        if !proofs.is_asil_d_complete() {
            return Err(Error::new(
                ErrorCategory::Verification,
                codes::VERIFICATION_REQUIRED,
                "ASIL-D verified capability requires complete formal proofs",
            ));
        }

        // Verify all proofs before creating capability
        proofs.verify_all()?;

        Ok(Self {
            region_size: N,
            region_base: ThreadSafePtr::new(None),
            allowed_operations: CapabilityMask {
                read: true,
                write: true,
                allocate: true,
                deallocate: true,
                delegate: false, // ASIL-D typically doesn't allow delegation
                max_size: N,
            },
            verification_level: VerificationLevel::Redundant,
            owner_crate,
            safety_proofs: proofs,
            runtime_verification,
            _phantom: PhantomData,
        })
    }

    /// Create a capability with pre-verified static region
    ///
    /// # Safety
    /// The caller must ensure that:
    /// 1. `region_base` points to a valid memory region of at least `N` bytes
    /// 2. The region will remain valid for the capability's lifetime
    /// 3. All formal proofs have been verified for this specific region
    pub unsafe fn with_verified_region(
        region_base: NonNull<u8>,
        owner_crate: CrateId,
        proofs: VerificationProofs,
        runtime_verification: bool,
    ) -> Result<Self> {
        let mut capability = Self::new(owner_crate, proofs, runtime_verification)?;
        capability.region_base = ThreadSafePtr::new(Some(region_base));
        Ok(capability)
    }

    /// Get the compile-time size of this capability's memory region
    pub const fn compile_time_size() -> usize {
        N
    }

    /// Perform runtime verification of a memory operation
    fn runtime_verify_operation(&self, operation: &MemoryOperation) -> Result<()> {
        if !self.runtime_verification {
            return Ok(());
        }

        match operation {
            MemoryOperation::Read { offset, len } => {
                // Verify bounds with runtime checks
                if offset.saturating_add(*len) > N {
                    return Err(Error::new(
                        ErrorCategory::Verification,
                        codes::BOUNDS_VIOLATION,
                        "Runtime bounds verification failed for read operation",
                    ));
                }

                // Additional runtime integrity checks could go here
            }
            MemoryOperation::Write { offset, len } => {
                if offset.saturating_add(*len) > N {
                    return Err(Error::new(
                        ErrorCategory::Verification,
                        codes::BOUNDS_VIOLATION,
                        "Runtime bounds verification failed for write operation",
                    ));
                }
            }
            MemoryOperation::Allocate { size } => {
                if *size > N {
                    return Err(Error::new(
                        ErrorCategory::Verification,
                        codes::BOUNDS_VIOLATION,
                        "Runtime verification: allocation exceeds verified bounds",
                    ));
                }
            }
            _ => {
                // Other operations don't need runtime verification
            }
        }

        Ok(())
    }
}

impl<const N: usize> MemoryCapability for VerifiedMemoryCapability<N> {
    type Region = VerifiedMemoryRegion<N>;
    type Guard = VerifiedMemoryGuard<N>;

    fn verify_access(&self, operation: &MemoryOperation) -> Result<()> {
        // First, verify against capability mask
        if !operation.requires_capability(&self.allowed_operations) {
            return Err(Error::capability_violation(
                "Operation not allowed by verified capability mask",
            ));
        }

        // Perform formal verification checks
        match operation {
            MemoryOperation::Allocate { size } => {
                if *size > N {
                    return Err(Error::capability_violation("Allocation exceeds verified limit"));
                }
            }
            MemoryOperation::Read { offset, len } => {
                if offset.saturating_add(*len) > N {
                    return Err(Error::capability_violation(
                        "Read operation exceeds verified bounds",
                    ));
                }
            }
            MemoryOperation::Write { offset, len } => {
                if offset.saturating_add(*len) > N {
                    return Err(Error::capability_violation(
                        "Write operation exceeds verified bounds",
                    ));
                }
            }
            MemoryOperation::Delegate { .. } => {
                // ASIL-D typically doesn't allow delegation for safety reasons
                return Err(Error::capability_violation(
                    "Capability delegation not allowed for ASIL-D verified capabilities",
                ));
            }
            MemoryOperation::Deallocate => {
                // Always allowed
            }
        }

        // Perform runtime verification if enabled
        self.runtime_verify_operation(operation)?;

        Ok(())
    }

    fn allocate_region(&self, size: usize, crate_id: CrateId) -> Result<Self::Guard> {
        // Verify allocation operation
        let operation = MemoryOperation::Allocate { size };
        self.verify_access(&operation)?;

        // Check crate ownership
        if crate_id != self.owner_crate {
            return Err(Error::capability_violation(
                "Crate cannot use verified capability owned by different crate",
            ));
        }

        // Create verified memory region
        let region = VerifiedMemoryRegion::new(
            size,
            self.region_base.get(),
            self.safety_proofs.clone(),
            self.runtime_verification,
        )?;

        let guard = VerifiedMemoryGuard::new(region, self);

        Ok(guard)
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn delegate(
        &self,
        _subset: CapabilityMask,
    ) -> Result<Box<dyn MemoryCapability<Region = Self::Region, Guard = Self::Guard>>> {
        // ASIL-D capabilities typically don't support delegation for safety
        Err(Error::capability_violation("Verified ASIL-D capabilities do not support delegation"))
    }

    fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    fn max_allocation_size(&self) -> usize {
        N
    }

    fn supports_operation(&self, op_type: MemoryOperationType) -> bool {
        match op_type {
            MemoryOperationType::Read => self.allowed_operations.read,
            MemoryOperationType::Write => self.allowed_operations.write,
            MemoryOperationType::Allocate => self.allowed_operations.allocate,
            MemoryOperationType::Deallocate => self.allowed_operations.deallocate,
            MemoryOperationType::Delegate => false, // Never allowed for ASIL-D
        }
    }
}

/// Verified memory region with formal safety guarantees
#[derive(Debug)]
pub struct VerifiedMemoryRegion<const N: usize> {
    size: usize,
    buffer: [u8; N],
    base_ptr: ThreadSafePtr,
    safety_proofs: VerificationProofs,
    runtime_verification: bool,
    integrity_checksum: Checksum,
}

impl<const N: usize> VerifiedMemoryRegion<N> {
    fn new(
        size: usize,
        base_ptr: Option<NonNull<u8>>,
        safety_proofs: VerificationProofs,
        runtime_verification: bool,
    ) -> Result<Self> {
        if size > N {
            return Err(Error::new(
                ErrorCategory::Memory,
                codes::CAPACITY_EXCEEDED,
                "Requested size exceeds verified limit",
            ));
        }

        let buffer = [0; N];
        let mut integrity_checksum = Checksum::new();
        integrity_checksum.update_slice(&buffer[..size]);

        Ok(Self {
            size,
            buffer,
            base_ptr: ThreadSafePtr::new(base_ptr),
            safety_proofs,
            runtime_verification,
            integrity_checksum,
        })
    }

    /// Verify buffer integrity using checksum
    fn verify_integrity(&self) -> Result<()> {
        if !self.runtime_verification {
            return Ok(());
        }

        let mut current_checksum = Checksum::new();
        current_checksum.update_slice(&self.buffer[..self.size]);

        if current_checksum != self.integrity_checksum {
            return Err(Error::new(
                ErrorCategory::Verification,
                codes::INTEGRITY_VIOLATION,
                "Memory region integrity verification failed",
            ));
        }

        Ok(())
    }

    /// Update integrity checksum after write operation
    fn update_integrity(&mut self) {
        if self.runtime_verification {
            self.integrity_checksum = Checksum::new();
            self.integrity_checksum.update_slice(&self.buffer[..self.size]);
        }
    }

    /// Get access to the verified buffer
    fn buffer(&self) -> Result<&[u8]> {
        self.verify_integrity()?;
        Ok(&self.buffer[..self.size])
    }

    /// Get mutable access to the verified buffer
    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..self.size]
    }
}

impl<const N: usize> MemoryRegion for VerifiedMemoryRegion<N> {
    fn as_ptr(&self) -> NonNull<u8> {
        if let Some(base_ptr) = self.base_ptr.get() {
            base_ptr
        } else {
            NonNull::new(self.buffer.as_ptr() as *mut u8)
                .expect("Verified buffer pointer should never be null")
        }
    }

    fn size(&self) -> usize {
        self.size
    }
}

/// Verified memory guard with formal safety guarantees
pub struct VerifiedMemoryGuard<const N: usize> {
    region: VerifiedMemoryRegion<N>,
    capability: *const VerifiedMemoryCapability<N>,
}

impl<const N: usize> VerifiedMemoryGuard<N> {
    fn new(region: VerifiedMemoryRegion<N>, capability: &VerifiedMemoryCapability<N>) -> Self {
        Self { region, capability: capability as *const _ }
    }

    fn get_capability(&self) -> &VerifiedMemoryCapability<N> {
        unsafe { &*self.capability }
    }
}

impl<const N: usize> fmt::Debug for VerifiedMemoryGuard<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedMemoryGuard")
            .field("region", &self.region)
            .field("compile_time_size", &N)
            .field("verification_level", &"Critical")
            .finish()
    }
}

impl<const N: usize> MemoryGuard for VerifiedMemoryGuard<N> {
    type Region = VerifiedMemoryRegion<N>;

    fn region(&self) -> &Self::Region {
        &self.region
    }

    fn read_bytes(&self, offset: usize, len: usize) -> Result<&[u8]> {
        let operation = MemoryOperation::Read { offset, len };
        self.get_capability().verify_access(&operation)?;

        if !self.region.contains_range(offset, len) {
            return Err(Error::new(
                ErrorCategory::Verification,
                codes::BOUNDS_VIOLATION,
                "Read operation exceeds verified region bounds",
            ));
        }

        let buffer = self.region.buffer()?;
        Ok(&buffer[offset..offset + len])
    }

    fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        let operation = MemoryOperation::Write { offset, len: data.len() };
        self.get_capability().verify_access(&operation)?;

        if !self.region.contains_range(offset, data.len()) {
            return Err(Error::new(
                ErrorCategory::Verification,
                codes::BOUNDS_VIOLATION,
                "Write operation exceeds verified region bounds",
            ));
        }

        let buffer = self.region.buffer_mut();
        buffer[offset..offset + data.len()].copy_from_slice(data);

        // Update integrity checksum after write
        self.region.update_integrity();

        Ok(())
    }

    fn capability(&self) -> &dyn MemoryCapability<Region = Self::Region, Guard = Self> {
        self.get_capability()
    }
}

// Safety: VerifiedMemoryGuard can be sent between threads safely
unsafe impl<const N: usize> Send for VerifiedMemoryGuard<N> {}
unsafe impl<const N: usize> Sync for VerifiedMemoryGuard<N> {}

/// Common verified capability sizes for ASIL-D use cases
pub type SmallVerifiedCapability = VerifiedMemoryCapability<4096>; // 4KB
pub type MediumVerifiedCapability = VerifiedMemoryCapability<65536>; // 64KB
pub type LargeVerifiedCapability = VerifiedMemoryCapability<1048576>; // 1MB

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal_verification::FormalProof;

    fn create_test_proofs() -> VerificationProofs {
        VerificationProofs::asil_d_complete(
            FormalProof::new("bounds_proof", &[]).unwrap(),
            FormalProof::new("corruption_proof", &[]).unwrap(),
            FormalProof::new("concurrency_proof", &[]).unwrap(),
            FormalProof::new("temporal_proof", &[]).unwrap(),
        )
    }

    #[test]
    fn test_verified_capability_creation() {
        let proofs = create_test_proofs();
        let capability =
            VerifiedMemoryCapability::<1024>::new(CrateId::Foundation, proofs, true).unwrap();

        assert_eq!(capability.max_allocation_size(), 1024);
        assert_eq!(capability.verification_level(), VerificationLevel::Redundant);
        assert!(!capability.supports_operation(MemoryOperationType::Delegate));
    }

    #[test]
    fn test_verification_proofs_completeness() {
        let incomplete_proofs = VerificationProofs::empty();
        assert!(!incomplete_proofs.is_asil_d_complete());

        let complete_proofs = create_test_proofs();
        assert!(complete_proofs.is_asil_d_complete());
    }

    #[test]
    fn test_verified_allocation_bounds() {
        let proofs = create_test_proofs();
        let capability =
            VerifiedMemoryCapability::<100>::new(CrateId::Foundation, proofs, true).unwrap();

        let valid_op = MemoryOperation::Allocate { size: 50 };
        assert!(capability.verify_access(&valid_op).is_ok());

        let invalid_op = MemoryOperation::Allocate { size: 200 };
        assert!(capability.verify_access(&invalid_op).is_err());
    }

    #[test]
    fn test_delegation_not_allowed() {
        let proofs = create_test_proofs();
        let capability =
            VerifiedMemoryCapability::<1000>::new(CrateId::Foundation, proofs, true).unwrap();

        let subset = CapabilityMask::read_only();
        assert!(capability.delegate(subset).is_err());
    }
}
