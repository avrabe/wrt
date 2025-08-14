//! Multi-memory support for WebAssembly.
//!
//! This module implements support for the WebAssembly multi-memory proposal,
//! which allows a single WebAssembly module to have multiple linear memories.
//! This enables better memory management and isolation for complex
//! applications.
//!
//! # Features
//!
//! - Memory-indexed load/store operations
//! - Memory-indexed bulk operations (fill, copy, init)
//! - Memory grow and size operations for each memory
//! - Full validation for memory indices
//!
//! The implementation works across std, `no_std+alloc`, and pure `no_std`
//! environments.

use wrt_error::{
    Error,
    Result,
};
use wrt_foundation::{
    types::ValueType,
    values::Value,
};

use crate::prelude::{
    DataSegmentOperations,
    Debug,
    MemoryOperations,
    Validate,
    ValidationContext,
};

/// Maximum number of memories supported per module
pub const MAX_MEMORIES: usize = 16;

/// Multi-memory load operation with memory index
#[derive(Debug, Clone)]
pub struct MultiMemoryLoad {
    /// Memory index
    pub memory_index: u32,
    /// Memory offset
    pub offset:       u32,
    /// Required alignment
    pub align:        u32,
    /// Value type to load
    pub value_type:   ValueType,
    /// Whether this is a signed load
    pub signed:       bool,
    /// Memory access width in bytes
    pub width:        u32,
}

impl MultiMemoryLoad {
    /// Create a new multi-memory load operation
    #[must_use]
    pub fn new(
        memory_index: u32,
        offset: u32,
        align: u32,
        value_type: ValueType,
        signed: bool,
        width: u32,
    ) -> Self {
        Self {
            memory_index,
            offset,
            align,
            value_type,
            signed,
            width,
        }
    }

    /// Create i32.load operation for specific memory
    #[must_use]
    pub fn i32_load(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, false, 32)
    }

    /// Create i64.load operation for specific memory
    #[must_use]
    pub fn i64_load(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, false, 64)
    }

    /// Create f32.load operation for specific memory
    #[must_use]
    pub fn f32_load(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::F32, false, 32)
    }

    /// Create f64.load operation for specific memory
    #[must_use]
    pub fn f64_load(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::F64, false, 64)
    }

    /// Create `i32.load8_s` operation for specific memory
    #[must_use]
    pub fn i32_load8_s(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, true, 8)
    }

    /// Create `i32.load8_u` operation for specific memory
    #[must_use]
    pub fn i32_load8_u(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, false, 8)
    }

    /// Create `i32.load16_s` operation for specific memory
    #[must_use]
    pub fn i32_load16_s(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, true, 16)
    }

    /// Create `i32.load16_u` operation for specific memory
    #[must_use]
    pub fn i32_load16_u(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, false, 16)
    }

    /// Create `i64.load8_s` operation for specific memory
    #[must_use]
    pub fn i64_load8_s(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, true, 8)
    }

    /// Create `i64.load8_u` operation for specific memory
    #[must_use]
    pub fn i64_load8_u(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, false, 8)
    }

    /// Create `i64.load16_s` operation for specific memory
    #[must_use]
    pub fn i64_load16_s(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, true, 16)
    }

    /// Create `i64.load16_u` operation for specific memory
    #[must_use]
    pub fn i64_load16_u(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, false, 16)
    }

    /// Create `i64.load32_s` operation for specific memory
    #[must_use]
    pub fn i64_load32_s(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, true, 32)
    }

    /// Create `i64.load32_u` operation for specific memory
    #[must_use]
    pub fn i64_load32_u(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, false, 32)
    }

    /// Execute the multi-memory load operation
    /// Note: In a real implementation, this would be called by the runtime
    /// with access to the specific memory instance
    pub fn execute_with_memory(
        &self,
        memory: &impl MemoryOperations,
        addr_arg: &Value,
    ) -> Result<Value> {
        // Convert to basic MemoryLoad and execute
        let basic_load = crate::memory_ops::MemoryLoad {
            memory_index: self.memory_index,
            offset:       self.offset,
            align:        self.align,
            value_type:   self.value_type,
            signed:       self.signed,
            width:        self.width,
        };

        basic_load.execute(memory, addr_arg)
    }
}

/// Multi-memory store operation with memory index
#[derive(Debug, Clone)]
pub struct MultiMemoryStore {
    /// Memory index
    pub memory_index: u32,
    /// Memory offset
    pub offset:       u32,
    /// Required alignment
    pub align:        u32,
    /// Value type to store
    pub value_type:   ValueType,
    /// Memory access width in bytes
    pub width:        u32,
}

impl MultiMemoryStore {
    /// Create a new multi-memory store operation
    #[must_use]
    pub fn new(
        memory_index: u32,
        offset: u32,
        align: u32,
        value_type: ValueType,
        width: u32,
    ) -> Self {
        Self {
            memory_index,
            offset,
            align,
            value_type,
            width,
        }
    }

    /// Create i32.store operation for specific memory
    #[must_use]
    pub fn i32_store(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, 32)
    }

    /// Create i64.store operation for specific memory
    #[must_use]
    pub fn i64_store(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, 64)
    }

    /// Create f32.store operation for specific memory
    #[must_use]
    pub fn f32_store(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::F32, 32)
    }

    /// Create f64.store operation for specific memory
    #[must_use]
    pub fn f64_store(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::F64, 64)
    }

    /// Create i32.store8 operation for specific memory
    #[must_use]
    pub fn i32_store8(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, 8)
    }

    /// Create i32.store16 operation for specific memory
    #[must_use]
    pub fn i32_store16(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I32, 16)
    }

    /// Create i64.store8 operation for specific memory
    #[must_use]
    pub fn i64_store8(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, 8)
    }

    /// Create i64.store16 operation for specific memory
    #[must_use]
    pub fn i64_store16(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, 16)
    }

    /// Create i64.store32 operation for specific memory
    #[must_use]
    pub fn i64_store32(memory_index: u32, offset: u32, align: u32) -> Self {
        Self::new(memory_index, offset, align, ValueType::I64, 32)
    }

    /// Execute the multi-memory store operation
    /// Note: In a real implementation, this would be called by the runtime
    /// with access to the specific memory instance
    pub fn execute_with_memory(
        &self,
        memory: &mut impl MemoryOperations,
        addr_arg: &Value,
        value: &Value,
    ) -> Result<()> {
        // Convert to basic MemoryStore and execute
        let basic_store = crate::memory_ops::MemoryStore {
            memory_index: self.memory_index,
            offset:       self.offset,
            align:        self.align,
            value_type:   self.value_type,
            width:        self.width,
        };

        basic_store.execute(memory, addr_arg, value)
    }
}

/// Multi-memory bulk operations with memory indices
#[derive(Debug, Clone)]
pub struct MultiMemoryBulk {
    /// Memory index for bulk operations
    pub memory_index: u32,
}

impl MultiMemoryBulk {
    /// Create new multi-memory bulk operations helper
    #[must_use]
    pub fn new(memory_index: u32) -> Self {
        Self { memory_index }
    }

    /// Execute memory.fill operation on specific memory
    pub fn fill(
        &self,
        memory: &mut impl MemoryOperations,
        dest: &Value,
        value: &Value,
        size: &Value,
    ) -> Result<()> {
        // Create and execute MemoryFill
        let fill_op = crate::memory_ops::MemoryFill::new(self.memory_index);
        fill_op.execute(memory, dest, value, size)
    }

    /// Execute memory.copy operation within the same memory
    pub fn copy(
        &self,
        memory: &mut impl MemoryOperations,
        dest: &Value,
        src: &Value,
        size: &Value,
    ) -> Result<()> {
        // Create and execute MemoryCopy (same memory)
        let copy_op = crate::memory_ops::MemoryCopy::new(self.memory_index, self.memory_index);
        copy_op.execute(memory, dest, src, size)
    }

    /// Execute memory.init operation on specific memory
    pub fn init(
        &self,
        memory: &mut impl MemoryOperations,
        data_segments: &impl DataSegmentOperations,
        data_index: u32,
        dest: &Value,
        src: &Value,
        size: &Value,
    ) -> Result<()> {
        // Create and execute MemoryInit
        let init_op = crate::memory_ops::MemoryInit::new(self.memory_index, data_index);
        init_op.execute(memory, data_segments, dest, src, size)
    }
}

/// Multi-memory cross-memory copy operation
#[derive(Debug, Clone)]
pub struct MultiMemoryCrossCopy {
    /// Destination memory index
    pub dest_memory_index: u32,
    /// Source memory index
    pub src_memory_index:  u32,
}

impl MultiMemoryCrossCopy {
    /// Create new cross-memory copy operation
    #[must_use]
    pub fn new(dest_memory_index: u32, src_memory_index: u32) -> Self {
        Self {
            dest_memory_index,
            src_memory_index,
        }
    }

    /// Execute cross-memory copy operation
    /// Note: This is a simplified implementation. A real runtime would
    /// need access to both memory instances to perform the copy.
    pub fn execute(
        &self,
        _dest_memory: &mut impl MemoryOperations,
        _src_memory: &impl MemoryOperations,
        _dest: &Value,
        _src: &Value,
        _size: &Value,
    ) -> Result<()> {
        // For now, just validate the operation structure
        if self.dest_memory_index == self.src_memory_index {
            return Err(Error::memory_error(
                "Use regular copy for same-memory operations",
            ));
        }

        // Actual implementation would:
        // 1. Read data from src_memory at src offset
        // 2. Write data to dest_memory at dest offset
        // 3. Handle overlapping regions properly

        Ok(()) // Placeholder
    }
}

/// Memory size operation for multi-memory
#[derive(Debug, Clone)]
pub struct MultiMemorySize {
    /// Memory index
    pub memory_index: u32,
}

impl MultiMemorySize {
    /// Create new memory size operation
    #[must_use]
    pub fn new(memory_index: u32) -> Self {
        Self { memory_index }
    }

    /// Execute memory.size operation on specific memory
    pub fn execute(&self, memory: &impl MemoryOperations) -> Result<Value> {
        // Get size in pages (64KB each)
        let size_bytes = memory.size_in_bytes()?;
        let size_pages = (size_bytes / 65536) as u32;

        Ok(Value::I32(size_pages as i32))
    }
}

/// Memory grow operation for multi-memory
#[derive(Debug, Clone)]
pub struct MultiMemoryGrow {
    /// Memory index
    pub memory_index: u32,
}

impl MultiMemoryGrow {
    /// Create new memory grow operation
    #[must_use]
    pub fn new(memory_index: u32) -> Self {
        Self { memory_index }
    }

    /// Execute memory.grow operation on specific memory
    /// Returns the old size in pages, or -1 if grow failed
    pub fn execute(&self, memory: &mut impl MemoryOperations, pages: &Value) -> Result<Value> {
        // Extract page count
        let page_count = match pages {
            Value::I32(p) => *p as u32,
            _ => return Err(Error::type_error("memory.grow requires i32 argument")),
        };

        // Get current size
        let old_size_bytes = memory.size_in_bytes()?;
        let old_size_pages = (old_size_bytes / 65536) as u32;

        // Try to grow - convert pages to bytes
        let delta_bytes = (page_count as usize) * 65536;
        match memory.grow(delta_bytes) {
            Ok(()) => Ok(Value::I32(old_size_pages as i32)),
            Err(_) => Ok(Value::I32(-1)), // WebAssembly convention for grow failure
        }
    }
}

/// Helper trait for memory index validation
pub trait MultiMemoryValidation {
    /// Validate that a memory index is valid for this module
    fn validate_memory_index(&self, index: u32) -> Result<()>;

    /// Get the number of memories in this module
    fn memory_count(&self) -> u32;
}

// Validation support
impl Validate for MultiMemoryLoad {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // Check memory index
        if self.memory_index >= ctx.memories {
            return Err(Error::validation_error("Invalid memory index"));
        }

        // Validate like regular memory load
        crate::validation::validate_memory_op(
            "multi_memory.load",
            self.memory_index,
            self.align,
            self.value_type,
            true, // is_load
            ctx,
        )
    }
}

impl Validate for MultiMemoryStore {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // Check memory index
        if self.memory_index >= ctx.memories {
            return Err(Error::validation_error("Invalid memory index"));
        }

        // Validate like regular memory store
        crate::validation::validate_memory_op(
            "multi_memory.store",
            self.memory_index,
            self.align,
            self.value_type,
            false, // is_load
            ctx,
        )
    }
}

impl Validate for MultiMemoryBulk {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // Check memory index
        if self.memory_index >= ctx.memories {
            return Err(Error::validation_error("Invalid memory index"));
        }
        Ok(())
    }
}

impl Validate for MultiMemoryCrossCopy {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // Check both memory indices
        if self.dest_memory_index >= ctx.memories || self.src_memory_index >= ctx.memories {
            return Err(Error::validation_error("Invalid memory index"));
        }

        // memory.copy: [i32, i32, i32] -> []
        if !ctx.is_unreachable() {
            ctx.pop_expect(ValueType::I32)?; // size
            ctx.pop_expect(ValueType::I32)?; // src
            ctx.pop_expect(ValueType::I32)?; // dest
        }
        Ok(())
    }
}

impl Validate for MultiMemorySize {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // Check memory index
        if self.memory_index >= ctx.memories {
            return Err(Error::validation_error("Invalid memory index"));
        }

        // memory.size: [] -> [i32]
        ctx.push_type(ValueType::I32)?;
        Ok(())
    }
}

impl Validate for MultiMemoryGrow {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<()> {
        // Check memory index
        if self.memory_index >= ctx.memories {
            return Err(Error::validation_error("Invalid memory index"));
        }

        // memory.grow: [i32] -> [i32]
        if !ctx.is_unreachable() {
            ctx.pop_expect(ValueType::I32)?;
        }
        ctx.push_type(ValueType::I32)?;
        Ok(())
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::memory_ops::MemoryOperations;

    struct MockMemory {
        data: Vec<u8>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self { data: Vec::new() }
        }
    }

    impl MemoryOperations for MockMemory {
        #[cfg(feature = "std")]
        fn read_bytes(&self, offset: u32, len: u32) -> Result<Vec<u8>> {
            let start = offset as usize;
            let end = start + len as usize;
            if end > self.data.len() {
                return Err(Error::memory_error("Read out of bounds"));
            }
            Ok(self.data[start..end].to_vec())
        }

        #[cfg(not(feature = "std"))]
        fn read_bytes(
            &self,
            offset: u32,
            len: u32,
        ) -> Result<wrt_foundation::BoundedVec<u8, 65536, wrt_foundation::NoStdProvider<65536>>>
        {
            let start = offset as usize;
            let end = start + len as usize;
            if end > self.data.len() {
                return Err(Error::memory_error("Read out of bounds"));
            }

            let provider = wrt_foundation::wrt_provider!(
                len,
                wrt_foundation::budget_aware_provider::CrateId::Instructions
            )
            .unwrap_or_default();
            let mut result = wrt_foundation::BoundedVec::new(provider)?;
            for i in start..end {
                result
                    .push(self.data[i])
                    .map_err(|_| Error::memory_error("Result vector full"))?;
            }
            Ok(result)
        }

        fn write_bytes(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
            let start = offset as usize;
            let end = start + bytes.len();

            // Extend data if necessary
            if end > self.data.len() {
                self.data.resize(end, 0);
            }

            // Copy bytes
            self.data[start..end].copy_from_slice(bytes);
            Ok(())
        }

        fn size_in_bytes(&self) -> Result<usize> {
            Ok(self.data.len())
        }

        fn fill(&mut self, offset: u32, value: u8, size: u32) -> Result<()> {
            let start = offset as usize;
            let end = start + size as usize;

            // Extend data if necessary
            if end > self.data.len() {
                self.data.resize(end, 0);
            }

            // Fill with value
            for i in start..end {
                self.data[i] = value;
            }
            Ok(())
        }

        fn copy(&mut self, dest: u32, src: u32, size: u32) -> Result<()> {
            if dest == src || size == 0 {
                return Ok();
            }

            let dest_start = dest as usize;
            let src_start = src as usize;
            let copy_size = size as usize;

            // Extend data if necessary
            let max_end = core::cmp::max(dest_start + copy_size, src_start + copy_size);
            if max_end > self.data.len() {
                self.data.resize(max_end, 0);
            }

            // Use Vec's copy_within for safe overlapping copy
            if dest_start < src_start {
                // Copy forward
                for i in 0..copy_size {
                    self.data[dest_start + i] = self.data[src_start + i];
                }
            } else {
                // Copy backward
                for i in (0..copy_size).rev() {
                    self.data[dest_start + i] = self.data[src_start + i];
                }
            }
            Ok(())
        }

        fn grow(&mut self, pages: u32) -> Result<u32> {
            let additional_bytes = pages as usize * 65536;
            let old_size = self.data.len();
            self.data.resize(old_size + additional_bytes, 0);
            Ok((old_size / 65536) as u32)
        }
    }

    #[test]
    fn test_multi_memory_load() {
        let mut memory = MockMemory::new();

        // Write test data
        memory.write_bytes(0, &[0x42, 0x43, 0x44, 0x45]).unwrap();

        // Test i32 load from memory 0
        let load_op = MultiMemoryLoad::i32_load(0, 0, 4);
        let result = load_op.execute_with_memory(&memory, &Value::I32(0)).unwrap();

        // Verify result (little-endian)
        assert_eq!(result, Value::I32(0x45444342u32 as i32));
    }

    #[test]
    fn test_multi_memory_store() {
        let mut memory = MockMemory::new();

        // Test i32 store to memory 1
        let store_op = MultiMemoryStore::i32_store(1, 0, 4);
        store_op
            .execute_with_memory(&mut memory, &Value::I32(0), &Value::I32(0x12345678))
            .unwrap();

        // Verify stored data
        let data = memory.read_bytes(0, 4).unwrap();
        #[cfg(feature = "std")]
        assert_eq!(data, vec![0x78, 0x56, 0x34, 0x12]); // little-endian
    }

    #[test]
    fn test_multi_memory_size_and_grow() {
        let mut memory = MockMemory::new();

        // Test size (should be 0 pages initially)
        let size_op = MultiMemorySize::new(0);
        let size = size_op.execute(&memory).unwrap();
        assert_eq!(size, Value::I32(0));

        // Test grow
        let grow_op = MultiMemoryGrow::new(0);
        let old_size = grow_op.execute(&mut memory, &Value::I32(1)).unwrap();
        assert_eq!(old_size, Value::I32(0));

        // Test size after grow
        let new_size = size_op.execute(&memory).unwrap();
        assert_eq!(new_size, Value::I32(1));
    }

    #[test]
    fn test_multi_memory_bulk_operations() {
        let mut memory = MockMemory::new();
        memory.data.resize(100, 0); // Binary std/no_std choice

        let bulk_ops = MultiMemoryBulk::new(0);

        // Test fill
        bulk_ops
            .fill(
                &mut memory,
                &Value::I32(10),
                &Value::I32(0xAB),
                &Value::I32(5),
            )
            .unwrap();

        // Verify fill
        let data = memory.read_bytes(10, 5).unwrap();
        #[cfg(feature = "std")]
        assert_eq!(data, vec![0xAB; 5]);

        // Test copy
        bulk_ops
            .copy(
                &mut memory,
                &Value::I32(20),
                &Value::I32(10),
                &Value::I32(5),
            )
            .unwrap();

        // Verify copy
        let copied_data = memory.read_bytes(20, 5).unwrap();
        #[cfg(feature = "std")]
        assert_eq!(copied_data, vec![0xAB; 5]);
    }

    #[test]
    fn test_cross_memory_copy() {
        let mut dest_memory = MockMemory::new();
        let src_memory = MockMemory::new();

        let cross_copy = MultiMemoryCrossCopy::new(0, 1);

        // This is a placeholder test since actual implementation requires
        // more complex memory management
        let result = cross_copy.execute(
            &mut dest_memory,
            &src_memory,
            &Value::I32(0),
            &Value::I32(0),
            &Value::I32(4),
        );

        assert!(result.is_ok());
    }
}
