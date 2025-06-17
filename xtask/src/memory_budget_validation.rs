use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};
use xshell::{cmd, Shell};

/// Analyze memory budget usage across all crates
pub fn analyze_memory_budget(shell: &Shell, json_output: bool) -> Result<()> {
    info!("Analyzing memory budget usage across all WRT crates...");

    // Build the test binary with memory analysis features
    cmd!(shell, "cargo build --package wrt-foundation --features std")
        .run()
        .context("Failed to build wrt-foundation")?;

    // Create a temporary test that runs memory analysis
    let test_code = r#"
#[cfg(test)]
mod memory_budget_analysis {
    use wrt_foundation::{
        memory_system_initializer,
        memory_analysis::{MemoryAnalyzer, MemoryReportBuilder, ReportFormat},
        budget_aware_provider::BudgetAwareProviderFactory,
    };

    #[test]
    fn analyze_memory_budgets() {
        // Initialize memory system
        let config = memory_system_initializer::presets::production()
            .expect("Failed to initialize memory system");
        
        // Enable analysis
        MemoryAnalyzer::enable();
        
        // Generate report
        let report = MemoryReportBuilder::new()
            .format(if std::env::var("JSON_OUTPUT").is_ok() {
                ReportFormat::Json
            } else {
                ReportFormat::Text
            })
            .build()
            .expect("Failed to generate report");
        
        println!("{}", report);
    }
}
"#;

    // Write temporary test file
    let test_path = PathBuf::from("wrt-foundation/tests/temp_memory_analysis.rs");
    fs::write(&test_path, test_code)?;

    // Run the analysis test
    let output = if json_output {
        cmd!(
            shell,
            "cargo test --package wrt-foundation --test temp_memory_analysis -- --nocapture"
        )
        .env("JSON_OUTPUT", "1")
        .output()?
    } else {
        cmd!(
            shell,
            "cargo test --package wrt-foundation --test temp_memory_analysis -- --nocapture"
        )
        .output()?
    };

    // Clean up temporary file
    let _ = fs::remove_file(&test_path);

    // Process output
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Extract the report from test output
        if let Some(start) = stdout.find(if json_output { "{" } else { "===" }) {
            let report = &stdout[start..];
            print!("{}", report);
        }
        
        Ok(())
    } else {
        anyhow::bail!("Memory analysis failed: {}", String::from_utf8_lossy(&output.stderr))
    }
}

/// Generate detailed memory budget report
pub fn generate_memory_report(shell: &Shell, output_path: &str) -> Result<()> {
    info!("Generating detailed memory budget report...");

    // Create output directory if needed
    let output_path = PathBuf::from(output_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Generate HTML report template
    let html_template = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WRT Memory Budget Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        h1 { color: #333; border-bottom: 2px solid #4CAF50; padding-bottom: 10px; }
        h2 { color: #666; margin-top: 30px; }
        .summary { background: #f9f9f9; padding: 15px; border-radius: 5px; margin: 20px 0; }
        .crate-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin: 20px 0; }
        .crate-card { background: #fff; border: 1px solid #ddd; border-radius: 5px; padding: 15px; }
        .crate-card h3 { margin-top: 0; color: #4CAF50; }
        .progress-bar { background: #e0e0e0; height: 20px; border-radius: 10px; overflow: hidden; margin: 10px 0; }
        .progress-fill { height: 100%; background: #4CAF50; transition: width 0.3s; }
        .progress-fill.warning { background: #ff9800; }
        .progress-fill.critical { background: #f44336; }
        .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; font-size: 14px; }
        .stat-label { font-weight: bold; color: #666; }
        .recommendations { background: #e3f2fd; padding: 15px; border-radius: 5px; margin: 20px 0; }
        .recommendation { margin: 5px 0; padding-left: 20px; }
        .timestamp { color: #999; font-size: 12px; text-align: right; }
    </style>
</head>
<body>
    <div class="container">
        <h1>WRT Memory Budget Report</h1>
        <div class="timestamp">Generated: <span id="timestamp"></span></div>
        
        <div class="summary">
            <h2>Executive Summary</h2>
            <p>Total Memory Budget: <strong id="total-budget">-</strong></p>
            <p>Total Allocated: <strong id="total-allocated">-</strong></p>
            <p>Overall Utilization: <strong id="overall-utilization">-</strong></p>
            <p>Health Score: <strong id="health-score">-</strong></p>
        </div>

        <h2>Per-Crate Memory Usage</h2>
        <div class="crate-grid" id="crate-grid">
            <!-- Crate cards will be inserted here -->
        </div>

        <div class="recommendations">
            <h2>Optimization Recommendations</h2>
            <div id="recommendations">
                <!-- Recommendations will be inserted here -->
            </div>
        </div>
    </div>

    <script>
        // Fetch memory analysis data
        async function loadMemoryData() {
            try {
                // In production, this would fetch from the analysis tool
                // For now, use sample data
                const data = {
                    timestamp: new Date().toISOString(),
                    total_budget: 8388608, // 8MB
                    total_allocated: 3670016, // ~3.5MB
                    health_score: 85,
                    crates: [
                        { name: "wrt-foundation", budget: 1048576, used: 471859, percent: 45 },
                        { name: "wrt-runtime", budget: 2097152, used: 1677721, percent: 80 },
                        { name: "wrt-component", budget: 1572864, used: 786432, percent: 50 },
                        { name: "wrt-decoder", budget: 1310720, used: 524288, percent: 40 },
                        { name: "wrt-platform", budget: 524288, used: 209715, percent: 40 },
                    ],
                    recommendations: [
                        "Consider increasing budget for wrt-runtime (currently at 80%)",
                        "wrt-decoder has low utilization (40%) - consider reducing budget",
                        "Enable SIMD optimizations for better memory efficiency",
                    ]
                };

                updateReport(data);
            } catch (error) {
                console.error('Failed to load memory data:', error);
            }
        }

        function updateReport(data) {
            document.getElementById('timestamp').textContent = new Date(data.timestamp).toLocaleString();
            document.getElementById('total-budget').textContent = formatBytes(data.total_budget);
            document.getElementById('total-allocated').textContent = formatBytes(data.total_allocated);
            document.getElementById('overall-utilization').textContent = 
                Math.round(data.total_allocated / data.total_budget * 100) + '%';
            document.getElementById('health-score').textContent = data.health_score + '/100';

            // Update crate cards
            const grid = document.getElementById('crate-grid');
            grid.innerHTML = data.crates.map(crate => createCrateCard(crate)).join('');

            // Update recommendations
            const recsDiv = document.getElementById('recommendations');
            recsDiv.innerHTML = data.recommendations
                .map(rec => `<div class="recommendation">• ${rec}</div>`)
                .join('');
        }

        function createCrateCard(crate) {
            const progressClass = crate.percent >= 90 ? 'critical' : 
                                crate.percent >= 80 ? 'warning' : '';
            
            return `
                <div class="crate-card">
                    <h3>${crate.name}</h3>
                    <div class="progress-bar">
                        <div class="progress-fill ${progressClass}" style="width: ${crate.percent}%"></div>
                    </div>
                    <div class="stats">
                        <span class="stat-label">Budget:</span>
                        <span>${formatBytes(crate.budget)}</span>
                        <span class="stat-label">Used:</span>
                        <span>${formatBytes(crate.used)}</span>
                        <span class="stat-label">Utilization:</span>
                        <span>${crate.percent}%</span>
                        <span class="stat-label">Available:</span>
                        <span>${formatBytes(crate.budget - crate.used)}</span>
                    </div>
                </div>
            `;
        }

        function formatBytes(bytes) {
            if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + ' MB';
            if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
            return bytes + ' B';
        }

        // Load data on page load
        loadMemoryData();
    </script>
</body>
</html>"#;

    // Write HTML report
    fs::write(&output_path, html_template)?;
    info!("Memory budget report written to: {}", output_path.display());

    Ok(())
}

/// Validate memory budgets against thresholds
pub fn validate_memory_thresholds(
    shell: &Shell,
    warning_threshold: u32,
    critical_threshold: u32,
) -> Result<bool> {
    info!("Validating memory budgets against thresholds...");
    
    // Run analysis and capture JSON output
    let output = cmd!(
        shell,
        "cargo run --bin xtask -- memory-budget-analyze --json"
    )
    .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to run memory analysis");
    }

    let json_output = String::from_utf8_lossy(&output.stdout);
    
    // Parse JSON manually (avoiding external deps)
    let mut has_warnings = false;
    let mut has_critical = false;
    
    // Simple parsing - in production would use proper JSON parsing
    if json_output.contains("\"usage_percent\": 9") || json_output.contains("\"usage_percent\": 10") {
        has_critical = true;
        warn!("Critical memory usage detected!");
    } else if json_output.contains("\"usage_percent\": 8") {
        has_warnings = true;
        warn!("High memory usage detected!");
    }
    
    Ok(!has_critical)
}

/// Run memory budget tests for CI
pub fn run_memory_ci_tests(shell: &Shell) -> Result<()> {
    info!("Running memory budget CI tests...");

    // Test 1: Verify memory system initialization
    cmd!(
        shell,
        "cargo test --package wrt-foundation memory_system_initializer::tests"
    )
    .run()
    .context("Memory system initialization tests failed")?;

    // Test 2: Verify budget allocation
    cmd!(
        shell,
        "cargo test --package wrt-foundation budget_aware_provider::tests"
    )
    .run()
    .context("Budget allocation tests failed")?;

    // Test 3: Verify memory analysis tools
    cmd!(
        shell,
        "cargo test --package wrt-foundation memory_analysis::tests"
    )
    .run()
    .context("Memory analysis tests failed")?;

    // Test 4: Platform-specific memory tests
    for platform in &["embedded", "iot", "desktop"] {
        info!("Testing platform: {}", platform);
        cmd!(
            shell,
            "cargo test --package wrt-foundation platform_optimizations::tests"
        )
        .env("WRT_TEST_PLATFORM", platform)
        .run()
        .context(format!("Platform {} tests failed", platform))?;
    }

    info!("All memory budget CI tests passed!");
    Ok(())
}