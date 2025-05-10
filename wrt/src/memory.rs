//! Module for WebAssembly linear memory
//!
//! This module provides memory types and re-exports for WebAssembly memory.
//!
//! # Safety Features
//!
//! The memory implementation includes several safety features:
//!
//! - Checksum verification for data integrity
//! - Bounds checking for all memory operations
//! - Alignment validation
//! - Thread safety guarantees
//! - Memory access tracking
//!
//! # Usage
//!
//! ```no_run
//! use wrt::{Memory, MemoryType};
//! use wrt_types::types::Limits;
//!
//! // Create a memory type with initial 1 page (64KB) and max 2 pages
//! let mem_type = MemoryType {
//!     limits: Limits { min: 1, max: Some(2) },
//! };
//!
//! // Create a new memory instance
//! let mut memory = create_memory(mem_type).unwrap();
//!
//! // Write data to memory
//! memory.write(0, &[1, 2, 3, 4]).unwrap();
//!
//! // Read data from memory
//! let mut buffer = [0; 4];
//! memory.read(0, &mut buffer).unwrap();
//! assert_eq!(buffer, [1, 2, 3, 4]);
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use crate::{
    behavior::{ControlFlow, FrameBehavior, InstructionExecutor, StackBehavior},
    error::{kinds, Error, Result},
    prelude::TypesValue as Value,
    stackless::StacklessEngine,
};

use wrt_error::Result as WrtResult;
use wrt_types::safe_memory::{MemoryProvider, MemorySafety, SafeSlice};
use wrt_types::verification::VerificationLevel;

// Re-export memory types from wrt-runtime
pub use wrt_runtime::{Memory, MemoryType, PAGE_SIZE};

// Re-export the memory operations from wrt-instructions
#[cfg(feature = "std")]
pub use wrt_instructions::memory_ops::{MemoryLoad, MemoryStore};

/// Maximum number of memory pages allowed by WebAssembly spec
pub const MAX_PAGES: u32 = 65536;

/// Create a new memory instance
///
/// This is a convenience function that creates a memory instance
/// with the given type.
///
/// # Arguments
///
/// * `mem_type` - The memory type
///
/// # Returns
///
/// A new memory instance
///
/// # Errors
///
/// Returns an error if the memory cannot be created
pub fn create_memory(mem_type: MemoryType) -> Result<Memory> {
    Memory::new(mem_type)
}

/// Create a new memory instance with a name
///
/// This is a convenience function that creates a memory instance
/// with the given type and name.
///
/// # Arguments
///
/// * `mem_type` - The memory type
/// * `name` - The debug name for the memory
///
/// # Returns
///
/// A new memory instance
///
/// # Errors
///
/// Returns an error if the memory cannot be created
pub fn create_memory_with_name(mem_type: MemoryType, name: &str) -> Result<Memory> {
    Memory::new_with_name(mem_type, name)
}

/// Create a new memory instance with a specific verification level
///
/// This is a convenience function that creates a memory instance
/// with the given type and verification level.
///
/// # Arguments
///
/// * `mem_type` - The memory type
/// * `level` - The verification level
///
/// # Returns
///
/// A new memory instance
///
/// # Errors
///
/// Returns an error if the memory cannot be created
pub fn create_memory_with_verification(
    mem_type: MemoryType,
    level: VerificationLevel,
) -> Result<Memory> {
    let mut memory = Memory::new(mem_type)?;
    memory.set_verification_level(level);
    Ok(memory)
}

/// Get a safe slice of memory with integrity verification
///
/// This is a convenience function that gets a safe slice of memory
/// with the given offset and length.
///
/// # Arguments
///
/// * `memory` - The memory instance
/// * `offset` - The offset in bytes
/// * `len` - The length in bytes
///
/// # Returns
///
/// A safe slice with integrity verification
///
/// # Errors
///
/// Returns an error if the slice would be invalid
pub fn get_safe_slice(memory: &Memory, offset: usize, len: usize) -> Result<SafeSlice<'_>> {
    memory.borrow_slice(offset, len)
}

/// Verify the integrity of a memory instance
///
/// This is a convenience function that verifies the integrity
/// of a memory instance.
///
/// # Arguments
///
/// * `memory` - The memory instance
///
/// # Errors
///
/// Returns an error if memory corruption is detected
pub fn verify_memory_integrity(memory: &Memory) -> Result<()> {
    memory.verify_integrity()
}

/// Get memory statistics
///
/// This is a convenience function that gets statistics about
/// a memory instance.
///
/// # Arguments
///
/// * `memory` - The memory instance
///
/// # Returns
///
/// Memory statistics
pub fn get_memory_stats(memory: &Memory) -> wrt_types::safe_memory::MemoryStats {
    memory.memory_stats()
}

/// Define a simple struct to represent memory arguments
#[derive(Debug, Clone, Copy)]
pub struct MemoryArg {
    pub offset: u32,
    pub align: u32,
}

/// WebAssembly Load instruction implementation
#[derive(Debug)]
pub struct Load {
    /// Offset within memory
    pub offset: u32,
    /// Alignment hint
    pub align: u32,
    /// Memory index (usually 0)
    pub mem_idx: u32,
    /// Type of the load (i32, i64, f32, f64)
    pub load_type: LoadType,
}

/// Type of WebAssembly load instructions
#[derive(Debug, Clone, Copy)]
pub enum LoadType {
    /// i32.load
    I32,
    /// i64.load
    I64,
    /// f32.load
    F32,
    /// f64.load
    F64,
}

impl Load {
    /// Create a new i32 load instruction
    pub fn i32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadType::I32,
        }
    }

    /// Create a new i64 load instruction
    pub fn i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadType::I64,
        }
    }

    /// Create a new f32 load instruction
    pub fn f32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadType::F32,
        }
    }

    /// Create a new f64 load instruction
    pub fn f64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadType::F64,
        }
    }
}

/// WebAssembly LoadSigned instruction implementation
#[derive(Debug)]
pub struct LoadSigned {
    /// Offset within memory
    pub offset: u32,
    /// Alignment hint
    pub align: u32,
    /// Memory index (usually 0)
    pub mem_idx: u32,
    /// Type of the load (i8_i32, i16_i32, i8_i64, i16_i64, i32_i64)
    pub load_type: LoadSignedType,
}

/// Type of WebAssembly signed load instructions
#[derive(Debug, Clone, Copy)]
pub enum LoadSignedType {
    /// i32.load8_s
    I8_I32,
    /// i32.load16_s
    I16_I32,
    /// i64.load8_s
    I8_I64,
    /// i64.load16_s
    I16_I64,
    /// i64.load32_s
    I32_I64,
}

impl LoadSigned {
    /// Create a new i8->i32 load instruction with sign extension
    pub fn i8_i32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadSignedType::I8_I32,
        }
    }

    /// Create a new i16->i32 load instruction with sign extension
    pub fn i16_i32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadSignedType::I16_I32,
        }
    }

    /// Create a new i8->i64 load instruction with sign extension
    pub fn i8_i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadSignedType::I8_I64,
        }
    }

    /// Create a new i16->i64 load instruction with sign extension
    pub fn i16_i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadSignedType::I16_I64,
        }
    }

    /// Create a new i32->i64 load instruction with sign extension
    pub fn i32_i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadSignedType::I32_I64,
        }
    }
}

/// WebAssembly LoadUnsigned instruction implementation
#[derive(Debug)]
pub struct LoadUnsigned {
    /// Offset within memory
    pub offset: u32,
    /// Alignment hint
    pub align: u32,
    /// Memory index (usually 0)
    pub mem_idx: u32,
    /// Type of the load (u8_i32, u16_i32, u8_i64, u16_i64, u32_i64)
    pub load_type: LoadUnsignedType,
}

/// Type of WebAssembly unsigned load instructions
#[derive(Debug, Clone, Copy)]
pub enum LoadUnsignedType {
    /// i32.load8_u
    U8_I32,
    /// i32.load16_u
    U16_I32,
    /// i64.load8_u
    U8_I64,
    /// i64.load16_u
    U16_I64,
    /// i64.load32_u
    U32_I64,
}

impl LoadUnsigned {
    /// Create a new u8->i32 load instruction with zero extension
    pub fn u8_i32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadUnsignedType::U8_I32,
        }
    }

    /// Create a new u16->i32 load instruction with zero extension
    pub fn u16_i32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadUnsignedType::U16_I32,
        }
    }

    /// Create a new u8->i64 load instruction with zero extension
    pub fn u8_i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadUnsignedType::U8_I64,
        }
    }

    /// Create a new u16->i64 load instruction with zero extension
    pub fn u16_i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadUnsignedType::U16_I64,
        }
    }

    /// Create a new u32->i64 load instruction with zero extension
    pub fn u32_i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            load_type: LoadUnsignedType::U32_I64,
        }
    }
}

/// WebAssembly Store instruction implementation
#[derive(Debug)]
pub struct Store {
    /// Offset within memory
    pub offset: u32,
    /// Alignment hint
    pub align: u32,
    /// Memory index (usually 0)
    pub mem_idx: u32,
    /// Type of the store (i32, i64, f32, f64)
    pub store_type: StoreType,
}

/// Type of WebAssembly store instructions
#[derive(Debug, Clone, Copy)]
pub enum StoreType {
    /// i32.store
    I32,
    /// i64.store
    I64,
    /// f32.store
    F32,
    /// f64.store
    F64,
}

impl Store {
    /// Create a new i32 store instruction
    pub fn i32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            store_type: StoreType::I32,
        }
    }

    /// Create a new i64 store instruction
    pub fn i64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            store_type: StoreType::I64,
        }
    }

    /// Create a new f32 store instruction
    pub fn f32(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            store_type: StoreType::F32,
        }
    }

    /// Create a new f64 store instruction
    pub fn f64(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            store_type: StoreType::F64,
        }
    }
}

/// WebAssembly StoreTruncated instruction implementation
#[derive(Debug)]
pub struct StoreTruncated<F, T> {
    /// Offset within memory
    pub offset: u32,
    /// Alignment hint
    pub align: u32,
    /// Memory index (usually 0)
    pub mem_idx: u32,
    /// Phantom data for from type
    pub _from: PhantomData<F>,
    /// Phantom data for to type
    pub _to: PhantomData<T>,
}

impl<F, T> StoreTruncated<F, T> {
    /// Create a new store truncated instruction
    pub fn new(offset: u32, align: u32) -> Self {
        Self {
            offset,
            align,
            mem_idx: 0,
            _from: PhantomData,
            _to: PhantomData,
        }
    }
}

/// WebAssembly MemoryInit instruction implementation
#[derive(Debug)]
pub struct MemoryInit {
    /// Data segment index
    pub data_idx: u32,
    /// Memory index (usually 0)
    pub mem_idx: u32,
}

impl MemoryInit {
    /// Create a new memory init instruction
    pub fn new(data_idx: u32, mem_idx: u32) -> Self {
        Self { data_idx, mem_idx }
    }
}

/// WebAssembly DataDrop instruction implementation
#[derive(Debug)]
pub struct DataDrop {
    /// Data segment index
    pub data_idx: u32,
}

impl DataDrop {
    /// Create a new data drop instruction
    pub fn new(data_idx: u32) -> Self {
        Self { data_idx }
    }
}

// Implementation of the core instructions

impl InstructionExecutor for Load {
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        let addr = stack.pop_i32()? as u32;
        let effective_addr = addr.wrapping_add(self.offset);

        let memory = frame.get_memory(self.mem_idx as usize, engine)?;

        match self.load_type {
            LoadType::I32 => {
                memory.check_alignment(effective_addr, 4, self.align)?;
                let value = memory.read_i32(effective_addr)?;
                stack.push(Value::I32(value))?;
            }
            LoadType::I64 => {
                memory.check_alignment(effective_addr, 8, self.align)?;
                let value = memory.read_i64(effective_addr)?;
                stack.push(Value::I64(value))?;
            }
            LoadType::F32 => {
                memory.check_alignment(effective_addr, 4, self.align)?;
                let value = memory.read_f32(effective_addr)?;
                stack.push(Value::F32(value))?;
            }
            LoadType::F64 => {
                memory.check_alignment(effective_addr, 8, self.align)?;
                let value = memory.read_f64(effective_addr)?;
                stack.push(Value::F64(value))?;
            }
        }

        Ok(ControlFlow::Continue)
    }
}

impl InstructionExecutor for LoadSigned {
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        let addr = stack.pop_i32()? as u32;
        let effective_addr = addr.wrapping_add(self.offset);

        let memory = frame.get_memory(self.mem_idx as usize, engine)?;

        match self.load_type {
            LoadSignedType::I8_I32 => {
                memory.check_alignment(effective_addr, 1, self.align)?;
                let value = memory.read_i8(effective_addr)? as i32;
                stack.push(Value::I32(value))?;
            }
            LoadSignedType::I16_I32 => {
                memory.check_alignment(effective_addr, 2, self.align)?;
                let value = memory.read_i16(effective_addr)? as i32;
                stack.push(Value::I32(value))?;
            }
            LoadSignedType::I8_I64 => {
                memory.check_alignment(effective_addr, 1, self.align)?;
                let value = memory.read_i8(effective_addr)? as i64;
                stack.push(Value::I64(value))?;
            }
            LoadSignedType::I16_I64 => {
                memory.check_alignment(effective_addr, 2, self.align)?;
                let value = memory.read_i16(effective_addr)? as i64;
                stack.push(Value::I64(value))?;
            }
            LoadSignedType::I32_I64 => {
                memory.check_alignment(effective_addr, 4, self.align)?;
                let value = memory.read_i32(effective_addr)? as i64;
                stack.push(Value::I64(value))?;
            }
        }

        Ok(ControlFlow::Continue)
    }
}

impl InstructionExecutor for LoadUnsigned {
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        let addr = stack.pop_i32()? as u32;
        let effective_addr = addr.wrapping_add(self.offset);

        let memory = frame.get_memory(self.mem_idx as usize, engine)?;

        match self.load_type {
            LoadUnsignedType::U8_I32 => {
                memory.check_alignment(effective_addr, 1, self.align)?;
                let value = memory.read_u8(effective_addr)? as i32;
                stack.push(Value::I32(value))?;
            }
            LoadUnsignedType::U16_I32 => {
                memory.check_alignment(effective_addr, 2, self.align)?;
                let value = memory.read_u16(effective_addr)? as i32;
                stack.push(Value::I32(value))?;
            }
            LoadUnsignedType::U8_I64 => {
                memory.check_alignment(effective_addr, 1, self.align)?;
                let value = memory.read_u8(effective_addr)? as i64;
                stack.push(Value::I64(value))?;
            }
            LoadUnsignedType::U16_I64 => {
                memory.check_alignment(effective_addr, 2, self.align)?;
                let value = memory.read_u16(effective_addr)? as i64;
                stack.push(Value::I64(value))?;
            }
            LoadUnsignedType::U32_I64 => {
                memory.check_alignment(effective_addr, 4, self.align)?;
                let value = memory.read_u32(effective_addr)? as i64;
                stack.push(Value::I64(value))?;
            }
        }

        Ok(ControlFlow::Continue)
    }
}

impl InstructionExecutor for Store {
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        match self.store_type {
            StoreType::I32 => {
                let value = stack.pop_i32()?;
                let addr = stack.pop_i32()? as u32;
                let effective_addr = addr.wrapping_add(self.offset);

                let memory = frame.get_memory(self.mem_idx as usize, engine)?;
                memory.check_alignment(effective_addr, 4, self.align)?;
                memory.write_i32(effective_addr, value)?;
            }
            StoreType::I64 => {
                let value = stack.pop_i64()?;
                let addr = stack.pop_i32()? as u32;
                let effective_addr = addr.wrapping_add(self.offset);

                let memory = frame.get_memory(self.mem_idx as usize, engine)?;
                memory.check_alignment(effective_addr, 8, self.align)?;
                memory.write_i64(effective_addr, value)?;
            }
            StoreType::F32 => {
                let value = stack.pop_f32()?;
                let addr = stack.pop_i32()? as u32;
                let effective_addr = addr.wrapping_add(self.offset);

                let memory = frame.get_memory(self.mem_idx as usize, engine)?;
                memory.check_alignment(effective_addr, 4, self.align)?;
                memory.write_f32(effective_addr, value)?;
            }
            StoreType::F64 => {
                let value = stack.pop_f64()?;
                let addr = stack.pop_i32()? as u32;
                let effective_addr = addr.wrapping_add(self.offset);

                let memory = frame.get_memory(self.mem_idx as usize, engine)?;
                memory.check_alignment(effective_addr, 8, self.align)?;
                memory.write_f64(effective_addr, value)?;
            }
        }

        Ok(ControlFlow::Continue)
    }
}

impl<F, T> InstructionExecutor for StoreTruncated<F, T>
where
    F: Copy + Into<i64> + 'static + std::fmt::Debug,
    T: Copy + 'static + std::fmt::Debug,
    i64: From<T>,
{
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        let value = match std::any::TypeId::of::<F>() {
            id if id == std::any::TypeId::of::<i32>() => {
                // Handle i32 case
                let value = stack.pop_i32()?;
                value as i64
            }
            _ => {
                // Handle i64 case
                let value = stack.pop_i64()?;
                value
            }
        };

        let addr = stack.pop_i32()? as u32;
        let effective_addr = addr.wrapping_add(self.offset);

        let memory = frame.get_memory(self.mem_idx as usize, engine)?;
        memory.check_alignment(effective_addr, std::mem::size_of::<T>() as u32, self.align)?;

        // Instead of using unsafe casts, handle each specific case
        match std::mem::size_of::<T>() {
            1 => memory.write_i8(effective_addr, value as i8)?,
            2 => memory.write_i16(effective_addr, value as i16)?,
            4 => memory.write_i32(effective_addr, value as i32)?,
            _ => {
                return Err(Error::new(kinds::ExecutionError(format!(
                    "Unsupported truncation size: {} bytes",
                    std::mem::size_of::<T>()
                ))))
            }
        }

        Ok(ControlFlow::Continue)
    }
}

impl InstructionExecutor for MemoryInit {
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        let n = stack.pop_i32()? as usize;
        let src = stack.pop_i32()? as usize;
        let dst = stack.pop_i32()? as usize;

        // Get the memory and data segment
        let memory = frame.get_memory(self.mem_idx as usize, engine)?;
        let data_segment = frame.get_data_segment(self.data_idx, engine)?;

        // Safety checks for bounds
        let mem_size = memory.size() as usize * PAGE_SIZE;
        let data_len = data_segment.data().len();

        if src.checked_add(n).map_or(true, |end| end > data_len)
            || dst.checked_add(n).map_or(true, |end| end > mem_size)
        {
            return Err(Error::new(kinds::MemoryAccessOutOfBoundsError {
                address: dst as u64,
                length: n as u64,
            }));
        }

        // Perform the initialization
        memory.init(dst, data_segment.data(), src, n)?;

        Ok(ControlFlow::Continue)
    }
}

impl InstructionExecutor for DataDrop {
    fn execute(
        &self,
        _stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        frame.drop_data_segment(self.data_idx, engine)?;
        Ok(ControlFlow::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_types::types::Limits;

    #[test]
    fn test_create_memory() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let memory = create_memory(mem_type).unwrap();
        assert_eq!(memory.size(), 1);
        assert_eq!(memory.size_in_bytes(), PAGE_SIZE);
    }

    #[test]
    fn test_create_memory_with_name() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let memory = create_memory_with_name(mem_type, "test").unwrap();
        assert_eq!(memory.debug_name(), Some("test"));
    }

    #[test]
    fn test_create_memory_with_verification() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let level = VerificationLevel::High;
        let memory = create_memory_with_verification(mem_type, level).unwrap();
        assert_eq!(memory.verification_level(), level);
    }

    #[test]
    fn test_get_safe_slice() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = create_memory(mem_type).unwrap();
        let data = [1, 2, 3, 4];
        memory.write(0, &data).unwrap();
        let slice = get_safe_slice(&memory, 0, 4).unwrap();
        assert_eq!(slice.data().unwrap(), &data);
    }

    #[test]
    fn test_verify_memory_integrity() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let memory = create_memory(mem_type).unwrap();
        verify_memory_integrity(&memory).unwrap();
    }

    #[test]
    fn test_get_memory_stats() {
        let mem_type = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(2),
            },
        };
        let mut memory = create_memory(mem_type).unwrap();
        let data = [1, 2, 3, 4];
        memory.write(0, &data).unwrap();
        let stats = get_memory_stats(&memory);
        assert_eq!(stats.total_size, PAGE_SIZE);
        assert_eq!(stats.access_count, 1);
    }
}
