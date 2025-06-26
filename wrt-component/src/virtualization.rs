use crate::{
    canonical_abi::canonical_options::CanonicalOptions, 
    post_return::PostReturnRegistry, 
    components::component_instantiation::ComponentInstance,
};

// Placeholder types
pub use crate::types::ComponentInstanceId;
pub type ResourceHandle = u32;
pub type ValType = u32;
use core::{
    fmt,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    safe_memory::{SafeMemory, NoStdProvider},
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;

const MAX_VIRTUAL_COMPONENTS: usize = 256;
const MAX_VIRTUAL_IMPORTS: usize = 1024;
const MAX_VIRTUAL_EXPORTS: usize = 1024;
const MAX_CAPABILITY_GRANTS: usize = 512;
const MAX_VIRTUAL_MEMORY_REGIONS: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualizationError {
    pub kind: VirtualizationErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VirtualizationErrorKind {
    CapabilityDenied,
    ResourceExhaustion,
    InvalidVirtualComponent,
    MemoryViolation,
    ImportNotFound,
    ExportConflict,
    VirtualizationNotSupported,
}

impl fmt::Display for VirtualizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VirtualizationError {}

// Conversion to wrt_error::Error for unified error handling
impl From<VirtualizationError> for wrt_error::Error {
    fn from(err: VirtualizationError) -> Self {
        use wrt_error::{ErrorCategory, codes};
        match err.kind {
            VirtualizationErrorKind::CapabilityDenied => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_CAPABILITY_DENIED,
                "Virtualization capability denied",
            ),
            VirtualizationErrorKind::ResourceExhaustion => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_RESOURCE_LIFECYCLE_ERROR,
                "Virtualization resource exhausted",
            ),
            VirtualizationErrorKind::InvalidVirtualComponent => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_VIRTUALIZATION_ERROR,
                "Invalid virtual component",
            ),
            VirtualizationErrorKind::MemoryViolation => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_VIRTUALIZATION_ERROR,
                "Virtualization memory violation",
            ),
            VirtualizationErrorKind::ImportNotFound => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_INSTANTIATION_RUNTIME_ERROR,
                "Virtual import not found",
            ),
            VirtualizationErrorKind::ExportConflict => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_INSTANTIATION_RUNTIME_ERROR,
                "Virtual export conflict",
            ),
            VirtualizationErrorKind::VirtualizationNotSupported => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_VIRTUALIZATION_ERROR,
                "Virtualization not supported",
            ),
        }
    }
}

pub type VirtualizationResult<T> = wrt_error::Result<T>;

#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    Memory { max_size: usize },
    FileSystem { read_only: bool, path_prefix: Option<String> },
    Network { allowed_hosts: BoundedVec<String, 32, NoStdProvider<65536>> },
    Time { precision_ms: u64 },
    Random,
    Threading { max_threads: u32 },
    Logging { max_level: LogLevel },
    Custom { name: String, data: BoundedVec<u8, 256, NoStdProvider<65536>> },
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

#[derive(Debug, Clone)]
pub struct CapabilityGrant {
    pub capability: Capability,
    pub granted_to: ComponentInstanceId,
    pub granted_at: u64,
    pub expires_at: Option<u64>,
    pub revocable: bool,
}

#[derive(Debug, Clone)]
pub struct VirtualComponent {
    pub instance_id: ComponentInstanceId,
    pub name: String,
    pub parent: Option<ComponentInstanceId>,
    pub children: BoundedVec<ComponentInstanceId, MAX_VIRTUAL_COMPONENTS, NoStdProvider<65536>>,
    pub capabilities: BoundedVec<Capability, MAX_CAPABILITY_GRANTS, NoStdProvider<65536>>,
    pub virtual_imports: BoundedHashMap<String, VirtualImport, MAX_VIRTUAL_IMPORTS>,
    pub virtual_exports: BoundedHashMap<String, VirtualExport, MAX_VIRTUAL_EXPORTS>,
    pub memory_regions: BoundedVec<VirtualMemoryRegion, MAX_VIRTUAL_MEMORY_REGIONS, NoStdProvider<65536>>,
    pub isolation_level: IsolationLevel,
    pub resource_limits: ResourceLimits,
    pub is_sandboxed: bool,
}

#[derive(Debug, Clone)]
pub struct VirtualImport {
    pub name: String,
    pub val_type: ValType,
    pub required: bool,
    pub virtual_source: Option<VirtualSource>,
    pub capability_required: Option<Capability>,
}

#[derive(Debug, Clone)]
pub struct VirtualExport {
    pub name: String,
    pub val_type: ValType,
    pub visibility: ExportVisibility,
    pub capability_required: Option<Capability>,
}

#[derive(Debug, Clone)]
pub enum VirtualSource {
    HostFunction { name: String },
    ParentComponent { export_name: String },
    SiblingComponent { instance_id: ComponentInstanceId, export_name: String },
    VirtualProvider { provider_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportVisibility {
    Public,
    Parent,
    Children,
    Siblings,
    Private,
}

#[derive(Debug, Clone)]
pub struct VirtualMemoryRegion {
    pub start_addr: usize,
    pub size: usize,
    pub permissions: MemoryPermissions,
    pub shared: bool,
    pub mapped_to: Option<ComponentInstanceId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsolationLevel {
    None,
    Basic,
    Strong,
    Complete,
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory: usize,
    pub max_cpu_time_ms: u64,
    pub max_file_handles: u32,
    pub max_network_connections: u32,
    pub max_threads: u32,
    pub max_recursive_calls: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 1024 * 1024,
            max_cpu_time_ms: 5000,
            max_file_handles: 10,
            max_network_connections: 5,
            max_threads: 1,
            max_recursive_calls: 100,
        }
    }
}

pub struct VirtualizationManager {
    virtual_components:
        BoundedHashMap<ComponentInstanceId, VirtualComponent, MAX_VIRTUAL_COMPONENTS>,
    capability_grants: BoundedVec<CapabilityGrant, MAX_CAPABILITY_GRANTS, NoStdProvider<65536>>,
    host_exports: BoundedHashMap<String, HostExport, MAX_VIRTUAL_EXPORTS>,
    sandbox_registry: BoundedHashMap<ComponentInstanceId, SandboxState, MAX_VIRTUAL_COMPONENTS>,
    next_virtual_id: AtomicU32,
    virtualization_enabled: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct HostExport {
    pub name: String,
    pub val_type: ValType,
    pub handler: HostExportHandler,
    pub required_capability: Option<Capability>,
}

#[derive(Debug, Clone)]
pub enum HostExportHandler {
    Memory { allocator: String },
    FileSystem { base_path: String },
    Network { endpoint: String },
    Time,
    Random,
    Logging { destination: String },
    Custom { handler_id: String },
}

#[derive(Debug, Clone)]
pub struct SandboxState {
    pub instance_id: ComponentInstanceId,
    pub active: bool,
    pub resource_usage: ResourceUsage,
    pub violation_count: u32,
    pub last_violation: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub memory_used: usize,
    pub cpu_time_used_ms: u64,
    pub file_handles_used: u32,
    pub network_connections_used: u32,
    pub threads_used: u32,
    pub recursive_calls_depth: u32,
}

impl VirtualizationManager {
    pub fn new() -> VirtualizationResult<Self> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let capability_grants = BoundedVec::new(provider).map_err(|_| VirtualizationError {
            kind: VirtualizationErrorKind::ResourceExhaustion,
            message: "Failed to create capability grants storage".to_string(),
        })?;

        Ok(Self {
            virtual_components: BoundedHashMap::new(),
            capability_grants,
            host_exports: BoundedHashMap::new(),
            sandbox_registry: BoundedHashMap::new(),
            next_virtual_id: AtomicU32::new(1000),
            virtualization_enabled: AtomicBool::new(true),
        })
    }

    pub fn enable_virtualization(&self) {
        self.virtualization_enabled.store(true, Ordering::SeqCst);
    }

    pub fn disable_virtualization(&self) {
        self.virtualization_enabled.store(false, Ordering::SeqCst);
    }

    pub fn is_virtualization_enabled(&self) -> bool {
        self.virtualization_enabled.load(Ordering::SeqCst)
    }

    pub fn create_virtual_component(
        &mut self,
        name: &str,
        parent: Option<ComponentInstanceId>,
        isolation_level: IsolationLevel,
    ) -> VirtualizationResult<ComponentInstanceId> {
        if !self.is_virtualization_enabled() {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::VirtualizationNotSupported,
                message: "Virtualization is disabled".to_string(),
            });
        }

        let instance_id =
            ComponentInstanceId::new(self.next_virtual_id.fetch_add(1, Ordering::SeqCst));

        let virtual_component = VirtualComponent {
            instance_id,
            name: name.to_string(),
            parent,
            children: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| VirtualizationError {
                    kind: VirtualizationErrorKind::ResourceExhaustion,
                    message: "Failed to create children storage".to_string(),
                })?
            },
            capabilities: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| VirtualizationError {
                    kind: VirtualizationErrorKind::ResourceExhaustion,
                    message: "Failed to create capabilities storage".to_string(),
                })?
            },
            virtual_imports: BoundedHashMap::new(),
            virtual_exports: BoundedHashMap::new(),
            memory_regions: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| VirtualizationError {
                    kind: VirtualizationErrorKind::ResourceExhaustion,
                    message: "Failed to create memory regions storage".to_string(),
                })?
            },
            isolation_level,
            resource_limits: ResourceLimits::default(),
            is_sandboxed: isolation_level != IsolationLevel::None,
        };

        if let Some(parent_id) = parent {
            if let Some(parent_component) = self.virtual_components.get_mut(&parent_id) {
                parent_component.children.push(instance_id).map_err(|_| VirtualizationError {
                    kind: VirtualizationErrorKind::ResourceExhaustion,
                    message: "Parent component has too many children".to_string(),
                })?;
            }
        }

        self.virtual_components.insert(instance_id, virtual_component).map_err(|_| {
            VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "Too many virtual components".to_string(),
            }
        })?;

        if isolation_level != IsolationLevel::None {
            let sandbox_state = SandboxState {
                instance_id,
                active: true,
                resource_usage: ResourceUsage::default(),
                violation_count: 0,
                last_violation: None,
            };
            self.sandbox_registry.insert(instance_id, sandbox_state).map_err(|_| {
                VirtualizationError {
                    kind: VirtualizationErrorKind::ResourceExhaustion,
                    message: "Too many sandboxed components".to_string(),
                }
            })?;
        }

        Ok(instance_id)
    }

    pub fn grant_capability(
        &mut self,
        instance_id: ComponentInstanceId,
        capability: Capability,
        expires_at: Option<u64>,
        revocable: bool,
    ) -> VirtualizationResult<()> {
        if !self.virtual_components.contains_key(&instance_id) {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::InvalidVirtualComponent,
                message: "Component not found".to_string(),
            });
        }

        let grant = CapabilityGrant {
            capability: capability.clone(),
            granted_to: instance_id,
            granted_at: self.get_current_time(),
            expires_at,
            revocable,
        };

        self.capability_grants.push(grant).map_err(|_| VirtualizationError {
            kind: VirtualizationErrorKind::ResourceExhaustion,
            message: "Too many capability grants".to_string(),
        })?;

        if let Some(component) = self.virtual_components.get_mut(&instance_id) {
            component.capabilities.push(capability).map_err(|_| VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "Component has too many capabilities".to_string(),
            })?;
        }

        Ok(())
    }

    pub fn check_capability(
        &self,
        instance_id: ComponentInstanceId,
        capability: &Capability,
    ) -> bool {
        if let Some(component) = self.virtual_components.get(&instance_id) {
            component.capabilities.iter().any(|cap| self.capability_matches(cap, capability))
        } else {
            false
        }
    }

    pub fn add_virtual_import(
        &mut self,
        instance_id: ComponentInstanceId,
        import: VirtualImport,
    ) -> VirtualizationResult<()> {
        let component =
            self.virtual_components.get_mut(&instance_id).ok_or_else(|| VirtualizationError {
                kind: VirtualizationErrorKind::InvalidVirtualComponent,
                message: "Component not found".to_string(),
            })?;

        let import_name = import.name.clone();
        component.virtual_imports.insert(import_name, import).map_err(|_| VirtualizationError {
            kind: VirtualizationErrorKind::ResourceExhaustion,
            message: "Too many virtual imports".to_string(),
        })?;

        Ok(())
    }

    pub fn add_virtual_export(
        &mut self,
        instance_id: ComponentInstanceId,
        export: VirtualExport,
    ) -> VirtualizationResult<()> {
        let component =
            self.virtual_components.get_mut(&instance_id).ok_or_else(|| VirtualizationError {
                kind: VirtualizationErrorKind::InvalidVirtualComponent,
                message: "Component not found".to_string(),
            })?;

        let export_name = export.name.clone();
        component.virtual_exports.insert(export_name, export).map_err(|_| VirtualizationError {
            kind: VirtualizationErrorKind::ExportConflict,
            message: "Export already exists or too many exports".to_string(),
        })?;

        Ok(())
    }

    pub fn allocate_virtual_memory(
        &mut self,
        instance_id: ComponentInstanceId,
        size: usize,
        permissions: MemoryPermissions,
    ) -> VirtualizationResult<usize> {
        let component =
            self.virtual_components.get_mut(&instance_id).ok_or_else(|| VirtualizationError {
                kind: VirtualizationErrorKind::InvalidVirtualComponent,
                message: "Component not found".to_string(),
            })?;

        if component.isolation_level == IsolationLevel::None {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::MemoryViolation,
                message: "Virtual memory not available for non-isolated components".to_string(),
            });
        }

        if !self.check_capability(instance_id, &Capability::Memory { max_size: size }) {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::CapabilityDenied,
                message: "Insufficient memory capability".to_string(),
            });
        }

        let current_usage =
            component.memory_regions.iter().map(|region| region.size).sum::<usize>();

        if current_usage + size > component.resource_limits.max_memory {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "Memory limit exceeded".to_string(),
            });
        }

        let start_addr = self.find_virtual_address_space(size)?;

        let memory_region =
            VirtualMemoryRegion { start_addr, size, permissions, shared: false, mapped_to: None };

        component.memory_regions.push(memory_region).map_err(|_| VirtualizationError {
            kind: VirtualizationErrorKind::ResourceExhaustion,
            message: "Too many memory regions".to_string(),
        })?;

        Ok(start_addr)
    }

    pub fn resolve_virtual_import(
        &self,
        instance_id: ComponentInstanceId,
        import_name: &str,
    ) -> VirtualizationResult<Option<ComponentValue>> {
        let component =
            self.virtual_components.get(&instance_id).ok_or_else(|| VirtualizationError {
                kind: VirtualizationErrorKind::InvalidVirtualComponent,
                message: "Component not found".to_string(),
            })?;

        let import =
            component.virtual_imports.get(import_name).ok_or_else(|| VirtualizationError {
                kind: VirtualizationErrorKind::ImportNotFound,
                message: "Component not found",
            })?;

        if let Some(ref capability) = import.capability_required {
            if !self.check_capability(instance_id, capability) {
                return Err(VirtualizationError {
                    kind: VirtualizationErrorKind::CapabilityDenied,
                    message: "Component not found",
                });
            }
        }

        match &import.virtual_source {
            Some(VirtualSource::HostFunction { name }) => self.resolve_host_function(name),
            Some(VirtualSource::ParentComponent { export_name }) => {
                if let Some(parent_id) = component.parent {
                    self.resolve_parent_export(parent_id, export_name)
                } else {
                    Ok(None)
                }
            }
            Some(VirtualSource::SiblingComponent { instance_id: sibling_id, export_name }) => {
                self.resolve_sibling_export(*sibling_id, export_name)
            }
            Some(VirtualSource::VirtualProvider { provider_id }) => {
                self.resolve_virtual_provider(provider_id)
            }
            None => Ok(None),
        }
    }

    pub fn update_resource_usage(
        &mut self,
        instance_id: ComponentInstanceId,
        usage_update: ResourceUsage,
    ) -> VirtualizationResult<()> {
        if let Some(sandbox_state) = self.sandbox_registry.get_mut(&instance_id) {
            sandbox_state.resource_usage = usage_update;

            if let Some(component) = self.virtual_components.get(&instance_id) {
                self.check_resource_limits(component, &sandbox_state.resource_usage)?;
            }
        }
        Ok(())
    }

    fn capability_matches(&self, granted: &Capability, requested: &Capability) -> bool {
        match (granted, requested) {
            (
                Capability::Memory { max_size: granted_size },
                Capability::Memory { max_size: requested_size },
            ) => granted_size >= requested_size,
            (
                Capability::Threading { max_threads: granted },
                Capability::Threading { max_threads: requested },
            ) => granted >= requested,
            (Capability::Random, Capability::Random) => true,
            (Capability::Time { .. }, Capability::Time { .. }) => true,
            (a, b) => a == b,
        }
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

    fn find_virtual_address_space(&self, size: usize) -> VirtualizationResult<usize> {
        let base_addr = 0x10000000;
        Ok(base_addr)
    }

    fn resolve_host_function(&self, name: &str) -> VirtualizationResult<Option<ComponentValue>> {
        if let Some(export) = self.host_exports.get(name) {
            match &export.handler {
                HostExportHandler::Memory { .. } => Ok(Some(ComponentValue::U32(0))),
                HostExportHandler::Time => Ok(Some(ComponentValue::U64(self.get_current_time()))),
                HostExportHandler::Random => Ok(Some(ComponentValue::U32(42))),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn resolve_parent_export(
        &self,
        parent_id: ComponentInstanceId,
        export_name: &str,
    ) -> VirtualizationResult<Option<ComponentValue>> {
        if let Some(parent) = self.virtual_components.get(&parent_id) {
            if let Some(export) = parent.virtual_exports.get(export_name) {
                match export.visibility {
                    ExportVisibility::Public | ExportVisibility::Children => Ok(None),
                    _ => Err(VirtualizationError {
                        kind: VirtualizationErrorKind::CapabilityDenied,
                        message: "Export not visible to children".to_string(),
                    }),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn resolve_sibling_export(
        &self,
        sibling_id: ComponentInstanceId,
        export_name: &str,
    ) -> VirtualizationResult<Option<ComponentValue>> {
        if let Some(sibling) = self.virtual_components.get(&sibling_id) {
            if let Some(export) = sibling.virtual_exports.get(export_name) {
                match export.visibility {
                    ExportVisibility::Public | ExportVisibility::Siblings => Ok(None),
                    _ => Err(VirtualizationError {
                        kind: VirtualizationErrorKind::CapabilityDenied,
                        message: "Export not visible to siblings".to_string(),
                    }),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn resolve_virtual_provider(
        &self,
        provider_id: &str,
    ) -> VirtualizationResult<Option<ComponentValue>> {
        Ok(None)
    }

    fn check_resource_limits(
        &self,
        component: &VirtualComponent,
        usage: &ResourceUsage,
    ) -> VirtualizationResult<()> {
        let limits = &component.resource_limits;

        if usage.memory_used > limits.max_memory {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "Memory limit exceeded".to_string(),
            });
        }

        if usage.cpu_time_used_ms > limits.max_cpu_time_ms {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "CPU time limit exceeded".to_string(),
            });
        }

        if usage.threads_used > limits.max_threads {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "Thread limit exceeded".to_string(),
            });
        }

        if usage.recursive_calls_depth > limits.max_recursive_calls {
            return Err(VirtualizationError {
                kind: VirtualizationErrorKind::ResourceExhaustion,
                message: "Recursion limit exceeded".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for VirtualizationManager {
    fn default() -> Self {
        // Use new() which properly handles allocation or panic in development
        Self::new().expect("VirtualizationManager allocation should not fail in default construction")
    }
}

pub fn create_memory_capability(max_size: usize) -> Capability {
    Capability::Memory { max_size }
}

pub fn create_network_capability(allowed_hosts: &[&str]) -> VirtualizationResult<Capability> {
    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
    let mut hosts = BoundedVec::new(provider).map_err(|_| VirtualizationError {
        kind: VirtualizationErrorKind::ResourceExhaustion,
        message: "Failed to create network hosts storage".to_string(),
    })?;
    for host in allowed_hosts {
        hosts.push(host.to_string()).map_err(|_| VirtualizationError {
            kind: VirtualizationErrorKind::ResourceExhaustion,
            message: "Too many allowed hosts".to_string(),
        })?;
    }
    Ok(Capability::Network { allowed_hosts: hosts })
}

pub fn create_threading_capability(max_threads: u32) -> Capability {
    Capability::Threading { max_threads }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtualization_manager_creation() {
        let manager = VirtualizationManager::new().unwrap();
        assert!(manager.is_virtualization_enabled());
    }

    #[test]
    fn test_virtual_component_creation() {
        let mut manager = VirtualizationManager::new().unwrap();
        let result =
            manager.create_virtual_component("test-component", None, IsolationLevel::Basic);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_granting() {
        let mut manager = VirtualizationManager::new().unwrap();
        let instance_id = manager
            .create_virtual_component("test-component", None, IsolationLevel::Basic)
            .unwrap();

        let capability = create_memory_capability(1024);
        let result = manager.grant_capability(instance_id, capability.clone(), None, true);
        assert!(result.is_ok());
        assert!(manager.check_capability(instance_id, &capability));
    }

    #[test]
    fn test_virtual_memory_allocation() {
        let mut manager = VirtualizationManager::new().unwrap();
        let instance_id = manager
            .create_virtual_component("test-component", None, IsolationLevel::Strong)
            .unwrap();

        let capability = create_memory_capability(2048);
        manager.grant_capability(instance_id, capability, None, true).unwrap();

        let permissions = MemoryPermissions { read: true, write: true, execute: false };

        let result = manager.allocate_virtual_memory(instance_id, 1024, permissions);
        assert!(result.is_ok());
    }
}
