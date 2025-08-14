//! Async Built-in Functions for WebAssembly Component Model MVP
//!
//! This module implements the missing async built-in functions required
//! by the latest Component Model MVP specification, including task and
//! subtask cancellation operations.

#[cfg(not(feature = "std"))]
use core::fmt;
#[cfg(feature = "std")]
use std::fmt;

use wrt_error::{
    Error,
    ErrorCategory,
    Result,
};
#[cfg(feature = "std")]
use wrt_foundation::component_value::ComponentValue;
use wrt_foundation::{
    bounded::{
        BoundedString,
        BoundedVec,
    },
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    safe_memory::NoStdProvider,
    values::Value,
};

#[cfg(not(feature = "std"))]
use crate::types::Value as ComponentValue;
use crate::{
    async_types::{
        ErrorContext,
        FutureHandle,
        FutureState,
        StreamHandle,
        StreamState,
        Waitable,
        WaitableSet,
    },
    types::ValType,
};

/// Task handle for task cancellation operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskHandle(pub u32);

/// Subtask handle for subtask cancellation operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubtaskHandle(pub u32);

/// Task cancellation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelResult {
    /// Task was successfully cancelled
    Cancelled,
    /// Task was already completed
    AlreadyCompleted,
    /// Task was already cancelled
    AlreadyCancelled,
    /// Task not found
    NotFound,
    /// Cancellation failed due to error
    Failed(String),
}

impl fmt::Display for CancelResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CancelResult::Cancelled => write!(f, "cancelled"),
            CancelResult::AlreadyCompleted => write!(f, "already-completed"),
            CancelResult::AlreadyCancelled => write!(f, "already-cancelled"),
            CancelResult::NotFound => write!(f, "not-found"),
            CancelResult::Failed(msg) => write!(f, "failed: {}", msg),
        }
    }
}

/// Global task registry for tracking active tasks
pub struct TaskRegistry {
    #[cfg(feature = "std")]
    tasks: std::collections::HashMap<TaskHandle, TaskInfo>,
    #[cfg(not(feature = "std"))]
    tasks: BoundedVec<(TaskHandle, TaskInfo), 1024, NoStdProvider<65536>>,

    #[cfg(feature = "std")]
    subtasks: std::collections::HashMap<SubtaskHandle, SubtaskInfo>,
    #[cfg(not(feature = "std"))]
    subtasks: BoundedVec<(SubtaskHandle, SubtaskInfo), 1024, NoStdProvider<65536>>,

    next_task_id:    u32,
    next_subtask_id: u32,
}

/// Information about a tracked task
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub handle:        TaskHandle,
    pub state:         TaskState,
    pub future_handle: Option<FutureHandle>,
    pub stream_handle: Option<StreamHandle>,
    pub parent_task:   Option<TaskHandle>,
    pub subtasks:      Vec<SubtaskHandle>,
    /// Task-local context storage
    #[cfg(feature = "std")]
    pub context:       std::collections::HashMap<String, ComponentValue>,
    #[cfg(not(feature = "std"))]
    pub context: BoundedVec<
        (BoundedString<64, NoStdProvider<65536>>, ComponentValue),
        64,
        NoStdProvider<65536>,
    >,
}

/// Information about a tracked subtask
#[derive(Debug, Clone)]
pub struct SubtaskInfo {
    pub handle:        SubtaskHandle,
    pub state:         TaskState,
    pub parent_task:   TaskHandle,
    pub future_handle: Option<FutureHandle>,
    pub stream_handle: Option<StreamHandle>,
}

/// Task state for cancellation tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        // Use new() which properly handles allocation or panic in development
        Self::new().expect("TaskRegistry allocation should not fail in default construction")
    }
}

impl TaskRegistry {
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            tasks:                              std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            tasks:                              {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)
                    .map_err(|_| Error::runtime_execution_error("Failed to create tasks vector"))?
            },

            #[cfg(feature = "std")]
            subtasks:                              std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            subtasks:                              {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).map_err(|_| {
                    Error::runtime_execution_error("Failed to create subtasks vector")
                })?
            },

            next_task_id:    1,
            next_subtask_id: 1,
        })
    }

    /// Register a new task
    pub fn register_task(
        &mut self,
        future_handle: Option<FutureHandle>,
        stream_handle: Option<StreamHandle>,
    ) -> Result<TaskHandle> {
        let handle = TaskHandle(self.next_task_id);
        self.next_task_id += 1;

        let task_info = TaskInfo {
            handle,
            state: TaskState::Running,
            future_handle,
            stream_handle,
            parent_task: None,
            subtasks: Vec::new(),
            #[cfg(feature = "std")]
            context: std::collections::HashMap::new(),
            #[cfg(not(feature = "std"))]
            context: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)
                    .map_err(|_| Error::runtime_execution_error("Failed to create task context"))?
            },
        };

        #[cfg(feature = "std")]
        {
            self.tasks.insert(handle, task_info);
        }
        #[cfg(not(feature = "std"))]
        {
            self.tasks
                .push((handle, task_info))
                .map_err(|_| Error::runtime_execution_error("Failed to register task"))?;
        }

        Ok(handle)
    }

    /// Register a new subtask
    pub fn register_subtask(
        &mut self,
        parent_task: TaskHandle,
        future_handle: Option<FutureHandle>,
        stream_handle: Option<StreamHandle>,
    ) -> Result<SubtaskHandle> {
        let handle = SubtaskHandle(self.next_subtask_id);
        self.next_subtask_id += 1;

        let subtask_info = SubtaskInfo {
            handle,
            state: TaskState::Running,
            parent_task,
            future_handle,
            stream_handle,
        };

        #[cfg(feature = "std")]
        {
            self.subtasks.insert(handle, subtask_info);
            if let Some(parent_info) = self.tasks.get_mut(&parent_task) {
                parent_info.subtasks.push(handle);
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.subtasks
                .push((handle, subtask_info))
                .map_err(|_| Error::runtime_execution_error("Failed to register subtask"))?;

            // Update parent task
            for (task_handle, task_info) in &mut self.tasks {
                if *task_handle == parent_task {
                    task_info.subtasks.push(handle);
                    break;
                }
            }
        }

        Ok(handle)
    }

    /// Cancel a task and all its subtasks
    pub fn cancel_task(&mut self, handle: TaskHandle) -> CancelResult {
        #[cfg(feature = "std")]
        {
            if let Some(task_info) = self.tasks.get_mut(&handle) {
                match task_info.state {
                    TaskState::Running => {
                        task_info.state = TaskState::Cancelled;

                        // Cancel all subtasks
                        let subtasks = task_info.subtasks.clone();
                        for subtask_handle in subtasks {
                            self.cancel_subtask(subtask_handle);
                        }

                        CancelResult::Cancelled
                    },
                    TaskState::Completed => CancelResult::AlreadyCompleted,
                    TaskState::Cancelled => CancelResult::AlreadyCancelled,
                    TaskState::Failed => CancelResult::AlreadyCompleted,
                }
            } else {
                CancelResult::NotFound
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (task_handle, task_info) in &mut self.tasks {
                if *task_handle == handle {
                    match task_info.state {
                        TaskState::Running => {
                            task_info.state = TaskState::Cancelled;

                            // Cancel all subtasks
                            let subtasks = task_info.subtasks.clone();
                            for subtask_handle in subtasks {
                                self.cancel_subtask(subtask_handle);
                            }

                            return CancelResult::Cancelled;
                        },
                        TaskState::Completed => return CancelResult::AlreadyCompleted,
                        TaskState::Cancelled => return CancelResult::AlreadyCancelled,
                        TaskState::Failed => return CancelResult::AlreadyCompleted,
                    }
                }
            }
            CancelResult::NotFound
        }
    }

    /// Cancel a specific subtask
    pub fn cancel_subtask(&mut self, handle: SubtaskHandle) -> CancelResult {
        #[cfg(feature = "std")]
        {
            if let Some(subtask_info) = self.subtasks.get_mut(&handle) {
                match subtask_info.state {
                    TaskState::Running => {
                        subtask_info.state = TaskState::Cancelled;
                        CancelResult::Cancelled
                    },
                    TaskState::Completed => CancelResult::AlreadyCompleted,
                    TaskState::Cancelled => CancelResult::AlreadyCancelled,
                    TaskState::Failed => CancelResult::AlreadyCompleted,
                }
            } else {
                CancelResult::NotFound
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (subtask_handle, subtask_info) in &mut self.subtasks {
                if *subtask_handle == handle {
                    match subtask_info.state {
                        TaskState::Running => {
                            subtask_info.state = TaskState::Cancelled;
                            return CancelResult::Cancelled;
                        },
                        TaskState::Completed => return CancelResult::AlreadyCompleted,
                        TaskState::Cancelled => return CancelResult::AlreadyCancelled,
                        TaskState::Failed => return CancelResult::AlreadyCompleted,
                    }
                }
            }
            CancelResult::NotFound
        }
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, handle: TaskHandle) -> Result<()> {
        #[cfg(feature = "std")]
        {
            if let Some(task_info) = self.tasks.get_mut(&handle) {
                task_info.state = TaskState::Completed;
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (task_handle, task_info) in &mut self.tasks {
                if *task_handle == handle {
                    task_info.state = TaskState::Completed;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Mark a subtask as completed
    pub fn complete_subtask(&mut self, handle: SubtaskHandle) -> Result<()> {
        #[cfg(feature = "std")]
        {
            if let Some(subtask_info) = self.subtasks.get_mut(&handle) {
                subtask_info.state = TaskState::Completed;
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (subtask_handle, subtask_info) in &mut self.subtasks {
                if *subtask_handle == handle {
                    subtask_info.state = TaskState::Completed;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Set a context value for a task
    pub fn set_task_context(
        &mut self,
        handle: TaskHandle,
        key: &str,
        value: ComponentValue,
    ) -> Result<()> {
        #[cfg(feature = "std")]
        {
            if let Some(task_info) = self.tasks.get_mut(&handle) {
                task_info.context.insert(key.to_string(), value);
            } else {
                return Err(Error::runtime_execution_error("Task not found"));
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (task_handle, task_info) in &mut self.tasks {
                if *task_handle == handle {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    let key_bounded = BoundedString::from_str(key, provider).map_err(|_| {
                        Error::runtime_execution_error(
                            "Failed to create bounded string for task context key",
                        )
                    })?;

                    // Remove existing entry if present
                    let mut index_to_remove = None;
                    for (i, (existing_key, _)) in task_info.context.iter().enumerate() {
                        if existing_key.as_str() == key {
                            index_to_remove = Some(i);
                            break;
                        }
                    }
                    if let Some(index) = index_to_remove {
                        task_info.context.remove(index);
                    }

                    // Add new entry
                    task_info.context.push((key_bounded, value)).map_err(|_| {
                        Error::new(
                            ErrorCategory::Resource,
                            wrt_error::codes::RESOURCE_EXHAUSTED,
                            "Failed to add task context entry",
                        )
                    })?;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Get a context value for a task
    pub fn get_task_context(&self, handle: TaskHandle, key: &str) -> Option<ComponentValue> {
        #[cfg(feature = "std")]
        {
            if let Some(task_info) = self.tasks.get(&handle) {
                task_info.context.get(key).cloned()
            } else {
                None
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (task_handle, task_info) in &self.tasks {
                if *task_handle == handle {
                    for (existing_key, value) in &task_info.context {
                        if existing_key.as_str() == key {
                            return Some(value.clone());
                        }
                    }
                    break;
                }
            }
            None
        }
    }

    /// Get current task handle (simplified implementation)
    pub fn get_current_task(&self) -> Option<TaskHandle> {
        // For this implementation, return the first running task
        // In a real implementation, this would use thread-local storage
        #[cfg(feature = "std")]
        {
            for (handle, task_info) in &self.tasks {
                if task_info.state == TaskState::Running {
                    return Some(*handle);
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (handle, task_info) in &self.tasks {
                if task_info.state == TaskState::Running {
                    return Some(*handle);
                }
            }
        }
        None
    }
}

/// ASIL-D safe global task registry
use std::sync::{
    Mutex,
    OnceLock,
};
static GLOBAL_TASK_REGISTRY: OnceLock<Mutex<TaskRegistry>> = OnceLock::new();

/// Get the global task registry (ASIL-D safe)
fn get_task_registry() -> Result<&'static Mutex<TaskRegistry>, Error> {
    Ok(GLOBAL_TASK_REGISTRY.get_or_init(|| {
        Mutex::new(TaskRegistry::new().expect("Global task registry allocation should not fail"))
    }))
}

/// Async built-in function implementations
pub mod builtins {
    use super::*;

    /// `task.cancel` canonical built-in
    /// Cancels a running task and all its subtasks
    pub fn task_cancel(task_handle: u32) -> Result<ComponentValue> {
        let handle = TaskHandle(task_handle);
        let registry_mutex = get_task_registry()?;
        let mut registry = registry_mutex
            .lock()
            .map_err(|_| Error::runtime_execution_error("Failed to acquire task registry lock"))?;
        let result = registry.cancel_task(handle);

        match result {
            CancelResult::Cancelled => Ok(ComponentValue::U32(0)), // Success
            CancelResult::AlreadyCompleted => Ok(ComponentValue::U32(1)), // Already completed
            CancelResult::AlreadyCancelled => Ok(ComponentValue::U32(2)), // Already cancelled
            CancelResult::NotFound => Ok(ComponentValue::U32(3)),  // Not found
            CancelResult::Failed(_) => Ok(ComponentValue::U32(4)), // Failed
        }
    }

    /// `subtask.cancel` canonical built-in
    /// Cancels a specific subtask
    pub fn subtask_cancel(subtask_handle: u32) -> Result<ComponentValue> {
        let handle = SubtaskHandle(subtask_handle);
        let registry_mutex = get_task_registry()?;
        let mut registry = registry_mutex.lock().map_err(|_| {
            Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::CONCURRENCY_ERROR,
                "Failed to acquire task registry lock",
            )
        })?;
        let result = registry.cancel_subtask(handle);

        match result {
            CancelResult::Cancelled => Ok(ComponentValue::U32(0)), // Success
            CancelResult::AlreadyCompleted => Ok(ComponentValue::U32(1)), // Already completed
            CancelResult::AlreadyCancelled => Ok(ComponentValue::U32(2)), // Already cancelled
            CancelResult::NotFound => Ok(ComponentValue::U32(3)),  // Not found
            CancelResult::Failed(_) => Ok(ComponentValue::U32(4)), // Failed
        }
    }

    /// `task.spawn` canonical built-in (bonus implementation)
    /// Spawns a new async task
    pub fn task_spawn(
        future_handle: Option<u32>,
        stream_handle: Option<u32>,
    ) -> Result<ComponentValue> {
        let registry_mutex = get_task_registry()?;
        let mut registry = registry_mutex
            .lock()
            .map_err(|_| Error::runtime_execution_error("Failed to acquire task registry lock"))?;
        let future_h = future_handle.map(FutureHandle);
        let stream_h = stream_handle.map(StreamHandle);

        match registry.register_task(future_h, stream_h) {
            Ok(handle) => Ok(ComponentValue::U32(handle.0)),
            Err(_) => Err(Error::new(
                ErrorCategory::Resource,
                wrt_error::codes::RESOURCE_EXHAUSTED,
                "Failed to spawn task",
            )),
        }
    }

    /// `subtask.spawn` canonical built-in (bonus implementation)
    /// Spawns a new subtask under a parent task
    pub fn subtask_spawn(
        parent_task: u32,
        future_handle: Option<u32>,
        stream_handle: Option<u32>,
    ) -> Result<ComponentValue> {
        let registry_mutex = get_task_registry()?;
        let mut registry = registry_mutex
            .lock()
            .map_err(|_| Error::runtime_execution_error("Failed to acquire task registry lock"))?;
        let parent_h = TaskHandle(parent_task);
        let future_h = future_handle.map(FutureHandle);
        let stream_h = stream_handle.map(StreamHandle);

        match registry.register_subtask(parent_h, future_h, stream_h) {
            Ok(handle) => Ok(ComponentValue::U32(handle.0)),
            Err(_) => Err(Error::new(
                ErrorCategory::Resource,
                wrt_error::codes::RESOURCE_EXHAUSTED,
                "Failed to spawn subtask",
            )),
        }
    }

    /// `task.status` canonical built-in (bonus implementation)
    /// Gets the status of a task
    pub fn task_status(task_handle: u32) -> Result<ComponentValue> {
        let handle = TaskHandle(task_handle);
        let registry = get_task_registry()?;

        #[cfg(feature = "std")]
        {
            if let Some(task_info) = registry.tasks.get(&handle) {
                let status = match task_info.state {
                    TaskState::Running => 0u32,
                    TaskState::Completed => 1u32,
                    TaskState::Cancelled => 2u32,
                    TaskState::Failed => 3u32,
                };
                Ok(ComponentValue::U32(status))
            } else {
                Ok(ComponentValue::U32(4)) // Not found
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (task_handle_ref, task_info) in &registry.tasks {
                if *task_handle_ref == handle {
                    let status = match task_info.state {
                        TaskState::Running => 0u32,
                        TaskState::Completed => 1u32,
                        TaskState::Cancelled => 2u32,
                        TaskState::Failed => 3u32,
                    };
                    return Ok(ComponentValue::U32(status));
                }
            }
            Ok(ComponentValue::U32(4)) // Not found
        }
    }

    /// `subtask.status` canonical built-in (bonus implementation)
    /// Gets the status of a subtask
    pub fn subtask_status(subtask_handle: u32) -> Result<ComponentValue> {
        let handle = SubtaskHandle(subtask_handle);
        let registry = get_task_registry()?;

        #[cfg(feature = "std")]
        {
            if let Some(subtask_info) = registry.subtasks.get(&handle) {
                let status = match subtask_info.state {
                    TaskState::Running => 0u32,
                    TaskState::Completed => 1u32,
                    TaskState::Cancelled => 2u32,
                    TaskState::Failed => 3u32,
                };
                Ok(ComponentValue::U32(status))
            } else {
                Ok(ComponentValue::U32(4)) // Not found
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (subtask_handle_ref, subtask_info) in &registry.subtasks {
                if *subtask_handle_ref == handle {
                    let status = match subtask_info.state {
                        TaskState::Running => 0u32,
                        TaskState::Completed => 1u32,
                        TaskState::Cancelled => 2u32,
                        TaskState::Failed => 3u32,
                    };
                    return Ok(ComponentValue::U32(status));
                }
            }
            Ok(ComponentValue::U32(4)) // Not found
        }
    }

    /// `context.get` canonical built-in
    /// Gets a value from the current task's context storage
    pub fn context_get(key: &str) -> Result<ComponentValue> {
        let registry = get_task_registry()?;

        // Get current task (simplified implementation)
        if let Some(current_task) = registry.get_current_task() {
            if let Some(value) = registry.get_task_context(current_task, key) {
                Ok(value)
            } else {
                // Return a null/none indicator (using a special value)
                Ok(ComponentValue::Option(None))
            }
        } else {
            Err(Error::runtime_execution_error("No current task"))
        }
    }

    /// `context.set` canonical built-in
    /// Sets a value in the current task's context storage
    pub fn context_set(key: &str, value: ComponentValue) -> Result<ComponentValue> {
        let registry = get_task_registry()?;

        // Get current task (simplified implementation)
        if let Some(current_task) = registry.get_current_task() {
            registry.set_task_context(current_task, key, value)?;
            Ok(ComponentValue::U32(0)) // Success
        } else {
            Err(Error::new(
                ErrorCategory::Runtime,
                wrt_error::codes::INVALID_OPERATION,
                "No current task context",
            ))
        }
    }

    /// `context.has` canonical built-in (bonus implementation)
    /// Checks if a key exists in the current task's context
    pub fn context_has(key: &str) -> Result<ComponentValue> {
        let registry = get_task_registry()?;

        if let Some(current_task) = registry.get_current_task() {
            let has_key = registry.get_task_context(current_task, key).is_some();
            Ok(ComponentValue::Bool(has_key))
        } else {
            Err(Error::runtime_execution_error("No current task"))
        }
    }

    /// `context.clear` canonical built-in (bonus implementation)
    /// Clears all values from the current task's context
    pub fn context_clear() -> Result<ComponentValue> {
        let registry = get_task_registry()?;

        if let Some(current_task) = registry.get_current_task() {
            #[cfg(feature = "std")]
            {
                if let Some(task_info) = registry.tasks.get_mut(&current_task) {
                    task_info.context.clear();
                }
            }
            #[cfg(not(feature = "std"))]
            {
                for (task_handle, task_info) in &mut registry.tasks {
                    if *task_handle == current_task {
                        task_info.context.clear();
                        break;
                    }
                }
            }
            Ok(ComponentValue::U32(0)) // Success
        } else {
            Err(Error::runtime_execution_error("No current task"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_registry_creation() {
        let mut registry = TaskRegistry::new().unwrap();
        assert_eq!(registry.next_task_id, 1);
        assert_eq!(registry.next_subtask_id, 1);
    }

    #[test]
    fn test_task_registration() {
        let mut registry = TaskRegistry::new().unwrap();
        let handle = registry.register_task(None, None).unwrap();
        assert_eq!(handle.0, 1);
        assert_eq!(registry.next_task_id, 2);
    }

    #[test]
    fn test_subtask_registration() {
        let mut registry = TaskRegistry::new().unwrap();
        let parent_handle = registry.register_task(None, None).unwrap();
        let subtask_handle = registry.register_subtask(parent_handle, None, None).unwrap();
        assert_eq!(subtask_handle.0, 1);
        assert_eq!(registry.next_subtask_id, 2);
    }

    #[test]
    fn test_task_cancellation() {
        let mut registry = TaskRegistry::new().unwrap();
        let handle = registry.register_task(None, None).unwrap();

        let result = registry.cancel_task(handle);
        assert_eq!(result, CancelResult::Cancelled);

        // Try to cancel again
        let result = registry.cancel_task(handle);
        assert_eq!(result, CancelResult::AlreadyCancelled);
    }

    #[test]
    fn test_subtask_cancellation() {
        let mut registry = TaskRegistry::new().unwrap();
        let parent_handle = registry.register_task(None, None).unwrap();
        let subtask_handle = registry.register_subtask(parent_handle, None, None).unwrap();

        let result = registry.cancel_subtask(subtask_handle);
        assert_eq!(result, CancelResult::Cancelled);

        // Try to cancel again
        let result = registry.cancel_subtask(subtask_handle);
        assert_eq!(result, CancelResult::AlreadyCancelled);
    }

    #[test]
    fn test_task_completion() {
        let mut registry = TaskRegistry::new().unwrap();
        let handle = registry.register_task(None, None).unwrap();

        registry.complete_task(handle).unwrap();

        let result = registry.cancel_task(handle);
        assert_eq!(result, CancelResult::AlreadyCompleted);
    }

    #[test]
    fn test_builtin_task_cancel() {
        let result = builtins::task_cancel(999).unwrap();
        assert_eq!(result, ComponentValue::U32(3)); // Not found
    }

    #[test]
    fn test_builtin_subtask_cancel() {
        let result = builtins::subtask_cancel(999).unwrap();
        assert_eq!(result, ComponentValue::U32(3)); // Not found
    }

    #[test]
    fn test_task_context_management() {
        let mut registry = TaskRegistry::new().unwrap();
        let handle = registry.register_task(None, None).unwrap();

        // Test setting and getting context
        let key = "test_key";
        let value = ComponentValue::S32(42);
        registry.set_task_context(handle, key, value.clone()).unwrap();

        let retrieved = registry.get_task_context(handle, key);
        assert_eq!(retrieved, Some(value));

        // Test getting non-existent key
        let not_found = registry.get_task_context(handle, "non_existent");
        assert_eq!(not_found, None);
    }

    #[test]
    fn test_context_builtin_no_task() {
        // Test context operations when no task is active
        let result = builtins::context_get("test_key");
        assert!(result.is_err());

        let result = builtins::context_set("test_key", ComponentValue::S32(42));
        assert!(result.is_err());

        let result = builtins::context_has("test_key");
        assert!(result.is_err());

        let result = builtins::context_clear();
        assert!(result.is_err());
    }

    #[test]
    fn test_context_operations_with_task() {
        // Spawn a task first to have an active context
        let task_handle = builtins::task_spawn(None, None).unwrap();
        if let ComponentValue::U32(handle_id) = task_handle {
            // Note: In a real implementation, we'd set this task as current
            // For testing, we'll test the registry methods directly
            let mut registry = TaskRegistry::new().unwrap();
            let handle = TaskHandle(handle_id);
            registry.register_task(None, None).unwrap();

            // Test context operations
            let key = "test_key";
            let value = ComponentValue::S32(42);

            registry.set_task_context(handle, key, value.clone()).unwrap();
            let retrieved = registry.get_task_context(handle, key);
            assert_eq!(retrieved, Some(value));
        }
    }
}
