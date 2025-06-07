//! Task management for WebAssembly Component Model async operations
//! SW-REQ-ID: REQ_FUNC_031
//!
//! This module implements the task management system required for async support
//! in the Component Model MVP specification.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(feature = "std")]
use std::{boxed::Box, collections::BTreeMap, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, component_value::ComponentValue, prelude::*, resource::ResourceHandle,
};

use crate::{
    async_types::{
        AsyncReadResult, ErrorContext, ErrorContextHandle, Future, FutureHandle, Stream,
        StreamHandle, Waitable, WaitableSet,
    },
    resource_lifecycle::ResourceLifecycleManager,
    types::{ValType, Value},
    WrtResult,
};

/// Maximum number of tasks in no_std environments
const MAX_TASKS: usize = 256;

/// Maximum number of subtasks per task in no_std environments
const MAX_SUBTASKS: usize = 32;

/// Maximum call stack depth per task
const MAX_TASK_CALL_DEPTH: usize = 64;

/// Task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(pub u32);

/// Task management system
pub struct TaskManager {
    /// All tasks in the system
    #[cfg(feature = "std")]
    tasks: BTreeMap<TaskId, Task>,
    #[cfg(not(any(feature = "std", )))]
    tasks: BoundedVec<(TaskId, Task), MAX_TASKS>,

    /// Ready queue for runnable tasks
    #[cfg(feature = "std")]
    ready_queue: Vec<TaskId>,
    #[cfg(not(any(feature = "std", )))]
    ready_queue: BoundedVec<TaskId, MAX_TASKS>,

    /// Currently executing task
    current_task: Option<TaskId>,

    /// Next task ID
    next_task_id: u32,

    /// Resource manager for task-owned resources
    resource_manager: ResourceLifecycleManager,

    /// Maximum number of concurrent tasks
    max_concurrent_tasks: usize,
}

/// Task state
#[derive(Debug, Clone)]
pub struct Task {
    /// Task ID
    pub id: TaskId,
    /// Task state
    pub state: TaskState,
    /// Task type
    pub task_type: TaskType,
    /// Parent task (if this is a subtask)
    pub parent: Option<TaskId>,
    /// Subtasks spawned by this task
    #[cfg(feature = "std")]
    pub subtasks: Vec<TaskId>,
    #[cfg(not(any(feature = "std", )))]
    pub subtasks: BoundedVec<TaskId, MAX_SUBTASKS>,
    /// Borrowed resource handles
    #[cfg(feature = "std")]
    pub borrowed_handles: Vec<ResourceHandle>,
    #[cfg(not(any(feature = "std", )))]
    pub borrowed_handles: BoundedVec<ResourceHandle, 64>,
    /// Task-local storage
    pub context: TaskContext,
    /// Waiting on waitables
    pub waiting_on: Option<WaitableSet>,
    /// Return values (when completed)
    #[cfg(feature = "std")]
    pub return_values: Option<Vec<Value>>,
    #[cfg(not(any(feature = "std", )))]
    pub return_values: Option<BoundedVec<Value, 16>>,
    /// Error context (if failed)
    pub error_context: Option<ErrorContextHandle>,
}

/// Task state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is being created
    Starting,
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is waiting for I/O or other async operation
    Waiting,
    /// Task has completed successfully
    Completed,
    /// Task was cancelled
    Cancelled,
    /// Task failed with error
    Failed,
}

/// Task type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Main component function call
    ComponentFunction,
    /// Async operation (stream/future handling)
    AsyncOperation,
    /// Background task
    Background,
    /// Cleanup task
    Cleanup,
}

/// Task-local context and storage
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Component instance that owns this task
    pub component_instance: u32,
    /// Function being executed
    pub function_index: Option<u32>,
    /// Call stack for this task
    #[cfg(feature = "std")]
    pub call_stack: Vec<CallFrame>,
    #[cfg(not(any(feature = "std", )))]
    pub call_stack: BoundedVec<CallFrame, MAX_TASK_CALL_DEPTH>,
    /// Task-local storage
    #[cfg(feature = "std")]
    pub storage: BTreeMap<String, ComponentValue>,
    #[cfg(not(any(feature = "std", )))]
    pub storage: BoundedVec<(BoundedString<64>, ComponentValue), 32>,
    /// Task creation time (simplified)
    pub created_at: u64,
    /// Task deadline (if any)
    pub deadline: Option<u64>,
}

/// Call frame for task call stack
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Function being called
    pub function_index: u32,
    /// Component instance
    pub component_instance: u32,
    /// Local variables
    #[cfg(feature = "std")]
    pub locals: Vec<Value>,
    #[cfg(not(any(feature = "std", )))]
    pub locals: BoundedVec<Value, 32>,
    /// Return address
    pub return_address: Option<u32>,
}

/// Task execution result
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Task completed with values
    Completed(Vec<Value>),
    /// Task is waiting for I/O
    Waiting(WaitableSet),
    /// Task yielded voluntarily
    Yielded,
    /// Task was cancelled
    Cancelled,
    /// Task failed with error
    Failed(ErrorContextHandle),
}

impl TaskManager {
    /// Create a new task manager
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            tasks: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            tasks: BoundedVec::new(),
            #[cfg(feature = "std")]
            ready_queue: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            ready_queue: BoundedVec::new(),
            current_task: None,
            next_task_id: 0,
            resource_manager: ResourceLifecycleManager::new(),
            max_concurrent_tasks: MAX_TASKS,
        }
    }

    /// Set maximum concurrent tasks
    pub fn set_max_concurrent_tasks(&mut self, max: usize) {
        self.max_concurrent_tasks = max;
    }

    /// Spawn a new task
    pub fn spawn_task(
        &mut self,
        task_type: TaskType,
        component_instance: u32,
        function_index: Option<u32>,
    ) -> WrtResult<TaskId> {
        // Check task limit
        if self.tasks.len() >= self.max_concurrent_tasks {
            return Err(wrt_foundation::WrtError::ResourceExhausted(
                "Maximum concurrent tasks reached".into(),
            ));
        }

        let task_id = TaskId(self.next_task_id);
        self.next_task_id += 1;

        let task = Task {
            id: task_id,
            state: TaskState::Starting,
            task_type,
            parent: self.current_task,
            #[cfg(feature = "std")]
            subtasks: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            subtasks: BoundedVec::new(),
            #[cfg(feature = "std")]
            borrowed_handles: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            borrowed_handles: BoundedVec::new(),
            context: TaskContext {
                component_instance,
                function_index,
                #[cfg(feature = "std")]
                call_stack: Vec::new(),
                #[cfg(not(any(feature = "std", )))]
                call_stack: BoundedVec::new(),
                #[cfg(feature = "std")]
                storage: BTreeMap::new(),
                #[cfg(not(any(feature = "std", )))]
                storage: BoundedVec::new(),
                created_at: self.get_current_time(),
                deadline: None,
            },
            waiting_on: None,
            return_values: None,
            error_context: None,
        };

        // Add to parent's subtasks
        if let Some(parent_id) = self.current_task {
            if let Some(parent_task) = self.get_task_mut(parent_id) {
                #[cfg(feature = "std")]
                {
                    parent_task.subtasks.push(task_id);
                }
                #[cfg(not(any(feature = "std", )))]
                {
                    let _ = parent_task.subtasks.push(task_id);
                }
            }
        }

        // Insert task
        #[cfg(feature = "std")]
        {
            self.tasks.insert(task_id, task);
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.tasks.push((task_id, task)).map_err(|_| {
                wrt_foundation::WrtError::ResourceExhausted("Task storage full".into())
            })?;
        }

        // Mark as ready
        self.make_ready(task_id)?;

        Ok(task_id)
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: TaskId) -> Option<&Task> {
        #[cfg(feature = "std")]
        {
            self.tasks.get(&task_id)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.tasks.iter().find(|(id, _)| *id == task_id).map(|(_, task)| task)
        }
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Task> {
        #[cfg(feature = "std")]
        {
            self.tasks.get_mut(&task_id)
        }
        #[cfg(not(any(feature = "std", )))]
        {
            self.tasks.iter_mut().find(|(id, _)| *id == task_id).map(|(_, task)| task)
        }
    }

    /// Make a task ready to run
    pub fn make_ready(&mut self, task_id: TaskId) -> WrtResult<()> {
        if let Some(task) = self.get_task_mut(task_id) {
            if task.state == TaskState::Starting || task.state == TaskState::Waiting {
                task.state = TaskState::Ready;

                #[cfg(feature = "std")]
                {
                    self.ready_queue.push(task_id);
                }
                #[cfg(not(any(feature = "std", )))]
                {
                    self.ready_queue.push(task_id).map_err(|_| {
                        wrt_foundation::WrtError::ResourceExhausted("Ready queue full".into())
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Get next ready task
    pub fn next_ready_task(&mut self) -> Option<TaskId> {
        #[cfg(feature = "std")]
        {
            if self.ready_queue.is_empty() {
                None
            } else {
                Some(self.ready_queue.remove(0))
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            if self.ready_queue.is_empty() {
                None
            } else {
                // Remove first element
                let task_id = self.ready_queue[0];
                for i in 1..self.ready_queue.len() {
                    self.ready_queue[i - 1] = self.ready_queue[i];
                }
                let _ = self.ready_queue.pop();
                Some(task_id)
            }
        }
    }

    /// Switch to a task (make it current)
    pub fn switch_to_task(&mut self, task_id: TaskId) -> WrtResult<()> {
        if let Some(task) = self.get_task_mut(task_id) {
            if task.state == TaskState::Ready {
                task.state = TaskState::Running;
                self.current_task = Some(task_id);
                Ok(())
            } else {
                Err(wrt_foundation::WrtError::InvalidState("Task is not ready to run".into()))
            }
        } else {
            Err(wrt_foundation::WrtError::InvalidInput("Task not found".into()))
        }
    }

    /// Complete current task with return values
    pub fn task_return(&mut self, values: Vec<Value>) -> WrtResult<()> {
        if let Some(task_id) = self.current_task {
            if let Some(task) = self.get_task_mut(task_id) {
                task.state = TaskState::Completed;
                #[cfg(feature = "std")]
                {
                    task.return_values = Some(values);
                }
                #[cfg(not(any(feature = "std", )))]
                {
                    let mut bounded_values = BoundedVec::new();
                    for value in values {
                        bounded_values.push(value).map_err(|_| {
                            wrt_foundation::WrtError::ResourceExhausted(
                                "Too many return values".into(),
                            )
                        })?;
                    }
                    task.return_values = Some(bounded_values);
                }

                // Clean up borrowed resources
                self.cleanup_task_resources(task_id)?;

                self.current_task = task.parent;
                Ok(())
            } else {
                Err(wrt_foundation::WrtError::InvalidInput("Current task not found".into()))
            }
        } else {
            Err(wrt_foundation::WrtError::InvalidState("No current task".into()))
        }
    }

    /// Wait for waitables
    pub fn task_wait(&mut self, waitables: WaitableSet) -> WrtResult<u32> {
        if let Some(task_id) = self.current_task {
            // Check if any waitables are immediately ready
            if let Some(ready_index) = waitables.first_ready() {
                return Ok(ready_index);
            }

            // Put task in waiting state
            if let Some(task) = self.get_task_mut(task_id) {
                task.state = TaskState::Waiting;
                task.waiting_on = Some(waitables);
                self.current_task = task.parent;

                // Return special value indicating we're waiting
                Ok(u32::MAX) // Convention: MAX means "blocking"
            } else {
                Err(wrt_foundation::WrtError::InvalidInput("Current task not found".into()))
            }
        } else {
            Err(wrt_foundation::WrtError::InvalidState("No current task".into()))
        }
    }

    /// Poll waitables without blocking
    pub fn task_poll(&self, waitables: &WaitableSet) -> WrtResult<Option<u32>> {
        Ok(waitables.first_ready())
    }

    /// Yield current task voluntarily
    pub fn task_yield(&mut self) -> WrtResult<()> {
        if let Some(task_id) = self.current_task {
            if let Some(task) = self.get_task_mut(task_id) {
                task.state = TaskState::Ready;

                // Add back to ready queue
                #[cfg(feature = "std")]
                {
                    self.ready_queue.push(task_id);
                }
                #[cfg(not(any(feature = "std", )))]
                {
                    let _ = self.ready_queue.push(task_id);
                }

                self.current_task = task.parent;
                Ok(())
            } else {
                Err(wrt_foundation::WrtError::InvalidInput("Current task not found".into()))
            }
        } else {
            Err(wrt_foundation::WrtError::InvalidState("No current task".into()))
        }
    }

    /// Cancel a task
    pub fn task_cancel(&mut self, task_id: TaskId) -> WrtResult<()> {
        if let Some(task) = self.get_task_mut(task_id) {
            if task.state != TaskState::Completed && task.state != TaskState::Failed {
                task.state = TaskState::Cancelled;

                // Cancel all subtasks
                let subtasks = task.subtasks.clone();
                for subtask_id in subtasks {
                    self.task_cancel(subtask_id)?;
                }

                // Clean up resources
                self.cleanup_task_resources(task_id)?;

                // If this was the current task, switch to parent
                if self.current_task == Some(task_id) {
                    self.current_task = task.parent;
                }
            }
        }
        Ok(())
    }

    /// Handle backpressure for a task
    pub fn task_backpressure(&mut self) -> WrtResult<()> {
        // Simple backpressure: yield current task
        self.task_yield()
    }

    /// Update waitable states and wake waiting tasks
    pub fn update_waitables(&mut self) -> WrtResult<()> {
        let mut tasks_to_wake = Vec::new();

        // Check all waiting tasks
        #[cfg(feature = "std")]
        {
            for (task_id, task) in &mut self.tasks {
                if task.state == TaskState::Waiting {
                    if let Some(ref mut waitables) = task.waiting_on {
                        if waitables.has_ready() {
                            tasks_to_wake.push(*task_id);
                        }
                    }
                }
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            for (task_id, task) in &mut self.tasks {
                if task.state == TaskState::Waiting {
                    if let Some(ref mut waitables) = task.waiting_on {
                        if waitables.has_ready() {
                            tasks_to_wake.push(*task_id);
                        }
                    }
                }
            }
        }

        // Wake ready tasks
        for task_id in tasks_to_wake {
            self.make_ready(task_id)?;
        }

        Ok(())
    }

    /// Clean up resources owned by a task
    fn cleanup_task_resources(&mut self, task_id: TaskId) -> WrtResult<()> {
        if let Some(task) = self.get_task(task_id) {
            // Drop borrowed resources
            for handle in &task.borrowed_handles {
                // In a real implementation, would properly release borrows
                let _ = self.resource_manager.drop_resource(*handle);
            }
        }
        Ok(())
    }

    /// Get current time (simplified)
    fn get_current_time(&self) -> u64 {
        // In a real implementation, would use proper time measurement
        0
    }

    /// Get current task ID
    pub fn current_task_id(&self) -> Option<TaskId> {
        self.current_task
    }

    /// Get task count
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get ready task count
    pub fn ready_task_count(&self) -> usize {
        self.ready_queue.len()
    }

    /// Check if there are any ready tasks
    pub fn has_ready_tasks(&self) -> bool {
        !self.ready_queue.is_empty()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskState::Starting => write!(f, "starting"),
            TaskState::Ready => write!(f, "ready"),
            TaskState::Running => write!(f, "running"),
            TaskState::Waiting => write!(f, "waiting"),
            TaskState::Completed => write!(f, "completed"),
            TaskState::Cancelled => write!(f, "cancelled"),
            TaskState::Failed => write!(f, "failed"),
        }
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::ComponentFunction => write!(f, "component-function"),
            TaskType::AsyncOperation => write!(f, "async-operation"),
            TaskType::Background => write!(f, "background"),
            TaskType::Cleanup => write!(f, "cleanup"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_manager_creation() {
        let manager = TaskManager::new();
        assert_eq!(manager.task_count(), 0);
        assert_eq!(manager.ready_task_count(), 0);
        assert!(!manager.has_ready_tasks());
        assert_eq!(manager.current_task_id(), None);
    }

    #[test]
    fn test_spawn_task() {
        let mut manager = TaskManager::new();

        let task_id = manager.spawn_task(TaskType::ComponentFunction, 1, Some(0)).unwrap();

        assert_eq!(task_id, TaskId(0));
        assert_eq!(manager.task_count(), 1);
        assert_eq!(manager.ready_task_count(), 1);
        assert!(manager.has_ready_tasks());
    }

    #[test]
    fn test_task_execution_cycle() {
        let mut manager = TaskManager::new();

        // Spawn task
        let task_id = manager.spawn_task(TaskType::ComponentFunction, 1, Some(0)).unwrap();

        // Get next ready task
        let next_task = manager.next_ready_task().unwrap();
        assert_eq!(next_task, task_id);
        assert_eq!(manager.ready_task_count(), 0);

        // Switch to task
        manager.switch_to_task(task_id).unwrap();
        assert_eq!(manager.current_task_id(), Some(task_id));

        let task = manager.get_task(task_id).unwrap();
        assert_eq!(task.state, TaskState::Running);
    }

    #[test]
    fn test_task_return() {
        let mut manager = TaskManager::new();

        let task_id = manager.spawn_task(TaskType::ComponentFunction, 1, Some(0)).unwrap();

        manager.switch_to_task(task_id).unwrap();

        // Return from task
        let return_values = vec![Value::U32(42)];
        manager.task_return(return_values).unwrap();

        let task = manager.get_task(task_id).unwrap();
        assert_eq!(task.state, TaskState::Completed);
        assert!(task.return_values.is_some());
    }

    #[test]
    fn test_task_yield() {
        let mut manager = TaskManager::new();

        let task_id = manager.spawn_task(TaskType::ComponentFunction, 1, Some(0)).unwrap();

        manager.switch_to_task(task_id).unwrap();
        manager.task_yield().unwrap();

        let task = manager.get_task(task_id).unwrap();
        assert_eq!(task.state, TaskState::Ready);
        assert_eq!(manager.ready_task_count(), 1);
    }

    #[test]
    fn test_task_cancel() {
        let mut manager = TaskManager::new();

        let task_id = manager.spawn_task(TaskType::ComponentFunction, 1, Some(0)).unwrap();

        manager.task_cancel(task_id).unwrap();

        let task = manager.get_task(task_id).unwrap();
        assert_eq!(task.state, TaskState::Cancelled);
    }

    #[test]
    fn test_subtask_tracking() {
        let mut manager = TaskManager::new();

        // Spawn parent task
        let parent_id = manager.spawn_task(TaskType::ComponentFunction, 1, Some(0)).unwrap();

        manager.switch_to_task(parent_id).unwrap();

        // Spawn subtask
        let child_id = manager.spawn_task(TaskType::AsyncOperation, 1, Some(1)).unwrap();

        let parent = manager.get_task(parent_id).unwrap();
        assert!(parent.subtasks.contains(&child_id));

        let child = manager.get_task(child_id).unwrap();
        assert_eq!(child.parent, Some(parent_id));
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(TaskState::Starting.to_string(), "starting");
        assert_eq!(TaskState::Running.to_string(), "running");
        assert_eq!(TaskState::Completed.to_string(), "completed");
    }

    #[test]
    fn test_task_type_display() {
        assert_eq!(TaskType::ComponentFunction.to_string(), "component-function");
        assert_eq!(TaskType::AsyncOperation.to_string(), "async-operation");
        assert_eq!(TaskType::Background.to_string(), "background");
    }
}
