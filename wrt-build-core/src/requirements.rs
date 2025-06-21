//! Requirements traceability and safety verification
//!
//! This module implements SCORE-inspired safety verification with:
//! - Requirements file parsing and validation
//! - File traceability checking
//! - ASIL compliance verification
//! - Certification readiness assessment

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{BuildError, BuildResult};

/// Requirements file structure
#[derive(Debug, Deserialize)]
pub struct Requirements {
    /// Requirements metadata
    pub metadata: RequirementsMetadata,
    /// List of requirements
    pub requirement: Vec<Requirement>,
}

/// Requirements metadata
#[derive(Debug, Deserialize)]
pub struct RequirementsMetadata {
    /// Project name
    pub project: String,
    /// Project version
    pub version: String,
    /// ASIL level for the project
    pub asil_level: String,
    /// Verification method used
    pub verification_method: String,
}

/// Individual requirement definition
#[derive(Debug, Deserialize, Clone)]
pub struct Requirement {
    /// Unique requirement ID
    pub id: String,
    /// Requirement name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// ASIL level for this requirement
    pub asil_level: String,
    /// Requirement category
    pub category: String,
    /// Verification method for this requirement
    pub verification_method: String,
    /// Source files implementing this requirement
    pub source_files: Vec<String>,
    /// Test files verifying this requirement
    pub test_files: Vec<String>,
    /// Documentation files for this requirement
    pub documentation_files: Vec<String>,
    /// Current implementation status
    pub status: String,
    /// Target platforms for this requirement
    pub platform: Vec<String>,
}

/// Requirements verification results
#[derive(Debug, Serialize)]
pub struct RequirementsVerificationResult {
    /// Total number of requirements
    pub total_requirements: usize,
    /// Number of verified requirements
    pub verified_requirements: usize,
    /// List of missing files
    pub missing_files: Vec<String>,
    /// List of incomplete requirements
    pub incomplete_requirements: Vec<String>,
    /// Certification readiness percentage
    pub certification_readiness: f64,
}

impl Requirements {
    /// Load requirements from TOML file
    pub fn load(path: &Path) -> BuildResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BuildError::Workspace(format!("Failed to read requirements file: {}", e))
        })?;

        toml::from_str(&content)
            .map_err(|e| BuildError::Verification(format!("Failed to parse requirements: {}", e)))
    }

    /// Initialize a sample requirements file
    pub fn init_sample(path: &Path) -> BuildResult<()> {
        let sample = r#"[metadata]
project = "WRT - WebAssembly Runtime"
version = "0.2.0"
asil_level = "ASIL-D"
verification_method = "SCORE-inspired"

[[requirement]]
id = "REQ-SAFETY-001"
name = "Memory Safety"
description = "All memory operations shall be bounds-checked and verified"
asil_level = "ASIL-D"
category = "Safety"
verification_method = "Static Analysis + Formal Verification"
source_files = ["wrt-runtime/src/memory.rs", "wrt-foundation/src/bounded.rs"]
test_files = ["wrt-runtime/src/memory_test.rs"]
documentation_files = ["docs/source/safety/memory_safety.rst"]
status = "Implemented"
platform = ["all"]

[[requirement]]
id = "REQ-SAFETY-002"
name = "No Panic in Production"
description = "Production code shall not contain panic! macros"
asil_level = "ASIL-D"
category = "Safety"
verification_method = "Static Analysis"
source_files = ["wrt-*/src/**/*.rs"]
test_files = ["xtask/src/check_panics.rs"]
documentation_files = ["docs/source/safety/panic_handling.rst"]
status = "Implemented"
platform = ["all"]
"#;

        std::fs::write(path, sample).map_err(|e| {
            BuildError::Verification(format!("Failed to write requirements file: {}", e))
        })?;

        println!(
            "{} Created sample requirements.toml at {}",
            "✅".bright_green(),
            path.display()
        );
        Ok(())
    }

    /// Verify all requirements
    pub fn verify(&self, workspace_root: &Path) -> BuildResult<RequirementsVerificationResult> {
        let mut missing_files = Vec::new();
        let mut incomplete_requirements = Vec::new();
        let mut verified_count = 0;

        for req in &self.requirement {
            let mut req_complete = true;

            // Check source files
            for file in &req.source_files {
                let path = workspace_root.join(file);
                if !path.exists() && !file.contains("*") {
                    missing_files.push(format!("{} (source for {})", file, req.id));
                    req_complete = false;
                }
            }

            // Check test files
            for file in &req.test_files {
                let path = workspace_root.join(file);
                if !path.exists() && !file.contains("*") {
                    missing_files.push(format!("{} (test for {})", file, req.id));
                    req_complete = false;
                }
            }

            // Check documentation files
            for file in &req.documentation_files {
                let path = workspace_root.join(file);
                if !path.exists() && !file.contains("*") {
                    missing_files.push(format!("{} (doc for {})", file, req.id));
                    req_complete = false;
                }
            }

            if req_complete {
                verified_count += 1;
            } else {
                incomplete_requirements.push(req.id.clone());
            }
        }

        let certification_readiness =
            (verified_count as f64 / self.requirement.len() as f64) * 100.0;

        Ok(RequirementsVerificationResult {
            total_requirements: self.requirement.len(),
            verified_requirements: verified_count,
            missing_files,
            incomplete_requirements,
            certification_readiness,
        })
    }

    /// Generate requirements traceability matrix
    pub fn generate_traceability_matrix(&self) -> String {
        let mut matrix = String::new();

        matrix.push_str("# Requirements Traceability Matrix\n\n");
        matrix.push_str("| ID | Name | ASIL | Status | Sources | Tests | Docs |\n");
        matrix.push_str("|-------|------|------|--------|---------|-------|------|\n");

        for req in &self.requirement {
            matrix.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                req.id,
                req.name,
                req.asil_level,
                req.status,
                req.source_files.len(),
                req.test_files.len(),
                req.documentation_files.len()
            ));
        }

        matrix
    }
}
