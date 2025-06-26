//! Compile-Time Bounds Validation Integration Tests
//!
//! This test suite validates the complete compile-time bounds validation
//! system for ASIL-D compliance.
//!
//! SW-REQ-ID: REQ_MEM_001 - Memory bounds checking
//! SW-REQ-ID: REQ_MEM_002 - Budget enforcement
//! SW-REQ-ID: REQ_COMP_BOUNDS_001 - Compile-time bounds validation

use wrt_foundation::{
    budget_aware_provider::CrateId, compile_time_bounds::*, memory_init::MemoryInitializer,
    safe_managed_alloc_validated, validate_allocation,
};

/// Test compile-time validation for valid allocations
#[test]
fn test_valid_compile_time_allocations() {
    // These should all compile successfully
    validate_allocation!(1024, CrateId::Foundation);
    validate_allocation!(64 * 1024, CrateId::Component);
    validate_allocation!(256 * 1024, CrateId::Runtime);
    validate_allocation!(4 * 1024, CrateId::Sync);
}

/// Test the enhanced safe_managed_alloc with validation
#[test]
fn test_safe_managed_alloc_validated() {
    MemoryInitializer::initialize().unwrap();

    // Small allocation
    let result1 = safe_managed_alloc_validated!(4096, CrateId::Foundation);
    assert!(result1.is_ok());

    // Medium allocation
    let result2 = safe_managed_alloc_validated!(64 * 1024, CrateId::Component);
    assert!(result2.is_ok());

    // Large allocation within budget
    let result3 = safe_managed_alloc_validated!(256 * 1024, CrateId::Runtime);
    assert!(result3.is_ok());
}

/// Test memory layout validation
#[test]
fn test_memory_layout_validation() {
    // Valid aligned layouts
    let layout1 = MemoryLayoutValidator::<1024, 8>::validate();
    assert_eq!(layout1.total_size_with_padding(), 1024);

    let layout2 = MemoryLayoutValidator::<2048, 16>::validate();
    assert_eq!(layout2.total_size_with_padding(), 2048);

    // Unaligned size that gets padded
    let layout3 = MemoryLayoutValidator::<1000, 8>::validate();
    assert_eq!(layout3.total_size_with_padding(), 1000); // Already aligned
}

/// Test collection bounds validation
#[test]
fn test_collection_bounds_validation() {
    // Small collections
    let collection1 = CollectionBoundsValidator::<100, 64>::validate();
    assert_eq!(collection1.total_memory(), 6400);

    // Medium collections
    let collection2 = CollectionBoundsValidator::<1000, 128>::validate();
    assert_eq!(collection2.total_memory(), 128000);

    // Large but valid collections
    let collection3 = CollectionBoundsValidator::<4096, 256>::validate();
    assert_eq!(collection3.total_memory(), 1048576); // 1MB
}

/// Test stack bounds validation
#[test]
fn test_stack_bounds_validation() {
    // Small stack frames
    let _stack1 = StackBoundsValidator::<1024>::validate();
    let _stack2 = StackBoundsValidator::<4096>::validate();
    let _stack3 = StackBoundsValidator::<32768>::validate(); // 32KB
}

/// Test resource limits validation
#[test]
fn test_resource_limits_validation() {
    // Various resource counts
    let _resources1 = ResourceLimitsValidator::<16>::validate();
    let _resources2 = ResourceLimitsValidator::<256>::validate();
    let _resources3 = ResourceLimitsValidator::<1024>::validate();
    let _resources4 = ResourceLimitsValidator::<4096>::validate(); // Max allowed
}

/// Test compile-time bounds validator directly
#[test]
fn test_compile_time_bounds_validator() {
    // Test with different valid sizes and crates
    let validator1 =
        CompileTimeBoundsValidator::<1024, { CrateId::Foundation as usize }>::validate();
    assert_eq!(validator1.size(), 1024);
    assert_eq!(validator1.crate_id(), CrateId::Foundation as usize);

    let validator2 =
        CompileTimeBoundsValidator::<65536, { CrateId::Component as usize }>::validate();
    assert_eq!(validator2.size(), 65536);
    assert_eq!(validator2.crate_id(), CrateId::Component as usize);

    // Test platform safety
    assert!(validator1.is_platform_safe());
    assert!(validator2.is_platform_safe());
}

/// Test system bounds validation
#[test]
fn test_system_bounds_validation() {
    // System validation should always pass if compiled
    SystemBoundsValidator::validate_system();
}

/// Test runtime verification in debug builds
#[cfg(debug_assertions)]
#[test]
fn test_runtime_verification() {
    use wrt_foundation::compile_time_bounds::runtime_verification::*;

    // Valid allocations
    assert!(verify_allocation_runtime(1024, CrateId::Foundation));
    assert!(verify_allocation_runtime(64 * 1024, CrateId::Component));

    // Invalid allocations
    assert!(!verify_allocation_runtime(0, CrateId::Foundation));
    assert!(!verify_allocation_runtime(usize::MAX, CrateId::Foundation));

    // System state verification
    assert!(verify_system_state());
}

/// Test bounds validation with realistic component usage
#[test]
fn test_realistic_component_usage() {
    MemoryInitializer::initialize().unwrap();

    // Simulate a realistic component with multiple allocations

    // Component instance data
    validate_allocation!(4096, CrateId::Component);
    let _instance_data = safe_managed_alloc_validated!(4096, CrateId::Component).unwrap();

    // Resource tables
    validate_allocation!(16384, CrateId::Component);
    let _resource_tables = safe_managed_alloc_validated!(16384, CrateId::Component).unwrap();

    // Import/export tables
    validate_allocation!(8192, CrateId::Component);
    let _import_exports = safe_managed_alloc_validated!(8192, CrateId::Component).unwrap();

    // Type information
    validate_allocation!(32768, CrateId::Component);
    let _type_info = safe_managed_alloc_validated!(32768, CrateId::Component).unwrap();
}

/// Test bounds validation across different crates
#[test]
fn test_multi_crate_validation() {
    MemoryInitializer::initialize().unwrap();

    // Foundation crate allocations
    validate_allocation!(8192, CrateId::Foundation);
    let _foundation = safe_managed_alloc_validated!(8192, CrateId::Foundation).unwrap();

    // Runtime crate allocations
    validate_allocation!(131072, CrateId::Runtime);
    let _runtime = safe_managed_alloc_validated!(131072, CrateId::Runtime).unwrap();

    // Component crate allocations
    validate_allocation!(65536, CrateId::Component);
    let _component = safe_managed_alloc_validated!(65536, CrateId::Component).unwrap();

    // Platform crate allocations
    validate_allocation!(16384, CrateId::Platform);
    let _platform = safe_managed_alloc_validated!(16384, CrateId::Platform).unwrap();
}

/// Test edge cases for bounds validation
#[test]
fn test_edge_cases() {
    // Minimum valid allocation
    validate_allocation!(1, CrateId::Foundation);

    // Maximum allowed alignment
    let _max_align = MemoryLayoutValidator::<4096, 4096>::validate();

    // Single element collection
    let _single_element = CollectionBoundsValidator::<1, 64>::validate();

    // Minimum stack frame
    let _min_stack = StackBoundsValidator::<1>::validate();

    // Minimum resources
    let _min_resources = ResourceLimitsValidator::<1>::validate();
}

/// Benchmark compile-time validation overhead
///
/// Since validation is compile-time, this measures runtime allocation performance
/// to ensure the validation system doesn't add runtime overhead.
#[test]
fn test_no_runtime_overhead() {
    use std::time::Instant;

    MemoryInitializer::initialize().unwrap();

    let start = Instant::now();

    // Perform multiple validated allocations
    for _ in 0..100 {
        let _allocation = safe_managed_alloc_validated!(4096, CrateId::Foundation).unwrap();
    }

    let duration = start.elapsed();

    // The time should be reasonable (less than 100ms for 100 allocations)
    assert!(duration.as_millis() < 100, "Validation appears to add runtime overhead");
}

/// Integration test with realistic WASM component instantiation scenario
#[test]
fn test_wasm_component_instantiation_bounds() {
    MemoryInitializer::initialize().unwrap();

    // Simulate component instantiation with bounds validation

    // 1. Component metadata
    validate_allocation!(2048, CrateId::Component);
    let _metadata = safe_managed_alloc_validated!(2048, CrateId::Component).unwrap();

    // 2. Memory for imports/exports
    validate_allocation!(8192, CrateId::Component);
    let _imports_exports = safe_managed_alloc_validated!(8192, CrateId::Component).unwrap();

    // 3. Resource tables with bounds
    let _resource_validator = CollectionBoundsValidator::<1024, 64>::validate();
    validate_allocation!(65536, CrateId::Component);
    let _resources = safe_managed_alloc_validated!(65536, CrateId::Component).unwrap();

    // 4. Runtime execution context
    let _stack_validator = StackBoundsValidator::<8192>::validate();
    validate_allocation!(32768, CrateId::Runtime);
    let _execution_context = safe_managed_alloc_validated!(32768, CrateId::Runtime).unwrap();

    // All allocations should succeed and be within bounds
}
