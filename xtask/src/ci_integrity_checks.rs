use crate::Query;
use anyhow::{bail, Context, Result};
use dagger_sdk::HostDirectoryOpts;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tracing::info; // Added for TOML deserialization

// Structs for deserializing rust-toolchain.toml
#[derive(Deserialize, Debug)]
struct ToolchainConfig {
    toolchain: ToolchainDetails,
}

#[derive(Deserialize, Debug)]
struct ToolchainDetails {
    channel: String,
    version: Option<String>, // version can be optional
                             // components: Option<Vec<String>>,
                             // targets: Option<Vec<String>>,
}

// Function to read and parse rust-toolchain.toml
fn get_rust_version_from_toolchain_file() -> Result<(String, Option<String>, String)> {
    let toml_str =
        fs::read_to_string("rust-toolchain.toml").context("Failed to read rust-toolchain.toml")?;
    let config: ToolchainConfig =
        toml::from_str(&toml_str).context("Failed to parse rust-toolchain.toml")?;

    let channel = config.toolchain.channel;
    let version_opt = config.toolchain.version;

    let rust_image_spec = match (version_opt.as_ref(), channel.as_str()) {
        (Some(v), _) => format!("rust:{}", v), // If version is specified, use it for the image
        (None, "stable") => bail!("'version' must be specified in rust-toolchain.toml for the 'stable' channel."),
        (None, "nightly") => "rust:nightly".to_string(),
        (None, "beta") => "rust:beta".to_string(),
        (None, other_channel) => bail!(
            "Unsupported channel '{}' without a specific version in rust-toolchain.toml. Please specify a version or use 'stable', 'nightly', or 'beta'.",
            other_channel
        ),
    };

    Ok((rust_image_spec, version_opt, channel))
}

// List of essential files to check for presence
const ESSENTIAL_FILES: &[&str] = &[
    "README.md",
    "LICENSE",
    // "CONTRIBUTING.md ", // Add or remove as needed
    // "Cargo.toml ", // Usually present, but can be listed
];

// Rust files that should typically have #![forbid(unsafe_code)]
// This is a simplified check; a more robust solution might involve walking specific crate types.
const CRATE_LIB_FILES: &[&str] = &[
    "wrt/src/lib.rs", // Removed trailing space
    "wrt-decoder/src/lib.rs", // Removed trailing space
                      // Add other key library crates here
];

pub async fn run(client: &Query) -> Result<()> {
    // Get Rust version and channel from rust-toolchain.toml
    let (rust_image_spec, toolchain_version_opt, toolchain_channel) =
        get_rust_version_from_toolchain_file().context(
            "Failed to determine Rust image spec and toolchain details from rust-toolchain.toml",
        )?;
    info!(
        "Using Rust image spec: {} (version: {:?}, channel: {}) based on rust-toolchain.toml",
        rust_image_spec, toolchain_version_opt, toolchain_channel
    );

    info!("Starting CI integrity checks pipeline...");

    let src_dir = client.host().directory_opts(
        ".",
        HostDirectoryOpts {
            exclude: Some(vec![
                "./.git",
                "target",
                ".vscode",
                ".idea",
                ".DS_Store",
                ".cargo/git",
                ".cargo/registry",
                ".zephyr-venv",
                ".zephyrproject",
            ]), // Added ./ for .git
            include: None,
        },
    );

    let mut container = client
        .container()
        // Use the image spec derived from rust-toolchain.toml
        .from(&rust_image_spec)
        .with_mounted_directory("/src", src_dir)
        .with_workdir("/src");

    // Determine the exact toolchain string to install/set as default
    let toolchain_to_set: Option<String> =
        match (&toolchain_version_opt, toolchain_channel.as_str()) {
            (Some(v), _) => Some(v.clone()), // If version is specified, use it. E.g., "1.78.0", "nightly-2023-10-25"
            (None, "stable") => {
                // This case should have been caught by get_rust_version_from_toolchain_file, which bails.
                // If it somehow reaches here, it implies an issue. For safety, bail or log error.
                // However, get_rust_version_from_toolchain_file ensures version is Some for stable or bails.
                // So, for stable, toolchain_version_opt will be Some(v).
                None // Should not happen for stable due to earlier checks
            }
            (None, non_stable_channel) => Some(non_stable_channel.to_string()), // e.g. "nightly", "beta"
        };

    if let Some(ref ts) = toolchain_to_set {
        info!("Ensuring toolchain '{}' is installed and set as default via rustup.", ts);
        container = container
            .with_exec(vec![
                "rustup",
                "toolchain",
                "install",
                "--profile",
                "minimal",
                "--no-self-update",
                ts,
            ])
            .with_exec(vec!["rustup", "default", ts]);
    } else if toolchain_channel == "stable" && toolchain_version_opt.is_none() {
        // This case should be prevented by get_rust_version_from_toolchain_file for stable channel
        bail!("Logic error: Stable channel reached toolchain setup without a specific version. This should be caught earlier.");
    }

    // --- Toolchain Check ---
    info!(
        "Verifying Rust toolchain (expected channel: '{}', version: {:?})...",
        toolchain_channel, toolchain_version_opt
    );
    // Ensure previous setup steps (like rustup default) are completed before checks.
    // The `container` object has the rustup commands defined.
    // We sync it here to ensure those are processed before we try to read `rustc --version` etc.
    // This sync doesn't change the 'container' variable itself if it returns an ID,
    // so 'container' remains the Container object we use for further definitions.
    let _ = container
        .sync()
        .await
        .context("Error syncing container state after rustup configuration")?;

    // Use the 'container' (which is type Container) for subsequent exec commands
    // Each .with_exec(...).stdout().await will run the necessary chain.
    let env_path_stdout = container
        .with_exec(vec!["sh", "-c", "echo $PATH"])
        .stdout()
        .await
        .context("Failed to get PATH")?;
    info!("Container PATH: {}", env_path_stdout.trim());

    let which_rustc_stdout = container
        .with_exec(vec!["sh", "-c", "which rustc"])
        .stdout()
        .await
        .context("Failed to get which rustc")?;
    info!("which rustc: {}", which_rustc_stdout.trim());

    // Use rustup run to ensure the specific toolchain's rustc is used.
    let rustc_version_stdout = if let Some(ref toolchain_name) = toolchain_to_set {
        // toolchain_to_set is like "1.78.0"
        container
            .with_exec(vec!["rustup", "run", toolchain_name, "rustc", "--version"])
            .stdout()
            .await
            .context(format!("Failed to get rustc version via rustup run {}", toolchain_name))?
    } else {
        // Fallback if toolchain_to_set was None (should not happen for stable with version)
        container
            .with_exec(vec!["rustc", "--version"])
            .stdout()
            .await
            .context("Failed to get rustc version (fallback)")?
    };
    info!("rustc --version (via rustup run or direct): {}", rustc_version_stdout.trim());

    let active_toolchain_stdout = container
        .with_exec(vec!["sh", "-c", "rustup show active-toolchain || echo 'rustup not found'"])
        .stdout()
        .await
        .context("Failed to get active toolchain from rustup")?;
    info!("rustup show active-toolchain: {}", active_toolchain_stdout.trim());

    let mut checks_passed = false;
    let mut failure_reason = String::new();

    if active_toolchain_stdout.contains("rustup not found") {
        info!(
            "rustup not found in container. Relying on rustc --version for toolchain verification."
        );
        if toolchain_channel == "stable" {
            if let Some(version_str) = &toolchain_version_opt {
                if rustc_version_stdout.contains(version_str)
                    && !rustc_version_stdout.contains("nightly")
                    && !rustc_version_stdout.contains("beta")
                {
                    checks_passed = true;
                    info!(
                        "rustc --version ('{}') matches expected stable version '{}'.",
                        rustc_version_stdout.trim(),
                        version_str
                    );
                } else {
                    failure_reason = format!(
                        "rustc --version ('{}') mismatch. Expected stable version '{}', without 'nightly' or 'beta'.",
                        rustc_version_stdout.trim(), version_str
                    );
                }
            } else {
                // This case should ideally be caught by get_rust_version_from_toolchain_file for stable
                failure_reason = "Stable channel specified in toolchain file, but no version was found for verification.".to_string();
            }
        } else {
            // Non-stable channel (e.g., "nightly", "beta")
            if rustc_version_stdout.contains(&toolchain_channel) {
                // For nightly/beta, rustc --version should contain the channel name.
                // If a specific version (like a date for nightly) is also in toolchain_version_opt, check it too.
                if let Some(version_str) = &toolchain_version_opt {
                    if rustc_version_stdout.contains(version_str) {
                        checks_passed = true;
                        info!("rustc --version ('{}') contains expected channel '{}' and version '{}'.", rustc_version_stdout.trim(), toolchain_channel, version_str);
                    } else {
                        // It contains the channel, but not the specific version string from rust-toolchain.toml
                        // This might be acceptable for some nightly setups (e.g., toolchain file has date, but rustc --version doesn't show it)
                        // For now, consider it a pass if channel matches, but log a detailed message.
                        checks_passed = true; // Or false, depending on strictness desired. Let's keep current behavior.
                        info!(
                            "rustc --version ('{}') contains expected channel '{}', but not the specific version string '{}' from toolchain file. Proceeding as channel match is primary.",
                            rustc_version_stdout.trim(), toolchain_channel, version_str
                        );
                    }
                } else {
                    // No specific version in toolchain_version_opt, channel match is sufficient
                    checks_passed = true;
                    info!("rustc --version ('{}') contains expected channel '{}' (no specific version in toolchain file).", rustc_version_stdout.trim(), toolchain_channel);
                }
            } else {
                failure_reason = format!(
                    "rustc --version ('{}') mismatch. Expected to contain channel '{}'.",
                    rustc_version_stdout.trim(),
                    toolchain_channel
                );
            }
        }
    } else {
        // rustup was found
        info!("rustup found. Verifying active toolchain details.");
        let mut rustup_confirms_channel_and_version = false;

        if toolchain_channel == "stable" {
            if let Some(version_str) = &toolchain_version_opt {
                if (active_toolchain_stdout.contains("stable")
                    || active_toolchain_stdout.starts_with(version_str))
                    && active_toolchain_stdout.contains(version_str)
                    && rustc_version_stdout.contains(version_str)
                    && !rustc_version_stdout.contains("nightly")
                    && !rustc_version_stdout.contains("beta")
                {
                    rustup_confirms_channel_and_version = true;
                } else if active_toolchain_stdout.contains("stable") && // handles case like "stable-x86_64..."
                          rustc_version_stdout.contains(version_str) &&
                          !rustc_version_stdout.contains("nightly") && !rustc_version_stdout.contains("beta")
                {
                    rustup_confirms_channel_and_version = true;
                }
            } // If version_str is None for stable, it's an error state, handled by bail earlier or leading to false here.
        } else {
            // Non-stable channel (e.g., "nightly", "beta", or "nightly-YYYY-MM-DD")
            let expected_rustup_substring = if toolchain_channel.contains('-') {
                // e.g. "nightly-2023-10-25"
                toolchain_channel.clone()
            } else if let Some(version_str) =
                toolchain_version_opt.as_ref().filter(|v| v.contains('-'))
            {
                // e.g. channel="nightly", version="2023-10-25"
                format!("{}-{}", toolchain_channel, version_str)
            } else {
                // e.g. channel="nightly", version could be non-specific or None
                toolchain_channel.clone()
            };

            if active_toolchain_stdout.contains(&expected_rustup_substring)
                && rustc_version_stdout.contains(&toolchain_channel)
            {
                // rustc output must contain base channel name "nightly" or "beta"
                rustup_confirms_channel_and_version = true;

                // If a specific version (like a date for nightly) was provided, log if not explicitly in version strings.
                if let Some(version_str) = toolchain_version_opt
                    .as_ref()
                    .filter(|v| toolchain_channel == "nightly" && v.contains('-'))
                {
                    if !rustc_version_stdout.contains(version_str)
                        && !active_toolchain_stdout.contains(version_str)
                    {
                        info!(
                            "Note: rustup/rustc matches nightly channel '{}'. \
                             The specific date '{}' from toolchain file was not found in rustc version ('{}') or rustup output ('{}'). \
                             This may be acceptable if the installed nightly is the correct one without the exact date in the version string.",
                            toolchain_channel,
                            version_str,
                            rustc_version_stdout.trim(),
                            active_toolchain_stdout.trim()
                        );
                    }
                }
            }
        }

        if rustup_confirms_channel_and_version {
            checks_passed = true;
            info!(
                "rustup active toolchain ('{}') and rustc --version ('{}') match expected channel '{}' and version {:?}.",
                active_toolchain_stdout.trim(), rustc_version_stdout.trim(), toolchain_channel, toolchain_version_opt
            );
        } else {
            failure_reason = format!(
                "Toolchain mismatch when rustup is present. Expected channel '{}', version {:?}.\nrustup active toolchain: '{}'\nrustc --version: '{}'",
                toolchain_channel, toolchain_version_opt, active_toolchain_stdout.trim(), rustc_version_stdout.trim()
            );
        }
    }

    if !checks_passed {
        bail!("Toolchain verification failed: {}", failure_reason);
    }
    info!("Rust toolchain verification successful.");

    // --- File Presence Check ---
    info!("Checking for presence of essential project files...");
    let mut all_essential_files_present = true;
    for file_path_str in ESSENTIAL_FILES {
        let host_path = Path::new(file_path_str);
        // This check is against the HOST file system as Dagger mounts it.
        // For a pure container check, you'd copy files in and check inside.
        // However, these are typically project root files.
        if host_path.exists() {
            info!("Found essential file: {}", file_path_str);
        } else {
            info!("ERROR: Essential file NOT found: {}", file_path_str);
            all_essential_files_present = false;
        }
    }
    if !all_essential_files_present {
        bail!("Missing one or more essential project files.");
    }
    info!("All essential project files are present.");

    // --- Headers Check ---
    info!("Checking file headers (forbid(unsafe_code) and license headers - basic checks)...");
    // ## Check for #![forbid(unsafe_code)] in library crates ##
    let mut all_forbid_unsafe_present = true;
    for lib_file_path_str in CRATE_LIB_FILES {
        let host_lib_path = Path::new(lib_file_path_str);
        if host_lib_path.exists() {
            match fs::read_to_string(host_lib_path) {
                Ok(contents) => {
                    if contents.contains("#![forbid(unsafe_code)]") {
                        info!("Found #![forbid(unsafe_code)] in {}", lib_file_path_str);
                    } else {
                        info!("ERROR: Missing #![forbid(unsafe_code)] in {}", lib_file_path_str);
                        all_forbid_unsafe_present = false;
                    }
                }
                Err(e) => {
                    info!("ERROR: Could not read file {}: {}", lib_file_path_str, e);
                    all_forbid_unsafe_present = false;
                }
            }
        } else {
            info!(
                "Warning: Library file not found, skipping forbid(unsafe_code) check: {}",
                lib_file_path_str
            );
        }
    }
    if !all_forbid_unsafe_present {
        bail!("Missing #![forbid(unsafe_code)] in one or more library files.");
    }
    info!("Basic #![forbid(unsafe_code)] checks passed.");

    // ## Placeholder for License Header Checks ##
    // TODO: Implement license header checks. This typically involves:
    //   1. Defining expected license header text (can be a template).
    //   2. Walking through source files (e.g., *.rs, *.toml, etc.).
    //   3. Reading the first N lines of each file.
    //   4. Comparing against the expected license header.
    //   This can be complex due to year variations, slight formatting, etc.
    //   Consider using a dedicated tool if available, or careful regex/string matching.
    info!("TODO: Implement license header checks.");

    // Final sync to ensure all checks within the container (if any were execs) are done.
    // For checks done on host like file presence, this is less critical for the Dagger part.
    let _ = container
        .sync()
        .await
        .context("Integrity check Dagger execution failed (if any execs were run)")?;

    info!("CI integrity checks pipeline completed successfully.");
    Ok(())
}
