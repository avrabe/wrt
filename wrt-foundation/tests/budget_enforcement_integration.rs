//! Comprehensive cross-crate integration tests for budget enforcement
//!
//! These tests validate that the budget enforcement system works correctly
//! across all WRT crates and prevents budget bypass attempts.

#[cfg(test)]
mod budget_enforcement_integration_tests {
    use core::mem::size_of;
    use wrt_foundation::{
        safe_managed_alloc,
        {
            bounded::{BoundedMap, BoundedString, BoundedVec},
            budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
            budget_provider::BudgetProvider,
            memory_analysis::MemoryAnalyzer,
            memory_system_initializer,
            migration::migration_provider,
            runtime_monitoring::{EnforcementPolicy, MonitoringConfig, RuntimeMonitor},
            safe_memory::NoStdProvider,
            WrtError, WrtResult,
        },
    };

    // Helper to initialize test environment
    fn init_test_env() -> WrtResult<()> {
        // Initialize memory system with strict enforcement
        let _ = memory_system_initializer::initialize_global_memory_system(
            wrt_foundation::safety_system::SafetyLevel::Standard,
            wrt_foundation::global_memory_config::MemoryEnforcementLevel::Strict,
            None,
        )?;

        // Enable runtime monitoring
        RuntimeMonitor::enable(MonitoringConfig {
            check_interval_ms: 100,
            enforcement_policy: EnforcementPolicy::Strict,
            alert_threshold_percent: 80,
            critical_threshold_percent: 95,
        })?;

        // Enable memory analysis
        MemoryAnalyzer::enable);

        Ok(())
    }

    #[test]
    fn test_budget_provider_enforcement() -> WrtResult<()> {
        init_test_env()?;

        // Test 1: Budget provider should track allocations
        let provider1 = BudgetProvider::<1024>::new(CrateId::Foundation)?;
        let initial_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;

        // Create bounded collections
        let vec = BoundedVec::<u32, 100, _>::new(provider1)?;
        let updated_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;

        // Verify allocation was tracked
        assert!(
            updated_stats.current_allocation > initial_stats.current_allocation,
            "Allocation not tracked properly"
        ;

        // Test 2: Dropping should return allocation
        drop(vec;
        let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;
        assert_eq!(
            final_stats.current_allocation, initial_stats.current_allocation,
            "Memory not returned on drop"
        ;

        Ok(())
    }

    #[test]
    fn test_cross_crate_budget_isolation() -> WrtResult<()> {
        init_test_env()?;

        // Each crate should have its own budget
        let crates = [CrateId::Runtime, CrateId::Component, CrateId::Decoder, CrateId::Format];

        let mut providers = Vec::new);

        // Allocate from each crate
        for &crate_id in &crates {
            let provider = BudgetProvider::<4096>::new(crate_id)?;
            providers.push(provider);
        }

        // Verify each crate has independent allocation
        for &crate_id in &crates {
            let stats = BudgetAwareProviderFactory::get_crate_stats(crate_id)?;
            assert!(stats.current_allocation >= 4096);
            assert!(stats.allocation_count >= 1);
        }

        // Exhaust one crate's budget shouldn't affect others
        let runtime_budget =
            BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?.budget_limit;

        // Try to allocate most of Runtime's budget
        let large_size = (runtime_budget / 2) as usize;
        let _large_provider = BudgetProvider::<{ 1024 * 1024 }>::new(CrateId::Runtime)?;

        // Other crates should still be able to allocate
        for &crate_id in &[CrateId::Component, CrateId::Decoder] {
            let provider = BudgetProvider::<1024>::new(crate_id;
            assert!(provider.is_ok(), "Cross-crate interference detected");
        }

        Ok(())
    }

    #[test]
    fn test_budget_exhaustion_handling() -> WrtResult<()> {
        init_test_env()?;

        // Get current budget for a crate
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;
        let budget = stats.budget_limit;

        // Try to allocate more than the budget allows
        let mut allocations = Vec::new);
        let allocation_size = 65536; // 64KB chunks

        // Allocate until we hit the limit
        let mut total_allocated = 0;
        loop {
            match BudgetProvider::<65536>::new(CrateId::Foundation) {
                Ok(provider) => {
                    total_allocated += allocation_size;
                    allocations.push(provider);
                }
                Err(_) => {
                    // Should fail when budget exhausted
                    break;
                }
            }

            // Safety check to prevent infinite loop
            if total_allocated > budget * 2 {
                panic!("Budget enforcement not working!";
            }
        }

        // Verify we're near the budget limit
        let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;
        assert!(final_stats.current_allocation >= (budget * 80 / 100);

        // Free some memory
        allocations.truncate(allocations.len() / 2;

        // Should be able to allocate again
        let provider = BudgetProvider::<1024>::new(CrateId::Foundation;
        assert!(provider.is_ok(), "Cannot allocate after freeing memory");

        Ok(())
    }

    #[test]
    fn test_migration_provider_tracking() -> WrtResult<()> {
        init_test_env()?;

        // Migration providers should still be tracked
        let provider = migration_provider::<1024>);
        let vec = BoundedVec::<u8, 100, _>::new(provider)?;

        // Check that some allocation is tracked
        let stats = BudgetAwareProviderFactory::get_global_stats()?;
        assert!(stats.total_allocated > 0, "Migration provider not tracked");

        drop(vec;

        Ok(())
    }

    #[test]
    fn test_shared_pool_enforcement() -> WrtResult<()> {
        init_test_env()?;

        // Test shared pool allocations
        let shared1 = BudgetAwareProviderFactory::create_shared_provider::<1024>()?;
        let shared2 = BudgetAwareProviderFactory::create_shared_provider::<2048>()?;

        // Verify shared pool tracking
        let stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        assert!(stats.total_allocated >= 3072);
        assert!(stats.allocation_count >= 2);

        // Test shared pool limits
        let pool_size = stats.total_size;
        let mut shared_allocations = Vec::new);

        // Try to exhaust shared pool
        let chunk_size = 4096;
        while BudgetAwareProviderFactory::can_allocate_shared(chunk_size) {
            match BudgetAwareProviderFactory::create_shared_provider::<4096>() {
                Ok(provider) => shared_allocations.push(provider),
                Err(_) => break,
            }
        }

        // Verify we can't allocate more
        let result = BudgetAwareProviderFactory::create_shared_provider::<4096>);
        assert!(result.is_err(), "Shared pool limit not enforced");

        Ok(())
    }

    #[test]
    fn test_runtime_monitoring_alerts() -> WrtResult<()> {
        init_test_env()?;

        // Allocate significant memory to trigger alerts
        let mut providers = Vec::new);

        // Allocate 70% of Runtime budget
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        let target = (stats.budget_limit * 70 / 100) as usize;
        let mut allocated = 0;

        while allocated < target {
            match BudgetProvider::<65536>::new(CrateId::Runtime) {
                Ok(provider) => {
                    providers.push(provider);
                    allocated += 65536;
                }
                Err(_) => break,
            }
        }

        // Check for monitoring alerts
        let monitoring_stats = RuntimeMonitor::get_stats()?;
        assert!(monitoring_stats.total_checks > 0);

        // Allocate more to trigger critical alert
        while allocated < (stats.budget_limit * 95 / 100) as usize {
            match BudgetProvider::<65536>::new(CrateId::Runtime) {
                Ok(provider) => {
                    providers.push(provider);
                    allocated += 65536;
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    #[test]
    fn test_type_safety_enforcement() -> WrtResult<()> {
        init_test_env()?;

        // This test verifies that our type system prevents misuse

        // Valid: Using BudgetProvider
        let provider = BudgetProvider::<1024>::new(CrateId::Foundation)?;
        let vec1 = BoundedVec::<u32, 10, _>::new(provider)?;

        // Valid: Using safe_provider! macro
        let vec2 = wrt_foundation::budget_collection!(
            vec: u32, capacity: 10, size: 1024
        )?;

        // The following would fail to compile with our enforcement:
        // let guard = safe_managed_alloc!(1024, CrateId::Foundation)?;
        // Modern system: no unsafe memory extraction possible
        // let bad_vec = BoundedVec::<u32, 10, _>::new(bad_provider)?;

        assert_eq!(vec1.capacity(), vec2.capacity);

        Ok(())
    }

    #[test]
    fn test_memory_analysis_integration() -> WrtResult<()> {
        init_test_env()?;

        // Perform various allocations
        let _p1 = BudgetProvider::<4096>::new(CrateId::Foundation)?;
        let _p2 = BudgetProvider::<8192>::new(CrateId::Runtime)?;
        let _p3 = BudgetProvider::<16384>::new(CrateId::Component)?;

        // Generate memory snapshot
        let snapshot = MemoryAnalyzer::capture_snapshot()?;

        // Verify snapshot contains data
        assert!(snapshot.total_allocated > 0);
        assert!(!snapshot.crate_usage.is_empty();
        assert!(snapshot.crate_usage.len() >= 3);

        // Check access pattern analysis
        let patterns = MemoryAnalyzer::analyze_access_patterns()?;
        assert!(patterns.total_accesses > 0);

        // Generate health report
        let health = MemoryAnalyzer::generate_health_report()?;
        assert!(health.health_score > 0);
        assert_eq!(health.critical_issue_count, 0;

        Ok(())
    }

    #[test]
    fn test_platform_aware_budgets() -> WrtResult<()> {
        use wrt_foundation::{
            safe_managed_alloc,
            {
                bounded::{BoundedMap, BoundedString, BoundedVec},
                budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
                budget_provider::BudgetProvider,
                memory_analysis::MemoryAnalyzer,
                memory_system_initializer,
                migration::migration_provider,
                runtime_monitoring::{EnforcementPolicy, MonitoringConfig, RuntimeMonitor},
                safe_memory::NoStdProvider,
                WrtError, WrtResult,
            },
        };

        // Test with different simulated platforms
        for platform in &["embedded", "iot", "desktop"] {
            std::env::set_var("WRT_TEST_PLATFORM", platform;

            // Re-initialize for platform
            init_test_env()?;

            let discovery = PlatformDiscovery::new);
            let caps = discovery.discover_extended()?;

            // Verify platform-appropriate limits
            match platform {
                &"embedded" => {
                    assert!(caps.memory_subsystem.total_memory <= 2 * 1024 * 1024);
                }
                &"iot" => {
                    assert!(caps.memory_subsystem.total_memory <= 128 * 1024 * 1024);
                }
                &"desktop" => {
                    assert!(caps.memory_subsystem.total_memory >= 256 * 1024 * 1024);
                }
                _ => {}
            }

            // Test allocation within platform limits
            let size = match platform {
                &"embedded" => 1024,   // 1KB
                &"iot" => 65536,       // 64KB
                &"desktop" => 1048576, // 1MB
                _ => 4096,
            };

            // Should succeed within platform limits
            let provider = match size {
                1024 => BudgetProvider::<1024>::new(CrateId::Runtime),
                65536 => BudgetProvider::<65536>::new(CrateId::Runtime),
                1048576 => BudgetProvider::<1048576>::new(CrateId::Runtime),
                _ => BudgetProvider::<4096>::new(CrateId::Runtime),
            };

            assert!(provider.is_ok(), "Cannot allocate appropriate size for platform");
        }

        Ok(())
    }

    #[test]
    fn test_enforcement_prevents_bypass() -> WrtResult<()> {
        init_test_env()?;

        // Test that various bypass attempts are caught

        // 1. Direct NoStdProvider creation should be deprecated
        // Example of deprecated pattern (commented out):
        // #[allow(deprecated)]
        // let guard = safe_managed_alloc!(1024, CrateId::Foundation)?;
        // Modern system: direct provider extraction not possible
        // This compiles but is deprecated

        // 2. Migration providers are tracked
        let migration = migration_provider::<1024>);
        let stats_before = BudgetAwareProviderFactory::get_global_stats()?;
        let _vec = BoundedVec::<u8, 100, _>::new(migration)?;
        let stats_after = BudgetAwareProviderFactory::get_global_stats()?;
        assert!(stats_after.total_allocated > stats_before.total_allocated);

        // 3. Type constraints prevent misuse in generic contexts
        fn only_budget_provider<P: wrt_foundation::enforcement::BudgetProviderOnly>(p: P) {
            let _ = p.crate_id);
        }

        let good = BudgetProvider::<1024>::new(CrateId::Foundation)?;
        only_budget_provider(good); // This compiles

        // This would not compile:
        // only_budget_provider(direct); // Error: NoStdProvider doesn't impl BudgetProviderOnly

        Ok(())
    }

    #[test]
    fn test_concurrent_allocation_safety() -> WrtResult<()> {
        init_test_env()?;

        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let success_count = Arc::new(AtomicUsize::new(0;
        let fail_count = Arc::new(AtomicUsize::new(0;

        // Spawn multiple threads trying to allocate
        let mut handles = vec![];

        for i in 0..10 {
            let success = Arc::clone(&success_count);
            let fail = Arc::clone(&fail_count);

            let handle = thread::spawn(move || {
                // Each thread tries to allocate
                for _ in 0..100 {
                    match BudgetProvider::<1024>::new(CrateId::Runtime) {
                        Ok(_provider) => {
                            success.fetch_add(1, Ordering::Relaxed;
                            // Hold provider briefly
                            thread::yield_now);
                        }
                        Err(_) => {
                            fail.fetch_add(1, Ordering::Relaxed;
                        }
                    }
                }
            };

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        let total_success = success_count.load(Ordering::Relaxed;
        let total_fail = fail_count.load(Ordering::Relaxed;

        // Verify results
        println!("Concurrent test: {} success, {} failed", total_success, total_fail;
        assert!(total_success > 0, "No successful allocations");

        // Verify final state is consistent
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        assert!(stats.current_allocation <= stats.budget_limit);

        Ok(())
    }
}
