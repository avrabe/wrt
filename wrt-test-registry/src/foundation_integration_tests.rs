//! Integration tests for WRT Foundation unified types
//!
//! This module provides test cases that verify the integration and functionality
//! of the new unified type system, memory providers, and safety primitives
//! from the WRT foundation.

use crate::prelude::*;

/// Test unified type system functionality
pub fn test_unified_types(config: &TestConfig) -> TestResult {
    // Test type system validation
    assert_test!(DefaultTypes::validate_configuration());
    assert_test!(EmbeddedTypes::validate_configuration());
    assert_test!(DesktopTypes::validate_configuration());
    assert_test!(SafetyCriticalTypes::validate_configuration());

    // Test platform capacities
    let default_caps = DefaultTypes::capacities();
    assert_eq_test!(default_caps.small_capacity, 64);
    assert_eq_test!(default_caps.medium_capacity, 1024);
    assert_eq_test!(default_caps.large_capacity, 65536);

    let embedded_caps = EmbeddedTypes::capacities();
    assert_eq_test!(embedded_caps.small_capacity, 16);
    assert_eq_test!(embedded_caps.medium_capacity, 128);
    assert_eq_test!(embedded_caps.large_capacity, 1024);

    Ok(())
}

/// Test memory provider hierarchy
pub fn test_memory_providers(config: &TestConfig) -> TestResult {
    // Test small provider
    let mut small_provider = SmallProvider::new();
    assert_eq_test!(small_provider.total_memory(), 8192);
    assert_eq_test!(small_provider.available_memory(), 8192);
    assert_test!(small_provider.is_empty());

    let memory = small_provider.allocate(1024)
        .map_err(|e| format!("Failed to allocate from small provider: {:?}", e))?;
    
    assert_eq_test!(memory.len(), 1024);
    assert_test!(!small_provider.is_empty());
    assert_test!(small_provider.available_memory() < 8192);

    // Test medium provider
    let mut medium_provider = MediumProvider::new();
    assert_eq_test!(medium_provider.total_memory(), 65536);
    assert_test!(medium_provider.can_allocate(32768));
    assert_test!(!medium_provider.can_allocate(100000));

    // Test large provider
    let large_provider = LargeProvider::new();
    assert_eq_test!(large_provider.total_memory(), 1048576);
    assert_test!(large_provider.can_allocate(1000000));

    // Test memory provider factory
    let factory_small = MemoryProviderFactory::create_small();
    assert_eq_test!(factory_small.total_memory(), 8192);

    let factory_medium = MemoryProviderFactory::create_medium();
    assert_eq_test!(factory_medium.total_memory(), 65536);

    let factory_large = MemoryProviderFactory::create_large();
    assert_eq_test!(factory_large.total_memory(), 1048576);

    #[cfg(feature = "std")]
    {
        let std_provider = MemoryProviderFactory::create_std();
        assert_eq_test!(std_provider.total_memory(), usize::MAX);
        assert_test!(std_provider.can_allocate(1024 * 1024));
    }

    Ok(())
}

/// Test safety system integration
pub fn test_safety_system(config: &TestConfig) -> TestResult {
    // Test ASIL level properties
    assert_test!(AsilLevel::QM < AsilLevel::ASIL_A);
    assert_test!(AsilLevel::ASIL_A < AsilLevel::ASIL_B);
    assert_test!(AsilLevel::ASIL_B < AsilLevel::ASIL_C);
    assert_test!(AsilLevel::ASIL_C < AsilLevel::ASIL_D);

    // Test ASIL level requirements
    assert_test!(!AsilLevel::QM.requires_memory_protection());
    assert_test!(!AsilLevel::ASIL_A.requires_memory_protection());
    assert_test!(!AsilLevel::ASIL_B.requires_memory_protection());
    assert_test!(AsilLevel::ASIL_C.requires_memory_protection());
    assert_test!(AsilLevel::ASIL_D.requires_memory_protection());

    assert_test!(!AsilLevel::QM.requires_cfi());
    assert_test!(!AsilLevel::ASIL_A.requires_cfi());
    assert_test!(!AsilLevel::ASIL_B.requires_cfi());
    assert_test!(AsilLevel::ASIL_C.requires_cfi());
    assert_test!(AsilLevel::ASIL_D.requires_cfi());

    assert_test!(!AsilLevel::ASIL_C.requires_redundancy());
    assert_test!(AsilLevel::ASIL_D.requires_redundancy());

    // Test safety context
    let safety_ctx = SafetyContext::new(AsilLevel::ASIL_B);
    assert_eq_test!(safety_ctx.compile_time_asil, AsilLevel::ASIL_B);
    assert_eq_test!(safety_ctx.effective_asil(), AsilLevel::ASIL_B);
    assert_eq_test!(safety_ctx.violation_count(), 0);

    // Test safety context upgrade
    safety_ctx.upgrade_runtime_asil(AsilLevel::ASIL_D)
        .map_err(|e| format!("Failed to upgrade ASIL level: {:?}", e))?;
    assert_eq_test!(safety_ctx.effective_asil(), AsilLevel::ASIL_D);

    // Should not be able to downgrade below compile-time level
    let downgrade_result = safety_ctx.upgrade_runtime_asil(AsilLevel::ASIL_A);
    assert_test!(downgrade_result.is_err());

    // Test violation recording
    let initial_violations = safety_ctx.violation_count();
    let count = safety_ctx.record_violation();
    assert_eq_test!(count, initial_violations + 1);
    assert_eq_test!(safety_ctx.violation_count(), initial_violations + 1);

    // Test verification frequency
    let qm_ctx = SafetyContext::new(AsilLevel::QM);
    assert_eq_test!(qm_ctx.effective_asil().verification_frequency(), 0);

    let asil_d_ctx = SafetyContext::new(AsilLevel::ASIL_D);
    assert_eq_test!(asil_d_ctx.effective_asil().verification_frequency(), 1);

    Ok(())
}

/// Test platform capacity validation
pub fn test_platform_capacities(config: &TestConfig) -> TestResult {
    // Test default platform capacities
    let default_caps = PlatformCapacities::default();
    assert_test!(default_caps.validate());

    // Test embedded platform capacities
    let embedded_caps = PlatformCapacities::embedded();
    assert_test!(embedded_caps.validate());
    assert_test!(embedded_caps.small_capacity < default_caps.small_capacity);
    assert_test!(embedded_caps.medium_capacity < default_caps.medium_capacity);
    assert_test!(embedded_caps.large_capacity < default_caps.large_capacity);

    // Test desktop platform capacities
    let desktop_caps = PlatformCapacities::desktop();
    assert_test!(desktop_caps.validate());
    assert_test!(desktop_caps.small_capacity > default_caps.small_capacity);
    assert_test!(desktop_caps.medium_capacity > default_caps.medium_capacity);
    assert_test!(desktop_caps.large_capacity > default_caps.large_capacity);

    // Test safety critical platform capacities
    let safety_caps = PlatformCapacities::safety_critical();
    assert_test!(safety_caps.validate());
    assert_test!(safety_caps.small_capacity <= embedded_caps.small_capacity);
    assert_test!(safety_caps.medium_capacity <= embedded_caps.medium_capacity);

    // Test invalid capacities
    let invalid_caps = PlatformCapacities {
        small_capacity: 100,
        medium_capacity: 50, // Invalid: medium < small
        large_capacity: 200,
        memory_provider_size: 1024,
    };
    assert_test!(!invalid_caps.validate());

    Ok(())
}

/// Test error standardization across foundation types
pub fn test_foundation_errors(config: &TestConfig) -> TestResult {
    // Test safety error helpers
    let safety_violation = wrt_error::Error::safety_violation("Test violation");
    assert_eq_test!(safety_violation.category, ErrorCategory::Safety);

    let memory_corruption = wrt_error::Error::memory_corruption("Test corruption");
    assert_eq_test!(memory_corruption.category, ErrorCategory::Safety);

    let verification_failed = wrt_error::Error::verification_failed("Test verification");
    assert_eq_test!(verification_failed.category, ErrorCategory::Safety);

    // Test unified type errors
    let config_error = wrt_error::Error::unified_type_config("Test config");
    assert_eq_test!(config_error.category, ErrorCategory::Type);

    let capacity_error = wrt_error::Error::platform_capacity_mismatch("Test capacity");
    assert_eq_test!(capacity_error.category, ErrorCategory::Capacity);

    // Test memory system errors
    let alloc_error = wrt_error::Error::memory_allocation_failed("Test allocation");
    assert_eq_test!(alloc_error.category, ErrorCategory::Memory);

    let provider_error = wrt_error::Error::memory_provider_capacity_exceeded("Test provider");
    assert_eq_test!(provider_error.category, ErrorCategory::Capacity);

    // Test bounded collection errors
    let bounded_error = wrt_error::Error::bounded_collection_capacity_exceeded("Test bounded");
    assert_eq_test!(bounded_error.category, ErrorCategory::Capacity);

    let conversion_error = wrt_error::Error::bounded_collection_conversion("Test conversion");
    assert_eq_test!(conversion_error.category, ErrorCategory::Type);

    Ok(())
}

/// Register all foundation integration tests
pub fn register_foundation_tests() {
    crate::register_test!(
        "unified_types_functionality",
        "foundation",
        false,
        "Test unified type system functionality",
        test_unified_types
    );

    crate::register_test!(
        "memory_provider_hierarchy",
        "foundation",
        false,
        "Test memory provider hierarchy",
        test_memory_providers
    );

    crate::register_test!(
        "safety_system_integration",
        "foundation",
        false,
        "Test safety system integration",
        test_safety_system
    );

    crate::register_test!(
        "platform_capacities_validation",
        "foundation",
        false,
        "Test platform capacity validation",
        test_platform_capacities
    );

    crate::register_test!(
        "foundation_error_standardization",
        "foundation",
        false,
        "Test error standardization across foundation types",
        test_foundation_errors
    );
}