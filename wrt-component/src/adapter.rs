//! Core module to component adapter
//!
//! This module provides adaptation between WebAssembly core modules and components,
//! allowing core modules to be used within the component model ecosystem.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(feature = "std")]
use std::{boxed::Box, string::String, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, component::ComponentType, component_value::ComponentValue, prelude::*,
};

use crate::{
    canonical::CanonicalAbi,
    component::Component,
    execution_engine::ComponentExecutionEngine,
    types::{ValType, Value},
    WrtResult,
};

/// Maximum number of adapted functions in no_std environments
const MAX_ADAPTED_FUNCTIONS: usize = 256;

/// Adapter that wraps a core WebAssembly module for use in components
#[derive(Debug, Clone)]
pub struct CoreModuleAdapter {
    /// Module name/identifier
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64>,

    /// Function adapters
    #[cfg(feature = "std")]
    pub functions: Vec<FunctionAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub functions: BoundedVec<FunctionAdapter, MAX_ADAPTED_FUNCTIONS>,

    /// Memory adapters
    #[cfg(feature = "std")]
    pub memories: Vec<MemoryAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub memories: BoundedVec<MemoryAdapter, 16>,

    /// Table adapters
    #[cfg(feature = "std")]
    pub tables: Vec<TableAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub tables: BoundedVec<TableAdapter, 16>,

    /// Global adapters
    #[cfg(feature = "std")]
    pub globals: Vec<GlobalAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub globals: BoundedVec<GlobalAdapter, 64>,
}

/// Adapter for core module functions
#[derive(Debug, Clone)]
pub struct FunctionAdapter {
    /// Core function index
    pub core_index: u32,
    /// Component function signature
    pub component_signature: ComponentType,
    /// Core function signature (WebAssembly types)
    pub core_signature: CoreFunctionSignature,
    /// Adaptation mode
    pub mode: AdaptationMode,
}

/// Core WebAssembly function signature
#[derive(Debug, Clone, PartialEq)]
pub struct CoreFunctionSignature {
    /// Parameter types (WebAssembly core types)
    #[cfg(feature = "std")]
    pub params: Vec<CoreValType>,
    #[cfg(not(any(feature = "std", )))]
    pub params: BoundedVec<CoreValType, 32>,
    /// Result types (WebAssembly core types)
    #[cfg(feature = "std")]
    pub results: Vec<CoreValType>,
    #[cfg(not(any(feature = "std", )))]
    pub results: BoundedVec<CoreValType, 8>,
}

/// WebAssembly core value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreValType {
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
    /// 128-bit vector (SIMD)
    V128,
    /// Function reference
    FuncRef,
    /// External reference
    ExternRef,
}

/// Function adaptation mode
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptationMode {
    /// Direct mapping (no adaptation needed)
    Direct,
    /// Lift core types to component types
    Lift,
    /// Lower component types to core types
    Lower,
    /// Bidirectional adaptation
    Bidirectional,
}

/// Memory adapter
#[derive(Debug, Clone)]
pub struct MemoryAdapter {
    /// Core memory index
    pub core_index: u32,
    /// Memory limits
    pub limits: MemoryLimits,
    /// Shared flag
    pub shared: bool,
}

/// Memory limits
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryLimits {
    /// Minimum size in pages
    pub min: u32,
    /// Maximum size in pages (if any)
    pub max: Option<u32>,
}

/// Table adapter
#[derive(Debug, Clone)]
pub struct TableAdapter {
    /// Core table index
    pub core_index: u32,
    /// Element type
    pub element_type: CoreValType,
    /// Table limits
    pub limits: TableLimits,
}

/// Table limits
#[derive(Debug, Clone, PartialEq)]
pub struct TableLimits {
    /// Minimum size
    pub min: u32,
    /// Maximum size (if any)
    pub max: Option<u32>,
}

/// Global adapter
#[derive(Debug, Clone)]
pub struct GlobalAdapter {
    /// Core global index
    pub core_index: u32,
    /// Global type
    pub global_type: CoreValType,
    /// Mutability
    pub mutable: bool,
}

impl CoreModuleAdapter {
    /// Create a new core module adapter
    #[cfg(feature = "std")]
    pub fn new(name: String) -> Self {
        Self {
            name,
            functions: Vec::new(),
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
        }
    }

    /// Create a new core module adapter (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn new(name: BoundedString<64>) -> Self {
        Self {
            name,
            functions: BoundedVec::new(),
            memories: BoundedVec::new(),
            tables: BoundedVec::new(),
            globals: BoundedVec::new(),
        }
    }

    /// Add a function adapter
    pub fn add_function(&mut self, adapter: FunctionAdapter) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.functions.push(adapter);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.functions.push(adapter).map_err(|_| {
                wrt_foundation::WrtError::ResourceExhausted("Too many function adapters".into())
            })
        }
    }

    /// Add a memory adapter
    pub fn add_memory(&mut self, adapter: MemoryAdapter) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.memories.push(adapter);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.memories.push(adapter).map_err(|_| {
                wrt_foundation::WrtError::ResourceExhausted("Too many memory adapters".into())
            })
        }
    }

    /// Add a table adapter
    pub fn add_table(&mut self, adapter: TableAdapter) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.tables.push(adapter);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.tables.push(adapter).map_err(|_| {
                wrt_foundation::WrtError::ResourceExhausted("Too many table adapters".into())
            })
        }
    }

    /// Add a global adapter
    pub fn add_global(&mut self, adapter: GlobalAdapter) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.globals.push(adapter);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.globals.push(adapter).map_err(|_| {
                wrt_foundation::WrtError::ResourceExhausted("Too many global adapters".into())
            })
        }
    }

    /// Get function adapter by index
    pub fn get_function(&self, index: u32) -> Option<&FunctionAdapter> {
        self.functions.get(index as usize)
    }

    /// Get memory adapter by index
    pub fn get_memory(&self, index: u32) -> Option<&MemoryAdapter> {
        self.memories.get(index as usize)
    }

    /// Get table adapter by index
    pub fn get_table(&self, index: u32) -> Option<&TableAdapter> {
        self.tables.get(index as usize)
    }

    /// Get global adapter by index
    pub fn get_global(&self, index: u32) -> Option<&GlobalAdapter> {
        self.globals.get(index as usize)
    }

    /// Convert this adapter to a component
    pub fn to_component(&self) -> WrtResult<Component> {
        let mut component = Component::new();

        // Convert function adapters to component functions
        for func_adapter in &self.functions {
            // Add the function to the component
            // This is simplified - in reality would need more complex conversion
            component.add_function(func_adapter.component_signature.clone())?;
        }

        // Convert memory adapters to component memories
        for mem_adapter in &self.memories {
            component.add_memory(mem_adapter.limits.min, mem_adapter.limits.max)?;
        }

        // Convert table adapters to component tables
        for table_adapter in &self.tables {
            component.add_table(
                self.core_type_to_component_type(table_adapter.element_type),
                table_adapter.limits.min,
                table_adapter.limits.max,
            )?;
        }

        // Convert global adapters to component globals
        for global_adapter in &self.globals {
            component.add_global(
                self.core_type_to_component_type(global_adapter.global_type),
                global_adapter.mutable,
            )?;
        }

        Ok(component)
    }

    /// Convert core type to component type
    fn core_type_to_component_type(&self, core_type: CoreValType) -> ComponentType {
        match core_type {
            CoreValType::I32 => ComponentType::Unit, // Simplified
            CoreValType::I64 => ComponentType::Unit,
            CoreValType::F32 => ComponentType::Unit,
            CoreValType::F64 => ComponentType::Unit,
            CoreValType::V128 => ComponentType::Unit,
            CoreValType::FuncRef => ComponentType::Unit,
            CoreValType::ExternRef => ComponentType::Unit,
        }
    }

    /// Adapt a core function call to component model
    pub fn adapt_function_call(
        &self,
        func_index: u32,
        args: &[Value],
        engine: &mut ComponentExecutionEngine,
    ) -> WrtResult<Value> {
        let adapter = self
            .get_function(func_index)
            .ok_or_else(|| wrt_foundation::WrtError::invalid_input("Invalid input")))?;

        match adapter.mode {
            AdaptationMode::Direct => {
                // Direct call - no adaptation needed
                self.call_core_function_direct(adapter.core_index, args, engine)
            }
            AdaptationMode::Lift => {
                // Lower component args to core args, call, then lift result
                let core_args = self.lower_args_to_core(args, &adapter.core_signature)?;
                let core_result =
                    self.call_core_function_direct(adapter.core_index, &core_args, engine)?;
                self.lift_result_to_component(core_result, &adapter.component_signature)
            }
            AdaptationMode::Lower => {
                // Already have core args, call directly
                let core_result =
                    self.call_core_function_direct(adapter.core_index, args, engine)?;
                self.lift_result_to_component(core_result, &adapter.component_signature)
            }
            AdaptationMode::Bidirectional => {
                // Full bidirectional adaptation
                let core_args = self.lower_args_to_core(args, &adapter.core_signature)?;
                let core_result =
                    self.call_core_function_direct(adapter.core_index, &core_args, engine)?;
                self.lift_result_to_component(core_result, &adapter.component_signature)
            }
        }
    }

    /// Call a core function directly
    fn call_core_function_direct(
        &self,
        _core_index: u32,
        args: &[Value],
        _engine: &mut ComponentExecutionEngine,
    ) -> WrtResult<Value> {
        // Simplified implementation - in reality would call actual core module
        if let Some(first_arg) = args.first() {
            Ok(first_arg.clone())
        } else {
            Ok(Value::U32(0))
        }
    }

    /// Lower component arguments to core arguments
    fn lower_args_to_core(
        &self,
        args: &[Value],
        _core_signature: &CoreFunctionSignature,
    ) -> WrtResult<Vec<Value>> {
        // Simplified lowering - in reality would use canonical ABI
        #[cfg(feature = "std")]
        {
            Ok(args.to_vec())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let mut result = Vec::new();
            for arg in args {
                result.push(arg.clone());
            }
            Ok(result)
        }
    }

    /// Lift core result to component result
    fn lift_result_to_component(
        &self,
        result: Value,
        _component_signature: &ComponentType,
    ) -> WrtResult<Value> {
        // Simplified lifting - in reality would use canonical ABI
        Ok(result)
    }
}

impl FunctionAdapter {
    /// Create a new function adapter
    pub fn new(
        core_index: u32,
        component_signature: ComponentType,
        core_signature: CoreFunctionSignature,
        mode: AdaptationMode,
    ) -> Self {
        Self { core_index, component_signature, core_signature, mode }
    }

    /// Check if this adapter needs canonical ABI processing
    pub fn needs_canonical_abi(&self) -> bool {
        matches!(
            self.mode,
            AdaptationMode::Lift | AdaptationMode::Lower | AdaptationMode::Bidirectional
        )
    }
}

impl CoreFunctionSignature {
    /// Create a new core function signature
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            params: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            params: BoundedVec::new(),
            #[cfg(feature = "std")]
            results: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            results: BoundedVec::new(),
        }
    }

    /// Add a parameter type
    pub fn add_param(&mut self, param_type: CoreValType) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.params.push(param_type);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.params.push(param_type).map_err(|_| {
                wrt_foundation::WrtError::ResourceExhausted("Too many parameters".into())
            })
        }
    }

    /// Add a result type
    pub fn add_result(&mut self, result_type: CoreValType) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            self.results.push(result_type);
            Ok(())
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.results
                .push(result_type)
                .map_err(|_| wrt_foundation::WrtError::ResourceExhausted("Too many results".into()))
        }
    }
}

impl Default for CoreFunctionSignature {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryAdapter {
    /// Create a new memory adapter
    pub fn new(core_index: u32, min: u32, max: Option<u32>, shared: bool) -> Self {
        Self { core_index, limits: MemoryLimits { min, max }, shared }
    }
}

impl TableAdapter {
    /// Create a new table adapter
    pub fn new(core_index: u32, element_type: CoreValType, min: u32, max: Option<u32>) -> Self {
        Self { core_index, element_type, limits: TableLimits { min, max } }
    }
}

impl GlobalAdapter {
    /// Create a new global adapter
    pub fn new(core_index: u32, global_type: CoreValType, mutable: bool) -> Self {
        Self { core_index, global_type, mutable }
    }
}

impl fmt::Display for CoreValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreValType::I32 => write!(f, "i32"),
            CoreValType::I64 => write!(f, "i64"),
            CoreValType::F32 => write!(f, "f32"),
            CoreValType::F64 => write!(f, "f64"),
            CoreValType::V128 => write!(f, "v128"),
            CoreValType::FuncRef => write!(f, "funcref"),
            CoreValType::ExternRef => write!(f, "externref"),
        }
    }
}

impl fmt::Display for AdaptationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdaptationMode::Direct => write!(f, "direct"),
            AdaptationMode::Lift => write!(f, "lift"),
            AdaptationMode::Lower => write!(f, "lower"),
            AdaptationMode::Bidirectional => write!(f, "bidirectional"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_module_adapter_creation() {
        #[cfg(feature = "std")]
        {
            let adapter = CoreModuleAdapter::new("test_module".to_string());
            assert_eq!(adapter.name, "test_module");
            assert_eq!(adapter.functions.len(), 0);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let name = BoundedString::from_str("test_module").unwrap();
            let adapter = CoreModuleAdapter::new(name);
            assert_eq!(adapter.name.as_str(), "test_module");
            assert_eq!(adapter.functions.len(), 0);
        }
    }

    #[test]
    fn test_function_adapter() {
        let mut core_sig = CoreFunctionSignature::new();
        core_sig.add_param(CoreValType::I32).unwrap();
        core_sig.add_result(CoreValType::I32).unwrap();

        let adapter =
            FunctionAdapter::new(0, ComponentType::Unit, core_sig, AdaptationMode::Direct);

        assert_eq!(adapter.core_index, 0);
        assert_eq!(adapter.mode, AdaptationMode::Direct);
        assert!(!adapter.needs_canonical_abi());
    }

    #[test]
    fn test_core_val_type_display() {
        assert_eq!(CoreValType::I32.to_string(), "i32");
        assert_eq!(CoreValType::F64.to_string(), "f64");
        assert_eq!(CoreValType::FuncRef.to_string(), "funcref");
    }

    #[test]
    fn test_adaptation_mode_display() {
        assert_eq!(AdaptationMode::Direct.to_string(), "direct");
        assert_eq!(AdaptationMode::Lift.to_string(), "lift");
        assert_eq!(AdaptationMode::Bidirectional.to_string(), "bidirectional");
    }

    #[test]
    fn test_memory_adapter() {
        let adapter = MemoryAdapter::new(0, 1, Some(10), false);
        assert_eq!(adapter.core_index, 0);
        assert_eq!(adapter.limits.min, 1);
        assert_eq!(adapter.limits.max, Some(10));
        assert!(!adapter.shared);
    }

    #[test]
    fn test_table_adapter() {
        let adapter = TableAdapter::new(0, CoreValType::FuncRef, 0, None);
        assert_eq!(adapter.core_index, 0);
        assert_eq!(adapter.element_type, CoreValType::FuncRef);
        assert_eq!(adapter.limits.min, 0);
        assert_eq!(adapter.limits.max, None);
    }

    #[test]
    fn test_global_adapter() {
        let adapter = GlobalAdapter::new(0, CoreValType::I32, true);
        assert_eq!(adapter.core_index, 0);
        assert_eq!(adapter.global_type, CoreValType::I32);
        assert!(adapter.mutable);
    }
}
