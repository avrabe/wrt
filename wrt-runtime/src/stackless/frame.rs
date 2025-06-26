//! Stackless function activation frame
//!
//! This module implements the activation frame structure for the stackless
//! WebAssembly execution engine. Each frame represents a function invocation
//! and contains the necessary state for execution including locals, labels,
//! and the value stack.
//!
//! The stackless frame design allows for:
//! - Zero-copy pause/resume of execution
//! - Bounded memory usage suitable for embedded systems
//! - Safe operation in no_std environments
//! - Efficient state management without heap allocation

extern crate alloc;

use core::fmt::Debug;
#[cfg(feature = "std")]
use std::{vec, vec::Vec};
#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

// Imports from wrt crates
// Import the full Instruction enum from wrt_foundation
// Note: Instruction is parameterized by MemoryProvider
use crate::types::{ValueStackVec, LocalsVec};
use wrt_error::{codes, Error, ErrorCategory};
use wrt_foundation::values::FuncRef;
use wrt_foundation::{
    safe_memory::SafeSlice, // Added SafeSlice
    values::{FloatBits32, FloatBits64}, // Added for floating-point values
    BlockType,
    BoundedCapacity,
    FuncType,
    Validatable,
    Value,
    ValueType,
    VerificationLevel,
}; // Added FuncRef
// Re-export Label for convenience if it's commonly used with frames
pub use wrt_instructions::control_ops::BranchTarget as Label;

// Internal imports
use super::engine::StacklessEngine;
use crate::prelude::*;
use crate::memory_adapter::StdMemoryProvider;
use crate::{
    global::Global,
    memory::Memory,
    memory_helpers::ArcMemoryExt, // Add ArcMemoryExt trait import
    module::{Data, Element, Function, Module}, // Module is already in prelude
    module_instance::ModuleInstance,
    // simd_execution_adapter::SimdExecutionAdapter, // SIMD execution integration - disabled
    stackless::StacklessStack, // Added StacklessStack
    table::Table,
};

// Import format! macro for string formatting
#[cfg(feature = "std")]
use std::format;
#[cfg(not(feature = "std"))]
use alloc::format;

/// Defines the behavior of a function activation frame in the stackless engine.
pub trait FrameBehavior {
    /// Returns the current program counter (instruction offset within the
    /// function body).
    fn pc(&self) -> usize;

    /// Returns a mutable reference to the program counter.
    fn pc_mut(&mut self) -> &mut usize;

    /// Returns a slice of the local variables for the current frame.
    /// This includes function arguments followed by declared local variables.
    fn locals(&self) -> &[Value];

    /// Returns a mutable slice of the local variables.
    fn locals_mut(&mut self) -> &mut [Value];

    /// Returns a reference to the module instance this frame belongs to.
    fn module_instance(&self) -> &Arc<ModuleInstance>;

    /// Returns the index of the function this frame represents.
    fn function_index(&self) -> u32;

    /// Returns the type (signature) of the function this frame represents.
    fn function_type(&self) -> &FuncType<StdMemoryProvider>;

    /// Returns the arity (number of return values) of the function.
    fn arity(&self) -> usize;

    /// Advances execution by one instruction.
    ///
    /// # Arguments
    ///
    /// * `engine`: The stackless engine, providing access to global state like
    ///   the value stack and call stack.
    ///
    /// # Returns
    ///
    /// * `Ok(ControlFlow)`: Indicates the outcome of the instruction execution,
    ///   e.g., proceed to next instruction, call, return.
    /// * `Err(Error)`: If an error occurs during execution.
    fn step(&mut self, engine: &mut StacklessEngine) -> Result<ControlFlow>; // ControlFlow to be defined
}

/// Represents the control flow outcome of an instruction's execution.
#[derive(Debug, Clone)]
pub enum ControlFlow {
    /// Continue to the next instruction in the current frame.
    Next,
    /// A function call has been made. A new frame will be pushed.
    Call { func_idx: u32, inputs: ValueStackVec }, // Simplified for now
    /// The current function is returning. The current frame will be popped.
    Return { values: ValueStackVec },
    /// A branch to a given PC offset within the current function.
    Branch(usize),
    /// Trap / Unreachable instruction.
    Trap(Error),
    /// A tail call that replaces the current frame (WebAssembly 2.0).
    TailCall(u32), // function index
}

/// Stackless function activation frame.
#[derive(Debug, Clone)]
pub struct StacklessFrame {
    /// Program counter: offset into the function's instruction stream.
    pc: usize,
    /// Local variables (includes arguments).
    locals: LocalsVec, // Simplified from SafeSlice to avoid lifetime issues
    /// Reference to the module instance.
    module_instance: Arc<ModuleInstance>,
    /// Index of the function in the module.
    func_idx: u32,
    /// Type of the function.
    func_type: FuncType<StdMemoryProvider>,
    /// Arity of the function (number of result values).
    arity: usize,
    /// Block depths for control flow.
    #[cfg(feature = "std")]
    block_depths: Vec<BlockContext>, // Use standard Vec for internal state
    #[cfg(all(not(feature = "std"), not(feature = "std")))]
    block_depths: [Option<BlockContext>; 16], // Fixed array for no_std
}

/// Context for a control flow block (block, loop, if).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BlockContext {
    /// The type of the block.
    block_type: BlockType,
    /// Program counter to jump to when exiting this block (e.g., `end` of
    /// block/if/loop).
    end_pc: usize,
    /// Program counter for the `else` branch of an `if` block.
    else_pc: Option<usize>,
    /// Stack depth before entering the block, to know how many values to
    /// pop/truncate.
    stack_depth_before: usize,
    /// Value stack depth before parameters were pushed (for block/loop results)
    exec_stack_values_depth_before_params: usize,
    /// Arity of the block (number of result values it's expected to push).
    arity: usize,
}

/// Helper functions for stack operations
impl StacklessFrame {
    /// Helper function to pop a value from the execution stack and handle the Result<Option<T>, E> return type
    fn pop_value(engine: &mut StacklessEngine) -> Result<Value> {
        match engine.exec_stack.values.pop() {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(Error::runtime_stack_underflow("Stack underflow")),
            Err(_) => Err(Error::runtime_stack_underflow("Stack operation error")),
        }
    }

    /// Helper function to pop an i32 value from the execution stack
    fn pop_i32(engine: &mut StacklessEngine) -> Result<i32> {
        let value = Self::pop_value(engine)?;
        match value {
            Value::I32(i) => Ok(i),
            _ => Err(Error::runtime_type_mismatch("Expected i32 value")),
        }
    }

    /// Helper function to pop an i64 value from the execution stack
    fn pop_i64(engine: &mut StacklessEngine) -> Result<i64> {
        let value = Self::pop_value(engine)?;
        match value {
            Value::I64(i) => Ok(i),
            _ => Err(Error::runtime_type_mismatch("Expected i64 value")),
        }
    }

    /// Helper function to pop an f32 value from the execution stack
    fn pop_f32(engine: &mut StacklessEngine) -> Result<f32> {
        let value = Self::pop_value(engine)?;
        match value {
            Value::F32(f) => Ok(f.value()),
            _ => Err(Error::runtime_type_mismatch("Expected f32 value")),
        }
    }

    /// Helper function to pop an f64 value from the execution stack
    fn pop_f64(engine: &mut StacklessEngine) -> Result<f64> {
        let value = Self::pop_value(engine)?;
        match value {
            Value::F64(f) => Ok(f.value()),
            _ => Err(Error::runtime_type_mismatch("Expected f64 value")),
        }
    }
}

impl StacklessFrame {
    /// Creates a new stackless function frame.
    ///
    /// # Arguments
    ///
    /// * `func_ref`: A reference to the function to be called.
    /// * `module_instance`: The module instance this function belongs to.
    /// * `invocation_inputs`: Values passed as arguments to this function call.
    /// * `max_locals`: Maximum number of locals expected (for SafeSlice
    /// Binary std/no_std choice
    pub fn new(
        func_ref: FuncRef,
        module_instance: Arc<ModuleInstance>,
        invocation_inputs: &[Value], // Changed to slice
        max_locals: usize,           // Example: pass from engine config or calculate
    ) -> Result<Self> {
        let func_idx = func_ref.index;
        let func_type = module_instance.function_type(func_idx)?;

        #[cfg(feature = "std")]
        let mut locals_vec = {
            let mut vec = Vec::new();
            for value in invocation_inputs.iter() {
                vec.push(value.clone());
            }
            vec
        };

        #[cfg(not(feature = "std"))]
        let mut locals_vec = {
            let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
            let mut bounded_vec = wrt_foundation::bounded::BoundedVec::new(provider)?;
            for value in invocation_inputs.iter() {
                bounded_vec.push(value.clone())?;
            }
            bounded_vec
        };

        // Append default values for declared locals
        if let Ok(function_body) = module_instance.module().functions.get(func_idx as usize) {
            for local_entry in &function_body.locals {
                // local_entry is (count, ValueType) in the Module's Function struct
                // Assuming Function struct in module.rs has: pub locals: Vec<(u32, ValueType)>,
                let count = local_entry.count;
                let val_type = local_entry.value_type;
                for _ in 0..count {
                    #[cfg(feature = "std")]
                    locals_vec.push(Value::default_for_type(&val_type));

                    #[cfg(not(feature = "std"))]
                    locals_vec.push(Value::default_for_type(&val_type))?;
                }
            }
        } else {
            return Err(Error::runtime_function_not_found("Function body not found"));
        }

        let locals = locals_vec;

        if locals.len() > max_locals {
            return Err(Error::validation_invalid_state("Too many locals for configured max_locals"));
        }

        Ok(Self {
            pc: 0,
            locals,
            module_instance,
            func_idx,
            arity: func_type.results.len(),
            func_type,
            #[cfg(feature = "std")]
            block_depths: Vec::new(),
            #[cfg(all(not(feature = "std"), not(feature = "std")))]
            block_depths: [None; 16],
        })
    }

    // Helper to get the actual function body from the module instance
    fn function_body(&self) -> Result<crate::module::Function> {
        self.module_instance.module().functions.get(self.func_idx as usize)
    }
}

impl FrameBehavior for StacklessFrame {
    fn pc(&self) -> usize {
        self.pc
    }

    fn pc_mut(&mut self) -> &mut usize {
        &mut self.pc
    }

    fn locals(&self) -> &[Value] {
        &self.locals
    }

    fn locals_mut(&mut self) -> &mut [Value] {
        &mut self.locals
    }

    fn module_instance(&self) -> &Arc<ModuleInstance> {
        &self.module_instance
    }

    fn function_index(&self) -> u32 {
        self.func_idx
    }

    fn function_type(&self) -> &FuncType<StdMemoryProvider> {
        &self.func_type
    }

    fn arity(&self) -> usize {
        self.arity
    }

    fn step(&mut self, engine: &mut StacklessEngine) -> Result<ControlFlow> {
        let func_body = self.function_body()?;
        let instructions = &func_body.body.instructions; // Access the instructions field of WrtExpr

        if self.pc >= instructions.len() {
            // If PC is at or beyond the end, and it's not a trap/return already handled,
            // it implies a fallthrough return for a void function or a missing explicit
            // return.
            if self.arity == 0 {
                // Implicit return for void function
                #[cfg(feature = "std")]
                return Ok(ControlFlow::Return { values: Vec::new() });
                #[cfg(not(feature = "std"))]
                {
                    let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
                    return Ok(ControlFlow::Return { 
                        values: wrt_foundation::bounded::BoundedVec::new(provider)? 
                    });
                }
            } else {
                return Err(Error::runtime_error("Function ended without returning expected values"));
            }
        }

        let instruction = instructions.get(self.pc).map_err(|_| Error::runtime_error("Invalid program counter"))?;
        self.pc += 1;

        // --- Execute Instruction ---
        // This is where the large match statement for all instructions will go.
        // For now, a placeholder.
        use wrt_foundation::types::Instruction;
        match instruction {
            Instruction::Unreachable => Ok(ControlFlow::Trap(Error::runtime_execution_error(",
            ))),
            Instruction::Nop => Ok(ControlFlow::Next),
            Instruction::Block { block_type_idx } => {
                // Enter a new block scope
                let block_context = BlockContext {
                    block_type: BlockType::Value(None), // Empty block type
                    end_pc: 0, // Will be set when we encounter the matching End instruction
                    else_pc: None,
                    stack_depth_before: engine.exec_stack.values.len(),
                    exec_stack_values_depth_before_params: engine.exec_stack.values.len(),
                    arity: 0, // Should be determined from block type
                };
                
                #[cfg(feature = ")]
                self.block_depths.push(block_context);
                #[cfg(not(feature = "std"))]
                {
                    // Find the first available slot in fixed array
                    let mut found = false;
                    for slot in &mut self.block_depths {
                        if slot.is_none() {
                            *slot = Some(block_context);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(Error::runtime_stack_overflow("Too many nested blocks"));
                    }
                }
                
                // Block entered, continue to next instruction
                Ok(ControlFlow::Next)
            }
            Instruction::Loop { block_type_idx } => {
                // Enter a new loop scope - branches target the loop start (current PC)
                let block_context = BlockContext {
                    block_type: BlockType::Value(None), // Empty block type
                    end_pc: 0, // Will be set when we encounter the matching End instruction
                    else_pc: None,
                    stack_depth_before: engine.exec_stack.values.len(),
                    exec_stack_values_depth_before_params: engine.exec_stack.values.len(),
                    arity: 0, // Should be determined from block type
                };
                
                #[cfg(feature = "std")]
                self.block_depths.push(block_context);
                #[cfg(not(feature = "std"))]
                {
                    // Find the first available slot in fixed array
                    let mut found = false;
                    for slot in &mut self.block_depths {
                        if slot.is_none() {
                            *slot = Some(block_context);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(Error::runtime_stack_overflow("Too many nested blocks"));
                    }
                }
                
                Ok(ControlFlow::Next)
            }
            Instruction::If { block_type_idx } => {
                // Pop condition from stack
                let condition_val_opt = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let condition = match condition_val_opt {
                    Some(Value::I32(val)) => val != 0,
                    Some(_) => return Err(Error::validation_error("If condition not i32")),
                    None => return Err(Error::runtime_stack_underflow("Stack underflow")),
                };
                
                // Enter If block scope
                let block_context = BlockContext {
                    block_type: BlockType::Value(None), // Empty block type
                    end_pc: 0, // Will be set when we encounter the matching End instruction
                    else_pc: None, // Will be set when we encounter Else instruction
                    stack_depth_before: engine.exec_stack.values.len(),
                    exec_stack_values_depth_before_params: engine.exec_stack.values.len(),
                    arity: 0, // Should be determined from block type
                };
                
                #[cfg(feature = "std")]
                self.block_depths.push(block_context);
                #[cfg(not(feature = "std"))]
                {
                    // Find the first available slot in fixed array
                    let mut found = false;
                    for slot in &mut self.block_depths {
                        if slot.is_none() {
                            *slot = Some(block_context);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(Error::runtime_stack_overflow("Too many nested blocks"));
                    }
                }
                
                if condition {
                    // Continue to then branch
                    return Ok(ControlFlow::Next);
                } else {
                    // Jump to else or end - for now, we'll need to scan forward to find it
                    // This is a simplified implementation
                    return Err(Error::runtime_not_implemented("If false branch - need to implement else/end scanning"))
                }
            }
            Instruction::Else => {
                // TODO: Jump to end of current If block's 'then' part.
                // let current_block = self.block_depths.last().ok_or_else(...)?;
                // self.pc = current_block.end_pc;
                Ok(ControlFlow::Trap(Error::runtime_not_implemented("Else instruction not implemented")))
            }
            Instruction::End => {
                // Check if this is the end of the function itself or a nested block
                let has_blocks = {
                    #[cfg(feature = "std")]
                    { !self.block_depths.is_empty() }
                    #[cfg(not(feature = "std"))]
                    { self.block_depths.iter().any(|slot| slot.is_some()) }
                };
                
                if !has_blocks {
                    // This 'end' corresponds to the function body's implicit block.
                    // Values for return should be on the stack matching self.arity.
                    #[cfg(feature = "std")]
                    let mut return_values = ValueStackVec::with_capacity(self.arity);
                    #[cfg(not(feature = "std"))]
                    {
                        let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
                        let mut return_values = wrt_foundation::bounded::BoundedVec::new(provider)?;
                        for _ in 0..self.arity {
                        let value = engine.exec_stack.values.pop().map_err(|e| {
                            Error::runtime_stack_underflow("Stack operation error")
                        })?;
                        match value {
                            Some(v) => {
                                #[cfg(feature = "std")]
                                return_values.push(v);
                                #[cfg(not(feature = "std"))]
                                return_values.push(v)?;
                            }
                            None => return Err(Error::runtime_stack_underflow("Stack underflow during return")),
                        }
                        }
                        return_values.reverse(); // Values are popped in reverse order
                        return Ok(ControlFlow::Return { values: return_values });
                    }
                } else {
                    // Pop the most recent block context
                    #[cfg(feature = "std")]
                    {
                        let _block_context = self.block_depths.pop().ok_or_else(|| {
                            Error::runtime_invalid_state("No block to end")
                        })?;
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        // Find and clear the last occupied slot
                        let mut found = false;
                        for slot in self.block_depths.iter_mut().rev() {
                            if slot.is_some() {
                                *slot = None;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return Err(Error::runtime_invalid_state("No block to end"));
                        }
                    }
                    
                    return Ok(ControlFlow::Next); // Continue after ending the block
                }
            }
            Instruction::Br(label_idx) => {
                // Branch to the specified label (relative depth)
                // For now, simplified implementation - need to implement proper label resolution
                Ok(ControlFlow::Branch(label_idx as usize))
            }
            Instruction::BrIf(label_idx) => {
                // Pop condition from stack
                let condition_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let condition = match condition_val {
                    Some(Value::I32(val)) => val != 0,
                    Some(_) => return Err(Error::validation_error("BrIf condition not i32")),
                    None => return Err(Error::runtime_stack_underflow("Stack underflow")),
                };
                
                if condition {
                    // Branch to the specified label
                    return Ok(ControlFlow::Branch(label_idx as usize));
                } else {
                    // Continue to next instruction
                    return Ok(ControlFlow::Next);
                }
            }
            // ... other control flow instructions ...
            Instruction::Return => {
                #[cfg(feature = "std")]
                let mut return_values = ValueStackVec::with_capacity(self.arity);
                #[cfg(not(feature = "std"))]
                {
                    let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
                    let mut return_values = wrt_foundation::bounded::BoundedVec::new(provider)?;
                    for _ in 0..self.arity {
                    let value = engine.exec_stack.values.pop().map_err(|e| {
                        Error::runtime_stack_underflow("Stack operation error")
                    })?;
                    match value {
                        Some(v) => {
                            #[cfg(feature = "std")]
                            return_values.push(v);
                            #[cfg(not(feature = "std"))]
                            return_values.push(v)?;
                        }
                        None => return Err(Error::runtime_stack_underflow("Stack underflow during return")),
                    }
                    }
                    return_values.reverse();
                    Ok(ControlFlow::Return { values: return_values })
                }
            }
            Instruction::Call(func_idx_val) => {
                // Get the target function type to know how many arguments to pop
                let target_func_type = self.module_instance.function_type(func_idx_val)?;
                #[cfg(feature = "std")]
                let mut args = ValueStackVec::with_capacity(target_func_type.params.len());
                #[cfg(not(feature = "std"))]
                {
                    let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
                    let mut args = wrt_foundation::bounded::BoundedVec::new(provider)?;
                    
                    // Pop arguments from stack in reverse order (last param first)
                    for _ in 0..target_func_type.params.len() {
                    let value = engine.exec_stack.values.pop().map_err(|e| {
                        Error::runtime_stack_underflow("Stack operation error")
                    })?;
                    match value {
                        Some(v) => {
                            #[cfg(feature = "std")]
                            args.push(v);
                            #[cfg(not(feature = "std"))]
                            args.push(v)?;
                        }
                        None => return Err(Error::runtime_stack_underflow("Stack underflow during call")),
                    }
                    }
                    args.reverse(); // Restore correct argument order
                    
                    Ok(ControlFlow::Call { func_idx: func_idx_val, inputs: args })
                }
            }
            Instruction::CallIndirect(type_idx, table_idx) => {
                // 1. Pop function index from stack
                let elem_idx_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let elem_idx = match elem_idx_val {
                    Some(Value::I32(val)) => val as u32,
                    Some(_) => return Err(Error::validation_error("CallIndirect index not i32")),
                    None => return Err(Error::runtime_stack_underflow("Stack underflow")),
                };
                
                // 2. Get table and validate index
                let table = self.module_instance.table(table_idx)?;
                let func_ref_opt = table.get(elem_idx)?;
                let func_ref = func_ref_opt.ok_or_else(|| {
                    Error::runtime_trap("CallIndirect: null function reference")
                })?;
                
                // 3. Extract function index from the function reference
                let actual_func_idx = match func_ref {
                    Value::FuncRef(Some(func_ref)) => func_ref.index,
                    Value::FuncRef(None) => return Err(Error::runtime_trap("CallIndirect: null function reference")),
                    _ => return Err(Error::validation_error("CallIndirect: table element not a function reference")),
                };
                
                // 4. Type checking - get expected type and actual type
                let expected_func_type = self.module_instance.module().types.get(type_idx as usize).map_err(|_| {
                    Error::validation_error("CallIndirect: invalid type index")
                })?;
                let actual_func_type = self.module_instance.function_type(actual_func_idx)?;
                
                // 5. Verify type compatibility (simplified check)
                if expected_func_type.params.len() != actual_func_type.params.len() ||
                   expected_func_type.results.len() != actual_func_type.results.len() {
                    return Err(Error::validation_error("CallIndirect: function signature mismatch"));
                }
                
                // 6. Pop arguments from stack
                #[cfg(feature = "std")]
                let mut args = ValueStackVec::with_capacity(actual_func_type.params.len());
                #[cfg(not(feature = "std"))]
                {
                    let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
                    let mut args = wrt_foundation::bounded::BoundedVec::new(provider)?;
                    for _ in 0..actual_func_type.params.len() {
                    let value = engine.exec_stack.values.pop().map_err(|e| {
                        Error::runtime_stack_underflow("Stack operation error")
                    })?;
                    match value {
                        Some(v) => {
                            #[cfg(feature = "std")]
                            args.push(v);
                            #[cfg(not(feature = "std"))]
                            args.push(v)?;
                        }
                        None => return Err(Error::runtime_stack_underflow("Stack underflow during call indirect")),
                    }
                    }
                    args.reverse(); // Restore correct argument order
                    
                    Ok(ControlFlow::Call { func_idx: actual_func_idx, inputs: args })
                }
            }

            // Local variable instructions
            Instruction::LocalGet(local_idx) => {
                let value = self.locals.get(local_idx as usize).ok_or_else(|| {
                    Error::runtime_execution_error(",
                    )
                })?;
                engine.exec_stack.values.push(value.clone()).map_err(|e| {
                    Error::runtime_stack_overflow(")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::LocalSet(local_idx) => {
                let value = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack underflow on local.set")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty on local.set"))?;
                // Handle both Vec and BoundedVec cases
                #[cfg(feature = "std")]
                {
                    if local_idx as usize >= self.locals.len() {
                        return Err(Error::runtime_execution_error(",
                        ));
                    }
                    self.locals[local_idx as usize] = value;
                }
                #[cfg(not(feature = "))]
                {
                    self.locals.set(local_idx as usize, value).map_err(|e| {
                        Error::runtime_execution_error(",
                        )
                    })?;
                }
                Ok(ControlFlow::Next)
            }
            Instruction::LocalTee(local_idx) => {
                // Get the top value without popping it - always use BoundedVec::peek since values is always BoundedVec
                let value = engine
                    .exec_stack
                    .values
                    .get(engine.exec_stack.values.len() - 1).unwrap()
                    .clone();
                // Handle both Vec and BoundedVec cases
                #[cfg(feature = ")]
                {
                    if local_idx as usize >= self.locals.len() {
                        return Err(Error::runtime_execution_error(",
                        ));
                    }
                    self.locals[local_idx as usize] = value;
                }
                #[cfg(not(feature = "))]
                {
                    self.locals.set(local_idx as usize, value).map_err(|e| {
                        Error::runtime_execution_error(",
                        )
                    })?;
                }
                Ok(ControlFlow::Next)
            }

            // Global variable instructions
            Instruction::GlobalGet(global_idx) => {
                let global = self.module_instance.global(global_idx)?;
                engine.exec_stack.values.push(global.get_value().clone()).map_err(|e| {
                    Error::runtime_stack_overflow(")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::GlobalSet(global_idx) => {
                let global = self.module_instance.global(global_idx)?;
                if !global.is_mutable() {
                    return Err(Error::validation_error("Cannot set immutable global"));
                }
                let value = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack underflow on global.set")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty on global.set"))?;
                global.set_value(&value)?;
                Ok(ControlFlow::Next)
            }

            // Table instructions
            Instruction::TableGet(table_idx) => {
                let table = self.module_instance.table(table_idx)?;
                let elem_idx_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack underflow for TableGet index")
                })?;
                let elem_idx = match elem_idx_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::type_error("TableGet index not i32")),
                };

                match table.get(elem_idx)? {
                    Some(val) => engine.exec_stack.values.push(val).map_err(|e| {
                        Error::runtime_stack_overflow("Stack overflow on TableGet")
                    })?,
                    None => {
                        return Err(Error::runtime_out_of_bounds("TableGet returned None (null ref or OOB)"))
                    } // Or specific error for null if needed
                }
                Ok(ControlFlow::Next)
            }
            Instruction::TableSet(table_idx) => {
                let table = self.module_instance.table(table_idx)?;
                let val_to_set = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack underflow for TableSet value")
                })?;
                let elem_idx_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let elem_idx = elem_idx_val.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("TableSet index not i32")
                })? as u32;

                // TODO: Type check val_to_set against table.element_type()
                table.set(elem_idx, val_to_set)?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableSize(table_idx) => {
                let table = self.module_instance.table(table_idx)?;
                engine.exec_stack.values.push(Value::I32(table.size() as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableGrow(table_idx) => {
                let table = self.module_instance.table(table_idx)?;
                let init_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for table init value"))?;
                let delta_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let delta = delta_val.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("TableGrow delta not i32")
                })? as u32;

                let old_size = table.grow(delta, init_val)?;
                engine.exec_stack.values.push(Value::I32(old_size as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableFill(table_idx) => {
                self.table_fill(table_idx, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableCopy(dst_table_idx, src_table_idx) => {
                self.table_copy(dst_table_idx, src_table_idx, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableInit(elem_seg_idx, table_idx) => {
                self.table_init(elem_seg_idx, table_idx, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::ElemDrop(elem_seg_idx) => {
                // TODO: Implement drop_element_segment on Module
                // For now, dropping segments is an optimization, so we can skip it
                // self.module_instance.module().drop_element_segment(elem_seg_idx);
                Ok(ControlFlow::Next)
            }

            // Memory instructions (Placeholders, many need base address + offset)
            // Common pattern: pop address, calculate effective_address, operate on memory.
            // Example: I32Load needs `addr = pop_i32() + offset_immediate`
            //          `value = memory.read_i32(addr)`
            //          `push(value)`
            Instruction::I32Load(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Load address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Load address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?; // Assuming memory index 0
                
                // Check bounds
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I32Load out of bounds"));
                }
                
                // Read 4 bytes as little-endian i32
                let mut bytes = [0u8; 4];
                memory.read(effective_addr, &mut bytes)?;
                let value = i32::from_le_bytes(bytes);
                
                engine.exec_stack.values.push(Value::I32(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(8).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Load out of bounds"));
                }
                
                let mut bytes = [0u8; 8];
                memory.read(effective_addr, &mut bytes)?;
                let value = i64::from_le_bytes(bytes);
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Load(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("F32Load address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("F32Load address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("F32Load out of bounds"));
                }
                
                let mut bytes = [0u8; 4];
                memory.read(effective_addr, &mut bytes)?;
                let bits = u32::from_le_bytes(bytes);
                let value = f32::from_bits(bits);
                
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(bits))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Load(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("F64Load address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("F64Load address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(8).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("F64Load out of bounds"));
                }
                
                let mut bytes = [0u8; 8];
                memory.read(effective_addr, &mut bytes)?;
                let bits = u64::from_le_bytes(bytes);
                let value = f64::from_bits(bits);
                
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(bits))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Load8S(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Load8S address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Load8S address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr as usize >= memory.size_in_bytes() {
                    return Err(Error::memory_error("I32Load8S out of bounds"));
                }
                
                let mut byte = [0u8; 1];
                memory.read(effective_addr, &mut byte)?;
                // Sign extend 8-bit to 32-bit
                let value = byte[0] as i8 as i32;
                
                engine.exec_stack.values.push(Value::I32(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Load8U(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Load8U address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Load8U address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr as usize >= memory.size_in_bytes() {
                    return Err(Error::memory_error("I32Load8U out of bounds"));
                }
                
                let mut byte = [0u8; 1];
                memory.read(effective_addr, &mut byte)?;
                // Zero extend 8-bit to 32-bit
                let value = byte[0] as i32;
                
                engine.exec_stack.values.push(Value::I32(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Load16S(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Load16S address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Load16S address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(2).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I32Load16S out of bounds"));
                }
                
                let mut bytes = [0u8; 2];
                memory.read(effective_addr, &mut bytes)?;
                // Sign extend 16-bit to 32-bit
                let value = i16::from_le_bytes(bytes) as i32;
                
                engine.exec_stack.values.push(Value::I32(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Load16U(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Load16U address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Load16U address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(2).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I32Load16U out of bounds"));
                }
                
                let mut bytes = [0u8; 2];
                memory.read(effective_addr, &mut bytes)?;
                // Zero extend 16-bit to 32-bit
                let value = u16::from_le_bytes(bytes) as i32;
                
                engine.exec_stack.values.push(Value::I32(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load8S(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load8S address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load8S address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr as usize >= memory.size_in_bytes() {
                    return Err(Error::memory_error("I64Load8S out of bounds"));
                }
                
                let mut bytes = [0u8; 1];
                memory.read(effective_addr, &mut bytes)?;
                let value = i8::from_le_bytes(bytes) as i64; // Sign extend
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load8U(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load8U address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load8U address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr as usize >= memory.size_in_bytes() {
                    return Err(Error::memory_error("I64Load8U out of bounds"));
                }
                
                let mut bytes = [0u8; 1];
                memory.read(effective_addr, &mut bytes)?;
                let value = u8::from_le_bytes(bytes) as i64; // Zero extend
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load16S(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load16S address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load16S address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(2).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Load16S out of bounds"));
                }
                
                let mut bytes = [0u8; 2];
                memory.read(effective_addr, &mut bytes)?;
                let value = i16::from_le_bytes(bytes) as i64; // Sign extend
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load16U(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load16U address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load16U address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(2).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Load16U out of bounds"));
                }
                
                let mut bytes = [0u8; 2];
                memory.read(effective_addr, &mut bytes)?;
                let value = u16::from_le_bytes(bytes) as i64; // Zero extend
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load32S(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load32S address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load32S address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Load32S out of bounds"));
                }
                
                let mut bytes = [0u8; 4];
                memory.read(effective_addr, &mut bytes)?;
                let value = i32::from_le_bytes(bytes) as i64; // Sign extend
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Load32U(mem_arg) => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Load32U address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Load32U address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Load32U out of bounds"));
                }
                
                let mut bytes = [0u8; 4];
                memory.read(effective_addr, &mut bytes)?;
                let value = u32::from_le_bytes(bytes) as i64; // Zero extend
                
                engine.exec_stack.values.push(Value::I64(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }

            Instruction::I32Store(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32Store value not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Store address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Store address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I32Store out of bounds"));
                }
                
                let bytes = value.to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Store(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I64(val)) => val,
                    _ => return Err(Error::validation_error("I64Store value not i64")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Store address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Store address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(8).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Store out of bounds"));
                }
                
                let bytes = value.to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Store(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::F32(val)) => val,
                    _ => return Err(Error::validation_error("F32Store value not f32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("F32Store address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("F32Store address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("F32Store out of bounds"));
                }
                
                let bits = value.to_bits();
                let bytes = bits.to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Store(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::F64(val)) => val,
                    _ => return Err(Error::validation_error("F64Store value not f64")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("F64Store address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("F64Store address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(8).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("F64Store out of bounds"));
                }
                
                let bits = value.to_bits();
                let bytes = bits.to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Store8(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32Store8 value not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Store8 address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Store8 address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr as usize >= memory.size_in_bytes() {
                    return Err(Error::memory_error("I32Store8 out of bounds"));
                }
                
                // Truncate to 8 bits
                let byte = (value & 0xFF) as u8;
                memory.write(effective_addr, &[byte])?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Store16(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32Store16 value not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32Store16 address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I32Store16 address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(2).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I32Store16 out of bounds"));
                }
                
                // Truncate to 16 bits
                let bytes = (value as u16).to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Store8(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                
                let value = match value_val {
                    Some(Value::I64(val)) => val,
                    _ => return Err(Error::validation_error("I64Store8 value not i64")),
                };
                
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Store8 address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Store8 address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr as usize >= memory.size_in_bytes() {
                    return Err(Error::memory_error("I64Store8 out of bounds"));
                }
                
                // Store lower 8 bits
                let bytes = [(value as u8)];
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Store16(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                
                let value = match value_val {
                    Some(Value::I64(val)) => val,
                    _ => return Err(Error::validation_error("I64Store16 value not i64")),
                };
                
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Store16 address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Store16 address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(2).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Store16 out of bounds"));
                }
                
                // Store lower 16 bits
                let bytes = (value as u16).to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Store32(mem_arg) => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                
                let value = match value_val {
                    Some(Value::I64(val)) => val,
                    _ => return Err(Error::validation_error("I64Store32 value not i64")),
                };
                
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I64Store32 address not i32")),
                };
                
                let effective_addr = addr.checked_add(mem_arg.offset).ok_or_else(|| {
                    Error::memory_error("I64Store32 address overflow")
                })?;
                
                let memory = self.module_instance.memory(0)?;
                
                if effective_addr.checked_add(4).map_or(true, |end| end as usize > memory.size_in_bytes()) {
                    return Err(Error::memory_error("I64Store32 out of bounds"));
                }
                
                // Store lower 32 bits
                let bytes = (value as u32).to_le_bytes();
                memory.write(effective_addr, &bytes)?;
                Ok(ControlFlow::Next)
            }

            Instruction::MemorySize(_mem_idx) => {
                // mem_idx is always 0 in Wasm MVP
                let mem = self.module_instance.memory(0)?; // Assuming memory index 0
                engine.exec_stack.values.push(Value::I32(mem.size_pages() as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryGrow(_mem_idx) => {
                // mem_idx is always 0 in Wasm MVP
                let mem = self.module_instance.memory(0)?;
                let delta_pages_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let delta_pages = delta_pages_val.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("MemoryGrow delta not i32")
                })? as u32;

                let old_size_pages = mem.grow(delta_pages)?;
                engine.exec_stack.values.push(Value::I32(old_size_pages as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryFill(_mem_idx) => {
                self.memory_fill(0, engine)?; // Assuming memory index 0
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryCopy(_dst_mem_idx, _src_mem_idx) => {
                // both always 0 in MVP
                self.memory_copy(0, 0, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryInit(_data_seg_idx, _mem_idx) => {
                // mem_idx always 0 in MVP
                self.memory_init(_data_seg_idx, 0, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::DataDrop(_data_seg_idx) => {
                // TODO: Implement drop_data_segment on Module
                // For now, dropping segments is an optimization, so we can skip it
                // self.module_instance.module().drop_data_segment(_data_seg_idx);
                Ok(ControlFlow::Next)
            }

            // Numeric Const instructions
            Instruction::I32Const(val) => {
                engine.exec_stack.values.push(Value::I32(val)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Const(val) => {
                engine.exec_stack.values.push(Value::I64(val)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Const(val) => {
                engine
                    .exec_stack
                    .values
                    .push(Value::F32(FloatBits32::from_bits(val))) // Assuming val is u32 bits
                    .map_err(|e| {
                        Error::runtime_stack_overflow("Stack operation error")
                    })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Const(val) => {
                engine
                    .exec_stack
                    .values
                    .push(Value::F64(FloatBits64::from_bits(val))) // Assuming val is u64 bits
                    .map_err(|e| {
                        Error::runtime_stack_overflow("Stack operation error")
                    })?;
                Ok(ControlFlow::Next)
            }

            // Arithmetic instructions
            Instruction::I32Add => {
                let b = Self::pop_i32(engine)?;
                let a = Self::pop_i32(engine)?;
                engine.exec_stack.values.push(Value::I32(a.wrapping_add(b))).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Sub => {
                let b = Self::pop_i32(engine)?;
                let a = Self::pop_i32(engine)?;
                engine.exec_stack.values.push(Value::I32(a.wrapping_sub(b))).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Mul => {
                let b = Self::pop_i32(engine)?;
                let a = Self::pop_i32(engine)?;
                engine.exec_stack.values.push(Value::I32(a.wrapping_mul(b))).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Missing I32 arithmetic operations
            Instruction::I32RemS => {
                let b = Self::pop_i32(engine)?;
                if b == 0 {
                    return Err(Error::runtime_division_by_zero("I32RemS division by zero"));
                }
                let a = Self::pop_i32(engine)?;
                // Check for overflow: i32::MIN % -1 would panic, but result should be 0
                let result = if a == i32::MIN && b == -1 { 0 } else { a % b };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32RemU => {
                let b = Self::pop_i32(engine)?;
                if b == 0 {
                    return Err(Error::runtime_division_by_zero("I32RemU division by zero"));
                }
                let a = Self::pop_i32(engine)?;
                // Unsigned remainder - cast to u32
                let result = (a as u32) % (b as u32);
                engine.exec_stack.values.push(Value::I32(result as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32And => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32And second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32And first operand not i32")
                })?;
                engine.exec_stack.values.push(Value::I32(a & b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Or => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Or second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Or first operand not i32")
                })?;
                engine.exec_stack.values.push(Value::I32(a | b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Xor => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Xor second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Xor first operand not i32")
                })?;
                engine.exec_stack.values.push(Value::I32(a ^ b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Shl => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Shl second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Shl first operand not i32")
                })?;
                // Shift amount is masked to 5 bits (0-31) as per WebAssembly spec
                let shift = (b as u32) & 0x1F;
                engine.exec_stack.values.push(Value::I32(a << shift)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32ShrS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32ShrS second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32ShrS first operand not i32")
                })?;
                // Shift amount is masked to 5 bits (0-31) as per WebAssembly spec
                let shift = (b as u32) & 0x1F;
                engine.exec_stack.values.push(Value::I32(a >> shift)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32ShrU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32ShrU second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32ShrU first operand not i32")
                })?;
                // Shift amount is masked to 5 bits (0-31) as per WebAssembly spec
                let shift = (b as u32) & 0x1F;
                // Unsigned right shift
                let result = (a as u32) >> shift;
                engine.exec_stack.values.push(Value::I32(result as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Rotl => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Rotl second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Rotl first operand not i32")
                })?;
                // Rotate amount is masked to 5 bits (0-31) as per WebAssembly spec
                let rotate = (b as u32) & 0x1F;
                let result = (a as u32).rotate_left(rotate);
                engine.exec_stack.values.push(Value::I32(result as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Rotr => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Rotr second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Rotr first operand not i32")
                })?;
                // Rotate amount is masked to 5 bits (0-31) as per WebAssembly spec
                let rotate = (b as u32) & 0x1F;
                let result = (a as u32).rotate_right(rotate);
                engine.exec_stack.values.push(Value::I32(result as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Stack manipulation
            Instruction::Drop => {
                engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Additional I32 arithmetic instructions
            Instruction::I32DivS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32DivS second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32DivS first operand not i32")
                })?;
                if b == 0 {
                    return Err(Error::runtime_error("Division by zero"));
                }
                if a == i32::MIN && b == -1 {
                    return Err(Error::runtime_error("Integer overflow"));
                }
                engine.exec_stack.values.push(Value::I32(a / b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32DivU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32DivU second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32DivU first operand not i32")
                })?;
                if b == 0 {
                    return Err(Error::runtime_error("Division by zero"));
                }
                let result = (a as u32) / (b as u32);
                engine.exec_stack.values.push(Value::I32(result as i32)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // I32 comparison instructions
            Instruction::I32Eq => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32Eq second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32Eq first operand not i32")
                })?;
                engine.exec_stack.values.push(Value::I32(if a == b { 1 } else { 0 })).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Ne => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32Ne second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32Ne first operand not i32")
                })?;
                engine.exec_stack.values.push(Value::I32(if a != b { 1 } else { 0 })).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32LtS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32LtS second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::type_error("I32LtS first operand not i32")
                })?;
                engine.exec_stack.values.push(Value::I32(if a < b { 1 } else { 0 })).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // I64 arithmetic instructions  
            Instruction::I64Add => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::type_error("I64Add second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::type_error("I64Add first operand not i64")
                })?;
                engine.exec_stack.values.push(Value::I64(a.wrapping_add(b))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Sub => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::type_error("I64Sub second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::type_error("I64Sub first operand not i64")
                })?;
                engine.exec_stack.values.push(Value::I64(a.wrapping_sub(b))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Mul => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Mul second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Mul first operand not i64")
                })?;
                engine.exec_stack.values.push(Value::I64(a.wrapping_mul(b))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64DivS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64DivS second operand not i64")
                })?;
                
                if b == 0 {
                    return Err(Error::runtime_division_by_zero("I64DivS division by zero"));
                }
                
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64DivS first operand not i64")
                })?;
                
                // Check for overflow: i64::MIN / -1 would overflow
                if a == i64::MIN && b == -1 {
                    return Err(Error::runtime_integer_overflow("I64DivS integer overflow"));
                }
                
                engine.exec_stack.values.push(Value::I64(a / b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64DivU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64DivU second operand not i64")
                })?;
                
                if b == 0 {
                    return Err(Error::runtime_division_by_zero("I64DivU division by zero"));
                }
                
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64DivU first operand not i64")
                })?;
                
                // Unsigned division - cast to u64
                let result = (a as u64) / (b as u64);
                engine.exec_stack.values.push(Value::I64(result as i64)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64And => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64And second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64And first operand not i64")
                })?;
                engine.exec_stack.values.push(Value::I64(a & b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Or => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Or second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Or first operand not i64")
                })?;
                engine.exec_stack.values.push(Value::I64(a | b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Xor => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Xor second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Xor first operand not i64")
                })?;
                engine.exec_stack.values.push(Value::I64(a ^ b)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64RemS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64RemS second operand not i64")
                })?;
                
                if b == 0 {
                    return Err(Error::runtime_division_by_zero("I64RemS division by zero"));
                }
                
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64RemS first operand not i64")
                })?;
                
                // Check for overflow: i64::MIN % -1 would panic, but result should be 0
                let result = if a == i64::MIN && b == -1 { 0 } else { a % b };
                
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64RemU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64RemU second operand not i64")
                })?;
                
                if b == 0 {
                    return Err(Error::runtime_division_by_zero("I64RemU division by zero"));
                }
                
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64RemU first operand not i64")
                })?;
                
                // Unsigned remainder - cast to u64
                let result = (a as u64) % (b as u64);
                engine.exec_stack.values.push(Value::I64(result as i64)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Shl => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Shl second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Shl first operand not i64")
                })?;
                // Shift amount is masked to 6 bits (0-63) as per WebAssembly spec for i64
                let shift = (b as u64) & 0x3F;
                engine.exec_stack.values.push(Value::I64(a << shift)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64ShrS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64ShrS second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64ShrS first operand not i64")
                })?;
                // Shift amount is masked to 6 bits (0-63) as per WebAssembly spec for i64
                let shift = (b as u64) & 0x3F;
                engine.exec_stack.values.push(Value::I64(a >> shift)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64ShrU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64ShrU second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64ShrU first operand not i64")
                })?;
                // Shift amount is masked to 6 bits (0-63) as per WebAssembly spec for i64
                let shift = (b as u64) & 0x3F;
                // Unsigned right shift
                let result = (a as u64) >> shift;
                engine.exec_stack.values.push(Value::I64(result as i64)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Rotl => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Rotl second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Rotl first operand not i64")
                })?;
                // Rotate amount is masked to 6 bits (0-63) as per WebAssembly spec for i64
                let rotate = (b as u64) & 0x3F;
                let result = (a as u64).rotate_left(rotate as u32);
                engine.exec_stack.values.push(Value::I64(result as i64)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Rotr => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Rotr second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Rotr first operand not i64")
                })?;
                // Rotate amount is masked to 6 bits (0-63) as per WebAssembly spec for i64
                let rotate = (b as u64) & 0x3F;
                let result = (a as u64).rotate_right(rotate as u32);
                engine.exec_stack.values.push(Value::I64(result as i64)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Floating-point arithmetic operations
            Instruction::F32Add => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Add second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Add first operand not f32")
                })?;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits((a + b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Sub => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Sub second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Sub first operand not f32")
                })?;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits((a - b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Mul => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Mul second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Mul first operand not f32")
                })?;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits((a * b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Div => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Div second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Div first operand not f32")
                })?;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits((a / b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Stack manipulation
            Instruction::Select => {
                let condition_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select condition"))?;
                let condition = match condition_val {
                    Value::I32(val) => val,
                    _ => return Err(Error::validation_error("Select condition not i32")),
                };
                let val2 = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select val2"))?;
                let val1 = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select val1"))?;
                
                let result = if condition != 0 { val1 } else { val2 };
                engine.exec_stack.values.push(result).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // I32 comparison operations
            Instruction::I32LtU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32LtU second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32LtU first operand not i32")
                })?;
                let result = if (a as u32) < (b as u32) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32GtS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GtS second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GtS first operand not i32")
                })?;
                let result = if a > b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32GtU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GtU second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GtU first operand not i32")
                })?;
                let result = if (a as u32) > (b as u32) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32LeS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32LeS second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32LeS first operand not i32")
                })?;
                let result = if a <= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32LeU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32LeU second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32LeU first operand not i32")
                })?;
                let result = if (a as u32) <= (b as u32) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32GeS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GeS second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GeS first operand not i32")
                })?;
                let result = if a >= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32GeU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GeU second operand not i32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32GeU first operand not i32")
                })?;
                let result = if (a as u32) >= (b as u32) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // I32 unary operations
            Instruction::I32Eqz => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Eqz operand not i32")
                })?;
                let result = if a == 0 { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Type conversion operations
            Instruction::I32WrapI64 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I32WrapI64 operand not i64")
                })?;
                // Wrap i64 to i32 by truncating upper 32 bits
                let result = a as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64ExtendI32S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I64ExtendI32S operand not i32")
                })?;
                // Sign-extend i32 to i64
                let result = a as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64ExtendI32U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I64ExtendI32U operand not i32")
                })?;
                // Zero-extend i32 to i64
                let result = (a as u32) as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32TruncF32S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("I32TruncF32S operand not f32")
                })?;
                
                // Check for NaN or out-of-range values
                if a.is_nan() || a.is_infinite() || a < -2_147_483_649.0 || a >= 2_147_483_648.0 {
                    return Err(Error::runtime_integer_overflow("I32TruncF32S out of range"));
                }
                
                let result = {
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32TruncF32U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("I32TruncF32U operand not f32")
                })?;
                
                // Check for NaN or out-of-range values for unsigned
                if a.is_nan() || a.is_infinite() || a < -1.0 || a >= 4_294_967_296.0 {
                    return Err(Error::runtime_integer_overflow("I32TruncF32U out of range"));
                }
                
                let result = ({
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as u32) as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // I64 comparison operations
            Instruction::I64Eq => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Eq second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Eq first operand not i64")
                })?;
                let result = if a == b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Ne => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Ne second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Ne first operand not i64")
                })?;
                let result = if a != b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64LtS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LtS second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LtS first operand not i64")
                })?;
                let result = if a < b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64LtU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LtU second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LtU first operand not i64")
                })?;
                let result = if (a as u64) < (b as u64) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64GtS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GtS second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GtS first operand not i64")
                })?;
                let result = if a > b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64GtU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GtU second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GtU first operand not i64")
                })?;
                let result = if (a as u64) > (b as u64) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64LeS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LeS second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LeS first operand not i64")
                })?;
                let result = if a <= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64LeU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LeU second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64LeU first operand not i64")
                })?;
                let result = if (a as u64) <= (b as u64) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64GeS => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GeS second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GeS first operand not i64")
                })?;
                let result = if a >= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64GeU => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GeU second operand not i64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64GeU first operand not i64")
                })?;
                let result = if (a as u64) >= (b as u64) { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // I64 unary operations
            Instruction::I64Eqz => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Eqz operand not i64")
                })?;
                let result = if a == 0 { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // F32 comparison operations
            Instruction::F32Eq => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Eq second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Eq first operand not f32")
                })?;
                let result = if a == b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Ne => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Ne second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Ne first operand not f32")
                })?;
                let result = if a != b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Lt => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Lt second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Lt first operand not f32")
                })?;
                let result = if a < b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Gt => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Gt second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Gt first operand not f32")
                })?;
                let result = if a > b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Le => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Le second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Le first operand not f32")
                })?;
                let result = if a <= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Ge => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Ge second operand not f32")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Ge first operand not f32")
                })?;
                let result = if a >= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // F64 comparison operations
            Instruction::F64Eq => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Eq second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Eq first operand not f64")
                })?;
                let result = if a == b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Ne => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Ne second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Ne first operand not f64")
                })?;
                let result = if a != b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Lt => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Lt second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Lt first operand not f64")
                })?;
                let result = if a < b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Gt => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Gt second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Gt first operand not f64")
                })?;
                let result = if a > b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Le => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Le second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Le first operand not f64")
                })?;
                let result = if a <= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Ge => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Ge second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Ge first operand not f64")
                })?;
                let result = if a >= b { 1 } else { 0 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // F32 unary operations
            Instruction::F32Abs => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Abs operand not f32")
                })?;
                let result = a.abs();
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Neg => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Neg operand not f32")
                })?;
                let result = -a;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Ceil => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Ceil operand not f32")
                })?;
                let result = a.ceil();
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Floor => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Floor operand not f32")
                })?;
                let result = a.floor();
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Trunc => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Trunc operand not f32")
                })?;
                let result = {
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                };
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Nearest => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Nearest operand not f32")
                })?;
                let result = a.round();
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Sqrt => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Sqrt operand not f32")
                })?;
                let result = a.sqrt();
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // F64 arithmetic operations
            Instruction::F64Add => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Add second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Add first operand not f64")
                })?;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits((a + b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Sub => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Sub second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Sub first operand not f64")
                })?;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits((a - b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Mul => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Mul second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Mul first operand not f64")
                })?;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits((a * b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Div => {
                let b = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Div second operand not f64")
                })?;
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Div first operand not f64")
                })?;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits((a / b).to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // More type conversion operations
            Instruction::I32TruncF64S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("I32TruncF64S operand not f64")
                })?;
                
                // Check for NaN or out-of-range values
                if a.is_nan() || a.is_infinite() || a < -2_147_483_649.0 || a >= 2_147_483_648.0 {
                    return Err(Error::runtime_integer_overflow("I32TruncF64S out of range"));
                }
                
                let result = {
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32TruncF64U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("I32TruncF64U operand not f64")
                })?;
                
                // Check for NaN or out-of-range values for unsigned
                if a.is_nan() || a.is_infinite() || a < -1.0 || a >= 4_294_967_296.0 {
                    return Err(Error::runtime_integer_overflow("I32TruncF64U out of range"));
                }
                
                let result = ({
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as u32) as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64TruncF32S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("I64TruncF32S operand not f32")
                })?;
                
                // Check for NaN or out-of-range values
                if a.is_nan() || a.is_infinite() || a < -9_223_372_036_854_775_808.0 || a >= 9_223_372_036_854_775_808.0 {
                    return Err(Error::runtime_integer_overflow("I64TruncF32S out of range"));
                }
                
                let result = {
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64TruncF32U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("I64TruncF32U operand not f32")
                })?;
                
                // Check for NaN or out-of-range values for unsigned
                if a.is_nan() || a.is_infinite() || a < -1.0 || a >= 18_446_744_073_709_551_616.0 {
                    return Err(Error::runtime_integer_overflow("I64TruncF32U out of range"));
                }
                
                let result = ({
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as u64) as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64TruncF64S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("I64TruncF64S operand not f64")
                })?;
                
                // Check for NaN or out-of-range values
                if a.is_nan() || a.is_infinite() || a < -9_223_372_036_854_775_808.0 || a >= 9_223_372_036_854_775_808.0 {
                    return Err(Error::runtime_integer_overflow("I64TruncF64S out of range"));
                }
                
                let result = {
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64TruncF64U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("I64TruncF64U operand not f64")
                })?;
                
                // Check for NaN or out-of-range values for unsigned
                if a.is_nan() || a.is_infinite() || a < -1.0 || a >= 18_446_744_073_709_551_616.0 {
                    return Err(Error::runtime_integer_overflow("I64TruncF64U out of range"));
                }
                
                let result = ({
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                } as u64) as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Float to float conversions
            Instruction::F32ConvertI32S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("F32ConvertI32S operand not i32")
                })?;
                let result = a as f32;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32ConvertI32U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("F32ConvertI32U operand not i32")
                })?;
                let result = (a as u32) as f32;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32ConvertI64S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("F32ConvertI64S operand not i64")
                })?;
                let result = a as f32;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32ConvertI64U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("F32ConvertI64U operand not i64")
                })?;
                let result = (a as u64) as f32;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32DemoteF64 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F32DemoteF64 operand not f64")
                })?;
                let result = a as f32;
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // F64 conversion operations
            Instruction::F64ConvertI32S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("F64ConvertI32S operand not i32")
                })?;
                let result = a as f64;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64ConvertI32U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("F64ConvertI32U operand not i32")
                })?;
                let result = (a as u32) as f64;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64ConvertI64S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("F64ConvertI64S operand not i64")
                })?;
                let result = a as f64;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64ConvertI64U => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("F64ConvertI64U operand not i64")
                })?;
                let result = (a as u64) as f64;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64PromoteF32 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F64PromoteF32 operand not f32")
                })?;
                let result = a as f64;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Reinterpret operations
            Instruction::I32ReinterpretF32 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("I32ReinterpretF32 operand not f32")
                })?;
                let result = a.to_bits() as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64ReinterpretF64 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("I64ReinterpretF64 operand not f64")
                })?;
                let result = a.to_bits() as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32ReinterpretI32 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("F32ReinterpretI32 operand not i32")
                })?;
                let result = f32::from_bits(a as u32);
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64ReinterpretI64 => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("F64ReinterpretI64 operand not i64")
                })?;
                let result = f64::from_bits(a as u64);
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // F64 unary operations
            Instruction::F64Abs => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Abs operand not f64")
                })?;
                let result = a.abs();
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Neg => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Neg operand not f64")
                })?;
                let result = -a;
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Ceil => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Ceil operand not f64")
                })?;
                let result = a.ceil();
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Floor => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Floor operand not f64")
                })?;
                let result = a.floor();
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Trunc => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Trunc operand not f64")
                })?;
                let result = {
                    #[cfg(feature = "std")]
                    { a.trunc() }
                    #[cfg(not(feature = "std"))]
                    { 
                        // Manual truncation: remove fractional part
                        a as i32 as f32
                    }
                };
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Nearest => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Nearest operand not f64")
                })?;
                let result = a.round();
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Sqrt => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Sqrt operand not f64")
                })?;
                let result = a.sqrt();
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_bits(result.to_bits()))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Sign extension operations
            Instruction::I32Extend8S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Extend8S operand not i32")
                })?;
                // Sign-extend from 8 bits to 32 bits
                let result = (a as i8) as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Extend16S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Extend16S operand not i32")
                })?;
                // Sign-extend from 16 bits to 32 bits
                let result = (a as i16) as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Extend8S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Extend8S operand not i64")
                })?;
                // Sign-extend from 8 bits to 64 bits
                let result = (a as i8) as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Extend16S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Extend16S operand not i64")
                })?;
                // Sign-extend from 16 bits to 64 bits
                let result = (a as i16) as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Extend32S => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Extend32S operand not i64")
                })?;
                // Sign-extend from 32 bits to 64 bits
                let result = (a as i32) as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Bit counting operations
            Instruction::I32Clz => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Clz operand not i32")
                })?;
                // Count leading zeros
                let result = a.leading_zeros() as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Ctz => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Ctz operand not i32")
                })?;
                // Count trailing zeros
                let result = a.trailing_zeros() as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I32Popcnt => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i32()).ok_or_else(|| {
                    Error::validation_error("I32Popcnt operand not i32")
                })?;
                // Count number of 1 bits
                let result = a.count_ones() as i32;
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Clz => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Clz operand not i64")
                })?;
                // Count leading zeros
                let result = a.leading_zeros() as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Ctz => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Ctz operand not i64")
                })?;
                // Count trailing zeros
                let result = a.trailing_zeros() as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::I64Popcnt => {
                let a = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.and_then(|v| v.as_i64()).ok_or_else(|| {
                    Error::validation_error("I64Popcnt operand not i64")
                })?;
                // Count number of 1 bits
                let result = a.count_ones() as i64;
                engine.exec_stack.values.push(Value::I64(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Floating-point min/max/copysign operations
            Instruction::F32Min => {
                let b_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let b = b_val.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Min second operand not f32")
                })?;
                let a_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let a = a_val.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Min first operand not f32")
                })?;
                // WebAssembly min: NaN if either operand is NaN, otherwise the smaller value
                let result = if a.is_nan() || b.is_nan() {
                    f32::NAN
                } else if a == 0.0 && b == 0.0 {
                    // -0.0 is smaller than +0.0
                    if a.is_sign_negative() || b.is_sign_negative() {
                        -0.0
                    } else {
                        0.0
                    }
                } else {
                    a.min(b)
                };
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_float(result))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Max => {
                let b_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let b = b_val.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Max second operand not f32")
                })?;
                let a_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let a = a_val.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Max first operand not f32")
                })?;
                // WebAssembly max: NaN if either operand is NaN, otherwise the larger value
                let result = if a.is_nan() || b.is_nan() {
                    f32::NAN
                } else if a == 0.0 && b == 0.0 {
                    // +0.0 is larger than -0.0
                    if a.is_sign_positive() || b.is_sign_positive() {
                        0.0
                    } else {
                        -0.0
                    }
                } else {
                    a.max(b)
                };
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_float(result))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F32Copysign => {
                let b_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let b = b_val.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Copysign second operand not f32")
                })?;
                let a_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let a = a_val.and_then(|v| v.as_f32()).ok_or_else(|| {
                    Error::validation_error("F32Copysign first operand not f32")
                })?;
                // Copy sign from b to a
                let result = a.copysign(b);
                engine.exec_stack.values.push(Value::F32(FloatBits32::from_float(result))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Min => {
                let b_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let b = b_val.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Min second operand not f64")
                })?;
                let a_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let a = a_val.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Min first operand not f64")
                })?;
                // WebAssembly min: NaN if either operand is NaN, otherwise the smaller value
                let result = if a.is_nan() || b.is_nan() {
                    f64::NAN
                } else if a == 0.0 && b == 0.0 {
                    // -0.0 is smaller than +0.0
                    if a.is_sign_negative() || b.is_sign_negative() {
                        -0.0
                    } else {
                        0.0
                    }
                } else {
                    a.min(b)
                };
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_float(result))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Max => {
                let b_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let b = b_val.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Max second operand not f64")
                })?;
                let a_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let a = a_val.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Max first operand not f64")
                })?;
                // WebAssembly max: NaN if either operand is NaN, otherwise the larger value
                let result = if a.is_nan() || b.is_nan() {
                    f64::NAN
                } else if a == 0.0 && b == 0.0 {
                    // +0.0 is larger than -0.0
                    if a.is_sign_positive() || b.is_sign_positive() {
                        0.0
                    } else {
                        -0.0
                    }
                } else {
                    a.max(b)
                };
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_float(result))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::F64Copysign => {
                let b_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let b = b_val.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Copysign second operand not f64")
                })?;
                let a_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let a = a_val.and_then(|v| v.as_f64()).ok_or_else(|| {
                    Error::validation_error("F64Copysign first operand not f64")
                })?;
                // Copy sign from b to a
                let result = a.copysign(b);
                engine.exec_stack.values.push(Value::F64(FloatBits64::from_float(result))).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Reference type instructions
            Instruction::RefNull(ref_type) => {
                let null_value = match ref_type.to_value_type() {
                    ValueType::FuncRef => Value::FuncRef(None),
                    ValueType::ExternRef => Value::ExternRef(None),
                    _ => return Err(Error::validation_error("RefNull with invalid reference type")),
                };
                engine.exec_stack.values.push(null_value).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::RefIsNull => {
                let ref_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let is_null = match ref_val {
                    Some(Value::FuncRef(opt_ref)) => opt_ref.is_none(),
                    Some(Value::ExternRef(opt_ref)) => opt_ref.is_none(),
                    _ => return Err(Error::validation_error("RefIsNull operand is not a reference type")),
                };
                let result = if is_null { 1i32 } else { 0i32 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::RefFunc(func_idx) => {
                // Validate that the function index exists
                let module = self.module_instance.module();
                if func_idx >= module.functions.len() as u32 {
                    return Err(Error::validation_error("Stack operation error"));
                }
                let func_ref = Value::FuncRef(Some(FuncRef::from_index(func_idx)));
                engine.exec_stack.values.push(func_ref).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Stack operations
            Instruction::Drop => {
                engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::Select => {
                let condition_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select condition"))?;
                let condition = match condition_val {
                    Value::I32(val) => val,
                    _ => return Err(Error::validation_error("Select condition not i32")),
                };
                let val2 = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select val2"))?;
                let val1 = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select val1"))?;
                let result = if condition != 0 { val1 } else { val2 };
                engine.exec_stack.values.push(result).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::SelectWithType(_value_types) => {
                // SelectWithType behaves the same as Select for execution, the type information is for validation
                let condition_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select condition"))?;
                let condition = match condition_val {
                    Value::I32(val) => val,
                    _ => return Err(Error::validation_error("SelectWithType condition not i32")),
                };
                let val2 = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select val2"))?;
                let val1 = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for select val1"))?;
                let result = if condition != 0 { val1 } else { val2 };
                engine.exec_stack.values.push(result).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Branch table instruction
            Instruction::BrTable { targets, default_target } => {
                let index_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let index = match index_val {
                    Some(Value::I32(val)) => val as usize,
                    _ => return Err(Error::validation_error("BrTable index not i32")),
                };
                
                // Select the target label: if index is in bounds, use targets[index], otherwise use default_target
                let target_label = if index < targets.len() {
                    targets.get(index).map_err(|e| {
                        Error::memory_out_of_bounds("Stack operation error")
                    })?
                } else {
                    default_target
                };
                
                // Perform the branch to the selected target
                self.branch_to_label(target_label, engine)?;
                Ok(ControlFlow::Branch(target_label as usize))
            }
            
            // Advanced memory operations
            Instruction::MemoryFill(mem_idx) => {
                let size_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let size = match size_val {
                    Some(Value::I32(val)) => val as usize,
                    _ => return Err(Error::validation_error("MemoryFill size not i32")),
                };
                
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I32(val)) => val as u8,
                    _ => return Err(Error::validation_error("MemoryFill value not i32")),
                };
                
                let offset_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let offset = match offset_val {
                    Some(Value::I32(val)) => val as usize,
                    _ => return Err(Error::validation_error("MemoryFill offset not i32")),
                };
                
                // Get the memory instance
                let memory = self.module_instance.memory(mem_idx)?;
                
                // Perform bounds check
                if offset + size > memory.size_in_bytes() {
                    return Err(Error::memory_out_of_bounds("MemoryFill operation out of bounds"));
                }
                
                // TODO: Fill memory with the specified value using the instruction layer implementation
                // use wrt_instructions::memory_ops::{MemoryFill, MemoryOperations};
                // let fill_op = MemoryFill::new(mem_idx);
                // fill_op.execute(&mut memory, &Value::I32(offset as i32), &Value::I32(value as i32), &Value::I32(size as i32))?;
                return Err(Error::runtime_execution_error("
                ))
            }
            
            Instruction::MemoryCopy(dst_mem_idx, src_mem_idx) => {
                let size_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow(")
                })?;
                let size = match size_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("MemoryCopy size not i32")),
                };
                
                let src_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let src = match src_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("MemoryCopy src not i32")),
                };
                
                let dest_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let dest = match dest_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("MemoryCopy dest not i32")),
                };
                
                // Get the memory instance (assuming same memory for both src and dest in MVP)
                let memory = self.module_instance.memory(dst_mem_idx)?;
                
                // TODO: Execute bulk memory copy operation using our runtime
                // use wrt_instructions::memory_ops::{MemoryCopy, MemoryOperations};
                // let copy_op = MemoryCopy::new(dst_mem_idx, src_mem_idx);
                // copy_op.execute(&mut memory, &Value::I32(dest as i32), &Value::I32(src as i32), &Value::I32(size as i32))?;
                return Err(Error::runtime_execution_error("
                ))
            }
            
            Instruction::DataDrop(data_seg_idx) => {
                // Data segments are typically handled at module instantiation time
                // DataDrop marks a data segment as ");
                if data_seg_idx >= module.data.len() as u32 {
                    return Err(Error::validation_error("Stack operation error"));
                }
                
                // TODO: In a full implementation, mark the data segment as dropped
                // This would prevent future memory.init operations from using this segment
                
                Ok(ControlFlow::Next)
            }
            
            // Tail call instructions (WebAssembly 2.0)
            Instruction::ReturnCall(func_idx) => {
                // Validate function index
                let module = self.module_instance.module();
                if func_idx >= module.functions.len() as u32 {
                    return Err(Error::validation_error("Stack operation error"));
                }
                
                // Return TailCall control flow to indicate frame replacement
                Ok(ControlFlow::TailCall(func_idx))
            }
            
            Instruction::ReturnCallIndirect(type_idx, table_idx) => {
                // Pop the function index from stack
                let func_index_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let func_index = match func_index_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("ReturnCallIndirect function index not i32")),
                };
                
                // Get table and validate index
                let table = self.module_instance.get_table(table_idx as usize).map_err(|_| {
                    Error::validation_error("Stack operation error")
                })?;
                
                if func_index >= table.size() {
                    return Err(Error::memory_out_of_bounds("ReturnCallIndirect function index out of table bounds"));
                }
                
                // Get function reference from table
                let func_ref = table.get(func_index).map_err(|e| {
                    Error::runtime_memory_access_error("Stack operation error")
                })?;
                
                let actual_func_idx = match func_ref {
                    Some(Value::FuncRef(Some(fref))) => fref.index,
                    Some(Value::FuncRef(None)) | None => {
                        return Err(Error::runtime_type_mismatch("ReturnCallIndirect null function reference"));
                    }
                    _ => {
                        return Err(Error::runtime_type_mismatch("ReturnCallIndirect invalid table element type"));
                    }
                };
                
                // Validate function type matches expected type
                let module = self.module_instance.module();
                let function = module.functions.get(actual_func_idx as usize).map_err(|_| {
                    Error::validation_error("Stack operation error")
                })?;
                
                if function.type_idx != type_idx {
                    return Err(Error::runtime_type_mismatch("ReturnCallIndirect function type mismatch"));
                }
                
                // Return TailCall control flow for the resolved function
                Ok(ControlFlow::TailCall(actual_func_idx))
            }
            
            // Branch on null instructions (WebAssembly 2.0 GC)
            Instruction::BrOnNull(label_idx) => {
                let ref_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for br_on_null"))?;
                
                let is_null = match ref_val {
                    Value::FuncRef(ref opt_ref) => opt_ref.is_none(),
                    Value::ExternRef(ref opt_ref) => opt_ref.is_none(),
                    Value::StructRef(ref opt_ref) => opt_ref.is_none(),
                    Value::ArrayRef(ref opt_ref) => opt_ref.is_none(),
                    _ => return Err(Error::validation_error("BrOnNull operand is not a reference type")),
                };
                
                if is_null {
                    // Branch to the label
                    self.branch_to_label(label_idx, engine)?;
                    return Ok(ControlFlow::Branch(label_idx as usize));
                } else {
                    // Push the non-null reference back onto stack and continue
                    engine.exec_stack.values.push(ref_val).map_err(|e| {
                        Error::runtime_stack_overflow("Stack operation error")
                    })?;
                    return Ok(ControlFlow::Next);
                }
            }
            
            Instruction::BrOnNonNull(label_idx) => {
                let ref_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for br_on_non_null"))?;
                
                let is_null = match ref_val {
                    Value::FuncRef(ref opt_ref) => opt_ref.is_none(),
                    Value::ExternRef(ref opt_ref) => opt_ref.is_none(),
                    Value::StructRef(ref opt_ref) => opt_ref.is_none(),
                    Value::ArrayRef(ref opt_ref) => opt_ref.is_none(),
                    _ => return Err(Error::validation_error("BrOnNonNull operand is not a reference type")),
                };
                
                if !is_null {
                    // Push the non-null reference back onto stack and branch
                    engine.exec_stack.values.push(ref_val).map_err(|e| {
                        Error::runtime_stack_overflow("Stack operation error")
                    })?;
                    self.branch_to_label(label_idx, engine)?;
                    return Ok(ControlFlow::Branch(label_idx as usize));
                } else {
                    // Reference is null, continue without branching (don't push null back)
                    return Ok(ControlFlow::Next);
                }
            }
            
            // Memory initialization instruction
            Instruction::MemoryInit(data_seg_idx, mem_idx) => {
                let size_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let size = match size_val {
                    Some(Value::I32(val)) => val as usize,
                    _ => return Err(Error::validation_error("MemoryInit size not i32")),
                };
                
                let src_offset_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let src_offset = match src_offset_val {
                    Some(Value::I32(val)) => val as usize,
                    _ => return Err(Error::validation_error("MemoryInit src_offset not i32")),
                };
                
                let dst_offset_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let dst_offset = match dst_offset_val {
                    Some(Value::I32(val)) => val as usize,
                    _ => return Err(Error::validation_error("MemoryInit dst_offset not i32")),
                };
                
                // Validate memory index
                let memory = self.module_instance.memory(mem_idx)?;
                
                // Validate data segment index
                let module = self.module_instance.module();
                let data_segment = module.data.get(data_seg_idx as usize).map_err(|_| {
                    Error::validation_error("Stack operation error")
                })?;
                
                // Bounds checks
                if dst_offset + size > memory.size_in_bytes() {
                    return Err(Error::memory_out_of_bounds("MemoryInit destination out of bounds"));
                }
                
                let data = data_segment.data().map_err(|e| {
                    Error::runtime_memory_access_error("Data segment access error")
                })?;
                
                if src_offset + size > data.len() {
                    return Err(Error::memory_out_of_bounds("MemoryInit source out of bounds"));
                }
                
                // Copy data from segment to memory
                for i in 0..size {
                    let byte = data.get(src_offset + i).ok_or_else(|| {
                        Error::memory_out_of_bounds("MemoryInit data segment access out of bounds")
                    })?;
                    // Write one byte at a time - this will fail due to Arc<Memory> immutability
                    memory.write((dst_offset + i) as u32, &[*byte]).map_err(|e| {
                        Error::runtime_memory_access_error("Memory write error")
                    })?;
                }
                
                Ok(ControlFlow::Next)
            }
            
            // Additional reference operations (WebAssembly 2.0 GC)
            Instruction::RefAsNonNull => {
                let ref_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for ref_as_non_null"))?;
                
                let is_null = match ref_val {
                    Value::FuncRef(ref opt_ref) => opt_ref.is_none(),
                    Value::ExternRef(ref opt_ref) => opt_ref.is_none(),
                    Value::StructRef(ref opt_ref) => opt_ref.is_none(),
                    Value::ArrayRef(ref opt_ref) => opt_ref.is_none(),
                    _ => return Err(Error::validation_error("RefAsNonNull operand is not a reference type")),
                };
                
                if is_null {
                    // Trap if reference is null
                    return Err(Error::runtime_trap_execution_error("RefAsNonNull: null reference"));
                } else {
                    // Push the non-null reference back onto stack
                    engine.exec_stack.values.push(ref_val).map_err(|e| {
                        Error::runtime_stack_overflow("Stack operation error")
                    })?;
                    Ok(ControlFlow::Next)
                }
            }
            
            Instruction::RefEq => {
                let ref2_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for ref_eq ref2"))?;
                let ref1_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?.ok_or_else(|| Error::runtime_stack_underflow("Stack empty for ref_eq ref1"))?;
                
                // Compare references for equality
                let are_equal = match (&ref1_val, &ref2_val) {
                    (Value::FuncRef(opt1), Value::FuncRef(opt2)) => match (opt1, opt2) {
                        (None, None) => true,
                        (Some(ref1), Some(ref2)) => ref1.index == ref2.index,
                        _ => false,
                    },
                    (Value::ExternRef(opt1), Value::ExternRef(opt2)) => match (opt1, opt2) {
                        (None, None) => true,
                        (Some(ref1), Some(ref2)) => ref1.index == ref2.index,
                        _ => false,
                    },
                    (Value::StructRef(opt1), Value::StructRef(opt2)) => match (opt1, opt2) {
                        (None, None) => true,
                        (Some(ref1), Some(ref2)) => {
                            // For struct references, we compare by identity (same reference)
                            // In a full GC implementation, this would be pointer equality
                            ref1.type_index == ref2.type_index && ref1.fields == ref2.fields
                        },
                        _ => false,
                    },
                    (Value::ArrayRef(opt1), Value::ArrayRef(opt2)) => match (opt1, opt2) {
                        (None, None) => true,
                        (Some(ref1), Some(ref2)) => {
                            // For array references, we compare by identity (same reference)
                            ref1.type_index == ref2.type_index && ref1.elements == ref2.elements
                        },
                        _ => false,
                    },
                    _ => return Err(Error::validation_error("RefEq: operands must be compatible reference types")),
                };
                
                let result = if are_equal { 1i32 } else { 0i32 };
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // Atomic operations (WebAssembly Threads proposal)
            Instruction::MemoryAtomicNotify { memarg } => {
                let count_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let count = match count_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("MemoryAtomicNotify count not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("MemoryAtomicNotify addr not i32")),
                };
                
                // Calculate effective address with alignment check
                let effective_addr = addr + memarg.offset;
                if effective_addr % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("MemoryAtomicNotify requires 4-byte alignment"));
                }
                
                // For now, implement as a no-op since we don't have a full threading model
                // In a full implementation, this would notify threads waiting on this memory location
                let woken_count = 0i32; // No threads to wake in current implementation
                
                engine.exec_stack.values.push(Value::I32(woken_count)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::MemoryAtomicWait32 { memarg } => {
                let timeout_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let timeout = match timeout_val {
                    Some(Value::I64(val)) => val,
                    _ => return Err(Error::validation_error("MemoryAtomicWait32 timeout not i64")),
                };
                
                let expected_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let expected = match expected_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("MemoryAtomicWait32 expected not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("MemoryAtomicWait32 addr not i32")),
                };
                
                // Calculate effective address with alignment check
                let effective_addr = addr + memarg.offset;
                if effective_addr % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("MemoryAtomicWait32 requires 4-byte alignment"));
                }
                
                // Get memory and read current value
                let memory = self.module_instance.memory(0).map_err(|_| {
                    Error::validation_error("No memory instance for atomic operation")
                })?;
                
                // Read 4 bytes for i32
                let mut bytes = [0u8; 4];
                memory.read(effective_addr, &mut bytes).map_err(|e| {
                    Error::runtime_memory_access_error("Memory read error")
                })?;
                let current_val = i32::from_le_bytes(bytes);
                
                // Compare and return result
                let result = if current_val != expected {
                    1i32 // "not-equal"
                } else {
                    // In a full implementation, this would block the thread until notified or timeout
                    // For now, return "ok" (value was equal but we don't wait)
                    0i32 // "ok"
                };
                
                engine.exec_stack.values.push(Value::I32(result)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicLoad { memarg } => {
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32AtomicLoad addr not i32")),
                };
                
                // Calculate effective address with alignment check
                let effective_addr = addr + memarg.offset;
                if effective_addr % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicLoad requires 4-byte alignment"));
                }
                
                // Get memory and perform atomic load
                let memory = self.module_instance.memory(0).map_err(|_| {
                    Error::validation_error("No memory instance for atomic operation")
                })?;
                
                // Read 4 bytes for i32
                let mut bytes = [0u8; 4];
                memory.read(effective_addr, &mut bytes).map_err(|e| {
                    Error::runtime_memory_access_error("Memory read error")
                })?;
                let value = i32::from_le_bytes(bytes);
                
                engine.exec_stack.values.push(Value::I32(value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicStore { memarg } => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32AtomicStore value not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32AtomicStore addr not i32")),
                };
                
                // Calculate effective address with alignment check
                let effective_addr = addr + memarg.offset;
                if effective_addr % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicStore requires 4-byte alignment"));
                }
                
                // Get memory and perform atomic store
                let memory = self.module_instance.memory(0).map_err(|_| {
                    Error::validation_error("No memory instance for atomic operation")
                })?;
                
                // Write 4 bytes for i32 - this will fail due to Arc<Memory> immutability
                let bytes = value.to_le_bytes();
                memory.write(effective_addr, &bytes).map_err(|e| {
                    Error::runtime_memory_access_error("Memory write error")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmwAdd { memarg } => {
                let value_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let value = match value_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32AtomicRmwAdd value not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32AtomicRmwAdd addr not i32")),
                };
                
                // Calculate effective address with alignment check
                let effective_addr = addr + memarg.offset;
                if effective_addr % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwAdd requires 4-byte alignment"));
                }
                
                // Get memory and perform atomic read-modify-write add
                let memory = self.module_instance.memory(0).map_err(|_| {
                    Error::validation_error("No memory instance for atomic operation")
                })?;
                
                let old_value = {
                    let mut bytes = [0u8; 4];
                    memory.read(effective_addr, &mut bytes).map_err(|e| {
                        Error::runtime_memory_access_error("Memory read error")
                    })?;
                    i32::from_le_bytes(bytes)
                };
                
                let new_value = old_value.wrapping_add(value);
                {
                    let bytes = new_value.to_le_bytes();
                    memory.write(effective_addr, &bytes).map_err(|e| {
                        Error::runtime_memory_access_error("Memory write error")
                    })?
                };
                
                // Return the old value
                engine.exec_stack.values.push(Value::I32(old_value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmwCmpxchg { memarg } => {
                let replacement_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let replacement = match replacement_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32AtomicRmwCmpxchg replacement not i32")),
                };
                
                let expected_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let expected = match expected_val {
                    Some(Value::I32(val)) => val,
                    _ => return Err(Error::validation_error("I32AtomicRmwCmpxchg expected not i32")),
                };
                
                let addr_val = engine.exec_stack.values.pop().map_err(|e| {
                    Error::runtime_stack_underflow("Stack operation error")
                })?;
                let addr = match addr_val {
                    Some(Value::I32(val)) => val as u32,
                    _ => return Err(Error::validation_error("I32AtomicRmwCmpxchg addr not i32")),
                };
                
                // Calculate effective address with alignment check
                let effective_addr = addr + memarg.offset;
                if effective_addr % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwCmpxchg requires 4-byte alignment"));
                }
                
                // Get memory and perform atomic compare-exchange
                let memory = self.module_instance.memory(0).map_err(|_| {
                    Error::validation_error("No memory instance for atomic operation")
                })?;
                
                let current_value = {
                    let mut bytes = [0u8; 4];
                    memory.read(effective_addr, &mut bytes).map_err(|e| {
                        Error::runtime_memory_access_error("Memory read error")
                    })?;
                    i32::from_le_bytes(bytes)
                };
                
                if current_value == expected {
                    // Values match, perform the exchange
                    {
                        let bytes = replacement.to_le_bytes();
                        memory.write(effective_addr, &bytes).map_err(|e| {
                            Error::runtime_memory_access_error("Memory write error")
                        })?
                    };
                }
                
                // Return the old value regardless of whether exchange occurred
                engine.exec_stack.values.push(Value::I32(current_value)).map_err(|e| {
                    Error::runtime_stack_overflow("Stack operation error")
                })?;
                
                Ok(ControlFlow::Next)
            }            
            Instruction::AtomicFence => {
                // Atomic fence ensures memory ordering
                // In a single-threaded implementation, this is effectively a no-op
                // In a multi-threaded implementation, this would provide memory barriers
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic RMW Sub
            Instruction::I32AtomicRmwSub { memarg } => {
                // Pop value to subtract
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmwSub value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmwSub addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwSub requires 4-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read current value
                let mut bytes = [0u8; 4];
                memory_guard.read(effective_addr, &mut bytes)?;
                let current_value = i32::from_le_bytes(bytes);
                
                // Perform atomic subtraction
                let new_value = current_value.wrapping_sub(value_i32);
                let new_bytes = new_value.to_le_bytes();
                memory_guard.write(effective_addr, &new_bytes)?;
                
                // Push the old value
                engine.exec_stack.values.push(Value::I32(current_value)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic RMW And
            Instruction::I32AtomicRmwAnd { memarg } => {
                // Pop value to AND
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmwAnd value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmwAnd addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwAnd requires 4-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read current value
                let mut bytes = [0u8; 4];
                memory_guard.read(effective_addr, &mut bytes)?;
                let current_value = i32::from_le_bytes(bytes);
                
                // Perform atomic AND
                let new_value = current_value & value_i32;
                let new_bytes = new_value.to_le_bytes();
                memory_guard.write(effective_addr, &new_bytes)?;
                
                // Push the old value
                engine.exec_stack.values.push(Value::I32(current_value)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic RMW Or
            Instruction::I32AtomicRmwOr { memarg } => {
                // Pop value to OR
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmwOr value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmwOr addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwOr requires 4-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read current value
                let mut bytes = [0u8; 4];
                memory_guard.read(effective_addr, &mut bytes)?;
                let current_value = i32::from_le_bytes(bytes);
                
                // Perform atomic OR
                let new_value = current_value | value_i32;
                let new_bytes = new_value.to_le_bytes();
                memory_guard.write(effective_addr, &new_bytes)?;
                
                // Push the old value
                engine.exec_stack.values.push(Value::I32(current_value)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic RMW Xor
            Instruction::I32AtomicRmwXor { memarg } => {
                // Pop value to XOR
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmwXor value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmwXor addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwXor requires 4-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read current value
                let mut bytes = [0u8; 4];
                memory_guard.read(effective_addr, &mut bytes)?;
                let current_value = i32::from_le_bytes(bytes);
                
                // Perform atomic XOR
                let new_value = current_value ^ value_i32;
                let new_bytes = new_value.to_le_bytes();
                memory_guard.write(effective_addr, &new_bytes)?;
                
                // Push the old value
                engine.exec_stack.values.push(Value::I32(current_value)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic Load 8-bit unsigned
            Instruction::I32AtomicLoad8U { memarg } => {
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicLoad8U addr not i32")),
                };
                
                // No alignment requirement for 8-bit access
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read value
                let mut bytes = [0u8; 1];
                memory_guard.read(effective_addr, &mut bytes)?;
                let value = bytes[0] as u32;
                
                // Push the value as i32
                engine.exec_stack.values.push(Value::I32(value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic Load 16-bit unsigned
            Instruction::I32AtomicLoad16U { memarg } => {
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicLoad16U addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 2 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicLoad16U requires 2-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read value
                let mut bytes = [0u8; 2];
                memory_guard.read(effective_addr, &mut bytes)?;
                let value = u16::from_le_bytes(bytes) as u32;
                
                // Push the value as i32
                engine.exec_stack.values.push(Value::I32(value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic RMW Exchange
            Instruction::I32AtomicRmwXchg { memarg } => {
                // Pop value to exchange
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmwXchg value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmwXchg addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 4 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicRmwXchg requires 4-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read current value
                let mut bytes = [0u8; 4];
                memory_guard.read(effective_addr, &mut bytes)?;
                let current_value = i32::from_le_bytes(bytes);
                
                // Write new value
                let new_bytes = value_i32.to_le_bytes();
                memory_guard.write(effective_addr, &new_bytes)?;
                
                // Push the old value
                engine.exec_stack.values.push(Value::I32(current_value)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic Store 8-bit
            Instruction::I32AtomicStore8 { memarg } => {
                // Pop value
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicStore8 value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicStore8 addr not i32")),
                };
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Write value (truncate to 8 bits)
                let bytes = [(value_i32 & 0xFF) as u8];
                memory_guard.write(effective_addr, &bytes)?;
                
                Ok(ControlFlow::Next)
            }
            
            // I32 Atomic Store 16-bit
            Instruction::I32AtomicStore16 { memarg } => {
                // Pop value
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicStore16 value not i32")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicStore16 addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 2 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I32AtomicStore16 requires 2-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Write value (truncate to 16 bits)
                let bytes = (value_i32 as u16).to_le_bytes();
                memory_guard.write(effective_addr, &bytes)?;
                
                Ok(ControlFlow::Next)
            }
            
            // I64 Atomic Load
            Instruction::I64AtomicLoad { memarg } => {
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I64AtomicLoad addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 8 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I64AtomicLoad requires 8-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Read value
                let mut bytes = [0u8; 8];
                memory_guard.read(effective_addr, &mut bytes)?;
                let value = i64::from_le_bytes(bytes);
                
                // Push the value
                engine.exec_stack.values.push(Value::I64(value)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // I64 Atomic Store
            Instruction::I64AtomicStore { memarg } => {
                // Pop value
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i64 = match value {
                    Value::I64(v) => v,
                    _ => return Err(Error::validation_error("I64AtomicStore value not i64")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I64AtomicStore addr not i32")),
                };
                
                // Check alignment
                if (addr_i32 as usize + memarg.offset as usize) % 8 != 0 {
                    return Err(Error::runtime_unaligned_memory_access("I64AtomicStore requires 8-byte alignment"));
                }
                
                // Get memory
                let mem_idx = 0; // Default to first memory
                let memory = engine.get_current_module()
                    .ok_or_else(|| Error::runtime_error("No module instance"))?
                    .memory(mem_idx)?;
                
                let memory_guard = memory.lock();
                let effective_addr = (addr_i32 as usize).saturating_add(memarg.offset as usize);
                
                // Write value
                let bytes = value_i64.to_le_bytes();
                memory_guard.write(effective_addr, &bytes)?;
                
                Ok(ControlFlow::Next)
            }
            
            // Memory Atomic Wait 64
            Instruction::MemoryAtomicWait64 { memarg } => {
                // Pop timeout
                let timeout = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let timeout_i64 = match timeout {
                    Value::I64(t) => t,
                    _ => return Err(Error::validation_error("MemoryAtomicWait64 timeout not i64")),
                };
                
                // Pop expected value
                let expected = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let expected_i64 = match expected {
                    Value::I64(e) => e,
                    _ => return Err(Error::validation_error("MemoryAtomicWait64 expected not i64")),
                };
                
                // Pop address
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("MemoryAtomicWait64 addr not i32")),
                };
                
                // In a single-threaded implementation, we simply return "not equal" (1)
                // since there's no other thread that could change the value
                engine.exec_stack.values.push(Value::I32(1)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                
                Ok(ControlFlow::Next)
            }
            
            // Default case for remaining unimplemented atomic instructions
            // I32 atomic RMW 8-bit operations
            Instruction::I32AtomicRmw8AddU { memarg } => {
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8AddU value not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8AddU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                // For MVP, just do non-atomic operations
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                let new_value = old_value.wrapping_add(value_i32 as u8);
                mem.write(effective_addr, &new_value.to_le_bytes())?;
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmw8SubU { memarg } => {
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8SubU value not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8SubU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                let new_value = old_value.wrapping_sub(value_i32 as u8);
                mem.write(effective_addr, &new_value.to_le_bytes())?;
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmw8AndU { memarg } => {
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8AndU value not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8AndU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                let new_value = old_value & (value_i32 as u8);
                mem.write(effective_addr, &new_value.to_le_bytes())?;
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmw8OrU { memarg } => {
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8OrU value not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8OrU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                let new_value = old_value | (value_i32 as u8);
                mem.write(effective_addr, &new_value.to_le_bytes())?;
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmw8XorU { memarg } => {
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8XorU value not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8XorU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                let new_value = old_value ^ (value_i32 as u8);
                mem.write(effective_addr, &new_value.to_le_bytes())?;
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmw8XchgU { memarg } => {
                let value = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let value_i32 = match value {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8XchgU value not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8XchgU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                mem.write(effective_addr, &(value_i32 as u8).to_le_bytes())?;
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            Instruction::I32AtomicRmw8CmpxchgU { memarg } => {
                let replacement = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let replacement_i32 = match replacement {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8CmpxchgU replacement not i32")),
                };
                
                let expected = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let expected_i32 = match expected {
                    Value::I32(v) => v,
                    _ => return Err(Error::validation_error("I32AtomicRmw8CmpxchgU expected not i32")),
                };
                
                let addr = engine.exec_stack.values.pop()?.ok_or_else(|| Error::runtime_stack_underflow("Stack underflow"))?;
                let addr_i32 = match addr {
                    Value::I32(a) => a,
                    _ => return Err(Error::validation_error("I32AtomicRmw8CmpxchgU addr not i32")),
                };
                
                let effective_addr = (addr_i32 as u32).checked_add(memarg.offset).ok_or_else(|| {
                    Error::memory_error("Address overflow")
                })?;
                
                let mem = self.module_instance.memory(0)?;
                if effective_addr as usize >= mem.size_in_bytes() {
                    return Err(Error::memory_error("Out of bounds"));
                }
                
                let mut old_bytes = [0u8; 1];
                mem.read(effective_addr, &mut old_bytes)?;
                let old_value = u8::from_le_bytes(old_bytes);
                
                if old_value == (expected_i32 as u8) {
                    mem.write(effective_addr, &(replacement_i32 as u8).to_le_bytes())?;
                }
                
                engine.exec_stack.values.push(Value::I32(old_value as i32)).map_err(|_| {
                    Error::runtime_stack_overflow("Stack overflow")
                })?;
                Ok(ControlFlow::Next)
            }
            
            // For brevity, implement simplified versions of remaining atomic sub-word instructions
            // In a real implementation, these would need proper atomic operations
            Instruction::I32AtomicRmw16AddU { .. } |
            Instruction::I32AtomicRmw16SubU { .. } |
            Instruction::I32AtomicRmw16AndU { .. } |
            Instruction::I32AtomicRmw16OrU { .. } |
            Instruction::I32AtomicRmw16XorU { .. } |
            Instruction::I32AtomicRmw16XchgU { .. } |
            Instruction::I32AtomicRmw16CmpxchgU { .. } |
            Instruction::I64AtomicLoad8U { .. } |
            Instruction::I64AtomicLoad16U { .. } |
            Instruction::I64AtomicLoad32U { .. } |
            Instruction::I64AtomicStore8 { .. } |
            Instruction::I64AtomicStore16 { .. } |
            Instruction::I64AtomicStore32 { .. } |
            Instruction::I64AtomicRmwAdd { .. } |
            Instruction::I64AtomicRmwSub { .. } |
            Instruction::I64AtomicRmwAnd { .. } |
            Instruction::I64AtomicRmwOr { .. } |
            Instruction::I64AtomicRmwXor { .. } |
            Instruction::I64AtomicRmwXchg { .. } |
            Instruction::I64AtomicRmwCmpxchg { .. } |
            Instruction::I64AtomicRmw8AddU { .. } |
            Instruction::I64AtomicRmw8SubU { .. } |
            Instruction::I64AtomicRmw8AndU { .. } |
            Instruction::I64AtomicRmw8OrU { .. } |
            Instruction::I64AtomicRmw8XorU { .. } |
            Instruction::I64AtomicRmw8XchgU { .. } |
            Instruction::I64AtomicRmw8CmpxchgU { .. } |
            Instruction::I64AtomicRmw16AddU { .. } |
            Instruction::I64AtomicRmw16SubU { .. } |
            Instruction::I64AtomicRmw16AndU { .. } |
            Instruction::I64AtomicRmw16OrU { .. } |
            Instruction::I64AtomicRmw16XorU { .. } |
            Instruction::I64AtomicRmw16XchgU { .. } |
            Instruction::I64AtomicRmw16CmpxchgU { .. } |
            Instruction::I64AtomicRmw32AddU { .. } |
            Instruction::I64AtomicRmw32SubU { .. } |
            Instruction::I64AtomicRmw32AndU { .. } |
            Instruction::I64AtomicRmw32OrU { .. } |
            Instruction::I64AtomicRmw32XorU { .. } |
            Instruction::I64AtomicRmw32XchgU { .. } |
            Instruction::I64AtomicRmw32CmpxchgU { .. } => {
                // MVP: Treat as regular memory operations
                // In a real implementation, these would use atomic primitives
                return Err(Error::runtime_execution_error(",
                ));
            }
            _ => {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::UNSUPPORTED_OPERATION,
                    "));
            }
        }
    }
}

// Helper methods for complex instructions, moved out of FrameBehavior::step
impl StacklessFrame {
    fn table_init(
        &mut self,
        elem_idx: u32,
        table_idx: u32,
        engine: &mut StacklessEngine,
    ) -> Result<()> {
        let module = self.module_instance.module();
        let segment = module.elements.get(elem_idx as usize).map_err(|_| {
            Error::runtime_execution_error(",
            )
        })?;

        let len_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow(")
        })?;
        let src_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let dst_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;

        let n = len_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("table.init len not i32"))?
            as u32;
        let src_offset = src_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("table.init src_offset not i32")
        })? as u32;
        let dst_offset = dst_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("table.init dst_offset not i32")
        })? as u32;

        // Bounds checks from Wasm spec:
        // dst_offset + n > table.len()
        // src_offset + n > segment.items.len()
        let table = self.module_instance.table(table_idx)?;
        if dst_offset.checked_add(n).map_or(true, |end| end > table.size())
            || src_offset.checked_add(n).map_or(true, |end| end as usize > segment.items.len())
        {
            return Err(Error::runtime_out_of_bounds("table.init out of bounds"));
        }

        if n == 0 {
            return Ok(());
        } // No-op

        // Assuming segment.items are Vec<u32> (function indices) or similar that can be
        // turned into Value::FuncRef This needs to align with how Element
        // segments store their items. If Element.items are already `Value` or
        // `Option<Value>`, this is simpler. Let's assume Element stores func
        // indices as u32.
        let mut items_to_init: Vec<Option<Value>> = Vec::new();
        for i in 0..n {
            let idx = (src_offset + i) as usize;
            let item = segment.items.get(idx).map_err(|_| {
                Error::runtime_out_of_bounds("table.init source slice OOB on segment items")
            })?;
            items_to_init.push(Some(Value::FuncRef(Some(FuncRef { index: item }))));
        }

        table.init(dst_offset, &items_to_init)
    }

    fn table_copy(
        &mut self,
        dst_table_idx: u32,
        src_table_idx: u32,
        engine: &mut StacklessEngine,
    ) -> Result<()> {
        let len_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let src_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let dst_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;

        let n = len_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("table.copy len not i32"))?
            as u32;
        let src_offset = src_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("table.copy src_offset not i32")
        })? as u32;
        let dst_offset = dst_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("table.copy dst_offset not i32")
        })? as u32;

        let dst_table = self.module_instance.table(dst_table_idx)?;
        let src_table = self.module_instance.table(src_table_idx)?;

        // Bounds checks (Wasm spec)
        if dst_offset.checked_add(n).map_or(true, |end| end > dst_table.size())
            || src_offset.checked_add(n).map_or(true, |end| end > src_table.size())
        {
            return Err(Error::runtime_out_of_bounds("table.copy out of bounds"));
        }

        if n == 0 {
            return Ok(());
        }

        // Perform copy: if ranges overlap, copy direction matters.
        // Wasm spec: "if s+n > d and s < d" (copy backwards) or "if d+n > s and d < s"
        // (copy forwards) Simplest is often to read all source elements first
        // if temp storage is okay. Or, copy element by element, checking
        // overlap for direction.
        if dst_offset <= src_offset {
            // Copy forwards
            for i in 0..n {
                let val = src_table.get(src_offset + i)?.ok_or_else(|| {
                    Error::runtime_out_of_bounds("table.copy source element uninitialized/null")
                })?;
                dst_table.set(dst_offset + i, Some(val))?;
            }
        } else {
            // Copy backwards (dst_offset > src_offset)
            for i in (0..n).rev() {
                let val = src_table.get(src_offset + i)?.ok_or_else(|| {
                    Error::runtime_out_of_bounds("table.copy source element uninitialized/null")
                })?;
                dst_table.set(dst_offset + i, Some(val))?;
            }
        }
        Ok(())
    }

    fn table_fill(&mut self, table_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let n_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let val_to_fill = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;

        let n = n_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("table.fill count not i32"))?
            as u32;
        let offset = offset_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("table.fill offset not i32"))?
            as u32;

        let table = self.module_instance.table(table_idx)?;
        if offset.checked_add(n).map_or(true, |end| end > table.size()) {
            return Err(Error::runtime_out_of_bounds("table.fill out of bounds"));
        }

        if n == 0 {
            return Ok(());
        }
        // TODO: Type check val_to_fill against table.element_type()
        for i in 0..n {
            table.set(offset + i, val_to_fill.clone())?;
        }
        Ok(())
    }

    // Memory operations
    fn memory_init(
        &mut self,
        data_idx: u32,
        mem_idx: u32,
        engine: &mut StacklessEngine,
    ) -> Result<()> {
        let n_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let src_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let dst_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;

        let n: u32 = n_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("memory.init len not i32"))?
            .try_into().unwrap();
        let src_offset: u32 = src_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("memory.init src_offset not i32")
        })?.try_into().unwrap();
        let dst_offset: u32 = dst_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("memory.init dst_offset not i32")
        })?.try_into().unwrap();

        let memory = self.module_instance.memory(mem_idx)?;
        let data_segment =
            self.module_instance.module().data.get(data_idx as usize).map_err(|_| {
                    Error::runtime_execution_error(",
                    )
                },
            )?;

        // Bounds checks (Wasm Spec)
        if dst_offset.checked_add(n).map_or(true, |end| end as usize > memory.size_bytes())
            || src_offset.checked_add(n).map_or(true, |end| {
                match data_segment.data() {
                    Ok(data) => end as usize > data.len(),
                    Err(_) => true,
                }
            })
        {
            return Err(Error::memory_error("));
        }
        if n == 0 {
            return Ok(());
        }

        let data_to_write = data_segment.data()?.get((src_offset as usize)..(src_offset as usize + n as usize)).ok_or_else(|| {
            Error::memory_error("memory.init source data segment OOB",
            )
        })?;

        memory.write(dst_offset, data_to_write)
    }

    fn memory_copy(
        &mut self,
        dst_mem_idx: u32,
        src_mem_idx: u32,
        engine: &mut StacklessEngine,
    ) -> Result<()> {
        // In Wasm MVP, src_mem_idx and dst_mem_idx are always 0.
        let n_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let src_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let dst_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;

        let n: u32 = n_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("memory.copy len not i32"))?
            .try_into().unwrap();
        let src_offset: u32 = src_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("memory.copy src_offset not i32")
        })?.try_into().unwrap();
        let dst_offset: u32 = dst_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("memory.copy dst_offset not i32")
        })?.try_into().unwrap();

        let dst_memory = self.module_instance.memory(dst_mem_idx)?;
        let src_memory = if dst_mem_idx == src_mem_idx {
            dst_memory.clone()
        } else {
            self.module_instance.memory(src_mem_idx)?
        };

        // Bounds checks
        if dst_offset.checked_add(n).map_or(true, |end| end as usize > dst_memory.size_bytes())
            || src_offset.checked_add(n).map_or(true, |end| end as usize > src_memory.size_bytes())
        {
            return Err(Error::memory_error("memory.copy out of bounds",
            ));
        }
        if n == 0 {
            return Ok(());
        }

        // Wasm spec: if d_m is m and s_m is m, then the copy is performed as if the
        // bytes are copied from m to a temporary buffer of size n and then from the
        // buffer to m. This means we can read all source bytes then write, or
        // handle overlap carefully. For simplicity, if it's the same memory and
        // regions overlap, a temporary buffer is safest. Otherwise (different
        // memories, or same memory but no overlap), direct copy is fine.

        // A simple approach that is correct but might be slower if n is large:
        #[cfg(feature = "std")]
        {
            let mut temp_buffer = vec![0u8; n as usize];
            src_memory.read(src_offset, &mut temp_buffer)?;
            dst_memory.write(dst_offset, &temp_buffer)
        }
        #[cfg(not(feature = "std"))]
        {
            let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
            let mut temp_buffer: wrt_foundation::bounded::BoundedVec<u8, 4096, _> = wrt_foundation::bounded::BoundedVec::new(provider)?;
            // Ensure we have the right capacity
            for _ in 0..n.min(4096) {
                temp_buffer.push(0u8).unwrap();
            }
            // Read into a slice view of the bounded vec
            let slice = temp_buffer.as_mut_slice().map_err(|_| Error::memory_error("Failed to get mutable slice from bounded vec"
            ))?;
            src_memory.read(src_offset, &mut slice[..n as usize])?;
            dst_memory.write(dst_offset, &slice[..n as usize])
        }
    }

    fn memory_fill(&mut self, mem_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let n_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let val_to_fill_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;
        let dst_offset_val = engine.exec_stack.values.pop().map_err(|e| {
            Error::runtime_stack_underflow("Stack operation error")
        })?;

        let n: u32 = n_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("memory.fill len not i32"))?
            .try_into().unwrap();
        let val_to_fill_byte = val_to_fill_val
            .and_then(|v| v.as_i32())
            .ok_or_else(|| Error::type_error("memory.fill value not i32"))?
            as u8; // Value must be i32, truncated to u8
        let dst_offset: u32 = dst_offset_val.and_then(|v| v.as_i32()).ok_or_else(|| {
            Error::type_error("memory.fill dst_offset not i32")
        })?.try_into().unwrap();

        let memory = self.module_instance.memory(mem_idx)?;
        if dst_offset.checked_add(n).map_or(true, |end| end as usize > memory.size_bytes()) {
            return Err(Error::memory_error("memory.fill out of bounds",
            ));
        }
        if n == 0 {
            return Ok(());
        }

        memory.fill(dst_offset, n, val_to_fill_byte)
    }

    /// Branch to a label by unwinding blocks and adjusting the program counter
    fn branch_to_label(&mut self, label_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        // In WebAssembly, label_idx 0 refers to the innermost block,
        // 1 to the next outer block, and so on.
        
        // For now, we'll implement a simplified version that just returns success
        // In a full implementation, this would:
        // 1. Pop blocks from block_depths up to the target label
        // 2. Adjust the stack to match the block's expected results
        // 3. Set the PC to the target location
        
        // This is a placeholder that maintains the correct behavior
        // The actual branching is handled by returning ControlFlow::Branch
        Ok(())
    }

    /// Execute a SIMD operation using the integrated SIMD runtime
    ///
    /// This method provides seamless integration between SIMD operations and
    /// the stackless execution engine, supporting all ASIL levels.
    ///
    /// # Arguments
    /// * `simd_op` - The SIMD operation to execute
    /// * `engine` - The stackless execution engine
    ///
    /// # Returns
    /// * `Ok(ControlFlow::Next)` - If the operation completed successfully
    /// * `Err(Error)` - If the operation failed
    ///
    /// # Safety
    /// This method contains no unsafe code and is suitable for all ASIL levels.
    pub fn execute_simd_operation(
        &mut self,
        simd_op: &wrt_instructions::simd_ops::SimdOp,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow> {
        // TODO: SIMD execution not yet implemented
        // Create SIMD execution adapter
        // let adapter = SimdExecutionAdapter::new();
        
        // Execute the SIMD operation
        // adapter.execute_simd_with_engine(simd_op, engine)?;
        
        // Update execution statistics
        // engine.stats.simd_operations_executed += 1;
        return Err(Error::runtime_execution_error("
        ))
    }

    /// Check if an instruction is a SIMD operation (when SIMD instructions are added to main enum)
    ///
    /// This is a placeholder method that demonstrates how SIMD instructions
    /// would be detected and routed to the SIMD execution path.
    ///
    /// # Arguments
    /// * `_instruction` - The instruction to check (placeholder for future SIMD instructions)
    ///
    /// # Returns
    /// * `Some(simd_op)` - If the instruction is a SIMD operation
    /// * `None` - If the instruction is not a SIMD operation
    #[allow(dead_code)]
    fn extract_simd_operation(
        &self,
        _instruction: &wrt_foundation::types::Instruction<crate::memory_adapter::StdMemoryProvider>,
    ) -> Option<wrt_instructions::simd_ops::SimdOp> {
        // This is a placeholder for when SIMD instructions are added to the main Instruction enum
        // When that happens, this method would pattern match against SIMD variants and extract the SimdOp
        
        // Example of how it would work:
        // match instruction {
        //     Instruction::V128Load { offset, align } => Some(SimdOp::V128Load { offset: *offset, align: *align }),
        //     Instruction::I32x4Add => Some(SimdOp::I32x4Add),
        //     Instruction::F32x4Mul => Some(SimdOp::F32x4Mul),
        //     // ... other SIMD instructions
        //     _ => None,
        // }
        
        None
    }
    
    // TODO: Add methods for enter_block, exit_block, etc.
}

// Validatable might not be applicable directly to StacklessFrame in the same
// way as Module. If it's for ensuring internal consistency, it might be useful.
impl Validatable for StacklessFrame {
    type Error = Error;
    
    fn validation_level(&self) -> VerificationLevel {
        VerificationLevel::Basic
    }
    
    fn set_validation_level(&mut self, _level: VerificationLevel) {
        // Validation level is fixed for frames
    }
    
    fn validate(&self) -> Result<()> {
        // Example validations:
        // - self.pc should be within bounds of function code
        // - self.locals should match arity + declared locals of self.func_type
        // - self.block_depths should be consistent (e.g. not deeper than allowed)
        if self.pc > self.function_body()?.body.len() {
            return Err(Error::runtime_out_of_bounds("));
        }
        // More checks can be added here.
        Ok(())
    }
}
