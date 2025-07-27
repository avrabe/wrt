use wrt_foundation::{
    budget_aware_provider::{
        BudgetAwareProviderFactory,
        CrateId,
    },
    budget_provider::BudgetProvider,
    memory_system_initializer,
};

fn main() {
    // Initialize memory system
    memory_system_initializer::presets::development().unwrap();

    println!("=== Testing shared pool allocation ===");

    // Get initial stats
    let initial_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
    println!("Initial stats:");
    println!("  Allocated: {} bytes", initial_stats.allocated_bytes);
    println!("  Budget: {} bytes", initial_stats.budget_bytes);
    println!("  Provider count: {}", initial_stats.provider_count);

    // Create a provider that should go to shared pool (4096 < 16KB threshold)
    println!("\nCreating BudgetProvider<4096>...");
    let provider = BudgetProvider::<4096>::new(CrateId::Component).unwrap();

    // Check stats after creation
    let after_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
    println!("\nAfter creation:");
    println!("  Allocated: {} bytes", after_stats.allocated_bytes);
    println!("  Budget: {} bytes", after_stats.budget_bytes);
    println!("  Provider count: {}", after_stats.provider_count);

    // Check shared pool stats
    if let Ok(shared_stats) = BudgetAwareProviderFactory::get_shared_pool_stats() {
        println!("\nShared pool stats:");
        println!("  4KB providers available: {}", shared_stats.available_4k);
        println!("  Total allocated: {} bytes", shared_stats.allocated);
        println!("  Total budget: {} bytes", shared_stats.total_budget);
    }

    // Drop the provider
    drop(provider;

    // Check final stats
    let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
    println!("\nAfter drop:");
    println!("  Allocated: {} bytes", final_stats.allocated_bytes);
    println!("  Budget: {} bytes", final_stats.budget_bytes);
    println!("  Provider count: {}", final_stats.provider_count);

    memory_system_initializer::complete_global_memory_initialization().unwrap();
}
