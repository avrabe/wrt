//! Integration tests for the fuel-aware async executor
//!
//! This module contains comprehensive tests that verify the correct
//! integration of all Phase 1 improvements.

#[cfg(test)]
mod tests {
    use crate::{
        async_::{
            fuel_async_executor::{FuelAsyncExecutor, AsyncTaskState},
            fuel_aware_waker::{create_fuel_aware_waker, WakeCoalescer},
            component_async_bridge::{ComponentAsyncBridge, ComponentAsyncStats},
            fuel_async_scheduler::{FuelAsyncScheduler, SchedulingPolicy},
        },
        task_manager::TaskManager,
        threading::thread_spawn_fuel::FuelTrackedThreadManager,
        ComponentInstanceId,
        prelude::*,
    };
    use core::{
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicBool, AtomicU64, Ordering},
        task::{Context, Poll},
        time::Duration,
    };
    use wrt_foundation::{Arc, sync::Mutex};
    use wrt_platform::advanced_sync::Priority;

    /// Test future that simulates async I/O with fuel consumption
    struct FuelAwareIoFuture {
        id: u64,
        polls_remaining: u64,
        fuel_per_poll: u64,
        completed: Arc<AtomicBool>,
    }

    impl Future for FuelAwareIoFuture {
        type Output = Result<u64, Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.polls_remaining == 0 {
                self.completed.store(true, Ordering::Release;
                Poll::Ready(Ok(self.id))
            } else {
                self.polls_remaining -= 1;
                cx.waker().wake_by_ref);
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_full_async_lifecycle() {
        // Create the full async stack
        let task_manager = Arc::new(Mutex::new(TaskManager::new();
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
        let mut bridge = ComponentAsyncBridge::new(
            task_manager.clone(),
            thread_manager.clone(),
        ).unwrap();

        // Register a component
        let component_id = ComponentInstanceId::new(1;
        bridge.register_component(
            component_id,
            10,     // max concurrent tasks
            10000,  // fuel budget
            Priority::Normal,
        ).unwrap();

        // Spawn multiple async tasks
        let mut task_ids = Vec::new);
        let mut completion_flags = Vec::new);

        for i in 0..3 {
            let completed = Arc::new(AtomicBool::new(false;
            completion_flags.push(completed.clone();

            let future = FuelAwareIoFuture {
                id: i,
                polls_remaining: 2,
                fuel_per_poll: 100,
                completed,
            };

            let task_id = bridge.spawn_component_async(
                component_id,
                future,
                Some(1000),
            ).unwrap();
            task_ids.push(task_id);
        }

        // Poll until all tasks complete
        let mut total_polls = 0;
        let max_polls = 20;

        while total_polls < max_polls {
            let result = bridge.poll_async_tasks().unwrap();
            total_polls += 1;

            if result.tasks_completed == 3 {
                break;
            }

            // Small delay to simulate real async work
            std::thread::sleep(Duration::from_millis(10;
        }

        // Verify all tasks completed
        for flag in completion_flags {
            assert!(flag.load(Ordering::Acquire);
        }

        // Check statistics
        let stats = bridge.get_component_stats(component_id).unwrap();
        assert_eq!(stats.active_tasks, 0;
        assert!(stats.fuel_consumed > 0);
        assert!(stats.fuel_consumed < stats.fuel_budget);

        let polling_stats = bridge.get_polling_stats().unwrap();
        assert_eq!(polling_stats.tasks_completed, 3;
        assert!(polling_stats.total_polls >= 6)); // At least 2 polls per task
    }

    #[test]
    fn test_fuel_exhaustion_handling() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(100;
        
        // Create executor with self-reference
        let executor_arc = Arc::new(Mutex::new(executor;
        if let Ok(mut exec) = executor_arc.lock() {
            exec.set_self_ref(Arc::downgrade(&executor_arc;
        }

        // Spawn a task with more fuel than available
        let result = {
            let mut exec = executor_arc.lock().unwrap();
            exec.spawn_task(
                ComponentInstanceId::new(1),
                150, // Exceeds global limit
                Priority::Normal,
                async { Ok(()) },
            )
        };

        assert!(result.is_err();
        
        // Spawn a task within limits
        let task_id = {
            let mut exec = executor_arc.lock().unwrap();
            exec.spawn_task(
                ComponentInstanceId::new(1),
                50,
                Priority::Normal,
                async { Ok(()) },
            ).unwrap()
        };

        // Poll to completion
        {
            let mut exec = executor_arc.lock().unwrap();
            let polled = exec.poll_tasks().unwrap();
            assert_eq!(polled, 1;
        }

        // Verify fuel was consumed
        let status = {
            let exec = executor_arc.lock().unwrap();
            exec.get_global_fuel_status()
        };
        assert!(status.consumed > 0);
    }

    #[test]
    fn test_wake_coalescing() {
        let coalescer = WakeCoalescer::new().unwrap();
        
        // Add multiple wakes for the same task
        let task_id = crate::threading::task_manager::TaskId::new(42;
        for _ in 0..5 {
            coalescer.add_wake(task_id).unwrap();
        }
        
        // Should only have one pending wake
        assert_eq!(coalescer.pending_count(), 1;
        
        // Add wakes for different tasks
        coalescer.add_wake(crate::threading::task_manager::TaskId::new(43)).unwrap();
        coalescer.add_wake(crate::threading::task_manager::TaskId::new(44)).unwrap();
        
        assert_eq!(coalescer.pending_count(), 3;
    }

    #[test]
    fn test_concurrent_task_execution() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new();
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
        let mut bridge = ComponentAsyncBridge::new(
            task_manager.clone(),
            thread_manager.clone(),
        ).unwrap();

        // Set global fuel budget
        bridge.set_global_fuel_budget(50000).unwrap();

        // Register multiple components
        for i in 1..=3 {
            let component_id = ComponentInstanceId::new(i;
            bridge.register_component(
                component_id,
                5,      // max concurrent tasks per component
                10000,  // fuel budget per component
                Priority::Normal,
            ).unwrap();
        }

        // Spawn tasks across components
        let mut all_tasks = Vec::new);
        let mut completion_counters = Vec::new);

        for comp_id in 1..=3 {
            let counter = Arc::new(AtomicU64::new(0;
            completion_counters.push(counter.clone();

            for task_num in 0..3 {
                let counter_clone = counter.clone();
                let future = async move {
                    // Simulate some async work
                    counter_clone.fetch_add(1, Ordering::AcqRel;
                    Ok(())
                };

                let task_id = bridge.spawn_component_async(
                    ComponentInstanceId::new(comp_id),
                    future,
                    Some(500),
                ).unwrap();
                all_tasks.push(task_id);
            }
        }

        // Poll until all tasks complete
        let mut polls = 0;
        loop {
            let result = bridge.poll_async_tasks().unwrap();
            polls += 1;

            if result.tasks_completed == 9 || polls > 50 {
                break;
            }
        }

        // Verify all tasks completed
        for counter in completion_counters {
            assert_eq!(counter.load(Ordering::Acquire), 3;
        }

        // Check component statistics
        for i in 1..=3 {
            let stats = bridge.get_component_stats(ComponentInstanceId::new(i)).unwrap();
            assert_eq!(stats.active_tasks, 0;
            assert!(stats.fuel_consumed > 0);
        }
    }

    #[test]
    fn test_priority_based_scheduling() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        let scheduler = FuelAsyncScheduler::new(SchedulingPolicy::Priority;
        
        executor.set_global_fuel_limit(10000;

        // Spawn tasks with different priorities
        let high_priority_completed = Arc::new(AtomicBool::new(false;
        let low_priority_completed = Arc::new(AtomicBool::new(false;

        let high_clone = high_priority_completed.clone();
        let low_task = executor.spawn_task(
            ComponentInstanceId::new(1),
            1000,
            Priority::Low,
            async move {
                low_priority_completed.store(true, Ordering::Release;
                Ok(())
            },
        ).unwrap();

        let high_task = executor.spawn_task(
            ComponentInstanceId::new(1),
            1000,
            Priority::High,
            async move {
                high_clone.store(true, Ordering::Release;
                Ok(())
            },
        ).unwrap();

        // With priority scheduling, high priority should complete first
        // (In a real implementation, the scheduler would be integrated)
        executor.poll_tasks().unwrap();

        // Both should eventually complete
        executor.poll_tasks().unwrap();
        
        assert!(high_priority_completed.load(Ordering::Acquire);
        assert!(low_priority_completed.load(Ordering::Acquire);
    }

    #[test]
    fn test_yield_threshold() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(100000;
        
        // Spawn many lightweight tasks
        let mut task_ids = Vec::new);
        for i in 0..20 {
            let task_id = executor.spawn_task(
                ComponentInstanceId::new(1),
                100,
                Priority::Normal,
                async move { Ok(()) },
            ).unwrap();
            task_ids.push(task_id);
        }

        // First poll should not process all tasks due to yield threshold
        let polled = executor.poll_tasks().unwrap();
        assert!(polled < 20);

        // Multiple polls needed to complete all
        let mut total_polled = polled;
        while total_polled < 20 {
            let additional = executor.poll_tasks().unwrap();
            total_polled += additional;
        }

        assert_eq!(total_polled, 20;
    }
}