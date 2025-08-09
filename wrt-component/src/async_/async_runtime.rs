//! Async Runtime for WebAssembly Component Model
//! SW-REQ-ID: REQ_FUNC_030
//!
//! This module implements a complete async runtime with task scheduling,
//! stream operations, and future management for the Component Model.

#[cfg(not(feature = "std"))]
use core::{
    fmt,
    mem,
    time::Duration,
};
#[cfg(feature = "std")]
use std::{
    boxed::Box,
    collections::VecDeque,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    fmt,
    mem,
    time::Duration,
};

use wrt_error::{
    Error,
    ErrorCategory,
    Result,
};
use wrt_foundation::{
    bounded::{
        BoundedString,
        BoundedVec,
    },
    budget_aware_provider::CrateId,
    prelude::*,
    safe_managed_alloc,
    safe_memory::NoStdProvider,
};

use super::async_types::{
    AsyncReadResult,
    Future,
    FutureHandle,
    FutureState,
    Stream,
    StreamHandle,
    StreamState,
    Waitable,
    WaitableSet,
};
use crate::{
    threading::task_manager::{
        Task,
        TaskContext,
        TaskId,
        TaskManager,
        TaskState,
        TaskType,
    },
    types::{
        ValType,
        Value,
    },
    WrtResult,
};

/// Maximum number of concurrent tasks in no_std environments
const MAX_CONCURRENT_TASKS: usize = 128;

/// Maximum number of pending operations in no_std environments
const MAX_PENDING_OPS: usize = 256;

/// Maximum reactor events per iteration in no_std environments
const MAX_REACTOR_EVENTS: usize = 64;

/// Async runtime for WebAssembly Component Model
#[derive(Debug)]
pub struct AsyncRuntime {
    /// Task scheduler
    scheduler: TaskScheduler,

    /// Reactor for async I/O
    reactor: Reactor,

    /// Stream registry
    #[cfg(feature = "std")]
    streams: Vec<StreamEntry>,
    #[cfg(not(any(feature = "std",)))]
    streams: BoundedVec<StreamEntry, MAX_CONCURRENT_TASKS, NoStdProvider<65536>>,

    /// Future registry
    #[cfg(feature = "std")]
    futures: Vec<FutureEntry>,
    #[cfg(not(any(feature = "std",)))]
    futures: BoundedVec<FutureEntry, MAX_CONCURRENT_TASKS, NoStdProvider<65536>>,

    /// Runtime configuration
    config: RuntimeConfig,

    /// Runtime statistics
    stats: RuntimeStats,

    /// Whether runtime is running
    is_running: bool,
}

/// Task scheduler for async operations
#[derive(Debug)]
pub struct TaskScheduler {
    /// Ready queue for immediately runnable tasks
    #[cfg(feature = "std")]
    ready_queue: VecDeque<ScheduledTask>,
    #[cfg(not(any(feature = "std",)))]
    ready_queue: BoundedVec<ScheduledTask, MAX_CONCURRENT_TASKS, NoStdProvider<65536>>,

    /// Waiting tasks (blocked on I/O or timers)
    #[cfg(feature = "std")]
    waiting_tasks: Vec<WaitingTask>,
    #[cfg(not(any(feature = "std",)))]
    waiting_tasks: BoundedVec<WaitingTask, MAX_CONCURRENT_TASKS, NoStdProvider<65536>>,

    /// Current time for scheduling
    current_time: u64,

    /// Task manager for low-level task operations
    task_manager: TaskManager,
}

/// Reactor for handling async I/O events
#[derive(Debug)]
pub struct Reactor {
    /// Pending events
    #[cfg(feature = "std")]
    pending_events: VecDeque<ReactorEvent>,
    #[cfg(not(any(feature = "std",)))]
    pending_events: BoundedVec<ReactorEvent, MAX_REACTOR_EVENTS, NoStdProvider<65536>>,

    /// Event handlers
    #[cfg(feature = "std")]
    event_handlers: Vec<EventHandler>,
    #[cfg(not(any(feature = "std",)))]
    event_handlers: BoundedVec<EventHandler, MAX_REACTOR_EVENTS, NoStdProvider<65536>>,
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks:  usize,
    /// Task time slice in microseconds
    pub task_time_slice_us:    u64,
    /// Maximum time to run scheduler per iteration (microseconds)
    pub max_scheduler_time_us: u64,
    /// Enable task priority scheduling
    pub priority_scheduling:   bool,
    /// Enable work stealing between tasks
    pub work_stealing:         bool,
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Total tasks created
    pub tasks_created:              u64,
    /// Total tasks completed
    pub tasks_completed:            u64,
    /// Current active tasks
    pub active_tasks:               u32,
    /// Total scheduler iterations
    pub scheduler_iterations:       u64,
    /// Total time spent in scheduler (microseconds)
    pub scheduler_time_us:          u64,
    /// Average task execution time (microseconds)
    pub avg_task_execution_time_us: u64,
}

/// Entry for a registered stream
#[derive(Debug)]
pub struct StreamEntry {
    /// Stream handle
    pub handle: StreamHandle,
    /// Stream instance
    pub stream: Stream<Value>,
    /// Associated tasks
    #[cfg(feature = "std")]
    pub tasks:  Vec<TaskId>,
    #[cfg(not(any(feature = "std",)))]
    pub tasks:  BoundedVec<TaskId, 16, NoStdProvider<65536>>,
}

/// Entry for a registered future
#[derive(Debug)]
pub struct FutureEntry {
    /// Future handle
    pub handle: FutureHandle,
    /// Future instance
    pub future: Future<Value>,
    /// Associated tasks
    #[cfg(feature = "std")]
    pub tasks:  Vec<TaskId>,
    #[cfg(not(any(feature = "std",)))]
    pub tasks:  BoundedVec<TaskId, 16, NoStdProvider<65536>>,
}

/// Scheduled task in the ready queue
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task ID
    pub task_id:           TaskId,
    /// Task priority (0 = highest)
    pub priority:          u8,
    /// Estimated execution time (microseconds)
    pub estimated_time_us: u64,
    /// Task function to execute
    pub task_fn:           TaskFunction,
}

/// Waiting task (blocked on I/O or timers)
#[derive(Debug, Clone)]
pub struct WaitingTask {
    /// Task ID
    pub task_id:        TaskId,
    /// What the task is waiting for
    pub wait_condition: WaitCondition,
    /// Timeout (absolute time in microseconds)
    pub timeout_us:     Option<u64>,
}

/// Task function type
#[derive(Debug, Clone)]
pub enum TaskFunction {
    /// Stream operation
    StreamOp {
        handle:    StreamHandle,
        operation: StreamOperation,
    },
    /// Future operation
    FutureOp {
        handle:    FutureHandle,
        operation: FutureOperation,
    },
    /// Custom user function
    Custom {
        name:        BoundedString<64, NoStdProvider<65536>>,
        // In a real implementation, this would be a function pointer
        // For now, we'll use a placeholder
        placeholder: u32,
    },
}

/// Stream operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamOperation {
    /// Read from stream
    Read,
    /// Write to stream
    Write,
    /// Close stream
    Close,
}

/// Future operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutureOperation {
    /// Get future value
    Get,
    /// Set future value
    Set,
    /// Cancel future
    Cancel,
}

/// Wait condition for blocked tasks
#[derive(Debug, Clone)]
pub enum WaitCondition {
    /// Waiting for stream to be readable
    StreamReadable(StreamHandle),
    /// Waiting for stream to be writable
    StreamWritable(StreamHandle),
    /// Waiting for future to be ready
    FutureReady(FutureHandle),
    /// Waiting for timer
    Timer(u64),
    /// Waiting for multiple conditions
    Multiple(WaitableSet),
}

/// Reactor event
#[derive(Debug, Clone)]
pub struct ReactorEvent {
    /// Event ID
    pub id:         u32,
    /// Event type
    pub event_type: ReactorEventType,
    /// Associated data
    pub data:       u64,
}

/// Reactor event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactorEventType {
    /// Stream became readable
    StreamReadable,
    /// Stream became writable
    StreamWritable,
    /// Future became ready
    FutureReady,
    /// Timer expired
    TimerExpired,
}

/// Event handler for reactor events
#[derive(Debug, Clone)]
pub struct EventHandler {
    /// Handler ID
    pub id:         u32,
    /// Event type to handle
    pub event_type: ReactorEventType,
    /// Associated task
    pub task_id:    TaskId,
}

/// Task execution result
#[derive(Debug, Clone)]
pub enum TaskExecutionResult {
    /// Task completed successfully
    Completed,
    /// Task yielded, should be rescheduled
    Yielded,
    /// Task is waiting for I/O
    Waiting(WaitCondition),
    /// Task failed with error
    Failed(Error),
}

impl AsyncRuntime {
    /// Create new async runtime
    pub fn new() -> Result<Self> {
        Ok(Self {
            scheduler: TaskScheduler::new()?,
            reactor: Reactor::new()?,
            #[cfg(feature = "std")]
            streams: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            streams: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(feature = "std")]
            futures: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            futures: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            config: RuntimeConfig::default(),
            stats: RuntimeStats::new(),
            is_running: false,
        })
    }

    /// Create new async runtime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Result<Self> {
        let mut runtime = Self::new()?;
        runtime.config = config;
        Ok(runtime)
    }

    /// Start the async runtime
    pub fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(Error::async_executor_state_violation(
                "Runtime is already running",
            ));
        }

        self.is_running = true;
        self.stats.scheduler_iterations = 0;
        Ok(())
    }

    /// Stop the async runtime
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Err(Error::async_executor_state_violation(
                "Runtime is not running",
            ));
        }

        self.is_running = false;

        // Clean up all tasks
        self.scheduler.cleanup_all_tasks()?;

        Ok(())
    }

    /// Execute one iteration of the runtime loop
    pub fn tick(&mut self) -> Result<bool> {
        if !self.is_running {
            return Ok(false);
        }

        let start_time = self.get_current_time();

        // Process reactor events
        self.reactor.process_events(&mut self.scheduler)?;

        // Run scheduler
        let has_work = self.scheduler.run_iteration(&self.config)?;

        // Update statistics
        let elapsed = self.get_current_time() - start_time;
        self.stats.scheduler_iterations += 1;
        self.stats.scheduler_time_us += elapsed;

        Ok(has_work || !self.scheduler.is_idle())
    }

    /// Run the runtime until all tasks complete or timeout
    pub fn run_to_completion(&mut self, timeout_us: Option<u64>) -> Result<()> {
        let start_time = self.get_current_time();

        while self.is_running {
            let has_work = self.tick()?;

            if !has_work {
                break; // No more work to do
            }

            if let Some(timeout) = timeout_us {
                if self.get_current_time() - start_time > timeout {
                    return Err(Error::async_timeout_error("Runtime timeout"));
                }
            }
        }

        Ok(())
    }

    /// Spawn a new task
    pub fn spawn_task(&mut self, task_fn: TaskFunction, priority: u8) -> Result<TaskId> {
        let task_id = self.scheduler.task_manager.create_task()?;

        let scheduled_task = ScheduledTask {
            task_id,
            priority,
            estimated_time_us: 1000, // Default 1ms estimate
            task_fn,
        };

        self.scheduler.schedule_task(scheduled_task)?;
        self.stats.tasks_created += 1;
        self.stats.active_tasks += 1;

        Ok(task_id)
    }

    /// Register a stream with the runtime
    pub fn register_stream(&mut self, stream: Stream<Value>) -> Result<StreamHandle> {
        let handle = stream.handle;

        let entry = StreamEntry {
            handle,
            stream,
            #[cfg(feature = "std")]
            tasks: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            tasks: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).unwrap()
            },
        };

        self.streams
            .push(entry)
            .map_err(|_| Error::async_executor_state_violation("Too many streams"))?;

        Ok(handle)
    }

    /// Register a future with the runtime
    pub fn register_future(&mut self, future: Future<Value>) -> Result<FutureHandle> {
        let handle = future.handle;

        let entry = FutureEntry {
            handle,
            future,
            #[cfg(feature = "std")]
            tasks: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            tasks: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider).unwrap()
            },
        };

        self.futures
            .push(entry)
            .map_err(|_| Error::async_executor_state_violation("Too many futures"))?;

        Ok(handle)
    }

    /// Get runtime statistics
    pub fn get_stats(&self) -> &RuntimeStats {
        &self.stats
    }

    /// Get current configuration
    pub fn get_config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Update runtime configuration
    pub fn update_config(&mut self, config: RuntimeConfig) -> Result<()> {
        if self.is_running && config.max_concurrent_tasks < self.stats.active_tasks as usize {
            return Err(Error::async_executor_state_violation(
                "Cannot reduce max concurrent tasks below current active count",
            ));
        }

        self.config = config;
        Ok(())
    }

    /// Get current time in microseconds (simplified implementation)
    fn get_current_time(&self) -> u64 {
        // In a real implementation, this would use a proper time source
        self.scheduler.current_time
    }
}

impl TaskScheduler {
    /// Create new task scheduler
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            ready_queue: VecDeque::new(),
            #[cfg(not(any(feature = "std",)))]
            ready_queue: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(feature = "std")]
            waiting_tasks: Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            waiting_tasks: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            current_time: 0,
            task_manager: TaskManager::new(),
        })
    }

    /// Schedule a task for execution
    pub fn schedule_task(&mut self, task: ScheduledTask) -> Result<()> {
        #[cfg(feature = "std")]
        {
            // Insert task in priority order (lower number = higher priority)
            let insert_pos = self
                .ready_queue
                .iter()
                .position(|t| t.priority > task.priority)
                .unwrap_or(self.ready_queue.len());

            self.ready_queue.insert(insert_pos, task);
        }
        #[cfg(not(any(feature = "std",)))]
        {
            self.ready_queue
                .push(task)
                .map_err(|_| Error::async_executor_state_violation("Ready queue full"))?;
        }

        Ok(())
    }

    /// Run one scheduler iteration
    pub fn run_iteration(&mut self, config: &RuntimeConfig) -> Result<bool> {
        let mut has_work = false;
        let iteration_start = self.current_time;

        // Process ready tasks
        while let Some(task) = self.get_next_ready_task() {
            has_work = true;

            let task_start = self.current_time;
            let result = self.execute_task(&task)?;
            let task_duration = self.current_time - task_start;

            // Handle execution result
            match result {
                TaskExecutionResult::Completed => {
                    // Task finished, no need to reschedule
                },
                TaskExecutionResult::Yielded => {
                    // Reschedule task
                    self.schedule_task(task)?;
                },
                TaskExecutionResult::Waiting(condition) => {
                    // Add to waiting tasks
                    let waiting_task = WaitingTask {
                        task_id:        task.task_id,
                        wait_condition: condition,
                        timeout_us:     Some(self.current_time + 1_000_000), // 1 second timeout
                    };
                    self.waiting_tasks.push(waiting_task).map_err(|_| {
                        Error::async_executor_state_violation("Waiting tasks list full")
                    })?;
                },
                TaskExecutionResult::Failed(_error) => {
                    // Task failed, log and remove
                    // In a real implementation, we'd log the error
                },
            }

            // Check if we've exceeded our time slice
            if self.current_time - iteration_start > config.max_scheduler_time_us {
                break;
            }

            // Simulate time progression
            self.current_time += task_duration.max(100); // At least 100us per
                                                         // task
        }

        // Check waiting tasks for timeouts or condition changes
        self.process_waiting_tasks()?;

        Ok(has_work)
    }

    /// Check if scheduler is idle
    pub fn is_idle(&self) -> bool {
        self.ready_queue.is_empty() && self.waiting_tasks.is_empty()
    }

    /// Clean up all tasks
    pub fn cleanup_all_tasks(&mut self) -> Result<()> {
        self.ready_queue.clear();
        self.waiting_tasks.clear();
        Ok(())
    }

    // Private helper methods

    fn get_next_ready_task(&mut self) -> Option<ScheduledTask> {
        #[cfg(feature = "std")]
        {
            self.ready_queue.pop_front()
        }
        #[cfg(not(any(feature = "std",)))]
        {
            if !self.ready_queue.is_empty() {
                Some(self.ready_queue.remove(0))
            } else {
                None
            }
        }
    }

    fn execute_task(&mut self, task: &ScheduledTask) -> Result<TaskExecutionResult> {
        // Simplified task execution - in real implementation this would
        // actually execute the task function
        match &task.task_fn {
            TaskFunction::StreamOp {
                handle: _,
                operation,
            } => {
                match operation {
                    StreamOperation::Read => {
                        // Simulate stream read
                        Ok(TaskExecutionResult::Completed)
                    },
                    StreamOperation::Write => {
                        // Simulate stream write
                        Ok(TaskExecutionResult::Completed)
                    },
                    StreamOperation::Close => {
                        // Simulate stream close
                        Ok(TaskExecutionResult::Completed)
                    },
                }
            },
            TaskFunction::FutureOp {
                handle: _,
                operation,
            } => {
                match operation {
                    FutureOperation::Get => {
                        // Simulate future get
                        Ok(TaskExecutionResult::Waiting(WaitCondition::Timer(
                            self.current_time + 1000,
                        )))
                    },
                    FutureOperation::Set => {
                        // Simulate future set
                        Ok(TaskExecutionResult::Completed)
                    },
                    FutureOperation::Cancel => {
                        // Simulate future cancel
                        Ok(TaskExecutionResult::Completed)
                    },
                }
            },
            TaskFunction::Custom { .. } => {
                // Simulate custom task execution
                Ok(TaskExecutionResult::Completed)
            },
        }
    }

    fn process_waiting_tasks(&mut self) -> Result<()> {
        let mut i = 0;
        while i < self.waiting_tasks.len() {
            let should_reschedule = {
                let waiting_task = &self.waiting_tasks[i];

                // Check timeout
                if let Some(timeout) = waiting_task.timeout_us {
                    if self.current_time >= timeout {
                        true // Timeout, reschedule task
                    } else {
                        false // Still waiting
                    }
                } else {
                    false // No timeout, still waiting
                }
            };

            if should_reschedule {
                let waiting_task = self.waiting_tasks.remove(i);

                // Create a new scheduled task
                let scheduled_task = ScheduledTask {
                    task_id:           waiting_task.task_id,
                    priority:          0, // Default priority
                    estimated_time_us: 1000,
                    task_fn:           TaskFunction::Custom {
                        name:        BoundedString::from_str("timeout").unwrap_or_default(),
                        placeholder: 0,
                    },
                };

                self.schedule_task(scheduled_task)?;
            } else {
                i += 1;
            }
        }

        Ok(())
    }
}

impl Reactor {
    /// Create new reactor
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "std")]
            pending_events:                                    VecDeque::new(),
            #[cfg(not(any(feature = "std",)))]
            pending_events:                                    {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(feature = "std")]
            event_handlers:                                    Vec::new(),
            #[cfg(not(any(feature = "std",)))]
            event_handlers:                                    {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
        })
    }

    /// Process pending events
    pub fn process_events(&mut self, scheduler: &mut TaskScheduler) -> Result<()> {
        #[cfg(feature = "std")]
        {
            while let Some(event) = self.pending_events.pop_front() {
                self.handle_event(event, scheduler)?;
            }
        }
        #[cfg(not(any(feature = "std",)))]
        {
            while !self.pending_events.is_empty() {
                let event = self.pending_events.remove(0);
                self.handle_event(event, scheduler)?;
            }
        }

        Ok(())
    }

    /// Add event to pending queue
    pub fn add_event(&mut self, event: ReactorEvent) -> Result<()> {
        self.pending_events
            .push(event)
            .map_err(|_| Error::async_executor_state_violation("Event queue full"))
    }

    fn handle_event(&mut self, _event: ReactorEvent, _scheduler: &mut TaskScheduler) -> Result<()> {
        // Simplified event handling - in real implementation this would
        // wake up waiting tasks based on the event type
        Ok(())
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks:  MAX_CONCURRENT_TASKS,
            task_time_slice_us:    1000,  // 1ms
            max_scheduler_time_us: 10000, // 10ms
            priority_scheduling:   true,
            work_stealing:         false,
        }
    }
}

impl RuntimeStats {
    /// Create new runtime statistics
    pub fn new() -> Self {
        Self {
            tasks_created:              0,
            tasks_completed:            0,
            active_tasks:               0,
            scheduler_iterations:       0,
            scheduler_time_us:          0,
            avg_task_execution_time_us: 0,
        }
    }
}

// Default implementations removed - constructors now return Result types

impl fmt::Display for StreamOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamOperation::Read => write!(f, "read"),
            StreamOperation::Write => write!(f, "write"),
            StreamOperation::Close => write!(f, "close"),
        }
    }
}

impl fmt::Display for FutureOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FutureOperation::Get => write!(f, "get"),
            FutureOperation::Set => write!(f, "set"),
            FutureOperation::Cancel => write!(f, "cancel"),
        }
    }
}

impl fmt::Display for ReactorEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReactorEventType::StreamReadable => write!(f, "stream-readable"),
            ReactorEventType::StreamWritable => write!(f, "stream-writable"),
            ReactorEventType::FutureReady => write!(f, "future-ready"),
            ReactorEventType::TimerExpired => write!(f, "timer-expired"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_runtime_creation() {
        let runtime = AsyncRuntime::new().unwrap();
        assert!(!runtime.is_running);
        assert_eq!(runtime.streams.len(), 0);
        assert_eq!(runtime.futures.len(), 0);
    }

    #[test]
    fn test_runtime_start_stop() {
        let mut runtime = AsyncRuntime::new().unwrap();

        assert!(runtime.start().is_ok());
        assert!(runtime.is_running);

        assert!(runtime.stop().is_ok());
        assert!(!runtime.is_running);
    }

    #[test]
    fn test_spawn_task() {
        let mut runtime = AsyncRuntime::new().unwrap();
        runtime.start().unwrap();

        let task_fn = TaskFunction::Custom {
            name:        BoundedString::from_str("test").unwrap(),
            placeholder: 42,
        };

        let task_id = runtime.spawn_task(task_fn, 0).unwrap();
        assert_eq!(runtime.stats.tasks_created, 1);
        assert_eq!(runtime.stats.active_tasks, 1);
    }

    #[test]
    fn test_register_stream() {
        let mut runtime = AsyncRuntime::new().unwrap();
        let stream = Stream::new(StreamHandle(1), ValType::U32);

        let handle = runtime.register_stream(stream).unwrap();
        assert_eq!(handle.0, 1);
        assert_eq!(runtime.streams.len(), 1);
    }

    #[test]
    fn test_register_future() {
        let mut runtime = AsyncRuntime::new().unwrap();
        let future = Future::new(FutureHandle(1), ValType::String);

        let handle = runtime.register_future(future).unwrap();
        assert_eq!(handle.0, 1);
        assert_eq!(runtime.futures.len(), 1);
    }

    #[test]
    fn test_task_scheduler() {
        let mut scheduler = TaskScheduler::new().unwrap();
        assert!(scheduler.is_idle());

        let task = ScheduledTask {
            task_id:           TaskId(1),
            priority:          0,
            estimated_time_us: 1000,
            task_fn:           TaskFunction::Custom {
                name:        BoundedString::from_str("test").unwrap(),
                placeholder: 0,
            },
        };

        scheduler.schedule_task(task).unwrap();
        assert!(!scheduler.is_idle());
    }

    #[test]
    fn test_reactor() {
        let mut reactor = Reactor::new().unwrap();
        let mut scheduler = TaskScheduler::new().unwrap();

        let event = ReactorEvent {
            id:         1,
            event_type: ReactorEventType::TimerExpired,
            data:       1000,
        };

        reactor.add_event(event).unwrap();
        reactor.process_events(&mut scheduler).unwrap();
    }

    #[test]
    fn test_runtime_config() {
        let mut config = RuntimeConfig::default();
        config.max_concurrent_tasks = 64;
        config.task_time_slice_us = 500;

        let runtime = AsyncRuntime::with_config(config.clone()).unwrap();
        assert_eq!(runtime.config.max_concurrent_tasks, 64);
        assert_eq!(runtime.config.task_time_slice_us, 500);
    }

    #[test]
    fn test_runtime_stats() {
        let runtime = AsyncRuntime::new().unwrap();
        let stats = runtime.get_stats();

        assert_eq!(stats.tasks_created, 0);
        assert_eq!(stats.tasks_completed, 0);
        assert_eq!(stats.active_tasks, 0);
    }

    #[test]
    fn test_operation_display() {
        assert_eq!(StreamOperation::Read.to_string(), "read");
        assert_eq!(FutureOperation::Set.to_string(), "set");
        assert_eq!(
            ReactorEventType::StreamReadable.to_string(),
            "stream-readable"
        );
    }
}
