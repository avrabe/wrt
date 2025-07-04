//! Requirements verification system with SCORE-inspired methodology
//!
//! This module provides comprehensive requirements traceability and verification
//! capabilities for safety-critical systems, supporting both simple file-based
//! verification and advanced SCORE-inspired safety requirement tracking.

pub mod documentation;
pub mod legacy;
pub mod model;
pub mod platform;
pub mod safety;

pub use model::{
    ComplianceReport, CoverageLevel, RequirementId, RequirementRegistry, RequirementType,
    SafetyRequirement, VerificationMethod, VerificationStatus,
};

// Re-export simple requirements types for backward compatibility
pub use legacy::{
    EnhancedRequirementsVerifier, Requirement, Requirements, RequirementsMetadata,
    RequirementsVerificationResult,
};

// Export safety verification framework
pub use safety::{
    CertificationReadiness, ComplianceVerificationResult, ComplianceViolation, CoverageData,
    FileCoverage, PlatformSummary, PlatformVerification, SafetyReport, SafetyVerificationFramework,
    TestCoverageType, TestResult, TestSummary, ViolationSeverity, ViolationType,
};

// Export documentation verification framework
pub use documentation::{
    DocumentationAnalysis, DocumentationLocation, DocumentationReport, DocumentationStandards,
    DocumentationVerificationConfig, DocumentationVerificationFramework,
    DocumentationVerificationResult, DocumentationViolation, DocumentationViolationSeverity,
    DocumentationViolationType,
};

// Export platform verification framework
pub use platform::{
    ComprehensivePlatformLimits, ContainerRuntime, ExternalLimitSources, PlatformId,
    PlatformVerificationConfig, PlatformVerificationEngine,
};
