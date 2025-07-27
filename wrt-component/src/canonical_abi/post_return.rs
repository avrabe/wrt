//! Post-return implementation for WebAssembly Component Model MVP
//!
//! This module implements the post-return canonical ABI functionality required
//! by the latest Component Model MVP specification for proper resource cleanup
//! and memory management after component function calls.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

use wrt_error::{Error, ErrorCategory, Result};
use wrt_foundation::{
    bounded::BoundedVec,
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    values::Value,
};

#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;

#[cfg(not(feature = "std"))]
use crate::types::Value as ComponentValue;

use crate::{
    canonical_abi::canonical_options::CanonicalOptions,
    instance::Instance,
    memory::Memory,
    runtime::Runtime,
};

/// Post-return cleanup entry tracking resources that need cleanup
#[derive(Debug, Clone)]
pub struct PostReturnEntry {
    /// Function index to call for cleanup
    pub func_index: u32,
    /// Arguments to pass to the cleanup function
    #[cfg(feature = "std")]
    pub args: Vec<ComponentValue>,
    #[cfg(not(feature = "std"))]
    pub args: BoundedVec<ComponentValue, 16, NoStdProvider<65536>>,
    /// Priority of this cleanup operation (higher = more critical)
    pub priority: CleanupPriority,
    /// Resource type being cleaned up
    pub resource_type: ResourceType,
}

/// Priority levels for post-return cleanup operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanupPriority {
    /// Low priority - optional cleanup
    Low = 0,
    /// Normal priority - standard cleanup
    Normal = 1,
    /// High priority - important cleanup
    High = 2,
    /// Critical priority - must not fail
    Critical = 3,
}

/// Type of resource being cleaned up
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// Memory allocations
    Memory,
    /// File handles
    File,
    /// Network connections
    Network,
    /// Component instances
    Component,
    /// Stream handles
    Stream,
    /// Future handles
    Future,
    /// Generic resource
    Generic,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Memory => write!(f, "memory"),
            ResourceType::File => write!(f, "file"),
            ResourceType::Network => write!(f, "network"),
            ResourceType::Component => write!(f, "component"),
            ResourceType::Stream => write!(f, "stream"),
            ResourceType::Future => write!(f, "future"),
            ResourceType::Generic => write!(f, "generic"),
        }
    }
}

/// Post-return execution context
#[derive(Debug)]
pub struct PostReturnContext {
    /// Cleanup entries pending execution
    #[cfg(feature = "std")]
    entries: Vec<PostReturnEntry>,
    #[cfg(not(feature = "std"))]
    entries: BoundedVec<PostReturnEntry, 256, NoStdProvider<65536>>,
    
    /// Whether post-return is currently executing
    is_executing: bool,
    
    /// Error recovery mode
    error_recovery: ErrorRecoveryMode,
    
    /// Statistics for post-return operations
    stats: PostReturnStats,
}

/// Error recovery strategy for post-return operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRecoveryMode {
    /// Continue executing remaining cleanup operations on error
    Continue,
    /// Stop on first error
    StopOnError,
    /// Best effort - try all operations but collect errors
    BestEffort,
}

/// Statistics for post-return operations
#[derive(Debug, Default, Clone)]
pub struct PostReturnStats {
    /// Total number of cleanup operations executed
    pub operations_executed: u64,
    /// Number of successful operations
    pub operations_successful: u64,
    /// Number of failed operations
    pub operations_failed: u64,
    /// Total time spent in post-return (microseconds)
    pub total_time_us: u64,
    /// Maximum single operation time (microseconds)
    pub max_operation_time_us: u64,
}

impl Default for PostReturnContext {
    fn default() -> Self {
        // For Default trait, we prefer to panic rather than return invalid state
        // This ensures capability-based allocation is always properly initialized
        Self::new().expect("Failed to create PostReturnContext with proper capability allocation")
    }
}

impl PostReturnContext {
    /// Create a new post-return context
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            entries: Vec::new(),
            #[cfg(not(feature = "std"))]
            entries: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            is_executing: false,
            error_recovery: ErrorRecoveryMode::BestEffort,
            stats: PostReturnStats::default(),
        })
    }

    /// Add a cleanup entry to be executed during post-return
    pub fn add_cleanup(&mut self, entry: PostReturnEntry) -> Result<()> {
        if self.is_executing {
            return Err(Error::runtime_invalid_state("cannot_add_cleanup_during_post_return";
        }

        #[cfg(feature = "std")]
        {
            self.entries.push(entry);
        }
        #[cfg(not(feature = "std"))]
        {
            self.entries.push(entry).map_err(|_| {
                Error::resource_exhausted("post_return_cleanup_queue_full")
            })?;
        }

        // Sort by priority (highest first)
        #[cfg(feature = "std")]
        {
            self.entries.sort_by(|a, b| b.priority.cmp(&a.priority;
        }
        #[cfg(not(feature = "std"))]
        {
            // Simple bubble sort for no_std
            let len = self.entries.len);
            for i in 0..len {
                for j in 0..len - 1 - i {
                    if self.entries.get(j).unwrap().priority < self.entries.get(j + 1).unwrap().priority {
                        // Manual swap for BoundedVec
                        let temp = self.entries.get(j).unwrap().clone();
                        *self.entries.get_mut(j).unwrap() = self.entries.get(j + 1).unwrap().clone();
                        *self.entries.get_mut(j + 1).unwrap() = temp;
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute all pending post-return cleanup operations
    pub fn execute_post_return(&mut self, instance: &mut Instance, memory: &mut Memory, options: &CanonicalOptions) -> Result<()> {
        if self.is_executing {
            return Err(Error::runtime_invalid_state("post_return_already_executing";
        }

        if !options.has_post_return() {
            return Ok(()); // No post-return function configured
        }

        self.is_executing = true;
        let start_time = current_time_us);
        let mut errors = Vec::new());

        // Execute each cleanup operation in priority order
        let entries = mem::take(&mut self.entries;
        for entry in entries {
            let operation_start = current_time_us);
            
            match self.execute_cleanup_entry(instance, memory, &entry) {
                Ok(()) => {
                    self.stats.operations_successful += 1;
                }
                Err(e) => {
                    self.stats.operations_failed += 1;
                    errors.push((entry.resource_type, e);
                    
                    match self.error_recovery {
                        ErrorRecoveryMode::StopOnError => break,
                        ErrorRecoveryMode::Continue | ErrorRecoveryMode::BestEffort => continue,
                    }
                }
            }
            
            let operation_time = current_time_us() - operation_start;
            if operation_time > self.stats.max_operation_time_us {
                self.stats.max_operation_time_us = operation_time;
            }
            self.stats.operations_executed += 1;
        }

        self.stats.total_time_us += current_time_us() - start_time;
        self.is_executing = false;

        // Handle collected errors based on recovery mode
        if !errors.is_empty() && self.error_recovery == ErrorRecoveryMode::StopOnError {
            let (resource_type, error) = errors.into_iter().next().unwrap();
            return Err(Error::runtime_execution_error(&format!("Post-return cleanup failed for {}: {}", resource_type, error);
        }

        Ok(())
    }

    /// Execute a single cleanup entry
    fn execute_cleanup_entry(&self, instance: &mut Instance, memory: &mut Memory, entry: &PostReturnEntry) -> Result<()> {
        // Convert ComponentValue args to raw values for function call
        let mut raw_args = Vec::new());
        
        #[cfg(feature = "std")]
        {
            for arg in &entry.args {
                raw_args.push(self.component_value_to_raw(arg)?;
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for i in 0..entry.args.len() {
                if let Some(arg) = entry.args.get(i) {
                    raw_args.push(self.component_value_to_raw(arg)?;
                }
            }
        }

        // Call the post-return function
        instance.call_function(entry.func_index, &raw_args)
            .map_err(|e| Error::runtime_execution_error(&format!("Post-return function call failed: {}", e)))?;

        Ok(())
    }

    /// Convert ComponentValue to raw value for function calls
    fn component_value_to_raw(&self, value: &ComponentValue) -> Result<Value> {
        match value {
            ComponentValue::Bool(b) => Ok(Value::I32(if *b { 1 } else { 0 })),
            ComponentValue::S8(v) => Ok(Value::I32(*v as i32)),
            ComponentValue::U8(v) => Ok(Value::I32(*v as i32)),
            ComponentValue::S16(v) => Ok(Value::I32(*v as i32)),
            ComponentValue::U16(v) => Ok(Value::I32(*v as i32)),
            ComponentValue::S32(v) => Ok(Value::I32(*v)),
            ComponentValue::U32(v) => Ok(Value::I32(*v as i32)),
            ComponentValue::S64(v) => Ok(Value::I64(*v)),
            ComponentValue::U64(v) => Ok(Value::I64(*v as i64)),
            ComponentValue::F32(v) => Ok(Value::F32(wrt_foundation::FloatBits32::from_bits(*v))),
            ComponentValue::F64(v) => Ok(Value::F64(wrt_foundation::FloatBits64::from_bits(*v))),
            #[cfg(feature = "std")]
            ComponentValue::String(s) => {
                // For strings, we typically pass pointer and length
                // This is a simplified version - real implementation would need
                // proper memory management
                Ok(Value::I32(s.as_ptr() as i32))
            }
            #[cfg(not(feature = "std"))]
            ComponentValue::String(s) => {
                Ok(Value::I32(s.as_str().as_ptr() as i32))
            }
            _ => Err(Error::runtime_execution_error("unsupported_component_value_type"))
        }
    }

    /// Set error recovery mode
    pub fn set_error_recovery(&mut self, mode: ErrorRecoveryMode) {
        self.error_recovery = mode;
    }

    /// Get current statistics
    pub fn stats(&self) -> &PostReturnStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = PostReturnStats::default());
    }

    /// Check if post-return is currently executing
    pub fn is_executing(&self) -> bool {
        self.is_executing
    }

    /// Get number of pending cleanup operations
    pub fn pending_operations(&self) -> usize {
        self.entries.len()
    }
}

/// Helper functions for post-return management
pub mod helpers {
    use super::*;

    /// Create a memory cleanup entry
    pub fn create_memory_cleanup(func_index: u32, ptr: u32, size: u32) -> Result<PostReturnEntry> {
        Ok(PostReturnEntry {
            func_index,
            #[cfg(feature = "std")]
            args: vec![ComponentValue::U32(ptr), ComponentValue::U32(size)],
            #[cfg(not(feature = "std"))]
            args: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                let mut args = BoundedVec::new(provider)?;
                args.push(ComponentValue::U32(ptr))?;
                args.push(ComponentValue::U32(size))?;
                args
            },
            priority: CleanupPriority::High,
            resource_type: ResourceType::Memory,
        })
    }

    /// Create a resource handle cleanup entry
    pub fn create_resource_cleanup(func_index: u32, handle: u32, resource_type: ResourceType) -> Result<PostReturnEntry> {
        Ok(PostReturnEntry {
            func_index,
            #[cfg(feature = "std")]
            args: vec![ComponentValue::U32(handle)],
            #[cfg(not(feature = "std"))]
            args: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                let mut args = BoundedVec::new(provider)?;
                args.push(ComponentValue::U32(handle))?;
                args
            },
            priority: CleanupPriority::Critical,
            resource_type,
        })
    }

    /// Create a component cleanup entry
    pub fn create_component_cleanup(func_index: u32, instance_id: u32) -> Result<PostReturnEntry> {
        Ok(PostReturnEntry {
            func_index,
            #[cfg(feature = "std")]
            args: vec![ComponentValue::U32(instance_id)],
            #[cfg(not(feature = "std"))]
            args: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                let mut args = BoundedVec::new(provider)?;
                args.push(ComponentValue::U32(instance_id))?;
                args
            },
            priority: CleanupPriority::Normal,
            resource_type: ResourceType::Component,
        })
    }
}

/// Get current time in microseconds (platform-specific)
fn current_time_us() -> u64 {
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
    #[cfg(not(feature = "std"))]
    {
        // ASIL-D safe: Use atomic counter instead of unsafe static mut
        use core::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0;
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_return_context_creation() {
        let context = PostReturnContext::new().unwrap();
        assert_eq!(context.pending_operations(), 0);
        assert!(!context.is_executing();
        assert_eq!(context.stats().operations_executed, 0);
    }

    #[test]
    fn test_cleanup_entry_creation() {
        let entry = helpers::create_memory_cleanup(1, 0x1000, 4096).unwrap();
        assert_eq!(entry.func_index, 1);
        assert_eq!(entry.priority, CleanupPriority::High;
        assert_eq!(entry.resource_type, ResourceType::Memory;
        
        #[cfg(feature = "std")]
        {
            assert_eq!(entry.args.len(), 2;
        }
        #[cfg(not(feature = "std"))]
        {
            assert_eq!(entry.args.len(), 2;
        }
    }

    #[test]
    fn test_cleanup_priority_ordering() {
        let mut context = PostReturnContext::new().unwrap();
        
        let low_entry = PostReturnEntry {
            func_index: 1,
            #[cfg(feature = "std")]
            args: vec![],
            #[cfg(not(feature = "std"))]
            args: {
                let provider = safe_managed_alloc!(65536, CrateId::Component).unwrap();
                BoundedVec::new(provider).unwrap()
            },
            priority: CleanupPriority::Low,
            resource_type: ResourceType::Generic,
        };
        
        let high_entry = PostReturnEntry {
            func_index: 2,
            #[cfg(feature = "std")]
            args: vec![],
            #[cfg(not(feature = "std"))]
            args: {
                let provider = safe_managed_alloc!(65536, CrateId::Component).unwrap();
                BoundedVec::new(provider).unwrap()
            },
            priority: CleanupPriority::High,
            resource_type: ResourceType::Generic,
        };
        
        // Add low priority first, then high priority
        context.add_cleanup(low_entry).unwrap();
        context.add_cleanup(high_entry).unwrap();
        
        assert_eq!(context.pending_operations(), 2;
        
        // High priority should be first after sorting
        #[cfg(feature = "std")]
        {
            assert_eq!(context.entries[0].priority, CleanupPriority::High;
            assert_eq!(context.entries[1].priority, CleanupPriority::Low;
        }
    }

    #[test]
    fn test_error_recovery_modes() {
        let mut context = PostReturnContext::new();
        
        context.set_error_recovery(ErrorRecoveryMode::StopOnError;
        assert_eq!(context.error_recovery, ErrorRecoveryMode::StopOnError;
        
        context.set_error_recovery(ErrorRecoveryMode::BestEffort;
        assert_eq!(context.error_recovery, ErrorRecoveryMode::BestEffort;
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::Memory.to_string(), "memory";
        assert_eq!(ResourceType::File.to_string(), "file";
        assert_eq!(ResourceType::Stream.to_string(), "stream";
    }
}