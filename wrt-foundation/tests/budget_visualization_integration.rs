//! Integration tests for budget visualization and debug functionality
//!
//! These tests validate the complete visualization and monitoring system.

#[cfg(test)]
mod visualization_integration_tests {
    use wrt_foundation::{
        bounded::{BoundedString, BoundedVec},
        budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
        budget_provider::BudgetProvider,
        budget_visualization::{
            quick_ascii_dump, quick_debug_dump, quick_json_dump, BudgetVisualizer, DebugDumper,
            VisualizationConfig, VisualizationFormat,
        },
        memory_system_initializer, WrtResult,
    };

    fn setup_test_environment() -> WrtResult<()> {
        memory_system_initializer::initialize_global_memory_system(
            wrt_foundation::safety_system::SafetyLevel::Standard,
            wrt_foundation::global_memory_config::MemoryEnforcementLevel::Strict,
            Some(16 * 1024 * 1024), // 16MB for testing
        )
    }

    #[test]
    fn test_ascii_visualization_generation() -> WrtResult<()> {
        setup_test_environment()?;

        // Create some memory usage
        let _provider1 = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Runtime)?;
        let _provider2 = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Component)?;
        let _provider3 = BudgetProvider::<{ 16 * 1024 }>::new(CrateId::Foundation)?;

        // Generate ASCII visualization
        let config = VisualizationConfig::default();
        let ascii_viz = BudgetVisualizer::generate_visualization(config)?;

        // Verify content
        assert!(ascii_viz.contains("WRT Memory Budget Visualization"));
        assert!(ascii_viz.contains("Total Active Providers"));
        assert!(ascii_viz.contains("Foundation"));
        assert!(ascii_viz.contains("Runtime"));
        assert!(ascii_viz.contains("Component"));

        println!("Generated ASCII visualization:\n{}", ascii_viz);
        Ok(())
    }

    #[test]
    fn test_json_visualization_generation() -> WrtResult<()> {
        setup_test_environment()?;

        // Create some memory usage
        let _provider = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Decoder)?;

        // Generate JSON visualization
        let config = VisualizationConfig {
            format: VisualizationFormat::Json,
            include_crate_details: true,
            include_shared_pool: true,
            ..Default::default()
        };

        let json_viz = BudgetVisualizer::generate_visualization(config)?;

        // Verify JSON structure
        assert!(json_viz.starts_with('{'));
        assert!(json_viz.ends_with('}'));
        assert!(json_viz.contains("timestamp"));
        assert!(json_viz.contains("global_stats"));
        assert!(json_viz.contains("crates"));
        assert!(json_viz.contains("shared_pool"));

        println!("Generated JSON visualization:\n{}", json_viz);
        Ok(())
    }

    #[test]
    fn test_csv_visualization_generation() -> WrtResult<()> {
        setup_test_environment()?;

        // Create memory usage in multiple crates
        let _p1 = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Format)?;
        let _p2 = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Host)?;

        // Generate CSV visualization
        let config = VisualizationConfig { format: VisualizationFormat::Csv, ..Default::default() };

        let csv_viz = BudgetVisualizer::generate_visualization(config)?;

        // Verify CSV structure
        assert!(csv_viz.contains("timestamp,crate,allocated_bytes"));
        assert!(csv_viz.contains("Format"));
        assert!(csv_viz.contains("Host"));

        // Should have multiple lines
        let lines: Vec<&str> = csv_viz.lines().collect();
        assert!(lines.len() > 1); // Header + data

        println!("Generated CSV visualization:\n{}", csv_viz);
        Ok(())
    }

    #[test]
    fn test_html_visualization_generation() -> WrtResult<()> {
        setup_test_environment()?;

        // Create some allocation pattern
        let _provider = BudgetProvider::<{ 256 * 1024 }>::new(CrateId::Platform)?;

        // Generate HTML visualization
        let config = VisualizationConfig {
            format: VisualizationFormat::Html,
            include_crate_details: true,
            include_shared_pool: false,
            ..Default::default()
        };

        let html_viz = BudgetVisualizer::generate_visualization(config)?;

        // Verify HTML structure
        assert!(html_viz.contains("<!DOCTYPE html>"));
        assert!(html_viz.contains("<title>WRT Memory Budget Report</title>"));
        assert!(html_viz.contains("<h1>WRT Memory Budget Report</h1>"));
        assert!(html_viz.contains("</html>"));

        println!("Generated HTML visualization length: {} chars", html_viz.len());
        Ok(())
    }

    #[test]
    fn test_markdown_visualization_generation() -> WrtResult<()> {
        setup_test_environment()?;

        // Create diverse memory usage
        let providers = [
            BudgetProvider::<{ 16 * 1024 }>::new(CrateId::Debug)?,
            BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Instructions)?,
            BudgetProvider::<{ 8 * 1024 }>::new(CrateId::Panic)?,
        ];

        // Generate Markdown visualization
        let config = VisualizationConfig {
            format: VisualizationFormat::Markdown,
            include_crate_details: true,
            include_shared_pool: true,
            ..Default::default()
        };

        let md_viz = BudgetVisualizer::generate_visualization(config)?;

        // Verify Markdown structure
        assert!(md_viz.contains("# WRT Memory Budget Report"));
        assert!(md_viz.contains("## Global Statistics"));
        assert!(md_viz.contains("## Crate Memory Usage"));
        assert!(md_viz.contains("| Crate | Allocated |"));

        println!("Generated Markdown visualization:\n{}", md_viz);
        Ok(())
    }

    #[test]
    fn test_debug_dump_generation() -> WrtResult<()> {
        setup_test_environment()?;

        // Create complex memory usage scenario
        let mut allocations = Vec::new();

        // Mix of small and large allocations
        for i in 0..5 {
            let provider = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Runtime)?;
            let vec = BoundedVec::<u8, 1000, _>::new(provider)?;
            allocations.push(vec);
        }

        // Generate debug dump
        let debug_dump = DebugDumper::generate_debug_dump()?;

        // Verify debug dump content
        assert!(debug_dump.contains("WRT MEMORY DEBUG DUMP"));
        assert!(debug_dump.contains("SYSTEM INFORMATION"));
        assert!(debug_dump.contains("GLOBAL MEMORY STATISTICS"));
        assert!(debug_dump.contains("PER-CRATE BREAKDOWN"));
        assert!(debug_dump.contains("SHARED POOL DETAILS"));
        assert!(debug_dump.contains("MEMORY HEALTH ANALYSIS"));
        assert!(debug_dump.contains("END OF DEBUG DUMP"));

        // Should contain Runtime usage
        assert!(debug_dump.contains("Runtime:"));

        println!("Debug dump length: {} chars", debug_dump.len());
        Ok(())
    }

    #[test]
    fn test_quick_functions() -> WrtResult<()> {
        setup_test_environment()?;

        // Create some memory usage
        let _provider = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Foundation)?;

        // Test quick functions
        let ascii_result = quick_ascii_dump();
        assert!(ascii_result.is_ok());
        let ascii_content = ascii_result?;
        assert!(ascii_content.contains("WRT Memory Budget Visualization"));

        let json_result = quick_json_dump();
        assert!(json_result.is_ok());
        let json_content = json_result?;
        assert!(json_content.starts_with('{'));

        let debug_result = quick_debug_dump();
        assert!(debug_result.is_ok());
        let debug_content = debug_result?;
        assert!(debug_content.contains("WRT MEMORY DEBUG DUMP"));

        Ok(())
    }

    #[test]
    fn test_visualization_with_different_configurations() -> WrtResult<()> {
        setup_test_environment()?;

        // Create memory usage
        let _p1 = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Component)?;
        let _p2 = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Decoder)?;

        // Test different configurations
        let configs = [
            VisualizationConfig {
                include_crate_details: true,
                include_shared_pool: true,
                chart_width: 40,
                ..Default::default()
            },
            VisualizationConfig {
                include_crate_details: false,
                include_shared_pool: true,
                chart_width: 80,
                ..Default::default()
            },
            VisualizationConfig {
                include_crate_details: true,
                include_shared_pool: false,
                show_percentages: false,
                ..Default::default()
            },
        ];

        for (i, config) in configs.iter().enumerate() {
            let result = BudgetVisualizer::generate_visualization(config.clone());
            assert!(result.is_ok(), "Config {} failed", i);

            let content = result?;
            assert!(!content.is_empty(), "Config {} produced empty content", i);

            // Verify configuration effects
            if !config.include_crate_details {
                assert!(!content.contains("Crate Budget Utilization"));
            }
            if !config.include_shared_pool {
                assert!(!content.contains("Shared Pool Status"));
            }
        }

        Ok(())
    }

    #[test]
    fn test_memory_snapshot_comparison() -> WrtResult<()> {
        setup_test_environment()?;

        // Capture initial snapshot
        let before = DebugDumper::capture_memory_snapshot()?;

        // Make some allocations
        let _providers = vec![
            BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Foundation)?,
            BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Runtime)?,
            BudgetProvider::<{ 16 * 1024 }>::new(CrateId::Component)?,
        ];

        // Capture after snapshot
        let after = DebugDumper::capture_memory_snapshot()?;

        // Compare snapshots
        let comparison = DebugDumper::compare_snapshots(&before, &after);

        // Verify comparison content
        assert!(comparison.contains("MEMORY SNAPSHOT COMPARISON"));
        assert!(comparison.contains("Total Memory Change:"));
        assert!(comparison.contains("Per-Crate Changes:"));

        println!("Snapshot comparison:\n{}", comparison);
        Ok(())
    }

    #[test]
    fn test_visualization_under_memory_pressure() -> WrtResult<()> {
        setup_test_environment()?;

        // Create high memory usage to test visualization under pressure
        let mut providers = Vec::new();

        // Allocate until we get significant utilization
        for _ in 0..20 {
            match BudgetProvider::<{ 256 * 1024 }>::new(CrateId::Runtime) {
                Ok(provider) => providers.push(provider),
                Err(_) => break, // Budget exhausted
            }
        }

        // Generate visualization under pressure
        let config = VisualizationConfig {
            format: VisualizationFormat::Ascii,
            include_crate_details: true,
            include_shared_pool: true,
            ..Default::default()
        };

        let result = BudgetVisualizer::generate_visualization(config);

        // Should still work under pressure
        assert!(result.is_ok());
        let content = result?;

        // Should show high utilization
        assert!(content.contains("Runtime"));

        // Generate debug dump as well
        let debug_dump = DebugDumper::generate_debug_dump()?;
        assert!(debug_dump.contains("Runtime:"));

        Ok(())
    }

    #[test]
    fn test_monitoring_macros_integration() -> WrtResult<()> {
        setup_test_environment()?;

        // Test monitor_operation! macro
        let result = wrt_foundation::monitor_operation!("test operation", {
            let provider = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Foundation)?;
            BoundedVec::<u8, 100, _>::new(provider)
        });

        assert!(result.is_ok());

        // Test print_memory_status! macro
        wrt_foundation::print_memory_status!("Test Status");

        // Test debug_memory_dump! macro
        wrt_foundation::debug_memory_dump!("Test Debug");

        // Test assert_memory_usage! macro
        wrt_foundation::assert_memory_usage!(total < 50 * 1024 * 1024); // 50MB limit
        wrt_foundation::assert_memory_usage!(crate_usage(CrateId::Foundation) < 10 * 1024 * 1024);

        // Test check_memory_health! macro
        wrt_foundation::check_memory_health!("Integration test");

        Ok(())
    }

    #[test]
    fn test_error_handling_in_visualization() -> WrtResult<()> {
        // Test without initializing memory system
        // This should handle errors gracefully

        let config = VisualizationConfig::default();
        let result = BudgetVisualizer::generate_visualization(config);

        // Should either succeed (if already initialized) or fail gracefully
        match result {
            Ok(content) => assert!(!content.is_empty()),
            Err(_) => {} // Expected if not initialized
        }

        // Test debug dump without initialization
        let debug_result = DebugDumper::generate_debug_dump();
        match debug_result {
            Ok(content) => assert!(!content.is_empty()),
            Err(_) => {} // Expected if not initialized
        }

        Ok(())
    }
}
