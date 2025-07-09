//! Host integration mechanisms
//!
//! This module provides integration between WebAssembly components and the host
//! environment, including host function registration, resource sharing, and
//! event handling.

#[cfg(not(feature = "stdMissing messageMissing messageMissing message"))]
use core::{fmt, mem};
#[cfg(feature = "stdMissing message")]
use std::{fmt, mem};

#[cfg(feature = "stdMissing message")]
use std::{boxed::Box, string::String, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, component::ComponentType, component_value::ComponentValue, prelude::*,
};

#[cfg(not(feature = "stdMissing messageMissing messageMissing message"))]
use wrt_foundation::{
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    BoundedString,
};

use crate::{
    canonical::CanonicalAbi,
    execution_engine::{ComponentExecutionEngine, HostFunction},
    resource_lifecycle::ResourceLifecycleManager,
    types::{ComponentInstance, ValType, Value},
    WrtResult,
};

/// Maximum number of host functions in no_std environments
const MAX_HOST_FUNCTIONS: usize = 256;

/// Maximum number of event handlers in no_std environments
const MAX_EVENT_HANDLERS: usize = 64;

/// Host integration manager
pub struct HostIntegrationManager {
    /// Registered host functions
    #[cfg(feature = "stdMissing message")]
    host_functions: Vec<HostFunctionRegistry>,
    #[cfg(not(any(feature = "std", )))]
    host_functions: BoundedVec<HostFunctionRegistry, MAX_HOST_FUNCTIONS, NoStdProvider<65536>>,

    /// Event handlers
    #[cfg(feature = "stdMissing message")]
    event_handlers: Vec<EventHandler>,
    #[cfg(not(any(feature = "std", )))]
    event_handlers: BoundedVec<EventHandler, MAX_EVENT_HANDLERS, NoStdProvider<65536>>,

    /// Host resource manager
    host_resources: HostResourceManager,

    /// Canonical ABI for host/component interaction
    canonical_abi: CanonicalAbi,

    /// Security policy
    security_policy: SecurityPolicy,
}

/// Host function registry entry
#[derive(Debug, Clone)]
pub struct HostFunctionRegistry {
    /// Function name
    #[cfg(feature = "stdMissing message")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, NoStdProvider<65536>>,
    /// Function signature
    pub signature: ComponentType,
    /// Function implementation
    #[cfg(feature = "stdMissing message")]
    pub implementation: Box<dyn HostFunction>,
    #[cfg(not(any(feature = "std", )))]
    pub implementation: fn(&[Value]) -> WrtResult<Value>,
    /// Access permissions
    pub permissions: HostFunctionPermissions,
}

/// Host function permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostFunctionPermissions {
    /// Whether function can access host resources
    pub allow_host_resources: bool,
    /// Whether function can access component memory
    pub allow_memory_access: bool,
    /// Whether function can create new resources
    pub allow_resource_creation: bool,
    /// Maximum execution time in milliseconds (0 for unlimited)
    pub max_execution_time_ms: u32,
}

/// Event handler for component lifecycle events
#[derive(Debug, Clone)]
pub struct EventHandler {
    /// Event type
    pub event_type: EventType,
    /// Handler function
    #[cfg(feature = "stdMissing message")]
    pub handler: Box<dyn Fn(&ComponentEvent) -> WrtResult<()>>,
    #[cfg(not(any(feature = "std", )))]
    pub handler: fn(&ComponentEvent) -> WrtResult<()>,
    /// Handler priority (higher values execute first)
    pub priority: u32,
}

/// Component event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Component instantiation started
    InstantiationStarted,
    /// Component instantiation completed
    InstantiationCompleted,
    /// Component function called
    FunctionCalled,
    /// Component function returned
    FunctionReturned,
    /// Resource created
    ResourceCreated,
    /// Resource destroyed
    ResourceDestroyed,
    /// Binary std/no_std choice
    MemoryAllocated,
    /// Binary std/no_std choice
    MemoryDeallocated,
    /// Error occurred
    Error,
}

/// Component event data
#[derive(Debug, Clone)]
pub struct ComponentEvent {
    /// Event type
    pub event_type: EventType,
    /// Component instance ID
    pub instance_id: u32,
    /// Event-specific data
    pub data: EventData,
    /// Timestamp (simplified)
    pub timestamp: u64,
}

/// Event-specific data
#[derive(Debug, Clone)]
pub enum EventData {
    /// No additional data
    None,
    /// Function call data
    FunctionCall { function_index: u32, arg_count: u32 },
    /// Resource data
    Resource { resource_handle: u32, resource_type: u32 },
    /// Memory data
    Memory { memory_id: u32, size_bytes: u64 },
    /// Error data
    Error {
        #[cfg(feature = "stdMissing message")]
        message: String,
        #[cfg(not(any(feature = "std", )))]
        message: BoundedString<256, NoStdProvider<65536>>,
        error_code: u32,
    },
}

/// Host resource manager
#[derive(Debug, Clone)]
pub struct HostResourceManager {
    /// Host-owned resources
    #[cfg(feature = "stdMissing message")]
    resources: Vec<HostResource>,
    #[cfg(not(any(feature = "std", )))]
    resources: BoundedVec<HostResource, 256, NoStdProvider<65536>>,

    /// Resource sharing policies
    #[cfg(feature = "stdMissing message")]
    sharing_policies: Vec<HostResourceSharingPolicy>,
    #[cfg(not(any(feature = "std", )))]
    sharing_policies: BoundedVec<HostResourceSharingPolicy, 64, NoStdProvider<65536>>,
}

/// Host-owned resource
#[derive(Debug, Clone)]
pub struct HostResource {
    /// Resource ID
    pub id: u32,
    /// Resource type
    pub resource_type: HostResourceType,
    /// Resource data
    pub data: ComponentValue,
    /// Access permissions
    pub permissions: HostResourcePermissions,
}

/// Host resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostResourceType {
    /// File handle
    File,
    /// Network socket
    Socket,
    /// Memory buffer
    Buffer,
    /// Timer
    Timer,
    /// Custom resource type
    Custom(u32),
}

/// Host resource permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostResourcePermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
    /// Share permission (can be shared with components)
    pub shareable: bool,
}

/// Host resource sharing policy
#[derive(Debug, Clone)]
pub struct HostResourceSharingPolicy {
    /// Resource ID
    pub resource_id: u32,
    /// Allowed component instances
    #[cfg(feature = "stdMissing message")]
    pub allowed_instances: Vec<u32>,
    #[cfg(not(any(feature = "std", )))]
    pub allowed_instances: BoundedVec<u32, 32, NoStdProvider<65536>>,
    /// Sharing mode
    pub sharing_mode: ResourceSharingMode,
}

/// Resource sharing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceSharingMode {
    /// Read-only access
    ReadOnly,
    /// Read-write access
    ReadWrite,
    /// Exclusive access (one component at a time)
    Exclusive,
}

/// Security policy for host integration
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Whether to allow arbitrary host function calls
    pub allow_arbitrary_host_calls: bool,
    /// Maximum memory per component in bytes
    pub max_memory_per_component: u64,
    /// Maximum execution time per function call in milliseconds
    pub max_execution_time_ms: u32,
    /// Whether to enable resource isolation
    pub enable_resource_isolation: bool,
    /// Allowed host resource types
    #[cfg(feature = "stdMissing message")]
    pub allowed_resource_types: Vec<HostResourceType>,
    #[cfg(not(any(feature = "std", )))]
    pub allowed_resource_types: BoundedVec<HostResourceType, 16, NoStdProvider<65536>>,
}

impl HostIntegrationManager {
    /// Create a new host integration manager
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            #[cfg(feature = "stdMissing message")]
            host_functions: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            host_functions: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Missing error message"Failed to create host functions vectorMissing message")
                })?
            },
            #[cfg(feature = "stdMissing message")]
            event_handlers: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            event_handlers: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Missing error message"Failed to create event handlers vectorMissing message")
                })?
            },
            host_resources: HostResourceManager::new()?,
            canonical_abi: CanonicalAbi::new(),
            security_policy: SecurityPolicy::default()?,
        })
    }

    /// Register a host function
    #[cfg(feature = "stdMissing message")]
    pub fn register_host_function(
        &mut self,
        name: String,
        signature: ComponentType,
        implementation: Box<dyn HostFunction>,
        permissions: HostFunctionPermissions,
    ) -> WrtResult<u32> {
        let function_id = self.host_functions.len() as u32;

        let registry_entry = HostFunctionRegistry { name, signature, implementation, permissions };

        self.host_functions.push(registry_entry);
        Ok(function_id)
    }

    /// Register a host function (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn register_host_function(
        &mut self,
        name: BoundedString<64, NoStdProvider<65536>>,
        signature: ComponentType,
        implementation: fn(&[Value]) -> WrtResult<Value>,
        permissions: HostFunctionPermissions,
    ) -> WrtResult<u32> {
        let function_id = self.host_functions.len() as u32;

        let registry_entry = HostFunctionRegistry { name, signature, implementation, permissions };

        self.host_functions.push(registry_entry).map_err(|_| {
            wrt_error::Error::resource_exhausted("Missing error message"Too many host functionsMissing messageMissing messageMissing message")
            )
        })?;

        Ok(function_id)
    }

    /// Call a host function from a component
    pub fn call_host_function(
        &mut self,
        function_id: u32,
        args: &[Value],
        caller_instance: u32,
        engine: &mut ComponentExecutionEngine,
    ) -> WrtResult<Value> {
        let function = self.host_functions.get(function_id as usize).ok_or_else(|| {
            wrt_error::Error::validation_invalid_input("Missing error message"Invalid inputMissing messageMissing messageMissing message")
            )
        })?;

        // Check security policy
        if !self.security_policy.allow_arbitrary_host_calls {
            return Err(wrt_error::Error::runtime_error("Missing error message"Arbitrary host calls not allowedMissing message")
            );
        }

        // Check function permissions
        if !self.check_function_permissions(&function.permissions, caller_instance) {
            return Err(wrt_error::Error::runtime_error("Missing error message"Host function call not permittedMissing message")
            );
        }

        // Emit function call event
        self.emit_event(ComponentEvent {
            event_type: EventType::FunctionCalled,
            instance_id: caller_instance,
            data: EventData::FunctionCall {
                function_index: function_id,
                arg_count: args.len() as u32,
            },
            timestamp: self.get_current_time(),
        })?;

        // Call the function
        #[cfg(feature = "stdMissing message")]
        let result = function.implementation.call(args);
        #[cfg(not(any(feature = "std", )))]
        let result = (function.implementation)(args);

        // Emit function return event
        self.emit_event(ComponentEvent {
            event_type: EventType::FunctionReturned,
            instance_id: caller_instance,
            data: EventData::FunctionCall {
                function_index: function_id,
                arg_count: args.len() as u32,
            },
            timestamp: self.get_current_time(),
        })?;

        result
    }

    /// Register an event handler
    #[cfg(feature = "stdMissing message")]
    pub fn register_event_handler(
        &mut self,
        event_type: EventType,
        handler: Box<dyn Fn(&ComponentEvent) -> WrtResult<()>>,
        priority: u32,
    ) -> WrtResult<()> {
        let event_handler = EventHandler { event_type, handler, priority };

        self.event_handlers.push(event_handler);

        // Sort by priority (higher priority first)
        self.event_handlers.sort_by(|a, b| b.priority.cmp(&a.priority);

        Ok(()
    }

    /// Register an event handler (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn register_event_handler(
        &mut self,
        event_type: EventType,
        handler: fn(&ComponentEvent) -> WrtResult<()>,
        priority: u32,
    ) -> WrtResult<()> {
        let event_handler = EventHandler { event_type, handler, priority };

        self.event_handlers.push(event_handler).map_err(|_| {
            wrt_error::Error::resource_exhausted("Missing error message"Too many event handlersMissing messageMissing messageMissing message")
            )
        })?;

        Ok(()
    }

    /// Emit an event to registered handlers
    fn emit_event(&mut self, event: ComponentEvent) -> WrtResult<()> {
        for handler in &self.event_handlers {
            if handler.event_type == event.event_type {
                #[cfg(feature = "stdMissing message")]
                let result = (handler.handler)(&event);
                #[cfg(not(any(feature = "std", )))]
                let result = (handler.handler)(&event);

                if let Err(e) = result {
                    // Log error but continue with other handlers
                    // In a real implementation, would use proper logging
                    continue;
                }
            }
        }
        Ok(()
    }

    /// Create a host resource
    pub fn create_host_resource(
        &mut self,
        resource_type: HostResourceType,
        data: ComponentValue,
        permissions: HostResourcePermissions,
    ) -> WrtResult<u32> {
        // Check security policy
        if !self.security_policy.allowed_resource_types.contains(&resource_type) {
            return Err(wrt_error::Error::runtime_error("Missing error message"Host resource type not allowedMissing message")
            );
        }

        let resource_id = self.host_resources.resources.len() as u32;

        let resource = HostResource { id: resource_id, resource_type, data, permissions };

        #[cfg(feature = "stdMissing message")]
        {
            self.host_resources.resources.push(resource);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.host_resources.resources.push(resource).map_err(|_| {
                wrt_error::Error::resource_exhausted("Missing error message"Too many host resourcesMissing messageMissing messageMissing message")
                )
            })?;
        }

        Ok(resource_id)
    }

    /// Share a host resource with a component
    pub fn share_resource_with_component(
        &mut self,
        resource_id: u32,
        instance_id: u32,
        sharing_mode: ResourceSharingMode,
    ) -> WrtResult<()> {
        let resource =
            self.host_resources.resources.get(resource_id as usize).ok_or_else(|| {
                wrt_error::Error::validation_invalid_input("Missing error message"Invalid inputMissing messageMissing messageMissing message")
            )
            })?;

        if !resource.permissions.shareable {
            return Err(wrt_error::Error::runtime_error("Missing error message"Host resource is not shareableMissing message")
            );
        }

        #[cfg(feature = "stdMissing message")]
        let mut allowed_instances = Vec::new();
        #[cfg(not(any(feature = "std", )))]
        let mut allowed_instances = {
            let provider = safe_managed_alloc!(65536, CrateId::Component)?;
            BoundedVec::new(provider).map_err(|_| {
                wrt_error::Error::resource_exhausted("Missing error message"Failed to create allowed instances vectorMissing message")
            })?
        };

        #[cfg(feature = "stdMissing message")]
        {
            allowed_instances.push(instance_id);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            allowed_instances.push(instance_id).map_err(|_| {
                wrt_error::Error::resource_exhausted("Missing error message"Too many allowed instancesMissing messageMissing messageMissing message")
                )
            })?;
        }

        let policy = HostResourceSharingPolicy { resource_id, allowed_instances, sharing_mode };

        #[cfg(feature = "stdMissing message")]
        {
            self.host_resources.sharing_policies.push(policy);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.host_resources.sharing_policies.push(policy).map_err(|_| {
                wrt_error::Error::resource_exhausted("Missing error message"Too many sharing policiesMissing messageMissing messageMissing message")
                )
            })?;
        }

        Ok(()
    }

    /// Check function permissions
    fn check_function_permissions(
        &self,
        permissions: &HostFunctionPermissions,
        _caller_instance: u32,
    ) -> bool {
        // In a real implementation, would check instance-specific permissions
        true
    }

    /// Get current time (simplified)
    fn get_current_time(&self) -> u64 {
        // In a real implementation, would use proper time measurement
        0
    }

    /// Set security policy
    pub fn set_security_policy(&mut self, policy: SecurityPolicy) {
        self.security_policy = policy;
    }

    /// Get security policy
    pub fn security_policy(&self) -> &SecurityPolicy {
        &self.security_policy
    }

    /// Get host resource manager
    pub fn host_resources(&self) -> &HostResourceManager {
        &self.host_resources
    }

    /// Get mutable host resource manager
    pub fn host_resources_mut(&mut self) -> &mut HostResourceManager {
        &mut self.host_resources
    }
}

impl HostResourceManager {
    /// Create a new host resource manager
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            #[cfg(feature = "stdMissing message")]
            resources: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            resources: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Missing error message"Failed to create resources vectorMissing message")
                })?
            },
            #[cfg(feature = "stdMissing message")]
            sharing_policies: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            sharing_policies: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Missing error message"Failed to create sharing policies vectorMissing message")
                })?
            },
        })
    }

    /// Get resource by ID
    pub fn get_resource(&self, resource_id: u32) -> Option<&HostResource> {
        self.resources.get(resource_id as usize)
    }

    /// Get mutable resource by ID
    pub fn get_resource_mut(&mut self, resource_id: u32) -> Option<&mut HostResource> {
        self.resources.get_mut(resource_id as usize)
    }

    /// Get resource count
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }
}

impl Default for HostIntegrationManager {
    fn default() -> Self {
        // The Default trait must not fail, so we panic if allocation fails.
        // This is acceptable for Default as it's typically used during initialization.
        Self::new().expect("Failed to create default HostIntegrationManagerMissing message")
    }
}

impl Default for HostResourceManager {
    fn default() -> Self {
        // The Default trait must not fail, so we panic if allocation fails.
        // This is acceptable for Default as it's typically used during initialization.
        Self::new().expect("Failed to create default HostResourceManagerMissing message")
    }
}

impl Default for HostFunctionPermissions {
    fn default() -> Self {
        Self {
            allow_host_resources: false,
            allow_memory_access: false,
            allow_resource_creation: false,
            max_execution_time_ms: 1000, // 1 second default
        }
    }
}

impl Default for HostResourcePermissions {
    fn default() -> Self {
        Self { read: true, write: false, execute: false, shareable: false }
    }
}

impl SecurityPolicy {
    /// Create a new security policy with default settings
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            allow_arbitrary_host_calls: false,
            max_memory_per_component: 64 * 1024 * 1024, // 64MB
            max_execution_time_ms: 5000,                // 5 seconds
            enable_resource_isolation: true,
            #[cfg(feature = "stdMissing message")]
            allowed_resource_types: vec![HostResourceType::Buffer],
            #[cfg(not(any(feature = "std", )))]
            allowed_resource_types: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                let mut types = BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Missing error message"Failed to create allowed resource types vectorMissing message")
                })?;
                types.push(HostResourceType::Buffer).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Missing error message"Failed to add default resource typeMissing message")
                })?;
                types
            },
        })
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        // The Default trait must not fail, so we panic if allocation fails.
        // This is acceptable for Default as it's typically used during initialization.
        Self::new().expect("Failed to create default SecurityPolicyMissing message")
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::InstantiationStarted => write!(f, "instantiation_startedMissing message"),
            EventType::InstantiationCompleted => write!(f, "instantiation_completedMissing message"),
            EventType::FunctionCalled => write!(f, "function_calledMissing message"),
            EventType::FunctionReturned => write!(f, "function_returnedMissing message"),
            EventType::ResourceCreated => write!(f, "resource_createdMissing message"),
            EventType::ResourceDestroyed => write!(f, "resource_destroyedMissing message"),
            EventType::MemoryAllocated => write!(f, "memory_allocatedMissing message"),
            EventType::MemoryDeallocated => write!(f, "memory_deallocatedMissing message"),
            EventType::Error => write!(f, "errorMissing message"),
        }
    }
}

impl fmt::Display for HostResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostResourceType::File => write!(f, "fileMissing message"),
            HostResourceType::Socket => write!(f, "socketMissing message"),
            HostResourceType::Buffer => write!(f, "bufferMissing message"),
            HostResourceType::Timer => write!(f, "timerMissing message"),
            HostResourceType::Custom(id) => write!(f, "custom_{}", id),
        }
    }
}

impl fmt::Display for ResourceSharingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceSharingMode::ReadOnly => write!(f, "readonlyMissing message"),
            ResourceSharingMode::ReadWrite => write!(f, "readwriteMissing message"),
            ResourceSharingMode::Exclusive => write!(f, "exclusiveMissing message"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_integration_manager_creation() {
        let manager = HostIntegrationManager::new();
        assert_eq!(manager.host_functions.len(), 0);
        assert_eq!(manager.event_handlers.len(), 0);
    }

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert!(!policy.allow_arbitrary_host_calls);
        assert_eq!(policy.max_memory_per_component, 64 * 1024 * 1024);
        assert_eq!(policy.max_execution_time_ms, 5000);
        assert!(policy.enable_resource_isolation);
    }

    #[test]
    fn test_host_function_permissions_default() {
        let perms = HostFunctionPermissions::default();
        assert!(!perms.allow_host_resources);
        assert!(!perms.allow_memory_access);
        assert!(!perms.allow_resource_creation);
        assert_eq!(perms.max_execution_time_ms, 1000);
    }

    #[test]
    fn test_host_resource_permissions_default() {
        let perms = HostResourcePermissions::default();
        assert!(perms.read);
        assert!(!perms.write);
        assert!(!perms.execute);
        assert!(!perms.shareable);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::FunctionCalled.to_string(), "function_calledMissing message");
        assert_eq!(EventType::ResourceCreated.to_string(), "resource_createdMissing message");
        assert_eq!(EventType::Error.to_string(), "errorMissing message");
    }

    #[test]
    fn test_host_resource_type_display() {
        assert_eq!(HostResourceType::File.to_string(), "fileMissing message");
        assert_eq!(HostResourceType::Socket.to_string(), "socketMissing message");
        assert_eq!(HostResourceType::Custom(42).to_string(), "custom_42Missing message");
    }

    #[test]
    fn test_resource_sharing_mode_display() {
        assert_eq!(ResourceSharingMode::ReadOnly.to_string(), "readonlyMissing message");
        assert_eq!(ResourceSharingMode::ReadWrite.to_string(), "readwriteMissing message");
        assert_eq!(ResourceSharingMode::Exclusive.to_string(), "exclusiveMissing message");
    }

    #[test]
    fn test_host_resource_manager() {
        let manager = HostResourceManager::new();
        assert_eq!(manager.resource_count(), 0);
    }
}
