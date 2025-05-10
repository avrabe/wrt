//! Memory operations for WebAssembly instructions.
//!
//! This module provides implementations for WebAssembly memory access instructions,
//! including load and store operations for various value types.
//!
//! # Memory Operation Architecture
//!
//! This module separates memory operations from the underlying memory implementation,
//! allowing different execution engines to share the same memory access code. The key
//! components are:
//!
//! - `MemoryLoad`: Handles all WebAssembly load operations with various data types and widths
//! - `MemoryStore`: Handles all WebAssembly store operations with various data types and widths
//!
//! Both structures work with the `Memory` implementation from the `wrt-runtime` crate.
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
//! All memory operations perform proper bounds and alignment checking before accessing
//! memory. This ensures that WebAssembly's memory safety guarantees are preserved.
//!
//! # Usage
//!
//! ```no_run
//! use wrt_instructions::memory_ops::{MemoryLoad, MemoryStore};
//! use wrt_instructions::Value;
//! use wrt_runtime::Memory;
//! use wrt_types::types::Limits;
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

use crate::prelude::*;

/// Memory trait defining the requirements for memory operations
pub trait MemoryOperations {
    /// Read bytes from memory
    fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>>;

    /// Write bytes to memory
    fn write_bytes(&mut self, offset: u32, bytes: &[u8]) -> Result<()>;

    /// Get the size of memory in bytes
    fn size_in_bytes(&self) -> Result<usize>;

    /// Grow memory by the specified number of pages
    fn grow(&mut self, pages: u32) -> Result<u32>;
}

/// Memory load operation
#[derive(Debug, Clone)]
pub struct MemoryLoad {
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
#[derive(Debug, Clone)]
pub struct MemoryStore {
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
    /// * `offset` - Memory offset
    /// * `align` - Required alignment
    ///
    /// # Returns
    ///
    /// A new MemoryLoad for i32 values
    pub fn i32(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I32, signed: false, width: 32 }
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
    /// A new MemoryLoad for i64 values
    pub fn i64(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I64, signed: false, width: 64 }
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
    /// A new MemoryLoad for f32 values
    pub fn f32(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::F32, signed: false, width: 32 }
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
    /// A new MemoryLoad for f64 values
    pub fn f64(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::F64, signed: false, width: 64 }
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
    /// A new MemoryLoad for i32 values loading from 8-bit memory
    pub fn i32_load8(offset: u32, align: u32, signed: bool) -> Self {
        Self { offset, align, value_type: ValueType::I32, signed, width: 8 }
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
    /// A new MemoryLoad for i32 values loading from 16-bit memory
    pub fn i32_load16(offset: u32, align: u32, signed: bool) -> Self {
        Self { offset, align, value_type: ValueType::I32, signed, width: 16 }
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
    /// A new MemoryLoad for i64 values loading from 8-bit memory
    pub fn i64_load8(offset: u32, align: u32, signed: bool) -> Self {
        Self { offset, align, value_type: ValueType::I64, signed, width: 8 }
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
    /// A new MemoryLoad for i64 values loading from 16-bit memory
    pub fn i64_load16(offset: u32, align: u32, signed: bool) -> Self {
        Self { offset, align, value_type: ValueType::I64, signed, width: 16 }
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
    /// A new MemoryLoad for i64 values loading from 32-bit memory
    pub fn i64_load32(offset: u32, align: u32, signed: bool) -> Self {
        Self { offset, align, value_type: ValueType::I64, signed, width: 32 }
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
    pub fn execute(&self, memory: &impl MemoryOperations, addr_arg: &Value) -> Result<Value> {
        // Extract address from argument
        let addr = match addr_arg {
            Value::I32(a) => *a as u32,
            _ => {
                return Err(Error::type_error(format!(
                    "Memory load expects I32 address, got {:?}",
                    addr_arg
                )));
            }
        };

        // Calculate effective address
        let effective_addr = addr.checked_add(self.offset).ok_or_else(|| {
            Error::memory_error(format!(
                "Address overflow in memory.load: addr={}, offset={}",
                addr, self.offset
            ))
        })?;

        // Verify alignment if required - make configurable later
        if self.align > 1 && effective_addr % self.align != 0 {
            return Err(Error::memory_error(format!(
                "Unaligned memory access: addr={}, align={}",
                effective_addr, self.align
            )));
        }

        // Perform the load based on the type and width
        match (self.value_type, self.width) {
            (ValueType::I32, 32) => {
                let bytes = memory.read_bytes(effective_addr, 4)?;
                if bytes.len() < 4 {
                    return Err(Error::memory_error("Insufficient bytes read for i32 value"));
                }
                let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok(Value::I32(value))
            }
            (ValueType::I64, 64) => {
                let bytes = memory.read_bytes(effective_addr, 8)?;
                if bytes.len() < 8 {
                    return Err(Error::memory_error("Insufficient bytes read for i64 value"));
                }
                let value = i64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Ok(Value::I64(value))
            }
            (ValueType::F32, 32) => {
                let bytes = memory.read_bytes(effective_addr, 4)?;
                if bytes.len() < 4 {
                    return Err(Error::memory_error("Insufficient bytes read for f32 value"));
                }
                let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok(Value::F32(value))
            }
            (ValueType::F64, 64) => {
                let bytes = memory.read_bytes(effective_addr, 8)?;
                if bytes.len() < 8 {
                    return Err(Error::memory_error("Insufficient bytes read for f64 value"));
                }
                let value = f64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Ok(Value::F64(value))
            }
            (ValueType::I32, 8) => {
                let bytes = memory.read_bytes(effective_addr, 1)?;
                if bytes.is_empty() {
                    return Err(Error::memory_error("Insufficient bytes read for i8 value"));
                }
                let byte = bytes[0];
                let value = if self.signed { (byte as i8) as i32 } else { byte as i32 };
                Ok(Value::I32(value))
            }
            (ValueType::I64, 8) => {
                let bytes = memory.read_bytes(effective_addr, 1)?;
                if bytes.is_empty() {
                    return Err(Error::memory_error("Insufficient bytes read for i8 value"));
                }
                let byte = bytes[0];
                let value = if self.signed { (byte as i8) as i64 } else { byte as i64 };
                Ok(Value::I64(value))
            }
            (ValueType::I32, 16) => {
                let bytes = memory.read_bytes(effective_addr, 2)?;
                if bytes.len() < 2 {
                    return Err(Error::memory_error("Insufficient bytes read for i16 value"));
                }
                let value = if self.signed {
                    (i16::from_le_bytes([bytes[0], bytes[1]])) as i32
                } else {
                    (u16::from_le_bytes([bytes[0], bytes[1]])) as i32
                };
                Ok(Value::I32(value))
            }
            (ValueType::I64, 16) => {
                let bytes = memory.read_bytes(effective_addr, 2)?;
                if bytes.len() < 2 {
                    return Err(Error::memory_error("Insufficient bytes read for i16 value"));
                }
                let value = if self.signed {
                    (i16::from_le_bytes([bytes[0], bytes[1]])) as i64
                } else {
                    (u16::from_le_bytes([bytes[0], bytes[1]])) as i64
                };
                Ok(Value::I64(value))
            }
            (ValueType::I64, 32) => {
                let bytes = memory.read_bytes(effective_addr, 4)?;
                if bytes.len() < 4 {
                    return Err(Error::memory_error("Insufficient bytes read for i32 value"));
                }
                let value = if self.signed {
                    (i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) as i64
                } else {
                    (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) as i64
                };
                Ok(Value::I64(value))
            }
            _ => Err(Error::type_error(format!(
                "Unsupported memory load: type={:?}, width={}",
                self.value_type, self.width
            ))),
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
    /// A new MemoryStore for i32 values
    pub fn i32(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I32, width: 32 }
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
    /// A new MemoryStore for i64 values
    pub fn i64(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I64, width: 64 }
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
    /// A new MemoryStore for f32 values
    pub fn f32(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::F32, width: 32 }
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
    /// A new MemoryStore for f64 values
    pub fn f64(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::F64, width: 64 }
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
    /// A new MemoryStore for storing an i32 value as 8 bits
    pub fn i32_store8(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I32, width: 8 }
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
    /// A new MemoryStore for storing an i32 value as 16 bits
    pub fn i32_store16(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I32, width: 16 }
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
    /// A new MemoryStore for storing an i64 value as 8 bits
    pub fn i64_store8(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I64, width: 8 }
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
    /// A new MemoryStore for storing an i64 value as 16 bits
    pub fn i64_store16(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I64, width: 16 }
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
    /// A new MemoryStore for storing an i64 value as 32 bits
    pub fn i64_store32(offset: u32, align: u32) -> Self {
        Self { offset, align, value_type: ValueType::I64, width: 32 }
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
        memory: &mut impl MemoryOperations,
        addr_arg: &Value,
        value: &Value,
    ) -> Result<()> {
        // Extract address from argument
        let addr = match addr_arg {
            Value::I32(a) => *a as u32,
            _ => {
                return Err(Error::type_error(format!(
                    "Memory store expects I32 address, got {:?}",
                    addr_arg
                )));
            }
        };

        // Calculate effective address
        let effective_addr = addr.checked_add(self.offset).ok_or_else(|| {
            Error::memory_error(format!(
                "Address overflow in memory.store: addr={}, offset={}",
                addr, self.offset
            ))
        })?;

        // Verify alignment if required
        if self.align > 1 && effective_addr % self.align != 0 {
            return Err(Error::memory_error(format!(
                "Unaligned memory access: addr={}, align={}",
                effective_addr, self.align
            )));
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
                let bytes = v.to_le_bytes();
                memory.write_bytes(effective_addr, &bytes)
            }
            (ValueType::F64, 64, Value::F64(v)) => {
                let bytes = v.to_le_bytes();
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
            _ => Err(Error::type_error(format!(
                "Type mismatch for memory store: expected {:?}, got {:?}",
                self.value_type, value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_runtime::MemoryType;
    use wrt_types::types::Limits;

    /// Mock memory implementation for testing
    struct MockMemory {
        data: Vec<u8>,
    }

    impl MockMemory {
        fn new(size: usize) -> Self {
            Self { data: vec![0; size] }
        }
    }

    impl MemoryOperations for MockMemory {
        fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>> {
            let start = offset as usize;
            let end = start + len as usize;

            if end > self.data.len() {
                return Err(Error::from(kinds::MemoryAccessOutOfBoundsError(format!(
                    "Memory access out of bounds: offset={}, len={}, size={}",
                    offset,
                    len,
                    self.data.len()
                ))));
            }

            Ok(self.data[start..end].to_vec())
        }

        fn write_bytes(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
            let start = offset as usize;
            let end = start + bytes.len();

            if end > self.data.len() {
                return Err(Error::from(kinds::MemoryAccessOutOfBoundsError(format!(
                    "Memory access out of bounds: offset={}, len={}, size={}",
                    offset,
                    bytes.len(),
                    self.data.len()
                ))));
            }

            self.data[start..end].copy_from_slice(bytes);
            Ok(())
        }

        fn size_in_bytes(&self) -> Result<usize> {
            Ok(self.data.len())
        }

        fn grow(&mut self, pages: u32) -> Result<u32> {
            let old_pages = (self.data.len() / 65536) as u32;
            let new_size = self.data.len() + (pages as usize * 65536);
            self.data.resize(new_size, 0);
            Ok(old_pages)
        }
    }

    #[test]
    fn test_memory_load() {
        let mut memory = MockMemory::new(65536);

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
        assert_eq!(result, Value::I32(65535));

        // Test effective address calculation with offset
        let load = MemoryLoad::i32(4, 4);
        let result = load.execute(&memory, &Value::I32(4)).unwrap();
        assert_eq!(result, Value::I32(256));
    }

    #[test]
    fn test_memory_store() {
        let mut memory = MockMemory::new(65536);

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
}
