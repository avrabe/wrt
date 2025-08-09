//! Specific enforcement scenario tests
//!
//! These tests validate specific enforcement scenarios and edge cases
//! to ensure the budget system cannot be bypassed.

#[cfg(test)]
mod enforcement_scenario_tests {
    use wrt_foundation::{
        bounded::{
            BoundedString,
            BoundedVec,
        },
        budget_aware_provider::{
            BudgetAwareProviderFactory,
            CrateId,
        },
        budget_provider::BudgetProvider,
        memory_system_initializer,
        safe_managed_alloc,
        safe_memory::SafeMemoryHandler,
        WrtResult,
    };

    fn setup() -> WrtResult<()> {
        memory_system_initializer::presets::test()
    }

    #[test]
    fn test_nested_allocation_tracking() -> WrtResult<()> {
        setup()?;

        // Test that nested allocations are properly tracked
        let provider1 = BudgetProvider::<4096>::new(CrateId::Foundation)?;
        let mut vec = BoundedVec::<BoundedString<32, _>, 10, _>::new(provider1)?;

        // Add strings which themselves need providers
        for i in 0..5 {
            let str_provider = BudgetProvider::<256>::new(CrateId::Foundation)?;
            let mut string = BoundedString::new(str_provider)?;
            string.push_str(&format!("test{}", i))?;
            vec.push(string)?;
        }

        // Check total allocation includes both vec and string providers
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;
        assert!(stats.current_allocation >= 4096 + (5 * 256));

        Ok(())
    }

    #[test]
    fn test_transfer_between_crates() -> WrtResult<()> {
        setup()?;

        // Test that memory cannot be "transferred" between crates
        let runtime_provider = BudgetProvider::<1024>::new(CrateId::Runtime)?;
        let component_provider = BudgetProvider::<1024>::new(CrateId::Component)?;

        // Create data in Runtime
        let mut runtime_vec = BoundedVec::<u32, 100, _>::new(runtime_provider)?;
        runtime_vec.push(42)?;

        // Create data in Component
        let mut component_vec = BoundedVec::<u32, 100, _>::new(component_provider)?;
        component_vec.push(99)?;

        // Verify each crate tracks its own allocation
        let runtime_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        let component_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component)?;

        assert!(runtime_stats.current_allocation >= 1024);
        assert!(component_stats.current_allocation >= 1024);
        assert_eq!(runtime_stats.allocation_count, 1);
        assert_eq!(component_stats.allocation_count, 1);

        Ok(())
    }

    #[test]
    fn test_safe_memory_handler_integration() -> WrtResult<()> {
        setup()?;

        // Test that SafeMemoryHandler works with BudgetProvider
        let provider = BudgetProvider::<2048>::new(CrateId::Decoder)?;
        let mut handler = SafeMemoryHandler::new(provider);

        // Write some data
        let data = b"Hello, WebAssembly!";
        let offset = handler.write(0, data)?;
        assert_eq!(offset, data.len());

        // Read it back
        let mut buffer = [0u8; 32];
        let read = handler.read(0, &mut buffer)?;
        assert_eq!(read, data.len());
        assert_eq!(&buffer[..read], data);

        // Verify allocation is tracked
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Decoder)?;
        assert!(stats.current_allocation >= 2048);

        Ok(())
    }

    #[test]
    fn test_allocation_failure_cleanup() -> WrtResult<()> {
        setup()?;

        // Get a small budget
        let mut providers = Vec::new();

        // Allocate most of the budget
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Format)?;
        let chunk_size = 65536; // 64KB
        let chunks_needed = (stats.budget_limit / chunk_size) - 1;

        for _ in 0..chunks_needed {
            if let Ok(p) = BudgetProvider::<65536>::new(CrateId::Format) {
                providers.push(p);
            }
        }

        // Try to create a collection that would exceed budget
        let result = {
            let provider = BudgetProvider::<{ 1024 * 1024 }>::new(CrateId::Format);
            provider.map(|p| BoundedVec::<u8, 1000, _>::new(p))
        };

        // Should fail due to budget limit
        assert!(result.is_err());

        // Verify no leaked allocation
        let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Format)?;
        assert_eq!(final_stats.allocation_count as usize, providers.len());

        Ok(())
    }

    #[test]
    fn test_zero_size_allocations() -> WrtResult<()> {
        setup()?;

        // Test handling of zero-size types
        let provider = BudgetProvider::<1024>::new(CrateId::Foundation)?;
        let mut vec = BoundedVec::<(), 1000, _>::new(provider)?;

        // Push many zero-size elements
        for _ in 0..1000 {
            vec.push(())?;
        }

        assert_eq!(vec.len(), 1000);

        // Even with ZSTs, the provider itself should be tracked
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Foundation)?;
        assert!(stats.current_allocation >= 1024);

        Ok(())
    }

    #[test]
    fn test_reallocation_tracking() -> WrtResult<()> {
        setup()?;

        // Test that reallocations are properly tracked
        let provider = BudgetProvider::<4096>::new(CrateId::Runtime)?;
        let mut string = BoundedString::<1024, _>::new(provider)?;

        let initial_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;

        // Grow the string
        for i in 0..100 {
            string.push_str(&format!("{}", i))?;
        }

        // Stats should still show same allocation count
        let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        assert_eq!(final_stats.allocation_count, initial_stats.allocation_count);
        assert_eq!(
            final_stats.current_allocation,
            initial_stats.current_allocation
        );

        Ok(())
    }

    #[test]
    fn test_enforcement_with_generics() -> WrtResult<()> {
        setup()?;

        use wrt_foundation::{
            bounded::{
                BoundedString,
                BoundedVec,
            },
            budget_aware_provider::{
                BudgetAwareProviderFactory,
                CrateId,
            },
            budget_provider::BudgetProvider,
            memory_system_initializer,
            safe_managed_alloc,
            safe_memory::SafeMemoryHandler,
            WrtResult,
        };

        // Generic function that only accepts budget providers
        fn process_with_provider<const N: usize, P>(provider: P) -> WrtResult<usize>
        where
            P: BudgetProviderOnly + wrt_foundation::safe_memory::Provider,
        {
            let vec = BoundedVec::<u8, 100, _>::new(provider)?;
            Ok(vec.capacity())
        }

        // This compiles and works
        let provider = BudgetProvider::<1024>::new(CrateId::Component)?;
        let capacity = process_with_provider(provider)?;
        assert_eq!(capacity, 100);

        // The modern system prevents unsafe memory extraction:
        // Providers are automatically cleaned up via RAII
        // No manual release() method exists

        Ok(())
    }

    #[test]
    fn test_compile_time_size_validation() -> WrtResult<()> {
        setup()?;

        // Test compile-time size constraints

        // Valid sizes
        let _small = wrt_foundation::constrained_provider!(
            1024, CrateId::Foundation, max: 4096
        )?;

        let _medium = wrt_foundation::constrained_provider!(
            8192, CrateId::Foundation, range: 4096..=16384
        )?;

        // These would fail at compile time:
        // let _too_big = constrained_provider!(
        //     8192, CrateId::Foundation, max: 4096
        // ); // Static assertion failure!

        // let _too_small = constrained_provider!(
        //     1024, CrateId::Foundation, range: 2048..=4096
        // ); // Static assertion failure!

        Ok(())
    }

    #[test]
    fn test_shared_pool_isolation() -> WrtResult<()> {
        setup()?;

        // Shared pool should be isolated from crate budgets
        let shared = BudgetAwareProviderFactory::create_shared_provider::<4096>()?;
        let crate_provider = BudgetProvider::<4096>::new(CrateId::Host)?;

        // Both should succeed independently
        let shared_vec = BoundedVec::<u32, 100, _>::new(shared)?;
        let crate_vec = BoundedVec::<u32, 100, _>::new(crate_provider)?;

        // Check stats are tracked separately
        let shared_stats = BudgetAwareProviderFactory::get_shared_pool_stats()?;
        let crate_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Host)?;

        assert!(shared_stats.total_allocated >= 4096);
        assert!(crate_stats.current_allocation >= 4096);

        // Exhausting shared pool shouldn't affect crate budgets
        let mut shared_providers = Vec::new();
        while let Ok(p) = BudgetAwareProviderFactory::create_shared_provider::<65536>() {
            shared_providers.push(p);
            if shared_providers.len() > 100 {
                break; // Safety limit
            }
        }

        // Can still allocate from crate budget
        let another_crate = BudgetProvider::<1024>::new(CrateId::Host)?;
        drop(another_crate); // Just testing allocation succeeds

        Ok(())
    }

    #[test]
    fn test_migration_helper_warnings() -> WrtResult<()> {
        setup()?;

        // Using migration helpers should work but be tracked
        let provider = wrt_foundation::migration::migration_provider::<1024>();

        // Should work like a normal provider
        let vec = BoundedVec::<u8, 50, _>::new(provider)?;
        assert_eq!(vec.capacity(), 50);

        // Should be tracked in global stats
        let stats = BudgetAwareProviderFactory::get_global_stats()?;
        assert!(stats.total_allocated > 0);

        // Using the nostd_provider! macro (migration helper)
        let provider2 = wrt_foundation::nostd_provider!(2048);
        let vec2 = BoundedVec::<u16, 25, _>::new(provider2)?;
        assert_eq!(vec2.capacity(), 25);

        Ok(())
    }

    #[test]
    fn test_dynamic_reallocation() -> WrtResult<()> {
        setup()?;

        // Test dynamic budget reallocation
        let initial_runtime = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        let initial_component = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component)?;

        // Request reallocation from Runtime to Component
        let result = BudgetAwareProviderFactory::request_reallocation(
            CrateId::Runtime,
            CrateId::Component,
            1024 * 1024, // 1MB
        )?;

        assert!(result.success);
        assert_eq!(result.amount_transferred, 1024 * 1024);

        // Verify budgets were adjusted
        let final_runtime = BudgetAwareProviderFactory::get_crate_stats(CrateId::Runtime)?;
        let final_component = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component)?;

        assert_eq!(
            final_runtime.budget_limit,
            initial_runtime.budget_limit - 1024 * 1024
        );
        assert_eq!(
            final_component.budget_limit,
            initial_component.budget_limit + 1024 * 1024
        );

        Ok(())
    }
}
