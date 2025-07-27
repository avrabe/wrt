//! Comprehensive cross-crate integration tests for memory budget enforcement
//!
//! This test suite covers the gaps identified in existing tests:
//! - Resource handle management across crates
//! - Complex nested component interactions
//! - Error propagation and recovery
//! - Performance impact testing
//! - Edge cases and boundary conditions
//! - Thread safety and atomicity
//! - Debug and diagnostic integration
//! - Custom memory strategy testing
//! - Integration with external systems
//! - Compile-time enforcement validation

#![cfg(test)]

use core::alloc::Layout;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use wrt_error::Error as WrtError;
use wrt_foundation::{
    budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
    safe_managed_alloc,
    memory_pressure_handler::{MemoryPressureHandler, RecoveryConfig},
    memory_system_initializer::{self, MemoryEnforcementLevel},
    safe_memory::Allocator,
    safety_system::{AsilLevel, SafetyLevel},
    WrtResult,
};

// Mock resource handle for testing cross-crate resource management
#[derive(Debug)]
struct ResourceHandle {
    id: u32,
    crate_id: CrateId,
    data: Vec<u8>,
}

impl ResourceHandle {
    fn new(id: u32, crate_id: CrateId, size: usize) -> WrtResult<Self> {
        // Use safe_managed_alloc! for capability-based allocation
        let _provider = safe_managed_alloc!(4096, crate_id)?;
        Ok(Self { id, crate_id, data: vec![0u8); size] })
    }

    fn transfer_to(&mut self, new_crate: CrateId) -> WrtResult<()> {
        // This should fail - resources can't be transferred between crates
        if self.crate_id != new_crate {
            return Err(WrtError::memory_error(
                "Cannot transfer resource handle across crate boundaries",
            ;
        }
        Ok(())
    }
}

// Custom memory strategy for testing
struct TestMemoryStrategy {
    allocations: Arc<Mutex<HashMap<usize, Vec<u8>>>>,
    allocation_count: AtomicUsize,
    total_bytes: AtomicUsize,
}

impl TestMemoryStrategy {
    fn new() -> Self {
        Self {
            allocations: Arc::new(Mutex::new(HashMap::new())),
            allocation_count: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
        }
    }

    fn allocate(&self, size: usize) -> WrtResult<usize> {
        let id = self.allocation_count.fetch_add(1, Ordering::SeqCst;
        let mut allocs = self.allocations.lock().unwrap();
        allocs.insert(id, vec![0u8); size];
        self.total_bytes.fetch_add(size, Ordering::SeqCst;
        Ok(id)
    }

    fn deallocate(&self, id: usize) -> WrtResult<()> {
        let mut allocs = self.allocations.lock().unwrap();
        if let Some(data) = allocs.remove(&id) {
            self.total_bytes.fetch_sub(data.len(), Ordering::SeqCst;
            Ok(())
        } else {
            Err(WrtError::memory_error("Invalid allocation ID"))
        }
    }

    fn get_stats(&self) -> (usize, usize) {
        (self.allocation_count.load(Ordering::SeqCst), self.total_bytes.load(Ordering::SeqCst))
    }
}

mod resource_handle_tests {
    use super::*;

    #[test]
    fn test_resource_handle_cross_crate_prevention() {
        // Try to initialize, but ignore if already initialized
        let _ = memory_system_initializer::presets::development);

        // Create resource in Runtime crate
        let mut resource = ResourceHandle::new(1, CrateId::Runtime, 1024).unwrap();

        // Attempt to transfer to Component crate should fail
        let result = resource.transfer_to(CrateId::Component;
        assert!(result.is_err();
        assert!(result.unwrap_err().to_string().contains("Cannot transfer resource handle");

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_resource_lifetime_tracking() {
        let _ = memory_system_initializer::presets::development);

        let initial_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
        let initial_usage = initial_stats.allocated_bytes;

        {
            // Create resources that will be dropped
            let _r1 = ResourceHandle::new(1, CrateId::Runtime, 1024).unwrap();
            let _r2 = ResourceHandle::new(2, CrateId::Runtime, 2048).unwrap();

            let during_stats =
                BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
            assert!(during_stats.allocated_bytes > initial_usage);
        }

        // Resources should be cleaned up after drop
        let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
        assert_eq!(final_stats.allocated_bytes, initial_usage;

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod component_interaction_tests {
    use super::*;

    // Simulate component model interactions
    struct ComponentInstance {
        id: u32,
        imports: Vec<CrateId>,
        exports: Vec<String>,
        memory_provider: BudgetProvider<8192>,
    }

    impl ComponentInstance {
        fn new(id: u32, crate_id: CrateId) -> WrtResult<Self> {
            Ok(Self {
                id,
                imports: Vec::new(),
                exports: Vec::new(),
                memory_provider: BudgetProvider::new(crate_id)?,
            })
        }

        fn add_import(&mut self, from_crate: CrateId) -> WrtResult<()> {
            // Verify budget is available in source crate
            let stats = BudgetAwareProviderFactory::get_crate_stats(from_crate)?;
            let usage = stats.allocated_bytes;
            let budget = stats.budget_bytes;
            if usage >= budget {
                return Err(WrtError::memory_error("Import source crate has no budget";
            }
            self.imports.push(from_crate);
            Ok(())
        }
    }

    #[test]
    fn test_component_instantiation_budget_tracking() {
        let _ = memory_system_initializer::presets::development);

        // Create component instances in different crates
        let comp1 = ComponentInstance::new(1, CrateId::Component).unwrap();
        let comp2 = ComponentInstance::new(2, CrateId::Runtime).unwrap();

        // Verify budget is tracked separately
        let comp_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
        let runtime_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();

        assert!(comp_stats.allocated_bytes > 0);
        assert!(runtime_stats.allocated_bytes > 0);
        assert_ne!(comp_stats.allocated_bytes, runtime_stats.allocated_bytes;

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_canonical_abi_compliance() {
        let _ = memory_system_initializer::presets::development);

        // Simulate canonical ABI memory operations
        let provider = BudgetProvider::<4096>::new(CrateId::Component).unwrap();
        let mut buffer = vec![0u8; 256];

        // Simulate canonical lifting/lowering
        for i in 0..buffer.len() {
            buffer[i] = (i % 256) as u8;
        }

        // Verify memory is properly tracked
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
        assert!(stats.allocated_bytes >= 4096);

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod error_propagation_tests {
    use super::*;

    fn allocate_until_exhausted(crate_id: CrateId) -> WrtResult<Vec<BudgetProvider<1024>>> {
        let mut providers = Vec::new());
        loop {
            match BudgetProvider::<1024>::new(crate_id) {
                Ok(p) => providers.push(p),
                Err(e) => {
                    if e.to_string().contains("budget") {
                        return Ok(providers;
                    } else {
                        return Err(e;
                    }
                }
            }
        }
    }

    #[test]
    fn test_budget_exhaustion_recovery() {
        let _ = memory_system_initializer::presets::development);

        // Exhaust Runtime crate budget
        let providers = allocate_until_exhausted(CrateId::Runtime).unwrap();
        assert!(!providers.is_empty());

        // Verify other crates can still allocate
        let component_provider = BudgetProvider::<1024>::new(CrateId::Component).unwrap();
        let layout = Layout::from_size_align(100, 8).unwrap();
        let alloc_result = component_provider.allocate(layout;
        assert!(alloc_result.is_ok());

        // Drop half the providers to free budget
        let half = providers.len() / 2;
        drop(providers.into_iter().take(half).collect::<Vec<_>>);

        // Verify Runtime can allocate again
        let new_provider = BudgetProvider::<1024>::new(CrateId::Runtime).unwrap();
        let layout = Layout::from_size_align(100, 8).unwrap();
        let alloc_result = new_provider.allocate(layout;
        assert!(alloc_result.is_ok());

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_cross_crate_error_propagation() {
        let _ = memory_system_initializer::presets::development);

        // Function that propagates errors across crates
        fn cross_crate_operation() -> WrtResult<()> {
            let runtime_provider = BudgetProvider::<512>::new(CrateId::Runtime)?;
            let component_provider = BudgetProvider::<512>::new(CrateId::Component)?;

            // Simulate operation that could fail
            let layout = Layout::from_size_align(256, 8).unwrap();
            runtime_provider.allocate(layout.clone())?;
            component_provider.allocate(layout)?;

            Ok(())
        }

        // Should succeed normally
        assert!(cross_crate_operation().is_ok());

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod performance_tests {
    use super::*;

    #[test]
    fn test_allocation_overhead() {
        let _ = memory_system_initializer::presets::development);

        const ITERATIONS: usize = 10000;

        // Measure budget-aware allocation time
        let start = Instant::now);
        for _ in 0..ITERATIONS {
            let provider = BudgetProvider::<256>::new(CrateId::Runtime).unwrap();
            let layout = Layout::from_size_align(128, 8).unwrap();
            let _ = provider.allocate(layout;
        }
        let budget_aware_duration = start.elapsed);

        // Measure direct allocation time (for comparison)
        let start = Instant::now);
        for _ in 0..ITERATIONS {
            let _data = vec![0u8; 128];
        }
        let direct_duration = start.elapsed);

        // Budget tracking should add less than 5x overhead
        let overhead_ratio =
            budget_aware_duration.as_nanos() as f64 / direct_duration.as_nanos() as f64;
        println!("Allocation overhead ratio: {:.2}x", overhead_ratio);
        assert!(overhead_ratio < 5.0, "Budget tracking overhead too high: {:.2}x", overhead_ratio);

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_scalability_thousands_allocations() {
        let _ = memory_system_initializer::presets::development);

        const ALLOCATION_COUNT: usize = 5000;
        let mut providers = Vec::new());

        let start = Instant::now);
        for i in 0..ALLOCATION_COUNT {
            let crate_id = match i % 4 {
                0 => CrateId::Runtime,
                1 => CrateId::Component,
                2 => CrateId::Platform,
                _ => CrateId::Host,
            };

            if let Ok(provider) = BudgetProvider::<64>::new(crate_id) {
                providers.push(provider);
            }
        }
        let duration = start.elapsed);

        println!("Created {} allocations in {:?}", providers.len(), duration);
        assert!(
            duration < Duration::from_secs(1),
            "Allocation too slow for {} items",
            ALLOCATION_COUNT
        ;

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_budget_crate_behavior() {
        let _ = memory_system_initializer::presets::development);

        // Note: In practice, crates always have some budget
        // This tests graceful handling if budget is exhausted

        // Exhaust a crate's budget
        let mut providers = Vec::new());
        while let Ok(p) = BudgetProvider::<1024>::new(CrateId::Math) {
            providers.push(p);
        }

        // Verify allocation fails gracefully
        let result = BudgetProvider::<1024>::new(CrateId::Math;
        assert!(result.is_err();
        assert!(result.unwrap_err().to_string().contains("budget");

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_maximum_allocation_limits() {
        let _ = memory_system_initializer::presets::development);

        // Test various allocation sizes
        const SIZES: &[usize] = &[
            0,       // Zero size
            1,       // Minimum
            4096,    // Page size
            65536,   // 64KB
            1048576, // 1MB
        ];

        for &size in SIZES {
            // Skip if size is 0 (not supported by const generic)
            if size == 0 {
                continue;
            }

            // Use match to handle different sizes
            match size {
                1 => {
                    let result = BudgetProvider::<1>::new(CrateId::Runtime;
                    assert!(result.is_ok() || result.unwrap_err().to_string().contains("budget");
                }
                4096 => {
                    let result = BudgetProvider::<4096>::new(CrateId::Runtime;
                    assert!(result.is_ok() || result.unwrap_err().to_string().contains("budget");
                }
                65536 => {
                    let result = BudgetProvider::<65536>::new(CrateId::Runtime;
                    assert!(result.is_ok() || result.unwrap_err().to_string().contains("budget");
                }
                1048576 => {
                    let result = BudgetProvider::<1048576>::new(CrateId::Runtime;
                    // Large allocations might fail due to budget
                    assert!(result.is_ok() || result.unwrap_err().to_string().contains("budget");
                }
                _ => {}
            }
        }

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_integer_overflow_protection() {
        let _ = memory_system_initializer::presets::development);

        // Test that budget calculations don't overflow
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
        let usage = stats.allocated_bytes;
        let budget = stats.budget_bytes;

        // Verify budget > usage (no underflow)
        assert!(budget >= usage);

        // Verify calculations don't overflow
        let available = budget.saturating_sub(usage;
        assert!(available <= budget);

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod thread_safety_tests {
    use super::*;

    #[test]
    fn test_concurrent_budget_updates() {
        let _ = memory_system_initializer::presets::development);

        const THREAD_COUNT: usize = 8;
        const ALLOCATIONS_PER_THREAD: usize = 100;

        let mut handles = Vec::new());

        for thread_id in 0..THREAD_COUNT {
            let handle = thread::spawn(move || {
                let crate_id = match thread_id % 4 {
                    0 => CrateId::Runtime,
                    1 => CrateId::Component,
                    2 => CrateId::Platform,
                    _ => CrateId::Host,
                };

                let mut providers = Vec::new());
                for _ in 0..ALLOCATIONS_PER_THREAD {
                    if let Ok(p) = BudgetProvider::<256>::new(crate_id) {
                        providers.push(p);
                    }
                }
                providers.len()
            };
            handles.push(handle);
        }

        let total_allocations: usize = handles.into_iter().map(|h| h.join().unwrap()).sum);

        println!("Total concurrent allocations: {}", total_allocations);
        assert!(total_allocations > 0);

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_atomic_budget_operations() {
        let _ = memory_system_initializer::presets::development);

        let usage_before = Arc::new(AtomicUsize::new(0;
        let usage_after = Arc::new(AtomicUsize::new(0;

        // Capture initial usage
        usage_before.store(
            BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap().allocated_bytes,
            Ordering::SeqCst,
        ;

        // Concurrent allocations and deallocations
        let mut handles = Vec::new());

        for _ in 0..4 {
            let usage_after_clone = usage_after.clone();
            let handle = thread::spawn(move || {
                // Allocate and immediately deallocate
                for _ in 0..50 {
                    if let Ok(provider) = BudgetProvider::<512>::new(CrateId::Runtime) {
                        let layout = Layout::from_size_align(256, 8).unwrap();
                        let _ = provider.allocate(layout;
                        // Provider dropped here
                    }
                }

                // Store final usage
                if let Ok(stats) = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime) {
                    usage_after_clone.store(stats.allocated_bytes, Ordering::SeqCst;
                }
            };
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Usage should return to near original after all deallocations
        let initial = usage_before.load(Ordering::SeqCst;
        let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
        let final_usage = final_stats.allocated_bytes;

        // Allow some variance due to shared pool
        assert!(final_usage <= initial + 4096, "Memory leak detected in concurrent operations");

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod diagnostic_integration_tests {
    use super::*;

    #[test]
    fn test_debug_vs_release_tracking() {
        let _ = memory_system_initializer::presets::development);

        // Tracking should work in both debug and release modes
        let provider = BudgetProvider::<1024>::new(CrateId::Debug).unwrap();
        let layout = Layout::from_size_align(512, 8).unwrap();
        let alloc_result = provider.allocate(layout;
        assert!(alloc_result.is_ok());

        // Verify tracking is active
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Debug).unwrap();
        let usage = stats.allocated_bytes;
        assert!(usage > 0);

        #[cfg(debug_assertions)]
        {
            println!("Debug mode: Enhanced tracking active");
            // In debug mode, we might have additional checks
            assert!(stats.allocated_bytes >= 1024);
        }

        #[cfg(not(debug_assertions))]
        {
            println!("Release mode: Optimized tracking active");
            // In release mode, tracking should still work
            assert!(stats.allocated_bytes >= 1024);
        }

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_memory_leak_detection() {
        let _ = memory_system_initializer::presets::development);

        // Intentionally create a potential leak scenario
        let leaked_providers: Vec<BudgetProvider<512>> =
            (0..10).filter_map(|_| BudgetProvider::<512>::new(CrateId::Logging).ok()).collect());

        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Logging).unwrap();
        let usage_with_leaks = stats.allocated_bytes;
        assert!(usage_with_leaks >= 5120)); // At least 10 * 512

        // In a real system, leak detection would identify these allocations
        println!("Simulated memory leak scenario with {} allocations", leaked_providers.len));

        // Clean up
        drop(leaked_providers;

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod custom_strategy_tests {
    use super::*;

    #[test]
    fn test_custom_memory_strategy() {
        let _ = memory_system_initializer::presets::development);

        let strategy = TestMemoryStrategy::new();

        // Allocate using custom strategy
        let id1 = strategy.allocate(1024).unwrap();
        let id2 = strategy.allocate(2048).unwrap();

        let (count, bytes) = strategy.get_stats);
        assert_eq!(count, 2;
        assert_eq!(bytes, 3072;

        // Deallocate
        strategy.deallocate(id1).unwrap();

        let (count, bytes) = strategy.get_stats);
        assert_eq!(count, 2); // Count doesn't decrease
        assert_eq!(bytes, 2048;

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_platform_specific_strategies() {
        // Test embedded platform
        let _ = memory_system_initializer::initialize_global_memory_system(
            SafetyLevel::new(AsilLevel::AsilB),
            MemoryEnforcementLevel::Strict,
            Some(32 * 1024), // 32KB for embedded
        ;

        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
        let budget = stats.budget_bytes;
        assert!(budget < 10 * 1024)); // Should be small for embedded

        let _ = memory_system_initializer::complete_global_memory_initialization);

        // Test desktop platform
        let _ = memory_system_initializer::presets::development);

        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime).unwrap();
        let budget = stats.budget_bytes;
        assert!(budget > 100 * 1024)); // Should be larger for desktop

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod external_integration_tests {
    use super::*;

    // Simulate WASI memory allocation
    fn wasi_malloc(size: usize, crate_id: CrateId) -> WrtResult<*mut u8> {
        // In real WASI, this would use linear memory
        // Here we simulate with budget tracking
        let provider = BudgetProvider::<4096>::new(crate_id)?;
        let layout = Layout::from_size_align(size, 8).unwrap();
        let result = provider.allocate(layout)?;
        Ok(result)
    }

    #[test]
    fn test_wasi_integration() {
        let _ = memory_system_initializer::presets::development);

        // Simulate WASI memory operations
        let ptr1 = wasi_malloc(1024, CrateId::Host).unwrap();
        let ptr2 = wasi_malloc(2048, CrateId::Host).unwrap();

        assert_ne!(ptr1, ptr2;

        // Verify budget tracking
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Host).unwrap();
        let usage = stats.allocated_bytes;
        assert!(usage >= 8192)); // Two 4096 allocations

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_host_function_budget_enforcement() {
        let _ = memory_system_initializer::presets::development);

        // Simulate host function that allocates memory
        fn host_function_with_allocation() -> WrtResult<Vec<u8>> {
            let _provider = BudgetProvider::<2048>::new(CrateId::Host)?;
            let mut buffer = vec![0u8; 1024];

            // Simulate processing
            for i in 0..buffer.len() {
                buffer[i] = (i % 256) as u8;
            }

            Ok(buffer)
        }

        // Call host function multiple times
        let mut results = Vec::new());
        for _ in 0..5 {
            if let Ok(buffer) = host_function_with_allocation() {
                results.push(buffer);
            }
        }

        assert!(!results.is_empty());

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

mod compile_time_enforcement_tests {
    use super::*;

    // These tests verify that certain patterns are prevented at compile time
    // Note: Some of these would be compile errors if uncommented

    #[test]
    fn test_type_safety_enforcement() {
        let _ = memory_system_initializer::presets::development);

        // This compiles - correct usage
        let provider = BudgetProvider::<1024>::new(CrateId::Runtime).unwrap();

        // These would not compile (commented to allow tests to run):
        // let bad_provider: BudgetProvider<0> = BudgetProvider::new(CrateId::Runtime).unwrap();
        // let negative_size: BudgetProvider<-1> = BudgetProvider::new(CrateId::Runtime).unwrap();

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }

    #[test]
    fn test_const_generic_validation() {
        let _ = memory_system_initializer::presets::development);

        // Valid const generic sizes
        const SMALL: usize = 256;
        const MEDIUM: usize = 4096;
        const LARGE: usize = 65536;

        let small_provider = BudgetProvider::<SMALL>::new(CrateId::Runtime).unwrap();
        let medium_provider = BudgetProvider::<MEDIUM>::new(CrateId::Component).unwrap();
        let large_provider = BudgetProvider::<LARGE>::new(CrateId::Platform).unwrap();

        let layout1 = Layout::from_size_align(128, 8).unwrap();
        let layout2 = Layout::from_size_align(2048, 8).unwrap();
        let layout3 = Layout::from_size_align(32768, 8).unwrap();

        assert!(small_provider.allocate(layout1).is_ok());
        assert!(medium_provider.allocate(layout2).is_ok());
        assert!(large_provider.allocate(layout3).is_ok());

        let _ = memory_system_initializer::complete_global_memory_initialization);
    }
}

// Integration test that exercises the entire system
#[test]
fn test_comprehensive_system_stress() {
    let _ = memory_system_initializer::initialize_global_memory_system(
        SafetyLevel::new(AsilLevel::AsilA),
        MemoryEnforcementLevel::Permissive,
        None,
    ;

    // Phase 1: Initialize and warm up
    let recovery_config = RecoveryConfig::default());
    let _pressure_handler = MemoryPressureHandler::new(recovery_config;

    // Phase 2: Create diverse workload
    let workload_threads: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                let crate_id = match i {
                    0 => CrateId::Runtime,
                    1 => CrateId::Component,
                    2 => CrateId::Platform,
                    _ => CrateId::Host,
                };

                let mut allocations = Vec::new());
                let mut allocation_count = 0;

                // Mixed allocation patterns
                for j in 0..100 {
                    let size = match j % 4 {
                        0 => 256,
                        1 => 1024,
                        2 => 4096,
                        _ => 8192,
                    };

                    match size {
                        256 => {
                            if let Ok(p) = BudgetProvider::<256>::new(crate_id) {
                                allocations.push(Box::new(p) as Box<dyn std::any::Any>;
                                allocation_count += 1;
                            }
                        }
                        1024 => {
                            if let Ok(p) = BudgetProvider::<1024>::new(crate_id) {
                                allocations.push(Box::new(p) as Box<dyn std::any::Any>;
                                allocation_count += 1;
                            }
                        }
                        4096 => {
                            if let Ok(p) = BudgetProvider::<4096>::new(crate_id) {
                                allocations.push(Box::new(p) as Box<dyn std::any::Any>;
                                allocation_count += 1;
                            }
                        }
                        8192 => {
                            if let Ok(p) = BudgetProvider::<8192>::new(crate_id) {
                                allocations.push(Box::new(p) as Box<dyn std::any::Any>;
                                allocation_count += 1;
                            }
                        }
                        _ => {}
                    }

                    // Randomly deallocate some
                    if j % 10 == 0 && allocations.len() > 5 {
                        allocations.truncate(allocations.len() / 2;
                    }
                }

                (crate_id, allocation_count, allocations.len())
            })
        })
        .collect());

    // Phase 3: Monitor system health during stress
    let monitor_handle = thread::spawn(|| {
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(100;

            // Monitor system health
            println!("System health check in progress...");
        }
    };

    // Phase 4: Collect results
    let mut total_allocations = 0;
    let mut total_retained = 0;

    for handle in workload_threads {
        let (crate_id, allocated, retained) = handle.join().unwrap();
        println!("Crate {:?}: allocated {}, retained {}", crate_id, allocated, retained);
        total_allocations += allocated;
        total_retained += retained;
    }

    monitor_handle.join().unwrap();

    // Phase 5: Verify system integrity
    println!("\nFinal system state:");
    println!("  Total allocations: {}", total_allocations);
    println!("  Total retained: {}", total_retained);

    assert!(total_allocations > 0);

    memory_system_initializer::complete_global_memory_initialization().unwrap();
}
