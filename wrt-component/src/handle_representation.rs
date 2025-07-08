use crate::{
    generative_types::{GenerativeResourceType, GenerativeTypeRegistry},
    type_bounds::{TypeBoundsChecker, TypeRelation},
    virtualization::{Capability, VirtualizationManager},
    ComponentInstanceId, ResourceHandle, TypeId,
};
use core::{
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    component_value::ComponentValue,
    safe_memory::SafeMemory,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

// Enable vec! and format! macros for no_std
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{vec, format};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

#[cfg(not(feature = "std"))]
use wrt_foundation::{BoundedString as String, BoundedVec as Vec};

const MAX_HANDLE_REPRESENTATIONS: usize = 1024;
const MAX_HANDLE_OPERATIONS: usize = 512;
const MAX_HANDLE_METADATA: usize = 256;
const MAX_ACCESS_POLICIES: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub struct HandleRepresentationError {
    pub kind: HandleRepresentationErrorKind,
    pub message: String,
    pub handle: Option<ResourceHandle>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HandleRepresentationErrorKind {
    InvalidHandle,
    TypeMismatch,
    AccessDenied,
    HandleNotFound,
    OperationNotSupported,
    ResourceExhausted,
    ValidationFailed,
    CapabilityRequired,
}

impl fmt::Display for HandleRepresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HandleRepresentationError {}

// Conversion to wrt_error::Error for unified error handling
impl From<HandleRepresentationError> for wrt_error::Error {
    fn from(err: HandleRepresentationError) -> Self {
        use wrt_error::{ErrorCategory, codes};
        match err.kind {
            HandleRepresentationErrorKind::InvalidHandle => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_HANDLE_REPRESENTATION_ERROR,
                "Invalid resource handle",
            ),
            HandleRepresentationErrorKind::TypeMismatch => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_ABI_RUNTIME_ERROR,
                "Handle type mismatch",
            ),
            HandleRepresentationErrorKind::AccessDenied => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_CAPABILITY_DENIED,
                "Handle access denied",
            ),
            HandleRepresentationErrorKind::HandleNotFound => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_HANDLE_REPRESENTATION_ERROR,
                "Resource handle not found",
            ),
            HandleRepresentationErrorKind::OperationNotSupported => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_ABI_RUNTIME_ERROR,
                "Handle operation not supported",
            ),
            HandleRepresentationErrorKind::ResourceExhausted => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_RESOURCE_LIFECYCLE_ERROR,
                "Handle resource exhausted",
            ),
            HandleRepresentationErrorKind::ValidationFailed => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_CONFIGURATION_INVALID,
                "Handle validation failed",
            ),
            HandleRepresentationErrorKind::CapabilityRequired => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_CAPABILITY_DENIED,
                "Handle operation requires capability",
            ),
        }
    }
}

pub type HandleRepresentationResult<T> = wrt_error::Result<T>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleRepresentation {
    pub handle: ResourceHandle,
    pub type_id: TypeId,
    pub component_id: ComponentInstanceId,
    pub access_rights: AccessRights,
    pub is_owned: bool,
    pub reference_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccessRights {
    pub can_read: bool,
    pub can_write: bool,
    pub can_drop: bool,
    pub can_share: bool,
    pub can_borrow: bool,
}

impl AccessRights {
    pub fn read_only() -> Self {
        Self {
            can_read: true,
            can_write: false,
            can_drop: false,
            can_share: false,
            can_borrow: true,
        }
    }

    pub fn full_access() -> Self {
        Self { can_read: true, can_write: true, can_drop: true, can_share: true, can_borrow: true }
    }

    pub fn no_access() -> Self {
        Self {
            can_read: false,
            can_write: false,
            can_drop: false,
            can_share: false,
            can_borrow: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HandleMetadata {
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
    pub creator_component: ComponentInstanceId,
    pub tags: BoundedVec<String, 16>,
    pub custom_data: BoundedHashMap<String, ComponentValue, 32>,
}

#[derive(Debug, Clone)]
pub enum HandleOperation {
    Read { fields: BoundedVec<String, 16> },
    Write { fields: BoundedVec<(String, ComponentValue), 16> },
    Call { method: String, args: BoundedVec<ComponentValue, 16> },
    Drop,
    Share { target_component: ComponentInstanceId },
    Borrow { mutable: bool },
    Return { from_borrow: bool },
}

#[derive(Debug, Clone)]
pub struct HandleAccessPolicy {
    pub component_id: ComponentInstanceId,
    pub resource_type: TypeId,
    pub allowed_operations: BoundedVec<HandleOperation, 32>,
    pub required_capability: Option<Capability>,
    pub expiry: Option<u64>,
}

pub struct HandleRepresentationManager {
    representations:
        BoundedHashMap<ResourceHandle, HandleRepresentation, MAX_HANDLE_REPRESENTATIONS>,
    metadata: BoundedHashMap<ResourceHandle, HandleMetadata, MAX_HANDLE_METADATA>,
    access_policies: BoundedVec<HandleAccessPolicy, MAX_ACCESS_POLICIES>,
    type_registry: GenerativeTypeRegistry,
    bounds_checker: TypeBoundsChecker,
    virt_manager: Option<VirtualizationManager>,
    next_handle_id: AtomicU32,
    strict_type_checking: AtomicBool,
}

impl HandleRepresentationManager {
    pub fn new() -> HandleRepresentationResult<Self> {
        Ok(Self {
            representations: BoundedHashMap::new(),
            metadata: BoundedHashMap::new(),
            access_policies: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            type_registry: GenerativeTypeRegistry::new(),
            bounds_checker: TypeBoundsChecker::new(),
            virt_manager: None,
            next_handle_id: AtomicU32::new(1),
            strict_type_checking: AtomicBool::new(true),
        })
    }

    pub fn with_virtualization(mut self, virt_manager: VirtualizationManager) -> Self {
        self.virt_manager = Some(virt_manager);
        self
    }

    pub fn set_strict_type_checking(&self, strict: bool) {
        self.strict_type_checking.store(strict, Ordering::SeqCst);
    }

    pub fn create_handle(
        &mut self,
        component_id: ComponentInstanceId,
        resource_type: GenerativeResourceType,
        access_rights: AccessRights,
    ) -> HandleRepresentationResult<ResourceHandle> {
        let handle_id = self.next_handle_id.fetch_add(1, Ordering::SeqCst);
        let handle = ResourceHandle::new(handle_id);

        let representation = HandleRepresentation {
            handle,
            type_id: resource_type.type_id,
            component_id,
            access_rights,
            is_owned: true,
            reference_count: 1,
        };

        self.representations.insert(handle, representation).map_err(|_| {
            HandleRepresentationError {
                kind: HandleRepresentationErrorKind::ResourceExhausted,
                message: "Too many handle representations".to_string(),
                handle: Some(handle),
            }
        })?;

        // Create metadata
        let metadata = HandleMetadata {
            created_at: self.get_current_time(),
            last_accessed: self.get_current_time(),
            access_count: 0,
            creator_component: component_id,
            tags: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            custom_data: BoundedHashMap::new(),
        };

        self.metadata.insert(handle, metadata).map_err(|_| HandleRepresentationError {
            kind: HandleRepresentationErrorKind::ResourceExhausted,
            message: "Too many handle metadata entries".to_string(),
            handle: Some(handle),
        })?;

        // Map handle to resource type
        self.type_registry.map_resource_handle(handle, resource_type).map_err(|e| {
            HandleRepresentationError {
                kind: HandleRepresentationErrorKind::ValidationFailed,
                message: "Handle operation failed".to_string(),
                handle: Some(handle),
            }
        })?;

        Ok(handle)
    }

    pub fn get_representation(
        &self,
        handle: ResourceHandle,
    ) -> HandleRepresentationResult<&HandleRepresentation> {
        self.representations.get(&handle).ok_or_else(|| HandleRepresentationError {
            kind: HandleRepresentationErrorKind::HandleNotFound,
            message: "Component operation error".to_string(),
            handle: Some(handle),
        })
    }

    pub fn perform_operation(
        &mut self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
        operation: HandleOperation,
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        // Check if handle exists and get representation
        let representation = self.get_representation(handle)?;

        // Verify component has access
        self.verify_access(component_id, handle, &operation)?;

        // Update metadata
        if let Some(metadata) = self.metadata.get_mut(&handle) {
            metadata.last_accessed = self.get_current_time();
            metadata.access_count = metadata.access_count.saturating_add(1);
        }

        // Perform the operation
        match operation {
            HandleOperation::Read { fields } => self.handle_read_operation(handle, &fields),
            HandleOperation::Write { fields } => self.handle_write_operation(handle, &fields),
            HandleOperation::Call { method, args } => {
                self.handle_call_operation(handle, &method, &args)
            }
            HandleOperation::Drop => self.handle_drop_operation(component_id, handle),
            HandleOperation::Share { target_component } => {
                self.handle_share_operation(component_id, handle, target_component)
            }
            HandleOperation::Borrow { mutable } => {
                self.handle_borrow_operation(component_id, handle, mutable)
            }
            HandleOperation::Return { from_borrow } => {
                self.handle_return_operation(component_id, handle, from_borrow)
            }
        }
    }

    pub fn add_access_policy(
        &mut self,
        policy: HandleAccessPolicy,
    ) -> HandleRepresentationResult<()> {
        self.access_policies.push(policy).map_err(|_| HandleRepresentationError {
            kind: HandleRepresentationErrorKind::ResourceExhausted,
            message: "Too many access policies".to_string(),
            handle: None,
        })
    }

    pub fn share_handle(
        &mut self,
        source_component: ComponentInstanceId,
        target_component: ComponentInstanceId,
        handle: ResourceHandle,
        new_access_rights: AccessRights,
    ) -> HandleRepresentationResult<ResourceHandle> {
        // Verify source has share permission
        let operation = HandleOperation::Share { target_component };
        self.verify_access(source_component, handle, &operation)?;

        // Get original representation
        let original = self.get_representation(handle)?.clone();

        // Create new handle for target component
        let new_handle_id = self.next_handle_id.fetch_add(1, Ordering::SeqCst);
        let new_handle = ResourceHandle::new(new_handle_id);

        // Create shared representation
        let shared_representation = HandleRepresentation {
            handle: new_handle,
            type_id: original.type_id,
            component_id: target_component,
            access_rights: new_access_rights,
            is_owned: false,
            reference_count: 1,
        };

        self.representations.insert(new_handle, shared_representation).map_err(|_| {
            HandleRepresentationError {
                kind: HandleRepresentationErrorKind::ResourceExhausted,
                message: "Too many handle representations".to_string(),
                handle: Some(new_handle),
            }
        })?;

        // Copy metadata with updated info
        if let Some(original_metadata) = self.metadata.get(&handle) {
            let mut shared_metadata = original_metadata.clone();
            shared_metadata.tags.push("Component operation error".to_string()).ok();

            self.metadata.insert(new_handle, shared_metadata).map_err(|_| {
                HandleRepresentationError {
                    kind: HandleRepresentationErrorKind::ResourceExhausted,
                    message: "Too many metadata entries".to_string(),
                    handle: Some(new_handle),
                }
            })?;
        }

        // Increment reference count on original
        if let Some(original_repr) = self.representations.get_mut(&handle) {
            original_repr.reference_count = original_repr.reference_count.saturating_add(1);
        }

        Ok(new_handle)
    }

    pub fn drop_handle(
        &mut self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
    ) -> HandleRepresentationResult<()> {
        // Verify component can drop
        let operation = HandleOperation::Drop;
        self.verify_access(component_id, handle, &operation)?;

        // Get representation
        let representation =
            self.representations.get_mut(&handle).ok_or_else(|| HandleRepresentationError {
                kind: HandleRepresentationErrorKind::HandleNotFound,
                message: "Component operation error".to_string(),
                handle: Some(handle),
            })?;

        // Decrement reference count
        representation.reference_count = representation.reference_count.saturating_sub(1);

        // If reference count reaches zero, actually drop
        if representation.reference_count == 0 {
            self.representations.remove(&handle);
            self.metadata.remove(&handle);

            // Unmap from type registry
            if let Err(e) = self.type_registry.unmap_resource_handle(handle) {
                // Log error but don't fail the drop
                #[cfg(feature = "std")]
                eprintln!("Warning: Failed to unmap handle from type registry: {}", e);
                #[cfg(not(feature = "std"))]
                {
                    // In no_std, we can't print to stderr, so we silently ignore the error
                    let _ = e;
                }
            }
        }

        Ok(()
    }

    pub fn get_handle_metadata(&self, handle: ResourceHandle) -> Option<&HandleMetadata> {
        self.metadata.get(&handle)
    }

    pub fn update_handle_metadata<F>(
        &mut self,
        handle: ResourceHandle,
        updater: F,
    ) -> HandleRepresentationResult<()>
    where
        F: FnOnce(&mut HandleMetadata),
    {
        let metadata = self.metadata.get_mut(&handle).ok_or_else(|| HandleRepresentationError {
            kind: HandleRepresentationErrorKind::HandleNotFound,
            message: "Component operation error".to_string(),
            handle: Some(handle),
        })?;

        updater(metadata);
        Ok(()
    }

    pub fn validate_handle_type(
        &self,
        handle: ResourceHandle,
        expected_type: TypeId,
    ) -> HandleRepresentationResult<()> {
        let representation = self.get_representation(handle)?;

        if self.strict_type_checking.load(Ordering::Acquire) {
            // Strict checking - must be exact match or subtype
            if representation.type_id != expected_type {
                // Check if it's a valid subtype
                if !self.bounds_checker.is_subtype(representation.type_id, expected_type) {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::TypeMismatch,
                        message: format!(
                            "Handle type {} does not match expected type {}",
                            representation.type_id.0, expected_type.0
                        ),
                        handle: Some(handle),
                    });
                }
            }
        }

        Ok(()
    }

    fn verify_access(
        &self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
        operation: &HandleOperation,
    ) -> HandleRepresentationResult<()> {
        let representation = self.get_representation(handle)?;

        // Check if component owns or has access to the handle
        if representation.component_id != component_id
            && !self.has_shared_access(component_id, handle)
        {
            return Err(HandleRepresentationError {
                kind: HandleRepresentationErrorKind::AccessDenied,
                message: "Component operation error".to_string(),
                handle: Some(handle),
            });
        }

        // Check specific operation permissions
        match operation {
            HandleOperation::Read { .. } => {
                if !representation.access_rights.can_read {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::AccessDenied,
                        message: "Read access denied".to_string(),
                        handle: Some(handle),
                    });
                }
            }
            HandleOperation::Write { .. } => {
                if !representation.access_rights.can_write {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::AccessDenied,
                        message: "Write access denied".to_string(),
                        handle: Some(handle),
                    });
                }
            }
            HandleOperation::Drop => {
                if !representation.access_rights.can_drop {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::AccessDenied,
                        message: "Drop access denied".to_string(),
                        handle: Some(handle),
                    });
                }
            }
            HandleOperation::Share { .. } => {
                if !representation.access_rights.can_share {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::AccessDenied,
                        message: "Share access denied".to_string(),
                        handle: Some(handle),
                    });
                }
            }
            HandleOperation::Borrow { .. } => {
                if !representation.access_rights.can_borrow {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::AccessDenied,
                        message: "Borrow access denied".to_string(),
                        handle: Some(handle),
                    });
                }
            }
            _ => {}
        }

        // Check access policies
        self.check_access_policies(component_id, handle, operation)?;

        // Check virtualization capabilities if enabled
        if let Some(ref virt_manager) = self.virt_manager {
            self.check_virtualization_capabilities(component_id, operation, virt_manager)?;
        }

        Ok(()
    }

    fn has_shared_access(&self, component_id: ComponentInstanceId, handle: ResourceHandle) -> bool {
        // Check if there's any handle representation for this component that references the same resource
        self.representations.iter().any(|(h, repr)| {
            repr.component_id == component_id
                && repr.type_id
                    == self.get_representation(handle).map(|r| r.type_id).unwrap_or_default()
        })
    }

    fn check_access_policies(
        &self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
        operation: &HandleOperation,
    ) -> HandleRepresentationResult<()> {
        let representation = self.get_representation(handle)?;
        let current_time = self.get_current_time();

        for policy in self.access_policies.iter() {
            if policy.component_id == component_id && policy.resource_type == representation.type_id
            {
                // Check expiry
                if let Some(expiry) = policy.expiry {
                    if current_time > expiry {
                        continue;
                    }
                }

                // Check if operation is allowed
                let operation_allowed = policy.allowed_operations.iter().any(|allowed_op| {
                    matches!(
                        (allowed_op, operation),
                        (HandleOperation::Read { .. }, HandleOperation::Read { .. })
                            | (HandleOperation::Write { .. }, HandleOperation::Write { .. })
                            | (HandleOperation::Drop, HandleOperation::Drop)
                            | (HandleOperation::Share { .. }, HandleOperation::Share { .. })
                            | (HandleOperation::Borrow { .. }, HandleOperation::Borrow { .. })
                    )
                });

                if !operation_allowed {
                    return Err(HandleRepresentationError {
                        kind: HandleRepresentationErrorKind::OperationNotSupported,
                        message: "Operation not allowed by policy".to_string(),
                        handle: Some(handle),
                    });
                }
            }
        }

        Ok(()
    }

    fn check_virtualization_capabilities(
        &self,
        component_id: ComponentInstanceId,
        operation: &HandleOperation,
        virt_manager: &VirtualizationManager,
    ) -> HandleRepresentationResult<()> {
        // For certain operations, check if component has required capabilities
        match operation {
            HandleOperation::Call { .. } => {
                // May require specific capability for external calls
                // This is a simplified check - real implementation would be more sophisticated
                Ok(()
            }
            _ => Ok(()),
        }
    }

    fn handle_read_operation(
        &self,
        handle: ResourceHandle,
        fields: &[String],
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        // This is a placeholder - actual implementation would read from the resource
        Ok(Some(ComponentValue::I32(42))
    }

    fn handle_write_operation(
        &mut self,
        handle: ResourceHandle,
        fields: &[(String, ComponentValue)],
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        // This is a placeholder - actual implementation would write to the resource
        Ok(None)
    }

    fn handle_call_operation(
        &mut self,
        handle: ResourceHandle,
        method: &str,
        args: &[ComponentValue],
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        // This is a placeholder - actual implementation would call the method
        Ok(Some(ComponentValue::String("Component operation error".to_string()))
    }

    fn handle_drop_operation(
        &mut self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        self.drop_handle(component_id, handle)?;
        Ok(None)
    }

    fn handle_share_operation(
        &mut self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
        target_component: ComponentInstanceId,
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        let new_handle =
            self.share_handle(component_id, target_component, handle, AccessRights::read_only())?;

        Ok(Some(ComponentValue::U32(new_handle.id()))
    }

    fn handle_borrow_operation(
        &mut self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
        mutable: bool,
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        // This is a placeholder - actual implementation would create a borrowed reference
        Ok(Some(ComponentValue::Bool(true))
    }

    fn handle_return_operation(
        &mut self,
        component_id: ComponentInstanceId,
        handle: ResourceHandle,
        from_borrow: bool,
    ) -> HandleRepresentationResult<Option<ComponentValue>> {
        // This is a placeholder - actual implementation would return a borrowed reference
        Ok(None)
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

impl Default for HandleRepresentationManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default HandleRepresentationManagerMissing message")
    }
}

pub struct TypedHandle<T> {
    handle: ResourceHandle,
    type_id: TypeId,
    _phantom: PhantomData<T>,
}

impl<T> TypedHandle<T> {
    pub fn new(handle: ResourceHandle, type_id: TypeId) -> Self {
        Self { handle, type_id, _phantom: PhantomData }
    }

    pub fn handle(&self) -> ResourceHandle {
        self.handle
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

impl<T> Clone for TypedHandle<T> {
    fn clone(&self) -> Self {
        Self { handle: self.handle, type_id: self.type_id, _phantom: PhantomData }
    }
}

impl<T> Copy for TypedHandle<T> {}

pub fn create_access_rights(
    read: bool,
    write: bool,
    drop: bool,
    share: bool,
    borrow: bool,
) -> AccessRights {
    AccessRights {
        can_read: read,
        can_write: write,
        can_drop: drop,
        can_share: share,
        can_borrow: borrow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_representation_manager_creation() {
        let manager = HandleRepresentationManager::new().unwrap();
        assert!(manager.strict_type_checking.load(Ordering::Acquire);
    }

    #[test]
    fn test_access_rights_presets() {
        let read_only = AccessRights::read_only();
        assert!(read_only.can_read);
        assert!(!read_only.can_write);
        assert!(!read_only.can_drop);

        let full_access = AccessRights::full_access();
        assert!(full_access.can_read);
        assert!(full_access.can_write);
        assert!(full_access.can_drop);
        assert!(full_access.can_share);

        let no_access = AccessRights::no_access();
        assert!(!no_access.can_read);
        assert!(!no_access.can_write);
        assert!(!no_access.can_drop);
    }

    #[test]
    fn test_handle_creation() {
        let mut manager = HandleRepresentationManager::new().unwrap();
        let component_id = ComponentInstanceId::new(1);

        let resource_type =
            manager.type_registry.create_resource_type(component_id, "test-resourceMissing message").unwrap();

        let handle = manager
            .create_handle(component_id, resource_type, AccessRights::full_access()
            .unwrap();

        assert!(handle.id() > 0);

        // Verify representation was created
        let repr = manager.get_representation(handle).unwrap();
        assert_eq!(repr.component_id, component_id);
        assert_eq!(repr.type_id, resource_type.type_id);
        assert!(repr.is_owned);
        assert_eq!(repr.reference_count, 1);
    }

    #[test]
    fn test_typed_handle() {
        struct MyResource;

        let handle = ResourceHandle::new(42);
        let type_id = TypeId(100);

        let typed_handle = TypedHandle::<MyResource>::new(handle, type_id);
        assert_eq!(typed_handle.handle().id(), 42);
        assert_eq!(typed_handle.type_id().0, 100);
    }
}
