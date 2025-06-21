//! Preemptive fuel-based scheduler for async tasks
//!
//! This module provides advanced scheduling with preemption based on fuel consumption
//! and priority levels, essential for ASIL-B real-time guarantees.

use crate::{
    async_::{
        fuel_async_executor::{AsyncTaskState, FuelAsyncTask},
        fuel_async_scheduler::{SchedulingPolicy, ScheduledTask},
        fuel_priority_inheritance::{FuelPriorityInheritanceProtocol, ResourceId},
    },
    task_manager::TaskId,
    ComponentInstanceId,
    prelude::*,
};
use core::{
    cmp::Ordering as CmpOrdering,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

/// Maximum number of tasks in preemptive scheduler
const MAX_PREEMPTIVE_TASKS: usize = 128;

/// Maximum number of priority levels
const MAX_PRIORITY_LEVELS: usize = 8;

/// Fuel quantum for time slicing
const DEFAULT_FUEL_QUANTUM: u64 = 1000;

/// Fuel costs for preemptive operations
const PREEMPTION_FUEL: u64 = 20;
const CONTEXT_SWITCH_FUEL: u64 = 15;
const PRIORITY_BOOST_FUEL: u64 = 10;
const AGING_CHECK_FUEL: u64 = 5;

/// Preemptive scheduler with fuel-based time slicing
pub struct FuelPreemptiveScheduler {
    /// Tasks organized by priority level
    priority_queues: BoundedVec<TaskPriorityQueue, MAX_PRIORITY_LEVELS>,
    /// Currently running task
    current_task: Option<RunningTaskContext>,
    /// Task information indexed by TaskId
    task_info: BoundedHashMap<TaskId, PreemptiveTaskInfo, MAX_PREEMPTIVE_TASKS>,
    /// Scheduling configuration
    config: PreemptiveSchedulerConfig,
    /// Priority inheritance protocol
    priority_protocol: FuelPriorityInheritanceProtocol,
    /// Scheduler statistics
    stats: PreemptiveSchedulerStats,
    /// Current fuel time for scheduling decisions
    current_fuel_time: AtomicU64,
    /// Whether preemption is enabled
    preemption_enabled: AtomicBool,
    /// Verification level for fuel tracking
    verification_level: VerificationLevel,
}

/// Priority queue for a specific priority level
#[derive(Debug)]
pub struct TaskPriorityQueue {
    /// Priority level of this queue
    pub priority: Priority,
    /// Tasks at this priority level
    pub tasks: BoundedVec<TaskId, MAX_PREEMPTIVE_TASKS>,
    /// Round-robin position for this priority level
    pub round_robin_position: AtomicUsize,
    /// Total fuel consumed by tasks at this priority
    pub fuel_consumed: AtomicU64,
    /// Number of context switches for this priority
    pub context_switches: AtomicUsize,
}

/// Information about a task in the preemptive scheduler
#[derive(Debug, Clone)]
pub struct PreemptiveTaskInfo {
    /// Task identifier
    pub task_id: TaskId,
    /// Component owning the task
    pub component_id: ComponentInstanceId,
    /// Base priority of the task
    pub base_priority: Priority,
    /// Current effective priority (may be boosted)
    pub effective_priority: Priority,
    /// Fuel budget for the task
    pub fuel_budget: u64,
    /// Fuel consumed by the task
    pub fuel_consumed: u64,
    /// Fuel quantum for time slicing
    pub fuel_quantum: u64,
    /// Current state of the task
    pub state: AsyncTaskState,
    /// Time when task last ran
    pub last_run_time: u64,
    /// Total time the task has been running
    pub total_run_time: u64,
    /// Number of times the task has been preempted
    pub preemption_count: usize,
    /// Priority boost level (0 = no boost)
    pub priority_boost: u32,
    /// Deadline for the task (if any)
    pub deadline: Option<Duration>,
    /// Whether the task can be preempted
    pub preemptible: bool,
}

/// Currently running task context
#[derive(Debug, Clone)]
pub struct RunningTaskContext {
    /// Task identifier
    pub task_id: TaskId,
    /// When the task started running (fuel time)
    pub start_time: u64,
    /// Fuel quantum allocated for this run
    pub allocated_quantum: u64,
    /// Fuel consumed in current run
    pub consumed_quantum: u64,
    /// Priority when task started running
    pub run_priority: Priority,
    /// Whether task can be preempted
    pub preemptible: bool,
}

/// Configuration for the preemptive scheduler
#[derive(Debug, Clone)]
pub struct PreemptiveSchedulerConfig {
    /// Default fuel quantum for time slicing
    pub default_fuel_quantum: u64,
    /// Whether to enable priority aging
    pub enable_priority_aging: bool,
    /// Fuel threshold for priority aging
    pub aging_fuel_threshold: u64,
    /// Maximum priority boost level
    pub max_priority_boost: u32,
    /// Whether to enable deadline scheduling
    pub enable_deadline_scheduling: bool,
    /// Whether to enable priority inheritance
    pub enable_priority_inheritance: bool,
    /// Minimum fuel quantum (prevents starvation)
    pub min_fuel_quantum: u64,
    /// Maximum fuel quantum
    pub max_fuel_quantum: u64,
}

/// Scheduler performance statistics
#[derive(Debug, Clone)]
pub struct PreemptiveSchedulerStats {
    /// Total number of preemptions
    pub total_preemptions: AtomicUsize,
    /// Total number of context switches
    pub total_context_switches: AtomicUsize,
    /// Total number of priority boosts
    pub total_priority_boosts: AtomicUsize,
    /// Total fuel consumed by scheduler overhead
    pub scheduler_fuel_consumed: AtomicU64,
    /// Average task run time in fuel units
    pub average_task_run_time: AtomicU64,
    /// Number of deadline misses
    pub deadline_misses: AtomicUsize,
    /// Total tasks scheduled
    pub total_tasks_scheduled: AtomicUsize,
    /// Current active tasks
    pub active_tasks: AtomicUsize,
}

impl Default for PreemptiveSchedulerConfig {
    fn default() -> Self {
        Self {
            default_fuel_quantum: DEFAULT_FUEL_QUANTUM,
            enable_priority_aging: true,
            aging_fuel_threshold: 5000,
            max_priority_boost: 3,
            enable_deadline_scheduling: true,
            enable_priority_inheritance: true,
            min_fuel_quantum: 100,
            max_fuel_quantum: 10000,
        }
    }
}

impl FuelPreemptiveScheduler {
    /// Create a new preemptive scheduler
    pub fn new(
        config: PreemptiveSchedulerConfig,
        verification_level: VerificationLevel,
    ) -> Result<Self, Error> {
        let provider = safe_managed_alloc!(8192, CrateId::Component)?;
        
        // Initialize priority queues for each priority level
        let mut priority_queues = BoundedVec::new(provider.clone())?;
        let priorities = [Priority::Low, Priority::Normal, Priority::High, Priority::Critical];
        
        for &priority in &priorities {
            let queue = TaskPriorityQueue {
                priority,
                tasks: BoundedVec::new(provider.clone())?,
                round_robin_position: AtomicUsize::new(0),
                fuel_consumed: AtomicU64::new(0),
                context_switches: AtomicUsize::new(0),
            };
            priority_queues.push(queue).map_err(|_| {
                Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    "Failed to initialize priority queues".to_string(),
                )
            })?;
        }

        let priority_protocol = FuelPriorityInheritanceProtocol::new(verification_level)?;

        Ok(Self {
            priority_queues,
            current_task: None,
            task_info: BoundedHashMap::new(),
            config,
            priority_protocol,
            stats: PreemptiveSchedulerStats {
                total_preemptions: AtomicUsize::new(0),
                total_context_switches: AtomicUsize::new(0),
                total_priority_boosts: AtomicUsize::new(0),
                scheduler_fuel_consumed: AtomicU64::new(0),
                average_task_run_time: AtomicU64::new(0),
                deadline_misses: AtomicUsize::new(0),
                total_tasks_scheduled: AtomicUsize::new(0),
                active_tasks: AtomicUsize::new(0),
            },
            current_fuel_time: AtomicU64::new(0),
            preemption_enabled: AtomicBool::new(true),
            verification_level,
        })
    }

    /// Add a task to the preemptive scheduler
    pub fn add_task(
        &mut self,
        task_id: TaskId,
        component_id: ComponentInstanceId,
        base_priority: Priority,
        fuel_budget: u64,
        deadline: Option<Duration>,
        preemptible: bool,
    ) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionInsert, self.verification_level);
        self.consume_scheduler_fuel(10)?;

        let fuel_quantum = self.calculate_fuel_quantum(base_priority, fuel_budget);

        let task_info = PreemptiveTaskInfo {
            task_id,
            component_id,
            base_priority,
            effective_priority: base_priority,
            fuel_budget,
            fuel_consumed: 0,
            fuel_quantum,
            state: AsyncTaskState::Ready,
            last_run_time: 0,
            total_run_time: 0,
            preemption_count: 0,
            priority_boost: 0,
            deadline,
            preemptible,
        };

        self.task_info.insert(task_id, task_info).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many tasks in scheduler".to_string(),
            )
        })?;

        // Add to appropriate priority queue
        self.add_task_to_priority_queue(task_id, base_priority)?;

        self.stats.total_tasks_scheduled.fetch_add(1, Ordering::AcqRel);
        self.stats.active_tasks.fetch_add(1, Ordering::AcqRel);

        Ok(())
    }

    /// Select the next task to run (with preemption logic)
    pub fn schedule_next_task(&mut self) -> Result<Option<TaskId>, Error> {
        record_global_operation(OperationType::FunctionCall, self.verification_level);
        self.consume_scheduler_fuel(15)?;

        let current_time = self.current_fuel_time.load(Ordering::Acquire);

        // Check if current task should be preempted
        if let Some(ref current) = self.current_task {
            if self.should_preempt_current_task(current, current_time)? {
                self.preempt_current_task(current_time)?;
            } else {
                // Current task continues running
                return Ok(Some(current.task_id));
            }
        }

        // Select the highest priority ready task
        let next_task = self.select_highest_priority_task()?;

        if let Some(task_id) = next_task {
            self.start_task_execution(task_id, current_time)?;
        }

        Ok(next_task)
    }

    /// Update task state and handle completion
    pub fn update_task_state(
        &mut self,
        task_id: TaskId,
        new_state: AsyncTaskState,
        fuel_consumed: u64,
    ) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionMutate, self.verification_level);
        self.consume_scheduler_fuel(8)?;

        let current_time = self.current_fuel_time.fetch_add(fuel_consumed, Ordering::AcqRel);

        if let Some(task_info) = self.task_info.get_mut(&task_id) {
            task_info.fuel_consumed += fuel_consumed;
            task_info.state = new_state;

            // Update current task context if this is the running task
            if let Some(ref mut current) = self.current_task {
                if current.task_id == task_id {
                    current.consumed_quantum += fuel_consumed;
                    task_info.total_run_time += fuel_consumed;
                }
            }

            // Handle task completion or blocking
            match new_state {
                AsyncTaskState::Completed | AsyncTaskState::Failed | AsyncTaskState::Cancelled => {
                    self.remove_task_from_scheduler(task_id)?;
                }
                AsyncTaskState::Waiting => {
                    // Task is blocked, remove from ready queues
                    self.remove_task_from_priority_queues(task_id)?;
                    if let Some(ref current) = self.current_task {
                        if current.task_id == task_id {
                            self.current_task = None;
                        }
                    }
                }
                AsyncTaskState::Ready => {
                    // Task is ready, ensure it's in the right priority queue
                    self.update_task_priority_queue(task_id)?;
                }
                _ => {}
            }
        }

        // Check for priority aging
        if self.config.enable_priority_aging {
            self.check_priority_aging(current_time)?;
        }

        Ok(())
    }

    /// Check if a task should be preempted
    pub fn should_preempt_current_task(
        &self,
        current: &RunningTaskContext,
        current_time: u64,
    ) -> Result<bool, Error> {
        // Don't preempt non-preemptible tasks
        if !current.preemptible {
            return Ok(false);
        }

        // Check if quantum is exhausted
        if current.consumed_quantum >= current.allocated_quantum {
            return Ok(true);
        }

        // Check if a higher priority task is ready
        if let Some(task_info) = self.task_info.get(&current.task_id) {
            let current_priority = task_info.effective_priority;
            
            // Look for higher priority ready tasks
            for queue in self.priority_queues.iter() {
                if queue.priority > current_priority && !queue.tasks.is_empty() {
                    // Check if any task in this higher priority queue is ready
                    for &task_id in queue.tasks.iter() {
                        if let Some(info) = self.task_info.get(&task_id) {
                            if info.state == AsyncTaskState::Ready {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        // Check deadline violations
        if self.config.enable_deadline_scheduling {
            if let Some(task_info) = self.task_info.get(&current.task_id) {
                if let Some(deadline) = task_info.deadline {
                    let deadline_fuel = deadline.as_millis() as u64;
                    let elapsed = current_time.saturating_sub(task_info.last_run_time);
                    if elapsed > deadline_fuel {
                        self.stats.deadline_misses.fetch_add(1, Ordering::AcqRel);
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Preempt the currently running task
    pub fn preempt_current_task(&mut self, current_time: u64) -> Result<(), Error> {
        if let Some(current) = self.current_task.take() {
            record_global_operation(OperationType::ControlFlow, self.verification_level);
            self.consume_scheduler_fuel(PREEMPTION_FUEL)?;

            // Update task info
            if let Some(task_info) = self.task_info.get_mut(&current.task_id) {
                task_info.preemption_count += 1;
                task_info.total_run_time += current_time.saturating_sub(current.start_time);
                
                // Add back to appropriate priority queue if still ready
                if task_info.state == AsyncTaskState::Ready {
                    self.add_task_to_priority_queue(current.task_id, task_info.effective_priority)?;
                }
            }

            self.stats.total_preemptions.fetch_add(1, Ordering::AcqRel);
        }

        Ok(())
    }

    /// Start executing a task
    pub fn start_task_execution(&mut self, task_id: TaskId, current_time: u64) -> Result<(), Error> {
        record_global_operation(OperationType::ControlFlow, self.verification_level);
        self.consume_scheduler_fuel(CONTEXT_SWITCH_FUEL)?;

        if let Some(task_info) = self.task_info.get_mut(&task_id) {
            let quantum = task_info.fuel_quantum;
            let preemptible = task_info.preemptible;
            let priority = task_info.effective_priority;

            task_info.last_run_time = current_time;

            // Remove from priority queue
            self.remove_task_from_priority_queues(task_id)?;

            // Set as current task
            self.current_task = Some(RunningTaskContext {
                task_id,
                start_time: current_time,
                allocated_quantum: quantum,
                consumed_quantum: 0,
                run_priority: priority,
                preemptible,
            });

            self.stats.total_context_switches.fetch_add(1, Ordering::AcqRel);
        }

        Ok(())
    }

    /// Check for priority aging and boost old tasks
    pub fn check_priority_aging(&mut self, current_time: u64) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionIterate, self.verification_level);
        self.consume_scheduler_fuel(AGING_CHECK_FUEL)?;

        for (task_id, task_info) in self.task_info.iter_mut() {
            if task_info.state == AsyncTaskState::Ready {
                let wait_time = current_time.saturating_sub(task_info.last_run_time);
                
                if wait_time > self.config.aging_fuel_threshold 
                   && task_info.priority_boost < self.config.max_priority_boost {
                    
                    // Boost priority
                    let old_priority = task_info.effective_priority;
                    task_info.priority_boost += 1;
                    task_info.effective_priority = self.boost_priority(task_info.base_priority, task_info.priority_boost);
                    
                    if task_info.effective_priority != old_priority {
                        // Move task to new priority queue
                        self.remove_task_from_priority_queues(*task_id)?;
                        self.add_task_to_priority_queue(*task_id, task_info.effective_priority)?;
                        
                        self.stats.total_priority_boosts.fetch_add(1, Ordering::AcqRel);
                        self.consume_scheduler_fuel(PRIORITY_BOOST_FUEL)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get scheduler statistics
    pub fn get_statistics(&self) -> PreemptiveSchedulerStats {
        self.stats.clone()
    }

    /// Get information about a specific task
    pub fn get_task_info(&self, task_id: TaskId) -> Option<&PreemptiveTaskInfo> {
        self.task_info.get(&task_id)
    }

    /// Enable or disable preemption
    pub fn set_preemption_enabled(&self, enabled: bool) {
        self.preemption_enabled.store(enabled, Ordering::SeqCst);
    }

    // Private helper methods

    fn calculate_fuel_quantum(&self, priority: Priority, fuel_budget: u64) -> u64 {
        let base_quantum = match priority {
            Priority::Critical => self.config.max_fuel_quantum,
            Priority::High => self.config.default_fuel_quantum * 2,
            Priority::Normal => self.config.default_fuel_quantum,
            Priority::Low => self.config.default_fuel_quantum / 2,
        };

        // Ensure quantum is within bounds and doesn't exceed budget
        base_quantum
            .max(self.config.min_fuel_quantum)
            .min(self.config.max_fuel_quantum)
            .min(fuel_budget)
    }

    fn boost_priority(&self, base_priority: Priority, boost_level: u32) -> Priority {
        match (base_priority, boost_level) {
            (Priority::Low, 1..=2) => Priority::Normal,
            (Priority::Low, 3..) => Priority::High,
            (Priority::Normal, 1..=2) => Priority::High,
            (Priority::Normal, 3..) => Priority::Critical,
            (Priority::High, 1..) => Priority::Critical,
            _ => base_priority,
        }
    }

    fn add_task_to_priority_queue(&mut self, task_id: TaskId, priority: Priority) -> Result<(), Error> {
        for queue in self.priority_queues.iter_mut() {
            if queue.priority == priority {
                queue.tasks.push(task_id).map_err(|_| {
                    Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_LIMIT_EXCEEDED,
                        "Priority queue is full".to_string(),
                    )
                })?;
                return Ok(());
            }
        }
        Err(Error::new(
            ErrorCategory::Resource,
            codes::RESOURCE_NOT_FOUND,
            "Priority queue not found".to_string(),
        ))
    }

    fn remove_task_from_priority_queues(&mut self, task_id: TaskId) -> Result<(), Error> {
        for queue in self.priority_queues.iter_mut() {
            queue.tasks.retain(|&id| id != task_id);
        }
        Ok(())
    }

    fn update_task_priority_queue(&mut self, task_id: TaskId) -> Result<(), Error> {
        if let Some(task_info) = self.task_info.get(&task_id) {
            let priority = task_info.effective_priority;
            self.remove_task_from_priority_queues(task_id)?;
            self.add_task_to_priority_queue(task_id, priority)?;
        }
        Ok(())
    }

    fn select_highest_priority_task(&mut self) -> Result<Option<TaskId>, Error> {
        // Start from highest priority and work down
        for queue in self.priority_queues.iter_mut().rev() {
            if !queue.tasks.is_empty() {
                // Round-robin within the same priority level
                let position = queue.round_robin_position.load(Ordering::Acquire);
                let queue_len = queue.tasks.len();
                
                for i in 0..queue_len {
                    let index = (position + i) % queue_len;
                    if let Some(&task_id) = queue.tasks.get(index) {
                        if let Some(task_info) = self.task_info.get(&task_id) {
                            if task_info.state == AsyncTaskState::Ready {
                                // Update round-robin position
                                queue.round_robin_position.store((index + 1) % queue_len, Ordering::Release);
                                return Ok(Some(task_id));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn remove_task_from_scheduler(&mut self, task_id: TaskId) -> Result<(), Error> {
        self.task_info.remove(&task_id);
        self.remove_task_from_priority_queues(task_id)?;
        
        if let Some(ref current) = self.current_task {
            if current.task_id == task_id {
                self.current_task = None;
            }
        }
        
        self.stats.active_tasks.fetch_sub(1, Ordering::AcqRel);
        Ok(())
    }

    fn consume_scheduler_fuel(&self, amount: u64) -> Result<(), Error> {
        self.stats.scheduler_fuel_consumed.fetch_add(amount, Ordering::AcqRel);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preemptive_scheduler_creation() {
        let config = PreemptiveSchedulerConfig::default();
        let scheduler = FuelPreemptiveScheduler::new(config, VerificationLevel::Standard).unwrap();
        
        let stats = scheduler.get_statistics();
        assert_eq!(stats.active_tasks.load(Ordering::Acquire), 0);
        assert_eq!(stats.total_preemptions.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_task_addition() {
        let config = PreemptiveSchedulerConfig::default();
        let mut scheduler = FuelPreemptiveScheduler::new(config, VerificationLevel::Standard).unwrap();
        
        let result = scheduler.add_task(
            TaskId::new(1),
            ComponentInstanceId::new(1),
            Priority::Normal,
            5000,
            None,
            true,
        );
        
        assert!(result.is_ok());
        
        let stats = scheduler.get_statistics();
        assert_eq!(stats.active_tasks.load(Ordering::Acquire), 1);
        assert_eq!(stats.total_tasks_scheduled.load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_priority_scheduling() {
        let config = PreemptiveSchedulerConfig::default();
        let mut scheduler = FuelPreemptiveScheduler::new(config, VerificationLevel::Standard).unwrap();
        
        // Add low priority task
        scheduler.add_task(
            TaskId::new(1),
            ComponentInstanceId::new(1),
            Priority::Low,
            5000,
            None,
            true,
        ).unwrap();
        
        // Add high priority task
        scheduler.add_task(
            TaskId::new(2),
            ComponentInstanceId::new(1),
            Priority::High,
            5000,
            None,
            true,
        ).unwrap();
        
        // High priority task should be selected first
        let next_task = scheduler.schedule_next_task().unwrap();
        assert_eq!(next_task, Some(TaskId::new(2)));
    }

    #[test]
    fn test_preemption_logic() {
        let config = PreemptiveSchedulerConfig {
            default_fuel_quantum: 100,
            ..Default::default()
        };
        let mut scheduler = FuelPreemptiveScheduler::new(config, VerificationLevel::Standard).unwrap();
        
        // Start a task
        scheduler.add_task(
            TaskId::new(1),
            ComponentInstanceId::new(1),
            Priority::Normal,
            5000,
            None,
            true,
        ).unwrap();
        
        scheduler.schedule_next_task().unwrap();
        
        // Simulate fuel consumption beyond quantum
        scheduler.update_task_state(
            TaskId::new(1),
            AsyncTaskState::Ready,
            150, // Exceeds quantum of 100
        ).unwrap();
        
        // Task should be preempted
        let running_context = scheduler.current_task.as_ref();
        if let Some(context) = running_context {
            let should_preempt = scheduler.should_preempt_current_task(
                context,
                scheduler.current_fuel_time.load(Ordering::Acquire),
            ).unwrap();
            assert!(should_preempt);
        }
    }

    #[test]
    fn test_priority_aging() {
        let config = PreemptiveSchedulerConfig {
            enable_priority_aging: true,
            aging_fuel_threshold: 100,
            max_priority_boost: 2,
            ..Default::default()
        };
        let mut scheduler = FuelPreemptiveScheduler::new(config, VerificationLevel::Standard).unwrap();
        
        // Add low priority task
        scheduler.add_task(
            TaskId::new(1),
            ComponentInstanceId::new(1),
            Priority::Low,
            5000,
            None,
            true,
        ).unwrap();
        
        // Simulate aging
        scheduler.current_fuel_time.store(1000, Ordering::Release);
        scheduler.check_priority_aging(1000).unwrap();
        
        // Task priority should be boosted
        let task_info = scheduler.get_task_info(TaskId::new(1)).unwrap();
        assert!(task_info.priority_boost > 0);
        assert!(task_info.effective_priority > Priority::Low);
    }
}