// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Memory operations for WebAssembly instructions.
//!
//! This module provides implementations for WebAssembly memory access
//! instructions, including load and store operations for various value types.
//!
//! # Memory Operation Architecture
//!
//! This module separates memory operations from the underlying memory
//! implementation, allowing different execution engines to share the same
//! memory access code. The key components are:
//!
//! - `MemoryLoad`: Handles all WebAssembly load operations with various data
//!   types and widths
//! - `MemoryStore`: Handles all WebAssembly store operations with various data
//!   types and widths
//!
//! Both structures work with the `Memory` implementation from the `wrt-runtime`
//! crate.
//!
//! # Features
//!
//! - Support for all WebAssembly numeric types (i32, i64, f32, f64)
//! - Support for partial-width loads and stores (8-bit, 16-bit, 32-bit)
//! - Signed and unsigned operations
//! - Alignment checking and validation
//! - Bounds checking
//!
//! # Memory Safety
//!
//! All memory operations perform proper bounds and alignment checking before
//! accessing memory. This ensures that WebAssembly's memory safety guarantees
//! are preserved.
//!
//! # Usage
//!
//! ```no_run
//! use wrt_instructions::memory_ops::{MemoryLoad, MemoryStore};
//! use wrt_instructions::Value;
//! use wrt_runtime::Memory;
//! use wrt_foundation::types::Limits;
//!
//! // Create a memory instance
//! let mem_type = MemoryType {
//!     limits: Limits { min: 1, max: Some(2) },
//! };
//! let mut memory = Memory::new(mem_type).unwrap();
//!
//! // Store a value
//! let store = MemoryStore::i32(0, 4); // offset 0, 4-byte aligned
//! store.execute(&mut memory, &Value::I32(0), &Value::I32(42)).unwrap();
//!
//! // Load a value
//! let load = MemoryLoad::i32(0, 4); // offset 0, 4-byte aligned
//! let result = load.execute(&memory, &Value::I32(0)).unwrap();
//! assert_eq!(result, Value::I32(42));
//! ```

use crate::prelude::{Debug, Error, PartialEq, PureInstruction, Result, Value, ValueType};
use wrt_foundation::traits::BoundedCapacity;
use crate::validation::{Validate, ValidationContext, validate_memory_op};


/// Memory trait defining the requirements for memory operations
pub trait MemoryOperations {
    /// Read bytes from memory
    #[cfg(feature = "std")]
    fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>>;
    
    #[cfg(not(any(feature = "std", )))]
    fn read_bytes(&self, offset: u32, len: u32) -> Result<wrt_foundation::BoundedVec<u8, 65_536, wrt_foundation::NoStdProvider<65_536>>>;

    /// Write bytes to memory
    fn write_bytes(&mut self, offset: u32, bytes: &[u8]) -> Result<()>;

    /// Get the size of memory in bytes
    fn size_in_bytes(&self) -> Result<usize>;

    /// Grow memory by the specified number of bytes
    fn grow(&mut self, bytes: usize) -> Result<()>;

    /// Fill memory region with a byte value (bulk memory operation)
    fn fill(&mut self, offset: u32, value: u8, size: u32) -> Result<()>;

    /// Copy memory region within the same memory (bulk memory operation)
    fn copy(&mut self, dest: u32, src: u32, size: u32) -> Result<()>;
}

/// Memory load operation
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryLoad {
    /// Memory index (for multi-memory support)
    pub memory_index: u32,
    /// Memory offset
    pub offset: u32,
    /// Required alignment
    pub align: u32,
    /// Value type to load
    pub value_type: ValueType,
    /// Whether this is a signed load (for smaller-than-register loads)
    pub signed: bool,
    /// Memory access width in bytes (8, 16, 32, 64)
    pub width: u32,
}

/// Memory store operation
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryStore {
    /// Memory index (for multi-memory support)
    pub memory_index: u32,
    /// Memory offset
    pub offset: u32,
    /// Required alignment
    pub align: u32,
    /// Value type to store
    pub value_type: ValueType,
    /// Memory access width in bytes (8, 16, 32, 64)
    pub width: u32,
}

impl MemoryLoad {
    /// Creates a new i32 load operation
    ///
    /// # Arguments
    ///
    /// * `memory_index` - Memory index (for multi-memory support)
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i32 values
    #[must_use] pub fn i32(memory_index: u32, offset: u32, align: u32) -> Self {
        Self { memory_index, offset, align, value_type: ValueType::I32, signed: false, width: 32 }
    }

    /// Creates a new i32 load operation (legacy - assumes memory 0)
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i32 values from memory 0
    #[must_use] pub fn i32_legacy(offset: u32, align: u32) -> Self {
        Self::i32(0, offset, align)
    }

    /// Creates a new i64 load operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i64 values
    #[must_use] pub fn i64(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, signed: false, width: 64 }
    }

    /// Creates a new f32 load operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for f32 values
    #[must_use] pub fn f32(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::F32, signed: false, width: 32 }
    }

    /// Creates a new f64 load operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for f64 values
    #[must_use] pub fn f64(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::F64, signed: false, width: 64 }
    }

    /// Creates a new i32 load8 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    /// * `signed` - Whether this is a signed load
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i32 values loading from 8-bit memory
    #[must_use] pub fn i32_load8(offset: u32, align: u32, signed: bool) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I32, signed, width: 8 }
    }

    /// Creates a new i32 load16 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    /// * `signed` - Whether this is a signed load
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i32 values loading from 16-bit memory
    #[must_use] pub fn i32_load16(offset: u32, align: u32, signed: bool) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I32, signed, width: 16 }
    }

    /// Creates a new i64 load8 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    /// * `signed` - Whether this is a signed load
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i64 values loading from 8-bit memory
    #[must_use] pub fn i64_load8(offset: u32, align: u32, signed: bool) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, signed, width: 8 }
    }

    /// Creates a new i64 load16 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    /// * `signed` - Whether this is a signed load
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i64 values loading from 16-bit memory
    #[must_use] pub fn i64_load16(offset: u32, align: u32, signed: bool) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, signed, width: 16 }
    }

    /// Creates a new i64 load32 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    /// * `signed` - Whether this is a signed load
    ///
    /// # Returns
    ///
    /// A new `MemoryLoad` for i64 values loading from 32-bit memory
    #[must_use] pub fn i64_load32(offset: u32, align: u32, signed: bool) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, signed, width: 32 }
    }

    /// Execute the memory load operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to operate on
    /// * `addr_arg` - The base address to load from
    ///
    /// # Returns
    ///
    /// The loaded value or an error
    ///
    /// Returns an error if the memory access is invalid
    pub fn execute(&self, memory: &(impl MemoryOperations + ?Sized), addr_arg: &Value) -> Result<Value> {
        // Extract address from argument
        let addr = match addr_arg {
            Value::I32(a) => *a as u32,
            _ => {
                return Err(Error::type_error(
                    "Memory load expects I32 address, got unexpected value"
                ));
            }
        };

        // Calculate effective address
        let effective_addr = addr.checked_add(self.offset).ok_or_else(|| {
            Error::memory_error(
                "Address overflow in memory load"
            )
        })?;

        // Verify alignment if required - make configurable later
        if self.align > 1 && effective_addr % self.align != 0 {
            return Err(Error::memory_error(
                "Unaligned memory access"
            ));
        }

        // Perform the load based on the type and width
        match (self.value_type, self.width) {
            (ValueType::I32, 32) => {
                let bytes = memory.read_bytes(effective_addr, 4)?;
                if bytes.len() < 4 {
                    return Err(Error::memory_error("Insufficient bytes read for i32 value"));
                }
                #[cfg(feature = "std")]
                let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                #[cfg(not(any(feature = "std", )))]
                let value = {
                    let mut arr = [0u8; 4];
                    for i in 0..4 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i32::from_le_bytes(arr)
                };
                Ok(Value::I32(value))
            }
            (ValueType::I64, 64) => {
                let bytes = memory.read_bytes(effective_addr, 8)?;
                if bytes.len() < 8 {
                    return Err(Error::memory_error("Insufficient bytes read for i64 value"));
                }
                #[cfg(feature = "std")]
                let value = i64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                #[cfg(not(any(feature = "std", )))]
                let value = {
                    let mut arr = [0u8; 8];
                    for i in 0..8 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i64::from_le_bytes(arr)
                };
                Ok(Value::I64(value))
            }
            (ValueType::F32, 32) => {
                let bytes = memory.read_bytes(effective_addr, 4)?;
                if bytes.len() < 4 {
                    return Err(Error::memory_error("Insufficient bytes read for f32 value"));
                }
                #[cfg(feature = "std")]
                let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                #[cfg(not(any(feature = "std", )))]
                let value = {
                    let mut arr = [0u8; 4];
                    for i in 0..4 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    f32::from_le_bytes(arr)
                };
                Ok(Value::F32(wrt_foundation::FloatBits32::from_float(value)))
            }
            (ValueType::F64, 64) => {
                let bytes = memory.read_bytes(effective_addr, 8)?;
                if bytes.len() < 8 {
                    return Err(Error::memory_error("Insufficient bytes read for f64 value"));
                }
                #[cfg(feature = "std")]
                let value = f64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                #[cfg(not(any(feature = "std", )))]
                let value = {
                    let mut arr = [0u8; 8];
                    for i in 0..8 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    f64::from_le_bytes(arr)
                };
                Ok(Value::F64(wrt_foundation::FloatBits64::from_float(value)))
            }
            (ValueType::I32, 8) => {
                let bytes = memory.read_bytes(effective_addr, 1)?;
                if bytes.is_empty() {
                    return Err(Error::memory_error("Insufficient bytes read for i8 value"));
                }
                #[cfg(feature = "std")]
                let byte = bytes.get(0).copied().ok_or_else(|| Error::memory_error("Index out of bounds"))?;
                #[cfg(not(any(feature = "std", )))]
                let byte = bytes.get(0).map_err(|_| Error::memory_error("Index out of bounds"))?;
                let value = if self.signed { i32::from(byte as i8) } else { i32::from(byte) };
                Ok(Value::I32(value))
            }
            (ValueType::I64, 8) => {
                let bytes = memory.read_bytes(effective_addr, 1)?;
                if bytes.is_empty() {
                    return Err(Error::memory_error("Insufficient bytes read for i8 value"));
                }
                #[cfg(feature = "std")]
                let byte = bytes.get(0).copied().ok_or_else(|| Error::memory_error("Index out of bounds"))?;
                #[cfg(not(any(feature = "std", )))]
                let byte = bytes.get(0).map_err(|_| Error::memory_error("Index out of bounds"))?;
                let value = if self.signed { i64::from(byte as i8) } else { i64::from(byte) };
                Ok(Value::I64(value))
            }
            (ValueType::I32, 16) => {
                let bytes = memory.read_bytes(effective_addr, 2)?;
                if bytes.len() < 2 {
                    return Err(Error::memory_error("Insufficient bytes read for i16 value"));
                }
                #[cfg(feature = "std")]
                let value = if self.signed {
                    (i16::from_le_bytes([bytes[0], bytes[1]])) as i32
                } else {
                    (u16::from_le_bytes([bytes[0], bytes[1]])) as i32
                };
                #[cfg(not(any(feature = "std", )))]
                let value = if self.signed {
                    let mut arr = [0u8; 2];
                    for i in 0..2 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i32::from(i16::from_le_bytes(arr))
                } else {
                    let mut arr = [0u8; 2];
                    for i in 0..2 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i32::from(u16::from_le_bytes(arr))
                };
                Ok(Value::I32(value))
            }
            (ValueType::I64, 16) => {
                let bytes = memory.read_bytes(effective_addr, 2)?;
                if bytes.len() < 2 {
                    return Err(Error::memory_error("Insufficient bytes read for i16 value"));
                }
                #[cfg(feature = "std")]
                let value = if self.signed {
                    (i16::from_le_bytes([bytes[0], bytes[1]])) as i64
                } else {
                    (u16::from_le_bytes([bytes[0], bytes[1]])) as i64
                };
                #[cfg(not(any(feature = "std", )))]
                let value = if self.signed {
                    let mut arr = [0u8; 2];
                    for i in 0..2 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i64::from(i16::from_le_bytes(arr))
                } else {
                    let mut arr = [0u8; 2];
                    for i in 0..2 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i64::from(u16::from_le_bytes(arr))
                };
                Ok(Value::I64(value))
            }
            (ValueType::I64, 32) => {
                let bytes = memory.read_bytes(effective_addr, 4)?;
                if bytes.len() < 4 {
                    return Err(Error::memory_error("Insufficient bytes read for i32 value"));
                }
                #[cfg(feature = "std")]
                let value = if self.signed {
                    (i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) as i64
                } else {
                    (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) as i64
                };
                #[cfg(not(any(feature = "std", )))]
                let value = if self.signed {
                    let mut arr = [0u8; 4];
                    for i in 0..4 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i64::from(i32::from_le_bytes(arr))
                } else {
                    let mut arr = [0u8; 4];
                    for i in 0..4 {
                        arr[i] = bytes.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                    }
                    i64::from(u32::from_le_bytes(arr))
                };
                Ok(Value::I64(value))
            }
            _ => Err(Error::type_error(
                "Unsupported memory load operation"
            )),
        }
    }
}

impl MemoryStore {
    /// Creates a new i32 store operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for i32 values
    #[must_use] pub fn i32(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I32, width: 32 }
    }

    /// Creates a new i64 store operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for i64 values
    #[must_use] pub fn i64(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, width: 64 }
    }

    /// Creates a new f32 store operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for f32 values
    #[must_use] pub fn f32(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::F32, width: 32 }
    }

    /// Creates a new f64 store operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for f64 values
    #[must_use] pub fn f64(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::F64, width: 64 }
    }

    /// Creates a new i32 store8 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for storing an i32 value as 8 bits
    #[must_use] pub fn i32_store8(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I32, width: 8 }
    }

    /// Creates a new i32 store16 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for storing an i32 value as 16 bits
    #[must_use] pub fn i32_store16(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I32, width: 16 }
    }

    /// Creates a new i64 store8 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for storing an i64 value as 8 bits
    #[must_use] pub fn i64_store8(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, width: 8 }
    }

    /// Creates a new i64 store16 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for storing an i64 value as 16 bits
    #[must_use] pub fn i64_store16(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, width: 16 }
    }

    /// Creates a new i64 store32 operation
    ///
    /// # Arguments
    ///
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new `MemoryStore` for storing an i64 value as 32 bits
    #[must_use] pub fn i64_store32(offset: u32, align: u32) -> Self {
        Self { memory_index: 0, offset, align, value_type: ValueType::I64, width: 32 }
    }

    /// Execute the memory store operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to operate on
    /// * `addr_arg` - The base address to store to
    /// * `value` - The value to store
    ///
    /// # Returns
    ///
    /// Success or an error
    ///
    /// Returns an error if the memory access is invalid
    pub fn execute(
        &self,
        memory: &mut (impl MemoryOperations + ?Sized),
        addr_arg: &Value,
        value: &Value,
    ) -> Result<()> {
        // Extract address from argument
        let addr = match addr_arg {
            Value::I32(a) => *a as u32,
            _ => {
                return Err(Error::type_error(
                    "Memory store expects I32 address, got unexpected value"
                ));
            }
        };

        // Calculate effective address
        let effective_addr = addr.checked_add(self.offset).ok_or_else(|| {
            Error::memory_error(
                "Address overflow in memory store"
            )
        })?;

        // Verify alignment if required
        if self.align > 1 && effective_addr % self.align != 0 {
            return Err(Error::memory_error(
                "Unaligned memory access"
            ));
        }

        // Perform the store based on the type and width
        match (self.value_type, self.width, value) {
            (ValueType::I32, 32, Value::I32(v)) => {
                let bytes = v.to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::I64, 64, Value::I64(v)) => {
                let bytes = v.to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::F32, 32, Value::F32(v)) => {
                let bytes = v.to_bits().to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::F64, 64, Value::F64(v)) => {
                let bytes = v.to_bits().to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }

            (ValueType::I32, 8, Value::I32(v)) => {
                let bytes = [((*v) & 0xFF) as u8];
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::I64, 8, Value::I64(v)) => {
                let bytes = [((*v) & 0xFF) as u8];
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::I32, 16, Value::I32(v)) => {
                let bytes = (*v as u16).to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::I64, 16, Value::I64(v)) => {
                let bytes = (*v as u16).to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::I64, 32, Value::I64(v)) => {
                let bytes = (*v as u32).to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            _ => Err(Error::type_error(
                "Type mismatch for memory store"
            )),
        }
    }
}

/// Memory fill operation (WebAssembly bulk memory proposal)
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryFill {
    /// Memory index (for multi-memory support)
    pub memory_index: u32,
}

/// Memory copy operation (WebAssembly bulk memory proposal)
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCopy {
    /// Destination memory index
    pub dest_memory_index: u32,
    /// Source memory index
    pub src_memory_index: u32,
}

/// Memory init operation (WebAssembly bulk memory proposal)
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryInit {
    /// Memory index
    pub memory_index: u32,
    /// Data segment index
    pub data_index: u32,
}

/// Data drop operation (WebAssembly bulk memory proposal)
#[derive(Debug, Clone, PartialEq)]
pub struct DataDrop {
    /// Data segment index
    pub data_index: u32,
}

/// Trait for data segment operations (needed for memory.init and data.drop)
pub trait DataSegmentOperations {
    /// Get data segment bytes
    #[cfg(feature = "std")]
    fn get_data_segment(&self, data_index: u32) -> Result<Option<Vec<u8>>>;
    
    #[cfg(not(any(feature = "std", )))]
    fn get_data_segment(&self, data_index: u32) -> Result<Option<wrt_foundation::BoundedVec<u8, 65_536, wrt_foundation::NoStdProvider<65_536>>>>;
    
    /// Drop (mark as unavailable) a data segment
    fn drop_data_segment(&mut self, data_index: u32) -> Result<()>;
}

impl MemoryFill {
    /// Create a new memory fill operation
    #[must_use] pub fn new(memory_index: u32) -> Self {
        Self { memory_index }
    }

    /// Execute memory.fill operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to operate on
    /// * `dest` - Destination address (i32)
    /// * `value` - Fill byte value (i32, only low 8 bits used)
    /// * `size` - Number of bytes to fill (i32)
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(
        &self,
        memory: &mut (impl MemoryOperations + ?Sized),
        dest: &Value,
        value: &Value,
        size: &Value,
    ) -> Result<()> {
        // Extract arguments
        let dest_addr = match dest {
            Value::I32(addr) => *addr as u32,
            _ => return Err(Error::type_error("memory.fill dest must be i32")),
        };

        let fill_byte = match value {
            Value::I32(val) => (*val & 0xFF) as u8,
            _ => return Err(Error::type_error("memory.fill value must be i32")),
        };

        let fill_size = match size {
            Value::I32(sz) => *sz as u32,
            _ => return Err(Error::type_error("memory.fill size must be i32")),
        };

        // Check for overflow
        let end_addr = dest_addr.checked_add(fill_size).ok_or_else(|| {
            Error::memory_error("memory.fill address overflow")
        })?;

        // Check bounds
        let memory_size = memory.size_in_bytes()? as u32;
        if end_addr > memory_size {
            return Err(Error::memory_error("memory.fill out of bounds"));
        }

        // Perform fill operation
        memory.fill(dest_addr, fill_byte, fill_size)
    }
}

impl MemoryCopy {
    /// Create a new memory copy operation
    #[must_use] pub fn new(dest_memory_index: u32, src_memory_index: u32) -> Self {
        Self { dest_memory_index, src_memory_index }
    }

    /// Execute memory.copy operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to operate on (currently assumes same memory for src/dest)
    /// * `dest` - Destination address (i32)
    /// * `src` - Source address (i32)
    /// * `size` - Number of bytes to copy (i32)
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(
        &self,
        memory: &mut (impl MemoryOperations + ?Sized),
        dest: &Value,
        src: &Value,
        size: &Value,
    ) -> Result<()> {
        // Extract arguments
        let dest_addr = match dest {
            Value::I32(addr) => *addr as u32,
            _ => return Err(Error::type_error("memory.copy dest must be i32")),
        };

        let src_addr = match src {
            Value::I32(addr) => *addr as u32,
            _ => return Err(Error::type_error("memory.copy src must be i32")),
        };

        let copy_size = match size {
            Value::I32(sz) => *sz as u32,
            _ => return Err(Error::type_error("memory.copy size must be i32")),
        };

        // Check for overflow
        let dest_end = dest_addr.checked_add(copy_size).ok_or_else(|| {
            Error::memory_error("memory.copy dest address overflow")
        })?;

        let src_end = src_addr.checked_add(copy_size).ok_or_else(|| {
            Error::memory_error("memory.copy src address overflow")
        })?;

        // Check bounds
        let memory_size = memory.size_in_bytes()? as u32;
        if dest_end > memory_size || src_end > memory_size {
            return Err(Error::memory_error("memory.copy out of bounds"));
        }

        // Perform copy operation (handles overlapping regions correctly)
        memory.copy(dest_addr, src_addr, copy_size)
    }
}

impl MemoryInit {
    /// Create a new memory init operation
    #[must_use] pub fn new(memory_index: u32, data_index: u32) -> Self {
        Self { memory_index, data_index }
    }

    /// Execute memory.init operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to operate on
    /// * `data_segments` - Access to data segments
    /// * `dest` - Destination address in memory (i32)
    /// * `src` - Source offset in data segment (i32)
    /// * `size` - Number of bytes to copy (i32)
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(
        &self,
        memory: &mut (impl MemoryOperations + ?Sized),
        data_segments: &(impl DataSegmentOperations + ?Sized),
        dest: &Value,
        src: &Value,
        size: &Value,
    ) -> Result<()> {
        // Extract arguments
        let dest_addr = match dest {
            Value::I32(addr) => *addr as u32,
            _ => return Err(Error::type_error("memory.init dest must be i32")),
        };

        let src_offset = match src {
            Value::I32(offset) => *offset as u32,
            _ => return Err(Error::type_error("memory.init src must be i32")),
        };

        let copy_size = match size {
            Value::I32(sz) => *sz as u32,
            _ => return Err(Error::type_error("memory.init size must be i32")),
        };

        // Get data segment
        let data = data_segments.get_data_segment(self.data_index)?
            .ok_or_else(|| Error::memory_error("Data segment has been dropped"))?;

        // Check bounds in data segment
        let data_len = data.len() as u32;
        let src_end = src_offset.checked_add(copy_size).ok_or_else(|| {
            Error::memory_error("memory.init src offset overflow")
        })?;

        if src_end > data_len {
            return Err(Error::memory_error("memory.init src out of bounds"));
        }

        // Check bounds in memory
        let dest_end = dest_addr.checked_add(copy_size).ok_or_else(|| {
            Error::memory_error("memory.init dest address overflow")
        })?;

        let memory_size = memory.size_in_bytes()? as u32;
        if dest_end > memory_size {
            return Err(Error::memory_error("memory.init dest out of bounds"));
        }

        // Copy data from segment to memory
        #[cfg(feature = "std")]
        {
            let src_slice = &data[src_offset as usize..src_end as usize];
            memory.write_bytes(dest_addr, src_slice)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            // Binary std/no_std choice
            for (i, offset) in (src_offset..src_end).enumerate() {
                let byte = data.get(offset as usize).map_err(|_| Error::memory_error("Data segment index out of bounds"))?;
                memory.write_bytes(dest_addr + i as u32, &[byte])?;
            }
            Ok(())
        }
    }
}

impl DataDrop {
    /// Create a new data drop operation
    #[must_use] pub fn new(data_index: u32) -> Self {
        Self { data_index }
    }

    /// Execute data.drop operation
    ///
    /// # Arguments
    ///
    /// * `data_segments` - Access to data segments
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(
        &self,
        data_segments: &mut (impl DataSegmentOperations + ?Sized),
    ) -> Result<()> {
        data_segments.drop_data_segment(self.data_index)
    }
}

/// Unified memory operation enum combining all memory instructions
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryOp {
    /// Load operation
    Load(MemoryLoad),
    /// Store operation
    Store(MemoryStore),
    /// Size operation (memory.size)
    Size(MemorySize),
    /// Grow operation (memory.grow)
    Grow(MemoryGrow),
    /// Fill operation (memory.fill)
    Fill(MemoryFill),
    /// Copy operation (memory.copy)
    Copy(MemoryCopy),
    /// Init operation (memory.init)
    Init(MemoryInit),
    /// Data drop operation (data.drop)
    DataDrop(DataDrop),
}

/// Memory size operation (memory.size)
#[derive(Debug, Clone, PartialEq)]
pub struct MemorySize {
    /// Memory index (0 for MVP, but allows for multi-memory proposal)
    pub memory_index: u32,
}

impl MemorySize {
    /// Create a new memory size operation
    #[must_use] pub fn new(memory_index: u32) -> Self {
        Self { memory_index }
    }

    /// Execute memory.size operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to query
    ///
    /// # Returns
    ///
    /// The size of memory in pages (64KiB pages) as an i32 Value
    pub fn execute(&self, memory: &(impl MemoryOperations + ?Sized)) -> Result<Value> {
        let size_in_bytes = memory.size_in_bytes()?;
        let size_in_pages = (size_in_bytes / 65_536) as i32;
        Ok(Value::I32(size_in_pages))
    }
}

/// Memory grow operation (memory.grow)
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryGrow {
    /// Memory index (0 for MVP, but allows for multi-memory proposal)
    pub memory_index: u32,
}

impl MemoryGrow {
    /// Create a new memory grow operation
    #[must_use] pub fn new(memory_index: u32) -> Self {
        Self { memory_index }
    }

    /// Execute memory.grow operation
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to grow
    /// * `delta` - Number of pages to grow by (i32 value)
    ///
    /// # Returns
    ///
    /// The previous size in pages, or -1 if the operation failed (as i32 Value)
    pub fn execute(&self, memory: &mut (impl MemoryOperations + ?Sized), delta: &Value) -> Result<Value> {
        // Extract delta pages
        let delta_pages = match delta {
            Value::I32(pages) => *pages,
            _ => return Err(Error::type_error("memory.grow delta must be i32")),
        };

        // Negative delta is not allowed
        if delta_pages < 0 {
            return Ok(Value::I32(-1));
        }

        // Get current size in pages
        let current_size_bytes = memory.size_in_bytes()?;
        let current_size_pages = (current_size_bytes / 65_536) as i32;

        // Try to grow the memory
        let delta_bytes = (delta_pages as usize) * 65_536;
        
        // Check if growth would exceed limits
        let _new_size_bytes = current_size_bytes.saturating_add(delta_bytes);
        
        // Attempt to grow - this will fail if it exceeds max size
        match memory.grow(delta_bytes) {
            Ok(()) => Ok(Value::I32(current_size_pages)),
            Err(_) => Ok(Value::I32(-1)), // Growth failed, return -1
        }
    }
}

/// Execution context for unified memory operations
pub trait MemoryContext {
    /// Pop a value from the stack
    fn pop_value(&mut self) -> Result<Value>;
    
    /// Push a value to the stack
    fn push_value(&mut self, value: Value) -> Result<()>;
    
    /// Get memory instance by index
    fn get_memory(&mut self, index: u32) -> Result<&mut dyn MemoryOperations>;
    
    /// Get data segment operations
    fn get_data_segments(&mut self) -> Result<&mut dyn DataSegmentOperations>;
    
    /// Execute memory.init operation (helper to avoid borrowing issues)
    fn execute_memory_init(
        &mut self,
        memory_index: u32,
        data_index: u32,
        dest: i32,
        src: i32,
        size: i32,
    ) -> Result<()>;
}

impl MemoryOp {
    /// Helper to extract 3 i32 arguments from stack
    fn pop_three_i32s(ctx: &mut impl MemoryContext) -> Result<(i32, i32, i32)> {
        let arg3 = ctx.pop_value()?.into_i32().map_err(|_| {
            Error::type_error("Expected i32 for memory operation")
        })?;
        let arg2 = ctx.pop_value()?.into_i32().map_err(|_| {
            Error::type_error("Expected i32 for memory operation")
        })?;
        let arg1 = ctx.pop_value()?.into_i32().map_err(|_| {
            Error::type_error("Expected i32 for memory operation")
        })?;
        Ok((arg1, arg2, arg3))
    }
}

impl<T: MemoryContext> PureInstruction<T, Error> for MemoryOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            Self::Load(load) => {
                let addr = context.pop_value()?;
                let memory = context.get_memory(load.memory_index)?;
                let result = load.execute(memory, &addr)?;
                context.push_value(result)
            }
            Self::Store(store) => {
                let value = context.pop_value()?;
                let addr = context.pop_value()?;
                let memory = context.get_memory(store.memory_index)?;
                store.execute(memory, &addr, &value)
            }
            Self::Size(size) => {
                let memory = context.get_memory(size.memory_index)?;
                let result = size.execute(memory)?;
                context.push_value(result)
            }
            Self::Grow(grow) => {
                let delta = context.pop_value()?;
                let memory = context.get_memory(grow.memory_index)?;
                let result = grow.execute(memory, &delta)?;
                context.push_value(result)
            }
            Self::Fill(fill) => {
                let (dest, value, size) = Self::pop_three_i32s(context)?;
                let memory = context.get_memory(fill.memory_index)?;
                fill.execute(
                    memory,
                    &Value::I32(dest),
                    &Value::I32(value),
                    &Value::I32(size),
                )
            }
            Self::Copy(copy) => {
                let (dest, src, size) = Self::pop_three_i32s(context)?;
                let memory = context.get_memory(copy.dest_memory_index)?;
                // Note: For multi-memory, would need to handle src_memory_index
                copy.execute(
                    memory,
                    &Value::I32(dest),
                    &Value::I32(src),
                    &Value::I32(size),
                )
            }
            Self::Init(init) => {
                let (dest, src, size) = Self::pop_three_i32s(context)?;
                // Work around borrowing by calling a helper method on context
                context.execute_memory_init(
                    init.memory_index,
                    init.data_index,
                    dest,
                    src,
                    size,
                )
            }
            Self::DataDrop(drop) => {
                let data_segments = context.get_data_segments()?;
                drop.execute(data_segments)
            }
        }
    }
}

impl Validate for MemoryOp {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        match self {
            Self::Load(load) => load.validate(ctx),
            Self::Store(store) => store.validate(ctx),
            Self::Size(size) => size.validate(ctx),
            Self::Grow(grow) => grow.validate(ctx),
            Self::Fill(fill) => fill.validate(ctx),
            Self::Copy(copy) => copy.validate(ctx),
            Self::Init(init) => init.validate(ctx),
            Self::DataDrop(drop) => drop.validate(ctx),
        }
    }
}

#[cfg(all(test, any(feature = "std", )))]
mod tests {
    // Import Vec and vec! based on feature flags
        use std::{vec, vec::Vec};
    #[cfg(feature = "std")]
    use std::vec::Vec;

    use super::*;

    /// Mock memory implementation for testing
    #[cfg(feature = "std")]
    struct MockMemory {
        data: Vec<u8>,
    }

    #[cfg(not(feature = "std"))]
    struct MockMemory {
        data: wrt_foundation::BoundedVec<u8, 65536, wrt_foundation::NoStdProvider<65536>>,
    }

    impl MockMemory {
        #[cfg(feature = "std")]
        fn new(size: usize) -> Self {
            let mut data = Vec::with_capacity(size);
            for _ in 0..size { data.push(0); }
            Self { data }
        }

        #[cfg(not(feature = "std"))]
        fn new(size: usize) -> Self {
            let provider = wrt_foundation::NoStdProvider::<65536>::default();
            let mut data = wrt_foundation::BoundedVec::new(provider).unwrap();
            for _ in 0..core::cmp::min(size, 65536) { 
                data.push(0).unwrap(); 
            }
            Self { data }
        }
    }

    impl MemoryOperations for MockMemory {
        #[cfg(feature = "std")]
        fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>> {
            let start = offset as usize;
            let end = start + len as usize;

            if end > self.data.len() {
                return Err(Error::memory_error("Memory access out of bounds"));
            }

            Ok(self.data[start..end].to_vec())
        }

        #[cfg(not(any(feature = "std", )))]
        fn read_bytes(&self, offset: u32, len: u32) -> Result<wrt_foundation::BoundedVec<u8, 65_536, wrt_foundation::NoStdProvider<65_536>>> {
            let start = offset as usize;
            let end = start + len as usize;

            if end > self.data.len() {
                return Err(Error::memory_error("Memory access out of bounds"));
            }

            let provider = wrt_foundation::NoStdProvider::<65536>::default();
            let mut result = wrt_foundation::BoundedVec::new(provider)?;
            for i in start..end {
                let byte = self.data.get(i).map_err(|_| Error::memory_error("Index out of bounds"))?;
                result.push(byte).map_err(|_| Error::memory_error("BoundedVec capacity exceeded"))?;
            }
            Ok(result)
        }

        #[cfg(feature = "std")]
        fn write_bytes(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
            let start = offset as usize;
            let end = start + bytes.len();

            if end > self.data.len() {
                return Err(Error::memory_error("Memory access out of bounds"));
            }

            self.data[start..end].copy_from_slice(bytes);
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        fn write_bytes(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
            let start = offset as usize;
            let end = start + bytes.len();

            if end > self.data.len() {
                return Err(Error::memory_error("Memory access out of bounds"));
            }

            for (i, &byte) in bytes.iter().enumerate() {
                self.data.set(start + i, byte).map_err(|_| Error::memory_error("Index out of bounds"))?;
            }
            Ok(())
        }

        fn size_in_bytes(&self) -> Result<usize> {
            #[cfg(feature = "std")]
            {
                Ok(self.data.len())
            }
            #[cfg(not(feature = "std"))]
            {
                Ok(self.data.len())
            }
        }

        #[cfg(feature = "std")]
        fn grow(&mut self, bytes: usize) -> Result<()> {
            let new_size = self.data.len() + bytes;
            self.data.resize(new_size, 0);
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        fn grow(&mut self, bytes: usize) -> Result<()> {
            let current_len = self.data.len();
            let new_size = current_len + bytes;
            
            // Check if we can fit the new size in our bounded capacity
            if new_size > 65536 {
                return Err(Error::memory_error("Cannot grow beyond bounded capacity"));
            }
            
            for _ in current_len..new_size {
                self.data.push(0).map_err(|_| Error::memory_error("BoundedVec capacity exceeded"))?;
            }
            Ok(())
        }

        #[cfg(feature = "std")]
        fn fill(&mut self, offset: u32, value: u8, size: u32) -> Result<()> {
            let start = offset as usize;
            let end = start + size as usize;

            if end > self.data.len() {
                return Err(Error::memory_error("Memory fill out of bounds"));
            }

            for i in start..end {
                self.data[i] = value;
            }
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        fn fill(&mut self, offset: u32, value: u8, size: u32) -> Result<()> {
            let start = offset as usize;
            let end = start + size as usize;

            if end > self.data.len() {
                return Err(Error::memory_error("Memory fill out of bounds"));
            }

            for i in start..end {
                self.data.set(i, value).map_err(|_| Error::memory_error("Index out of bounds"))?;
            }
            Ok(())
        }

        #[cfg(feature = "std")]
        fn copy(&mut self, dest: u32, src: u32, size: u32) -> Result<()> {
            let dest_start = dest as usize;
            let dest_end = dest_start + size as usize;
            let src_start = src as usize;
            let src_end = src_start + size as usize;

            if dest_end > self.data.len() || src_end > self.data.len() {
                return Err(Error::memory_error("Memory copy out of bounds"));
            }

            // Handle overlapping regions correctly by copying to a temporary buffer
            if size > 0 {
                let temp: Vec<u8> = self.data[src_start..src_end].to_vec();
                self.data[dest_start..dest_end].copy_from_slice(&temp);
            }
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        fn copy(&mut self, dest: u32, src: u32, size: u32) -> Result<()> {
            let dest_start = dest as usize;
            let dest_end = dest_start + size as usize;
            let src_start = src as usize;
            let src_end = src_start + size as usize;

            if dest_end > self.data.len() || src_end > self.data.len() {
                return Err(Error::memory_error("Memory copy out of bounds"));
            }

            // Handle overlapping regions correctly by copying byte by byte
            if size > 0 {
                // Create a temporary buffer on the stack
                let mut temp_buffer = [0u8; 256]; // Limited buffer for no_std
                let copy_size = core::cmp::min(size as usize, 256);
                
                for chunk_start in (0..size as usize).step_by(256) {
                    let chunk_end = core::cmp::min(chunk_start + 256, size as usize);
                    let chunk_size = chunk_end - chunk_start;
                    
                    // Copy source to temp buffer
                    for i in 0..chunk_size {
                        temp_buffer[i] = self.data.get(src_start + chunk_start + i)
                            .map_err(|_| Error::memory_error("Source index out of bounds"))?;
                    }
                    
                    // Copy temp buffer to destination
                    for i in 0..chunk_size {
                        self.data.set(dest_start + chunk_start + i, temp_buffer[i])
                            .map_err(|_| Error::memory_error("Destination index out of bounds"))?;
                    }
                }
            }
            Ok(())
        }
    }

    #[test]
    fn test_memory_load() {
        let mut memory = MockMemory::new(65_536);

        // Store some test values
        memory.write_bytes(0, &[42, 0, 0, 0]).unwrap(); // i32 = 42
        memory.write_bytes(4, &[0, 1, 0, 0, 0, 0, 0, 0]).unwrap(); // i64 = 256
        memory.write_bytes(12, &[0, 0, 0x40, 0x40]).unwrap(); // f32 = 3.0
        memory.write_bytes(16, &[0, 0, 0, 0, 0, 0, 0x08, 0x40]).unwrap(); // f64 = 3.0
        memory.write_bytes(24, &[0xFF]).unwrap(); // i8 = -1 (signed)
        memory.write_bytes(25, &[0xFF, 0xFF]).unwrap(); // i16 = -1 (signed)

        // Test i32.load
        let load = MemoryLoad::i32(0, 4);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(42));

        // Test i64.load
        let load = MemoryLoad::i64(4, 8);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I64(256));

        // Test f32.load
        let load = MemoryLoad::f32(12, 4);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::F32(3.0));

        // Test f64.load
        let load = MemoryLoad::f64(16, 8);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::F64(3.0));

        // Test i32.load8_s
        let load = MemoryLoad::i32_load8(24, 1, true);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(-1));

        // Test i32.load8_u
        let load = MemoryLoad::i32_load8(24, 1, false);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(255));

        // Test i32.load16_s
        let load = MemoryLoad::i32_load16(25, 2, true);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(-1));

        // Test i32.load16_u
        let load = MemoryLoad::i32_load16(25, 2, false);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(65_535));

        // Test effective address calculation with offset
        let load = MemoryLoad::i32(4, 4);
        let result = load.execute(&memory, &Value::I32(4)).unwrap();
        assert_eq!(result, Value::I32(256));
    }

    #[test]
    fn test_memory_store() {
        let mut memory = MockMemory::new(65_536);

        // Test i32.store
        let store = MemoryStore::i32(0, 4);
        store.execute(&mut memory, &Value::I32(0), &Value::I32(42)).unwrap();

        let load = MemoryLoad::i32(0, 4);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(42));

        // Test i64.store
        let store = MemoryStore::i64(8, 8);
        store.execute(&mut memory, &Value::I32(0), &Value::I64(0x0102030405060708)).unwrap();

        let load = MemoryLoad::i64(8, 8);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I64(0x0102030405060708));

        // Test f32.store
        let store = MemoryStore::f32(16, 4);
        store.execute(&mut memory, &Value::I32(0), &Value::F32(3.14159)).unwrap();

        let load = MemoryLoad::f32(16, 4);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::F32(3.14159));

        // Test f64.store
        let store = MemoryStore::f64(24, 8);
        store.execute(&mut memory, &Value::I32(0), &Value::F64(2.71828)).unwrap();

        let load = MemoryLoad::f64(24, 8);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::F64(2.71828));

        // Test i32.store8
        let store = MemoryStore::i32_store8(32, 1);
        store.execute(&mut memory, &Value::I32(0), &Value::I32(0xFF)).unwrap();

        let load = MemoryLoad::i32_load8(32, 1, false);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(0xFF));

        // Test i32.store16
        let store = MemoryStore::i32_store16(33, 1);
        store.execute(&mut memory, &Value::I32(0), &Value::I32(0xABCD)).unwrap();

        let load = MemoryLoad::i32_load16(33, 1, false);
        let result = load.execute(&memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(0xABCD));

        // Test effective address calculation with offset
        let store = MemoryStore::i32(4, 4);
        store.execute(&mut memory, &Value::I32(4), &Value::I32(0xDEADBEEF)).unwrap();

        let load = MemoryLoad::i32(4, 4);
        let result = load.execute(&memory, &Value::I32(4)).unwrap();
        assert_eq!(result, Value::I32(0xDEADBEEF));
    }

    #[test]
    fn test_memory_access_errors() {
        let mut memory = MockMemory::new(100);

        // Out of bounds access
        let load = MemoryLoad::i32(100, 4);
        let result = load.execute(&memory, &Value::I32(0));
        assert!(result.is_err());

        // Offset + address out of bounds
        let load = MemoryLoad::i32(50, 4);
        let result = load.execute(&memory, &Value::I32(60));
        assert!(result.is_err());

        // Store out of bounds
        let store = MemoryStore::i32(100, 4);
        let result = store.execute(&mut memory, &Value::I32(0), &Value::I32(42));
        assert!(result.is_err());
    }

    /// Mock data segment operations for testing
    struct MockDataSegments {
        #[cfg(feature = "std")]
        segments: Vec<Option<Vec<u8>>>,
        #[cfg(not(any(feature = "std", )))]
        segments: wrt_foundation::BoundedVec<Option<wrt_foundation::BoundedVec<u8, 65_536, wrt_foundation::NoStdProvider<65_536>>>, 16, wrt_foundation::NoStdProvider<1024>>,
    }

    impl MockDataSegments {
        fn new() -> Self {
            #[cfg(feature = "std")]
            {
                let mut segments = Vec::new();
                let mut seg1 = Vec::new();
                for val in [1, 2, 3, 4, 5] { seg1.push(val); }
                let mut seg2 = Vec::new();
                for val in [0xAA, 0xBB, 0xCC, 0xDD] { seg2.push(val); }
                segments.push(Some(seg1));
                segments.push(Some(seg2));
                segments.push(None); // Dropped segment
                Self { segments }
            }
            #[cfg(not(any(feature = "std", )))]
            {
                let mut segments = wrt_foundation::BoundedVec::new();
                
                let mut seg1 = wrt_foundation::BoundedVec::new();
                for &b in &[1, 2, 3, 4, 5] {
                    seg1.push(b).unwrap();
                }
                segments.push(Some(seg1)).unwrap();
                
                let mut seg2 = wrt_foundation::BoundedVec::new();
                for &b in &[0xAA, 0xBB, 0xCC, 0xDD] {
                    seg2.push(b).unwrap();
                }
                segments.push(Some(seg2)).unwrap();
                
                segments.push(None).unwrap(); // Dropped segment
                
                Self { segments }
            }
        }
    }

    impl DataSegmentOperations for MockDataSegments {
        #[cfg(feature = "std")]
        fn get_data_segment(&self, data_index: u32) -> Result<Option<Vec<u8>>> {
            if (data_index as usize) < self.segments.len() {
                Ok(self.segments[data_index as usize].clone())
            } else {
                Err(Error::validation_error("Invalid data segment index"))
            }
        }

        #[cfg(not(any(feature = "std", )))]
        fn get_data_segment(&self, data_index: u32) -> Result<Option<wrt_foundation::BoundedVec<u8, 65_536, wrt_foundation::NoStdProvider<65_536>>>> {
            if (data_index as usize) < self.segments.len() {
                Ok(self.segments.get(data_index as usize).ok_or_else(|| Error::validation_error("Invalid data segment index"))?.clone())
            } else {
                Err(Error::validation_error("Invalid data segment index"))
            }
        }

        fn drop_data_segment(&mut self, data_index: u32) -> Result<()> {
            #[cfg(feature = "std")]
            let segments_len = self.segments.len();
            #[cfg(not(feature = "std"))]
            let segments_len = self.segments.len();

            if (data_index as usize) < segments_len {
                #[cfg(feature = "std")]
                {
                    self.segments[data_index as usize] = None;
                }
                #[cfg(not(any(feature = "std", )))]
                {
                    *self.segments.get_mut(data_index as usize).unwrap() = None;
                }
                Ok(())
            } else {
                Err(Error::validation_error("Invalid data segment index"))
            }
        }
    }

    #[test]
    fn test_memory_fill() {
        let mut memory = MockMemory::new(1024);
        let fill_op = MemoryFill::new(0);

        // Fill 10 bytes with value 0x42 starting at offset 100
        fill_op
            .execute(&mut memory, &Value::I32(100), &Value::I32(0x42), &Value::I32(10))
            .unwrap();

        // Verify the fill worked
        let data = memory.read_bytes(100, 10).unwrap();
        assert_eq!(data.len(), 10);
        #[cfg(feature = "std")]
        assert!(data.iter().all(|&b| b == 0x42));
        #[cfg(not(feature = "std"))]
        for i in 0..10 {
            assert_eq!(data.get(i).unwrap(), 0x42);
        }
    }

    #[test]
    fn test_memory_copy() {
        let mut memory = MockMemory::new(1024);
        
        // Set up source data
        memory.write_bytes(0, &[1, 2, 3, 4, 5]).unwrap();
        
        let copy_op = MemoryCopy::new(0, 0);

        // Copy 5 bytes from offset 0 to offset 100
        copy_op
            .execute(&mut memory, &Value::I32(100), &Value::I32(0), &Value::I32(5))
            .unwrap();

        // Verify the copy worked
        let data = memory.read_bytes(100, 5).unwrap();
        #[cfg(feature = "std")]
        {
            let expected = [1, 2, 3, 4, 5];
            assert_eq!(data, expected);
        }
        #[cfg(not(feature = "std"))]
        {
            assert_eq!(data.len(), 5);
            for i in 0..5 {
                assert_eq!(data.get(i).unwrap(), (i + 1) as u8);
            }
        }
    }

    #[test]
    fn test_memory_copy_overlapping() {
        let mut memory = MockMemory::new(1024);
        
        // Set up source data
        memory.write_bytes(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        
        let copy_op = MemoryCopy::new(0, 0);

        // Copy overlapping: copy 5 bytes from offset 0 to offset 2
        copy_op
            .execute(&mut memory, &Value::I32(2), &Value::I32(0), &Value::I32(5))
            .unwrap();

        // Verify overlapping copy worked correctly
        let data = memory.read_bytes(0, 8).unwrap();
        #[cfg(feature = "std")]
        {
            let expected = [1, 2, 1, 2, 3, 4, 5, 8];
            assert_eq!(data, expected);
        }
        #[cfg(not(feature = "std"))]
        {
            let expected = [1, 2, 1, 2, 3, 4, 5, 8];
            for i in 0..8 {
                assert_eq!(data.get(i).unwrap(), expected[i]);
            }
        }
    }

    #[test]
    fn test_memory_init() {
        let mut memory = MockMemory::new(1024);
        let data_segments = MockDataSegments::new();
        let init_op = MemoryInit::new(0, 0);

        // Copy 3 bytes from data segment 0 (starting at offset 1) to memory at offset 100
        init_op
            .execute(
                &mut memory,
                &data_segments,
                &Value::I32(100),
                &Value::I32(1),
                &Value::I32(3),
            )
            .unwrap();

        // Verify the init worked (should copy bytes [2, 3, 4] from segment [1, 2, 3, 4, 5])
        let data = memory.read_bytes(100, 3).unwrap();
        #[cfg(feature = "std")]
        {
            let expected = [2, 3, 4];
            assert_eq!(data, expected);
        }
        #[cfg(not(feature = "std"))]
        {
            assert_eq!(data.len(), 3);
            for i in 0..3 {
                assert_eq!(data.get(i).unwrap(), (i + 2) as u8);
            }
        }
    }

    #[test]
    fn test_data_drop() {
        let mut data_segments = MockDataSegments::new();
        let drop_op = DataDrop::new(0);

        // Verify segment 0 exists initially
        assert!(data_segments.get_data_segment(0).unwrap().is_some());

        // Drop segment 0
        drop_op.execute(&mut data_segments).unwrap();

        // Verify segment 0 is now dropped
        assert!(data_segments.get_data_segment(0).unwrap().is_none());
    }

    #[test]
    fn test_memory_init_dropped_segment() {
        let mut memory = MockMemory::new(1024);
        let mut data_segments = MockDataSegments::new();
        
        // Drop segment 0 first
        data_segments.drop_data_segment(0).unwrap();
        
        let init_op = MemoryInit::new(0, 0);

        // Try to init from dropped segment - should fail
        let result = init_op.execute(
            &mut memory,
            &data_segments,
            &Value::I32(100),
            &Value::I32(0),
            &Value::I32(3),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_bulk_memory_bounds_checking() {
        let mut memory = MockMemory::new(100);
        
        // Test memory.fill out of bounds
        let fill_op = MemoryFill::new(0);
        let result = fill_op.execute(&mut memory, &Value::I32(95), &Value::I32(0x42), &Value::I32(10));
        assert!(result.is_err());

        // Test memory.copy out of bounds
        let copy_op = MemoryCopy::new(0, 0);
        let result = copy_op.execute(&mut memory, &Value::I32(95), &Value::I32(0), &Value::I32(10));
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_size() {
        // Create memory with 2 pages (128 KiB)
        let memory = MockMemory::new(2 * 65_536);
        let size_op = MemorySize::new(0);
        
        let result = size_op.execute(&memory).unwrap();
        assert_eq!(result, Value::I32(2));
        
        // Test with partial page
        let memory = MockMemory::new(65_536 + 100); // 1 page + 100 bytes
        let result = size_op.execute(&memory).unwrap();
        assert_eq!(result, Value::I32(1)); // Should return 1 (partial pages are truncated)
    }

    #[test]
    fn test_memory_grow() {
        // Create memory with 1 page (64 KiB)
        let mut memory = MockMemory::new(65_536);
        let grow_op = MemoryGrow::new(0);
        
        // Grow by 2 pages
        let result = grow_op.execute(&mut memory, &Value::I32(2)).unwrap();
        assert_eq!(result, Value::I32(1)); // Previous size was 1 page
        
        // Check new size
        assert_eq!(memory.size_in_bytes().unwrap(), 3 * 65_536);
        
        // Test grow with 0 pages (should succeed)
        let result = grow_op.execute(&mut memory, &Value::I32(0)).unwrap();
        assert_eq!(result, Value::I32(3)); // Previous size was 3 pages
        
        // Test grow with negative pages (should fail)
        let result = grow_op.execute(&mut memory, &Value::I32(-1)).unwrap();
        assert_eq!(result, Value::I32(-1)); // Growth failed
    }

    // Tests for unified MemoryOp
    struct MockMemoryContext {
        stack: Vec<Value>,
        memory: MockMemory,
        data_segments: MockDataSegments,
    }

    impl MockMemoryContext {
        fn new(memory_size: usize) -> Self {
            Self {
                stack: Vec::new(),
                memory: MockMemory::new(memory_size),
                data_segments: MockDataSegments::new(),
            }
        }
    }

    impl MemoryContext for MockMemoryContext {
        fn pop_value(&mut self) -> Result<Value> {
            self.stack.pop().ok_or_else(|| {
                Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
            })
        }

        fn push_value(&mut self, value: Value) -> Result<()> {
            self.stack.push(value);
            Ok(())
        }

        fn get_memory(&mut self, _index: u32) -> Result<&mut dyn MemoryOperations> {
            Ok(&mut self.memory)
        }

        fn get_data_segments(&mut self) -> Result<&mut dyn DataSegmentOperations> {
            Ok(&mut self.data_segments)
        }
        
        fn execute_memory_init(
            &mut self,
            memory_index: u32,
            data_index: u32,
            dest: i32,
            src: i32,
            size: i32,
        ) -> Result<()> {
            let init_op = MemoryInit::new(memory_index, data_index);
            init_op.execute(
                &mut self.memory,
                &self.data_segments,
                &Value::I32(dest),
                &Value::I32(src),
                &Value::I32(size),
            )
        }
    }

    #[test]
    fn test_unified_memory_size() {
        let mut ctx = MockMemoryContext::new(2 * 65_536); // 2 pages
        
        // Execute memory.size
        let op = MemoryOp::Size(MemorySize::new(0));
        op.execute(&mut ctx).unwrap();
        
        // Should push 2 (pages) onto stack
        assert_eq!(ctx.pop_value().unwrap(), Value::I32(2));
    }

    #[test]
    fn test_unified_memory_grow() {
        let mut ctx = MockMemoryContext::new(65_536); // 1 page
        
        // Push delta (2 pages)
        ctx.push_value(Value::I32(2)).unwrap();
        
        // Execute memory.grow
        let op = MemoryOp::Grow(MemoryGrow::new(0));
        op.execute(&mut ctx).unwrap();
        
        // Should push previous size (1 page) onto stack
        assert_eq!(ctx.pop_value().unwrap(), Value::I32(1));
        
        // Verify memory actually grew
        assert_eq!(ctx.memory.size_in_bytes().unwrap(), 3 * 65_536);
    }

    #[test]
    fn test_unified_memory_fill() {
        let mut ctx = MockMemoryContext::new(1024);
        
        // Push arguments: dest=100, value=0x42, size=10
        ctx.push_value(Value::I32(100)).unwrap(); // dest
        ctx.push_value(Value::I32(0x42)).unwrap(); // value
        ctx.push_value(Value::I32(10)).unwrap(); // size
        
        // Execute memory.fill
        let op = MemoryOp::Fill(MemoryFill::new(0));
        op.execute(&mut ctx).unwrap();
        
        // Verify memory was filled
        let data = ctx.memory.read_bytes(100, 10).unwrap();
        #[cfg(feature = "std")]
        assert!(data.iter().all(|&b| b == 0x42));
        #[cfg(not(feature = "std"))]
        for i in 0..10 {
            assert_eq!(*data.get(i).unwrap(), 0x42);
        }
    }

    #[test]
    fn test_unified_memory_copy() {
        let mut ctx = MockMemoryContext::new(1024);
        
        // Initialize source data
        ctx.memory.write_bytes(200, &[1, 2, 3, 4, 5]).unwrap();
        
        // Push arguments: dest=100, src=200, size=5
        ctx.push_value(Value::I32(100)).unwrap(); // dest
        ctx.push_value(Value::I32(200)).unwrap(); // src
        ctx.push_value(Value::I32(5)).unwrap(); // size
        
        // Execute memory.copy
        let op = MemoryOp::Copy(MemoryCopy::new(0, 0));
        op.execute(&mut ctx).unwrap();
        
        // Verify memory was copied
        let data = ctx.memory.read_bytes(100, 5).unwrap();
        #[cfg(feature = "std")]
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
        #[cfg(not(feature = "std"))]
        for i in 0..5 {
            assert_eq!(*data.get(i).unwrap(), (i + 1) as u8);
        }
    }
}

// Validation implementations

impl Validate for MemoryLoad {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        validate_memory_op(
            "memory.load",
            0, // memory index - always 0 for now
            self.align,
            self.value_type,
            true, // is_load
            ctx
        )
    }
}

impl Validate for MemoryStore {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        validate_memory_op(
            "memory.store",
            0, // memory index - always 0 for now
            self.align,
            self.value_type,
            false, // is_load
            ctx
        )
    }
}

impl Validate for MemoryFill {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // memory.fill: [i32, i32, i32] -> []
        // Stack: [dest_addr, value, size]
        if !ctx.is_unreachable() {
            ctx.pop_expect(ValueType::I32)?; // size
            ctx.pop_expect(ValueType::I32)?; // value
            ctx.pop_expect(ValueType::I32)?; // dest_addr
        }
        Ok(())
    }
}

impl Validate for MemoryCopy {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // memory.copy: [i32, i32, i32] -> []
        // Stack: [dest_addr, src_addr, size]
        if !ctx.is_unreachable() {
            ctx.pop_expect(ValueType::I32)?; // size
            ctx.pop_expect(ValueType::I32)?; // src_addr
            ctx.pop_expect(ValueType::I32)?; // dest_addr
        }
        Ok(())
    }
}

impl Validate for MemoryInit {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // memory.init: [i32, i32, i32] -> []
        // Stack: [dest_addr, src_offset, size]
        if !ctx.is_unreachable() {
            ctx.pop_expect(ValueType::I32)?; // size
            ctx.pop_expect(ValueType::I32)?; // src_offset
            ctx.pop_expect(ValueType::I32)?; // dest_addr
        }
        Ok(())
    }
}

impl Validate for DataDrop {
    fn validate(&self, _ctx: &mut ValidationContext) -> Result<()> {
        // data.drop: [] -> []
        // No stack operations required
        Ok(())
    }
}

impl Validate for MemorySize {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // memory.size: [] -> [i32]
        // Pushes current memory size in pages
        if !ctx.is_unreachable() {
            ctx.push_type(ValueType::I32)?;
        }
        Ok(())
    }
}

impl Validate for MemoryGrow {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // memory.grow: [i32] -> [i32]
        // Pops delta pages, pushes previous size (or -1 on failure)
        if !ctx.is_unreachable() {
            ctx.pop_expect(ValueType::I32)?; // delta pages
            ctx.push_type(ValueType::I32)?; // previous size or -1
        }
        Ok(())
    }
}
