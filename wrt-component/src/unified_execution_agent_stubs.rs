//! Minimal stubs for unified execution engine compilation
//! These stubs allow the unified engine to compile without all dependencies

use wrt_foundation::{
    bounded::{BoundedVec, BoundedString},
    prelude::*,
    safe_memory::NoStdProvider,
    WrtResult,
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

use crate::types::Value;

/// Canonical ABI processor stub
#[derive(Debug, Default)]
pub struct CanonicalAbi;

impl CanonicalAbi {
    pub fn new() -> Self {
        Self
    }
}

/// Canonical options stub
#[derive(Debug, Default, Clone)]
pub struct CanonicalOptions;

/// Resource handle stub
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHandle(pub u32);

/// Resource lifecycle manager stub  
#[derive(Debug)]
pub struct ResourceLifecycleManager {
    next_handle: u32,
}

impl ResourceLifecycleManager {
    pub fn new() -> Self {
        Self { next_handle: 1 }
    }
    
    pub fn create_resource(&mut self, _type_id: u32, _data: ComponentValue) -> WrtResult<ResourceHandle> {
        let handle = ResourceHandle(self.next_handle);
        self.next_handle += 1;
        Ok(handle)
    }
    
    pub fn drop_resource(&mut self, _handle: ResourceHandle) -> WrtResult<()> {
        Ok(())
    }
    
    pub fn borrow_resource(&mut self, _handle: ResourceHandle) -> WrtResult<&ComponentValue> {
        // Return a dummy value - in real implementation this would be tracked
        static DUMMY: ComponentValue = ComponentValue::Bool(false);
        Ok(&DUMMY)
    }
    
    pub fn transfer_ownership(&mut self, _handle: ResourceHandle, _new_owner: u32) -> WrtResult<()> {
        Ok(())
    }
}

/// Runtime bridge configuration stub
#[derive(Debug, Default, Clone)]
pub struct RuntimeBridgeConfig;

/// Component runtime bridge stub
#[derive(Debug)]
pub struct ComponentRuntimeBridge;

impl ComponentRuntimeBridge {
    pub fn new() -> Self {
        Self
    }
    
    pub fn with_config(_config: RuntimeBridgeConfig) -> Self {
        Self
    }
    
    pub fn execute_component_function(
        &mut self,
        _instance_id: u32,
        _function_name: &str,
        _args: &[wrt_foundation::component_value::ComponentValue],
    ) -> core::result::Result<wrt_foundation::component_value::ComponentValue, wrt_error::Error> {
        // Return a dummy successful result
        Ok(wrt_foundation::component_value::ComponentValue::U32(42))
    }
    
    pub fn register_component_instance(
        &mut self,
        _component_id: u32,
        _module_name: alloc::string::String,
        _function_count: u32,
        _memory_size: u32,
    ) -> core::result::Result<u32, wrt_error::Error> {
        Ok(1)
    }
    
    #[cfg(feature = "std")]
    pub fn register_host_function<F>(
        &mut self,
        _name: alloc::string::String,
        _signature: crate::component_instantiation::FunctionSignature,
        _func: F,
    ) -> core::result::Result<usize, wrt_error::Error>
    where
        F: Fn(&[ComponentValue]) -> core::result::Result<ComponentValue, wrt_error::Error> + Send + Sync + 'static,
    {
        Ok(0)
    }
    
    #[cfg(not(feature = "std"))]
    pub fn register_host_function(
        &mut self,
        _name: BoundedString<64, NoStdProvider::<65536>>,
        _signature: crate::component_instantiation::FunctionSignature,
        _func: fn(&[ComponentValue]) -> core::result::Result<ComponentValue, wrt_error::Error>,
    ) -> core::result::Result<usize, wrt_error::Error> {
        Ok(0)
    }
}

/// Component value conversion stubs
impl From<wrt_foundation::component_value::ComponentValue> for Value {
    fn from(cv: wrt_foundation::component_value::ComponentValue) -> Self {
        match cv {
            wrt_foundation::component_value::ComponentValue::Bool(b) => Value::Bool(b),
            wrt_foundation::component_value::ComponentValue::U8(v) => Value::U8(v),
            wrt_foundation::component_value::ComponentValue::U16(v) => Value::U16(v),
            wrt_foundation::component_value::ComponentValue::U32(v) => Value::U32(v),
            wrt_foundation::component_value::ComponentValue::U64(v) => Value::U64(v),
            wrt_foundation::component_value::ComponentValue::S8(v) => Value::S8(v),
            wrt_foundation::component_value::ComponentValue::S16(v) => Value::S16(v),
            wrt_foundation::component_value::ComponentValue::S32(v) => Value::S32(v),
            wrt_foundation::component_value::ComponentValue::S64(v) => Value::S64(v),
            wrt_foundation::component_value::ComponentValue::F32(v) => Value::F32(v),
            wrt_foundation::component_value::ComponentValue::F64(v) => Value::F64(v),
            wrt_foundation::component_value::ComponentValue::Char(c) => Value::Char(c),
            wrt_foundation::component_value::ComponentValue::String(s) => Value::String(s),
            _ => Value::Bool(false), // Fallback
        }
    }
}

impl From<Value> for wrt_foundation::component_value::ComponentValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Bool(b) => wrt_foundation::component_value::ComponentValue::Bool(b),
            Value::U8(v) => wrt_foundation::component_value::ComponentValue::U8(v),
            Value::U16(v) => wrt_foundation::component_value::ComponentValue::U16(v),
            Value::U32(v) => wrt_foundation::component_value::ComponentValue::U32(v),
            Value::U64(v) => wrt_foundation::component_value::ComponentValue::U64(v),
            Value::S8(v) => wrt_foundation::component_value::ComponentValue::S8(v),
            Value::S16(v) => wrt_foundation::component_value::ComponentValue::S16(v),
            Value::S32(v) => wrt_foundation::component_value::ComponentValue::S32(v),
            Value::S64(v) => wrt_foundation::component_value::ComponentValue::S64(v),
            Value::F32(v) => wrt_foundation::component_value::ComponentValue::F32(v),
            Value::F64(v) => wrt_foundation::component_value::ComponentValue::F64(v),
            Value::Char(c) => wrt_foundation::component_value::ComponentValue::Char(c),
            Value::String(s) => wrt_foundation::component_value::ComponentValue::String(s),
            _ => wrt_foundation::component_value::ComponentValue::Bool(false), // Fallback
        }
    }
}

#[cfg(feature = "async")]
pub mod async_stubs {
    use super::*;
    
    /// Async read result stub
    #[derive(Debug, Clone)]
    pub enum AsyncReadResult {
        Ready(Vec<u8>),
        Pending,
        Error(alloc::string::String),
    }
    
    /// Future handle stub
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FutureHandle(pub u32);
    
    /// Future state stub
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FutureState {
        Pending,
        Ready,
        Error,
    }
    
    /// Stream handle stub
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StreamHandle(pub u32);
    
    /// Stream state stub
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum StreamState {
        Open,
        Closed,
        Error,
    }
    
    /// Component future stub
    #[derive(Debug)]
    pub struct Future;
    
    /// Stream stub
    #[derive(Debug)]
    pub struct Stream;
}

#[cfg(feature = "async")]
pub use async_stubs::*;

#[cfg(feature = "cfi")]
pub mod cfi_stubs {
    use super::*;
    
    /// CFI control flow operations stub
    #[derive(Debug, Default)]
    pub struct DefaultCfiControlFlowOps;
    
    /// CFI control flow protection stub
    #[derive(Debug, Default, Clone)]
    pub struct CfiControlFlowProtection {
        pub software_config: CfiSoftwareConfig,
    }
    
    /// CFI software config stub
    #[derive(Debug, Default, Clone)]
    pub struct CfiSoftwareConfig {
        pub max_shadow_stack_depth: usize,
    }
    
    impl Default for CfiSoftwareConfig {
        fn default() -> Self {
            Self {
                max_shadow_stack_depth: 1024,
            }
        }
    }
    
    /// CFI execution context stub
    #[derive(Debug, Default, Clone)]
    pub struct CfiExecutionContext {
        pub current_function: u32,
        pub current_instruction: u32,
        pub shadow_stack: BoundedVec<u32, 1024, NoStdProvider::<65536, NoStdProvider<65536>>>,
        pub violation_count: u32,
        pub landing_pad_expectations: BoundedVec<LandingPadExpectation, 16, NoStdProvider::<65536, NoStdProvider<65536>>>,
        pub metrics: CfiMetrics,
    }
    
    /// Landing pad expectation stub
    #[derive(Debug, Clone)]
    pub struct LandingPadExpectation {
        pub function_index: u32,
        pub instruction_offset: u32,
        pub deadline: Option<u64>,
    }
    
    /// CFI metrics stub
    #[derive(Debug, Default, Clone)]
    pub struct CfiMetrics {
        pub landing_pads_validated: u64,
        pub shadow_stack_operations: u64,
    }
    
    /// CFI protected branch target stub
    #[derive(Debug, Clone)]
    pub struct CfiProtectedBranchTarget {
        pub target: u32,
        pub protection: CfiProtection,
    }
    
    /// CFI protection stub
    #[derive(Debug, Default, Clone)]
    pub struct CfiProtection {
        pub landing_pad: Option<CfiLandingPad>,
    }
    
    /// CFI landing pad stub
    #[derive(Debug, Clone)]
    pub struct CfiLandingPad {
        pub label: u32,
    }
    
    impl DefaultCfiControlFlowOps {
        pub fn call_indirect_with_cfi(
            &mut self,
            _type_idx: u32,
            _table_idx: u32,
            _protection: &CfiControlFlowProtection,
            _context: &mut CfiExecutionContext,
        ) -> core::result::Result<CfiProtectedBranchTarget, wrt_error::Error> {
            Ok(CfiProtectedBranchTarget {
                target: 0,
                protection: CfiProtection::default(),
            })
        }
        
        pub fn return_with_cfi(
            &mut self,
            _protection: &CfiControlFlowProtection,
            _context: &mut CfiExecutionContext,
        ) -> core::result::Result<(), wrt_error::Error> {
            Ok(())
        }
        
        pub fn branch_with_cfi(
            &mut self,
            _label_idx: u32,
            _conditional: bool,
            _protection: &CfiControlFlowProtection,
            _context: &mut CfiExecutionContext,
        ) -> core::result::Result<CfiProtectedBranchTarget, wrt_error::Error> {
            Ok(CfiProtectedBranchTarget {
                target: _label_idx,
                protection: CfiProtection::default(),
            })
        }
    }
    
    impl Default for CfiExecutionContext {
        fn default() -> Self {
            Self {
                current_function: 0,
                current_instruction: 0,
                shadow_stack: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
                violation_count: 0,
                landing_pad_expectations: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
                metrics: CfiMetrics::default(),
            }
        }
    }
}

#[cfg(feature = "cfi")]
pub use cfi_stubs::*;