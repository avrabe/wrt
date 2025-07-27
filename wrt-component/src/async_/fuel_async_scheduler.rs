//! Fuel-based async scheduler for deterministic task scheduling
//!
//! This module provides scheduling algorithms that use fuel consumption
//! for deterministic timing guarantees across different ASIL levels.

use crate::{
    async_::fuel_async_executor::{AsyncTaskState, AsyncTaskStatus, FuelAsyncExecutor},
    threading::task_manager::{TaskId, TaskState},
    ComponentInstanceId,
    prelude::*,
};
use core::{
    cmp::Ordering as CmpOrdering,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedMap, BoundedVec},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

/// Maximum number of scheduling events to track
const MAX_SCHEDULING_EVENTS: usize = 256;

/// Fuel costs for scheduling operations
const SCHEDULE_TASK_FUEL: u64 = 3;
const PRIORITIZE_TASK_FUEL: u64 = 5;
const DEADLINE_CHECK_FUEL: u64 = 2;

/// Scheduling policy for fuel-based async execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Cooperative scheduling - tasks yield voluntarily
    Cooperative,
    /// Priority-based scheduling with fuel inheritance
    PriorityBased,
    /// Deadline-based scheduling with WCET guarantees
    DeadlineBased,
    /// Round-robin with fuel quotas
    RoundRobin,
}

/// Task scheduling entry with fuel tracking
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub task_id: TaskId,
    pub component_id: ComponentInstanceId,
    pub priority: Priority,
    pub fuel_quota: u64,
    pub fuel_consumed: u64,
    pub deadline: Option<Duration>,
    pub last_scheduled: AtomicU64,
    pub schedule_count: AtomicUsize,
    pub state: AsyncTaskState,
}

/// Fuel-based async scheduler
pub struct FuelAsyncScheduler {
    /// Current scheduling policy
    policy: SchedulingPolicy,
    /// Scheduled tasks indexed by task ID
    scheduled_tasks: BoundedMap<TaskId, ScheduledTask, 128>,
    /// Priority queue for priority-based scheduling
    priority_queue: BoundedVec<TaskId, 128>,
    /// Round-robin queue
    round_robin_queue: BoundedVec<TaskId, 128>,
    /// Current round-robin position
    round_robin_position: AtomicUsize,
    /// Global scheduling time (in fuel units)
    global_schedule_time: AtomicU64,
    /// Verification level for scheduling operations
    verification_level: VerificationLevel,
    /// Fuel quantum for round-robin scheduling
    fuel_quantum: u64,
}

impl FuelAsyncScheduler {
    /// Create a new fuel-based async scheduler
    pub fn new(policy: SchedulingPolicy, verification_level: VerificationLevel) -> Result<Self, Error> {
        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        
        Ok(Self {
            policy,
            scheduled_tasks: BoundedMap::new(provider.clone())?,
            priority_queue: BoundedVec::new(provider.clone())?,
            round_robin_queue: BoundedVec::new(provider)?,
            round_robin_position: AtomicUsize::new(0),
            global_schedule_time: AtomicU64::new(0),
            verification_level,
            fuel_quantum: 1000, // Default quantum
        })
    }

    /// Set the scheduling policy
    pub fn set_policy(&mut self, policy: SchedulingPolicy) {
        self.policy = policy;
        // Clear queues when policy changes
        self.priority_queue.clear);
        self.round_robin_queue.clear);
        self.round_robin_position.store(0, Ordering::SeqCst;
    }

    /// Set the fuel quantum for round-robin scheduling
    pub fn set_fuel_quantum(&mut self, quantum: u64) {
        self.fuel_quantum = quantum;
    }

    /// Add a task to the scheduler
    pub fn add_task(
        &mut self,
        task_id: TaskId,
        component_id: ComponentInstanceId,
        priority: Priority,
        fuel_quota: u64,
        deadline: Option<Duration>,
    ) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionInsert, self.verification_level;

        let scheduled_task = ScheduledTask {
            task_id,
            component_id,
            priority,
            fuel_quota,
            fuel_consumed: 0,
            deadline,
            last_scheduled: AtomicU64::new(0),
            schedule_count: AtomicUsize::new(0),
            state: AsyncTaskState::Ready,
        };

        self.scheduled_tasks.insert(task_id, scheduled_task).map_err(|_| {
            Error::resource_limit_exceeded("Too many scheduled tasks")
        })?;

        // Add to appropriate scheduling queue
        match self.policy {
            SchedulingPolicy::Cooperative => {
                // Tasks are polled in order of readiness
            }
            SchedulingPolicy::PriorityBased => {
                self.insert_priority_queue(task_id)?;
            }
            SchedulingPolicy::DeadlineBased => {
                self.insert_deadline_queue(task_id)?;
            }
            SchedulingPolicy::RoundRobin => {
                self.round_robin_queue.push(task_id).map_err(|_| {
                    Error::resource_limit_exceeded("Round-robin queue is full")
                })?;
            }
        }

        Ok(())
    }

    /// Remove a task from the scheduler
    pub fn remove_task(&mut self, task_id: TaskId) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionRemove, self.verification_level;

        self.scheduled_tasks.remove(&task_id;
        
        // Remove from all queues
        self.priority_queue.retain(|&id| id != task_id;
        self.round_robin_queue.retain(|&id| id != task_id;

        Ok(())
    }

    /// Get the next task to schedule based on the current policy
    pub fn next_task(&mut self) -> Option<TaskId> {
        record_global_operation(OperationType::FunctionCall, self.verification_level;

        let current_time = self.global_schedule_time.fetch_add(SCHEDULE_TASK_FUEL, Ordering::AcqRel;

        match self.policy {
            SchedulingPolicy::Cooperative => self.next_cooperative_task(),
            SchedulingPolicy::PriorityBased => self.next_priority_task(),
            SchedulingPolicy::DeadlineBased => self.next_deadline_task(current_time),
            SchedulingPolicy::RoundRobin => self.next_round_robin_task(),
        }
    }

    /// Update task state after execution
    pub fn update_task_state(
        &mut self,
        task_id: TaskId,
        fuel_consumed: u64,
        new_state: AsyncTaskState,
    ) -> Result<(), Error> {
        if let Some(task) = self.scheduled_tasks.get_mut(&task_id) {
            task.fuel_consumed += fuel_consumed;
            task.state = new_state;
            task.last_scheduled.store(
                self.global_schedule_time.load(Ordering::Acquire),
                Ordering::Release,
            ;
            task.schedule_count.fetch_add(1, Ordering::AcqRel;

            // Re-prioritize if necessary
            if matches!(new_state, AsyncTaskState::Ready) {
                match self.policy {
                    SchedulingPolicy::PriorityBased => {
                        self.reprioritize_task(task_id)?;
                    }
                    SchedulingPolicy::DeadlineBased => {
                        self.reorder_deadline_queue(task_id)?;
                    }
                    _ => {}
                }
            }

            record_global_operation(OperationType::CollectionMutate, self.verification_level;
        }

        Ok(())
    }

    /// Check for deadline violations
    pub fn check_deadlines(&self, current_time: u64) -> Vec<TaskId> {
        let mut violations = Vec::new());

        for (task_id, task) in self.scheduled_tasks.iter() {
            if let Some(deadline) = task.deadline {
                let deadline_fuel = deadline.as_millis() as u64; // 1ms = 1 fuel
                let elapsed = current_time.saturating_sub(task.last_scheduled.load(Ordering::Acquire;

                if elapsed > deadline_fuel {
                    violations.push(*task_id);
                }
            }
        }

        record_global_operation(OperationType::CollectionIterate, self.verification_level;
        violations
    }

    /// Get scheduling statistics
    pub fn get_statistics(&self) -> SchedulingStatistics {
        let mut total_fuel_consumed = 0;
        let mut total_schedule_count = 0;
        let mut ready_tasks = 0;
        let mut waiting_tasks = 0;

        for task in self.scheduled_tasks.values() {
            total_fuel_consumed += task.fuel_consumed;
            total_schedule_count += task.schedule_count.load(Ordering::Acquire;

            match task.state {
                AsyncTaskState::Ready => ready_tasks += 1,
                AsyncTaskState::Waiting => waiting_tasks += 1,
                _ => {}
            }
        }

        SchedulingStatistics {
            policy: self.policy,
            total_tasks: self.scheduled_tasks.len(),
            ready_tasks,
            waiting_tasks,
            total_fuel_consumed,
            total_schedule_count,
            global_schedule_time: self.global_schedule_time.load(Ordering::Acquire),
            fuel_quantum: self.fuel_quantum,
        }
    }

    // Private helper methods

    fn next_cooperative_task(&self) -> Option<TaskId> {
        // Simple cooperative scheduling - first ready task
        for (task_id, task) in self.scheduled_tasks.iter() {
            if task.state == AsyncTaskState::Ready {
                return Some(*task_id;
            }
        }
        None
    }

    fn next_priority_task(&mut self) -> Option<TaskId> {
        record_global_operation(OperationType::CollectionLookup, self.verification_level;

        while let Some(task_id) = self.priority_queue.pop() {
            if let Some(task) = self.scheduled_tasks.get(&task_id) {
                if task.state == AsyncTaskState::Ready {
                    return Some(task_id;
                }
            }
        }
        None
    }

    fn next_deadline_task(&self, current_time: u64) -> Option<TaskId> {
        let mut best_task: Option<TaskId> = None;
        let mut earliest_deadline = u64::MAX;

        for (task_id, task) in self.scheduled_tasks.iter() {
            if task.state == AsyncTaskState::Ready {
                if let Some(deadline) = task.deadline {
                    let deadline_fuel = deadline.as_millis() as u64;
                    let task_deadline = task.last_scheduled.load(Ordering::Acquire) + deadline_fuel;

                    if task_deadline < earliest_deadline {
                        earliest_deadline = task_deadline;
                        best_task = Some(*task_id;
                    }
                } else if best_task.is_none() {
                    // Tasks without deadlines have lower priority
                    best_task = Some(*task_id;
                }
            }
        }

        record_global_operation(OperationType::CollectionIterate, self.verification_level;
        best_task
    }

    fn next_round_robin_task(&mut self) -> Option<TaskId> {
        if self.round_robin_queue.is_empty() {
            return None;
        }

        let start_pos = self.round_robin_position.load(Ordering::Acquire;
        let queue_len = self.round_robin_queue.len);

        for i in 0..queue_len {
            let pos = (start_pos + i) % queue_len;
            if let Some(&task_id) = self.round_robin_queue.get(pos) {
                if let Some(task) = self.scheduled_tasks.get(&task_id) {
                    if task.state == AsyncTaskState::Ready {
                        // Update position for next round
                        self.round_robin_position.store((pos + 1) % queue_len, Ordering::Release;
                        return Some(task_id;
                    }
                }
            }
        }

        None
    }

    fn insert_priority_queue(&mut self, task_id: TaskId) -> Result<(), Error> {
        let task_priority = self.scheduled_tasks.get(&task_id)
            .map(|t| t.priority)
            .unwrap_or(Priority::Normal;

        // Insert in priority order (higher priority first)
        let mut insert_pos = self.priority_queue.len);
        for (i, &existing_id) in self.priority_queue.iter().enumerate() {
            if let Some(existing_task) = self.scheduled_tasks.get(&existing_id) {
                if task_priority > existing_task.priority {
                    insert_pos = i;
                    break;
                }
            }
        }

        self.priority_queue.insert(insert_pos, task_id).map_err(|_| {
            Error::resource_limit_exceeded("Priority queue is full")
        })
    }

    fn insert_deadline_queue(&mut self, task_id: TaskId) -> Result<(), Error> {
        // For deadline scheduling, we use the priority queue but order by deadline
        self.priority_queue.push(task_id).map_err(|_| {
            Error::resource_limit_exceeded("Deadline queue is full")
        })?;

        // Sort by deadline (earliest first)
        self.sort_deadline_queue);
        Ok(())
    }

    fn sort_deadline_queue(&mut self) {
        // Simple bubble sort for small queues
        let len = self.priority_queue.len);
        for i in 0..len {
            for j in 0..len.saturating_sub(1 + i) {
                if self.should_swap_deadline_tasks(j, j + 1) {
                    // Swap tasks
                    let temp = self.priority_queue[j];
                    self.priority_queue[j] = self.priority_queue[j + 1];
                    self.priority_queue[j + 1] = temp;
                }
            }
        }
    }

    fn should_swap_deadline_tasks(&self, i: usize, j: usize) -> bool {
        if let (Some(&task_a), Some(&task_b)) = (self.priority_queue.get(i), self.priority_queue.get(j)) {
            if let (Some(task_a_info), Some(task_b_info)) = 
                (self.scheduled_tasks.get(&task_a), self.scheduled_tasks.get(&task_b)) 
            {
                match (task_a_info.deadline, task_b_info.deadline) {
                    (Some(deadline_a), Some(deadline_b)) => deadline_a > deadline_b,
                    (None, Some(_)) => true, // Tasks without deadlines go to the end
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn reprioritize_task(&mut self, task_id: TaskId) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionMutate, self.verification_level;

        // Remove task from current position
        self.priority_queue.retain(|&id| id != task_id;

        // Re-insert with current priority
        self.insert_priority_queue(task_id)
    }

    fn reorder_deadline_queue(&mut self, _task_id: TaskId) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionMutate, self.verification_level;

        // Re-sort the entire deadline queue
        self.sort_deadline_queue);
        Ok(())
    }
}

/// Scheduling statistics
#[derive(Debug, Clone)]
pub struct SchedulingStatistics {
    pub policy: SchedulingPolicy,
    pub total_tasks: usize,
    pub ready_tasks: usize,
    pub waiting_tasks: usize,
    pub total_fuel_consumed: u64,
    pub total_schedule_count: usize,
    pub global_schedule_time: u64,
    pub fuel_quantum: u64,
}

impl SchedulingStatistics {
    pub fn average_fuel_per_task(&self) -> f64 {
        if self.total_tasks > 0 {
            self.total_fuel_consumed as f64 / self.total_tasks as f64
        } else {
            0.0
        }
    }

    pub fn scheduling_efficiency(&self) -> f64 {
        if self.global_schedule_time > 0 {
            (self.total_fuel_consumed as f64 / self.global_schedule_time as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = FuelAsyncScheduler::new(
            SchedulingPolicy::Cooperative,
            VerificationLevel::Standard,
        ).unwrap();

        let stats = scheduler.get_statistics);
        assert_eq!(stats.policy, SchedulingPolicy::Cooperative;
        assert_eq!(stats.total_tasks, 0);
    }

    #[test]
    fn test_task_addition() {
        let mut scheduler = FuelAsyncScheduler::new(
            SchedulingPolicy::Cooperative,
            VerificationLevel::Standard,
        ).unwrap();

        let task_id = TaskId::new(1;
        scheduler.add_task(
            task_id,
            ComponentInstanceId::new(1),
            Priority::Normal,
            1000,
            None,
        ).unwrap();

        let stats = scheduler.get_statistics);
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.ready_tasks, 1);
    }

    #[test]
    fn test_priority_scheduling() {
        let mut scheduler = FuelAsyncScheduler::new(
            SchedulingPolicy::PriorityBased,
            VerificationLevel::Standard,
        ).unwrap();

        let task1 = TaskId::new(1;
        let task2 = TaskId::new(2;

        // Add low priority task first
        scheduler.add_task(task1, ComponentInstanceId::new(1), Priority::Low, 1000, None).unwrap();
        // Add high priority task second
        scheduler.add_task(task2, ComponentInstanceId::new(1), Priority::High, 1000, None).unwrap();

        // High priority task should be scheduled first
        let next = scheduler.next_task);
        assert_eq!(next, Some(task2;
    }

    #[test]
    fn test_round_robin_scheduling() {
        let mut scheduler = FuelAsyncScheduler::new(
            SchedulingPolicy::RoundRobin,
            VerificationLevel::Standard,
        ).unwrap();

        let task1 = TaskId::new(1;
        let task2 = TaskId::new(2;

        scheduler.add_task(task1, ComponentInstanceId::new(1), Priority::Normal, 1000, None).unwrap();
        scheduler.add_task(task2, ComponentInstanceId::new(1), Priority::Normal, 1000, None).unwrap();

        // Should alternate between tasks
        assert_eq!(scheduler.next_task(), Some(task1;
        scheduler.update_task_state(task1, 100, AsyncTaskState::Waiting).unwrap();
        assert_eq!(scheduler.next_task(), Some(task2;
    }
}