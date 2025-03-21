//! WebAssembly table instructions
//!
//! This module contains implementations for all WebAssembly table instructions,
//! including table access and manipulation operations.

use crate::error::Error;
use crate::execution::Frame;
use crate::format;
use crate::Value;
use crate::Vec;

/// Execute a table.get instruction
///
/// Gets an element from a table.
pub fn table_get(stack: &mut Vec<Value>, frame: &Frame, table_idx: u32) -> Result<(), Error> {
    if table_idx as usize >= frame.module.table_addrs.len() {
        return Err(Error::Execution(format!(
            "Invalid table index: {}",
            table_idx
        )));
    }

    let table_addr = &frame.module.table_addrs[table_idx as usize];
    let table = &frame.module.tables[table_addr.table_idx as usize];

    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let Value::I32(idx) = value {
        if idx < 0 || idx as u32 >= table.size() {
            return Err(Error::Execution(format!(
                "Table index out of bounds: {}",
                idx
            )));
        }

        let elem = table.get(idx as u32)?.unwrap_or(Value::FuncRef(None));
        stack.push(elem);
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 for table index".into()))
    }
}

/// Execute a table.set instruction
///
/// Sets an element in a table.
pub fn table_set(stack: &mut Vec<Value>, frame: &mut Frame, table_idx: u32) -> Result<(), Error> {
    if table_idx as usize >= frame.module.table_addrs.len() {
        return Err(Error::Execution(format!(
            "Invalid table index: {}",
            table_idx
        )));
    }

    let table_addr = &frame.module.table_addrs[table_idx as usize];
    let table = &mut frame.module.tables[table_addr.table_idx as usize];

    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let idx = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if let Value::I32(idx) = idx {
        if idx < 0 || idx as u32 >= table.size() {
            return Err(Error::Execution(format!(
                "Table index out of bounds: {}",
                idx
            )));
        }

        table.set(idx as u32, Some(value))?;
        Ok(())
    } else {
        Err(Error::Execution("Expected i32 for table index".into()))
    }
}

/// Execute a table.size instruction
///
/// Returns the current size of a table.
pub fn table_size(stack: &mut Vec<Value>, frame: &Frame, table_idx: u32) -> Result<(), Error> {
    if table_idx as usize >= frame.module.table_addrs.len() {
        return Err(Error::Execution(format!(
            "Invalid table index: {}",
            table_idx
        )));
    }

    let table_addr = &frame.module.table_addrs[table_idx as usize];
    let table = &frame.module.tables[table_addr.table_idx as usize];

    stack.push(Value::I32(table.size() as i32));

    Ok(())
}

/// Execute a table.grow instruction
///
/// Grows a table by a number of elements.
pub fn table_grow(stack: &mut Vec<Value>, frame: &mut Frame, table_idx: u32) -> Result<(), Error> {
    let Value::I32(n) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for table.grow".into()));
    };

    let value = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    if n < 0 {
        return Err(Error::Execution(format!("Invalid table size: {}", n)));
    }

    let table_addr = &frame.module.table_addrs[table_idx as usize];
    let table = &mut frame.module.tables[table_addr.table_idx as usize];

    let old_size = table.size();
    if let Err(_) = table.grow(n as u32) {
        stack.push(Value::I32(-1));
    } else {
        stack.push(Value::I32(old_size as i32));
    }

    Ok(())
}

/// Execute a table.init instruction
///
/// Initializes a table segment.
pub fn table_init(
    stack: &mut Vec<Value>,
    frame: &mut Frame,
    table_idx: u32,
    elem_idx: u32,
) -> Result<(), Error> {
    let Value::I32(size) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution("Expected i32 for table.init size".into()));
    };

    let Value::I32(src_offset) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for table.init source offset".into(),
        ));
    };

    let Value::I32(dst_offset) = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?
    else {
        return Err(Error::Execution(
            "Expected i32 for table.init destination offset".into(),
        ));
    };

    if size < 0 || src_offset < 0 || dst_offset < 0 {
        return Err(Error::Execution("Invalid table offset".into()));
    }

    let table_addr = &frame.module.table_addrs[table_idx as usize];
    let table = &mut frame.module.tables[table_addr.table_idx as usize];

    if dst_offset as u32 + size as u32 > table.size() {
        return Err(Error::Execution("Table out of bounds".into()));
    }

    // TODO: Implement table initialization from element segment

    Ok(())
}

/// Execute a table.copy instruction
///
/// Copies elements from one table to another.
pub fn table_copy(
    stack: &mut Vec<Value>,
    frame: &mut Frame,
    dst_table_idx: u32,
    src_table_idx: u32,
) -> Result<(), Error> {
    let size = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let src_offset = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;
    let dst_offset = stack
        .pop()
        .ok_or(Error::Execution("Stack underflow".into()))?;

    let Value::I32(size) = size else {
        return Err(Error::Execution("Expected i32 value".into()));
    };
    let Value::I32(src_offset) = src_offset else {
        return Err(Error::Execution("Expected i32 value".into()));
    };
    let Value::I32(dst_offset) = dst_offset else {
        return Err(Error::Execution("Expected i32 value".into()));
    };

    if size < 0 || src_offset < 0 || dst_offset < 0 {
        return Err(Error::Execution("Invalid table offset".into()));
    }

    let src_table_addr = &frame.module.table_addrs[src_table_idx as usize];
    let dst_table_addr = &frame.module.table_addrs[dst_table_idx as usize];

    // Get both tables as mutable references
    let (src_table, dst_table) = if src_table_addr.table_idx < dst_table_addr.table_idx {
        let (left, right) = frame
            .module
            .tables
            .split_at_mut(src_table_addr.table_idx as usize + 1);
        (
            &mut left[src_table_addr.table_idx as usize],
            &mut right[dst_table_addr.table_idx as usize - src_table_addr.table_idx as usize - 1],
        )
    } else {
        let (left, right) = frame
            .module
            .tables
            .split_at_mut(dst_table_addr.table_idx as usize + 1);
        (
            &mut right[src_table_addr.table_idx as usize - dst_table_addr.table_idx as usize - 1],
            &mut left[dst_table_addr.table_idx as usize],
        )
    };

    if src_offset as u32 + size as u32 > src_table.size()
        || dst_offset as u32 + size as u32 > dst_table.size()
    {
        return Err(Error::Execution("Table out of bounds".into()));
    }

    // TODO: Implement table copy

    Ok(())
}
