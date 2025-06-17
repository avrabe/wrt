//! # WRT WASI Implementation
//!
//! WASI (WebAssembly System Interface) Preview2 implementation for the WRT WebAssembly runtime.
//! This crate provides WASI host functions that integrate seamlessly with the WRT component model
//! and resource management system.
//!
//! ## Features
//!
//! - **WASI Preview2**: Complete implementation of WASI Preview2 interfaces
//! - **Component Model**: Native integration with WebAssembly Component Model
//! - **Resource Management**: Built on WRT's proven resource management patterns
//! - **Memory Safety**: Uses WRT's safe memory allocation system
//! - **Platform Abstraction**: Works across std/no_std environments
//! - **Preview3 Preparation**: Foundation for future WASI Preview3 features
//!
//! ## Supported WASI Interfaces
//!
//! ### WASI Preview2
//! - `wasi:filesystem` - File and directory operations
//! - `wasi:cli` - Command line arguments and environment variables
//! - `wasi:clocks` - Time and monotonic clock access
//! - `wasi:io` - Stream I/O operations
//! - `wasi:random` - Random number generation
//!
//! ### Future (Preview3 Preparation)
//! - `wasi:sockets` - Network socket operations
//! - Async/await support
//! - Threading primitives
//!
//! ## Usage
//!
//! ```rust
//! use wrt_wasi::{WasiCapabilities, ComponentModelProvider};
//! use wrt_component::ComponentLinker;
//!
//! // Create WASI capabilities
//! let mut capabilities = WasiCapabilities::minimal();
//! capabilities.filesystem.add_allowed_path("/tmp");
//! capabilities.environment.args_access = true;
//!
//! // Create WASI provider
//! let provider = ComponentModelProvider::new(capabilities)?;
//!
//! // Link with component
//! let mut linker = ComponentLinker::new()?;
//! linker.link_wasi_provider(&provider)?;
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export core WRT types for convenience
pub use wrt_error::{Error, ErrorCategory, Result};
pub use wrt_foundation::{Resource, MemoryProvider};
pub use wrt_host::{CallbackRegistry, HostFunction};
pub use wrt_component::ComponentLinker;

// Re-export safety-aware allocation macros
pub use wrt_foundation::{safety_aware_alloc, safe_managed_alloc, CrateId};

// Safety configuration for WASI
use wrt_foundation::safety_features::{allocation::MEMORY_STRATEGY, runtime};

/// WASI-specific crate ID for memory allocation tracking
pub const WASI_CRATE_ID: CrateId = CrateId::new("wrt-wasi");

/// Get the current safety level for WASI operations
pub const fn wasi_safety_level() -> &'static str {
    runtime::current_safety_level()
}

/// Get maximum allocation size for WASI operations based on safety level
pub const fn wasi_max_allocation_size() -> usize {
    runtime::max_allocation_size()
}

// WASI Preview2 interfaces
#[cfg(feature = "preview2")]
pub mod preview2 {
    //! WASI Preview2 interface implementations
    
    #[cfg(feature = "wasi-filesystem")]
    pub mod filesystem;
    
    #[cfg(feature = "wasi-cli")]
    pub mod cli;
    
    #[cfg(feature = "wasi-clocks")]
    pub mod clocks;
    
    #[cfg(feature = "wasi-io")]
    pub mod io;
    
    #[cfg(feature = "wasi-random")]
    pub mod random;
}

// Preview3 preparation layer
#[cfg(feature = "preview3-prep")]
pub mod preview3 {
    //! WASI Preview3 preparation layer
    //!
    //! This module provides the foundation for future WASI Preview3 features
    //! including async/await, threading, and advanced I/O operations.
    
    pub mod preparation;
}

// Host provider for component model integration
pub mod host_provider {
    //! Host provider implementations for WASI integration
    
    pub mod component_model_provider;
    pub mod resource_manager;
}

// WASI capabilities and security model
pub mod capabilities;

// WIT interface bindings
#[cfg(feature = "preview2")]
pub mod wit_bindings;

// Re-export main types for convenience
pub use capabilities::{WasiCapabilities, WasiFileSystemCapabilities, WasiEnvironmentCapabilities};

#[cfg(feature = "preview2")]
pub use host_provider::component_model_provider::ComponentModelProvider;

#[cfg(feature = "preview2")]
pub use host_provider::resource_manager::WasiResourceManager;

/// WASI version enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasiVersion {
    /// WASI Preview2 (component model)
    Preview2,
    /// WASI Preview3 (future - async/threading)
    #[cfg(feature = "preview3-prep")]
    Preview3,
}

impl Default for WasiVersion {
    fn default() -> Self {
        Self::Preview2
    }
}

/// WASI host provider trait for integration with different execution engines
pub trait WasiHostProvider {
    /// Get all host functions provided by this WASI implementation
    fn get_host_functions(&self) -> Result<Vec<HostFunction>>;
    
    /// Get the number of functions provided
    fn function_count(&self) -> usize;
    
    /// Get the WASI version supported by this provider
    fn version(&self) -> WasiVersion;
    
    /// Get the capabilities enabled for this provider
    fn capabilities(&self) -> &WasiCapabilities;
}

/// Error types specific to WASI operations
pub mod error {
    use wrt_error::{ErrorCategory, ErrorCode, ErrorKind};
    
    /// WASI-specific error codes
    pub mod codes {
        use wrt_error::ErrorCode;
        
        /// WASI permission denied
        pub const WASI_PERMISSION_DENIED: ErrorCode = ErrorCode(0x2001);
        /// WASI file not found
        pub const WASI_FILE_NOT_FOUND: ErrorCode = ErrorCode(0x2002);
        /// WASI invalid file descriptor
        pub const WASI_INVALID_FD: ErrorCode = ErrorCode(0x2003);
        /// WASI capability not available
        pub const WASI_CAPABILITY_UNAVAILABLE: ErrorCode = ErrorCode(0x2004);
        /// WASI resource limit exceeded
        pub const WASI_RESOURCE_LIMIT: ErrorCode = ErrorCode(0x2005);
    }
    
    /// WASI-specific error kinds
    pub mod kinds {
        use wrt_error::ErrorKind;
        
        /// WASI permission error
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct WasiPermissionError(pub &'static str);
        impl ErrorKind for WasiPermissionError {}
        
        /// WASI file system error
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct WasiFileSystemError(pub &'static str);
        impl ErrorKind for WasiFileSystemError {}
        
        /// WASI resource error
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct WasiResourceError(pub &'static str);
        impl ErrorKind for WasiResourceError {}
    }
}

// Prelude for common imports
pub mod prelude {
    //! Common imports for WASI implementations
    
    pub use super::{
        WasiVersion, WasiHostProvider, WasiCapabilities,
        Error, ErrorCategory, Result,
        Resource, MemoryProvider,
        CallbackRegistry, HostFunction,
        ComponentLinker,
    };
    
    #[cfg(feature = "preview2")]
    pub use super::{ComponentModelProvider, WasiResourceManager};
    
    pub use super::error::{codes, kinds};
    
    // Re-export commonly used WRT foundation types
    pub use wrt_foundation::{
        BoundedVec, BoundedMap, BoundedString,
        capability_context, safe_capability_alloc, CrateId,
    };
    
    // Re-export platform abstractions
    pub use wrt_platform::{
        memory::MemoryProvider as PlatformMemoryProvider,
        filesystem::PlatformFilesystem,
        time::PlatformTime,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasi_version_default() {
        assert_eq!(WasiVersion::default(), WasiVersion::Preview2);
    }
    
    #[test]
    fn test_error_codes_are_unique() {
        use error::codes::*;
        
        let codes = [
            WASI_PERMISSION_DENIED,
            WASI_FILE_NOT_FOUND,
            WASI_INVALID_FD,
            WASI_CAPABILITY_UNAVAILABLE,
            WASI_RESOURCE_LIMIT,
        ];
        
        // Ensure all codes are unique
        for (i, &code1) in codes.iter().enumerate() {
            for &code2 in codes.iter().skip(i + 1) {
                assert_ne!(code1.0, code2.0, "Error codes must be unique");
            }
        }
    }
}