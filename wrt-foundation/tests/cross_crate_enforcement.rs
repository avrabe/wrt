//! Cross-crate enforcement tests
//!
//! These tests simulate real usage patterns across different WRT crates
//! to ensure budget enforcement works in practice.

#[cfg(test)]
mod cross_crate_enforcement_tests {
    use wrt_foundation::{
        bounded::{BoundedMap, BoundedString, BoundedVec},
        budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
        budget_provider::BudgetProvider,
        memory_system_initializer,
        runtime_monitoring::{EnforcementPolicy, MonitoringConfig, RuntimeMonitor},
        safe_memory::SafeMemoryHandler,
        WrtResult,
    };

    fn setup_strict_enforcement() -> WrtResult<()> {
        // Initialize with strict enforcement
        memory_system_initializer::initialize_global_memory_system(
            wrt_foundation::safety_system::SafetyLevel::SafetyCritical,
            wrt_foundation::global_memory_config::MemoryEnforcementLevel::Strict,
            Some(16 * 1024 * 1024), // 16MB total budget for testing
        )?;

        // Enable strict monitoring
        RuntimeMonitor::enable(MonitoringConfig {
            check_interval_ms: 50,
            enforcement_policy: EnforcementPolicy::Strict,
            alert_threshold_percent: 75,
            critical_threshold_percent: 90,
        })?;

        Ok(())
    }

    #[test]
    fn test_runtime_decoder_interaction() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Simulate Runtime creating instruction buffer
        let runtime_provider = BudgetProvider::<{ 256 * 1024 }>::new(CrateId::Runtime)?;
        let mut instruction_buffer = BoundedVec::<u8, 65536, _>::new(runtime_provider)?;

        // Fill with mock instructions
        for i in 0..1000 {
            instruction_buffer.push((i % 256) as u8)?;
        }

        // Simulate Decoder processing instructions
        let decoder_provider = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Decoder)?;
        let mut decoded_ops = BoundedVec::<u32, 1000, _>::new(decoder_provider)?;

        // Decode instructions
        for chunk in instruction_buffer.as_slice().chunks(4) {
            if chunk.len() == 4 {
                let op = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                decoded_ops.push(op)?;
            }
        }

        // Verify both crates tracked their allocations
        let runtime_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        let decoder_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Decoder)?;

        assert!(runtime_stats.current_allocation >= 256 * 1024);
        assert!(decoder_stats.current_allocation >= 128 * 1024);

        Ok(())
    }

    #[test]
    fn test_component_format_interaction() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Simulate Component model types
        let component_provider = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Component)?;
        let mut component_types =
            BoundedMap::<u32, BoundedString<64, _>, 100, _>::new(component_provider)?;

        // Add component type definitions
        for i in 0..10 {
            let str_provider = BudgetProvider::<512>::new(CrateId::Component)?;
            let mut type_name = BoundedString::new(str_provider)?;
            type_name.push_str(&format!("component:type{}", i))?;
            component_types.insert(i, type_name)?;
        }

        // Simulate Format serialization
        let format_provider = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Format)?;
        let mut serialized = BoundedVec::<u8, 32768, _>::new(format_provider)?;

        // Serialize component data
        for (id, name) in component_types.iter() {
            serialized.extend_from_slice(&id.to_le_bytes())?;
            serialized.push(name.len() as u8)?;
            serialized.extend_from_slice(name.as_bytes())?;
        }

        // Verify allocations
        let component_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component)?;
        let format_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Format)?;

        assert!(component_stats.current_allocation >= 64 * 1024 + (10 * 512));
        assert!(format_stats.current_allocation >= 32 * 1024);

        Ok(())
    }

    #[test]
    fn test_platform_host_interaction() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Simulate Platform memory mapping
        let platform_provider = BudgetProvider::<{ 1024 * 1024 }>::new(CrateId::Platform)?;
        let mut memory_map = SafeMemoryHandler::new(platform_provider);

        // Write some data
        let data = vec![0xAA; 4096];
        memory_map.write(0, &data)?;

        // Simulate Host accessing memory
        let host_provider = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Host)?;
        let mut host_buffer = BoundedVec::<u8, 4096, _>::new(host_provider)?;

        // Read from platform memory
        let mut temp = vec![0u8; 4096];
        memory_map.read(0, &mut temp)?;
        host_buffer.extend_from_slice(&temp)?;

        // Verify separate tracking
        let platform_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Platform)?;
        let host_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Host)?;

        assert!(platform_stats.current_allocation >= 1024 * 1024);
        assert!(host_stats.current_allocation >= 64 * 1024);
        assert_ne!(platform_stats.allocation_count, host_stats.allocation_count);

        Ok(())
    }

    #[test]
    fn test_debug_logging_interaction() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Simulate Debug trace buffer
        let debug_provider = BudgetProvider::<{ 16 * 1024 }>::new(CrateId::Debug)?;
        let mut trace_buffer = BoundedVec::<BoundedString<128, _>, 100, _>::new(debug_provider)?;

        // Add debug traces
        for i in 0..20 {
            let str_provider = BudgetProvider::<256>::new(CrateId::Debug)?;
            let mut trace = BoundedString::new(str_provider)?;
            trace.push_str(&format!("DEBUG: Operation {} completed", i))?;
            trace_buffer.push(trace)?;
        }

        // Simulate Logging formatting traces
        let logging_provider = BudgetProvider::<{ 8 * 1024 }>::new(CrateId::Logging)?;
        let mut log_output = BoundedString::<8192, _>::new(logging_provider)?;

        for trace in trace_buffer.iter() {
            log_output.push_str(trace.as_str())?;
            log_output.push('\n')?;
        }

        // Verify allocations
        let debug_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Debug)?;
        let logging_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Logging)?;

        assert!(debug_stats.current_allocation >= 16 * 1024 + (20 * 256));
        assert!(logging_stats.current_allocation >= 8 * 1024);

        Ok(())
    }

    #[test]
    fn test_math_sync_interaction() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Simulate Math computation buffers
        let math_provider = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Math)?;
        let mut compute_buffer = BoundedVec::<f64, 1024, _>::new(math_provider)?;

        // Fill with values
        for i in 0..512 {
            compute_buffer.push(i as f64 * 3.14159)?;
        }

        // Simulate Sync coordination state
        let sync_provider = BudgetProvider::<{ 4 * 1024 }>::new(CrateId::Sync)?;
        let mut sync_state = BoundedMap::<u32, u64, 64, _>::new(sync_provider)?;

        // Track computation state
        sync_state.insert(0, 0)?; // Start index
        sync_state.insert(1, 512)?; // End index
        sync_state.insert(2, compute_buffer.len() as u64)?;

        // Verify separate budgets
        let math_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Math)?;
        let sync_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Sync)?;

        assert!(math_stats.current_allocation >= 32 * 1024);
        assert!(sync_stats.current_allocation >= 4 * 1024);

        Ok(())
    }

    #[test]
    fn test_enforcement_under_pressure() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Try to allocate significant portions from multiple crates
        let crates = [
            CrateId::Runtime,
            CrateId::Component,
            CrateId::Decoder,
            CrateId::Format,
            CrateId::Platform,
        ];

        let mut allocations = Vec::new();
        let mut failed_crates = Vec::new();

        // Each crate tries to allocate 2MB
        for &crate_id in &crates {
            match BudgetProvider::<{ 2 * 1024 * 1024 }>::new(crate_id) {
                Ok(provider) => {
                    allocations.push((crate_id, provider));
                }
                Err(_) => {
                    failed_crates.push(crate_id);
                }
            }
        }

        // With 16MB total and 5 crates wanting 2MB each (10MB total)
        // Some should succeed, some might fail based on budget distribution
        assert!(!allocations.is_empty(), "No allocations succeeded");
        assert!(
            allocations.len() < crates.len(),
            "All large allocations succeeded - budget not enforced"
        );

        // Verify total doesn't exceed system budget
        let global_stats = BudgetAwareProviderFactory::get_global_stats()?;
        assert!(global_stats.total_allocated <= 16 * 1024 * 1024);

        // Check monitoring caught high usage
        let monitor_stats = RuntimeMonitor::get_stats()?;
        assert!(monitor_stats.alerts_triggered > 0 || monitor_stats.critical_alerts > 0);

        Ok(())
    }

    #[test]
    fn test_panic_handler_minimal_allocation() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Panic handler should work with minimal allocation
        let panic_provider = BudgetProvider::<512>::new(CrateId::Panic)?;
        let mut panic_msg = BoundedString::<256, _>::new(panic_provider)?;

        panic_msg.push_str("PANIC: Out of memory in critical section")?;

        // Verify minimal allocation
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Panic)?;
        assert_eq!(stats.current_allocation, 512);
        assert_eq!(stats.allocation_count, 1);

        Ok(())
    }

    #[test]
    fn test_intercept_minimal_overhead() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Intercept should have minimal memory overhead
        let intercept_provider = BudgetProvider::<1024>::new(CrateId::Intercept)?;
        let mut intercept_buffer = BoundedVec::<(u32, u32), 32, _>::new(intercept_provider)?;

        // Record some interceptions
        for i in 0..10 {
            intercept_buffer.push((i, i * 2))?;
        }

        // Should use minimal memory
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Intercept)?;
        assert!(stats.current_allocation <= 2048); // Small overhead acceptable

        Ok(())
    }

    #[test]
    fn test_instructions_efficient_storage() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Instructions crate should efficiently store bytecode
        let instructions_provider = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Instructions)?;
        let mut bytecode = BoundedVec::<u8, 65536, _>::new(instructions_provider)?;

        // Simulate realistic bytecode
        let pattern = [0x20, 0x00, 0x41, 0x01, 0x6A, 0x21, 0x00]; // local.get 0, i32.const 1, i32.add, local.set 0
        for _ in 0..1000 {
            bytecode.extend_from_slice(&pattern)?;
        }

        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Instructions)?;
        assert_eq!(stats.allocation_count, 1); // Single allocation
        assert!(stats.current_allocation >= 128 * 1024);

        Ok(())
    }

    #[test]
    fn test_full_system_stress() -> WrtResult<()> {
        setup_strict_enforcement()?;

        // Simulate realistic full system usage
        let mut all_allocations = Vec::new();

        // Runtime: Main execution state
        let runtime_main = BudgetProvider::<{ 512 * 1024 }>::new(CrateId::Runtime)?;
        all_allocations.push(("runtime_main", runtime_main));

        // Component: Type definitions
        let component_types = BudgetProvider::<{ 256 * 1024 }>::new(CrateId::Component)?;
        all_allocations.push(("component_types", component_types));

        // Decoder: Instruction decoding
        let decoder_buffer = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Decoder)?;
        all_allocations.push(("decoder_buffer", decoder_buffer));

        // Format: Serialization
        let format_buffer = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Format)?;
        all_allocations.push(("format_buffer", format_buffer));

        // Platform: Memory pages
        let platform_memory = BudgetProvider::<{ 1024 * 1024 }>::new(CrateId::Platform)?;
        all_allocations.push(("platform_memory", platform_memory));

        // Verify system is still healthy
        let health = wrt_foundation::memory_analysis::MemoryAnalyzer::generate_health_report()?;
        assert!(health.health_score >= 50); // At least moderate health
        assert_eq!(health.critical_issue_count, 0);

        // Get recommendations
        let recommendations = BudgetAwareProviderFactory::get_recommendations()?;

        // There might be optimization suggestions but no critical issues
        for rec in &recommendations {
            assert!(rec.estimated_impact <= 50); // No severe issues
        }

        Ok(())
    }
}
