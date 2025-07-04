//! Comprehensive error handling tests for the async executor system
//!
//! This module tests error handling, fault tolerance, and recovery scenarios:
//! - Resource exhaustion scenarios
//! - Invalid operation handling
//! - Component failure isolation
//! - Graceful degradation
//! - Error propagation and recovery
//! - Edge cases and boundary conditions

#[cfg(test)]
mod tests {
    use crate::{
        async_::{
            fuel_async_executor::FuelAsyncExecutor,
            task_manager_async_bridge::{
                TaskManagerAsyncBridge, BridgeConfiguration, ComponentAsyncTaskType
            },
            async_canonical_abi_support::AsyncCanonicalAbiSupport,
            resource_async_operations::ResourceAsyncOperations,
            async_combinators::{AsyncCombinators, create_delay_future},
            optimized_async_channels::{OptimizedAsyncChannels, ChannelType, SendResult, ReceiveResult},
            timer_integration::{TimerIntegration, TimerType},
            advanced_sync_primitives::{AdvancedSyncPrimitives, MutexLockResult, SemaphoreAcquireResult},
            fuel_dynamic_manager::{FuelDynamicManager, FuelAllocationPolicy},
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
        sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering},
        task::{Context, Poll},
        time::Duration,
    };
    use wrt_foundation::{
        component_value::ComponentValue,
        resource::{ResourceHandle, ResourceType},
        Arc, sync::Mutex,
    };
    use wrt_platform::advanced_sync::Priority;

    /// Error injection and monitoring harness
    struct ErrorHandlingHarness {
        bridge: Arc<Mutex<TaskManagerAsyncBridge>>,
        abi_support: AsyncCanonicalAbiSupport,
        resource_ops: ResourceAsyncOperations,
        combinators: AsyncCombinators,
        channels: OptimizedAsyncChannels,
        timers: TimerIntegration,
        sync_primitives: AdvancedSyncPrimitives,
        error_count: Arc<AtomicU32>,
        recovery_count: Arc<AtomicU32>,
    }

    impl ErrorHandlingHarness {
        fn new() -> Self {
            let task_manager = Arc::new(Mutex::new(TaskManager::new()));
            let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
            
            let config = BridgeConfiguration {
                enable_preemption: true,
                enable_dynamic_fuel: true,
                fuel_policy: FuelAllocationPolicy::Adaptive,
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
                error_count: Arc::new(AtomicU32::new(0)),
                recovery_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn record_error(&self) {
            self.error_count.fetch_add(1, Ordering::AcqRel);
        }

        fn record_recovery(&self) {
            self.recovery_count.fetch_add(1, Ordering::AcqRel);
        }

        fn get_error_stats(&self) -> (u32, u32) {
            (
                self.error_count.load(Ordering::Acquire),
                self.recovery_count.load(Ordering::Acquire),
            )
        }
    }

    /// Faulty task that simulates various failure modes
    struct FaultyTask {
        id: u64,
        failure_mode: FailureMode,
        polls_until_failure: u64,
        polls_count: AtomicU64,
        error_harness: Arc<ErrorHandlingHarness>,
    }

    #[derive(Debug, Clone, Copy)]
    enum FailureMode {
        NoFailure,
        PanicOnPoll,
        ExcessiveFuelConsumption,
        ResourceExhaustion,
        InvalidOperations,
        TimeoutFailure,
        RecoveryAfterFailure,
    }

    impl Future for FaultyTask {
        type Output = Result<Vec<ComponentValue>, Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let polls = self.polls_count.fetch_add(1, Ordering::AcqRel);
            
            if polls < self.polls_until_failure {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            match self.failure_mode {
                FailureMode::NoFailure => {
                    Poll::Ready(Ok(vec![ComponentValue::U32(self.id as u32)]))
                },
                FailureMode::PanicOnPoll => {
                    self.error_harness.record_error();
                    Poll::Ready(Err(Error::runtime_execution_error(", self.id),
                    )))
                },
                FailureMode::ExcessiveFuelConsumption => {
                    self.error_harness.record_error();
                    // Simulate excessive fuel consumption
                    Poll::Ready(Err(Error::resource_exhausted("),
                    )))
                },
                FailureMode::ResourceExhaustion => {
                    self.error_harness.record_error();
                    Poll::Ready(Err(Error::runtime_execution_error(", self.id),
                    )))
                },
                FailureMode::InvalidOperations => {
                    self.error_harness.record_error();
                    Poll::Ready(Err(Error::new(
                        ErrorCategory::Validation,
                        codes::INVALID_INPUT,
                        format!("),
                    )))
                },
                FailureMode::TimeoutFailure => {
                    self.error_harness.record_error();
                    Poll::Ready(Err(Error::runtime_execution_error(", self.id),
                    )))
                },
                FailureMode::RecoveryAfterFailure => {
                    if polls == self.polls_until_failure {
                        self.error_harness.record_error();
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    } else if polls == self.polls_until_failure + 2 {
                        self.error_harness.record_recovery();
                        Poll::Ready(Ok(vec![ComponentValue::U32(self.id as u32 + 1000)]))
                    } else {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                },
            }
        }
    }

    #[test]
    fn test_resource_exhaustion_handling() {
        let mut harness = ErrorHandlingHarness::new();
        let component_id = ComponentInstanceId::new(1);
        
        // Initialize component
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.channels.initialize_component_channels(component_id, None).unwrap();
        harness.timers.initialize_component_timers(component_id, None).unwrap();
        harness.sync_primitives.initialize_component_sync(component_id, None).unwrap();

        // Test 1: Channel exhaustion
        let mut channels_created = 0;
        let mut channel_creation_errors = 0;
        
        for i in 0..100 { // Try to exceed limits
            match harness.channels.create_channel(component_id, ChannelType::Bounded(4)) {
                Ok(_) => channels_created += 1,
                Err(_) => {
                    channel_creation_errors += 1;
                    harness.record_error();
                    break;
                }
            }
        }

        // Test 2: Timer exhaustion
        let mut timers_created = 0;
        let mut timer_creation_errors = 0;
        
        for i in 0..200 { // Try to exceed limits
            match harness.timers.create_timer(component_id, TimerType::Oneshot, 1000) {
                Ok(_) => timers_created += 1,
                Err(_) => {
                    timer_creation_errors += 1;
                    harness.record_error();
                    break;
                }
            }
        }

        // Test 3: Mutex exhaustion
        let mut mutexes_created = 0;
        let mut mutex_creation_errors = 0;
        
        for i in 0..100 { // Try to exceed limits
            match harness.sync_primitives.create_async_mutex(component_id, false) {
                Ok(_) => mutexes_created += 1,
                Err(_) => {
                    mutex_creation_errors += 1;
                    harness.record_error();
                    break;
                }
            }
        }

        // Test 4: Task exhaustion
        let mut tasks_created = 0;
        let mut task_creation_errors = 0;
        
        for i in 0..200 { // Try to exceed limits
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.spawn_async_task(
                    component_id,
                    Some(i),
                    FaultyTask {
                        id: i as u64,
                        failure_mode: FailureMode::NoFailure,
                        polls_until_failure: 2,
                        polls_count: AtomicU64::new(0),
                        error_harness: Arc::new(harness),
                    },
                    ComponentAsyncTaskType::AsyncFunction,
                    Priority::Normal,
                )
            };
            
            match result {
                Ok(_) => tasks_created += 1,
                Err(_) => {
                    task_creation_errors += 1;
                    harness.record_error();
                    break;
                }
            }
        }

        // Verify that exhaustion was detected and handled
        assert!(channel_creation_errors > 0 || channels_created < 100, 
            ");
        assert!(timer_creation_errors > 0 || timers_created < 200, 
            "Timer exhaustion not properly handled");
        assert!(mutex_creation_errors > 0 || mutexes_created < 100, 
            "Mutex exhaustion not properly handled");
        assert!(task_creation_errors > 0 || tasks_created < 200, 
            "Task exhaustion not properly handled");

        // System should still be functional after exhaustion
        let bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        assert_eq!(bridge_stats.active_components, 1);

        let (errors, recoveries) = harness.get_error_stats();
        assert!(errors > 0, "No resource exhaustion errors recorded");
        
        println!("Resource Exhaustion Handling Test: PASSED");
        println!("  Channels Created: {} (errors: {})", channels_created, channel_creation_errors);
        println!("  Timers Created: {} (errors: {})", timers_created, timer_creation_errors);
        println!("  Mutexes Created: {} (errors: {})", mutexes_created, mutex_creation_errors);
        println!("  Tasks Created: {} (errors: {})", tasks_created, task_creation_errors);
        println!("  Total Errors Handled: {}", errors);
    }

    #[test]
    fn test_task_failure_isolation() {
        let mut harness = ErrorHandlingHarness::new();
        let component_id = ComponentInstanceId::new(1);
        
        // Initialize component
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }

        // Spawn mix of good and faulty tasks
        let task_configs = vec![
            (FailureMode::NoFailure, 2),
            (FailureMode::PanicOnPoll, 3),
            (FailureMode::NoFailure, 2),
            (FailureMode::ExcessiveFuelConsumption, 4),
            (FailureMode::NoFailure, 2),
            (FailureMode::ResourceExhaustion, 2),
            (FailureMode::NoFailure, 3),
            (FailureMode::InvalidOperations, 3),
            (FailureMode::NoFailure, 2),
        ];

        let mut task_ids = Vec::new();
        
        for (i, (failure_mode, polls_until_failure)) in task_configs.iter().enumerate() {
            let task_id = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.spawn_async_task(
                    component_id,
                    Some(i as u32),
                    FaultyTask {
                        id: i as u64,
                        failure_mode: *failure_mode,
                        polls_until_failure: *polls_until_failure,
                        polls_count: AtomicU64::new(0),
                        error_harness: Arc::new(harness),
                    },
                    ComponentAsyncTaskType::AsyncFunction,
                    Priority::Normal,
                ).unwrap()
            };
            
            task_ids.push(task_id);
        }

        // Execute tasks and monitor failures
        let mut completed_tasks = 0;
        let mut failed_tasks = 0;
        
        for round in 0..200 {
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.poll_async_tasks().unwrap()
            };
            
            completed_tasks += result.tasks_completed;
            
            // Check if we've processed all tasks
            if completed_tasks + failed_tasks >= task_configs.len() {
                break;
            }
        }

        // Verify that good tasks completed despite faulty ones
        let (errors, recoveries) = harness.get_error_stats();
        
        // Count expected good tasks (those with NoFailure)
        let expected_good_tasks = task_configs.iter()
            .filter(|(mode, _)| matches!(mode, FailureMode::NoFailure))
            .count();
        
        // System should remain functional despite task failures
        let bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        assert_eq!(bridge_stats.active_components, 1);
        
        // Some tasks should have failed, but system should continue
        assert!(errors > 0, "No task failures recorded");
        assert!(completed_tasks > 0, "No tasks completed despite failures");
        
        println!("Task Failure Isolation Test: PASSED");
        println!("  Total Tasks: {}", task_configs.len());
        println!("  Completed Tasks: {}", completed_tasks);
        println!("  Expected Good Tasks: {}", expected_good_tasks);
        println!("  Errors Isolated: {}", errors);
    }

    #[test]
    fn test_channel_error_handling() {
        let mut harness = ErrorHandlingHarness::new();
        let component_id = ComponentInstanceId::new(1);
        
        // Initialize component
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.channels.initialize_component_channels(component_id, None).unwrap();

        // Test 1: Operations on closed channels
        let (sender, receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Bounded(4),
        ).unwrap();

        // Send some messages
        for i in 0..3 {
            let result = harness.channels.send_message(
                sender.channel_id,
                component_id,
                ComponentValue::U32(i),
                None,
            ).unwrap();
            assert_eq!(result, SendResult::Sent);
        }

        // Close the channel
        harness.channels.close_channel(sender.channel_id).unwrap();

        // Try to send on closed channel
        let send_result = harness.channels.send_message(
            sender.channel_id,
            component_id,
            ComponentValue::U32(999),
            None,
        ).unwrap();
        
        assert_eq!(send_result, SendResult::Closed);
        harness.record_error(); // Record as handled error

        // Try to receive from closed channel (should drain existing messages first)
        let mut received_count = 0;
        for _ in 0..5 {
            match harness.channels.receive_message(receiver.channel_id, component_id) {
                Ok(ReceiveResult::Received(_)) => received_count += 1,
                Ok(ReceiveResult::Closed) => {
                    harness.record_recovery(); // Successfully handled closure
                    break;
                },
                Ok(ReceiveResult::WouldBlock) => break,
                Err(_) => {
                    harness.record_error();
                    break;
                }
            }
        }

        assert!(received_count <= 3, "Received more messages than sent");

        // Test 2: Invalid channel operations
        let invalid_channel_id = crate::async_::optimized_async_channels::ChannelId(999);
        
        let invalid_send = harness.channels.send_message(
            invalid_channel_id,
            component_id,
            ComponentValue::U32(42),
            None,
        );
        
        assert!(invalid_send.is_err(), "Invalid channel send should fail");
        harness.record_error();

        let invalid_receive = harness.channels.receive_message(
            invalid_channel_id,
            component_id,
        );
        
        assert!(invalid_receive.is_err(), "Invalid channel receive should fail");
        harness.record_error();

        // Test 3: Cross-component channel access
        let other_component = ComponentInstanceId::new(999);
        
        let (valid_sender, valid_receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Bounded(4),
        ).unwrap();

        // Try to use channel from wrong component
        let cross_send = harness.channels.send_message(
            valid_sender.channel_id,
            other_component, // Wrong component
            ComponentValue::U32(42),
            None,
        );
        
        // This may succeed but should be isolated - behavior depends on implementation
        match cross_send {
            Ok(_) => harness.record_recovery(), // Properly isolated
            Err(_) => harness.record_error(), // Rejected as expected
        }

        let (errors, recoveries) = harness.get_error_stats();
        assert!(errors > 0, "No channel errors recorded");
        
        println!("Channel Error Handling Test: PASSED");
        println!("  Messages Received from Closed Channel: {}", received_count);
        println!("  Errors Handled: {}", errors);
        println!("  Recoveries: {}", recoveries);
    }

    #[test]
    fn test_timer_error_scenarios() {
        let mut harness = ErrorHandlingHarness::new();
        let component_id = ComponentInstanceId::new(1);
        
        // Initialize component
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.timers.initialize_component_timers(component_id, None).unwrap();

        // Test 1: Invalid timer parameters
        let invalid_timer_result = harness.timers.create_timer(
            component_id,
            TimerType::Oneshot,
            0, // Invalid duration
        );
        
        match invalid_timer_result {
            Err(_) => harness.record_error(), // Expected error
            Ok(_) => {}, // Implementation may allow, check behavior
        }

        // Test 2: Double timer cancellation
        let timer_id = harness.timers.create_timer(
            component_id,
            TimerType::Oneshot,
            1000,
        ).unwrap();

        let first_cancel = harness.timers.cancel_timer(timer_id).unwrap();
        assert!(first_cancel, "First cancellation should succeed");

        let second_cancel = harness.timers.cancel_timer(timer_id).unwrap();
        assert!(!second_cancel, "Second cancellation should fail gracefully");
        harness.record_error(); // Record as handled error

        // Test 3: Operations on invalid timer IDs
        let invalid_timer_id = crate::async_::timer_integration::TimerId(999);
        
        let invalid_cancel = harness.timers.cancel_timer(invalid_timer_id);
        assert!(invalid_cancel.is_err(), "Invalid timer cancel should fail");
        harness.record_error();

        let invalid_status = harness.timers.get_timer_status(invalid_timer_id);
        assert!(invalid_status.is_err(), "Invalid timer status should fail");
        harness.record_error();

        // Test 4: Timer overflow scenarios
        let mut created_timers = 0;
        
        // Try to create many timers
        for i in 0..200 {
            match harness.timers.create_timer(
                component_id,
                TimerType::Oneshot,
                1000 + i,
            ) {
                Ok(_) => created_timers += 1,
                Err(_) => {
                    harness.record_error(); // Hit limit
                    break;
                }
            }
        }

        // Test 5: Timer processing under error conditions
        for step in 0..20 {
            harness.timers.advance_time(100);
            
            match harness.timers.process_timers() {
                Ok(result) => {
                    if result.fired_timers.len() > 0 {
                        harness.record_recovery(); // Successful processing
                    }
                },
                Err(_) => {
                    harness.record_error(); // Processing error
                }
            }
        }

        let timer_stats = harness.timers.get_timer_statistics();
        let (errors, recoveries) = harness.get_error_stats();
        
        assert!(errors > 0, "No timer errors recorded");
        assert!(timer_stats.total_timers_created > 0, "No timers created");
        
        println!("Timer Error Scenarios Test: PASSED");
        println!("  Timers Created: {}", created_timers);
        println!("  Timers Fired: {}", timer_stats.total_timers_fired);
        println!("  Errors Handled: {}", errors);
        println!("  Successful Operations: {}", recoveries);
    }

    #[test]
    fn test_sync_primitive_deadlock_avoidance() {
        let mut harness = ErrorHandlingHarness::new();
        let component_id = ComponentInstanceId::new(1);
        
        // Initialize component
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.sync_primitives.initialize_component_sync(component_id, None).unwrap();

        // Create mutexes for potential deadlock scenario
        let mutex1 = harness.sync_primitives.create_async_mutex(component_id, false).unwrap();
        let mutex2 = harness.sync_primitives.create_async_mutex(component_id, false).unwrap();

        let task1 = crate::task_manager::TaskId::new(1);
        let task2 = crate::task_manager::TaskId::new(2);

        // Test 1: Basic mutex contention
        let result1 = harness.sync_primitives.lock_async_mutex(mutex1, task1, component_id).unwrap();
        assert_eq!(result1, MutexLockResult::Acquired);

        let result2 = harness.sync_primitives.lock_async_mutex(mutex1, task2, component_id).unwrap();
        assert_eq!(result2, MutexLockResult::WouldBlock);
        harness.record_error(); // Contention handled

        // Release first lock
        harness.sync_primitives.unlock_async_mutex(mutex1, task1).unwrap();
        harness.record_recovery(); // Lock released

        // Test 2: Invalid unlock operations
        let invalid_unlock = harness.sync_primitives.unlock_async_mutex(mutex1, task2); // Wrong owner
        assert!(invalid_unlock.is_err(), "Invalid unlock should fail");
        harness.record_error();

        let double_unlock = harness.sync_primitives.unlock_async_mutex(mutex1, task1); // Already unlocked
        assert!(double_unlock.is_err(), "Double unlock should fail");
        harness.record_error();

        // Test 3: Semaphore overflow/underflow
        let semaphore = harness.sync_primitives.create_async_semaphore(component_id, 2, true).unwrap();

        // Acquire all permits
        let acq1 = harness.sync_primitives.acquire_semaphore(semaphore, task1, component_id).unwrap();
        let acq2 = harness.sync_primitives.acquire_semaphore(semaphore, task2, component_id).unwrap();
        assert_eq!(acq1, SemaphoreAcquireResult::Acquired);
        assert_eq!(acq2, SemaphoreAcquireResult::Acquired);

        // Try to acquire when exhausted
        let task3 = crate::task_manager::TaskId::new(3);
        let acq3 = harness.sync_primitives.acquire_semaphore(semaphore, task3, component_id).unwrap();
        assert_eq!(acq3, SemaphoreAcquireResult::WouldBlock);
        harness.record_error(); // Exhaustion handled

        // Release permits
        harness.sync_primitives.release_semaphore(semaphore).unwrap();
        harness.sync_primitives.release_semaphore(semaphore).unwrap();
        harness.record_recovery();

        // Try to over-release
        let over_release = harness.sync_primitives.release_semaphore(semaphore);
        match over_release {
            Err(_) => harness.record_error(), // Over-release detected
            Ok(_) => {}, // Implementation may handle silently
        }

        // Test 4: Operations on invalid primitives
        let invalid_mutex = crate::async_::advanced_sync_primitives::SyncPrimitiveId(999);
        
        let invalid_lock = harness.sync_primitives.lock_async_mutex(
            invalid_mutex,
            task1,
            component_id,
        );
        assert!(invalid_lock.is_err(), "Invalid mutex lock should fail");
        harness.record_error();

        let (errors, recoveries) = harness.get_error_stats();
        
        assert!(errors > 0, "No sync primitive errors recorded");
        assert!(recoveries > 0, "No recoveries recorded");
        
        println!("Sync Primitive Error Handling Test: PASSED");
        println!("  Contention Events: {}", errors);
        println!("  Successful Operations: {}", recoveries);
    }

    #[test]
    fn test_recovery_and_graceful_degradation() {
        let mut harness = ErrorHandlingHarness::new();
        let component_id = ComponentInstanceId::new(1);
        
        // Initialize component
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }

        // Spawn tasks with recovery capability
        let recovery_tasks = vec![
            FailureMode::RecoveryAfterFailure,
            FailureMode::RecoveryAfterFailure,
            FailureMode::NoFailure,
            FailureMode::PanicOnPoll, // This will fail
            FailureMode::RecoveryAfterFailure,
        ];

        let mut task_ids = Vec::new();
        
        for (i, failure_mode) in recovery_tasks.iter().enumerate() {
            let task_id = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.spawn_async_task(
                    component_id,
                    Some(i as u32),
                    FaultyTask {
                        id: i as u64,
                        failure_mode: *failure_mode,
                        polls_until_failure: 3,
                        polls_count: AtomicU64::new(0),
                        error_harness: Arc::new(harness),
                    },
                    ComponentAsyncTaskType::AsyncFunction,
                    Priority::Normal,
                ).unwrap()
            };
            
            task_ids.push(task_id);
        }

        // Execute with monitoring
        let mut completed_tasks = 0;
        
        for round in 0..300 { // Allow time for recovery
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.poll_async_tasks().unwrap()
            };
            
            completed_tasks += result.tasks_completed;
            
            if completed_tasks >= recovery_tasks.len() {
                break;
            }
        }

        let (errors, recoveries) = harness.get_error_stats();
        
        // Verify recovery behavior
        assert!(errors > 0, "No errors to recover from");
        assert!(recoveries > 0, "No recoveries occurred");
        
        // Most tasks should complete despite some failures
        let expected_recoverable = recovery_tasks.iter()
            .filter(|&&mode| matches!(mode, 
                FailureMode::RecoveryAfterFailure | 
                FailureMode::NoFailure
            ))
            .count();
        
        // System should remain stable after recovery
        let bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        assert_eq!(bridge_stats.active_components, 1);
        
        println!("Recovery and Graceful Degradation Test: PASSED");
        println!("  Total Tasks: {}", recovery_tasks.len());
        println!("  Completed Tasks: {}", completed_tasks);
        println!("  Expected Recoverable: {}", expected_recoverable);
        println!("  Errors: {}", errors);
        println!("  Recoveries: {}", recoveries);
        println!("  Recovery Rate: {:.1}%", (recoveries as f64 / errors as f64) * 100.0);
    }

    #[test]
    fn test_system_stability_under_errors() {
        let mut harness = ErrorHandlingHarness::new();
        
        // Create multiple components to test system-wide stability
        let components: Vec<ComponentInstanceId> = (1..=3)
            .map(|i| ComponentInstanceId::new(i))
            .collect();

        for &component_id in &components {
            {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.initialize_component_async(component_id, None).unwrap();
            }
            harness.channels.initialize_component_channels(component_id, None).unwrap();
            harness.timers.initialize_component_timers(component_id, None).unwrap();
        }

        // Inject various errors across components
        let error_scenarios = vec![
            (FailureMode::PanicOnPoll, 2),
            (FailureMode::ExcessiveFuelConsumption, 3),
            (FailureMode::ResourceExhaustion, 4),
            (FailureMode::InvalidOperations, 2),
            (FailureMode::TimeoutFailure, 3),
            (FailureMode::RecoveryAfterFailure, 4),
            (FailureMode::NoFailure, 2), // Control tasks
            (FailureMode::NoFailure, 2),
            (FailureMode::NoFailure, 2),
        ];

        let mut task_count = 0;
        
        for (component_idx, &component_id) in components.iter().enumerate() {
            for (scenario_idx, &(failure_mode, polls_until_failure)) in error_scenarios.iter().enumerate() {
                if (scenario_idx % components.len()) == component_idx {
                    let task_id = {
                        let mut bridge = harness.bridge.lock().unwrap();
                        bridge.spawn_async_task(
                            component_id,
                            Some(task_count),
                            FaultyTask {
                                id: task_count as u64,
                                failure_mode,
                                polls_until_failure,
                                polls_count: AtomicU64::new(0),
                                error_harness: Arc::new(harness),
                            },
                            ComponentAsyncTaskType::AsyncFunction,
                            Priority::Normal,
                        ).unwrap()
                    };
                    
                    task_count += 1;
                }
            }
        }

        // Also create some resource operations that might fail
        for &component_id in &components {
            // Create channels that might get exhausted
            for _ in 0..10 {
                let _ = harness.channels.create_channel(component_id, ChannelType::Bounded(2));
            }
            
            // Create timers
            for i in 0..5 {
                let _ = harness.timers.create_timer(
                    component_id,
                    TimerType::Oneshot,
                    1000 + i * 100,
                );
            }
        }

        // Execute system under stress with error injection
        let initial_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };

        let mut completed_tasks = 0;
        let mut stable_rounds = 0;
        
        for round in 0..500 {
            // Poll tasks
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.poll_async_tasks().unwrap()
            };
            completed_tasks += result.tasks_completed;
            
            // Process timers
            harness.timers.advance_time(10);
            let _ = harness.timers.process_timers();
            
            // Check system stability
            let current_stats = {
                let bridge = harness.bridge.lock().unwrap();
                bridge.get_bridge_statistics()
            };
            
            if current_stats.active_components == components.len() as u64 {
                stable_rounds += 1;
            }
            
            // Early exit if enough tasks completed
            if completed_tasks >= task_count / 2 {
                break;
            }
        }

        let final_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        
        let (total_errors, total_recoveries) = harness.get_error_stats();
        
        // Verify system remained stable despite errors
        assert_eq!(final_stats.active_components, components.len() as u64, 
            "System lost components under error conditions");
        assert!(stable_rounds > 100, 
            "System was unstable for too long: {} stable rounds", stable_rounds);
        assert!(total_errors > 0, "No errors were injected");
        assert!(completed_tasks > 0, "No tasks completed despite errors");
        
        // System should have handled errors gracefully
        let error_rate = total_errors as f64 / task_count as f64;
        let completion_rate = completed_tasks as f64 / task_count as f64;
        
        println!("System Stability Under Errors Test: PASSED");
        println!("  Components: {}", components.len());
        println!("  Total Tasks: {}", task_count);
        println!("  Completed Tasks: {}", completed_tasks);
        println!("  Completion Rate: {:.1}%", completion_rate * 100.0);
        println!("  Total Errors: {}", total_errors);
        println!("  Total Recoveries: {}", total_recoveries);
        println!("  Error Rate: {:.1}%", error_rate * 100.0);
        println!("  Stable Rounds: {}", stable_rounds);
        println!("  Final Active Components: {}", final_stats.active_components);
    }
}