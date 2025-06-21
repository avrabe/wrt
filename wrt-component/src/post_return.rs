//! Post-return cleanup functions for WebAssembly Component Model
//!
//! This module implements the post-return cleanup mechanism that allows
//! components to perform cleanup after function calls, particularly for
//! managing resources, memory allocations, and async operations.

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

use wrt_foundation::bounded::{BoundedVec, BoundedString};

use wrt_foundation::prelude::*;

use crate::{
    async_execution_engine::{AsyncExecutionEngine, ExecutionId},
    async_canonical::AsyncCanonicalAbi,
    task_cancellation::{CancellationToken, SubtaskManager, SubtaskResult, SubtaskState},
    borrowed_handles::{HandleLifetimeTracker, LifetimeScope},
    resource_lifecycle_management::{ResourceLifecycleManager, ResourceId, ComponentId},
    resource_representation::ResourceRepresentationManager,
    async_types::{StreamHandle, FutureHandle, ErrorContextHandle},
    canonical_realloc::ReallocManager,
    component_resolver::ComponentValue,
    task_manager::TaskId,
    types::{ComponentError, ComponentInstanceId, TypeId, Value},
};

use wrt_error::{Error, ErrorCategory, Result};

/// Post-return function signature: () -> ()
pub type PostReturnFn = fn();

/// Maximum number of cleanup tasks per instance in no_std
const MAX_CLEANUP_TASKS_NO_STD: usize = 256;

/// Maximum cleanup handlers per type
const MAX_CLEANUP_HANDLERS: usize = 64;

/// Post-return cleanup registry with async support
#[derive(Debug)]
pub struct PostReturnRegistry {
    /// Registered post-return functions per instance
    #[cfg(feature = "std")]
    functions: BTreeMap<ComponentInstanceId, PostReturnFunction>,
    #[cfg(not(any(feature = "std", )))]
    functions: BoundedVec<(ComponentInstanceId, PostReturnFunction), MAX_CLEANUP_TASKS_NO_STD, NoStdProvider<65536>>,
    
    /// Cleanup tasks waiting to be executed
    #[cfg(feature = "std")]
    pending_cleanups: BTreeMap<ComponentInstanceId, Vec<CleanupTask>>,
    #[cfg(not(any(feature = "std", )))]
    pending_cleanups: BoundedVec<(ComponentInstanceId, BoundedVec<CleanupTask, MAX_CLEANUP_TASKS_NO_STD, NoStdProvider<65536>>), MAX_CLEANUP_TASKS_NO_STD>,
    
    /// Async execution engine for async cleanup
    async_engine: Option<Arc<AsyncExecutionEngine>>,
    
    /// Task cancellation manager
    cancellation_manager: Option<Arc<SubtaskManager>>,
    
    /// Handle lifetime tracker for resource cleanup
    handle_tracker: Option<Arc<HandleLifetimeTracker>>,
    
    /// Resource lifecycle manager
    resource_manager: Option<Arc<ResourceLifecycleManager>>,
    
    /// Resource representation manager
    representation_manager: Option<Arc<ResourceRepresentationManager>>,
    
    /// Execution metrics
    metrics: PostReturnMetrics,
    
    /// Maximum cleanup tasks per instance
    max_cleanup_tasks: usize,
}

#[derive(Debug, Clone)]
struct PostReturnFunction {
    /// Function index in the component
    func_index: u32,
    /// Cached function reference for performance
    #[cfg(feature = "std")]
    func_ref: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
    /// Whether the function is currently being executed
    executing: bool,
    /// Associated cancellation token for cleanup
    cancellation_token: Option<CancellationToken>,
}

#[derive(Debug, Clone)]
pub struct CleanupTask {
    /// Type of cleanup task
    pub task_type: CleanupTaskType,
    /// Instance that created this task
    pub source_instance: ComponentInstanceId,
    /// Priority of the task (higher = execute first)
    pub priority: u8,
    /// Task-specific data
    pub data: CleanupData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CleanupTaskType {
    /// Binary std/no_std choice
    DeallocateMemory,
    /// Close resource handle
    CloseResource,
    /// Release reference
    ReleaseReference,
    /// Custom cleanup
    Custom,
    /// Async cleanup (for streams/futures)
    AsyncCleanup,
    /// Cancel async execution
    CancelAsyncExecution,
    /// Drop borrowed handles
    DropBorrowedHandles,
    /// End lifetime scope
    EndLifetimeScope,
    /// Release resource representation
    ReleaseResourceRepresentation,
    /// Finalize subtask
    FinalizeSubtask,
}

#[derive(Debug, Clone)]
pub enum CleanupData {
    /// Binary std/no_std choice
    Memory { ptr: i32, size: i32, align: i32 },
    /// Resource cleanup data
    Resource { handle: u32, resource_type: TypeId },
    /// Reference cleanup data
    Reference { ref_id: u32, ref_count: u32 },
    /// Custom cleanup data
    #[cfg(feature = "std")]
    Custom { cleanup_id: String, parameters: Vec<ComponentValue> },
    #[cfg(not(any(feature = "std", )))]
    Custom { cleanup_id: BoundedString<64, NoStdProvider<65536>>, parameters: BoundedVec<ComponentValue, 16, NoStdProvider<65536>> },
    /// Async cleanup data
    Async { 
        stream_handle: Option<StreamHandle>, 
        future_handle: Option<FutureHandle>, 
        error_context_handle: Option<ErrorContextHandle>,
        task_id: Option<TaskId>,
        execution_id: Option<ExecutionId>,
        cancellation_token: Option<CancellationToken>,
    },
    /// Async execution cancellation
    AsyncExecution {
        execution_id: ExecutionId,
        force_cancel: bool,
    },
    /// Borrowed handle cleanup
    BorrowedHandle {
        borrow_handle: u32,
        lifetime_scope: LifetimeScope,
        source_component: ComponentId,
    },
    /// Lifetime scope cleanup
    LifetimeScope {
        scope: LifetimeScope,
        component: ComponentId,
        task: TaskId,
    },
    /// Resource representation cleanup
    ResourceRepresentation {
        handle: u32,
        resource_id: ResourceId,
        component: ComponentId,
    },
    /// Subtask finalization
    Subtask {
        execution_id: ExecutionId,
        task_id: TaskId,
        result: Option<SubtaskResult>,
        force_cleanup: bool,
    },
}

#[derive(Debug, Default, Clone)]
pub struct PostReturnMetrics {
    /// Total post-return functions executed
    pub total_executions: u64,
    /// Total cleanup tasks processed
    pub total_cleanup_tasks: u64,
    /// Failed cleanup attempts
    pub failed_cleanups: u64,
    /// Average cleanup time (microseconds)
    pub avg_cleanup_time_us: u64,
    /// Peak pending cleanup tasks
    pub peak_pending_tasks: usize,
    /// Async cleanup operations
    pub async_cleanups: u64,
    /// Resource cleanups
    pub resource_cleanups: u64,
    /// Handle cleanups
    pub handle_cleanups: u64,
    /// Cancellation cleanups
    pub cancellation_cleanups: u64,
}

/// Context for post-return execution with async support
pub struct PostReturnContext {
    /// Instance being cleaned up
    pub instance_id: ComponentInstanceId,
    /// Cleanup tasks to execute
    #[cfg(feature = "std")]
    pub tasks: Vec<CleanupTask>,
    #[cfg(not(any(feature = "std", )))]
    pub tasks: BoundedVec<CleanupTask, MAX_CLEANUP_TASKS_NO_STD, NoStdProvider<65536>>,
    /// Binary std/no_std choice
    pub realloc_manager: Option<Arc<ReallocManager>>,
    /// Custom cleanup handlers
    #[cfg(feature = "std")]
    pub custom_handlers: BTreeMap<String, Box<dyn Fn(&CleanupData) -> Result<()> + Send + Sync>>,
    #[cfg(not(any(feature = "std", )))]
    pub custom_handlers: BoundedVec<(BoundedString<64, NoStdProvider<65536>>, fn(&CleanupData) -> core::result::Result<(), NoStdProvider<65536>>), MAX_CLEANUP_HANDLERS>,
    /// Async canonical ABI for async cleanup
    pub async_abi: Option<Arc<AsyncCanonicalAbi>>,
    /// Component ID for this context
    pub component_id: ComponentId,
    /// Current task ID
    pub task_id: TaskId,
}

impl PostReturnRegistry {
    pub fn new(max_cleanup_tasks: usize) -> Self {
        Self {
            #[cfg(feature = "std")]
            functions: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            functions: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            pending_cleanups: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            pending_cleanups: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            async_engine: None,
            cancellation_manager: None,
            handle_tracker: None,
            resource_manager: None,
            representation_manager: None,
            metrics: PostReturnMetrics::default(),
            max_cleanup_tasks,
        }
    }
    
    /// Create new registry with async support
    pub fn new_with_async(
        max_cleanup_tasks: usize,
        async_engine: Option<Arc<AsyncExecutionEngine>>,
        cancellation_manager: Option<Arc<SubtaskManager>>,
        handle_tracker: Option<Arc<HandleLifetimeTracker>>,
        resource_manager: Option<Arc<ResourceLifecycleManager>>,
        representation_manager: Option<Arc<ResourceRepresentationManager>>,
    ) -> Self {
        Self {
            #[cfg(feature = "std")]
            functions: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            functions: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(feature = "std")]
            pending_cleanups: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            pending_cleanups: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            async_engine,
            cancellation_manager,
            handle_tracker,
            resource_manager,
            representation_manager,
            metrics: PostReturnMetrics::default(),
            max_cleanup_tasks,
        }
    }

    /// Register a post-return function for an instance
    pub fn register_post_return(
        &mut self,
        instance_id: ComponentInstanceId,
        func_index: u32,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        let post_return_fn = PostReturnFunction { 
            func_index, 
            #[cfg(feature = "std")]
            func_ref: None, 
            executing: false,
            cancellation_token,
        };

        #[cfg(feature = "std")]
        {
            self.functions.insert(instance_id, post_return_fn);
            self.pending_cleanups.insert(instance_id, Vec::new());
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.functions.push((instance_id, post_return_fn)).map_err(|_| {
                Error::new(
                    ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many post-return functions"
                )
            })?;
            self.pending_cleanups.push((instance_id, BoundedVec::new(NoStdProvider::<65536>::default()).unwrap())).map_err(|_| {
                Error::new(
                    ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many cleanup instances"
                )
            })?;
        }

        Ok(())
    }

    /// Schedule a cleanup task to be executed during post-return
    pub fn schedule_cleanup(
        &mut self,
        instance_id: ComponentInstanceId,
        task: CleanupTask,
    ) -> Result<()> {
        #[cfg(feature = "std")]
        {
            let cleanup_tasks = self
                .pending_cleanups
                .get_mut(&instance_id)
                .ok_or_else(|| {
                    Error::new(
                        ErrorCategory::Runtime,
                        wrt_error::codes::EXECUTION_ERROR,
                        "Instance not found for cleanup"
                    )
                })?;

            if cleanup_tasks.len() >= self.max_cleanup_tasks {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many cleanup tasks"
                ));
            }

            cleanup_tasks.push(task);

            // Update peak tasks metric
            let total_pending: usize = self.pending_cleanups.values().map(|tasks| tasks.len()).sum();
            if total_pending > self.metrics.peak_pending_tasks {
                self.metrics.peak_pending_tasks = total_pending;
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (id, cleanup_tasks) in &mut self.pending_cleanups {
                if *id == instance_id {
                    cleanup_tasks.push(task).map_err(|_| {
                        Error::new(
                            ErrorCategory::Resource,
                            wrt_error::codes::RESOURCE_EXHAUSTED,
                            "Too many cleanup tasks"
                        )
                    })?;
                    
                    // Update peak tasks metric
                    let total_pending: usize = self.pending_cleanups.iter().map(|(_, tasks)| tasks.len()).sum();
                    if total_pending > self.metrics.peak_pending_tasks {
                        self.metrics.peak_pending_tasks = total_pending;
                    }
                    
                    return Ok(());
                }
            }
            
            return Err(Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::EXECUTION_ERROR,
                "Instance not found for cleanup"
            ));
        }

        Ok(())
    }

    /// Execute post-return cleanup for an instance
    pub fn execute_post_return(
        &mut self,
        instance_id: ComponentInstanceId,
        context: PostReturnContext,
    ) -> Result<()> {
        // Check if post-return function exists and isn't already executing
        #[cfg(feature = "std")]
        let post_return_fn = self
            .functions
            .get_mut(&instance_id)
            .ok_or_else(|| {
                Error::new(
                    ErrorCategory::Runtime,
                    wrt_error::codes::EXECUTION_ERROR,
                    "Post-return function not found"
                )
            })?;
        
        #[cfg(not(any(feature = "std", )))]
        let post_return_fn = {
            let mut found = None;
            for (id, func) in &mut self.functions {
                if *id == instance_id {
                    found = Some(func);
                    break;
                }
            }
            found.ok_or_else(|| {
                Error::new(
                    ErrorCategory::Runtime,
                    wrt_error::codes::EXECUTION_ERROR,
                    "Post-return function not found"
                )
            })?
        };

        if post_return_fn.executing {
            return Err(Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::EXECUTION_ERROR,
                "Post-return function already executing"
            ));
        }

        post_return_fn.executing = true;

        // Simple timing implementation for no_std
        let result = self.execute_cleanup_tasks(instance_id, context);

        // Update metrics
        self.metrics.total_executions += 1;
        if result.is_err() {
            self.metrics.failed_cleanups += 1;
        }

        post_return_fn.executing = false;

        // Clear pending cleanups
        #[cfg(feature = "std")]
        {
            if let Some(cleanup_tasks) = self.pending_cleanups.get_mut(&instance_id) {
                cleanup_tasks.clear();
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (id, cleanup_tasks) in &mut self.pending_cleanups {
                if *id == instance_id {
                    cleanup_tasks.clear();
                    break;
                }
            }
        }

        result
    }

    /// Execute all pending cleanup tasks
    fn execute_cleanup_tasks(
        &mut self,
        instance_id: ComponentInstanceId,
        mut context: PostReturnContext,
    ) -> Result<()> {
        // Get all pending cleanup tasks
        #[cfg(feature = "std")]
        let mut all_tasks = context.tasks;
        #[cfg(not(any(feature = "std", )))]
        let mut all_tasks = context.tasks;

        #[cfg(feature = "std")]
        {
            if let Some(pending) = self.pending_cleanups.get(&instance_id) {
                all_tasks.extend(pending.iter().cloned());
            }
            all_tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (id, pending) in &self.pending_cleanups {
                if *id == instance_id {
                    for task in pending {
                        if all_tasks.push(task.clone()).is_err() {
                            break; // Skip if no more space
                        }
                    }
                    break;
                }
            }
            // Simple bubble sort for no_std
            for i in 0..all_tasks.len() {
                for j in 0..(all_tasks.len() - 1 - i) {
                    if all_tasks[j].priority < all_tasks[j + 1].priority {
                        let temp = all_tasks[j].clone();
                        all_tasks[j] = all_tasks[j + 1].clone();
                        all_tasks[j + 1] = temp;
                    }
                }
            }
        }

        // Execute each task
        for task in &all_tasks {
            self.execute_single_cleanup_task(task, &mut context)?;
            self.metrics.total_cleanup_tasks += 1;
        }

        Ok(())
    }

    /// Execute a single cleanup task
    fn execute_single_cleanup_task(
        &mut self,
        task: &CleanupTask,
        context: &mut PostReturnContext,
    ) -> Result<()> {
        match task.task_type {
            CleanupTaskType::DeallocateMemory => self.cleanup_memory(task, context),
            CleanupTaskType::CloseResource => self.cleanup_resource(task, context),
            CleanupTaskType::ReleaseReference => self.cleanup_reference(task, context),
            CleanupTaskType::Custom => self.cleanup_custom(task, context),
            CleanupTaskType::AsyncCleanup => self.cleanup_async(task, context),
            CleanupTaskType::CancelAsyncExecution => self.cleanup_cancel_async_execution(task, context),
            CleanupTaskType::DropBorrowedHandles => self.cleanup_drop_borrowed_handles(task, context),
            CleanupTaskType::EndLifetimeScope => self.cleanup_end_lifetime_scope(task, context),
            CleanupTaskType::ReleaseResourceRepresentation => self.cleanup_release_resource_representation(task, context),
            CleanupTaskType::FinalizeSubtask => self.cleanup_finalize_subtask(task, context),
        }
    }

    /// Binary std/no_std choice
    fn cleanup_memory(
        &self,
        task: &CleanupTask,
        context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::Memory { ptr, size, align } = &task.data {
            if let Some(realloc_manager) = &context.realloc_manager {
                // Binary std/no_std choice
                // For now, we just acknowledge the cleanup
            }
        }
        Ok(())
    }

    /// Clean up resource handle
    fn cleanup_resource(
        &mut self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::Resource { handle, resource_type: _ } = &task.data {
            self.metrics.resource_cleanups += 1;
            
            // Use resource manager if available
            if let Some(resource_manager) = &self.resource_manager {
                // In a real implementation, this would drop the resource
            }
        }
        Ok(())
    }

    /// Clean up reference count
    fn cleanup_reference(
        &self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::Reference { ref_id: _, ref_count: _ } = &task.data {
            // Binary std/no_std choice
            // Implementation would depend on reference counting system
        }
        Ok(())
    }

    /// Execute custom cleanup
    fn cleanup_custom(
        &self,
        task: &CleanupTask,
        context: &mut PostReturnContext,
    ) -> Result<()> {
        match &task.data {
            #[cfg(feature = "std")]
            CleanupData::Custom { cleanup_id, parameters: _ } => {
                if let Some(handler) = context.custom_handlers.get(cleanup_id) {
                    handler(&task.data)?;
                }
            }
            #[cfg(not(any(feature = "std", )))]
            CleanupData::Custom { cleanup_id, parameters: _ } => {
                for (id, handler) in &context.custom_handlers {
                    if id.as_str() == cleanup_id.as_str() {
                        handler(&task.data)?;
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Clean up async resources (streams, futures, etc.)
    fn cleanup_async(
        &mut self,
        task: &CleanupTask,
        context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::Async { 
            stream_handle, 
            future_handle, 
            error_context_handle,
            task_id: _,
            execution_id: _,
            cancellation_token,
        } = &task.data {
            self.metrics.async_cleanups += 1;
            
            // Cancel operations if cancellation token is available
            if let Some(token) = cancellation_token {
                let _ = token.cancel();
            }
            
            // Clean up async ABI resources
            if let Some(async_abi) = &context.async_abi {
                if let Some(stream) = stream_handle {
                    let _ = async_abi.stream_close_readable(*stream);
                    let _ = async_abi.stream_close_writable(*stream);
                }
                
                if let Some(future) = future_handle {
                    // Future cleanup would be handled by the async ABI
                }
                
                if let Some(error_ctx) = error_context_handle {
                    let _ = async_abi.error_context_drop(*error_ctx);
                }
            }
        }
        Ok(())
    }
    
    /// Cancel async execution
    fn cleanup_cancel_async_execution(
        &mut self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::AsyncExecution { execution_id, force_cancel } = &task.data {
            self.metrics.cancellation_cleanups += 1;
            
            if let Some(async_engine) = &self.async_engine {
                // In a real implementation, this would cancel the execution
                if *force_cancel {
                    // Force cancel the execution
                } else {
                    // Graceful cancellation
                }
            }
        }
        Ok(())
    }
    
    /// Drop borrowed handles
    fn cleanup_drop_borrowed_handles(
        &mut self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::BorrowedHandle { 
            borrow_handle: _, 
            lifetime_scope, 
            source_component: _ 
        } = &task.data {
            self.metrics.handle_cleanups += 1;
            
            if let Some(handle_tracker) = &self.handle_tracker {
                // In a real implementation, this would invalidate the borrow
                // For now, we just acknowledge the cleanup
            }
        }
        Ok(())
    }
    
    /// End lifetime scope
    fn cleanup_end_lifetime_scope(
        &mut self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::LifetimeScope { scope, component: _, task: _ } = &task.data {
            if let Some(handle_tracker) = &self.handle_tracker {
                // In a real implementation, this would end the scope
                // For now, we just acknowledge the cleanup
            }
        }
        Ok(())
    }
    
    /// Release resource representation
    fn cleanup_release_resource_representation(
        &mut self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::ResourceRepresentation { 
            handle, 
            resource_id: _, 
            component: _ 
        } = &task.data {
            self.metrics.resource_cleanups += 1;
            
            if let Some(repr_manager) = &self.representation_manager {
                // In a real implementation, this would drop the resource representation
                // let _ = canon_resource_drop(repr_manager, *handle);
            }
        }
        Ok(())
    }
    
    /// Finalize subtask
    fn cleanup_finalize_subtask(
        &mut self,
        task: &CleanupTask,
        _context: &mut PostReturnContext,
    ) -> Result<()> {
        if let CleanupData::Subtask { 
            execution_id, 
            task_id: _, 
            result: _, 
            force_cleanup 
        } = &task.data {
            if let Some(cancellation_manager) = &self.cancellation_manager {
                if *force_cleanup {
                    // Force cleanup the subtask
                } else {
                    // Graceful finalization
                }
            }
        }
        Ok(())
    }

    /// Remove all cleanup tasks for an instance
    pub fn cleanup_instance(
        &mut self,
        instance_id: ComponentInstanceId,
    ) -> Result<()> {
        #[cfg(feature = "std")]
        {
            self.functions.remove(&instance_id);
            self.pending_cleanups.remove(&instance_id);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            // Remove from functions
            let mut i = 0;
            while i < self.functions.len() {
                if self.functions[i].0 == instance_id {
                    self.functions.remove(i);
                    break;
                } else {
                    i += 1;
                }
            }
            
            // Remove from pending cleanups
            let mut i = 0;
            while i < self.pending_cleanups.len() {
                if self.pending_cleanups[i].0 == instance_id {
                    self.pending_cleanups.remove(i);
                    break;
                } else {
                    i += 1;
                }
            }
        }
        Ok(())
    }

    /// Get execution metrics
    pub fn metrics(&self) -> &PostReturnMetrics {
        &self.metrics
    }

    /// Reset metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = PostReturnMetrics::default();
    }
}

/// Helper functions for common cleanup scenarios
pub mod helpers {
    use super::*;

    /// Create a memory cleanup task
    pub fn memory_cleanup_task(
        instance_id: ComponentInstanceId,
        ptr: i32,
        size: i32,
        align: i32,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::DeallocateMemory,
            source_instance: instance_id,
            priority,
            data: CleanupData::Memory { ptr, size, align },
        }
    }

    /// Create a resource cleanup task
    pub fn resource_cleanup_task(
        instance_id: ComponentInstanceId,
        handle: u32,
        resource_type: TypeId,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::CloseResource,
            source_instance: instance_id,
            priority,
            data: CleanupData::Resource { handle, resource_type },
        }
    }

    /// Create an async cleanup task for streams, futures, and async operations
    pub fn async_cleanup_task(
        instance_id: ComponentInstanceId,
        stream_handle: Option<StreamHandle>,
        future_handle: Option<FutureHandle>,
        error_context_handle: Option<ErrorContextHandle>,
        task_id: Option<TaskId>,
        execution_id: Option<ExecutionId>,
        cancellation_token: Option<CancellationToken>,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::AsyncCleanup,
            source_instance: instance_id,
            priority,
            data: CleanupData::Async { 
                stream_handle, 
                future_handle, 
                error_context_handle,
                task_id,
                execution_id,
                cancellation_token,
            },
        }
    }
    
    /// Create an async execution cancellation task
    pub fn async_execution_cleanup_task(
        instance_id: ComponentInstanceId,
        execution_id: ExecutionId,
        force_cancel: bool,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::CancelAsyncExecution,
            source_instance: instance_id,
            priority,
            data: CleanupData::AsyncExecution { execution_id, force_cancel },
        }
    }
    
    /// Create a borrowed handle cleanup task
    pub fn borrowed_handle_cleanup_task(
        instance_id: ComponentInstanceId,
        borrow_handle: u32,
        lifetime_scope: LifetimeScope,
        source_component: ComponentId,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::DropBorrowedHandles,
            source_instance: instance_id,
            priority,
            data: CleanupData::BorrowedHandle { 
                borrow_handle, 
                lifetime_scope, 
                source_component 
            },
        }
    }
    
    /// Create a lifetime scope cleanup task
    pub fn lifetime_scope_cleanup_task(
        instance_id: ComponentInstanceId,
        scope: LifetimeScope,
        component: ComponentId,
        task: TaskId,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::EndLifetimeScope,
            source_instance: instance_id,
            priority,
            data: CleanupData::LifetimeScope { scope, component, task },
        }
    }
    
    /// Create a resource representation cleanup task
    pub fn resource_representation_cleanup_task(
        instance_id: ComponentInstanceId,
        handle: u32,
        resource_id: ResourceId,
        component: ComponentId,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::ReleaseResourceRepresentation,
            source_instance: instance_id,
            priority,
            data: CleanupData::ResourceRepresentation { handle, resource_id, component },
        }
    }
    
    /// Create a subtask finalization cleanup task
    pub fn subtask_cleanup_task(
        instance_id: ComponentInstanceId,
        execution_id: ExecutionId,
        task_id: TaskId,
        result: Option<SubtaskResult>,
        force_cleanup: bool,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::FinalizeSubtask,
            source_instance: instance_id,
            priority,
            data: CleanupData::Subtask { execution_id, task_id, result, force_cleanup },
        }
    }

    /// Create a custom cleanup task
    #[cfg(feature = "std")]
    pub fn custom_cleanup_task(
        instance_id: ComponentInstanceId,
        cleanup_id: &str,
        parameters: Vec<ComponentValue>,
        priority: u8,
    ) -> CleanupTask {
        CleanupTask {
            task_type: CleanupTaskType::Custom,
            source_instance: instance_id,
            priority,
            data: CleanupData::Custom { 
                cleanup_id: cleanup_id.to_string(), 
                parameters 
            },
        }
    }
    
    /// Create a custom cleanup task (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn custom_cleanup_task(
        instance_id: ComponentInstanceId,
        cleanup_id: &str,
        parameters: BoundedVec<ComponentValue, 16, NoStdProvider<65536>>,
        priority: u8,
    ) -> Result<CleanupTask> {
        let cleanup_id = BoundedString::from_str(cleanup_id).map_err(|_| {
            Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::EXECUTION_ERROR,
                "Cleanup ID too long"
            )
        })?;

        let param_vec = parameters;

        Ok(CleanupTask {
            task_type: CleanupTaskType::Custom,
            source_instance: instance_id,
            priority,
            data: CleanupData::Custom { cleanup_id, parameters: param_vec },
        })
    }
}

impl Default for PostReturnRegistry {
    fn default() -> Self {
        Self::new(1024) // Default max 1024 cleanup tasks per instance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_realloc::ReallocManager;

    #[test]
    fn test_post_return_registry_creation() {
        let registry = PostReturnRegistry::new(100);
        assert_eq!(registry.max_cleanup_tasks, 100);
        assert_eq!(registry.functions.len(), 0);
    }

    #[test]
    fn test_register_post_return() {
        let mut registry = PostReturnRegistry::new(100);
        let instance_id = ComponentInstanceId(1);

        assert!(registry.register_post_return(instance_id, 42).is_ok());
        assert!(registry.functions.contains_key(&instance_id));
    }

    #[test]
    fn test_schedule_cleanup() {
        let mut registry = PostReturnRegistry::new(100);
        let instance_id = ComponentInstanceId(1);

        registry.register_post_return(instance_id, 42).unwrap();

        let task = helpers::memory_cleanup_task(instance_id, 0x1000, 64, 8, 10);
        assert!(registry.schedule_cleanup(instance_id, task).is_ok());

        assert_eq!(registry.pending_cleanups[&instance_id].len(), 1);
    }

    #[test]
    fn test_cleanup_task_helpers() {
        let instance_id = ComponentInstanceId(1);

        // Test memory cleanup task
        let mem_task = helpers::memory_cleanup_task(instance_id, 0x1000, 64, 8, 10);
        assert_eq!(mem_task.task_type, CleanupTaskType::DeallocateMemory);
        assert_eq!(mem_task.priority, 10);

        // Test resource cleanup task
        let res_task = helpers::resource_cleanup_task(instance_id, 42, TypeId(1), 5);
        assert_eq!(res_task.task_type, CleanupTaskType::CloseResource);
        assert_eq!(res_task.priority, 5);

        // Test async cleanup task
        let async_task = helpers::async_cleanup_task(instance_id, Some(1), Some(2), Some(3), 8);
        assert_eq!(async_task.task_type, CleanupTaskType::AsyncCleanup);
        assert_eq!(async_task.priority, 8);

        // Test custom cleanup task
        let custom_task = helpers::custom_cleanup_task(
            instance_id,
            "custom_cleanup",
            vec![ComponentValue::U32(42)],
            7,
        );
        assert!(custom_task.is_ok());
        let custom_task = custom_task.unwrap();
        assert_eq!(custom_task.task_type, CleanupTaskType::Custom);
        assert_eq!(custom_task.priority, 7);
    }

    #[test]
    fn test_cleanup_task_limits() {
        let mut registry = PostReturnRegistry::new(2); // Small limit for testing
        let instance_id = ComponentInstanceId(1);

        registry.register_post_return(instance_id, 42).unwrap();

        // Add tasks up to limit
        let task1 = helpers::memory_cleanup_task(instance_id, 0x1000, 64, 8, 10);
        let task2 = helpers::memory_cleanup_task(instance_id, 0x2000, 64, 8, 10);
        let task3 = helpers::memory_cleanup_task(instance_id, 0x3000, 64, 8, 10);

        assert!(registry.schedule_cleanup(instance_id, task1).is_ok());
        assert!(registry.schedule_cleanup(instance_id, task2).is_ok());
        assert!(registry.schedule_cleanup(instance_id, task3).is_err()); // Should fail
    }

    #[test]
    fn test_metrics() {
        let registry = PostReturnRegistry::new(100);
        let metrics = registry.metrics();

        assert_eq!(metrics.total_executions, 0);
        assert_eq!(metrics.total_cleanup_tasks, 0);
        assert_eq!(metrics.failed_cleanups, 0);
    }
}
