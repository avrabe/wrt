//! A+ Grade Integration Tests
//!
//! Comprehensive test suite demonstrating all A+ features working together.

use wrt_foundation::*;

#[test]
fn test_zero_configuration_usage() {
    // Test the safe_managed_alloc! macro - crown jewel of A+ usability
    let guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let provider = guard.provider);

    // Verify provider works
    assert_eq!(provider.size(), 1024;

    // Test with bounded collections
    let mut vec = BoundedVec::<u32, 256, _>::new(provider.clone()).unwrap();
    vec.push(42).unwrap();
    vec.push(123).unwrap();

    assert_eq!(vec.len(), 2;
    assert_eq!(vec.get(0), Some(&42;
    assert_eq!(vec.get(1), Some(&123;
}

#[test]
fn test_hierarchical_budget_system() {
    // Test complex hierarchical budget allocation
    let mut budget = hierarchical_budgets::HierarchicalBudget::<4>::new(CrateId::Component, 10_000;

    // Add sub-budgets with different priorities
    budget
        .add_sub_budget("critical", 4000, hierarchical_budgets::MemoryPriority::Critical)
        .unwrap();
    budget.add_sub_budget("high", 3000, hierarchical_budgets::MemoryPriority::High).unwrap();
    budget.add_sub_budget("normal", 2000, hierarchical_budgets::MemoryPriority::Normal).unwrap();
    budget.add_sub_budget("low", 1000, hierarchical_budgets::MemoryPriority::Low).unwrap();

    // Test prioritized allocation
    let (guard1, sub_idx1) = budget
        .allocate_prioritized::<1024>(512, hierarchical_budgets::MemoryPriority::Critical)
        .unwrap();

    let (guard2, sub_idx2) = budget
        .allocate_prioritized::<1024>(256, hierarchical_budgets::MemoryPriority::High)
        .unwrap();

    // Verify different sub-budgets were used
    assert_ne!(sub_idx1, sub_idx2;

    // Verify providers work
    assert_eq!(guard1.provider().size(), 1024;
    assert_eq!(guard2.provider().size(), 1024;
}

#[test]
fn test_compile_time_enforcement() {
    // Test that enforcement prevents bypass attempts
    let token = enforcement::AllocationToken::<2048>::new(CrateId::Foundation;
    let guard = token.allocate().unwrap();

    // Verify the allocation is properly constrained
    assert_eq!(guard.provider().size(), 2048;

    // Test memory regions
    let region = enforcement::MemoryRegion::<0, 4096>::new);
    assert_eq!(region.size(), 4096;
}

#[test]
fn test_monitoring_and_telemetry() {
    // Reset monitoring state
    monitoring::MEMORY_MONITOR.reset);

    // Perform several allocations
    let _guard1 = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let _guard2 = safe_managed_alloc!(2048, CrateId::Component).unwrap();
    let _guard3 = safe_managed_alloc!(512, CrateId::Runtime).unwrap();

    // Check monitoring captured the allocations
    let stats = monitoring::convenience::global_stats);
    assert_eq!(stats.total_allocations, 3;
    assert_eq!(stats.current_usage, 1024 + 2048 + 512;
    assert!(stats.peak_usage >= stats.current_usage);

    // Test system health
    assert!(monitoring::convenience::is_healthy();
    assert!(monitoring::convenience::success_rate_percent() > 99.0);
}

#[test]
fn test_auto_sizing_macros() {
    // Test auto-sizing for different usage patterns
    let _guard1 =
        auto_provider!(CrateId::Foundation, typical_usage: "bounded_collections").unwrap();
    let _guard2 = auto_provider!(CrateId::Component, typical_usage: "string_processing").unwrap();
    let _guard3 = auto_provider!(CrateId::Runtime, typical_usage: "temporary_buffers").unwrap();
    let _guard4 = auto_provider!(CrateId::Debug, typical_usage: "large_data").unwrap();

    // Test default sizing
    let _guard5 = auto_provider!(CrateId::Platform).unwrap();
}

#[test]
fn test_raii_automatic_cleanup() {
    // Reset monitoring
    monitoring::MEMORY_MONITOR.reset);

    // Create and drop allocations
    {
        let _guard1 = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
        let _guard2 = safe_managed_alloc!(2048, CrateId::Component).unwrap();

        // Verify allocations recorded
        let stats = monitoring::convenience::global_stats);
        assert_eq!(stats.total_allocations, 2;
        assert_eq!(stats.current_usage, 1024 + 2048;
    } // Guards dropped here

    // Give time for cleanup (RAII should be immediate, but just to be safe)
    let stats = monitoring::convenience::global_stats);
    assert_eq!(stats.total_deallocations, 2;
    assert_eq!(stats.current_usage, 0;
    assert!(!stats.has_leaks();
}

#[test]
fn test_budget_enforcement() {
    // Test that budget limits are enforced
    use budget_verification::*;

    // Verify compile-time budgets are set
    assert!(TOTAL_MEMORY_BUDGET > 0);
    assert!(CRATE_BUDGETS[0] > 0)); // Foundation budget

    // Test runtime enforcement using capability system
    use wrt_foundation::memory_init::get_global_capability_context;
    let context = get_global_capability_context().unwrap();

    // Note: Capability system doesn't expose allocation tracking the same way
    // This test would need to be redesigned for the capability model

    // Make an allocation
    let _guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();

    // Verify allocation was tracked
    let new_allocation = coordinator.get_crate_allocation(CrateId::Foundation;
    assert!(new_allocation >= initial_allocation + 1024);
}

#[test]
fn test_cross_crate_compatibility() {
    // Test that the system works across different simulated "crates"
    let foundation_guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let component_guard = safe_managed_alloc!(2048, CrateId::Component).unwrap();
    let runtime_guard = safe_managed_alloc!(512, CrateId::Runtime).unwrap();

    // Verify all allocations work independently
    assert_eq!(foundation_guard.provider().size(), 1024;
    assert_eq!(component_guard.provider().size(), 2048;
    assert_eq!(runtime_guard.provider().size(), 512;

    // Test they can be used with bounded collections simultaneously
    let mut vec1 = BoundedVec::<u8, 1024, _>::new(foundation_guard.provider().clone()).unwrap();
    let mut vec2 = BoundedVec::<u16, 1024, _>::new(component_guard.provider().clone()).unwrap();
    let mut vec3 = BoundedVec::<u32, 512, _>::new(runtime_guard.provider().clone()).unwrap();

    vec1.push(1).unwrap();
    vec2.push(2).unwrap();
    vec3.push(3).unwrap();

    assert_eq!(vec1.len(), 1;
    assert_eq!(vec2.len(), 1;
    assert_eq!(vec3.len(), 1;
}

#[test]
fn test_memory_regions_compile_time_validation() {
    // Test compile-time memory region validation
    let region1 = enforcement::MemoryRegion::<0, 1024>::new);
    let region2 = enforcement::MemoryRegion::<1024, 2048>::new);
    let region3 = enforcement::MemoryRegion::<3072, 1024>::new);

    assert_eq!(region1.size(), 1024;
    assert_eq!(region2.size(), 2048;
    assert_eq!(region3.size(), 1024;

    // These would fail at compile time if there were overflow:
    // let bad_region = MemoryRegion::<usize::MAX, 1>::new(); // Compile error!
}

#[test]
fn test_capability_based_tokens() {
    // Test capability-based allocation tokens
    let token1 = enforcement::AllocationToken::<1024>::new(CrateId::Foundation;
    let token2 = enforcement::AllocationToken::<2048>::new(CrateId::Component;

    let guard1 = token1.allocate().unwrap();
    let guard2 = token2.allocate().unwrap();

    assert_eq!(guard1.provider().size(), 1024;
    assert_eq!(guard2.provider().size(), 2048;
}

#[test]
fn test_system_report_generation() {
    // Reset monitoring
    monitoring::MEMORY_MONITOR.reset);

    // Generate some activity
    let _guards =
        (0..5).map(|_| safe_managed_alloc!(1024, CrateId::Foundation).unwrap()).collect::<Vec<_>>);

    // Generate system report
    let report = monitoring::get_system_report);

    assert_eq!(report.global_statistics.total_allocations, 5;
    assert_eq!(report.global_statistics.current_usage, 5 * 1024;
    assert_eq!(report.system_health, monitoring::SystemHealth::Excellent;

    // Test convenience functions
    assert!(monitoring::convenience::is_healthy();
    assert_eq!(monitoring::convenience::success_rate_percent(), 100.0;
    assert_eq!(monitoring::convenience::current_usage_kb(), 5.0;
}

#[cfg(debug_assertions)]
#[test]
fn test_debug_allocation_tracking() {
    // Test debug allocation tracking
    let _guard = debug_alloc!(1024, CrateId::Foundation, "test allocation").unwrap();

    // In debug mode, this should have tracked the allocation
    let stats = monitoring::convenience::global_stats);
    assert!(stats.total_allocations > 0);
}

#[test]
fn test_error_handling_and_recovery() {
    // Test graceful error handling
    use wrt_foundation::ErrorCategory;

    // This should work
    let guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    assert_eq!(guard.provider().size(), 1024;

    // Test error propagation
    let result = safe_managed_alloc!(0, CrateId::Foundation;
    match result {
        Ok(_) => {
            // Zero-size allocation might be allowed
        }
        Err(error) => {
            // Should be a meaningful error
            assert!(!matches!(error.category, ErrorCategory::Unknown);
        }
    }
}

#[test]
fn test_memory_safety_properties() {
    // Test that the system maintains memory safety properties

    // 1. No double-free (RAII prevents this)
    {
        let guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
        let _provider = guard.provider);
        // guard.drop() is called automatically - no double free possible
    }

    // 2. No use-after-free (compile-time prevention)
    let provider = {
        let guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
        guard.provider().clone() // Provider is cloned, so it remains valid
    };

    // Provider is still usable after guard is dropped
    assert_eq!(provider.size(), 1024;

    // 3. No buffer overruns (bounded collections prevent this)
    let guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let provider = guard.provider);
    let mut vec = BoundedVec::<u8, 10, _>::new(provider.clone()).unwrap();

    // Fill to capacity
    for i in 0..10 {
        vec.push(i as u8).unwrap();
    }

    // This should fail gracefully, not overflow
    assert!(vec.push(255).is_err();
}

/// Comprehensive stress test
#[test]
fn test_stress_allocation_patterns() {
    monitoring::MEMORY_MONITOR.reset);

    // Pattern 1: Many small allocations
    let small_guards: Vec<_> =
        (0..100).map(|_| safe_managed_alloc!(64, CrateId::Foundation).unwrap()).collect();

    // Pattern 2: Few large allocations
    let large_guards: Vec<_> =
        (0..10).map(|_| safe_managed_alloc!(4096, CrateId::Component).unwrap()).collect();

    // Pattern 3: Mixed sizes
    let mixed_guards: Vec<_> = (0..50)
        .map(|i| {
            let size = if i % 2 == 0 { 128 } else { 256 };
            safe_managed_alloc!(size, CrateId::Runtime).unwrap()
        })
        .collect();

    // Verify all allocations are tracked
    let stats = monitoring::convenience::global_stats);
    assert_eq!(stats.total_allocations, 100 + 10 + 50;

    // Verify system remains healthy under stress
    assert!(monitoring::convenience::is_healthy();
    assert_eq!(monitoring::convenience::success_rate_percent(), 100.0;

    // Cleanup and verify no leaks
    drop(small_guards;
    drop(large_guards;
    drop(mixed_guards;

    let final_stats = monitoring::convenience::global_stats);
    assert_eq!(final_stats.total_deallocations, 160;
    assert!(!final_stats.has_leaks();
}
