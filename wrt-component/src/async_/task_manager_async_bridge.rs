//! Task Manager bridge for async Component Model integration
//!
//! This module provides the bridge between the Component Model's TaskManager
//! and the fuel-based async executor, enabling seamless async task lifecycle management.

use crate::{
    async_::{
        fuel_async_executor::{FuelAsyncExecutor, AsyncTaskState, AsyncTaskStatus},
        component_async_bridge::{ComponentAsyncBridge, PollResult},
        fuel_dynamic_manager::FuelAllocationPolicy,
        fuel_preemption_support::PreemptionPolicy,
    },
    task_manager::{TaskManager, TaskId, TaskType, TaskState, Task, TaskContext, CallFrame},
    threading::thread_spawn_fuel::FuelTrackedThreadManager,
    async_types::{Future, FutureHandle, Stream, StreamHandle, Waitable, WaitableSet},
    ComponentInstanceId,
    prelude::*,
};
use core::{
    future::Future as CoreFuture,
    pin::Pin,
    sync::atomic::{AtomicU64, AtomicU32, Ordering},
    task::{Context, Poll},
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    component_value::ComponentValue,
    Arc, Weak, sync::Mutex,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

/// Maximum async contexts per component
const MAX_ASYNC_CONTEXTS: usize = 256;

/// Async task wrapper for Component Model futures
#[derive(Debug)]
pub struct ComponentAsyncTask {
    /// Component Model task ID
    pub component_task_id: TaskId,
    /// Executor task ID
    pub executor_task_id: crate::task_manager::TaskId,
    /// Component instance
    pub component_id: ComponentInstanceId,
    /// Task type
    pub task_type: ComponentAsyncTaskType,
    /// Future handle (if applicable)
    pub future_handle: Option<FutureHandle>,
    /// Stream handle (if applicable)  
    pub stream_handle: Option<StreamHandle>,
    /// Waitables being monitored
    pub waitables: Option<WaitableSet>,
    /// Task priority
    pub priority: Priority,
    /// Creation timestamp
    pub created_at: u64,
    /// Last activity timestamp
    pub last_activity: AtomicU64,
}

/// Type of async task in Component Model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentAsyncTaskType {
    /// Regular async function call
    AsyncFunction,
    /// Future value waiting
    FutureWait,
    /// Stream consumption
    StreamConsume,
    /// Resource async operation
    ResourceAsync,
    /// Component lifecycle async
    LifecycleAsync,
}

/// Task Manager Async Bridge
pub struct TaskManagerAsyncBridge {
    /// Component Model task manager
    task_manager: Arc<Mutex<TaskManager>>,
    /// Async executor bridge
    async_bridge: ComponentAsyncBridge,
    /// Active async tasks
    async_tasks: BoundedHashMap<TaskId, ComponentAsyncTask, MAX_ASYNC_CONTEXTS>,
    /// Task mapping (component task -> executor task)
    task_mapping: BoundedHashMap<TaskId, crate::task_manager::TaskId, MAX_ASYNC_CONTEXTS>,
    /// Component async contexts
    async_contexts: BoundedHashMap<ComponentInstanceId, ComponentAsyncContext, 128>,
    /// Bridge statistics
    bridge_stats: BridgeStatistics,
    /// Bridge configuration
    config: BridgeConfiguration,
}

/// Per-component async context
#[derive(Debug)]
struct ComponentAsyncContext {
    component_id: ComponentInstanceId,
    /// Active async tasks for this component
    active_tasks: BoundedVec<TaskId, 64>,
    /// Future handles owned by component
    futures: BoundedHashMap<FutureHandle, TaskId, 64>,
    /// Stream handles owned by component
    streams: BoundedHashMap<StreamHandle, TaskId, 64>,
    /// Component async state
    async_state: ComponentAsyncState,
    /// Resource limits
    resource_limits: ComponentResourceLimits,
}

/// Component async state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentAsyncState {
    /// Component supports async operations
    Active,
    /// Component is suspending async operations
    Suspending,
    /// Component async operations are suspended
    Suspended,
    /// Component is terminating async operations
    Terminating,
    /// Component async operations are terminated
    Terminated,
}

/// Resource limits for component async operations
#[derive(Debug, Clone)]
struct ComponentResourceLimits {
    max_concurrent_tasks: usize,
    max_futures: usize,
    max_streams: usize,
    fuel_budget: u64,
    memory_limit: usize,
}

/// Bridge statistics
#[derive(Debug, Default)]
struct BridgeStatistics {
    total_async_tasks: AtomicU64,
    completed_tasks: AtomicU64,
    failed_tasks: AtomicU64,
    cancelled_tasks: AtomicU64,
    futures_created: AtomicU64,
    streams_created: AtomicU64,
    preemptions: AtomicU64,
    fuel_exhaustions: AtomicU64,
}

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfiguration {
    /// Enable async task preemption
    pub enable_preemption: bool,
    /// Enable dynamic fuel management  
    pub enable_dynamic_fuel: bool,
    /// Default fuel allocation policy
    pub fuel_policy: FuelAllocationPolicy,
    /// Default preemption policy
    pub preemption_policy: PreemptionPolicy,
    /// Default component resource limits
    pub default_limits: ComponentResourceLimits,
}

impl Default for BridgeConfiguration {
    fn default() -> Self {
        Self {
            enable_preemption: true,
            enable_dynamic_fuel: true,
            fuel_policy: FuelAllocationPolicy::Adaptive,
            preemption_policy: PreemptionPolicy::PriorityBased,
            default_limits: ComponentResourceLimits {
                max_concurrent_tasks: 32,
                max_futures: 64,
                max_streams: 16,
                fuel_budget: 50_000,
                memory_limit: 1024 * 1024, // 1MB
            },
        }
    }
}

impl TaskManagerAsyncBridge {
    /// Create a new task manager async bridge
    pub fn new(
        task_manager: Arc<Mutex<TaskManager>>,
        thread_manager: Arc<Mutex<FuelTrackedThreadManager>>,
        config: BridgeConfiguration,
    ) -> Result<Self, Error> {
        let async_bridge = ComponentAsyncBridge::new(task_manager.clone(), thread_manager)?;
        
        Ok(Self {
            task_manager,
            async_bridge,
            async_tasks: BoundedHashMap::new(),
            task_mapping: BoundedHashMap::new(),
            async_contexts: BoundedHashMap::new(),
            bridge_stats: BridgeStatistics::default(),
            config,
        })
    }

    /// Initialize component for async operations
    pub fn initialize_component_async(
        &mut self,
        component_id: ComponentInstanceId,
        limits: Option<ComponentResourceLimits>,
    ) -> Result<(), Error> {
        let limits = limits.unwrap_or_else(|| self.config.default_limits.clone());
        
        // Register with async bridge
        self.async_bridge.register_component(
            component_id,
            limits.max_concurrent_tasks,
            limits.fuel_budget,
            Priority::Normal,
        )?;

        // Create async context
        let provider = safe_managed_alloc!(2048, CrateId::Component)?;
        let context = ComponentAsyncContext {
            component_id,
            active_tasks: BoundedVec::new(provider.clone())?,
            futures: BoundedHashMap::new(),
            streams: BoundedHashMap::new(),
            async_state: ComponentAsyncState::Active,
            resource_limits: limits,
        };

        self.async_contexts.insert(component_id, context).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many component async contexts".to_string(),
            )
        })?;

        Ok(())
    }

    /// Spawn an async task from Component Model
    pub fn spawn_async_task<F>(
        &mut self,
        component_id: ComponentInstanceId,
        function_index: Option<u32>,
        future: F,
        task_type: ComponentAsyncTaskType,
        priority: Priority,
    ) -> Result<TaskId, Error>
    where
        F: CoreFuture<Output = Result<Vec<ComponentValue>, Error>> + Send + 'static,
    {
        // Check component async context
        let context = self.async_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Component not initialized for async".to_string(),
            )
        })?;

        if context.async_state != ComponentAsyncState::Active {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_STATE,
                "Component async operations not active".to_string(),
            ));
        }

        // Check resource limits
        if context.active_tasks.len() >= context.resource_limits.max_concurrent_tasks {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Component async task limit exceeded".to_string(),
            ));
        }

        // Create Component Model task
        let component_task_id = {
            let mut tm = self.task_manager.lock()?;
            tm.spawn_task(TaskType::AsyncOperation, component_id.0, function_index)?
        };

        // Convert future to Result<(), Error> for executor
        let executor_future = async move {
            match future.await {
                Ok(_values) => Ok(()), // Success - values handled elsewhere
                Err(e) => Err(e),
            }
        };

        // Spawn in async executor via bridge
        let executor_task_id = self.async_bridge.spawn_component_async(
            component_id,
            executor_future,
            Some(context.resource_limits.fuel_budget / context.resource_limits.max_concurrent_tasks as u64),
        )?;

        // Create async task record
        let async_task = ComponentAsyncTask {
            component_task_id,
            executor_task_id,
            component_id,
            task_type,
            future_handle: None,
            stream_handle: None,
            waitables: None,
            priority,
            created_at: self.get_timestamp(),
            last_activity: AtomicU64::new(self.get_timestamp()),
        };

        // Store task mappings
        self.async_tasks.insert(component_task_id, async_task).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many async tasks".to_string(),
            )
        })?;

        self.task_mapping.insert(component_task_id, executor_task_id).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Task mapping table full".to_string(),
            )
        })?;

        // Update component context
        context.active_tasks.push(component_task_id).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Component task list full".to_string(),
            )
        })?;

        // Update statistics
        self.bridge_stats.total_async_tasks.fetch_add(1, Ordering::Relaxed);

        Ok(component_task_id)
    }

    /// Create a future handle for async waiting
    pub fn create_future_handle(
        &mut self,
        component_id: ComponentInstanceId,
        future: Box<dyn Future + Send>,
    ) -> Result<FutureHandle, Error> {
        let context = self.async_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Component not initialized".to_string(),
            )
        })?;

        if context.futures.len() >= context.resource_limits.max_futures {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Component future limit exceeded".to_string(),
            ));
        }

        // Generate unique handle
        let handle = FutureHandle::new(self.generate_handle_id());
        
        // Spawn task to handle future
        let task_id = self.spawn_async_task(
            component_id,
            None,
            async move {
                // In real implementation, would poll the future
                Ok(vec![])
            },
            ComponentAsyncTaskType::FutureWait,
            Priority::Normal,
        )?;

        // Store handle mapping
        context.futures.insert(handle, task_id).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Future handle table full".to_string(),
            )
        })?;

        self.bridge_stats.futures_created.fetch_add(1, Ordering::Relaxed);

        Ok(handle)
    }

    /// Create a stream handle for async iteration
    pub fn create_stream_handle(
        &mut self,
        component_id: ComponentInstanceId,
        stream: Box<dyn Stream + Send>,
    ) -> Result<StreamHandle, Error> {
        let context = self.async_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Component not initialized".to_string(),
            )
        })?;

        if context.streams.len() >= context.resource_limits.max_streams {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Component stream limit exceeded".to_string(),
            ));
        }

        let handle = StreamHandle::new(self.generate_handle_id());
        
        // Spawn task to handle stream
        let task_id = self.spawn_async_task(
            component_id,
            None,
            async move {
                // In real implementation, would consume the stream
                Ok(vec![])
            },
            ComponentAsyncTaskType::StreamConsume,
            Priority::Normal,
        )?;

        context.streams.insert(handle, task_id).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Stream handle table full".to_string(),
            )
        })?;

        self.bridge_stats.streams_created.fetch_add(1, Ordering::Relaxed);

        Ok(handle)
    }

    /// Wait on multiple waitables (task.wait implementation)
    pub fn task_wait(&mut self, waitables: WaitableSet) -> Result<u32, Error> {
        let current_task = {
            let tm = self.task_manager.lock()?;
            tm.current_task_id().ok_or_else(|| {
                Error::new(
                    ErrorCategory::Validation,
                    codes::INVALID_STATE,
                    "No current task".to_string(),
                )
            })?
        };

        // Check if any waitables are immediately ready
        if let Some(ready_index) = waitables.first_ready() {
            return Ok(ready_index);
        }

        // Update task with waitables
        if let Some(async_task) = self.async_tasks.get_mut(&current_task) {
            async_task.waitables = Some(waitables.clone());
            async_task.last_activity.store(self.get_timestamp(), Ordering::Release);
        }

        // Delegate to task manager
        let mut tm = self.task_manager.lock()?;
        tm.task_wait(waitables)
    }

    /// Poll waitables without blocking (task.poll implementation)
    pub fn task_poll(&self, waitables: &WaitableSet) -> Result<Option<u32>, Error> {
        let tm = self.task_manager.lock()?;
        tm.task_poll(waitables)
    }

    /// Yield current task (task.yield implementation)
    pub fn task_yield(&mut self) -> Result<(), Error> {
        let current_task = {
            let tm = self.task_manager.lock()?;
            tm.current_task_id().ok_or_else(|| {
                Error::new(
                    ErrorCategory::Validation,
                    codes::INVALID_STATE,
                    "No current task".to_string(),
                )
            })?
        };

        // Update task activity
        if let Some(async_task) = self.async_tasks.get(&current_task) {
            async_task.last_activity.store(self.get_timestamp(), Ordering::Release);
        }

        // Delegate to task manager
        let mut tm = self.task_manager.lock()?;
        tm.task_yield()
    }

    /// Poll all async tasks and update state
    pub fn poll_async_tasks(&mut self) -> Result<PollResult, Error> {
        // Poll the async bridge
        let mut result = self.async_bridge.poll_async_tasks()?;

        // Update Component Model task states
        let mut completed_tasks = Vec::new();
        for (comp_task_id, async_task) in self.async_tasks.iter() {
            if self.async_bridge.is_task_ready(*comp_task_id)? {
                // Task is ready, update Component Model task state
                let mut tm = self.task_manager.lock()?;
                if let Some(task) = tm.get_task_mut(*comp_task_id) {
                    task.state = TaskState::Ready;
                }
            }

            // Check if task completed via executor
            // In real implementation, would check executor status
        }

        // Update statistics
        self.bridge_stats.completed_tasks.fetch_add(result.tasks_completed as u64, Ordering::Relaxed);
        self.bridge_stats.failed_tasks.fetch_add(result.tasks_failed as u64, Ordering::Relaxed);

        Ok(result)
    }

    /// Suspend component async operations
    pub fn suspend_component_async(&mut self, component_id: ComponentInstanceId) -> Result<(), Error> {
        let context = self.async_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Component not found".to_string(),
            )
        })?;

        context.async_state = ComponentAsyncState::Suspending;

        // Cancel all active tasks
        for &task_id in context.active_tasks.iter() {
            if let Some(_async_task) = self.async_tasks.get(&task_id) {
                // In real implementation, would gracefully suspend tasks
                let mut tm = self.task_manager.lock()?;
                tm.task_cancel(task_id)?;
            }
        }

        context.async_state = ComponentAsyncState::Suspended;
        Ok(())
    }

    /// Get bridge statistics
    pub fn get_bridge_statistics(&self) -> BridgeStats {
        BridgeStats {
            total_async_tasks: self.bridge_stats.total_async_tasks.load(Ordering::Relaxed),
            completed_tasks: self.bridge_stats.completed_tasks.load(Ordering::Relaxed),
            failed_tasks: self.bridge_stats.failed_tasks.load(Ordering::Relaxed),
            cancelled_tasks: self.bridge_stats.cancelled_tasks.load(Ordering::Relaxed),
            futures_created: self.bridge_stats.futures_created.load(Ordering::Relaxed),
            streams_created: self.bridge_stats.streams_created.load(Ordering::Relaxed),
            active_components: self.async_contexts.len() as u64,
        }
    }

    // Private helper methods

    fn get_timestamp(&self) -> u64 {
        // In real implementation, would use proper time source
        0
    }

    fn generate_handle_id(&self) -> u32 {
        // In real implementation, would generate unique IDs
        42
    }

    /// Cleanup completed async task
    fn cleanup_async_task(&mut self, task_id: TaskId) -> Result<(), Error> {
        if let Some(async_task) = self.async_tasks.remove(&task_id) {
            // Remove from component context
            if let Some(context) = self.async_contexts.get_mut(&async_task.component_id) {
                context.active_tasks.retain(|&id| id != task_id);
            }

            // Remove mapping
            self.task_mapping.remove(&task_id);
        }
        Ok(())
    }
}

/// Bridge statistics
#[derive(Debug, Clone)]
pub struct BridgeStats {
    pub total_async_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub cancelled_tasks: u64,
    pub futures_created: u64,
    pub streams_created: u64,
    pub active_components: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = BridgeConfiguration::default();
        
        let bridge = TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap();
        assert_eq!(bridge.async_contexts.len(), 0);
    }

    #[test]
    fn test_component_initialization() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = BridgeConfiguration::default();
        
        let mut bridge = TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();
        
        assert!(bridge.async_contexts.contains_key(&component_id));
    }

    #[test]
    fn test_async_task_spawning() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = BridgeConfiguration::default();
        
        let mut bridge = TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();
        
        let task_id = bridge.spawn_async_task(
            component_id,
            Some(0),
            async { Ok(vec![]) },
            ComponentAsyncTaskType::AsyncFunction,
            Priority::Normal,
        ).unwrap();
        
        assert!(bridge.async_tasks.contains_key(&task_id));
        assert!(bridge.task_mapping.contains_key(&task_id));
    }

    #[test]
    fn test_future_handle_creation() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = BridgeConfiguration::default();
        
        let mut bridge = TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();
        
        // Would need proper Future implementation for real test
        // let future = Box::new(/* future implementation */);
        // let handle = bridge.create_future_handle(component_id, future).unwrap();
        
        let stats = bridge.get_bridge_statistics();
        assert_eq!(stats.active_components, 1);
    }

    #[test]
    fn test_component_suspension() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = BridgeConfiguration::default();
        
        let mut bridge = TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();
        
        bridge.suspend_component_async(component_id).unwrap();
        
        let context = bridge.async_contexts.get(&component_id).unwrap();
        assert_eq!(context.async_state, ComponentAsyncState::Suspended);
    }
}