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

    println!("Initial stats:";
    let initial_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
    println!("  Allocated: {}", initial_stats.allocated_bytes;
    println!("  Budget: {}", initial_stats.budget_bytes;

    // Create a budget provider
    println!("\nCreating BudgetProvider<4096>...";
    let provider = BudgetProvider::<4096>::new(CrateId::Component).unwrap();

    println!("\nAfter creation:";
    let after_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
    println!("  Allocated: {}", after_stats.allocated_bytes;
    println!("  Budget: {}", after_stats.budget_bytes;

    // Drop the provider
    drop(provider;

    println!("\nAfter drop:";
    let final_stats = BudgetAwareProviderFactory::get_crate_stats(CrateId::Component).unwrap();
    println!("  Allocated: {}", final_stats.allocated_bytes;
    println!("  Budget: {}", final_stats.budget_bytes;

    memory_system_initializer::complete_global_memory_initialization().unwrap();
}
