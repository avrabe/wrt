//! Simple A+ Test to verify core functionality

use wrt_foundation::*;

#[test]
fn test_basic_managed_allocation() {
    // Test the core safe_managed_alloc! macro
    let guard = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let provider = guard.provider);

    // Verify provider works
    assert_eq!(provider.size(), 1024;

    // Test basic provider functionality
    assert_eq!(provider.capacity(), 1024;

    // Test provider can be used for memory operations
    let slice = provider.borrow_slice(0, 10).unwrap();
    assert_eq!(slice.len(), 10;

    // Verify the guard itself
    assert_eq!(guard.crate_id(), CrateId::Foundation;
}

#[test]
fn test_auto_provider_macro() {
    // Test auto-sizing macros
    let _guard1 =
        auto_provider!(CrateId::Foundation, typical_usage: "bounded_collections").unwrap();
    let _guard2 = auto_provider!(CrateId::Component).unwrap();
}

#[test]
fn test_monitoring_basics() {
    // Reset monitoring
    monitoring::MEMORY_MONITOR.reset);

    // Make an allocation
    let _guard = safe_managed_alloc!(512, CrateId::Foundation).unwrap();

    // Check basic monitoring
    let stats = monitoring::convenience::global_stats);
    assert!(stats.total_allocations > 0);
    assert!(stats.current_usage >= 512);
}
