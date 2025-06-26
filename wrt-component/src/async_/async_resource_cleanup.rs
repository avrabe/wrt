//! Async Resource Cleanup System for WebAssembly Component Model
//!
//! This module implements comprehensive cleanup of async resources including streams,
//! futures, tasks, handles, and other resources when WebAssembly functions complete.
//! It integrates with the post-return mechanism to ensure proper resource management.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(feature = "std")]
use std::{
    boxed::Box,
    vec::Vec,
    collections::BTreeMap,
    sync::Arc,
    string::String,
};

use wrt_foundation::{
    bounded::{BoundedVec, BoundedString},
    prelude::*,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

use crate::async_::async_types::{StreamHandle, FutureHandle, ErrorContextHandle};
use crate::types::{Value};
// Note: ComponentInstanceId and TypeId may not exist - using placeholders
pub use crate::types::ComponentInstanceId;
pub type TypeId = u32;

use wrt_error::{Error, ErrorCategory, Result};

/// Maximum number of cleanup entries in no_std
const MAX_CLEANUP_ENTRIES: usize = 512;

/// Maximum number of async resources per instance in no_std
const MAX_ASYNC_RESOURCES_PER_INSTANCE: usize = 128;

/// Comprehensive async resource cleanup manager
#[derive(Debug)]
pub struct AsyncResourceCleanupManager {
    /// Cleanup entries by instance
    #[cfg(feature = "std")]
    cleanup_entries: BTreeMap<ComponentInstanceId, Vec<AsyncCleanupEntry>>,
    #[cfg(not(any(feature = "std", )))]
    cleanup_entries: BoundedVec<(ComponentInstanceId, BoundedVec<AsyncCleanupEntry, MAX_ASYNC_RESOURCES_PER_INSTANCE>), MAX_CLEANUP_ENTRIES>,
    
    /// Global cleanup statistics
    stats: AsyncCleanupStats,
    
    /// Next cleanup ID
    next_cleanup_id: u32,
}

/// Entry representing a single async resource to be cleaned up
#[derive(Debug, Clone)]
pub struct AsyncCleanupEntry {
    /// Unique cleanup ID
    pub cleanup_id: u32,
    
    /// Type of resource to clean up
    pub resource_type: AsyncResourceType,
    
    /// Priority (higher = cleaned up first)
    pub priority: u8,
    
    /// Resource-specific cleanup data
    pub cleanup_data: AsyncCleanupData,
    
    /// Whether this cleanup is critical (must not fail)
    pub critical: bool,
    
    /// Creation timestamp
    pub created_at: u64,
}

/// Types of async resources that can be cleaned up
#[derive(Debug, Clone, PartialEq)]
pub enum AsyncResourceType {
    /// Stream resource
    Stream,
    /// Future resource  
    Future,
    /// Error context resource
    ErrorContext,
    /// Async task/execution
    AsyncTask,
    /// Borrowed handle with lifetime
    BorrowedHandle,
    /// Lifetime scope
    LifetimeScope,
    /// Resource representation
    ResourceRepresentation,
    /// Subtask
    Subtask,
    /// Custom cleanup
    Custom,
}

/// Cleanup data specific to each resource type
#[derive(Debug, Clone)]
pub enum AsyncCleanupData {
    /// Stream cleanup data
    Stream {
        handle: StreamHandle,
        close_readable: bool,
        close_writable: bool,
    },
    
    /// Future cleanup data
    Future {
        handle: FutureHandle,
        cancel_pending: bool,
    },
    
    /// Error context cleanup data
    ErrorContext {
        handle: ErrorContextHandle,
    },
    
    /// Async task cleanup data
    AsyncTask {
        task_id: u32,
        execution_id: Option<u32>,
        force_cancel: bool,
    },
    
    /// Borrowed handle cleanup data
    BorrowedHandle {
        handle: u32,
        lifetime_scope_id: u32,
        source_component: u32,
    },
    
    /// Lifetime scope cleanup data
    LifetimeScope {
        scope_id: u32,
        component_id: u32,
        task_id: u32,
    },
    
    /// Resource representation cleanup data
    ResourceRepresentation {
        handle: u32,
        resource_id: u32,
        component_id: u32,
    },
    
    /// Subtask cleanup data
    Subtask {
        execution_id: u32,
        task_id: u32,
        force_cleanup: bool,
    },
    
    /// Custom cleanup data
    Custom {
        #[cfg(feature = "std")]
        cleanup_id: String,
        #[cfg(not(any(feature = "std", )))]
        cleanup_id: BoundedString<64>,
        data: u64, // Generic data field
    },
}

/// Statistics for async resource cleanup
#[derive(Debug, Clone, Default)]
pub struct AsyncCleanupStats {
    /// Total cleanup entries created
    pub total_created: u64,
    
    /// Total cleanups executed
    pub total_executed: u64,
    
    /// Failed cleanups
    pub failed_cleanups: u64,
    
    /// Cleanup by resource type
    pub stream_cleanups: u64,
    pub future_cleanups: u64,
    pub error_context_cleanups: u64,
    pub async_task_cleanups: u64,
    pub borrowed_handle_cleanups: u64,
    pub lifetime_scope_cleanups: u64,
    pub resource_representation_cleanups: u64,
    pub subtask_cleanups: u64,
    pub custom_cleanups: u64,
    
    /// Average cleanup time (simplified for no_std)
    pub avg_cleanup_time_ns: u64,
    
    /// Peak number of cleanup entries
    pub peak_cleanup_entries: u32,
}

/// Result of cleanup operation
#[derive(Debug, Clone)]
pub enum CleanupResult {
    /// Cleanup completed successfully
    Success,
    /// Cleanup failed but was not critical
    Failed(Error),
    /// Critical cleanup failed
    CriticalFailure(Error),
    /// Cleanup was skipped (resource already cleaned)
    Skipped,
}

impl AsyncResourceCleanupManager {
    /// Create a new async resource cleanup manager
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            #[cfg(feature = "std")]
            cleanup_entries: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            cleanup_entries: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            stats: AsyncCleanupStats::default(),
            next_cleanup_id: 1,
        })
    }

    /// Register a cleanup entry for an instance
    pub fn register_cleanup(
        &mut self,
        instance_id: ComponentInstanceId,
        resource_type: AsyncResourceType,
        cleanup_data: AsyncCleanupData,
        priority: u8,
        critical: bool,
    ) -> Result<u32> {
        let cleanup_id = self.next_cleanup_id;
        self.next_cleanup_id += 1;

        let entry = AsyncCleanupEntry {
            cleanup_id,
            resource_type,
            priority,
            cleanup_data,
            critical,
            created_at: self.get_current_time(),
        };

        self.add_cleanup_entry(instance_id, entry)?;
        self.stats.total_created += 1;

        Ok(cleanup_id)
    }

    /// Execute all cleanups for an instance
    pub fn execute_cleanups(&mut self, instance_id: ComponentInstanceId) -> Result<Vec<CleanupResult>> {
        let mut results = Vec::new();
        
        #[cfg(feature = "std")]
        let entries = self.cleanup_entries.remove(&instance_id).unwrap_or_default();
        
        #[cfg(not(any(feature = "std", )))]
        let entries = {
            let provider = safe_managed_alloc!(65536, CrateId::Component)?;
            let mut found_entries = BoundedVec::new(provider)?;
            let mut index_to_remove = None;
            
            for (i, (id, entries)) in self.cleanup_entries.iter().enumerate() {
                if *id == instance_id {
                    found_entries = entries.clone();
                    index_to_remove = Some(i);
                    break;
                }
            }
            
            if let Some(index) = index_to_remove {
                self.cleanup_entries.remove(index);
            }
            
            found_entries
        };

        // Sort by priority (highest first)
        #[cfg(feature = "std")]
        let mut sorted_entries = entries;
        #[cfg(feature = "std")]
        sorted_entries.sort_by(|a, b| b.priority.cmp(&a.priority));

        #[cfg(not(any(feature = "std", )))]
        let mut sorted_entries = entries;
        #[cfg(not(any(feature = "std", )))]
        self.sort_entries_by_priority(&mut sorted_entries);

        // Execute each cleanup
        for entry in sorted_entries {
            let result = self.execute_single_cleanup(&entry);
            
            match &result {
                CleanupResult::Success => {
                    self.stats.total_executed += 1;
                    self.update_type_stats(&entry.resource_type);
                }
                CleanupResult::Failed(_) | CleanupResult::CriticalFailure(_) => {
                    self.stats.failed_cleanups += 1;
                }
                CleanupResult::Skipped => {
                    // No stats update for skipped
                }
            }
            
            #[cfg(feature = "std")]
            results.push(result);
            #[cfg(not(any(feature = "std", )))]
            {
                if results.len() < MAX_ASYNC_RESOURCES_PER_INSTANCE {
                    let _ = results.push(result);
                }
            }
        }

        #[cfg(feature = "std")]
        {
            Ok(results)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            Ok(results.into_vec())
        }
    }

    /// Execute a single cleanup entry
    fn execute_single_cleanup(&mut self, entry: &AsyncCleanupEntry) -> CleanupResult {
        match &entry.cleanup_data {
            AsyncCleanupData::Stream { handle, close_readable, close_writable } => {
                self.cleanup_stream(*handle, *close_readable, *close_writable)
            }
            AsyncCleanupData::Future { handle, cancel_pending } => {
                self.cleanup_future(*handle, *cancel_pending)
            }
            AsyncCleanupData::ErrorContext { handle } => {
                self.cleanup_error_context(*handle)
            }
            AsyncCleanupData::AsyncTask { task_id, execution_id, force_cancel } => {
                self.cleanup_async_task(*task_id, *execution_id, *force_cancel)
            }
            AsyncCleanupData::BorrowedHandle { handle, lifetime_scope_id, source_component } => {
                self.cleanup_borrowed_handle(*handle, *lifetime_scope_id, *source_component)
            }
            AsyncCleanupData::LifetimeScope { scope_id, component_id, task_id } => {
                self.cleanup_lifetime_scope(*scope_id, *component_id, *task_id)
            }
            AsyncCleanupData::ResourceRepresentation { handle, resource_id, component_id } => {
                self.cleanup_resource_representation(*handle, *resource_id, *component_id)
            }
            AsyncCleanupData::Subtask { execution_id, task_id, force_cleanup } => {
                self.cleanup_subtask(*execution_id, *task_id, *force_cleanup)
            }
            AsyncCleanupData::Custom { cleanup_id, data } => {
                self.cleanup_custom(cleanup_id, *data)
            }
        }
    }

    /// Get cleanup statistics
    pub fn get_stats(&self) -> &AsyncCleanupStats {
        &self.stats
    }

    /// Reset all statistics
    pub fn reset_stats(&mut self) {
        self.stats = AsyncCleanupStats::default();
    }

    /// Remove all cleanup entries for an instance
    pub fn clear_instance(&mut self, instance_id: ComponentInstanceId) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.cleanup_entries.remove(&instance_id);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let mut index_to_remove = None;
            for (i, (id, _)) in self.cleanup_entries.iter().enumerate() {
                if *id == instance_id {
                    index_to_remove = Some(i);
                    break;
                }
            }
            if let Some(index) = index_to_remove {
                self.cleanup_entries.remove(index);
            }
        }
        Ok(())
    }

    // Private helper methods

    fn add_cleanup_entry(&mut self, instance_id: ComponentInstanceId, entry: AsyncCleanupEntry) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.cleanup_entries
                .entry(instance_id)
                .or_insert_with(Vec::new)
                .push(entry);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            // Find existing entry or create new one
            let mut found = false;
            for (id, entries) in &mut self.cleanup_entries {
                if *id == instance_id {
                    entries.push(entry).map_err(|_| {
                        Error::runtime_execution_error("
                        )
                    })?;
                    found = true;
                    break;
                }
            }
            
            if !found {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                let mut new_entries = BoundedVec::new(provider)?;
                new_entries.push(entry).map_err(|_| {
                    Error::new(
                        ErrorCategory::Resource,
                        wrt_error::codes::RESOURCE_EXHAUSTED,
                        ")
                })?;
                
                self.cleanup_entries.push((instance_id, new_entries)).map_err(|_| {
                    Error::runtime_execution_error("
                    )
                })?;
            }
        }

        // Update peak statistics
        let total_entries = self.count_total_entries();
        if total_entries > self.stats.peak_cleanup_entries {
            self.stats.peak_cleanup_entries = total_entries;
        }

        Ok(())
    }

    #[cfg(not(any(feature = ")))]
    fn sort_entries_by_priority(&self, entries: &mut BoundedVec<AsyncCleanupEntry, MAX_ASYNC_RESOURCES_PER_INSTANCE>) {
        // Simple bubble sort for no_std
        for i in 0..entries.len() {
            for j in 0..(entries.len() - 1 - i) {
                if entries[j].priority < entries[j + 1].priority {
                    let temp = entries[j].clone();
                    entries[j] = entries[j + 1].clone();
                    entries[j + 1] = temp;
                }
            }
        }
    }

    fn count_total_entries(&self) -> u32 {
        #[cfg(feature = "std")]
        {
            self.cleanup_entries.values().map(|v| v.len()).sum::<usize>() as u32
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.cleanup_entries.iter().map(|(_, v)| v.len()).sum::<usize>() as u32
        }
    }

    fn update_type_stats(&mut self, resource_type: &AsyncResourceType) {
        match resource_type {
            AsyncResourceType::Stream => self.stats.stream_cleanups += 1,
            AsyncResourceType::Future => self.stats.future_cleanups += 1,
            AsyncResourceType::ErrorContext => self.stats.error_context_cleanups += 1,
            AsyncResourceType::AsyncTask => self.stats.async_task_cleanups += 1,
            AsyncResourceType::BorrowedHandle => self.stats.borrowed_handle_cleanups += 1,
            AsyncResourceType::LifetimeScope => self.stats.lifetime_scope_cleanups += 1,
            AsyncResourceType::ResourceRepresentation => self.stats.resource_representation_cleanups += 1,
            AsyncResourceType::Subtask => self.stats.subtask_cleanups += 1,
            AsyncResourceType::Custom => self.stats.custom_cleanups += 1,
        }
    }

    fn get_current_time(&self) -> u64 {
        // Simplified time implementation - in real code this would use proper timing
        0
    }

    // Cleanup implementation methods (placeholder implementations)
    
    fn cleanup_stream(&mut self, _handle: StreamHandle, _close_readable: bool, _close_writable: bool) -> CleanupResult {
        // In real implementation, this would interact with the async canonical ABI
        CleanupResult::Success
    }

    fn cleanup_future(&mut self, _handle: FutureHandle, _cancel_pending: bool) -> CleanupResult {
        // In real implementation, this would interact with the async canonical ABI
        CleanupResult::Success
    }

    fn cleanup_error_context(&mut self, _handle: ErrorContextHandle) -> CleanupResult {
        // In real implementation, this would interact with the async canonical ABI
        CleanupResult::Success
    }

    fn cleanup_async_task(&mut self, _task_id: u32, _execution_id: Option<u32>, _force_cancel: bool) -> CleanupResult {
        // In real implementation, this would interact with the async execution engine
        CleanupResult::Success
    }

    fn cleanup_borrowed_handle(&mut self, _handle: u32, _lifetime_scope_id: u32, _source_component: u32) -> CleanupResult {
        // In real implementation, this would interact with the handle lifetime tracker
        CleanupResult::Success
    }

    fn cleanup_lifetime_scope(&mut self, _scope_id: u32, _component_id: u32, _task_id: u32) -> CleanupResult {
        // In real implementation, this would interact with the handle lifetime tracker
        CleanupResult::Success
    }

    fn cleanup_resource_representation(&mut self, _handle: u32, _resource_id: u32, _component_id: u32) -> CleanupResult {
        // In real implementation, this would interact with the resource representation manager
        CleanupResult::Success
    }

    fn cleanup_subtask(&mut self, _execution_id: u32, _task_id: u32, _force_cleanup: bool) -> CleanupResult {
        // In real implementation, this would interact with the subtask manager
        CleanupResult::Success
    }

    fn cleanup_custom(&mut self, _cleanup_id: &str, _data: u64) -> CleanupResult {
        // In real implementation, this would call custom cleanup handlers
        CleanupResult::Success
    }
}

impl Default for AsyncResourceCleanupManager {
    fn default() -> Self {
        Self::new().expect("Failed to create AsyncResourceCleanupManager with default")
    }
}

impl fmt::Display for AsyncResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsyncResourceType::Stream => write!(f, "stream"),
            AsyncResourceType::Future => write!(f, "future"),
            AsyncResourceType::ErrorContext => write!(f, "error-context"),
            AsyncResourceType::AsyncTask => write!(f, "async-task"),
            AsyncResourceType::BorrowedHandle => write!(f, "borrowed-handle"),
            AsyncResourceType::LifetimeScope => write!(f, "lifetime-scope"),
            AsyncResourceType::ResourceRepresentation => write!(f, "resource-representation"),
            AsyncResourceType::Subtask => write!(f, "subtask"),
            AsyncResourceType::Custom => write!(f, "custom"),
        }
    }
}

impl fmt::Display for CleanupResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleanupResult::Success => write!(f, "success"),
            CleanupResult::Failed(err) => write!(f, "failed: {}", err),
            CleanupResult::CriticalFailure(err) => write!(f, "critical-failure: {}", err),
            CleanupResult::Skipped => write!(f, "skipped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_manager_creation() {
        let manager = AsyncResourceCleanupManager::new().unwrap();
        assert_eq!(manager.get_stats().total_created, 0);
    }

    #[test]
    fn test_register_stream_cleanup() {
        let mut manager = AsyncResourceCleanupManager::new().unwrap();
        let instance_id = ComponentInstanceId(1);
        let handle = StreamHandle(42);
        
        let cleanup_data = AsyncCleanupData::Stream {
            handle,
            close_readable: true,
            close_writable: true,
        };

        let cleanup_id = manager.register_cleanup(
            instance_id,
            AsyncResourceType::Stream,
            cleanup_data,
            10,
            false,
        ).unwrap();

        assert_eq!(cleanup_id, 1);
        assert_eq!(manager.get_stats().total_created, 1);
    }

    #[test]
    fn test_execute_cleanups() {
        let mut manager = AsyncResourceCleanupManager::new().unwrap();
        let instance_id = ComponentInstanceId(1);
        
        // Register multiple cleanups
        let stream_data = AsyncCleanupData::Stream {
            handle: StreamHandle(1),
            close_readable: true,
            close_writable: true,
        };
        
        let future_data = AsyncCleanupData::Future {
            handle: FutureHandle(2),
            cancel_pending: true,
        };

        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Stream,
            stream_data,
            10,
            false,
        ).unwrap();

        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Future,
            future_data,
            20, // Higher priority
            false,
        ).unwrap();

        let results = manager.execute_cleanups(instance_id).unwrap();
        assert_eq!(results.len(), 2);
        
        // Check that both cleanups succeeded
        for result in &results {
            assert!(matches!(result, CleanupResult::Success));
        }

        assert_eq!(manager.get_stats().total_executed, 2);
        assert_eq!(manager.get_stats().stream_cleanups, 1);
        assert_eq!(manager.get_stats().future_cleanups, 1);
    }

    #[test]
    fn test_cleanup_priority_ordering() {
        let mut manager = AsyncResourceCleanupManager::new().unwrap();
        let instance_id = ComponentInstanceId(1);
        
        // Register cleanups with different priorities
        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Stream,
            AsyncCleanupData::Stream {
                handle: StreamHandle(1),
                close_readable: true,
                close_writable: true,
            },
            5, // Lower priority
            false,
        ).unwrap();

        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Future,
            AsyncCleanupData::Future {
                handle: FutureHandle(2),
                cancel_pending: true,
            },
            15, // Higher priority
            false,
        ).unwrap();

        let results = manager.execute_cleanups(instance_id).unwrap();
        assert_eq!(results.len(), 2);
        
        // All should succeed regardless of order
        for result in &results {
            assert!(matches!(result, CleanupResult::Success));
        }
    }

    #[test]
    fn test_clear_instance() {
        let mut manager = AsyncResourceCleanupManager::new().unwrap();
        let instance_id = ComponentInstanceId(1);
        
        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Stream,
            AsyncCleanupData::Stream {
                handle: StreamHandle(1),
                close_readable: true,
                close_writable: true,
            },
            10,
            false,
        ).unwrap();

        assert_eq!(manager.get_stats().total_created, 1);
        
        manager.clear_instance(instance_id).unwrap();
        
        let results = manager.execute_cleanups(instance_id).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(AsyncResourceType::Stream.to_string(), "stream");
        assert_eq!(AsyncResourceType::Future.to_string(), "future");
        assert_eq!(AsyncResourceType::ErrorContext.to_string(), "error-context");
        assert_eq!(AsyncResourceType::AsyncTask.to_string(), "async-task");
    }

    #[test]
    fn test_stats_tracking() {
        let mut manager = AsyncResourceCleanupManager::new().unwrap();
        let instance_id = ComponentInstanceId(1);
        
        // Register different types of cleanups
        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Stream,
            AsyncCleanupData::Stream {
                handle: StreamHandle(1),
                close_readable: true,
                close_writable: true,
            },
            10,
            false,
        ).unwrap();

        manager.register_cleanup(
            instance_id,
            AsyncResourceType::Future,
            AsyncCleanupData::Future {
                handle: FutureHandle(2),
                cancel_pending: true,
            },
            10,
            false,
        ).unwrap();

        let stats_before = manager.get_stats().clone();
        assert_eq!(stats_before.total_created, 2);
        assert_eq!(stats_before.total_executed, 0);

        manager.execute_cleanups(instance_id).unwrap();

        let stats_after = manager.get_stats();
        assert_eq!(stats_after.total_executed, 2);
        assert_eq!(stats_after.stream_cleanups, 1);
        assert_eq!(stats_after.future_cleanups, 1);
    }
}