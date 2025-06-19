//! WRT Build Core - Centralized build system for WebAssembly Runtime
//!
//! This library provides the core functionality for building, testing, and
//! verifying the WRT (WebAssembly Runtime) project. It serves as the single
//! source of truth for all build operations, replacing the previous fragmented
//! approach with justfile, xtask, and shell scripts.
//!
//! # Architecture
//!
//! The build system is organized around a central [`BuildSystem`] struct that
//! manages workspace operations and coordinates various build tasks:
//!
//! - **Build Operations**: Compilation of all WRT components
//! - **Test Execution**: Running unit, integration, and verification tests
//! - **Safety Verification**: SCORE-inspired safety checks and formal verification
//! - **Documentation Generation**: API docs, guides, and verification reports
//! - **Coverage Analysis**: Code coverage metrics and reporting
//!
//! # Design Principles
//!
//! - **Single Source of Truth**: All build logic centralized in this library
//! - **AI-Friendly**: Clear, linear architecture for AI agent integration
//! - **Cross-Platform**: Works on all target platforms (std/no_std)
//! - **Functional Safety**: Supports ISO 26262, IEC 61508 compliance
//! - **Deterministic**: Reproducible builds with comprehensive caching

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::expect_used
)]

// Re-export core types for convenience
pub use anyhow::{Context, Result};
pub use std::path::{Path, PathBuf};

// Core modules
pub mod build;
pub mod ci;
pub mod config;
pub mod error;
pub mod fuzz;
pub mod kani;
pub mod matrix;
pub mod memory;
pub mod requirements;
pub mod test;
pub mod tools;
pub mod text_search;
pub mod tool_versions;
pub mod validation;
pub mod verify;
pub mod verification_tool;

// Public API
pub use build::BuildSystem;
pub use config::{BuildConfig, WorkspaceConfig};
pub use error::{BuildError, BuildResult};

/// Build system version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default workspace root detection
pub fn detect_workspace_root() -> Result<PathBuf> {
    let current = std::env::current_dir().context("Failed to get current directory")?;

    let mut path = current.as_path();
    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if it's a workspace root
            let content =
                std::fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;
            if content.contains("[workspace]") {
                return Ok(path.to_path_buf());
            }
        }

        match path.parent() {
            Some(parent) => path = parent,
            None => break,
        }
    }

    anyhow::bail!("Could not find workspace root (Cargo.toml with [workspace])")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_detection() {
        // This test should find the WRT workspace root
        let workspace = detect_workspace_root();
        assert!(workspace.is_ok(), "Should detect workspace root");

        let root = workspace.unwrap();
        assert!(root.join("Cargo.toml").exists(), "Should have Cargo.toml");
        assert!(
            root.join("wrt-build-core").exists(),
            "Should contain this crate"
        );
    }

    #[test]
    fn test_version_defined() {
        assert!(!VERSION.is_empty(), "Version should be defined");
    }
}
