//! Phase 2 integration tests for advanced fuel management and preemption
//!
//! This module tests the dynamic fuel allocation and preemption features.

#[cfg(test)]
mod tests {
    use crate::{
        async_::{
            fuel_async_executor::{FuelAsyncExecutor, AsyncTaskState},
            fuel_dynamic_manager::{FuelDynamicManager, FuelAllocationPolicy},
            fuel_preemption_support::{FuelPreemptionManager, PreemptionPolicy, PreemptionReason},
            component_async_bridge::ComponentAsyncBridge,
        },
        task_manager::TaskManager,
        threading::thread_spawn_fuel::FuelTrackedThreadManager,
        ComponentInstanceId,
        prelude::*,
    };
    use core::{
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicU64, AtomicBool, Ordering},
        task::{Context, Poll},
        time::Duration,
    };
    use wrt_foundation::{Arc, sync::Mutex};
    use wrt_platform::advanced_sync::Priority;

    /// Future that consumes variable fuel
    struct VariableFuelFuture {
        id: u64,
        polls_remaining: u64,
        fuel_per_poll: AtomicU64,
        total_consumed: AtomicU64,
    }

    impl Future for VariableFuelFuture {
        type Output = Result<u64, Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.polls_remaining == 0 {
                Poll::Ready(Ok(self.total_consumed.load(Ordering::Acquire)))
            } else {
                self.polls_remaining -= 1;
                let fuel = self.fuel_per_poll.load(Ordering::Acquire;
                self.total_consumed.fetch_add(fuel, Ordering::AcqRel;
                cx.waker().wake_by_ref);
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_dynamic_fuel_allocation() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(100_000;
        executor.enable_dynamic_fuel_management(FuelAllocationPolicy::Adaptive).unwrap();
        
        // Create executor with self-reference
        let executor_arc = Arc::new(Mutex::new(executor;
        if let Ok(mut exec) = executor_arc.lock() {
            exec.set_self_ref(Arc::downgrade(&executor_arc;
        }

        // Spawn task that will exhaust fuel
        let task_id = {
            let mut exec = executor_arc.lock().unwrap();
            exec.spawn_task(
                ComponentInstanceId::new(1),
                500, // Initial small budget
                Priority::Normal,
                VariableFuelFuture {
                    id: 1,
                    polls_remaining: 10,
                    fuel_per_poll: AtomicU64::new(100),
                    total_consumed: AtomicU64::new(0),
                },
            ).unwrap()
        };

        // Poll multiple times - should get emergency fuel
        let mut total_polls = 0;
        for _ in 0..15 {
            let mut exec = executor_arc.lock().unwrap();
            let polled = exec.poll_tasks().unwrap();
            total_polls += polled;
            
            if polled == 0 {
                break;
            }
        }

        // Task should complete despite initial budget being too small
        let status = {
            let exec = executor_arc.lock().unwrap();
            exec.get_task_status(task_id).unwrap()
        };
        assert_eq!(status.state, AsyncTaskState::Completed;
    }

    #[test]
    fn test_priority_based_preemption() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(100_000;
        executor.enable_preemption(PreemptionPolicy::PriorityBased).unwrap();
        
        let executor_arc = Arc::new(Mutex::new(executor;
        if let Ok(mut exec) = executor_arc.lock() {
            exec.set_self_ref(Arc::downgrade(&executor_arc;
        }

        // Track execution order
        let execution_order = Arc::new(Mutex::new(Vec::new();

        // Spawn low priority task first
        let order_clone1 = execution_order.clone();
        let low_task = {
            let mut exec = executor_arc.lock().unwrap();
            exec.spawn_task(
                ComponentInstanceId::new(1),
                5000,
                Priority::Low,
                async move {
                    order_clone1.lock().unwrap().push("low_start");
                    // Simulate long running task
                    for _ in 0..5 {
                        // In real impl would yield
                    }
                    order_clone1.lock().unwrap().push("low_end");
                    Ok(())
                },
            ).unwrap()
        };

        // Poll once to start low priority task
        {
            let mut exec = executor_arc.lock().unwrap();
            exec.poll_tasks().unwrap();
        }

        // Now spawn high priority task
        let order_clone2 = execution_order.clone();
        let high_task = {
            let mut exec = executor_arc.lock().unwrap();
            exec.spawn_task(
                ComponentInstanceId::new(1),
                5000,
                Priority::High,
                async move {
                    order_clone2.lock().unwrap().push("high");
                    Ok(())
                },
            ).unwrap()
        };

        // Poll to completion
        for _ in 0..10 {
            let mut exec = executor_arc.lock().unwrap();
            if exec.poll_tasks().unwrap() == 0 {
                break;
            }
        }

        // Verify both completed
        let low_status = executor_arc.lock().unwrap().get_task_status(low_task).unwrap();
        let high_status = executor_arc.lock().unwrap().get_task_status(high_task).unwrap();
        assert_eq!(low_status.state, AsyncTaskState::Completed;
        assert_eq!(high_status.state, AsyncTaskState::Completed;
    }

    #[test]
    fn test_fair_share_allocation() {
        let mut fuel_manager = FuelDynamicManager::new(FuelAllocationPolicy::FairShare, 100_000).unwrap();
        
        // Register multiple components
        for i in 1..=3 {
            fuel_manager.register_component(
                ComponentInstanceId::new(i),
                30_000, // Equal base quota
                Priority::Normal,
            ).unwrap();
        }

        // Calculate allocations
        let alloc1 = fuel_manager.calculate_fuel_allocation(
            crate::threading::task_manager::TaskId::new(1),
            ComponentInstanceId::new(1),
            1000,
            Priority::Normal,
        ).unwrap();

        let alloc2 = fuel_manager.calculate_fuel_allocation(
            crate::threading::task_manager::TaskId::new(2),
            ComponentInstanceId::new(2),
            1000,
            Priority::Normal,
        ).unwrap();

        // Should get equal allocations in fair share mode
        assert_eq!(alloc1, alloc2;
    }

    #[test]
    fn test_fuel_exhaustion_recovery() {
        let mut fuel_manager = FuelDynamicManager::new(FuelAllocationPolicy::Adaptive, 10_000).unwrap();
        let task_id = crate::threading::task_manager::TaskId::new(1;
        
        // Simulate task execution
        fuel_manager.update_task_history(task_id, 1000, 10, false).unwrap();
        
        // Handle exhaustion
        let emergency_fuel = fuel_manager.handle_fuel_exhaustion(task_id).unwrap();
        assert!(emergency_fuel > 0);
        
        // Check reserve was reduced
        let stats = fuel_manager.get_allocation_stats);
        assert!(stats.reserve_fuel < 10_000);
    }

    #[test]
    fn test_preemption_with_dynamic_fuel() {
        let task_manager = Arc::new(Mutex::new(TaskManager::new();
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
        let mut bridge = ComponentAsyncBridge::new(
            task_manager.clone(),
            thread_manager.clone(),
        ).unwrap();

        // Set up with both features
        bridge.set_global_fuel_budget(100_000).unwrap();
        
        // Register component with limited budget
        let component_id = ComponentInstanceId::new(1;
        bridge.register_component(
            component_id,
            10, // max tasks
            20_000, // limited fuel budget
            Priority::Normal,
        ).unwrap();

        // Spawn multiple tasks that compete for fuel
        let mut task_ids = Vec::new);
        let completed = Arc::new(AtomicU64::new(0;

        for i in 0..5 {
            let completed_clone = completed.clone();
            let task_id = bridge.spawn_component_async(
                component_id,
                async move {
                    // Simulate work
                    completed_clone.fetch_add(1, Ordering::AcqRel;
                    Ok(())
                },
                Some(5000), // Each wants 5000 fuel
            ).unwrap();
            task_ids.push(task_id);
        }

        // Poll until all complete or max iterations
        let mut iterations = 0;
        loop {
            let result = bridge.poll_async_tasks().unwrap();
            iterations += 1;
            
            if result.tasks_completed == 5 || iterations > 50 {
                break;
            }
        }

        // All should complete despite limited budget
        assert_eq!(completed.load(Ordering::Acquire), 5;
        
        // Check component stats
        let stats = bridge.get_component_stats(component_id).unwrap();
        assert_eq!(stats.active_tasks, 0;
        assert!(stats.fuel_consumed <= stats.fuel_budget);
    }

    #[test]
    fn test_quantum_based_scheduling() {
        let mut preemption_mgr = FuelPreemptionManager::new(PreemptionPolicy::Cooperative).unwrap();
        
        // Register tasks with different quantums
        let task1 = crate::threading::task_manager::TaskId::new(1;
        let task2 = crate::threading::task_manager::TaskId::new(2;
        
        preemption_mgr.register_task(task1, Priority::Normal, true, 1000).unwrap();
        preemption_mgr.register_task(task2, Priority::Normal, true, 500).unwrap();
        
        // Update quantums
        preemption_mgr.update_quantum(task1, 100).unwrap();
        preemption_mgr.update_quantum(task2, 100).unwrap();
        
        // Task 2 should have less quantum remaining
        // In real usage, this affects scheduling decisions
        
        // Refill quantums
        preemption_mgr.refill_quantums(2000;
        
        let stats = preemption_mgr.get_statistics);
        assert_eq!(stats.total_preemptions, 0); // No preemptions yet
    }

    #[test]
    fn test_allocation_policy_comparison() {
        let base_fuel = 1000;
        let task_id = crate::threading::task_manager::TaskId::new(1;
        let component_id = ComponentInstanceId::new(1;
        
        // Test different policies
        let policies = vec![
            FuelAllocationPolicy::Fixed,
            FuelAllocationPolicy::Adaptive,
            FuelAllocationPolicy::PriorityAdaptive,
            FuelAllocationPolicy::FairShare,
            FuelAllocationPolicy::PerformanceOptimized,
        ];
        
        for policy in policies {
            let mut manager = FuelDynamicManager::new(policy, 100_000).unwrap();
            manager.register_component(component_id, 50_000, Priority::Normal).unwrap();
            
            let allocation = manager.calculate_fuel_allocation(
                task_id,
                component_id,
                base_fuel,
                Priority::Normal,
            ).unwrap();
            
            match policy {
                FuelAllocationPolicy::Fixed => assert_eq!(allocation, base_fuel),
                _ => assert!(allocation > 0), // Other policies may adjust
            }
        }
    }
}