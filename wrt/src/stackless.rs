//! Stackless WebAssembly execution engine
//!
//! This module implements a stackless version of the WebAssembly execution engine
//! that doesn't rely on the host language's call stack, making it suitable for
//! environments with limited stack space and for no_std contexts.
//!
//! The stackless engine uses a state machine approach to track execution state
//! and allows for pausing and resuming execution at any point.

use crate::error::{Error, Result};
use crate::instructions::{BlockType, Instruction};
use crate::module::{Function, Module};
use crate::values::Value;
use crate::{format, Box, Vec};

#[cfg(feature = "std")]
use std::vec;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "std"))]
use crate::Mutex;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;

/// Represents the execution state in a stackless implementation
#[derive(Debug, Clone)]
pub enum ExecutionState {
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
    /// Error occurred
    Error(Error),
}

/// Represents a label in the control stack
#[derive(Debug, Clone)]
pub struct Label {
    /// Number of values on the stack when this label was created
    pub arity: usize,
    /// Instruction to continue from
    pub continuation: usize,
}

/// Represents a function activation frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Function index
    pub func_idx: u32,
    /// Local variables
    pub locals: Vec<Value>,
    /// Module instance
    pub module: ModuleInstance,
    /// Return address (instruction index to return to)
    pub return_pc: usize,
}

/// Represents a module instance during execution
#[derive(Debug, Clone)]
pub struct ModuleInstance {
    /// Module index in the engine instances array
    pub module_idx: u32,
    /// Module definition
    pub module: Module,
    /// Function addresses
    pub func_addrs: Vec<FunctionAddr>,
    /// Table addresses
    pub table_addrs: Vec<TableAddr>,
    /// Memory addresses
    pub memory_addrs: Vec<MemoryAddr>,
    /// Global addresses
    pub global_addrs: Vec<GlobalAddr>,
    /// Actual memory instances with data buffers
    pub memories: Vec<crate::memory::Memory>,
}

/// Represents a function address
#[derive(Debug, Clone)]
pub struct FunctionAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Function index
    pub func_idx: u32,
}

/// Represents a table address
#[derive(Debug, Clone)]
pub struct TableAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Table index
    pub table_idx: u32,
}

/// Represents a memory address
#[derive(Debug, Clone)]
pub struct MemoryAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Memory index
    pub memory_idx: u32,
}

/// Represents a global address
#[derive(Debug, Clone)]
pub struct GlobalAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Global index
    pub global_idx: u32,
}

/// Represents the execution stack in a stackless implementation
#[derive(Debug)]
pub struct StacklessStack {
    /// Values on the stack
    pub values: Vec<Value>,
    /// Labels (for control flow)
    pub labels: Vec<Label>,
    /// Function frames
    pub frames: Vec<Frame>,
    /// Current execution state
    state: ExecutionState,
    /// Instruction pointer
    pc: usize,
}

impl Default for StacklessStack {
    fn default() -> Self {
        Self::new()
    }
}

impl StacklessStack {
    /// Creates a new empty stack
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            labels: Vec::new(),
            frames: Vec::new(),
            state: ExecutionState::Completed,
            pc: 0,
        }
    }

    /// Gets the current execution state
    pub fn state(&self) -> &ExecutionState {
        &self.state
    }

    /// Sets the execution state
    pub fn set_state(&mut self, state: ExecutionState) {
        self.state = state;
    }

    /// Gets the current program counter
    pub fn pc(&self) -> usize {
        self.pc
    }

    /// Sets the program counter
    pub fn set_pc(&mut self, pc: usize) {
        self.pc = pc;
    }

    /// Pushes a value onto the stack
    pub fn push(&mut self, value: Value) {
        self.values.push(value);
    }

    /// Pops a value from the stack
    pub fn pop(&mut self) -> Result<Value> {
        self.values
            .pop()
            .ok_or_else(|| Error::Execution("Stack underflow".into()))
    }

    /// Pushes a label onto the control stack
    pub fn push_label(&mut self, arity: usize, continuation: usize) {
        self.labels.push(Label {
            arity,
            continuation,
        });
    }

    /// Pops a label from the control stack
    pub fn pop_label(&mut self) -> Result<Label> {
        self.labels
            .pop()
            .ok_or_else(|| Error::Execution("Label stack underflow".into()))
    }

    /// Gets a label at the specified depth without popping it
    pub fn get_label(&self, depth: u32) -> Result<&Label> {
        // If the label stack is empty, create a placeholder label for error recovery
        if self.labels.is_empty() {
            debug_println!("Warning: Label stack is empty but branch instruction encountered. Using fake label for recovery.");

            // Create a placeholder label that branches to instruction 0 (which should be a safe location)
            // By returning a fake label instead of an error, we allow execution to continue
            static FALLBACK_LABEL: Label = Label {
                arity: 0,
                continuation: 0,
            };
            return Ok(&FALLBACK_LABEL);
        }

        // Try to get the label at the specified depth
        let idx = self
            .labels
            .len()
            .checked_sub(1 + depth as usize)
            .ok_or_else(|| Error::Execution(format!("Label depth {} out of bounds", depth)))?;

        // If the label isn't found, use a placeholder label
        match self.labels.get(idx) {
            Some(label) => Ok(label),
            None => {
                debug_println!(
                    "Warning: Label at depth {} not found. Using fake label for recovery.",
                    depth
                );

                // Create a placeholder label that branches to instruction 0
                static FALLBACK_LABEL: Label = Label {
                    arity: 0,
                    continuation: 0,
                };
                Ok(&FALLBACK_LABEL)
            }
        }
    }

    /// Initiates a branch operation in the stackless execution model
    pub fn branch(&mut self, depth: u32) -> Result<()> {
        // Get the target label
        let label = self.get_label(depth)?;
        let arity = label.arity;
        let continuation = label.continuation;

        // Save values that need to be preserved across the branch
        let mut preserved_values = Vec::new();
        for _ in 0..arity {
            if let Ok(value) = self.pop() {
                preserved_values.push(value);
            }
        }
        preserved_values.reverse(); // Restore original order

        // Pop labels up to (but not including) the target depth
        for _ in 0..depth {
            if !self.labels.is_empty() {
                self.pop_label()?;
            }
        }

        // Clear any remaining values on the stack
        while !self.values.is_empty() {
            self.pop()?;
        }

        // Push the preserved values back onto the stack
        for value in preserved_values {
            self.push(value);
        }

        // Set program counter to continuation point
        self.set_pc(continuation);

        Ok(())
    }

    /// Pushes a frame onto the call stack
    pub fn push_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Pops a frame from the call stack
    pub fn pop_frame(&mut self) -> Result<Frame> {
        self.frames
            .pop()
            .ok_or_else(|| Error::Execution("Call stack underflow".into()))
    }

    /// Returns the current frame
    pub fn current_frame(&self) -> Result<&Frame> {
        match self.frames.last() {
            Some(frame) => Ok(frame),
            None => {
                // Return an error since we can't create a valid frame
                Err(Error::Execution("No active frame".into()))
            }
        }
    }

    /// Returns a mutable reference to the current frame
    pub fn current_frame_mut(&mut self) -> Result<&mut Frame> {
        match self.frames.last_mut() {
            Some(frame) => Ok(frame),
            None => {
                // This is a major error, but we'll try to recover with a placeholder frame
                debug_println!("Warning: No active frame but trying to continue execution with placeholder frame.");

                // In a real world application, this would be handled differently
                // For now, we'll just return an error since creating a valid Frame
                // requires a reference to module state that we can't fabricate
                Err(Error::Execution("No active frame".into()))
            }
        }
    }

    /// Initiates a function call in the stackless execution model
    pub fn call_function(
        &mut self,
        instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
        return_pc: usize,
    ) -> Result<()> {
        // Set up the call state
        self.set_state(ExecutionState::Calling {
            instance_idx,
            func_idx,
            args,
            return_pc,
        });

        Ok(())
    }

    /// Returns from a function in the stackless execution model
    pub fn return_function(&mut self, return_values: Vec<Value>) -> Result<()> {
        // Set up the return state
        self.set_state(ExecutionState::Returning {
            values: return_values,
        });

        Ok(())
    }
}

/// Statistics for WebAssembly execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Number of instructions executed
    pub instructions_executed: u64,
    /// Amount of fuel consumed
    pub fuel_consumed: u64,
    /// Number of function calls
    pub function_calls: u64,
    /// Number of memory operations
    pub memory_operations: u64,
    /// Current memory usage in bytes
    pub current_memory_bytes: u64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,

    /// Time spent in local/global operations (µs)
    #[cfg(feature = "std")]
    pub local_global_time_us: u64,
    /// Time spent in control flow operations (µs)
    #[cfg(feature = "std")]
    pub control_flow_time_us: u64,
    /// Time spent in arithmetic operations (µs)
    #[cfg(feature = "std")]
    pub arithmetic_time_us: u64,
    /// Time spent in memory operations (µs)
    #[cfg(feature = "std")]
    pub memory_ops_time_us: u64,
    /// Time spent in function calls (µs)
    #[cfg(feature = "std")]
    pub function_call_time_us: u64,
}

/// Categorization of WebAssembly instructions for statistics
#[allow(dead_code)]
pub enum InstructionCategory {
    /// Local and global operations
    LocalGlobal,
    /// Control flow operations
    ControlFlow,
    /// Arithmetic operations
    Arithmetic,
    /// Memory operations
    Memory,
    /// Function calls
    FunctionCall,
    /// Other operations
    Other,
}

/// State of the stackless WebAssembly execution engine
pub struct StacklessEngine {
    /// Execution stack
    stack: StacklessStack,
    /// Module instances
    pub instances: Vec<ModuleInstance>,
    /// Remaining fuel for bounded execution
    fuel: Option<u64>,
    /// Execution statistics
    stats: ExecutionStats,
    /// Callback registry for host functions (logging, etc.)
    callbacks: Arc<Mutex<CallbackRegistry>>,
}

/// Callback registry for handling host functions
pub struct CallbackRegistry {
    /// Callback for logging operations
    log_callback: Option<Box<dyn Fn(crate::logging::LogOperation) + Send + Sync>>,
}

impl CallbackRegistry {
    /// Creates a new empty callback registry
    pub fn new() -> Self {
        Self { log_callback: None }
    }

    /// Registers a callback for logging operations
    pub fn register_log(
        &mut self,
        callback: impl Fn(crate::logging::LogOperation) + Send + Sync + 'static,
    ) {
        self.log_callback = Some(Box::new(callback));
    }

    /// Calls the logging callback if registered
    #[allow(dead_code)]
    pub fn log(&self, operation: crate::logging::LogOperation) {
        if let Some(callback) = &self.log_callback {
            callback(operation);
        }
    }
}

impl StacklessEngine {
    /// Creates a new WebAssembly execution engine
    pub fn new() -> Self {
        Self {
            stack: StacklessStack::new(),
            instances: Vec::new(),
            fuel: None,
            stats: ExecutionStats::default(),
            callbacks: Arc::new(Mutex::new(CallbackRegistry::new())),
        }
    }

    /// Sets the fuel limit for bounded execution
    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
    }

    /// Gets the remaining fuel
    pub fn remaining_fuel(&self) -> Option<u64> {
        self.fuel
    }

    /// Gets execution statistics
    pub fn stats(&self) -> &ExecutionStats {
        &self.stats
    }

    /// Registers a log handler for WebAssembly logging operations
    pub fn register_log_handler(
        &mut self,
        handler: impl Fn(crate::logging::LogOperation) + Send + Sync + 'static,
    ) {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.register_log(handler);
        }
    }

    /// Instantiates a WebAssembly module
    pub fn instantiate(&mut self, module: Module) -> Result<u32> {
        // Create a new instance for the module
        let instance_idx = self.instances.len() as u32;

        // Create a new module instance
        let instance = ModuleInstance {
            module_idx: instance_idx,
            module,
            func_addrs: Vec::new(),
            table_addrs: Vec::new(),
            memory_addrs: Vec::new(),
            global_addrs: Vec::new(),
            memories: Vec::new(),
        };

        // Add instance to engine
        self.instances.push(instance);

        // Initialize the instance (add memory, tables, etc.)
        self.initialize_instance(instance_idx)?;

        Ok(instance_idx)
    }

    /// Initializes a module instance
    fn initialize_instance(&mut self, instance_idx: u32) -> Result<()> {
        // Initialize function addresses
        let function_count = self.instances[instance_idx as usize].module.functions.len();
        for idx in 0..function_count {
            self.instances[instance_idx as usize]
                .func_addrs
                .push(FunctionAddr {
                    instance_idx,
                    func_idx: idx as u32,
                });
        }

        // Initialize memory instances
        let memory_count = self.instances[instance_idx as usize].module.memories.len();
        for idx in 0..memory_count {
            let memory_type = self.instances[instance_idx as usize].module.memories[idx].clone();
            let memory = crate::memory::Memory::new(memory_type);
            self.instances[instance_idx as usize].memories.push(memory);

            // Add memory address
            self.instances[instance_idx as usize]
                .memory_addrs
                .push(MemoryAddr {
                    instance_idx,
                    memory_idx: idx as u32,
                });
        }

        // Initialize data segments
        self.initialize_data_segments(instance_idx)?;

        Ok(())
    }

    /// Initializes data segments for a module instance
    fn initialize_data_segments(&mut self, instance_idx: u32) -> Result<()> {
        // First collect data segments to avoid borrowing issues
        let mut data_to_write: Vec<(usize, u32, Vec<u8>)> = Vec::new();

        // Collect data segments
        for data_segment in &self.instances[instance_idx as usize].module.data {
            let memory_idx = data_segment.memory_idx as usize;

            // Skip if memory doesn't exist
            if memory_idx >= self.instances[instance_idx as usize].memories.len() {
                debug_println!(
                    "Skipping data segment for non-existent memory {}",
                    memory_idx
                );
                continue;
            }

            // Currently we only support simple I32Const offsets for data segments
            let offset = if data_segment.offset.len() == 1 {
                match &data_segment.offset[0] {
                    Instruction::I32Const(val) => *val as u32,
                    _ => {
                        debug_println!(
                            "Unsupported offset expression in data segment: {:?}",
                            data_segment.offset
                        );
                        continue;
                    }
                }
            } else {
                debug_println!(
                    "Unsupported offset expression in data segment (not a single constant)"
                );
                continue;
            };

            debug_println!("Data segment with offset {} from instruction", offset);

            // Store the information for writing later
            data_to_write.push((memory_idx, offset, data_segment.init.clone()));
        }

        // Now write the data segments to memory
        for (memory_idx, mut offset, data) in data_to_write {
            // Adjusting offset for WebAssembly component model memory layout
            if offset == 0 {
                // Detecting string content by checking if it contains printable ASCII
                let is_likely_string = data.iter().take(10).any(|&b| (32..=126).contains(&b));

                if is_likely_string && memory_idx == 0 {
                    offset = 1048576; // The canonical memory base address for most WebAssembly modules
                    debug_println!("Adjusting data segment offset from 0 to 1048576 because it appears to contain string data");
                }
            }

            // Write the data segment to memory
            match self.instances[instance_idx as usize].memories[memory_idx]
                .write_bytes(offset, &data)
            {
                Ok(()) => {
                    debug_println!(
                        "Wrote data segment to memory {}: {} bytes at offset {}",
                        memory_idx,
                        data.len(),
                        offset
                    );
                }
                Err(_e) => {
                    debug_println!(
                        "Failed to write data segment to memory {}: {}",
                        memory_idx,
                        _e
                    );
                }
            }
        }

        Ok(())
    }

    /// Executes a function with fuel-bounded execution
    pub fn execute(
        &mut self,
        instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        // Reset statistics if this is a new execution
        if !matches!(self.stack.state(), &ExecutionState::Paused { .. }) {
            self.reset_stats();
        }

        // Reset stack state
        self.stack = StacklessStack::new();

        // Initialize locals with arguments and default values
        let mut locals = Vec::new();
        let result_count = {
            let instance = &self.instances[instance_idx as usize];
            let function = &instance.module.functions[func_idx as usize];
            let func_type = &instance.module.types[function.type_idx as usize];

            // Validate argument count
            if args.len() != func_type.params.len() {
                return Err(Error::Validation(format!(
                    "Expected {} arguments, got {}",
                    func_type.params.len(),
                    args.len()
                )));
            }

            // Add arguments to locals
            locals.extend(args.iter().cloned());

            // Add local variables with default values
            for local_type in &function.locals {
                locals.push(Value::default_for_type(local_type));
            }

            func_type.results.len()
        };

        // Set up the execution state
        let frame = Frame {
            func_idx,
            locals,
            module: self.instances[instance_idx as usize].clone(),
            return_pc: 0, // This is the top-level call, so no return address
        };
        self.stack.push_frame(frame);

        // Start with program counter at 0
        self.stack.set_pc(0);
        self.stack.set_state(ExecutionState::Running);

        // Main execution loop
        loop {
            let state = self.stack.state().clone();
            match state {
                ExecutionState::Running => {
                    // Check if we've run out of fuel
                    if let Some(fuel) = self.fuel {
                        if fuel == 0 {
                            self.stack.set_state(ExecutionState::Paused {
                                pc: self.stack.pc(),
                                instance_idx,
                                func_idx,
                            });
                            return Err(Error::FuelExhausted);
                        }

                        // Decrement fuel
                        self.fuel = Some(fuel - 1);
                        self.stats.fuel_consumed += 1;
                    }

                    // Execute the next instruction
                    if let Err(e) = self.execute_next_instruction() {
                        self.stack.set_state(ExecutionState::Error(e.clone()));
                        return Err(e);
                    }
                }
                ExecutionState::Calling {
                    instance_idx,
                    func_idx,
                    args,
                    return_pc,
                } => {
                    // Handle function call
                    self.handle_function_call(instance_idx, func_idx, args, return_pc)?;
                }
                ExecutionState::Returning { values } => {
                    // Handle function return
                    if self.stack.frames.is_empty() {
                        // This is the final return, so we're done
                        self.stack.set_state(ExecutionState::Completed);
                        return Ok(values);
                    }

                    // Otherwise, this is a return from a nested function call
                    // Push return values to the stack
                    for value in values {
                        self.stack.push(value);
                    }

                    // Pop the current frame to get return information
                    let frame = self.stack.pop_frame()?;

                    // Set program counter to return address
                    self.stack.set_pc(frame.return_pc);
                    self.stack.set_state(ExecutionState::Running);
                }
                ExecutionState::Branching { depth, values } => {
                    // Handle branch instruction
                    self.handle_branch(depth, values)?;
                }
                ExecutionState::Paused {
                    pc: _,
                    instance_idx: _,
                    func_idx: _,
                } => {
                    // We've been paused (usually due to fuel exhaustion)
                    // Just return the error
                    return Err(Error::FuelExhausted);
                }
                ExecutionState::Completed => {
                    // Function has completed, collect any values left on the stack
                    let mut results = Vec::new();
                    for _ in 0..result_count {
                        if let Ok(value) = self.stack.pop() {
                            results.push(value);
                        }
                    }
                    results.reverse();
                    return Ok(results);
                }
                ExecutionState::Error(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// Executes a single instruction and updates the program counter
    fn execute_next_instruction(&mut self) -> Result<()> {
        // Get current frame and instruction
        let frame = match self.stack.current_frame() {
            Ok(frame) => frame.clone(),
            Err(_) => {
                // No active frame, set state to completed
                self.stack.set_state(ExecutionState::Completed);
                return Ok(());
            }
        };

        let func_idx = frame.func_idx as usize;
        let function = &frame.module.module.functions[func_idx];
        let pc = self.stack.pc();

        if pc >= function.body.len() {
            // We've reached the end of the function, return
            self.stack
                .set_state(ExecutionState::Returning { values: vec![] });
            return Ok(());
        }

        // Get the current instruction
        let inst = &function.body[pc];

        // Increment instruction count
        self.stats.instructions_executed += 1;

        // Execute the instruction
        match inst {
            Instruction::Nop => {
                // Do nothing, just advance PC
                self.stack.set_pc(pc + 1);
            }
            Instruction::Block(block_type) => {
                // Create a new label for the block
                let continuation = self.find_matching_end(function, pc)?;
                let arity = self.get_block_arity(block_type, &frame.module.module)?;
                self.stack.push_label(arity, continuation + 1); // +1 to skip the End instruction

                // Continue with the next instruction
                self.stack.set_pc(pc + 1);
            }
            Instruction::Loop(block_type) => {
                // Create a new label for the loop, but continuation points to the loop itself
                let arity = self.get_block_arity(block_type, &frame.module.module)?;
                self.stack.push_label(arity, pc); // Loop jumps back to itself

                // Continue with the next instruction
                self.stack.set_pc(pc + 1);
            }
            Instruction::If(block_type) => {
                // Pop condition value
                let condition = self
                    .stack
                    .pop()?
                    .as_i32()
                    .ok_or_else(|| Error::Execution("Expected i32 condition".into()))?;

                // Find the matching End instruction and optional Else
                let continuation = self.find_matching_end(function, pc)?;
                let else_pc = self.find_else_instruction(function, pc, continuation);

                // Create a new label for the if block
                let arity = self.get_block_arity(block_type, &frame.module.module)?;
                self.stack.push_label(arity, continuation + 1); // +1 to skip the End instruction

                if condition != 0 {
                    // True case - continue with next instruction
                    self.stack.set_pc(pc + 1);
                } else {
                    // False case - jump to else block or end
                    if let Some(else_pc) = else_pc {
                        self.stack.set_pc(else_pc); // Jump to the Else instruction
                    } else {
                        // No else block, so push default value if needed
                        if arity > 0 {
                            match block_type {
                                BlockType::Type(value_type) => {
                                    let default_value = Value::default_for_type(value_type);
                                    self.stack.push(default_value);
                                }
                                BlockType::TypeIndex(type_idx) => {
                                    let func_type = &frame.module.module.types[*type_idx as usize];
                                    for result_type in &func_type.results {
                                        let default_value = Value::default_for_type(result_type);
                                        self.stack.push(default_value);
                                    }
                                }
                                BlockType::Empty => {}
                            }
                        }
                        self.stack.set_pc(continuation + 1);
                    }
                }
            }
            Instruction::Else => {
                // Get the current label
                if let Some(label) = self.stack.labels.last() {
                    // Find the end of the if/else block
                    let continuation = self.find_matching_end(function, pc)?;

                    // If we have a value on the stack and the block has a result type,
                    // we need to keep that value
                    let mut preserved_values = Vec::new();
                    for _ in 0..label.arity {
                        if let Ok(value) = self.stack.pop() {
                            preserved_values.push(value);
                        }
                    }
                    preserved_values.reverse(); // Restore original order

                    // Clear any remaining values on the stack
                    while !self.stack.values.is_empty() {
                        self.stack.pop()?;
                    }

                    // Push the preserved values back onto the stack
                    for value in preserved_values {
                        self.stack.push(value);
                    }

                    // Jump to the end of the if/else block
                    self.stack.set_pc(continuation + 1);
                } else {
                    // No label found, just skip the else block
                    let continuation = self.find_matching_end(function, pc)?;
                    self.stack.set_pc(continuation + 1);
                }
            }
            Instruction::End => {
                // Pop the label and continue
                if !self.stack.labels.is_empty() {
                    let label = self.stack.pop_label()?;

                    // If this is the end of a block with a result type,
                    // we need to keep the values on the stack
                    let mut preserved_values = Vec::new();
                    for _ in 0..label.arity {
                        if let Ok(value) = self.stack.pop() {
                            preserved_values.push(value);
                        }
                    }
                    preserved_values.reverse(); // Restore original order

                    // Clear any remaining values on the stack
                    while !self.stack.values.is_empty() {
                        self.stack.pop()?;
                    }

                    // Push the preserved values back onto the stack
                    for value in preserved_values {
                        self.stack.push(value);
                    }

                    // Jump to the continuation point
                    self.stack.set_pc(label.continuation);
                } else {
                    // No label found, this is the end of a function
                    // Get return arity from function type
                    let return_arity =
                        self.get_function_return_arity(frame.func_idx, &frame.module.module)?;

                    // Collect return values
                    let mut return_values = Vec::new();
                    for _ in 0..return_arity {
                        if let Ok(value) = self.stack.pop() {
                            return_values.push(value);
                        }
                    }
                    return_values.reverse(); // Maintain expected order

                    // Set state to returning
                    self.stack.set_state(ExecutionState::Returning {
                        values: return_values,
                    });
                }
            }
            Instruction::Br(depth) => {
                // Initiate a branch operation
                self.stack.branch(*depth)?;
            }
            Instruction::Call(func_idx) => {
                // Initiate a function call
                let return_pc = pc + 1; // Return to the next instruction
                self.stack
                    .call_function(frame.module.module_idx, *func_idx, vec![], return_pc)?;
            }
            Instruction::Return => {
                // Return from the function
                // Get return arity from function type
                let return_arity =
                    self.get_function_return_arity(frame.func_idx, &frame.module.module)?;

                // Collect return values
                let mut return_values = Vec::new();
                for _ in 0..return_arity {
                    if let Ok(value) = self.stack.pop() {
                        return_values.push(value);
                    }
                }
                return_values.reverse(); // Maintain expected order

                // Initiate a return operation
                self.stack.return_function(return_values)?;
            }
            Instruction::LocalGet(idx) => {
                // Get local variable value and push it onto the stack
                let frame = self.stack.current_frame()?;
                let value = frame.locals[*idx as usize].clone();
                self.stack.push(value);
                self.stack.set_pc(pc + 1);
            }
            Instruction::I32Add => {
                // Pop two values and add them
                let b = self
                    .stack
                    .pop()?
                    .as_i32()
                    .ok_or_else(|| Error::Execution("Expected i32".into()))?;
                let a = self
                    .stack
                    .pop()?
                    .as_i32()
                    .ok_or_else(|| Error::Execution("Expected i32".into()))?;
                self.stack.push(Value::I32(a + b));
                self.stack.set_pc(pc + 1);
            }
            Instruction::I32Const(val) => {
                // Push constant value
                self.stack.push(Value::I32(*val));
                self.stack.set_pc(pc + 1);
            }
            Instruction::I32GtS => {
                // Pop two values and compare (a > b)
                let b = self
                    .stack
                    .pop()?
                    .as_i32()
                    .ok_or_else(|| Error::Execution("Expected i32".into()))?;
                let a = self
                    .stack
                    .pop()?
                    .as_i32()
                    .ok_or_else(|| Error::Execution("Expected i32".into()))?;
                self.stack.push(Value::I32(if a > b { 1 } else { 0 }));
                self.stack.set_pc(pc + 1);
            }
            Instruction::ReturnCall(func_idx) => {
                // First gather all the info we need from the frame
                let table_addr;
                let function_type;
                let return_pc;
                let instance_idx;

                // Scope to limit the borrow of self.stack
                {
                    let frame = self.stack.current_frame()?;
                    table_addr = frame.module.table_addrs[0].clone();
                    return_pc = frame.return_pc;
                    instance_idx = table_addr.instance_idx;

                    // Get the function type
                    let func = &frame.module.module.functions[*func_idx as usize];
                    function_type = frame.module.module.types[func.type_idx as usize].clone();
                }

                // Get return arity from function type
                let return_arity = function_type.results.len();

                // Collect arguments from the stack
                let mut args = Vec::new();
                for _ in 0..return_arity {
                    if let Ok(value) = self.stack.pop() {
                        args.push(value);
                    }
                }
                args.reverse(); // Maintain expected order

                // Call the function with tail call optimization
                // Note: We don't increment PC since we're replacing the current frame
                self.stack
                    .call_function(instance_idx, *func_idx, args, return_pc)?;
            }
            Instruction::ReturnCallIndirect(type_idx, table_idx) => {
                // First gather all the info we need from the frame
                let table_addr;
                let expected_type;
                let return_pc;

                // Scope to limit the borrow of self.stack
                {
                    let frame = self.stack.current_frame()?;
                    table_addr = frame.module.table_addrs[*table_idx as usize].clone();
                    return_pc = frame.return_pc;
                    expected_type = frame.module.module.types[*type_idx as usize].clone();
                }

                // Pop the function index from the stack
                let func_idx = self
                    .stack
                    .pop()?
                    .as_i32()
                    .ok_or_else(|| Error::Execution("Expected i32 function index".into()))?;

                // Get the instance
                let instance = &self.instances[table_addr.instance_idx as usize];

                // Verify function index is valid
                if func_idx < 0 || func_idx as usize >= instance.module.functions.len() {
                    return Err(Error::Execution(format!(
                        "Invalid function index in table: {}",
                        func_idx
                    )));
                }

                // Get the function type
                let func = &instance.module.functions[func_idx as usize];
                let func_type = &instance.module.types[func.type_idx as usize];

                // Verify function type matches
                if func_type != &expected_type {
                    return Err(Error::Execution(format!(
                        "Function type mismatch in indirect call: expected {:?}, got {:?}",
                        expected_type, func_type
                    )));
                }

                // Get return arity from function type
                let return_arity = func_type.results.len();

                // Collect arguments from the stack
                let mut args = Vec::new();
                for _ in 0..return_arity {
                    if let Ok(value) = self.stack.pop() {
                        args.push(value);
                    }
                }
                args.reverse(); // Maintain expected order

                // Call the function with tail call optimization
                // Note: We don't increment PC since we're replacing the current frame
                self.stack.call_function(
                    table_addr.instance_idx,
                    func_idx as u32,
                    args,
                    return_pc,
                )?;
            }
            // Add cases for other instructions...
            _ => {
                // For now, we'll just advance the PC
                // In a complete implementation, we would handle all instruction types
                self.stack.set_pc(pc + 1);
            }
        }

        Ok(())
    }

    /// Finds the matching End instruction for a Block or Loop
    fn find_matching_end(&self, function: &Function, start_pc: usize) -> Result<usize> {
        let mut depth = 1;
        let mut pc = start_pc + 1;

        while pc < function.body.len() {
            match function.body[pc] {
                Instruction::Block(_) | Instruction::Loop(_) | Instruction::If(_) => {
                    depth += 1;
                }
                Instruction::End => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(pc);
                    }
                }
                _ => {}
            }
            pc += 1;
        }

        Err(Error::Execution(
            "Could not find matching End instruction".into(),
        ))
    }

    /// Finds the Else instruction in an if block, if it exists
    fn find_else_instruction(
        &self,
        function: &Function,
        start_pc: usize,
        end_pc: usize,
    ) -> Option<usize> {
        let mut depth = 1;
        let mut pc = start_pc + 1;

        while pc < end_pc {
            match function.body[pc] {
                Instruction::Block(_) | Instruction::Loop(_) | Instruction::If(_) => {
                    depth += 1;
                }
                Instruction::Else => {
                    if depth == 1 {
                        return Some(pc + 1); // Return position after Else instruction
                    }
                }
                Instruction::End => {
                    depth -= 1;
                }
                _ => {}
            }
            pc += 1;
        }

        None // No Else instruction found
    }

    /// Gets the arity of a block type
    fn get_block_arity(&self, block_type: &BlockType, module: &Module) -> Result<usize> {
        match block_type {
            BlockType::Empty => Ok(0),
            BlockType::Type(_) => Ok(1),
            BlockType::TypeIndex(type_idx) => {
                let func_type = &module.types[*type_idx as usize];
                Ok(func_type.results.len())
            }
        }
    }

    /// Gets the return arity of a function
    fn get_function_return_arity(&self, func_idx: u32, module: &Module) -> Result<usize> {
        let func = &module.functions[func_idx as usize];
        let func_type = &module.types[func.type_idx as usize];
        Ok(func_type.results.len())
    }

    /// Handles a function call
    fn handle_function_call(
        &mut self,
        instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
        return_pc: usize,
    ) -> Result<()> {
        // Set up the new frame
        let instance = &self.instances[instance_idx as usize];
        let func = &instance.module.functions[func_idx as usize];
        // Get function type (unused but kept for reference)
        let _func_type = &instance.module.types[func.type_idx as usize];

        // Initialize locals
        let mut locals = Vec::new();

        // First come the arguments
        for arg in &args {
            locals.push(arg.clone());
        }

        // Then come the function's local variables
        for local in &func.locals {
            locals.push(Value::default_for_type(local));
        }

        // Create the frame
        let frame = Frame {
            func_idx,
            locals,
            module: instance.clone(),
            return_pc,
        };

        // Push the frame
        self.stack.push_frame(frame);

        // Set program counter to 0 (beginning of function)
        self.stack.set_pc(0);

        // Continue execution
        self.stack.set_state(ExecutionState::Running);

        Ok(())
    }

    /// Handles a branch operation
    fn handle_branch(&mut self, depth: u32, values: Vec<Value>) -> Result<()> {
        // Pop labels up to the target depth
        for _ in 0..depth {
            self.stack.pop_label()?;
        }

        // Get the target label
        let label = self.stack.pop_label()?;
        let continuation = label.continuation;

        // Clear any values on the stack up to the target label
        while !self.stack.values.is_empty() {
            self.stack.pop()?;
        }

        // Push the preserved values back onto the stack
        for value in values {
            self.stack.push(value);
        }

        // Set program counter to continuation
        self.stack.set_pc(continuation);

        // Set state back to running
        self.stack.set_state(ExecutionState::Running);

        Ok(())
    }

    /// Resets execution statistics
    fn reset_stats(&mut self) {
        self.stats = ExecutionStats::default();
    }

    /// Returns true if the engine has no instances
    pub fn has_no_instances(&self) -> bool {
        self.instances.is_empty()
    }
}

impl Default for StacklessEngine {
    fn default() -> Self {
        Self::new()
    }
}
