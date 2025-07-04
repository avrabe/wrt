//! # WRT Dagger - Containerized Build Wrapper
//!
//! This module provides optional containerized build capabilities for WRT using
//! Dagger. It serves as a thin wrapper around cargo-wrt, enabling consistent
//! builds across different environments through containerization.
//!
//! ## Features
//!
//! - **Containerized Builds**: Run builds in isolated containers
//! - **Cross-Platform Consistency**: Same environment on all platforms
//! - **CI/CD Integration**: Perfect for GitHub Actions and other CI systems
//! - **cargo-wrt Integration**: Leverages the unified WRT build system
//!
//! ## Usage
//!
//! ```rust,no_run
//! use wrt_dagger::{
//!     ContainerConfig,
//!     DaggerPipeline,
//! };
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ContainerConfig::default();
//!     let pipeline = DaggerPipeline::new(config).await?;
//!
//!     // Run cargo-wrt build in container
//!     pipeline.build().await?;
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Configuration for containerized builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Base container image to use
    pub base_image: String,

    /// Rust version to install
    pub rust_version: String,

    /// Additional system packages to install
    pub system_packages: Vec<String>,

    /// Environment variables to set in container
    pub environment: HashMap<String, String>,

    /// Working directory in container
    pub work_dir: String,

    /// Whether to cache dependencies
    pub cache_dependencies: bool,

    /// Timeout for build operations (in seconds)
    pub timeout: u64,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        let mut environment = HashMap::new();
        environment.insert("RUST_LOG".to_string(), "info".to_string());
        environment.insert("CARGO_INCREMENTAL".to_string(), "0".to_string());

        Self {
            base_image: "ubuntu:22.04".to_string(),
            rust_version: "1.86.0".to_string(),
            system_packages: vec![
                "curl".to_string(),
                "build-essential".to_string(),
                "pkg-config".to_string(),
                "libssl-dev".to_string(),
            ],
            environment,
            work_dir: "/workspace".to_string(),
            cache_dependencies: true,
            timeout: 3600, // 1 hour
        }
    }
}

/// Dagger pipeline for WRT builds
pub struct DaggerPipeline {
    config: ContainerConfig,
    // Note: dagger_sdk::Query field removed due to API changes
    // Will be re-added when Dagger SDK API stabilizes
}

impl DaggerPipeline {
    /// Create a new Dagger pipeline with the given configuration
    pub async fn new(config: ContainerConfig) -> Result<Self> {
        #[cfg(feature = "dagger")]
        {
            // Note: Dagger connection disabled due to API changes
            // Falling back to local execution even with dagger feature enabled
            eprintln!(
                "⚠️  Dagger SDK integration disabled due to API changes. Using local fallback."
            );
            Ok(Self { config })
        }

        #[cfg(not(feature = "dagger"))]
        {
            Ok(Self { config })
        }
    }

    /// Run cargo-wrt build in container
    pub async fn build(&self) -> Result<String> {
        self.run_cargo_wrt(&["build"]).await
    }

    /// Run cargo-wrt test in container
    pub async fn test(&self) -> Result<String> {
        self.run_cargo_wrt(&["test"]).await
    }

    /// Run cargo-wrt ci in container
    pub async fn ci(&self) -> Result<String> {
        self.run_cargo_wrt(&["ci"]).await
    }

    /// Run cargo-wrt verify with ASIL level in container
    pub async fn verify(&self, asil_level: &str) -> Result<String> {
        self.run_cargo_wrt(&["verify", "--asil", asil_level]).await
    }

    /// Run cargo-wrt coverage in container
    pub async fn coverage(&self) -> Result<String> {
        self.run_cargo_wrt(&["coverage", "--html"]).await
    }

    /// Run arbitrary cargo-wrt command in container
    pub async fn run_cargo_wrt(&self, args: &[&str]) -> Result<String> {
        #[cfg(feature = "dagger")]
        {
            self.run_dagger_command(args).await
        }

        #[cfg(not(feature = "dagger"))]
        {
            self.run_local_fallback(args).await
        }
    }

    #[cfg(feature = "dagger")]
    async fn run_dagger_command(&self, _args: &[&str]) -> Result<String> {
        // Note: Dagger SDK API has changed significantly in v0.11+
        // This is a placeholder for future implementation when the API stabilizes
        anyhow::bail!(
            "Dagger integration is currently disabled due to API changes in dagger-sdk v0.11+. \
             Please use the local fallback mode or help update the integration."
        );

        // TODO: Implement updated Dagger SDK v0.11+ integration
        // The previous implementation was based on older API that has changed
        // significantly
    }

    #[cfg(not(feature = "dagger"))]
    async fn run_local_fallback(&self, args: &[&str]) -> Result<String> {
        use std::process::Command;

        eprintln!("⚠️  Dagger feature not enabled, falling back to local cargo-wrt execution");

        let mut cmd = Command::new("cargo-wrt");
        cmd.args(args);

        let output = cmd.output().context("Failed to execute cargo-wrt locally")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo-wrt failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }
}

/// Builder for creating custom container configurations
pub struct ContainerConfigBuilder {
    config: ContainerConfig,
}

impl ContainerConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ContainerConfig::default(),
        }
    }

    /// Set the base container image
    pub fn base_image(mut self, image: &str) -> Self {
        self.config.base_image = image.to_string();
        self
    }

    /// Set the Rust version
    pub fn rust_version(mut self, version: &str) -> Self {
        self.config.rust_version = version.to_string();
        self
    }

    /// Add a system package
    pub fn add_package(mut self, package: &str) -> Self {
        self.config.system_packages.push(package.to_string());
        self
    }

    /// Set an environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.config.environment.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the working directory
    pub fn work_dir(mut self, dir: &str) -> Self {
        self.config.work_dir = dir.to_string();
        self
    }

    /// Enable/disable dependency caching
    pub fn cache_dependencies(mut self, cache: bool) -> Self {
        self.config.cache_dependencies = cache;
        self
    }

    /// Set build timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ContainerConfig {
        self.config
    }
}

impl Default for ContainerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for common operations
pub mod utils {
    use super::*;

    /// Create a pipeline optimized for CI/CD environments
    pub async fn ci_pipeline() -> Result<DaggerPipeline> {
        let config = ContainerConfigBuilder::new()
            .base_image("rust:1.86-slim-bullseye")
            .add_package("git")
            .add_package("ca-certificates")
            .env("CI", "true")
            .env("CARGO_TERM_COLOR", "always")
            .timeout(7200) // 2 hours for CI
            .build();

        DaggerPipeline::new(config).await
    }

    /// Create a pipeline optimized for development
    pub async fn dev_pipeline() -> Result<DaggerPipeline> {
        let config = ContainerConfigBuilder::new()
            .cache_dependencies(true)
            .env("RUST_LOG", "debug")
            .timeout(1800) // 30 minutes for dev
            .build();

        DaggerPipeline::new(config).await
    }

    /// Create a pipeline for ASIL-D safety verification
    pub async fn safety_pipeline() -> Result<DaggerPipeline> {
        let config = ContainerConfigBuilder::new()
            .base_image("ubuntu:22.04")
            .add_package("kani-verifier") // Note: This would need proper installation
            .env("KANI_REACH_CHECKS", "1")
            .env("ASIL_LEVEL", "D")
            .timeout(10800) // 3 hours for formal verification
            .build();

        DaggerPipeline::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config_default() {
        let config = ContainerConfig::default();
        assert_eq!(config.base_image, "ubuntu:22.04");
        assert_eq!(config.rust_version, "1.86.0");
        assert!(!config.system_packages.is_empty());
    }

    #[test]
    fn test_container_config_builder() {
        let config = ContainerConfigBuilder::new()
            .base_image("alpine:latest")
            .rust_version("1.85.0")
            .add_package("git")
            .env("DEBUG", "1")
            .work_dir("/app")
            .timeout(1200)
            .build();

        assert_eq!(config.base_image, "alpine:latest");
        assert_eq!(config.rust_version, "1.85.0");
        assert!(config.system_packages.contains(&"git".to_string()));
        assert_eq!(config.environment.get("DEBUG"), Some(&"1".to_string()));
        assert_eq!(config.work_dir, "/app");
        assert_eq!(config.timeout, 1200);
    }

    #[tokio::test]
    async fn test_pipeline_creation() {
        let config = ContainerConfig::default();
        let result = DaggerPipeline::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_utils_pipelines() {
        // These should not fail to create (even if Dagger is not available)
        assert!(utils::ci_pipeline().await.is_ok());
        assert!(utils::dev_pipeline().await.is_ok());
        assert!(utils::safety_pipeline().await.is_ok());
    }
}
