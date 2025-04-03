//! WebAssembly memory instructions
//!
//! This module contains implementations for all WebAssembly memory instructions,
//! including loads, stores, and memory management operations.

use crate::{
    behavior::{FrameBehavior, StackBehavior},
    error::{Error, Result},
    instructions::InstructionExecutor,
    stack::Stack,
    values::Value,
    StacklessEngine,
};
use std::sync::Arc;

/// Execute an i32 load instruction
///
/// Loads a 32-bit integer from memory at the specified address.
pub fn i32_load(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 4, align)?;
    let value = memory.read_i32(effective_addr)?;
    stack.push(Value::I32(value))
}

/// Execute an i64 load instruction
///
/// Loads a 64-bit integer from memory at the specified address.
pub fn i64_load(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 8, align)?;
    let value = memory.read_i64(effective_addr)?;
    stack.push(Value::I64(value))
}

/// Execute an f32 load instruction
///
/// Loads a 32-bit float from memory at the specified address.
pub fn f32_load(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 4, align)?;
    let value = memory.read_f32(effective_addr)?;
    stack.push(Value::F32(value))
}

/// Execute an f64 load instruction
///
/// Loads a 64-bit float from memory at the specified address.
pub fn f64_load(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 8, align)?;
    let value = memory.read_f64(effective_addr)?;
    stack.push(Value::F64(value))
}

/// Execute an i32 `load8_s` instruction
///
/// Loads an 8-bit signed integer from memory and sign-extends it to 32 bits.
pub fn i32_load8_s(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 1, align)?;
    let value = memory.read_i8(effective_addr)? as i32;
    stack.push(Value::I32(value))
}

/// Execute an i32 `load8_u` instruction
///
/// Loads an 8-bit unsigned integer from memory and zero-extends it to 32 bits.
pub fn i32_load8_u(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 1, align)?;
    let value = memory.read_u8(effective_addr)? as i32;
    stack.push(Value::I32(value))
}

/// Execute an i32 `load16_s` instruction
///
/// Loads a 16-bit signed integer from memory and sign-extends it to 32 bits.
pub fn i32_load16_s(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 2, align)?;
    let value = memory.read_i16(effective_addr)? as i32;
    stack.push(Value::I32(value))
}

/// Execute an i32 `load16_u` instruction
///
/// Loads a 16-bit unsigned integer from memory and zero-extends it to 32 bits.
pub fn i32_load16_u(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 2, align)?;
    let value = memory.read_u16(effective_addr)? as i32;
    stack.push(Value::I32(value))
}

/// Execute an i32 store instruction
///
/// Stores a 32-bit integer to memory at the specified address.
pub fn i32_store(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = stack.pop()?.as_i32()?;
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 4, align)?;
    memory.write_i32(effective_addr, value)
}

/// Execute an i64 store instruction
///
/// Stores a 64-bit integer to memory at the specified address.
pub fn i64_store(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = stack.pop()?.as_i64()?;
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 8, align)?;
    memory.write_i64(effective_addr, value)
}

/// Execute memory size instruction
///
/// Returns the current size of memory in pages (64KB per page).
pub fn memory_size(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    mem_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let memory = frame.get_memory(mem_idx as usize)?;
    let size_pages = memory.size(); // Size in pages
    stack.push(Value::I32(size_pages as i32))
}

/// Execute memory grow instruction
///
/// Grows memory by the specified number of pages, returns previous size or -1 on failure.
pub fn memory_grow(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    mem_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let delta_pages = stack.pop()?.as_i32()?;
    let memory = frame.get_memory_mut(mem_idx as usize)?;
    let old_size = memory.grow(delta_pages as u32)?;
    stack.push(Value::I32(old_size as i32))
}

/// Pop a memory address from the stack
///
/// Helper function to get a memory address from the stack.
fn pop_memory_address(stack: &mut dyn Stack) -> Result<u32> {
    match stack.pop()? {
        Value::I32(addr) => Ok(addr),
        _ => Err(Error::TypeMismatch("Expected i32 address".to_string())),
    }
}

/// Execute memory fill instruction
///
/// Fills a region of memory with a given byte value.
pub fn memory_fill(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    mem_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let n = stack.pop()?.as_i32()? as usize;
    let val = stack.pop()?.as_i32()? as u8;
    let d = stack.pop()?.as_i32()? as usize;
    let memory = frame.get_memory_mut(mem_idx as usize)?;
    memory.fill(d, val, n)
}

/// Execute memory copy instruction
///
/// Copies data from one region of memory to another, possibly overlapping.
pub fn memory_copy(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    dst_mem: u32,
    src_mem: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let n = stack.pop()?.as_i32()? as usize;
    let s = stack.pop()?.as_i32()? as usize;
    let d = stack.pop()?.as_i32()? as usize;
    let memory_src = frame.get_memory(src_mem as usize)?;
    let memory_dst = frame.get_memory_mut(dst_mem as usize)?;
    memory_dst.copy_within_or_between(memory_src, s, d, n)
}

/// Execute memory init instruction
///
/// Copies data from a passive data segment to memory.
pub fn memory_init(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    data_idx: u32,
    mem_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let n = stack.pop()?.as_i32()? as usize;
    let s = stack.pop()?.as_i32()? as usize;
    let d = stack.pop()?.as_i32()? as usize;
    let data_segment = frame.get_data_segment(data_idx)?;
    let memory = frame.get_memory_mut(mem_idx as usize)?;
    memory.init(d, &data_segment.init, s, n)
}

/// Execute data drop instruction
///
/// Drops a passive data segment, freeing its resources.
pub fn data_drop(
    _stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    data_idx: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    frame.drop_data_segment(data_idx)
}

/// Load signed 8-bit value and extend to i64
pub fn i64_load8_s(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let value = i64::from(frame.load_i8(addr, align)?);
    stack.push(Value::I64(value))
}

/// Load unsigned 8-bit value and extend to i64
pub fn i64_load8_u(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let value = i64::from(frame.load_u8(addr, align)?);
    stack.push(Value::I64(value))
}

/// Load signed 16-bit value and extend to i64
pub fn i64_load16_s(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let value = i64::from(frame.load_i16(addr, align)?);
    stack.push(Value::I64(value))
}

/// Load unsigned 16-bit value and extend to i64
pub fn i64_load16_u(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let value = i64::from(frame.load_u16(addr, align)?);
    stack.push(Value::I64(value))
}

/// Load signed 32-bit value and extend to i64
pub fn i64_load32_s(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let value = i64::from(frame.load_i32(addr, align)?);
    stack.push(Value::I64(value))
}

/// Load unsigned 32-bit value and extend to i64
pub fn i64_load32_u(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let value = i64::from(frame.load_i32(addr, align)? as u32);
    stack.push(Value::I64(value))
}

/// Store f32 value
pub fn f32_store(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::F32(v) => v,
        _ => return Err(Error::TypeMismatch("Expected f32 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 4, align)?;
    memory.write_f32(addr as u32, value)
}

/// Store f64 value
pub fn f64_store(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::F64(v) => v,
        _ => return Err(Error::TypeMismatch("Expected f64 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 8, align)?;
    memory.write_f64(addr as u32, value)
}

/// Store low 8 bits of i32
pub fn i32_store8(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::I32(v) => v,
        _ => return Err(Error::TypeMismatch("Expected i32 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 1, align)?;
    memory.write_u8(addr as u32, (value & 0xFF) as u8)
}

/// Store low 16 bits of i32
pub fn i32_store16(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::I32(v) => v,
        _ => return Err(Error::TypeMismatch("Expected i32 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 2, align)?;
    memory.write_u16(addr as u32, (value & 0xFFFF) as u16)
}

/// Store low 8 bits of i64
pub fn i64_store8(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::I64(v) => v,
        _ => return Err(Error::TypeMismatch("Expected i64 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 1, align)?;
    memory.write_u8(addr as u32, (value & 0xFF) as u8)
}

/// Store low 16 bits of i64
pub fn i64_store16(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::I64(v) => v,
        _ => return Err(Error::TypeMismatch("Expected i64 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 2, align)?;
    memory.write_u16(addr as u32, (value & 0xFFFF) as u16)
}

/// Store low 32 bits of i64
pub fn i64_store32(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let value = match stack.pop()? {
        Value::I64(v) => v,
        _ => return Err(Error::TypeMismatch("Expected i64 value".to_string())),
    };
    let addr = match stack.pop()? {
        Value::I32(addr) => addr as usize + offset as usize,
        _ => return Err(Error::TypeMismatch("Expected i32 address".to_string())),
    };
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 4, align)?;
    memory.write_u32(addr as u32, (value & 0xFFFFFFFF) as u32)
}

/// Execute a v128 store instruction
///
/// Stores a 128-bit value (16 bytes) to memory at the specified address.
pub fn v128_store(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    engine: &StacklessEngine,
) -> Result<()> {
    let value = stack.pop()?.as_v128()?;
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    memory.check_alignment(addr as u32, 16, align)?;
    memory.write_v128(addr as u32, value)
}

/// Load a v128 value from memory.
///
/// Pops the base address (i32), calculates the effective address using the offset,
/// checks alignment, reads 16 bytes, and pushes the V128 value onto the stack.
pub fn v128_load(
    stack: &mut dyn Stack,
    frame: &dyn FrameBehavior,
    offset: u32,
    align: u32,
    _engine: &StacklessEngine,
) -> Result<()> {
    let addr = stack.pop()?.as_i32()?;
    let memory = frame.get_memory(0)?;
    let effective_addr = (addr as u32).wrapping_add(offset);
    memory.check_alignment(effective_addr, 16, align)?;
    let bytes = memory.read_bytes(effective_addr, 16)?;
    let value: [u8; 16] = bytes.try_into().map_err(|v: Vec<u8>| {
        Error::Execution(format!("Expected 16 bytes for v128 load, got {}", v.len()))
    })?;
    stack.push(Value::V128(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        behavior::{ControlFlowBehavior, FrameBehavior, StackBehavior},
        error::{Error, Result},
        global::Global,
        memory::{DefaultMemory, MemoryBehavior, PAGE_SIZE},
        module::DataSegment,
        stack::{Label as StackLabel, Stack},
        table::Table,
        types::{BlockType, FuncType, GlobalType, Label, MemoryType, TableType, ValueType},
        values::Value,
        StacklessEngine,
    };
    use std::sync::{Arc, MutexGuard, RwLock};

    // --- Mocks ---

    #[derive(Debug, Default)]
    struct MockStack {
        values: Vec<Value>,
        labels: Vec<StackLabel>,
    }

    impl MockStack {
        // Removed new() as Default derive handles it.
        // fn new() -> Self {
        //     Self { values: Vec::new(), labels: Vec::new() }
        // }
    }

    impl StackBehavior for MockStack {
        fn push(&mut self, value: Value) -> Result<()> {
            self.values.push(value);
            Ok(())
        }
        fn pop(&mut self) -> Result<Value> {
            self.values.pop().ok_or(Error::StackUnderflow)
        }
        fn peek(&self) -> Result<&Value> {
            self.values.last().ok_or(Error::StackUnderflow)
        }
        fn peek_mut(&mut self) -> Result<&mut Value> {
            self.values.last_mut().ok_or(Error::StackUnderflow)
        }
        fn values(&self) -> &[Value] {
            &self.values
        }
        fn values_mut(&mut self) -> &mut [Value] {
            &mut self.values
        }
        fn len(&self) -> usize {
            self.values.len()
        }
        fn is_empty(&self) -> bool {
            self.values.is_empty()
        }

        // Implement label methods from StackBehavior trait using our internal Vec<StackLabel>
        fn push_label(&mut self, arity: usize, pc: usize) {
            // Note: StackBehavior uses behavior::Label internally
            self.labels.push(StackLabel {
                arity,
                pc,
                continuation: pc,
            });
        }
        fn pop_label(&mut self) -> Result<crate::behavior::Label> {
            self.labels
                .pop()
                .map(|l| crate::behavior::Label {
                    arity: l.arity,
                    pc: l.pc,
                    continuation: l.continuation,
                })
                .ok_or(Error::LabelStackUnderflow)
        }
        fn get_label(&self, index: usize) -> Option<&crate::behavior::Label> {
            // This requires conversion which is inefficient. Returning None for now.
            // Test setup should avoid needing this if possible.
            None
        }
    }

    // Implement Stack trait (which requires StackBehavior)
    impl Stack for MockStack {
        // We only need to implement the label methods specific to the `Stack` trait
        // (using stack::Label aka StackLabel)
        fn push_label(&mut self, label: StackLabel) -> Result<()> {
            self.labels.push(label);
            Ok(())
        }
        fn pop_label(&mut self) -> Result<StackLabel> {
            self.labels.pop().ok_or(Error::LabelStackUnderflow)
        }
        fn get_label(&self, idx: usize) -> Result<&StackLabel> {
            let len = self.labels.len();
            if idx >= len {
                return Err(Error::InvalidLabelIndex(idx));
            }
            // Stack indices are relative from top (0 is top)
            self.labels
                .get(len - 1 - idx)
                .ok_or(Error::InvalidLabelIndex(idx))
        }
        fn get_label_mut(&mut self, idx: usize) -> Result<&mut StackLabel> {
            let len = self.labels.len();
            if idx >= len {
                return Err(Error::InvalidLabelIndex(idx));
            }
            // Stack indices are relative from top (0 is top)
            self.labels
                .get_mut(len - 1 - idx)
                .ok_or(Error::InvalidLabelIndex(idx))
        }
        fn labels_len(&self) -> usize {
            self.labels.len()
        }
    }

    #[derive(Debug)]
    struct MockMemory {
        data: Arc<RwLock<Vec<u8>>>,
        mem_type: crate::types::MemoryType,
        peak_memory_used: Arc<RwLock<usize>>,
    }

    impl MockMemory {
        fn new(size_pages: u32) -> Self {
            let mem_type = crate::types::MemoryType {
                min: size_pages,
                max: None,
            };
            Self::new_with_type(mem_type)
        }

        fn new_with_type(mem_type: crate::types::MemoryType) -> Self {
            let initial_size = (mem_type.min as usize) * PAGE_SIZE;
            Self {
                data: Arc::new(RwLock::new(vec![0; initial_size])),
                mem_type,
                peak_memory_used: Arc::new(RwLock::new(initial_size)),
            }
        }

        fn check_bounds(&self, addr: u32, len: u32) -> Result<()> {
            let data_len = self.data.read().unwrap().len();
            let end = addr.checked_add(len).ok_or_else(|| {
                Error::InvalidMemoryAccess(format!("Address overflow: addr={}, len={}", addr, len))
            })?;
            if (end as usize) > data_len {
                Err(Error::InvalidMemoryAccess(format!(
                    "Access out of bounds: addr={}, len={}, size={}",
                    addr, len, data_len
                )))
            } else {
                Ok(())
            }
        }
    }

    impl MemoryBehavior for MockMemory {
        fn type_(&self) -> &crate::types::MemoryType {
            &self.mem_type
        }
        fn size(&self) -> u32 {
            (self.data.read().unwrap().len() / PAGE_SIZE) as u32
        }
        fn size_bytes(&self) -> usize {
            self.data.read().unwrap().len()
        }
        fn grow(&self, pages: u32) -> Result<u32> {
            let old_size_bytes = self.data.read().unwrap().len();
            let old_size_pages = (old_size_bytes / PAGE_SIZE) as u32;

            let new_size_pages = old_size_pages
                .checked_add(pages)
                .ok_or_else(|| Error::MemoryGrowError("Addition overflow".to_string()))?;

            if let Some(max) = self.mem_type.max {
                if new_size_pages > max {
                    return Ok(u32::MAX); // Return -1 as u32::MAX
                }
            }

            let new_size_bytes = new_size_pages as usize * PAGE_SIZE;

            let mut data_guard = self.data.write().unwrap();
            data_guard.resize(new_size_bytes, 0);

            let mut peak_guard = self.peak_memory_used.write().unwrap();
            *peak_guard = (*peak_guard).max(new_size_bytes);

            Ok(old_size_pages)
        }
        fn read_byte(&self, addr: u32) -> Result<u8> {
            self.check_bounds(addr, 1)?;
            Ok(self.data.read().unwrap()[addr as usize])
        }
        fn write_byte(&self, addr: u32, value: u8) -> Result<()> {
            self.check_bounds(addr, 1)?;
            self.data.write().unwrap()[addr as usize] = value;
            Ok(())
        }
        fn read_bytes(&self, addr: u32, len: usize) -> Result<Vec<u8>> {
            self.check_bounds(addr, len as u32)?;
            Ok(self.data.read().unwrap()[addr as usize..addr as usize + len].to_vec())
        }
        fn write_bytes(&self, addr: u32, bytes: &[u8]) -> Result<()> {
            self.check_bounds(addr, bytes.len() as u32)?;
            self.data.write().unwrap()[addr as usize..addr as usize + bytes.len()]
                .copy_from_slice(bytes);
            Ok(())
        }
        fn check_alignment(&self, addr: u32, _access_size: u32, align: u32) -> Result<()> {
            let alignment = 1 << align;
            if addr % alignment != 0 {
                return Err(Error::InvalidAlignment(format!(
                    "Invalid alignment: address {} is not aligned to {}",
                    addr, alignment
                )));
            }
            Ok(())
        }
        fn read_i8(&self, addr: u32) -> Result<i8> {
            Ok(self.read_byte(addr)? as i8)
        }
        fn write_i8(&self, addr: u32, value: i8) -> Result<()> {
            self.write_byte(addr, value as u8)
        }
        fn read_u8(&self, addr: u32) -> Result<u8> {
            Ok(self.read_byte(addr)?)
        }
        fn write_u8(&self, addr: u32, value: u8) -> Result<()> {
            self.write_byte(addr, value)
        }
        fn read_i16(&self, addr: u32) -> Result<i16> {
            Ok(i16::from_le_bytes(
                self.read_bytes(addr, 2)?.try_into().unwrap(),
            ))
        }
        fn write_i16(&self, addr: u32, value: i16) -> Result<()> {
            self.write_bytes(addr, &value.to_le_bytes())
        }
        fn read_u16(&self, addr: u32) -> Result<u16> {
            Ok(u16::from_le_bytes(
                self.read_bytes(addr, 2)?.try_into().unwrap(),
            ))
        }
        fn write_u16(&self, addr: u32, value: u16) -> Result<()> {
            self.write_bytes(addr, &value.to_le_bytes())
        }
        fn read_i32(&self, addr: u32) -> Result<i32> {
            Ok(i32::from_le_bytes(
                self.read_bytes(addr, 4)?.try_into().unwrap(),
            ))
        }
        fn write_i32(&self, addr: u32, value: i32) -> Result<()> {
            self.write_bytes(addr, &value.to_le_bytes())
        }
        fn read_u32(&self, addr: u32) -> Result<u32> {
            Ok(u32::from_le_bytes(
                self.read_bytes(addr, 4)?.try_into().unwrap(),
            ))
        }
        fn write_u32(&self, addr: u32, value: u32) -> Result<()> {
            self.write_bytes(addr, &value.to_le_bytes())
        }
        fn read_i64(&self, addr: u32) -> Result<i64> {
            Ok(i64::from_le_bytes(
                self.read_bytes(addr, 8)?.try_into().unwrap(),
            ))
        }
        fn write_i64(&self, addr: u32, value: i64) -> Result<()> {
            self.write_bytes(addr, &value.to_le_bytes())
        }
        fn read_u64(&self, addr: u32) -> Result<u64> {
            Ok(u64::from_le_bytes(
                self.read_bytes(addr, 8)?.try_into().unwrap(),
            ))
        }
        fn write_u64(&self, addr: u32, value: u64) -> Result<()> {
            self.write_bytes(addr, &value.to_le_bytes())
        }
        fn read_f32(&self, addr: u32) -> Result<f32> {
            Ok(f32::from_bits(self.read_i32(addr)? as u32))
        }
        fn write_f32(&self, addr: u32, value: f32) -> Result<()> {
            self.write_i32(addr, value.to_bits() as i32)
        }
        fn read_f64(&self, addr: u32) -> Result<f64> {
            Ok(f64::from_bits(self.read_i64(addr)? as u64))
        }
        fn write_f64(&self, addr: u32, value: f64) -> Result<()> {
            self.write_i64(addr, value.to_bits() as i64)
        }
        fn read_v128(&self, addr: u32) -> Result<[u8; 16]> {
            self.check_bounds(addr, 16)?;
            let data_guard = self.data.read().unwrap();
            let bytes: [u8; 16] = data_guard[addr as usize..addr as usize + 16]
                .try_into()
                .map_err(|_| Error::Execution("Failed to read 16 bytes for v128".to_string()))?;
            Ok(bytes)
        }
        fn write_v128(&self, addr: u32, value: [u8; 16]) -> Result<()> {
            self.check_bounds(addr, 16)?;
            self.data.write().unwrap()[addr as usize..addr as usize + 16].copy_from_slice(&value);
            Ok(())
        }
        fn peak_memory_used(&self) -> usize {
            *self.peak_memory_used.read().unwrap()
        }
        fn reset_peak_memory_used(&self) {
            *self.peak_memory_used.write().unwrap() = 0;
        }
        fn fill(&self, addr: usize, val: u8, len: usize) -> Result<()> {
            self.check_bounds(addr as u32, len as u32)?;
            self.data.write().unwrap()[addr..addr + len].fill(val);
            Ok(())
        }
        fn copy(&self, dst_addr: usize, src_addr: usize, len: usize) -> Result<()> {
            self.check_bounds(src_addr as u32, len as u32)?;
            self.check_bounds(dst_addr as u32, len as u32)?;
            self.data
                .write()
                .unwrap()
                .copy_within(src_addr..src_addr + len, dst_addr);
            Ok(())
        }
        fn get_data_segment(&self, _idx: u32) -> Result<Arc<DataSegment>> {
            Err(Error::Unimplemented("get_data_segment mock".to_string()))
        }
    }

    #[derive(Clone)]
    struct MockFrame {
        memory: Arc<MockMemory>,
        locals: Vec<Value>,
        pc: usize,
        func_idx: u32,
        instance_idx: usize,
        label_stack: Vec<Label>,
        arity: usize,
        return_pc: usize,
        data_segments: Vec<Option<Arc<DataSegment>>>,
    }

    impl MockFrame {
        fn new(memory: Arc<MockMemory>) -> Self {
            Self {
                memory,
                locals: vec![],
                pc: 0,
                func_idx: 0,
                instance_idx: 0,
                label_stack: vec![],
                arity: 0,
                return_pc: 0,
                data_segments: vec![None; 1], // Start with one data segment slot
            }
        }
    }

    impl FrameBehavior for MockFrame {
        fn locals(&mut self) -> &mut Vec<Value> {
            &mut self.locals
        }
        fn get_local(&self, idx: usize) -> Result<Value> {
            self.locals
                .get(idx)
                .cloned()
                .ok_or(Error::InvalidLocalIndex(idx))
        }
        fn set_local(&mut self, idx: usize, value: Value) -> Result<()> {
            if idx < self.locals.len() {
                self.locals[idx] = value;
                Ok(())
            } else {
                Err(Error::InvalidLocalIndex(idx))
            }
        }
        fn get_global(&self, _idx: usize) -> Result<Arc<Global>> {
            Err(Error::Unimplemented("get_global mock".to_string()))
        }
        fn set_global(&mut self, _idx: usize, _value: Value) -> Result<()> {
            Err(Error::Unimplemented("set_global mock".to_string()))
        }
        fn get_table(&self, _idx: usize) -> Result<Arc<Table>> {
            Err(Error::Unimplemented("get_table mock".to_string()))
        }
        fn get_function_type(&self, _func_idx: u32) -> Result<FuncType> {
            Err(Error::Unimplemented("get_function_type mock".to_string()))
        }
        fn pc(&self) -> usize {
            self.pc
        }
        fn set_pc(&mut self, pc: usize) {
            self.pc = pc;
        }
        fn func_idx(&self) -> u32 {
            self.func_idx
        }
        fn instance_idx(&self) -> u32 {
            0
        }
        fn locals_len(&self) -> usize {
            self.locals.len()
        }
        fn label_stack(&mut self) -> &mut Vec<Label> {
            &mut self.label_stack
        }
        fn arity(&self) -> usize {
            self.arity
        }
        fn set_arity(&mut self, arity: usize) {
            self.arity = arity;
        }
        fn label_arity(&self) -> usize {
            self.label_stack.last().map_or(self.arity, |l| l.arity)
        }
        fn return_pc(&self) -> usize {
            self.return_pc
        }
        fn set_return_pc(&mut self, pc: usize) {
            self.return_pc = pc;
        }
        fn as_any(&mut self) -> &mut dyn std::any::Any {
            self
        }
        fn get_memory(&self, _idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
            Ok(self.memory.clone())
        }
        fn get_memory_mut(&mut self, _idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
            Ok(self.memory.clone())
        }
        fn get_table_mut(&mut self, _idx: usize) -> Result<Arc<Table>> {
            Err(Error::Unimplemented("get_table_mut mock".to_string()))
        }
        fn load_i32(&self, addr: usize, align: u32) -> Result<i32> {
            self.memory.check_alignment(addr as u32, 4, align)?;
            self.memory.read_i32(addr as u32)
        }
        fn store_i32(&mut self, addr: usize, align: u32, value: i32) -> Result<()> {
            self.memory.check_alignment(addr as u32, 4, align)?;
            self.memory.write_i32(addr as u32, value)
        }
        fn load_i64(&self, addr: usize, align: u32) -> Result<i64> {
            self.memory.check_alignment(addr as u32, 8, align)?;
            self.memory.read_i64(addr as u32)
        }
        fn store_i64(&mut self, addr: usize, align: u32, value: i64) -> Result<()> {
            self.memory.check_alignment(addr as u32, 8, align)?;
            self.memory.write_i64(addr as u32, value)
        }
        fn load_f32(&self, addr: usize, align: u32) -> Result<f32> {
            self.memory.check_alignment(addr as u32, 4, align)?;
            self.memory.read_f32(addr as u32)
        }
        fn store_f32(&mut self, addr: usize, align: u32, value: f32) -> Result<()> {
            self.memory.check_alignment(addr as u32, 4, align)?;
            self.memory.write_f32(addr as u32, value)
        }
        fn load_f64(&self, addr: usize, align: u32) -> Result<f64> {
            self.memory.check_alignment(addr as u32, 8, align)?;
            self.memory.read_f64(addr as u32)
        }
        fn store_f64(&mut self, addr: usize, align: u32, value: f64) -> Result<()> {
            self.memory.check_alignment(addr as u32, 8, align)?;
            self.memory.write_f64(addr as u32, value)
        }
        fn load_i8(&self, addr: usize, align: u32) -> Result<i8> {
            self.memory.check_alignment(addr as u32, 1, align)?;
            self.memory.read_i8(addr as u32)
        }
        fn store_i8(&mut self, addr: usize, align: u32, value: i8) -> Result<()> {
            self.memory.check_alignment(addr as u32, 1, align)?;
            self.memory.write_i8(addr as u32, value)
        }
        fn load_u8(&self, addr: usize, align: u32) -> Result<u8> {
            self.memory.check_alignment(addr as u32, 1, align)?;
            self.memory.read_u8(addr as u32)
        }
        fn store_u8(&mut self, addr: usize, align: u32, value: u8) -> Result<()> {
            self.memory.check_alignment(addr as u32, 1, align)?;
            self.memory.write_u8(addr as u32, value)
        }
        fn load_i16(&self, addr: usize, align: u32) -> Result<i16> {
            self.memory.check_alignment(addr as u32, 2, align)?;
            self.memory.read_i16(addr as u32)
        }
        fn store_i16(&mut self, addr: usize, align: u32, value: i16) -> Result<()> {
            self.memory.check_alignment(addr as u32, 2, align)?;
            self.memory.write_i16(addr as u32, value)
        }
        fn load_u16(&self, addr: usize, align: u32) -> Result<u16> {
            self.memory.check_alignment(addr as u32, 2, align)?;
            self.memory.read_u16(addr as u32)
        }
        fn store_u16(&mut self, addr: usize, align: u32, value: u16) -> Result<()> {
            self.memory.check_alignment(addr as u32, 2, align)?;
            self.memory.write_u16(addr as u32, value)
        }
        fn load_v128(&self, addr: usize, align: u32) -> Result<[u8; 16]> {
            self.memory.check_alignment(addr as u32, 16, align)?;
            self.memory.read_v128(addr as u32)
        }
        fn store_v128(&mut self, addr: usize, align: u32, value: [u8; 16]) -> Result<()> {
            self.memory.check_alignment(addr as u32, 16, align)?;
            self.memory.write_v128(addr as u32, value)
        }
        fn memory_size(&self) -> Result<u32> {
            Ok(self.memory.size())
        }
        fn memory_grow(&mut self, pages: u32) -> Result<u32> {
            self.memory.grow(pages)
        }
        fn table_get(&self, _table_idx: u32, _idx: u32) -> Result<Value> {
            Err(Error::Unimplemented("table_get mock".to_string()))
        }
        fn table_set(&mut self, _table_idx: u32, _idx: u32, _value: Value) -> Result<()> {
            Err(Error::Unimplemented("table_set mock".to_string()))
        }
        fn table_size(&self, _table_idx: u32) -> Result<u32> {
            Err(Error::Unimplemented("table_size mock".to_string()))
        }
        fn table_grow(&mut self, _table_idx: u32, _delta: u32, _value: Value) -> Result<u32> {
            Err(Error::Unimplemented("table_grow mock".to_string()))
        }
        fn table_init(
            &mut self,
            _table_idx: u32,
            _elem_idx: u32,
            _dst: u32,
            _src: u32,
            _n: u32,
        ) -> Result<()> {
            Err(Error::Unimplemented("table_init mock".to_string()))
        }
        fn table_copy(
            &mut self,
            _dst_table: u32,
            _src_table: u32,
            _dst: u32,
            _src: u32,
            _n: u32,
        ) -> Result<()> {
            Err(Error::Unimplemented("table_copy mock".to_string()))
        }
        fn elem_drop(&mut self, _elem_idx: u32) -> Result<()> {
            Err(Error::Unimplemented("elem_drop mock".to_string()))
        }
        fn table_fill(&mut self, _table_idx: u32, _dst: u32, _val: Value, _n: u32) -> Result<()> {
            Err(Error::Unimplemented("table_fill mock".to_string()))
        }
        fn pop_bool(&mut self, _stack: &mut dyn Stack) -> Result<bool> {
            Err(Error::Unimplemented("pop_bool mock".to_string()))
        }
        fn pop_i32(&mut self, _stack: &mut dyn Stack) -> Result<i32> {
            Err(Error::Unimplemented("pop_i32 mock".to_string()))
        }
        fn get_two_tables_mut(
            &mut self,
            _idx1: u32,
            _idx2: u32,
        ) -> Result<(MutexGuard<Table>, MutexGuard<Table>)> {
            Err(Error::Unimplemented("get_two_tables_mut mock".to_string()))
        }
        fn set_data_segment(&mut self, idx: u32, segment: Arc<DataSegment>) -> Result<()> {
            if idx as usize >= self.data_segments.len() {
                self.data_segments.resize(idx as usize + 1, None);
            }
            self.data_segments[idx as usize] = Some(segment);
            Ok(())
        }
        fn drop_data_segment(&mut self, idx: u32) -> Result<()> {
            if idx as usize >= self.data_segments.len()
                || self.data_segments[idx as usize].is_none()
            {
                return Err(Error::InvalidDataIndex(idx));
            }
            self.data_segments[idx as usize] = None;
            Ok(())
        }
    }

    impl ControlFlowBehavior for MockFrame {
        fn enter_block(&mut self, _ty: BlockType, _stack_len: usize) -> Result<()> {
            Ok(())
        }
        fn enter_loop(&mut self, _ty: BlockType, _stack_len: usize) -> Result<()> {
            Ok(())
        }
        fn enter_if(&mut self, _ty: BlockType, _stack_len: usize, _condition: bool) -> Result<()> {
            Ok(())
        }
        fn enter_else(&mut self, _stack_len: usize) -> Result<()> {
            Ok(())
        }
        fn exit_block(&mut self, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn branch(&mut self, _label_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn return_(&mut self, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn call(&mut self, _func_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
            Ok(())
        }
        fn call_indirect(
            &mut self,
            _type_idx: u32,
            _table_idx: u32,
            _entry: u32,
            _stack: &mut dyn Stack,
        ) -> Result<()> {
            Ok(())
        }
        fn set_label_arity(&mut self, _arity: usize) {}
    }

    fn setup_test() -> (MockStack, MockFrame, StacklessEngine) {
        let memory = Arc::new(MockMemory::new(1));
        let frame = MockFrame::new(memory.clone());
        let stack = MockStack::new();
        let engine = StacklessEngine::new();
        (stack, frame, engine)
    }

    // --- Tests ---

    #[test]
    fn test_i32_load() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(0))?;
        frame.store_i32(0, 0, 12345)?;
        i32_load(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(12345));
        Ok(())
    }

    #[test]
    fn test_i64_load() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(8))?;
        frame.store_i64(8, 0, 9876543210)?;
        i64_load(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I64(9876543210));
        Ok(())
    }

    #[test]
    fn test_f32_load() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(16))?;
        frame.store_f32(16, 0, 3.14)?;
        f32_load(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::F32(3.14));
        Ok(())
    }

    #[test]
    fn test_f64_load() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(24))?;
        frame.store_f64(24, 0, 2.71828)?;
        f64_load(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::F64(2.71828));
        Ok(())
    }

    #[test]
    fn test_i32_load8_s() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(0))?;
        frame.store_i8(0, 0, -1)?;
        i32_load8_s(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(-1));
        Ok(())
    }

    #[test]
    fn test_i32_load8_u() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(1))?;
        frame.store_u8(1, 0, 255)?;
        i32_load8_u(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(255));
        Ok(())
    }

    #[test]
    fn test_i32_load16_s() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(2))?;
        frame.store_i16(2, 0, -1)?;
        i32_load16_s(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(-1));
        Ok(())
    }

    #[test]
    fn test_i32_load16_u() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(4))?;
        frame.store_u16(4, 0, 65535)?;
        i32_load16_u(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(65535));
        Ok(())
    }

    #[test]
    fn test_i32_store() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(100))?;
        stack.push(Value::I32(54321))?;
        i32_store(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_i32(100, 0)?, 54321);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_i64_store() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(108))?;
        stack.push(Value::I64(1234567890123))?;
        i64_store(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_i64(108, 0)?, 1234567890123);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_f32_store() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(116))?;
        stack.push(Value::F32(1.234))?;
        f32_store(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_f32(116, 0)?, 1.234);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_f64_store() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(120))?;
        stack.push(Value::F64(5.678))?;
        f64_store(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_f64(120, 0)?, 5.678);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_i32_store8() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(128))?;
        stack.push(Value::I32(0xABCDEF42))?;
        i32_store8(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_u8(128, 0)?, 0x42);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_i32_store16() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(130))?;
        stack.push(Value::I32(0xABCDEF42))?;
        i32_store16(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_u16(130, 0)?, 0xEF42);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_i64_store8() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(132))?;
        stack.push(Value::I64(0x12345678_ABCDEF88))?;
        i64_store8(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_u8(132, 0)?, 0x88);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_i64_store16() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(134))?;
        stack.push(Value::I64(0x12345678_ABCDEF88))?;
        i64_store16(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_u16(134, 0)?, 0xEF88);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_i64_store32() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(136))?;
        stack.push(Value::I64(0x12345678_ABCDEF88))?;
        i64_store32(&mut stack, &mut frame, 0, 0, &StacklessEngine::new())?;
        assert_eq!(frame.load_i32(136, 0)?, 0xABCDEF88_u32 as i32);
        assert!(stack.is_empty());
        Ok(())
    }

    #[test]
    fn test_memory_size() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        memory_size(&mut stack, &frame, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(1));
        Ok(())
    }

    #[test]
    fn test_memory_grow() -> Result<()> {
        let memory_arc = Arc::new(MockMemory::new(1));
        let mut frame = MockFrame::new(memory_arc.clone());
        let mut stack = MockStack::new();
        stack.push(Value::I32(2))?;
        memory_grow(&mut stack, &mut frame, 0, &StacklessEngine::new())?;
        assert_eq!(stack.pop()?, Value::I32(1));
        assert_eq!(frame.memory_size()?, 3);

        let mem_max_type = crate::types::MemoryType {
            min: 1,
            max: Some(1),
        };
        let memory_max = Arc::new(MockMemory::new_with_type(mem_max_type));
        let mut frame_max = MockFrame::new(memory_max);
        let mut stack_max = MockStack::new();
        stack_max.push(Value::I32(1))?;
        memory_grow(&mut stack_max, &mut frame_max, 0, &StacklessEngine::new())?;
        // WASM spec says memory.grow returns -1 (represented as u32::MAX) on failure
        assert_eq!(stack_max.pop()?, Value::I32(u32::MAX as i32));
        assert_eq!(frame_max.memory_size()?, 1);
        Ok(())
    }

    #[test]
    fn test_v128_store() -> Result<()> {
        let (mut stack, mut frame, engine) = setup_test();
        let data = [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        stack.push(Value::I32(16))?;
        stack.push(Value::V128(data))?;
        v128_store(&mut stack, &mut frame, 0, 0, &engine)?;
        let memory = frame.get_memory(0).unwrap();
        let stored_data = memory.read_v128(16)?;
        assert_eq!(stored_data, data);
        Ok(())
    }

    #[test]
    fn test_memory_fill() -> Result<()> {
        let (mut stack, mut frame, engine) = setup_test();
        stack.push(Value::I32(10))?;
        stack.push(Value::I32(0xAB))?;
        stack.push(Value::I32(5))?;
        memory_fill(&mut stack, &mut frame, 0, &engine)?;

        for i in 0..5 {
            assert_eq!(frame.load_u8(10 + i, 1)?, 0xAB);
        }
        assert!(stack.is_empty());

        stack.push(Value::I32(crate::memory::PAGE_SIZE as i32 - 2))?;
        stack.push(Value::I32(0xFF))?;
        stack.push(Value::I32(5))?;
        let result = memory_fill(&mut stack, &mut frame, 0, &engine);
        assert!(matches!(result, Err(Error::MemoryAccessOutOfBounds(_))));
        Ok(())
    }

    #[test]
    fn test_memory_copy() -> Result<()> {
        let (mut stack, mut frame, engine) = setup_test();
        let memory = frame.get_memory(0).unwrap();
        let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        memory.write_bytes(0, &data)?;
        stack.push(Value::I32(16))?; // dst
        stack.push(Value::I32(0))?; // src
        stack.push(Value::I32(16))?; // len
        memory_copy(&mut stack, &mut frame, 0, 0, &engine)?;
        let copied_data = memory.read_bytes(16, 16)?;
        assert_eq!(copied_data, data);
        Ok(())
    }

    #[test]
    fn test_memory_init() -> Result<()> {
        let (mut stack, mut frame, engine) = setup_test();
        let data_vec = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let data_segment = Arc::new(crate::module::DataSegment::new(data_vec.clone()));
        frame.set_data_segment(0, data_segment)?;
        stack.push(Value::I32(0))?; // dst
        stack.push(Value::I32(0))?; // offset in data segment
        stack.push(Value::I32(16))?; // len
        memory_init(&mut stack, &mut frame, 0, 0, &engine)?;
        let memory = frame.get_memory(0).unwrap();
        let memory_data = memory.read_bytes(0, 16)?;
        assert_eq!(memory_data, data_vec);
        Ok(())
    }

    #[test]
    fn test_data_drop() -> Result<()> {
        let (mut stack, mut frame, engine) = setup_test();
        let data_vec = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let data_segment = Arc::new(crate::module::DataSegment::new(data_vec));
        frame.set_data_segment(0, data_segment)?;
        stack.push(Value::I32(0))?; // data index
        data_drop(&mut stack, &mut frame, 0, &engine)?;
        assert!(
            frame.drop_data_segment(0).is_err(),
            "Data segment should have been dropped by the instruction"
        );
        assert!(stack.is_empty());
        Ok(())
    }
}
