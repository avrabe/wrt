//! Command line test runner for wrt-test-registry
//!
//! This runner provides a unified interface for running tests in the wrt-test-registry.
//! It is designed to work with both standard and no_std environments.
//!
//! The runner follows the modular design and safety principles of the WRT project:
//! - Uses `wrt-error` for consistent error handling
//! - Uses `wrt-types` bounded collections for memory safety
//! - Follows the implementation sequence

use crate::prelude::*;
use clap::{Parser, Subcommand};
use colored::Colorize;

/// Command-line interface for running WebAssembly Runtime (WRT) tests.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Filter tests by name (supports partial matching)
    #[arg(short, long)]
    name: Option<String>,

    /// Filter tests by category (supports partial matching)
    #[arg(short, long)]
    category: Option<String>,

    /// Skip tests that require the standard library
    #[arg(long)]
    no_std: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Safety verification level (none, sampling, standard, full)
    #[arg(long, default_value = "standard")]
    verification: String,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered tests
    List {
        /// Filter tests by name (supports partial matching)
        #[arg(short, long)]
        name: Option<String>,

        /// Filter tests by category (supports partial matching)
        #[arg(short, long)]
        category: Option<String>,

        /// Show only tests that require the standard library
        #[arg(long)]
        std_only: bool,

        /// Show only tests that don't require the standard library
        #[arg(long)]
        no_std_only: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let registry = TestRegistry::global();

    // Register all compatibility tests
    crate::compatibility::register_compatibility_tests();

    // Set verification level based on command line argument
    let verification_level = match cli.verification.to_lowercase().as_str() {
        "none" => VerificationLevel::None,
        "sampling" => VerificationLevel::Sampling,
        "standard" => VerificationLevel::Standard,
        "full" => VerificationLevel::Full,
        _ => {
            eprintln!(
                "{}",
                format!(
                    "Invalid verification level: {}. Valid options are: none, sampling, standard, full",
                    cli.verification
                )
                .red()
            );
            std::process::exit(1);
        }
    };

    // Apply the verification level to the global registry
    if let Err(e) = registry.set_verification_level(verification_level) {
        eprintln!("{}", format!("Failed to set verification level: {}", e).red());
        // Decide if this is a fatal error for the runner
        // For now, we'll print the error and continue with the default or previously set level.
        // std::process::exit(1); // Optionally exit if setting level is critical
    }

    match cli.command {
        Some(Commands::List { name, category, std_only, no_std_only }) => {
            list_tests(registry, name.as_deref(), category.as_deref(), std_only, no_std_only);
        }
        None => {
            // Run tests
            run_tests(
                registry,
                cli.name.as_deref(),
                cli.category.as_deref(),
                !cli.no_std,
                cli.verbose,
                verification_level,
            );
        }
    }
}

/// Collect test statistics for a category
struct CategoryStats {
    total: usize,
    requires_std: usize,
    no_std: usize,
}

/// List tests matching the filters
fn list_tests(
    registry: &TestRegistry,
    name_filter: Option<&str>,
    category_filter: Option<&str>,
    std_only: bool,
    no_std_only: bool,
) {
    println!("{}", "WebAssembly Runtime (WRT) Test Registry".green().bold());

    // Use bounded collections for safety
    let mut category_stats: BoundedHashMap<String, CategoryStats, 100> = BoundedHashMap::new();
    registry.with_tests(|tests| {
        // First, collect all categories
        for test in tests {
            let category = test.category().to_string();

            // Skip tests that don't match the filters
            if !should_include_test(test, name_filter, Some(&category), std_only, no_std_only) {
                continue;
            }

            // Update category stats
            let entry = match category_stats.get_mut(&category) {
                Some(entry) => entry,
                None => {
                    // Insert new entry if it doesn't exist
                    let new_entry = CategoryStats { total: 0, requires_std: 0, no_std: 0 };

                    if let Err(e) = category_stats.try_insert(category.clone(), new_entry) {
                        // Skip if we can't insert (capacity reached)
                        eprintln!("Category capacity exceeded: {}", e);
                        continue;
                    }

                    category_stats.get_mut(&category).unwrap()
                }
            };

            entry.total += 1;
            if test.requires_std() {
                entry.requires_std += 1;
            } else {
                entry.no_std += 1;
            }
        }

        // Display by category
        println!("\n{}", "Tests by Category:".underline());

        // Create a bounded collection of sorted categories
        let mut sorted_categories = BoundedVec::<String, 100>::new();
        for (category, _) in category_stats.iter() {
            if let Err(e) = sorted_categories.try_push(category.clone()) {
                eprintln!("Too many categories to display: {}", e);
                break;
            }
        }

        // Sort categories
        sorted_categories.sort();

        for category in sorted_categories.iter() {
            if let Some(stats) = category_stats.get(category) {
                println!(
                    "{}: {} tests ({} std, {} no_std)",
                    category.bold(),
                    stats.total,
                    stats.requires_std,
                    stats.no_std
                );

                // List tests in this category
                registry.with_tests(|tests| {
                    let mut tests_in_category = BoundedVec::<String, 1024>::new();
                    for test in tests {
                        if test.category() == category {
                            // Skip tests that don't match the filters
                            if !should_include_test(test, name_filter, None, std_only, no_std_only)
                            {
                                continue;
                            }

                            let test_info = format!(
                                "  - {} {}",
                                test.name(),
                                if test.requires_std() {
                                    "(requires std)".dimmed()
                                } else {
                                    "(no_std compatible)".dimmed()
                                }
                            );

                            // Add to collection with error handling
                            if let Err(e) = tests_in_category.try_push(test_info) {
                                eprintln!("Too many tests to display: {}", e);
                                break;
                            }
                        }
                    }

                    // Sort and display tests
                    tests_in_category.sort();
                    for test_info in tests_in_category.iter() {
                        println!("{}", test_info);
                    }
                });

                println!(); // Add a blank line between categories
            }
        }
    });
}

/// Helper function to check if a test should be included based on filters
fn should_include_test(
    test: &Box<dyn TestCase>,
    name_filter: Option<&str>,
    category_filter: Option<&str>,
    std_only: bool,
    no_std_only: bool,
) -> bool {
    // Skip tests that don't match the name filter
    if let Some(filter) = name_filter {
        if !test.name().contains(filter) {
            return false;
        }
    }

    // Skip tests that don't match the category filter
    if let Some(filter) = category_filter {
        let category = test.category();
        if !category.contains(filter) {
            return false;
        }
    }

    // Skip tests that don't match the std/no_std filter
    if std_only && !test.requires_std() {
        return false;
    }

    if no_std_only && test.requires_std() {
        return false;
    }

    true
}

/// Run tests that match the given filters
fn run_tests(
    registry: &TestRegistry,
    name_filter: Option<&str>,
    category_filter: Option<&str>,
    allow_std: bool,
    verbose: bool,
    verification_level: VerificationLevel,
) {
    println!("{}", "Running WebAssembly Runtime Tests".green().bold());
    println!(
        "Verification level: {}",
        match verification_level {
            VerificationLevel::None => "None".normal(),
            VerificationLevel::Sampling => "Sampling".yellow(),
            VerificationLevel::Standard => "Standard".normal(),
            VerificationLevel::Full => "Full".red(),
        }
    );

    // Create a safe test configuration
    let mut test_config = TestConfig {
        is_std: true,
        features: BoundedVec::new(),
        params: HashMap::new(),
        verification_level,
    };

    // Add standard features
    let _ = test_config.features.try_push("std".to_string());

    let start_time = std::time::Instant::now();
    let num_tests = registry.run_filtered_tests(name_filter, category_filter, allow_std);
    let elapsed = start_time.elapsed();

    // Get the test statistics
    let stats = registry.get_stats();

    // Print test results with color coding
    if verbose {
        println!("\n{}", "Test Results:".underline());
        println!("Tests run: {}", num_tests);
        println!("Passed: {}", stats.passed.to_string().green());
        println!("Failed: {}", stats.failed.to_string().red());
        println!("Skipped: {}", stats.skipped.to_string().yellow());
        println!("Time: {:.2} seconds", elapsed.as_secs_f64());
    } else {
        print_test_statistics(&stats);
    }
}

/// Display a summary of test results
fn print_test_statistics(stats: &TestStats) {
    // Create a summary bar that shows passed/failed/skipped proportions
    let total = stats.passed + stats.failed + stats.skipped;
    if total == 0 {
        println!("No tests were run.");
        return;
    }

    let bar_width = 50;
    let passed_width = (stats.passed * bar_width) / total;
    let failed_width = (stats.failed * bar_width) / total;
    let skipped_width = bar_width - passed_width - failed_width;

    let passed_bar = "█".repeat(passed_width).green();
    let failed_bar = "█".repeat(failed_width).red();
    let skipped_bar = "█".repeat(skipped_width).yellow();

    println!("\n{}{}{}", passed_bar, failed_bar, skipped_bar);
    println!(
        "{} passed, {} failed, {} skipped (total: {})",
        stats.passed.to_string().green(),
        stats.failed.to_string().red(),
        stats.skipped.to_string().yellow(),
        total
    );

    // Print execution time if available
    println!("Execution time: {:.2} seconds", stats.execution_time_ms as f64 / 1000.0);
}
