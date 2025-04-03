use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tera::{Context, Tera};
use xshell::Shell;

mod check_imports;
mod fs_ops;
mod wasm_ops;
mod wast_tests;

#[derive(Parser, Debug)]
pub struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run linters and formatters.
    Lint(LintOpts),
    /// Run tests.
    Test(TestOpts),
    /// Build the project.
    Build(BuildOpts),
    /// Run coverage analysis.
    Coverage(CoverageOpts),
    /// Analyze and list demangled symbols for specified crates.
    Symbols(SymbolsOpts),
    /// Check Rust import organization (std/core/alloc first)
    CheckImports {
        #[arg(default_value = "wrt", help = "First directory to check")]
        dir1: PathBuf,
        #[arg(default_value = "wrtd", help = "Second directory to check")]
        dir2: PathBuf,
    },
    /// WebAssembly related tasks
    Wasm(WasmArgs),
    /// Filesystem operations
    Fs(FsArgs),
    /// Run WAST test suite
    RunWastTests {
        #[arg(long, help = "Create/overwrite wast_passed.md and wast_failed.md")]
        create_files: bool,
        #[arg(
            long,
            help = "Only run tests listed in wast_passed.md and error on regressions"
        )]
        verify_passing: bool,
    },
}

#[derive(Args, Debug)]
pub struct LintOpts {
    #[clap(long)]
    fix: bool,
}

#[derive(Args, Debug)]
pub struct TestOpts {
    #[clap(long)]
    coverage: bool,
}

#[derive(Args, Debug)]
pub struct BuildOpts {
    #[clap(long)]
    release: bool,
}

#[derive(Args, Debug)]
pub struct CoverageOpts {
    // Add options for coverage if needed
}

#[derive(Args, Debug)]
pub struct SymbolsOpts {
    /// Package to analyze (e.g., 'wrt').
    #[clap(long, default_value = "wrt")]
    package: String,

    /// Build profile to use.
    #[clap(long, default_value = "release")]
    profile: String,

    /// Target triple for the build (optional).
    #[clap(long)]
    target: Option<String>,

    /// Comma-separated list of features to enable (e.g., 'std').
    #[clap(long, use_value_delimiter = true, value_delimiter = ',')]
    features: Vec<String>,

    /// Output format for the symbol list.
    #[clap(long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Output file path. If not specified, prints to stdout.
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Text,     // Simple list of demangled symbols
    Json,     // Structured data including crate info and symbols
    Markdown, // List formatted as Markdown
    Rst,      // Fragment formatted as reStructuredText for Sphinx
}

#[derive(clap::Args, Debug)]
struct WasmArgs {
    #[command(subcommand)]
    command: WasmCommands,
}

#[derive(clap::Subcommand, Debug)]
enum WasmCommands {
    /// Build all WAT files in the specified directory
    Build {
        #[arg(default_value = "example")]
        directory: PathBuf,
    },
    /// Check if WASM files are up-to-date with their WAT counterparts
    Check {
        #[arg(default_value = "example")]
        directory: PathBuf,
    },
    /// Convert a single WAT file to WASM
    Convert {
        /// Input WAT file path
        wat_file: PathBuf,
    },
}

#[derive(clap::Args, Debug)]
struct FsArgs {
    #[command(subcommand)]
    command: FsCommands,
}

#[derive(clap::Subcommand, Debug)]
enum FsCommands {
    /// Remove a directory or file recursively (like rm -rf)
    RmRf { path: PathBuf },
    /// Create a directory, including parent directories (like mkdir -p)
    MkdirP { path: PathBuf },
    /// Find files matching a pattern and delete them
    FindDelete {
        /// Starting directory
        directory: PathBuf,
        /// Filename pattern (e.g., *.wasm)
        pattern: String,
    },
    /// Count files matching a pattern in a directory
    CountFiles {
        /// Starting directory
        directory: PathBuf,
        /// Filename pattern (e.g., plantuml-*)
        pattern: String,
    },
    /// Report the size of a file in bytes
    FileSize { path: PathBuf },
    /// Copy a file
    Cp {
        source: PathBuf,
        destination: PathBuf,
    },
}

// Structs for deserializing cargo --message-format=json output
#[derive(Deserialize, Debug)]
struct CargoMessage {
    reason: String,
    package_id: Option<String>,
    target: Option<CargoTarget>,
    profile: Option<CargoProfile>,
    filenames: Option<Vec<PathBuf>>,
}

#[derive(Deserialize, Debug)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
    crate_types: Vec<String>,
    src_path: PathBuf,
}

#[derive(Deserialize, Debug)]
struct CargoProfile {
    opt_level: String,
    debuginfo: Option<u32>,
    debug_assertions: bool,
    overflow_checks: bool,
    test: bool,
}

// Structs for deserializing cargo metadata output
#[derive(Deserialize, Debug)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Option<Resolve>,
    workspace_root: PathBuf,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    version: String,
    id: String, // Format: "pkg_name version (path)"
    features: Option<serde_json::Value>, // Can be complex, might need refinement
                // Add other fields if needed (e.g., dependencies)
}

#[derive(Deserialize, Debug)]
struct Resolve {
    root: Option<String>, // Package ID of the root package
    nodes: Vec<ResolveNode>,
}

#[derive(Deserialize, Debug)]
struct ResolveNode {
    id: String, // Package ID
    features: Option<Vec<String>>, // Resolved features for this node
                // Add other fields if needed (e.g., deps)
}

// Struct for JSON and template output
#[derive(Serialize, Debug)]
struct SymbolOutput {
    package_name: String,
    version: Option<String>,
    features: Vec<String>,
    symbol_count: usize,
    symbols: Vec<String>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let sh = Shell::new()?;

    match opts.command {
        Command::Lint(opts) => run_lint(&sh, opts)?,
        Command::Test(opts) => run_test(&sh, opts)?,
        Command::Build(opts) => run_build(&sh, opts)?,
        Command::Coverage(opts) => run_coverage(&sh, opts)?,
        Command::Symbols(opts) => run_symbols(&sh, opts)?,
        Command::CheckImports { dir1, dir2 } => check_imports::run(&[&dir1, &dir2])?,
        Command::Wasm(args) => match args.command {
            WasmCommands::Build { directory } => wasm_ops::build_all_wat(&directory)?,
            WasmCommands::Check { directory } => wasm_ops::check_all_wat(&directory)?,
            WasmCommands::Convert { wat_file } => {
                let wasm_file = wasm_ops::wat_to_wasm_path(&wat_file)?;
                wasm_ops::convert_wat(&wat_file, &wasm_file, false)?;
            }
        },
        Command::Fs(args) => match args.command {
            FsCommands::RmRf { path } => fs_ops::rmrf(&path)?,
            FsCommands::MkdirP { path } => fs_ops::mkdirp(&path)?,
            FsCommands::FindDelete { directory, pattern } => {
                fs_ops::find_delete(&directory, &pattern)?
            }
            FsCommands::CountFiles { directory, pattern } => {
                fs_ops::count_files(&directory, &pattern)?
            }
            FsCommands::FileSize { path } => fs_ops::file_size(&path)?,
            FsCommands::Cp {
                source,
                destination,
            } => fs_ops::copy_file(&source, &destination)?,
        },
        Command::RunWastTests {
            create_files,
            verify_passing,
        } => wast_tests::run(create_files, verify_passing)?,
    }

    Ok(())
}

fn run_lint(sh: &Shell, opts: LintOpts) -> Result<()> {
    println!("Running lint checks...");
    if opts.fix {
        sh.cmd("cargo").arg("fmt").run()?;
        sh.cmd("cargo")
            .arg("clippy")
            .arg("--fix")
            .arg("--allow-dirty")
            .run()?;
    } else {
        sh.cmd("cargo").arg("fmt").arg("--check").run()?;
        sh.cmd("cargo").arg("clippy").run()?;
    }
    Ok(())
}

fn run_test(sh: &Shell, opts: TestOpts) -> Result<()> {
    println!("Running tests...");
    if opts.coverage {
        sh.cmd("cargo")
            .arg("llvm-cov")
            .arg("--all-features")
            .arg("test")
            .arg("--lcov")
            .arg("--output-path")
            .arg("coverage.lcov")
            .run()?;
        println!("Coverage report generated at coverage.lcov");
    } else {
        sh.cmd("cargo").arg("test").run()?;
    }
    Ok(())
}

fn run_build(sh: &Shell, opts: BuildOpts) -> Result<()> {
    println!("Building project...");
    let mut cmd = sh.cmd("cargo").arg("build");
    if opts.release {
        cmd = cmd.arg("--release");
    }
    cmd.run()?;
    Ok(())
}

fn run_coverage(sh: &Shell, _opts: CoverageOpts) -> Result<()> {
    let lcov_path = PathBuf::from("coverage.lcov");
    let html_output_dir = PathBuf::from("target/llvm-cov/html");
    let summary_rst_path = PathBuf::from("docs/source/_generated_coverage_summary.rst");

    // 1. Generate LCOV data
    println!("Generating LCOV coverage data...");
    sh.cmd("cargo")
        .arg("llvm-cov")
        .arg("test") // Run tests to generate coverage
        .arg("--all-features")
        .arg("--lcov")
        .arg("--output-path")
        .arg(&lcov_path)
        .run()?;
    println!("LCOV data generated at {}", lcov_path.display());

    if !lcov_path.exists() {
        anyhow::bail!("LCOV file was not generated: {}", lcov_path.display());
    }

    // 2. Generate HTML report from LCOV data
    println!("Generating HTML coverage report from LCOV data...");
    // Ensure the target directory exists
    fs_ops::mkdirp(&html_output_dir.parent().unwrap())?; // Create target/llvm-cov if needed
    sh.cmd("cargo")
        .arg("llvm-cov")
        .arg("report") // Use report subcommand
        .arg("--lcov")
        .arg(&lcov_path) // Input LCOV
        .arg("--html")
        .arg("--output-dir") // Specify output directory
        .arg(&html_output_dir)
        .run()?; // removed .arg("test") - we are reporting, not testing again
    println!("HTML report generated in {}", html_output_dir.display());

    // 3. Parse LCOV data for summary
    println!("Parsing LCOV data for summary...");
    let file = File::open(&lcov_path)
        .with_context(|| format!("Failed to open LCOV file: {}", lcov_path.display()))?;
    let reader = BufReader::new(file);

    let mut total_lines = 0u64;
    let mut covered_lines = 0u64;

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("LF:") {
            // Lines Found
            if let Ok(count) = line[3..].parse::<u64>() {
                total_lines = total_lines.saturating_add(count);
            }
        } else if line.starts_with("LH:") {
            // Lines Hit
            if let Ok(count) = line[3..].parse::<u64>() {
                covered_lines = covered_lines.saturating_add(count);
            }
        }
    }

    let percentage = if total_lines > 0 {
        (covered_lines as f64 / total_lines as f64) * 100.0
    } else {
        100.0 // Or 0.0 if preferred for no lines found
    };

    println!(
        "Coverage Summary: {} / {} lines covered ({:.2}%)",
        covered_lines, total_lines, percentage
    );

    // 4. Generate RST summary file
    println!(
        "Generating RST summary file: {}",
        summary_rst_path.display()
    );
    let rst_content = format!(
        ".. container:: coverage-summary

   **Code Coverage:** {:.2}% ({}/{} lines)

   `Full HTML Report <../_static/coverage/index.html>`_
",
        percentage, covered_lines, total_lines
    );

    // Ensure parent directory exists for the RST file
    if let Some(parent) = summary_rst_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create directory for RST file: {}",
                parent.display()
            )
        })?;
    }
    fs::write(&summary_rst_path, rst_content).with_context(|| {
        format!(
            "Failed to write RST summary file: {}",
            summary_rst_path.display()
        )
    })?;
    println!("RST summary file generated successfully.");

    Ok(())
}

fn run_symbols(sh: &Shell, opts: SymbolsOpts) -> Result<()> {
    println!(
        "Running symbol analysis for package '{}' with profile '{}'...",
        opts.package, opts.profile
    );
    if !opts.features.is_empty() {
        println!("Features: {}", opts.features.join(","));
    }
    if let Some(target) = &opts.target {
        println!("Target: {}", target);
    }

    // 1. Build the target crate and capture JSON output
    println!("Building target with JSON output...");
    let mut build_cmd_args = vec![
        "build",
        "--message-format=json",
        "--lib",
        "--package",
        &opts.package,
        "--profile",
        &opts.profile,
    ];

    if let Some(target) = &opts.target {
        build_cmd_args.extend_from_slice(&["--target", target]);
    }

    let feature_string;
    if !opts.features.is_empty() {
        feature_string = opts.features.join(",");
        build_cmd_args.extend_from_slice(&["--features", &feature_string]);
    }

    // Use std::process::Command to capture output easily
    let output = StdCommand::new("cargo").args(&build_cmd_args).output()?; // Use std::process::Command::output

    if !output.status.success() {
        eprintln!("Cargo build failed!");
        eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Cargo build failed");
    }

    println!("Build complete.");

    // 2. Find the build artifact from JSON messages
    let mut artifact_path: Option<PathBuf> = None;
    let stdout = String::from_utf8(output.stdout)?;

    for line in stdout.lines() {
        match serde_json::from_str::<CargoMessage>(line) {
            Ok(message) => {
                if message.reason == "compiler-artifact"
                    && message.target.as_ref().map_or(false, |t| {
                        t.name == opts.package && t.kind.contains(&"lib".to_string())
                    })
                {
                    if let Some(filenames) = message.filenames {
                        // Find the .rlib file
                        if let Some(rlib) = filenames
                            .into_iter()
                            .find(|f| f.extension().map_or(false, |ext| ext == "rlib"))
                        {
                            artifact_path = Some(rlib);
                            break; // Found the artifact we need
                        }
                    }
                }
            }
            Err(e) => {
                // Ignore lines that aren't valid JSON messages (like notes from cargo)
                // eprintln!("Failed to parse cargo JSON message: {}\nLine: {}", e, line);
            }
        }
    }

    let artifact_path = match artifact_path {
        Some(path) => path,
        None => anyhow::bail!(
            "Could not find .rlib artifact for package '{}' in build output",
            opts.package
        ),
    };

    println!("Found artifact: {}", artifact_path.display());

    // 3. Extract mangled symbols (using nm or similar)
    println!("Extracting symbols using nm...");
    let nm_output = StdCommand::new("nm")
        .arg("-g") // Show only global/external symbols
        .arg(&artifact_path)
        .output()?;

    if !nm_output.status.success() {
        eprintln!("nm command failed!");
        eprintln!("Stderr: {}", String::from_utf8_lossy(&nm_output.stderr));
        anyhow::bail!("nm command failed for artifact {}", artifact_path.display());
    }

    let nm_stdout = String::from_utf8(nm_output.stdout)?;
    let mut mangled_symbols: Vec<String> = Vec::new();

    for line in nm_stdout.lines() {
        // Basic parsing: Look for lines with format like "<address> T <symbol>"
        // or potentially just "<symbol>" if no address/type info is present (less common for .rlib)
        // Rust symbols often start with _ZN, _RNv, __ZN, etc.
        let parts: Vec<&str> = line.split_whitespace().collect();
        let symbol_name = if parts.len() >= 3
            && (parts[1] == "T"
                || parts[1] == "t"
                || parts[1] == "D"
                || parts[1] == "d"
                || parts[1] == "R"
                || parts[1] == "r")
        {
            // Format: <addr> <type> <symbol>
            parts[2]
        } else if parts.len() == 1 {
            // Sometimes just the symbol name might be listed
            parts[0]
        } else {
            continue; // Skip lines that don't match expected formats
        };

        // Filter for likely Rust mangled symbols (can be refined)
        if symbol_name.starts_with("_ZN")
            || symbol_name.starts_with("_RNv")
            || symbol_name.starts_with("__ZN")
            || symbol_name.starts_with("$S")
            || symbol_name.starts_with("$s")
        {
            mangled_symbols.push(symbol_name.to_string());
        }
    }

    // Deduplicate (nm might list symbols multiple times)
    mangled_symbols.sort_unstable();
    mangled_symbols.dedup();

    println!(
        "Extracted {} potential mangled symbols.",
        mangled_symbols.len()
    );

    // 4. Demangle symbols
    println!("Demangling symbols...");
    let demangled_symbols: Vec<String> = mangled_symbols
        .iter()
        .map(|mangled| rustc_demangle::demangle(mangled).to_string())
        // Optionally filter out symbols that didn't demangle well or are compiler internal
        // .filter(|demangled| !demangled.starts_with("_"))
        .collect();

    println!("Demangled {} symbols.", demangled_symbols.len());

    // 5. Gather metadata (if needed for JSON/doc/graph)
    let mut crate_version: Option<String> = None;
    let mut crate_features: Vec<String> = Vec::new();

    // Only gather metadata if needed for specific outputs
    if opts.format == OutputFormat::Json {
        println!("Gathering crate metadata...");
        let metadata_output = StdCommand::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            // Optionally add --filter-platform if a target is specified
            // and --features / --no-default-features matching the build?
            // For now, just get general metadata and find our package.
            .output()?;

        if !metadata_output.status.success() {
            eprintln!("cargo metadata failed!");
            eprintln!(
                "Stderr: {}",
                String::from_utf8_lossy(&metadata_output.stderr)
            );
            anyhow::bail!("cargo metadata failed");
        }

        let metadata: Metadata = serde_json::from_slice(&metadata_output.stdout)?;

        // Find the target package in the metadata
        if let Some(pkg) = metadata.packages.iter().find(|p| p.name == opts.package) {
            crate_version = Some(pkg.version.clone());

            // Find resolved features for this package in the resolve graph
            if let Some(resolve) = metadata.resolve {
                if let Some(node) = resolve.nodes.iter().find(|n| n.id.starts_with(&pkg.id)) {
                    // Match by Package ID prefix
                    if let Some(features) = &node.features {
                        crate_features = features.clone();
                    }
                }
            }
            println!(
                "Found metadata: Version={}, Features={:?}",
                crate_version.as_deref().unwrap_or("N/A"),
                crate_features
            );
        } else {
            eprintln!(
                "Warning: Could not find package '{}' in cargo metadata output.",
                opts.package
            );
        }
    }

    // 6. Format and output results
    println!("Formatting and writing output...");

    // Prepare structured data (used for JSON and potentially doc)
    let output_data = SymbolOutput {
        package_name: opts.package.clone(),
        version: crate_version.clone(),
        features: crate_features.clone(),
        symbol_count: demangled_symbols.len(),
        symbols: demangled_symbols.clone(), // Clone symbols for output data
    };

    // Determine if JSON output is needed
    let needs_json = opts.format == OutputFormat::Json;

    // --- Handle Output ---
    let mut output_writer: Box<dyn Write> = match &opts.output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(std::io::stdout()),
    };

    if needs_json {
        // Output as JSON
        let json_string = serde_json::to_string_pretty(&output_data)?;
        writeln!(output_writer, "{}", json_string)?;
        println!("Output written as JSON.");
    } else {
        // Format and write output
        println!("Formatting output as {:?}...", opts.format);
        let output_content = match opts.format {
            OutputFormat::Text => {
                // Simple text output: one symbol per line
                output_data.symbols.join("\n")
            }
            OutputFormat::Json => {
                // JSON output (potentially for graphs or structured data use)
                // Pretty print JSON for readability
                serde_json::to_string_pretty(&output_data)?
            }
            OutputFormat::Markdown => {
                // Markdown output
                let mut md = format!("# Symbols for `{}`\n\n", output_data.package_name);
                if let Some(v) = &output_data.version {
                    md.push_str(&format!("Version: `{}`\n", v));
                }
                if !output_data.features.is_empty() {
                    md.push_str(&format!(
                        "Features: `{}`\n",
                        output_data.features.join(", ")
                    ));
                }
                md.push_str(&format!("Symbol Count: {}\n\n", output_data.symbol_count));
                md.push_str("## Demangled Symbols:\n\n");
                for symbol in &output_data.symbols {
                    md.push_str(&format!("- `{}`\n", symbol));
                }
                md
            }
            OutputFormat::Rst => {
                // Use Tera for RST generation (similar to previous doc_data logic)
                let tera = match Tera::new("xtask/templates/**/*.tera") {
                    Ok(t) => t,
                    Err(e) => anyhow::bail!("Failed to load Tera templates: {}", e),
                };
                let mut context = Context::new();
                context.insert("pkg", &output_data); // Pass the whole struct

                match tera.render("symbols.rst.tera", &context) {
                    Ok(rendered) => rendered,
                    Err(e) => anyhow::bail!("Failed to render RST template: {}", e),
                }
            }
        };

        // Write to file or stdout
        if let Some(output_path) = opts.output {
            println!("Writing output to {}...", output_path.display());
            // Ensure parent directory exists
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = File::create(&output_path)?;
            file.write_all(output_content.as_bytes())?;
            println!("Output written successfully.");
        } else {
            // Print to stdout if no output file is specified
            println!("\n--- Symbol Output ---");
            println!("{}", output_content);
            println!("--- End Symbol Output ---");
        }
    }

    println!("Symbol analysis complete.");

    Ok(())
}
