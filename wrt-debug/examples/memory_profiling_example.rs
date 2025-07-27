//! Example of using the memory profiling module

use wrt_debug::memory_profiling::{
    init_profiler,
    profile_operation,
    with_profiler,
    AllocationType,
    MemoryProfiler,
};
use wrt_foundation::budget_aware_provider::CrateId;

fn main() {
    // Initialize global memory system first
    wrt_foundation::memory_system_initializer::presets::development()
        .expect("Failed to initialize memory system"));

    // Initialize the memory profiler
    init_profiler().unwrap();

    // Enable allocation tracking and profiling
    MemoryProfiler::enable_allocation_tracking);
    MemoryProfiler::enable_profiling);

    // Track some allocations
    let alloc_id1 = with_profiler(|profiler| {
        profiler.track_allocation(
            CrateId::Runtime,
            1024,
            AllocationType::Heap,
            "example_allocation_1",
        )
    })
    .unwrap();

    let alloc_id2 = with_profiler(|profiler| {
        profiler.track_allocation(
            CrateId::Component,
            2048,
            AllocationType::Bounded,
            "example_allocation_2",
        )
    })
    .unwrap();

    // Profile an operation
    let result = profile_operation!("compute_sum", CrateId::Runtime, {
        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        sum
    };

    println!("Computation result: {}", result);

    // Track a deallocation
    with_profiler(|profiler| profiler.track_deallocation(alloc_id1)).unwrap();

    // Generate a profiling report
    let report = with_profiler(|profiler| profiler.generate_profile_report()).unwrap();

    println!("Memory Profiling Report:");
    println!("  Total allocations: {}", report.total_allocations);
    println!("  Total deallocations: {}", report.total_deallocations);
    println!("  Active allocations: {}", report.active_allocations);

    // Check for memory leaks
    let leaks = with_profiler(|profiler| profiler.detect_leaks()).unwrap();

    if !leaks.is_empty() {
        println!("\nPotential memory leaks detected:");
        for leak in leaks.iter() {
            println!(
                "  Allocation #{} ({}): {} bytes - confidence: {}%",
                leak.allocation.id, leak.reason, leak.allocation.size, leak.confidence
            ;
        }
    }

    println!("\nPerformance Metrics:");
    println!(
        "  Average operation time: {} µs",
        report.performance_metrics.avg_operation_time
    ;
    println!(
        "  Memory churn rate: {} bytes/µs",
        report.performance_metrics.memory_churn_rate
    ;

    // Complete memory system cleanup
    wrt_foundation::memory_system_initializer::complete_global_memory_initialization()
        .expect("Failed to complete memory system"));
}
