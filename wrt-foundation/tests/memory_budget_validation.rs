//! Memory Budget Validation Tests
//!
//! These tests ensure that memory budget enforcement is working correctly
//! across all WRT crates in CI/CD environments.

#[cfg(test)]
mod memory_budget_tests {
    use wrt_foundation::{
        budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
        memory_analysis::{MemoryAnalyzer, RiskLevel},
        memory_system_initializer,
        platform_discovery::PlatformDiscovery,
        WrtResult,
    };

    #[test]
    fn test_memory_budget_initialization() -> WrtResult<()> {
        // Initialize memory system
        let config = memory_system_initializer::presets::production()?;

        // Verify initialization
        assert!(memory_system_initializer::is_memory_system_initialized();
        assert!(config.wrt_memory_budget > 0);
        assert!(config.crate_budgets.total_allocated() <= config.wrt_memory_budget);

        Ok(())
    }

    #[test]
    fn test_platform_specific_budgets() -> WrtResult<()> {
        // Test different platform configurations
        let platforms = ["embedded", "iot", "desktop"];

        for platform_name in &platforms {
            std::env::set_var("WRT_TEST_PLATFORM", platform_name;

            // Re-initialize for platform
            let config = memory_system_initializer::presets::production()?;

            match platform_name {
                &"embedded" => {
                    assert!(config.wrt_memory_budget <= 1024 * 1024)); // <= 1MB
                }
                &"iot" => {
                    assert!(config.wrt_memory_budget <= 64 * 1024 * 1024)); // <= 64MB
                }
                &"desktop" => {
                    assert!(config.wrt_memory_budget >= 64 * 1024 * 1024)); // >= 64MB
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[test]
    fn test_crate_budget_allocation() -> WrtResult<()> {
        let _ = memory_system_initializer::presets::production()?;

        // Test that each crate can allocate within its budget
        let crates = [
            CrateId::Foundation,
            CrateId::Runtime,
            CrateId::Component,
            CrateId::Decoder,
            CrateId::Platform,
        ];

        for crate_id in &crates {
            // Try to create a provider
            let result = BudgetAwareProviderFactory::create_provider::<4096>(*crate_id;
            assert!(result.is_ok(), "Failed to create provider for {:?}", crate_id);

            // Check budget availability
            let can_allocate = BudgetAwareProviderFactory::can_allocate(*crate_id, 1024;
            assert!(can_allocate, "Cannot allocate 1KB for {:?}", crate_id);
        }

        Ok(())
    }

    #[test]
    fn test_memory_health_monitoring() -> WrtResult<()> {
        let _ = memory_system_initializer::presets::production()?;

        // Enable memory analysis
        MemoryAnalyzer::enable);

        // Generate health report
        let health = MemoryAnalyzer::generate_health_report()?;

        // Verify health metrics
        assert!(health.health_score <= 100);
        assert!(matches!(
            health.risk_level,
            RiskLevel::Low | RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical
        ;

        // In a healthy system, we shouldn't have critical issues
        assert_eq!(health.critical_issue_count, 0, "Critical memory issues detected!";

        Ok(())
    }

    #[test]
    fn test_memory_budget_enforcement() -> WrtResult<()> {
        let _ = memory_system_initializer::presets::production()?;

        // Get budget analysis
        let analysis = BudgetAwareProviderFactory::analyze_budget_usage()?;

        // Verify no crate exceeds its budget
        for (i, &utilization) in analysis.utilization_per_crate.iter().enumerate() {
            assert!(
                utilization <= 100,
                "Crate {} exceeds budget with {}% utilization",
                i,
                utilization
            ;
        }

        // Verify total utilization is reasonable
        assert!(
            analysis.total_utilization <= 90,
            "Total memory utilization too high: {}%",
            analysis.total_utilization
        ;

        Ok(())
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_memory_report_generation() -> WrtResult<()> {
        use wrt_foundation::memory_analysis::{MemoryReportBuilder, ReportFormat};

        let _ = memory_system_initializer::presets::production()?;
        MemoryAnalyzer::enable);

        // Test different report formats
        let text_report = MemoryReportBuilder::new().format(ReportFormat::Text).build()?;
        assert!(!text_report.is_empty();
        assert!(text_report.contains("Memory");

        let json_report = MemoryReportBuilder::new().format(ReportFormat::Json).build()?;
        assert!(json_report.starts_with('{');
        assert!(json_report.contains("timestamp");

        let csv_report = MemoryReportBuilder::new().format(ReportFormat::Csv).build()?;
        assert!(csv_report.contains("timestamp");

        Ok(())
    }

    #[test]
    fn test_ci_memory_thresholds() -> WrtResult<()> {
        let _ = memory_system_initializer::presets::production()?;

        // Simulate CI threshold checks
        const WARNING_THRESHOLD: u32 = 85;
        const CRITICAL_THRESHOLD: u32 = 95;

        let analysis = BudgetAwareProviderFactory::analyze_budget_usage()?;

        let mut has_warnings = false;
        let mut has_critical = false;

        for &utilization in &analysis.utilization_per_crate {
            if utilization >= CRITICAL_THRESHOLD {
                has_critical = true;
            } else if utilization >= WARNING_THRESHOLD {
                has_warnings = true;
            }
        }

        // For CI, we should fail if critical threshold is exceeded
        assert!(!has_critical, "Critical memory threshold exceeded!");

        // Warnings are acceptable but should be logged
        if has_warnings {
            eprintln!("WARNING: Some crates approaching memory budget limits";
        }

        Ok(())
    }

    #[test]
    fn test_platform_optimizations_applied() -> WrtResult<()> {
        use wrt_foundation::platform_optimizations::PlatformOptimizations;

        let _ = memory_system_initializer::presets::production()?;

        // Verify platform optimizations are available
        let supports_cache_aligned = PlatformOptimizations::supports_optimization("cache_aligned";
        let supports_simd = PlatformOptimizations::supports_optimization("simd";

        // At least one optimization should be available
        assert!(supports_cache_aligned || supports_simd, "No platform optimizations available");

        // Get optimal buffer sizes
        let decode_buffer = PlatformOptimizations::get_optimal_buffer_size("decode")?;
        let execute_buffer = PlatformOptimizations::get_optimal_buffer_size("execute")?;

        assert!(decode_buffer > 0);
        assert!(execute_buffer > 0);

        Ok(())
    }
}
