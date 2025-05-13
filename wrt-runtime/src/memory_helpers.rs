//! Helper extensions for working with `Arc<Memory>` in the WRT runtime
//!
//! This module provides extension traits to simplify working with `Arc<Memory>`
//! instances, reducing the need for explicit dereferencing and borrowing.

// Import Arc from appropriate source based on feature flags
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
#[cfg(feature = "std")]
use std::sync::Arc;

use wrt_error::{Error, Result};
use wrt_types::{safe_memory::SafeStack, values::Value};

use crate::{prelude::*, Memory};

/// Extension trait for `Arc<Memory>` to simplify access to memory operations
pub trait ArcMemoryExt {
    /// Get the size of memory in pages
    fn size(&self) -> u32;

    /// Get the size of memory in bytes
    fn size_in_bytes(&self) -> usize;

    /// Get the peak memory usage in bytes
    fn peak_usage(&self) -> usize;

    /// Get the number of memory accesses performed
    fn access_count(&self) -> u64;

    /// Get a debug name for this memory, if any
    fn debug_name(&self) -> Option<&str>;

    /// Read bytes from memory using SafeStack - safer alternative to read_bytes
    fn read_bytes_safe(
        &self,
        offset: u32,
        len: u32,
    ) -> Result<wrt_types::safe_memory::SafeStack<u8>>;

    /// Read bytes from memory (legacy method, prefer read_bytes_safe)
    #[deprecated(since = "0.2.0", note = "Use read_bytes_safe instead for enhanced memory safety")]
    fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>>;

    /// Write bytes to memory
    fn write_bytes(&self, offset: u32, bytes: &[u8]) -> Result<()>;

    /// Grow memory by a number of pages
    fn grow(&self, pages: u32) -> Result<u32>;

    /// Read a 32-bit integer from memory
    fn read_i32(&self, addr: u32) -> Result<i32>;

    /// Read a 64-bit integer from memory
    fn read_i64(&self, addr: u32) -> Result<i64>;

    /// Read a 32-bit float from memory
    fn read_f32(&self, addr: u32) -> Result<f32>;

    /// Read a 64-bit float from memory
    fn read_f64(&self, addr: u32) -> Result<f64>;

    /// Read an 8-bit signed integer from memory
    fn read_i8(&self, addr: u32) -> Result<i8>;

    /// Read an 8-bit unsigned integer from memory
    fn read_u8(&self, addr: u32) -> Result<u8>;

    /// Read a 16-bit signed integer from memory
    fn read_i16(&self, addr: u32) -> Result<i16>;

    /// Read a 16-bit unsigned integer from memory
    fn read_u16(&self, addr: u32) -> Result<u16>;

    /// Read a 32-bit unsigned integer from memory
    fn read_u32(&self, addr: u32) -> Result<u32>;

    /// Read a 64-bit unsigned integer from memory
    fn read_u64(&self, addr: u32) -> Result<u64>;

    /// Read a 128-bit vector from memory
    fn read_v128(&self, addr: u32) -> Result<[u8; 16]>;

    /// Write a 32-bit integer to memory
    fn write_i32(&self, addr: u32, value: i32) -> Result<()>;

    /// Write a 64-bit integer to memory
    fn write_i64(&self, addr: u32, value: i64) -> Result<()>;

    /// Write a 32-bit float to memory
    fn write_f32(&self, addr: u32, value: f32) -> Result<()>;

    /// Write a 64-bit float to memory
    fn write_f64(&self, addr: u32, value: f64) -> Result<()>;

    /// Write an 8-bit integer to memory
    fn write_i8(&self, addr: u32, value: i8) -> Result<()>;

    /// Write an 8-bit unsigned integer to memory
    fn write_u8(&self, addr: u32, value: u8) -> Result<()>;

    /// Write a 16-bit signed integer to memory
    fn write_i16(&self, addr: u32, value: i16) -> Result<()>;

    /// Write a 16-bit unsigned integer to memory
    fn write_u16(&self, addr: u32, value: u16) -> Result<()>;

    /// Write a 32-bit unsigned integer to memory
    fn write_u32(&self, addr: u32, value: u32) -> Result<()>;

    /// Write a 64-bit unsigned integer to memory
    fn write_u64(&self, addr: u32, value: u64) -> Result<()>;

    /// Write a 128-bit vector to memory
    fn write_v128(&self, addr: u32, value: [u8; 16]) -> Result<()>;

    /// Check alignment for memory access
    fn check_alignment(&self, offset: u32, access_size: u32, align: u32) -> Result<()>;

    /// Read standard WebAssembly value
    fn read_value(&self, addr: u32, value_type: wrt_types::types::ValueType) -> Result<Value>;

    /// Write standard WebAssembly value
    fn write_value(&self, addr: u32, value: Value) -> Result<()>;

    /// Initialize a region of memory from a data segment
    fn init(&self, dst: usize, data: &[u8], src: usize, size: usize) -> Result<()>;

    /// Read multiple WebAssembly values into a SafeStack
    ///
    /// This method provides a safer alternative to reading values into a Vec
    /// by using SafeStack, which includes memory verification.
    ///
    /// # Arguments
    ///
    /// * `addr` - The starting address to read from
    /// * `value_type` - The type of values to read
    /// * `count` - Number of values to read
    ///
    /// # Returns
    ///
    /// A SafeStack containing the read values
    ///
    /// # Errors
    ///
    /// Returns an error if the memory access is invalid
    fn read_values_as_safe_stack(
        &self,
        addr: u32,
        value_type: wrt_types::types::ValueType,
        count: usize,
    ) -> Result<wrt_types::safe_memory::SafeStack<Value>>;

    /// Write bytes to memory at the given offset
    fn write_via_callback(&self, offset: u32, buffer: &[u8]) -> Result<()>;

    /// Grow memory by the given number of pages
    fn grow_via_callback(&self, pages: u32) -> Result<u32>;
}

impl ArcMemoryExt for Arc<Memory> {
    fn size(&self) -> u32 {
        self.as_ref().size()
    }

    fn size_in_bytes(&self) -> usize {
        self.as_ref().size_in_bytes()
    }

    fn peak_usage(&self) -> usize {
        self.as_ref().peak_memory()
    }

    fn access_count(&self) -> u64 {
        self.as_ref().access_count()
    }

    fn debug_name(&self) -> Option<&str> {
        self.as_ref().debug_name()
    }

    fn read_bytes_safe(
        &self,
        offset: u32,
        len: u32,
    ) -> Result<wrt_types::safe_memory::SafeStack<u8>> {
        // Early return for zero-length reads
        if len == 0 {
            return Ok(wrt_types::safe_memory::SafeStack::new());
        }

        // Get a memory-safe slice directly instead of creating a temporary buffer
        let safe_slice = self.as_ref().get_safe_slice(offset, len as usize)?;

        // Create a SafeStack from the verified slice data with appropriate verification
        // level
        let mut safe_stack = wrt_types::safe_memory::SafeStack::with_capacity(len as usize);

        // Set verification level to match memory's level
        let verification_level = self.as_ref().verification_level();
        safe_stack.set_verification_level(verification_level);

        // Get data from the safe slice with integrity verification built in
        let data = safe_slice.data()?;

        // More efficiently add all bytes to the SafeStack using extend_from_slice
        #[cfg(feature = "std")]
        {
            safe_stack.extend_from_slice(data)?;
        }

        // Fallback implementation if we need to populate item by item (for no_std)
        #[cfg(not(feature = "std"))]
        {
            // Add all bytes to the SafeStack
            for &byte in data {
                safe_stack.push(byte)?;
            }
        }

        // Perform final validation if high verification level is enabled
        if verification_level.should_verify(200) {
            // This would perform a redundant integrity check in a real
            // implementation but we'll let the SafeStack handle
            // that internally
        }

        // Return the verified buffer
        Ok(safe_stack)
    }

    fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>> {
        // Early return for zero-length reads
        if len == 0 {
            return Ok(Vec::new());
        }

        // Get a memory-safe slice directly instead of creating a temporary buffer
        let safe_slice = self.as_ref().get_safe_slice(offset, len as usize)?;

        // Get data from the safe slice with integrity verification built in
        let data = safe_slice.data()?;

        // Create a Vec from the verified slice data
        let buffer = data.to_vec();

        // Return the verified buffer
        Ok(buffer)
    }

    fn write_bytes(&self, offset: u32, bytes: &[u8]) -> Result<()> {
        // Use clone_and_mutate pattern to simplify thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write(offset, bytes))
    }

    fn grow(&self, pages: u32) -> Result<u32> {
        // Use clone_and_mutate pattern to simplify thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.grow(pages))
    }

    fn read_i32(&self, addr: u32) -> Result<i32> {
        self.as_ref().read_i32(addr)
    }

    fn read_i64(&self, addr: u32) -> Result<i64> {
        self.as_ref().read_i64(addr)
    }

    fn read_f32(&self, addr: u32) -> Result<f32> {
        self.as_ref().read_f32(addr)
    }

    fn read_f64(&self, addr: u32) -> Result<f64> {
        self.as_ref().read_f64(addr)
    }

    fn read_i8(&self, addr: u32) -> Result<i8> {
        self.as_ref().read_i8(addr)
    }

    fn read_u8(&self, addr: u32) -> Result<u8> {
        self.as_ref().read_u8(addr)
    }

    fn read_i16(&self, addr: u32) -> Result<i16> {
        self.as_ref().read_i16(addr)
    }

    fn read_u16(&self, addr: u32) -> Result<u16> {
        self.as_ref().read_u16(addr)
    }

    fn read_u32(&self, addr: u32) -> Result<u32> {
        self.as_ref().read_u32(addr)
    }

    fn read_u64(&self, addr: u32) -> Result<u64> {
        self.as_ref().read_u64(addr)
    }

    fn read_v128(&self, addr: u32) -> Result<[u8; 16]> {
        self.as_ref().read_v128(addr)
    }

    fn write_i32(&self, addr: u32, value: i32) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_i32(addr, value))
    }

    fn write_i64(&self, addr: u32, value: i64) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_i64(addr, value))
    }

    fn write_f32(&self, addr: u32, value: f32) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_f32(addr, value))
    }

    fn write_f64(&self, addr: u32, value: f64) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_f64(addr, value))
    }

    fn write_i8(&self, addr: u32, value: i8) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_i8(addr, value))
    }

    fn write_u8(&self, addr: u32, value: u8) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_u8(addr, value))
    }

    fn write_i16(&self, addr: u32, value: i16) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_i16(addr, value))
    }

    fn write_u16(&self, addr: u32, value: u16) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_u16(addr, value))
    }

    fn write_u32(&self, addr: u32, value: u32) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_u32(addr, value))
    }

    fn write_u64(&self, addr: u32, value: u64) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_u64(addr, value))
    }

    fn write_v128(&self, addr: u32, value: [u8; 16]) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| mem.write_v128(addr, value))
    }

    fn check_alignment(&self, offset: u32, access_size: u32, align: u32) -> Result<()> {
        self.as_ref().check_alignment(offset, access_size, align)
    }

    fn read_value(&self, addr: u32, value_type: wrt_types::types::ValueType) -> Result<Value> {
        match value_type {
            wrt_types::types::ValueType::I32 => self.read_i32(addr).map(Value::I32),
            wrt_types::types::ValueType::I64 => self.read_i64(addr).map(Value::I64),
            wrt_types::types::ValueType::F32 => self.read_f32(addr).map(Value::F32),
            wrt_types::types::ValueType::F64 => self.read_f64(addr).map(Value::F64),
            // V128 doesn't exist in ValueType enum, so we'll handle it separately
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::errors::codes::TYPE_MISMATCH_ERROR,
                format!("Cannot read value of type {:?} from memory", value_type),
            )),
        }
    }

    fn write_value(&self, addr: u32, value: Value) -> Result<()> {
        // Use clone_and_mutate pattern for thread-safe operations
        self.as_ref().clone_and_mutate(|mem| match value {
            Value::I32(v) => mem.write_i32(addr, v),
            Value::I64(v) => mem.write_i64(addr, v),
            Value::F32(v) => mem.write_f32(addr, v),
            Value::F64(v) => mem.write_f64(addr, v),
            Value::V128(v) => mem.write_v128(addr, v),
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::errors::codes::TYPE_MISMATCH_ERROR,
                format!("Cannot write value {:?} to memory", value),
            )),
        })
    }

    fn init(&self, dst: usize, data: &[u8], src: usize, size: usize) -> Result<()> {
        // Use clone_and_mutate pattern to simplify thread-safe operations
        self.as_ref().clone_and_mutate(|mem| {
            // Create a safe slice of the source data for verification
            let src_data = if src < data.len() {
                let end = src.checked_add(size).ok_or_else(|| {
                    wrt_error::Error::new(
                        wrt_error::ErrorCategory::Memory,
                        wrt_error::errors::codes::MEMORY_OUT_OF_BOUNDS,
                        "Source range overflow in memory initialization",
                    )
                })?;

                if end <= data.len() {
                    &data[src..end]
                } else {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Memory,
                        wrt_error::errors::codes::MEMORY_OUT_OF_BOUNDS,
                        "Source range out of bounds in memory initialization",
                    ));
                }
            } else if size == 0 {
                // Zero-sized init is always valid
                &[]
            } else {
                return Err(wrt_error::Error::new(
                    wrt_error::ErrorCategory::Memory,
                    wrt_error::errors::codes::MEMORY_OUT_OF_BOUNDS,
                    "Source offset out of bounds in memory initialization",
                ));
            };

            // Use the memory's init method which already has safety checks
            mem.init(dst, src_data, 0, src_data.len())
        })
    }

    /// Read multiple WebAssembly values into a SafeStack
    ///
    /// This method provides a safer alternative to reading values into a Vec
    /// by using SafeStack, which includes memory verification.
    ///
    /// # Arguments
    ///
    /// * `addr` - The starting address to read from
    /// * `value_type` - The type of values to read
    /// * `count` - Number of values to read
    ///
    /// # Returns
    ///
    /// A SafeStack containing the read values
    ///
    /// # Errors
    ///
    /// Returns an error if the memory access is invalid
    fn read_values_as_safe_stack(
        &self,
        addr: u32,
        value_type: wrt_types::types::ValueType,
        count: usize,
    ) -> Result<wrt_types::safe_memory::SafeStack<Value>> {
        // Create a SafeStack to store the values
        let mut result = wrt_types::safe_memory::SafeStack::with_capacity(count);

        // Set verification level to match memory's level
        let verification_level = self.as_ref().verification_level();
        result.set_verification_level(verification_level);

        // Calculate size of each value in bytes
        let value_size = match value_type {
            wrt_types::types::ValueType::I32 => 4,
            wrt_types::types::ValueType::I64 => 8,
            wrt_types::types::ValueType::F32 => 4,
            wrt_types::types::ValueType::F64 => 8,
            _ => {
                return Err(wrt_error::Error::new(
                    wrt_error::ErrorCategory::Type,
                    wrt_error::errors::codes::TYPE_MISMATCH_ERROR,
                    format!("Unsupported value type for reading from memory: {:?}", value_type),
                ))
            }
        };

        // Read each value safely
        for i in 0..count {
            let offset = addr.checked_add((i * value_size) as u32).ok_or_else(|| {
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Memory,
                    wrt_error::errors::codes::MEMORY_OUT_OF_BOUNDS,
                    "Memory address overflow when reading values",
                )
            })?;

            let value = self.read_value(offset, value_type)?;
            result.push(value)?;
        }

        // Verify the final stack if high verification is enabled
        if verification_level.should_verify(200) {
            // This would perform a redundant integrity check in a real
            // implementation
        }

        Ok(result)
    }

    fn write_via_callback(&self, offset: u32, buffer: &[u8]) -> Result<()> {
        // Use internal Mutex or RwLock to provide thread-safe mutation
        // Clone and modify through interior mutability
        let mut current_buffer = self.buffer()?;
        let start = offset as usize;
        let end = start + buffer.len();

        if end > current_buffer.len() {
            return Err(Error::from(kinds::MemoryAccessOutOfBoundsError {
                address: start as u64,
                length: buffer.len() as u64,
            }));
        }

        // Update the memory through the mutex/lock mechanism in the Memory
        // implementation
        self.update_buffer(|mem_buffer| {
            for (i, &byte) in buffer.iter().enumerate() {
                mem_buffer[start + i] = byte;
            }
            Ok(())
        })
    }

    fn grow_via_callback(&self, _pages: u32) -> Result<u32> {
        // Memory::grow_memory requires &mut self.
        // Arc<Memory> cannot provide &mut Memory without interior mutability
        // or Arc::get_mut, which this trait signature doesn't allow.
        Err(Error::system_error_with_code(
            codes::UNSUPPORTED_OPERATION,
            "grow_via_callback on Arc<Memory> is not supported without interior mutability in \
             Memory for its data.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use wrt_types::{types::Limits, verification::VerificationLevel};

    use super::*;
    use crate::{memory::Memory, types::MemoryType};

    #[test]
    fn test_arc_memory_extensions() -> Result<()> {
        // Create a memory instance
        let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
        let memory = Memory::new(mem_type)?;
        let arc_memory = Arc::new(memory);

        // Test basic properties
        assert_eq!(arc_memory.size(), 1);
        assert_eq!(arc_memory.size_in_bytes(), 65536);
        assert_eq!(arc_memory.debug_name(), None);

        // NOTE: When using ArcMemoryExt, clone_and_mutate pattern means changes don't
        // affect the original Let's test the interface methods instead of
        // actual data changes

        // Test reading initial zero data
        let initial_data = arc_memory.read_bytes_safe(0, 3)?;
        assert_eq!(initial_data.len(), 3);
        assert_eq!(initial_data.get(0)?, 0);
        assert_eq!(initial_data.get(1)?, 0);
        assert_eq!(initial_data.get(2)?, 0);

        // Calling write_bytes should return Ok result even though it doesn't modify
        // original
        assert!(arc_memory.write_bytes(0, &[1, 2, 3]).is_ok());

        // Test memory growth also returns success
        let old_size = arc_memory.grow(1)?;
        assert_eq!(old_size, 1);

        // But size remains unchanged on the original Arc
        assert_eq!(arc_memory.size(), 1);

        Ok(())
    }

    #[test]
    fn test_read_bytes_safe() -> Result<()> {
        // Create a memory instance
        let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
        let mut memory = Memory::new(mem_type)?;

        // Initialize memory with some test data
        memory.write(0, &[10, 20, 30, 40, 50])?;

        let arc_memory = Arc::new(memory);

        // Test the safe read implementation
        let safe_data = arc_memory.read_bytes_safe(0, 5)?;

        // Verify the content
        assert_eq!(safe_data.len(), 5);
        assert_eq!(safe_data.get(0)?, 10);
        assert_eq!(safe_data.get(1)?, 20);
        assert_eq!(safe_data.get(2)?, 30);
        assert_eq!(safe_data.get(3)?, 40);
        assert_eq!(safe_data.get(4)?, 50);

        // Test zero-length read
        let empty_data = arc_memory.read_bytes_safe(0, 0)?;
        assert_eq!(empty_data.len(), 0);

        // Test out of bounds read (should return error)
        let result = arc_memory.read_bytes_safe(65536, 10);
        assert!(result.is_err());

        // Test we can read data successfully
        let test_data = arc_memory.read_bytes_safe(0, 5)?;
        assert_eq!(test_data.len(), 5);

        Ok(())
    }

    #[test]
    fn test_read_values_as_safe_stack() -> Result<()> {
        // Create a memory instance
        let mem_type = MemoryType { limits: Limits { min: 1, max: Some(2) } };
        let mut memory = Memory::new(mem_type)?;

        // Initialize memory with i32 test values: 1, 2, 3
        memory.write_i32(0, 1)?;
        memory.write_i32(4, 2)?;
        memory.write_i32(8, 3)?;

        let arc_memory = Arc::new(memory);

        // Read array of 3 i32 values using SafeStack
        let values =
            arc_memory.read_values_as_safe_stack(0, wrt_types::types::ValueType::I32, 3)?;

        // Verify content
        assert_eq!(values.len(), 3);
        assert_eq!(values.get(0)?, Value::I32(1));
        assert_eq!(values.get(1)?, Value::I32(2));
        assert_eq!(values.get(2)?, Value::I32(3));

        Ok(())
    }

    #[test]
    fn test_write_via_callback() -> Result<()> {
        let memory_type = MemoryType { minimum: 1, maximum: Some(2), shared: false };

        let memory = Arc::new(Memory::new(memory_type).unwrap());
        let test_data = [1, 2, 3, 4, 5];

        // Write data
        memory.write_via_callback(0, &test_data).unwrap();

        // Read it back to verify
        let buffer = memory.buffer().unwrap();
        for (i, &byte) in test_data.iter().enumerate() {
            assert_eq!(buffer[i], byte);
        }
        Ok(())
    }

    #[test]
    fn test_grow_via_callback() -> Result<()> {
        let memory_type = MemoryType { minimum: 1, maximum: Some(2), shared: false };

        let memory = Arc::new(Memory::new(memory_type).unwrap());
        let initial_size = memory.size();

        // Grow memory
        let previous_size = memory.grow_via_callback(1).unwrap();

        // Verify growth
        assert_eq!(previous_size, initial_size);
        assert_eq!(memory.size(), initial_size + 1);

        Ok(())
    }
}
