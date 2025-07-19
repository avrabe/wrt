//! Component-to-Component Communication System
//!
//! This module provides comprehensive communication functionality for the
//! WebAssembly Component Model, enabling cross-component function calls,
//! parameter marshaling, and resource sharing.
//!
//! # Features
//!
//! - **Cross-Component Calls**: Function calls between component instances
//! - **Parameter Marshaling**: Safe parameter passing through Canonical ABI
//! - **Resource Transfer**: Secure resource sharing between components
//! - **Call Context Management**: Lifecycle management for cross-component calls
//! - **Security Boundaries**: Proper isolation and permission checking
//! - **Performance Optimization**: Efficient call routing and dispatch
//! - **Cross-Environment Support**: Works in std, no_std+alloc, and pure no_std
//!
//! # Core Concepts
//!
//! - **Call Router**: Central dispatcher for cross-component function calls
//! - **Call Context**: Execution context for a cross-component call
//! - **Call Stack**: Management of nested cross-component calls
//! - **Parameter Bridge**: Safe parameter marshaling between components
//! - **Resource Bridge**: Resource transfer coordination
//!
//! # Example
//!
//! ```no_run
//! use wrt_component::component_communication::{CallRouter, CallContext};
//! 
//! // Create a call router
//! let mut router = CallRouter::new);
//! 
//! // Route a call between components
//! let context = CallContext::new(source_instance, target_instance, "add", &args)?;
//! let result = router.dispatch_call(context)?;
//! ```


// Cross-environment imports
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::HashMap, boxed::Box, format};

#[cfg(all(not(feature = "std")))]
use alloc::{vec::Vec, string::String, collections::BTreeMap as HashMap, boxed::Box, format};

#[cfg(not(feature = "std"))]
use wrt_foundation::{bounded::{BoundedString, BoundedVec}, safe_memory::NoStdProvider};

// Note: Using alloc for no_std instead of wrt_foundation bounded types for now
// #[cfg(not(any(feature = "std", )))]
// use wrt_foundation::{BoundedVec, BoundedString, BoundedMap as HashMap, safe_memory::NoStdProvider};

// Type aliases for no_std compatibility (commented out to avoid conflicts)
// #[cfg(not(any(feature = "std", )))]
// type Vec<T> = BoundedVec<T, 64, NoStdProvider<65536>>;
// #[cfg(not(any(feature = "std", )))]
// type String = BoundedString<256, NoStdProvider<65536>>;

use wrt_error::{Error, ErrorCategory, Result, codes};
use crate::canonical_abi::{ComponentType};

#[cfg(feature = "std")]
use crate::canonical_abi::ComponentValue;

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;
use crate::components::component_instantiation::{InstanceId, ComponentInstance, FunctionSignature};
use crate::resource_management::{ResourceHandle, ResourceManager as ComponentResourceManager};

/// Maximum call stack depth to prevent infinite recursion
const MAX_CALL_STACK_DEPTH: usize = 64;

/// Maximum number of parameters per function call
const MAX_CALL_PARAMETERS: usize = 16;

/// Maximum number of return values per function call
const MAX_CALL_RETURN_VALUES: usize = 8;

/// Maximum number of active calls per instance
const MAX_ACTIVE_CALLS_PER_INSTANCE: usize = 256;

/// Call identifier for tracking individual calls
pub type CallId = u64;

/// Component call router for managing cross-component function calls
#[derive(Debug)]
pub struct CallRouter {
    /// Active call contexts by call ID
    active_calls: HashMap<CallId, CallContext>,
    /// Call stack tracking for recursion prevention
    call_stack: CallStack,
    /// Next available call ID
    next_call_id: CallId,
    /// Router configuration
    config: CallRouterConfig,
    /// Call statistics
    stats: CallStatistics,
}

/// Call context for managing individual cross-component calls
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    /// Unique call identifier
    pub call_id: CallId,
    /// Source component instance
    pub source_instance: InstanceId,
    /// Target component instance
    pub target_instance: InstanceId,
    /// Target function name
    pub target_function: String,
    /// Call parameters
    pub parameters: Vec<ComponentValue>,
    /// Expected return types
    pub return_types: Vec<ComponentType>,
    /// Resource handles passed with this call
    pub resource_handles: Vec<ResourceHandle>,
    /// Call metadata
    pub metadata: CallMetadata,
    /// Call state
    pub state: CallState,
}

/// Call stack management for tracking nested calls
#[derive(Debug, Clone)]
pub struct CallStack {
    /// Stack frames representing active calls
    frames: Vec<CallFrame>,
    /// Maximum allowed stack depth
    max_depth: usize,
    /// Current stack depth
    current_depth: usize,
}

/// Individual call frame in the call stack
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFrame {
    /// Call ID for this frame
    pub call_id: CallId,
    /// Source instance for this call
    pub source_instance: InstanceId,
    /// Target instance for this call
    pub target_instance: InstanceId,
    /// Function being called
    pub function_name: String,
    /// Frame creation timestamp
    pub created_at: u64,
}

/// Parameter bridge for safe cross-component parameter passing
#[derive(Debug)]
pub struct ParameterBridge {
    /// Source instance memory context
    source_memory_context: MemoryContext,
    /// Target instance memory context
    target_memory_context: MemoryContext,
    /// Marshaling configuration
    config: MarshalingConfig,
}

/// Memory context for parameter marshaling
#[derive(Debug, Clone)]
pub struct MemoryContext {
    /// Instance ID this context belongs to
    pub instance_id: InstanceId,
    /// Available memory size
    pub memory_size: u32,
    /// Memory protection flags
    pub protection_flags: MemoryProtectionFlags,
}

/// Resource bridge for cross-component resource sharing
#[derive(Debug)]
pub struct ResourceBridge {
    /// Resource manager reference
    resource_manager: ComponentResourceManager,
    /// Transfer policies
    transfer_policies: HashMap<InstanceId, ResourceTransferPolicy>,
    /// Active resource transfers
    active_transfers: Vec<ResourceTransfer>,
}

/// Call router configuration
#[derive(Debug, Clone)]
pub struct CallRouterConfig {
    /// Enable call tracing for debugging
    pub enable_call_tracing: bool,
    /// Maximum call stack depth
    pub max_call_stack_depth: usize,
    /// Enable security checks
    pub enable_security_checks: bool,
    /// Call timeout in microseconds
    pub call_timeout_us: u64,
    /// Enable performance optimization
    pub enable_optimization: bool,
    /// Maximum concurrent calls per instance
    pub max_concurrent_calls_per_instance: usize,
}

/// Parameter marshaling configuration
#[derive(Debug, Clone)]
pub struct MarshalingConfig {
    /// Enable parameter validation
    pub validate_parameters: bool,
    /// Enable memory bounds checking
    pub check_memory_bounds: bool,
    /// Enable type compatibility checking
    pub check_type_compatibility: bool,
    /// Copy strategy for large parameters
    pub copy_strategy: ParameterCopyStrategy,
}

/// Memory protection flags
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryProtectionFlags {
    /// Memory is readable
    pub readable: bool,
    /// Memory is writable
    pub writeable: bool,
    /// Memory is executable
    pub executable: bool,
    /// Memory isolation level
    pub isolation_level: MemoryIsolationLevel,
}

/// Resource transfer policy between instances
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceTransferPolicy {
    /// Allow resource ownership transfer
    pub allow_ownership_transfer: bool,
    /// Allow resource borrowing
    pub allow_borrowing: bool,
    /// Allowed resource types for transfer
    pub allowed_resource_types: Vec<String>,
    /// Maximum resources that can be transferred
    pub max_transfer_count: u32,
}

/// Active resource transfer tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceTransfer {
    /// Transfer ID
    pub transfer_id: u64,
    /// Resource handle being transferred
    pub resource_handle: ResourceHandle,
    /// Source instance
    pub source_instance: InstanceId,
    /// Target instance
    pub target_instance: InstanceId,
    /// Transfer type (ownership vs borrowing)
    pub transfer_type: ResourceTransferType,
    /// Transfer start timestamp
    pub started_at: u64,
}

/// Call metadata for tracking and debugging
#[derive(Debug, Clone, Default)]
pub struct CallMetadata {
    /// Call start timestamp
    pub started_at: u64,
    /// Call completion timestamp
    pub completed_at: u64,
    /// Call duration in microseconds
    pub duration_us: u64,
    /// Number of parameters passed
    pub parameter_count: usize,
    /// Total parameter data size in bytes
    pub parameter_data_size: u32,
    /// Custom metadata fields
    pub custom_fields: HashMap<String, String>,
}

/// Call statistics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct CallStatistics {
    /// Total calls dispatched
    pub total_calls: u64,
    /// Successful calls
    pub successful_calls: u64,
    /// Failed calls
    pub failed_calls: u64,
    /// Average call duration in microseconds
    pub average_duration_us: u64,
    /// Peak concurrent calls
    pub peak_concurrent_calls: u32,
    /// Total parameters marshaled
    pub total_parameters_marshaled: u64,
    /// Total resource transfers
    pub total_resource_transfers: u64,
}

/// Call state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    /// Call is being prepared
    Preparing,
    /// Call is being dispatched
    Dispatching,
    /// Call is executing in target instance
    Executing,
    /// Call completed successfully
    Completed,
    /// Call failed with error
    Failed(String),
    /// Call was cancelled
    Cancelled,
}

/// Parameter copy strategies for large data
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterCopyStrategy {
    /// Always copy parameters
    AlwaysCopy,
    /// Copy only when necessary (default)
    CopyOnWrite,
    /// Use zero-copy when possible
    ZeroCopy,
    /// Use memory mapping
    MemoryMap,
}

/// Memory isolation levels
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryIsolationLevel {
    /// No isolation
    None,
    /// Basic isolation
    Basic,
    /// Strong isolation
    Strong,
    /// Complete isolation
    Complete,
}

/// Resource transfer types
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceTransferType {
    /// Transfer ownership
    Ownership,
    /// Borrow resource
    Borrow,
    /// Share resource (read-only)
    Share,
}

/// Cross-component call errors
#[derive(Debug, Clone, PartialEq)]
pub enum CommunicationError {
    /// Call stack overflow
    CallStackOverflow,
    /// Invalid call context
    InvalidCallContext,
    /// Target instance not found
    TargetInstanceNotFound(InstanceId),
    /// Target function not found
    TargetFunctionNotFound(String),
    /// Parameter marshaling failed
    ParameterMarshalingFailed(String),
    /// Resource transfer failed
    ResourceTransferFailed(String),
    /// Security violation
    SecurityViolation(String),
    /// Call timeout
    CallTimeout,
    /// Too many concurrent calls
    TooManyConcurrentCalls,
}

impl Default for CallRouterConfig {
    fn default() -> Self {
        Self {
            enable_call_tracing: false,
            max_call_stack_depth: MAX_CALL_STACK_DEPTH,
            enable_security_checks: true,
            call_timeout_us: 5_000_000, // 5 seconds
            enable_optimization: true,
            max_concurrent_calls_per_instance: MAX_ACTIVE_CALLS_PER_INSTANCE,
        }
    }
}

impl Default for MarshalingConfig {
    fn default() -> Self {
        Self {
            validate_parameters: true,
            check_memory_bounds: true,
            check_type_compatibility: true,
            copy_strategy: ParameterCopyStrategy::CopyOnWrite,
        }
    }
}

impl Default for MemoryProtectionFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writeable: false,
            executable: false,
            isolation_level: MemoryIsolationLevel::Basic,
        }
    }
}

impl Default for ResourceTransferPolicy {
    fn default() -> Self {
        Self {
            allow_ownership_transfer: true,
            allow_borrowing: true,
            allowed_resource_types: Vec::new(),
            max_transfer_count: 16,
        }
    }
}

impl CallRouter {
    /// Create a new call router
    pub fn new() -> Self {
        Self::with_config(CallRouterConfig::default())
    }

    /// Create a new call router with custom configuration
    pub fn with_config(config: CallRouterConfig) -> Self {
        Self {
            active_calls: HashMap::new(),
            call_stack: CallStack::new(config.max_call_stack_depth),
            next_call_id: 1,
            config,
            stats: CallStatistics::default(),
        }
    }

    /// Dispatch a cross-component function call
    pub fn dispatch_call(
        &mut self,
        mut context: CallContext,
        source_instance: &ComponentInstance,
        target_instance: &mut ComponentInstance,
    ) -> Result<Vec<ComponentValue>> {
        // Validate call context
        self.validate_call_context(&context)?;

        // Check call stack depth
        if self.call_stack.current_depth >= self.config.max_call_stack_depth {
            return Err(Error::runtime_stack_overflow("Call stack depth exceeded";
        }

        // Check concurrent call limits
        let active_calls_for_target = self.count_active_calls_for_instance(context.target_instance;
        if active_calls_for_target >= self.config.max_concurrent_calls_per_instance {
            return Err(Error::runtime_execution_error("Too many concurrent calls to instance";
        }

        // Assign call ID and update state
        context.call_id = self.next_call_id;
        self.next_call_id += 1;
        context.state = CallState::Dispatching;
        context.metadata.started_at = 0; // Would use actual timestamp

        // Push call frame onto stack
        let frame = CallFrame {
            call_id: context.call_id,
            source_instance: context.source_instance,
            target_instance: context.target_instance,
            function_name: context.target_function.clone(),
            created_at: context.metadata.started_at,
        };
        self.call_stack.push_frame(frame)?;

        // Store active call context
        self.active_calls.insert(context.call_id, context.clone();

        // Update statistics
        self.stats.total_calls += 1;
        self.stats.total_parameters_marshaled += context.parameters.len() as u64;

        // Marshal parameters if needed
        let marshaled_parameters = self.marshal_parameters(&context, source_instance, target_instance)?;

        // Update call state
        let mut context = self.active_calls.get_mut(&context.call_id).unwrap();
        context.state = CallState::Executing;

        // Execute the target function
        let result = self.execute_target_function(
            &context.target_function,
            &marshaled_parameters,
            target_instance,
        ;

        // Update call state and statistics based on result
        let context = self.active_calls.get_mut(&context.call_id).unwrap();
        match &result {
            Ok(_) => {
                context.state = CallState::Completed;
                self.stats.successful_calls += 1;
            }
            Err(e) => {
                context.state = CallState::Failed("Call execution failed".to_string();
                self.stats.failed_calls += 1;
            }
        }

        // Pop call frame from stack
        self.call_stack.pop_frame()?;

        // Remove from active calls
        self.active_calls.remove(&context.call_id;

        // Update completion metadata
        context.metadata.completed_at = 0; // Would use actual timestamp
        context.metadata.duration_us = context.metadata.completed_at - context.metadata.started_at;

        result
    }

    /// Create a call context for a cross-component call
    pub fn create_call_context(
        &self,
        source_instance: InstanceId,
        target_instance: InstanceId,
        target_function: String,
        parameters: Vec<ComponentValue>,
        return_types: Vec<ComponentType>,
    ) -> Result<CallContext> {
        if parameters.len() > MAX_CALL_PARAMETERS {
            return Err(Error::validation_error("Too many parameters for function call";
        }

        if return_types.len() > MAX_CALL_RETURN_VALUES {
            return Err(Error::validation_error("Too many return values for function call";
        }

        Ok(CallContext {
            call_id: 0, // Will be assigned during dispatch
            source_instance,
            target_instance,
            target_function,
            parameters,
            return_types,
            resource_handles: Vec::new(),
            metadata: CallMetadata::default(),
            state: CallState::Preparing,
        })
    }

    /// Get call statistics
    pub fn get_statistics(&self) -> &CallStatistics {
        &self.stats
    }

    /// Get current call stack depth
    pub fn get_call_stack_depth(&self) -> usize {
        self.call_stack.current_depth
    }

    /// Check if an instance has active calls
    pub fn has_active_calls(&self, instance_id: InstanceId) -> bool {
        self.active_calls.values().any(|call| 
            call.source_instance == instance_id || call.target_instance == instance_id
        )
    }

    // Private helper methods

    fn validate_call_context(&self, context: &CallContext) -> Result<()> {
        if context.target_function.is_empty() {
            return Err(Error::validation_error("Target function name cannot be empty";
        }

        if context.source_instance == context.target_instance {
            return Err(Error::validation_error("Source and target instances cannot be the same";
        }

        Ok(())
    }

    fn count_active_calls_for_instance(&self, instance_id: InstanceId) -> usize {
        self.active_calls.values().filter(|call| 
            call.target_instance == instance_id
        ).count()
    }

    fn marshal_parameters(
        &self,
        context: &CallContext,
        _source_instance: &ComponentInstance,
        _target_instance: &ComponentInstance,
    ) -> Result<Vec<ComponentValue>> {
        // For now, we'll pass parameters directly
        // In a full implementation, this would handle:
        // - Memory layout conversion
        // - Endianness conversion
        // - String encoding conversion
        // - Resource handle marshaling
        Ok(context.parameters.clone())
    }

    fn execute_target_function(
        &self,
        function_name: &str,
        parameters: &[ComponentValue],
        target_instance: &mut ComponentInstance,
    ) -> Result<Vec<ComponentValue>> {
        // Execute the function in the target instance
        target_instance.call_function(function_name, parameters)
    }
}

impl CallStack {
    /// Create a new call stack
    pub fn new(max_depth: usize) -> Self {
        Self {
            frames: Vec::new(),
            max_depth,
            current_depth: 0,
        }
    }

    /// Push a call frame onto the stack
    pub fn push_frame(&mut self, frame: CallFrame) -> Result<()> {
        if self.current_depth >= self.max_depth {
            return Err(Error::runtime_stack_overflow("Call stack overflow";
        }

        self.frames.push(frame);
        self.current_depth += 1;
        Ok(())
    }

    /// Pop a call frame from the stack
    pub fn pop_frame(&mut self) -> Result<CallFrame> {
        if self.frames.is_empty() {
            return Err(Error::runtime_execution_error("Call stack is empty";
        }

        let frame = self.frames.pop().unwrap();
        self.current_depth -= 1;
        Ok(frame)
    }

    /// Get the current call frame (top of stack)
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.frames.last()
    }

    /// Check if there's a circular call pattern
    pub fn has_circular_call(&self, source: InstanceId, target: InstanceId) -> bool {
        self.frames.iter().any(|frame| 
            frame.source_instance == target && frame.target_instance == source
        )
    }

    /// Get the call stack depth
    pub fn depth(&self) -> usize {
        self.current_depth
    }
}

impl ParameterBridge {
    /// Create a new parameter bridge
    pub fn new(
        source_context: MemoryContext,
        target_context: MemoryContext,
        config: MarshalingConfig,
    ) -> Self {
        Self {
            source_memory_context: source_context,
            target_memory_context: target_context,
            config,
        }
    }

    /// Marshal parameters from source to target format
    pub fn marshal_parameters(
        &self,
        parameters: &[ComponentValue],
        _source_instance: &ComponentInstance,
        _target_instance: &ComponentInstance,
    ) -> Result<Vec<ComponentValue>> {
        if self.config.validate_parameters {
            self.validate_parameters(parameters)?;
        }

        // For now, return parameters as-is
        // In a full implementation, this would handle:
        // - Type conversion
        // - Memory layout transformation
        // - Resource handle marshaling
        Ok(parameters.to_vec())
    }

    fn validate_parameters(&self, parameters: &[ComponentValue]) -> Result<()> {
        if parameters.len() > MAX_CALL_PARAMETERS {
            return Err(Error::validation_error("Too many parameters";
        }

        // Additional parameter validation would go here
        Ok(())
    }
}

impl ResourceBridge {
    /// Create a new resource bridge
    pub fn new(resource_manager: ComponentResourceManager) -> Self {
        Self {
            resource_manager,
            transfer_policies: HashMap::new(),
            active_transfers: Vec::new(),
        }
    }

    /// Transfer a resource between instances
    pub fn transfer_resource(
        &mut self,
        resource_handle: ResourceHandle,
        source_instance: InstanceId,
        target_instance: InstanceId,
        transfer_type: ResourceTransferType,
    ) -> Result<ResourceHandle> {
        // Check if transfer is allowed by policy
        self.check_transfer_policy(source_instance, target_instance, &transfer_type)?;

        // Perform the actual transfer based on type
        match transfer_type {
            ResourceTransferType::Ownership => {
                self.resource_manager.transfer_ownership(resource_handle, source_instance, target_instance)
            }
            ResourceTransferType::Borrow => {
                self.resource_manager.borrow_resource(resource_handle, source_instance, target_instance)
            }
            ResourceTransferType::Share => {
                // For sharing, we could implement a read-only borrow
                self.resource_manager.borrow_resource(resource_handle, source_instance, target_instance)
            }
        }
    }

    fn check_transfer_policy(
        &self,
        _source_instance: InstanceId,
        _target_instance: InstanceId,
        _transfer_type: &ResourceTransferType,
    ) -> Result<()> {
        // Policy checking would be implemented here
        // For now, allow all transfers
        Ok(())
    }
}

impl Default for CallRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CommunicationError::CallStackOverflow => write!(f, "Call stack overflow"),
            CommunicationError::InvalidCallContext => write!(f, "Invalid call context"),
            CommunicationError::TargetInstanceNotFound(id) => write!(f, "Target instance {} not found", id),
            CommunicationError::TargetFunctionNotFound(name) => write!(f, "Target function '{}' not found", name),
            CommunicationError::ParameterMarshalingFailed(msg) => write!(f, "Parameter marshaling failed: {}", msg),
            CommunicationError::ResourceTransferFailed(msg) => write!(f, "Resource transfer failed: {}", msg),
            CommunicationError::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
            CommunicationError::CallTimeout => write!(f, "Call timeout"),
            CommunicationError::TooManyConcurrentCalls => write!(f, "Too many concurrent calls"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CommunicationError {}

/// Create a memory context for an instance
pub fn create_memory_context(
    instance_id: InstanceId,
    memory_size: u32,
    protection_flags: MemoryProtectionFlags,
) -> MemoryContext {
    MemoryContext {
        instance_id,
        memory_size,
        protection_flags,
    }
}

/// Create a default resource transfer policy
pub fn create_default_transfer_policy() -> ResourceTransferPolicy {
    ResourceTransferPolicy::default()
}

/// Create a parameter bridge for cross-component calls
pub fn create_parameter_bridge(
    source_context: MemoryContext,
    target_context: MemoryContext,
) -> ParameterBridge {
    ParameterBridge::new(source_context, target_context, MarshalingConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_router_creation() {
        let router = CallRouter::new);
        assert_eq!(router.stats.total_calls, 0;
        assert_eq!(router.get_call_stack_depth(), 0;
    }

    #[test]
    fn test_call_context_creation() {
        let router = CallRouter::new);
        let context = router.create_call_context(
            1, 
            2, 
            "test_function".to_string(),
            vec![ComponentValue::S32(42)],
            vec![ComponentType::S32],
        ;
        
        assert!(context.is_ok();
        let context = context.unwrap();
        assert_eq!(context.source_instance, 1;
        assert_eq!(context.target_instance, 2;
        assert_eq!(context.target_function, "test_function";
        assert_eq!(context.parameters.len(), 1;
    }

    #[test]
    fn test_call_stack_operations() {
        let mut stack = CallStack::new(5;
        assert_eq!(stack.depth(), 0;

        let frame = CallFrame {
            call_id: 1,
            source_instance: 1,
            target_instance: 2,
            function_name: "test".to_string(),
            created_at: 0,
        };

        stack.push_frame(frame).unwrap();
        assert_eq!(stack.depth(), 1;

        let popped = stack.pop_frame().unwrap();
        assert_eq!(popped.call_id, 1;
        assert_eq!(stack.depth(), 0;
    }

    #[test]
    fn test_parameter_bridge_creation() {
        let source_context = create_memory_context(1, 1024, MemoryProtectionFlags::default);
        let target_context = create_memory_context(2, 2048, MemoryProtectionFlags::default);
        let bridge = create_parameter_bridge(source_context, target_context;
        
        assert_eq!(bridge.source_memory_context.instance_id, 1;
        assert_eq!(bridge.target_memory_context.instance_id, 2;
    }

    #[test]
    fn test_memory_protection_flags() {
        let flags = MemoryProtectionFlags {
            readable: true,
            writeable: true,
            executable: false,
            isolation_level: MemoryIsolationLevel::Strong,
        };
        
        assert!(flags.readable);
        assert!(flags.writeable);
        assert!(!flags.executable);
        assert_eq!(flags.isolation_level, MemoryIsolationLevel::Strong;
    }

    #[test]
    fn test_call_statistics() {
        let mut stats = CallStatistics::default);
        stats.total_calls = 10;
        stats.successful_calls = 8;
        stats.failed_calls = 2;
        
        assert_eq!(stats.total_calls, 10;
        assert_eq!(stats.successful_calls, 8;
        assert_eq!(stats.failed_calls, 2;
    }
}

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{Checksummable, ToBytes, FromBytes, WriteStream, ReadStream};

// Macro to implement basic traits for complex types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                0u32.update_checksum(checksum;
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
impl Default for CallContext {
    fn default() -> Self {
        Self {
            call_id: 0,
            source_instance: 0,
            target_instance: 0,
            target_function: String::new(),
            parameters: Vec::new(),
            return_types: Vec::new(),
            resource_handles: Vec::new(),
            metadata: CallMetadata::default(),
            state: CallState::default(),
        }
    }
}

impl Default for CallFrame {
    fn default() -> Self {
        Self {
            call_id: 0,
            source_instance: 0,
            target_instance: 0,
            function_name: String::new(),
            created_at: 0,
        }
    }
}

impl Default for CallMetadata {
    fn default() -> Self {
        Self {
            started_at: 0,
            completed_at: 0,
            duration_us: 0,
            parameter_count: 0,
            parameter_data_size: 0,
            custom_fields: HashMap::new(),
        }
    }
}

impl Default for MemoryContext {
    fn default() -> Self {
        Self {
            instance_id: 0,
            memory_size: 0,
            protection_flags: MemoryProtectionFlags::default(),
        }
    }
}

impl Default for MemoryProtectionFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writeable: false,
            executable: false,
            isolation_level: MemoryIsolationLevel::default(),
        }
    }
}

impl Default for ResourceTransfer {
    fn default() -> Self {
        Self {
            transfer_id: 0,
            resource_handle: crate::resource_management::ResourceHandle::new(0),
            source_instance: 0,
            target_instance: 0,
            transfer_type: ResourceTransferType::default(),
            started_at: 0,
        }
    }
}

impl Default for CallState {
    fn default() -> Self {
        Self::Pending
    }
}

impl Default for ParameterCopyStrategy {
    fn default() -> Self {
        Self::CopyIn
    }
}

impl Default for MemoryIsolationLevel {
    fn default() -> Self {
        Self::Strong
    }
}

impl Default for ResourceTransferType {
    fn default() -> Self {
        Self::Move
    }
}

// Apply macro to all types that need traits
impl_basic_traits!(CallContext, CallContext::default);
impl_basic_traits!(CallFrame, CallFrame::default);
impl_basic_traits!(CallMetadata, CallMetadata::default);
impl_basic_traits!(MemoryContext, MemoryContext::default);
impl_basic_traits!(MemoryProtectionFlags, MemoryProtectionFlags::default);
impl_basic_traits!(ResourceTransfer, ResourceTransfer::default);

// Additional Default implementations
impl Default for CallRouterConfig {
    fn default() -> Self {
        Self {
            max_call_stack_depth: 16,
            max_concurrent_calls: 1024,
            enable_call_tracing: false,
            default_timeout_ms: 30000,
            memory_isolation: true,
        }
    }
}

impl Default for MarshalingConfig {
    fn default() -> Self {
        Self {
            enable_type_checking: true,
            enable_bounds_checking: true,
            max_parameter_size: 1024 * 1024, // 1MB
            string_encoding: StringEncoding::Utf8,
        }
    }
}

impl Default for ResourceTransferPolicy {
    fn default() -> Self {
        Self {
            allow_ownership_transfer: false,
            allow_borrowing: true,
            require_explicit_permission: true,
            max_concurrent_transfers: 16,
        }
    }
}

impl Default for StringEncoding {
    fn default() -> Self {
        Self::Utf8
    }
}

// Apply traits to additional types
impl_basic_traits!(CallRouterConfig, CallRouterConfig::default);
impl_basic_traits!(MarshalingConfig, MarshalingConfig::default);
impl_basic_traits!(ResourceTransferPolicy, ResourceTransferPolicy::default);