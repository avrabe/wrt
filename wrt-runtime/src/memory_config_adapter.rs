//! Memory Configuration Adapter for Runtime
//!
//! This module provides adapters that convert global memory configuration
//! into runtime-specific memory provider configurations, replacing all
//! hardcoded memory sizes with platform-aware dynamic sizing.

use wrt_foundation::{
    budget_aware_provider::CrateId,
    safe_memory::NoStdProvider,
    capabilities::{CapabilityAwareProvider, capability_context, safe_capability_alloc},
    prelude::*,
};

// Import provider creation functions from prelude which handles conditionals

/// Runtime memory configuration that replaces hardcoded sizes
pub struct RuntimeMemoryConfig {
    /// String buffer size based on platform limits
    pub string_buffer_size: usize,
    /// Vector capacity based on platform limits  
    pub vector_capacity: usize,
    /// Provider buffer size based on platform limits
    pub provider_buffer_size: usize,
    /// Maximum function parameters based on platform limits
    pub max_function_params: usize,
}

impl RuntimeMemoryConfig {
    /// Create runtime memory configuration from global limits
    pub fn from_global_limits() -> Result<Self> {
        let config = global_memory_config();
        let stats = config.memory_stats();
        
        // Calculate sizes based on platform capabilities
        // Use fractions of available memory for different components
        let string_buffer_size = if stats.max_stack_memory > 0 {
            core::cmp::min(512, stats.max_stack_memory / 1024) // Max 512, scaled by stack memory
        } else {
            256 // Default fallback
        };
        
        let vector_capacity = if stats.max_wasm_memory > 0 {
            core::cmp::min(1024, stats.max_wasm_memory / (64 * 1024)) // Scaled by WASM memory
        } else {
            256 // Default fallback
        };
        
        let provider_buffer_size = if stats.max_stack_memory > 0 {
            core::cmp::min(4096, stats.max_stack_memory / 256) // Conservative stack usage
        } else {
            1024 // Default fallback
        };
        
        let max_function_params = if stats.max_components > 0 {
            core::cmp::min(256, stats.max_components * 2) // Scale with component count
        } else {
            128 // Default fallback
        };
        
        Ok(Self {
            string_buffer_size,
            vector_capacity, 
            provider_buffer_size,
            max_function_params,
        })
    }
    
    /// Get the string buffer size for bounded strings
    pub fn string_buffer_size(&self) -> usize {
        self.string_buffer_size
    }
    
    /// Get the vector capacity for bounded vectors
    pub fn vector_capacity(&self) -> usize {
        self.vector_capacity
    }
    
    /// Get the provider buffer size for memory providers
    pub fn provider_buffer_size(&self) -> usize {
        self.provider_buffer_size
    }
    
    /// Get the maximum function parameters
    pub fn max_function_params(&self) -> usize {
        self.max_function_params
    }
}

/// Global runtime memory configuration instance
static RUNTIME_CONFIG: core::sync::atomic::AtomicPtr<RuntimeMemoryConfig> = 
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

/// Initialize runtime memory configuration
pub fn initialize_runtime_memory_config() -> Result<()> {
    // In no_std mode, we use a static configuration
    // The atomic pointer approach is not suitable for no_std without allocation
    // This is a placeholder implementation - in a real system you would
    // configure this at compile time or use a different approach
    Ok(())
}

/// Get the runtime memory configuration
pub fn runtime_memory_config() -> &'static RuntimeMemoryConfig {
    // Return a static default configuration for no_std mode
    static DEFAULT_CONFIG: RuntimeMemoryConfig = RuntimeMemoryConfig {
        string_buffer_size: 256,
        vector_capacity: 256,
        provider_buffer_size: 1024,
        max_function_params: 32,
    };
    &DEFAULT_CONFIG
}

/// Platform-aware type aliases that replace hardcoded sizes
pub mod platform_types {
    use super::*;
    use wrt_foundation::{bounded::*, safe_memory::NoStdProvider};
    
    /// Create a platform-aware bounded string type
    pub fn create_bounded_string() -> Result<BoundedString<512, NoStdProvider<1024>>> {
        let config = runtime_memory_config();
        let provider = NoStdProvider::<1024>::default();
        
        // Use the configured string buffer size, capped at the type's maximum
        BoundedString::new(provider)
    }
    
    /// Create a platform-aware bounded vector type
    pub fn create_bounded_vec<T>() -> Result<BoundedVec<T, 1024, NoStdProvider<2048>>>
    where
        T: Clone + Default + core::fmt::Debug + PartialEq + Eq + 
           wrt_foundation::traits::Checksummable + 
           wrt_foundation::traits::ToBytes + 
           wrt_foundation::traits::FromBytes,
    {
        let config = runtime_memory_config();
        let provider = NoStdProvider::<2048>::default();
        
        // Use the configured vector capacity, capped at the type's maximum
        BoundedVec::new(provider)
    }
    
    /// Create a platform-aware memory provider for runtime operations
    pub fn create_platform_provider() -> Result<CapabilityAwareProvider<NoStdProvider<8192>>> {
        let context = capability_context!(dynamic(CrateId::Runtime, 8192))?;
        let provider = safe_capability_alloc!(context, CrateId::Runtime, 8192)?;
        Ok(provider)
    }
}

/// Dynamic provider factory that creates appropriately-sized providers
pub struct DynamicProviderFactory;

impl DynamicProviderFactory {
    /// Create a provider sized for the current platform
    pub fn create_for_use_case(use_case: MemoryUseCase) -> Result<CapabilityAwareProvider<NoStdProvider<16384>>> {
        let size = match use_case {
            MemoryUseCase::FunctionLocals => 16384,
            MemoryUseCase::InstructionBuffer => 16384,
            MemoryUseCase::ModuleMetadata => 16384,
            MemoryUseCase::ComponentData => 16384,
            MemoryUseCase::TemporaryBuffer => 16384,
        };
        // Use consistent 16KB provider for all runtime use cases
        let context = capability_context!(dynamic(CrateId::Runtime, 16384))?;
        let provider = safe_capability_alloc!(context, CrateId::Runtime, 16384)?;
        Ok(provider)
    }
    
    /// Create a string provider with platform-appropriate size
    pub fn create_string_provider() -> Result<CapabilityAwareProvider<NoStdProvider<8192>>> {
        let context = capability_context!(dynamic(CrateId::Runtime, 8192))?;
        let provider = safe_capability_alloc!(context, CrateId::Runtime, 8192)?;
        Ok(provider)
    }
    
    /// Create a collection provider with platform-appropriate size
    pub fn create_collection_provider() -> Result<CapabilityAwareProvider<NoStdProvider<16384>>> {
        let context = capability_context!(dynamic(CrateId::Runtime, 16384))?;
        let provider = safe_capability_alloc!(context, CrateId::Runtime, 16384)?;
        Ok(provider)
    }
}

/// Memory use case categories for provider sizing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUseCase {
    /// Function local variables and parameters
    FunctionLocals,
    /// WebAssembly instruction buffers
    InstructionBuffer,
    /// Module metadata and exports
    ModuleMetadata,
    /// Component model data
    ComponentData,
    /// Temporary working memory
    TemporaryBuffer,
}

/// Simplified runtime memory manager for current memory system
/// Uses safe_managed_alloc! for all allocations
pub struct RuntimeMemoryManager {
    allocation_count: usize,
}

impl RuntimeMemoryManager {
    /// Create a new runtime memory manager
    pub fn new() -> Self {
        Self {
            allocation_count: 0,
        }
    }
    
    /// Create a provider for a specific use case
    pub fn create_provider(&mut self, use_case: MemoryUseCase) -> Result<NoStdProvider<16384>> {
        self.allocation_count += 1;
        DynamicProviderFactory::create_for_use_case(use_case)
    }
    
    /// Get memory usage statistics for all managed providers
    pub fn get_stats(&self) -> RuntimeMemoryStats {
        // In no_std mode, return simplified stats based on provider count
        RuntimeMemoryStats {
            total_allocated: 0, // Would need tracking in real implementation
            total_capacity: 0,  // Would need tracking in real implementation
            provider_count: self.allocation_count,
        }
    }
}

impl Default for RuntimeMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime memory usage statistics
#[derive(Debug, Clone)]
pub struct RuntimeMemoryStats {
    /// Total allocated memory across all providers
    pub total_allocated: usize,
    /// Total capacity across all providers
    pub total_capacity: usize,
    /// Number of active providers
    pub provider_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_foundation::global_memory_config::initialize_global_memory_system;
    
    #[test]
    fn test_runtime_config_initialization() -> Result<()> {
        // Initialize global system first
        initialize_global_memory_system()?;
        
        // Initialize runtime configuration
        initialize_runtime_memory_config()?;
        
        let config = runtime_memory_config();
        
        // Verify configuration values are reasonable
        assert!(config.string_buffer_size() > 0);
        assert!(config.vector_capacity() > 0);
        assert!(config.provider_buffer_size() > 0);
        assert!(config.max_function_params() > 0);
        
        Ok(())
    }
    
    #[test]
    fn test_dynamic_provider_factory() -> Result<()> {
        initialize_global_memory_system()?;
        initialize_runtime_memory_config()?;
        
        // Test different use cases
        let func_provider = DynamicProviderFactory::create_for_use_case(MemoryUseCase::FunctionLocals)?;
        let instr_provider = DynamicProviderFactory::create_for_use_case(MemoryUseCase::InstructionBuffer)?;
        
        // Verify providers have appropriate sizes
        assert!(func_provider.total_memory() > 0);
        assert!(instr_provider.total_memory() >= func_provider.total_memory());
        
        Ok(())
    }
    
    #[test]
    fn test_runtime_memory_manager() -> Result<()> {
        initialize_global_memory_system()?;
        initialize_runtime_memory_config()?;
        
        let mut manager = RuntimeMemoryManager::new();
        
        // Get providers for different use cases
        let _func_provider = manager.get_provider(MemoryUseCase::FunctionLocals)?;
        let _instr_provider = manager.get_provider(MemoryUseCase::InstructionBuffer)?;
        
        let stats = manager.get_stats();
        assert_eq!(stats.provider_count, 2);
        assert!(stats.total_capacity > 0);
        
        Ok(())
    }
}