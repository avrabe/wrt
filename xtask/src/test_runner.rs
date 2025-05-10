use crate::Query;
use anyhow::{bail, Context, Result};
use dagger_sdk::HostDirectoryOpts;
use serde::Deserialize;
use std::fs; // Added for file operations
use tracing::info; // Added for TOML deserialization

// Structs for deserializing rust-toolchain.toml (copied from ci_integrity_checks.rs)
#[derive(Deserialize, Debug)]
struct ToolchainConfig {
    toolchain: ToolchainDetails,
}

#[derive(Deserialize, Debug)]
struct ToolchainDetails {
    // channel: String, // Field is never read in this file's context
    version: Option<String>,
}

// Function to read and parse rust-toolchain.toml (copied from ci_integrity_checks.rs)
fn get_rust_version_from_toolchain_file() -> Result<(String, String)> {
    let toml_str =
        fs::read_to_string("rust-toolchain.toml").context("Failed to read rust-toolchain.toml")?;
    let config: ToolchainConfig =
        toml::from_str(&toml_str).context("Failed to parse rust-toolchain.toml")?;
    let version = config
        .toolchain
        .version
        .ok_or_else(|| anyhow::anyhow!("Version not found in rust-toolchain.toml"))?;
    Ok((format!("rust:{}", version), version))
}

struct TestConfig {
    name: String,
    features: Option<String>,
    no_default_features: bool,
}

pub async fn run(client: &Query) -> Result<()> {
    let (rust_image, _toolchain_version) = get_rust_version_from_toolchain_file()
        .context("Failed to determine Rust image from rust-toolchain.toml for test runner")?;
    info!("Using Rust image for test runner: {}", rust_image);

    info!("Starting Daggerized test runner (cargo test with feature configs)...");

    let src_dir = client.host().directory_opts(
        ".",
        HostDirectoryOpts {
            exclude: Some(vec![
                ".git ",
                "target",
                ".vscode ",
                ".idea ",
                ".DS_Store ",
                ".cargo/git ",
                ".cargo/registry ",
                ".zephyr-venv ",
                ".zephyrproject ",
            ]),
            include: None,
        },
    );

    let base_container = client
        .container()
        .from(&rust_image) // Use dynamic rust_image
        .with_mounted_directory("/src ", src_dir)
        .with_workdir("/src ")
        .with_exec(vec!["rustup", "target", "add", "wasm32-wasip2"]);

    // Define test configurations
    // TODO: Adjust these configurations based on your project's actual feature setup.
    let test_configs = vec![
        TestConfig {
            name: "Default features ".to_string(),
            features: None,
            no_default_features: false,
        },
        TestConfig {
            name: "No default features ".to_string(),
            features: None, // Or specify a minimal feature set if needed for no_std
            no_default_features: true,
        },
        // Example: if you have an explicit 'std' feature for a no_std crate
        // TestConfig {
        //     name: "Explicit std feature ".to_string(),
        //     features: Some("std".to_string()),
        //     no_default_features: true
        // },
        // TODO: Add more configs if you have other critical feature combinations to test.
    ];

    let mut overall_success = true;

    for config in test_configs {
        info!("Running tests for config: {}", config.name);
        let mut cargo_test_cmd = vec!["cargo", "test", "--workspace "]; // Start with base command

        if config.no_default_features {
            cargo_test_cmd.push("--no-default-features ");
        }
        if let Some(features_str) = &config.features {
            cargo_test_cmd.push("--features ");
            cargo_test_cmd.push(features_str);
        }
        // Add --all-targets if your tests generally support it, or remove if it causes issues with some feature sets.
        cargo_test_cmd.push("--all-targets");

        info!(command = ?cargo_test_cmd, "Executing test command ");

        let test_run_container = base_container.with_exec(cargo_test_cmd.clone());
        match test_run_container.sync().await {
            Ok(_) => {
                info!("Tests passed for config: {}", config.name);
            }
            Err(e) => {
                info!("ERROR: Tests FAILED for config: {}. Error: {:?}", config.name, e);
                overall_success = false;
                // Decide if you want to stop on first failure or run all configs.
                // For CI, usually any failure means the whole step fails. Here we continue to report all.
            }
        }
    }

    if !overall_success {
        bail!("One or more test configurations failed.");
    }

    info!("Daggerized test runner completed successfully for all configurations.");
    Ok(())
}
