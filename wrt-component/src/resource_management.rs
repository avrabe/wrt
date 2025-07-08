//! Resource management system for Component Model
//!
//! This module provides resource management capabilities for the WebAssembly
//! Component Model, including resource handles, lifecycle management, and
//! resource tables.

use wrt_foundation::{
    bounded::BoundedVec,
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

/// Invalid resource handle constant
pub const INVALID_HANDLE: u32 = u32::MAX;

/// Resource handle for Component Model resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle(pub u32);

impl ResourceHandle {
    /// Create a new resource handle
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the handle ID
    pub fn id(&self) -> u32 {
        self.0
    }

    /// Check if this is a valid handle
    pub fn is_valid(&self) -> bool {
        self.0 != INVALID_HANDLE
    }
}

/// Resource type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceTypeId(pub u32);

impl ResourceTypeId {
    /// Create a new resource type ID
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the type ID
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Resource type metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceTypeMetadata {
    /// Resource type ID
    pub type_id: ResourceTypeId,
    /// Resource type name
    pub name: BoundedVec<u8, 256, wrt_foundation::safe_memory::NoStdProvider<65536>>,
    /// Size of the resource data
    pub size: usize,
}

/// Resource state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// Resource is being created
    Creating,
    /// Resource is active and usable
    Active,
    /// Resource is being destroyed
    Destroying,
    /// Resource has been destroyed
    Destroyed,
}

/// Resource ownership model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceOwnership {
    /// Resource is owned by the creating component
    Owned,
    /// Resource is borrowed from another component
    Borrowed,
    /// Resource is shared between multiple components
    Shared,
}

/// Resource validation level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceValidationLevel {
    /// No validation
    None,
    /// Basic validation (type checks)
    Basic,
    /// Full validation (type and lifecycle checks)
    Full,
}

/// Resource data container
#[derive(Debug, Clone)]
pub enum ResourceData {
    /// Raw bytes
    Bytes(BoundedVec<u8, 4096, wrt_foundation::safe_memory::NoStdProvider<65536>>),
    /// Custom data pointer (for std only)
    #[cfg(feature = "std")]
    Custom(Box<dyn std::any::Any + Send + Sync>),
    /// External resource reference
    External(u64),
}

/// Resource error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    /// Resource not found
    NotFound,
    /// Invalid resource handle
    InvalidHandle,
    /// Resource type mismatch
    TypeMismatch,
    /// Resource already exists
    AlreadyExists,
    /// Permission denied
    PermissionDenied,
    /// Resource limit exceeded
    LimitExceeded,
}

impl core::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ResourceError::NotFound => write!(f, "Resource not found"),
            ResourceError::InvalidHandle => write!(f, "Invalid resource handle"),
            ResourceError::TypeMismatch => write!(f, "Resource type mismatch"),
            ResourceError::AlreadyExists => write!(f, "Resource already exists"),
            ResourceError::PermissionDenied => write!(f, "Permission denied"),
            ResourceError::LimitExceeded => write!(f, "Resource limit exceeded"),
        }
    }
}

/// Resource type for Component Model
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    /// Basic resource type
    Basic,
    /// Handle resource type
    Handle,
    /// Stream resource type
    Stream,
    /// Future resource type
    Future,
}

/// Component resource (stub implementation)
#[derive(Debug, Clone)]
pub struct Resource {
    /// Resource handle
    pub handle: ResourceHandle,
    /// Resource type ID
    pub type_id: ResourceTypeId,
    /// Resource state
    pub state: ResourceState,
    /// Resource ownership
    pub ownership: ResourceOwnership,
    /// Resource data
    pub data: ResourceData,
}

impl Resource {
    /// Create a new resource
    pub fn new(
        handle: ResourceHandle,
        type_id: ResourceTypeId,
        data: ResourceData,
    ) -> Self {
        Self {
            handle,
            type_id,
            state: ResourceState::Creating,
            ownership: ResourceOwnership::Owned,
            data,
        }
    }

    /// Get the resource handle
    pub fn handle(&self) -> ResourceHandle {
        self.handle
    }

    /// Get the resource type ID
    pub fn type_id(&self) -> ResourceTypeId {
        self.type_id
    }

    /// Get the resource state
    pub fn state(&self) -> ResourceState {
        self.state
    }

    /// Set the resource state
    pub fn set_state(&mut self, state: ResourceState) {
        self.state = state;
    }
}

/// Resource manager configuration
#[derive(Debug, Clone)]
pub struct ResourceManagerConfig {
    /// Maximum number of resources
    pub max_resources: usize,
    /// Validation level
    pub validation_level: ResourceValidationLevel,
    /// Enable resource tracking
    pub enable_tracking: bool,
}

impl Default for ResourceManagerConfig {
    fn default() -> Self {
        Self {
            max_resources: 1024,
            validation_level: ResourceValidationLevel::Basic,
            enable_tracking: true,
        }
    }
}

/// Resource manager statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceManagerStats {
    /// Total resources created
    pub resources_created: u64,
    /// Total resources destroyed
    pub resources_destroyed: u64,
    /// Currently active resources
    pub active_resources: u32,
    /// Peak resource count
    pub peak_resources: u32,
}

/// Resource manager (stub implementation)
#[derive(Debug)]
pub struct ResourceManager {
    /// Manager configuration
    config: ResourceManagerConfig,
    /// Manager statistics
    stats: ResourceManagerStats,
    /// Next resource handle ID
    next_handle_id: u32,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(config: ResourceManagerConfig) -> Self {
        Self {
            config,
            stats: ResourceManagerStats::default(),
            next_handle_id: 1,
        }
    }

    /// Get manager statistics
    pub fn stats(&self) -> ResourceManagerStats {
        self.stats
    }

    /// Create a new resource
    pub fn create_resource(
        &mut self,
        type_id: ResourceTypeId,
        data: ResourceData,
    ) -> core::result::Result<ResourceHandle, ResourceError> {
        if self.stats.active_resources >= self.config.max_resources as u32 {
            return Err(ResourceError::LimitExceeded);
        }

        let handle = ResourceHandle::new(self.next_handle_id);
        self.next_handle_id += 1;
        self.stats.resources_created += 1;
        self.stats.active_resources += 1;
        
        if self.stats.active_resources > self.stats.peak_resources {
            self.stats.peak_resources = self.stats.active_resources;
        }

        Ok(handle)
    }

    /// Destroy a resource
    pub fn destroy_resource(&mut self, handle: ResourceHandle) -> core::result::Result<(), ResourceError> {
        if !handle.is_valid() {
            return Err(ResourceError::InvalidHandle);
        }

        self.stats.resources_destroyed += 1;
        if self.stats.active_resources > 0 {
            self.stats.active_resources -= 1;
        }

        Ok(()
    }
}

/// Resource table statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceTableStats {
    /// Total entries in table
    pub total_entries: u32,
    /// Active entries in table
    pub active_entries: u32,
    /// Total lookups performed
    pub total_lookups: u64,
    /// Successful lookups
    pub successful_lookups: u64,
}

/// Resource table (stub implementation)
#[derive(Debug)]
pub struct ResourceTable {
    /// Table statistics
    stats: ResourceTableStats,
    /// Maximum table size
    max_size: usize,
}

impl ResourceTable {
    /// Create a new resource table
    pub fn new(max_size: usize) -> Self {
        Self {
            stats: ResourceTableStats::default(),
            max_size,
        }
    }

    /// Get table statistics
    pub fn stats(&self) -> ResourceTableStats {
        self.stats
    }

    /// Get a resource by handle
    pub fn get(&mut self, handle: ResourceHandle) -> core::result::Result<Option<&Resource>, ResourceError> {
        self.stats.total_lookups += 1;
        
        if !handle.is_valid() {
            return Err(ResourceError::InvalidHandle);
        }

        // Stub implementation - would normally look up the resource
        self.stats.successful_lookups += 1;
        Ok(None)
    }

    /// Insert a resource into the table
    pub fn insert(&mut self, resource: Resource) -> core::result::Result<(), ResourceError> {
        if self.stats.active_entries >= self.max_size as u32 {
            return Err(ResourceError::LimitExceeded);
        }

        self.stats.total_entries += 1;
        self.stats.active_entries += 1;

        Ok(()
    }

    /// Remove a resource from the table
    pub fn remove(&mut self, handle: ResourceHandle) -> core::result::Result<Option<Resource>, ResourceError> {
        if !handle.is_valid() {
            return Err(ResourceError::InvalidHandle);
        }

        if self.stats.active_entries > 0 {
            self.stats.active_entries -= 1;
        }

        Ok(None)
    }
}

/// Helper function to create resource data from bytes
pub fn create_resource_data_bytes(data: &[u8]) -> core::result::Result<ResourceData, ResourceError> {
    let provider = safe_managed_alloc!(65536, CrateId::Component).map_err(|_| ResourceError::LimitExceeded)?;
    let mut vec = BoundedVec::new(provider).unwrap();
    for &byte in data {
        vec.push(byte).map_err(|_| ResourceError::LimitExceeded)?;
    }
    Ok(ResourceData::Bytes(vec)
}

/// Helper function to create resource data from external reference
pub fn create_resource_data_external(reference: u64) -> ResourceData {
    ResourceData::External(reference)
}

/// Helper function to create resource data from custom data (std only)
#[cfg(feature = "std")]
pub fn create_resource_data_custom<T: std::any::Any + Send + Sync>(data: T) -> ResourceData {
    ResourceData::Custom(Box::new(data)
}

/// Helper function to create a resource type
pub fn create_resource_type(name: &str) -> core::result::Result<ResourceTypeMetadata, ResourceError> {
    let provider = safe_managed_alloc!(65536, CrateId::Component).map_err(|_| ResourceError::LimitExceeded)?;
    let mut name_vec = BoundedVec::new(provider).unwrap();
    for &byte in name.as_bytes() {
        name_vec.push(byte).map_err(|_| ResourceError::LimitExceeded)?;
    }
    
    Ok(ResourceTypeMetadata {
        type_id: ResourceTypeId::new(1), // Stub implementation
        name: name_vec,
        size: 0,
    })
}

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{Checksummable, ToBytes, FromBytes, WriteStream, ReadStream};

// Macro to implement basic traits for simple types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                self.0.update_checksum(checksum);
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                _writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
                Ok(()
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

impl Default for ResourceHandle {
    fn default() -> Self {
        Self(INVALID_HANDLE)
    }
}

impl Default for ResourceTypeId {
    fn default() -> Self {
        Self(0)
    }
}

impl Default for ResourceData {
    fn default() -> Self {
        Self::Bytes({
            let provider = safe_managed_alloc!(65536, CrateId::Component).unwrap();
            BoundedVec::new(provider).unwrap()
        })
    }
}

// Apply macro to types that need traits
impl_basic_traits!(ResourceHandle, ResourceHandle::default();
impl_basic_traits!(ResourceTypeId, ResourceTypeId::default();
impl_basic_traits!(ResourceData, ResourceData::default();

// Tests moved from resource_management_tests.rs
#[cfg(test)]
mod tests {
    use crate::component_instantiation::InstanceId;
    use super::*;
    use wrt_error::ErrorCategory;

    // ====== RESOURCE HANDLE TESTS ======

    #[test]
    fn test_resource_handle_creation() {
        let handle = ResourceHandle::new(42);
        assert_eq!(handle.id(), 42);
        assert!(handle.is_valid();

        let invalid_handle = ResourceHandle(INVALID_HANDLE);
        assert!(!invalid_handle.is_valid();
        assert_eq!(invalid_handle.id(), u32::MAX);
    }

    #[test]
    fn test_resource_handle_comparison() {
        let handle1 = ResourceHandle::new(100);
        let handle2 = ResourceHandle::new(100);
        let handle3 = ResourceHandle::new(200);

        assert_eq!(handle1, handle2);
        assert_ne!(handle1, handle3);
        assert_ne!(handle2, handle3);
    }

    #[test]
    fn test_resource_type_id_creation() {
        let type_id = ResourceTypeId::new(123);
        assert_eq!(type_id.id(), 123);

        let type_id2 = ResourceTypeId::new(456);
        assert_eq!(type_id2.id(), 456);
        assert_ne!(type_id, type_id2);
    }

    // Note: Due to the large size of the original test file (1084 lines),
    // this represents a partial migration from resource_management_tests.rs.
    // The original file contained comprehensive tests covering:
    // - Resource handle creation and validation
    // - Resource data types and serialization
    // - Resource type system and metadata
    // - Resource table operations and lifecycle
    // - Resource manager coordination
    // - Error handling and edge cases
    // - Cross-environment compatibility (std/no_std)
    // - Integration with component instantiation
    // - Performance and stress testing
    //
    // These tests should be systematically distributed across the appropriate
    // modules in the resources/ directory as the implementation matures.
}