//! Demonstration of budget visualization capabilities
//!
//! This example shows how to use the visualization and monitoring
//! features to track memory usage in a WRT application.

use wrt_foundation::{
    bounded::{BoundedMap, BoundedString, BoundedVec},
    budget_aware_provider::{BudgetAwareProviderFactory, CrateId},
    budget_provider::BudgetProvider,
    budget_visualization::{
        quick_ascii_dump, quick_debug_dump, BudgetVisualizer, DebugDumper, VisualizationConfig,
        VisualizationFormat,
    },
    memory_system_initializer, WrtResult,
};

#[cfg(feature = "std")]
use std::{thread, time::Duration};

fn main() -> WrtResult<()> {
    println!("🚀 WRT Budget Visualization Demo");
    println!("=================================\n");

    // Initialize the memory system
    println!("📋 Initializing memory system...");
    memory_system_initializer::initialize_global_memory_system(
        wrt_foundation::safety_system::SafetyLevel::Standard,
        wrt_foundation::global_memory_config::MemoryEnforcementLevel::Strict,
        Some(32 * 1024 * 1024), // 32MB total budget
    )?;
    println!("✅ Memory system initialized\n");

    // Show initial state
    println!("📊 Initial Memory State:");
    println!("{}", quick_ascii_dump()?);

    // Simulate application startup - Foundation allocations
    println!("🏗️  Simulating Foundation startup...");
    let foundation_providers = simulate_foundation_startup()?;

    // Show state after foundation
    println!("📊 After Foundation Startup:");
    println!("{}", quick_ascii_dump()?);

    // Simulate runtime loading
    println!("⚡ Simulating Runtime loading...");
    let runtime_providers = simulate_runtime_loading()?;

    // Show state after runtime
    println!("📊 After Runtime Loading:");
    println!("{}", quick_ascii_dump()?);

    // Simulate component instantiation
    println!("🔧 Simulating Component instantiation...");
    let component_providers = simulate_component_instantiation()?;

    // Show state after components
    println!("📊 After Component Instantiation:");
    println!("{}", quick_ascii_dump()?);

    // Generate various visualization formats
    demonstrate_visualization_formats()?;

    // Demonstrate monitoring macros
    demonstrate_monitoring_macros()?;

    // Simulate memory pressure
    println!("🔥 Simulating memory pressure...");
    simulate_memory_pressure()?;

    // Generate final debug dump
    println!("🔬 Final Debug Dump:");
    println!("{}", quick_debug_dump()?);

    // Cleanup demo
    drop(foundation_providers);
    drop(runtime_providers);
    drop(component_providers);

    println!("🏁 Demo completed successfully!");
    Ok(())
}

fn simulate_foundation_startup() -> WrtResult<Vec<BudgetProvider<{ 64 * 1024 }>>> {
    let mut providers = Vec::new();

    // Simulate core foundation allocations
    for i in 0..3 {
        let provider = wrt_foundation::monitor_operation!(
            format!("Foundation allocation {}", i),
            BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Foundation)
        )?;
        providers.push(provider);

        #[cfg(feature = "std")]
        thread::sleep(Duration::from_millis(100));
    }

    Ok(providers)
}

fn simulate_runtime_loading() -> WrtResult<Vec<BudgetProvider<{ 256 * 1024 }>>> {
    let mut providers = Vec::new();

    // Simulate runtime module loading
    for i in 0..4 {
        let provider = wrt_foundation::monitor_operation!(
            format!("Runtime module {}", i),
            BudgetProvider::<{ 256 * 1024 }>::new(CrateId::Runtime)
        )?;
        providers.push(provider);

        #[cfg(feature = "std")]
        thread::sleep(Duration::from_millis(150));
    }

    Ok(providers)
}

fn simulate_component_instantiation() -> WrtResult<Vec<Box<dyn core::any::Any>>> {
    let mut allocations: Vec<Box<dyn core::any::Any>> = Vec::new();

    // Simulate component type definitions
    for i in 0..2 {
        let provider = BudgetProvider::<{ 128 * 1024 }>::new(CrateId::Component)?;
        let component_types = BoundedMap::<u32, BoundedString<64, _>, 50, _>::new(provider)?;
        allocations.push(Box::new(component_types));

        #[cfg(feature = "std")]
        thread::sleep(Duration::from_millis(100));
    }

    // Simulate decoder buffers
    for i in 0..3 {
        let provider = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Decoder)?;
        let decode_buffer = BoundedVec::<u8, 32768, _>::new(provider)?;
        allocations.push(Box::new(decode_buffer));
    }

    Ok(allocations)
}

fn demonstrate_visualization_formats() -> WrtResult<()> {
    println!("🎨 Demonstrating Different Visualization Formats");
    println!("================================================\n");

    // ASCII format (compact)
    println!("📝 ASCII Format (Compact):");
    let ascii_config = VisualizationConfig {
        format: VisualizationFormat::Ascii,
        chart_width: 40,
        include_crate_details: true,
        include_shared_pool: false,
        ..Default::default()
    };
    println!("{}\n", BudgetVisualizer::generate_visualization(ascii_config)?);

    // JSON format (for APIs)
    println!("📋 JSON Format (sample):");
    let json_config = VisualizationConfig {
        format: VisualizationFormat::Json,
        include_crate_details: true,
        include_shared_pool: true,
        ..Default::default()
    };
    let json_output = BudgetVisualizer::generate_visualization(json_config)?;
    // Show first 200 characters
    let preview =
        if json_output.len() > 200 { format!("{}...", &json_output[..200]) } else { json_output };
    println!("{}\n", preview);

    // Markdown format (for documentation)
    println!("📖 Markdown Format (sample):");
    let md_config = VisualizationConfig {
        format: VisualizationFormat::Markdown,
        include_crate_details: true,
        include_shared_pool: false,
        ..Default::default()
    };
    let md_output = BudgetVisualizer::generate_visualization(md_config)?;
    let md_lines: Vec<&str> = md_output.lines().take(10).collect();
    println!("{}\n", md_lines.join("\n"));

    Ok(())
}

fn demonstrate_monitoring_macros() -> WrtResult<()> {
    println!("🔍 Demonstrating Monitoring Macros");
    println!("===================================\n");

    // Monitor a complex operation
    let result = wrt_foundation::monitor_operation!("Complex allocation operation", {
        let provider1 = BudgetProvider::<{ 32 * 1024 }>::new(CrateId::Format)?;
        let provider2 = BudgetProvider::<{ 64 * 1024 }>::new(CrateId::Host)?;

        let vec1 = BoundedVec::<u32, 1000, _>::new(provider1)?;
        let vec2 = BoundedVec::<u8, 8000, _>::new(provider2)?;

        (vec1, vec2)
    });

    let (_vec1, _vec2) = result?;

    // Print current memory status
    wrt_foundation::print_memory_status!("After macro demonstration");

    // Assert reasonable memory usage
    wrt_foundation::assert_memory_usage!(total < 50 * 1024 * 1024, "Total memory too high");

    // Check memory health
    wrt_foundation::check_memory_health!("Macro demonstration");

    #[cfg(feature = "std")]
    {
        // Benchmark allocation performance
        println!("⏱️  Benchmarking allocation performance:");
        wrt_foundation::benchmark_allocation!("Small provider creation", 100, {
            let _provider = BudgetProvider::<1024>::new(CrateId::Foundation)?;
        });
    }

    Ok(())
}

fn simulate_memory_pressure() -> WrtResult<()> {
    let mut pressure_allocations = Vec::new();

    // Try to allocate large chunks until we hit limits
    for i in 0..10 {
        match BudgetProvider::<{ 1024 * 1024 }>::new(CrateId::Platform) {
            Ok(provider) => {
                println!("  ✅ Large allocation {} succeeded", i + 1);
                pressure_allocations.push(provider);
            }
            Err(e) => {
                println!("  ❌ Large allocation {} failed: {:?}", i + 1, e);
                break;
            }
        }
    }

    // Show state under pressure
    println!("📊 Memory State Under Pressure:");
    println!("{}", quick_ascii_dump()?);

    Ok(())
}

#[cfg(feature = "std")]
fn save_reports_example() -> WrtResult<()> {
    use std::fs;

    println!("💾 Saving visualization reports...");

    // Create output directory
    fs::create_dir_all("./demo_reports")?;

    // Save different formats
    wrt_foundation::save_visualization!("./demo_reports/budget_report.html", Html);
    wrt_foundation::save_visualization!("./demo_reports/budget_data.json", Json);
    wrt_foundation::save_visualization!("./demo_reports/budget_data.csv", Csv);
    wrt_foundation::save_visualization!("./demo_reports/budget_report.md", Markdown);

    println!("✅ Reports saved to ./demo_reports/");
    Ok(())
}

// Include the save_reports_example in main if desired
#[allow(dead_code)]
fn extended_demo() -> WrtResult<()> {
    #[cfg(feature = "std")]
    save_reports_example()?;
    Ok(())
}
