//! Bridge between Component Model async and fuel-based executor
//!
//! This module provides integration between the Component Model's async
//! requirements and our fuel-based executor implementation.

use crate::{
    async_::{
        fuel_async_executor::{FuelAsyncExecutor, AsyncTaskState},
        fuel_async_scheduler::SchedulingPolicy,
    },
    task_manager::{TaskManager, TaskId as ComponentTaskId, TaskType, TaskState},
    threading::thread_spawn_fuel::{FuelTrackedThreadManager, ThreadFuelStatus},
    ComponentInstanceId,
    prelude::*,
};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};
use wrt_foundation::{
    bounded_collections::BoundedMap,
    verification::VerificationLevel,
    Arc, Weak, sync::Mutex,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

/// Maximum concurrent async operations per component
const MAX_ASYNC_OPS_PER_COMPONENT: usize = 64;

/// Bridge between Component Model async and fuel executor
pub struct ComponentAsyncBridge {
    /// Fuel-based async executor
    executor: Arc<Mutex<FuelAsyncExecutor>>,
    /// Component task manager for async integration
    task_manager: Arc<Mutex<TaskManager>>,
    /// Thread manager for fuel tracking
    thread_manager: Arc<Mutex<FuelTrackedThreadManager>>,
    /// Mapping from component tasks to executor tasks
    task_mapping: BoundedMap<ComponentTaskId, crate::threading::task_manager::TaskId, 1024>,
    /// Per-component async operation limits
    component_limits: BoundedMap<ComponentInstanceId, AsyncComponentLimits, 256>,
    /// Global async fuel budget
    global_async_fuel_budget: AtomicU64,
    /// Verification level for async operations
    verification_level: VerificationLevel,
}

/// Per-component async operation limits
#[derive(Debug, Clone)]
struct AsyncComponentLimits {
    max_concurrent_tasks: usize,
    active_tasks: AtomicU64,
    fuel_budget: u64,
    fuel_consumed: AtomicU64,
    priority: Priority,
}

impl ComponentAsyncBridge {
    /// Create a new component async bridge
    pub fn new(
        task_manager: Arc<Mutex<TaskManager>>,
        thread_manager: Arc<Mutex<FuelTrackedThreadManager>>,
    ) -> Result<Self, Error> {
        let executor = Arc::new(Mutex::new(FuelAsyncExecutor::new()?;
        
        // Set up executor self-reference for proper waker creation
        let weak_executor = Arc::downgrade(&executor;
        if let Ok(mut exec) = executor.lock() {
            exec.set_self_ref(weak_executor;
        }
        
        Ok(Self {
            executor,
            task_manager,
            thread_manager,
            task_mapping: BoundedMap::new(provider.clone())?,
            component_limits: BoundedMap::new(provider.clone())?,
            global_async_fuel_budget: AtomicU64::new(u64::MAX),
            verification_level: VerificationLevel::Standard,
        })
    }

    /// Register a component with async limits
    pub fn register_component(
        &mut self,
        component_id: ComponentInstanceId,
        max_concurrent_tasks: usize,
        fuel_budget: u64,
        priority: Priority,
    ) -> Result<(), Error> {
        let limits = AsyncComponentLimits {
            max_concurrent_tasks,
            active_tasks: AtomicU64::new(0),
            fuel_budget,
            fuel_consumed: AtomicU64::new(0),
            priority,
        };

        self.component_limits.insert(component_id, limits).map_err(|_| {
            Error::resource_limit_exceeded("Too many registered components")
        })?;

        Ok(())
    }

    /// Spawn an async task for a component
    pub fn spawn_component_async<F>(
        &mut self,
        component_id: ComponentInstanceId,
        future: F,
        fuel_budget: Option<u64>,
    ) -> Result<ComponentTaskId, Error>
    where
        F: Future<Output = Result<(), Error>> + Send + 'static,
    {
        // Check component limits
        let limits = self.component_limits.get(&component_id).ok_or_else(|| {
            Error::validation_invalid_input("Component not registered for async")
        })?;

        let active = limits.active_tasks.load(Ordering::Acquire;
        if active >= limits.max_concurrent_tasks as u64 {
            return Err(Error::resource_limit_exceeded("Component async task limit exceeded";
        }

        // Determine fuel budget
        let task_fuel = fuel_budget.unwrap_or(limits.fuel_budget / limits.max_concurrent_tasks as u64;
        
        // Check fuel availability
        let consumed = limits.fuel_consumed.load(Ordering::Acquire;
        if consumed + task_fuel > limits.fuel_budget {
            return Err(Error::resource_limit_exceeded("Component fuel budget exceeded";
        }

        // Create component task
        let component_task_id = {
            let mut tm = self.task_manager.lock()?;
            tm.spawn_task(TaskType::AsyncOperation, component_id.0, None)?
        };

        // Spawn in fuel executor
        let executor_task_id = {
            let mut exec = self.executor.lock()?;
            exec.spawn_task(
                component_id,
                task_fuel,
                limits.priority,
                future,
            )?
        };

        // Update tracking
        self.task_mapping.insert(component_task_id, executor_task_id).map_err(|_| {
            Error::resource_limit_exceeded("Task mapping table full")
        })?;

        limits.active_tasks.fetch_add(1, Ordering::AcqRel;
        limits.fuel_consumed.fetch_add(task_fuel, Ordering::AcqRel;

        Ok(component_task_id)
    }

    /// Poll async tasks and advance execution
    pub fn poll_async_tasks(&mut self) -> Result<PollResult, Error> {
        let mut result = PollResult::default);

        // Poll the fuel executor
        let tasks_polled = {
            let mut exec = self.executor.lock()?;
            exec.poll_tasks()?
        };
        result.tasks_polled = tasks_polled;

        // Update component task states based on executor state
        let mut completed_tasks = Vec::new);
        for (comp_task_id, exec_task_id) in self.task_mapping.iter() {
            let exec = self.executor.lock()?;
            if let Some(status) = exec.get_task_status(*exec_task_id) {
                match status.state {
                    AsyncTaskState::Completed => {
                        completed_tasks.push(*comp_task_id);
                        result.tasks_completed += 1;
                    }
                    AsyncTaskState::Failed => {
                        completed_tasks.push(*comp_task_id);
                        result.tasks_failed += 1;
                    }
                    AsyncTaskState::Waiting => {
                        result.tasks_waiting += 1;
                    }
                    _ => {}
                }
            }
        }

        // Clean up completed tasks
        for task_id in completed_tasks {
            self.cleanup_component_task(task_id)?;
        }

        // Collect fuel statistics
        let exec = self.executor.lock()?;
        let fuel_status = exec.get_global_fuel_status);
        result.total_fuel_consumed = fuel_status.consumed;
        result.fuel_remaining = fuel_status.remaining);

        Ok(result)
    }

    /// Check if a component task is ready
    pub fn is_task_ready(&self, task_id: ComponentTaskId) -> Result<bool, Error> {
        if let Some(exec_task_id) = self.task_mapping.get(&task_id) {
            let exec = self.executor.lock()?;
            if let Some(status) = exec.get_task_status(*exec_task_id) {
                Ok(status.state == AsyncTaskState::Ready)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Get component async statistics
    pub fn get_component_stats(&self, component_id: ComponentInstanceId) -> Result<ComponentAsyncStats, Error> {
        let limits = self.component_limits.get(&component_id).ok_or_else(|| {
            Error::validation_invalid_input("Component not registered")
        })?;

        Ok(ComponentAsyncStats {
            component_id,
            active_tasks: limits.active_tasks.load(Ordering::Acquire),
            max_tasks: limits.max_concurrent_tasks as u64,
            fuel_consumed: limits.fuel_consumed.load(Ordering::Acquire),
            fuel_budget: limits.fuel_budget,
        })
    }

    /// Clean up a completed component task
    fn cleanup_component_task(&mut self, task_id: ComponentTaskId) -> Result<(), Error> {
        // Remove from mapping
        if let Some(exec_task_id) = self.task_mapping.remove(&task_id) {
            // Get component ID from task manager
            let component_id = {
                let tm = self.task_manager.lock()?;
                if let Some(task) = tm.get_task(task_id) {
                    ComponentInstanceId::new(task.context.component_instance)
                } else {
                    return Ok();
                }
            };

            // Update component limits
            if let Some(limits) = self.component_limits.get(&component_id) {
                limits.active_tasks.fetch_sub(1, Ordering::AcqRel;
                
                // Return unused fuel
                let exec = self.executor.lock()?;
                if let Some(status) = exec.get_task_status(exec_task_id) {
                    let unused_fuel = status.fuel_budget - status.fuel_consumed;
                    if unused_fuel > 0 {
                        limits.fuel_consumed.fetch_sub(unused_fuel, Ordering::AcqRel;
                    }
                }
            }

            // Update task manager state
            let mut tm = self.task_manager.lock()?;
            if let Some(task) = tm.get_task_mut(task_id) {
                task.state = TaskState::Completed;
            }
        }

        Ok(())
    }

    /// Set global async fuel budget
    pub fn set_global_fuel_budget(&mut self, budget: u64) -> Result<(), Error> {
        self.global_async_fuel_budget.store(budget, Ordering::SeqCst;
        let mut exec = self.executor.lock()?;
        exec.set_global_fuel_limit(budget;
        Ok(())
    }

    /// Get executor polling statistics
    pub fn get_polling_stats(&self) -> Result<crate::async_::fuel_async_executor::PollingStatistics, Error> {
        let exec = self.executor.lock()?;
        Ok(exec.get_polling_stats())
    }
}

/// Result of polling async tasks
#[derive(Debug, Default)]
pub struct PollResult {
    pub tasks_polled: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_waiting: usize,
    pub total_fuel_consumed: u64,
    pub fuel_remaining: u64,
}

/// Component async statistics
#[derive(Debug, Clone)]
pub struct ComponentAsyncStats {
    pub component_id: ComponentInstanceId,
    pub active_tasks: u64,
    pub max_tasks: u64,
    pub fuel_consumed: u64,
    pub fuel_budget: u64,
}

impl ComponentAsyncStats {
    pub fn utilization_percentage(&self) -> f64 {
        if self.max_tasks == 0 {
            0.0
        } else {
            (self.active_tasks as f64 / self.max_tasks as f64) * 100.0
        }
    }

    pub fn fuel_usage_percentage(&self) -> f64 {
        if self.fuel_budget == 0 {
            0.0
        } else {
            (self.fuel_consumed as f64 / self.fuel_budget as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new();
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
        
        let bridge = ComponentAsyncBridge::new(task_manager, thread_manager).unwrap();
        assert_eq!(bridge.component_limits.len(), 0;
    }

    #[test]
    fn test_component_registration() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new();
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
        
        let mut bridge = ComponentAsyncBridge::new(task_manager, thread_manager).unwrap();
        
        let component_id = ComponentInstanceId::new(1;
        bridge.register_component(component_id, 10, 10000, Priority::Normal).unwrap();
        
        let stats = bridge.get_component_stats(component_id).unwrap();
        assert_eq!(stats.active_tasks, 0;
        assert_eq!(stats.max_tasks, 10;
        assert_eq!(stats.fuel_budget, 10000;
    }

    #[test]
    fn test_async_stats() {
        let stats = ComponentAsyncStats {
            component_id: ComponentInstanceId::new(1),
            active_tasks: 5,
            max_tasks: 10,
            fuel_consumed: 2500,
            fuel_budget: 10000,
        };

        assert_eq!(stats.utilization_percentage(), 50.0;
        assert_eq!(stats.fuel_usage_percentage(), 25.0;
    }
}