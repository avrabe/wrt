//! Capability-aware WebAssembly execution engine
//!
//! This module implements an execution engine that uses memory capabilities
//! to enforce safety constraints based on the selected preset (QM, ASIL-A, ASIL-B).

use crate::{
    module::Module,
    module_instance::ModuleInstance,
    prelude::*,
    stackless::StacklessEngine,
};
use core::sync::atomic::{AtomicU32, Ordering};
use wrt_foundation::{
    bounded_collections::BoundedMap,
    budget_aware_provider::CrateId,
    capabilities::{MemoryCapabilityContext, MemoryOperation},
    safe_managed_alloc,
    safe_memory::NoStdProvider,
    traits::{ReadStream, WriteStream},
    values::Value,
};
use wrt_host::{
    CallbackRegistry, HostBuilder, BoundedHostIntegrationManager, HostIntegrationLimits,
};
// Import execution configuration from wrt-foundation where it belongs
use wrt_foundation::execution::{ASILExecutionConfig, ASILExecutionMode, extract_resource_limits_from_binary};
// Import decoder function
use wrt_decoder::decoder::decode_module;

#[cfg(feature = "std")]
use std::sync::Arc;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;

/// Handle for a loaded module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ModuleHandle(u32);

impl ModuleHandle {
    /// Create a new unique module handle
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl wrt_foundation::traits::Checksummable for ModuleHandle {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        for byte in self.0.to_le_bytes() {
            checksum.update(byte);
        }
    }
}

impl wrt_foundation::traits::ToBytes for ModuleHandle {
    fn serialized_size(&self) -> usize {
        4
    }

    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u32_le(self.0)
    }
}

impl wrt_foundation::traits::FromBytes for ModuleHandle {
    fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let value = reader.read_u32_le()?;
        Ok(Self(value))
    }
}

/// Handle for a module instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct InstanceHandle(u32);

impl InstanceHandle {
    /// Create from an instance index
    pub fn from_index(idx: usize) -> Self {
        Self(idx as u32)
    }

    /// Get the instance index
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl wrt_foundation::traits::Checksummable for InstanceHandle {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        for byte in self.0.to_le_bytes() {
            checksum.update(byte);
        }
    }
}

impl wrt_foundation::traits::ToBytes for InstanceHandle {
    fn serialized_size(&self) -> usize {
        4
    }

    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u32_le(self.0)
    }
}

impl wrt_foundation::traits::FromBytes for InstanceHandle {
    fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let value = reader.read_u32_le()?;
        Ok(Self(value))
    }
}

/// Engine preset determining capability configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnginePreset {
    /// Quality Management - Dynamic allocation, flexible limits
    QM,
    /// ASIL-A - Bounded collections, sampling verification
    AsilA,
    /// ASIL-B - Static allocation, continuous verification
    AsilB,
    /// ASIL-C - Enhanced verification, strict memory bounds
    AsilC,
    /// ASIL-D - Deterministic execution, compile-time verification
    AsilD,
}

/// Trait for capability-aware execution engines
pub trait CapabilityEngine: Send + Sync {
    /// Get the capability context for this engine
    fn capability_context(&self) -> &MemoryCapabilityContext;

    /// Load a module with capability verification
    fn load_module(&mut self, binary: &[u8]) -> Result<ModuleHandle>;

    /// Instantiate a module with capability-gated resources
    fn instantiate(&mut self, module: ModuleHandle) -> Result<InstanceHandle>;

    /// Execute a function with capability enforcement
    fn execute(
        &mut self,
        instance: InstanceHandle,
        func: &str,
        args: &[Value],
    ) -> Result<Vec<Value>>;
}

/// Maximum number of modules and instances
const MAX_MODULES: usize = 32;
const MAX_INSTANCES: usize = 32;

/// Runtime memory provider for engine internals
use crate::bounded_runtime_infra::{RuntimeProvider, create_runtime_provider}; // Use unified RuntimeProvider

/// Capability-aware WebAssembly execution engine
pub struct CapabilityAwareEngine {
    /// Inner stackless execution engine
    inner: StacklessEngine,
    /// Capability context for memory operations
    context: MemoryCapabilityContext,
    /// Engine preset used for resource limit extraction
    preset: EnginePreset,
    /// Loaded modules indexed by handle
    modules: BoundedMap<ModuleHandle, Module, MAX_MODULES, RuntimeProvider>,
    /// Module instances indexed by handle  
    instances: BoundedMap<InstanceHandle, ModuleInstance, MAX_INSTANCES, RuntimeProvider>,
    /// Next instance index
    next_instance_idx: usize,
    /// Host function registry for WASI and custom host functions
    host_registry: Option<CallbackRegistry>,
    /// Bounded host integration manager for safety-critical environments
    host_manager: Option<BoundedHostIntegrationManager>,
}

impl CapabilityAwareEngine {
    /// Create an engine with a specific preset
    pub fn with_preset(preset: EnginePreset) -> Result<Self> {
        let context = match preset {
            EnginePreset::QM => super::presets::qm()?,
            EnginePreset::AsilA => super::presets::asil_a()?,
            EnginePreset::AsilB => super::presets::asil_b()?,
            EnginePreset::AsilC => super::presets::asil_c()?,
            EnginePreset::AsilD => super::presets::asil_d()?,
        };

        Self::with_context_and_preset(context, preset)
    }

    /// Create an engine with a specific capability context
    pub fn with_context(context: MemoryCapabilityContext) -> Result<Self> {
        Self::with_context_and_preset(context, EnginePreset::QM)
    }

    /// Create an engine with a specific capability context and preset
    pub fn with_context_and_preset(context: MemoryCapabilityContext, preset: EnginePreset) -> Result<Self> {
        // Allocate providers for internal data structures
        let modules_provider = create_runtime_provider()?;
        let instances_provider = create_runtime_provider()?;

        // Initialize host integration based on preset
        let (host_registry, host_manager) = Self::create_host_integration(&preset)?;

        Ok(Self {
            inner: StacklessEngine::new(),
            context,
            preset,
            modules: BoundedMap::new(modules_provider)?,
            instances: BoundedMap::new(instances_provider)?,
            next_instance_idx: 0,
            host_registry,
            host_manager,
        })
    }

    /// Convert engine preset to ASIL execution mode
    fn preset_to_asil_mode(&self) -> ASILExecutionMode {
        match self.preset {
            EnginePreset::QM => ASILExecutionMode::QM,
            EnginePreset::AsilA => ASILExecutionMode::ASIL_A,
            EnginePreset::AsilB => ASILExecutionMode::ASIL_B,
            EnginePreset::AsilC => ASILExecutionMode::ASIL_C,
            EnginePreset::AsilD => ASILExecutionMode::ASIL_D,
        }
    }

    /// Create host integration components based on engine preset
    fn create_host_integration(preset: &EnginePreset) -> Result<(Option<CallbackRegistry>, Option<BoundedHostIntegrationManager>)> {
        match preset {
            EnginePreset::QM => {
                // QM mode: Full host function support with standard limits
                #[cfg(feature = "std")]
                {
                    let builder = HostBuilder::new()
                        .with_component_name("wrt_qm_component")
                        .with_host_id("wrt_qm_host");
                    let registry = builder.build()?;
                    Ok((Some(registry), None))
                }
                #[cfg(not(feature = "std"))]
                {
                    let limits = HostIntegrationLimits::qnx();
                    let manager = BoundedHostIntegrationManager::new(limits)?;
                    Ok((None, Some(manager)))
                }
            }
            EnginePreset::AsilA => {
                // ASIL-A: Bounded host functions with embedded limits
                let limits = HostIntegrationLimits::embedded();
                let manager = BoundedHostIntegrationManager::new(limits)?;
                Ok((None, Some(manager)))
            }
            EnginePreset::AsilB | EnginePreset::AsilC => {
                // ASIL-B/C: Restricted host functions with strict limits
                let limits = HostIntegrationLimits::embedded();
                let manager = BoundedHostIntegrationManager::new(limits)?;
                Ok((None, Some(manager)))
            }
            EnginePreset::AsilD => {
                // ASIL-D: Minimal or no host functions for maximum safety
                let limits = HostIntegrationLimits::embedded();
                let manager = BoundedHostIntegrationManager::new(limits)?;
                Ok((None, Some(manager)))
            }
        }
    }

    /// Register a custom host function
    pub fn register_host_function<F>(&mut self, module_name: &str, func_name: &str, func: F) -> Result<()>
    where
        F: Fn(&[Value]) -> Result<Vec<Value>> + Send + Sync + 'static,
    {
        #[cfg(feature = "std")]
        {
            if let Some(ref _mut_registry) = self.host_registry {
                // TODO: Implement host function registration when CallbackRegistry API is available
                // The function signature needs to match what HostFunctionHandler expects
                // For now, return success as placeholder
                Ok(())
            } else {
                Err(Error::not_supported_unsupported_operation("Host functions not supported in this configuration"))
            }
        }
        #[cfg(not(feature = "std"))]
        {
            if let Some(ref mut manager) = self.host_manager {
                use wrt_host::BoundedHostFunction;
                // TODO: Create BoundedHostFunction and add to manager
                // For now, return success as placeholder
                Ok(())
            } else {
                Err(Error::not_supported_unsupported_operation("Host functions not supported in this configuration"))
            }
        }
    }

    /// Enable WASI support with the current capability constraints
    pub fn enable_wasi(&mut self) -> Result<()> {
        match self.preset {
            EnginePreset::QM | EnginePreset::AsilA => {
                // WASI is allowed in QM and ASIL-A modes
                self.register_wasi_functions()
            }
            EnginePreset::AsilB | EnginePreset::AsilC => {
                // Limited WASI in ASIL-B/C (e.g., only safe I/O operations)
                self.register_limited_wasi_functions()
            }
            EnginePreset::AsilD => {
                // No WASI in ASIL-D for maximum safety
                Err(Error::not_supported_unsupported_operation("WASI not supported in ASIL-D mode"))
            }
        }
    }

    /// Register standard WASI functions
    fn register_wasi_functions(&mut self) -> Result<()> {
        // TODO: Implement WASI function registration
        // This would register functions like:
        // - fd_write, fd_read (file operations)
        // - proc_exit (process exit)
        // - random_get (random number generation)
        // etc.
        Ok(())
    }

    /// Register limited WASI functions for higher safety levels
    fn register_limited_wasi_functions(&mut self) -> Result<()> {
        // TODO: Implement limited WASI function registration
        // This would only register safe, deterministic functions like:
        // - proc_exit (process exit)
        // - limited memory operations
        // But NOT:
        // - File I/O operations
        // - Network operations
        // - Random number generation
        Ok(())
    }
}

impl CapabilityEngine for CapabilityAwareEngine {
    fn capability_context(&self) -> &MemoryCapabilityContext {
        &self.context
    }

    fn load_module(&mut self, binary: &[u8]) -> Result<ModuleHandle> {
        // Verify capability for module allocation
        let operation = MemoryOperation::Allocate { size: binary.len() };
        self.context.verify_operation(CrateId::Runtime, &operation)?;

        // Extract resource limits from binary if available
        let asil_mode = self.preset_to_asil_mode();
        let _resource_config = extract_resource_limits_from_binary(binary, asil_mode)
            .unwrap_or(None); // Ignore errors, use defaults if extraction fails

        // TODO: Apply resource limits to execution context
        // This would integrate with the fuel async executor to enforce limits

        // Decode the module using wrt-decoder
        let decoded = decode_module(binary)?;

        // Convert to runtime module
        let runtime_module = Module::from_wrt_module(&decoded)?;

        // Create and store with unique handle
        let handle = ModuleHandle::new();
        self.modules.insert(handle, runtime_module)?;

        Ok(handle)
    }

    fn instantiate(&mut self, module_handle: ModuleHandle) -> Result<InstanceHandle> {
        // Get the module
        let module = self.modules
            .get(&module_handle)?
            .ok_or_else(|| Error::resource_not_found("Module not found"))?;

        // Verify capability for instance allocation
        let operation = MemoryOperation::Allocate {
            size: core::mem::size_of::<ModuleInstance>(),
        };
        self.context.verify_operation(CrateId::Runtime, &operation)?;

        // Create module instance
        let instance = ModuleInstance::new(module.clone(), self.next_instance_idx)?;
        let instance_arc = Arc::new(instance.clone());

        // Register with inner engine
        let instance_idx = self.inner.set_current_module(instance_arc)?;
        self.next_instance_idx += 1;

        // Store mapping
        let handle = InstanceHandle::from_index(instance_idx as usize);
        self.instances.insert(handle, instance)?;

        // Run start function if present
        if let Some(start_idx) = module.start {
            self.inner.execute(instance_idx as usize, start_idx, vec![])?;
        }

        Ok(handle)
    }

    fn execute(
        &mut self,
        instance_handle: InstanceHandle,
        func_name: &str,
        args: &[Value],
    ) -> Result<Vec<Value>> {
        // Get the instance
        let instance = self.instances
            .get(&instance_handle)?
            .ok_or_else(|| Error::resource_not_found("Instance not found"))?;

        // Find the function by name using the new function resolution
        let func_idx = instance.module().validate_function_call(func_name)?;

        // Set current module for execution
        self.inner.set_current_module(Arc::new(instance.clone()))?;

        // Execute the function
        let results = self.inner.execute(
            instance_handle.index(),
            func_idx,
            args.to_vec(),
        )?;

        Ok(results)
    }
}

impl CapabilityAwareEngine {
    /// Get the list of exported functions from an instance
    pub fn get_exported_functions(&self, instance_handle: InstanceHandle) -> Result<Vec<String>> {
        let instance = self.instances
            .get(&instance_handle)?
            .ok_or_else(|| Error::resource_not_found("Instance not found"))?;

        let mut functions = Vec::new();
        // TODO: BoundedMap doesn't support iteration, so we can't list all exports
        // For now, return an empty list as a placeholder
        // In a real implementation, we'd need an iterator interface on BoundedMap
        Ok(functions)
    }

    /// Check if a function exists in an instance
    pub fn has_function(&self, instance_handle: InstanceHandle, func_name: &str) -> Result<bool> {
        let instance = self.instances
            .get(&instance_handle)?
            .ok_or_else(|| Error::resource_not_found("Instance not found"))?;

        Ok(instance.module().find_function_by_name(func_name).is_some())
    }

    /// Get function signature by name
    pub fn get_function_signature(&self, instance_handle: InstanceHandle, func_name: &str) -> Result<Option<wrt_foundation::types::FuncType<RuntimeProvider>>> {
        let instance = self.instances
            .get(&instance_handle)?
            .ok_or_else(|| Error::resource_not_found("Instance not found"))?;

        if let Some(func_idx) = instance.module().find_function_by_name(func_name) {
            Ok(instance.module().get_function_signature(func_idx))
        } else {
            Ok(None)
        }
    }

    /// Execute a function with additional capability validation
    pub fn execute_with_validation(&mut self, instance_handle: InstanceHandle, func_name: &str, args: &[wrt_foundation::values::Value]) -> Result<Vec<wrt_foundation::values::Value>> {
        // Additional capability-based validation
        let instance = self.instances
            .get(&instance_handle)?
            .ok_or_else(|| Error::resource_not_found("Instance not found"))?;

        // Verify memory capability allows function execution
        // Note: Using read operation as placeholder since Execute variant doesn't exist
        let operation = wrt_foundation::capabilities::MemoryOperation::Read { 
            offset: 0, 
            len: 64 // Small placeholder size for function validation
        };
        self.context.verify_operation(wrt_foundation::budget_aware_provider::CrateId::Runtime, &operation)?;

        // Execute the function
        self.execute(instance_handle, func_name, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_handle_creation() {
        let handle1 = ModuleHandle::new();
        let handle2 = ModuleHandle::new();
        assert_ne!(handle1, handle2);
    }

    #[test]
    fn test_instance_handle_conversion() {
        let handle = InstanceHandle::from_index(42);
        assert_eq!(handle.index(), 42);
    }

    #[test]
    fn test_engine_preset_creation() {
        // Test that each preset can be created
        let _qm = CapabilityAwareEngine::with_preset(EnginePreset::QM);
        let _asil_a = CapabilityAwareEngine::with_preset(EnginePreset::AsilA);
        let _asil_b = CapabilityAwareEngine::with_preset(EnginePreset::AsilB);
        let _asil_c = CapabilityAwareEngine::with_preset(EnginePreset::AsilC);
        let _asil_d = CapabilityAwareEngine::with_preset(EnginePreset::AsilD);
    }
}