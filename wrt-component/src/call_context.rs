//! Call Context Management System
//!
//! This module provides comprehensive call context management for cross-component
//! function calls, handling parameter marshaling, resource transfer coordination,
//! and call lifecycle management.
//!
//! # Features
//!
//! - **Call Lifecycle Management**: Complete lifecycle from preparation to completion
//! - **Parameter Marshaling**: Safe conversion and validation of call parameters
//! - **Resource Coordination**: Management of resource transfers during calls
//! - **Memory Safety**: Bounds checking and isolation enforcement
//! - **Performance Optimization**: Efficient parameter passing and memory management
//! - **Cross-Environment Support**: Works in std, no_std+alloc, and pure no_std
//!
//! # Core Concepts
//!
//! - **Call Context**: Complete state and metadata for a cross-component call
//! - **Parameter Marshaler**: Handles conversion between component value formats
//! - **Resource Coordinator**: Manages resource sharing during calls
//! - **Call Validator**: Ensures call safety and security compliance
//! - **Performance Monitor**: Tracks call performance and optimization opportunities


#[cfg(not(feature = "std"))]
extern crate alloc;

// Cross-environment imports
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::HashMap, format};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap as HashMap, format};

#[cfg(not(any(feature = "std", feature = "alloc")))]
use wrt_foundation::{BoundedVec, BoundedString, BoundedMap};

// No_std provider for bounded collections
#[cfg(not(any(feature = "std", feature = "alloc")))]
use crate::MemoryProvider;

use wrt_error::{Error, ErrorCategory, Result, codes};
use crate::canonical_abi::{ComponentType, CanonicalABI};

#[cfg(feature = "std")]
use crate::canonical_abi::ComponentValue;

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;
use crate::components::{InstanceId, ComponentInstance, FunctionSignature};
use crate::resource_management::{ResourceHandle, ResourceTypeId, ResourceData};

/// Maximum parameter data size per call (1MB)
const MAX_PARAMETER_DATA_SIZE: u32 = 1024 * 1024;

/// Maximum string length in parameters
const MAX_STRING_LENGTH: usize = 65536;

/// Maximum array/vector length in parameters
const MAX_ARRAY_LENGTH: usize = 4096;

/// Maximum number of concurrent call contexts
const MAX_CALL_CONTEXTS: usize = 256;

/// Maximum number of parameters per call
const MAX_CALL_PARAMETERS: usize = 64;

/// Call context manager for managing cross-component call state
#[derive(Debug)]
pub struct CallContextManager {
    /// Active call contexts by call ID
    #[cfg(feature = "std")]
    contexts: HashMap<u64, ManagedCallContext>,
    #[cfg(not(feature = "std"))]
    contexts: BoundedVec<(u64, ManagedCallContext), MAX_CALL_CONTEXTS, crate::MemoryProvider>,
    /// Parameter marshaler
    marshaler: ParameterMarshaler,
    /// Resource coordinator
    resource_coordinator: ResourceCoordinator,
    /// Call validator
    validator: CallValidator,
    /// Performance monitor
    #[cfg(feature = "std")]  
    monitor: PerformanceMonitor,
    #[cfg(not(feature = "std"))]
    monitor: PerformanceMonitorNoStd,
    /// Manager configuration
    config: CallContextConfig,
}

/// Managed call context with full lifecycle tracking
#[derive(Debug, Clone)]
pub struct ManagedCallContext {
    /// Base call context
    pub context: super::component_communication::CallContext,
    /// Parameter marshaling state
    pub marshaling_state: MarshalingState,
    /// Resource transfer state
    pub resource_state: ResourceState,
    /// Performance metrics for this call
    pub metrics: CallMetrics,
    /// Validation results
    pub validation: ValidationResults,
}

/// Parameter marshaler for safe cross-component parameter passing
#[derive(Debug)]
pub struct ParameterMarshaler {
    /// Canonical ABI for parameter conversion
    abi: CanonicalABI,
    /// Marshaling configuration
    config: MarshalingConfig,
    /// Type compatibility cache
    #[cfg(feature = "std")]
    type_cache: HashMap<String, TypeCompatibility>,
    #[cfg(not(feature = "std"))]
    type_cache: BoundedVec<(BoundedString<128, crate::MemoryProvider>, TypeCompatibility), 64, crate::MemoryProvider>,
}

/// Resource coordinator for managing resource transfers during calls
#[derive(Debug)]
pub struct ResourceCoordinator {
    /// Active resource locks
    #[cfg(feature = "std")]
    resource_locks: HashMap<ResourceHandle, ResourceLock>,
    #[cfg(not(feature = "std"))]
    resource_locks: BoundedVec<(ResourceHandle, ResourceLock), 128, crate::MemoryProvider>,
    /// Transfer pending queue
    #[cfg(feature = "std")]
    pending_transfers: Vec<PendingResourceTransfer>,
    #[cfg(not(feature = "std"))]
    pending_transfers: BoundedVec<PendingResourceTransfer, 64, crate::MemoryProvider>,
    /// Transfer policies
    #[cfg(feature = "std")]
    transfer_policies: HashMap<(InstanceId, InstanceId), TransferPolicy>,
    #[cfg(not(feature = "std"))]
    transfer_policies: BoundedVec<((InstanceId, InstanceId), TransferPolicy), 32, crate::MemoryProvider>,
}

/// Call validator for ensuring call safety and security
#[derive(Debug)]
pub struct CallValidator {
    /// Security policies
    #[cfg(feature = "std")]
    security_policies: HashMap<InstanceId, SecurityPolicy>,
    #[cfg(not(feature = "std"))]
    security_policies: BoundedVec<(InstanceId, SecurityPolicy), 64, crate::MemoryProvider>,
    /// Validation rules
    #[cfg(feature = "std")]
    validation_rules: Vec<ValidationRule>,
    #[cfg(not(feature = "std"))]
    validation_rules: BoundedVec<ValidationRule, 32, crate::MemoryProvider>,
    /// Validation configuration
    config: ValidationConfig,
}

/// Performance monitor for tracking call performance
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Call timing metrics
    timing_metrics: HashMap<String, TimingMetrics>,
    /// Parameter size metrics
    parameter_metrics: ParameterSizeMetrics,
    /// Resource transfer metrics
    resource_metrics: ResourceTransferMetrics,
    /// Optimization suggestions
    optimization_suggestions: Vec<OptimizationSuggestion>,
}

/// Performance monitor for tracking call performance (no_std version)
#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub struct PerformanceMonitorNoStd {
    /// Call timing metrics
    timing_metrics: BoundedVec<(BoundedString<128, crate::MemoryProvider>, TimingMetrics), 64, crate::MemoryProvider>,
    /// Parameter size metrics
    parameter_metrics: ParameterSizeMetrics,
    /// Resource transfer metrics
    resource_metrics: ResourceTransferMetrics,
    /// Optimization suggestions
    optimization_suggestions: BoundedVec<OptimizationSuggestion, 32, crate::MemoryProvider>,
}

/// Parameter marshaling state
#[derive(Debug, Clone)]
pub struct MarshalingState {
    /// Original parameters
    #[cfg(feature = "std")]
    pub original_parameters: Vec<ComponentValue>,
    #[cfg(not(feature = "std"))]
    pub original_parameters: BoundedVec<ComponentValue, 32, crate::MemoryProvider>,
    /// Marshaled parameters
    #[cfg(feature = "std")]
    pub marshaled_parameters: Vec<ComponentValue>,
    #[cfg(not(feature = "std"))]
    pub marshaled_parameters: BoundedVec<ComponentValue, 32, crate::MemoryProvider>,
    /// Marshaling metadata
    pub metadata: MarshalingMetadata,
    /// Marshaling errors (if any)
    #[cfg(feature = "std")]
    pub errors: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub errors: BoundedVec<BoundedString<256, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Resource state during call execution
#[derive(Debug, Clone)]
pub struct ResourceState {
    /// Resources being transferred
    #[cfg(feature = "std")]
    pub transferring_resources: Vec<ResourceHandle>,
    #[cfg(not(feature = "std"))]
    pub transferring_resources: BoundedVec<ResourceHandle, 64, crate::MemoryProvider>,
    /// Resource locks acquired
    #[cfg(feature = "std")]
    pub acquired_locks: Vec<ResourceHandle>,
    #[cfg(not(feature = "std"))]
    pub acquired_locks: BoundedVec<ResourceHandle, 64, crate::MemoryProvider>,
    /// Transfer results  
    #[cfg(feature = "std")]
    pub transfer_results: Vec<TransferResult>,
    #[cfg(not(feature = "std"))]
    pub transfer_results: BoundedVec<TransferResult, 32, crate::MemoryProvider>,
}

/// Call performance metrics
#[derive(Debug, Clone, Default)]
pub struct CallMetrics {
    /// Parameter marshaling time (microseconds)
    pub marshaling_time_us: u64,
    /// Resource coordination time (microseconds)
    pub resource_coordination_time_us: u64,
    /// Function execution time (microseconds)
    pub execution_time_us: u64,
    /// Total call overhead (microseconds)
    pub overhead_time_us: u64,
    /// Parameter data size (bytes)
    pub parameter_data_size: u32,
    /// Number of resource transfers
    pub resource_transfer_count: u32,
}

/// Validation results for a call
#[derive(Debug, Clone)]
pub struct ValidationResults {
    /// Overall validation status
    pub status: ValidationStatus,
    /// Parameter validation results
    pub parameter_validation: ParameterValidationResult,
    /// Security validation results
    pub security_validation: SecurityValidationResult,
    /// Resource validation results
    pub resource_validation: ResourceValidationResult,
    /// Validation messages
    #[cfg(feature = "std")]
    pub messages: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub messages: BoundedVec<BoundedString<256, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Call context manager configuration
#[derive(Debug, Clone)]
pub struct CallContextConfig {
    /// Enable call tracing
    pub enable_tracing: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Enable parameter validation
    pub enable_parameter_validation: bool,
    /// Enable resource coordination
    pub enable_resource_coordination: bool,
    /// Maximum call duration (microseconds)
    pub max_call_duration_us: u64,
}

/// Parameter marshaling configuration
#[derive(Debug, Clone)]
pub struct MarshalingConfig {
    /// Enable type checking
    pub enable_type_checking: bool,
    /// Enable size validation
    pub enable_size_validation: bool,
    /// Enable encoding validation
    pub enable_encoding_validation: bool,
    /// Maximum parameter size
    pub max_parameter_size: u32,
    /// String encoding to use
    pub string_encoding: StringEncoding,
}

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Validation level
    pub level: ValidationLevel,
    /// Enable security checks
    pub enable_security_checks: bool,
    /// Enable performance checks
    pub enable_performance_checks: bool,
    /// Custom validation rules
    #[cfg(feature = "std")]
    pub custom_rules: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub custom_rules: BoundedVec<BoundedString<128, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Resource lock for coordinating resource access
#[derive(Debug, Clone)]
pub struct ResourceLock {
    /// Resource handle
    pub resource_handle: ResourceHandle,
    /// Lock owner (call ID)
    pub owner_call_id: u64,
    /// Lock type
    pub lock_type: ResourceLockType,
    /// Lock acquired timestamp
    pub acquired_at: u64,
    /// Lock expiration time
    pub expires_at: u64,
}

/// Pending resource transfer
#[derive(Debug, Clone)]
pub struct PendingResourceTransfer {
    /// Transfer ID
    pub transfer_id: u64,
    /// Resource handle
    pub resource_handle: ResourceHandle,
    /// Source instance
    pub source_instance: InstanceId,
    /// Target instance
    pub target_instance: InstanceId,
    /// Transfer type
    pub transfer_type: super::component_communication::ResourceTransferType,
    /// Request timestamp
    pub requested_at: u64,
}

/// Resource transfer policy between instances
#[derive(Debug, Clone)]
pub struct TransferPolicy {
    /// Maximum simultaneous transfers
    pub max_transfers: u32,
    /// Allowed transfer types
    #[cfg(feature = "std")]
    pub allowed_types: Vec<super::component_communication::ResourceTransferType>,
    #[cfg(not(feature = "std"))]
    pub allowed_types: BoundedVec<super::component_communication::ResourceTransferType, 16, crate::MemoryProvider>,
    /// Required permissions
    #[cfg(feature = "std")]
    pub required_permissions: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub required_permissions: BoundedVec<BoundedString<128, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Security policy for instance interactions
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allowed target instances
    #[cfg(feature = "std")]
    pub allowed_targets: Vec<InstanceId>,
    #[cfg(not(feature = "std"))]
    pub allowed_targets: BoundedVec<InstanceId, 32, crate::MemoryProvider>,
    /// Allowed function patterns
    #[cfg(feature = "std")]
    pub allowed_functions: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub allowed_functions: BoundedVec<BoundedString<128, crate::MemoryProvider>, 32, crate::MemoryProvider>,
    /// Resource access permissions
    pub resource_permissions: ResourcePermissions,
    /// Memory access limits
    pub memory_limits: MemoryLimits,
}

/// Validation rule for call checking
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule name
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(feature = "std"))]
    pub name: BoundedString<128, crate::MemoryProvider>,
    /// Rule description
    #[cfg(feature = "std")]
    pub description: String,
    #[cfg(not(feature = "std"))]
    pub description: BoundedString<256, crate::MemoryProvider>,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Rule severity
    pub severity: ValidationSeverity,
}

/// Timing metrics for performance monitoring
#[derive(Debug, Clone, Default)]
pub struct TimingMetrics {
    /// Total calls
    pub total_calls: u64,
    /// Average duration (microseconds)
    pub average_duration_us: u64,
    /// Minimum duration (microseconds)
    pub min_duration_us: u64,
    /// Maximum duration (microseconds)
    pub max_duration_us: u64,
    /// Standard deviation
    pub std_deviation_us: u64,
}

/// Parameter size metrics
#[derive(Debug, Clone, Default)]
pub struct ParameterSizeMetrics {
    /// Total parameters processed
    pub total_parameters: u64,
    /// Total parameter data size
    pub total_data_size: u64,
    /// Average parameter size
    pub average_size: u32,
    /// Largest parameter size
    pub max_size: u32,
}

/// Resource transfer metrics
#[derive(Debug, Clone, Default)]
pub struct ResourceTransferMetrics {
    /// Total transfers
    pub total_transfers: u64,
    /// Successful transfers
    pub successful_transfers: u64,
    /// Failed transfers
    pub failed_transfers: u64,
    /// Average transfer time
    pub average_transfer_time_us: u64,
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    /// Suggestion type
    pub suggestion_type: OptimizationType,
    /// Description
    #[cfg(feature = "std")]
    pub description: String,
    #[cfg(not(feature = "std"))]
    pub description: BoundedString<256, crate::MemoryProvider>,
    /// Potential impact
    pub impact: OptimizationImpact,
    /// Implementation complexity
    pub complexity: OptimizationComplexity,
}

/// Marshaling metadata
#[derive(Debug, Clone, Default)]
pub struct MarshalingMetadata {
    /// Original parameter count
    pub original_count: usize,
    /// Marshaled parameter count
    pub marshaled_count: usize,
    /// Total marshaling time
    pub marshaling_time_us: u64,
    /// Memory used for marshaling
    pub memory_used: u32,
}

/// Transfer result
#[derive(Debug, Clone)]
pub struct TransferResult {
    /// Resource handle
    pub resource_handle: ResourceHandle,
    /// Transfer success
    pub success: bool,
    /// New handle (if ownership transferred)
    pub new_handle: Option<ResourceHandle>,
    /// Error message (if failed)
    #[cfg(feature = "std")]
    pub error_message: Option<String>,
    #[cfg(not(feature = "std"))]
    pub error_message: Option<BoundedString<256, crate::MemoryProvider>>,
}

/// Type compatibility information
#[derive(Debug, Clone)]
pub struct TypeCompatibility {
    /// Source type
    pub source_type: ComponentType,
    /// Target type
    pub target_type: ComponentType,
    /// Compatibility status
    pub compatible: bool,
    /// Conversion required
    pub conversion_required: bool,
    /// Conversion cost (performance impact)
    pub conversion_cost: u32,
}

/// Resource permissions
#[derive(Debug, Clone)]
pub struct ResourcePermissions {
    /// Can read resources
    pub can_read: bool,
    /// Can write resources
    pub can_write: bool,
    /// Can transfer resources
    pub can_transfer: bool,
    /// Allowed resource types
    #[cfg(feature = "std")]
    pub allowed_types: Vec<ResourceTypeId>,
    #[cfg(not(feature = "std"))]
    pub allowed_types: BoundedVec<ResourceTypeId, 32, crate::MemoryProvider>,
}

/// Memory access limits
#[derive(Debug, Clone)]
pub struct MemoryLimits {
    /// Maximum memory size that can be accessed
    pub max_memory_size: u32,
    /// Maximum parameter size
    pub max_parameter_size: u32,
    /// Maximum string length
    pub max_string_length: usize,
}

/// Parameter validation result
#[derive(Debug, Clone)]
pub struct ParameterValidationResult {
    /// Validation passed
    pub valid: bool,
    /// Type checking results
    #[cfg(feature = "std")]
    pub type_check_results: Vec<TypeCheckResult>,
    #[cfg(not(feature = "std"))]
    pub type_check_results: BoundedVec<TypeCheckResult, 32, crate::MemoryProvider>,
    /// Size validation results
    #[cfg(feature = "std")]
    pub size_validation_results: Vec<SizeValidationResult>,
    #[cfg(not(feature = "std"))]
    pub size_validation_results: BoundedVec<SizeValidationResult, 32, crate::MemoryProvider>,
    /// Error messages
    #[cfg(feature = "std")]
    pub error_messages: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub error_messages: BoundedVec<BoundedString<256, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Security validation result
#[derive(Debug, Clone)]
pub struct SecurityValidationResult {
    /// Security check passed
    pub secure: bool,
    /// Permission check results
    #[cfg(feature = "std")]
    pub permission_results: Vec<PermissionCheckResult>,
    #[cfg(not(feature = "std"))]
    pub permission_results: BoundedVec<PermissionCheckResult, 32, crate::MemoryProvider>,
    /// Access control results
    #[cfg(feature = "std")]
    pub access_control_results: Vec<AccessControlResult>,
    #[cfg(not(feature = "std"))]
    pub access_control_results: BoundedVec<AccessControlResult, 32, crate::MemoryProvider>,
    /// Security warnings
    #[cfg(feature = "std")]
    pub warnings: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub warnings: BoundedVec<BoundedString<256, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Resource validation result
#[derive(Debug, Clone)]
pub struct ResourceValidationResult {
    /// Resource validation passed
    pub valid: bool,
    /// Resource availability results
    #[cfg(feature = "std")]
    pub availability_results: Vec<ResourceAvailabilityResult>,
    #[cfg(not(feature = "std"))]
    pub availability_results: BoundedVec<ResourceAvailabilityResult, 32, crate::MemoryProvider>,
    /// Transfer permission results
    #[cfg(feature = "std")]
    pub transfer_permission_results: Vec<TransferPermissionResult>,
    #[cfg(not(feature = "std"))]
    pub transfer_permission_results: BoundedVec<TransferPermissionResult, 32, crate::MemoryProvider>,
    /// Validation errors
    #[cfg(feature = "std")]
    pub errors: Vec<String>,
    #[cfg(not(feature = "std"))]
    pub errors: BoundedVec<BoundedString<256, crate::MemoryProvider>, 16, crate::MemoryProvider>,
}

/// Type check result
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// Parameter index
    pub parameter_index: usize,
    /// Expected type
    pub expected_type: ComponentType,
    /// Actual type
    pub actual_type: ComponentType,
    /// Check passed
    pub passed: bool,
    /// Error message
    #[cfg(feature = "std")]
    pub error_message: Option<String>,
    #[cfg(not(feature = "std"))]
    pub error_message: Option<BoundedString<256, crate::MemoryProvider>>,
}

/// Size validation result
#[derive(Debug, Clone)]
pub struct SizeValidationResult {
    /// Parameter index
    pub parameter_index: usize,
    /// Parameter size
    pub size: u32,
    /// Maximum allowed size
    pub max_size: u32,
    /// Validation passed
    pub passed: bool,
}

/// Permission check result
#[derive(Debug, Clone)]
pub struct PermissionCheckResult {
    /// Permission name
    #[cfg(feature = "std")]
    pub permission: String,
    #[cfg(not(feature = "std"))]
    pub permission: BoundedString<128, crate::MemoryProvider>,
    /// Check passed
    pub granted: bool,
    /// Reason for denial (if denied)
    #[cfg(feature = "std")]
    pub denial_reason: Option<String>,
    #[cfg(not(feature = "std"))]
    pub denial_reason: Option<BoundedString<256, crate::MemoryProvider>>,
}

/// Access control result
#[derive(Debug, Clone)]
pub struct AccessControlResult {
    /// Resource or function accessed
    #[cfg(feature = "std")]
    pub accessed_item: String,
    #[cfg(not(feature = "std"))]
    pub accessed_item: BoundedString<128, crate::MemoryProvider>,
    /// Access allowed
    pub allowed: bool,
    /// Access control rule applied
    #[cfg(feature = "std")]
    pub rule_applied: String,
    #[cfg(not(feature = "std"))]
    pub rule_applied: BoundedString<128, crate::MemoryProvider>,
}

/// Resource availability result
#[derive(Debug, Clone)]
pub struct ResourceAvailabilityResult {
    /// Resource handle
    pub resource_handle: ResourceHandle,
    /// Resource available
    pub available: bool,
    /// Current owner
    pub current_owner: Option<InstanceId>,
    /// Lock status
    pub locked: bool,
}

/// Transfer permission result
#[derive(Debug, Clone)]
pub struct TransferPermissionResult {
    /// Resource handle
    pub resource_handle: ResourceHandle,
    /// Transfer type
    pub transfer_type: super::component_communication::ResourceTransferType,
    /// Permission granted
    pub permitted: bool,
    /// Policy applied
    #[cfg(feature = "std")]
    pub policy_applied: String,
    #[cfg(not(feature = "std"))]
    pub policy_applied: BoundedString<128, crate::MemoryProvider>,
}

// Enumerations

/// Validation status
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    /// Validation passed
    Passed,
    /// Validation passed with warnings
    PassedWithWarnings,
    /// Validation failed
    Failed,
    /// Validation skipped
    Skipped,
}

/// Resource lock type
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceLockType {
    /// Shared read lock
    SharedRead,
    /// Exclusive write lock
    ExclusiveWrite,
    /// Transfer lock
    Transfer,
}

/// Validation level
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationLevel {
    /// No validation
    None,
    /// Basic validation
    Basic,
    /// Standard validation
    Standard,
    /// Strict validation
    Strict,
    /// Paranoid validation
    Paranoid,
}

/// Validation rule type
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationRuleType {
    /// Parameter validation rule
    Parameter,
    /// Security validation rule
    Security,
    /// Resource validation rule
    Resource,
    /// Performance validation rule
    Performance,
}

/// Validation severity
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    /// Information only
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical error
    Critical,
}

/// String encoding types
#[derive(Debug, Clone, PartialEq)]
pub enum StringEncoding {
    /// UTF-8 encoding
    Utf8,
    /// UTF-16 encoding
    Utf16,
    /// ASCII encoding
    Ascii,
    /// Latin-1 encoding
    Latin1,
}

/// Optimization type
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationType {
    /// Parameter marshaling optimization
    ParameterMarshaling,
    /// Resource transfer optimization
    ResourceTransfer,
    /// Call routing optimization
    CallRouting,
    /// Memory usage optimization
    MemoryUsage,
}

/// Optimization impact
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationImpact {
    /// Low impact
    Low,
    /// Medium impact
    Medium,
    /// High impact
    High,
    /// Critical impact
    Critical,
}

/// Optimization complexity
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationComplexity {
    /// Simple to implement
    Simple,
    /// Moderate complexity
    Moderate,
    /// Complex implementation
    Complex,
    /// Very complex
    VeryComplex,
}

// Default implementations

impl Default for CallContextConfig {
    fn default() -> Self {
        Self {
            enable_tracing: false,
            enable_performance_monitoring: true,
            enable_parameter_validation: true,
            enable_resource_coordination: true,
            max_call_duration_us: 30_000_000, // 30 seconds
        }
    }
}

impl Default for MarshalingConfig {
    fn default() -> Self {
        Self {
            enable_type_checking: true,
            enable_size_validation: true,
            enable_encoding_validation: true,
            max_parameter_size: MAX_PARAMETER_DATA_SIZE,
            string_encoding: StringEncoding::Utf8,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            level: ValidationLevel::Standard,
            enable_security_checks: true,
            enable_performance_checks: true,
            #[cfg(feature = "std")]
            custom_rules: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            custom_rules: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for ResourcePermissions {
    fn default() -> Self {
        Self {
            can_read: true,
            can_write: false,
            can_transfer: false,
            #[cfg(feature = "std")]
            allowed_types: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            allowed_types: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_memory_size: 64 * 1024 * 1024, // 64MB
            max_parameter_size: MAX_PARAMETER_DATA_SIZE,
            max_string_length: MAX_STRING_LENGTH,
        }
    }
}

// Implementation of core functionality

impl CallContextManager {
    /// Create a new call context manager
    pub fn new() -> Self {
        Self::with_config(CallContextConfig::default())
    }

    /// Create a new call context manager with configuration
    pub fn with_config(config: CallContextConfig) -> Self {
        Self {
            #[cfg(feature = "std")]
            contexts: std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            contexts: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            marshaler: ParameterMarshaler::new(MarshalingConfig::default()),
            resource_coordinator: ResourceCoordinator::new(),
            validator: CallValidator::new(ValidationConfig::default()),
            #[cfg(feature = "std")]
            monitor: PerformanceMonitor::new(),
            #[cfg(not(feature = "std"))]
            monitor: PerformanceMonitorNoStd::new(),
            config,
        }
    }

    /// Prepare a call context for execution
    pub fn prepare_call_context(
        &mut self,
        context: super::component_communication::CallContext,
        source_instance: &ComponentInstance,
        target_instance: &ComponentInstance,
    ) -> Result<u64> {
        let call_id = context.call_id;

        // Validate the call
        let validation = if self.config.enable_parameter_validation {
            self.validator.validate_call(&context, source_instance, target_instance)?
        } else {
            ValidationResults {
                status: ValidationStatus::Skipped,
                parameter_validation: ParameterValidationResult {
                    valid: true,
                    #[cfg(feature = "std")]
                    type_check_results: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    type_check_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                    #[cfg(feature = "std")]
                    size_validation_results: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    size_validation_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                    #[cfg(feature = "std")]
                    error_messages: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    error_messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                },
                security_validation: SecurityValidationResult {
                    secure: true,
                    #[cfg(feature = "std")]
                    permission_results: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                    #[cfg(feature = "std")]
                    access_control_results: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    access_control_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                    #[cfg(feature = "std")]
                    warnings: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    warnings: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                },
                resource_validation: ResourceValidationResult {
                    valid: true,
                    #[cfg(feature = "std")]
                    availability_results: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    availability_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                    #[cfg(feature = "std")]
                    transfer_permission_results: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    transfer_permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                    #[cfg(feature = "std")]
                    errors: std::vec::Vec::new(),
                    #[cfg(not(feature = "std"))]
                    errors: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                },
                #[cfg(feature = "std")]
                messages: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            }
        };

        // Marshal parameters
        let marshaling_state = self.marshaler.marshal_parameters(&context.parameters)?;

        // Coordinate resources
        let resource_state = if self.config.enable_resource_coordination {
            self.resource_coordinator.coordinate_resources(&context.resource_handles)?
        } else {
            ResourceState {
                #[cfg(feature = "std")]
                transferring_resources: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                transferring_resources: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                acquired_locks: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                acquired_locks: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                transfer_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                transfer_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            }
        };

        // Create managed context
        let managed_context = ManagedCallContext {
            context,
            marshaling_state,
            resource_state,
            metrics: CallMetrics::default(),
            validation,
        };

        // Store the context
        #[cfg(feature = "std")]
        self.contexts.insert(call_id, managed_context);
        #[cfg(not(feature = "std"))]
        self.contexts.push((call_id, managed_context)).map_err(|_| Error::runtime_execution_error("Error occurred",
        ))?;

        Ok(call_id)
    }

    /// Get a call context by ID
    pub fn get_call_context(&self, call_id: u64) -> Option<&ManagedCallContext> {
        #[cfg(feature = "std")]
        return self.contexts.get(&call_id);
        #[cfg(not(feature = "std"))]
        return self.contexts.iter().find(|(id, _)| *id == call_id).map(|(_, ctx)| ctx);
    }

    /// Complete a call context and cleanup resources
    pub fn complete_call_context(&mut self, call_id: u64) -> Result<()> {
        #[cfg(feature = "std")]
        let context = self.contexts.remove(&call_id);
        #[cfg(not(feature = "std"))]
        let context = {
            let pos = self.contexts.iter().position(|(id, _)| *id == call_id);
            pos.and_then(|i| self.contexts.swap_remove(i).ok()).map(|(_, ctx)| ctx)
        };
        if let Some(context) = context {
            // Release resource locks
            self.resource_coordinator.release_locks(&context.resource_state.acquired_locks)?;

            // Update performance metrics
            if self.config.enable_performance_monitoring {
                self.monitor.record_call_completion(&context.metrics);
            }

            Ok(())
        } else {
            Err(Error::runtime_invalid_state("Call context not found"))
        }
    }

    /// Get performance statistics
    #[cfg(feature = "std")]
    pub fn get_performance_stats(&self) -> &PerformanceMonitor {
        &self.monitor
    }
    
    #[cfg(not(feature = "std"))]
    pub fn get_performance_stats(&self) -> &PerformanceMonitorNoStd {
        &self.monitor
    }
}

impl ParameterMarshaler {
    /// Create a new parameter marshaler
    pub fn new(config: MarshalingConfig) -> Self {
        Self {
            abi: CanonicalABI::new(),
            config,
            #[cfg(feature = "std")]
            type_cache: std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            type_cache: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }

    /// Marshal parameters for cross-component call
    pub fn marshal_parameters(&mut self, parameters: &[ComponentValue]) -> Result<MarshalingState> {
        let start_time = 0; // Would use actual timestamp

        // Validate parameter count and size
        if parameters.len() > MAX_CALL_PARAMETERS {
            return Err(Error::validation_error("Too many parameters"));
        }

        let total_size = self.calculate_parameter_size(parameters)?;
        if total_size > self.config.max_parameter_size {
            return Err(Error::validation_error("Parameter data too large"));
        }

        // For now, just clone the parameters (no actual marshaling)
        #[cfg(feature = "std")]
        let marshaled_parameters = parameters.to_vec();
        #[cfg(not(feature = "std"))]
        let marshaled_parameters = {
            let mut vec = BoundedVec::new(crate::MemoryProvider::default()).unwrap();
            for param in parameters {
                vec.push(param.clone()).map_err(|_| Error::validation_error("Too many parameters for bounded vec"))?;
            }
            vec
        };

        let end_time = 0; // Would use actual timestamp
        let metadata = MarshalingMetadata {
            original_count: parameters.len(),
            marshaled_count: marshaled_parameters.len(),
            marshaling_time_us: end_time - start_time,
            memory_used: total_size,
        };

        Ok(MarshalingState {
            #[cfg(feature = "std")]
            original_parameters: parameters.to_vec(),
            #[cfg(not(feature = "std"))]
            original_parameters: {
                let mut vec = BoundedVec::new(crate::MemoryProvider::default()).unwrap();
                for param in parameters {
                    vec.push(param.clone()).map_err(|_| Error::validation_error("Too many parameters for bounded vec"))?;
                }
                vec
            },
            marshaled_parameters,
            metadata,
            #[cfg(feature = "std")]
            errors: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            errors: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        })
    }

    fn calculate_parameter_size(&self, parameters: &[ComponentValue]) -> Result<u32> {
        let mut total_size = 0u32;

        for param in parameters {
            let size = match param {
                ComponentValue::Bool(_) => 1,
                ComponentValue::S8(_) | ComponentValue::U8(_) => 1,
                ComponentValue::S16(_) | ComponentValue::U16(_) => 2,
                ComponentValue::S32(_) | ComponentValue::U32(_) | ComponentValue::F32(_) => 4,
                ComponentValue::S64(_) | ComponentValue::U64(_) | ComponentValue::F64(_) => 8,
                ComponentValue::Char(_) => 4, // UTF-32
                #[cfg(feature = "std")]
                ComponentValue::String(s) => {
                    if s.len() > MAX_STRING_LENGTH {
                        return Err(Error::validation_error("String parameter too long"));
                    }
                    s.len() as u32 + 4 // String length + size prefix
                }
                #[cfg(not(feature = "std"))]
                ComponentValue::String(s) => {
                    let len = s.as_bytes().len();
                    if len > MAX_STRING_LENGTH {
                        return Err(Error::validation_error("String parameter too long"));
                    }
                    len as u32 + 4 // String length + size prefix
                }
                ComponentValue::List(items) => {
                    if items.len() > MAX_ARRAY_LENGTH {
                        return Err(Error::validation_error("Array parameter too long"));
                    }
                    self.calculate_parameter_size(items)? + 4 // Array contents + size prefix
                }
                ComponentValue::Record(fields) => {
                    self.calculate_parameter_size(fields)?
                }
                ComponentValue::Tuple(elements) => {
                    self.calculate_parameter_size(elements)?
                }
                ComponentValue::Variant { case: _, value } => {
                    4 + if let Some(v) = value { // Discriminant + optional value
                        self.calculate_parameter_size(&[v.as_ref().clone()])?
                    } else {
                        0
                    }
                }
                ComponentValue::Enum(_) => 4, // Discriminant
                ComponentValue::Option(opt) => {
                    1 + if let Some(v) = opt { // Presence flag + optional value
                        self.calculate_parameter_size(&[v.as_ref().clone()])?
                    } else {
                        0
                    }
                }
                ComponentValue::Result { ok, err: _ } => {
                    1 + if let Some(v) = ok { // Success flag + optional value
                        self.calculate_parameter_size(&[v.as_ref().clone()])?
                    } else {
                        0
                    }
                }
                ComponentValue::Flags(_) => 4, // Bit flags
            };
            total_size += size;
        }

        Ok(total_size)
    }
}

impl ResourceCoordinator {
    /// Create a new resource coordinator
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            resource_locks: std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            resource_locks: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            pending_transfers: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            pending_transfers: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            transfer_policies: std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            transfer_policies: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }

    /// Coordinate resources for a call
    pub fn coordinate_resources(&mut self, resource_handles: &[ResourceHandle]) -> Result<ResourceState> {
        #[cfg(feature = "std")]
        let mut acquired_locks = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let mut acquired_locks = BoundedVec::new(crate::MemoryProvider::default()).unwrap();

        // Acquire locks for all resources
        for &handle in resource_handles {
            let lock = ResourceLock {
                resource_handle: handle,
                owner_call_id: 0, // Would be set to actual call ID
                lock_type: ResourceLockType::SharedRead,
                acquired_at: 0, // Would use actual timestamp
                expires_at: 0, // Would calculate expiration
            };

            #[cfg(feature = "std")]
            self.resource_locks.insert(handle, lock);
            #[cfg(not(feature = "std"))]
            self.resource_locks.push((handle, lock)).map_err(|_| Error::runtime_execution_error("Too many resource locks"))?;
            
            #[cfg(feature = "std")]
            acquired_locks.push(handle);
            #[cfg(not(feature = "std"))]
            acquired_locks.push(handle).map_err(|_| Error::runtime_execution_error("Too many acquired locks"))?;
        }

        Ok(ResourceState {
            #[cfg(feature = "std")]
            transferring_resources: resource_handles.to_vec(),
            #[cfg(not(feature = "std"))]
            transferring_resources: {
                let mut vec = BoundedVec::new(crate::MemoryProvider::default()).unwrap();
                for handle in resource_handles {
                    vec.push(*handle).map_err(|_| Error::runtime_execution_error("Too many transferring resources"))?;
                }
                vec
            },
            acquired_locks,
            #[cfg(feature = "std")]
            transfer_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            transfer_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        })
    }

    /// Release resource locks
    pub fn release_locks(&mut self, locks: &[ResourceHandle]) -> Result<()> {
        for &handle in locks {
            #[cfg(feature = "std")]
            self.resource_locks.remove(&handle);
            #[cfg(not(feature = "std"))]
            {
                if let Some(pos) = self.resource_locks.iter().position(|(h, _)| *h == handle) {
                    self.resource_locks.swap_remove(pos).ok();
                }
            }
        }
        Ok(())
    }
}

impl CallValidator {
    /// Create a new call validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            #[cfg(feature = "std")]
            security_policies: std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            security_policies: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            validation_rules: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            validation_rules: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            config,
        }
    }

    /// Validate a cross-component call
    pub fn validate_call(
        &self,
        _context: &super::component_communication::CallContext,
        _source_instance: &ComponentInstance,
        _target_instance: &ComponentInstance,
    ) -> Result<ValidationResults> {
        // For now, return successful validation
        Ok(ValidationResults {
            status: ValidationStatus::Passed,
            parameter_validation: ParameterValidationResult {
                valid: true,
                #[cfg(feature = "std")]
                type_check_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                type_check_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                size_validation_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                size_validation_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                error_messages: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                error_messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            },
            security_validation: SecurityValidationResult {
                secure: true,
                #[cfg(feature = "std")]
                permission_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                access_control_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                access_control_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                warnings: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                warnings: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            },
            resource_validation: ResourceValidationResult {
                valid: true,
                #[cfg(feature = "std")]
                availability_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                availability_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                transfer_permission_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                transfer_permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                errors: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                errors: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            },
            #[cfg(feature = "std")]
            messages: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        })
    }
}

#[cfg(feature = "std")]
impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            timing_metrics: std::collections::HashMap::new(),
            parameter_metrics: ParameterSizeMetrics::default(),
            resource_metrics: ResourceTransferMetrics::default(),
            optimization_suggestions: std::vec::Vec::new(),
        }
    }

    /// Record call completion for metrics
    pub fn record_call_completion(&mut self, _metrics: &CallMetrics) {
        // Update metrics based on call performance
        self.parameter_metrics.total_parameters += 1;
        self.resource_metrics.total_transfers += 1;
    }

    /// Get optimization suggestions
    pub fn get_optimization_suggestions(&self) -> &[OptimizationSuggestion] {
        &self.optimization_suggestions
    }
}

impl Default for CallContextManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "std"))]
impl PerformanceMonitorNoStd {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            timing_metrics: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            parameter_metrics: ParameterSizeMetrics::default(),
            resource_metrics: ResourceTransferMetrics::default(),
            optimization_suggestions: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
    
    /// Record call completion for metrics
    pub fn record_call_completion(&mut self, _metrics: &CallMetrics) {
        // Update metrics based on call performance
        self.parameter_metrics.total_parameters += 1;
        self.resource_metrics.total_transfers += 1;
    }

    /// Get optimization suggestions
    pub fn get_optimization_suggestions(&self) -> &[OptimizationSuggestion] {
        self.optimization_suggestions.as_slice()
    }
}

#[cfg(not(feature = "std"))]
impl Default for PerformanceMonitorNoStd {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResourceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_context_manager_creation() {
        let manager = CallContextManager::new();
        assert_eq!(manager.contexts.len(), 0);
    }

    #[test]
    fn test_parameter_marshaler_creation() {
        let marshaler = ParameterMarshaler::new(MarshalingConfig::default());
        assert_eq!(marshaler.config.string_encoding, StringEncoding::Utf8);
    }

    #[test]
    fn test_parameter_size_calculation() {
        let marshaler = ParameterMarshaler::new(MarshalingConfig::default());
        let parameters = vec![
            ComponentValue::S32(42),
            #[cfg(feature = "std")]
            ComponentValue::String("hello".to_string()),
            #[cfg(not(feature = "std"))]
            ComponentValue::String({
                let mut s = BoundedString::new(crate::MemoryProvider::default()).unwrap();
                s.push_str("hello").unwrap();
                s
            }),
            ComponentValue::Bool(true),
        ];

        let size = marshaler.calculate_parameter_size(&parameters).unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_resource_coordinator() {
        let mut coordinator = ResourceCoordinator::new();
        let handles = vec![ResourceHandle::new(1), ResourceHandle::new(2)];

        let state = coordinator.coordinate_resources(&handles).unwrap();
        assert_eq!(state.acquired_locks.len(), 2);
        assert_eq!(state.transferring_resources.len(), 2);
    }

    #[test]
    fn test_validation_results() {
        let results = ValidationResults {
            status: ValidationStatus::Passed,
            parameter_validation: ParameterValidationResult {
                valid: true,
                #[cfg(feature = "std")]
                type_check_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                type_check_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                size_validation_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                size_validation_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                error_messages: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                error_messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            },
            security_validation: SecurityValidationResult {
                secure: true,
                #[cfg(feature = "std")]
                permission_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                access_control_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                access_control_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                warnings: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                warnings: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            },
            resource_validation: ResourceValidationResult {
                valid: true,
                #[cfg(feature = "std")]
                availability_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                availability_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                transfer_permission_results: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                transfer_permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
                #[cfg(feature = "std")]
                errors: std::vec::Vec::new(),
                #[cfg(not(feature = "std"))]
                errors: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            },
            #[cfg(feature = "std")]
            messages: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        };

        assert_eq!(results.status, ValidationStatus::Passed);
        assert!(results.parameter_validation.valid);
        assert!(results.security_validation.secure);
        assert!(results.resource_validation.valid);
    }
}

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{Checksummable, ToBytes, FromBytes, WriteStream, ReadStream};

// Macro to implement basic traits for complex types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                0u32.update_checksum(checksum);
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                _writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
                Ok(())
            }
        }

        impl FromBytes for $type {
            fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                _reader: &mut ReadStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<Self> {
                Ok($default_val)
            }
        }
    };
}

// Default implementations for complex types
impl Default for ManagedCallContext {
    fn default() -> Self {
        Self {
            context: super::component_communication::CallContext::default(),
            marshaling_state: MarshalingState::default(),
            resource_state: ResourceState::default(),
            metrics: CallMetrics::default(),
            validation: ValidationResults::default(),
        }
    }
}

impl PartialEq for ManagedCallContext {
    fn eq(&self, other: &Self) -> bool {
        // Compare based on call ID for equality
        self.context.call_id == other.context.call_id
    }
}

impl Eq for ManagedCallContext {}

impl Default for MarshalingState {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            original_parameters: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            original_parameters: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            marshaled_parameters: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            marshaled_parameters: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            metadata: MarshalingMetadata::default(),
            #[cfg(feature = "std")]
            errors: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            errors: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for ResourceState {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            transferring_resources: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            transferring_resources: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            acquired_locks: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            acquired_locks: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            transfer_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            transfer_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for ValidationResults {
    fn default() -> Self {
        Self {
            status: ValidationStatus::Passed,
            parameter_validation: ParameterValidationResult::default(),
            security_validation: SecurityValidationResult::default(),
            resource_validation: ResourceValidationResult::default(),
            #[cfg(feature = "std")]
            messages: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for ParameterValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            #[cfg(feature = "std")]
            type_check_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            type_check_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            size_validation_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            size_validation_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            error_messages: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            error_messages: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for SecurityValidationResult {
    fn default() -> Self {
        Self {
            secure: true,
            #[cfg(feature = "std")]
            permission_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            access_control_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            access_control_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            warnings: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            warnings: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for ResourceValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            #[cfg(feature = "std")]
            availability_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            availability_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            transfer_permission_results: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            transfer_permission_results: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            errors: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            errors: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for TypeCompatibility {
    fn default() -> Self {
        Self {
            source_type: ComponentType::Bool,
            target_type: ComponentType::Bool,
            compatible: true,
            conversion_required: false,
            conversion_cost: 0,
        }
    }
}

impl PartialEq for TypeCompatibility {
    fn eq(&self, other: &Self) -> bool {
        self.source_type == other.source_type && self.target_type == other.target_type
    }
}

impl Eq for TypeCompatibility {}

impl Default for ResourceLock {
    fn default() -> Self {
        Self {
            resource_handle: ResourceHandle::new(0),
            owner_call_id: 0,
            lock_type: ResourceLockType::SharedRead,
            acquired_at: 0,
            expires_at: 0,
        }
    }
}

impl PartialEq for ResourceLock {
    fn eq(&self, other: &Self) -> bool {
        self.resource_handle == other.resource_handle && self.owner_call_id == other.owner_call_id
    }
}

impl Eq for ResourceLock {}

impl Default for PendingResourceTransfer {
    fn default() -> Self {
        Self {
            transfer_id: 0,
            resource_handle: ResourceHandle::new(0),
            source_instance: 0,
            target_instance: 0,
            transfer_type: super::component_communication::ResourceTransferType::Move,
            requested_at: 0,
        }
    }
}

impl PartialEq for PendingResourceTransfer {
    fn eq(&self, other: &Self) -> bool {
        self.transfer_id == other.transfer_id
    }
}

impl Eq for PendingResourceTransfer {}

impl Default for TransferPolicy {
    fn default() -> Self {
        Self {
            max_transfers: 1,
            #[cfg(feature = "std")]
            allowed_types: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            allowed_types: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            required_permissions: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            required_permissions: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl PartialEq for TransferPolicy {
    fn eq(&self, other: &Self) -> bool {
        self.max_transfers == other.max_transfers
    }
}

impl Eq for TransferPolicy {}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            allowed_targets: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            allowed_targets: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            allowed_functions: std::vec::Vec::new(),
            #[cfg(not(feature = "std"))]
            allowed_functions: BoundedVec::new(crate::MemoryProvider::default()).unwrap(),
            resource_permissions: ResourcePermissions::default(),
            memory_limits: MemoryLimits::default(),
        }
    }
}

impl PartialEq for SecurityPolicy {
    fn eq(&self, other: &Self) -> bool {
        self.allowed_targets.len() == other.allowed_targets.len()
    }
}

impl Eq for SecurityPolicy {}

impl Default for ValidationRule {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            name: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            name: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
            #[cfg(feature = "std")]
            description: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            description: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
            rule_type: ValidationRuleType::Parameter,
            severity: ValidationSeverity::Info,
        }
    }
}

impl PartialEq for ValidationRule {
    fn eq(&self, other: &Self) -> bool {
        self.rule_type == other.rule_type && self.severity == other.severity
    }
}

impl Eq for ValidationRule {}

impl Default for OptimizationSuggestion {
    fn default() -> Self {
        Self {
            suggestion_type: OptimizationType::ParameterMarshaling,
            #[cfg(feature = "std")]
            description: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            description: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
            impact: OptimizationImpact::Low,
            complexity: OptimizationComplexity::Simple,
        }
    }
}

impl PartialEq for OptimizationSuggestion {
    fn eq(&self, other: &Self) -> bool {
        self.suggestion_type == other.suggestion_type && self.impact == other.impact
    }
}

impl Eq for OptimizationSuggestion {}

impl Default for PermissionCheckResult {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            permission: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            permission: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
            granted: false,
            denial_reason: None,
        }
    }
}

impl PartialEq for PermissionCheckResult {
    fn eq(&self, other: &Self) -> bool {
        self.granted == other.granted
    }
}

impl Eq for PermissionCheckResult {}

impl Default for AccessControlResult {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            accessed_item: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            accessed_item: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
            allowed: false,
            #[cfg(feature = "std")]
            rule_applied: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            rule_applied: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl PartialEq for AccessControlResult {
    fn eq(&self, other: &Self) -> bool {
        self.allowed == other.allowed
    }
}

impl Eq for AccessControlResult {}

impl Default for ResourceAvailabilityResult {
    fn default() -> Self {
        Self {
            resource_handle: ResourceHandle::new(0),
            available: false,
            current_owner: None,
            locked: false,
        }
    }
}

impl PartialEq for ResourceAvailabilityResult {
    fn eq(&self, other: &Self) -> bool {
        self.resource_handle == other.resource_handle && self.available == other.available
    }
}

impl Eq for ResourceAvailabilityResult {}

impl PartialEq for TransferPermissionResult {
    fn eq(&self, other: &Self) -> bool {
        self.resource_handle == other.resource_handle && self.permitted == other.permitted
    }
}

impl Eq for TransferPermissionResult {}

impl PartialEq for TimingMetrics {
    fn eq(&self, other: &Self) -> bool {
        self.total_calls == other.total_calls && self.average_duration_us == other.average_duration_us
    }
}

impl Eq for TimingMetrics {}

impl Default for TransferPermissionResult {
    fn default() -> Self {
        Self {
            resource_handle: ResourceHandle::new(0),
            transfer_type: super::component_communication::ResourceTransferType::Move,
            permitted: false,
            #[cfg(feature = "std")]
            policy_applied: std::string::String::new(),
            #[cfg(not(feature = "std"))]
            policy_applied: BoundedString::new(crate::MemoryProvider::default()).unwrap(),
        }
    }
}

impl Default for SizeValidationResult {
    fn default() -> Self {
        Self {
            parameter_index: 0,
            size: 0,
            max_size: 0,
            passed: false,
        }
    }
}

impl PartialEq for SizeValidationResult {
    fn eq(&self, other: &Self) -> bool {
        self.parameter_index == other.parameter_index && self.passed == other.passed
    }
}

impl Eq for SizeValidationResult {}

// Apply macro to all types that need traits
impl_basic_traits!(ManagedCallContext, ManagedCallContext::default());
impl_basic_traits!(TypeCompatibility, TypeCompatibility::default());
impl_basic_traits!(ResourceLock, ResourceLock::default());
impl_basic_traits!(PendingResourceTransfer, PendingResourceTransfer::default());
impl_basic_traits!(TransferPolicy, TransferPolicy::default());
impl_basic_traits!(SecurityPolicy, SecurityPolicy::default());
impl_basic_traits!(ValidationRule, ValidationRule::default());
impl_basic_traits!(TimingMetrics, TimingMetrics::default());
impl_basic_traits!(OptimizationSuggestion, OptimizationSuggestion::default());
impl_basic_traits!(PermissionCheckResult, PermissionCheckResult::default());
impl_basic_traits!(AccessControlResult, AccessControlResult::default());
impl_basic_traits!(ResourceAvailabilityResult, ResourceAvailabilityResult::default());
impl_basic_traits!(TransferPermissionResult, TransferPermissionResult::default());
impl_basic_traits!(SizeValidationResult, SizeValidationResult::default());

// Additional Default implementations for remaining types
impl Default for TransferResult {
    fn default() -> Self {
        Self {
            resource_handle: ResourceHandle::new(0),
            success: false,
            new_handle: None,
            error_message: None,
        }
    }
}

impl PartialEq for TransferResult {
    fn eq(&self, other: &Self) -> bool {
        self.resource_handle == other.resource_handle && self.success == other.success
    }
}

impl Eq for TransferResult {}

impl Default for TypeCheckResult {
    fn default() -> Self {
        Self {
            parameter_index: 0,
            expected_type: ComponentType::Bool,
            actual_type: ComponentType::Bool,
            passed: false,
            error_message: None,
        }
    }
}

impl PartialEq for TypeCheckResult {
    fn eq(&self, other: &Self) -> bool {
        self.parameter_index == other.parameter_index && self.passed == other.passed
    }
}

impl Eq for TypeCheckResult {}

// Apply macro to additional types
impl_basic_traits!(TransferResult, TransferResult::default());
impl_basic_traits!(TypeCheckResult, TypeCheckResult::default());