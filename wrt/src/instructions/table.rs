//! WebAssembly table instructions
//!
//! This module contains implementations for all WebAssembly table instructions,
//! including table access and manipulation operations.

use crate::{
    behavior::{FrameBehavior, StackBehavior},
    error::{Error, Result},
    instructions::InstructionExecutor,
    stack::Stack,
    values::Value,
    StacklessEngine,
};

/// Execute a table.get instruction
///
/// Gets an element from a table.
pub fn table_get(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    table_idx: u32,
) -> Result<()> {
    let idx = stack.pop()?.as_i32()?;
    let table = frame.get_table(table_idx)?;
    let elem = table.get(idx as u32)?;
    stack.push(elem.into())
}

/// Execute a table.set instruction
///
/// Sets an element in a table.
pub fn table_set(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    table_idx: u32,
) -> Result<()> {
    let val = stack.pop()?;
    let idx = stack.pop()?.as_i32()?;
    let table = frame.get_table_mut(table_idx)?;
    table.set(idx as u32, val.try_into()?)?;
    Ok(())
}

/// Execute a table.size instruction
///
/// Returns the current size of a table.
pub fn table_size(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    table_idx: u32,
) -> Result<()> {
    let table = frame.get_table(table_idx)?;
    stack.push(Value::I32(table.size() as i32))
}

/// Execute a table.grow instruction
///
/// Grows a table by a number of elements.
pub fn table_grow(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    table_idx: u32,
) -> Result<()> {
    let n = stack.pop()?.as_i32()?;
    let val = stack.pop()?;
    let table = frame.get_table_mut(table_idx)?;
    let prev_size = table.grow(n as u32, val.try_into()?)?;
    stack.push(Value::I32(prev_size as i32))
}

/// Execute a table.init instruction
///
/// Initializes a table segment.
pub fn table_init(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    table_idx: u32,
    elem_idx: u32,
) -> Result<()> {
    let n = stack.pop()?.as_i32()?;
    let s = stack.pop()?.as_i32()?;
    let d = stack.pop()?.as_i32()?;
    let table = frame.get_table_mut(table_idx)?;
    let elem_segment = frame.get_element_segment(elem_idx)?;
    table.init(d as u32, elem_segment)?;
    Ok(())
}

/// Execute a table.copy instruction
///
/// Copies elements from one table to another.
pub fn table_copy(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    dst_table_idx: u32,
    src_table_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let n = stack.pop()?.as_i32().ok_or(Error::ValueTypeMismatch)?;
    let s = stack.pop()?.as_i32().ok_or(Error::ValueTypeMismatch)?;
    let d = stack.pop()?.as_i32().ok_or(Error::ValueTypeMismatch)?;

    let (dst_table, src_table) = frame.get_two_tables_mut(dst_table_idx, src_table_idx)?;

    // Bounds checking
    let dst_size = dst_table.size() as i32;
    let src_size = src_table.size() as i32;

    if s.checked_add(n).map_or(true, |end| end > src_size)
        || d.checked_add(n).map_or(true, |end| end > dst_size)
    {
        return Err(Error::TableIndexOutOfBounds);
    }

    // Perform the copy - using write lock on the table
    dst_table
        .write()
        .map_err(|_| Error::PoisonedLock)?
        .copy_from(
            &src_table.read().map_err(|_| Error::PoisonedLock)?,
            s as u32,
            d as u32,
            n as u32,
        );

    // Old direct call:
    // table.copy_within(s as u32, d as u32, n as u32)?;
    Ok(())
}

pub fn elem_drop(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    elem_idx: u32,
) -> Result<()> {
    frame.elem_drop(elem_idx)?;
    Ok(())
}

pub fn table_fill(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    table_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let n = stack
        .pop()?
        .as_i32()
        .ok_or_else(|| Error::InvalidType("Expected i32 for table_fill count".to_string()))?;
    let val = stack.pop()?;
    let d = stack
        .pop()?
        .as_i32()
        .ok_or_else(|| Error::InvalidType("Expected i32 for table_fill offset".to_string()))?;

    let table = frame.get_table_mut(table_idx as usize)?;
    let table_size = table.size() as i32;

    if d.checked_add(n).map_or(true, |end| end > table_size) {
        return Err(Error::TableIndexOutOfBounds);
    }

    // Validate value type matches table type
    // Call element_type() on the TableType within the Arc
    if val.type_() != table.type_().element_type {
        return Err(Error::TypeMismatch(format!(
            "Expected type {}, found type {}",
            table.type_().element_type,
            val.type_()
        )));
    }

    // Get write lock on the internal elements Vec via the Arc
    let mut table_guard = table.elements.write().map_err(|_| Error::PoisonedLock)?;
    for i in 0..n {
        let idx = (d + i) as u32;
        // Use standard Vec indexing and clone the Option<Value>
        table_guard[idx as usize] = val.clone();
    }

    Ok(())
}
