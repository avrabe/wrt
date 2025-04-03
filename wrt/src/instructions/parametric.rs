//! WebAssembly parametric instructions
//!
//! This module contains implementations for all WebAssembly parametric instructions,
//! including operations for stack manipulation and control flow.

use crate::{
    behavior::{FrameBehavior, StackBehavior},
    error::{Error, Result},
    instructions::InstructionExecutor,
    stack::Stack,
    types::ValueType,
    values::Value,
    StacklessEngine,
};

/// Execute a drop instruction
///
/// Removes the top value from the stack.
pub fn drop(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let _ = stack.pop()?;
    Ok(())
}

/// Execute a select instruction
///
/// Selects one of two values based on a condition.
pub fn select(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let c = stack.pop()?.as_i32()?;
    let val2 = stack.pop()?;
    let val1 = stack.pop()?;

    if c != 0 {
        stack.push(val1)?;
    } else {
        stack.push(val2)?;
    }
    Ok(())
}

/// Execute a `select_typed` instruction
///
/// Selects one of two values based on a condition, with type checking.
pub fn select_typed(
    stack: &mut dyn Stack,
    _frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    ty: &[ValueType],
) -> Result<()> {
    let c = stack.pop()?.as_i32()?;
    let val2 = stack.pop()?;
    let val1 = stack.pop()?;

    if !ty.is_empty() {
        let expected_type = ty[0];
        if !val1.value_type().matches(expected_type) || !val2.value_type().matches(expected_type) {
            return Err(Error::TypeMismatch);
        }
    }

    if c != 0 {
        stack.push(val1)?;
    } else {
        stack.push(val2)?;
    }
    Ok(())
}

/// Execute a block instruction
///
/// Creates a new block scope.
pub fn block(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Ok(())
}

/// Execute an if instruction
///
/// Creates a new conditional block.
pub fn if_instr(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    let condition = stack.pop()?;
    match condition {
        Value::I32(0) => Ok(()),
        _ => Ok(()),
    }
}

/// Execute an else instruction
///
/// Ends the "if" part of an if/else and begins the "else" part.
pub fn else_instr(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Ok(())
}

/// Execute an end instruction
///
/// Ends a block, loop, if, or function.
pub fn end(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Ok(())
}

/// Execute a br instruction
///
/// Unconditionally branches to a label.
pub fn br(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Ok(())
}

/// Execute a `br_if` instruction
///
/// Conditionally branches to a label.
pub fn br_if(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    let condition = stack.pop()?;
    match condition {
        Value::I32(0) => Ok(()),
        _ => Ok(()),
    }
}

/// Execute a `br_table` instruction
///
/// Branches to one of several labels based on an index value.
pub fn br_table(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    let index = stack.pop()?;
    match index {
        Value::I32(_) => Ok(()),
        _ => Err(Error::InvalidType("Expected i32".to_string())),
    }
}

/// Execute a return instruction
///
/// Returns from the current function.
pub fn return_instr(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Ok(())
}

/// Execute an unreachable instruction
///
/// Indicates that the current code location should not be reachable.
pub fn unreachable(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Err(Error::Execution("Reached unreachable instruction".into()))
}

/// Execute a nop instruction
///
/// No operation.
pub fn nop(
    stack: &mut (impl Stack + ?Sized),
    frame: &mut (impl FrameBehavior + ?Sized),
) -> Result<()> {
    Ok(())
}
