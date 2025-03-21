//! WebAssembly parametric instructions
//!
//! This module contains implementations for all WebAssembly parametric instructions,
//! including operations for stack manipulation and control flow.

use crate::error::Error;
use crate::instructions::ValueType;
use crate::values::Value;
use crate::Vec;

/// Execute a drop instruction
///
/// Removes the top value from the stack.
pub fn drop(stack: &mut Vec<Value>) -> Result<(), Error> {
    stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    Ok(())
}

/// Execute a select instruction
///
/// Selects one of two values based on a condition.
pub fn select(stack: &mut Vec<Value>) -> Result<(), Error> {
    // Pop the condition value
    let Value::I32(condition) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for select condition".into()));
    };

    // Pop the two values to select from
    let value2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let value1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    // Push the selected value
    if condition != 0 {
        stack.push(value1);
    } else {
        stack.push(value2);
    }

    Ok(())
}

/// Execute a select_typed instruction
///
/// Selects one of two values based on a condition, with type checking.
pub fn select_typed(stack: &mut Vec<Value>, _value_type: ValueType) -> Result<(), Error> {
    // Pop the condition value
    let Value::I32(condition) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for select_typed condition".into(),
        ));
    };

    // Pop the two values to select from
    let value2 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let value1 = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    // Check that the values have the same type
    if std::mem::discriminant(&value1) != std::mem::discriminant(&value2) {
        return Err(Error::Execution(
            "Values must have the same type for select_typed".into(),
        ));
    }

    // Push the selected value
    if condition != 0 {
        stack.push(value1);
    } else {
        stack.push(value2);
    }

    Ok(())
}

/// Execute a block instruction
///
/// Creates a new block scope.
pub fn block(stack: &mut Vec<Value>) -> Result<(), Error> {
    // No stack manipulation needed for block instruction
    Ok(())
}

/// Execute an if instruction
///
/// Creates a new conditional block.
pub fn if_instr(stack: &mut Vec<Value>) -> Result<(), Error> {
    // Pop the condition value
    let Value::I32(condition) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for if condition".into()));
    };

    // No further stack manipulation needed for if instruction
    Ok(())
}

/// Execute an else instruction
///
/// Ends the "if" part of an if/else and begins the "else" part.
pub fn else_instr(stack: &mut Vec<Value>) -> Result<(), Error> {
    // No stack manipulation needed for else instruction
    Ok(())
}

/// Execute an end instruction
///
/// Ends a block, loop, if, or function.
pub fn end(stack: &mut Vec<Value>) -> Result<(), Error> {
    // No stack manipulation needed for end instruction
    Ok(())
}

/// Execute a br instruction
///
/// Unconditionally branches to a label.
pub fn br(stack: &mut Vec<Value>, label_idx: u32) -> Result<(), Error> {
    // No stack manipulation needed for br instruction
    Ok(())
}

/// Execute a br_if instruction
///
/// Conditionally branches to a label.
pub fn br_if(stack: &mut Vec<Value>, label_idx: u32) -> Result<(), Error> {
    // Pop the condition value
    let Value::I32(condition) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for br_if condition".into()));
    };

    // No further stack manipulation needed for br_if instruction
    Ok(())
}

/// Execute a br_table instruction
///
/// Branches to one of several labels based on an index value.
pub fn br_table(stack: &mut Vec<Value>, labels: &[u32], default_label: u32) -> Result<(), Error> {
    // Pop the index value
    let Value::I32(index) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for br_table index".into()));
    };

    // No further stack manipulation needed for br_table instruction
    Ok(())
}

/// Execute a return instruction
///
/// Returns from the current function.
pub fn return_instr(stack: &mut Vec<Value>) -> Result<(), Error> {
    // No stack manipulation needed for return instruction
    Ok(())
}

/// Execute an unreachable instruction
///
/// Indicates that the current code location should not be reachable.
pub fn unreachable(stack: &mut Vec<Value>) -> Result<(), Error> {
    // No stack manipulation needed for unreachable instruction
    Ok(())
}

/// Execute a nop instruction
///
/// No operation.
pub fn nop(stack: &mut Vec<Value>) -> Result<(), Error> {
    // No stack manipulation needed for nop instruction
    Ok(())
}
