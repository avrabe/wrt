#![allow(unsafe_code)] // Temporary: Allow unsafe code for placeholder logic
//! Stackless function activation frame

use crate::prelude::*;
use core::fmt::Debug;

// Imports from wrt crates
use wrt_error::{codes, Error};
use wrt_decoder::instructions::Instruction;
use wrt_types::{
    BoundedCapacity, Validatable, VerificationLevel, FuncType, BlockType, Value, ValueType,
    safe_memory::SafeSlice, // Added SafeSlice
};
use wrt_types::values::FuncRef; // Added FuncRef

// Internal imports
use super::engine::StacklessEngine;
use crate::{
    global::Global,
    module::{Data, Element, Function, Module}, // Module is already in prelude
    memory::Memory,
    module_instance::ModuleInstance,
    stackless::StacklessStack, // Added StacklessStack
    table::Table,
};

// Re-export Label for convenience if it's commonly used with frames
pub use wrt_instructions::control_ops::BranchTarget as Label;


/// Defines the behavior of a function activation frame in the stackless engine.
pub trait FrameBehavior {
    /// Returns the current program counter (instruction offset within the function body).
    fn pc(&self) -> usize;

    /// Returns a mutable reference to the program counter.
    fn pc_mut(&mut self) -> &mut usize;

    /// Returns a slice of the local variables for the current frame.
    /// This includes function arguments followed by declared local variables.
    fn locals(&self) -> &SafeSlice<Value>;

    /// Returns a mutable slice of the local variables.
    fn locals_mut(&mut self) -> &mut SafeSlice<Value>;

    /// Returns a reference to the module instance this frame belongs to.
    fn module_instance(&self) -> &Arc<ModuleInstance>;

    /// Returns the index of the function this frame represents.
    fn function_index(&self) -> u32;

    /// Returns the type (signature) of the function this frame represents.
    fn function_type(&self) -> &FuncType;

    /// Returns the arity (number of return values) of the function.
    fn arity(&self) -> usize;

    /// Advances execution by one instruction.
    ///
    /// # Arguments
    ///
    /// * `engine`: The stackless engine, providing access to global state like
    ///           the value stack and call stack.
    ///
    /// # Returns
    ///
    /// * `Ok(ControlFlow)`: Indicates the outcome of the instruction execution,
    ///                      e.g., proceed to next instruction, call, return.
    /// * `Err(Error)`: If an error occurs during execution.
    fn step(&mut self, engine: &mut StacklessEngine) -> Result<ControlFlow>; // ControlFlow to be defined
}

/// Represents the control flow outcome of an instruction's execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlFlow {
    /// Continue to the next instruction in the current frame.
    Next,
    /// A function call has been made. A new frame will be pushed.
    Call { func_idx: u32, inputs: Vec<Value> }, // Simplified for now
    /// The current function is returning. The current frame will be popped.
    Return { values: Vec<Value> },
    /// A branch to a given PC offset within the current function.
    Branch(usize),
    /// Trap / Unreachable instruction.
    Trap(Error),
}


/// Stackless function activation frame.
#[derive(Debug, Clone)]
pub struct StacklessFrame {
    /// Program counter: offset into the function's instruction stream.
    pc: usize,
    /// Local variables (includes arguments).
    locals: SafeSlice<Value>, // Max 65536 locals as per Wasm spec, adjust SafeSlice capacity
    /// Reference to the module instance.
    module_instance: Arc<ModuleInstance>,
    /// Index of the function in the module.
    func_idx: u32,
    /// Type of the function.
    func_type: FuncType,
    /// Arity of the function (number of result values).
    arity: usize,
    /// Block depths for control flow.
    block_depths: Vec<BlockContext>, // Manages block context (pc, stack depth)
}

/// Context for a control flow block (block, loop, if).
#[derive(Debug, Clone)]
struct BlockContext {
    /// The type of the block.
    block_type: BlockType,
    /// Program counter to jump to when exiting this block (e.g., `end` of block/if/loop).
    end_pc: usize,
    /// Program counter for the `else` branch of an `if` block.
    else_pc: Option<usize>,
    /// Stack depth before entering the block, to know how many values to pop/truncate.
    stack_depth_before: usize,
    /// Value stack depth before parameters were pushed (for block/loop results)
    value_stack_depth_before_params: usize,
    /// Arity of the block (number of result values it's expected to push).
    arity: usize,
}


impl StacklessFrame {
    /// Creates a new stackless function frame.
    ///
    /// # Arguments
    ///
    /// * `func_ref`: A reference to the function to be called.
    /// * `module_instance`: The module instance this function belongs to.
    /// * `invocation_inputs`: Values passed as arguments to this function call.
    /// * `max_locals`: Maximum number of locals expected (for SafeSlice preallocation).
    pub fn new(
        func_ref: FuncRef,
        module_instance: Arc<ModuleInstance>,
        invocation_inputs: &[Value], // Changed to slice
        max_locals: usize, // Example: pass from engine config or calculate
    ) -> Result<Self> {
        let func_idx = func_ref.index;
        let func_type = module_instance.function_type(func_idx)?;

        let mut locals_vec: Vec<Value> = invocation_inputs.to_vec();

        // Append default values for declared locals
        if let Some(function_body) = module_instance.module().functions.get(func_idx as usize) {
            for local_entry in &function_body.locals {
                // local_entry is (count, ValueType) in the Module's Function struct
                // Assuming Function struct in module.rs has: pub locals: Vec<(u32, ValueType)>,
                let count = local_entry.0;
                let val_type = local_entry.1;
                for _ in 0..count {
                    locals_vec.push(Value::default_for_type(&val_type));
                }
            }
        } else {
            return Err(Error::new(
                codes::RUNTIME_FUNCTION_NOT_FOUND,
                format!("Function body not found for index {}", func_idx),
            ));
        }

        let locals = SafeSlice::new(&locals_vec, VerificationLevel::High)
            .map_err(|e| Error::new(codes::RUNTIME_INVALID_STATE, format!("Failed to create SafeSlice for locals: {}", e)))?;
        
        if locals.len() > max_locals {
             // This check is more for sizing SafeSlice correctly if it had a fixed capacity.
             // If SafeSlice dynamically grows or `new` takes a capacity, adjust this.
            return Err(Error::new(codes::RUNTIME_INVALID_STATE, "Too many locals for configured max_locals"));
        }


        Ok(Self {
            pc: 0,
            locals,
            module_instance,
            func_idx,
            arity: func_type.results.len(),
            func_type,
            block_depths: Vec::new(),
        })
    }

    // Helper to get the actual function body from the module instance
    fn function_body(&self) -> Result<&crate::module::Function> {
        self.module_instance
            .module()
            .functions
            .get(self.func_idx as usize)
            .ok_or_else(|| {
                Error::new(
                    codes::RUNTIME_FUNCTION_NOT_FOUND,
                    format!("Function body not found for index {}", self.func_idx),
                )
            })
    }
}

impl FrameBehavior for StacklessFrame {
    fn pc(&self) -> usize {
        self.pc
    }

    fn pc_mut(&mut self) -> &mut usize {
        &mut self.pc
    }

    fn locals(&self) -> &SafeSlice<Value> {
        &self.locals
    }

    fn locals_mut(&mut self) -> &mut SafeSlice<Value> {
        &mut self.locals
    }

    fn module_instance(&self) -> &Arc<ModuleInstance> {
        &self.module_instance
    }

    fn function_index(&self) -> u32 {
        self.func_idx
    }

    fn function_type(&self) -> &FuncType {
        &self.func_type
    }
    
    fn arity(&self) -> usize {
        self.arity
    }

    fn step(&mut self, engine: &mut StacklessEngine) -> Result<ControlFlow> {
        let func_body = self.function_body()?;
        let instructions = &func_body.code; // Assuming Function struct has `code: Vec<Instruction>`

        if self.pc >= instructions.len() {
            // If PC is at or beyond the end, and it's not a trap/return already handled,
            // it implies a fallthrough return for a void function or a missing explicit return.
            if self.arity == 0 {
                 // Implicit return for void function
                return Ok(ControlFlow::Return { values: Vec::new() });
            } else {
                return Err(Error::new(
                    codes::RUNTIME_MISSING_RETURN_VALUE,
                    "Function ended without returning expected values",
                ));
            }
        }

        let instruction = &instructions[self.pc];
        self.pc += 1;

        // --- Execute Instruction ---
        // This is where the large match statement for all instructions will go.
        // For now, a placeholder.
        match instruction {
            Instruction::Unreachable => {
                Ok(ControlFlow::Trap(Error::new(
                    codes::RUNTIME_UNREACHABLE,
                    "Unreachable instruction executed",
                )))
            }
            Instruction::Nop => Ok(ControlFlow::Next),
            Instruction::Block(_block_type_idx) => {
                // TODO: Resolve block_type_idx to BlockType from module
                // TODO: Push BlockContext to self.block_depths
                // Placeholder:
                // let block_type = self.module_instance.get_block_type(block_type_idx)?;
                // self.enter_block(block_type, engine.value_stack.len(), self.pc + ??? /* end_pc */, None);
                // Ok(ControlFlow::Next)
                todo!("Block instruction")
            }
            Instruction::Loop(_block_type_idx) => {
                // TODO: Similar to Block, but branches go to start of loop
                todo!("Loop instruction")
            }
            Instruction::If(_block_type_idx) => {
                // TODO: Pop condition. If true, proceed. If false, jump to else or end.
                // let condition = engine.value_stack.pop()?.as_i32()? != 0;
                // if condition { ... } else { self.pc = else_pc_or_end_pc; }
                todo!("If instruction")
            }
            Instruction::Else => {
                // TODO: Jump to end of current If block's 'then' part.
                // let current_block = self.block_depths.last().ok_or_else(...)?;
                // self.pc = current_block.end_pc;
                todo!("Else instruction")
            }
            Instruction::End => {
                // TODO: Pop BlockContext. Handle block results.
                // self.exit_block(engine)?;
                // Check if this is the end of the function itself
                if self.block_depths.is_empty() {
                    // This 'end' corresponds to the function body's implicit block.
                    // Values for return should be on the stack matching self.arity.
                    let mut return_values = Vec::with_capacity(self.arity);
                    for _ in 0..self.arity {
                        return_values.push(engine.value_stack.pop()
                            .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow on function return: {}", e)))?);
                    }
                    return_values.reverse(); // Values are popped in reverse order
                    return Ok(ControlFlow::Return { values: return_values });
                }
                Ok(ControlFlow::Next) // Continue if it's a nested block's end
            }
            Instruction::Br(label_idx) => {
                // TODO: Jump to label_idx (relative depth)
                // self.branch_to_label(*label_idx, engine)?;
                // Ok(ControlFlow::Branch(target_pc))
                todo!("Br instruction: label_idx={}", label_idx)
            }
            Instruction::BrIf(label_idx) => {
                // TODO: Pop condition. If true, Br(label_idx).
                todo!("BrIf instruction: label_idx={}", label_idx)
            }
            // ... other control flow instructions ...
            Instruction::Return => {
                let mut return_values = Vec::with_capacity(self.arity);
                 for _ in 0..self.arity {
                    return_values.push(engine.value_stack.pop()
                        .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow on explicit return: {}", e)))?);
                }
                return_values.reverse();
                Ok(ControlFlow::Return { values: return_values })
            }
            Instruction::Call(func_idx_val) => {
                // TODO: Pop arguments from stack according to target function type
                // let target_func_type = self.module_instance.function_type(*func_idx_val)?;
                // let mut args = Vec::with_capacity(target_func_type.params.len());
                // for _ in 0..target_func_type.params.len() { args.push(engine.value_stack.pop()?); }
                // args.reverse();
                // Ok(ControlFlow::Call { func_idx: *func_idx_val, inputs: args })
                todo!("Call instruction: func_idx={}", func_idx_val)
            }
            Instruction::CallIndirect(type_idx, table_idx) => {
                 // 1. Pop function index `elem_idx` from stack.
                 // 2. Validate `elem_idx` against table `table_idx`.
                 // 3. Get `FuncRef` from `table[elem_idx]`. If null, trap.
                 // 4. Get actual `func_idx` from `FuncRef`.
                 // 5. Get `target_func_type` using `self.module_instance.function_type(actual_func_idx)`.
                 // 6. Get `expected_func_type` from `self.module_instance.module().types[type_idx]`.
                 // 7. If types don't match, trap.
                 // 8. Pop args, Ok(ControlFlow::Call { func_idx: actual_func_idx, inputs: args })
                todo!("CallIndirect: type_idx={}, table_idx={}", type_idx, table_idx)
            }

            // Local variable instructions
            Instruction::LocalGet(local_idx) => {
                let value = self.locals.get(*local_idx as usize).cloned().ok_or_else(|| {
                    Error::new(codes::RUNTIME_INVALID_LOCAL_INDEX, format!("Invalid local index {} for get", local_idx))
                })?;
                engine.value_stack.push(value)
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on local.get: {}", e)))?;
                Ok(ControlFlow::Next)
            }
            Instruction::LocalSet(local_idx) => {
                let value = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow on local.set: {}", e)))?;
                self.locals.set(*local_idx as usize, value).map_err(|e| {
                     Error::new(codes::RUNTIME_INVALID_LOCAL_INDEX, format!("Invalid local index {} for set: {}", local_idx, e))
                })?;
                Ok(ControlFlow::Next)
            }
            Instruction::LocalTee(local_idx) => {
                let value = engine.value_stack.peek()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow on local.tee: {}", e)))?
                    .clone();
                self.locals.set(*local_idx as usize, value).map_err(|e| {
                     Error::new(codes::RUNTIME_INVALID_LOCAL_INDEX, format!("Invalid local index {} for tee: {}", local_idx, e))
                })?;
                Ok(ControlFlow::Next)
            }

            // Global variable instructions
            Instruction::GlobalGet(global_idx) => {
                let global = self.module_instance.global(*global_idx)?;
                engine.value_stack.push(global.get_value())
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on global.get: {}", e)))?;
                Ok(ControlFlow::Next)
            }
            Instruction::GlobalSet(global_idx) => {
                let global = self.module_instance.global(*global_idx)?;
                if !global.is_mutable() {
                    return Err(Error::new(codes::RUNTIME_GLOBAL_IMMUTABLE, "Cannot set immutable global"));
                }
                let value = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow on global.set: {}", e)))?;
                global.set_value(value)?;
                Ok(ControlFlow::Next)
            }

            // Table instructions
            Instruction::TableGet(table_idx) => {
                let table = self.module_instance.table(*table_idx)?;
                let elem_idx_val = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for TableGet index: {}", e)))?;
                let elem_idx = elem_idx_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "TableGet index not i32"))? as u32;
                
                match table.get(elem_idx)? {
                    Some(val) => engine.value_stack.push(val)
                        .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on TableGet: {}",e)))?,
                    None => return Err(Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "TableGet returned None (null ref or OOB)")), // Or specific error for null if needed
                }
                Ok(ControlFlow::Next)
            }
            Instruction::TableSet(table_idx) => {
                let table = self.module_instance.table(*table_idx)?;
                let val_to_set = engine.value_stack.pop()
                     .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for TableSet value: {}", e)))?;
                let elem_idx_val = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for TableSet index: {}", e)))?;
                let elem_idx = elem_idx_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "TableSet index not i32"))? as u32;

                // TODO: Type check val_to_set against table.element_type()
                table.set(elem_idx, val_to_set)?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableSize(table_idx) => {
                let table = self.module_instance.table(*table_idx)?;
                engine.value_stack.push(Value::I32(table.size() as i32))
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on TableSize: {}",e)))?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableGrow(table_idx) => {
                let table = self.module_instance.table(*table_idx)?;
                let init_val = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for TableGrow init value: {}", e)))?;
                let delta_val = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for TableGrow delta: {}", e)))?;
                let delta = delta_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "TableGrow delta not i32"))? as u32;

                let old_size = table.grow(delta, init_val)?;
                engine.value_stack.push(Value::I32(old_size as i32))
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on TableGrow result: {}",e)))?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableFill(table_idx) => {
                self.table_fill(*table_idx, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableCopy(dst_table_idx, src_table_idx) => {
                self.table_copy(*dst_table_idx, *src_table_idx, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::TableInit(elem_seg_idx, table_idx) => {
                self.table_init(*elem_seg_idx, *table_idx, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::ElemDrop(elem_seg_idx) => {
                self.module_instance.module().drop_element_segment(*elem_seg_idx);
                Ok(ControlFlow::Next)
            }


            // Memory instructions (Placeholders, many need base address + offset)
            // Common pattern: pop address, calculate effective_address, operate on memory.
            // Example: I32Load needs `addr = pop_i32() + offset_immediate`
            //          `value = memory.read_i32(addr)`
            //          `push(value)`
            Instruction::I32Load(_mem_arg) => todo!("I32Load"), // mem_arg contains align and offset
            Instruction::I64Load(_mem_arg) => todo!("I64Load"),
            Instruction::F32Load(_mem_arg) => todo!("F32Load"),
            Instruction::F64Load(_mem_arg) => todo!("F64Load"),
            Instruction::I32Load8S(_mem_arg) => todo!("I32Load8S"),
            Instruction::I32Load8U(_mem_arg) => todo!("I32Load8U"),
            Instruction::I32Load16S(_mem_arg) => todo!("I32Load16S"),
            Instruction::I32Load16U(_mem_arg) => todo!("I32Load16U"),
            Instruction::I64Load8S(_mem_arg) => todo!("I64Load8S"),
            Instruction::I64Load8U(_mem_arg) => todo!("I64Load8U"),
            Instruction::I64Load16S(_mem_arg) => todo!("I64Load16S"),
            Instruction::I64Load16U(_mem_arg) => todo!("I64Load16U"),
            Instruction::I64Load32S(_mem_arg) => todo!("I64Load32S"),
            Instruction::I64Load32U(_mem_arg) => todo!("I64Load32U"),

            Instruction::I32Store(_mem_arg) => todo!("I32Store"),
            Instruction::I64Store(_mem_arg) => todo!("I64Store"),
            Instruction::F32Store(_mem_arg) => todo!("F32Store"),
            Instruction::F64Store(_mem_arg) => todo!("F64Store"),
            Instruction::I32Store8(_mem_arg) => todo!("I32Store8"),
            Instruction::I32Store16(_mem_arg) => todo!("I32Store16"),
            Instruction::I64Store8(_mem_arg) => todo!("I64Store8"),
            Instruction::I64Store16(_mem_arg) => todo!("I64Store16"),
            Instruction::I64Store32(_mem_arg) => todo!("I64Store32"),

            Instruction::MemorySize(_mem_idx) => { // mem_idx is always 0 in Wasm MVP
                let mem = self.module_instance.memory(0)?; // Assuming memory index 0
                engine.value_stack.push(Value::I32(mem.size_pages() as i32))
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on MemorySize: {}",e)))?;
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryGrow(_mem_idx) => { // mem_idx is always 0 in Wasm MVP
                let mem = self.module_instance.memory(0)?;
                let delta_pages_val = engine.value_stack.pop()
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for MemoryGrow delta: {}", e)))?;
                let delta_pages = delta_pages_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "MemoryGrow delta not i32"))? as u32;
                
                let old_size_pages = mem.grow(delta_pages)?;
                engine.value_stack.push(Value::I32(old_size_pages as i32))
                    .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on MemoryGrow result: {}",e)))?;
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryFill(_mem_idx) => {
                self.memory_fill(0, engine)?; // Assuming memory index 0
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryCopy(_dst_mem_idx, _src_mem_idx) => { // both always 0 in MVP
                self.memory_copy(0, 0, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::MemoryInit(_data_seg_idx, _mem_idx) => { // mem_idx always 0 in MVP
                self.memory_init(*_data_seg_idx, 0, engine)?;
                Ok(ControlFlow::Next)
            }
            Instruction::DataDrop(_data_seg_idx) => {
                self.module_instance.module().drop_data_segment(*_data_seg_idx);
                Ok(ControlFlow::Next)
            }


            // Numeric Const instructions
            Instruction::I32Const(val) => engine.value_stack.push(Value::I32(*val))
                .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on I32Const: {}",e)))?,
            Instruction::I64Const(val) => engine.value_stack.push(Value::I64(*val))
                .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on I64Const: {}",e)))?,
            Instruction::F32Const(val) => engine.value_stack.push(Value::F32(f32::from_bits(*val))) // Assuming val is u32 bits
                .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on F32Const: {}",e)))?,
            Instruction::F64Const(val) => engine.value_stack.push(Value::F64(f64::from_bits(*val))) // Assuming val is u64 bits
                .map_err(|e| Error::new(codes::RUNTIME_STACK_OVERFLOW, format!("Stack overflow on F64Const: {}",e)))?,

            // TODO: Implement all other numeric, reference, parametric, vector instructions
            // For example:
            // Instruction::I32Add => {
            //     let b = engine.value_stack.pop()?.as_i32()?;
            //     let a = engine.value_stack.pop()?.as_i32()?;
            //     engine.value_stack.push(Value::I32(a.wrapping_add(b)))?;
            // }
            // Instruction::Drop => { engine.value_stack.pop()?; }
            // Instruction::Select => { ... }
            // Instruction::RefNull(heap_type) => { ... }
            // Instruction::RefIsNull => { ... }
            // Instruction::RefFunc(func_idx) => { ... }

            _ => {
                return Err(Error::new(
                    codes::RUNTIME_UNSUPPORTED_INSTRUCTION,
                    format!("Instruction {:?} not yet implemented in StacklessFrame::step", instruction),
                ));
            }
        }
        // If the instruction was handled and didn't return/trap/call/branch:
        if !matches!(instruction, Instruction::Unreachable | Instruction::Return /* | Call | Br... */) {
             Ok(ControlFlow::Next)
        } else {
            // This branch should ideally not be hit if all control flow instrs return their specific ControlFlow variant
            Err(Error::new(codes::RUNTIME_INVALID_STATE, "Unhandled instruction outcome in step"))
        }
    }
}

// Helper methods for complex instructions, moved out of FrameBehavior::step
impl StacklessFrame {
    fn table_init(&mut self, elem_idx: u32, table_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let module = self.module_instance.module();
        let segment = module.elements.get(elem_idx as usize).ok_or_else(|| {
            Error::new(codes::RUNTIME_INVALID_ELEMENT_SEGMENT_INDEX, format!("Invalid element segment index {}", elem_idx))
        })?;

        let len_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for table.init len: {}", e)))?;
        let src_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for table.init src_offset: {}", e)))?;
        let dst_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for table.init dst_offset: {}", e)))?;

        let n = len_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.init len not i32"))? as u32;
        let src_offset = src_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.init src_offset not i32"))? as u32;
        let dst_offset = dst_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.init dst_offset not i32"))? as u32;
        
        // Bounds checks from Wasm spec:
        // dst_offset + n > table.len()
        // src_offset + n > segment.items.len()
        let table = self.module_instance.table(table_idx)?;
        if dst_offset.checked_add(n).map_or(true, |end| end > table.size()) ||
           src_offset.checked_add(n).map_or(true, |end| end as usize > segment.items.len()) {
            return Err(Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "table.init out of bounds"));
        }

        if n == 0 { return Ok(()); } // No-op

        // Assuming segment.items are Vec<u32> (function indices) or similar that can be turned into Value::FuncRef
        // This needs to align with how Element segments store their items.
        // If Element.items are already `Value` or `Option<Value>`, this is simpler.
        // Let's assume Element stores func indices as u32.
        let items_to_init: Vec<Option<Value>> = segment.items
            .get(src_offset as usize .. (src_offset + n) as usize)
            .ok_or_else(|| Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "table.init source slice OOB on segment items"))?
            .iter()
            .map(|&func_idx| Some(Value::FuncRef(Some(FuncRef::from_index(func_idx))))) // Assuming items are u32 func indices
            .collect();
        
        table.init(dst_offset, &items_to_init)
    }

    fn table_copy(&mut self, dst_table_idx: u32, src_table_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let len_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for table.copy len: {}", e)))?;
        let src_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for table.copy src_offset: {}", e)))?;
        let dst_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("Stack underflow for table.copy dst_offset: {}", e)))?;

        let n = len_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.copy len not i32"))? as u32;
        let src_offset = src_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.copy src_offset not i32"))? as u32;
        let dst_offset = dst_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.copy dst_offset not i32"))? as u32;

        let dst_table = self.module_instance.table(dst_table_idx)?;
        let src_table = self.module_instance.table(src_table_idx)?;

        // Bounds checks (Wasm spec)
        if dst_offset.checked_add(n).map_or(true, |end| end > dst_table.size()) ||
           src_offset.checked_add(n).map_or(true, |end| end > src_table.size()) {
            return Err(Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "table.copy out of bounds"));
        }
        
        if n == 0 { return Ok(()); }

        // Perform copy: if ranges overlap, copy direction matters.
        // Wasm spec: "if s+n > d and s < d" (copy backwards) or "if d+n > s and d < s" (copy forwards)
        // Simplest is often to read all source elements first if temp storage is okay.
        // Or, copy element by element, checking overlap for direction.
        if dst_offset <= src_offset { // Copy forwards
            for i in 0..n {
                let val = src_table.get(src_offset + i)?.ok_or_else(|| Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "table.copy source element uninitialized/null"))?;
                dst_table.set(dst_offset + i, val)?;
            }
        } else { // Copy backwards (dst_offset > src_offset)
            for i in (0..n).rev() {
                let val = src_table.get(src_offset + i)?.ok_or_else(|| Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "table.copy source element uninitialized/null")))?;
                dst_table.set(dst_offset + i, val)?;
            }
        }
        Ok(())
    }

    fn table_fill(&mut self, table_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let n_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("table.fill count: {}", e)))?;
        let val_to_fill = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("table.fill value: {}", e)))?;
        let offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("table.fill offset: {}", e)))?;

        let n = n_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.fill count not i32"))? as u32;
        let offset = offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "table.fill offset not i32"))? as u32;

        let table = self.module_instance.table(table_idx)?;
        if offset.checked_add(n).map_or(true, |end| end > table.size()) {
            return Err(Error::new(codes::RUNTIME_TABLE_ACCESS_OUT_OF_BOUNDS, "table.fill out of bounds"));
        }

        if n == 0 { return Ok(()); }
        // TODO: Type check val_to_fill against table.element_type()
        for i in 0..n {
            table.set(offset + i, val_to_fill.clone())?;
        }
        Ok(())
    }
    
    // Memory operations
    fn memory_init(&mut self, data_idx: u32, mem_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let n_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.init len: {}", e)))?;
        let src_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.init src_offset: {}", e)))?;
        let dst_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.init dst_offset: {}", e)))?;

        let n = n_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.init len not i32"))? as usize;
        let src_offset = src_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.init src_offset not i32"))? as usize;
        let dst_offset = dst_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.init dst_offset not i32"))? as usize;
        
        let memory = self.module_instance.memory(mem_idx)?;
        let data_segment = self.module_instance.module().data_segments.get(data_idx as usize)
            .ok_or_else(|| Error::new(codes::RUNTIME_INVALID_DATA_SEGMENT_INDEX, format!("Invalid data segment index {}", data_idx)))?;

        // Bounds checks (Wasm Spec)
        if dst_offset.checked_add(n).map_or(true, |end| end > memory.size_bytes()) ||
           src_offset.checked_add(n).map_or(true, |end| end > data_segment.data.len()) {
            return Err(Error::new(codes::RUNTIME_MEMORY_ACCESS_OUT_OF_BOUNDS, "memory.init out of bounds"));
        }
        if n == 0 { return Ok(()); }

        let data_to_write = data_segment.data.get(src_offset .. src_offset + n)
            .ok_or_else(||Error::new(codes::RUNTIME_MEMORY_ACCESS_OUT_OF_BOUNDS, "memory.init source data segment OOB"))?;
        
        memory.write(dst_offset, data_to_write)
    }

    fn memory_copy(&mut self, dst_mem_idx: u32, src_mem_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        // In Wasm MVP, src_mem_idx and dst_mem_idx are always 0.
        let n_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.copy len: {}", e)))?;
        let src_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.copy src_offset: {}", e)))?;
        let dst_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.copy dst_offset: {}", e)))?;

        let n = n_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.copy len not i32"))? as usize;
        let src_offset = src_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.copy src_offset not i32"))? as usize;
        let dst_offset = dst_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.copy dst_offset not i32"))? as usize;

        let dst_memory = self.module_instance.memory(dst_mem_idx)?;
        let src_memory = if dst_mem_idx == src_mem_idx { Arc::clone(&dst_memory) } else { self.module_instance.memory(src_mem_idx)? };
        
        // Bounds checks
        if dst_offset.checked_add(n).map_or(true, |end| end > dst_memory.size_bytes()) ||
           src_offset.checked_add(n).map_or(true, |end| end > src_memory.size_bytes()) {
            return Err(Error::new(codes::RUNTIME_MEMORY_ACCESS_OUT_OF_BOUNDS, "memory.copy out of bounds"));
        }
        if n == 0 { return Ok(()); }

        // Wasm spec: if d_m is m and s_m is m, then the copy is performed as if the bytes are copied from m to a temporary buffer of size n and then from the buffer to m.
        // This means we can read all source bytes then write, or handle overlap carefully.
        // For simplicity, if it's the same memory and regions overlap, a temporary buffer is safest.
        // Otherwise (different memories, or same memory but no overlap), direct copy is fine.
        
        // A simple approach that is correct but might be slower if n is large:
        let mut temp_buffer = vec![0u8; n];
        src_memory.read(src_offset, &mut temp_buffer)?;
        dst_memory.write(dst_offset, &temp_buffer)
    }

    fn memory_fill(&mut self, mem_idx: u32, engine: &mut StacklessEngine) -> Result<()> {
        let n_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.fill len: {}", e)))?;
        let val_to_fill_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.fill value: {}", e)))?;
        let dst_offset_val = engine.value_stack.pop().map_err(|e| Error::new(codes::RUNTIME_STACK_UNDERFLOW, format!("memory.fill dst_offset: {}", e)))?;

        let n = n_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.fill len not i32"))? as usize;
        let val_to_fill_byte = val_to_fill_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.fill value not i32"))? as u8; // Value must be i32, truncated to u8
        let dst_offset = dst_offset_val.as_i32().ok_or_else(|| Error::new(codes::TYPE_MISMATCH_ERROR, "memory.fill dst_offset not i32"))? as usize;

        let memory = self.module_instance.memory(mem_idx)?;
        if dst_offset.checked_add(n).map_or(true, |end| end > memory.size_bytes()) {
             return Err(Error::new(codes::RUNTIME_MEMORY_ACCESS_OUT_OF_BOUNDS, "memory.fill out of bounds"));
        }
        if n == 0 { return Ok(()); }

        memory.fill(dst_offset, val_to_fill_byte, n)
    }

    // TODO: Add methods for enter_block, exit_block, branch_to_label, etc.
    // These will manipulate self.block_depths and self.pc, and interact with engine.value_stack.
}

// Validatable might not be applicable directly to StacklessFrame in the same way as Module.
// If it's for ensuring internal consistency, it might be useful.
impl Validatable for StacklessFrame {
    fn validate(&self, _level: VerificationLevel) -> Result<()> {
        // Example validations:
        // - self.pc should be within bounds of function code
        // - self.locals should match arity + declared locals of self.func_type
        // - self.block_depths should be consistent (e.g. not deeper than allowed)
        if self.pc > self.function_body()?.code.len() {
             return Err(Error::new(codes::VALIDATION_INVALID_PC, "PC out of bounds"));
        }
        // More checks can be added here.
        Ok(())
    }
}