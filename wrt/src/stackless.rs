//! Stackless WebAssembly execution engine
//!
//! This module implements a stackless version of the WebAssembly execution engine
//! that doesn't rely on the host language's call stack, making it suitable for
//! environments with limited stack space and for no_std contexts.
//!
//! The stackless engine uses a state machine approach to track execution state
//! and allows for pausing and resuming execution at any point.

use crate::{
    behavior::{
        self, ControlFlowBehavior, FrameBehavior, InstructionExecutor, Label, StackBehavior,
    },
    error::{Error, Result},
    execution::ExecutionStats,
    global::Global,
    instructions::Instruction,
    logging::LogOperation,
    memory::{DefaultMemory, MemoryBehavior},
    module::{ExportKind, Function, Module},
    module_instance::ModuleInstance,
    stack::{self, Stack, StacklessStack},
    stackless_frame::StacklessFrame,
    table::Table,
    types::{BlockType, FuncType, ValueType},
    values::Value,
    HostFunctionHandler,
};
use log::trace;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

/// Represents the execution state in a stackless implementation
#[derive(Debug, PartialEq)]
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
    /// Values on the stack
    pub values: Vec<Value>,
    /// Labels (for control flow)
    pub labels: Vec<Label>,
    /// Function frames
    pub frames: Vec<StacklessFrame>,
    /// Current execution state
    pub state: StacklessExecutionState,
    /// Instruction pointer
    pub pc: usize,
    /// Instance index
    pub instance_idx: usize,
    /// Function index
    pub func_idx: u32,
    /// Reference to the module
    pub module: Arc<Module>,
}

/// Registry for callbacks in the stackless implementation
pub struct StacklessCallbackRegistry {
    /// Names of exports that are known to be callbacks
    pub export_names: HashMap<String, HashMap<String, LogOperation>>,
    /// Registered callback functions
    pub callbacks: HashMap<String, HostFunctionHandler>,
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
    /// Execution stack
    pub stack: StacklessStack,
    /// Module instances
    pub instances: Vec<ModuleInstance>,
    /// Remaining fuel for bounded execution
    fuel: Option<u64>,
    /// Execution statistics
    stats: ExecutionStats,
    /// Callback registry for host functions (logging, etc.)
    callbacks: Arc<Mutex<StacklessCallbackRegistry>>,
    /// Maximum call depth for function calls
    max_call_depth: Option<usize>,
}

impl StacklessStack {
    /// Creates a new `StacklessStack` with the given module.
    #[must_use]
    pub const fn new(module: Arc<Module>, instance_idx: usize) -> Self {
        Self {
            values: Vec::new(),
            labels: Vec::new(),
            frames: Vec::new(),
            state: StacklessExecutionState::Running,
            pc: 0,
            instance_idx,
            func_idx: 0,
            module,
        }
    }

    /// Pushes a value onto the stack
    pub fn push(&mut self, value: Value) -> Result<()> {
        self.values.push(value);
        Ok(())
    }

    /// Pops a value from the stack
    pub fn pop(&mut self) -> Result<Value> {
        self.values.pop().ok_or(Error::StackUnderflow)
    }

    /// Pushes a label onto the control stack
    pub fn push_label(&mut self, arity: usize, pc: usize) -> Result<()> {
        self.labels.push(Label {
            arity,
            pc,
            continuation: pc,
        });
        Ok(())
    }

    /// Pops a label from the control stack
    pub fn pop_label(&mut self) -> Result<Label> {
        self.labels.pop().ok_or(Error::StackUnderflow)
    }

    /// Gets a label at the specified depth
    pub fn get_label(&self, idx: usize) -> Result<&Label> {
        self.labels
            .get(idx)
            .ok_or(Error::InvalidCode(format!("Invalid label index: {idx}")))
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
        &self.values
    }

    /// Returns a mutable slice containing all values on the stack.
    pub fn values_mut(&mut self) -> &mut [Value] {
        &mut self.values
    }

    /// Returns a reference to the top value on the stack without removing it.
    pub fn peek(&self) -> Result<&Value> {
        self.values.last().ok_or(Error::StackUnderflow)
    }

    /// Returns a mutable reference to the top value on the stack without removing it.
    pub fn peek_mut(&mut self) -> Result<&mut Value> {
        self.values.last_mut().ok_or(Error::StackUnderflow)
    }

    // Note: Implementations of the `Stack` and `StackBehavior` traits for StacklessStack
    // are added below to maintain compatibility where the engine expects these traits.
}

impl Default for StacklessEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StacklessEngine {
    /// Creates a new stackless WebAssembly engine
    #[must_use]
    pub fn new() -> Self {
        let empty_module = Module::new().expect("Failed to create empty module");
        Self {
            stack: StacklessStack::new(Arc::new(empty_module), 0),
            instances: Vec::new(),
            fuel: None,
            stats: ExecutionStats::default(),
            callbacks: Arc::new(Mutex::new(StacklessCallbackRegistry {
                export_names: HashMap::new(),
                callbacks: HashMap::new(),
            })),
            max_call_depth: None,
        }
    }

    /// Sets the fuel limit for bounded execution
    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
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
    pub const fn state(&self) -> &StacklessExecutionState {
        &self.stack.state
    }

    /// Sets the execution state
    pub fn set_state(&mut self, state: StacklessExecutionState) {
        self.stack.state = state;
    }

    /// Gets the number of module instances
    #[must_use]
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Gets a module instance by index
    pub fn get_instance(&self, instance_idx: usize) -> Result<&ModuleInstance> {
        self.instances
            .get(instance_idx)
            .ok_or(Error::Execution("Invalid instance index".into()))
    }

    /// Adds a module instance
    pub fn add_instance(&mut self, instance: ModuleInstance) -> usize {
        self.instances.push(instance);
        self.instances.len() - 1
    }

    /// Instantiates a module
    pub fn instantiate(&mut self, module: Module) -> Result<usize> {
        println!(
            "DEBUG: instantiate called for module with {} exports",
            module.exports.len()
        );
        let instance = ModuleInstance::new(module)?;
        Ok(self.add_instance(instance))
    }

    /// Checks if the engine currently has any module instances loaded.
    ///
    /// # Returns
    ///
    /// `true` if there are no instances, `false` otherwise.
    pub fn has_no_instances(&self) -> bool {
        self.instances.is_empty()
    }

    /// Registers a callback function for a specific export name.
    ///
    /// This allows host functions to be called from WebAssembly.
    pub fn register_callback(
        &mut self,
        export_name: &str,
        callback: HostFunctionHandler,
    ) -> Result<()> {
        let mut registry = self.callbacks.lock().map_err(|_| Error::PoisonedLock)?;
        if registry.callbacks.contains_key(export_name) {
            return Err(Error::Execution(format!(
                "Callback already registered for export: {export_name}"
            )));
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
    ) -> Result<()> {
        let mut registry = self.callbacks.lock().map_err(|_| Error::PoisonedLock)?;
        registry.export_names.extend(export_names);
        Ok(())
    }

    /// Attempts to get a lock on the callback registry.
    ///
    /// Returns an `Error::PoisonedLock` if the mutex is poisoned.
    fn get_callback_registry_lock(&self) -> Result<MutexGuard<'_, StacklessCallbackRegistry>> {
        self.callbacks.lock().map_err(|_| Error::PoisonedLock)
    }

    /// Finds a callback function by export name.
    ///
    /// Requires a lock on the callback registry.
    fn find_callback_locked(
        registry: &StacklessCallbackRegistry,
        export_name: &str,
    ) -> Option<HostFunctionHandler> {
        registry.callbacks.get(export_name).cloned()
    }

    /// Calls an exported function by name
    pub fn call_export(&mut self, export_name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Find the export in the *last added* instance (convention?)
        // TODO: Allow specifying instance index or handle multiple instances better
        let instance_idx = if self.instances.is_empty() {
            return Err(Error::Execution("No instances loaded".into()));
        } else {
            self.instances.len() - 1
        };
        let instance = &self.instances[instance_idx];

        let export = instance
            .module
            .exports
            .get(export_name)
            .ok_or_else(|| Error::ExportNotFound(export_name.to_string()))?;

        match export.external {
            crate::module::ExportKind::Function => {
                // Found the start function export
                let func_idx = if let crate::module::ExportValue::Function(idx, _) = export.value {
                    idx
                } else {
                    return Err(Error::ExportNotFound(format!(
                        "Export '{export_name}' is not a function"
                    )));
                };
                self.call_function(0, func_idx, args) // Assuming instance_idx 0 if not specified
            }
            _ => {
                return Err(Error::ExportNotFound(format!(
                    "Export '{export_name}' is not a function"
                )))
            }
        }
    }

    /// Calls a function by index within a specific instance
    pub fn call_function(
        &mut self,
        instance_idx: u32,
        func_idx: u32,
        args: &[Value],
    ) -> Result<Vec<Value>> {
        let module = self
            .instances
            .last()
            .ok_or(Error::InvalidInstanceIndex(self.instances.len()))?
            .module
            .clone();

        // Check if this is a host function callback
        // Find the export name associated with this function index
        let export_name = module.exports.iter().find_map(|export| {
            // Use export.kind and match the enum variant directly
            if let crate::module::ExportKind::Function = export.kind {
                // Also check the index associated with the Function kind
                if export.index == func_idx {
                    Some(export.name.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(name) = export_name {
            let registry_lock = self.get_callback_registry_lock()?;
            if let Some(callback) = Self::find_callback_locked(&registry_lock, &name) {
                println!("DEBUG: Calling host callback: {}", name);
                // Drop the lock before calling the callback to avoid deadlocks if the callback tries to use the engine
                drop(registry_lock);
                // Call the host function
                return callback(args);
            }
        }

        // Not a host callback, proceed with normal execution
        // Clone the Arc<Module> obtained from the map and pass it
        let initial_frame =
            StacklessFrame::new(module.clone().into(), func_idx, args, instance_idx)?;
        self.stack.frames.push(initial_frame);
        self.stack.state = StacklessExecutionState::Running;

        let result = self.run_loop();

        // Handle result
        match result {
            Ok(StacklessExecutionState::Completed) => {
                // Pop return values from the value stack
                let current_frame = self.stack.frames.last().ok_or(Error::Execution(
                    "Frame stack empty after function completion".into(),
                ))?;
                let func_type = current_frame.get_function_type()?;
                let arity = func_type.results.len();

                if self.stack.values.len() < arity {
                    return Err(Error::StackUnderflow);
                }
                let results = self.stack.values.split_off(self.stack.values.len() - arity);
                Ok(results)
            }
            Ok(state) => Err(Error::Execution(format!(
                "Execution finished in unexpected state: {state:?}"
            ))),
            Err(e) => Err(e),
        }
    }

    /// The main execution loop
    /// Continues execution until paused, completed, or an error occurs.
    pub fn run(&mut self) -> Result<StacklessExecutionState> {
        while self.stack.state == StacklessExecutionState::Running {
            self.step()?;
        }
        Ok(self.stack.state.clone()) // Return the final state
    }

    /// Executes a single step (instruction) of the engine.
    /// This is the core of the interpreter loop.
    pub fn step(&mut self) -> Result<()> {
        // Check fuel before executing anything
        if let Some(ref mut fuel) = self.fuel {
            if *fuel == 0 {
                // TODO: Save state for pause
                self.stack.state = StacklessExecutionState::Paused {
                    pc: 0,               // Placeholder
                    instance_idx: 0,     // Placeholder
                    func_idx: 0,         // Placeholder
                    expected_results: 0, // Placeholder
                };
                return Ok(());
            }
            *fuel -= 1; // Consume fuel
            self.stats.fuel_consumed += 1;
        }

        // Get current frame (must exist if state is Running)
        let current_frame = self
            .stack
            .frames
            .last_mut()
            .ok_or(Error::Execution("Execution frame stack empty".into()))?;

        let func = current_frame.get_function()?;
        let code = &func.code;
        let pc = current_frame.pc;

        if pc >= code.len() {
            // Reached end of function code naturally
            println!(
                "DEBUG: Reached end of function {} at PC {}",
                func.func_idx, pc
            );
            // Perform implicit return
            current_frame.return_(&mut self.stack)?;

            // Pop the completed frame
            let completed_frame = self.stack.frames.pop().unwrap(); // Safe due to check above
            let return_values = self
                .stack
                .values
                .split_off(self.stack.values.len() - completed_frame.arity);
            println!(
                "DEBUG: Popped frame for func {}, return values: {:?}",
                completed_frame.func_idx, return_values
            );

            if self.stack.frames.is_empty() {
                // Last frame completed, execution finished
                self.stack.state = StacklessExecutionState::Completed;
                // Push return values back for the caller
                self.stack.values.extend(return_values);
                println!(
                    "DEBUG: Final frame completed. State: Completed. Stack: {:?}",
                    self.stack.values
                );
            } else {
                // Return to caller frame
                let caller_frame = self.stack.frames.last_mut().unwrap(); // Safe: checked !is_empty()
                caller_frame.set_pc(completed_frame.return_pc);
                // Push return values onto caller's effective stack
                self.stack.values.extend(return_values);
                self.stack.state = StacklessExecutionState::Running;
                println!(
                    "DEBUG: Returning to caller frame func {}, PC set to {}, Stack: {:?}",
                    caller_frame.func_idx, caller_frame.pc, self.stack.values
                );
            }
            return Ok(());
        }

        let instruction = &code[pc];
        println!(
            "DEBUG: Executing PC={}, Func={}, Inst: {:?}, Stack: {:?}, Labels: {:?}",
            pc, current_frame.func_idx, instruction, self.stack.values, current_frame.label_stack
        );
        self.stats.instructions_executed += 1;

        // Execute instruction
        // Need to clone Arc<Module> for instruction execution context if needed
        // let frame_module = current_frame.module.clone();

        // Execute requires mutable frame and stack
        // Temporarily take mutable references
        let mut frame_ref = current_frame;
        let mut stack_ref = &mut self.stack;

        // The instruction execution might change the PC or state
        match instruction.execute(stack_ref, &mut frame_ref, self) {
            Ok(_) => {
                // If execution didn't change PC (e.g., branch, return), increment PC
                // Check if PC was already modified by the instruction execution (branch/return)
                if frame_ref.pc == pc {
                    frame_ref.pc += 1;
                }
                // State might have been changed (e.g., to Error by instruction)
                // No need to set Running explicitly unless changed
            }
            Err(e) => {
                // Instruction execution failed
                self.stack.state = StacklessExecutionState::Error(e);
            }
        }

        Ok(())
    }

    // Internal run loop helper
    fn run_loop(&mut self) -> Result<StacklessExecutionState> {
        loop {
            match self.stack.state {
                StacklessExecutionState::Running => {
                    self.step()?;
                }
                StacklessExecutionState::Paused { .. } => {
                    // Return paused state
                    return Ok(self.stack.state.clone());
                }
                StacklessExecutionState::Calling { .. } => {
                    // Handle call setup (push new frame)
                    // This state should ideally be handled within step() or call_function()
                    return Err(Error::Execution(
                        "Unexpected Calling state in run_loop".into(),
                    ));
                }
                StacklessExecutionState::Returning { .. } => {
                    // Handle return (pop frame, push results)
                    // This state should ideally be handled within step() or return instruction
                    return Err(Error::Execution(
                        "Unexpected Returning state in run_loop".into(),
                    ));
                }
                StacklessExecutionState::Branching { .. } => {
                    // This state should ideally be handled within step() or branch instruction
                    return Err(Error::Execution(
                        "Unexpected Branching state in run_loop".into(),
                    ));
                }
                StacklessExecutionState::Completed => {
                    // Execution finished successfully
                    return Ok(StacklessExecutionState::Completed);
                }
                StacklessExecutionState::Finished => {
                    // A potentially different terminal state?
                    return Ok(StacklessExecutionState::Finished);
                }
                StacklessExecutionState::Error(ref e) => {
                    // Propagate error
                    return Err(e.clone());
                }
            }
        }
    }
}

// Implement Stack trait for StacklessStack for compatibility
impl Stack for StacklessStack {
    // Delegate label operations to the current frame if possible, otherwise error
    // This might be problematic as the Stack trait is usually for operand stack + labels

    fn push_label(&mut self, label: stack::Label) -> Result<()> {
        if let Some(frame) = self.frames.last_mut() {
            frame.label_stack.push(behavior::Label {
                arity: label.arity,
                pc: label.pc,
                continuation: label.continuation,
            });
            Ok(())
        } else {
            Err(Error::Execution(
                "No active frame to push label onto".into(),
            ))
        }
    }

    fn pop_label(&mut self) -> Result<stack::Label> {
        if let Some(frame) = self.frames.last_mut() {
            frame
                .label_stack
                .pop()
                .map(|l| stack::Label {
                    arity: l.arity,
                    pc: l.pc,
                    continuation: l.continuation,
                })
                .ok_or(Error::Execution("Label stack empty in frame".into()))
        } else {
            Err(Error::Execution("No active frame to pop label from".into()))
        }
    }

    fn get_label(&self, idx: usize) -> Result<&stack::Label> {
        // Cannot provide a stable reference easily due to conversion
        Err(Error::Unimplemented("get_label for StacklessStack".into()))
    }

    fn get_label_mut(&mut self, idx: usize) -> Result<&mut stack::Label> {
        // Cannot provide a stable reference easily due to conversion
        Err(Error::Unimplemented(
            "get_label_mut for StacklessStack".into(),
        ))
    }

    fn labels_len(&self) -> usize {
        self.frames.last().map_or(0, |f| f.label_stack.len())
    }
}

// Implement StackBehavior for StacklessStack
impl StackBehavior for StacklessStack {
    fn push(&mut self, value: Value) -> Result<()> {
        self.values.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value> {
        self.values.pop().ok_or(Error::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value> {
        self.values.last().ok_or(Error::StackUnderflow)
    }

    fn peek_mut(&mut self) -> Result<&mut Value> {
        self.values.last_mut().ok_or(Error::StackUnderflow)
    }

    fn values(&self) -> &[Value] {
        &self.values
    }

    fn values_mut(&mut self) -> &mut [Value] {
        &mut self.values
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    // Delegate label operations to the current frame
    fn push_label(&mut self, arity: usize, pc: usize) {
        if let Some(frame) = self.frames.last_mut() {
            frame.push_label(arity, pc);
        } else {
            // Log error or handle? Pushing label without frame is likely an issue.
            eprintln!("Warning: push_label called on StacklessStack without an active frame.");
        }
    }

    fn pop_label(&mut self) -> Result<Label> {
        if let Some(frame) = self.frames.last_mut() {
            frame.pop_label()
        } else {
            Err(Error::Execution("No active frame to pop label from".into()))
        }
    }

    fn get_label(&self, index: usize) -> Option<&Label> {
        self.frames.last().and_then(|f| f.get_label(index))
    }
}
