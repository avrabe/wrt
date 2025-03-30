//! WebAssembly control flow instructions
//!
//! This module contains implementations for all WebAssembly control flow instructions,
//! including blocks, branches, calls, and returns.

use crate::{error::Error, execution::Stack, format, types::FuncType, Result, Value, Vec};

#[cfg(feature = "std")]
use std::vec;

#[cfg(not(feature = "std"))]
use alloc::vec;

use crate::instructions::BlockType;

/// Label type for control flow
#[derive(Debug, Clone, PartialEq)]
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
pub fn push_label(
    pc: usize,
    stack: &mut Stack,
    label_type: LabelType,
    block_type: BlockType,
    function_types: Option<&[FuncType]>,
) -> Result<()> {
    // Determine the function type for this block
    let func_type = match block_type {
        BlockType::Empty => FuncType {
            params: Vec::new(),
            results: Vec::new(),
        },
        BlockType::Type(value_type) => FuncType {
            params: Vec::new(),
            results: vec![value_type],
        },
        BlockType::TypeIndex(type_idx) => {
            if let Some(types) = function_types {
                if let Some(ty) = types.get(type_idx as usize) {
                    ty.clone()
                } else {
                    return Err(Error::Execution(format!(
                        "Invalid type index: {}",
                        type_idx
                    )));
                }
            } else {
                return Err(Error::Execution(format!(
                    "Invalid type index: {}",
                    type_idx
                )));
            }
        }
    };

    // Push the new label
    stack.push_label(func_type.results.len(), pc);

    Ok(())
}

/// Executes a block instruction
pub fn block(stack: &mut Stack) -> Result<()> {
    Ok(())
}

/// Execute a loop instruction
///
/// Creates a new loop scope with the given block type.
pub fn loop_instr(
    stack: &mut Stack,
    pc: usize,
    block_type: BlockType,
    function_types: Option<&[FuncType]>,
) -> Result<()> {
    push_label(pc, stack, LabelType::Loop, block_type, function_types)
}

/// Executes an if instruction
pub fn if_instr(stack: &mut Stack) -> Result<()> {
    let Value::I32(condition) = stack.pop()? else {
        return Err(Error::Execution("Expected i32 condition".into()));
    };
    Ok(())
}

/// Executes a br instruction
pub fn br(stack: &mut Stack, label_idx: u32) -> Result<()> {
    // Get the label from the stack - we need to access the label stack from the bottom up
    // since branch depths are counted from the innermost label (most recently pushed)
    let labels_len = stack.labels.len();

    if label_idx as usize >= labels_len {
        return Err(Error::Execution(format!(
            "Invalid branch target: {}",
            label_idx
        )));
    }

    // Calculate the index from the end of the stack (0 = most recent)
    let idx = labels_len - 1 - (label_idx as usize);

    if let Some(label) = stack.labels.get(idx) {
        // Store the continuation PC
        let continuation_pc = label.continuation;

        // Pop all labels up to and including the target
        for _ in 0..=label_idx {
            stack.pop_label()?;
        }

        // Set PC to the continuation of the label
        if let Some(frame) = stack.call_frames.last_mut() {
            frame.pc = continuation_pc;
            return Ok(());
        }
    }

    // If no active frame, return an error
    Err(Error::Execution("No active frame for branch".into()))
}

/// Executes a br_if instruction
pub fn br_if(stack: &mut Stack, label_idx: u32) -> Result<()> {
    let Value::I32(condition) = stack.pop()? else {
        return Err(Error::Execution("Expected i32 condition".into()));
    };

    // Only branch if condition is true (non-zero)
    if condition != 0 {
        // Perform the actual branch operation
        return br(stack, label_idx);
    }

    // If condition is false, just continue with the next instruction
    Ok(())
}

/// Executes a br_table instruction
pub fn br_table(stack: &mut Stack, labels: &[u32], default_label: u32) -> Result<()> {
    let Value::I32(index) = stack.pop()? else {
        return Err(Error::Execution("Expected i32 index".into()));
    };
    Ok(())
}

/// Executes a return instruction
pub fn return_instr(stack: &mut Stack) -> Result<()> {
    Ok(())
}

/// Executes a call instruction
pub fn call(func_idx: u32) -> Result<()> {
    Ok(())
}

/// Executes a call_indirect instruction
pub fn call_indirect(table_idx: u32, type_idx: u32) -> Result<()> {
    Ok(())
}

/// Executes an unreachable instruction
pub fn unreachable(stack: &mut Stack) -> Result<()> {
    Err(Error::Execution("Unreachable instruction executed".into()))
}

/// Executes a nop instruction
pub fn nop(stack: &mut Stack) -> Result<()> {
    Ok(())
}

/// Executes an end instruction
pub fn end(stack: &mut Stack) -> Result<()> {
    Ok(())
}

/// Executes an else instruction
pub fn else_instr(stack: &mut Stack) -> Result<()> {
    Ok(())
}
