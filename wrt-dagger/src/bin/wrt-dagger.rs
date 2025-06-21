#!/usr/bin/env cargo run --bin wrt-dagger --

//! # wrt-dagger - Containerized Build CLI
//!
//! Command-line interface for running WRT builds in Dagger containers.
//! This tool provides a convenient way to execute cargo-wrt commands
//! in isolated, reproducible container environments.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use wrt_dagger::{ContainerConfig, ContainerConfigBuilder, DaggerPipeline, utils};

#[derive(Parser)]
#[command(name = "wrt-dagger")]
#[command(about = "Containerized build tool for WRT using Dagger")]
#[command(version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Use verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Container base image
    #[arg(long, default_value = "ubuntu:22.04")]
    base_image: String,

    /// Rust version to use
    #[arg(long, default_value = "1.86.0")]
    rust_version: String,

    /// Build timeout in seconds
    #[arg(long, default_value = "3600")]
    timeout: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Build all WRT components in container
    Build,
    
    /// Run tests in container
    Test,
    
    /// Run full CI pipeline in container
    Ci,
    
    /// Run safety verification in container
    Verify {
        /// ASIL level (qm, a, b, c, d)
        #[arg(long, default_value = "c")]
        asil: String,
    },
    
    /// Generate code coverage in container
    Coverage,
    
    /// Run custom cargo-wrt command in container
    Run {
        /// cargo-wrt arguments
        args: Vec<String>,
    },
    
    /// Show container configuration
    Config,
    
    /// Run pre-configured CI pipeline
    CiPipeline,
    
    /// Run pre-configured development pipeline
    DevPipeline,
    
    /// Run pre-configured safety verification pipeline
    SafetyPipeline,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    // Create container configuration
    let config = ContainerConfigBuilder::new()
        .base_image(&cli.base_image)
        .rust_version(&cli.rust_version)
        .timeout(cli.timeout)
        .build();

    match cli.command {
        Commands::Build => {
            println!("🐋 Building WRT in container...");
            let pipeline = DaggerPipeline::new(config).await?;
            let output = pipeline.build().await?;
            println!("{}", output);
        }

        Commands::Test => {
            println!("🧪 Running tests in container...");
            let pipeline = DaggerPipeline::new(config).await?;
            let output = pipeline.test().await?;
            println!("{}", output);
        }

        Commands::Ci => {
            println!("🚀 Running CI pipeline in container...");
            let pipeline = DaggerPipeline::new(config).await?;
            let output = pipeline.ci().await?;
            println!("{}", output);
        }

        Commands::Verify { asil } => {
            println!("🔒 Running ASIL-{} verification in container...", asil.to_uppercase());
            let pipeline = DaggerPipeline::new(config).await?;
            let output = pipeline.verify(&asil).await?;
            println!("{}", output);
        }

        Commands::Coverage => {
            println!("📊 Generating coverage in container...");
            let pipeline = DaggerPipeline::new(config).await?;
            let output = pipeline.coverage().await?;
            println!("{}", output);
        }

        Commands::Run { args } => {
            println!("⚡ Running custom cargo-wrt command in container...");
            let pipeline = DaggerPipeline::new(config).await?;
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let output = pipeline.run_cargo_wrt(&args_str).await?;
            println!("{}", output);
        }

        Commands::Config => {
            println!("📋 Container Configuration:");
            let json = serde_json::to_string_pretty(&config)
                .context("Failed to serialize config")?;
            println!("{}", json);
        }

        Commands::CiPipeline => {
            println!("🏭 Running optimized CI pipeline...");
            let pipeline = utils::ci_pipeline().await?;
            let output = pipeline.ci().await?;
            println!("{}", output);
        }

        Commands::DevPipeline => {
            println!("💻 Running development pipeline...");
            let pipeline = utils::dev_pipeline().await?;
            let output = pipeline.build().await?;
            println!("{}", output);
        }

        Commands::SafetyPipeline => {
            println!("🛡️ Running safety verification pipeline...");
            let pipeline = utils::safety_pipeline().await?;
            let output = pipeline.verify("d").await?;
            println!("{}", output);
        }
    }

    Ok(())
}