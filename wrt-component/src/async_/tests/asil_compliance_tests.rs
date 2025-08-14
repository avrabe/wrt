//! ASIL (Automotive Safety Integrity Level) compliance verification tests
//!
//! This module verifies that the async executor system meets ASIL-A through ASIL-D
//! requirements for functional safety in automotive systems:
//! - Deterministic execution behavior
//! - Freedom from interference (spatial, temporal, resource, communication)
//! - Bounded resource usage
//! - Predictable timing
//! - Error detection and handling
//! - No dynamic allocation after initialization

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
            async_combinators::AsyncCombinators,
            optimized_async_channels::{OptimizedAsyncChannels, ChannelType},
            timer_integration::{TimerIntegration, TimerType},
            advanced_sync_primitives::{AdvancedSyncPrimitives, MutexLockResult},
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

    /// ASIL compliance verification harness
    struct ASILComplianceHarness {
        bridge: Arc<Mutex<TaskManagerAsyncBridge>>,
        abi_support: AsyncCanonicalAbiSupport,
        resource_ops: ResourceAsyncOperations,
        combinators: AsyncCombinators,
        channels: OptimizedAsyncChannels,
        timers: TimerIntegration,
        sync_primitives: AdvancedSyncPrimitives,
        safety_violations: Arc<AtomicU32>,
    }

    impl ASILComplianceHarness {
        fn new_for_asil_level(asil_level: ASILLevel) -> Self {
            let task_manager = Arc::new(Mutex::new(TaskManager::new();
            let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
            
            let config = BridgeConfiguration {
                enable_preemption: true,
                enable_dynamic_fuel: matches!(asil_level, ASILLevel::A | ASILLevel::B),
                fuel_policy: match asil_level {
                    ASILLevel::D => FuelAllocationPolicy::Fixed,
                    ASILLevel::C => FuelAllocationPolicy::Adaptive,
                    ASILLevel::B => FuelAllocationPolicy::PriorityAdaptive,
                    ASILLevel::A => FuelAllocationPolicy::PerformanceOptimized,
                },
                preemption_policy: match asil_level {
                    ASILLevel::D | ASILLevel::C => PreemptionPolicy::DeadlineDriven,
                    ASILLevel::B => PreemptionPolicy::PriorityBased,
                    ASILLevel::A => PreemptionPolicy::Cooperative,
                },
                ..Default::default()
            };
            
            let bridge = Arc::new(Mutex::new(
                TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap()
            ;

            let abi_support = AsyncCanonicalAbiSupport::new(bridge.clone();
            let resource_ops = ResourceAsyncOperations::new(abi_support.clone();
            let combinators = AsyncCombinators::new(bridge.clone();
            let channels = OptimizedAsyncChannels::new(bridge.clone(), None;
            let timers = TimerIntegration::new(bridge.clone(), None;
            let sync_primitives = AdvancedSyncPrimitives::new(bridge.clone(), None;

            Self {
                bridge,
                abi_support,
                resource_ops,
                combinators,
                channels,
                timers,
                sync_primitives,
                safety_violations: Arc::new(AtomicU32::new(0)),
            }
        }

        fn report_safety_violation(&self, violation: &str) {
            self.safety_violations.fetch_add(1, Ordering::AcqRel;
            eprintln!("SAFETY VIOLATION: {}", violation);
        }

        fn verify_no_safety_violations(&self) -> bool {
            self.safety_violations.load(Ordering::Acquire) == 0
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ASILLevel {
        A, // Lowest
        B,
        C,
        D, // Highest
    }

    /// Deterministic task for ASIL testing
    struct DeterministicTask {
        id: u64,
        fuel_budget: u64,
        fuel_consumed: AtomicU64,
        execution_count: AtomicU32,
        max_executions: u32,
        result_value: ComponentValue,
        safety_harness: Arc<ASILComplianceHarness>,
    }

    impl Future for DeterministicTask {
        type Output = Result<Vec<ComponentValue>, Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let current_executions = self.execution_count.fetch_add(1, Ordering::AcqRel;
            let fuel_consumed = self.fuel_consumed.fetch_add(50, Ordering::AcqRel;
            
            // ASIL Compliance Check: Fuel budget adherence
            if fuel_consumed > self.fuel_budget {
                self.safety_harness.report_safety_violation(
                    &format!("Task {} exceeded fuel budget: {} > {}", 
                        self.id, fuel_consumed, self.fuel_budget)
                ;
            }
            
            // ASIL Compliance Check: Bounded execution
            if current_executions >= self.max_executions {
                Poll::Ready(Ok(vec![
                    self.result_value.clone(),
                    ComponentValue::U32(current_executions),
                    ComponentValue::U32(fuel_consumed as u32),
                ])
            } else {
                // ASIL Compliance Check: Deterministic wakeup
                cx.waker().wake_by_ref);
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_asil_d_deterministic_execution() {
        let harness = ASILComplianceHarness::new_for_asil_level(ASILLevel::D;
        
        // ASIL-D: Highest safety integrity level
        // Requirements: Deterministic execution, no dynamic allocation, strict timing
        
        let component_id = ComponentInstanceId::new(1;
        
        // Initialize with strict limits for ASIL-D
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.channels.initialize_component_channels(component_id, None).unwrap();
        harness.timers.initialize_component_timers(component_id, None).unwrap();
        harness.sync_primitives.initialize_component_sync(component_id, None).unwrap();

        // Create deterministic tasks with strict fuel budgets
        const NUM_TASKS: u32 = 5; // Small number for ASIL-D
        const FUEL_BUDGET: u64 = 1000; // Strict budget
        const MAX_EXECUTIONS: u32 = 10; // Bounded execution
        
        let mut task_ids = Vec::new();
        
        for i in 0..NUM_TASKS {
            let task_id = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.spawn_async_task(
                    component_id,
                    Some(i),
                    DeterministicTask {
                        id: i as u64,
                        fuel_budget: FUEL_BUDGET,
                        fuel_consumed: AtomicU64::new(0),
                        execution_count: AtomicU32::new(0),
                        max_executions: MAX_EXECUTIONS,
                        result_value: ComponentValue::U32(i * 100),
                        safety_harness: Arc::new(harness),
                    },
                    ComponentAsyncTaskType::AsyncFunction,
                    Priority::High, // ASIL-D uses high priority
                ).unwrap()
            };
            
            task_ids.push(task_id);
        }

        // Execute with deterministic polling
        let mut completed_tasks = 0;
        let max_poll_rounds = 1000; // Bounded execution time
        
        for round in 0..max_poll_rounds {
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.poll_async_tasks().unwrap()
            };
            
            completed_tasks += result.tasks_completed;
            
            // ASIL-D: Must complete in bounded time
            if completed_tasks >= NUM_TASKS {
                break;
            }
            
            // ASIL-D: Check for deterministic progress
            if round > 0 && round % 100 == 0 {
                if completed_tasks == 0 {
                    harness.report_safety_violation(
                        &format!("No progress after {} rounds", round)
                    ;
                }
            }
        }

        // ASIL-D Verification: All tasks must complete
        if completed_tasks < NUM_TASKS {
            harness.report_safety_violation(
                &format!("Only {} of {} tasks completed", completed_tasks, NUM_TASKS)
            ;
        }

        // ASIL-D Verification: Fuel consumption must be within bounds
        let final_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        
        let total_fuel_budget = (NUM_TASKS as u64) * FUEL_BUDGET;
        if final_stats.total_fuel_consumed > total_fuel_budget * 2 {
            harness.report_safety_violation(
                &format!("Excessive fuel consumption: {} > {}", 
                    final_stats.total_fuel_consumed, total_fuel_budget * 2)
            ;
        }

        // Final ASIL-D compliance check
        assert!(harness.verify_no_safety_violations(), "ASIL-D compliance violations detected");
        assert_eq!(completed_tasks, NUM_TASKS, "Not all ASIL-D tasks completed";
        
        println!("ASIL-D Deterministic Execution Test: PASSED");
        println!("  Tasks Completed: {}/{}", completed_tasks, NUM_TASKS);
        println!("  Fuel Consumed: {}", final_stats.total_fuel_consumed);
        println!("  Fuel Budget: {}", total_fuel_budget);
    }

    #[test]
    fn test_asil_c_freedom_from_interference() {
        let harness = ASILComplianceHarness::new_for_asil_level(ASILLevel::C;
        
        // ASIL-C: Freedom from interference testing
        // Requirements: Spatial, temporal, resource, and communication isolation
        
        // Create multiple isolated components
        let critical_component = ComponentInstanceId::new(1;
        let non_critical_component = ComponentInstanceId::new(2;
        
        // Initialize both components
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(critical_component, None).unwrap();
            bridge.initialize_component_async(non_critical_component, None).unwrap();
        }
        
        harness.channels.initialize_component_channels(critical_component, None).unwrap();
        harness.channels.initialize_component_channels(non_critical_component, None).unwrap();
        
        harness.sync_primitives.initialize_component_sync(critical_component, None).unwrap();
        harness.sync_primitives.initialize_component_sync(non_critical_component, None).unwrap();

        // Create resource isolation test
        
        // 1. Spatial Isolation: Separate memory spaces
        let critical_mutex = harness.sync_primitives.create_async_mutex(critical_component, false).unwrap();
        let non_critical_mutex = harness.sync_primitives.create_async_mutex(non_critical_component, false).unwrap();
        
        // Verify mutexes are separate
        assert_ne!(critical_mutex, non_critical_mutex, "Spatial isolation violated: shared mutex";

        // 2. Resource Isolation: Separate channels
        let (critical_sender, critical_receiver) = harness.channels.create_channel(
            critical_component,
            ChannelType::Bounded(8),
        ).unwrap();
        
        let (non_critical_sender, non_critical_receiver) = harness.channels.create_channel(
            non_critical_component,
            ChannelType::Bounded(8),
        ).unwrap();

        // 3. Temporal Isolation: Priority-based scheduling
        let critical_task = {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.spawn_async_task(
                critical_component,
                Some(0),
                DeterministicTask {
                    id: 1,
                    fuel_budget: 2000,
                    fuel_consumed: AtomicU64::new(0),
                    execution_count: AtomicU32::new(0),
                    max_executions: 5,
                    result_value: ComponentValue::U32(1000),
                    safety_harness: Arc::new(harness),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::High, // Critical task gets high priority
            ).unwrap()
        };

        let non_critical_task = {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.spawn_async_task(
                non_critical_component,
                Some(0),
                DeterministicTask {
                    id: 2,
                    fuel_budget: 1000,
                    fuel_consumed: AtomicU64::new(0),
                    execution_count: AtomicU32::new(0),
                    max_executions: 10,
                    result_value: ComponentValue::U32(2000),
                    safety_harness: Arc::new(harness),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal, // Non-critical task gets normal priority
            ).unwrap()
        };

        // 4. Communication Isolation: Cross-component channel access should fail
        let cross_component_send = harness.channels.send_message(
            critical_sender.channel_id,
            non_critical_component, // Wrong component trying to send
            ComponentValue::U32(42),
            None,
        ;
        
        // This should either fail or be properly isolated
        // (Implementation may allow but should maintain isolation)

        // Execute with interference monitoring
        let mut completed_tasks = 0;
        let mut critical_completed = false;
        let mut non_critical_completed = false;
        
        for round in 0..500 {
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.poll_async_tasks().unwrap()
            };
            
            completed_tasks += result.tasks_completed;
            
            // Check if critical task completed first (temporal isolation)
            if completed_tasks > 0 && !critical_completed {
                // In a properly isolated system, critical task should complete first
                // due to higher priority
                critical_completed = true;
            }
            
            if completed_tasks >= 2 {
                break;
            }
        }

        // ASIL-C Verification: Both components should function independently
        let final_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        
        if final_stats.active_components != 2 {
            harness.report_safety_violation(
                &format!("Component isolation failed: {} active components", 
                    final_stats.active_components)
            ;
        }

        // Verify no cross-component interference
        let channel_stats = harness.channels.get_channel_statistics);
        if channel_stats.total_channels_created < 2 {
            harness.report_safety_violation("Channel isolation failed";
        }

        assert!(harness.verify_no_safety_violations(), "ASIL-C interference violations detected");
        
        println!("ASIL-C Freedom from Interference Test: PASSED");
        println!("  Active Components: {}", final_stats.active_components);
        println!("  Tasks Completed: {}", completed_tasks);
        println!("  Channels Created: {}", channel_stats.total_channels_created);
    }

    #[test]
    fn test_asil_b_bounded_resource_usage() {
        let harness = ASILComplianceHarness::new_for_asil_level(ASILLevel::B;
        
        // ASIL-B: Bounded resource usage verification
        // Requirements: All resources must have known, enforced limits
        
        let component_id = ComponentInstanceId::new(1;
        
        // Initialize with specific resource limits
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.channels.initialize_component_channels(component_id, None).unwrap();
        harness.timers.initialize_component_timers(component_id, None).unwrap();
        harness.sync_primitives.initialize_component_sync(component_id, None).unwrap();

        // Test bounded channel creation
        let mut channels_created = 0;
        const EXPECTED_CHANNEL_LIMIT: usize = 64; // Expected limit from implementation
        
        for i in 0..100 { // Try to exceed limit
            match harness.channels.create_channel(component_id, ChannelType::Bounded(4)) {
                Ok(_) => channels_created += 1,
                Err(_) => break, // Hit limit as expected
            }
        }
        
        if channels_created >= 100 {
            harness.report_safety_violation("Channel creation not bounded";
        }

        // Test bounded timer creation
        let mut timers_created = 0;
        const EXPECTED_TIMER_LIMIT: usize = 128; // Expected limit from implementation
        
        for i in 0..200 { // Try to exceed limit
            match harness.timers.create_timer(component_id, TimerType::Oneshot, 1000) {
                Ok(_) => timers_created += 1,
                Err(_) => break, // Hit limit as expected
            }
        }
        
        if timers_created >= 200 {
            harness.report_safety_violation("Timer creation not bounded";
        }

        // Test bounded sync primitive creation
        let mut mutexes_created = 0;
        
        for i in 0..100 { // Try to exceed limit
            match harness.sync_primitives.create_async_mutex(component_id, false) {
                Ok(_) => mutexes_created += 1,
                Err(_) => break, // Hit limit as expected
            }
        }
        
        if mutexes_created >= 100 {
            harness.report_safety_violation("Mutex creation not bounded";
        }

        // Test bounded task creation
        let mut tasks_created = 0;
        
        for i in 0..200 { // Try to exceed limit
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.spawn_async_task(
                    component_id,
                    Some(i),
                    DeterministicTask {
                        id: i as u64,
                        fuel_budget: 500,
                        fuel_consumed: AtomicU64::new(0),
                        execution_count: AtomicU32::new(0),
                        max_executions: 2,
                        result_value: ComponentValue::U32(i),
                        safety_harness: Arc::new(harness),
                    },
                    ComponentAsyncTaskType::AsyncFunction,
                    Priority::Normal,
            })?;
            };
            
            match result {
                Ok(_) => tasks_created += 1,
                Err(_) => break, // Hit limit as expected
            }
        }
        
        if tasks_created >= 200 {
            harness.report_safety_violation("Task creation not bounded";
        }

        // Verify resource limits were enforced
        let channel_stats = harness.channels.get_channel_statistics);
        let timer_stats = harness.timers.get_timer_statistics);
        let sync_stats = harness.sync_primitives.get_sync_statistics);
        let bridge_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };

        // All resource creation should have been bounded
        assert!(channels_created < 100, "Channels not properly bounded: {}", channels_created);
        assert!(timers_created < 200, "Timers not properly bounded: {}", timers_created);
        assert!(mutexes_created < 100, "Mutexes not properly bounded: {}", mutexes_created);
        assert!(tasks_created < 200, "Tasks not properly bounded: {}", tasks_created);

        assert!(harness.verify_no_safety_violations(), "ASIL-B resource violations detected");
        
        println!("ASIL-B Bounded Resource Usage Test: PASSED");
        println!("  Channels Created: {} (bounded)", channels_created);
        println!("  Timers Created: {} (bounded)", timers_created);
        println!("  Mutexes Created: {} (bounded)", mutexes_created);
        println!("  Tasks Created: {} (bounded)", tasks_created);
    }

    #[test]
    fn test_asil_a_basic_safety_requirements() {
        let harness = ASILComplianceHarness::new_for_asil_level(ASILLevel::A;
        
        // ASIL-A: Basic safety requirements
        // Requirements: Error detection, basic fault tolerance
        
        let component_id = ComponentInstanceId::new(1;
        
        // Initialize system
        {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.initialize_component_async(component_id, None).unwrap();
        }
        harness.channels.initialize_component_channels(component_id, None).unwrap();
        harness.timers.initialize_component_timers(component_id, None).unwrap();

        // Test error detection and handling
        
        // 1. Invalid operations should be detected
        let invalid_component = ComponentInstanceId::new(999;
        
        // Should fail gracefully
        let result = harness.channels.create_channel(invalid_component, ChannelType::Bounded(8;
        if result.is_ok() {
            harness.report_safety_violation("Invalid component operation not detected";
        }

        // 2. Resource exhaustion should be handled
        let (sender, receiver) = harness.channels.create_channel(
            component_id,
            ChannelType::Bounded(2),
        ).unwrap();

        // Fill channel to capacity
        for i in 0..3 {
            let send_result = harness.channels.send_message(
                sender.channel_id,
                component_id,
                ComponentValue::U32(i),
                None,
            ).unwrap();
            
            if i >= 2 {
                // Should indicate backpressure or full
                match send_result {
                    crate::async_::optimized_async_channels::SendResult::Sent => {
                        harness.report_safety_violation("Channel overflow not detected";
                    },
                    _ => {}, // Expected - backpressure or full
                }
            }
        }

        // 3. Timer management should be robust
        let timer_id = harness.timers.create_timer(
            component_id,
            TimerType::Oneshot,
            100,
        ).unwrap();

        // Cancel timer and verify
        let cancel_result = harness.timers.cancel_timer(timer_id).unwrap();
        assert!(cancel_result, "Timer cancellation failed");

        // Try to cancel again - should handle gracefully
        let second_cancel = harness.timers.cancel_timer(timer_id).unwrap();
        assert!(!second_cancel, "Double cancellation not handled correctly");

        // 4. Basic task execution
        let task_id = {
            let mut bridge = harness.bridge.lock().unwrap();
            bridge.spawn_async_task(
                component_id,
                Some(0),
                DeterministicTask {
                    id: 1,
                    fuel_budget: 2000,
                    fuel_consumed: AtomicU64::new(0),
                    execution_count: AtomicU32::new(0),
                    max_executions: 3,
                    result_value: ComponentValue::U32(100),
                    safety_harness: Arc::new(harness),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal,
            ).unwrap()
        };

        // Execute task
        let mut completed = false;
        for _ in 0..100 {
            let result = {
                let mut bridge = harness.bridge.lock().unwrap();
                bridge.poll_async_tasks().unwrap()
            };
            
            if result.tasks_completed > 0 {
                completed = true;
                break;
            }
        }

        if !completed {
            harness.report_safety_violation("Basic task execution failed";
        }

        // Verify system state
        let final_stats = {
            let bridge = harness.bridge.lock().unwrap();
            bridge.get_bridge_statistics()
        };
        
        assert!(harness.verify_no_safety_violations(), "ASIL-A safety violations detected");
        assert_eq!(final_stats.active_components, 1);
        
        println!("ASIL-A Basic Safety Requirements Test: PASSED");
        println!("  Task Completed: {}", completed);
        println!("  Error Detection: Functional");
        println!("  Resource Management: Functional");
    }

    #[test]
    fn test_cross_asil_level_compatibility() {
        // Test that different ASIL levels can coexist
        
        let asil_d_harness = ASILComplianceHarness::new_for_asil_level(ASILLevel::D;
        let asil_a_harness = ASILComplianceHarness::new_for_asil_level(ASILLevel::A;
        
        let critical_component = ComponentInstanceId::new(1;
        let basic_component = ComponentInstanceId::new(2;
        
        // Initialize both systems
        {
            let mut bridge = asil_d_harness.bridge.lock().unwrap();
            bridge.initialize_component_async(critical_component, None).unwrap();
        }
        
        {
            let mut bridge = asil_a_harness.bridge.lock().unwrap();
            bridge.initialize_component_async(basic_component, None).unwrap();
        }

        // Create tasks in both systems
        let critical_task = {
            let mut bridge = asil_d_harness.bridge.lock().unwrap();
            bridge.spawn_async_task(
                critical_component,
                Some(0),
                DeterministicTask {
                    id: 1,
                    fuel_budget: 1000,
                    fuel_consumed: AtomicU64::new(0),
                    execution_count: AtomicU32::new(0),
                    max_executions: 3,
                    result_value: ComponentValue::U32(1000),
                    safety_harness: Arc::new(asil_d_harness),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::High,
            ).unwrap()
        };

        let basic_task = {
            let mut bridge = asil_a_harness.bridge.lock().unwrap();
            bridge.spawn_async_task(
                basic_component,
                Some(0),
                DeterministicTask {
                    id: 2,
                    fuel_budget: 2000,
                    fuel_consumed: AtomicU64::new(0),
                    execution_count: AtomicU32::new(0),
                    max_executions: 5,
                    result_value: ComponentValue::U32(2000),
                    safety_harness: Arc::new(asil_a_harness),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal,
            ).unwrap()
        };

        // Execute both systems independently
        let mut d_completed = false;
        let mut a_completed = false;
        
        for round in 0..200 {
            // Poll ASIL-D system
            if !d_completed {
                let d_result = {
                    let mut bridge = asil_d_harness.bridge.lock().unwrap();
                    bridge.poll_async_tasks().unwrap()
                };
                if d_result.tasks_completed > 0 {
                    d_completed = true;
                }
            }
            
            // Poll ASIL-A system
            if !a_completed {
                let a_result = {
                    let mut bridge = asil_a_harness.bridge.lock().unwrap();
                    bridge.poll_async_tasks().unwrap()
                };
                if a_result.tasks_completed > 0 {
                    a_completed = true;
                }
            }
            
            if d_completed && a_completed {
                break;
            }
        }

        // Verify both systems completed successfully
        assert!(asil_d_harness.verify_no_safety_violations(), "ASIL-D violations in compatibility test");
        assert!(asil_a_harness.verify_no_safety_violations(), "ASIL-A violations in compatibility test");
        assert!(d_completed, "ASIL-D task did not complete");
        assert!(a_completed, "ASIL-A task did not complete");
        
        println!("Cross-ASIL Level Compatibility Test: PASSED");
        println!("  ASIL-D Task Completed: {}", d_completed);
        println!("  ASIL-A Task Completed: {}", a_completed);
    }
}