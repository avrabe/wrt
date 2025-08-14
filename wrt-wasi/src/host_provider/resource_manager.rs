//! WASI resource manager using WRT foundation patterns
//!
//! This module provides resource management for WASI handles using the proven
//! Resource<P> patterns from wrt-foundation.

use core::any::Any;

use wrt_error::{
    ErrorSource,
    Result,
};
#[cfg(feature = "std")]
use wrt_foundation::capabilities::CapabilityAwareProvider;
use wrt_foundation::{
    budget_aware_provider::CrateId as BudgetCrateId,
    resource::{
        Resource,
        ResourceOperation,
        ResourceRepr,
    },
    safe_managed_alloc,
    safe_memory::NoStdProvider,
    traits::{
        Checksummable,
        FromBytes,
        ReadStream,
        ToBytes,
        WriteStream,
    },
    verification::Checksum,
    BoundedMap,
    BoundedString,
    BoundedVec,
    CrateId,
};

use crate::prelude::*;

/// Maximum number of WASI resources per manager
const MAX_WASI_RESOURCES: usize = 256;

// Type alias for provider
#[cfg(feature = "std")]
type WasiProvider = CapabilityAwareProvider<NoStdProvider<8192>>;
#[cfg(not(feature = "std"))]
type WasiProvider = NoStdProvider<8192>;

// Helper function to create provider
fn create_wasi_provider() -> Result<WasiProvider> {
    #[cfg(feature = "std")]
    {
        let base_provider = safe_managed_alloc!(8192, BudgetCrateId::Wasi)?;
        let capability = Box::new(wrt_foundation::capabilities::DynamicMemoryCapability::new(
            8192,
            wrt_foundation::CrateId::Wasi,
            wrt_foundation::verification::VerificationLevel::Standard,
        ));
        Ok(CapabilityAwareProvider::new(
            base_provider,
            capability,
            wrt_foundation::CrateId::Wasi,
        ))
    }
    #[cfg(not(feature = "std"))]
    {
        let provider = safe_managed_alloc!(8192, BudgetCrateId::Wasi)?;
        Ok(provider)
    }
}

/// WASI resource handle type
pub type WasiHandle = u32;

/// WASI-specific resource types
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WasiResourceType {
    /// Null/empty resource type (default)
    #[default]
    Null,
    /// File descriptor
    FileDescriptor {
        #[cfg(feature = "std")]
        path:     BoundedString<256, CapabilityAwareProvider<NoStdProvider<8192>>>,
        #[cfg(not(feature = "std"))]
        path:     BoundedString<256, NoStdProvider<8192>>,
        readable: bool,
        writable: bool,
    },
    /// Directory handle
    DirectoryHandle {
        #[cfg(feature = "std")]
        path: BoundedString<256, CapabilityAwareProvider<NoStdProvider<8192>>>,
        #[cfg(not(feature = "std"))]
        path: BoundedString<256, NoStdProvider<8192>>,
    },
    /// Input stream
    InputStream {
        #[cfg(feature = "std")]
        name:     BoundedString<64, CapabilityAwareProvider<NoStdProvider<8192>>>,
        #[cfg(not(feature = "std"))]
        name:     BoundedString<64, NoStdProvider<8192>>,
        position: u64,
    },
    /// Output stream
    OutputStream {
        #[cfg(feature = "std")]
        name:     BoundedString<64, CapabilityAwareProvider<NoStdProvider<8192>>>,
        #[cfg(not(feature = "std"))]
        name:     BoundedString<64, NoStdProvider<8192>>,
        position: u64,
    },
    /// Clock handle
    ClockHandle { clock_type: WasiClockType },
    /// Random generator handle
    RandomHandle { secure: bool },
}

/// WASI clock types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WasiClockType {
    /// Realtime clock
    #[default]
    Realtime,
    /// Monotonic clock
    Monotonic,
    /// Process CPU time
    ProcessCpuTime,
    /// Thread CPU time
    ThreadCpuTime,
}

/// WASI resource manager
///
/// Manages WASI resource handles using WRT's proven resource management
/// patterns
#[derive(Debug)]
pub struct WasiResourceManager {
    /// Resource table using WRT foundation patterns
    resources:   BoundedMap<WasiHandle, WasiResource, MAX_WASI_RESOURCES, WasiProvider>,
    /// Next available handle ID
    next_handle: WasiHandle,
    /// Memory provider for allocations
    provider:    WasiProvider,
}

/// WASI resource wrapper using WRT Resource<P> pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasiResource {
    /// Base WRT resource
    #[cfg(feature = "std")]
    base:          Resource<CapabilityAwareProvider<NoStdProvider<8192>>>,
    #[cfg(not(feature = "std"))]
    base:          Resource<NoStdProvider<8192>>,
    /// WASI-specific resource type
    resource_type: WasiResourceType,
    /// Resource capabilities
    capabilities:  WasiResourceCapabilities,
}

impl Default for WasiResource {
    fn default() -> Self {
        Self {
            base:          Resource::new(
                0, // default ID
                wrt_foundation::resource::ResourceRepr::Opaque,
                None,
                wrt_foundation::verification::VerificationLevel::Standard,
            ),
            resource_type: WasiResourceType::Null,
            capabilities:  WasiResourceCapabilities::default(),
        }
    }
}

/// Capabilities for WASI resources
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasiResourceCapabilities {
    /// Can read from this resource
    pub readable:        bool,
    /// Can write to this resource  
    pub writable:        bool,
    /// Can seek in this resource
    pub seekable:        bool,
    /// Can get metadata from this resource
    pub metadata_access: bool,
}

impl WasiResourceManager {
    /// Create a new WASI resource manager
    pub fn new() -> Result<Self> {
        let provider = create_wasi_provider()?;
        #[cfg(feature = "std")]
        let resources = BoundedMap::new(provider.clone())?;
        #[cfg(not(feature = "std"))]
        let resources = {
            let provider2 = create_wasi_provider()?;
            BoundedMap::new(provider2)?
        };

        Ok(Self {
            resources,
            next_handle: 1, // Start at 1, reserve 0 for invalid handle
            provider,
        })
    }

    /// Create a new WASI resource and return its handle
    pub fn create_resource(
        &mut self,
        resource_type: WasiResourceType,
        capabilities: WasiResourceCapabilities,
    ) -> Result<WasiHandle> {
        // Create base WRT resource
        let base = Resource::new(
            self.next_handle,
            wrt_foundation::resource::ResourceRepr::Opaque,
            None,
            wrt_foundation::verification::VerificationLevel::Standard,
        );

        // Create WASI resource wrapper
        let wasi_resource = WasiResource {
            base,
            resource_type,
            capabilities,
        };

        // Get next handle ID
        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if self.next_handle == 0 {
            self.next_handle = 1; // Skip 0
        }

        // Store resource
        self.resources
            .insert(handle, wasi_resource)
            .map_err(|_| Error::runtime_execution_error("Failed to insert resource into map"))?;

        Ok(handle)
    }

    /// Get a WASI resource by handle
    pub fn get_resource(&self, handle: WasiHandle) -> Result<WasiResource> {
        self.resources.get(&handle)?.ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::WASI_INVALID_FD,
                "Invalid WASI handle",
            )
        })
    }

    /// Get a WASI resource by handle (for modification via update_resource)
    pub fn get_resource_mut(&mut self, handle: WasiHandle) -> Result<WasiResource> {
        // Note: BoundedMap doesn't support get_mut due to serialization constraints
        // Use get() and then update_resource() to modify
        self.resources
            .get(&handle)?
            .ok_or_else(|| Error::runtime_execution_error("Resource not found"))
    }

    /// Remove a WASI resource by handle
    pub fn remove_resource(&mut self, handle: WasiHandle) -> Result<WasiResource> {
        self.resources.remove(&handle)?.ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::WASI_INVALID_FD,
                "Invalid WASI handle",
            )
        })
    }

    /// Check if a handle is valid
    pub fn is_valid_handle(&self, handle: WasiHandle) -> bool {
        self.resources.contains_key(&handle).unwrap_or(false)
    }

    /// Get the number of active resources
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Create a file descriptor resource
    pub fn create_file_descriptor(
        &mut self,
        path: &str,
        readable: bool,
        writable: bool,
    ) -> Result<WasiHandle> {
        let path_string = BoundedString::from_str(path, self.provider.clone())
            .map_err(|_| Error::runtime_execution_error("Path string too long"))?;

        let resource_type = WasiResourceType::FileDescriptor {
            path: path_string,
            readable,
            writable,
        };

        let capabilities = WasiResourceCapabilities {
            readable,
            writable,
            seekable: true,
            metadata_access: true,
        };

        self.create_resource(resource_type, capabilities)
    }

    /// Create a directory handle resource
    pub fn create_directory_handle(&mut self, path: &str) -> Result<WasiHandle> {
        let path_string = BoundedString::from_str(path, self.provider.clone()).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::WASI_RESOURCE_LIMIT,
                "Invalid WASI handle",
            )
        })?;

        let resource_type = WasiResourceType::DirectoryHandle { path: path_string };

        let capabilities = WasiResourceCapabilities {
            readable:        true,
            writable:        false,
            seekable:        false,
            metadata_access: true,
        };

        self.create_resource(resource_type, capabilities)
    }

    /// Create an input stream resource
    pub fn create_input_stream(&mut self, name: &str) -> Result<WasiHandle> {
        let name_string = BoundedString::from_str(name, self.provider.clone())
            .map_err(|_| Error::runtime_execution_error("Stream name too long"))?;

        let resource_type = WasiResourceType::InputStream {
            name:     name_string,
            position: 0,
        };

        let capabilities = WasiResourceCapabilities {
            readable:        true,
            writable:        false,
            seekable:        false,
            metadata_access: false,
        };

        self.create_resource(resource_type, capabilities)
    }

    /// Create an output stream resource
    pub fn create_output_stream(&mut self, name: &str) -> Result<WasiHandle> {
        let name_string = BoundedString::from_str(name, self.provider.clone()).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::WASI_RESOURCE_LIMIT,
                "Invalid WASI handle",
            )
        })?;

        let resource_type = WasiResourceType::OutputStream {
            name:     name_string,
            position: 0,
        };

        let capabilities = WasiResourceCapabilities {
            readable:        false,
            writable:        true,
            seekable:        false,
            metadata_access: false,
        };

        self.create_resource(resource_type, capabilities)
    }

    /// Create a clock handle resource
    pub fn create_clock_handle(&mut self, clock_type: WasiClockType) -> Result<WasiHandle> {
        let resource_type = WasiResourceType::ClockHandle { clock_type };

        let capabilities = WasiResourceCapabilities {
            readable:        true,
            writable:        false,
            seekable:        false,
            metadata_access: false,
        };

        self.create_resource(resource_type, capabilities)
    }

    /// Create a random handle resource
    pub fn create_random_handle(&mut self, secure: bool) -> Result<WasiHandle> {
        let resource_type = WasiResourceType::RandomHandle { secure };

        let capabilities = WasiResourceCapabilities {
            readable:        true,
            writable:        false,
            seekable:        false,
            metadata_access: false,
        };

        self.create_resource(resource_type, capabilities)
    }
}

impl WasiResource {
    /// Get the WASI resource type
    pub fn resource_type(&self) -> &WasiResourceType {
        &self.resource_type
    }

    /// Get the resource capabilities
    pub fn capabilities(&self) -> &WasiResourceCapabilities {
        &self.capabilities
    }

    /// Check if the resource can be read
    pub fn is_readable(&self) -> bool {
        self.capabilities.readable
    }

    /// Check if the resource can be written
    pub fn is_writable(&self) -> bool {
        self.capabilities.writable
    }

    /// Check if the resource supports seeking
    pub fn is_seekable(&self) -> bool {
        self.capabilities.seekable
    }

    /// Verify an operation is allowed on this resource
    pub fn verify_operation(&self, _operation: ResourceOperation) -> Result<()> {
        // TODO: Implement operation verification when available in wrt-foundation
        Ok(())
    }

    /// Update stream position (for stream resources)
    pub fn update_position(&mut self, new_position: u64) -> Result<()> {
        match &mut self.resource_type {
            WasiResourceType::InputStream { position, .. } => {
                *position = new_position;
                Ok(())
            },
            WasiResourceType::OutputStream { position, .. } => {
                *position = new_position;
                Ok(())
            },
            _ => Err(Error::runtime_execution_error(
                "Invalid resource type for file operations",
            )),
        }
    }

    /// Get current position (for stream resources)
    pub fn get_position(&self) -> Option<u64> {
        match &self.resource_type {
            WasiResourceType::InputStream { position, .. } => Some(*position),
            WasiResourceType::OutputStream { position, .. } => Some(*position),
            _ => None,
        }
    }
}

impl Default for WasiResourceCapabilities {
    fn default() -> Self {
        Self {
            readable:        false,
            writable:        false,
            seekable:        false,
            metadata_access: false,
        }
    }
}

// Implement required traits for WasiResource to work with BoundedMap
impl Checksummable for WasiResource {
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Simple checksum based on resource type
        match &self.resource_type {
            WasiResourceType::Null => checksum.update_slice(b"null"),
            WasiResourceType::FileDescriptor {
                path,
                readable,
                writable,
            } => {
                checksum.update_slice(b"file");
                if let Ok(path_str) = path.as_str() {
                    checksum.update_slice(path_str.as_bytes());
                }
                checksum.update_slice(&[*readable as u8, *writable as u8]);
            },
            WasiResourceType::DirectoryHandle { path } => {
                checksum.update_slice(b"dir");
                if let Ok(path_str) = path.as_str() {
                    checksum.update_slice(path_str.as_bytes());
                }
            },
            WasiResourceType::InputStream { name, position } => {
                checksum.update_slice(b"in");
                if let Ok(name_str) = name.as_str() {
                    checksum.update_slice(name_str.as_bytes());
                }
                checksum.update_slice(&position.to_le_bytes());
            },
            WasiResourceType::OutputStream { name, position } => {
                checksum.update_slice(b"out");
                if let Ok(name_str) = name.as_str() {
                    checksum.update_slice(name_str.as_bytes());
                }
                checksum.update_slice(&position.to_le_bytes());
            },
            WasiResourceType::ClockHandle { clock_type } => {
                checksum.update_slice(b"clock");
                checksum.update_slice(&[*clock_type as u8]);
            },
            WasiResourceType::RandomHandle { secure } => {
                checksum.update_slice(b"random");
                checksum.update_slice(&[*secure as u8]);
            },
        }
    }
}

impl ToBytes for WasiResource {
    fn serialized_size(&self) -> usize {
        // Simple serialization size estimation
        64 // Fixed size for simplicity
    }

    fn to_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'_>,
        _provider: &P,
    ) -> Result<()> {
        // Write resource type discriminant
        match &self.resource_type {
            WasiResourceType::Null => writer.write_u8(0)?,
            WasiResourceType::FileDescriptor { .. } => writer.write_u8(1)?,
            WasiResourceType::DirectoryHandle { .. } => writer.write_u8(2)?,
            WasiResourceType::InputStream { .. } => writer.write_u8(3)?,
            WasiResourceType::OutputStream { .. } => writer.write_u8(4)?,
            WasiResourceType::ClockHandle { .. } => writer.write_u8(5)?,
            WasiResourceType::RandomHandle { .. } => writer.write_u8(6)?,
        }
        Ok(())
    }
}

impl FromBytes for WasiResource {
    fn from_bytes_with_provider<P: wrt_foundation::MemoryProvider>(
        reader: &mut ReadStream<'_>,
        _provider: &P,
    ) -> Result<Self> {
        let discriminant = reader.read_u8()?;
        let resource_type = match discriminant {
            0 => WasiResourceType::Null,
            _ => WasiResourceType::Null, // Default for unsupported types
        };

        Ok(WasiResource {
            base: Resource::new(
                0, // placeholder ID
                wrt_foundation::resource::ResourceRepr::Opaque,
                None,
                wrt_foundation::verification::VerificationLevel::Standard,
            ),
            resource_type,
            capabilities: WasiResourceCapabilities::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_manager_creation() -> Result<()> {
        let manager = WasiResourceManager::new()?;
        assert_eq!(manager.resource_count(), 0);
        Ok(())
    }

    #[test]
    fn test_file_descriptor_creation() -> Result<()> {
        let mut manager = WasiResourceManager::new()?;

        let fd = manager.create_file_descriptor("/tmp/test.txt", true, false)?;
        assert!(manager.is_valid_handle(fd));
        assert_eq!(manager.resource_count(), 1);

        let resource = manager.get_resource(fd)?;
        assert!(resource.is_readable());
        assert!(!resource.is_writable());

        Ok(())
    }

    #[test]
    fn test_stream_creation() -> Result<()> {
        let mut manager = WasiResourceManager::new()?;

        let input = manager.create_input_stream("stdin")?;
        let output = manager.create_output_stream("stdout")?;

        assert!(manager.is_valid_handle(input));
        assert!(manager.is_valid_handle(output));
        assert_eq!(manager.resource_count(), 2);

        let input_resource = manager.get_resource(input)?;
        let output_resource = manager.get_resource(output)?;

        assert!(input_resource.is_readable());
        assert!(!input_resource.is_writable());
        assert!(!output_resource.is_readable());
        assert!(output_resource.is_writable());

        Ok(())
    }

    #[test]
    fn test_resource_removal() -> Result<()> {
        let mut manager = WasiResourceManager::new()?;

        let fd = manager.create_file_descriptor("/tmp/test.txt", true, true)?;
        assert_eq!(manager.resource_count(), 1);

        let removed = manager.remove_resource(fd)?;
        assert_eq!(manager.resource_count(), 0);
        assert!(!manager.is_valid_handle(fd));

        match removed.resource_type() {
            WasiResourceType::FileDescriptor {
                readable, writable, ..
            } => {
                assert!(*readable);
                assert!(*writable);
            },
            _ => panic!("Expected file descriptor"),
        }

        Ok(())
    }

    #[test]
    fn test_invalid_handle_access() -> Result<()> {
        let manager = WasiResourceManager::new()?;

        let result = manager.get_resource(999);
        assert!(result.is_err());

        if let Err(error) = result {
            assert_eq!(error.code(), codes::WASI_INVALID_FD);
        }

        Ok(())
    }

    #[test]
    fn test_clock_handle_creation() -> Result<()> {
        let mut manager = WasiResourceManager::new()?;

        let monotonic = manager.create_clock_handle(WasiClockType::Monotonic)?;
        let realtime = manager.create_clock_handle(WasiClockType::Realtime)?;

        assert_ne!(monotonic, realtime);
        assert_eq!(manager.resource_count(), 2);

        Ok(())
    }
}
