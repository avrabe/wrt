//! Compile-Time Memory Bounds Validation for ASIL-D Compliance
//!
//! This module provides compile-time validation mechanisms that ensure
//! all memory allocations are statically verified and bounded.
//!
//! SW-REQ-ID: REQ_MEM_001 - Memory bounds checking
//! SW-REQ-ID: REQ_MEM_002 - Budget enforcement
//! SW-REQ-ID: REQ_COMP_BOUNDS_001 - Compile-time bounds validation

use crate::{
    budget_aware_provider::CrateId,
    budget_verification::{calculate_total_budget, CRATE_BUDGETS, TOTAL_MEMORY_BUDGET},
};

/// Maximum allocation size for any single allocation (16MB)
pub const MAX_SINGLE_ALLOCATION: usize = 16 * 1024 * 1024;

/// Maximum number of allocations per crate
pub const MAX_ALLOCATIONS_PER_CRATE: usize = 1024;

/// Compile-time memory bounds validator
///
/// This struct uses const generics to enforce memory bounds at compile time.
/// All bounds checking is performed during compilation, ensuring zero runtime overhead.
pub struct CompileTimeBoundsValidator<const SIZE: usize, const CRATE: usize>;

impl<const SIZE: usize, const CRATE: usize> CompileTimeBoundsValidator<SIZE, CRATE> {
    /// Validate bounds at compile time
    ///
    /// This const function performs all validation during compilation.
    /// If any constraint is violated, compilation will fail.
    pub const fn validate() -> Self {
        // Validate size constraints
        assert!(SIZE > 0, "Allocation size must be greater than zero");
        assert!(SIZE <= MAX_SINGLE_ALLOCATION, "Allocation size exceeds maximum allowed");

        // Validate crate constraints
        assert!(CRATE < CRATE_BUDGETS.len(), "Invalid crate ID");

        // Validate allocation fits within crate budget
        assert!(SIZE <= CRATE_BUDGETS[CRATE], "Allocation exceeds crate budget");

        // Validate total system budget
        let total_budget = calculate_total_budget);
        assert!(total_budget <= TOTAL_MEMORY_BUDGET, "Total budget exceeds system memory");

        Self
    }

    /// Get the validated size at compile time
    pub const fn size() -> usize {
        SIZE
    }

    /// Get the validated crate ID at compile time
    pub const fn crate_id() -> usize {
        CRATE
    }

    /// Check if this allocation is safe for the target platform
    pub const fn is_platform_safe() -> bool {
        // Additional platform-specific checks
        #[cfg(all(target_arch = "arm", target_os = "none"))]
        {
            // Embedded ARM: stricter limits
            SIZE <= 1024 * 1024 // Max 1MB per allocation
        }

        #[cfg(target_os = "qnx")]
        {
            // QNX: moderate limits
            SIZE <= 4 * 1024 * 1024 // Max 4MB per allocation
        }

        #[cfg(not(any(all(target_arch = "arm", target_os = "none"), target_os = "qnx")))]
        {
            // Other platforms: use global limit
            SIZE <= MAX_SINGLE_ALLOCATION
        }
    }
}

/// Compile-time allocation validator macro
///
/// This macro performs comprehensive compile-time validation of memory allocations.
///
/// # Usage
///
/// ```rust
/// // This will compile successfully
/// validate_allocation!(1024, CrateId::Component;
///
/// // This will cause a compile error
/// validate_allocation!(999_999_999, CrateId::Component); // Too large
/// ```
#[macro_export]
macro_rules! validate_allocation {
    ($size:expr, $crate_id:expr) => {{
        const _: () = {
            let validator = $crate::compile_time_bounds::CompileTimeBoundsValidator::<
                $size,
                { $crate_id as usize },
            >::validate);

            // Additional compile-time checks
            assert!(validator.is_platform_safe(), "Allocation not safe for target platform");
        };
    }};
}

/// Compile-time memory layout validator
///
/// Validates memory layouts and alignment requirements at compile time.
pub struct MemoryLayoutValidator<const SIZE: usize, const ALIGN: usize>;

impl<const SIZE: usize, const ALIGN: usize> MemoryLayoutValidator<SIZE, ALIGN> {
    /// Validate memory layout at compile time
    pub const fn validate() -> Self {
        // Validate size
        assert!(SIZE > 0, "Layout size must be greater than zero");
        assert!(SIZE <= MAX_SINGLE_ALLOCATION, "Layout size exceeds maximum");

        // Validate alignment
        assert!(ALIGN > 0, "Alignment must be greater than zero");
        assert!(ALIGN.is_power_of_two(), "Alignment must be a power of two");
        assert!(ALIGN <= 4096, "Alignment exceeds maximum (4KB)");

        // Validate size is aligned
        assert!(SIZE % ALIGN == 0, "Size must be aligned to alignment boundary");

        Self
    }

    /// Calculate total size with padding
    pub const fn total_size_with_padding() -> usize {
        // Round up to alignment boundary
        (SIZE + ALIGN - 1) & !(ALIGN - 1)
    }
}

/// Collection bounds validator for BoundedVec and similar types
pub struct CollectionBoundsValidator<const CAPACITY: usize, const ELEMENT_SIZE: usize>;

impl<const CAPACITY: usize, const ELEMENT_SIZE: usize>
    CollectionBoundsValidator<CAPACITY, ELEMENT_SIZE>
{
    /// Validate collection bounds at compile time
    pub const fn validate() -> Self {
        // Validate capacity
        assert!(CAPACITY > 0, "Collection capacity must be greater than zero");
        assert!(CAPACITY <= 1_000_000, "Collection capacity exceeds safety limit");

        // Validate element size
        assert!(ELEMENT_SIZE > 0, "Element size must be greater than zero");
        assert!(ELEMENT_SIZE <= 65536, "Element size exceeds safety limit (64KB)");

        // Validate total memory usage
        let total_memory = CAPACITY * ELEMENT_SIZE;
        assert!(total_memory <= MAX_SINGLE_ALLOCATION, "Collection total memory exceeds limit");

        Self
    }

    /// Get total memory usage for this collection
    pub const fn total_memory() -> usize {
        CAPACITY * ELEMENT_SIZE
    }
}

/// Stack frame bounds validator for function calls
pub struct StackBoundsValidator<const FRAME_SIZE: usize>;

impl<const FRAME_SIZE: usize> StackBoundsValidator<FRAME_SIZE> {
    /// Maximum allowed stack frame size (64KB)
    pub const MAX_FRAME_SIZE: usize = 64 * 1024;

    /// Validate stack frame size at compile time
    pub const fn validate() -> Self {
        assert!(FRAME_SIZE > 0, "Stack frame size must be greater than zero");
        assert!(FRAME_SIZE <= Self::MAX_FRAME_SIZE, "Stack frame size exceeds maximum");

        Self
    }
}

/// Resource limits validator
pub struct ResourceLimitsValidator<const RESOURCE_COUNT: usize>;

impl<const RESOURCE_COUNT: usize> ResourceLimitsValidator<RESOURCE_COUNT> {
    /// Maximum resources per component
    pub const MAX_RESOURCES: usize = 4096;

    /// Validate resource limits at compile time
    pub const fn validate() -> Self {
        assert!(RESOURCE_COUNT <= Self::MAX_RESOURCES, "Resource count exceeds maximum");

        Self
    }
}

/// Global system bounds validator
///
/// Validates overall system constraints that must hold across all components.
pub struct SystemBoundsValidator;

impl SystemBoundsValidator {
    /// Validate entire system bounds at compile time
    pub const fn validate_system() {
        // Check total budget doesn't exceed system memory
        let total_budget = calculate_total_budget);
        assert!(total_budget <= TOTAL_MEMORY_BUDGET, "System budget validation failed");

        // Check crate budget distribution is reasonable
        let mut max_crate_budget = 0;
        let mut i = 0;
        while i < CRATE_BUDGETS.len() {
            if CRATE_BUDGETS[i] > max_crate_budget {
                max_crate_budget = CRATE_BUDGETS[i];
            }
            i += 1;
        }

        // No single crate should use more than 50% of total memory
        assert!(max_crate_budget <= TOTAL_MEMORY_BUDGET / 2, "Single crate budget too large");
    }
}

// Force system validation at compile time
const _SYSTEM_VALIDATION: () = SystemBoundsValidator::validate_system);

/// Enhanced safe_managed_alloc macro with compile-time bounds validation
///
/// This macro extends the existing safe_managed_alloc with comprehensive
/// compile-time validation.
#[macro_export]
macro_rules! safe_managed_alloc_validated {
    ($size:expr, $crate_id:expr) => {{
        // Perform compile-time validation
        $crate::validate_allocation!($size, $crate_id;

        // Proceed with normal allocation
        $crate::safe_managed_alloc!($size, $crate_id)
    }};
}

/// Compile-time test cases to verify the validation system
#[cfg(test)]
mod compile_time_tests {
    use super::*;

    // Test valid allocation
    const _TEST_VALID: CompileTimeBoundsValidator<1024, { CrateId::Component as usize }> =
        CompileTimeBoundsValidator::validate);

    // Test memory layout validation
    const _TEST_LAYOUT: MemoryLayoutValidator<1024, 8> = MemoryLayoutValidator::validate);

    // Test collection bounds
    const _TEST_COLLECTION: CollectionBoundsValidator<100, 64> =
        CollectionBoundsValidator::validate);

    // Test stack bounds
    const _TEST_STACK: StackBoundsValidator<4096> = StackBoundsValidator::validate);

    // Test resource limits
    const _TEST_RESOURCES: ResourceLimitsValidator<256> = ResourceLimitsValidator::validate);
}

/// Runtime verification functions for additional safety
///
/// While the primary validation is compile-time, these functions provide
/// additional runtime checks for debug builds.
#[cfg(debug_assertions)]
pub mod runtime_verification {
    use super::*;

    /// Verify allocation at runtime (debug builds only)
    pub fn verify_allocation_runtime(size: usize, crate_id: CrateId) -> bool {
        if size == 0 || size > MAX_SINGLE_ALLOCATION {
            return false;
        }

        let crate_index = crate_id as usize;
        if crate_index >= CRATE_BUDGETS.len() {
            return false;
        }

        size <= CRATE_BUDGETS[crate_index]
    }

    /// Verify system state at runtime (debug builds only)
    pub fn verify_system_state() -> bool {
        calculate_total_budget() <= TOTAL_MEMORY_BUDGET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_time_validation_works() {
        // These should compile without issues
        validate_allocation!(1024, CrateId::Component;
        validate_allocation!({ 64 * 1024 }, CrateId::Runtime;
    }

    #[test]
    fn test_safe_managed_alloc_validated() {
        crate::memory_init::MemoryInitializer::initialize().unwrap();

        let result = safe_managed_alloc_validated!(4096, CrateId::Foundation;
        assert!(result.is_ok();
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_runtime_verification() {
        use runtime_verification::*;

        assert!(verify_allocation_runtime(1024, CrateId::Component);
        assert!(!verify_allocation_runtime(0, CrateId::Component);
        assert!(!verify_allocation_runtime(usize::MAX, CrateId::Component);

        assert!(verify_system_state();
    }

    #[test]
    fn test_memory_layout_validation() {
        let validator = MemoryLayoutValidator::<1024, 8>::validate);
        assert_eq!(validator.total_size_with_padding(), 1024;
    }

    #[test]
    fn test_collection_bounds_validation() {
        let validator = CollectionBoundsValidator::<100, 64>::validate);
        assert_eq!(validator.total_memory(), 6400;
    }
}
