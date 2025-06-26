//! WIT Debugger Integration for WRT Runtime
//!
//! This module provides integration between the WRT runtime and the WIT-aware
//! debugger from wrt-debug, enabling source-level debugging of WIT components.

extern crate alloc;

#[cfg(feature = "std")]
use std::{collections::BTreeMap, vec::Vec, boxed::Box};
#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, vec::Vec, boxed::Box};

use wrt_foundation::{
    BoundedString, BoundedVec, NoStdProvider,
    prelude::*,
    budget_aware_provider::CrateId,
    capabilities::CapabilityAwareProvider,
    capability_context, safe_capability_alloc
};
use wrt_error::{Error, Result};

// MemoryCapabilityContext and CapabilityGuardedProvider are imported above

// Type alias for the provider type that works with BoundedVec
// In std/alloc environments, use CapabilityAwareProvider wrapper
#[cfg(any(feature = "std", feature = "alloc"))]
type RuntimeProvider<const N: usize> = CapabilityAwareProvider<NoStdProvider<N>>;

// In no_std environments, use NoStdProvider directly
#[cfg(not(any(feature = "std", feature = "alloc")))]
type RuntimeProvider<const N: usize> = CapabilityAwareProvider<NoStdProvider<N>>;

/// Helper function to create a capability-aware provider
fn create_provider<const N: usize>() -> Result<RuntimeProvider<N>> {
    let context = capability_context!(dynamic(CrateId::Runtime, N))?;
    safe_capability_alloc!(context, CrateId::Runtime, N)
}

// Import debug types for this module
#[cfg(feature = "wit-debug-integration")]
use wrt_debug::{
    RuntimeDebugger, RuntimeState, DebugAction, BreakpointId,
    DebugError, DebugMemory, DebuggableRuntime, SourceSpan,
};

#[cfg(feature = "wit-debug-integration")]
use wrt_debug::{
    WitAwareDebugger, WitDebugger, ComponentId, FunctionId, TypeId,
};

// Re-export for convenience
#[cfg(feature = "wit-debug-integration")]
pub use wrt_debug::{
    WitDebugger, ComponentId, FunctionId, TypeId, SourceSpan,
};

/// Metadata about a component for debugging
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug, Clone)]
pub struct ComponentMetadata {
    /// Component name
    pub name: BoundedString<64, RuntimeProvider<1024>>,
    
    /// Source span in WIT
    pub source_span: SourceSpan,
    
    /// Binary start offset
    pub binary_start: u32,
    
    /// Binary end offset
    pub binary_end: u32,
    
    /// Exported functions
    pub exports: Vec<FunctionId>,
    
    /// Imported functions
    pub imports: Vec<FunctionId>,
}

/// Metadata about a function for debugging
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Function name
    pub name: BoundedString<64, RuntimeProvider<1024>>,
    
    /// Source span in WIT
    pub source_span: SourceSpan,
    
    /// Binary offset
    pub binary_offset: u32,
    
    /// Parameter types
    pub param_types: Vec<TypeId>,
    
    /// Return types
    pub return_types: Vec<TypeId>,
    
    /// Whether function is async
    pub is_async: bool,
}

/// Metadata about a type for debugging
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug, Clone)]
pub struct TypeMetadata {
    /// Type name
    pub name: BoundedString<64, RuntimeProvider<1024>>,
    
    /// Source span in WIT
    pub source_span: SourceSpan,
    
    /// Type kind (record, variant, etc.)
    pub kind: WitTypeKind,
    
    /// Size in bytes (if known)
    pub size: Option<u32>,
}

/// WIT type kind for debugging
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug, Clone, PartialEq)]
pub enum WitTypeKind {
    /// Primitive type
    Primitive,
    /// Record type
    Record,
    /// Variant type
    Variant,
    /// Enum type
    Enum,
    /// Flags type
    Flags,
    /// Resource type
    Resource,
    /// Function type
    Function,
    /// Interface type
    Interface,
    /// World type
    World,
}

/// Breakpoint information for WRT runtime
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique ID
    pub id: BreakpointId,
    /// Address to break at
    pub address: u32,
    /// Source file
    pub file_index: Option<u16>,
    /// Source line
    pub line: Option<u32>,
    /// Condition (simplified - would need expression evaluator)
    pub condition: Option<BreakpointCondition>,
    /// Hit count
    pub hit_count: u32,
    /// Enabled state
    pub enabled: bool,
}

/// Simple breakpoint conditions
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug, Clone)]
pub enum BreakpointCondition {
    /// Break when hit count reaches value
    HitCount(u32),
    /// Break when local variable equals value
    LocalEquals { index: u32, value: u64 },
    /// Always break
    Always,
}

/// WRT Runtime state that can be debugged
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug)]
pub struct WrtRuntimeState {
    /// Current program counter
    pc: u32,
    
    /// Stack pointer
    sp: u32,
    
    /// Current function index
    current_function: Option<u32>,
    
    /// Local variables
    locals: BoundedVec<u64, 256, RuntimeProvider<8192>>,
    
    /// Operand stack
    stack: BoundedVec<u64, 1024, RuntimeProvider<8192>>,
    
    /// Memory reference
    memory_base: Option<u32>,
    
    /// Memory size
    memory_size: u32,
}

#[cfg(feature = "wit-debug-integration")]
impl WrtRuntimeState {
    /// Create a new runtime state
    pub fn new() -> Result<Self> {
        let provider1 = create_provider::<8192>()?;
        let provider2 = create_provider::<8192>()?;
        
        Ok(Self {
            pc: 0,
            sp: 0,
            current_function: None,
            locals: BoundedVec::new(provider1).map_err(|_| Error::memory("Failed to create locals vector"))?,
            stack: BoundedVec::new(provider2).map_err(|_| Error::memory("Failed to create stack vector"))?,
            memory_base: None,
            memory_size: 0,
        })
    }
    
    /// Update program counter
    pub fn set_pc(&mut self, pc: u32) {
        self.pc = pc;
    }
    
    /// Update stack pointer
    pub fn set_sp(&mut self, sp: u32) {
        self.sp = sp;
    }
    
    /// Set current function
    pub fn set_current_function(&mut self, func_idx: u32) {
        self.current_function = Some(func_idx);
    }
    
    /// Add local variable
    pub fn add_local(&mut self, value: u64) -> Result<()> {
        self.locals.push(value)
            .map_err(|_| Error::runtime_error("Local variables overflow"))
    }
    
    /// Update local variable
    pub fn set_local(&mut self, index: u32, value: u64) -> Result<()> {
        if let Some(local) = self.locals.get_mut(index as usize) {
            *local = value;
            Ok(())
        } else {
            Err(Error::runtime_error("Invalid local variable index"))
        }
    }
    
    /// Push to operand stack
    pub fn push_stack(&mut self, value: u64) -> Result<()> {
        self.stack.push(value)
            .map_err(|_| Error::runtime_error("Operand stack overflow"))
    }
    
    /// Pop from operand stack
    pub fn pop_stack(&mut self) -> Option<u64> {
        self.stack.pop()
    }
    
    /// Set memory information
    pub fn set_memory(&mut self, base: u32, size: u32) {
        self.memory_base = Some(base);
        self.memory_size = size;
    }
}

#[cfg(feature = "wit-debug-integration")]
impl Default for WrtRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "wit-debug-integration")]
impl RuntimeState for WrtRuntimeState {
    fn pc(&self) -> u32 {
        self.pc
    }
    
    fn sp(&self) -> u32 {
        self.sp
    }
    
    fn fp(&self) -> Option<u32> {
        // WebAssembly doesn't have a traditional frame pointer
        None
    }
    
    fn read_local(&self, index: u32) -> Option<u64> {
        self.locals.get(index as usize).copied()
    }
    
    fn read_stack(&self, offset: u32) -> Option<u64> {
        if let Some(stack_len) = self.stack.len().checked_sub(offset as usize + 1) {
            self.stack.get(stack_len).copied()
        } else {
            None
        }
    }
    
    fn current_function(&self) -> Option<u32> {
        self.current_function
    }
}

/// WRT Memory accessor for debugging
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug)]
pub struct WrtDebugMemory {
    /// Memory data reference
    memory_data: BoundedVec<u8, 65536, RuntimeProvider<65536>>, // 64KB max for no_std
    
    /// Memory base address
    base_address: u32,
}

#[cfg(feature = "wit-debug-integration")]
impl WrtDebugMemory {
    /// Create a new debug memory accessor
    pub fn new(base_address: u32) -> Result<Self> {
        let provider = create_provider::<65536>()?;
        
        Ok(Self {
            memory_data: BoundedVec::new(provider).map_err(|_| Error::memory("Failed to create memory data vector"))?,
            base_address,
        })
    }
    
    /// Set memory data (for testing/simulation)
    pub fn set_memory_data(&mut self, data: &[u8]) -> Result<()> {
        self.memory_data.clear();
        for &byte in data {
            self.memory_data.push(byte)
                .map_err(|_| Error::runtime_error("Memory data overflow"))?;
        }
        Ok(())
    }
    
    /// Get memory size
    pub fn memory_size(&self) -> usize {
        self.memory_data.len()
    }
}

#[cfg(feature = "wit-debug-integration")]
impl Default for WrtDebugMemory {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(feature = "wit-debug-integration")]
impl DebugMemory for WrtDebugMemory {
    fn read_exact(&self, addr: u32, len: usize) -> Option<&[u8]> {
        let offset = addr.saturating_sub(self.base_address) as usize;
        if offset + len <= self.memory_data.len() {
            Some(&self.memory_data.as_slice()[offset..offset + len])
        } else {
            None
        }
    }
    
    fn is_valid_address(&self, addr: u32) -> bool {
        let offset = addr.saturating_sub(self.base_address) as usize;
        offset < self.memory_data.len()
    }
}

/// WRT Runtime with WIT debugging support
#[cfg(feature = "wit-debug-integration")]
#[derive(Debug)]
pub struct DebuggableWrtRuntime {
    /// Runtime state
    state: WrtRuntimeState,
    
    /// Debug memory accessor
    memory: WrtDebugMemory,
    
    /// Attached debugger
    debugger: Option<Box<dyn RuntimeDebugger>>,
    
    /// Debug mode enabled
    debug_mode: bool,
    
    /// Breakpoints
    breakpoints: BTreeMap<BreakpointId, Breakpoint>,
    
    /// Next breakpoint ID
    next_breakpoint_id: u32,
    
    /// Execution statistics
    instruction_count: u64,
    
    /// Function call depth
    call_depth: u32,
}

#[cfg(feature = "wit-debug-integration")]
impl DebuggableWrtRuntime {
    /// Create a new debuggable runtime
    pub fn new() -> Self {
        Self {
            state: WrtRuntimeState::new(),
            memory: WrtDebugMemory::new(0),
            debugger: None,
            debug_mode: false,
            breakpoints: BTreeMap::new(),
            next_breakpoint_id: 1,
            instruction_count: 0,
            call_depth: 0,
        }
    }
    
    /// Execute an instruction with debugging support
    pub fn execute_instruction(&mut self, instruction_addr: u32) -> Result<DebugAction> {
        self.state.set_pc(instruction_addr);
        self.instruction_count += 1;
        
        // Check for breakpoints
        for (_, breakpoint) in &mut self.breakpoints {
            if breakpoint.enabled && breakpoint.address == instruction_addr {
                breakpoint.hit_count += 1;
                
                // Check condition
                let should_break = match &breakpoint.condition {
                    Some(BreakpointCondition::Always) => true,
                    Some(BreakpointCondition::HitCount(count)) => {
                        breakpoint.hit_count >= *count
                    },
                    Some(BreakpointCondition::LocalEquals { index, value }) => {
                        self.state.read_local(*index) == Some(*value)
                    },
                    None => true,
                };
                
                if should_break {
                    if let Some(ref mut debugger) = self.debugger {
                        return Ok(debugger.on_breakpoint(breakpoint, &self.state));
                    }
                }
            }
        }
        
        // Call debugger for instruction stepping
        if self.debug_mode {
            if let Some(ref mut debugger) = self.debugger {
                return Ok(debugger.on_instruction(instruction_addr, &self.state));
            }
        }
        
        Ok(DebugAction::Continue)
    }
    
    /// Enter a function
    pub fn enter_function(&mut self, func_idx: u32) {
        self.state.set_current_function(func_idx);
        self.call_depth += 1;
        
        if let Some(ref mut debugger) = self.debugger {
            debugger.on_function_entry(func_idx, &self.state);
        }
    }
    
    /// Exit a function
    pub fn exit_function(&mut self, func_idx: u32) {
        if self.call_depth > 0 {
            self.call_depth -= 1;
        }
        
        if let Some(ref mut debugger) = self.debugger {
            debugger.on_function_exit(func_idx, &self.state);
        }
    }
    
    /// Handle a trap/error
    pub fn handle_trap(&mut self, trap_code: u32) {
        if let Some(ref mut debugger) = self.debugger {
            debugger.on_trap(trap_code, &self.state);
        }
    }
    
    /// Get mutable access to runtime state (for runtime updates)
    pub fn state_mut(&mut self) -> &mut WrtRuntimeState {
        &mut self.state
    }
    
    /// Get mutable access to debug memory (for runtime updates)
    pub fn memory_mut(&mut self) -> &mut WrtDebugMemory {
        &mut self.memory
    }
    
    /// Get execution statistics
    pub fn instruction_count(&self) -> u64 {
        self.instruction_count
    }
    
    /// Get call depth
    pub fn call_depth(&self) -> u32 {
        self.call_depth
    }
    
    /// Create a WIT debugger with component integration
    pub fn create_wit_debugger() -> WitDebugger {
        WitDebugger::new()
    }
    
    /// Attach a WIT debugger with component metadata
    pub fn attach_wit_debugger_with_components(
        &mut self, 
        mut wit_debugger: WitDebugger,
        components: Vec<(ComponentId, ComponentMetadata)>,
        functions: Vec<(FunctionId, FunctionMetadata)>,
        types: Vec<(TypeId, TypeMetadata)>,
    ) {
        // For now, we'll need to adapt the metadata to what WitDebugger expects
        // This would need to be implemented when we have access to WitDebugger's add methods
        
        // Attach the debugger
        self.attach_debugger(Box::new(wit_debugger));
    }
}

#[cfg(feature = "wit-debug-integration")]
impl Default for DebuggableWrtRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "wit-debug-integration")]
impl DebuggableRuntime for DebuggableWrtRuntime {
    fn attach_debugger(&mut self, debugger: Box<dyn RuntimeDebugger>) {
        self.debugger = Some(debugger);
    }
    
    fn detach_debugger(&mut self) {
        self.debugger = None;
    }
    
    fn has_debugger(&self) -> bool {
        self.debugger.is_some()
    }
    
    fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }
    
    fn add_breakpoint(&mut self, mut bp: Breakpoint) -> Result<(), DebugError> {
        // Check for duplicate address
        // Note: BoundedMap doesn't have values() method, so we iterate through entries manually
        for i in 0..self.breakpoints.entries.len() {
            if let Ok(entry) = self.breakpoints.entries.get(i) {
                if entry.1.address == bp.address {
                    return Err(DebugError::DuplicateBreakpoint);
                }
            }
        }
        
        // Assign ID if not set
        if bp.id == BreakpointId(0) {
            bp.id = BreakpointId(self.next_breakpoint_id);
            self.next_breakpoint_id += 1;
        }
        
        self.breakpoints.insert(bp.id, bp);
        Ok(())
    }
    
    fn remove_breakpoint(&mut self, id: BreakpointId) -> Result<(), DebugError> {
        self.breakpoints.remove(&id)
            .map(|_| ())
            .ok_or(DebugError::BreakpointNotFound)
    }
    
    fn get_state(&self) -> Box<dyn RuntimeState> {
        Box::new(self.state.clone())
    }
    
    fn get_memory(&self) -> Box<dyn DebugMemory> {
        Box::new(self.memory.clone())
    }
}

/// Helper function to create a debuggable runtime with WIT support
#[cfg(feature = "wit-debug-integration")]
pub fn create_wit_enabled_runtime() -> DebuggableWrtRuntime {
    DebuggableWrtRuntime::new()
}

/// Helper function to create component metadata for debugging
#[cfg(feature = "wit-debug-integration")]
pub fn create_component_metadata(
    name: &str,
    source_span: SourceSpan,
    binary_start: u32,
    binary_end: u32,
) -> Result<ComponentMetadata> {
    let provider = create_provider::<8192>()?;
    
    Ok(ComponentMetadata {
        name: BoundedString::from_str(name, provider)
            .map_err(|_| Error::runtime_error("Component name too long"))?,
        source_span,
        binary_start,
        binary_end,
        exports: Vec::new(create_provider::<1024>()?)?,
        imports: Vec::new(create_provider::<1024>()?)?,
    })
}

/// Helper function to create function metadata for debugging
#[cfg(feature = "wit-debug-integration")]
pub fn create_function_metadata(
    name: &str,
    source_span: SourceSpan,
    binary_offset: u32,
    is_async: bool,
) -> Result<FunctionMetadata> {
    let provider = create_provider::<8192>()?;
    
    Ok(FunctionMetadata {
        name: BoundedString::from_str(name, provider)
            .map_err(|_| Error::runtime_error("Function name too long"))?,
        source_span,
        binary_offset,
        param_types: Vec::new(create_provider::<1024>()?)?,
        return_types: Vec::new(create_provider::<1024>()?)?,
        is_async,
    })
}

/// Helper function to create type metadata for debugging
#[cfg(feature = "wit-debug-integration")]
pub fn create_type_metadata(
    name: &str,
    source_span: SourceSpan,
    kind: WitTypeKind,
    size: Option<u32>,
) -> Result<TypeMetadata> {
    // Use 8KB allocation for type metadata - sufficient for typical type names and metadata
    let provider = create_provider::<8192>()?;
    
    Ok(TypeMetadata {
        name: BoundedString::from_str(name, provider)
            .map_err(|_| Error::runtime_error("Type name too long"))?,
        source_span,
        kind,
        size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "wit-debug-integration")]
    #[test]
    fn test_debuggable_runtime_creation() {
        let runtime = DebuggableWrtRuntime::new();
        assert!(!runtime.has_debugger());
        assert!(!runtime.debug_mode);
        assert_eq!(runtime.instruction_count(), 0);
        assert_eq!(runtime.call_depth(), 0);
    }
    
    #[cfg(feature = "wit-debug-integration")]
    #[test]
    fn test_runtime_state() {
        let mut state = WrtRuntimeState::new();
        
        state.set_pc(100);
        assert_eq!(state.pc(), 100);
        
        state.set_sp(200);
        assert_eq!(state.sp(), 200);
        
        state.set_current_function(42);
        assert_eq!(state.current_function(), Some(42));
        
        assert!(state.add_local(123).is_ok());
        assert_eq!(state.read_local(0), Some(123));
        
        assert!(state.push_stack(456).is_ok());
        assert_eq!(state.read_stack(0), Some(456));
    }
    
    #[cfg(feature = "wit-debug-integration")]
    #[test]
    fn test_debug_memory() {
        let mut memory = WrtDebugMemory::new(1000);
        let test_data = &[1, 2, 3, 4, 5, 6, 7, 8];
        
        assert!(memory.set_memory_data(test_data).is_ok());
        assert_eq!(memory.memory_size(), 8);
        
        assert!(memory.is_valid_address(1000));
        assert!(memory.is_valid_address(1007));
        assert!(!memory.is_valid_address(1008));
        
        let bytes = memory.read_exact(1002, 4);
        assert_eq!(bytes, Some(&[3, 4, 5, 6][..]));
        
        assert_eq!(memory.read_u32(1000), Some(0x04030201));
    }
    
    #[cfg(feature = "wit-debug-integration")]
    #[test]
    fn test_breakpoint_management() {
        let mut runtime = DebuggableWrtRuntime::new();
        
        let bp = Breakpoint {
            id: BreakpointId(0), // Will be assigned
            address: 100,
            file_index: None,
            line: Some(10),
            condition: None,
            hit_count: 0,
            enabled: true,
        };
        
        assert!(runtime.add_breakpoint(bp).is_ok());
        
        // Try to add duplicate
        let bp2 = Breakpoint {
            id: BreakpointId(0),
            address: 100, // Same address
            file_index: None,
            line: Some(11),
            condition: None,
            hit_count: 0,
            enabled: true,
        };
        
        assert_eq!(runtime.add_breakpoint(bp2), Err(DebugError::DuplicateBreakpoint));
        
        // Remove breakpoint
        assert!(runtime.remove_breakpoint(BreakpointId(1)).is_ok());
        assert_eq!(runtime.remove_breakpoint(BreakpointId(1)), Err(DebugError::BreakpointNotFound));
    }
    
    #[cfg(feature = "wit-debug-integration")]
    #[test]
    fn test_wit_debugger_integration() {
        let mut runtime = DebuggableWrtRuntime::new();
        let wit_debugger = DebuggableWrtRuntime::create_wit_debugger();
        
        runtime.attach_debugger(Box::new(wit_debugger));
        assert!(runtime.has_debugger());
        
        runtime.set_debug_mode(true);
        
        // Simulate function execution
        runtime.enter_function(42);
        assert_eq!(runtime.call_depth(), 1);
        
        let action = runtime.execute_instruction(1000).unwrap();
        assert_eq!(action, DebugAction::Continue);
        
        runtime.exit_function(42);
        assert_eq!(runtime.call_depth(), 0);
    }
    
    #[cfg(feature = "wit-debug-integration")]
    #[test]
    fn test_metadata_helpers() {
        use wrt_debug::SourceSpan;
        
        let span = SourceSpan::new(0, 100, 0);
        
        let comp_meta = create_component_metadata("test-component", span, 1000, 2000);
        assert!(comp_meta.is_ok());
        
        let func_meta = create_function_metadata("test-function", span, 1500, false);
        assert!(func_meta.is_ok());
        
        let type_meta = create_type_metadata("test-type", span, WitTypeKind::Record, Some(16));
        assert!(type_meta.is_ok());
    }
}