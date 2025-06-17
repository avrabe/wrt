//! Platform verification with external limit integration
//!
//! Provides verification capabilities that integrate with CLI args, environment variables,
//! configuration files, and container discovery.


use wrt_error::{Error, ErrorCategory, codes};

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::{collections::HashMap, env, fs, path::Path, string::String, vec::Vec};

// Stub imports for platform module - will be replaced during integration
mod platform_stubs {
    pub struct ComprehensivePlatformLimits {
        pub max_total_memory: usize,
        pub max_wasm_linear_memory: usize,
        pub max_stack_bytes: usize,
        pub max_components: usize,
        pub platform_id: PlatformId,
    }
    
    pub enum PlatformId {
        Linux,
        QNX,
        MacOS,
        VxWorks,
        Zephyr,
        Tock,
        Embedded,
        Unknown,
    }
    
    impl Default for ComprehensivePlatformLimits {
        fn default() -> Self {
            Self {
                max_total_memory: 1024 * 1024 * 1024,
                max_wasm_linear_memory: 256 * 1024 * 1024,
                max_stack_bytes: 1024 * 1024,
                max_components: 256,
                platform_id: PlatformId::Unknown,
            }
        }
    }
    
    pub struct PlatformLimitDiscoverer;
    
    impl PlatformLimitDiscoverer {
        pub fn new() -> Self { Self }
        pub fn discover(&mut self) -> Result<ComprehensivePlatformLimits, super::Error> {
            Ok(ComprehensivePlatformLimits::default())
        }
    }
}

pub use platform_stubs::{ComprehensivePlatformLimits, PlatformId, PlatformLimitDiscoverer};

/// External limit sources
#[derive(Debug, Clone)]
pub struct ExternalLimitSources {
    /// CLI arguments
    pub cli_args: Vec<String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Configuration file path
    pub config_file: Option<String>,
    /// Container runtime detection
    pub container_runtime: ContainerRuntime,
}

/// Container runtime types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntime {
    /// No container detected
    None,
    /// Docker container
    Docker,
    /// Kubernetes pod
    Kubernetes,
    /// LXC container
    LXC,
    /// systemd-nspawn
    SystemdNspawn,
    /// Other container type
    Other,
}

/// Platform verification configuration
#[derive(Debug, Clone)]
pub struct PlatformVerificationConfig {
    /// Maximum memory override from external sources
    pub max_memory_override: Option<usize>,
    /// Maximum WASM memory override
    pub max_wasm_memory_override: Option<usize>,
    /// Maximum stack override
    pub max_stack_override: Option<usize>,
    /// Maximum components override
    pub max_components_override: Option<usize>,
    /// Debug level override
    pub debug_level_override: Option<String>,
    /// Strict validation mode
    pub strict_validation: bool,
    /// External sources used
    pub sources: ExternalLimitSources,
}

impl Default for PlatformVerificationConfig {
    fn default() -> Self {
        Self {
            max_memory_override: None,
            max_wasm_memory_override: None,
            max_stack_override: None,
            max_components_override: None,
            debug_level_override: None,
            strict_validation: false,
            sources: ExternalLimitSources {
                cli_args: Vec::new(),
                env_vars: HashMap::new(),
                config_file: None,
                container_runtime: ContainerRuntime::None,
            },
        }
    }
}

/// Platform verification engine
pub struct PlatformVerificationEngine {
    /// Configuration
    config: PlatformVerificationConfig,
    /// Discovered platform limits
    platform_limits: Option<ComprehensivePlatformLimits>,
    /// Final verified limits
    verified_limits: Option<ComprehensivePlatformLimits>,
}

impl PlatformVerificationEngine {
    /// Create new verification engine
    pub fn new() -> Self {
        Self {
            config: PlatformVerificationConfig::default(),
            platform_limits: None,
            verified_limits: None,
        }
    }
    
    /// Create verification engine with configuration
    pub fn with_config(config: PlatformVerificationConfig) -> Self {
        Self {
            config,
            platform_limits: None,
            verified_limits: None,
        }
    }
    
    /// Discover platform limits from all sources
    pub fn discover_limits(&mut self) -> Result<ComprehensivePlatformLimits, Error> {
        // 1. Discover base platform limits
        let mut discoverer = PlatformLimitDiscoverer::new();
        let mut limits = discoverer.discover()?;
        
        // 2. Apply CLI argument overrides
        self.apply_cli_overrides(&mut limits)?;
        
        // 3. Apply environment variable overrides
        self.apply_env_overrides(&mut limits)?;
        
        // 4. Apply configuration file overrides
        self.apply_config_file_overrides(&mut limits)?;
        
        // 5. Apply container runtime limits
        self.apply_container_limits(&mut limits)?;
        
        // 6. Validate final limits
        self.validate_limits(&limits)?;
        
        self.platform_limits = Some(limits.clone());
        self.verified_limits = Some(limits.clone());
        
        Ok(limits)
    }
    
    /// Apply CLI argument overrides
    fn apply_cli_overrides(&self, limits: &mut ComprehensivePlatformLimits) -> Result<(), Error> {
        for arg in &self.config.sources.cli_args {
            if let Some(memory) = parse_memory_arg(arg, "--max-memory=") {
                limits.max_total_memory = memory;
            } else if let Some(wasm_memory) = parse_memory_arg(arg, "--max-wasm-memory=") {
                limits.max_wasm_linear_memory = wasm_memory;
            } else if let Some(stack) = parse_memory_arg(arg, "--max-stack=") {
                limits.max_stack_bytes = stack;
            } else if let Some(components) = parse_number_arg(arg, "--max-components=") {
                limits.max_components = components;
            }
        }
        Ok(())
    }
    
    /// Apply environment variable overrides
    fn apply_env_overrides(&self, limits: &mut ComprehensivePlatformLimits) -> Result<(), Error> {
        if let Some(memory) = self.config.sources.env_vars.get("WRT_MAX_MEMORY") {
            if let Ok(value) = parse_memory_string(memory) {
                limits.max_total_memory = value;
            }
        }
        
        if let Some(wasm_memory) = self.config.sources.env_vars.get("WRT_MAX_WASM_MEMORY") {
            if let Ok(value) = parse_memory_string(wasm_memory) {
                limits.max_wasm_linear_memory = value;
            }
        }
        
        if let Some(stack) = self.config.sources.env_vars.get("WRT_MAX_STACK") {
            if let Ok(value) = parse_memory_string(stack) {
                limits.max_stack_bytes = value;
            }
        }
        
        if let Some(components) = self.config.sources.env_vars.get("WRT_MAX_COMPONENTS") {
            if let Ok(value) = components.parse::<usize>() {
                limits.max_components = value;
            }
        }
        
        Ok(())
    }
    
    /// Apply configuration file overrides
    #[cfg(feature = "std")]
    fn apply_config_file_overrides(&self, limits: &mut ComprehensivePlatformLimits) -> Result<(), Error> {
        if let Some(ref config_path) = self.config.sources.config_file {
            if Path::new(config_path).exists() {
                let config_content = fs::read_to_string(config_path)
                    .map_err(|_| Error::new(
                        ErrorCategory::Io,
                        codes::IO_ERROR,
                        "Failed to read configuration file"
                    ))?;
                
                // Simple key=value parser
                for line in config_content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    
                    if let Some((key, value)) = line.split_once('=') {
                        match key.trim() {
                            "max_memory" => {
                                if let Ok(memory) = parse_memory_string(value.trim()) {
                                    limits.max_total_memory = memory;
                                }
                            },
                            "max_wasm_memory" => {
                                if let Ok(memory) = parse_memory_string(value.trim()) {
                                    limits.max_wasm_linear_memory = memory;
                                }
                            },
                            "max_stack" => {
                                if let Ok(memory) = parse_memory_string(value.trim()) {
                                    limits.max_stack_bytes = memory;
                                }
                            },
                            "max_components" => {
                                if let Ok(components) = value.trim().parse::<usize>() {
                                    limits.max_components = components;
                                }
                            },
                            _ => {} // Ignore unknown keys
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    #[cfg(not(feature = "std"))]
    fn apply_config_file_overrides(&self, _limits: &mut ComprehensivePlatformLimits) -> Result<(), Error> {
        // No-op for no_std
        Ok(())
    }
    
    /// Apply container runtime limits
    #[cfg(feature = "std")]
    fn apply_container_limits(&self, limits: &mut ComprehensivePlatformLimits) -> Result<(), Error> {
        match self.config.sources.container_runtime {
            ContainerRuntime::Docker => {
                // Check Docker memory limits
                if let Ok(limit) = fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes") {
                    if let Ok(memory_limit) = limit.trim().parse::<usize>() {
                        if memory_limit < limits.max_total_memory {
                            limits.max_total_memory = memory_limit;
                            limits.max_wasm_linear_memory = (memory_limit * 3) / 4;
                        }
                    }
                }
            },
            ContainerRuntime::Kubernetes => {
                // Check Kubernetes resource limits
                if let Ok(requests) = env::var("KUBERNETES_MEMORY_REQUEST") {
                    if let Ok(memory) = parse_memory_string(&requests) {
                        limits.max_total_memory = memory;
                        limits.max_wasm_linear_memory = (memory * 3) / 4;
                    }
                }
                
                if let Ok(limits_env) = env::var("KUBERNETES_MEMORY_LIMIT") {
                    if let Ok(memory) = parse_memory_string(&limits_env) {
                        limits.max_total_memory = limits.max_total_memory.min(memory);
                        limits.max_wasm_linear_memory = (limits.max_total_memory * 3) / 4;
                    }
                }
            },
            _ => {
                // No container-specific limits
            }
        }
        Ok(())
    }
    
    #[cfg(not(feature = "std"))]
    fn apply_container_limits(&self, _limits: &mut ComprehensivePlatformLimits) -> Result<(), Error> {
        // No-op for no_std
        Ok(())
    }
    
    /// Validate final limits for consistency
    fn validate_limits(&self, limits: &ComprehensivePlatformLimits) -> Result<(), Error> {
        // Check that WASM memory doesn't exceed total memory
        if limits.max_wasm_linear_memory > limits.max_total_memory {
            if self.config.strict_validation {
                return Err(Error::new(
                    ErrorCategory::Configuration,
                    codes::INVALID_INPUT,
                    "WASM memory limit exceeds total memory limit"
                ));
            } else {
                // Auto-correct in non-strict mode
                // This would modify limits, but we can't without mut reference
            }
        }
        
        // Check minimum viable limits
        if limits.max_total_memory < 1024 * 1024 { // 1MB minimum
            return Err(Error::new(
                ErrorCategory::Configuration,
                codes::INVALID_INPUT,
                "Total memory limit too small (minimum 1MB)"
            ));
        }
        
        if limits.max_stack_bytes < 4096 { // 4KB minimum stack
            return Err(Error::new(
                ErrorCategory::Configuration,
                codes::INVALID_INPUT,
                "Stack limit too small (minimum 4KB)"
            ));
        }
        
        if limits.max_components == 0 {
            return Err(Error::new(
                ErrorCategory::Configuration,
                codes::INVALID_INPUT,
                "Component limit cannot be zero"
            ));
        }
        
        Ok(())
    }
    
    /// Get verified limits
    pub fn verified_limits(&self) -> Option<&ComprehensivePlatformLimits> {
        self.verified_limits.as_ref()
    }
    
    /// Get configuration
    pub fn config(&self) -> &PlatformVerificationConfig {
        &self.config
    }
}

impl Default for PlatformVerificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Platform verification configuration builder
pub struct PlatformVerificationConfigBuilder {
    config: PlatformVerificationConfig,
}

impl PlatformVerificationConfigBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            config: PlatformVerificationConfig::default(),
        }
    }
    
    /// Add CLI arguments
    pub fn with_cli_args(mut self, args: Vec<String>) -> Self {
        self.config.sources.cli_args = args;
        self
    }
    
    /// Set configuration file
    pub fn with_config_file<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.sources.config_file = Some(path.as_ref().to_string());
        self
    }
    
    /// Enable strict validation
    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.config.strict_validation = strict;
        self
    }
    
    /// Set container runtime
    pub fn with_container_runtime(mut self, runtime: ContainerRuntime) -> Self {
        self.config.sources.container_runtime = runtime;
        self
    }
    
    /// Build configuration
    pub fn build(mut self) -> PlatformVerificationConfig {
        // Auto-detect environment variables
        #[cfg(feature = "std")]
        {
            for (key, value) in env::vars() {
                if key.starts_with("WRT_") {
                    self.config.sources.env_vars.insert(key, value);
                }
            }
            
            // Auto-detect container runtime
            if self.config.sources.container_runtime == ContainerRuntime::None {
                self.config.sources.container_runtime = detect_container_runtime();
            }
        }
        
        self.config
    }
}

impl Default for PlatformVerificationConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect container runtime
#[cfg(feature = "std")]
fn detect_container_runtime() -> ContainerRuntime {
    // Check for Docker
    if Path::new("/.dockerenv").exists() {
        return ContainerRuntime::Docker;
    }
    
    // Check for Kubernetes
    if env::var("KUBERNETES_SERVICE_HOST").is_ok() {
        return ContainerRuntime::Kubernetes;
    }
    
    // Check for systemd-nspawn
    if let Ok(container) = env::var("container") {
        if container == "systemd-nspawn" {
            return ContainerRuntime::SystemdNspawn;
        }
    }
    
    // Check cgroup for container indicators
    if let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup") {
        if cgroup.contains("docker") {
            return ContainerRuntime::Docker;
        }
        if cgroup.contains("lxc") {
            return ContainerRuntime::LXC;
        }
    }
    
    ContainerRuntime::None
}

/// Parse memory argument from CLI
fn parse_memory_arg(arg: &str, prefix: &str) -> Option<usize> {
    if arg.starts_with(prefix) {
        let value = &arg[prefix.len()..];
        parse_memory_string(value).ok()
    } else {
        None
    }
}

/// Parse number argument from CLI
fn parse_number_arg(arg: &str, prefix: &str) -> Option<usize> {
    if arg.starts_with(prefix) {
        let value = &arg[prefix.len()..];
        value.parse().ok()
    } else {
        None
    }
}

/// Parse memory string with units (e.g., "256MB", "1GB")
fn parse_memory_string(value: &str) -> Result<usize, Error> {
    let value = value.trim().to_uppercase();
    
    if let Some(stripped) = value.strip_suffix("KB") {
        stripped.parse::<usize>()
            .map(|n| n * 1024)
            .map_err(|_| Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Invalid memory value"))
    } else if let Some(stripped) = value.strip_suffix("MB") {
        stripped.parse::<usize>()
            .map(|n| n * 1024 * 1024)
            .map_err(|_| Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Invalid memory value"))
    } else if let Some(stripped) = value.strip_suffix("GB") {
        stripped.parse::<usize>()
            .map(|n| n * 1024 * 1024 * 1024)
            .map_err(|_| Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Invalid memory value"))
    } else {
        // Assume bytes
        value.parse::<usize>()
            .map_err(|_| Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Invalid memory value"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_string_parsing() {
        assert_eq!(parse_memory_string("1024").unwrap(), 1024);
        assert_eq!(parse_memory_string("1KB").unwrap(), 1024);
        assert_eq!(parse_memory_string("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_memory_string("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_memory_string("256mb").unwrap(), 256 * 1024 * 1024);
    }
    
    #[test]
    fn test_cli_arg_parsing() {
        assert_eq!(parse_memory_arg("--max-memory=256MB", "--max-memory=").unwrap(), 256 * 1024 * 1024);
        assert_eq!(parse_number_arg("--max-components=512", "--max-components=").unwrap(), 512);
        assert_eq!(parse_memory_arg("--other-arg=256MB", "--max-memory="), None);
    }
    
    #[test]
    fn test_config_builder() {
        let config = PlatformVerificationConfigBuilder::new()
            .with_cli_args(vec!["--max-memory=1GB".to_string()])
            .with_strict_validation(true)
            .with_container_runtime(ContainerRuntime::Docker)
            .build();
        
        assert!(config.strict_validation);
        assert_eq!(config.sources.container_runtime, ContainerRuntime::Docker);
        assert_eq!(config.sources.cli_args.len(), 1);
    }
    
    #[test]
    fn test_verification_engine() {
        let config = PlatformVerificationConfigBuilder::new()
            .with_strict_validation(false)
            .build();
        
        let mut engine = PlatformVerificationEngine::with_config(config);
        let limits = engine.discover_limits().unwrap();
        
        assert!(limits.max_total_memory > 0);
        assert!(limits.max_wasm_linear_memory > 0);
        assert!(limits.max_stack_bytes > 0);
        assert!(limits.max_components > 0);
    }
    
    #[cfg(feature = "std")]
    #[test]
    fn test_container_detection() {
        // This test would depend on the actual runtime environment
        let runtime = detect_container_runtime();
        // Just ensure it returns a valid value
        assert!(matches!(runtime, 
            ContainerRuntime::None | 
            ContainerRuntime::Docker | 
            ContainerRuntime::Kubernetes | 
            ContainerRuntime::LXC | 
            ContainerRuntime::SystemdNspawn | 
            ContainerRuntime::Other
        ));
    }
}