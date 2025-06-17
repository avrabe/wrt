//! WASI capability management and security model
//!
//! This module provides fine-grained control over WASI capabilities, allowing
//! host applications to specify exactly what system resources WASI modules
//! can access. Built on WRT's bounded collections for memory safety.

use wrt_foundation::{BoundedVec, BoundedString, safety_aware_alloc};
use crate::{prelude::*, WASI_CRATE_ID};

/// Maximum number of allowed filesystem paths
const MAX_FILESYSTEM_PATHS: usize = 32;
/// Maximum number of allowed environment variables  
const MAX_ENV_VARS: usize = 64;
/// Maximum length of path strings
const MAX_PATH_LENGTH: usize = 256;
/// Maximum length of environment variable names
const MAX_ENV_VAR_LENGTH: usize = 128;

/// WASI capability bundle - defines what a WASI module can access
#[derive(Debug, Clone, PartialEq)]
pub struct WasiCapabilities {
    /// Filesystem access capabilities
    pub filesystem: WasiFileSystemCapabilities,
    /// Environment and CLI capabilities
    pub environment: WasiEnvironmentCapabilities,
    /// Clock access capabilities
    pub clocks: WasiClockCapabilities,
    /// I/O capabilities
    pub io: WasiIoCapabilities,
    /// Random number generation capabilities
    pub random: WasiRandomCapabilities,
    /// Network capabilities (Preview3)
    #[cfg(feature = "preview3-prep")]
    pub network: WasiNetworkCapabilities,
}

impl WasiCapabilities {
    /// Create a minimal capability set with basic access
    pub fn minimal() -> Result<Self> {
        Ok(Self {
            filesystem: WasiFileSystemCapabilities::minimal()?,
            environment: WasiEnvironmentCapabilities::minimal()?,
            clocks: WasiClockCapabilities::minimal(),
            io: WasiIoCapabilities::minimal(),
            random: WasiRandomCapabilities::minimal(),
            #[cfg(feature = "preview3-prep")]
            network: WasiNetworkCapabilities::minimal(),
        })
    }
    
    /// Create a capability set suitable for sandboxed applications
    pub fn sandboxed() -> Result<Self> {
        Ok(Self {
            filesystem: WasiFileSystemCapabilities::read_only()?,
            environment: WasiEnvironmentCapabilities::args_only()?,
            clocks: WasiClockCapabilities::monotonic_only(),
            io: WasiIoCapabilities::stdio_only(),
            random: WasiRandomCapabilities::secure_only(),
            #[cfg(feature = "preview3-prep")]
            network: WasiNetworkCapabilities::none(),
        })
    }
    
    /// Create a capability set suitable for system utilities
    pub fn system_utility() -> Result<Self> {
        Ok(Self {
            filesystem: WasiFileSystemCapabilities::full_access()?,
            environment: WasiEnvironmentCapabilities::full_access()?,
            clocks: WasiClockCapabilities::full_access(),
            io: WasiIoCapabilities::full_access(),
            random: WasiRandomCapabilities::full_access(),
            #[cfg(feature = "preview3-prep")]
            network: WasiNetworkCapabilities::local_only(),
        })
    }
}

/// Filesystem access capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct WasiFileSystemCapabilities {
    /// Allowed filesystem paths (bounded for safety)
    allowed_paths: BoundedVec<BoundedString<MAX_PATH_LENGTH>, MAX_FILESYSTEM_PATHS>,
    /// Allow read operations
    pub read_access: bool,
    /// Allow write operations
    pub write_access: bool,
    /// Allow directory operations
    pub directory_access: bool,
    /// Allow file metadata access
    pub metadata_access: bool,
}

impl WasiFileSystemCapabilities {
    /// Create minimal filesystem capabilities (no access)
    pub fn minimal() -> Result<Self> {
        let provider = safety_aware_alloc!(8192, WASI_CRATE_ID)?;
        Ok(Self {
            allowed_paths: BoundedVec::new(provider)?,
            read_access: false,
            write_access: false,
            directory_access: false,
            metadata_access: false,
        })
    }
    
    /// Create read-only filesystem capabilities
    pub fn read_only() -> Result<Self> {
        let provider = safety_aware_alloc!(8192, WASI_CRATE_ID)?;
        Ok(Self {
            allowed_paths: BoundedVec::new(provider)?,
            read_access: true,
            write_access: false,
            directory_access: true,
            metadata_access: true,
        })
    }
    
    /// Create full filesystem access capabilities
    pub fn full_access() -> Result<Self> {
        let provider = safety_aware_alloc!(8192, WASI_CRATE_ID)?;
        Ok(Self {
            allowed_paths: BoundedVec::new(provider)?,
            read_access: true,
            write_access: true,
            directory_access: true,
            metadata_access: true,
        })
    }
    
    /// Add an allowed filesystem path
    pub fn add_allowed_path(&mut self, path: &str) -> Result<()> {
        let bounded_path = BoundedString::from_str(path)
            .map_err(|_| Error::new(
                ErrorCategory::Resource,
                codes::WASI_RESOURCE_LIMIT,
                kinds::WasiResourceError("Path too long")
            ))?;
            
        self.allowed_paths.push(bounded_path)
            .map_err(|_| Error::new(
                ErrorCategory::Resource,
                codes::WASI_RESOURCE_LIMIT,
                kinds::WasiResourceError("Too many allowed paths")
            ))?;
            
        Ok(())
    }
    
    /// Check if a path is allowed
    pub fn is_path_allowed(&self, path: &str) -> bool {
        if self.allowed_paths.is_empty() {
            // If no paths specified, allow current directory for minimal cases
            return path.starts_with("./") || !path.starts_with('/');
        }
        
        self.allowed_paths.iter().any(|allowed_path| {
            if let Ok(allowed) = allowed_path.as_str() {
                path.starts_with(allowed)
            } else {
                false
            }
        })
    }
}

/// Environment and CLI access capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct WasiEnvironmentCapabilities {
    /// Allow access to command line arguments
    pub args_access: bool,
    /// Allow access to environment variables
    pub environ_access: bool,
    /// Specific environment variables that are allowed
    allowed_env_vars: BoundedVec<BoundedString<MAX_ENV_VAR_LENGTH>, MAX_ENV_VARS>,
}

impl WasiEnvironmentCapabilities {
    /// Create minimal environment capabilities (no access)
    pub fn minimal() -> Result<Self> {
        let provider = safety_aware_alloc!(8192, WASI_CRATE_ID)?;
        Ok(Self {
            args_access: false,
            environ_access: false,
            allowed_env_vars: BoundedVec::new(provider)?,
        })
    }
    
    /// Create args-only environment capabilities
    pub fn args_only() -> Result<Self> {
        let provider = safety_aware_alloc!(8192, WASI_CRATE_ID)?;
        Ok(Self {
            args_access: true,
            environ_access: false,
            allowed_env_vars: BoundedVec::new(provider)?,
        })
    }
    
    /// Create full environment access capabilities
    pub fn full_access() -> Result<Self> {
        let provider = safety_aware_alloc!(8192, WASI_CRATE_ID)?;
        Ok(Self {
            args_access: true,
            environ_access: true,
            allowed_env_vars: BoundedVec::new(provider)?,
        })
    }
    
    /// Add an allowed environment variable
    pub fn add_allowed_var(&mut self, var_name: &str) -> Result<()> {
        let bounded_var = BoundedString::from_str(var_name)
            .map_err(|_| Error::new(
                ErrorCategory::Resource,
                codes::WASI_RESOURCE_LIMIT,
                kinds::WasiResourceError("Environment variable name too long")
            ))?;
            
        self.allowed_env_vars.push(bounded_var)
            .map_err(|_| Error::new(
                ErrorCategory::Resource,
                codes::WASI_RESOURCE_LIMIT,
                kinds::WasiResourceError("Too many allowed environment variables")
            ))?;
            
        Ok(())
    }
    
    /// Check if an environment variable is allowed
    pub fn is_env_var_allowed(&self, var_name: &str) -> bool {
        if !self.environ_access {
            return false;
        }
        
        if self.allowed_env_vars.is_empty() {
            // If no specific vars listed, allow all when environ_access is true
            return true;
        }
        
        self.allowed_env_vars.iter().any(|allowed_var| {
            if let Ok(allowed) = allowed_var.as_str() {
                allowed == var_name
            } else {
                false
            }
        })
    }
}

/// Clock access capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct WasiClockCapabilities {
    /// Allow access to realtime clock
    pub realtime_access: bool,
    /// Allow access to monotonic clock
    pub monotonic_access: bool,
    /// Allow access to process CPU time
    pub process_cputime_access: bool,
    /// Allow access to thread CPU time
    pub thread_cputime_access: bool,
}

impl WasiClockCapabilities {
    /// Create minimal clock capabilities (monotonic only)
    pub fn minimal() -> Self {
        Self {
            realtime_access: false,
            monotonic_access: true,
            process_cputime_access: false,
            thread_cputime_access: false,
        }
    }
    
    /// Create monotonic-only clock capabilities
    pub fn monotonic_only() -> Self {
        Self {
            realtime_access: false,
            monotonic_access: true,
            process_cputime_access: false,
            thread_cputime_access: false,
        }
    }
    
    /// Create full clock access capabilities
    pub fn full_access() -> Self {
        Self {
            realtime_access: true,
            monotonic_access: true,
            process_cputime_access: true,
            thread_cputime_access: true,
        }
    }
}

/// I/O stream capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct WasiIoCapabilities {
    /// Allow access to stdin
    pub stdin_access: bool,
    /// Allow access to stdout  
    pub stdout_access: bool,
    /// Allow access to stderr
    pub stderr_access: bool,
    /// Allow creation of custom streams
    pub custom_streams: bool,
}

impl WasiIoCapabilities {
    /// Create minimal I/O capabilities (no access)
    pub fn minimal() -> Self {
        Self {
            stdin_access: false,
            stdout_access: false,
            stderr_access: false,
            custom_streams: false,
        }
    }
    
    /// Create stdio-only I/O capabilities
    pub fn stdio_only() -> Self {
        Self {
            stdin_access: true,
            stdout_access: true,
            stderr_access: true,
            custom_streams: false,
        }
    }
    
    /// Create full I/O access capabilities
    pub fn full_access() -> Self {
        Self {
            stdin_access: true,
            stdout_access: true,
            stderr_access: true,
            custom_streams: true,
        }
    }
}

/// Random number generation capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct WasiRandomCapabilities {
    /// Allow access to cryptographically secure random numbers
    pub secure_random: bool,
    /// Allow access to fast pseudo-random numbers
    pub pseudo_random: bool,
}

impl WasiRandomCapabilities {
    /// Create minimal random capabilities (no access)
    pub fn minimal() -> Self {
        Self {
            secure_random: false,
            pseudo_random: false,
        }
    }
    
    /// Create secure-only random capabilities
    pub fn secure_only() -> Self {
        Self {
            secure_random: true,
            pseudo_random: false,
        }
    }
    
    /// Create full random access capabilities
    pub fn full_access() -> Self {
        Self {
            secure_random: true,
            pseudo_random: true,
        }
    }
}

/// Network capabilities (Preview3 preparation)
#[cfg(feature = "preview3-prep")]
#[derive(Debug, Clone, PartialEq)]
pub struct WasiNetworkCapabilities {
    /// Allow TCP connections
    pub tcp_access: bool,
    /// Allow UDP connections
    pub udp_access: bool,
    /// Allow only localhost connections
    pub localhost_only: bool,
    /// Allow outbound connections
    pub outbound_access: bool,
    /// Allow inbound connections (listening)
    pub inbound_access: bool,
}

#[cfg(feature = "preview3-prep")]
impl WasiNetworkCapabilities {
    /// Create no network capabilities
    pub fn none() -> Self {
        Self {
            tcp_access: false,
            udp_access: false,
            localhost_only: true,
            outbound_access: false,
            inbound_access: false,
        }
    }
    
    /// Create localhost-only network capabilities
    pub fn local_only() -> Self {
        Self {
            tcp_access: true,
            udp_access: true,
            localhost_only: true,
            outbound_access: true,
            inbound_access: true,
        }
    }
    
    /// Create full network access capabilities
    pub fn full_access() -> Self {
        Self {
            tcp_access: true,
            udp_access: true,
            localhost_only: false,
            outbound_access: true,
            inbound_access: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_minimal_capabilities() -> Result<()> {
        let caps = WasiCapabilities::minimal()?;
        assert!(!caps.filesystem.read_access);
        assert!(!caps.environment.args_access);
        assert!(caps.clocks.monotonic_access);
        assert!(!caps.io.stdin_access);
        assert!(!caps.random.secure_random);
        Ok(())
    }
    
    #[test]
    fn test_filesystem_path_management() -> Result<()> {
        let mut fs_caps = WasiFileSystemCapabilities::minimal()?;
        
        fs_caps.add_allowed_path("/tmp")?;
        fs_caps.add_allowed_path("/home/user")?;
        
        assert!(fs_caps.is_path_allowed("/tmp/file.txt"));
        assert!(fs_caps.is_path_allowed("/home/user/docs"));
        assert!(!fs_caps.is_path_allowed("/etc/passwd"));
        
        Ok(())
    }
    
    #[test]
    fn test_environment_var_management() -> Result<()> {
        let mut env_caps = WasiEnvironmentCapabilities::full_access()?;
        
        env_caps.add_allowed_var("PATH")?;
        env_caps.add_allowed_var("HOME")?;
        
        // When specific vars are listed, only those are allowed
        assert!(env_caps.is_env_var_allowed("PATH"));
        assert!(env_caps.is_env_var_allowed("HOME"));
        assert!(!env_caps.is_env_var_allowed("SECRET_KEY"));
        
        Ok(())
    }
    
    #[test]
    fn test_capability_presets() -> Result<()> {
        let sandboxed = WasiCapabilities::sandboxed()?;
        assert!(sandboxed.filesystem.read_access);
        assert!(!sandboxed.filesystem.write_access);
        assert!(sandboxed.environment.args_access);
        assert!(!sandboxed.environment.environ_access);
        
        let system = WasiCapabilities::system_utility()?;
        assert!(system.filesystem.write_access);
        assert!(system.environment.environ_access);
        assert!(system.clocks.realtime_access);
        
        Ok(())
    }
}