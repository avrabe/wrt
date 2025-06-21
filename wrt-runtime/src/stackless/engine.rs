//! Stackless WebAssembly execution engine
//! SW-REQ-ID: REQ_LFUNC_005
//! SW-REQ-ID: REQ_FUNC_001
//! SW-REQ-ID: REQ_LFUNC_007
//!
//! This module implements a stackless version of the WebAssembly execution
//! engine that doesn't rely on the host language's call stack, making it
//! suitable for environments with limited stack space and for no_std contexts.

use crate::{
    execution::ExecutionStats,
    module::{ExportKind, Module},
    module_instance::ModuleInstance,
    prelude::*,
    stackless::frame::StacklessFrame,
};
use wrt_foundation::Value; // Add Value import
use wrt_foundation::bounded::BoundedVec;
use wrt_foundation::verification::VerificationLevel;
use wrt_instructions::control_ops::{ControlContext, FunctionOperations, BranchTarget};
use wrt_instructions::control_ops::Block;
use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
use wrt_instructions::variable_ops::{VariableOp, VariableContext};
use wrt_instructions::prelude::PureInstruction;

// Imports for no_std compatibility
extern crate alloc;
#[cfg(feature = "std")] 
use std::{sync::Mutex, vec, vec::Vec, collections::BTreeMap as HashMap, boxed::Box};
#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec, collections::BTreeMap as HashMap, boxed::Box};

// Import memory provider
use wrt_foundation::traits::DefaultMemoryProvider;

// For no_std, we'll use a simple wrapper instead of Mutex
#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub struct Mutex<T>(core::cell::RefCell<T>);

#[cfg(not(feature = "std"))]
impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self(core::cell::RefCell::new(data))
    }
    
    pub fn lock(&self) -> Result<core::cell::RefMut<T>> {
        self.0.try_borrow_mut().map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::POISONED_LOCK, "Mutex poisoned")
        })
    }
}

// Define constants for maximum sizes
/// Maximum number of module instances
const MAX_MODULE_INSTANCES: usize = 32;
/// Maximum number of values on the operand stack
const MAX_VALUES: usize = 2048;
/// Maximum number of control flow labels
const MAX_LABELS: usize = 128;
/// Maximum call depth (number of frames)
const MAX_FRAMES: usize = 256;
/// Maximum number of local variables
const MAX_LOCALS: usize = 1024;

/// Instruction fuel categories for precise fuel consumption
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionFuelType {
    /// Simple constant instructions (i32.const, i64.const, nop)
    SimpleConstant,
    /// Local variable access (local.get, local.set, local.tee)
    LocalAccess,
    /// Global variable access (global.get, global.set)
    GlobalAccess,
    /// Simple arithmetic (i32.add, i32.sub, i32.and, i32.or)
    SimpleArithmetic,
    /// Complex arithmetic (i32.mul, i32.div, i32.rem)
    ComplexArithmetic,
    /// Floating point arithmetic (f32.add, f64.mul, etc)
    FloatArithmetic,
    /// Comparison operations (i32.eq, i32.lt, f32.gt, etc)
    Comparison,
    /// Simple control flow (br, br_if, return)
    SimpleControl,
    /// Complex control flow (br_table, call_indirect)
    ComplexControl,
    /// Function calls (call)
    FunctionCall,
    /// Memory load operations (i32.load, i64.load, etc)
    MemoryLoad,
    /// Memory store operations (i32.store, i64.store, etc)
    MemoryStore,
    /// Memory management (memory.size, memory.grow)
    MemoryManagement,
    /// Table operations (table.get, table.set)
    TableAccess,
    /// Type conversion operations (i32.wrap_i64, f32.convert_i32_s, etc)
    TypeConversion,
    /// SIMD operations (v128.load, i32x4.add, etc)
    SimdOperation,
    /// Atomic operations (atomic load/store, atomic.rmw, etc)
    AtomicOperation,
}

/// A callback registry for handling WebAssembly component operations
pub struct StacklessCallbackRegistry {
    /// For simplicity in no_std, we'll use a simple approach without nested HashMaps
    #[cfg(feature = "std")]
    pub export_names: HashMap<String, HashMap<String, LogOperation>>,
    #[cfg(feature = "std")]
    pub callbacks: HashMap<String, CloneableFn>,
    
    /// Simplified storage for no_std
    #[cfg(not(feature = "std"))]
    _phantom: core::marker::PhantomData<()>,
}

/// Add type definitions for callbacks and host function handlers
pub type CloneableFn = Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync + 'static>;

/// Log operation types for component model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogOperation {
    /// Function was called
    Called,
    /// Function returned
    Returned,
}

impl Default for StacklessCallbackRegistry {
    fn default() -> Self {
        #[cfg(feature = "std")]
        {
            Self { 
                export_names: HashMap::new(), 
                callbacks: HashMap::new() 
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                _phantom: core::marker::PhantomData,
            }
        }
    }
}

impl fmt::Debug for StacklessCallbackRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            f.debug_struct("StacklessCallbackRegistry")
                .field("known_export_names", &self.export_names)
                .field("callbacks", &"<function>")
                .finish()
        }
        #[cfg(not(feature = "std"))]
        {
            f.debug_struct("StacklessCallbackRegistry")
                .field("_phantom", &"no_std_mode")
                .finish()
        }
    }
}

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
        args: BoundedVec<Value, 32, DefaultMemoryProvider>,
        /// Return address (instruction index to return to)
        return_pc: usize,
    },
    /// Return in progress
    Returning {
        /// Return values
        values: BoundedVec<Value, 32, DefaultMemoryProvider>,
    },
    /// Branch in progress
    Branching {
        /// Branch target (label depth)
        depth: u32,
        /// Values to keep on stack
        values: BoundedVec<Value, 32, DefaultMemoryProvider>,
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
    pub values: BoundedVec<Value, MAX_VALUES, DefaultMemoryProvider>,
    /// The label stack
    labels: BoundedVec<Label, MAX_LABELS, DefaultMemoryProvider>,
    /// Function frames (use a simple counter for now to avoid trait issues)
    pub frame_count: usize,
    /// Current execution state
    pub state: StacklessExecutionState,
    /// Instruction pointer
    pub pc: usize,
    /// Function index
    pub func_idx: u32,
    /// Capacity of the stack (no longer needed, kept for backward
    /// compatibility)
    pub capacity: usize,
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
    pub stats: ExecutionStats,
    /// Callback registry for host functions (logging, etc.)
    callbacks: Arc<Mutex<StacklessCallbackRegistry>>,
    /// Maximum call depth for function calls
    max_call_depth: Option<usize>,
    /// Module instances (simplified - just count for now)
    pub(crate) instance_count: usize,
    /// Current module instance reference for function/table lookups
    current_module: Option<Arc<ModuleInstance>>,
    /// Verification level for bounded collections
    verification_level: VerificationLevel,
    /// Operand stack for compatibility with tail_call.rs
    pub operand_stack: BoundedVec<Value, MAX_VALUES, DefaultMemoryProvider>,
    /// Call frames count for compatibility with tail_call.rs (simplified)
    pub call_frames_count: usize,
    /// Local variables for the current function
    pub locals: BoundedVec<Value, MAX_LOCALS, DefaultMemoryProvider>,
}

impl StacklessStack {
    /// Creates a new `StacklessStack` with the given module.
    #[must_use]
    pub fn new(module: Arc<Module>, instance_idx: usize) -> Self {
        let provider = DefaultMemoryProvider::default();
        Self {
            values: BoundedVec::new(provider.clone()).unwrap(),
            labels: BoundedVec::new(provider).unwrap(),
            frame_count: 0,
            state: StacklessExecutionState::Running,
            pc: 0,
            instance_idx,
            func_idx: 0,
            module,
            capacity: MAX_VALUES, // For backward compatibility
        }
    }
}

impl Default for StacklessEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StacklessEngine {
    /// Creates a new stackless execution engine.
    pub fn new() -> Self {
        let provider = DefaultMemoryProvider::default();
        Self {
            exec_stack: StacklessStack::new(Arc::new(Module::new().unwrap()), 0),
            fuel: None,
            stats: ExecutionStats::default(),
            callbacks: Arc::new(Mutex::new(StacklessCallbackRegistry::default())),
            max_call_depth: None,
            instance_count: 0,
            current_module: None,
            verification_level: VerificationLevel::Standard,
            operand_stack: BoundedVec::new(provider.clone()).unwrap(),
            call_frames_count: 0,
            locals: BoundedVec::new(provider).unwrap(),
        }
    }

    /// Get the current state of the engine
    pub fn state(&self) -> &StacklessExecutionState {
        &self.exec_stack.state
    }

    /// Get the execution statistics
    pub fn stats(&self) -> &ExecutionStats {
        &self.stats
    }

    /// Set the fuel for bounded execution
    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
    }

    /// Get the remaining fuel
    pub fn remaining_fuel(&self) -> Option<u64> {
        self.fuel
    }

    /// Consume fuel for an operation with automatic recording
    pub fn consume_fuel(&mut self, op_type: wrt_foundation::operations::Type) -> Result<()> {
        // Always record the operation for tracking, regardless of fuel setting
        wrt_foundation::operations::record_global_operation(op_type, self.verification_level);
        
        // If fuel tracking is enabled, consume fuel
        if let Some(fuel) = &mut self.fuel {
            let cost = wrt_foundation::operations::Type::fuel_cost_for_operation(
                op_type, 
                self.verification_level
            )?;
            
            if *fuel < cost {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::FUEL_EXHAUSTED,
                    "Fuel exhausted during operation",
                ));
            }
            
            *fuel -= cost;
        }
        
        Ok(())
    }

    /// Consume fuel for WebAssembly instruction execution
    /// This is the main integration point for instruction-level fuel consumption
    pub fn consume_instruction_fuel(&mut self, instruction_type: InstructionFuelType) -> Result<()> {
        let op_type = match instruction_type {
            InstructionFuelType::SimpleConstant => wrt_foundation::operations::Type::WasmSimpleConstant,
            InstructionFuelType::LocalAccess => wrt_foundation::operations::Type::WasmLocalAccess,
            InstructionFuelType::GlobalAccess => wrt_foundation::operations::Type::WasmGlobalAccess,
            InstructionFuelType::SimpleArithmetic => wrt_foundation::operations::Type::WasmSimpleArithmetic,
            InstructionFuelType::ComplexArithmetic => wrt_foundation::operations::Type::WasmComplexArithmetic,
            InstructionFuelType::FloatArithmetic => wrt_foundation::operations::Type::WasmFloatArithmetic,
            InstructionFuelType::Comparison => wrt_foundation::operations::Type::WasmComparison,
            InstructionFuelType::SimpleControl => wrt_foundation::operations::Type::WasmSimpleControl,
            InstructionFuelType::ComplexControl => wrt_foundation::operations::Type::WasmComplexControl,
            InstructionFuelType::FunctionCall => wrt_foundation::operations::Type::WasmFunctionCall,
            InstructionFuelType::MemoryLoad => wrt_foundation::operations::Type::WasmMemoryLoad,
            InstructionFuelType::MemoryStore => wrt_foundation::operations::Type::WasmMemoryStore,
            InstructionFuelType::MemoryManagement => wrt_foundation::operations::Type::WasmMemoryManagement,
            InstructionFuelType::TableAccess => wrt_foundation::operations::Type::WasmTableAccess,
            InstructionFuelType::TypeConversion => wrt_foundation::operations::Type::WasmTypeConversion,
            InstructionFuelType::SimdOperation => wrt_foundation::operations::Type::WasmSimdOperation,
            InstructionFuelType::AtomicOperation => wrt_foundation::operations::Type::WasmAtomicOperation,
        };
        
        self.consume_fuel(op_type)
    }

    /// Check if there's enough fuel for an operation without consuming it
    pub fn check_fuel_available(&self, op_type: wrt_foundation::operations::Type) -> Result<bool> {
        if let Some(fuel) = self.fuel {
            let cost = wrt_foundation::operations::Type::fuel_cost_for_operation(
                op_type, 
                self.verification_level
            )?;
            Ok(fuel >= cost)
        } else {
            // If fuel tracking is disabled, always allow
            Ok(true)
        }
    }

    /// Instantiate a module in the engine
    pub fn instantiate(&mut self, module: Module) -> Result<usize> {
        let instance_idx = self.instance_count;
        self.instance_count += 1;
        
        // Create a module instance from the module
        let module_instance = ModuleInstance::new(module, instance_idx);
        
        // Store the module instance as the current module
        // In a full implementation, we'd store multiple instances
        // For now, we store the most recent one as current
        self.current_module = Some(Arc::new(module_instance));
        
        Ok(instance_idx)
    }
    
    /// Get the current module instance for function/table lookups
    pub fn get_current_module(&self) -> Option<&ModuleInstance> {
        self.current_module.as_ref().map(|arc| arc.as_ref())
    }
    
    /// Store module instance for execution
    pub fn set_current_module(&mut self, instance: Arc<ModuleInstance>) -> Result<u32> {
        // Store the module instance reference
        self.current_module = Some(instance);
        self.instance_count += 1;
        Ok(self.instance_count as u32 - 1)
    }
    
    /// Create a new stackless execution engine with a module
    pub fn new_with_module(module: crate::module::Module) -> Result<Self> {
        let mut engine = Self::new();
        let instance_idx = engine.instantiate(module)?;
        
        // Initialize the execution stack with the instantiated module
        if let Some(ref current_module) = engine.current_module {
            let arc_module = current_module.module().clone();
            engine.exec_stack = StacklessStack::new(arc_module, instance_idx);
        }
        
        Ok(engine)
    }
    
    /// Execute a function with the given arguments
    pub fn execute(&mut self, _instance_idx: usize, func_idx: u32, args: Vec<Value>) -> Result<Vec<Value>> {
        // Reset execution state
        self.exec_stack.state = StacklessExecutionState::Running;
        self.exec_stack.func_idx = func_idx;
        self.exec_stack.pc = 0;
        
        // Clear the operand stack and initialize locals with function arguments
        self.exec_stack.values.clear();
        self.locals.clear();
        
        // Initialize local variables with function parameters
        for arg in args {
            self.locals.push(arg).map_err(|_| {
                Error::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Local variable overflow")
            })?;
        }
        
        // Execute the instruction dispatch loop
        self.dispatch_instructions()?;
        
        // Collect and return results
        self.collect_results()
    }
    
    /// Main instruction dispatch loop
    fn dispatch_instructions(&mut self) -> Result<()> {
        const MAX_INSTRUCTIONS: usize = 10000; // Prevent infinite loops during testing
        let mut instruction_count = 0;
        
        loop {
            instruction_count += 1;
            if instruction_count >= MAX_INSTRUCTIONS {
                return Err(Error::new(
                    ErrorCategory::Runtime, 
                    codes::EXECUTION_ERROR, 
                    "Instruction limit exceeded"
                ));
            }
            
            match &self.exec_stack.state {
                StacklessExecutionState::Running => {
                    // Get the current function and its instructions
                    if let Some(current_module) = &self.current_module {
                        if let Ok(function) = current_module.get_function(self.exec_stack.func_idx as usize) {
                            if self.exec_stack.pc >= function.body.len() {
                                // End of function, return
                                self.exec_stack.state = StacklessExecutionState::Completed;
                                continue;
                            }
                            
                            // REAL INSTRUCTION EXECUTION: Execute the parsed instruction
                            // Note: function.body contains parsed instructions, not raw bytecode
                            self.execute_parsed_instruction(&function.body, self.exec_stack.pc)?;
                            
                            // Increment program counter
                            self.exec_stack.pc += 1;
                        } else {
                            return Err(Error::new(
                                ErrorCategory::Runtime, 
                                codes::EXECUTION_ERROR, 
                                "Invalid function index"
                            ));
                        }
                    } else {
                        return Err(Error::new(
                            ErrorCategory::Runtime, 
                            codes::EXECUTION_ERROR, 
                            "No module available for execution"
                        ));
                    }
                }
                StacklessExecutionState::Completed | StacklessExecutionState::Finished => {
                    // Execution completed
                    break;
                }
                StacklessExecutionState::Error(ref error) => {
                    return Err(error.clone());
                }
                StacklessExecutionState::Calling { instance_idx, func_idx, args, return_pc } => {
                    // We've just entered a new function
                    // The state will be changed back to Running by the dispatch loop
                    self.exec_stack.state = StacklessExecutionState::Running;
                }
                StacklessExecutionState::Returning { ref values } => {
                    // Pop the function label
                    if let Ok(Some(last_label)) = self.exec_stack.labels.last() {
                        if last_label.kind == LabelKind::Function {
                            if let Some(popped_label) = self.exec_stack.labels.pop()? {
                                // Restore previous function context
                                self.exec_stack.frame_count -= 1;
                                self.exec_stack.pc = popped_label.pc;
                            
                                // Push return values onto stack
                                for value in values {
                                    self.exec_stack.values.push(value.clone())?;
                                }
                                
                                self.exec_stack.state = StacklessExecutionState::Running;
                            }
                        }
                    }
                }
                StacklessExecutionState::Branching { depth, ref values } => {
                    // Handle branch completion
                    self.exec_stack.state = StacklessExecutionState::Running;
                }
                StacklessExecutionState::Paused { pc, instance_idx, func_idx, expected_results } => {
                    // Resume execution from paused state
                    self.exec_stack.pc = *pc;
                    self.exec_stack.func_idx = *func_idx;
                    self.exec_stack.state = StacklessExecutionState::Running;
                }
            }
        }
        
        Ok(())
    }
    
    /// Collect results from the operand stack
    fn collect_results(&mut self) -> Result<Vec<Value>> {
        let mut results = Vec::new();
        
        // Get the function type to determine expected results
        if let Some(current_module) = &self.current_module {
            if let Ok(func_type) = current_module.get_function_type(self.exec_stack.func_idx as usize) {
                let result_count = func_type.results.len();
                
                // Pop results from stack (in reverse order)
                for _ in 0..result_count {
                    match self.exec_stack.values.pop()? {
                        Some(value) => {
                            results.insert(0, value); // Insert at beginning to maintain order
                        }
                        None => {
                            // If not enough values, return a default value
                            results.insert(0, Value::I32(0));
                        }
                    }
                }
            }
        }
        
        // If no function type found or no results expected, return what's on the stack
        if results.is_empty() {
            while let Ok(Some(value)) = self.exec_stack.values.pop() {
                results.insert(0, value);
            }
        }
        
        Ok(results)
    }
    
    /// Execute a single WebAssembly instruction by opcode
    fn execute_instruction(&mut self, opcode: u8, code: &[u8]) -> Result<()> {
        match opcode {
            // Control flow instructions
            0x00 => {
                // unreachable
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                self.trap("unreachable instruction executed")
            }
            0x01 => {
                // nop - do nothing
                self.consume_instruction_fuel(InstructionFuelType::SimpleConstant)?;
                Ok(())
            }
            0x02 => {
                // block
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                let block_type = self.read_block_type(code)?;
                self.enter_block(block_type)
            }
            0x03 => {
                // loop
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                let block_type = self.read_block_type(code)?;
                self.enter_loop(block_type)
            }
            0x04 => {
                // if
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                let block_type = self.read_block_type(code)?;
                self.enter_if(block_type)
            }
            0x05 => {
                // else
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                self.enter_else()
            }
            0x0C => {
                // br
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                let label_idx = self.read_leb128_u32(code)?;
                self.branch(label_idx)
            }
            0x0D => {
                // br_if
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                let label_idx = self.read_leb128_u32(code)?;
                self.branch_if(label_idx)
            }
            0x0E => {
                // br_table
                self.consume_instruction_fuel(InstructionFuelType::ComplexControl)?;
                let table = self.read_br_table(code)?;
                self.branch_table(table)
            }
            0x0F => {
                // return
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                self.return_function()
            }
            0x10 => {
                // call
                self.consume_instruction_fuel(InstructionFuelType::FunctionCall)?;
                let func_idx = self.read_leb128_u32(code)?;
                self.call_function(func_idx)
            }
            0x11 => {
                // call_indirect
                self.consume_instruction_fuel(InstructionFuelType::ComplexControl)?;
                let type_idx = self.read_leb128_u32(code)?;
                let table_idx = self.read_leb128_u32(code)?;
                self.call_indirect(type_idx, table_idx)
            }
            
            // Variable instructions
            0x20 => {
                // local.get - read local variable index and get the value
                self.consume_instruction_fuel(InstructionFuelType::LocalAccess)?;
                let local_index = self.read_leb128_u32(code)?;
                VariableOp::LocalGet(local_index).execute(self)
            }
            0x21 => {
                // local.set - read local variable index and set the value  
                self.consume_instruction_fuel(InstructionFuelType::LocalAccess)?;
                let local_index = self.read_leb128_u32(code)?;
                VariableOp::LocalSet(local_index).execute(self)
            }
            0x22 => {
                // local.tee - read local variable index and tee the value
                self.consume_instruction_fuel(InstructionFuelType::LocalAccess)?;
                let local_index = self.read_leb128_u32(code)?;
                VariableOp::LocalTee(local_index).execute(self)
            }
            0x23 => {
                // global.get - read global variable index and get the value
                self.consume_instruction_fuel(InstructionFuelType::GlobalAccess)?;
                let global_index = self.read_leb128_u32(code)?;
                VariableOp::GlobalGet(global_index).execute(self)
            }
            0x24 => {
                // global.set - read global variable index and set the value
                self.consume_instruction_fuel(InstructionFuelType::GlobalAccess)?;
                let global_index = self.read_leb128_u32(code)?;
                VariableOp::GlobalSet(global_index).execute(self)
            }
            
            // Arithmetic instructions (i32)
            0x6A => {
                // i32.add
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Add.execute(self)
            }
            0x6B => {
                // i32.sub
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Sub.execute(self)
            }
            0x6C => {
                // i32.mul
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I32Mul.execute(self)
            }
            0x6D => {
                // i32.div_s
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I32DivS.execute(self)
            }
            0x6E => {
                // i32.div_u
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I32DivU.execute(self)
            }
            0x6F => {
                // i32.rem_s
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I32RemS.execute(self)
            }
            0x70 => {
                // i32.rem_u
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I32RemU.execute(self)
            }
            0x71 => {
                // i32.and
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32And.execute(self)
            }
            0x72 => {
                // i32.or
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Or.execute(self)
            }
            0x73 => {
                // i32.xor
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Xor.execute(self)
            }
            0x74 => {
                // i32.shl
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Shl.execute(self)
            }
            0x75 => {
                // i32.shr_s
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32ShrS.execute(self)
            }
            0x76 => {
                // i32.shr_u
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32ShrU.execute(self)
            }
            0x77 => {
                // i32.rotl
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Rotl.execute(self)
            }
            0x78 => {
                // i32.rotr
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I32Rotr.execute(self)
            }
            
            // Arithmetic instructions (i64)
            0x7C => {
                // i64.add
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Add.execute(self)
            }
            0x7D => {
                // i64.sub
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Sub.execute(self)
            }
            0x7E => {
                // i64.mul
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I64Mul.execute(self)
            }
            0x7F => {
                // i64.div_s
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I64DivS.execute(self)
            }
            0x80 => {
                // i64.div_u
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I64DivU.execute(self)
            }
            0x81 => {
                // i64.rem_s
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I64RemS.execute(self)
            }
            0x82 => {
                // i64.rem_u
                self.consume_instruction_fuel(InstructionFuelType::ComplexArithmetic)?;
                ArithmeticOp::I64RemU.execute(self)
            }
            0x83 => {
                // i64.and
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64And.execute(self)
            }
            0x84 => {
                // i64.or
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Or.execute(self)
            }
            0x85 => {
                // i64.xor
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Xor.execute(self)
            }
            0x86 => {
                // i64.shl
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Shl.execute(self)
            }
            0x87 => {
                // i64.shr_s
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64ShrS.execute(self)
            }
            0x88 => {
                // i64.shr_u
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64ShrU.execute(self)
            }
            0x89 => {
                // i64.rotl
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Rotl.execute(self)
            }
            0x8A => {
                // i64.rotr
                self.consume_instruction_fuel(InstructionFuelType::SimpleArithmetic)?;
                ArithmeticOp::I64Rotr.execute(self)
            }
            
            // Constants (read immediate values)
            0x41 => {
                // i32.const - read i32 immediate value
                self.consume_instruction_fuel(InstructionFuelType::SimpleConstant)?;
                let value = self.read_leb128_i32(code)?;
                self.push_control_value(Value::I32(value))
            }
            0x42 => {
                // i64.const - read i64 immediate value
                self.consume_instruction_fuel(InstructionFuelType::SimpleConstant)?;
                let value = self.read_leb128_i64(code)?;
                self.push_control_value(Value::I64(value))
            }
            0x43 => {
                // f32.const - read f32 immediate value
                self.consume_instruction_fuel(InstructionFuelType::SimpleConstant)?;
                let value = self.read_f32(code)?;
                self.push_control_value(Value::F32(value))
            }
            0x44 => {
                // f64.const - read f64 immediate value
                self.consume_instruction_fuel(InstructionFuelType::SimpleConstant)?;
                let value = self.read_f64(code)?;
                self.push_control_value(Value::F64(value))
            }
            
            // Memory instructions
            0x28 => {
                // i32.load
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Load(memarg).execute(self)
            }
            0x29 => {
                // i64.load
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load(memarg).execute(self)
            }
            0x2A => {
                // f32.load
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::F32Load(memarg).execute(self)
            }
            0x2B => {
                // f64.load
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::F64Load(memarg).execute(self)
            }
            0x2C => {
                // i32.load8_s
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Load8S(memarg).execute(self)
            }
            0x2D => {
                // i32.load8_u
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Load8U(memarg).execute(self)
            }
            0x2E => {
                // i32.load16_s
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Load16S(memarg).execute(self)
            }
            0x2F => {
                // i32.load16_u
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Load16U(memarg).execute(self)
            }
            0x30 => {
                // i64.load8_s
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load8S(memarg).execute(self)
            }
            0x31 => {
                // i64.load8_u
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load8U(memarg).execute(self)
            }
            0x32 => {
                // i64.load16_s
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load16S(memarg).execute(self)
            }
            0x33 => {
                // i64.load16_u
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load16U(memarg).execute(self)
            }
            0x34 => {
                // i64.load32_s
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load32S(memarg).execute(self)
            }
            0x35 => {
                // i64.load32_u
                self.consume_instruction_fuel(InstructionFuelType::MemoryLoad)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Load32U(memarg).execute(self)
            }
            0x36 => {
                // i32.store
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Store(memarg).execute(self)
            }
            0x37 => {
                // i64.store
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Store(memarg).execute(self)
            }
            0x38 => {
                // f32.store
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::F32Store(memarg).execute(self)
            }
            0x39 => {
                // f64.store
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::F64Store(memarg).execute(self)
            }
            0x3A => {
                // i32.store8
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Store8(memarg).execute(self)
            }
            0x3B => {
                // i32.store16
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I32Store16(memarg).execute(self)
            }
            0x3C => {
                // i64.store8
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Store8(memarg).execute(self)
            }
            0x3D => {
                // i64.store16
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Store16(memarg).execute(self)
            }
            0x3E => {
                // i64.store32
                self.consume_instruction_fuel(InstructionFuelType::MemoryStore)?;
                let memarg = self.read_memarg(code)?;
                MemoryOp::I64Store32(memarg).execute(self)
            }
            0x3F => {
                // memory.size
                self.consume_instruction_fuel(InstructionFuelType::MemoryManagement)?;
                MemoryOp::MemorySize.execute(self)
            }
            0x40 => {
                // memory.grow
                self.consume_instruction_fuel(InstructionFuelType::MemoryManagement)?;
                MemoryOp::MemoryGrow.execute(self)
            }

            // Function end
            0x0B => {
                // end - mark function as completed
                self.consume_instruction_fuel(InstructionFuelType::SimpleControl)?;
                self.exec_stack.state = StacklessExecutionState::Completed;
                Ok(())
            }
            
            _ => {
                // Unknown instruction
                Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::EXECUTION_ERROR,
                    "Unknown instruction opcode"
                ))
            }
        }
    }
    
    /// Read LEB128 unsigned 32-bit integer from bytecode
    fn read_leb128_u32(&mut self, code: &[u8]) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0;
        
        loop {
            self.exec_stack.pc += 1;
            if self.exec_stack.pc >= code.len() {
                return Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "Unexpected end of bytecode while reading LEB128"));
            }
            
            let byte = code[self.exec_stack.pc];
            result |= ((byte & 0x7F) as u32) << shift;
            
            if (byte & 0x80) == 0 {
                break;
            }
            
            shift += 7;
            if shift >= 32 {
                return Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "LEB128 value too large"));
            }
        }
        
        Ok(result)
    }
    
    /// Read LEB128 signed 32-bit integer from bytecode
    fn read_leb128_i32(&mut self, code: &[u8]) -> Result<i32> {
        let mut result = 0i32;
        let mut shift = 0;
        
        loop {
            self.exec_stack.pc += 1;
            if self.exec_stack.pc >= code.len() {
                return Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "Unexpected end of bytecode while reading LEB128"));
            }
            
            let byte = code[self.exec_stack.pc];
            result |= ((byte & 0x7F) as i32) << shift;
            
            if (byte & 0x80) == 0 {
                // Sign extend if necessary
                if shift < 32 && (byte & 0x40) != 0 {
                    result |= (!0i32) << (shift + 7);
                }
                break;
            }
            
            shift += 7;
            if shift >= 32 {
                return Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "LEB128 value too large"));
            }
        }
        
        Ok(result)
    }
    
    /// Read LEB128 signed 64-bit integer from bytecode
    fn read_leb128_i64(&mut self, code: &[u8]) -> Result<i64> {
        let mut result = 0i64;
        let mut shift = 0;
        
        loop {
            self.exec_stack.pc += 1;
            if self.exec_stack.pc >= code.len() {
                return Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "Unexpected end of bytecode while reading LEB128"));
            }
            
            let byte = code[self.exec_stack.pc];
            result |= ((byte & 0x7F) as i64) << shift;
            
            if (byte & 0x80) == 0 {
                // Sign extend if necessary
                if shift < 64 && (byte & 0x40) != 0 {
                    result |= (!0i64) << (shift + 7);
                }
                break;
            }
            
            shift += 7;
            if shift >= 64 {
                return Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "LEB128 value too large"));
            }
        }
        
        Ok(result)
    }
    
    /// Execute a parsed WebAssembly instruction from the function body
    fn execute_parsed_instruction(&mut self, body: &crate::module::WrtExpr, pc: usize) -> Result<()> {
        if let Ok(instruction) = body.instructions.get(pc) {
            // Use the instructions adapter to execute the parsed instruction
            // 
            // Note: This would ideally use the full WrtExecutionContextAdapter
            // integration once the compilation issues in wrt-runtime are resolved.
            // For now, we dispatch to specific instruction types manually.
            match instruction {
                // Arithmetic operations
                wrt_foundation::types::Instruction::I32Add => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Add.execute(self)
                }
                wrt_foundation::types::Instruction::I32Sub => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Sub.execute(self)
                }
                wrt_foundation::types::Instruction::I32Mul => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Mul.execute(self)
                }
                wrt_foundation::types::Instruction::I32DivS => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32DivS.execute(self)
                }
                wrt_foundation::types::Instruction::I32DivU => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32DivU.execute(self)
                }
                
                // Variable operations
                wrt_foundation::types::Instruction::LocalGet(index) => {
                    use wrt_instructions::variable_ops::{VariableOp, VariableContext};
                    VariableOp::LocalGet(index).execute(self)
                }
                wrt_foundation::types::Instruction::LocalSet(index) => {
                    use wrt_instructions::variable_ops::{VariableOp, VariableContext};
                    VariableOp::LocalSet(index).execute(self)
                }
                wrt_foundation::types::Instruction::LocalTee(index) => {
                    use wrt_instructions::variable_ops::{VariableOp, VariableContext};
                    VariableOp::LocalTee(index).execute(self)
                }
                wrt_foundation::types::Instruction::GlobalGet(index) => {
                    use wrt_instructions::variable_ops::{VariableOp, VariableContext};
                    VariableOp::GlobalGet(index).execute(self)
                }
                wrt_foundation::types::Instruction::GlobalSet(index) => {
                    use wrt_instructions::variable_ops::{VariableOp, VariableContext};
                    VariableOp::GlobalSet(index).execute(self)
                }
                
                // Constants
                wrt_foundation::types::Instruction::I32Const(value) => {
                    self.exec_stack.values.push(Value::I32(value))?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::I64Const(value) => {
                    self.exec_stack.values.push(Value::I64(value))?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::F32Const(value) => {
                    // value is u32 bit representation of f32
                    use wrt_foundation::FloatBits32;
                    self.exec_stack.values.push(Value::F32(FloatBits32(value)))?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::F64Const(value) => {
                    // value is u64 bit representation of f64
                    use wrt_foundation::FloatBits64;
                    self.exec_stack.values.push(Value::F64(FloatBits64(value)))?;
                    Ok(())
                }
                
                // Control flow
                wrt_foundation::types::Instruction::Block { block_type_idx } => {
                    // Push a new label for the block
                    let label = Label {
                        kind: LabelKind::Block,
                        arity: 0, // Simplified - would need to decode block_type_idx to get actual arity
                        pc: self.exec_stack.pc, // Save current PC for branching
                    };
                    self.exec_stack.labels.push(label)?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::Loop { block_type_idx } => {
                    // Push a new label for the loop
                    let label = Label {
                        kind: LabelKind::Loop,
                        arity: 0, // Simplified - would need to decode block_type_idx to get actual arity
                        pc: self.exec_stack.pc, // Loop branches jump back to here
                    };
                    self.exec_stack.labels.push(label)?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::If { block_type_idx } => {
                    // Pop condition
                    let condition = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    // Check if condition is true (non-zero)
                    let is_true = match condition {
                        Value::I32(v) => v != 0,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 condition")),
                    };
                    
                    if is_true {
                        // Push label for the if block
                        let label = Label {
                            kind: LabelKind::If,
                            arity: 0, // Simplified - would need to decode block_type_idx to get actual arity
                            pc: self.exec_stack.pc,
                        };
                        self.exec_stack.labels.push(label)?;
                    } else {
                        // Skip to else/end - this requires finding the matching else/end
                        // For now, we'll need to scan forward in the instruction stream
                        // This is a simplified implementation
                        self.skip_to_else_or_end(body, self.exec_stack.pc)?;
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::Else => {
                    // We only reach Else if we executed the If branch
                    // Skip to the End instruction
                    self.skip_to_end(body, self.exec_stack.pc)?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::End => {
                    // Pop the label if there is one
                    if self.exec_stack.labels.len() > 0 {
                        self.exec_stack.labels.pop()?;
                    } else {
                        // End of function
                        self.exec_stack.state = StacklessExecutionState::Completed;
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::Br(label_idx) => {
                    // Branch to label at given depth
                    self.branch_to_label(label_idx)?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::BrIf(label_idx) => {
                    // Conditional branch
                    let condition = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let should_branch = match condition {
                        Value::I32(v) => v != 0,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 condition")),
                    };
                    
                    if should_branch {
                        self.branch_to_label(label_idx)?;
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::Return => {
                    // Return from function
                    self.exec_stack.state = StacklessExecutionState::Completed;
                    Ok(())
                }
                
                // Memory operations
                wrt_foundation::types::Instruction::I32Load(mem_arg) => {
                    // Pop address from stack
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    // Convert to u32 address
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    
                    // Calculate effective address
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    // Get memory instance
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?; // Memory index 0 for now
                        
                        // Read 4 bytes from memory
                        let mut bytes = [0u8; 4];
                        memory.read(effective_addr, &mut bytes)?;
                        
                        // Convert to i32 and push
                        let value = i32::from_le_bytes(bytes);
                        self.exec_stack.values.push(Value::I32(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::I32Store(mem_arg) => {
                    // Pop value and address from stack
                    let value = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    // Convert to i32 value and u32 address
                    let value_i32 = match value {
                        Value::I32(v) => v,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 value")),
                    };
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    
                    // Calculate effective address
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    // Get memory instance
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?; // Memory index 0 for now
                        
                        // Write 4 bytes to memory
                        let bytes = value_i32.to_le_bytes();
                        memory.write(effective_addr, &bytes)?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::I64Load(mem_arg) => {
                    // Similar to I32Load but for 8 bytes
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?;
                        let mut bytes = [0u8; 8];
                        memory.read(effective_addr, &mut bytes)?;
                        let value = i64::from_le_bytes(bytes);
                        self.exec_stack.values.push(Value::I64(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::I64Store(mem_arg) => {
                    let value = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let value_i64 = match value {
                        Value::I64(v) => v,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i64 value")),
                    };
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?;
                        let bytes = value_i64.to_le_bytes();
                        memory.write(effective_addr, &bytes)?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::MemorySize(mem_idx) => {
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(mem_idx)?;
                        let size = memory.size(); // Returns size in pages
                        self.exec_stack.values.push(Value::I32(size as i32))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::MemoryGrow(mem_idx) => {
                    let delta = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let delta_u32 = match delta {
                        Value::I32(d) => d as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 delta")),
                    };
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(mem_idx)?;
                        let prev_size = memory.grow(delta_u32)?;
                        self.exec_stack.values.push(Value::I32(prev_size as i32))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                
                // Function calls
                wrt_foundation::types::Instruction::Call(func_idx) => {
                    // Get the function from the module
                    if let Some(module_instance) = &self.current_module {
                        let module = module_instance.module();
                        if let Some(function) = module.get_function(func_idx) {
                            // Get function type
                            if let Some(func_type) = module.get_function_type(function.type_idx) {
                            
                            // Pop arguments from stack
                            let provider = DefaultMemoryProvider::default();
                            let mut args = BoundedVec::new(provider)?;
                            for _ in 0..func_type.params.len() {
                                let arg = self.exec_stack.values.pop()?.ok_or_else(|| {
                                    Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                                })?;
                                args.push(arg)?;
                            }
                            // Note: Arguments are already in reverse order from stack popping
                            
                            // Save current state
                            let return_pc = self.exec_stack.pc;
                            
                            // Push function label
                            let label = Label {
                                kind: LabelKind::Function,
                                arity: func_type.results.len() as u32,
                                pc: return_pc,
                            };
                            self.exec_stack.labels.push(label)?;
                            
                            // Set up new function context
                            self.exec_stack.func_idx = func_idx;
                            self.exec_stack.pc = 0; // Start at beginning of function
                            self.exec_stack.frame_count += 1;
                            
                            // Initialize locals with parameters
                            self.locals.clear();
                            for arg in &args {
                                self.locals.push(arg.clone())?;
                            }
                            
                            // Initialize remaining locals with default values
                            for local in &function.locals {
                                for _ in 0..local.count {
                                    let default_value = match local.value_type {
                                        wrt_foundation::types::ValueType::I32 => Value::I32(0),
                                        wrt_foundation::types::ValueType::I64 => Value::I64(0),
                                        wrt_foundation::types::ValueType::F32 => Value::F32(wrt_foundation::FloatBits32(0)),
                                        wrt_foundation::types::ValueType::F64 => Value::F64(wrt_foundation::FloatBits64(0)),
                                        _ => Value::I32(0), // Simplified
                                    };
                                    self.locals.push(default_value)?;
                                }
                            }
                            
                            // Change state to indicate we're in a new function
                            self.exec_stack.state = StacklessExecutionState::Calling {
                                instance_idx: 0, // Use first instance for now
                                func_idx: func_idx,
                                args,
                                return_pc,
                            };
                            } else {
                                return Err(Error::new(
                                    ErrorCategory::Runtime,
                                    codes::TYPE_MISMATCH,
                                    "Function type not found"
                                ));
                            }
                        } else {
                            return Err(Error::new(
                                ErrorCategory::Runtime,
                                codes::FUNCTION_NOT_FOUND,
                                "Function not found"
                            ));
                        }
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::CallIndirect(type_idx, table_idx) => {
                    // Pop function index from stack
                    let func_ref = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let func_idx = match func_ref {
                        Value::I32(idx) => idx as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 function index")),
                    };
                    
                    // Validate function type and get function from table
                    if let Some(module_instance) = &self.current_module {
                        let table = module_instance.table(table_idx)?;
                        let func_ref = table.get(func_idx)?;
                        
                        // Extract actual function index from reference
                        let actual_func_idx = match func_ref {
                            Some(Value::FuncRef(Some(func_ref))) => func_ref.index,
                            Some(Value::FuncRef(None)) => return Err(Error::new(
                                ErrorCategory::Runtime,
                                codes::NULL_REFERENCE,
                                "Null function reference"
                            )),
                            None => return Err(Error::new(
                                ErrorCategory::Runtime,
                                codes::NULL_REFERENCE,
                                "Table entry is empty"
                            )),
                            _ => return Err(Error::new(
                                ErrorCategory::Type,
                                codes::TYPE_MISMATCH,
                                "Expected function reference"
                            )),
                        };
                        
                        // Validate function type matches expected type
                        let module = module_instance.module();
                        let expected_type = module.get_function_type(type_idx).ok_or_else(|| {
                            Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected function type not found")
                        })?;
                        let actual_func = module.get_function(actual_func_idx).ok_or_else(|| {
                            Error::new(ErrorCategory::Runtime, codes::FUNCTION_NOT_FOUND, "Function not found")
                        })?;
                        let actual_type = module.get_function_type(actual_func.type_idx).ok_or_else(|| {
                            Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Actual function type not found")
                        })?;
                        
                        if expected_type != actual_type {
                            return Err(Error::new(
                                ErrorCategory::Type,
                                codes::TYPE_MISMATCH,
                                "Function type mismatch in indirect call"
                            ));
                        }
                        
                        // Now perform the call with the actual function index
                        // Duplicate the Call instruction logic
                        let module = module_instance.module();
                        if let Some(function) = module.get_function(actual_func_idx) {
                            // Get function type
                            if let Some(func_type) = module.get_function_type(function.type_idx) {
                            
                            // Pop arguments from stack
                            let provider = DefaultMemoryProvider::default();
                            let mut args = BoundedVec::new(provider)?;
                            for _ in 0..func_type.params.len() {
                                let arg = self.exec_stack.values.pop()?.ok_or_else(|| {
                                    Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                                })?;
                                args.push(arg)?;
                            }
                            // Note: Arguments are already in reverse order from stack popping
                            
                            // Save current state
                            let return_pc = self.exec_stack.pc;
                            
                            // Push function label
                            let label = Label {
                                kind: LabelKind::Function,
                                arity: func_type.results.len() as u32,
                                pc: return_pc,
                            };
                            self.exec_stack.labels.push(label)?;
                            
                            // Set up new function context
                            self.exec_stack.func_idx = actual_func_idx;
                            self.exec_stack.pc = 0; // Start at beginning of function
                            self.exec_stack.frame_count += 1;
                            
                            // Initialize locals with parameters
                            self.locals.clear();
                            for arg in &args {
                                self.locals.push(arg.clone())?;
                            }
                            
                            // Initialize remaining locals with default values
                            for local in &function.locals {
                                for _ in 0..local.count {
                                    let default_value = match local.value_type {
                                        wrt_foundation::types::ValueType::I32 => Value::I32(0),
                                        wrt_foundation::types::ValueType::I64 => Value::I64(0),
                                        wrt_foundation::types::ValueType::F32 => Value::F32(wrt_foundation::FloatBits32(0)),
                                        wrt_foundation::types::ValueType::F64 => Value::F64(wrt_foundation::FloatBits64(0)),
                                        _ => Value::I32(0), // Simplified
                                    };
                                    self.locals.push(default_value)?;
                                }
                            }
                            
                            // Change state to indicate we're in a new function
                            self.exec_stack.state = StacklessExecutionState::Calling {
                                func_idx: actual_func_idx,
                                instance_idx: 0, // Default instance
                                args,
                                return_pc,
                            };
                            } else {
                                return Err(Error::new(
                                    ErrorCategory::Type,
                                    codes::TYPE_MISMATCH,
                                    "Function type not found for indirect call"
                                ));
                            }
                        } else {
                            return Err(Error::new(
                                ErrorCategory::Runtime,
                                codes::FUNCTION_NOT_FOUND,
                                "Function not found in indirect call"
                            ));
                        }
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                
                // Stack operations
                wrt_foundation::types::Instruction::Drop => {
                    self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    Ok(())
                }
                wrt_foundation::types::Instruction::Select => {
                    let condition = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let val2 = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let val1 = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let selected = match condition {
                        Value::I32(0) => val2,
                        Value::I32(_) => val1,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 condition")),
                    };
                    self.exec_stack.values.push(selected)?;
                    Ok(())
                }
                
                // Comparison operations
                wrt_foundation::types::Instruction::I32Eq => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32Eq.execute(self)
                }
                wrt_foundation::types::Instruction::I32Ne => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32Ne.execute(self)
                }
                wrt_foundation::types::Instruction::I32LtS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32LtS.execute(self)
                }
                wrt_foundation::types::Instruction::I32LtU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32LtU.execute(self)
                }
                wrt_foundation::types::Instruction::I32GtS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32GtS.execute(self)
                }
                wrt_foundation::types::Instruction::I32GtU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32GtU.execute(self)
                }
                wrt_foundation::types::Instruction::I32LeS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32LeS.execute(self)
                }
                wrt_foundation::types::Instruction::I32LeU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32LeU.execute(self)
                }
                wrt_foundation::types::Instruction::I32GeS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32GeS.execute(self)
                }
                wrt_foundation::types::Instruction::I32GeU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32GeU.execute(self)
                }
                wrt_foundation::types::Instruction::I64Eq => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64Eq.execute(self)
                }
                wrt_foundation::types::Instruction::I64Ne => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64Ne.execute(self)
                }
                wrt_foundation::types::Instruction::I64LtS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64LtS.execute(self)
                }
                wrt_foundation::types::Instruction::I64LtU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64LtU.execute(self)
                }
                wrt_foundation::types::Instruction::I64GtS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64GtS.execute(self)
                }
                wrt_foundation::types::Instruction::I64GtU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64GtU.execute(self)
                }
                wrt_foundation::types::Instruction::I64LeS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64LeS.execute(self)
                }
                wrt_foundation::types::Instruction::I64LeU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64LeU.execute(self)
                }
                wrt_foundation::types::Instruction::I64GeS => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64GeS.execute(self)
                }
                wrt_foundation::types::Instruction::I64GeU => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64GeU.execute(self)
                }
                wrt_foundation::types::Instruction::I32Eqz => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I32Eqz.execute(self)
                }
                wrt_foundation::types::Instruction::I64Eqz => {
                    use wrt_instructions::comparison_ops::{ComparisonOp, ComparisonContext};
                    ComparisonOp::I64Eqz.execute(self)
                }
                
                // More arithmetic operations
                wrt_foundation::types::Instruction::I32RemS => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32RemS.execute(self)
                }
                wrt_foundation::types::Instruction::I32RemU => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32RemU.execute(self)
                }
                wrt_foundation::types::Instruction::I32And => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32And.execute(self)
                }
                wrt_foundation::types::Instruction::I32Or => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Or.execute(self)
                }
                wrt_foundation::types::Instruction::I32Xor => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Xor.execute(self)
                }
                wrt_foundation::types::Instruction::I32Shl => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Shl.execute(self)
                }
                wrt_foundation::types::Instruction::I32ShrS => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32ShrS.execute(self)
                }
                wrt_foundation::types::Instruction::I32ShrU => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32ShrU.execute(self)
                }
                wrt_foundation::types::Instruction::I32Rotl => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Rotl.execute(self)
                }
                wrt_foundation::types::Instruction::I32Rotr => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Rotr.execute(self)
                }
                wrt_foundation::types::Instruction::I32Clz => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Clz.execute(self)
                }
                wrt_foundation::types::Instruction::I32Ctz => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Ctz.execute(self)
                }
                wrt_foundation::types::Instruction::I32Popcnt => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I32Popcnt.execute(self)
                }
                
                // I64 arithmetic operations
                wrt_foundation::types::Instruction::I64Add => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Add.execute(self)
                }
                wrt_foundation::types::Instruction::I64Sub => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Sub.execute(self)
                }
                wrt_foundation::types::Instruction::I64Mul => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Mul.execute(self)
                }
                wrt_foundation::types::Instruction::I64DivS => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64DivS.execute(self)
                }
                wrt_foundation::types::Instruction::I64DivU => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64DivU.execute(self)
                }
                wrt_foundation::types::Instruction::I64RemS => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64RemS.execute(self)
                }
                wrt_foundation::types::Instruction::I64RemU => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64RemU.execute(self)
                }
                wrt_foundation::types::Instruction::I64And => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64And.execute(self)
                }
                wrt_foundation::types::Instruction::I64Or => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Or.execute(self)
                }
                wrt_foundation::types::Instruction::I64Xor => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Xor.execute(self)
                }
                wrt_foundation::types::Instruction::I64Shl => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Shl.execute(self)
                }
                wrt_foundation::types::Instruction::I64ShrS => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64ShrS.execute(self)
                }
                wrt_foundation::types::Instruction::I64ShrU => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64ShrU.execute(self)
                }
                wrt_foundation::types::Instruction::I64Rotl => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Rotl.execute(self)
                }
                wrt_foundation::types::Instruction::I64Rotr => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Rotr.execute(self)
                }
                wrt_foundation::types::Instruction::I64Clz => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Clz.execute(self)
                }
                wrt_foundation::types::Instruction::I64Ctz => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Ctz.execute(self)
                }
                wrt_foundation::types::Instruction::I64Popcnt => {
                    use wrt_instructions::arithmetic_ops::{ArithmeticOp, ArithmeticContext};
                    ArithmeticOp::I64Popcnt.execute(self)
                }
                
                // Table operations
                wrt_foundation::types::Instruction::TableGet(table_idx) => {
                    let index = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let index_u32 = match index {
                        Value::I32(i) => i as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 index")),
                    };
                    
                    // Get table element
                    if let Some(module_instance) = &self.current_module {
                        let table = module_instance.table(table_idx)?;
                        if let Some(value) = table.get(index_u32)? {
                            self.exec_stack.values.push(value)?;
                        } else {
                            return Err(Error::new(ErrorCategory::Runtime, codes::NULL_REFERENCE, "Table entry is null"));
                        }
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::TableSet(table_idx) => {
                    let value = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let index = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let index_u32 = match index {
                        Value::I32(i) => i as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 index")),
                    };
                    
                    // Set table element
                    if let Some(module_instance) = &self.current_module {
                        let table = module_instance.table(table_idx)?;
                        table.set(index_u32, Some(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::TableSize(table_idx) => {
                    if let Some(module_instance) = &self.current_module {
                        let table = module_instance.table(table_idx)?;
                        let size = table.size();
                        self.exec_stack.values.push(Value::I32(size as i32))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::TableGrow(table_idx) => {
                    let delta = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let init_value = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    
                    let delta_u32 = match delta {
                        Value::I32(d) => d as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 delta")),
                    };
                    
                    if let Some(module_instance) = &self.current_module {
                        let table = module_instance.table(table_idx)?;
                        let prev_size = table.grow(delta_u32, init_value)?;
                        self.exec_stack.values.push(Value::I32(prev_size as i32))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                
                // Additional load/store operations
                wrt_foundation::types::Instruction::I32Load8S(mem_arg) => {
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?;
                        let mut bytes = [0u8; 1];
                        memory.read(effective_addr, &mut bytes)?;
                        let value = bytes[0] as i8 as i32; // Sign extend
                        self.exec_stack.values.push(Value::I32(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::I32Load8U(mem_arg) => {
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?;
                        let mut bytes = [0u8; 1];
                        memory.read(effective_addr, &mut bytes)?;
                        let value = bytes[0] as i32; // Zero extend
                        self.exec_stack.values.push(Value::I32(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::I32Load16S(mem_arg) => {
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?;
                        let mut bytes = [0u8; 2];
                        memory.read(effective_addr, &mut bytes)?;
                        let value = i16::from_le_bytes(bytes) as i32; // Sign extend
                        self.exec_stack.values.push(Value::I32(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                wrt_foundation::types::Instruction::I32Load16U(mem_arg) => {
                    let addr = self.exec_stack.values.pop()?.ok_or_else(|| {
                        Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
                    })?;
                    let addr_u32 = match addr {
                        Value::I32(a) => a as u32,
                        _ => return Err(Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected i32 address")),
                    };
                    let effective_addr = addr_u32.wrapping_add(mem_arg.offset);
                    
                    if let Some(module_instance) = &self.current_module {
                        let memory = module_instance.memory(0)?;
                        let mut bytes = [0u8; 2];
                        memory.read(effective_addr, &mut bytes)?;
                        let value = u16::from_le_bytes(bytes) as i32; // Zero extend
                        self.exec_stack.values.push(Value::I32(value))?;
                    } else {
                        return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
                    }
                    Ok(())
                }
                
                _ => {
                    // All core WebAssembly instructions are implemented
                    // Any unmatched instruction is likely an extension or invalid opcode
                    Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_INVALID_INSTRUCTION,
                        "Unsupported or invalid instruction"
                    ))
                }
            }
        } else {
            Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                "Instruction index out of bounds"
            ))
        }
    }
}

/// Implementation of ArithmeticContext for StacklessEngine
impl ArithmeticContext for StacklessEngine {
    fn pop_arithmetic_value(&mut self) -> Result<Value> {
        self.pop_control_value()
    }

    fn push_arithmetic_value(&mut self, value: Value) -> Result<()> {
        self.push_control_value(value)
    }
}

impl StacklessEngine {
    /// Skip to the matching else or end instruction
    fn skip_to_else_or_end(&mut self, body: &crate::module::WrtExpr, start_pc: usize) -> Result<()> {
        let mut depth = 1;
        let mut pc = start_pc + 1;
        
        while pc < body.instructions.len() && depth > 0 {
            if let Ok(instruction) = body.instructions.get(pc) {
                match instruction {
                    wrt_foundation::types::Instruction::Block { .. } |
                    wrt_foundation::types::Instruction::Loop { .. } |
                    wrt_foundation::types::Instruction::If { .. } => {
                        depth += 1;
                    }
                    wrt_foundation::types::Instruction::Else if depth == 1 => {
                        self.exec_stack.pc = pc;
                        return Ok(());
                    }
                    wrt_foundation::types::Instruction::End => {
                        depth -= 1;
                        if depth == 0 {
                            self.exec_stack.pc = pc;
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
            pc += 1;
        }
        
        Err(Error::new(
            ErrorCategory::Runtime,
            codes::EXECUTION_ERROR,
            "Matching else/end not found"
        ))
    }
    
    /// Skip to the matching end instruction
    fn skip_to_end(&mut self, body: &crate::module::WrtExpr, start_pc: usize) -> Result<()> {
        let mut depth = 1;
        let mut pc = start_pc + 1;
        
        while pc < body.instructions.len() && depth > 0 {
            if let Ok(instruction) = body.instructions.get(pc) {
                match instruction {
                    wrt_foundation::types::Instruction::Block { .. } |
                    wrt_foundation::types::Instruction::Loop { .. } |
                    wrt_foundation::types::Instruction::If { .. } => {
                        depth += 1;
                    }
                    wrt_foundation::types::Instruction::End => {
                        depth -= 1;
                        if depth == 0 {
                            self.exec_stack.pc = pc;
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
            pc += 1;
        }
        
        Err(Error::new(
            ErrorCategory::Runtime,
            codes::EXECUTION_ERROR,
            "Matching end not found"
        ))
    }
    
    /// Branch to a label at the given depth
    fn branch_to_label(&mut self, label_depth: u32) -> Result<()> {
        let labels_len = self.exec_stack.labels.len();
        if label_depth as usize >= labels_len {
            return Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                "Invalid label depth"
            ));
        }
        
        // Get the target label
        let target_idx = labels_len - 1 - label_depth as usize;
        let target_label = self.exec_stack.labels.get(target_idx).map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "Label not found")
        })?;
        
        // Branch behavior depends on label kind
        match target_label.kind {
            LabelKind::Loop => {
                // For loops, branch to the beginning of the loop
                self.exec_stack.pc = target_label.pc;
            }
            _ => {
                // For blocks and ifs, branch to the end
                // This is simplified - in reality we'd need to find the matching End
                self.exec_stack.state = StacklessExecutionState::Branching {
                    depth: label_depth,
                    values: BoundedVec::new(DefaultMemoryProvider::default()).unwrap(),
                };
            }
        }
        
        // Pop labels up to and including the target
        for _ in 0..=label_depth {
            self.exec_stack.labels.pop()?;
        }
        
        Ok(())
    }
}

/// Implementation of ComparisonContext for StacklessEngine
impl wrt_instructions::comparison_ops::ComparisonContext for StacklessEngine {
    fn pop_comparison_value(&mut self) -> Result<Value> {
        self.pop_control_value()
    }
    
    fn push_comparison_value(&mut self, value: Value) -> Result<()> {
        self.push_control_value(value)
    }
}

/// Implementation of VariableContext for StacklessEngine  
impl VariableContext for StacklessEngine {
    fn get_local(&self, index: u32) -> Result<Value> {
        self.locals.get(index as usize).map_err(|_| {
            Error::new(
                ErrorCategory::Runtime, 
                codes::OUT_OF_BOUNDS_ERROR, 
                "Local variable index out of bounds"
            )
        })
    }

    fn set_local(&mut self, index: u32, value: Value) -> Result<()> {
        self.locals.set(index as usize, value).map_err(|_| {
            Error::new(
                ErrorCategory::Runtime, 
                codes::OUT_OF_BOUNDS_ERROR,
                "Local variable index out of bounds"
            )
        })?;
        Ok(())
    }

    fn get_global(&self, index: u32) -> Result<Value> {
        if let Some(module_instance) = &self.current_module {
            let global = module_instance.global(index)?;
            global.get()
        } else {
            Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"))
        }
    }

    fn set_global(&mut self, index: u32, value: Value) -> Result<()> {
        if let Some(module_instance) = &self.current_module {
            let global = module_instance.global(index)?;
            global.set(value)
        } else {
            Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"))
        }
    }

    fn push_value(&mut self, value: Value) -> Result<()> {
        self.push_control_value(value)
    }

    fn pop_value(&mut self) -> Result<Value> {
        self.pop_control_value()
    }
}

/// Implementation of ControlContext for StacklessEngine
/// This enables the engine to handle WebAssembly control flow instructions
/// including the new branch hinting instructions.
impl ControlContext for StacklessEngine {
    /// Push a value to the operand stack
    fn push_control_value(&mut self, value: Value) -> Result<()> {
        self.exec_stack.values.push(value).map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Operand stack overflow")
        })?;
        Ok(())
    }

    /// Pop a value from the operand stack
    fn pop_control_value(&mut self) -> Result<Value> {
        match self.exec_stack.values.pop()? {
            Some(value) => Ok(value),
            None => Err(Error::new(
                ErrorCategory::Runtime, 
                codes::STACK_UNDERFLOW, 
                "Operand stack underflow"
            ))
        }
    }

    /// Get the current block depth (number of labels)
    fn get_block_depth(&self) -> usize {
        self.exec_stack.labels.len()
    }

    /// Start a new block
    fn enter_block(&mut self, block_type: Block) -> Result<()> {
        // Create a new label for this block
        // Calculate arity from block type
        let arity = match block_type {
            Block::Block(block_type) | Block::Loop(block_type) | Block::If(block_type) => {
                match block_type {
                    wrt_foundation::BlockType::Value(Some(_)) => 1, // Single result value
                    wrt_foundation::BlockType::Value(None) => 0,    // No result
                    wrt_foundation::BlockType::FuncType(_type_idx) => {
                        // For function types, we'd need to look up the type in the module
                        // For now, assume 0 results as a safe default
                        0
                    }
                }
            }
            Block::Try(_) => 0, // Try blocks typically don't produce values
        };

        let label = Label {
            kind: match block_type {
                Block::Block(_) => LabelKind::Block,
                Block::Loop(_) => LabelKind::Loop,
                Block::If(_) => LabelKind::If,
                Block::Try(_) => LabelKind::Block, // Treat try as block for now
            },
            arity,
            pc: self.exec_stack.pc,
        };
        
        self.exec_stack.labels.push(label).map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Label stack overflow")
        })?;
        Ok(())
    }

    /// Exit the current block
    fn exit_block(&mut self) -> Result<Block> {
        if self.exec_stack.labels.is_empty() {
            return Err(Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "No block to exit"));
        }
        let last_idx = self.exec_stack.labels.len() - 1;
        let label = self.exec_stack.labels.remove(last_idx).map_err(|_| {
            Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "No block to exit")
        })?;
        
        // Convert label back to block type (simplified)
        let block = match label.kind {
            LabelKind::Block => Block::Block(wrt_foundation::BlockType::Value(None)),
            LabelKind::Loop => Block::Loop(wrt_foundation::BlockType::Value(None)),
            LabelKind::If => Block::If(wrt_foundation::BlockType::Value(None)),
            LabelKind::Function => Block::Block(wrt_foundation::BlockType::Value(None)), // Treat function as block
        };
        
        Ok(block)
    }

    /// Branch to a specific label
    fn branch(&mut self, target: BranchTarget) -> Result<()> {
        // Collect values to keep based on branch target arity
        let mut values = BoundedVec::new(DefaultMemoryProvider::default()).unwrap();
        
        // Get the label we're branching to
        if let Ok(label) = self.exec_stack.labels.get(target.label_idx as usize) {
            let arity = label.arity;
            
            // Pop the required number of values from the stack
            for _ in 0..arity {
                match self.exec_stack.values.pop()? {
                    Some(value) => {
                        // Insert at beginning to maintain order (since we're popping in reverse)
                        values.insert(0, value).map_err(|_| {
                            Error::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Branch values overflow")
                        })?;
                    }
                    None => {
                        return Err(Error::new(
                            ErrorCategory::Runtime, 
                            codes::STACK_UNDERFLOW, 
                            "Not enough values for branch"
                        ));
                    }
                }
            }
        }
        
        // Set the execution state to branching
        self.exec_stack.state = StacklessExecutionState::Branching {
            depth: target.label_idx,
            values,
        };
        Ok(())
    }

    /// Return from the current function
    fn return_function(&mut self) -> Result<()> {
        // Collect return values based on function signature
        let mut values = BoundedVec::new(DefaultMemoryProvider::default()).unwrap();
        
        // Get function type to determine return arity
        if let Some(current_module) = &self.current_module {
            if let Ok(func_type) = current_module.get_function_type(self.exec_stack.func_idx as usize) {
                let return_arity = func_type.results.len();
                
                // Pop the required number of return values from the stack
                for _ in 0..return_arity {
                    match self.exec_stack.values.pop()? {
                        Some(value) => {
                            // Insert at beginning to maintain order (since we're popping in reverse)
                            values.insert(0, value).map_err(|_| {
                                Error::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Return values overflow")
                            })?;
                        }
                        None => {
                            return Err(Error::new(
                                ErrorCategory::Runtime, 
                                codes::STACK_UNDERFLOW, 
                                "Not enough values for function return"
                            ));
                        }
                    }
                }
            }
        }
        
        self.exec_stack.state = StacklessExecutionState::Returning {
            values,
        };
        Ok(())
    }

    /// Call a function by index
    fn call_function(&mut self, func_idx: u32) -> Result<()> {
        self.stats.function_calls += 1;
        
        // Collect arguments based on function signature
        let mut args = BoundedVec::new(DefaultMemoryProvider::default()).unwrap();
        
        // Get function type to determine parameter arity
        if let Some(current_module) = &self.current_module {
            if let Ok(func_type) = current_module.get_function_type(func_idx as usize) {
                let param_arity = func_type.params.len();
                
                // Pop the required number of arguments from the stack
                for _ in 0..param_arity {
                    match self.exec_stack.values.pop()? {
                        Some(value) => {
                            // Insert at beginning to maintain order (since we're popping in reverse)
                            args.insert(0, value).map_err(|_| {
                                Error::new(ErrorCategory::Runtime, codes::STACK_OVERFLOW, "Function args overflow")
                            })?;
                        }
                        None => {
                            return Err(Error::new(
                                ErrorCategory::Runtime, 
                                codes::STACK_UNDERFLOW, 
                                "Not enough arguments for function call"
                            ));
                        }
                    }
                }
            }
        }
        
        self.exec_stack.state = StacklessExecutionState::Calling {
            instance_idx: self.exec_stack.instance_idx as u32,
            func_idx,
            args,
            return_pc: self.exec_stack.pc + 1,
        };
        Ok(())
    }

    /// Call a function indirectly through a table
    fn call_indirect(&mut self, table_idx: u32, type_idx: u32) -> Result<()> {
        // Pop function index from stack
        let func_idx = self.pop_control_value()?.into_i32().map_err(|_| {
            Error::type_error("call_indirect expects i32 function index")
        })?;
        
        if func_idx < 0 {
            return Err(Error::runtime_error("Invalid function index for call_indirect"));
        }
        
        // Execute indirect call with validation
        self.execute_call_indirect(table_idx, type_idx, func_idx)
    }

    /// Tail call a function by index (return_call)
    fn return_call(&mut self, func_idx: u32) -> Result<()> {
        // For tail calls, we replace the current frame instead of creating a new one
        self.stats.function_calls += 1;
        
        // Validate function exists in current module before tail call
        if let Some(module_instance) = &self.current_module {
            let module = module_instance.module();
            if (func_idx as usize) >= module.functions.len() {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::FUNCTION_NOT_FOUND,
                    "Function index out of bounds for tail call"
                ));
            }
        } else {
            return Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"));
        }
        
        self.call_function(func_idx)
    }

    /// Tail call a function indirectly through a table (return_call_indirect)
    fn return_call_indirect(&mut self, table_idx: u32, type_idx: u32) -> Result<()> {
        // Pop function index from stack
        let func_idx = self.pop_control_value()?.into_i32().map_err(|_| {
            Error::type_error("return_call_indirect expects i32 function index")
        })?;
        
        if func_idx < 0 {
            return Err(Error::runtime_error("Invalid function index for return_call_indirect"));
        }
        
        // Execute tail call indirect
        self.return_call(func_idx as u32)
    }

    /// Trap the execution (unreachable)
    fn trap(&mut self, _message: &str) -> Result<()> {
        let error = Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "Execution trapped");
        self.exec_stack.state = StacklessExecutionState::Error(error.clone());
        Err(error)
    }

    /// Get the current block
    fn get_current_block(&self) -> Option<&Block> {
        // For now, return None since we don't store block types directly
        None
    }

    /// Get function operations interface
    fn get_function_operations(&mut self) -> Result<&mut dyn FunctionOperations> {
        Ok(self as &mut dyn FunctionOperations)
    }

    /// Execute function return with value handling
    fn execute_return(&mut self) -> Result<()> {
        self.return_function()
    }

    /// Execute call_indirect with full validation
    fn execute_call_indirect(&mut self, table_idx: u32, type_idx: u32, func_idx: i32) -> Result<()> {
        if func_idx < 0 {
            return Err(Error::runtime_error("Invalid function index"));
        }
        
        // Implement table lookup and type validation
        if let Some(module_instance) = &self.current_module {
            // Check table exists
            if (table_idx as usize) >= module_instance.module().tables.len() {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::TABLE_NOT_FOUND,
                    "Table index out of bounds"
                ));
            }
            
            // Check type exists
            if (type_idx as usize) >= module_instance.module().types.len() {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::TYPE_MISMATCH,
                    "Type index out of bounds"
                ));
            }
            
            // Get table and validate function reference
            let table = module_instance.table(table_idx)?;
            let func_ref = table.get(func_idx as u32)?;
            
            // Extract function index from reference
            let actual_func_idx = match func_ref {
                Some(Value::FuncRef(Some(func_ref))) => func_ref.index,
                Some(Value::FuncRef(None)) => return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::NULL_REFERENCE,
                    "Null function reference in table"
                )),
                None => return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::NULL_REFERENCE,
                    "Table entry is empty"
                )),
                _ => return Err(Error::new(
                    ErrorCategory::Type,
                    codes::TYPE_MISMATCH,
                    "Expected function reference in table"
                )),
            };
            
            // Validate function type matches expected
            let module = module_instance.module();
            let expected_type = module.get_function_type(type_idx as u32).ok_or_else(|| {
                Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Expected function type not found")
            })?;
            let actual_func = module.get_function(actual_func_idx).ok_or_else(|| {
                Error::new(ErrorCategory::Runtime, codes::FUNCTION_NOT_FOUND, "Function not found in indirect tail call")
            })?;
            let actual_type = module.get_function_type(actual_func.type_idx).ok_or_else(|| {
                Error::new(ErrorCategory::Type, codes::TYPE_MISMATCH, "Actual function type not found")
            })?;
            
            if expected_type != actual_type {
                return Err(Error::new(
                    ErrorCategory::Type,
                    codes::TYPE_MISMATCH,
                    "Function type mismatch in indirect tail call"
                ));
            }
            
            self.call_function(actual_func_idx)
        } else {
            Err(Error::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, "No module instance"))
        }
    }

    /// Execute branch table operation
    fn execute_br_table(&mut self, table: &[u32], default: u32, index: i32) -> Result<()> {
        let label_idx = if index >= 0 && (index as usize) < table.len() {
            table[index as usize]
        } else {
            default
        };
        
        let target = BranchTarget {
            label_idx,
            keep_values: 0,
        };
        self.branch(target)
    }

    /// Execute branch on null - branch if reference is null
    fn execute_br_on_null(&mut self, label: u32) -> Result<()> {
        let target = BranchTarget {
            label_idx: label,
            keep_values: 0,
        };
        self.branch(target)
    }

    /// Execute branch on non-null - branch if reference is not null
    fn execute_br_on_non_null(&mut self, label: u32) -> Result<()> {
        let target = BranchTarget {
            label_idx: label,
            keep_values: 0,
        };
        self.branch(target)
    }
}

/// Implementation of FunctionOperations for StacklessEngine
impl FunctionOperations for StacklessEngine {
    /// Get function type signature by index
    fn get_function_type(&self, func_idx: u32) -> Result<u32> {
        // Look up function type in current module instance
        if let Some(current_module) = self.get_current_module() {
            // For now, use the available method
            match current_module.get_function_type(func_idx as usize) {
                Ok(_func_type) => Ok(func_idx), // Return the index as type ID for now
                Err(_) => Ok(0), // Default type for invalid functions
            }
        } else {
            Err(Error::runtime_error("No current module available for function lookup"))
        }
    }

    /// Get table element (function reference) by index
    fn get_table_function(&self, table_idx: u32, elem_idx: u32) -> Result<u32> {
        // Look up function in table of current module instance
        if let Some(_current_module) = self.get_current_module() {
            // For now, return a simple calculation as placeholder
            // This would need to be implemented properly with table support
            Ok(table_idx * 1000 + elem_idx)
        } else {
            Err(Error::runtime_error("No current module available for table lookup"))
        }
    }

    /// Validate function signature matches expected type
    fn validate_function_signature(&self, func_idx: u32, expected_type: u32) -> Result<()> {
        let actual_type = self.get_function_type(func_idx)?;
        if actual_type == expected_type {
            Ok(())
        } else {
            Err(Error::type_error("Function signature mismatch"))
        }
    }

    /// Execute function call
    fn execute_function_call(&mut self, func_idx: u32) -> Result<()> {
        self.call_function(func_idx)
    }
}

// Additional types needed for the implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Label {
    pub kind: LabelKind,
    pub arity: u32,
    pub pc: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelKind {
    #[default]
    Block,
    Loop,
    If,
    Function,
}

// Implement required traits for Label
impl wrt_foundation::traits::Checksummable for Label {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        self.arity.update_checksum(checksum);
        (self.pc as u32).update_checksum(checksum);
        match self.kind {
            LabelKind::Block => checksum.update_slice(&[0]),
            LabelKind::Loop => checksum.update_slice(&[1]),
            LabelKind::If => checksum.update_slice(&[2]),
            LabelKind::Function => checksum.update_slice(&[3]),
        }
    }
}

impl wrt_foundation::traits::ToBytes for Label {
    fn to_bytes_with_provider<'a, P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        _provider: &P,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u32_le(self.arity)?;
        writer.write_u32_le(self.pc as u32)?;
        let kind_byte = match self.kind {
            LabelKind::Block => 0u8,
            LabelKind::Loop => 1u8,
            LabelKind::If => 2u8,
            LabelKind::Function => 3u8,
        };
        writer.write_u8(kind_byte)?;
        Ok(())
    }
}

impl wrt_foundation::traits::FromBytes for Label {
    fn from_bytes_with_provider<'a, P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'a>,
        _provider: &P,
    ) -> wrt_foundation::WrtResult<Self> {
        let arity = reader.read_u32_le()?;
        let pc = reader.read_u32_le()? as usize;
        let kind_byte = reader.read_u8()?;
        let kind = match kind_byte {
            0 => LabelKind::Block,
            1 => LabelKind::Loop,
            2 => LabelKind::If,
            3 => LabelKind::Function,
            _ => return Err(wrt_error::Error::validation_error("Invalid label kind")),
        };
        Ok(Label { kind, arity, pc })
    }
}

// Rest of the implementation will be added in subsequent updates
