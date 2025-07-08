//! Comprehensive integration tests for the complete async executor system
//!
//! This module tests the full integration of all async components:
//! - Fuel-based async executor with all primitives
//! - Component Model integration
//! - Advanced async primitives (combinators, channels, timers, sync)
//! - Error handling and edge cases
//! - Performance and resource management

#[cfg(test)]
mod tests {
    use crate::{
        async_::{
            fuel_async_executor::FuelAsyncExecutor,
            task_manager_async_bridge::{
                TaskManagerAsyncBridge, BridgeConfiguration, ComponentAsyncTaskType
            },
            async_canonical_abi_support::{
                AsyncCanonicalAbiSupport, AsyncAbiOperationType
            },
            resource_async_operations::{
                ResourceAsyncOperations, ResourceAsyncOperationType
            },
            async_combinators::{
                AsyncCombinators, CombinatorType, create_delay_future
            },
            optimized_async_channels::{
                OptimizedAsyncChannels, ChannelType, SendResult, ReceiveResult
            },
            timer_integration::{
                TimerIntegration, TimerType, TimerProcessResult
            },
            advanced_sync_primitives::{
                AdvancedSyncPrimitives, SyncPrimitiveType, MutexLockResult, SemaphoreAcquireResult
            },
            fuel_dynamic_manager::{FuelDynamicManager, FuelAllocationPolicy},
            fuel_preemption_support::{FuelPreemptionManager, PreemptionPolicy},
        },
        task_manager::{TaskManager, TaskType, TaskState},
        threading::thread_spawn_fuel::FuelTrackedThreadManager,
        canonical_abi::CanonicalOptions,
        types::{ComponentType, FuncType, ValType, Value},
        ComponentInstanceId,
        prelude::*,
    };
    use core::{
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicU64, AtomicU32, Ordering},
        task::{Context, Poll},
        time::Duration,
    };
    use wrt_foundation::{
        component_value::ComponentValue,
        resource::{ResourceHandle, ResourceType},
        Arc, sync::Mutex,
    };
    use wrt_platform::advanced_sync::Priority;

    /// Complete async system under test
    struct AsyncSystemTestHarness {
        bridge: Arc<Mutex<TaskManagerAsyncBridge>>,
        abi_support: AsyncCanonicalAbiSupport,
        resource_ops: ResourceAsyncOperations,
        combinators: AsyncCombinators,
        channels: OptimizedAsyncChannels,
        timers: TimerIntegration,
        sync_primitives: AdvancedSyncPrimitives,
    }

    impl AsyncSystemTestHarness {
        fn new() -> Self {
            let task_manager = Arc::new(Mutex::new(TaskManager::new()));
            let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
            
            let config = BridgeConfiguration {
                enable_preemption: true,
                enable_dynamic_fuel: true,
                fuel_policy: FuelAllocationPolicy::Adaptive,
                preemption_policy: PreemptionPolicy::PriorityBased,
                ..Default::default()
            };
            
            let bridge = Arc::new(Mutex::new(
                TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap()
            ));

            let abi_support = AsyncCanonicalAbiSupport::new(bridge.clone());
            let resource_ops = ResourceAsyncOperations::new(abi_support.clone());
            let combinators = AsyncCombinators::new(bridge.clone());
            let channels = OptimizedAsyncChannels::new(bridge.clone(), None);
            let timers = TimerIntegration::new(bridge.clone(), None);
            let sync_primitives = AdvancedSyncPrimitives::new(bridge.clone(), None);

            Self {
                bridge,
                abi_support,
                resource_ops,
                combinators,
                channels,
                timers,
                sync_primitives,
            }
        }

        fn initialize_component(&mut self, component_id: ComponentInstanceId) -> Result<(), Error> {
            // Initialize all subsystems for this component
            {
                let mut bridge = self.bridge.lock()?;
                bridge.initialize_component_async(component_id, None)?;
            }
            
            self.abi_support.initialize_component_abi(component_id, CanonicalOptions::default())?;
            self.resource_ops.initialize_component_resources(component_id, None, None)?;
            self.channels.initialize_component_channels(component_id, None)?;
            self.timers.initialize_component_timers(component_id, None)?;
            self.sync_primitives.initialize_component_sync(component_id, None)?;
            
            Ok(())
        }
    }

    /// Complex async task that uses multiple primitives
    struct ComplexAsyncTask {
        id: u64,
        polls_remaining: u64,
        use_channels: bool,
        use_timers: bool,
        use_sync: bool,
        result_value: ComponentValue,
    }

    impl Future for ComplexAsyncTask {
        type Output = Result<Vec<ComponentValue>, Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.polls_remaining == 0 {
                Poll::Ready(Ok(vec![
                    self.result_value.clone(),
                    ComponentValue::U32(self.id as u32),
                ]))
            } else {
                self.polls_remaining -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_full_system_integration() {
        let mut harness = AsyncSystemTestHarness::new();
        
        // Initialize multiple components
        let component1 = ComponentInstanceId::new(1);
        let component2 = ComponentInstanceId::new(2);
        
        harness.initialize_component(component1).unwrap();
        harness.initialize_component(component2).unwrap();

        // Test cross-component async operations
        {
            let mut bridge = harness.bridge.lock().unwrap();
            
            // Spawn tasks in both components
            let task1 = bridge.spawn_async_task(
                component1,
                Some(0),
                ComplexAsyncTask {
                    id: 1,
                    polls_remaining: 3,
                    use_channels: true,
                    use_timers: false,
                    use_sync: false,
                    result_value: ComponentValue::U32(100),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal,
            ).unwrap();

            let task2 = bridge.spawn_async_task(
                component2,
                Some(1),
                ComplexAsyncTask {
                    id: 2,
                    polls_remaining: 2,
                    use_channels: false,
                    use_timers: true,
                    use_sync: false,
                    result_value: ComponentValue::U32(200),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::High,
            ).unwrap();

            // Poll until completion
            for _ in 0..20 {
                let result = bridge.poll_async_tasks().unwrap();
                if result.tasks_completed >= 2 {
                    break;
                }
            }

            let stats = bridge.get_bridge_statistics();
            assert!(stats.total_async_tasks >= 2);
            assert_eq!(stats.active_components, 2);
        }
    }

    #[test]
    fn test_async_combinators_with_channels() {
        let mut harness = AsyncSystemTestHarness::new();
        let component_id = ComponentInstanceId::new(1);
        harness.initialize_component(component_id).unwrap();

        // Create channel for communication
        let (sender, receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Bounded(8),
        ).unwrap();

        // Create futures that use channels
        let send_future = async move {
            for i in 0..5 {
                let _ = sender.send(ComponentValue::U32(i)).await;
            }
            Ok(ComponentValue::U32(42))
        };

        let recv_future = async move {
            let mut sum = 0u32;
            for _ in 0..5 {
                if let Ok(value) = receiver.receive().await {
                    if let ComponentValue::U32(n) = value {
                        sum += n;
                    }
                }
            }
            Ok(ComponentValue::U32(sum))
        };

        // Use combinator to join both futures
        let join_id = harness.combinators.join(
            component_id,
            vec![
                Box::pin(send_future),
                Box::pin(recv_future),
            ],
        ).unwrap();

        // Poll combinator
        for _ in 0..50 {
            let result = harness.combinators.poll_combinators().unwrap();
            if result.completed_combinators > 0 {
                break;
            }
        }

        let stats = harness.combinators.get_combinator_statistics();
        assert_eq!(stats.total_joins, 1);
    }

    #[test]
    fn test_timer_with_sync_primitives() {
        let mut harness = AsyncSystemTestHarness::new();
        let component_id = ComponentInstanceId::new(1);
        harness.initialize_component(component_id).unwrap();

        // Create mutex and semaphore
        let mutex_id = harness.sync_primitives.create_async_mutex(component_id, false).unwrap();
        let semaphore_id = harness.sync_primitives.create_async_semaphore(component_id, 2, true).unwrap();

        // Create timers
        let timer1 = harness.timers.create_timer(
            component_id,
            TimerType::Oneshot,
            100,
        ).unwrap();

        let timer2 = harness.timers.create_timer(
            component_id,
            TimerType::Interval(50),
            50,
        ).unwrap();

        // Simulate time passage and process timers
        for step in 0..10 {
            harness.timers.advance_time(20);
            let timer_result = harness.timers.process_timers().unwrap();
            
            if step == 2 { // Around 60ms
                // Timer should have fired, test sync operations
                let task_id = crate::task_manager::TaskId::new(1);
                
                let mutex_result = harness.sync_primitives.lock_async_mutex(
                    mutex_id, 
                    task_id, 
                    component_id
                ).unwrap();
                assert_eq!(mutex_result, MutexLockResult::Acquired);

                let sem_result = harness.sync_primitives.acquire_semaphore(
                    semaphore_id,
                    task_id,
                    component_id,
                ).unwrap();
                assert_eq!(sem_result, SemaphoreAcquireResult::Acquired);
            }
        }

        let timer_stats = harness.timers.get_timer_statistics();
        assert!(timer_stats.total_timers_created >= 2);

        let sync_stats = harness.sync_primitives.get_sync_statistics();
        assert_eq!(sync_stats.total_mutexes_created, 1);
        assert_eq!(sync_stats.total_semaphores_created, 1);
    }

    #[test]
    fn test_resource_operations_with_combinators() {
        let mut harness = AsyncSystemTestHarness::new();
        let component_id = ComponentInstanceId::new(1);
        harness.initialize_component(component_id).unwrap();

        // Create multiple async resource operations
        let resource_type = ResourceType::new("TestResource".to_string());
        
        let create_op1 = harness.resource_ops.async_create_resource(
            component_id,
            resource_type.clone(),
            vec![ComponentValue::U32(100)],
            None,
        ).unwrap();

        let create_op2 = harness.resource_ops.async_create_resource(
            component_id,
            resource_type.clone(),
            vec![ComponentValue::U32(200)],
            None,
        ).unwrap();

        // Create delay futures that simulate resource operations
        let resource_future1 = create_delay_future(100, ComponentValue::U32(1));
        let resource_future2 = create_delay_future(150, ComponentValue::U32(2));

        // Race the resource operations
        let race_id = harness.combinators.race(
            component_id,
            vec![resource_future1, resource_future2],
        ).unwrap();

        // Poll everything
        for _ in 0..30 {
            let _ = harness.resource_ops.poll_resource_operations();
            let combinator_result = harness.combinators.poll_combinators().unwrap();
            
            if combinator_result.completed_combinators > 0 {
                break;
            }
        }

        let resource_stats = harness.resource_ops.get_resource_statistics();
        assert_eq!(resource_stats.total_creates, 2);

        let combinator_stats = harness.combinators.get_combinator_statistics();
        assert_eq!(combinator_stats.total_races, 1);
    }

    #[test]
    fn test_component_isolation_under_load() {
        let mut harness = AsyncSystemTestHarness::new();
        
        // Create multiple components
        let components: Vec<ComponentInstanceId> = (1..=5)
            .map(|i| ComponentInstanceId::new(i))
            .collect();

        for &component_id in &components {
            harness.initialize_component(component_id).unwrap();
        }

        // Create resources and operations in each component
        for (i, &component_id) in components.iter().enumerate() {
            // Create channels
            let _ = harness.channels.create_channel(
                component_id,
                ChannelType::Bounded(16),
            );

            // Create timers
            let _ = harness.timers.create_timer(
                component_id,
                TimerType::Interval(100 + i as u64 * 10),
                100 + i as u64 * 10,
            );

            // Create sync primitives
            let _ = harness.sync_primitives.create_async_mutex(component_id, false);
            let _ = harness.sync_primitives.create_async_semaphore(component_id, 3, true);

            // Create async tasks
            let mut bridge = harness.bridge.lock().unwrap();
            let _ = bridge.spawn_async_task(
                component_id,
                Some(i as u32),
                ComplexAsyncTask {
                    id: i as u64 + 1,
                    polls_remaining: 5 + i as u64,
                    use_channels: i % 2 == 0,
                    use_timers: i % 3 == 0,
                    use_sync: i % 4 == 0,
                    result_value: ComponentValue::U32((i + 1) as u32 * 100),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal,
            );
        }

        // Simulate system load
        for round in 0..100 {
            // Poll all subsystems
            {
                let mut bridge = harness.bridge.lock().unwrap();
                let _ = bridge.poll_async_tasks();
            }
            
            let _ = harness.combinators.poll_combinators();
            let _ = harness.resource_ops.poll_resource_operations();
            
            // Advance time and process timers
            harness.timers.advance_time(10);
            let _ = harness.timers.process_timers();

            // Verify component isolation
            let bridge_stats = {
                let bridge = harness.bridge.lock().unwrap();
                bridge.get_bridge_statistics()
            };
            
            assert_eq!(bridge_stats.active_components, 5);
            
            if round % 20 == 0 {
                // Check that all components are still functional
                let timer_stats = harness.timers.get_timer_statistics();
                let sync_stats = harness.sync_primitives.get_sync_statistics();
                let channel_stats = harness.channels.get_channel_statistics();
                
                assert!(timer_stats.active_timers <= 5);
                assert!(sync_stats.active_primitives <= 10); // 2 per component
                assert!(channel_stats.active_channels <= 5);
            }
        }

        // Final verification - all components should still be active
        let final_bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        assert_eq!(final_bridge_stats.active_components, 5);
    }

    #[test]
    fn test_error_propagation_and_recovery() {
        let mut harness = AsyncSystemTestHarness::new();
        let component_id = ComponentInstanceId::new(1);
        harness.initialize_component(component_id).unwrap();

        // Test various error conditions
        
        // 1. Invalid resource operations
        let invalid_resource = ResourceHandle::new(999);
        let result = harness.resource_ops.async_call_resource_method(
            component_id,
            invalid_resource,
            "nonexistent".to_string(),
            vec![],
            None,
        );
        assert!(result.is_err());

        // 2. Component limits exceeded
        // Try to create too many timers
        let mut timer_ids = Vec::new();
        for i in 0..200 { // Exceed limit
            match harness.timers.create_timer(
                component_id,
                TimerType::Oneshot,
                1000,
            ) {
                Ok(id) => timer_ids.push(id),
                Err(_) => break, // Expected to fail at some point
            }
        }
        
        // Should have hit limit before creating 200 timers
        assert!(timer_ids.len() < 200);

        // 3. Invalid combinator operations
        let empty_futures: Vec<Box<dyn Future<Output = Result<ComponentValue, Error>> + Send>> = vec![];
        let result = harness.combinators.select(component_id, empty_futures);
        assert!(result.is_err());

        // 4. Channel operations on closed channels
        let (sender, receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Oneshot,
        ).unwrap();
        
        // Close the channel
        let channel_id = sender.channel_id;
        harness.channels.close_channel(channel_id).unwrap();
        
        // Try to send on closed channel should handle gracefully
        let send_result = harness.channels.send_message(
            channel_id,
            component_id,
            ComponentValue::U32(42),
            None,
        ).unwrap();
        
        // Should indicate channel is closed
        assert_eq!(send_result, crate::async_::optimized_async_channels::SendResult::Closed);

        // System should still be functional after errors
        let bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        assert_eq!(bridge_stats.active_components, 1);
    }

    #[test]
    fn test_fuel_consumption_and_limits() {
        let mut harness = AsyncSystemTestHarness::new();
        let component_id = ComponentInstanceId::new(1);
        harness.initialize_component(component_id).unwrap();

        // Track initial fuel state
        let initial_bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };

        // Perform various fuel-consuming operations
        
        // 1. Timer operations
        let timer_id = harness.timers.create_timer(
            component_id,
            TimerType::Oneshot,
            100,
        ).unwrap();
        
        harness.timers.advance_time(150);
        let _ = harness.timers.process_timers();

        // 2. Channel operations
        let (sender, receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Bounded(4),
        ).unwrap();

        for i in 0..4 {
            let _ = harness.channels.send_message(
                sender.channel_id,
                component_id,
                ComponentValue::U32(i),
                None,
            );
        }

        for _ in 0..4 {
            let _ = harness.channels.receive_message(
                receiver.channel_id,
                component_id,
            );
        }

        // 3. Sync primitive operations
        let mutex_id = harness.sync_primitives.create_async_mutex(component_id, false).unwrap();
        let task_id = crate::task_manager::TaskId::new(1);
        
        let _ = harness.sync_primitives.lock_async_mutex(mutex_id, task_id, component_id);
        let _ = harness.sync_primitives.unlock_async_mutex(mutex_id, task_id);

        // 4. ABI operations
        let _ = harness.abi_support.async_call(
            component_id,
            "test_function".to_string(),
            FuncType::new(vec![ValType::I32], vec![ValType::I32]),
            vec![ComponentValue::U32(42)],
            None,
        );

        // Check fuel consumption
        let timer_stats = harness.timers.get_timer_statistics();
        let channel_stats = harness.channels.get_channel_statistics();
        let sync_stats = harness.sync_primitives.get_sync_statistics();
        let abi_stats = harness.abi_support.get_abi_statistics();

        // All operations should have consumed fuel
        assert!(timer_stats.total_fuel_consumed > 0);
        assert!(channel_stats.total_fuel_consumed > 0);
        assert!(sync_stats.total_fuel_consumed > 0);
        assert!(abi_stats.total_fuel_consumed > 0);

        // Bridge should show increased fuel consumption
        let final_bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        
        assert!(final_bridge_stats.total_fuel_consumed > initial_bridge_stats.total_fuel_consumed);
    }

    #[test]
    fn test_concurrent_primitive_usage() {
        let mut harness = AsyncSystemTestHarness::new();
        let component_id = ComponentInstanceId::new(1);
        harness.initialize_component(component_id).unwrap();

        // Create primitives that will be used concurrently
        let mutex_id = harness.sync_primitives.create_async_mutex(component_id, false).unwrap();
        let semaphore_id = harness.sync_primitives.create_async_semaphore(component_id, 3, true).unwrap();
        let barrier_id = harness.sync_primitives.create_async_barrier(component_id, 3).unwrap();

        let (sender, receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Bounded(16),
        ).unwrap();

        // Simulate concurrent access from multiple tasks
        let task_ids: Vec<_> = (1..=5).map(|i| crate::task_manager::TaskId::new(i)).collect();

        // Test concurrent mutex access
        let mut mutex_results = Vec::new();
        for &task_id in &task_ids {
            let result = harness.sync_primitives.lock_async_mutex(
                mutex_id,
                task_id,
                component_id,
            ).unwrap();
            mutex_results.push(result);
        }

        // Only one should acquire, others should block
        let acquired_count = mutex_results.iter()
            .filter(|&&r| r == MutexLockResult::Acquired)
            .count();
        assert_eq!(acquired_count, 1);

        // Test concurrent semaphore access
        let mut sem_results = Vec::new();
        for &task_id in &task_ids {
            let result = harness.sync_primitives.acquire_semaphore(
                semaphore_id,
                task_id,
                component_id,
            ).unwrap();
            sem_results.push(result);
        }

        // Up to 3 should acquire (semaphore permits), others should block
        let sem_acquired_count = sem_results.iter()
            .filter(|&&r| r == SemaphoreAcquireResult::Acquired)
            .count();
        assert!(sem_acquired_count <= 3);

        // Test concurrent channel access
        // Multiple senders
        for i in 0..10 {
            let result = harness.channels.send_message(
                sender.channel_id,
                component_id,
                ComponentValue::U32(i),
                None,
            ).unwrap();
            
            // Should eventually succeed or indicate backpressure
            assert!(matches!(result, 
                crate::async_::optimized_async_channels::SendResult::Sent |
                crate::async_::optimized_async_channels::SendResult::WouldBlock
            );
        }

        // Multiple receivers
        let mut received_count = 0;
        for _ in 0..20 {
            match harness.channels.receive_message(receiver.channel_id, component_id) {
                Ok(crate::async_::optimized_async_channels::ReceiveResult::Received(_)) => {
                    received_count += 1;
                },
                Ok(crate::async_::optimized_async_channels::ReceiveResult::WouldBlock) => break,
                _ => break,
            }
        }

        assert!(received_count > 0);

        // Verify system state is consistent
        let sync_stats = harness.sync_primitives.get_sync_statistics();
        let channel_stats = harness.channels.get_channel_statistics();

        assert!(sync_stats.total_mutex_locks > 0);
        assert!(sync_stats.total_semaphore_acquires > 0);
        assert!(channel_stats.total_messages_sent > 0);
        assert!(channel_stats.total_messages_received > 0);
    }
}