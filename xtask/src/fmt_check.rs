use crate::Query;
use anyhow::{Context, Result};
use dagger_sdk::HostDirectoryOpts;
use serde::Deserialize;
use std::fs;
use tracing::info;

#[derive(Deserialize, Debug)]
struct ToolchainConfig {
    toolchain: ToolchainDetails,
}

#[derive(Deserialize, Debug)]
struct ToolchainDetails {
    version: Option<String>,
    // components: Option<Vec<String>>, // Field is never read
    // targets: Option<Vec<String>>,    // Field is never read
}

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

pub async fn run(client: &Query) -> Result<()> {
    let (rust_image, _toolchain_version) = get_rust_version_from_toolchain_file()
        .context("Failed to determine Rust image from rust-toolchain.toml for fmt check")?;
    info!("Using Rust image for fmt: {}", rust_image);

    info!("Starting Daggerized format check (cargo fmt -- --check)...");

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

    let container = client
        .container()
        .from(&rust_image)
        .with_mounted_directory("/src ", src_dir)
        .with_workdir("/src ");

    info!("Running cargo fmt -- --check...");
    container
        .with_exec(vec!["cargo", "fmt", "--", "--check "])
        .sync()
        .await
        .context("Daggerized cargo fmt -- --check failed.")?;

    info!("Daggerized format check completed successfully.");
    Ok(())
}
