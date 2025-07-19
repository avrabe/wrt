//! Component execution engine for WebAssembly Component Model
//!
//! This module provides the execution environment for WebAssembly components,
//! handling function calls, resource management, and interface interactions.

#[cfg(feature = "std")]
use std::{boxed::Box, format, string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{format, vec, string::String, boxed::Box};

#[cfg(not(feature = "std"))]
use wrt_foundation::{
    BoundedVec as Vec, 
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

#[cfg(feature = "std")]
use wrt_foundation::{bounded::BoundedVec, component_value::ComponentValue, prelude::*};

use crate::{
    components::component::{Component, ComponentInstance, ComponentType, ExportType, ImportType},
    memory_layout::MemoryLayout,
    string_encoding::StringEncoding,
    types::{ValType, Value},
    unified_execution_agent_stubs::{
        CanonicalAbi, CanonicalOptions, ResourceHandle, ResourceLifecycleManager,
        ComponentRuntimeBridge, RuntimeBridgeConfig,
    },
    WrtResult,
};

// Temporary module alias for canonical_abi types
mod canonical_abi {
    #[cfg(feature = "std")]
    pub use wrt_foundation::component_value::ComponentValue;
    #[cfg(not(feature = "std"))]
    pub use crate::types::Value as ComponentValue;
    pub use crate::types::ValType as ComponentType;
}

/// Maximum number of call frames in no_std environments
const MAX_CALL_FRAMES: usize = 256;

/// Maximum number of imported functions in no_std environments
const MAX_IMPORTS: usize = 512;

/// Represents a call frame in the component execution stack
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// The component instance being executed
    pub instance_id: u32,
    /// The function being called
    pub function_index: u32,
    /// Local variables for this frame
    #[cfg(feature = "std")]
    pub locals: Vec<Value>,
    #[cfg(not(any(feature = "std", )))]
    pub locals: Vec<Value>,
    /// Return address information
    pub return_address: Option<u32>,
}

impl CallFrame {
    /// Create a new call frame
    pub fn new(instance_id: u32, function_index: u32) -> WrtResult<Self> {
        Ok(Self {
            instance_id,
            function_index,
            #[cfg(feature = "std")]
            locals: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            locals: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Failed to allocate locals vector")
                })?
            },
            return_address: None,
        })
    }

    /// Push a local variable
    pub fn push_local(&mut self, value: Value) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.locals.push(value);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.locals.push(value).map_err(|_| 
                wrt_error::Error::resource_exhausted("Too many local variables")
            )?
        }
    }

    /// Get a local variable by index
    pub fn get_local(&self, index: usize) -> WrtResult<&Value> {
        self.locals.get(index).ok_or_else(|| {
            wrt_error::Error::validation_invalid_input("Invalid input")
        })
    }

    /// Set a local variable by index
    pub fn set_local(&mut self, index: usize, value: Value) -> WrtResult<()> {
        if index < self.locals.len() {
            self.locals[index] = value;
            Ok(())
        } else {
            Err(wrt_error::Error::validation_invalid_input("Invalid input"))
        }
    }
}

/// Host function callback trait
pub trait HostFunction {
    /// Call the host function with the given arguments
    fn call(&mut self, args: &[Value]) -> WrtResult<Value>;

    /// Get the function signature
    fn signature(&self) -> &ComponentType;
}

/// Component execution engine
pub struct ComponentExecutionEngine {
    /// Call stack
    #[cfg(feature = "std")]
    call_stack: Vec<CallFrame>,
    #[cfg(not(any(feature = "std", )))]
    call_stack: Vec<CallFrame>,

    /// Canonical ABI processor
    canonical_abi: CanonicalAbi,

    /// Resource lifecycle manager
    resource_manager: ResourceLifecycleManager,

    /// Runtime bridge for WebAssembly Core integration
    runtime_bridge: ComponentRuntimeBridge,

    /// Host function registry (legacy - now handled by runtime bridge)
    #[cfg(feature = "std")]
    host_functions: Vec<Box<dyn HostFunction>>,
    #[cfg(not(any(feature = "std", )))]
    host_functions: Vec<fn(&[Value]) -> WrtResult<Value>>,

    /// Current component instance
    current_instance: Option<u32>,

    /// Execution state
    state: ExecutionState,
}

/// Execution state of the engine
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    /// Ready to execute
    Ready,
    /// Currently executing
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed with error
    Failed,
    /// Execution suspended (for async operations)
    Suspended,
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionState::Ready => write!(f, "Ready"),
            ExecutionState::Running => write!(f, "Running"),
            ExecutionState::Completed => write!(f, "Completed"),
            ExecutionState::Failed => write!(f, "Failed"),
            ExecutionState::Suspended => write!(f, "Suspended"),
        }
    }
}

impl ComponentExecutionEngine {
    /// Create a new component execution engine
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            call_stack: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            call_stack: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Failed to allocate call stack")
                })?
            },
            canonical_abi: CanonicalAbi::new(),
            resource_manager: ResourceLifecycleManager::new(),
            runtime_bridge: ComponentRuntimeBridge::new(),
            #[cfg(feature = "std")]
            host_functions: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            host_functions: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Failed to allocate host functions")
                })?
            },
            current_instance: None,
            state: ExecutionState::Ready,
        })
    }

    /// Create a new component execution engine with custom runtime bridge configuration
    pub fn with_runtime_config(bridge_config: RuntimeBridgeConfig) -> WrtResult<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            call_stack: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            call_stack: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Failed to allocate call stack")
                })?
            },
            canonical_abi: CanonicalAbi::new(),
            resource_manager: ResourceLifecycleManager::new(),
            runtime_bridge: ComponentRuntimeBridge::with_config(bridge_config),
            #[cfg(feature = "std")]
            host_functions: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            host_functions: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    wrt_error::Error::resource_exhausted("Failed to allocate host functions")
                })?
            },
            current_instance: None,
            state: ExecutionState::Ready,
        })
    }

    /// Register a host function
    #[cfg(feature = "std")]
    pub fn register_host_function(&mut self, func: Box<dyn HostFunction>) -> WrtResult<u32> {
        let index = self.host_functions.len() as u32;
        self.host_functions.push(func);
        Ok(index)
    }

    /// Register a host function (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn register_host_function(
        &mut self,
        func: fn(&[Value]) -> WrtResult<Value>,
    ) -> WrtResult<u32> {
        let index = self.host_functions.len() as u32;
        self.host_functions.push(func).map_err(|_| 
            wrt_error::Error::resource_exhausted("Too many host functions")
        )?;
        Ok(index)
    }

    /// Call a component function
    pub fn call_function(
        &mut self,
        instance_id: u32,
        function_index: u32,
        args: &[Value],
    ) -> WrtResult<Value> {
        self.state = ExecutionState::Running;
        self.current_instance = Some(instance_id;

        // Create new call frame
        let mut frame = CallFrame::new(instance_id, function_index)?;

        // Push arguments as locals
        for arg in args {
            frame.push_local(arg.clone())?;
        }

        // Push frame to call stack
        #[cfg(feature = "std")]
        {
            self.call_stack.push(frame);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.call_stack.push(frame).map_err(|_| 
                wrt_error::Error::resource_exhausted("Call stack overflow")
            )?;
        }

        // Execute the function
        let result = self.execute_function_internal(function_index, args;

        // Pop the frame
        #[cfg(feature = "std")]
        {
            self.call_stack.pop);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let _ = self.call_stack.pop);
        }

        // Update state based on result
        match &result {
            Ok(_) => self.state = ExecutionState::Completed,
            Err(_) => self.state = ExecutionState::Failed,
        }

        self.current_instance = None;
        result
    }

    /// Execute function internal implementation
    fn execute_function_internal(
        &mut self,
        function_index: u32,
        args: &[Value],
    ) -> WrtResult<Value> {
        // Get current instance ID
        let instance_id = self.current_instance.ok_or_else(|| 
            wrt_error::Error::runtime_error("No current instance set")
        )?;

        // Convert component values to canonical ABI format
        let component_values = self.convert_values_to_component(args)?;

        // Delegate to runtime bridge for execution
        let function_name = {
            #[cfg(feature = "std")]
            {
                alloc::format!("func_{}", function_index)
            }
            #[cfg(not(any(feature = "std", )))]
            {
                let mut name = wrt_foundation::bounded::BoundedString::new);
                let _ = name.push_str("func_";
                name
            }
        };
        let result = self.runtime_bridge
            .execute_component_function(instance_id, &function_name, &component_values)
            .map_err(|_| wrt_error::Error::runtime_error("Failed to execute component function"))?;

        // Convert result back to engine value format
        self.convert_component_value_to_value(&result)
    }

    /// Call a host function
    pub fn call_host_function(&mut self, index: u32, args: &[Value]) -> WrtResult<Value> {
        #[cfg(feature = "std")]
        {
            if let Some(func) = self.host_functions.get_mut(index as usize) {
                func.call(args)
            } else {
                Err(wrt_error::Error::validation_invalid_input("Invalid input"))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            if let Some(func) = self.host_functions.get(index as usize) {
                func(args)
            } else {
                Err(wrt_error::Error::validation_invalid_input("Invalid input"))
            }
        }
    }

    /// Get the current execution state
    pub fn state(&self) -> &ExecutionState {
        &self.state
    }

    /// Get the current call stack depth
    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// Get the current instance ID
    pub fn current_instance(&self) -> Option<u32> {
        self.current_instance
    }

    /// Create a new resource
    pub fn create_resource(
        &mut self,
        type_id: u32,
        data: ComponentValue,
    ) -> WrtResult<ResourceHandle> {
        self.resource_manager.create_resource(type_id, data)
    }

    /// Drop a resource
    pub fn drop_resource(&mut self, handle: ResourceHandle) -> WrtResult<()> {
        self.resource_manager.drop_resource(handle)
    }

    /// Borrow a resource
    pub fn borrow_resource(&mut self, handle: ResourceHandle) -> WrtResult<&ComponentValue> {
        self.resource_manager.borrow_resource(handle)
    }

    /// Transfer resource ownership
    pub fn transfer_resource(&mut self, handle: ResourceHandle, new_owner: u32) -> WrtResult<()> {
        self.resource_manager.transfer_ownership(handle, new_owner)
    }

    /// Reset the execution engine
    pub fn reset(&mut self) {
        #[cfg(feature = "std")]
        {
            self.call_stack.clear);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.call_stack.clear);
        }

        self.current_instance = None;
        self.state = ExecutionState::Ready;
        self.resource_manager = ResourceLifecycleManager::new);
    }

    /// Get canonical ABI processor
    pub fn canonical_abi(&self) -> &CanonicalAbi {
        &self.canonical_abi
    }

    /// Get mutable canonical ABI processor
    pub fn canonical_abi_mut(&mut self) -> &mut CanonicalAbi {
        &mut self.canonical_abi
    }

    /// Get resource manager
    pub fn resource_manager(&self) -> &ResourceLifecycleManager {
        &self.resource_manager
    }

    /// Get mutable resource manager
    pub fn resource_manager_mut(&mut self) -> &mut ResourceLifecycleManager {
        &mut self.resource_manager
    }

    /// Get runtime bridge
    pub fn runtime_bridge(&self) -> &ComponentRuntimeBridge {
        &self.runtime_bridge
    }

    /// Get mutable runtime bridge
    pub fn runtime_bridge_mut(&mut self) -> &mut ComponentRuntimeBridge {
        &mut self.runtime_bridge
    }

    /// Convert engine values to component values
    #[cfg(feature = "std")]
    fn convert_values_to_component(&self, values: &[Value]) -> WrtResult<Vec<crate::canonical_abi::ComponentValue>> {
        let mut component_values = Vec::new);
        for value in values {
            let component_value = self.convert_value_to_component(value)?;
            component_values.push(component_value);
        }
        Ok(component_values)
    }

    /// Convert engine values to component values (no_std version)
    #[cfg(not(any(feature = "std", )))]
    fn convert_values_to_component(&self, values: &[Value]) -> WrtResult<Vec<crate::canonical_abi::ComponentValue>> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let mut component_values = BoundedVec::new(provider).map_err(|_| {
            wrt_error::Error::resource_exhausted("Failed to allocate component values vector")
        })?;
        for value in values {
            let component_value = self.convert_value_to_component(value)?;
            component_values.push(component_value).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many component values")
            })?;
        }
        Ok(component_values)
    }

    /// Convert a single engine value to component value
    fn convert_value_to_component(&self, value: &Value) -> WrtResult<crate::canonical_abi::ComponentValue> {
        use crate::canonical_abi::ComponentValue;
        match value {
            Value::Bool(b) => Ok(ComponentValue::Bool(*b)),
            Value::U8(v) => Ok(ComponentValue::U8(*v)),
            Value::U16(v) => Ok(ComponentValue::U16(*v)),
            Value::U32(v) => Ok(ComponentValue::U32(*v)),
            Value::U64(v) => Ok(ComponentValue::U64(*v)),
            Value::S8(v) => Ok(ComponentValue::S8(*v)),
            Value::S16(v) => Ok(ComponentValue::S16(*v)),
            Value::S32(v) => Ok(ComponentValue::S32(*v)),
            Value::S64(v) => Ok(ComponentValue::S64(*v)),
            Value::F32(v) => Ok(ComponentValue::F32(*v)),
            Value::F64(v) => Ok(ComponentValue::F64(*v)),
            Value::Char(c) => Ok(ComponentValue::Char(*c)),
            Value::String(s) => Ok(ComponentValue::String(s.clone())),
            _ => Err(wrt_error::Error::validation_invalid_input("Invalid input")),
        }
    }

    /// Convert component value back to engine value
    fn convert_component_value_to_value(&self, component_value: &crate::canonical_abi::ComponentValue) -> WrtResult<Value> {
        use crate::canonical_abi::ComponentValue;
        match component_value {
            ComponentValue::Bool(b) => Ok(Value::Bool(*b)),
            ComponentValue::U8(v) => Ok(Value::U8(*v)),
            ComponentValue::U16(v) => Ok(Value::U16(*v)),
            ComponentValue::U32(v) => Ok(Value::U32(*v)),
            ComponentValue::U64(v) => Ok(Value::U64(*v)),
            ComponentValue::S8(v) => Ok(Value::S8(*v)),
            ComponentValue::S16(v) => Ok(Value::S16(*v)),
            ComponentValue::S32(v) => Ok(Value::S32(*v)),
            ComponentValue::S64(v) => Ok(Value::S64(*v)),
            ComponentValue::F32(v) => Ok(Value::F32(*v)),
            ComponentValue::F64(v) => Ok(Value::F64(*v)),
            ComponentValue::Char(c) => Ok(Value::Char(*c)),
            ComponentValue::String(s) => Ok(Value::String(s.clone())),
            _ => Err(wrt_error::Error::validation_invalid_input("Invalid input")),
        }
    }

    /// Register a component instance with the runtime bridge
    pub fn register_component_instance(
        &mut self,
        component_id: u32,
        module_name: &str,
        function_count: u32,
        memory_size: u32,
    ) -> WrtResult<u32> {
        let module_name_string = {
            #[cfg(feature = "std")]
            {
                alloc::string::String::from(module_name)
            }
            #[cfg(not(any(feature = "std", )))]
            {
                wrt_foundation::bounded::BoundedString::from_str(module_name).map_err(|_| 
                    wrt_error::Error::validation_invalid_input("Invalid input")
                )?
            }
        };
        self.runtime_bridge
            .register_component_instance(component_id, module_name_string, function_count, memory_size)
            .map_err(|_| wrt_error::Error::runtime_error("Conversion error")
            )
    }

    /// Register a host function with the runtime bridge
    #[cfg(feature = "std")]
    pub fn register_runtime_host_function<F>(
        &mut self,
        name: &str,
        func: F,
    ) -> WrtResult<usize>
    where
        F: Fn(&[crate::canonical_abi::ComponentValue]) -> core::result::Result<crate::canonical_abi::ComponentValue, wrt_error::Error> + Send + Sync + 'static,
    {
        use crate::canonical_abi::ComponentType;
        
        let name_string = alloc::string::String::from(name;
        let signature = crate::component_instantiation::FunctionSignature {
            name: name_string.clone(),
            params: alloc::vec![ComponentType::S32], // Simplified for now
            returns: alloc::vec![ComponentType::S32],
        };
        
        self.runtime_bridge
            .register_host_function(name_string, signature, func)
            .map_err(|_| wrt_error::Error::runtime_error("Conversion error")
            )
    }

    /// Register a host function with the runtime bridge (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn register_runtime_host_function(
        &mut self,
        name: &str,
        func: fn(&[crate::canonical_abi::ComponentValue]) -> core::result::Result<crate::canonical_abi::ComponentValue, wrt_error::Error>,
    ) -> WrtResult<usize> {
        use crate::canonical_abi::ComponentType;
        
        let name_string = wrt_foundation::bounded::BoundedString::from_str(name).map_err(|_| 
            wrt_error::Error::validation_invalid_input("Invalid input")
        )?;
        
        let signature = crate::component_instantiation::FunctionSignature {
            name: name_string.clone(),
            params: wrt_foundation::bounded::BoundedVec::from_slice(&[ComponentType::S32]).map_err(|_| 
                wrt_error::Error::resource_exhausted("Too many parameters")
            )?,
            returns: wrt_foundation::bounded::BoundedVec::from_slice(&[ComponentType::S32]).map_err(|_| 
                wrt_error::Error::resource_exhausted("Too many return values")
            )?,
        };
        
        self.runtime_bridge
            .register_host_function(name_string, signature, func)
            .map_err(|_| wrt_error::Error::runtime_error("Conversion error")
            )
    }
}

impl Default for ComponentExecutionEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default ComponentExecutionEngine")
    }
}

/// Execution context for component calls
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Memory layout information
    pub memory_layout: MemoryLayout,
    /// String encoding options
    pub string_encoding: StringEncoding,
    /// Canonical options
    pub canonical_options: CanonicalOptions,
    /// Maximum call depth
    pub max_call_depth: u32,
    /// Maximum memory usage
    pub max_memory: u32,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self {
            memory_layout: MemoryLayout::new(1, 1),
            string_encoding: StringEncoding::Utf8,
            canonical_options: CanonicalOptions::default(),
            max_call_depth: 1024,
            max_memory: 1024 * 1024, // 1MB default
        }
    }

    /// Set memory layout
    pub fn with_memory_layout(mut self, layout: MemoryLayout) -> Self {
        self.memory_layout = layout;
        self
    }

    /// Set string encoding
    pub fn with_string_encoding(mut self, encoding: StringEncoding) -> Self {
        self.string_encoding = encoding;
        self
    }

    /// Set canonical options
    pub fn with_canonical_options(mut self, options: CanonicalOptions) -> Self {
        self.canonical_options = options;
        self
    }

    /// Set maximum call depth
    pub fn with_max_call_depth(mut self, depth: u32) -> Self {
        self.max_call_depth = depth;
        self
    }

    /// Set maximum memory usage
    pub fn with_max_memory(mut self, memory: u32) -> Self {
        self.max_memory = memory;
        self
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_engine_creation() {
        let engine = ComponentExecutionEngine::new().unwrap();
        assert_eq!(engine.state(), &ExecutionState::Ready;
        assert_eq!(engine.call_stack_depth(), 0;
        assert_eq!(engine.current_instance(), None;
    }

    #[test]
    fn test_call_frame_creation() {
        let frame = CallFrame::new(1, 2).unwrap();
        assert_eq!(frame.instance_id, 1;
        assert_eq!(frame.function_index, 2;
        assert_eq!(frame.locals.len(), 0;
        assert_eq!(frame.return_address, None;
    }

    #[test]
    fn test_call_frame_locals() {
        let mut frame = CallFrame::new(1, 2).unwrap();

        // Test pushing locals
        assert!(frame.push_local(Value::U32(42)).is_ok();
        assert!(frame.push_local(Value::Bool(true)).is_ok();

        // Test getting locals
        assert_eq!(frame.get_local(0).unwrap(), &Value::U32(42;
        assert_eq!(frame.get_local(1).unwrap(), &Value::Bool(true;
        assert!(frame.get_local(2).is_err();

        // Test setting locals
        assert!(frame.set_local(0, Value::U32(100)).is_ok();
        assert_eq!(frame.get_local(0).unwrap(), &Value::U32(100;
        assert!(frame.set_local(10, Value::U32(200)).is_err();
    }

    #[test]
    fn test_execution_context() {
        let context = ExecutionContext::new()
            .with_max_call_depth(512)
            .with_max_memory(2048)
            .with_string_encoding(StringEncoding::Utf16Le;

        assert_eq!(context.max_call_depth, 512;
        assert_eq!(context.max_memory, 2048;
        assert_eq!(context.string_encoding, StringEncoding::Utf16Le;
    }

    #[test]
    fn test_execution_state_display() {
        assert_eq!(ExecutionState::Ready.to_string(), "Ready";
        assert_eq!(ExecutionState::Running.to_string(), "Running";
        assert_eq!(ExecutionState::Completed.to_string(), "Completed";
        assert_eq!(ExecutionState::Failed.to_string(), "Failed";
        assert_eq!(ExecutionState::Suspended.to_string(), "Suspended";
    }

    #[cfg(not(any(feature = "std", )))]
    #[test]
    fn test_host_function_registration_nostd() {
        let mut engine = ComponentExecutionEngine::new().unwrap();

        fn test_func(_args: &[Value]) -> WrtResult<Value> {
            Ok(Value::U32(42))
        }

        let index = engine.register_host_function(test_func).unwrap();
        assert_eq!(index, 0;

        let result = engine.call_host_function(0, &[]).unwrap();
        assert_eq!(result, Value::U32(42;
    }
}
