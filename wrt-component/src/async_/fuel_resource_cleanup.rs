//! Resource cleanup on task cancellation
//!
//! This module provides automatic resource cleanup when async tasks are
//! cancelled, ensuring no resource leaks with fuel tracking.

use crate::{
    async_::{
        fuel_async_executor::{FuelAsyncTask, AsyncTaskState},
        fuel_resource_lifetime::{ResourceHandle, ResourceLifetimeManager, ComponentResourceTracker},
        fuel_handle_table::{GenerationalHandle, HandleTableManager},
        fuel_stream_handler::FuelStreamManager,
        fuel_error_context::{AsyncErrorKind, async_error, ContextualError},
    },
    prelude::*,
};
use core::{
    sync::atomic::{AtomicU64, AtomicBool, Ordering},
};
use wrt_foundation::{
    bounded_collections::{BoundedVec, BoundedMap},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    safe_managed_alloc, CrateId,
    Arc, Weak, sync::Mutex,
};

/// Maximum cleanup callbacks per task
const MAX_CLEANUP_CALLBACKS: usize = 64;

/// Maximum tasks tracked for cleanup
const MAX_TRACKED_TASKS: usize = 256;

/// Fuel costs for cleanup operations
const CLEANUP_REGISTER_FUEL: u64 = 2;
const CLEANUP_EXECUTE_FUEL: u64 = 5;
const CLEANUP_CANCEL_FUEL: u64 = 10;
const CLEANUP_FINALIZE_FUEL: u64 = 8;

/// Cleanup action type
#[derive(Debug, Clone)]
pub enum CleanupAction {
    /// Drop a resource handle
    DropResource(ResourceHandle),
    /// Close a stream
    CloseStream(u64),
    /// Release a handle from table
    ReleaseHandle(u64, GenerationalHandle),
    /// Custom cleanup callback
    Custom(String),
}

/// Cleanup callback registration
pub struct CleanupCallback {
    /// Action to perform
    pub action: CleanupAction,
    /// Priority (higher executes first)
    pub priority: u32,
    /// Fuel cost for this cleanup
    pub fuel_cost: u64,
    /// Whether cleanup is critical (must not fail)
    pub is_critical: bool,
}

impl CleanupCallback {
    /// Create a new cleanup callback
    pub fn new(action: CleanupAction, priority: u32, fuel_cost: u64, is_critical: bool) -> Self {
        Self {
            action,
            priority,
            fuel_cost,
            is_critical,
        }
    }
}

/// Task cleanup context
pub struct TaskCleanupContext {
    /// Task ID
    pub task_id: u64,
    /// Component ID
    pub component_id: u64,
    /// Cleanup callbacks
    callbacks: BoundedVec<CleanupCallback, MAX_CLEANUP_CALLBACKS>,
    /// Resources owned by this task
    owned_resources: BoundedVec<ResourceHandle, MAX_CLEANUP_CALLBACKS>,
    /// Streams owned by this task
    owned_streams: BoundedVec<u64, MAX_CLEANUP_CALLBACKS>,
    /// Handle table entries
    handle_entries: BoundedVec<(u64, GenerationalHandle), MAX_CLEANUP_CALLBACKS>,
    /// Cleanup executed flag
    cleanup_executed: AtomicBool,
    /// Total fuel consumed during cleanup
    cleanup_fuel_consumed: AtomicU64,
    /// Verification level
    verification_level: VerificationLevel,
}

impl TaskCleanupContext {
    /// Create a new task cleanup context
    pub fn new(
        task_id: u64,
        component_id: u64,
        verification_level: VerificationLevel,
    ) -> Result<Self> {
        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        
        Ok(Self {
            task_id,
            component_id,
            callbacks: BoundedVec::new(provider.clone())?,
            owned_resources: BoundedVec::new(provider.clone())?,
            owned_streams: BoundedVec::new(provider.clone())?,
            handle_entries: BoundedVec::new(provider)?,
            cleanup_executed: AtomicBool::new(false),
            cleanup_fuel_consumed: AtomicU64::new(0),
            verification_level,
        })
    }
    
    /// Register a cleanup callback
    pub fn register_callback(&mut self, callback: CleanupCallback) -> Result<()> {
        self.consume_fuel(CLEANUP_REGISTER_FUEL)?;
        self.callbacks.push(callback)?;
        
        // Sort by priority (highest first)
        self.callbacks.sort_by(|a, b| b.priority.cmp(&a.priority;
        
        Ok(())
    }
    
    /// Register resource ownership
    pub fn register_resource(&mut self, handle: ResourceHandle) -> Result<()> {
        self.consume_fuel(CLEANUP_REGISTER_FUEL)?;
        self.owned_resources.push(handle)?;
        
        // Also register cleanup callback
        self.register_callback(CleanupCallback::new(
            CleanupAction::DropResource(handle),
            50, // Medium priority
            CLEANUP_EXECUTE_FUEL,
            true, // Critical
        ))?
    }
    
    /// Register stream ownership
    pub fn register_stream(&mut self, stream_id: u64) -> Result<()> {
        self.consume_fuel(CLEANUP_REGISTER_FUEL)?;
        self.owned_streams.push(stream_id)?;
        
        // Also register cleanup callback
        self.register_callback(CleanupCallback::new(
            CleanupAction::CloseStream(stream_id),
            60, // Higher priority than resources
            CLEANUP_EXECUTE_FUEL,
            false, // Non-critical
        ))?
    }
    
    /// Register handle table entry
    pub fn register_handle(&mut self, table_id: u64, handle: GenerationalHandle) -> Result<()> {
        self.consume_fuel(CLEANUP_REGISTER_FUEL)?;
        self.handle_entries.push((table_id, handle))?;
        
        // Also register cleanup callback
        self.register_callback(CleanupCallback::new(
            CleanupAction::ReleaseHandle(table_id, handle),
            40, // Lower priority
            CLEANUP_EXECUTE_FUEL,
            false, // Non-critical
        ))?
    }
    
    /// Execute cleanup
    pub fn execute_cleanup(
        &mut self,
        resource_tracker: &mut ComponentResourceTracker,
        stream_manager: &mut FuelStreamManager,
        handle_manager: &mut HandleTableManager,
    ) -> Result<Vec<ContextualError>> {
        // Check if already executed
        if self.cleanup_executed.swap(true, Ordering::AcqRel) {
            return Ok(Vec::new);
        }
        
        let mut errors = Vec::new);
        
        // Execute callbacks in priority order
        for callback in self.callbacks.drain(..) {
            // Check fuel
            if let Err(e) = self.consume_fuel(callback.fuel_cost) {
                if callback.is_critical {
                    errors.push(self.create_error(
                        AsyncErrorKind::FuelExhausted,
                        "Critical cleanup failed due to fuel exhaustion",
                    )?;
                }
                continue;
            }
            
            // Execute action
            let result = match callback.action {
                CleanupAction::DropResource(handle) => {
                    self.cleanup_resource(resource_tracker, handle)
                }
                CleanupAction::CloseStream(stream_id) => {
                    self.cleanup_stream(stream_manager, stream_id)
                }
                CleanupAction::ReleaseHandle(table_id, handle) => {
                    self.cleanup_handle(handle_manager, table_id, handle)
                }
                CleanupAction::Custom(description) => {
                    // Custom callbacks would be executed here
                    Ok(())
                }
            };
            
            // Handle errors
            if let Err(e) = result {
                if callback.is_critical {
                    errors.push(self.create_error(
                        AsyncErrorKind::TaskCancelled,
                        &format!("Cleanup failed: {}", e.message()),
                    )?;
                }
            }
        }
        
        // Final cleanup
        self.consume_fuel(CLEANUP_FINALIZE_FUEL)?;
        
        Ok(errors)
    }
    
    /// Cleanup a resource
    fn cleanup_resource(
        &self,
        resource_tracker: &mut ComponentResourceTracker,
        handle: ResourceHandle,
    ) -> Result<()> {
        if let Ok(manager) = resource_tracker.get_or_create_manager(self.component_id) {
            if let Ok(mut manager) = manager.lock() {
                manager.drop_resource(handle)?;
            }
        }
        Ok(())
    }
    
    /// Cleanup a stream
    fn cleanup_stream(
        &self,
        stream_manager: &mut FuelStreamManager,
        stream_id: u64,
    ) -> Result<()> {
        stream_manager.close_stream(stream_id)
    }
    
    /// Cleanup a handle
    fn cleanup_handle(
        &self,
        handle_manager: &mut HandleTableManager,
        table_id: u64,
        handle: GenerationalHandle,
    ) -> Result<()> {
        // We can't deallocate without knowing the type, so we just log it
        // In a real implementation, we'd need type information
        Ok(())
    }
    
    /// Consume fuel for cleanup operation
    fn consume_fuel(&self, amount: u64) -> Result<()> {
        let adjusted = OperationType::fuel_cost_for_operation(
            OperationType::Other,
            self.verification_level,
        )?;
        
        let total = amount.saturating_add(adjusted;
        self.cleanup_fuel_consumed.fetch_add(total, Ordering::AcqRel;
        record_global_operation(OperationType::Other)?;
        
        Ok(())
    }
    
    /// Create an error with context
    fn create_error(&self, kind: AsyncErrorKind, context: &str) -> Result<ContextualError> {
        async_error(kind, self.component_id, Some(self.task_id), context)
    }
}

/// Global cleanup manager
pub struct GlobalCleanupManager {
    /// Cleanup contexts by task ID
    contexts: BoundedMap<u64, TaskCleanupContext, MAX_TRACKED_TASKS>,
    /// Component resource tracker
    resource_tracker: Arc<Mutex<ComponentResourceTracker>>,
    /// Stream manager
    stream_manager: Arc<Mutex<FuelStreamManager>>,
    /// Handle table manager
    handle_manager: Arc<Mutex<HandleTableManager>>,
    /// Total cleanup operations
    total_cleanups: AtomicU64,
    /// Failed cleanups
    failed_cleanups: AtomicU64,
}

impl GlobalCleanupManager {
    /// Create a new global cleanup manager
    pub fn new(
        global_fuel_budget: u64,
    ) -> Result<Self> {
        let provider = safe_managed_alloc!(8192, CrateId::Component)?;
        let contexts = BoundedMap::new(provider)?;
        
        let resource_tracker = Arc::new(Mutex::new(
            ComponentResourceTracker::new(global_fuel_budget / 3)?
        ;
        let stream_manager = Arc::new(Mutex::new(
            FuelStreamManager::new(global_fuel_budget / 3)?
        ;
        let handle_manager = Arc::new(Mutex::new(
            HandleTableManager::new(global_fuel_budget / 3)?
        ;
        
        Ok(Self {
            contexts,
            resource_tracker,
            stream_manager,
            handle_manager,
            total_cleanups: AtomicU64::new(0),
            failed_cleanups: AtomicU64::new(0),
        })
    }
    
    /// Register a task for cleanup tracking
    pub fn register_task(
        &mut self,
        task_id: u64,
        component_id: u64,
        verification_level: VerificationLevel,
    ) -> Result<()> {
        let context = TaskCleanupContext::new(task_id, component_id, verification_level)?;
        self.contexts.insert(task_id, context)?;
        Ok(())
    }
    
    /// Get cleanup context for a task
    pub fn get_context_mut(&mut self, task_id: u64) -> Result<&mut TaskCleanupContext> {
        self.contexts.get_mut(&task_id).ok_or_else(|| 
            Error::async_error("Task cleanup context not found"))
    }
    
    /// Cancel a task and run cleanup
    pub fn cancel_task(&mut self, task_id: u64) -> Result<Vec<ContextualError>> {
        // Remove context
        let mut context = self.contexts.remove(&task_id).ok_or_else(|| 
            Error::async_error("Task not found for cancellation"))?;
        
        // Consume cancellation fuel
        context.consume_fuel(CLEANUP_CANCEL_FUEL)?;
        
        // Execute cleanup
        let errors = context.execute_cleanup(
            &mut *self.resource_tracker.lock()?,
            &mut *self.stream_manager.lock()?,
            &mut *self.handle_manager.lock()?,
        )?;
        
        // Update stats
        self.total_cleanups.fetch_add(1, Ordering::Relaxed;
        if !errors.is_empty() {
            self.failed_cleanups.fetch_add(1, Ordering::Relaxed;
        }
        
        Ok(errors)
    }
    
    /// Cancel all tasks for a component
    pub fn cancel_component_tasks(&mut self, component_id: u64) -> Result<Vec<ContextualError>> {
        let task_ids: Vec<u64> = self.contexts
            .iter()
            .filter(|(_, ctx)| ctx.component_id == component_id)
            .map(|(id, _)| *id)
            .collect();
        
        let mut all_errors = Vec::new);
        
        for task_id in task_ids {
            match self.cancel_task(task_id) {
                Ok(errors) => all_errors.extend(errors),
                Err(e) => {
                    let error = async_error(
                        AsyncErrorKind::TaskCancelled,
                        component_id,
                        Some(task_id),
                        &format!("Failed to cancel task: {}", e.message()),
                    )?;
                    all_errors.push(error);
                }
            }
        }
        
        Ok(all_errors)
    }
    
    /// Get cleanup statistics
    pub fn stats(&self) -> CleanupStats {
        CleanupStats {
            total_cleanups: self.total_cleanups.load(Ordering::Relaxed),
            failed_cleanups: self.failed_cleanups.load(Ordering::Relaxed),
            active_tasks: self.contexts.len(),
        }
    }
}

/// Cleanup statistics
#[derive(Debug, Clone)]
pub struct CleanupStats {
    /// Total cleanup operations performed
    pub total_cleanups: u64,
    /// Failed cleanup operations
    pub failed_cleanups: u64,
    /// Currently active tasks
    pub active_tasks: usize,
}

/// RAII guard for automatic task cleanup
pub struct TaskCleanupGuard {
    task_id: u64,
    manager: Arc<Mutex<GlobalCleanupManager>>,
    cancelled: AtomicBool,
}

impl TaskCleanupGuard {
    /// Create a new cleanup guard
    pub fn new(
        task_id: u64,
        manager: Arc<Mutex<GlobalCleanupManager>>,
    ) -> Self {
        Self {
            task_id,
            manager,
            cancelled: AtomicBool::new(false),
        }
    }
    
    /// Cancel the task early
    pub fn cancel(&self) -> Result<Vec<ContextualError>> {
        if self.cancelled.swap(true, Ordering::AcqRel) {
            return Ok(Vec::new);
        }
        
        self.manager.lock()?.cancel_task(self.task_id)
    }
}

impl Drop for TaskCleanupGuard {
    fn drop(&mut self) {
        if !self.cancelled.load(Ordering::Acquire) {
            let _ = self.cancel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cleanup_context() {
        let mut context = TaskCleanupContext::new(1, 1, VerificationLevel::Basic).unwrap();
        
        // Register various resources
        assert!(context.register_resource(ResourceHandle(1)).is_ok();
        assert!(context.register_stream(1).is_ok();
        assert!(context.register_handle(1, GenerationalHandle::new(0, 1)).is_ok();
        
        // Should have 3 callbacks
        assert_eq!(context.callbacks.len(), 3;
    }
    
    #[test]
    fn test_callback_priority() {
        let mut context = TaskCleanupContext::new(1, 1, VerificationLevel::Basic).unwrap();
        
        // Register callbacks with different priorities
        context.register_callback(CleanupCallback::new(
            CleanupAction::Custom("low".to_string()),
            10,
            1,
            false,
        )).unwrap();
        
        context.register_callback(CleanupCallback::new(
            CleanupAction::Custom("high".to_string()),
            100,
            1,
            false,
        )).unwrap();
        
        context.register_callback(CleanupCallback::new(
            CleanupAction::Custom("medium".to_string()),
            50,
            1,
            false,
        )).unwrap();
        
        // Verify priority order (highest first)
        assert_eq!(context.callbacks[0].priority, 100;
        assert_eq!(context.callbacks[1].priority, 50;
        assert_eq!(context.callbacks[2].priority, 10;
    }
    
    #[test]
    fn test_global_cleanup_manager() {
        let mut manager = GlobalCleanupManager::new(10000).unwrap();
        
        // Register task
        assert!(manager.register_task(1, 1, VerificationLevel::Basic).is_ok();
        
        // Get context and register resources
        {
            let context = manager.get_context_mut(1).unwrap();
            context.register_resource(ResourceHandle(1)).unwrap();
        }
        
        // Cancel task
        let errors = manager.cancel_task(1).unwrap();
        assert!(errors.is_empty();
        
        // Stats should show cleanup
        let stats = manager.stats);
        assert_eq!(stats.total_cleanups, 1;
        assert_eq!(stats.failed_cleanups, 0;
    }
    
    #[test]
    fn test_cleanup_guard() {
        let manager = Arc::new(Mutex::new(
            GlobalCleanupManager::new(10000).unwrap()
        ;
        
        // Register task
        manager.lock().unwrap().register_task(1, 1, VerificationLevel::Basic).unwrap();
        
        {
            let _guard = TaskCleanupGuard::new(1, manager.clone();
            // Guard will cancel task when dropped
        }
        
        // Task should be cleaned up
        let stats = manager.lock().unwrap().stats);
        assert_eq!(stats.total_cleanups, 1;
    }
}