//! Tests for automatic shared pool routing improvements
//!
//! These tests validate that small allocations are automatically routed
//! to the shared pool for optimal memory usage.

#[cfg(test)]
mod shared_pool_routing_tests {
    use wrt_foundation::{
        bounded::{BoundedString, BoundedVec},
        budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
        budget_provider::BudgetProvider,
        memory_system_initializer, WrtResult,
    };

    fn setup_with_small_shared_pool() -> WrtResult<()> {
        // Initialize with limited total budget to test routing
        memory_system_initializer::initialize_global_memory_system(
            wrt_foundation::safety_system::SafetyLevel::Standard,
            wrt_foundation::global_memory_config::MemoryEnforcementLevel::Strict,
            Some(8 * 1024 * 1024), // 8MB total for testing
        )?;
        Ok(())
    }

    #[test]
    fn test_automatic_small_allocation_routing() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Check the routing threshold
        let threshold = BudgetAwareProviderFactory::get_auto_routing_threshold);
        assert_eq!(threshold, 16 * 1024); // 16KB

        // Small allocation should go to shared pool
        let small_provider = BudgetProvider::<4096>::new(CrateId::Runtime)?;
        let vec = BoundedVec::<u8, 1000, _>::new(small_provider)?;

        // Verify shared pool has allocations
        let shared_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        assert!(shared_stats.allocated > 0);

        // Large allocation should NOT go to shared pool
        let large_provider = BudgetProvider::<{ 1024 * 1024 }>::new(CrateId::Runtime)?;
        let large_vec = BoundedVec::<u8, 100000, _>::new(large_provider)?;

        // Verify crate budget was used for large allocation
        let crate_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        assert!(crate_stats.allocated_bytes >= 1024 * 1024);

        Ok(())
    }

    #[test]
    fn test_shared_pool_size_buckets() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Test different size buckets
        let sizes_and_expected = [
            (1024, 4096),         // 1KB -> 4KB bucket
            (8192, 16384),        // 8KB -> 16KB bucket
            (32768, 65536),       // 32KB -> 64KB bucket
            (128 * 1024, 262144), // 128KB -> 256KB bucket
        ];

        for (request_size, expected_bucket) in &sizes_and_expected {
            // Create provider with requested size
            let provider = match *request_size {
                1024 => BudgetProvider::<1024>::new(CrateId::Foundation)?,
                8192 => BudgetProvider::<8192>::new(CrateId::Foundation)?,
                32768 => BudgetProvider::<32768>::new(CrateId::Foundation)?,
                131072 => BudgetProvider::<131072>::new(CrateId::Foundation)?,
                _ => continue,
            };

            // Use the provider
            let vec = BoundedVec::<u8, 100, _>::new(provider)?;

            // Verify it was allocated from shared pool (for small sizes)
            let shared_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
            assert!(shared_stats.allocated > 0);
        }

        Ok(())
    }

    #[test]
    fn test_shared_pool_fallback_to_crate_budget() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Exhaust the shared pool
        let mut allocations = Vec::new());

        // Keep allocating until shared pool is exhausted
        loop {
            match BudgetAwareProviderFactory::create_shared_provider::<4096>(CrateId::Runtime) {
                Ok(provider) => allocations.push(provider),
                Err(_) => break, // Shared pool exhausted
            }

            if allocations.len() > 1000 {
                break; // Safety limit
            }
        }

        // Now try to create a small provider - should fall back to crate budget
        let fallback_provider = BudgetProvider::<4096>::new(CrateId::Component)?;
        let vec = BoundedVec::<u8, 100, _>::new(fallback_provider)?;

        // Verify Component crate budget was used
        let component_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component)?;
        assert!(component_stats.allocated_bytes >= 4096);

        Ok(())
    }

    #[test]
    fn test_emergency_borrowing() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Initialize dynamic budgets with emergency borrowing enabled
        let dynamic_config = wrt_foundation::budget_aware_provider::DynamicBudgetConfig {
            emergency_borrowing: true,
            emergency_reserve_percent: 10,
            ..Default::default()
        };
        BudgetAwareProviderFactory::initialize_dynamic_budgets(dynamic_config)?;

        // Exhaust one crate's budget
        let mut runtime_allocations = Vec::new());
        while let Ok(provider) = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Runtime) {
            runtime_allocations.push(provider);
            if runtime_allocations.len() > 100 {
                break; // Safety limit
            }
        }

        // Try one more allocation - should trigger emergency borrowing
        let borrowed_result = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Runtime;

        // In a system with emergency borrowing, this might succeed
        // if other crates have unused budget
        match borrowed_result {
            Ok(_) => {
                // Emergency borrowing worked
                println!("Emergency borrowing succeeded");
            }
            Err(_) => {
                // Emergency borrowing couldn't find available budget
                println!("Emergency borrowing failed - no available budget");
            }
        }

        Ok(())
    }

    #[test]
    fn test_shared_pool_cross_crate_sharing() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Multiple crates using shared pool
        let foundation_provider = BudgetProvider::<8192>::new(CrateId::Foundation)?;
        let runtime_provider = BudgetProvider::<8192>::new(CrateId::Runtime)?;
        let component_provider = BudgetProvider::<8192>::new(CrateId::Component)?;

        // All should come from shared pool
        let vec1 = BoundedVec::<u32, 100, _>::new(foundation_provider)?;
        let vec2 = BoundedVec::<u16, 200, _>::new(runtime_provider)?;
        let vec3 = BoundedVec::<u8, 400, _>::new(component_provider)?;

        // Verify shared pool tracking
        let shared_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        assert!(shared_stats.allocated >= 3 * 16384)); // 3 allocations, each rounded up to 16KB

        // Individual crate budgets should be minimally affected
        for &crate_id in &[CrateId::Foundation, CrateId::Runtime, CrateId::Component] {
            let stats = BudgetAwareProviderFactory::get_crate_stats(crate_id)?;
            // Provider count should increase but not allocated bytes (from shared pool)
            assert_eq!(stats.provider_count, 1);
        }

        Ok(())
    }

    #[test]
    fn test_auto_routing_configuration() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Test enabling/disabling auto-routing
        BudgetAwareProviderFactory::set_auto_routing_enabled(true)?;

        // Test threshold queries
        let threshold = BudgetAwareProviderFactory::get_auto_routing_threshold);
        assert_eq!(threshold, 16 * 1024;

        // Test shared pool availability checks
        assert!(BudgetAwareProviderFactory::can_allocate_shared(4096);
        assert!(BudgetAwareProviderFactory::can_allocate_shared(16384);
        assert!(!BudgetAwareProviderFactory::can_allocate_shared(10 * 1024 * 1024))); // Too large

        Ok(())
    }

    #[test]
    fn test_shared_pool_statistics() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Get initial stats
        let initial_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        assert!(initial_stats.total_budget > 0);
        assert_eq!(initial_stats.allocated, 0);

        // Make some allocations
        let _p1 = BudgetProvider::<4096>::new(CrateId::Foundation)?;
        let _p2 = BudgetProvider::<8192>::new(CrateId::Runtime)?;

        // Check updated stats
        let updated_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        assert!(updated_stats.allocated > initial_stats.allocated);
        assert!(updated_stats.allocation_count >= 2);

        Ok(())
    }

    #[test]
    fn test_size_specific_macros() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Test size-specific convenience macros
        let small = wrt_foundation::small_provider!(CrateId::Foundation)?;
        let medium = wrt_foundation::medium_provider!(CrateId::Foundation)?;
        let large = wrt_foundation::large_provider!(CrateId::Foundation)?;

        // Small should go to shared pool (4KB <= 16KB threshold)
        // Medium and Large should go to crate budget

        let small_vec = BoundedVec::<u8, 100, _>::new(small)?;
        let medium_vec = BoundedVec::<u8, 1000, _>::new(medium)?;
        let large_vec = BoundedVec::<u8, 10000, _>::new(large)?;

        // Verify Foundation crate stats
        let foundation_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;
        assert_eq!(foundation_stats.provider_count, 3;

        // Large and medium should contribute to crate allocation
        assert!(foundation_stats.allocated_bytes >= 65536 + 1048576)); // medium + large

        Ok(())
    }

    #[test]
    fn test_smart_allocation_patterns() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Test different allocation patterns

        // Pattern 1: Many small allocations (should use shared pool efficiently)
        let mut small_allocations = Vec::new());
        for i in 0..10 {
            let provider = BudgetProvider::<1024>::new(CrateId::Decoder)?;
            small_allocations.push(BoundedVec::<u8, 100, _>::new(provider)?;
        }

        // Pattern 2: Mixed allocation sizes
        let small = BudgetProvider::<2048>::new(CrateId::Format)?;
        let medium = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Format)?;

        let small_string = BoundedString::<512, _>::new(small)?;
        let medium_vec = BoundedVec::<u32, 1000, _>::new(medium)?;

        // Verify efficient usage
        let shared_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        let global_stats = BudgetAwareProviderFactory::get_global_stats()?;

        assert!(shared_stats.allocated > 0)); // Shared pool used
        assert!(global_stats.total_allocated_memory > 0)); // Total tracking works

        Ok(())
    }

    #[test]
    fn test_optimization_recommendations() -> WrtResult<()> {
        setup_with_small_shared_pool()?;

        // Create usage patterns that would trigger recommendations

        // High utilization in Runtime
        let mut runtime_providers = Vec::new());
        for _ in 0..20 {
            if let Ok(provider) = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Runtime) {
                runtime_providers.push(provider);
            } else {
                break;
            }
        }

        // Low utilization in Panic crate (minimal usage)
        let _panic_provider = BudgetProvider::<512>::new(CrateId::Panic)?;

        // Get recommendations
        let recommendations = BudgetAwareProviderFactory::get_recommendations()?;

        // Should have recommendations for high and low utilization
        assert!(!recommendations.is_empty());

        for rec in &recommendations {
            assert!(rec.estimated_impact <= 100);
            assert!(rec.difficulty >= 1 && rec.difficulty <= 5);
            assert!(!rec.description.is_empty());
        }

        Ok(())
    }
}
