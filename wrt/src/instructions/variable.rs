//! WebAssembly variable instructions
//!
//! This module contains implementations for all WebAssembly variable instructions,
//! including local and global variable access.

use crate::error::Error;
use crate::execution::Frame;
use crate::format;
use crate::Value;
use crate::Vec;

/// Execute a local.get instruction
///
/// Gets the value of a local variable.
pub fn local_get(stack: &mut Vec<Value>, frame: &Frame, local_idx: u32) -> Result<(), Error> {
    if local_idx as usize >= frame.locals.len() {
        return Err(Error::Execution(format!(
            "Invalid local index: {}",
            local_idx
        )));
    }

    stack.push(frame.locals[local_idx as usize].clone());
    Ok(())
}

/// Execute a local.set instruction
///
/// Sets the value of a local variable.
pub fn local_set(stack: &mut Vec<Value>, frame: &mut Frame, local_idx: u32) -> Result<(), Error> {
    if local_idx as usize >= frame.locals.len() {
        return Err(Error::Execution(format!(
            "Invalid local index: {}",
            local_idx
        )));
    }

    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    frame.locals[local_idx as usize] = value;
    Ok(())
}

/// Execute a local.tee instruction
///
/// Sets the value of a local variable and keeps the value on the stack.
pub fn local_tee(stack: &mut Vec<Value>, frame: &mut Frame, local_idx: u32) -> Result<(), Error> {
    if local_idx as usize >= frame.locals.len() {
        return Err(Error::Execution(format!(
            "Invalid local index: {}",
            local_idx
        )));
    }

    if stack.is_empty() {
        return Err(Error::Execution("Stack underflow".into()));
    }

    let value = stack.last().unwrap().clone();
    frame.locals[local_idx as usize] = value;
    Ok(())
}

/// Execute a global.get instruction
///
/// Gets the value of a global variable.
pub fn global_get(stack: &mut Vec<Value>, frame: &Frame, global_idx: u32) -> Result<(), Error> {
    if global_idx as usize >= frame.module.global_addrs.len() {
        return Err(Error::Execution(format!(
            "Invalid global index: {}",
            global_idx
        )));
    }

    let global_addr = &frame.module.global_addrs[global_idx as usize];
    let value = frame.module.globals[global_addr.global_idx as usize].get();
    stack.push(value);
    Ok(())
}

/// Execute a global.set instruction
///
/// Sets the value of a global variable.
pub fn global_set(stack: &mut Vec<Value>, frame: &mut Frame, global_idx: u32) -> Result<(), Error> {
    let global_addr = &frame.module.global_addrs[global_idx as usize];
    let global = &mut frame.module.globals[global_addr.global_idx as usize];

    if !global.type_().mutable {
        return Err(Error::Execution("Cannot set immutable global".into()));
    }

    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    if value.type_() != global.type_().content_type {
        return Err(Error::Execution(
            "Value type does not match global type".into(),
        ));
    }

    global.set(value)?;
    Ok(())
}
