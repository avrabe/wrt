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
    bounded::BoundedVec, prelude::*,
};

use crate::execution_engine::ComponentExecutionEngine;

#[cfg(not(feature = "std"))]
use wrt_foundation::{
    BoundedString, 
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

#[cfg(not(feature = "std"))]
// For no_std, use a simpler ComponentValue representation
use crate::types::Value as ComponentValue;

use crate::{
    canonical_abi::CanonicalABI,
    components::Component,
    types::{ValType, Value},
};

/// Maximum number of adapted functions in no_std environments
const MAX_ADAPTED_FUNCTIONS: usize = 256;

/// Adapter that wraps a core WebAssembly module for use in components
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoreModuleAdapter {
    /// Module name/identifier
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, NoStdProvider<65536>>,

    /// Function adapters
    #[cfg(feature = "std")]
    pub functions: Vec<FunctionAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub functions: BoundedVec<FunctionAdapter, MAX_ADAPTED_FUNCTIONS, NoStdProvider<65536>>,

    /// Memory adapters
    #[cfg(feature = "std")]
    pub memories: Vec<MemoryAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub memories: BoundedVec<MemoryAdapter, 16, NoStdProvider<65536>>,

    /// Table adapters
    #[cfg(feature = "std")]
    pub tables: Vec<TableAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub tables: BoundedVec<TableAdapter, 16, NoStdProvider<65536>>,

    /// Global adapters
    #[cfg(feature = "std")]
    pub globals: Vec<GlobalAdapter>,
    #[cfg(not(any(feature = "std", )))]
    pub globals: BoundedVec<GlobalAdapter, 64, NoStdProvider<65536>>,
}

/// Adapter for core module functions
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionAdapter {
    /// Core function index
    pub core_index: u32,
    /// Component function signature
    pub component_signature: WrtComponentType,
    /// Core function signature (WebAssembly types)
    pub core_signature: CoreFunctionSignature,
    /// Adaptation mode
    pub mode: AdaptationMode,
}

/// Core WebAssembly function signature
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoreFunctionSignature {
    /// Parameter types (WebAssembly core types)
    #[cfg(feature = "std")]
    pub params: Vec<CoreValType>,
    #[cfg(not(any(feature = "std", )))]
    pub params: BoundedVec<CoreValType, 32, NoStdProvider<65536>>,
    /// Result types (WebAssembly core types)
    #[cfg(feature = "std")]
    pub results: Vec<CoreValType>,
    #[cfg(not(any(feature = "std", )))]
    pub results: BoundedVec<CoreValType, 8, NoStdProvider<65536>>,
}

/// WebAssembly core value types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoreValType {
    #[default]
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
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AdaptationMode {
    #[default]
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryAdapter {
    /// Core memory index
    pub core_index: u32,
    /// Memory limits
    pub limits: MemoryLimits,
    /// Shared flag
    pub shared: bool,
}

/// Memory limits
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryLimits {
    /// Minimum size in pages
    pub min: u32,
    /// Maximum size in pages (if any)
    pub max: Option<u32>,
}

/// Table adapter
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableAdapter {
    /// Core table index
    pub core_index: u32,
    /// Element type
    pub element_type: CoreValType,
    /// Table limits
    pub limits: TableLimits,
}

/// Table limits
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableLimits {
    /// Minimum size
    pub min: u32,
    /// Maximum size (if any)
    pub max: Option<u32>,
}

/// Global adapter
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    pub fn new(name: BoundedString<64, NoStdProvider<65536>>) -> core::result::Result<Self, Error> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        Ok(Self {
            name,
            functions: BoundedVec::new(provider.clone())?,
            memories: BoundedVec::new(provider.clone())?,
            tables: BoundedVec::new(provider.clone())?,
            globals: BoundedVec::new(provider)?,
        })
    }

    /// Add a function adapter
    pub fn add_function(&mut self, adapter: FunctionAdapter) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.functions.push(adapter);
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            self.functions.push(adapter).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many function adapters")
            })
        }
    }

    /// Add a memory adapter
    pub fn add_memory(&mut self, adapter: MemoryAdapter) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.memories.push(adapter);
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            self.memories.push(adapter).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many memory adapters")
            })
        }
    }

    /// Add a table adapter
    pub fn add_table(&mut self, adapter: TableAdapter) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.tables.push(adapter);
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            self.tables.push(adapter).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many table adapters")
            })
        }
    }

    /// Add a global adapter
    pub fn add_global(&mut self, adapter: GlobalAdapter) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.globals.push(adapter);
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            self.globals.push(adapter).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many global adapters")
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
    pub fn to_component(&self) -> Result<Component> {
        let mut component = Component::new(WrtComponentType::default);

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
    fn core_type_to_component_type(&self, core_type: CoreValType) -> WrtComponentType {
        match core_type {
            CoreValType::I32 => WrtComponentType::Unit, // Simplified
            CoreValType::I64 => WrtComponentType::Unit,
            CoreValType::F32 => WrtComponentType::Unit,
            CoreValType::F64 => WrtComponentType::Unit,
            CoreValType::V128 => WrtComponentType::Unit,
            CoreValType::FuncRef => WrtComponentType::Unit,
            CoreValType::ExternRef => WrtComponentType::Unit,
        }
    }

    /// Adapt a core function call to component model
    pub fn adapt_function_call(
        &self,
        func_index: u32,
        args: &[Value],
        engine: &mut ComponentExecutionEngine,
    ) -> Result<Value> {
        let adapter = self
            .get_function(func_index)
            .ok_or_else(|| wrt_error::Error::runtime_function_not_found("Function adapter not found"))?;

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
    ) -> Result<Value> {
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
    ) -> Result<Vec<Value>> {
        // Simplified lowering - in reality would use canonical ABI
        #[cfg(feature = "std")]
        {
            Ok(args.to_vec())
        }
        #[cfg(not(feature = "std"))]
        {
            let mut result = Vec::new);
            for arg in args {
                result.push(arg.clone();
            }
            Ok(result)
        }
    }

    /// Lift core result to component result
    fn lift_result_to_component(
        &self,
        result: Value,
        _component_signature: &WrtComponentType,
    ) -> Result<Value> {
        // Simplified lifting - in reality would use canonical ABI
        Ok(result)
    }
}

impl FunctionAdapter {
    /// Create a new function adapter
    pub fn new(
        core_index: u32,
        component_signature: WrtComponentType,
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
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            params: Vec::new(),
            #[cfg(not(feature = "std"))]
            params: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(feature = "std")]
            results: Vec::new(),
            #[cfg(not(feature = "std"))]
            results: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
        })
    }

    /// Add a parameter type
    pub fn add_param(&mut self, param_type: CoreValType) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.params.push(param_type);
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            self.params.push(param_type).map_err(|_| {
                wrt_error::Error::resource_exhausted("Too many parameters")
            })
        }
    }

    /// Add a result type
    pub fn add_result(&mut self, result_type: CoreValType) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.results.push(result_type);
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            self.results
                .push(result_type)
                .map_err(|_| wrt_error::Error::resource_exhausted("Too many results"))
        }
    }
}

impl Default for CoreFunctionSignature {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            Self {
                #[cfg(feature = "std")]
                params: Vec::new(),
                #[cfg(not(feature = "std"))]
                params: BoundedVec::new_with_default_provider().unwrap(),
                #[cfg(feature = "std")]
                results: Vec::new(),
                #[cfg(not(feature = "std"))]
                results: BoundedVec::new_with_default_provider().unwrap(),
            }
        })
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

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{Checksummable, ToBytes, FromBytes, WriteStream, ReadStream};

// Macro to implement basic traits for simple types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                // Simple checksum without unsafe code
                0u32.update_checksum(checksum;
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
                // Simple stub implementation
                Ok(())
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

// Apply macro to all adapter types
impl_basic_traits!(FunctionAdapter, FunctionAdapter::default);
impl_basic_traits!(MemoryAdapter, MemoryAdapter::default);
impl_basic_traits!(TableAdapter, TableAdapter::default);
impl_basic_traits!(GlobalAdapter, GlobalAdapter::default);
impl_basic_traits!(MemoryLimits, MemoryLimits::default);
impl_basic_traits!(TableLimits, TableLimits::default);
impl_basic_traits!(CoreValType, CoreValType::default);
impl_basic_traits!(AdaptationMode, AdaptationMode::default);
impl_basic_traits!(CoreFunctionSignature, CoreFunctionSignature::default);

mod tests {
    use super::*;

    #[test]
    fn test_core_module_adapter_creation() {
        #[cfg(feature = "std")]
        {
            let adapter = CoreModuleAdapter::new("test_module".to_string();
            assert_eq!(adapter.name, "test_module";
            assert_eq!(adapter.functions.len(), 0;
        }
        #[cfg(not(feature = "std"))]
        {
            let name = BoundedString::from_str("test_module").unwrap();
            let adapter = CoreModuleAdapter::new(name).unwrap();
            assert_eq!(adapter.name.as_str(), "test_module";
            assert_eq!(adapter.functions.len(), 0;
        }
    }

    #[test]
    fn test_function_adapter() {
        let mut core_sig = CoreFunctionSignature::new().unwrap();
        core_sig.add_param(CoreValType::I32).unwrap();
        core_sig.add_result(CoreValType::I32).unwrap();

        let adapter =
            FunctionAdapter::new(0, WrtComponentType::Unit, core_sig, AdaptationMode::Direct;

        assert_eq!(adapter.core_index, 0;
        assert_eq!(adapter.mode, AdaptationMode::Direct;
        assert!(!adapter.needs_canonical_abi();
    }

    #[test]
    fn test_core_val_type_display() {
        assert_eq!(CoreValType::I32.to_string(), "i32";
        assert_eq!(CoreValType::F64.to_string(), "f64";
        assert_eq!(CoreValType::FuncRef.to_string(), "funcref";
    }

    #[test]
    fn test_adaptation_mode_display() {
        assert_eq!(AdaptationMode::Direct.to_string(), "direct";
        assert_eq!(AdaptationMode::Lift.to_string(), "lift";
        assert_eq!(AdaptationMode::Bidirectional.to_string(), "bidirectional";
    }

    #[test]
    fn test_memory_adapter() {
        let adapter = MemoryAdapter::new(0, 1, Some(10), false;
        assert_eq!(adapter.core_index, 0;
        assert_eq!(adapter.limits.min, 1;
        assert_eq!(adapter.limits.max, Some(10;
        assert!(!adapter.shared);
    }

    #[test]
    fn test_table_adapter() {
        let adapter = TableAdapter::new(0, CoreValType::FuncRef, 0, None;
        assert_eq!(adapter.core_index, 0;
        assert_eq!(adapter.element_type, CoreValType::FuncRef;
        assert_eq!(adapter.limits.min, 0;
        assert_eq!(adapter.limits.max, None;
    }

    #[test]
    fn test_global_adapter() {
        let adapter = GlobalAdapter::new(0, CoreValType::I32, true;
        assert_eq!(adapter.core_index, 0;
        assert_eq!(adapter.global_type, CoreValType::I32;
        assert!(adapter.mutable);
    }
}
