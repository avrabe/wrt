//! Macro and compile-time enforcement tests
//!
//! These tests validate that our macros and compile-time enforcement
//! mechanisms work correctly.

#[cfg(test)]
mod macro_enforcement_tests {
    use wrt_foundation::{
        safe_managed_alloc,
        {
            bounded::{BoundedString, BoundedVec},
            budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
            budget_provider::BudgetProvider,
            enforcement::{BudgetProviderOnly, ProviderConstraint},
            memory_system_initializer, WrtResult,
        },
    };

    fn setup() -> WrtResult<()> {
        memory_system_initializer::presets::test()
    }

    #[test]
    fn test_safe_provider_macro() -> WrtResult<()> {
        setup()?;

        // Test standard sizes
        let small = wrt_foundation::safe_provider!(small)?;
        assert_eq!(small.capacity(), 4096;

        let medium = wrt_foundation::safe_provider!(medium)?;
        assert_eq!(medium.capacity(), 65536;

        let large = wrt_foundation::safe_provider!(large)?;
        assert_eq!(large.capacity(), 1048576;

        // Test with explicit size
        let custom = wrt_foundation::safe_provider!(8192)?;
        assert_eq!(custom.capacity(), 8192;

        // Test with explicit crate
        let explicit = wrt_foundation::safe_provider!(16384, CrateId::Runtime)?;
        assert_eq!(explicit.capacity(), 16384;

        Ok(())
    }

    #[test]
    fn test_budget_collection_macro() -> WrtResult<()> {
        setup()?;

        // Test vector creation
        let vec = wrt_foundation::budget_collection!(
            vec: u32, capacity: 50, size: 2048
        )?;
        assert_eq!(vec.capacity(), 50;

        // Test string creation
        let string = wrt_foundation::budget_collection!(
            string: capacity: 128, size: 512
        )?;
        assert_eq!(string.capacity(), 128;

        Ok(())
    }

    #[test]
    fn test_constrained_provider_macro() -> WrtResult<()> {
        setup()?;

        // Test max constraint
        let provider1 = wrt_foundation::constrained_provider!(
            2048, CrateId::Foundation, max: 4096
        )?;
        assert_eq!(provider1.capacity(), 2048;

        // Test range constraint
        let provider2 = wrt_foundation::constrained_provider!(
            3072, CrateId::Foundation, range: 1024..=4096
        )?;
        assert_eq!(provider2.capacity(), 3072;

        // These would fail at compile time:
        // constrained_provider!(8192, CrateId::Foundation, max: 4096)
        // constrained_provider!(512, CrateId::Foundation, range: 1024..=4096)

        Ok(())
    }

    #[test]
    fn test_static_assert_macro() -> WrtResult<()> {
        setup()?;

        // These compile because assertions are true
        wrt_foundation::static_assert!(1 + 1 == 2);
        wrt_foundation::static_assert!(true, "This is always true");
        wrt_foundation::static_assert!(4096 <= 65536);

        // This would fail at compile time:
        // static_assert!(1 + 1 == 3)); // Compile error!

        Ok(())
    }

    #[test]
    fn test_assert_budget_limit_macro() -> WrtResult<()> {
        setup()?;

        // Test platform limits
        wrt_foundation::assert_budget_limit!(512 * 1024, embedded); // 512KB < 1MB ✓
        wrt_foundation::assert_budget_limit!(32 * 1024 * 1024, iot); // 32MB < 64MB ✓
        wrt_foundation::assert_budget_limit!(128 * 1024 * 1024, desktop); // 128MB < 256MB ✓

        // These would fail at compile time:
        // assert_budget_limit!(2 * 1024 * 1024, embedded);      // 2MB > 1MB ✗
        // assert_budget_limit!(128 * 1024 * 1024, iot);         // 128MB > 64MB ✗
        // assert_budget_limit!(512 * 1024 * 1024, desktop);     // 512MB > 256MB ✗

        Ok(())
    }

    #[test]
    fn test_type_constraint_enforcement() -> WrtResult<()> {
        setup()?;

        // Function that only accepts BudgetProvider
        fn only_budget_aware<P: BudgetProviderOnly>(provider: P) -> CrateId {
            provider.crate_id()
        }

        let good_provider = BudgetProvider::<1024>::new(CrateId::Runtime)?;
        let crate_id = only_budget_aware(good_provider;
        assert_eq!(crate_id, CrateId::Runtime;

        // This would not compile:
        // let guard = safe_managed_alloc!(1024, CrateId::Foundation)?;
        // Modern system: providers are automatically cleaned up via RAII
        // let _ = only_budget_aware(bad_provider); // Error: doesn't implement BudgetProviderOnly

        Ok(())
    }

    #[test]
    fn test_provider_constraint_trait() -> WrtResult<()> {
        setup()?;

        // Function with provider constraint
        fn process_with_constraint<const N: usize, P>(provider: P) -> bool
        where
            P: ProviderConstraint<N>,
        {
            provider.is_budget_aware()
        }

        let provider = BudgetProvider::<2048>::new(CrateId::Component)?;
        assert!(process_with_constraint(provider);

        // NoStdProvider would not implement ProviderConstraint

        Ok(())
    }

    #[test]
    fn test_macro_in_generic_context() -> WrtResult<()> {
        setup()?;

        // Generic function using our macros
        fn create_collection<T: Default + Clone>(
        ) -> WrtResult<BoundedVec<T, 100, BudgetProvider<4096>>> {
            let provider = wrt_foundation::safe_provider!(4096, CrateId::Foundation)?;
            BoundedVec::new(provider)
        }

        let vec: BoundedVec<u32, 100, _> = create_collection()?;
        assert_eq!(vec.capacity(), 100;

        Ok(())
    }

    #[test]
    fn test_macro_error_handling() -> WrtResult<()> {
        setup()?;

        // Exhaust a crate's budget
        let stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Panic)?;
        let budget = stats.budget_limit;

        // Allocate most of the budget
        let _large = BudgetProvider::<{ 512 * 1024 }>::new(CrateId::Panic)?;

        // Macro should properly propagate errors
        let result = wrt_foundation::safe_provider!(1024 * 1024, CrateId::Panic;
        assert!(result.is_err();

        // Constrained provider should also handle errors
        let result2 = wrt_foundation::constrained_provider!(
            1024 * 1024, CrateId::Panic, max: 2 * 1024 * 1024
        ;
        assert!(result2.is_err();

        Ok(())
    }

    #[test]
    fn test_nostd_provider_deprecation() -> WrtResult<()> {
        setup()?;

        // This should compile but with deprecation warning
        #[allow(deprecated)]
        let old_style = wrt_foundation::safe_memory::NoStdProvider::<1024>::default);

        // Can still be used (for now)
        #[allow(deprecated)]
        let vec = BoundedVec::<u8, 10, _>::new(old_style)?;
        assert_eq!(vec.capacity(), 10;

        // But we should use the new style
        let new_style = BudgetProvider::<1024>::new(CrateId::Foundation)?;
        let vec2 = BoundedVec::<u8, 10, _>::new(new_style)?;
        assert_eq!(vec2.capacity(), 10;

        Ok(())
    }

    #[test]
    fn test_macro_composability() -> WrtResult<()> {
        setup()?;

        // Macros should work together
        let provider = wrt_foundation::constrained_provider!(
            4096, CrateId::Runtime, range: 1024..=8192
        )?;

        // Use in static assertion
        wrt_foundation::static_assert!(4096 >= 1024);
        wrt_foundation::static_assert!(4096 <= 8192);

        // Use with collection macro
        let vec = BoundedVec::<u16, 200, _>::new(provider)?;
        assert_eq!(vec.capacity(), 200;

        Ok(())
    }

    #[test]
    fn test_enforcement_in_const_context() -> WrtResult<()> {
        setup()?;

        // Our enforcement should work in const contexts
        const SMALL_SIZE: usize = 4096;
        const MEDIUM_SIZE: usize = 65536;
        const LARGE_SIZE: usize = 1048576;

        // These constants can be used with our macros
        let small = wrt_foundation::safe_provider!(SMALL_SIZE)?;
        let medium = wrt_foundation::safe_provider!(MEDIUM_SIZE)?;
        let large = wrt_foundation::safe_provider!(LARGE_SIZE)?;

        assert_eq!(small.capacity(), SMALL_SIZE;
        assert_eq!(medium.capacity(), MEDIUM_SIZE;
        assert_eq!(large.capacity(), LARGE_SIZE;

        Ok(())
    }
}
