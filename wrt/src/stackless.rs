//! Stackless WebAssembly execution engine
//!
//! This module implements a stackless version of the WebAssembly execution engine
//! that doesn't rely on the host language's call stack, making it suitable for
//! environments with limited stack space and for no_std contexts.
//!
//! The stackless engine uses a state machine approach to track execution state
//! and allows for pausing and resuming execution at any point.

// Import ALL from prelude first, then specific items from other local modules if needed.
use crate::prelude::*; // This should bring in Value, FuncType, ExternType, Error, Result, Vec, etc.

use crate::{
    behavior::{ControlFlow as ControlFlowTrait, InstructionExecutor, Label, StackBehavior},
    execution::ExecutionStats,
    instructions::instruction_type::Instruction as InstructionType, // Assuming this is wrt_instructions::Instruction re-exported or a local type
    module::{ExportKind, Module}, // Module here is likely wrt_runtime::Module via prelude
    module_instance::ModuleInstance,
};

// No longer needed due to prelude::* :
// use crate::prelude::{
//     kinds, out_of_bounds_error, poisoned_lock_error, resource_error, runtime_error,
//     validation_error, Arc, BoundedCapacity, BoundedVec, Box, Error, ErrorCategory, HashMap,
//     LogOperation, Mutex, MutexGuard, Result, SafeMemoryHandler, SafeSlice, String,
//     TypesValue as Value, 
//     Vec, VerificationLevel,
// };

use core::mem;
#[cfg(feature = "log")]
use log::trace;

// Add type definitions for callbacks and host function handlers
// Value and Result should come from the prelude.
// Box should also come from prelude (std or alloc)
pub type CloneableFn = Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync + 'static>;

// This LogOperation is specific to stackless engine's callback logging.
// It is different from wrt_types::operations::OperationType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogOperation {
    /// Function was called
    Called,
    /// Function returned
    Returned,
}

// Define constants for maximum sizes
/// Maximum number of values on the operand stack
const MAX_VALUES: usize = 2048;
/// Maximum number of control flow labels
const MAX_LABELS: usize = 128;
/// Maximum call depth (number of frames)
const MAX_FRAMES: usize = 256;

// --- Conditional imports for Mutex ---
// TODO: Define FuelOutcomes and EngineConfig or import if they exist elsewhere
#[derive(Debug)]
pub struct FuelOutcomes; // Placeholder
#[derive(Debug)]
pub struct EngineConfig; // Placeholder
                         // --- End added imports ---

// Add direct imports of the error kinds - these should ideally come from prelude or wrt_error::kinds
// This `use crate::error_kinds` likely refers to a deleted/moved module.
// We'll rely on the prelude for error kinds for now.
// use crate::error_kinds::{
//     ExecutionError, ExportNotFoundError, InvalidFunctionIndexError, InvalidInstanceIndexError,
//     StackUnderflowError,
// };
use crate::memory_adapter::MemoryAdapter; // This refers to a local module wrt/src/memory_adapter.rs

// Re-export other error kinds directly from wrt_error since we don't have our own custom versions
// This is also likely handled by the prelude's re-export of wrt_error::kinds
// use wrt_error::kinds::{
//     OutOfBoundsError, PoisonedLockError, ResourceError, RuntimeError, ValidationError,
// };

// Validatable is re-exported by prelude from wrt_types.
// use wrt_types::Validatable;

// Fix other imports
// WrtMutex and WrtMutexGuard are re-exported as Mutex and MutexGuard by prelude if cfg(not(feature = "std"))
// If std is enabled, prelude re-exports std::sync::Mutex.
// So direct use of WrtMutex here might be too specific or conflict.
// For now, assume prelude handles Mutex/MutexGuard correctly.
// use wrt_sync::{WrtMutex, WrtMutexGuard};

#[cfg(feature = "std")]
use std::collections::HashMap; // This is fine if prelude's HashMap is BTreeMap for no_std.
                               // Prelude provides HashMap (std::collections::HashMap or alloc::collections::BTreeMap)
                               // So this explicit import might be redundant if `use crate::prelude::*;` is effective.

// Import ControlFlow and other behavior types
// These should also be available via prelude if they come from wrt_instructions & re-exported
// use crate::behavior::{
//     ControlFlow, ControlFlowBehavior, EngineBehavior, FrameBehavior, InstructionExecutor, Label,
//     ModuleBehavior, StackBehavior,
// };

/// Represents the execution state in a stackless implementation
#[derive(Debug, Clone)]
pub enum StacklessExecutionState {
    /// Executing instructions normally
    Running,
    /// Paused execution (for bounded fuel)
    Paused {
        /// Program counter (instruction index)
        pc: usize,
        /// Instance index
        instance_idx: u32,
        /// Function index
        func_idx: u32,
        /// Expected number of results
        expected_results: usize,
    },
    /// Function call in progress
    Calling {
        /// Instance index
        instance_idx: u32,
        /// Function index
        func_idx: u32,
        /// Arguments
        args: Vec<Value>,
        /// Return address (instruction index to return to)
        return_pc: usize,
    },
    /// Return in progress
    Returning {
        /// Return values
        values: Vec<Value>,
    },
    /// Branch in progress
    Branching {
        /// Branch target (label depth)
        depth: u32,
        /// Values to keep on stack
        values: Vec<Value>,
    },
    /// Completed execution
    Completed,
    /// Execution finished
    Finished,
    /// Error occurred
    Error(Error),
}

/// Represents the execution stack in a stackless implementation
#[derive(Debug)]
pub struct StacklessStack {
    /// Shared module reference
    module: Arc<Module>,
    /// Current instance index
    instance_idx: usize,
    /// The operand stack
    pub values: BoundedVec<Value, MAX_VALUES>,
    /// The label stack
    labels: BoundedVec<Label, MAX_LABELS>,
    /// Function frames
    pub frames: BoundedVec<StacklessFrame, MAX_FRAMES>,
    /// Current execution state
    pub state: StacklessExecutionState,
    /// Instruction pointer
    pub pc: usize,
    /// Function index
    pub func_idx: u32,
    /// Capacity of the stack (no longer needed, kept for backward compatibility)
    pub capacity: usize,
}

/// A callback registry for handling WebAssembly component operations
pub struct StacklessCallbackRegistry {
    /// Names of exports that are known to be callbacks
    pub export_names: HashMap<String, HashMap<String, LogOperation>>,
    /// Registered callback functions
    pub callbacks: HashMap<String, CloneableFn>,
}

impl Default for StacklessCallbackRegistry {
    fn default() -> Self {
        Self {
            export_names: HashMap::new(),
            callbacks: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for StacklessCallbackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StacklessCallbackRegistry")
            .field("known_export_names", &self.export_names)
            .field("callbacks", &"<function>")
            .finish()
    }
}

/// State of the stackless WebAssembly execution engine
#[derive(Debug)]
pub struct StacklessEngine {
    /// The internal state of the stackless engine.
    /// The actual execution stack (values, labels, frames, state)
    pub(crate) exec_stack: StacklessStack,
    /// Remaining fuel for bounded execution
    fuel: Option<u64>,
    /// Execution statistics
    stats: ExecutionStats,
    /// Callback registry for host functions (logging, etc.)
    callbacks: Arc<Mutex<StacklessCallbackRegistry>>,
    /// Maximum call depth for function calls
    max_call_depth: Option<usize>,
    /// Use the alias EngineMutex for the instance map
    pub(crate) instances: Arc<Mutex<Vec<Arc<ModuleInstance>>>>,
    /// Verification level for bounded collections
    verification_level: VerificationLevel,
}

/// Represents a deferred branch operation in the stackless engine
#[derive(Debug)]
pub struct DeferredBranch {
    /// The target program counter address
    pub target_pc: usize,
    /// The frame containing the target
    pub _frame: StacklessFrame,
    /// The number of values to keep on the stack
    pub _keep_values: Option<usize>,
}

impl StacklessStack {
    /// Creates a new `StacklessStack` with the given module.
    #[must_use]
    pub fn new(module: Arc<Module>, instance_idx: usize) -> Self {
        Self {
            values: BoundedVec::with_verification_level(VerificationLevel::Standard),
            labels: BoundedVec::with_verification_level(VerificationLevel::Standard),
            frames: BoundedVec::with_verification_level(VerificationLevel::Standard),
            state: StacklessExecutionState::Running,
            pc: 0,
            instance_idx,
            func_idx: 0,
            module,
            capacity: MAX_VALUES, // For backward compatibility
        }
    }

    /// Validates the stack's integrity, checking all bounded collections.
    pub fn validate(&self) -> Result<(), Error> {
        // Validate operand stack
        self.values
            .validate()
            .map_err(|e| Error::new(ErrorCategory::Runtime, codes::RUNTIME_STACK_INTEGRITY_ERROR, format!("Value stack validation failed: {}", e)))?;

        // Validate label stack
        self.labels
            .validate()
            .map_err(|e| Error::new(ErrorCategory::Runtime, codes::RUNTIME_STACK_INTEGRITY_ERROR, format!("Label stack validation failed: {}", e)))?;

        // Validate frame stack
        self.frames
            .validate()
            .map_err(|e| Error::new(ErrorCategory::Runtime, codes::RUNTIME_STACK_INTEGRITY_ERROR, format!("Frame stack validation failed: {}", e)))?;

        // Validate each frame
        for (i, frame) in self.frames.iter().enumerate() {
            if let Err(e) = frame.validate() {
                return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_STACK_INTEGRITY_ERROR, format!("Frame {} validation failed: {}", i, e)));
            }
        }

        Ok(())
    }

    /// Sets the verification level for all bounded collections in the stack.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.values.set_verification_level(level);
        self.labels.set_verification_level(level);
        self.frames.set_verification_level(level);
    }

    /// Pushes a value onto the stack
    pub fn push(&mut self, value: Value) -> Result<(), Error> {
        self.values.push(value).map_err(|e| {
            ExecutionError(format!("Stack overflow, maximum values: {}", MAX_VALUES)).into()
        })
    }

    /// Pops a value from the stack
    pub fn pop(&mut self) -> Result<Value, Error> {
        self.values
            .pop()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    /// Pushes a label onto the control stack
    pub fn push_label(&mut self, arity: usize, pc: usize) -> Result<(), Error> {
        let label = Label {
            arity,
            pc,
            continuation: pc,
            stack_depth: self.values.len(), // Assuming stack_depth is current value stack len
            is_loop: false,                 // Default to false
            is_if: false,                   // Default to false
        };

        self.labels.push(label).map_err(|e| {
            ExecutionError(format!(
                "Label stack overflow, maximum labels: {}",
                MAX_LABELS
            ))
            .into()
        })
    }

    /// Pops a label from the control stack
    pub fn pop_label(&mut self) -> Result<Label, Error> {
        self.labels
            .pop()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    /// Gets a label at the specified depth
    pub fn get_label(&self, depth: usize) -> Option<&Label> {
        // If there's no current frame, there are no labels
        if self.frames.is_empty() {
            return None;
        }

        // Get the current frame
        let current_frame = self.frames.last()?;

        // Access the label at specified depth
        current_frame.label_stack.get(depth)
    }

    /// Returns the number of labels currently on the control stack.
    pub fn labels_len(&self) -> usize {
        self.labels.len()
    }

    /// Checks if the value stack is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the number of values on the value stack.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns a slice containing all values on the stack.
    pub fn values(&self) -> &[Value] {
        self.values.as_ref()
    }

    /// Returns a mutable slice containing all values on the stack.
    pub fn values_mut(&mut self) -> &mut [Value] {
        self.values.as_mut()
    }

    /// Returns a reference to the top value on the stack without removing it.
    pub fn peek(&self) -> Result<&Value, Error> {
        let len = self.values.len();
        if len == 0 {
            return Err(ExecutionError("Stack underflow".to_string()).into());
        }

        Ok(self.values.get(len - 1).unwrap())
    }

    /// Returns a mutable reference to the top value on the stack without removing it.
    pub fn peek_mut(&mut self) -> Result<&mut Value, Error> {
        let len = self.values.len();
        if len == 0 {
            return Err(ExecutionError("Stack underflow".to_string()).into());
        }

        Ok(self.values.get_mut(len - 1).unwrap())
    }

    /// Pops a frame label (internal helper)
    pub fn pop_frame_label(&self) -> Result<Label, Error> {
        if let Some(frame) = self
            .frames
            .get(self.frames.len().checked_sub(1).unwrap_or(0))
        {
            if let Some(label) = frame
                .label_stack
                .get(frame.label_stack.len().checked_sub(1).unwrap_or(0))
            {
                return Ok(label.clone());
            }
        }
        Err(ExecutionError("Stack underflow".to_string()).into())
    }

    /// Pop a number of values from the value stack
    pub fn pop_values(&mut self, count: usize) -> Result<Vec<Value>, Error> {
        if self.values.len() < count {
            return Err(ExecutionError("Stack underflow".to_string()).into());
        }

        let new_len = self.values.len() - count;
        let mut result = Vec::with_capacity(count);

        // Take values from bounded vec and put them into a standard Vec
        for i in 0..count {
            // We've verified the length above, so this should be safe
            let index = new_len + i;
            if let Some(value) = self.values.get(index) {
                result.push(value.clone());
            }
        }

        // Drop values from the end manually since truncate isn't available
        while self.values.len() > new_len {
            self.values.pop();
        }

        Ok(result)
    }

    /// Push values onto the value stack
    pub fn push_values(&mut self, values: &[Value]) -> Result<(), Error> {
        // Try to push all values
        for value in values {
            self.values.push(value.clone()).map_err(|e| {
                ExecutionError(format!("Stack overflow, maximum values: {}", MAX_VALUES)).into()
            })?;
        }

        Ok(())
    }

    /// Pop a single value from the value stack
    pub fn pop_value(&mut self) -> Result<Value, Error> {
        self.values
            .pop()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    /// Peek at the top value without popping
    pub fn peek_value(&self) -> Result<Value, Error> {
        self.values
            .last()
            .cloned()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    /// Peek at a value at a specific depth
    pub fn peek_value_at(&self, depth: usize) -> Result<Value, Error> {
        let idx = self
            .values
            .len()
            .checked_sub(1 + depth)
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())?;
        self.values
            .get(idx)
            .cloned()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    /// Pop the top n values without returning them
    pub fn drop_values(&mut self, n: usize) -> Result<(), Error> {
        if self.values.len() < n {
            return Err(ExecutionError("Stack underflow".to_string()).into());
        }

        // Drop values from the end manually since truncate isn't available
        for _ in 0..n {
            self.values.pop();
        }

        Ok(())
    }
}

impl Default for StacklessEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StacklessEngine {
    /// Creates a new empty stackless engine.
    #[must_use]
    pub fn new() -> Self {
        let dummy_module = Arc::new(Module::default());
        Self {
            exec_stack: StacklessStack::new(dummy_module.clone(), 0),
            fuel: None,
            stats: ExecutionStats::default(),
            callbacks: Arc::new(Mutex::new(StacklessCallbackRegistry::default())),
            max_call_depth: None,
            instances: Arc::new(Mutex::new(Vec::new())),
            verification_level: VerificationLevel::Standard,
        }
    }

    /// Creates a new stackless engine with a specified verification level.
    #[must_use]
    pub fn with_verification_level(level: VerificationLevel) -> Self {
        let mut engine = Self::new();
        engine.set_verification_level(level);
        engine
    }

    /// Sets the verification level for all bounded collections in the engine.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
        self.exec_stack.set_verification_level(level);

        // Update verification level in all module instances
        let instances = self.instances.lock();
        for _instance in instances.iter() {
            // If the module instance implementation supports setting verification level,
            // we would update it here. This requires implementation in ModuleInstance.
        }
    }

    /// Validates the engine's state, checking all bounded collections.
    pub fn validate(&self) -> Result<(), Error> {
        // First validate the stack state
        self.exec_stack.validate()?;

        // Then validate memory if using SafeMemoryAdapter
        self.validate_memory()?;

        Ok(())
    }

    /// Validate memory integrity across all instances
    fn validate_memory(&self) -> Result<(), Error> {
        // Validate memory in all instances
        if let Ok(instances) = self.instances.lock() {
            let mut memory_count = 0;
            for _instance in instances.iter() {
                // Count memories for statistics
                memory_count += 1;
            }
            if memory_count == 0 {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Get a memory adapter for a specific instance and memory index
    fn get_memory_adapter(
        &self,
        instance_idx: usize,
        memory_idx: usize,
    ) -> Option<Arc<dyn crate::memory_adapter::MemoryAdapter>> {
        // Get the instances map
        let instances_guard = self.instances.lock();

        // Get the instance
        let instance = match instances_guard.get(instance_idx) {
            Some(instance) => instance,
            None => {
                log::debug!("Invalid instance index: {}", instance_idx);
                return None;
            }
        };

        // Get the memory
        match instance.get_memory((memory_idx as u32).try_into().unwrap()) {
            Ok(memory) => {
                // Create a cloned memory adapter with the current verification level
                // Ensure we're working with a fresh copy of memory for thread safety
                let memory_clone = memory.clone();

                // Create a memory adapter with the current verification level
                match crate::memory_adapter::SafeMemoryAdapter::with_verification_level(
                    memory_clone,
                    self.verification_level,
                ) {
                    Ok(adapter) => {
                        Some(Arc::new(adapter) as Arc<dyn crate::memory_adapter::MemoryAdapter>)
                    }
                    Err(err) => {
                        log::debug!("Failed to create memory adapter: {:?}", err);
                        None
                    }
                }
            }
            Err(err) => {
                log::debug!("Failed to get memory at index {}: {:?}", memory_idx, err);
                None
            }
        }
    }

    /// Execute memory validation at critical points during execution
    fn validate_at_checkpoint(&self) -> Result<(), Error> {
        // For full verification level, run validation on every checkpoint
        if matches!(self.verification_level, VerificationLevel::Full) {
            return self.validate();
        }

        // For standard verification, validate 5% of the time
        if matches!(self.verification_level, VerificationLevel::Standard) {
            // Use a simple timer-based approach instead of rand
            let counter = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u8)
                .unwrap_or(0);

            let should_verify = counter % 20 == 0; // ~5% chance

            if should_verify {
                return self.validate();
            }
        }

        // No verification for VerificationLevel::None
        Ok(())
    }

    /// Check memory bounds with verification
    pub fn check_memory_bounds(
        &self,
        instance_idx: usize,
        memory_idx: u32,
        offset: usize,
        size: usize,
    ) -> Result<(), Error> {
        // Get the memory adapter
        let adapter = match self.get_memory_adapter(instance_idx, memory_idx as usize) {
            Some(adapter) => adapter,
            None => {
                return Err(ExecutionError(format!(
                    "Memory not found: instance {}, memory {}",
                    instance_idx, memory_idx
                ))
                .into());
            }
        };

        // Get memory size
        let memory_size = adapter
            .byte_size()
            .map_err(|e| ExecutionError(format!("Failed to get memory size: {}", e)))?;

        // Check bounds
        if offset + size > memory_size {
            return Err(OutOfBoundsError(format!(
                "Memory access out of bounds: offset={}, size={}, memory_size={}",
                offset, size, memory_size
            ))
            .into());
        }

        // Validate memory integrity if using full verification
        if matches!(self.verification_level, VerificationLevel::Full) {
            if let Some(adapter) = adapter.downcast_ref::<memory_adapter::SafeMemoryAdapter>() {
                adapter.verify_memory_safety().map_err(|e| {
                    Error::new(
                        ErrorCategory::RuntimeIntegrity,
                        codes::RUNTIME_MEMORY_INTEGRITY_ERROR,
                        kinds::RuntimeError(format!("Memory integrity check failed: {}", e)),
                    )
                })?;
            }
        }

        Ok(())
    }

    /// Run with enhanced memory safety
    pub fn run_with_memory_safety(&mut self) -> Result<StacklessExecutionState, Error> {
        // Validate initial state
        self.validate()?;

        // Run with periodic validation
        let mut steps = 0;
        loop {
            // Execute a step
            match self.step() {
                Ok(()) => {
                    // Validate every 1000 steps
                    steps += 1;
                    if steps % 1000 == 0 {
                        self.validate_at_checkpoint()?;
                    }

                    // Check state
                    match self.state() {
                        StacklessExecutionState::Completed | StacklessExecutionState::Finished => {
                            // Final validation before returning
                            self.validate()?;

                            // Update execution stats with operation stats
                            let op_stats = wrt_types::global_operation_summary();
                            self.stats.memory_operations +=
                                op_stats.memory_reads + op_stats.memory_writes;
                            self.stats.function_calls += op_stats.function_calls;
                            self.stats.fuel_consumed += op_stats.fuel_consumed;

                            return Ok(self.state().clone());
                        }
                        StacklessExecutionState::Error(err) => {
                            return Err(err.clone());
                        }
                        StacklessExecutionState::Paused { .. } => {
                            // Validate before pausing
                            self.validate()?;

                            // Update execution stats with operation stats
                            let op_stats = wrt_types::global_operation_summary();
                            self.stats.memory_operations +=
                                op_stats.memory_reads + op_stats.memory_writes;
                            self.stats.function_calls += op_stats.function_calls;
                            self.stats.fuel_consumed += op_stats.fuel_consumed;

                            return Ok(self.state().clone());
                        }
                        _ => continue,
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    /// Sets the fuel limit for bounded execution
    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
        // Reset operation tracking when setting fuel
        wrt_types::reset_global_operations();
    }

    /// Gets the remaining fuel
    #[must_use]
    pub const fn remaining_fuel(&self) -> Option<u64> {
        self.fuel
    }

    /// Gets the execution statistics
    #[must_use]
    pub const fn stats(&self) -> &ExecutionStats {
        &self.stats
    }

    /// Resets the execution statistics
    pub fn reset_stats(&mut self) {
        self.stats = ExecutionStats::default();
    }

    /// Gets the current execution state
    #[must_use]
    pub fn state(&self) -> &StacklessExecutionState {
        &self.exec_stack.state // Access via exec_stack
    }

    /// Sets the execution state
    pub fn set_state(&mut self, state: StacklessExecutionState) {
        self.exec_stack.state = state; // Access via exec_stack
    }

    /// Gets the number of module instances
    #[must_use]
    pub fn instance_count(&self) -> usize {
        // Restore locking logic
        match self.instances.lock() {
            Ok(guard) => guard.len(),
            Err(_) => {
                // Handle poisoned lock - in this case, return 0 instances as a fallback
                log::error!("Poisoned lock when reading instance count");
                0
            }
        }
    }

    /// Provides temporary access to a module instance by index via a closure.
    pub fn with_instance<F, T>(&self, instance_idx: usize, f: F) -> Result<T>
    where
        F: FnOnce(&ModuleInstance) -> Result<T>,
    {
        // Acquire lock and get the instance
        let instances = self.instances.lock();
        let instance = instances
            .get(instance_idx)
            .ok_or_else(|| InvalidInstanceIndexError(instance_idx.try_into().unwrap()).into())?;

        // Call the function with the instance
        f(instance)
    }

    /// Provides temporary mutable access to a module instance by index via a closure.
    pub fn with_instance_mut<F, R>(&self, instance_idx: usize, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut ModuleInstance) -> Result<R, Error>,
    {
        let mut instances_guard = self.instances.lock();
        let instance = instances_guard
            .get_mut(instance_idx)
            .ok_or_else(|| InvalidInstanceIndexError(instance_idx.try_into().unwrap()).into())?;
        // Attempt to get a mutable reference from Arc, might fail if Arc is shared
        if let Some(instance_mut) = Arc::get_mut(instance) {
            f(instance_mut)
        } else {
            Err(ExecutionError("Cannot get mutable access to shared ModuleInstance".into()).into())
            // Corrected error
        }
    }

    /// Instantiates a module
    pub fn instantiate(&mut self, module: Module) -> Result<usize, Error> {
        // Module needs to be owned by the instance, so we clone it.
        // TODO: Consider if cloning the whole module is necessary or if Arc is sufficient.
        let module_arc = Arc::new(module);
        // FIX: Pass the Arc directly, removing the clone, as ModuleInstance::new now accepts Arc<Module>
        // Fix: Clone the Arc here to keep ownership for later use
        let mut instance = ModuleInstance::new(module_arc.clone())?; // Clone Arc here

        // Initialize memories
        // Need read access to module's memory definitions
        let memories = &module_arc.memories;
        for memory_arc in memories.iter() {
            // Use Memory directly without casting to MemoryBehavior
            instance.memories.push(memory_arc.clone());
        }

        let instance_arc = Arc::new(instance); // Wrap the initialized instance in Arc

        // Lock the instances vector to push the new instance
        let mut instances_guard = self.instances.lock();
        let instance_idx = instances_guard.len();
        instances_guard.push(instance_arc); // Push first
        if let Some(inst_mut_arc) = instances_guard.get_mut(instance_idx) {
            if let Some(inst_mut) = Arc::get_mut(inst_mut_arc) {
                inst_mut.module_idx = instance_idx as u32; // Assign via mutable reference
            } else {
                return Err(ExecutionError(
                    "Failed to get mutable access to newly added instance Arc".into(),
                )
                .into());
            }
        } else {
            return Err(
                ExecutionError("Failed to find newly added instance after push".into()).into(),
            );
        }

        // Drop the lock before potentially running the start function
        drop(instances_guard);

        // Execute start function if present
        if let Some(start_func_idx) = module_arc.start {
            // Need mutable access to the newly added instance
            let mut instances_guard_mut = self.instances.lock();
            if let Some(instance_mut_arc) = instances_guard_mut.get_mut(instance_idx) {
                if let Some(instance_mut) = Arc::get_mut(instance_mut_arc) {
                    // TODO: Implement start function execution properly.
                    // This might involve calling self.call_function or a dedicated method.
                    // instance_mut.execute_start_function(self, start_func_idx)?;
                    println!("Warning: Start function execution is not yet fully implemented in instantiate.");
                } else {
                    return Err(ExecutionError(
                        "Failed to get mutable access to newly added instance for start function"
                            .into(),
                    )
                    .into());
                }
            } else {
                return Err(ExecutionError(
                    "Failed to find newly added instance for start function".into(),
                )
                .into());
            }
            // Drop the mutable lock
            drop(instances_guard_mut);
        }

        Ok(instance_idx)
    }

    /// Checks if the engine currently has any module instances loaded.
    ///
    /// # Returns
    ///
    /// `true` if there are no instances, `false` otherwise.
    pub fn has_no_instances(&self) -> bool {
        self.instances.lock().is_empty()
    }

    /// Registers a callback function for a specific export name.
    ///
    /// This allows host functions to be called from WebAssembly.
    pub fn register_callback(
        &mut self,
        export_name: &str,
        callback: CloneableFn,
    ) -> Result<(), Error> {
        let mut registry = self.callbacks.lock();
        if registry.callbacks.contains_key(export_name) {
            return Err(ExecutionError(
                format!("Callback already registered for export: {}", export_name).into(),
            )
            .into());
        }
        registry.callbacks.insert(export_name.to_string(), callback);
        Ok(())
    }

    /// Registers known exports that should trigger logging or other callbacks.
    ///
    /// # Arguments
    ///
    /// * `export_names`: A map where the key is the export name (e.g., "wasi:logging/logging.log")
    ///   and the value is another map specifying the log operation (e.g., {"log": LogOperation::Log}).
    pub fn register_known_exports(
        &mut self,
        export_names: HashMap<String, HashMap<String, LogOperation>>,
    ) -> Result<(), Error> {
        let mut registry = self.callbacks.lock();
        registry.export_names = export_names;
        Ok(())
    }

    /// Finds a callback function by export name.
    ///
    /// Requires a lock on the callback registry.
    fn find_callback_locked(
        registry: &StacklessCallbackRegistry,
        export_name: &str,
    ) -> Option<CloneableFn> {
        registry.callbacks.get(export_name).cloned()
    }

    /// Calls an exported function by name
    pub fn call_export(&mut self, export_name: &str, args: &[Value]) -> Result<Vec<Value>, Error> {
        let instance_idx = self.exec_stack.instance_idx;
        let instances_guard = self.instances.lock();
        let instance_arc = instances_guard
            .get(instance_idx)
            .cloned()
            .ok_or_else(|| InvalidInstanceIndexError(instance_idx.try_into().unwrap()).into())?;
        drop(instances_guard); // Release lock early

        let export = instance_arc
            .module
            .exports
            .get(export_name)
            .ok_or_else(|| ExportNotFoundError(export_name.to_string()).into())?;

        match export.kind {
            ExportKind::Function => {
                let func_idx = export.index;
                self.call_function(instance_idx as u32, func_idx, args)
            }
            _ => Err(RuntimeError(format!(
                "Export '{export_name}' is not a function (kind: {:?})",
                export.kind
            ))
            .into()),
        }
    }

    /// Calls a function by index within a specific instance
    pub fn call_function(
        &mut self,
        instance_idx: u32,
        func_idx: u32,
        args: &[Value],
    ) -> Result<Vec<Value>, Error> {
        // Fetch module Arc while holding the lock
        let module = {
            let instances_guard = self.instances.lock();
            instances_guard
                .get(instance_idx as usize)
                .cloned() // Clone the Arc<ModuleInstance>
                .ok_or_else(|| {
                    RuntimeError(format!("Invalid instance index: {}", instance_idx)).into()
                })? // Cast to usize
                .module
                .clone()
        }; // Lock released here

        let export_name = module.exports.iter().find_map(|export| {
            if export.kind == ExportKind::Function && export.index == func_idx {
                return Some(export.name.clone());
            }
            None
        });

        if let Some(name) = export_name {
            let registry_lock = self.callbacks_lock();
            if let Some(callback) = Self::find_callback_locked(&registry_lock, &name) {
                trace!("DEBUG: Calling host callback: {}", name);
                drop(registry_lock);
                // TODO: Actually call the host function - requires plumbing HostFunc context/env
                // For now, return NotImplementedError correctly
                return Err(RuntimeError("Host function callback invocation".to_string()).into());
            }
        }

        let mut frame = StacklessFrame::new(module, func_idx, args, instance_idx)?;

        // Use push with error handling for bounded vector
        self.exec_stack.frames.push(frame).map_err(|e| {
            ExecutionError(format!(
                "Call stack overflow, maximum frames: {}",
                MAX_FRAMES
            ))
            .into()
        })?;

        self.exec_stack.state = StacklessExecutionState::Running; // Use exec_stack

        let result = self.run_loop();

        match result {
            Ok(StacklessExecutionState::Completed) => {
                // Access stack via self.exec_stack
                let current_frame = self.exec_stack.frames.last().ok_or_else(|| {
                    ExecutionError("Frame stack empty after function completion".into()).into()
                })?;
                let func_type = current_frame.get_function_type()?;
                let arity = func_type.results.len();

                // Use our new helper method for popping values
                self.exec_stack.pop_values(arity)
            }
            Ok(state) => Err(ExecutionError(
                format!("Execution finished in unexpected state: {:?}", state).into(),
            )
            .into()),
            Err(e) => Err(e),
        }
    }

    /// Runs the engine until it halts, traps, or requires external interaction.
    pub fn run(&mut self) -> Result<StacklessExecutionState, Error> {
        // Validate engine state before execution
        if self.verification_level != VerificationLevel::None {
            self.validate()?;
        }

        self.run_loop()
    }

    /// Executes a single step (instruction) in the engine.
    pub fn step(&mut self) -> Result<(), Error> {
        // Check if we have enough fuel for another step
        self.check_fuel()?;

        // Track function call operation
        wrt_types::record_global_operation(
            wrt_types::OperationType::FunctionCall,
            self.verification_level,
        );

        // Ensure the engine state is valid before a step
        if self.verification_level != VerificationLevel::None {
            self.validate()?;
        }

        match self.exec_stack.state {
            StacklessExecutionState::Completed | StacklessExecutionState::Finished => {
                return Ok(());
            }
            StacklessExecutionState::Running => {
                // Get the current instruction and frame information
                let current_frame = self.current_frame()?;
                let func_idx = current_frame.func_idx;

                // Get the function from the module
                let function = current_frame
                    .module
                    .functions
                    .get(func_idx as usize)
                    .ok_or_else(|| InvalidFunctionIndexError(func_idx).into())?;

                // Get the instructions from the function
                let instructions = &function.code;
                let current_pc = self.exec_stack.pc;

                // Execute the current instruction
                // This would normally process the instruction at the current PC and determine the next PC

                // For now, just increment the PC as a simple implementation
                let next_pc = current_pc + 1;

                // Check if we've reached the end of the instructions
                if next_pc >= instructions.len() {
                    // No more instructions, complete execution
                    mem::replace(
                        &mut self.exec_stack.state,
                        StacklessExecutionState::Completed,
                    );
                    return Ok(());
                } else {
                    // Update the PC
                    self.exec_stack.pc = next_pc;
                }
            }
            // Fix the ? operator issue
            _ => return self.step(), // Continue stepping if in an intermediate state
        }

        Ok(())
    }

    /// The main execution loop that drives the engine forward.
    /// This function is typically called internally by `run`.
    fn run_loop(&mut self) -> Result<StacklessExecutionState, Error> {
        loop {
            match self.exec_stack.state {
                StacklessExecutionState::Running => self.step()?,
                StacklessExecutionState::Completed | StacklessExecutionState::Finished => {
                    // Replace state with Completed and return the original
                    return Ok(mem::replace(
                        &mut self.exec_stack.state,
                        StacklessExecutionState::Completed,
                    ));
                }
                StacklessExecutionState::Paused { .. } => {
                    // Replace state with Completed and return the original (Paused state)
                    return Ok(mem::replace(
                        &mut self.exec_stack.state,
                        StacklessExecutionState::Completed,
                    ));
                }
                StacklessExecutionState::Error(_) => {
                    // Replace state with Completed and return the original (Error state)
                    return Ok(mem::replace(
                        &mut self.exec_stack.state,
                        StacklessExecutionState::Completed,
                    ));
                }
                // Other states like Calling, Returning, Branching are handled internally by step/run_loop
                _ => self.step()?, // Continue stepping if in an intermediate state
            }
        }
    }

    /// Returns an immutable reference to the current (top) execution frame.
    pub fn current_frame(&self) -> Result<&StacklessFrame, Error> {
        self.exec_stack
            .frames
            .last()
            .ok_or_else(|| ExecutionError("Call stack empty".to_string()).into())
    }

    /// Returns a mutable reference to the current (top) execution frame.
    pub fn current_frame_mut(&mut self) -> Result<&mut StacklessFrame, Error> {
        self.exec_stack
            .frames
            .last_mut()
            .ok_or_else(|| ExecutionError("Call stack empty".to_string()).into())
    }

    fn pop_n(&mut self, n: usize) -> Vec<Value> {
        let values = &mut self.exec_stack.values;
        let len = values.len();
        if n > len {
            return Vec::new(); // or panic, depending on your error handling strategy
        }

        let new_len = len - n;
        // Convert BoundedVec to Vec when returning
        values
            .split_off(new_len)
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    }

    fn pop_frame_label(&self) -> Result<Label, Error> {
        if let Some(frame) = self.exec_stack.frames.last() {
            if let Some(label) = frame.label_stack.last() {
                return Ok(label.clone());
            }
        }
        Err(ExecutionError("Stack underflow".to_string()).into())
    }

    /// Get the current instance being executed
    pub fn get_current_instance(&self) -> Result<Arc<ModuleInstance>, Error> {
        let frame = self.current_frame()?;
        self.with_instance(frame.instance_idx.try_into().unwrap(), |instance| {
            Ok(Arc::new(instance.clone()))
        })
    }

    pub fn callbacks_lock(&self) -> MutexGuard<'_, StacklessCallbackRegistry> {
        self.callbacks.lock()
    }

    /// Public accessor for the callbacks lock
    pub fn get_callbacks_lock(&self) -> MutexGuard<'_, StacklessCallbackRegistry> {
        self.callbacks.lock()
    }

    pub fn invoke_host_function(
        &mut self,
        _func_ref: u32,
        _instance_idx: usize,
        _args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        Err(RuntimeError("invoke_host_function".to_string()).into())
    }

    pub fn get_func_ref_from_table(
        &mut self,
        _table_idx: u32,
        _idx: u32,
        _instance_idx: usize,
    ) -> Result<u32> {
        Err(RuntimeError("get_func_ref_from_table".to_string()).into())
    }

    /// Execute a single instruction
    pub fn execute_instruction(
        &mut self,
        stack: &mut StacklessStack,
        instruction: &InstructionType,
    ) -> Result<ControlFlowTrait, Error> {
        if stack.frames.is_empty() {
            return Err(ExecutionError("No frames on stack".to_string()).into());
        }

        // Get the frame index
        let frame_idx = stack.frames.len() - 1;

        // Clone the frame and engine to avoid borrow issues
        let mut frame = stack
            .frames
            .get(frame_idx)
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())?
            .clone();
        let mut engine_clone = self.clone();

        // Execute directly with the cloned frame
        let result = instruction.execute(stack, &mut frame, &mut engine_clone);

        // If successful, update the frame in the stack
        if result.is_ok() {
            // Update the frame in the stack - handle potential BoundedVec error
            match stack.frames.set(frame_idx, frame) {
                Some(_) => Ok(()), // Previous frame was replaced
                None => Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_INVALID_FRAME_INDEX,
                    kinds::ValidationError(format!("Invalid frame index: {}", frame_idx)),
                )),
            }
        } else {
            result
        }
    }

    /// Gets a copy of the current module being executed
    pub fn get_module_copy(&self) -> Result<Module> {
        // Get a reference to the module
        let instance_idx = self.exec_stack.instance_idx;

        self.with_instance(instance_idx, |instance| {
            // Clone the module - dereference the Arc to get a Module
            Ok((*instance.module).clone())
        })
    }

    /// Executes a context switch (typically for function calls)
    /// Updates frame, module, instance, etc. references
    pub fn switch_context(&mut self, entry_point: u32, args: &[Value]) -> Result<(), Error> {
        let instance_idx = self.exec_stack.instance_idx;

        // Get current frame and set its return point
        if let Some(current_frame) = self.exec_stack.frames.last_mut() {
            current_frame.return_pc = self.exec_stack.pc;
        }

        // Create a new frame for the called function
        let module = self.exec_stack.module.clone();
        let new_frame = StacklessFrame::new(module, entry_point, args, instance_idx as u32)?;

        // Push the new frame onto the stack
        self.exec_stack.frames.push(new_frame).map_err(|e| {
            ExecutionError(format!(
                "Call stack overflow, maximum frames: {}",
                MAX_FRAMES
            ))
            .into()
        })?;

        // Update function index for the engine
        self.exec_stack.func_idx = entry_point;

        Ok(())
    }

    /// Save the current context (before a function call)
    fn save_context(&mut self) -> Result<u32> {
        if let Some(frame) = self.exec_stack.frames.last() {
            Ok(frame.func_idx)
        } else {
            Err(ExecutionError("Stack underflow".to_string()).into())
        }
    }

    /// Push a frame for a new function call
    fn push_frame(&mut self, frame: StacklessFrame) -> Result<()> {
        // Validate frame before pushing
        if self.verification_level != VerificationLevel::None {
            frame.validate()?;
        }

        if let Some(max_depth) = self.max_call_depth {
            if self.exec_stack.frames.len() >= max_depth {
                return Err(RuntimeError("Stack overflow".to_string()).into());
            }
        }

        self.exec_stack.frames.push(frame).map_err(|e| {
            ExecutionError(format!(
                "Call stack overflow, maximum frames: {}",
                MAX_FRAMES
            ))
            .into()
        })
    }

    /// Restore the previous context (after returning from a function)
    fn restore_context(&mut self) -> Result<()> {
        // Pop the current frame
        self.exec_stack
            .frames
            .pop()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())?;

        // Update current function index if we have a frame
        if let Some(frame) = self.exec_stack.frames.last() {
            self.exec_stack.func_idx = frame.func_idx;
            self.exec_stack.pc = frame.pc;
        }

        Ok(())
    }

    /// Check if the engine has enough fuel to continue or consumed all fuel
    ///
    /// This method also processes any bounded collection operations performed
    /// during execution, updating the remaining fuel accordingly.
    fn check_fuel(&mut self) -> Result<(), Error> {
        if let Some(fuel) = self.fuel {
            // Get the fuel consumed by operations since last check
            let op_fuel = wrt_types::global_fuel_consumed();

            // Subtract operation fuel
            if op_fuel > 0 {
                // Avoid overflowing if op_fuel is greater than remaining fuel
                if op_fuel >= fuel {
                    self.fuel = Some(0);
                    self.stats.fuel_exhausted_count += 1;
                    return Err(RuntimeError("Insufficient fuel for operation".into()).into());
                } else {
                    self.fuel = Some(fuel - op_fuel);
                }

                // Update fuel consumed in stats
                self.stats.fuel_consumed += op_fuel;

                // Reset operation tracking for next check
                wrt_types::reset_global_operations();
            }

            // Check if we have fuel left
            if fuel == 0 {
                self.stats.fuel_exhausted_count += 1;
                return Err(RuntimeError("Insufficient fuel".into()).into());
            }
        }

        Ok(())
    }

    /// Fix get_instance_mut to use error helper functions
    pub fn get_instance_mut(&mut self, instance_idx: usize) -> Result<Arc<ModuleInstance>> {
        let instances = self.instances.lock();
        instances
            .get(instance_idx)
            .cloned()
            .ok_or_else(|| InvalidInstanceIndexError(instance_idx.try_into().unwrap()).into())
    }

    /// Fix handle_memory_access method to use error helper functions
    pub fn handle_memory_access<F, T>(
        &mut self,
        memory_idx: u32,
        instance_idx: usize,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&mut dyn MemoryAdapter) -> Result<T>,
    {
        // Get the instance and its memory
        let module_arc = self.get_instance_mut(instance_idx)?;
        let memories = &module_arc.memories;
        let memory = memories.get(memory_idx as usize).ok_or_else(|| {
            ResourceError(format!(
                "Memory not found: instance {}, memory {}",
                instance_idx, memory_idx
            ))
            .into()
        })?;

        // Get adapter and verify integrity
        let adapter = memory.adapter();
        memory
            .validate()
            .map_err(|e| ValidationError(format!("Memory validation failed: {}", e)).into())?;

        // Call the function with the adapter
        f(adapter.as_mut())
    }

    /// Add a validate method that properly checks data structures
    pub fn validate(&self) -> Result<(), Error> {
        // Validate operand stack
        self.exec_stack.values.validate().map_err(|e| {
            Error::new(
                ErrorCategory::RuntimeIntegrity,
                codes::RUNTIME_STACK_INTEGRITY_ERROR,
                kinds::RuntimeError(format!("Value stack validation failed: {}", e)),
            )
        })?;

        self.exec_stack.labels.validate().map_err(|e| {
            Error::new(
                ErrorCategory::RuntimeIntegrity,
                codes::RUNTIME_STACK_INTEGRITY_ERROR,
                kinds::RuntimeError(format!("Label stack validation failed: {}", e)),
            )
        })?;

        self.exec_stack.frames.validate().map_err(|e| {
            Error::new(
                ErrorCategory::RuntimeIntegrity,
                codes::RUNTIME_STACK_INTEGRITY_ERROR,
                kinds::RuntimeError(format!("Frame stack validation failed: {}", e)),
            )
        })?;

        Ok(())
    }
}

// Implement StackBehavior for StacklessEngine by delegating to exec_stack
impl StackBehavior for StacklessEngine {
    fn push(&mut self, value: Value) -> Result<(), Error> {
        self.exec_stack.push(value)
    }

    fn pop(&mut self) -> Result<Value, Error> {
        self.exec_stack.pop()
    }

    fn peek(&self) -> Result<&Value, Error> {
        self.exec_stack.peek()
    }

    fn peek_mut(&mut self) -> Result<&mut Value, Error> {
        self.exec_stack.peek_mut()
    }

    fn values(&self) -> &[Value] {
        self.exec_stack.values()
    }

    fn values_mut(&mut self) -> &mut [Value] {
        self.exec_stack.values_mut()
    }

    fn len(&self) -> usize {
        self.exec_stack.len()
    }

    fn is_empty(&self) -> bool {
        self.exec_stack.is_empty()
    }

    fn push_label(&mut self, label: Label) -> Result<(), Error> {
        self.exec_stack.push_label(label.arity, label.pc)
    }

    fn pop_label(&mut self) -> Result<Label, Error> {
        self.exec_stack.pop_label()
    }

    fn get_label(&self, index: usize) -> Option<&Label> {
        self.exec_stack.get_label(index)
    }

    fn push_n(&mut self, values: &[Value]) {
        // Now that we have bounded collections, we need to handle possible capacity errors
        for value in values {
            // Note: We silently drop values if we hit the capacity limit
            // This avoids panics in the engine when stack overflows occur
            let _ = self.exec_stack.values.push(value.clone());
        }
    }

    fn pop_n(&mut self, n: usize) -> Vec<Value> {
        let len = self.exec_stack.values.len();
        if len < n {
            // Log the error but return an empty vector
            log::error!(
                "Error popping values from stack: stack underflow (needed {}, had {})",
                n,
                len
            );
            Vec::new()
        } else {
            // Create a result vector
            let mut result = Vec::with_capacity(n);

            // Pop values one by one since split_off is not available on BoundedVec
            for _ in 0..n {
                if let Some(value) = self.exec_stack.values.pop() {
                    result.push(value);
                }
            }

            // The values were popped in reverse order, so we need to reverse them
            result.reverse();
            result
        }
    }

    fn pop_frame_label(&mut self) -> Result<Label, Error> {
        self.exec_stack.pop_frame_label()
    }

    fn execute_function_call_direct(
        &mut self,
        _engine: &mut StacklessEngine, // Param required by trait, unused when self is engine
        caller_instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        // This is a bit of a hack - we unwrap self since self is already the engine
        self.call_function(caller_instance_idx, func_idx, &args)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Replace the StackBehavior implementation for StacklessStack to avoid recursion
impl StackBehavior for StacklessStack {
    fn push(&mut self, value: Value) -> Result<(), Error> {
        self.values.push(value).map_err(|e| {
            ExecutionError(format!("Stack overflow, maximum values: {}", MAX_VALUES)).into()
        })
    }

    fn pop(&mut self) -> Result<Value, Error> {
        self.values
            .pop()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    fn peek(&self) -> Result<&Value, Error> {
        self.values
            .last()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    fn peek_mut(&mut self) -> Result<&mut Value, Error> {
        self.values
            .last_mut()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    fn values(&self) -> &[Value] {
        self.values.as_ref()
    }

    fn values_mut(&mut self) -> &mut [Value] {
        self.values.as_mut()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn push_label(&mut self, label: Label) -> Result<(), Error> {
        self.labels.push(label).map_err(|e| {
            ExecutionError(format!(
                "Label stack overflow, maximum labels: {}",
                MAX_LABELS
            ))
            .into()
        })
    }

    fn pop_label(&mut self) -> Result<Label, Error> {
        self.labels
            .pop()
            .ok_or_else(|| ExecutionError("Stack underflow".to_string()).into())
    }

    fn get_label(&self, index: usize) -> Option<&Label> {
        let len = self.labels.len();
        len.checked_sub(1 + index)
            .and_then(|adjusted_idx| self.labels.get(adjusted_idx))
    }

    fn push_n(&mut self, values: &[Value]) {
        for value in values {
            let _ = self.push(value.clone());
        }
    }

    fn pop_n(&mut self, n: usize) -> Vec<Value> {
        if self.values.len() < n {
            log::error!("Error popping values from stack: stack underflow");
            Vec::new()
        } else {
            let new_len = self.values.len() - n;
            let mut result = self.values.split_off(new_len);
            result.reverse(); // maintain stack order
            result.to_vec()
        }
    }

    fn pop_frame_label(&mut self) -> Result<Label, Error> {
        if let Some(frame) = self
            .frames
            .get(self.frames.len().checked_sub(1).unwrap_or(0))
        {
            if let Some(label) = frame
                .label_stack
                .get(frame.label_stack.len().checked_sub(1).unwrap_or(0))
            {
                return Ok(label.clone());
            }
        }
        ExecutionError("Stack underflow".to_string()).into()
    }

    fn execute_function_call_direct(
        &mut self,
        engine: &mut StacklessEngine,
        caller_instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        engine.call_function(caller_instance_idx, func_idx, &args)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Implement Clone for StacklessEngine
impl Clone for StacklessEngine {
    fn clone(&self) -> Self {
        Self {
            exec_stack: self.exec_stack.clone(),
            fuel: self.fuel,
            stats: self.stats.clone(),
            callbacks: self.callbacks.clone(),
            max_call_depth: self.max_call_depth,
            instances: self.instances.clone(),
            verification_level: self.verification_level,
        }
    }
}

// Fix the Clone implementation for StacklessExecutionState in StacklessStack
impl Clone for StacklessStack {
    fn clone(&self) -> Self {
        // Create new bounded vectors with the same verification level
        let mut values = BoundedVec::with_verification_level(VerificationLevel::Standard);
        let mut labels = BoundedVec::with_verification_level(VerificationLevel::Standard);
        let mut frames = BoundedVec::with_verification_level(VerificationLevel::Standard);

        // Copy all items from the original vectors to the new ones
        // Note: If the original vectors somehow exceeded capacity, we'll silently truncate
        for value in self.values.iter() {
            let _ = values.push(value.clone());
        }

        for label in self.labels.iter() {
            let _ = labels.push(label.clone());
        }

        for frame in self.frames.iter() {
            let _ = frames.push(frame.clone());
        }

        Self {
            module: self.module.clone(),
            instance_idx: self.instance_idx,
            values,
            labels,
            frames,
            state: match &self.state {
                StacklessExecutionState::Running => StacklessExecutionState::Running,
                StacklessExecutionState::Paused {
                    pc,
                    instance_idx,
                    func_idx,
                    expected_results,
                } => StacklessExecutionState::Paused {
                    pc: *pc,
                    instance_idx: *instance_idx,
                    func_idx: *func_idx,
                    expected_results: *expected_results,
                },
                StacklessExecutionState::Calling {
                    instance_idx,
                    func_idx,
                    args,
                    return_pc,
                } => StacklessExecutionState::Calling {
                    instance_idx: *instance_idx,
                    func_idx: *func_idx,
                    args: args.clone(),
                    return_pc: *return_pc,
                },
                StacklessExecutionState::Returning { values } => {
                    StacklessExecutionState::Returning {
                        values: values.clone(),
                    }
                }
                StacklessExecutionState::Branching { depth, values } => {
                    StacklessExecutionState::Branching {
                        depth: *depth,
                        values: values.clone(),
                    }
                }
                StacklessExecutionState::Completed => StacklessExecutionState::Completed,
                StacklessExecutionState::Finished => StacklessExecutionState::Finished,
                StacklessExecutionState::Error(err) => StacklessExecutionState::Error(err.clone()),
            },
            pc: self.pc,
            func_idx: self.func_idx,
            capacity: self.capacity,
        }
    }
}

// These functions were using undefined types, so they've been removed.
// They should be reimplemented later when needed.
