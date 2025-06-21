use crate::{
    generative_types::{GenerativeResourceType, GenerativeTypeRegistry},
    handle_representation::{AccessRights, HandleOperation, HandleRepresentationManager},
    post_return::{CleanupTask, CleanupTaskType, PostReturnRegistry},
    type_bounds::{TypeBoundsChecker, TypeRelation},
    virtualization::{Capability, VirtualizationManager},
    ComponentInstanceId, ResourceHandle, TypeId,
};
use core::{
    fmt,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    component_value::WrtComponentValue,
    safe_memory::SafeMemory,
};

// Enable vec! and format! macros for no_std
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{vec, format};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

#[cfg(not(feature = "std"))]
use wrt_foundation::{BoundedString as String, BoundedVec as Vec, safe_memory::NoStdProvider};

const MAX_SHARING_AGREEMENTS: usize = 512;
const MAX_SHARED_RESOURCES: usize = 1024;
const MAX_SHARING_POLICIES: usize = 256;
const MAX_TRANSFER_QUEUE: usize = 128;
const MAX_SHARING_CALLBACKS: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceSharingError {
    pub kind: ResourceSharingErrorKind,
    pub message: String,
    pub source_component: Option<ComponentInstanceId>,
    pub target_component: Option<ComponentInstanceId>,
    pub resource: Option<ResourceHandle>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceSharingErrorKind {
    PermissionDenied,
    ResourceNotFound,
    InvalidSharingAgreement,
    PolicyViolation,
    TransferFailed,
    ResourceLimitExceeded,
    TypeMismatch,
    CircularDependency,
    ConcurrentAccess,
    CapabilityRequired,
}

impl fmt::Display for ResourceSharingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ResourceSharingError {}

pub type ResourceSharingResult<T> = Result<T, ResourceSharingError>;

#[derive(Debug, Clone)]
pub struct SharingAgreement {
    pub id: u32,
    pub source_component: ComponentInstanceId,
    pub target_component: ComponentInstanceId,
    pub resource_types: BoundedVec<TypeId, 32, NoStdProvider<65536>>,
    pub access_rights: AccessRights,
    pub transfer_policy: TransferPolicy,
    pub lifetime: SharingLifetime,
    pub established_at: u64,
    pub metadata: SharingMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferPolicy {
    Copy,            // Resource can be copied
    Move,            // Resource ownership is transferred
    Borrow,          // Temporary access granted
    SharedOwnership, // Both components own the resource
    Delegate,        // Target can further share the resource
}

#[derive(Debug, Clone)]
pub enum SharingLifetime {
    Permanent,
    Temporary { expires_at: u64 },
    SessionBased { session_id: u32 },
    RefCounted { initial_count: u32 },
    ConditionalWhile { condition: String },
}

#[derive(Debug, Clone)]
pub struct SharingMetadata {
    pub description: String,
    pub tags: BoundedVec<String, 16, NoStdProvider<65536>>,
    pub restrictions: BoundedVec<SharingRestriction, 16, NoStdProvider<65536>>,
    pub audit_log: BoundedVec<AuditEntry, 32, NoStdProvider<65536>>,
}

#[derive(Debug, Clone)]
pub enum SharingRestriction {
    NoFurtherSharing,
    ReadOnlyAfterSharing,
    MustReturnBy { deadline: u64 },
    MaxConcurrentAccess { limit: u32 },
    RequiredCapability { capability: Capability },
    GeographicRestriction { allowed_regions: BoundedVec<String, 8, NoStdProvider<65536>> },
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub action: AuditAction,
    pub component_id: ComponentInstanceId,
    pub success: bool,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditAction {
    ResourceShared,
    ResourceAccessed,
    ResourceModified,
    ResourceReturned,
    ResourceDropped,
    PolicyViolation,
}

#[derive(Debug, Clone)]
pub struct SharedResource {
    pub handle: ResourceHandle,
    pub resource_type: GenerativeResourceType,
    pub owner_component: ComponentInstanceId,
    pub shared_with: BoundedVec<ComponentInstanceId, 16, NoStdProvider<65536>>,
    pub sharing_agreements: BoundedVec<u32, 16, NoStdProvider<65536>>,
    pub access_count: AtomicU32,
    pub is_locked: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct ResourceTransferRequest {
    pub resource_handle: ResourceHandle,
    pub source_component: ComponentInstanceId,
    pub target_component: ComponentInstanceId,
    pub transfer_type: TransferType,
    pub access_rights: AccessRights,
    pub metadata: Option<WrtComponentValue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferType {
    Ownership,
    SharedAccess,
    TemporaryLoan { duration_ms: u64 },
    Delegation,
}

#[derive(Debug, Clone)]
pub struct SharingPolicy {
    pub id: u32,
    pub name: String,
    pub applies_to: PolicyScope,
    pub rules: BoundedVec<PolicyRule, 32, NoStdProvider<65536>>,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum PolicyScope {
    Global,
    ComponentPair { source: ComponentInstanceId, target: ComponentInstanceId },
    ResourceType { type_id: TypeId },
    Component { component_id: ComponentInstanceId },
}

#[derive(Debug, Clone)]
pub enum PolicyRule {
    RequireExplicitConsent,
    AllowedResourceTypes { types: BoundedVec<TypeId, 16, NoStdProvider<65536>> },
    DeniedResourceTypes { types: BoundedVec<TypeId, 16, NoStdProvider<65536>> },
    MaxShareCount { limit: u32 },
    RequiredCapabilities { capabilities: BoundedVec<Capability, 8, NoStdProvider<65536>> },
    TimeRestriction { allowed_hours: (u8, u8) },
}

pub type SharingCallback =
    Box<dyn Fn(&SharedResource, &SharingAgreement) -> ResourceSharingResult<()> + Send + Sync>;

pub struct CrossComponentResourceSharingManager {
    handle_manager: HandleRepresentationManager,
    type_registry: GenerativeTypeRegistry,
    bounds_checker: TypeBoundsChecker,
    virt_manager: Option<VirtualizationManager>,
    post_return_registry: PostReturnRegistry,

    sharing_agreements: BoundedHashMap<u32, SharingAgreement, MAX_SHARING_AGREEMENTS>,
    shared_resources: BoundedHashMap<ResourceHandle, SharedResource, MAX_SHARED_RESOURCES>,
    sharing_policies: BoundedVec<SharingPolicy, MAX_SHARING_POLICIES, NoStdProvider<65536>>,
    transfer_queue: BoundedVec<ResourceTransferRequest, MAX_TRANSFER_QUEUE, NoStdProvider<65536>>,

    callbacks: BoundedHashMap<String, SharingCallback, MAX_SHARING_CALLBACKS>,

    next_agreement_id: AtomicU32,
    next_policy_id: AtomicU32,
    enforce_policies: AtomicBool,
}

impl CrossComponentResourceSharingManager {
    pub fn new() -> Self {
        Self {
            handle_manager: HandleRepresentationManager::new(),
            type_registry: GenerativeTypeRegistry::new(),
            bounds_checker: TypeBoundsChecker::new(),
            virt_manager: None,
            post_return_registry: PostReturnRegistry::new(),

            sharing_agreements: BoundedHashMap::new(),
            shared_resources: BoundedHashMap::new(),
            sharing_policies: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            transfer_queue: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),

            callbacks: BoundedHashMap::new(),

            next_agreement_id: AtomicU32::new(1),
            next_policy_id: AtomicU32::new(1),
            enforce_policies: AtomicBool::new(true),
        }
    }

    pub fn with_virtualization(mut self, virt_manager: VirtualizationManager) -> Self {
        self.virt_manager = Some(virt_manager);
        self.handle_manager = self.handle_manager.with_virtualization(virt_manager);
        self
    }

    pub fn set_policy_enforcement(&self, enforce: bool) {
        self.enforce_policies.store(enforce, Ordering::SeqCst);
    }

    pub fn establish_sharing_agreement(
        &mut self,
        source_component: ComponentInstanceId,
        target_component: ComponentInstanceId,
        resource_types: BoundedVec<TypeId, 32, NoStdProvider<65536>>,
        access_rights: AccessRights,
        transfer_policy: TransferPolicy,
        lifetime: SharingLifetime,
    ) -> ResourceSharingResult<u32> {
        // Validate components
        self.validate_components(source_component, target_component)?;

        // Check policies
        if self.enforce_policies.load(Ordering::Acquire) {
            self.check_sharing_policies(source_component, target_component, &resource_types)?;
        }

        let agreement_id = self.next_agreement_id.fetch_add(1, Ordering::SeqCst);

        let agreement = SharingAgreement {
            id: agreement_id,
            source_component,
            target_component,
            resource_types,
            access_rights,
            transfer_policy,
            lifetime,
            established_at: self.get_current_time(),
            metadata: SharingMetadata {
                description: format!(
                    "Agreement between {} and {}",
                    source_component.id(),
                    target_component.id()
                ),
                tags: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
                restrictions: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
                audit_log: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            },
        };

        self.sharing_agreements.insert(agreement_id, agreement).map_err(|_| {
            ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                message: "Too many sharing agreements".to_string(),
                source_component: Some(source_component),
                target_component: Some(target_component),
                resource: None,
            }
        })?;

        // Audit log
        self.audit_action(
            agreement_id,
            AuditAction::ResourceShared,
            source_component,
            true,
            "Sharing agreement established",
        )?;

        Ok(agreement_id)
    }

    pub fn share_resource(
        &mut self,
        agreement_id: u32,
        resource_handle: ResourceHandle,
    ) -> ResourceSharingResult<ResourceHandle> {
        let agreement = self.get_agreement(agreement_id)?;

        // Verify resource type matches agreement
        let resource_type = self
            .handle_manager
            .get_representation(resource_handle)
            .map_err(|e| ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceNotFound,
                message: "Component not found",
                source_component: Some(agreement.source_component),
                target_component: Some(agreement.target_component),
                resource: Some(resource_handle),
            })?
            .type_id;

        if !agreement.resource_types.contains(&resource_type) {
            return Err(ResourceSharingError {
                kind: ResourceSharingErrorKind::TypeMismatch,
                message: "Resource type not covered by agreement".to_string(),
                source_component: Some(agreement.source_component),
                target_component: Some(agreement.target_component),
                resource: Some(resource_handle),
            });
        }

        // Create shared handle
        let shared_handle = self
            .handle_manager
            .share_handle(
                agreement.source_component,
                agreement.target_component,
                resource_handle,
                agreement.access_rights,
            )
            .map_err(|e| ResourceSharingError {
                kind: ResourceSharingErrorKind::TransferFailed,
                message: "Component not found",
                source_component: Some(agreement.source_component),
                target_component: Some(agreement.target_component),
                resource: Some(resource_handle),
            })?;

        // Track shared resource
        self.track_shared_resource(
            resource_handle,
            agreement.source_component,
            agreement.target_component,
            agreement_id,
        )?;

        // Execute callbacks
        self.execute_sharing_callbacks(resource_handle, &agreement)?;

        // Audit log
        self.audit_action(
            agreement_id,
            AuditAction::ResourceShared,
            agreement.source_component,
            true,
            &"Component not found",
        )?;

        Ok(shared_handle)
    }

    pub fn transfer_resource_ownership(
        &mut self,
        resource_handle: ResourceHandle,
        source_component: ComponentInstanceId,
        target_component: ComponentInstanceId,
    ) -> ResourceSharingResult<()> {
        // Create transfer request
        let request = ResourceTransferRequest {
            resource_handle,
            source_component,
            target_component,
            transfer_type: TransferType::Ownership,
            access_rights: AccessRights::full_access(),
            metadata: None,
        };

        // Process transfer
        self.process_resource_transfer(request)?;

        // Update ownership in shared resources
        if let Some(shared_resource) = self.shared_resources.get_mut(&resource_handle) {
            shared_resource.owner_component = target_component;
        }

        // Add cleanup task for source component
        let cleanup_task = CleanupTask::resource_cleanup(resource_handle);
        self.post_return_registry.add_cleanup_task(source_component, cleanup_task).map_err(
            |e| ResourceSharingError {
                kind: ResourceSharingErrorKind::TransferFailed,
                message: "Component not found",
                source_component: Some(source_component),
                target_component: Some(target_component),
                resource: Some(resource_handle),
            },
        )?;

        Ok(())
    }

    pub fn access_shared_resource(
        &mut self,
        component_id: ComponentInstanceId,
        resource_handle: ResourceHandle,
        operation: HandleOperation,
    ) -> ResourceSharingResult<Option<WrtComponentValue>> {
        // Check if resource is shared
        let shared_resource =
            self.shared_resources.get(&resource_handle).ok_or_else(|| ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceNotFound,
                message: "Resource not shared".to_string(),
                source_component: Some(component_id),
                target_component: None,
                resource: Some(resource_handle),
            })?;

        // Check if component has access
        if !shared_resource.shared_with.contains(&component_id)
            && shared_resource.owner_component != component_id
        {
            return Err(ResourceSharingError {
                kind: ResourceSharingErrorKind::PermissionDenied,
                message: "Component does not have access to shared resource".to_string(),
                source_component: Some(component_id),
                target_component: None,
                resource: Some(resource_handle),
            });
        }

        // Check if resource is locked
        if shared_resource.is_locked.load(Ordering::Acquire) {
            return Err(ResourceSharingError {
                kind: ResourceSharingErrorKind::ConcurrentAccess,
                message: "Resource is locked".to_string(),
                source_component: Some(component_id),
                target_component: None,
                resource: Some(resource_handle),
            });
        }

        // Increment access count
        shared_resource.access_count.fetch_add(1, Ordering::SeqCst);

        // Perform operation through handle manager
        let result = self
            .handle_manager
            .perform_operation(component_id, resource_handle, operation)
            .map_err(|e| ResourceSharingError {
                kind: ResourceSharingErrorKind::TransferFailed,
                message: "Component not found",
                source_component: Some(component_id),
                target_component: None,
                resource: Some(resource_handle),
            })?;

        // Audit access
        for agreement_id in shared_resource.sharing_agreements.iter() {
            self.audit_action(
                *agreement_id,
                AuditAction::ResourceAccessed,
                component_id,
                true,
                &"Component not found",
            )?;
        }

        Ok(result)
    }

    pub fn return_shared_resource(
        &mut self,
        component_id: ComponentInstanceId,
        resource_handle: ResourceHandle,
    ) -> ResourceSharingResult<()> {
        let shared_resource = self.shared_resources.get_mut(&resource_handle).ok_or_else(|| {
            ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceNotFound,
                message: "Resource not shared".to_string(),
                source_component: Some(component_id),
                target_component: None,
                resource: Some(resource_handle),
            }
        })?;

        // Remove component from shared list
        if let Some(pos) = shared_resource.shared_with.iter().position(|&id| id == component_id) {
            shared_resource.shared_with.remove(pos);
        }

        // Drop the handle for this component
        self.handle_manager.drop_handle(component_id, resource_handle).map_err(|e| {
            ResourceSharingError {
                kind: ResourceSharingErrorKind::TransferFailed,
                message: "Component not found",
                source_component: Some(component_id),
                target_component: None,
                resource: Some(resource_handle),
            }
        })?;

        // Audit return
        for agreement_id in shared_resource.sharing_agreements.iter() {
            self.audit_action(
                *agreement_id,
                AuditAction::ResourceReturned,
                component_id,
                true,
                &"Component not found",
            )?;
        }

        Ok(())
    }

    pub fn add_sharing_policy(&mut self, policy: SharingPolicy) -> ResourceSharingResult<u32> {
        let policy_id = policy.id;

        self.sharing_policies.push(policy).map_err(|_| ResourceSharingError {
            kind: ResourceSharingErrorKind::ResourceLimitExceeded,
            message: "Too many sharing policies".to_string(),
            source_component: None,
            target_component: None,
            resource: None,
        })?;

        Ok(policy_id)
    }

    pub fn register_sharing_callback(
        &mut self,
        name: String,
        callback: SharingCallback,
    ) -> ResourceSharingResult<()> {
        self.callbacks.insert(name, callback).map_err(|_| ResourceSharingError {
            kind: ResourceSharingErrorKind::ResourceLimitExceeded,
            message: "Too many callbacks".to_string(),
            source_component: None,
            target_component: None,
            resource: None,
        })?;

        Ok(())
    }

    pub fn get_shared_resources_for_component(
        &self,
        component_id: ComponentInstanceId,
    ) -> Vec<ResourceHandle> {
        self.shared_resources
            .iter()
            .filter(|(_, shared)| {
                shared.owner_component == component_id || shared.shared_with.contains(&component_id)
            })
            .map(|(handle, _)| *handle)
            .collect()
    }

    pub fn get_sharing_statistics(&self) -> SharingStatistics {
        SharingStatistics {
            total_agreements: self.sharing_agreements.len(),
            active_agreements: self.count_active_agreements(),
            total_shared_resources: self.shared_resources.len(),
            total_policies: self.sharing_policies.len(),
            pending_transfers: self.transfer_queue.len(),
        }
    }

    fn validate_components(
        &self,
        source: ComponentInstanceId,
        target: ComponentInstanceId,
    ) -> ResourceSharingResult<()> {
        if source == target {
            return Err(ResourceSharingError {
                kind: ResourceSharingErrorKind::InvalidSharingAgreement,
                message: "Cannot share with self".to_string(),
                source_component: Some(source),
                target_component: Some(target),
                resource: None,
            });
        }

        // Check for circular dependencies
        if self.would_create_circular_dependency(source, target) {
            return Err(ResourceSharingError {
                kind: ResourceSharingErrorKind::CircularDependency,
                message: "Would create circular dependency".to_string(),
                source_component: Some(source),
                target_component: Some(target),
                resource: None,
            });
        }

        Ok(())
    }

    fn check_sharing_policies(
        &self,
        source: ComponentInstanceId,
        target: ComponentInstanceId,
        resource_types: &[TypeId],
    ) -> ResourceSharingResult<()> {
        for policy in self.sharing_policies.iter().filter(|p| p.enabled) {
            if !self.policy_applies_to(policy, source, target, resource_types) {
                continue;
            }

            for rule in policy.rules.iter() {
                self.check_policy_rule(rule, source, target, resource_types)?;
            }
        }

        Ok(())
    }

    fn policy_applies_to(
        &self,
        policy: &SharingPolicy,
        source: ComponentInstanceId,
        target: ComponentInstanceId,
        resource_types: &[TypeId],
    ) -> bool {
        match &policy.applies_to {
            PolicyScope::Global => true,
            PolicyScope::ComponentPair { source: s, target: t } => *s == source && *t == target,
            PolicyScope::ResourceType { type_id } => resource_types.contains(type_id),
            PolicyScope::Component { component_id } => {
                *component_id == source || *component_id == target
            }
        }
    }

    fn check_policy_rule(
        &self,
        rule: &PolicyRule,
        source: ComponentInstanceId,
        target: ComponentInstanceId,
        resource_types: &[TypeId],
    ) -> ResourceSharingResult<()> {
        match rule {
            PolicyRule::DeniedResourceTypes { types } => {
                for resource_type in resource_types {
                    if types.contains(resource_type) {
                        return Err(ResourceSharingError {
                            kind: ResourceSharingErrorKind::PolicyViolation,
                            message: "Resource type denied by policy".to_string(),
                            source_component: Some(source),
                            target_component: Some(target),
                            resource: None,
                        });
                    }
                }
            }
            PolicyRule::AllowedResourceTypes { types } => {
                for resource_type in resource_types {
                    if !types.contains(resource_type) {
                        return Err(ResourceSharingError {
                            kind: ResourceSharingErrorKind::PolicyViolation,
                            message: "Resource type not allowed by policy".to_string(),
                            source_component: Some(source),
                            target_component: Some(target),
                            resource: None,
                        });
                    }
                }
            }
            PolicyRule::RequiredCapabilities { capabilities } => {
                if let Some(ref virt_manager) = self.virt_manager {
                    for capability in capabilities.iter() {
                        if !virt_manager.check_capability(target, capability) {
                            return Err(ResourceSharingError {
                                kind: ResourceSharingErrorKind::CapabilityRequired,
                                message: format!(
                                    "Target missing required capability: {:?}",
                                    capability
                                ),
                                source_component: Some(source),
                                target_component: Some(target),
                                resource: None,
                            });
                        }
                    }
                }
            }
            _ => {} // Other rules would be checked elsewhere
        }

        Ok(())
    }

    fn would_create_circular_dependency(
        &self,
        source: ComponentInstanceId,
        target: ComponentInstanceId,
    ) -> bool {
        // Simple check - in a real implementation, this would do graph traversal
        self.sharing_agreements.values().any(|agreement| {
            agreement.source_component == target && agreement.target_component == source
        })
    }

    fn track_shared_resource(
        &mut self,
        handle: ResourceHandle,
        owner: ComponentInstanceId,
        shared_with: ComponentInstanceId,
        agreement_id: u32,
    ) -> ResourceSharingResult<()> {
        if let Some(shared_resource) = self.shared_resources.get_mut(&handle) {
            // Add to existing shared resource
            if !shared_resource.shared_with.contains(&shared_with) {
                shared_resource.shared_with.push(shared_with).map_err(|_| {
                    ResourceSharingError {
                        kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                        message: "Too many components sharing resource".to_string(),
                        source_component: Some(owner),
                        target_component: Some(shared_with),
                        resource: Some(handle),
                    }
                })?;
            }

            if !shared_resource.sharing_agreements.contains(&agreement_id) {
                shared_resource.sharing_agreements.push(agreement_id).map_err(|_| {
                    ResourceSharingError {
                        kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                        message: "Too many agreements for resource".to_string(),
                        source_component: Some(owner),
                        target_component: Some(shared_with),
                        resource: Some(handle),
                    }
                })?;
            }
        } else {
            // Create new shared resource entry
            let resource_type =
                self.type_registry.get_resource_type(handle).map_err(|e| ResourceSharingError {
                    kind: ResourceSharingErrorKind::ResourceNotFound,
                    message: "Component not found",
                    source_component: Some(owner),
                    target_component: Some(shared_with),
                    resource: Some(handle),
                })?;

            let mut shared_with_vec = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();
            shared_with_vec.push(shared_with).map_err(|_| ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                message: "Failed to create shared_with list".to_string(),
                source_component: Some(owner),
                target_component: Some(shared_with),
                resource: Some(handle),
            })?;

            let mut agreements_vec = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();
            agreements_vec.push(agreement_id).map_err(|_| ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                message: "Failed to create agreements list".to_string(),
                source_component: Some(owner),
                target_component: Some(shared_with),
                resource: Some(handle),
            })?;

            let shared_resource = SharedResource {
                handle,
                resource_type,
                owner_component: owner,
                shared_with: shared_with_vec,
                sharing_agreements: agreements_vec,
                access_count: AtomicU32::new(0),
                is_locked: AtomicBool::new(false),
            };

            self.shared_resources.insert(handle, shared_resource).map_err(|_| {
                ResourceSharingError {
                    kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                    message: "Too many shared resources".to_string(),
                    source_component: Some(owner),
                    target_component: Some(shared_with),
                    resource: Some(handle),
                }
            })?;
        }

        Ok(())
    }

    fn process_resource_transfer(
        &mut self,
        request: ResourceTransferRequest,
    ) -> ResourceSharingResult<()> {
        // This would handle the actual transfer logic
        // For now, we'll add it to the queue
        self.transfer_queue.push(request).map_err(|_| ResourceSharingError {
            kind: ResourceSharingErrorKind::ResourceLimitExceeded,
            message: "Transfer queue full".to_string(),
            source_component: None,
            target_component: None,
            resource: None,
        })?;

        Ok(())
    }

    fn execute_sharing_callbacks(
        &self,
        handle: ResourceHandle,
        agreement: &SharingAgreement,
    ) -> ResourceSharingResult<()> {
        if let Some(shared_resource) = self.shared_resources.get(&handle) {
            for callback in self.callbacks.values() {
                callback(shared_resource, agreement)?;
            }
        }
        Ok(())
    }

    fn get_agreement(&self, agreement_id: u32) -> ResourceSharingResult<&SharingAgreement> {
        self.sharing_agreements.get(&agreement_id).ok_or_else(|| ResourceSharingError {
            kind: ResourceSharingErrorKind::InvalidSharingAgreement,
            message: "Component not found",
            source_component: None,
            target_component: None,
            resource: None,
        })
    }

    fn audit_action(
        &mut self,
        agreement_id: u32,
        action: AuditAction,
        component_id: ComponentInstanceId,
        success: bool,
        details: &str,
    ) -> ResourceSharingResult<()> {
        if let Some(agreement) = self.sharing_agreements.get_mut(&agreement_id) {
            let entry = AuditEntry {
                timestamp: self.get_current_time(),
                action,
                component_id,
                success,
                details: details.to_string(),
            };

            agreement.metadata.audit_log.push(entry).map_err(|_| ResourceSharingError {
                kind: ResourceSharingErrorKind::ResourceLimitExceeded,
                message: "Audit log full".to_string(),
                source_component: Some(agreement.source_component),
                target_component: Some(agreement.target_component),
                resource: None,
            })?;
        }

        Ok(())
    }

    fn count_active_agreements(&self) -> usize {
        let current_time = self.get_current_time();

        self.sharing_agreements
            .values()
            .filter(|agreement| {
                match &agreement.lifetime {
                    SharingLifetime::Permanent => true,
                    SharingLifetime::Temporary { expires_at } => current_time < *expires_at,
                    _ => true, // Other lifetimes require more complex checks
                }
            })
            .count()
    }

    fn get_current_time(&self) -> u64 {
        #[cfg(feature = "std")]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
        }
        #[cfg(not(feature = "std"))]
        {
            0
        }
    }
}

impl Default for CrossComponentResourceSharingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharingStatistics {
    pub total_agreements: usize,
    pub active_agreements: usize,
    pub total_shared_resources: usize,
    pub total_policies: usize,
    pub pending_transfers: usize,
}

pub fn create_basic_sharing_policy(name: &str) -> SharingPolicy {
    SharingPolicy {
        id: 0, // Will be assigned by manager
        name: name.to_string(),
        applies_to: PolicyScope::Global,
        rules: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        priority: 0,
        enabled: true,
    }
}

pub fn create_component_pair_policy(
    name: &str,
    source: ComponentInstanceId,
    target: ComponentInstanceId,
) -> SharingPolicy {
    SharingPolicy {
        id: 0,
        name: name.to_string(),
        applies_to: PolicyScope::ComponentPair { source, target },
        rules: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        priority: 0,
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharing_manager_creation() {
        let manager = CrossComponentResourceSharingManager::new();
        assert!(manager.enforce_policies.load(Ordering::Acquire));
    }

    #[test]
    fn test_transfer_policy() {
        assert_eq!(TransferPolicy::Copy, TransferPolicy::Copy);
        assert_ne!(TransferPolicy::Move, TransferPolicy::Borrow);
    }

    #[test]
    fn test_sharing_statistics() {
        let stats = SharingStatistics::default();
        assert_eq!(stats.total_agreements, 0);
        assert_eq!(stats.active_agreements, 0);
        assert_eq!(stats.total_shared_resources, 0);
    }

    #[test]
    fn test_basic_policy_creation() {
        let policy = create_basic_sharing_policy("test-policy");
        assert_eq!(policy.name, "test-policy");
        assert!(matches!(policy.applies_to, PolicyScope::Global));
        assert!(policy.enabled);
    }
}
