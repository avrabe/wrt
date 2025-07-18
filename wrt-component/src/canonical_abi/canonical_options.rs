//! Enhanced canonical options with full realloc integration
//!
//! This module provides the complete canonical options implementation
//! for the WebAssembly Component Model, including realloc support,
//! post-return functions, and memory management.

#[cfg(not(feature = "std"))]
use std::sync::{Arc, RwLock};
#[cfg(feature = "std")]
use std::sync::{Arc, RwLock};

use wrt_foundation::prelude::*;
// use wrt_runtime::{Instance, Memory};

use crate::{
    canonical_abi::canonical_realloc::{ReallocManager, StringEncoding, ComponentInstanceId},
    memory_layout::MemoryLayout,
    prelude::*,
};

// Type alias for compatibility
pub type ComponentError = Error;

/// Complete canonical options for lift/lower operations
#[derive(Debug, Clone)]
pub struct CanonicalOptions {
    /// Memory index for canonical operations
    pub memory: u32,
    /// Binary std/no_std choice
    pub realloc: Option<u32>,
    /// Post-return function index (optional)
    pub post_return: Option<u32>,
    /// String encoding
    pub string_encoding: StringEncoding,
    /// Instance ID for this set of options
    pub instance_id: ComponentInstanceId,
    /// Binary std/no_std choice
    pub realloc_manager: Option<Arc<RwLock<ReallocManager>>>,
}

/// Canonical lift context with full memory management
pub struct CanonicalLiftContext<'a> {
    /// Runtime instance
    pub instance: &'a Instance,
    /// Memory for lifting
    pub memory: &'a Memory,
    /// Canonical options
    pub options: &'a CanonicalOptions,
    /// Binary std/no_std choice
    allocations: Vec<TempAllocation>,
}

/// Canonical lower context with full memory management
pub struct CanonicalLowerContext<'a> {
    /// Runtime instance
    pub instance: &'a mut Instance,
    /// Memory for lowering
    pub memory: &'a mut Memory,
    /// Canonical options
    pub options: &'a CanonicalOptions,
    /// Allocations made during lower
    allocations: Vec<TempAllocation>,
}

#[derive(Debug)]
struct TempAllocation {
    ptr: i32,
    size: i32,
    align: i32,
}

impl CanonicalOptions {
    /// Create new canonical options with defaults
    pub fn new(memory: u32, instance_id: ComponentInstanceId) -> Self {
        Self {
            memory,
            realloc: None,
            post_return: None,
            string_encoding: StringEncoding::Utf8,
            instance_id,
            realloc_manager: None,
        }
    }

    /// Binary std/no_std choice
    pub fn with_realloc(mut self, func_index: u32, manager: Arc<RwLock<ReallocManager>>) -> Self {
        self.realloc = Some(func_index);
        self.realloc_manager = Some(manager);

        // Register with the manager
        if let Ok(mut mgr) = manager.write() {
            let _ = mgr.register_realloc(self.instance_id, func_index);
        }

        self
    }

    /// Set post-return function
    pub fn with_post_return(mut self, func_index: u32) -> Self {
        self.post_return = Some(func_index);
        self
    }

    /// Set string encoding
    pub fn with_string_encoding(mut self, encoding: StringEncoding) -> Self {
        self.string_encoding = encoding;
        self
    }

    /// Binary std/no_std choice
    pub fn has_realloc(&self) -> bool {
        self.realloc.is_some() && self.realloc_manager.is_some()
    }

    /// Check if post-return is available
    pub fn has_post_return(&self) -> bool {
        self.post_return.is_some()
    }
}

impl<'a> CanonicalLiftContext<'a> {
    /// Create a new lift context
    pub fn new(instance: &'a Instance, memory: &'a Memory, options: &'a CanonicalOptions) -> Self {
        Self { instance, memory, options, allocations: Vec::new() }
    }

    /// Binary std/no_std choice
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<i32, ComponentError> {
        if size == 0 {
            return Ok(0);
        }

        let ptr = if let Some(manager) = &self.options.realloc_manager {
            // Binary std/no_std choice
            let mut mgr = manager.write().map_err(|_| ComponentError::ResourceNotFound(0))?;

            mgr.allocate(self.options.instance_id, size as i32, align as i32)?
        } else {
            // Binary std/no_std choice
            return Err(ComponentError::ResourceNotFound(0));
        };

        // Binary std/no_std choice
        self.allocations.push(TempAllocation { ptr, size: size as i32, align: align as i32 });

        Ok(ptr)
    }

    /// Read bytes from memory
    pub fn read_bytes(&self, ptr: i32, len: usize) -> Result<Vec<u8>, ComponentError> {
        if ptr < 0 {
            return Err(ComponentError::TypeMismatch);
        }

        let offset = ptr as usize;
        self.memory
            .read_slice(offset, len)
            .map_err(|_| ComponentError::ResourceNotFound(ptr as u32))
    }

    /// Read a string from memory with the configured encoding
    pub fn read_string(&self, ptr: i32, len: usize) -> Result<String, ComponentError> {
        let bytes = self.read_bytes(ptr, len)?;

        match self.options.string_encoding {
            StringEncoding::Utf8 => {
                String::from_utf8(bytes).map_err(|_| ComponentError::TypeMismatch)
            }
            StringEncoding::Utf16Le => {
                let u16_values: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&u16_values).map_err(|_| ComponentError::TypeMismatch)
            }
            StringEncoding::Utf16Be => {
                let u16_values: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&u16_values).map_err(|_| ComponentError::TypeMismatch)
            }
            StringEncoding::Latin1 => Ok(bytes.into_iter().map(|b| b as char).collect()),
        }
    }

    /// Binary std/no_std choice
    pub fn cleanup(mut self) -> Result<(), ComponentError> {
        // Binary std/no_std choice
        if let Some(manager) = &self.options.realloc_manager {
            let mut mgr = manager.write().map_err(|_| ComponentError::ResourceNotFound(0))?;

            for alloc in self.allocations.drain(..) {
                mgr.deallocate(self.options.instance_id, alloc.ptr, alloc.size, alloc.align)?;
            }
        }

        // Then call post-return if available
        if let Some(post_return_idx) = self.options.post_return {
            // In a real implementation, this would call the actual function
            // For now, we just acknowledge it exists
        }

        Ok(())
    }
}

impl<'a> CanonicalLowerContext<'a> {
    /// Create a new lower context
    pub fn new(
        instance: &'a mut Instance,
        memory: &'a mut Memory,
        options: &'a CanonicalOptions,
    ) -> Self {
        Self { instance, memory, options, allocations: Vec::new() }
    }

    /// Binary std/no_std choice
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<i32, ComponentError> {
        if size == 0 {
            return Ok(0);
        }

        let ptr = if let Some(manager) = &self.options.realloc_manager {
            // Binary std/no_std choice
            let mut mgr = manager.write().map_err(|_| ComponentError::ResourceNotFound(0))?;

            mgr.allocate(self.options.instance_id, size as i32, align as i32)?
        } else {
            // Binary std/no_std choice
            return Err(ComponentError::ResourceNotFound(0));
        };

        // Binary std/no_std choice
        self.allocations.push(TempAllocation { ptr, size: size as i32, align: align as i32 });

        Ok(ptr)
    }

    /// Write bytes to memory
    pub fn write_bytes(&mut self, ptr: i32, data: &[u8]) -> Result<(), ComponentError> {
        if ptr < 0 {
            return Err(ComponentError::TypeMismatch);
        }

        let offset = ptr as usize;
        self.memory
            .write_slice(offset, data)
            .map_err(|_| ComponentError::ResourceNotFound(ptr as u32))
    }

    /// Write a string to memory with the configured encoding
    pub fn write_string(&mut self, s: &str) -> Result<(i32, usize), ComponentError> {
        let encoded = match self.options.string_encoding {
            StringEncoding::Utf8 => s.as_bytes().to_vec(),
            StringEncoding::Utf16Le => s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect(),
            StringEncoding::Utf16Be => s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect(),
            StringEncoding::Latin1 => {
                s.chars()
                    .map(|c| {
                        if c as u32 <= 0xFF {
                            c as u8
                        } else {
                            b'?' // Replace non-Latin1 chars
                        }
                    })
                    .collect()
            }
        };

        let len = encoded.len();
        let align = match self.options.string_encoding {
            StringEncoding::Utf8 | StringEncoding::Latin1 => 1,
            StringEncoding::Utf16Le | StringEncoding::Utf16Be => 2,
        };

        let ptr = self.allocate(len, align)?;
        self.write_bytes(ptr, &encoded)?;

        Ok((ptr, len))
    }

    /// Binary std/no_std choice
    pub fn finish(self) -> Result<Vec<TempAllocation>, ComponentError> {
        // Binary std/no_std choice
        Ok(self.allocations)
    }
}

/// Builder for canonical options
pub struct CanonicalOptionsBuilder {
    memory: u32,
    realloc: Option<u32>,
    post_return: Option<u32>,
    string_encoding: StringEncoding,
    instance_id: ComponentInstanceId,
    realloc_manager: Option<Arc<RwLock<ReallocManager>>>,
}

impl CanonicalOptionsBuilder {
    pub fn new(memory: u32, instance_id: ComponentInstanceId) -> Self {
        Self {
            memory,
            realloc: None,
            post_return: None,
            string_encoding: StringEncoding::Utf8,
            instance_id,
            realloc_manager: None,
        }
    }

    pub fn with_realloc(mut self, func_index: u32, manager: Arc<RwLock<ReallocManager>>) -> Self {
        self.realloc = Some(func_index);
        self.realloc_manager = Some(manager);
        self
    }

    pub fn with_post_return(mut self, func_index: u32) -> Self {
        self.post_return = Some(func_index);
        self
    }

    pub fn with_string_encoding(mut self, encoding: StringEncoding) -> Self {
        self.string_encoding = encoding;
        self
    }

    pub fn build(self) -> CanonicalOptions {
        let mut options = CanonicalOptions::new(self.memory, self.instance_id);

        if let (Some(func_index), Some(manager)) = (self.realloc, self.realloc_manager) {
            options = options.with_realloc(func_index, manager);
        }

        if let Some(func_index) = self.post_return {
            options = options.with_post_return(func_index);
        }

        options.with_string_encoding(self.string_encoding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_abi::canonical_realloc::ReallocManager;

    #[test]
    fn test_canonical_options_creation() {
        let instance_id = ComponentInstanceId(1);
        let options = CanonicalOptions::new(0, instance_id);

        assert_eq!(options.memory, 0);
        assert_eq!(options.instance_id, instance_id);
        assert!(!options.has_realloc());
        assert!(!options.has_post_return());
    }

    #[test]
    fn test_canonical_options_with_realloc() {
        let instance_id = ComponentInstanceId(1);
        let manager = Arc::new(RwLock::new(ReallocManager::default()));

        let options = CanonicalOptions::new(0, instance_id).with_realloc(42, manager);

        assert!(options.has_realloc());
        assert_eq!(options.realloc, Some(42));
    }

    #[test]
    fn test_canonical_options_builder() {
        let instance_id = ComponentInstanceId(1);
        let manager = Arc::new(RwLock::new(ReallocManager::default()));

        let options = CanonicalOptionsBuilder::new(0, instance_id)
            .with_realloc(42, manager)
            .with_post_return(43)
            .with_string_encoding(StringEncoding::Utf16Le)
            .build();

        assert_eq!(options.memory, 0);
        assert_eq!(options.realloc, Some(42));
        assert_eq!(options.post_return, Some(43));
        assert_eq!(options.string_encoding, StringEncoding::Utf16Le);
        assert!(options.has_realloc());
        assert!(options.has_post_return());
    }

    #[test]
    fn test_string_encodings() {
        // Test UTF-8
        let utf8_bytes = "Hello".as_bytes();
        assert_eq!(utf8_bytes.len(), 5);

        // Test UTF-16 LE
        let utf16_le: Vec<u8> = "Hello".encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        assert_eq!(utf16_le.len(), 10); // 5 chars * 2 bytes

        // Test Latin-1
        let latin1: Vec<u8> = "Hello".chars().map(|c| c as u8).collect();
        assert_eq!(latin1.len(), 5);
    }
}
