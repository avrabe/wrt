//! Cross-component function calls
//!
//! This module implements the mechanism for calling functions between different
//! component instances, handling type adaptation and resource management.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, component::ComponentType, component_value::ComponentValue, prelude::*,
};

use crate::{
    canonical::CanonicalAbi,
    execution_engine::ComponentExecutionEngine,
    resource_lifecycle::ResourceLifecycleManager,
    types::{ComponentInstance, ValType, Value},
    WrtResult,
};

/// Maximum number of call targets in no_std environments
const MAX_CALL_TARGETS: usize = 256;

/// Maximum call stack depth for cross-component calls
const MAX_CROSS_CALL_DEPTH: usize = 64;

/// Cross-component call manager
pub struct CrossComponentCallManager {
    /// Call targets registry
    #[cfg(feature = "std")]
    targets: Vec<CallTarget>,
    #[cfg(not(any(feature = "std", )))]
    targets: BoundedVec<CallTarget, MAX_CALL_TARGETS, NoStdProvider<65536>>,

    /// Call stack for tracking cross-component calls
    #[cfg(feature = "std")]
    call_stack: Vec<CrossCallFrame>,
    #[cfg(not(any(feature = "std", )))]
    call_stack: BoundedVec<CrossCallFrame, MAX_CROSS_CALL_DEPTH, NoStdProvider<65536>>,

    /// Canonical ABI processor
    canonical_abi: CanonicalAbi,

    /// Resource manager for cross-component resource transfer
    resource_manager: ResourceLifecycleManager,

    /// Maximum call depth
    max_call_depth: usize,
}

/// Call target for cross-component calls
#[derive(Debug, Clone)]
pub struct CallTarget {
    /// Target component instance ID
    pub target_instance: u32,
    /// Target function index within the component
    pub function_index: u32,
    /// Function signature
    pub signature: ComponentType,
    /// Call permissions
    pub permissions: CallPermissions,
    /// Resource transfer policy
    pub resource_policy: ResourceTransferPolicy,
}

/// Call permissions for cross-component calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallPermissions {
    /// Whether the call is allowed
    pub allowed: bool,
    /// Whether resources can be transferred
    pub allow_resource_transfer: bool,
    /// Whether memory access is allowed
    pub allow_memory_access: bool,
    /// Maximum call frequency (calls per second, 0 for unlimited)
    pub max_frequency: u32,
}

/// Resource transfer policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceTransferPolicy {
    /// No resource transfer allowed
    None,
    /// Transfer ownership
    Transfer,
    /// Borrow resources (caller retains ownership)
    Borrow,
    /// Copy resources (if possible)
    Copy,
}

/// Cross-component call frame
#[derive(Debug, Clone)]
pub struct CrossCallFrame {
    /// Caller instance ID
    pub caller_instance: u32,
    /// Target instance ID
    pub target_instance: u32,
    /// Function being called
    pub function_index: u32,
    /// Call start time (simplified - would use proper time type)
    pub start_time: u64,
    /// Resources transferred in this call
    #[cfg(feature = "std")]
    pub transferred_resources: Vec<TransferredResource>,
    #[cfg(not(any(feature = "std", )))]
    pub transferred_resources: BoundedVec<TransferredResource, 32, NoStdProvider<65536>>,
}

/// Record of a transferred resource
#[derive(Debug, Clone)]
pub struct TransferredResource {
    /// Resource handle
    pub handle: u32,
    /// Transfer type
    pub transfer_type: ResourceTransferPolicy,
    /// Original owner (for restoration on error)
    pub original_owner: u32,
}

/// Call result with resource tracking
#[derive(Debug, Clone)]
pub struct CrossCallResult {
    /// Function call result
    pub result: WrtResult<Value>,
    /// Resources that were transferred
    #[cfg(feature = "std")]
    pub transferred_resources: Vec<TransferredResource>,
    #[cfg(not(any(feature = "std", )))]
    pub transferred_resources: BoundedVec<TransferredResource, 32, NoStdProvider<65536>>,
    /// Call statistics
    pub stats: CallStatistics,
}

/// Call statistics
#[derive(Debug, Clone)]
pub struct CallStatistics {
    /// Call duration in nanoseconds
    pub duration_ns: u64,
    /// Number of arguments
    pub arg_count: u32,
    /// Number of resources transferred
    pub resources_transferred: u32,
    /// Memory bytes accessed
    pub memory_accessed: u64,
}

impl CrossComponentCallManager {
    /// Create a new cross-component call manager
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            targets: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            targets: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            call_stack: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            call_stack: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            canonical_abi: CanonicalAbi::new(),
            resource_manager: ResourceLifecycleManager::new(),
            max_call_depth: MAX_CROSS_CALL_DEPTH,
        }
    }

    /// Set maximum call depth
    pub fn set_max_call_depth(&mut self, depth: usize) {
        self.max_call_depth = depth;
    }

    /// Register a call target
    pub fn register_target(&mut self, target: CallTarget) -> WrtResult<u32> {
        let target_id = self.targets.len() as u32;

        #[cfg(feature = "std")]
        {
            self.targets.push(target);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.targets.push(target).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many call targets"
                )
            })?;
        }

        Ok(target_id)
    }

    /// Make a cross-component call
    pub fn call(
        &mut self,
        caller_instance: u32,
        target_id: u32,
        args: &[Value],
        engine: &mut ComponentExecutionEngine,
    ) -> WrtResult<CrossCallResult> {
        // Check call depth
        if self.call_stack.len() >= self.max_call_depth {
            return Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Resource,
                wrt_error::codes::RESOURCE_EXHAUSTED,
                "Maximum call depth exceeded"
            ));
        }

        // Get target
        let target = self
            .targets
            .get(target_id as usize)
            .ok_or_else(|| wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Validation,
                wrt_error::errors::codes::INVALID_INPUT,
                "Invalid input"
            ))?
            .clone();

        // Check permissions
        if !target.permissions.allowed {
            return Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Runtime,
                wrt_error::codes::RUNTIME_ERROR,
                "Cross-component call not allowed"
            ));
        }

        // Create call frame
        let start_time = self.get_current_time();
        let call_frame = CrossCallFrame {
            caller_instance,
            target_instance: target.target_instance,
            function_index: target.function_index,
            start_time,
            #[cfg(feature = "std")]
            transferred_resources: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            transferred_resources: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        };

        // Push call frame
        #[cfg(feature = "std")]
        {
            self.call_stack.push(call_frame);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.call_stack.push(call_frame).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Call stack overflow"
                )
            })?;
        }

        // Prepare arguments with resource transfer
        let (prepared_args, transferred_resources) =
            self.prepare_arguments(args, &target, caller_instance)?;

        // Update call frame with transferred resources
        if let Some(frame) = self.call_stack.last_mut() {
            frame.transferred_resources = transferred_resources.clone();
        }

        // Make the actual call
        let call_result =
            engine.call_function(target.target_instance, target.function_index, &prepared_args);

        // Calculate statistics
        let end_time = self.get_current_time();
        let stats = CallStatistics {
            duration_ns: end_time - start_time,
            arg_count: args.len() as u32,
            resources_transferred: transferred_resources.len() as u32,
            memory_accessed: 0, // Would be tracked by memory manager
        };

        // Handle call result
        let result = match call_result {
            Ok(value) => {
                // Call succeeded - finalize resource transfers
                self.finalize_resource_transfers(&transferred_resources)?;
                CrossCallResult { result: Ok(value), transferred_resources, stats }
            }
            Err(error) => {
                // Call failed - restore resources
                self.restore_resources(&transferred_resources)?;
                CrossCallResult {
                    result: Err(error),
                    #[cfg(feature = "std")]
                    transferred_resources: Vec::new(),
                    #[cfg(not(any(feature = "std", )))]
                    transferred_resources: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
                    stats,
                }
            }
        };

        // Pop call frame
        #[cfg(feature = "std")]
        {
            self.call_stack.pop();
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let _ = self.call_stack.pop();
        }

        Ok(result)
    }

    /// Prepare arguments for cross-component call
    fn prepare_arguments(
        &mut self,
        args: &[Value],
        target: &CallTarget,
        caller_instance: u32,
    ) -> Wrtcore::result::Result<(Vec<Value>, Vec<TransferredResource>)> {
        #[cfg(feature = "std")]
        let mut prepared_args = Vec::new();
        #[cfg(not(any(feature = "std", )))]
        let mut prepared_args = Vec::new();

        #[cfg(feature = "std")]
        let mut transferred_resources = Vec::new();
        #[cfg(not(any(feature = "std", )))]
        let mut transferred_resources = Vec::new();

        for arg in args {
            match arg {
                Value::Own(handle) | Value::Borrow(handle) => {
                    // Handle resource arguments
                    if target.permissions.allow_resource_transfer {
                        let transfer_type = target.resource_policy;
                        let transferred = self.transfer_resource(
                            *handle,
                            caller_instance,
                            target.target_instance,
                            transfer_type,
                        )?;
                        transferred_resources.push(transferred);
                        prepared_args.push(arg.clone());
                    } else {
                        return Err(wrt_foundation::Error::new(
                            wrt_foundation::ErrorCategory::Runtime,
                            wrt_error::codes::RUNTIME_ERROR,
                            "Resource transfer not allowed"
                        ));
                    }
                }
                _ => {
                    // Regular value arguments
                    prepared_args.push(arg.clone());
                }
            }
        }

        Ok((prepared_args, transferred_resources))
    }

    /// Transfer a resource between components
    fn transfer_resource(
        &mut self,
        handle: u32,
        from_instance: u32,
        to_instance: u32,
        transfer_type: ResourceTransferPolicy,
    ) -> WrtResult<TransferredResource> {
        match transfer_type {
            ResourceTransferPolicy::None => Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Runtime,
                wrt_error::codes::RUNTIME_ERROR,
                "Resource transfer not allowed"
            )),
            ResourceTransferPolicy::Transfer => {
                // Transfer ownership
                self.resource_manager.transfer_ownership(
                    wrt_foundation::resource::ResourceHandle(handle),
                    to_instance,
                )?;
                Ok(TransferredResource { handle, transfer_type, original_owner: from_instance })
            }
            ResourceTransferPolicy::Borrow => {
                // Borrow resource (no ownership change)
                Ok(TransferredResource { handle, transfer_type, original_owner: from_instance })
            }
            ResourceTransferPolicy::Copy => {
                // Copy resource (if possible)
                // This would create a new resource with copied data
                // For now, treat as borrow
                Ok(TransferredResource {
                    handle,
                    transfer_type: ResourceTransferPolicy::Borrow,
                    original_owner: from_instance,
                })
            }
        }
    }

    /// Finalize resource transfers after successful call
    fn finalize_resource_transfers(&mut self, transfers: &[TransferredResource]) -> WrtResult<()> {
        for transfer in transfers {
            match transfer.transfer_type {
                ResourceTransferPolicy::Transfer => {
                    // Transfer is already finalized
                }
                ResourceTransferPolicy::Borrow => {
                    // Nothing to finalize for borrow
                }
                ResourceTransferPolicy::Copy => {
                    // Finalize copy (would commit the copied resource)
                }
                ResourceTransferPolicy::None => {
                    // Should not happen
                }
            }
        }
        Ok(())
    }

    /// Restore resources after failed call
    fn restore_resources(&mut self, transfers: &[TransferredResource]) -> WrtResult<()> {
        for transfer in transfers {
            match transfer.transfer_type {
                ResourceTransferPolicy::Transfer => {
                    // Restore ownership to original owner
                    self.resource_manager.transfer_ownership(
                        wrt_foundation::resource::ResourceHandle(transfer.handle),
                        transfer.original_owner,
                    )?;
                }
                ResourceTransferPolicy::Borrow => {
                    // Nothing to restore for borrow
                }
                ResourceTransferPolicy::Copy => {
                    // Remove copied resource (would clean up the copy)
                }
                ResourceTransferPolicy::None => {
                    // Should not happen
                }
            }
        }
        Ok(())
    }

    /// Get current time (simplified)
    fn get_current_time(&self) -> u64 {
        // In a real implementation, would use proper time measurement
        0
    }

    /// Get call target by ID
    pub fn get_target(&self, target_id: u32) -> Option<&CallTarget> {
        self.targets.get(target_id as usize)
    }

    /// Get current call depth
    pub fn call_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// Check if a call from one instance to another is allowed
    pub fn is_call_allowed(&self, from_instance: u32, to_instance: u32) -> bool {
        // Check if there's a registered target for this call
        for target in &self.targets {
            if target.target_instance == to_instance {
                return target.permissions.allowed;
            }
        }
        false
    }

    /// Get call statistics for the current call
    pub fn current_call_stats(&self) -> Option<&CrossCallFrame> {
        self.call_stack.last()
    }
}

impl CallTarget {
    /// Create a new call target
    pub fn new(
        target_instance: u32,
        function_index: u32,
        signature: ComponentType,
        permissions: CallPermissions,
        resource_policy: ResourceTransferPolicy,
    ) -> Self {
        Self { target_instance, function_index, signature, permissions, resource_policy }
    }
}

impl Default for CallPermissions {
    fn default() -> Self {
        Self {
            allowed: true,
            allow_resource_transfer: false,
            allow_memory_access: false,
            max_frequency: 0, // Unlimited
        }
    }
}

impl Default for CrossComponentCallManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ResourceTransferPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceTransferPolicy::None => write!(f, "none"),
            ResourceTransferPolicy::Transfer => write!(f, "transfer"),
            ResourceTransferPolicy::Borrow => write!(f, "borrow"),
            ResourceTransferPolicy::Copy => write!(f, "copy"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_manager_creation() {
        let manager = CrossComponentCallManager::new();
        assert_eq!(manager.call_depth(), 0);
        assert_eq!(manager.targets.len(), 0);
    }

    #[test]
    fn test_register_target() {
        let mut manager = CrossComponentCallManager::new();

        let target = CallTarget::new(
            1,
            0,
            ComponentType::Unit,
            CallPermissions::default(),
            ResourceTransferPolicy::None,
        );

        let target_id = manager.register_target(target).unwrap();
        assert_eq!(target_id, 0);
        assert_eq!(manager.targets.len(), 1);
    }

    #[test]
    fn test_call_permissions() {
        let perms = CallPermissions::default();
        assert!(perms.allowed);
        assert!(!perms.allow_resource_transfer);
        assert!(!perms.allow_memory_access);
        assert_eq!(perms.max_frequency, 0);
    }

    #[test]
    fn test_resource_transfer_policy_display() {
        assert_eq!(ResourceTransferPolicy::None.to_string(), "none");
        assert_eq!(ResourceTransferPolicy::Transfer.to_string(), "transfer");
        assert_eq!(ResourceTransferPolicy::Borrow.to_string(), "borrow");
        assert_eq!(ResourceTransferPolicy::Copy.to_string(), "copy");
    }

    #[test]
    fn test_call_target_creation() {
        let target = CallTarget::new(
            1,
            0,
            ComponentType::Unit,
            CallPermissions::default(),
            ResourceTransferPolicy::Borrow,
        );

        assert_eq!(target.target_instance, 1);
        assert_eq!(target.function_index, 0);
        assert_eq!(target.resource_policy, ResourceTransferPolicy::Borrow);
    }

    #[test]
    fn test_is_call_allowed() {
        let mut manager = CrossComponentCallManager::new();

        // No targets registered - should not be allowed
        assert!(!manager.is_call_allowed(0, 1));

        // Register a target
        let target = CallTarget::new(
            1,
            0,
            ComponentType::Unit,
            CallPermissions::default(),
            ResourceTransferPolicy::None,
        );
        manager.register_target(target).unwrap();

        // Now should be allowed
        assert!(manager.is_call_allowed(0, 1));
    }
}
