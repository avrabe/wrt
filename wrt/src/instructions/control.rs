//! WebAssembly control flow instructions
//!
//! This module contains implementations for all WebAssembly control flow instructions,
//! including blocks, branches, calls, and returns.

use crate::{
    behavior::{ControlFlowBehavior, FrameBehavior, InstructionExecutor, Label},
    error::{Error, Result},
    stack::Stack,
    types::{BlockType, FuncType},
    values::Value,
    StacklessEngine,
};

#[cfg(feature = "std")]
use std::vec;

#[cfg(not(feature = "std"))]
use alloc::vec;

/// Label type for control flow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelType {
    /// Block instruction label
    Block,
    /// Loop instruction label
    Loop,
    /// If instruction label
    If,
}

/// Create and push a new label onto the label stack
///
/// This is used internally by block, loop, and if instructions.
pub fn push_label<S: Stack>(
    pc: usize,
    stack: &mut S,
    _label_type: LabelType,
    block_type: BlockType,
    function_types: Option<&[FuncType]>,
) -> Result<()> {
    // Determine the function type for this block
    let func_type = match block_type {
        BlockType::Empty => FuncType {
            params: vec![],
            results: vec![],
        },
        BlockType::Type(value_type) => FuncType {
            params: vec![],
            results: vec![value_type],
        },
        BlockType::Value(value_type) => FuncType {
            params: vec![],
            results: vec![value_type],
        },
        BlockType::FuncType(func_type) => func_type,
        BlockType::TypeIndex(type_idx) => {
            if let Some(types) = function_types {
                if let Some(ty) = types.get(type_idx as usize) {
                    ty.clone()
                } else {
                    return Err(Error::Execution(format!("Invalid type index: {type_idx}")));
                }
            } else {
                return Err(Error::Execution(format!("Invalid type index: {type_idx}")));
            }
        }
    };

    // Create a Label with all required fields
    let label = crate::stack::Label {
        arity: func_type.results.len(),
        pc,
        continuation: 0, // Default value, should be set by caller if needed
    };

    // Push the new label using the Stack trait's method
    <S as Stack>::push_label(stack, label)?;

    Ok(())
}

#[derive(Debug)]
pub struct Block {
    pub block_type: BlockType,
    pub instructions: Vec<Box<dyn InstructionExecutor>>,
}

impl InstructionExecutor for Block {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()> {
        // Save current state
        let current_arity = frame.arity();
        let current_label_arity = frame.label_arity();

        // Set up new block
        let new_arity = match &self.block_type {
            BlockType::Empty => 0,
            BlockType::Value(_val_type) => 1,
            BlockType::Type(_val_type) => 1,
            BlockType::TypeIndex(_) => 1, // Assuming type index always results in a single value
            BlockType::FuncType(func_type) => {
                if func_type.results.is_empty() {
                    0
                } else {
                    func_type.results.len()
                }
            }
        };
        frame.set_arity(new_arity);
        frame.set_label_arity(new_arity);

        // Pre-compute values for the label
        let arity = frame.label_arity();
        let pc = frame.pc();
        let continuation = frame.return_pc() + 1;

        // Push label with the pre-computed values
        frame.label_stack().push(Label {
            arity,
            pc,
            continuation,
        });

        // Execute instructions
        for instruction in &self.instructions {
            instruction.execute(stack, frame, engine)?;
        }

        // Restore state
        frame.label_stack().pop();
        frame.set_arity(current_arity);
        frame.set_label_arity(current_label_arity);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Loop {
    pub block_type: BlockType,
    pub instructions: Vec<Box<dyn InstructionExecutor>>,
}

impl InstructionExecutor for Loop {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()> {
        // Save current state
        let current_arity = frame.arity();
        let current_label_arity = frame.label_arity();

        // Set up new loop
        let new_arity = match &self.block_type {
            BlockType::Empty => 0,
            BlockType::Value(_val_type) => 1,
            BlockType::Type(_val_type) => 1,
            BlockType::TypeIndex(_) => 1, // Assuming type index always results in a single value
            BlockType::FuncType(func_type) => {
                if func_type.results.is_empty() {
                    0
                } else {
                    func_type.results.len()
                }
            }
        };
        frame.set_arity(new_arity);
        frame.set_label_arity(0); // Loops branch to the start

        // Pre-compute values for the label
        let arity = frame.label_arity();
        let pc = frame.pc();
        let continuation = frame.return_pc();

        // Push label with the pre-computed values
        frame.label_stack().push(Label {
            arity,
            pc,
            continuation,
        });

        // Execute instructions
        for instruction in &self.instructions {
            instruction.execute(stack, frame, engine)?;
        }

        // Restore state
        frame.label_stack().pop();
        frame.set_arity(current_arity);
        frame.set_label_arity(current_label_arity);

        Ok(())
    }
}

#[derive(Debug)]
pub struct If {
    pub block_type: BlockType,
    pub if_instructions: Vec<Box<dyn InstructionExecutor>>,
    pub else_instructions: Vec<Box<dyn InstructionExecutor>>,
}

impl InstructionExecutor for If {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()> {
        let condition = stack.pop()?;
        let condition_value = match condition {
            Value::I32(0) => false,
            Value::I32(_) => true,
            _ => return Err(Error::Execution("If condition must be i32".into())),
        };

        // Save current state
        let current_arity = frame.arity();
        let current_label_arity = frame.label_arity();

        // Set up new if block
        let new_arity = match &self.block_type {
            BlockType::Empty => 0,
            BlockType::Value(_val_type) => 1,
            BlockType::Type(_val_type) => 1,
            BlockType::TypeIndex(_) => 1, // Assuming type index always results in a single value
            BlockType::FuncType(func_type) => {
                if func_type.results.is_empty() {
                    0
                } else {
                    func_type.results.len()
                }
            }
        };
        frame.set_arity(new_arity);
        frame.set_label_arity(new_arity);

        // Pre-compute values for the label
        let arity = frame.label_arity();
        let pc = frame.pc();
        let continuation = frame.return_pc() + 1;

        // Push label with the pre-computed values
        frame.label_stack().push(Label {
            arity,
            pc,
            continuation,
        });

        // Execute the appropriate branch
        let instructions = if condition_value {
            &self.if_instructions
        } else {
            &self.else_instructions
        };

        for instruction in instructions {
            instruction.execute(stack, frame, engine)?;
        }

        // Restore state
        frame.label_stack().pop();
        frame.set_arity(current_arity);
        frame.set_label_arity(current_label_arity);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Br {
    pub label_idx: u32,
}

impl InstructionExecutor for Br {
    fn execute(
        &self,
        _stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        _engine: &StacklessEngine,
    ) -> Result<()> {
        // Get the label continuation address
        let label_stack = frame.label_stack();
        let label = label_stack
            .get(self.label_idx as usize)
            .ok_or_else(|| Error::Execution("Branch target out of bounds".into()))?;
        let continuation = label.continuation;

        // Set the return PC
        frame.set_return_pc(continuation);
        Ok(())
    }
}

#[derive(Debug)]
pub struct BrIf {
    pub label_idx: u32,
}

impl InstructionExecutor for BrIf {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        _engine: &StacklessEngine,
    ) -> Result<()> {
        let condition = stack.pop()?;
        let condition_value = match condition {
            Value::I32(0) => false,
            Value::I32(_) => true,
            _ => return Err(Error::Execution("BrIf condition must be i32".into())),
        };

        if condition_value {
            // Get the label continuation address
            let label_stack = frame.label_stack();
            let label = label_stack
                .get(self.label_idx as usize)
                .ok_or_else(|| Error::Execution("Branch target out of bounds".into()))?;
            let continuation = label.continuation;

            // Set the return PC
            frame.set_return_pc(continuation);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct BrTable {
    pub labels: Vec<u32>,
    pub default_label: u32,
}

impl InstructionExecutor for BrTable {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        _engine: &StacklessEngine,
    ) -> Result<()> {
        let index = stack.pop()?;
        let index_value = match index {
            Value::I32(i) => i as usize,
            _ => return Err(Error::Execution("BrTable index must be i32".into())),
        };

        let label_idx = self
            .labels
            .get(index_value)
            .copied()
            .unwrap_or(self.default_label);

        // Get the label continuation address
        let label_stack = frame.label_stack();
        let label = label_stack
            .get(label_idx as usize)
            .ok_or_else(|| Error::Execution("Branch target out of bounds".into()))?;
        let continuation = label.continuation;

        // Set the return PC
        frame.set_return_pc(continuation);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Return;

impl InstructionExecutor for Return {
    fn execute(
        &self,
        _stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        _engine: &StacklessEngine,
    ) -> Result<()> {
        frame.set_return_pc(usize::MAX);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Call {
    pub func_idx: u32,
}

impl InstructionExecutor for Call {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()> {
        engine.call(stack, frame, self.func_idx)
    }
}

#[derive(Debug)]
pub struct CallIndirect {
    pub type_idx: u32,
    pub table_idx: u32,
}

impl InstructionExecutor for CallIndirect {
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()> {
        engine.call_indirect(stack, frame, self.type_idx, self.table_idx)
    }
}

/// Executes an unreachable instruction
pub fn unreachable(
    _stack: &mut (impl Stack + ?Sized),
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Err(Error::Execution(
        "unreachable instruction executed".to_string(),
    ))
}

/// Executes a nop instruction
pub fn nop<S: Stack + ?Sized>(
    _stack: &mut S,
    _frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    // No operation
    Ok(())
}

/// Executes an else instruction
pub fn else_instr<S: Stack>(_stack: &mut S) -> Result<()> {
    // We handle this in the execution loop
    Ok(())
}

/// Execute a block instruction
pub fn block(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    ty: FuncType,
) -> Result<()> {
    frame.enter_block(BlockType::FuncType(ty), stack.len())?;
    Ok(())
}

/// Execute a loop instruction
pub fn loop_(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    ty: FuncType,
) -> Result<()> {
    frame.enter_loop(BlockType::FuncType(ty), stack.len())?;
    Ok(())
}

/// Execute an if instruction
pub fn if_(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    ty: FuncType,
) -> Result<()> {
    // Pop the condition from the stack
    let condition = stack.pop()?;
    let condition_bool = match condition {
        Value::I32(0) => false,
        Value::I32(_) => true,
        _ => return Err(Error::InvalidType("Expected i32 for condition".to_string())),
    };

    frame.enter_if(BlockType::FuncType(ty), stack.len(), condition_bool)?;
    Ok(())
}

/// Execute an else instruction
pub fn else_(stack: &mut impl Stack, frame: &mut (impl FrameBehavior + ?Sized)) -> Result<()> {
    frame.enter_else(stack.len())?;
    Ok(())
}

/// Execute an end instruction
pub fn end(stack: &mut impl Stack, frame: &mut (impl FrameBehavior + ?Sized)) -> Result<()> {
    frame.exit_block(stack)?;
    Ok(())
}

pub fn end_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    engine: &StacklessEngine,
) -> Result<()> {
    // Don't call on Module
    if let Some(frame_concrete) = frame
        .as_any()
        .downcast_mut::<crate::stackless_frame::StacklessFrame>()
    {
        frame_concrete.exit_block(stack)?;
        Ok(())
    } else {
        Err(Error::Execution("Invalid frame type for end".to_string()))
    }
}

pub fn br(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    label_idx: u32,
) -> Result<()> {
    frame.branch(label_idx, stack)?;
    Ok(())
}

pub fn br_if(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    label_idx: u32,
) -> Result<()> {
    let condition = stack.pop()?;
    match condition {
        Value::I32(c) => {
            if c != 0 {
                frame.branch(label_idx, stack)?;
            }
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected i32".to_string())),
    }
}

pub fn br_table(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    labels: Vec<u32>,
    default: u32,
) -> Result<()> {
    let index = stack.pop()?;
    match index {
        Value::I32(i) => {
            let label_idx = if (i as usize) < labels.len() {
                labels[i as usize]
            } else {
                default
            };
            frame.branch(label_idx, stack)?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected i32".to_string())),
    }
}

pub fn return_(stack: &mut impl Stack, frame: &mut (impl FrameBehavior + ?Sized)) -> Result<()> {
    frame.return_(stack)?;
    Ok(())
}

pub fn call(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    func_idx: u32,
) -> Result<()> {
    frame.call(func_idx, stack)?;
    Ok(())
}

pub fn call_indirect(
    stack: &mut impl Stack,
    frame: &mut (impl FrameBehavior + ?Sized),
    type_idx: u32,
    table_idx: u32,
) -> Result<()> {
    let table_entry = stack.pop()?;
    match table_entry {
        Value::I32(entry) => {
            // Convert i32 to u32
            let entry_u32 = entry as u32;
            frame.call_indirect(type_idx, table_idx, entry_u32, stack)?;
            Ok(())
        }
        _ => Err(Error::InvalidType("Expected i32".to_string())),
    }
}

// Create adapter functions that accept dyn Stack
pub fn block_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    block_type: BlockType,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Block instruction");
    // Logic to handle block execution start (push label)
    let (param_types, result_types) = block_type.get_types(frame.module_instance().module());
    let arity = result_types.len();
    // TODO: Need to determine the continuation PC (address after the matching End)
    let continuation_pc = frame.pc() + 1; // Placeholder
    stack.push_label(arity, continuation_pc)?;
    Ok(())
}

pub fn loop_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    block_type: BlockType,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Loop instruction");
    // Logic to handle loop execution start (push label pointing to loop start)
    let (param_types, result_types) = block_type.get_types(frame.module_instance().module());
    let arity = param_types.len(); // Loops target the beginning, arity based on params
    let continuation_pc = frame.pc(); // Loop back to the start of the loop instruction
    stack.push_label(arity, continuation_pc)?;
    Ok(())
}

pub fn if_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    block_type: BlockType,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing If instruction");
    let condition = stack.pop()?.as_i32()?;
    let (param_types, result_types) = block_type.get_types(frame.module_instance().module());
    let arity = result_types.len();
    // TODO: Need to determine continuation PCs for both branches
    let continuation_pc_after_end = frame.pc() + 1; // Placeholder
    stack.push_label(arity, continuation_pc_after_end)?;

    if condition == 0 {
        // Jump to Else or End
        // TODO: Implement logic to find the matching Else/End and update frame PC
        println!("Condition is false, skipping If block");
        // frame.set_pc(address_of_else_or_end);
    }
    // If condition is non-zero, continue execution into the If block
    Ok(())
}

pub fn else_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Else instruction");
    // Logic for when Else is encountered (usually involves jumping to End)
    // TODO: Pop the label pushed by If, find matching End, update frame PC
    Ok(())
}

pub fn br_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    label_idx: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Br instruction: label_idx={}", label_idx);
    let label = stack.get_label(label_idx as usize)?; // Get the target label
    let arity = label.arity;
    let values_to_transfer = stack.pop_n(arity)?;
    stack.pop_labels_until(label_idx as usize)?;
    stack.push_n(values_to_transfer)?;
    frame.set_pc(label.continuation); // Set PC to the label's continuation
    Ok(())
}

pub fn br_if_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    label_idx: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing BrIf instruction: label_idx={}", label_idx);
    let condition = stack.pop()?.as_i32()?;
    if condition != 0 {
        br_dyn(stack, frame, label_idx, engine)?;
    }
    // If condition is 0, do nothing, execution continues sequentially
    Ok(())
}

pub fn br_table_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    label_indices: Vec<u32>,
    default_label: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!(
        "Executing BrTable instruction: targets={:?}, default={}",
        label_indices, default_label
    );
    let value = stack.pop()?.as_i32()? as usize;
    let target_label_idx = if value < label_indices.len() {
        label_indices[value]
    } else {
        default_label
    };
    br_dyn(stack, frame, target_label_idx, engine)
}

pub fn return_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Return instruction");
    let frame_arity = frame.arity(); // Get expected return arity from the frame
    let results = stack.pop_n(frame_arity)?;

    // Pop frames/labels until the function call boundary (e.g., pop the current frame's label)
    // This assumes the frame label is at the top for a return.
    stack.pop_frame_label()?; // Specific method to pop the label associated with the current frame

    // Push results back onto the caller's stack context
    stack.push_n(results)?;

    // Signal return by setting PC beyond the end or using a specific flag
    frame.set_pc(usize::MAX);
    Ok(())
}

pub fn unreachable_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Unreachable instruction");
    Err(Error::Unreachable)
}

pub fn nop_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Nop instruction");
    Ok(())
}

pub fn call_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    func_idx: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing Call instruction: func_idx={}", func_idx);
    let module_instance = frame.module_instance();
    let func_type = module_instance.get_function_type(func_idx)?;
    let args_count = func_type.params.len();
    let args = stack.pop_n(args_count)?;

    // Execute the function call using the engine/stack context
    // This needs access to the main execution logic, likely on StacklessStack or Engine
    // stack.execute_function(engine, frame.instance_idx(), func_idx, args)?;
    let results = stack.execute_function_call_direct(engine, 0, func_idx, args)?;
    stack.push_n(results)?;

    Ok(())
}

pub fn call_indirect_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    type_idx: u32,
    table_idx: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!(
        "Executing CallIndirect instruction: type_idx={}, table_idx={}",
        type_idx, table_idx
    );
    let table_addr = stack.pop()?.as_i32()? as usize;
    let module_instance = frame.module_instance();
    let table = module_instance.get_table(table_idx as usize)?;
    let func_elem = table.get(table_addr)?;
    let func_idx = func_elem.ok_or(Error::IndirectCallToNull)?;

    // Type check
    let expected_type = module_instance.module().get_function_type(type_idx)?;
    let actual_type = module_instance.get_function_type(func_idx)?;
    if expected_type != actual_type {
        return Err(Error::IndirectCallTypeMismatch);
    }

    let args_count = actual_type.params.len();
    let args = stack.pop_n(args_count)?;

    // Execute indirect call
    // stack.execute_function(engine, frame.instance_idx(), func_idx, args)?;
    let results = stack.execute_function_call_direct(engine, table_idx, func_idx, args)?;
    stack.push_n(results)?;

    Ok(())
}

// Adapter functions for the rest
pub fn return_call_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    func_idx: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!("Executing ReturnCall instruction: func_idx={}", func_idx);
    // Pop current frame/label first
    return_dyn(stack, frame, engine)?;
    // Then perform the call (tail call)
    call_dyn(stack, frame, func_idx, engine)
}

pub fn return_call_indirect_dyn(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    type_idx: u32,
    table_idx: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    println!(
        "Executing ReturnCallIndirect instruction: type_idx={}, table_idx={}",
        type_idx, table_idx
    );
    // Pop current frame/label first
    return_dyn(stack, frame, engine)?;
    // Then perform the indirect call (tail call)
    call_indirect_dyn(stack, frame, type_idx, table_idx, engine)
}
