use std::{
    env, fs, io,
    path::{Path, PathBuf},
    collections::HashMap,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    
    // Memory budget validation
    if let Err(e) = validate_memory_budgets() {
        println!("cargo:warning=Memory budget validation failed: {}", e);
        // Don't fail the build, just warn
    } else {
        println!("cargo:warning=✅ Memory budget validation passed");
    }
    
    // Validate crate memory configurations
    if let Err(e) = validate_crate_memory_configs() {
        println!("cargo:warning=Crate memory config validation failed: {}", e);
    } else {
        println!("cargo:warning=✅ Crate memory configuration validation passed");
    }
    
    // Generate memory configuration constants
    if let Err(e) = generate_memory_constants() {
        println!("cargo:warning=Failed to generate memory constants: {}", e);
    } else {
        println!("cargo:warning=✅ Memory constants generated");
    }
    
    // If using skeptic for markdown examples
    #[cfg(feature = "test-examples")]
    {
        extern crate skeptic;
        
        // Test all markdown files with code examples
        skeptic::generate_doc_tests(&[
            "README.md",
            "docs/examples/README.md",
            // Add more markdown files here
        ]);
    }
}

/// Validate memory budgets across all crates
fn validate_memory_budgets() -> Result<(), String> {
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "Failed to get workspace root")?;
    
    let budget_config = load_budget_configuration(&workspace_root)?;
    
    // Validate total budget allocation
    let total_allocated: u64 = budget_config.values().sum();
    let max_budget = 128 * 1024 * 1024; // 128MB default max
    
    if total_allocated > max_budget {
        return Err(format!(
            "Total budget allocation ({} bytes) exceeds maximum ({} bytes)",
            total_allocated, max_budget
        ));
    }
    
    // Validate individual crate budgets
    for (crate_name, budget) in &budget_config {
        validate_crate_budget(crate_name, *budget)?;
    }
    
    // Check for budget balance
    let min_total = 16 * 1024 * 1024; // 16MB minimum
    if total_allocated < min_total {
        println!("cargo:warning=Total budget ({} bytes) is quite low, consider increasing for better performance", total_allocated);
    }
    
    Ok(())
}

/// Load budget configuration from environment or defaults
fn load_budget_configuration(workspace_root: &str) -> Result<HashMap<String, u64>, String> {
    let mut config = HashMap::new();
    
    // Default budget allocations (in bytes)
    config.insert("wrt-foundation".to_string(), 8 * 1024 * 1024);  // 8MB
    config.insert("wrt-runtime".to_string(), 16 * 1024 * 1024);     // 16MB
    config.insert("wrt-component".to_string(), 12 * 1024 * 1024);   // 12MB
    config.insert("wrt-decoder".to_string(), 4 * 1024 * 1024);      // 4MB
    config.insert("wrt-format".to_string(), 2 * 1024 * 1024);       // 2MB
    config.insert("wrt-host".to_string(), 4 * 1024 * 1024);         // 4MB
    config.insert("wrt-debug".to_string(), 2 * 1024 * 1024);        // 2MB
    config.insert("wrt-platform".to_string(), 8 * 1024 * 1024);     // 8MB
    config.insert("wrt-instructions".to_string(), 4 * 1024 * 1024); // 4MB
    config.insert("wrt-logging".to_string(), 1 * 1024 * 1024);      // 1MB
    config.insert("wrt-intercept".to_string(), 1 * 1024 * 1024);    // 1MB
    config.insert("wrt-panic".to_string(), 512 * 1024);             // 512KB
    config.insert("wrt-sync".to_string(), 1 * 1024 * 1024);         // 1MB
    config.insert("wrt-math".to_string(), 512 * 1024);              // 512KB
    config.insert("wrt-error".to_string(), 256 * 1024);             // 256KB
    config.insert("wrt-helper".to_string(), 256 * 1024);            // 256KB
    
    // Try to load custom configuration from file
    let config_path = Path::new(workspace_root).join("memory_budget.toml");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            // Basic TOML-like parsing for memory budgets
            for line in content.lines() {
                if let Some((key, value)) = parse_budget_line(line) {
                    config.insert(key, value);
                }
            }
        }
    }
    
    // Override with environment variables
    for (crate_name, _) in &config.clone() {
        let env_var = format!("WRT_BUDGET_{}", crate_name.to_uppercase().replace('-', "_"));
        if let Ok(budget_str) = env::var(&env_var) {
            if let Ok(budget) = budget_str.parse::<u64>() {
                config.insert(crate_name.clone(), budget);
                println!("cargo:warning=Using environment budget for {}: {} bytes", crate_name, budget);
            }
        }
    }
    
    Ok(config)
}

/// Parse a budget line from config file
fn parse_budget_line(line: &str) -> Option<(String, u64)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    
    let parts: Vec<&str> = line.split('=').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let key = parts[0].trim().trim_matches('"').to_string();
    let value_str = parts[1].trim().trim_matches('"');
    
    // Parse with unit suffixes (KB, MB, GB)
    let multiplier = if value_str.ends_with("GB") {
        1024 * 1024 * 1024
    } else if value_str.ends_with("MB") {
        1024 * 1024
    } else if value_str.ends_with("KB") {
        1024
    } else {
        1
    };
    
    let numeric_part = value_str.trim_end_matches("GB")
        .trim_end_matches("MB")
        .trim_end_matches("KB");
    
    if let Ok(value) = numeric_part.parse::<u64>() {
        Some((key, value * multiplier))
    } else {
        None
    }
}

/// Validate individual crate budget
fn validate_crate_budget(crate_name: &str, budget: u64) -> Result<(), String> {
    // Minimum budget per crate
    let min_budget = match crate_name {
        "wrt-runtime" | "wrt-component" => 4 * 1024 * 1024,  // 4MB minimum for large crates
        "wrt-foundation" | "wrt-platform" => 2 * 1024 * 1024, // 2MB minimum
        _ => 256 * 1024, // 256KB minimum for others
    };
    
    if budget < min_budget {
        return Err(format!(
            "Budget for {} ({} bytes) is below minimum ({} bytes)",
            crate_name, budget, min_budget
        ));
    }
    
    // Maximum budget per crate
    let max_budget = match crate_name {
        "wrt-runtime" => 32 * 1024 * 1024,      // 32MB max for runtime
        "wrt-component" => 24 * 1024 * 1024,    // 24MB max for component
        "wrt-foundation" => 16 * 1024 * 1024,   // 16MB max for foundation
        "wrt-platform" => 16 * 1024 * 1024,     // 16MB max for platform
        _ => 8 * 1024 * 1024, // 8MB max for others
    };
    
    if budget > max_budget {
        return Err(format!(
            "Budget for {} ({} bytes) exceeds maximum ({} bytes)",
            crate_name, budget, max_budget
        ));
    }
    
    Ok(())
}

/// Validate crate memory configurations
fn validate_crate_memory_configs() -> Result<(), String> {
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "Failed to get workspace root")?;
    
    let crates = [
        "wrt-foundation", "wrt-runtime", "wrt-component", "wrt-decoder",
        "wrt-format", "wrt-host", "wrt-debug", "wrt-platform",
        "wrt-instructions", "wrt-logging", "wrt-intercept", "wrt-panic",
        "wrt-sync", "wrt-math", "wrt-error", "wrt-helper"
    ];
    
    for crate_name in &crates {
        let crate_path = Path::new(&workspace_root).join(crate_name);
        if crate_path.exists() {
            validate_crate_memory_config(&crate_path, crate_name)?;
        }
    }
    
    Ok(())
}

/// Validate individual crate memory configuration
fn validate_crate_memory_config(crate_path: &Path, crate_name: &str) -> Result<(), String> {
    let lib_rs_path = crate_path.join("src").join("lib.rs");
    
    if lib_rs_path.exists() {
        let content = fs::read_to_string(&lib_rs_path)
            .map_err(|e| format!("Failed to read {}/src/lib.rs: {}", crate_name, e))?;
        
        // Check for memory system integration
        let has_memory_init = content.contains("memory_system_initializer") ||
                            content.contains("BudgetAwareProviderFactory") ||
                            content.contains("NoStdProvider");
        
        if !has_memory_init && needs_memory_system(crate_name) {
            println!("cargo:warning={} may need memory system integration", crate_name);
        }
        
        // Check for no_std compatibility
        let has_no_std = content.contains("#![no_std]") ||
                        content.contains("cfg_attr(not(feature = \"std\"), no_std)");
        
        if !has_no_std && should_support_no_std(crate_name) {
            println!("cargo:warning={} may need no_std support", crate_name);
        }
    }
    
    Ok(())
}

/// Check if crate needs memory system integration
fn needs_memory_system(crate_name: &str) -> bool {
    matches!(crate_name, 
        "wrt-foundation" | "wrt-runtime" | "wrt-component" | 
        "wrt-decoder" | "wrt-format" | "wrt-host" | "wrt-platform"
    )
}

/// Check if crate should support no_std
fn should_support_no_std(crate_name: &str) -> bool {
    !matches!(crate_name, "wrt-debug" | "wrt-helper")
}

/// Generate memory configuration constants
fn generate_memory_constants() -> Result<(), String> {
    let out_dir = env::var("OUT_DIR")
        .map_err(|_| "Failed to get OUT_DIR")?;
    
    let budget_config = load_budget_configuration(
        &env::var("CARGO_MANIFEST_DIR").unwrap_or_default()
    )?;
    
    let mut constants = String::new();
    constants.push_str("// Auto-generated memory budget constants\n");
    constants.push_str("// DO NOT EDIT - Generated by build.rs\n\n");
    
    // Generate total budget constant
    let total_budget: u64 = budget_config.values().sum();
    constants.push_str(&format!(
        "pub const TOTAL_MEMORY_BUDGET: usize = {};\n\n",
        total_budget
    ));
    
    // Generate per-crate constants
    constants.push_str("// Per-crate memory budgets\n");
    for (crate_name, budget) in &budget_config {
        let const_name = format!(
            "BUDGET_{}",
            crate_name.to_uppercase().replace('-', "_")
        );
        constants.push_str(&format!(
            "pub const {}: usize = {};\n",
            const_name, budget
        ));
    }
    
    // Generate platform-specific constants
    constants.push_str("\n// Platform-specific memory limits\n");
    constants.push_str("pub const EMBEDDED_MAX_BUDGET: usize = 4 * 1024 * 1024; // 4MB\n");
    constants.push_str("pub const IOT_MAX_BUDGET: usize = 16 * 1024 * 1024; // 16MB\n");
    constants.push_str("pub const DESKTOP_MAX_BUDGET: usize = 128 * 1024 * 1024; // 128MB\n");
    constants.push_str("pub const SERVER_MAX_BUDGET: usize = 512 * 1024 * 1024; // 512MB\n");
    
    // Generate validation timestamp
    constants.push_str(&format!(
        "\n// Validation timestamp: {}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));
    
    // Write constants file
    let constants_path = Path::new(&out_dir).join("memory_constants.rs");
    fs::write(&constants_path, constants)
        .map_err(|e| format!("Failed to write memory constants: {}", e))?;
    
    // Make constants available to build
    println!("cargo:rustc-env=MEMORY_CONSTANTS_PATH={}", constants_path.display());
    
    Ok(())
}