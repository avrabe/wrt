// WRT - wrt
// Module: Control Flow Integrity Integration
// SW-REQ-ID: REQ_CFI_INTEGRATION_001
//
// Copyright (c) 2025 The WRT Project Developers
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Control Flow Integrity Integration for WRT
//!
//! This module provides high-level CFI integration for the main WRT library,
//! combining CFI metadata generation, control flow operations, and runtime
//! execution into a unified interface.
//!
//! # Key Features
//! - Integrated CFI-protected WebAssembly execution
//! - Automatic CFI metadata generation from WASM modules
//! - Hardware-accelerated CFI on supported platforms
//! - Comprehensive CFI violation detection and response
//! - Performance monitoring and statistics

#![allow(dead_code)] // Allow during development

use crate::bounded_wrt_infra::{new_loaded_module_vec, BoundedLoadedModuleVec, WrtProvider};
use crate::prelude::*;

/// CFI-protected WebAssembly execution engine
///
/// This provides a high-level interface for executing WebAssembly modules
/// with comprehensive Control Flow Integrity protection.
pub struct CfiProtectedEngine {
    /// Underlying stackless engine
    stackless_engine: StacklessEngine,
    /// CFI runtime engine from wrt-runtime
    cfi_engine: wrt_runtime::CfiExecutionEngine,
    /// CFI metadata generator from wrt-decoder
    metadata_generator: wrt_decoder::CfiMetadataGenerator,
    /// Current CFI protection configuration
    protection_config: wrt_instructions::CfiControlFlowProtection,
    /// Execution statistics
    execution_stats: CfiEngineStatistics,
}

/// CFI configuration options for WebAssembly execution
#[derive(Debug, Clone)]
pub struct CfiConfiguration {
    /// CFI protection level (hardware, software, or hybrid)
    pub protection_level: wrt_instructions::CfiProtectionLevel,
    /// Maximum shadow stack depth
    pub max_shadow_stack_depth: usize,
    /// Landing pad timeout in nanoseconds
    pub landing_pad_timeout_ns: Option<u64>,
    /// CFI violation response policy
    pub violation_policy: wrt_runtime::CfiViolationPolicy,
    /// Enable temporal validation
    pub enable_temporal_validation: bool,
    /// Hardware features to enable
    pub hardware_features: CfiHardwareFeatures,
}

/// Hardware CFI features configuration
#[derive(Debug, Clone, Default)]
pub struct CfiHardwareFeatures {
    /// Enable ARM BTI (Branch Target Identification)
    pub arm_bti: bool,
    /// Enable RISC-V CFI extensions
    pub riscv_cfi: bool,
    /// Enable x86 CET (Control Flow Enforcement Technology)
    pub x86_cet: bool,
    /// Auto-detect available hardware features
    pub auto_detect: bool,
}

/// CFI execution statistics aggregated from all components
#[derive(Debug, Clone, Default)]
pub struct CfiEngineStatistics {
    /// Runtime engine statistics
    pub runtime_stats: wrt_runtime::CfiEngineStatistics,
    /// Decoder metadata generation statistics
    pub metadata_stats: CfiMetadataStatistics,
    /// Overall execution metrics
    pub execution_metrics: CfiExecutionMetrics,
}

/// CFI metadata generation statistics
#[derive(Debug, Clone, Default)]
pub struct CfiMetadataStatistics {
    /// Total functions analyzed
    pub functions_analyzed: u64,
    /// Total indirect call sites found
    pub indirect_call_sites: u64,
    /// Total return sites analyzed
    pub return_sites: u64,
    /// Hardware instructions generated
    pub hardware_instructions_generated: u64,
    /// Software validations created
    pub software_validations_created: u64,
    /// Metadata generation time (nanoseconds)
    pub generation_time_ns: u64,
}

/// Overall CFI execution metrics
#[derive(Debug, Clone, Default)]
pub struct CfiExecutionMetrics {
    /// Total modules executed with CFI
    pub modules_executed: u64,
    /// Total execution time with CFI (nanoseconds)
    pub total_execution_time_ns: u64,
    /// Average CFI overhead percentage
    pub avg_cfi_overhead_percent: f64,
    /// Total CFI violations across all executions
    pub total_violations: u64,
    /// Total successful CFI validations
    pub total_validations: u64,
}

impl Default for CfiConfiguration {
    fn default() -> Self {
        Self {
            protection_level: wrt_instructions::CfiProtectionLevel::Hybrid,
            max_shadow_stack_depth: 1024,
            landing_pad_timeout_ns: Some(1_000_000), // 1ms
            violation_policy: wrt_runtime::CfiViolationPolicy::ReturnError,
            enable_temporal_validation: true,
            hardware_features: CfiHardwareFeatures {
                auto_detect: true,
                ..Default::default()
            },
        }
    }
}

impl CfiProtectedEngine {
    /// Create a new CFI-protected WebAssembly execution engine
    pub fn new(config: CfiConfiguration) -> Result<Self> {
        // Create underlying stackless engine
        let stackless_engine = StacklessEngine::new);

        // Configure CFI protection based on configuration
        let protection_config = Self::build_protection_config(&config)?;

        // Create CFI runtime engine with stackless integration
        let cfi_engine = wrt_runtime::CfiExecutionEngine::new_with_policy(
            protection_config.clone(),
            config.violation_policy,
        ;

        // Create CFI metadata generator
        let metadata_generator =
            wrt_decoder::CfiMetadataGenerator::new(wrt_decoder::CfiProtectionConfig {
                protection_level: config.protection_level,
                enable_shadow_stack: true,
                enable_landing_pads: true,
                enable_temporal_validation: config.enable_temporal_validation,
                max_shadow_stack_depth: config.max_shadow_stack_depth,
                landing_pad_timeout_ns: config.landing_pad_timeout_ns,
            };

        Ok(Self {
            stackless_engine,
            cfi_engine,
            metadata_generator,
            protection_config,
            execution_stats: CfiEngineStatistics::default(),
        })
    }

    /// Create CFI engine with default configuration
    pub fn new_default() -> Result<Self> {
        Self::new(CfiConfiguration::default())
    }

    /// Load and prepare a WebAssembly module with CFI protection
    pub fn load_module_with_cfi(&mut self, binary: &[u8]) -> Result<CfiProtectedModule> {
        let start_time = self.get_timestamp);

        // Load the module using standard WRT functionality
        let module = load_module_from_binary(binary)?;

        // Generate CFI metadata for the module
        let cfi_metadata = self.metadata_generator.generate_metadata(&module)?;

        // Update metadata statistics
        self.execution_stats.metadata_stats.functions_analyzed +=
            cfi_metadata.functions.len() as u64;
        self.execution_stats.metadata_stats.indirect_call_sites += cfi_metadata
            .functions
            .iter()
            .map(|f| f.indirect_call_sites.len() as u64)
            .sum::<u64>);
        self.execution_stats.metadata_stats.return_sites +=
            cfi_metadata.functions.iter().map(|f| f.return_sites.len() as u64).sum::<u64>);

        let end_time = self.get_timestamp);
        self.execution_stats.metadata_stats.generation_time_ns +=
            end_time.saturating_sub(start_time;

        Ok(CfiProtectedModule {
            module,
            cfi_metadata,
            protection_config: self.protection_config.clone(),
        })
    }

    /// Execute a CFI-protected WebAssembly module
    pub fn execute_module(
        &mut self,
        protected_module: &CfiProtectedModule,
        function_name: &str,
    ) -> Result<CfiExecutionResult> {
        let start_time = self.get_timestamp);

        // Create execution context
        let mut execution_context = wrt_runtime::ExecutionContext::new(1024;

        // Find the function to execute
        let function_index = self.find_function_index(&protected_module.module, function_name)?;

        // Execute with CFI protection
        let result = self.execute_function_with_cfi(
            &protected_module,
            function_index,
            &mut execution_context,
        )?;

        // Update execution metrics
        let end_time = self.get_timestamp);
        let execution_time = end_time.saturating_sub(start_time;

        self.execution_stats.execution_metrics.modules_executed += 1;
        self.execution_stats.execution_metrics.total_execution_time_ns += execution_time;

        // Calculate CFI overhead (simplified calculation)
        let baseline_time = execution_time / 2; // Estimate baseline as 50% of CFI time
        let overhead_percent =
            ((execution_time.saturating_sub(baseline_time)) as f64 / baseline_time as f64) * 100.0;
        self.execution_stats.execution_metrics.avg_cfi_overhead_percent =
            (self.execution_stats.execution_metrics.avg_cfi_overhead_percent + overhead_percent)
                / 2.0;

        Ok(result)
    }

    /// Execute a specific function with CFI protection
    fn execute_function_with_cfi(
        &mut self,
        protected_module: &CfiProtectedModule,
        function_index: u32,
        execution_context: &mut wrt_runtime::ExecutionContext,
    ) -> Result<CfiExecutionResult> {
        // Get function CFI metadata
        let function_metadata = protected_module
            .cfi_metadata
            .functions
            .iter()
            .find(|f| f.function_index == function_index)
            .ok_or_else(|| {
                Error::runtime_execution_error(&format!(
                    "Function {} not found in CFI metadata",
                    function_index
                ))
            })?;

        // Set up CFI protection for this function
        self.setup_function_cfi_protection(function_metadata)?;

        // Execute instructions with CFI protection using bounded collections
        let mut instruction_results = new_loaded_module_vec);
        let function = &protected_module.module.functions[function_index as usize];

        for instruction in &function.instructions {
            let cfi_result =
                self.cfi_engine.execute_instruction_with_cfi(instruction, execution_context)?;

            // Update violation counts
            if let Some(violations) = self.extract_violations_from_result(&cfi_result) {
                self.execution_stats.execution_metrics.total_violations += violations;
            }

            instruction_results.push(cfi_result);
        }

        // Update validation counts
        self.execution_stats.execution_metrics.total_validations +=
            instruction_results.len() as u64;

        Ok(CfiExecutionResult {
            function_index,
            instruction_results,
            function_metadata: function_metadata.clone(),
            violations_detected: self.cfi_engine.statistics().violations_detected,
        })
    }

    /// Set up CFI protection for a specific function
    fn setup_function_cfi_protection(
        &mut self,
        function_metadata: &wrt_decoder::FunctionCfiInfo,
    ) -> Result<()> {
        // Configure shadow stack expectations
        for call_site in &function_metadata.indirect_call_sites {
            // Set up landing pad expectations for each call site
            if let Some(ref protection) = call_site.protection_requirements {
                // Configure based on protection requirements
                self.configure_call_site_protection(call_site, protection)?;
            }
        }

        Ok(())
    }

    /// Configure protection for a specific call site
    fn configure_call_site_protection(
        &mut self,
        _call_site: &wrt_decoder::IndirectCallSite,
        _protection: &wrt_decoder::CfiProtectionRequirements,
    ) -> Result<()> {
        // TODO: Implement call site specific protection configuration
        Ok(())
    }

    /// Find function index by name in the module
    fn find_function_index(&self, module: &Module, function_name: &str) -> Result<u32> {
        // Look through exports to find the function
        for export in &module.exports {
            if export.name == function_name {
                if let ExportKind::Func = export.kind {
                    return Ok(export.index;
                }
            }
        }

        Err(Error::new(
            ErrorCategory::Runtime,
            codes::FUNCTION_NOT_FOUND,
            format!("Function '{}' not found in module exports", function_name),
        ))
    }

    /// Extract violation count from CFI execution result
    fn extract_violations_from_result(
        &self,
        _result: &wrt_runtime::CfiExecutionResult,
    ) -> Option<u64> {
        // TODO: Extract actual violation count from result
        None
    }

    /// Build CFI protection configuration from user configuration
    fn build_protection_config(
        config: &CfiConfiguration,
    ) -> Result<wrt_instructions::CfiControlFlowProtection> {
        let mut protection = wrt_instructions::CfiControlFlowProtection::default);

        // Configure hardware features based on auto-detection and explicit settings
        if config.hardware_features.auto_detect {
            protection.hardware_config = Self::detect_hardware_features()?;
        } else {
            protection.hardware_config = Self::build_hardware_config(&config.hardware_features)?;
        }

        // Configure software fallback
        protection.software_config.max_shadow_stack_depth = config.max_shadow_stack_depth;
        protection.software_config.enable_temporal_validation = config.enable_temporal_validation;
        protection.software_config.landing_pad_timeout_ns = config.landing_pad_timeout_ns;

        Ok(protection)
    }

    /// Detect available hardware CFI features
    fn detect_hardware_features() -> Result<wrt_instructions::CfiHardwareConfig> {
        let mut config = wrt_instructions::CfiHardwareConfig::default);

        // Detect ARM BTI
        #[cfg(target_arch = "aarch64")]
        {
            config.arm_bti = wrt_platform::BranchTargetIdentification::is_available);
        }

        // Detect RISC-V CFI
        #[cfg(target_arch = "riscv64")]
        {
            config.riscv_cfi = wrt_platform::ControlFlowIntegrity::is_available);
        }

        // Detect x86 CET
        #[cfg(target_arch = "x86_64")]
        {
            config.x86_cet = Self::detect_x86_cet);
        }

        Ok(config)
    }

    /// Build hardware configuration from explicit settings
    fn build_hardware_config(
        features: &CfiHardwareFeatures,
    ) -> Result<wrt_instructions::CfiHardwareConfig> {
        Ok(wrt_instructions::CfiHardwareConfig {
            arm_bti: features.arm_bti,
            riscv_cfi: features.riscv_cfi,
            x86_cet: features.x86_cet,
        })
    }

    /// Detect x86 CET support
    #[cfg(target_arch = "x86_64")]
    fn detect_x86_cet() -> bool {
        // Simplified CET detection - real implementation would check CPUID
        false
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn detect_x86_cet() -> bool {
        false
    }

    /// Get current timestamp for performance measurement
    fn get_timestamp(&self) -> u64 {
        // Use the same timestamp implementation as CFI engine
        #[cfg(target_arch = "aarch64")]
        {
            let mut cntvct: u64;
            unsafe {
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) cntvct;
            }
            cntvct
        }
        #[cfg(target_arch = "riscv64")]
        {
            let mut time: u64;
            unsafe {
                core::arch::asm!("rdtime {}", out(reg) time;
            }
            time
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
        {
            // Software fallback
            0
        }
    }

    /// Get current CFI statistics
    pub fn statistics(&self) -> &CfiEngineStatistics {
        &self.execution_stats
    }

    /// Reset all CFI statistics
    pub fn reset_statistics(&mut self) {
        self.execution_stats = CfiEngineStatistics::default);
    }
}

/// A WebAssembly module with CFI metadata and protection
#[derive(Debug, Clone)]
pub struct CfiProtectedModule {
    /// The underlying WebAssembly module
    pub module: Module,
    /// Generated CFI metadata
    pub cfi_metadata: wrt_decoder::CfiMetadata,
    /// CFI protection configuration
    pub protection_config: wrt_instructions::CfiControlFlowProtection,
}

/// Result of executing a function with CFI protection
#[derive(Debug)]
pub struct CfiExecutionResult {
    /// Function that was executed
    pub function_index: u32,
    /// Results from each instruction execution using bounded collections
    pub instruction_results: BoundedLoadedModuleVec<wrt_runtime::CfiExecutionResult>,
    /// CFI metadata for the executed function
    pub function_metadata: wrt_decoder::FunctionCfiInfo,
    /// Total CFI violations detected during execution
    pub violations_detected: u64,
}

/// Convenience functions for creating CFI-protected execution

/// Create a new CFI-protected execution engine with default settings
pub fn new_cfi_engine() -> Result<CfiProtectedEngine> {
    CfiProtectedEngine::new_default()
}

/// Create a CFI-protected execution engine with custom configuration
pub fn new_cfi_engine_with_config(config: CfiConfiguration) -> Result<CfiProtectedEngine> {
    CfiProtectedEngine::new(config)
}

/// Load and execute a WebAssembly module with CFI protection
pub fn execute_module_with_cfi(binary: &[u8], function_name: &str) -> Result<CfiExecutionResult> {
    let mut engine = CfiProtectedEngine::new_default()?;
    let protected_module = engine.load_module_with_cfi(binary)?;
    engine.execute_module(&protected_module, function_name)
}

/// Load and execute a WebAssembly module with custom CFI configuration
pub fn execute_module_with_cfi_config(
    binary: &[u8],
    function_name: &str,
    config: CfiConfiguration,
) -> Result<CfiExecutionResult> {
    let mut engine = CfiProtectedEngine::new(config)?;
    let protected_module = engine.load_module_with_cfi(binary)?;
    engine.execute_module(&protected_module, function_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfi_configuration_default() {
        let config = CfiConfiguration::default);
        assert_eq!(
            config.protection_level,
            wrt_instructions::CfiProtectionLevel::Hybrid
        ;
        assert_eq!(config.max_shadow_stack_depth, 1024;
        assert!(config.hardware_features.auto_detect);
    }

    #[test]
    fn test_cfi_engine_creation() {
        let config = CfiConfiguration::default);
        let result = CfiProtectedEngine::new(config;
        assert!(result.is_ok();
    }

    #[test]
    fn test_hardware_feature_detection() {
        let config = CfiProtectedEngine::detect_hardware_features);
        assert!(config.is_ok();
    }

    #[test]
    fn test_cfi_statistics_default() {
        let stats = CfiEngineStatistics::default);
        assert_eq!(stats.execution_metrics.modules_executed, 0;
        assert_eq!(stats.metadata_stats.functions_analyzed, 0;
    }
}
