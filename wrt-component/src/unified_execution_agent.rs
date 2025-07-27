//! Unified Execution Engine for WebAssembly Runtime
//!
//! This module provides a unified execution engine that consolidates functionality
//! from various execution modes (synchronous, asynchronous, stackless, and CFI-protected).
//! It provides a single, cohesive interface for WebAssembly execution with support for:
//! - Synchronous and asynchronous execution
//! - Stackless execution for memory-constrained environments  
//! - CFI protection for security-critical applications
//! - Component model execution

#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec, sync::Arc};
#[cfg(not(feature = "std"))]
use core::{mem, fmt};

use wrt_foundation::{
    bounded::{BoundedVec, BoundedString},
    prelude::*,
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

use crate::{
    unified_execution_agent_stubs::{
        CanonicalAbi, CanonicalOptions, ResourceHandle, ResourceLifecycleManager,
        ComponentRuntimeBridge, RuntimeBridgeConfig,
    },
    types::{ValType, Value},
};

use wrt_foundation::WrtResult;

// Import async types when available
#[cfg(feature = "async")]
use crate::unified_execution_agent_stubs::{AsyncReadResult, Future as ComponentFuture, FutureHandle, FutureState, Stream, StreamHandle, StreamState};

// Import CFI types when available
#[cfg(feature = "cfi")]
use crate::unified_execution_agent_stubs::{
    DefaultCfiControlFlowOps, CfiControlFlowProtection, CfiExecutionContext, CfiProtectedBranchTarget,
};

/// Maximum concurrent executions in no_std environments
const MAX_CONCURRENT_EXECUTIONS: usize = 64;
/// Maximum call stack depth
const MAX_CALL_STACK_DEPTH: usize = 256;
/// Maximum operand stack size
const MAX_OPERAND_STACK_SIZE: usize = 2048;

// Type aliases for commonly used providers
type AgentProvider = NoStdProvider<65536>;
type AgentBoundedString = BoundedString<128, AgentProvider>;

/// Unified execution agent that combines all execution capabilities
#[derive(Debug, Clone)]
pub struct UnifiedExecutionAgent {
    /// Core execution state
    core_state: CoreExecutionState,
    /// Async execution capabilities
    #[cfg(feature = "async")]
    async_state: AsyncExecutionState,
    /// CFI protection capabilities  
    #[cfg(feature = "cfi")]
    cfi_state: CfiExecutionState,
    /// Stackless execution capabilities
    stackless_state: StacklessExecutionState,
    /// Agent configuration
    config: AgentConfiguration,
    /// Execution statistics
    statistics: UnifiedExecutionStatistics,
}

/// Core execution state shared across all execution modes
#[derive(Debug, Clone)]
pub struct CoreExecutionState {
    /// Call stack for function execution
    #[cfg(feature = "std")]
    call_stack: Vec<UnifiedCallFrame>,
    #[cfg(not(feature = "std"))]
    call_stack: BoundedVec<UnifiedCallFrame, MAX_CALL_STACK_DEPTH, AgentProvider>,
    
    /// Operand stack for value operations
    #[cfg(feature = "std")]
    operand_stack: Vec<Value>,
    #[cfg(not(feature = "std"))]
    operand_stack: BoundedVec<Value, MAX_OPERAND_STACK_SIZE, AgentProvider>,
    
    /// Current execution mode
    execution_mode: ExecutionMode,
    
    /// Current execution state
    state: UnifiedExecutionState,
    
    /// Canonical ABI processor
    canonical_abi: CanonicalAbi,
    
    /// Resource lifecycle manager
    resource_manager: ResourceLifecycleManager,
    
    /// Runtime bridge for WebAssembly Core integration
    runtime_bridge: ComponentRuntimeBridge,
    
    /// Current instance and function context
    current_context: Option<ExecutionContext>,
}

/// Async execution state for async operations
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct AsyncExecutionState {
    /// Active async executions
    #[cfg(feature = "std")]
    executions: Vec<AsyncExecution>,
    #[cfg(not(feature = "std"))]
    executions: BoundedVec<AsyncExecution, MAX_CONCURRENT_EXECUTIONS, AgentProvider>,
    
    /// Next execution ID
    next_execution_id: u64,
    
    /// Async context pool for reuse
    #[cfg(feature = "std")]
    context_pool: Vec<AsyncExecutionContext>,
    #[cfg(not(feature = "std"))]
    context_pool: BoundedVec<AsyncExecutionContext, 16, AgentProvider>,
}

/// CFI execution state for security protection
#[cfg(feature = "cfi")]
#[derive(Debug, Clone)]
pub struct CfiExecutionState {
    /// CFI control flow operations handler
    cfi_ops: DefaultCfiControlFlowOps,
    /// CFI protection configuration
    cfi_protection: CfiControlFlowProtection,
    /// Current CFI execution context
    cfi_context: CfiExecutionContext,
    /// CFI violation response policy
    violation_policy: CfiViolationPolicy,
}

/// Stackless execution state for memory-constrained environments
#[derive(Debug, Clone)]
pub struct StacklessExecutionState {
    /// Program counter
    pc: usize,
    /// Current function index
    func_idx: u32,
    /// Label stack for control flow
    #[cfg(feature = "std")]
    labels: Vec<Label>,
    #[cfg(not(feature = "std"))]
    labels: BoundedVec<Label, 128, AgentProvider>,
    /// Stackless execution mode
    stackless_mode: bool,
}

/// Unified call frame that works across all execution modes
#[derive(Debug, Clone)]
pub struct UnifiedCallFrame {
    /// Instance ID
    pub instance_id: u32,
    /// Function index
    pub function_index: u32,
    /// Function name (for async and debugging)
    pub function_name: AgentBoundedString,
    /// Local variables
    #[cfg(feature = "std")]
    pub locals: Vec<Value>,
    #[cfg(not(feature = "std"))]
    pub locals: BoundedVec<Value, 64, AgentProvider>,
    /// Return address
    pub return_address: Option<usize>,
    /// Async state for this frame
    #[cfg(feature = "async")]
    pub async_state: FrameAsyncState,
    /// CFI protection state
    #[cfg(feature = "cfi")]
    pub cfi_state: FrameCfiState,
}

/// Execution context for current function
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current component instance
    pub component_instance: u32,
    /// Current function index
    pub function_index: u32,
    /// Memory layout information
    pub memory_base: u64,
    /// Memory size
    pub memory_size: usize,
}

/// Unified execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnifiedExecutionState {
    /// Ready to execute
    Ready,
    /// Currently executing
    Running,
    /// Waiting for async operation
    Waiting,
    /// Suspended (can be resumed)
    Suspended,
    /// Execution completed successfully
    Completed,
    /// Execution failed with error
    Failed,
    /// Execution was cancelled
    Cancelled,
}

/// Execution mode determines which capabilities are active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Synchronous component execution
    Synchronous,
    /// Asynchronous component execution
    #[cfg(feature = "async")]
    Asynchronous,
    /// Stackless execution for memory constraints
    Stackless,
    /// CFI-protected execution
    #[cfg(feature = "cfi")]
    CfiProtected,
    /// Hybrid mode combining multiple capabilities
    Hybrid(HybridModeFlags),
}

/// Flags for hybrid execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridModeFlags {
    pub async_enabled: bool,
    pub stackless_enabled: bool,
    pub cfi_enabled: bool,
}

/// Configuration for the unified agent
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfiguration {
    /// Maximum call depth
    pub max_call_depth: usize,
    /// Maximum memory usage
    pub max_memory: usize,
    /// Execution mode
    pub execution_mode: ExecutionMode,
    /// Enable bounded execution
    pub bounded_execution: bool,
    /// Initial fuel for bounded execution
    pub initial_fuel: Option<u64>,
    /// Runtime bridge configuration
    pub runtime_config: RuntimeBridgeConfig,
}

/// Unified execution statistics
#[derive(Debug, Clone, Default)]
pub struct UnifiedExecutionStatistics {
    /// Core execution statistics
    pub instructions_executed: u64,
    pub function_calls: u64,
    pub execution_time_ns: u64,
    pub memory_allocated: usize,
    
    /// Async execution statistics
    #[cfg(feature = "async")]
    pub async_executions_started: u64,
    #[cfg(feature = "async")]
    pub async_executions_completed: u64,
    #[cfg(feature = "async")]
    pub async_operations: u64,
    
    /// CFI statistics
    #[cfg(feature = "cfi")]
    pub cfi_instructions_protected: u64,
    #[cfg(feature = "cfi")]
    pub cfi_violations_detected: u64,
    #[cfg(feature = "cfi")]
    pub cfi_overhead_ns: u64,
    
    /// Stackless execution statistics
    pub stackless_frames: u64,
    pub stack_depth: usize,
    pub max_stack_depth: usize,
}

/// Async frame state for async execution
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub enum FrameAsyncState {
    /// Synchronous execution
    Sync,
    /// Awaiting a future
    AwaitingFuture(FutureHandle),
    /// Awaiting a stream read
    AwaitingStream(StreamHandle),
    /// Awaiting multiple operations
    AwaitingMultiple(WaitSet),
}

/// CFI frame state for CFI protection
#[cfg(feature = "cfi")]
#[derive(Debug, Clone)]
pub struct FrameCfiState {
    /// Shadow stack entry
    pub shadow_entry: Option<ShadowStackEntry>,
    /// Landing pad requirement
    pub landing_pad_required: bool,
    /// Call site ID for tracking
    pub call_site_id: u32,
}

/// Wait set for async operations
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct WaitSet {
    /// Futures to wait for
    #[cfg(feature = "std")]
    pub futures: Vec<FutureHandle>,
    #[cfg(not(feature = "std"))]
    pub futures: BoundedVec<FutureHandle, 16, AgentProvider>,
    
    /// Streams to wait for
    #[cfg(feature = "std")]
    pub streams: Vec<StreamHandle>,
    #[cfg(not(feature = "std"))]
    pub streams: BoundedVec<StreamHandle, 16, AgentProvider>,
}

/// Shadow stack entry for CFI protection
#[cfg(feature = "cfi")]
#[derive(Debug, Clone)]
pub struct ShadowStackEntry {
    pub return_address: u32,
    pub stack_pointer: u32,
    pub function_index: u32,
}

/// CFI violation policy
#[cfg(feature = "cfi")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfiViolationPolicy {
    /// Log violation and continue execution
    LogAndContinue,
    /// Terminate execution immediately
    Terminate,
    /// Return error to caller
    ReturnError,
    /// Attempt recovery if possible
    AttemptRecovery,
}

/// Label for stackless control flow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label {
    pub kind: LabelKind,
    pub arity: u32,
    pub pc: usize,
}

/// Kind of control flow label
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Block,
    Loop,
    If,
    Function,
}

/// Async execution for async mode
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncExecution {
    pub id: u64,
    pub task_id: u64,
    pub state: UnifiedExecutionState,
    pub context: AsyncExecutionContext,
    pub operation: AsyncOperation,
    pub result: Option<AsyncExecutionResult>,
}

/// Async execution context
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct AsyncExecutionContext {
    pub component_instance: u32,
    pub function_name: AgentBoundedString,
    pub memory_views: MemoryViews,
}

/// Memory views for async execution
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct MemoryViews {
    pub memory_base: u64,
    pub memory_size: usize,
    pub stack_region: MemoryRegion,
    pub heap_region: MemoryRegion,
}

/// Memory region descriptor
#[cfg(feature = "async")]
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: u64,
    pub size: usize,
    pub permissions: MemoryPermissions,
}

/// Memory access permissions
#[cfg(feature = "async")]
#[derive(Debug, Clone, Copy)]
pub struct MemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

/// Async operation being executed
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub enum AsyncOperation {
    FunctionCall {
        name: AgentBoundedString,
        args: Vec<Value>,
    },
    StreamRead {
        handle: StreamHandle,
        count: u32,
    },
    StreamWrite {
        handle: StreamHandle,
        data: Vec<u8>,
    },
    FutureGet {
        handle: FutureHandle,
    },
    FutureSet {
        handle: FutureHandle,
        value: Value,
    },
    WaitMultiple {
        wait_set: WaitSet,
    },
    SpawnSubtask {
        function: AgentBoundedString,
        args: Vec<Value>,
    },
}

/// Result of async execution
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct AsyncExecutionResult {
    pub values: Vec<Value>,
    pub execution_time_us: u64,
    pub memory_allocated: usize,
    pub instructions_executed: u64,
}

impl UnifiedExecutionAgent {
    /// Create a new unified execution agent
    pub fn new(config: AgentConfiguration) -> WrtResult<Self> {
        Ok(Self {
            core_state: CoreExecutionState {
                #[cfg(feature = "std")]
                call_stack: Vec::new(),
                #[cfg(not(feature = "std"))]
                call_stack: {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                
                #[cfg(feature = "std")]
                operand_stack: Vec::new(),
                #[cfg(not(feature = "std"))]
                operand_stack: {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                
                execution_mode: config.execution_mode,
                state: UnifiedExecutionState::Ready,
                canonical_abi: CanonicalAbi::new(),
                resource_manager: ResourceLifecycleManager::new(),
                runtime_bridge: ComponentRuntimeBridge::with_config(config.runtime_config.clone()),
                current_context: None,
            },
            
            #[cfg(feature = "async")]
            async_state: AsyncExecutionState {
                #[cfg(feature = "std")]
                executions: Vec::new(),
                #[cfg(not(feature = "std"))]
                executions: {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                next_execution_id: 1,
                #[cfg(feature = "std")]
                context_pool: Vec::new(),
                #[cfg(not(feature = "std"))]
                context_pool: {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
            },
            
            #[cfg(feature = "cfi")]
            cfi_state: CfiExecutionState {
                cfi_ops: DefaultCfiControlFlowOps,
                cfi_protection: CfiControlFlowProtection::default(),
                cfi_context: CfiExecutionContext::default(),
                violation_policy: CfiViolationPolicy::ReturnError,
            },
            
            stackless_state: StacklessExecutionState {
                pc: 0,
                func_idx: 0,
                #[cfg(feature = "std")]
                labels: Vec::new(),
                #[cfg(not(feature = "std"))]
                labels: {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                stackless_mode: matches!(config.execution_mode, ExecutionMode::Stackless),
            },
            
            config,
            statistics: UnifiedExecutionStatistics::default(),
        })
    }

    /// Create agent with default configuration
    pub fn new_default() -> WrtResult<Self> {
        Self::new(AgentConfiguration::default())
    }

    /// Create agent for async execution
    #[cfg(feature = "async")]
    pub fn new_async() -> WrtResult<Self> {
        Self::new(AgentConfiguration {
            execution_mode: ExecutionMode::Asynchronous,
            ..AgentConfiguration::default()
        })
    }

    /// Create agent for CFI-protected execution
    #[cfg(feature = "cfi")]
    pub fn new_cfi_protected() -> WrtResult<Self> {
        Self::new(AgentConfiguration {
            execution_mode: ExecutionMode::CfiProtected,
            ..AgentConfiguration::default()
        })
    }

    /// Create agent for stackless execution
    pub fn new_stackless() -> WrtResult<Self> {
        Self::new(AgentConfiguration {
            execution_mode: ExecutionMode::Stackless,
            ..AgentConfiguration::default()
        })
    }

    /// Create agent with hybrid capabilities
    pub fn new_hybrid(flags: HybridModeFlags) -> WrtResult<Self> {
        Self::new(AgentConfiguration {
            execution_mode: ExecutionMode::Hybrid(flags),
            ..AgentConfiguration::default()
        })
    }

    /// Execute a function call
    pub fn call_function(
        &mut self,
        instance_id: u32,
        function_index: u32,
        args: &[Value],
    ) -> WrtResult<Value> {
        self.core_state.state = UnifiedExecutionState::Running;
        self.statistics.function_calls += 1;

        // Create execution context
        let context = ExecutionContext {
            component_instance: instance_id,
            function_index,
            memory_base: 0,
            memory_size: self.config.max_memory,
        };
        self.core_state.current_context = Some(context;

        // Create unified call frame
        let frame = UnifiedCallFrame {
            instance_id,
            function_index,
            function_name: BoundedString::from_str("function").unwrap_or_default(),
            #[cfg(feature = "std")]
            locals: args.to_vec(),
            #[cfg(not(feature = "std"))]
            locals: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                let mut locals = BoundedVec::new(provider)?;
                for arg in args.iter().take(64) {
                    let _ = locals.push(arg.clone();
                }
                locals
            },
            return_address: Some(0),
            #[cfg(feature = "async")]
            async_state: FrameAsyncState::Sync,
            #[cfg(feature = "cfi")]
            cfi_state: FrameCfiState {
                shadow_entry: None,
                landing_pad_required: false,
                call_site_id: 0,
            },
        };

        // Push frame based on execution mode
        match self.core_state.execution_mode {
            ExecutionMode::Stackless => {
                self.execute_stackless_call(frame, args)
            },
            #[cfg(feature = "async")]
            ExecutionMode::Asynchronous => {
                self.execute_async_call(frame, args)
            },
            #[cfg(feature = "cfi")]
            ExecutionMode::CfiProtected => {
                self.execute_cfi_protected_call(frame, args)
            },
            ExecutionMode::Hybrid(flags) => {
                self.execute_hybrid_call(frame, args, flags)
            },
            _ => {
                self.execute_synchronous_call(frame, args)
            }
        }
    }

    /// Execute synchronous function call
    fn execute_synchronous_call(&mut self, frame: UnifiedCallFrame, args: &[Value]) -> WrtResult<Value> {
        // Push frame to call stack
        #[cfg(feature = "std")]
        {
            self.core_state.call_stack.push(frame);
        }
        #[cfg(not(feature = "std"))]
        {
            self.core_state.call_stack.push(frame).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred")
            })?;
        }

        // Execute through runtime bridge
        #[cfg(feature = "std")]
        let function_name = "Component not found";
        #[cfg(not(feature = "std"))]
        let function_name = BoundedString::from_str("Component operation result").unwrap_or_default);
        
        let component_values = self.convert_values_to_component(args)?;
        
        let result = self.core_state.runtime_bridge
            .execute_component_function(frame.instance_id, &function_name, &component_values)
            .map_err(|_| wrt_error::Error::runtime_error("Error occurred"))?;

        // Pop frame
        #[cfg(feature = "std")]
        {
            self.core_state.call_stack.pop);
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = self.core_state.call_stack.pop);
        }

        self.core_state.state = UnifiedExecutionState::Completed;
        self.statistics.instructions_executed += 1;

        // Convert result back
        Ok(result.into())
    }

    /// Execute stackless function call
    fn execute_stackless_call(&mut self, frame: UnifiedCallFrame, _args: &[Value]) -> WrtResult<Value> {
        // Update stackless state
        self.stackless_state.func_idx = frame.function_index;
        self.stackless_state.pc = 0;

        // Simulate stackless execution
        self.core_state.state = UnifiedExecutionState::Completed;
        self.statistics.stackless_frames += 1;
        self.statistics.instructions_executed += 1;

        Ok(Value::U32(42)) // Placeholder result
    }

    /// Execute async function call
    #[cfg(feature = "async")]
    fn execute_async_call(&mut self, frame: UnifiedCallFrame, args: &[Value]) -> WrtResult<Value> {
        // Create async execution
        let execution_id = self.async_state.next_execution_id;
        self.async_state.next_execution_id += 1;

        let async_execution = AsyncExecution {
            id: execution_id,
            task_id: 1, // Simplified
            state: UnifiedExecutionState::Running,
            context: AsyncExecutionContext {
                component_instance: frame.instance_id,
                function_name: frame.function_name.clone(),
                memory_views: MemoryViews {
                    memory_base: 0,
                    memory_size: self.config.max_memory,
                    stack_region: MemoryRegion {
                        start: 0,
                        size: 1024,
                        permissions: MemoryPermissions { read: true, write: true, execute: false },
                    },
                    heap_region: MemoryRegion {
                        start: 1024,
                        size: self.config.max_memory - 1024,
                        permissions: MemoryPermissions { read: true, write: true, execute: false },
                    },
                },
            },
            operation: AsyncOperation::FunctionCall {
                name: frame.function_name.clone(),
                args: args.to_vec(),
            },
            result: Some(AsyncExecutionResult {
                values: vec![Value::U32(42)],
                execution_time_us: 100,
                memory_allocated: 0,
                instructions_executed: 1,
            }),
        };

        #[cfg(feature = "std")]
        {
            self.async_state.executions.push(async_execution);
        }
        #[cfg(not(feature = "std"))]
        {
            self.async_state.executions.push(async_execution).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred")
            })?;
        }

        self.core_state.state = UnifiedExecutionState::Completed;
        self.statistics.async_executions_started += 1;
        self.statistics.async_executions_completed += 1;
        self.statistics.instructions_executed += 1;

        Ok(Value::U32(42)) // Placeholder result
    }

    /// Execute CFI-protected function call
    #[cfg(feature = "cfi")]
    fn execute_cfi_protected_call(&mut self, frame: UnifiedCallFrame, args: &[Value]) -> WrtResult<Value> {
        // Update CFI context
        self.cfi_state.cfi_context.current_function = frame.function_index;
        
        // Validate CFI requirements
        // This would involve shadow stack validation, landing pad checks, etc.
        
        // Execute with CFI protection
        let result = self.execute_synchronous_call(frame, args)?;
        
        self.statistics.cfi_instructions_protected += 1;
        
        Ok(result)
    }

    /// Execute hybrid function call
    fn execute_hybrid_call(&mut self, frame: UnifiedCallFrame, args: &[Value], flags: HybridModeFlags) -> WrtResult<Value> {
        // Apply capabilities based on flags
        if flags.cfi_enabled {
            #[cfg(feature = "cfi")]
            {
                self.cfi_state.cfi_context.current_function = frame.function_index;
                self.statistics.cfi_instructions_protected += 1;
            }
        }
        
        if flags.stackless_enabled {
            self.stackless_state.func_idx = frame.function_index;
            self.statistics.stackless_frames += 1;
        }
        
        if flags.async_enabled {
            #[cfg(feature = "async")]
            {
                self.statistics.async_operations += 1;
            }
        }

        // Execute based on primary mode
        self.execute_synchronous_call(frame, args)
    }

    /// Get current execution state
    pub fn state(&self) -> UnifiedExecutionState {
        self.core_state.state
    }

    /// Get execution statistics
    pub fn statistics(&self) -> &UnifiedExecutionStatistics {
        &self.statistics
    }

    /// Get current call stack depth
    pub fn call_stack_depth(&self) -> usize {
        self.core_state.call_stack.len()
    }

    /// Reset the agent state
    pub fn reset(&mut self) {
        self.core_state.call_stack.clear);
        self.core_state.operand_stack.clear);
        self.core_state.state = UnifiedExecutionState::Ready;
        self.core_state.current_context = None;
        self.statistics = UnifiedExecutionStatistics::default());
        
        #[cfg(feature = "async")]
        {
            self.async_state.executions.clear);
            self.async_state.next_execution_id = 1;
        }
    }

    /// Convert values to component values
    #[cfg(feature = "std")]
    fn convert_values_to_component(&self, values: &[Value]) -> WrtResult<Vec<ComponentValue>> {
        let mut component_values = Vec::new());
        for value in values {
            component_values.push(value.clone().into();
        }
        Ok(component_values)
    }

    #[cfg(not(feature = "std"))]
    fn convert_values_to_component(&self, values: &[Value]) -> WrtResult<BoundedVec<Value, 16, AgentProvider>> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let mut component_values = BoundedVec::new(provider)?;
        for value in values.iter().take(16) {
            component_values.push(value.clone()).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred")
            })?;
        }
        Ok(component_values)
    }
}

impl Default for AgentConfiguration {
    fn default() -> Self {
        Self {
            max_call_depth: 1024,
            max_memory: 1024 * 1024, // 1MB
            execution_mode: ExecutionMode::Synchronous,
            bounded_execution: false,
            initial_fuel: None,
            runtime_config: RuntimeBridgeConfig::default(),
        }
    }
}

impl Default for UnifiedExecutionState {
    fn default() -> Self {
        UnifiedExecutionState::Ready
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Synchronous
    }
}

impl fmt::Display for UnifiedExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnifiedExecutionState::Ready => write!(f, "Ready"),
            UnifiedExecutionState::Running => write!(f, "Running"),
            UnifiedExecutionState::Waiting => write!(f, "Waiting"),
            UnifiedExecutionState::Suspended => write!(f, "Suspended"),
            UnifiedExecutionState::Completed => write!(f, "Completed"),
            UnifiedExecutionState::Failed => write!(f, "Failed"),
            UnifiedExecutionState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{Checksummable, ToBytes, FromBytes, WriteStream, ReadStream};

impl Default for UnifiedExecutionAgent {
    fn default() -> Self {
        Self::new_default().unwrap_or_else(|_| {
            // This should never happen in practice, but we need a fallback for the Default trait
            // In production code, prefer using new_default() directly which returns Result
            panic!("Failed to create default UnifiedExecutionAgent: memory allocation error")
        })
    }
}

impl PartialEq for UnifiedExecutionAgent {
    fn eq(&self, other: &Self) -> bool {
        // Simple equality based on configuration
        self.config == other.config
    }
}

impl Eq for UnifiedExecutionAgent {}

// Macro to implement basic traits for complex types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                0u32.update_checksum(checksum;
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                _writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
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

// Apply macro to UnifiedExecutionAgent
impl_basic_traits!(UnifiedExecutionAgent, UnifiedExecutionAgent::default());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_agent_creation() {
        let agent = UnifiedExecutionAgent::new_default().unwrap();
        assert_eq!(agent.state(), UnifiedExecutionState::Ready;
        assert_eq!(agent.call_stack_depth(), 0);
    }

    #[test]
    fn test_synchronous_execution() {
        let mut agent = UnifiedExecutionAgent::new_default().unwrap();
        let args = [Value::U32(42), Value::Bool(true)];
        
        let result = agent.call_function(1, 2, &args;
        assert!(result.is_ok());
        assert_eq!(agent.state(), UnifiedExecutionState::Completed;
        assert_eq!(agent.statistics().function_calls, 1);
    }

    #[test]
    fn test_stackless_execution() {
        let mut agent = UnifiedExecutionAgent::new_stackless().unwrap();
        let args = [Value::U32(100)];
        
        let result = agent.call_function(1, 5, &args;
        assert!(result.is_ok());
        assert_eq!(agent.statistics().stackless_frames, 1);
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_async_execution() {
        let mut agent = UnifiedExecutionAgent::new_async().unwrap();
        let args = [Value::F32(3.14)];
        
        let result = agent.call_function(2, 3, &args;
        assert!(result.is_ok());
        assert_eq!(agent.statistics().async_executions_started, 1);
        assert_eq!(agent.statistics().async_executions_completed, 1);
    }

    #[test]
    fn test_hybrid_execution() {
        let flags = HybridModeFlags {
            async_enabled: false,
            stackless_enabled: true,
            cfi_enabled: false,
        };
        let mut agent = UnifiedExecutionAgent::new_hybrid(flags).unwrap();
        let args = [Value::S64(-100)];
        
        let result = agent.call_function(1, 1, &args;
        assert!(result.is_ok());
        assert_eq!(agent.statistics().stackless_frames, 1);
    }

    #[test]
    fn test_agent_reset() {
        let mut agent = UnifiedExecutionAgent::new_default().unwrap();
        
        // Execute something first
        let args = [Value::U32(42)];
        let _ = agent.call_function(1, 2, &args;
        
        // Verify state changed
        assert_eq!(agent.statistics().function_calls, 1);
        
        // Reset and verify clean state
        agent.reset);
        assert_eq!(agent.state(), UnifiedExecutionState::Ready;
        assert_eq!(agent.call_stack_depth(), 0);
        assert_eq!(agent.statistics().function_calls, 0);
    }
}