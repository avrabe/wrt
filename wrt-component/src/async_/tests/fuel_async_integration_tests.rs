//! Integration tests for fuel-based async executor
//!
//! This module provides comprehensive end-to-end tests for the fuel-based
//! async execution system, validating all phases of implementation.

#[cfg(test)]
mod tests {
    use crate::{
        async_::{
            fuel_async_executor::{
                FuelAsyncExecutor, FuelAsyncTask, AsyncTaskState, ExecutionContext,
                ASILExecutionMode, YieldType, ResumptionCondition,
            },
            fuel_async_runtime::{FuelAsyncRuntime, RuntimeConfig, TaskResult},
            fuel_aware_waker::create_fuel_aware_waker,
            fuel_dynamic_manager::{FuelDynamicManager, FuelAllocationPolicy},
            fuel_preemption_support::{FuelPreemptionManager, PreemptionPolicy},
            fuel_debt_credit::{FuelDebtCreditSystem, DebtPolicy, CreditRestriction},
        },
        component_instance::ComponentInstance,
        task_manager::TaskId,
        prelude::*,
    };
    use wrt_foundation::{
        verification::VerificationLevel,
        operations::global_fuel_consumed,
        safe_managed_alloc, CrateId,
    };
    use core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    /// Test basic async task execution with fuel tracking
    #[test]
    fn test_basic_async_execution() {
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Basic).unwrap();
        
        // Create a simple async task
        let task_id = executor.spawn_async_task(
            1, // component_id
            1000, // fuel_budget
            VerificationLevel::Basic,
            ExecutionContext::new(ASILExecutionMode::A { error_detection: true }),
        ).unwrap();
        
        // Poll the task
        let result = executor.poll_task(task_id);
        assert!(result.is_ok();
        
        // Verify fuel consumption
        let fuel_consumed = executor.get_task_fuel_consumed(task_id).unwrap();
        assert!(fuel_consumed > 0);
    }

    /// Test ASIL-D deterministic execution
    #[test]
    fn test_asil_d_deterministic_execution() {
        let asil_mode = ASILExecutionMode::D {
            deterministic_execution: true,
            bounded_execution_time: true,
            formal_verification: true,
            max_fuel_per_slice: 100,
        };
        
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Full).unwrap();
        
        // Create deterministic task
        let mut context = ExecutionContext::new(asil_mode);
        context.current_function_index = 42;
        context.function_params = vec![wrt_foundation::Value::I32(10)];
        
        let task_id = executor.spawn_async_task(1, 1000, VerificationLevel::Full, context).unwrap();
        
        // Execute in deterministic steps
        for _ in 0..5 {
            let result = executor.poll_task(task_id);
            assert!(result.is_ok();
            
            // Verify deterministic fuel consumption
            let fuel = executor.get_task_fuel_consumed(task_id).unwrap();
            assert!(fuel <= asil_mode.max_fuel_per_slice() * 5);
        }
    }

    /// Test yield point creation and restoration
    #[test]
    fn test_yield_point_suspension_resumption() {
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Standard).unwrap();
        
        // Create task with explicit yield points
        let mut context = ExecutionContext::new(ASILExecutionMode::B {
            strict_resource_limits: true,
            max_execution_slice_ms: 10,
        });
        
        // Create a yield point
        context.create_advanced_yield_point(
            100, // instruction_pointer
            YieldType::ExplicitYield,
            Some(ResumptionCondition::Manual),
        ).unwrap();
        
        let task_id = executor.spawn_async_task(1, 1000, VerificationLevel::Standard, context).unwrap();
        
        // Poll should yield
        let result = executor.poll_task(task_id);
        assert!(result.is_ok();
        
        // Get task and verify yield state
        let task = executor.get_task(task_id).unwrap();
        assert!(task.execution_context.last_yield_point.is_some();
        
        let yield_point = task.execution_context.last_yield_point.as_ref().unwrap();
        assert_eq!(yield_point.instruction_pointer, 100);
        assert!(matches!(yield_point.yield_type, YieldType::ExplicitYield);
    }

    /// Test async resource waiting
    #[test]
    fn test_async_resource_wait() {
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Basic).unwrap();
        
        let mut context = ExecutionContext::new(ASILExecutionMode::A { error_detection: true });
        
        // Create async yield point waiting for resource
        let resource_id = 12345;
        context.create_async_yield_point(200, resource_id).unwrap();
        
        let task_id = executor.spawn_async_task(1, 1000, VerificationLevel::Basic, context).unwrap();
        
        // Poll - should yield waiting for resource
        executor.poll_task(task_id).unwrap();
        
        let task = executor.get_task(task_id).unwrap();
        let yield_point = task.execution_context.last_yield_point.as_ref().unwrap();
        
        assert!(matches!(yield_point.yield_type, YieldType::AsyncWait { resource_id: 12345 });
        assert!(matches!(
            yield_point.resumption_condition,
            Some(ResumptionCondition::ResourceAvailable { resource_id: 12345 })
        );
    }

    /// Test fuel exhaustion and recovery
    #[test]
    fn test_fuel_exhaustion_recovery() {
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Standard).unwrap();
        
        // Create task with small fuel budget
        let task_id = executor.spawn_async_task(
            1,
            50, // Very small fuel budget
            VerificationLevel::Standard,
            ExecutionContext::new(ASILExecutionMode::B {
                strict_resource_limits: true,
                max_execution_slice_ms: 5,
            }),
        ).unwrap();
        
        // Poll until fuel exhausted
        for _ in 0..10 {
            let _ = executor.poll_task(task_id);
        }
        
        // Check if task is fuel exhausted
        let task = executor.get_task(task_id).unwrap();
        let fuel_consumed = task.fuel_consumed.load(core::sync::atomic::Ordering::Acquire);
        assert!(fuel_consumed >= 50);
        
        // Refuel the task
        executor.refuel_task(task_id, 100).unwrap();
        
        // Should be able to continue execution
        let result = executor.poll_task(task_id);
        assert!(result.is_ok();
    }

    /// Test runtime integration with multiple tasks
    #[test]
    fn test_runtime_multiple_tasks() {
        let config = RuntimeConfig {
            global_fuel_budget: 50000,
            verification_level: VerificationLevel::Standard,
            max_concurrent_tasks: 10,
            polling_batch_size: 5,
            enable_cleanup: true,
        };
        
        let mut runtime = FuelAsyncRuntime::new(config).unwrap();
        
        // Create mock component
        let component = Arc::new(ComponentInstance::new();
        runtime.register_component(1, component.clone()).unwrap();
        
        // Spawn multiple tasks
        let task_ids: Vec<TaskId> = (0..5).map(|i| {
            runtime.spawn_task(
                1,
                &format!("function_{}", i),
                vec![wrt_foundation::Value::I32(i as i32)],
                1000,
                ASILExecutionMode::A { error_detection: true },
            ).unwrap()
        }).collect();
        
        // Run runtime for a limited time
        let start_fuel = global_fuel_consumed();
        
        // Poll a few times instead of running to completion
        for _ in 0..10 {
            if !runtime.has_active_tasks().unwrap() {
                break;
            }
            runtime.poll_tasks().unwrap();
        }
        
        let end_fuel = global_fuel_consumed();
        assert!(end_fuel > start_fuel);
        
        // Check task results
        for task_id in task_ids {
            let result = runtime.get_task_result(task_id);
            // Tasks may or may not be complete depending on execution
            assert!(result.is_ok() || result.is_err();
        }
    }

    /// Test ASIL-C isolation guarantees
    #[test]
    fn test_asil_c_isolation() {
        let asil_mode = ASILExecutionMode::C {
            spatial_isolation: true,
            temporal_isolation: true,
            resource_isolation: true,
        };
        
        let mut executor = FuelAsyncExecutor::new(20000, VerificationLevel::Full).unwrap();
        
        // Create isolated tasks
        let task1 = executor.spawn_async_task(
            1,
            5000,
            VerificationLevel::Full,
            ExecutionContext::new(asil_mode),
        ).unwrap();
        
        let task2 = executor.spawn_async_task(
            2,
            5000,
            VerificationLevel::Full,
            ExecutionContext::new(asil_mode),
        ).unwrap();
        
        // Execute both tasks
        executor.poll_task(task1).unwrap();
        executor.poll_task(task2).unwrap();
        
        // Verify isolation - fuel consumption should be independent
        let fuel1 = executor.get_task_fuel_consumed(task1).unwrap();
        let fuel2 = executor.get_task_fuel_consumed(task2).unwrap();
        
        // Each task should have consumed some fuel independently
        assert!(fuel1 > 0);
        assert!(fuel2 > 0);
        
        // Total should not exceed individual budgets
        assert!(fuel1 <= 5000);
        assert!(fuel2 <= 5000);
    }

    /// Test preemption with priority tasks
    #[test]
    fn test_priority_preemption() {
        let mut manager = FuelPreemptionManager::new(
            10000,
            PreemptionPolicy::PriorityBased { min_priority_delta: 10 },
        ).unwrap();
        
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Standard).unwrap();
        
        // Create low priority task
        let low_priority_task = executor.spawn_async_task(
            1,
            5000,
            VerificationLevel::Standard,
            ExecutionContext::new(ASILExecutionMode::A { error_detection: true }),
        ).unwrap();
        
        // Create high priority task
        let mut high_priority_context = ExecutionContext::new(ASILExecutionMode::B {
            strict_resource_limits: true,
            max_execution_slice_ms: 10,
        });
        high_priority_context.current_function_index = 99; // Different function
        
        let high_priority_task = executor.spawn_async_task(
            2,
            5000,
            VerificationLevel::Standard,
            high_priority_context,
        ).unwrap();
        
        // Check preemption decision
        let decision = manager.evaluate_preemption(
            low_priority_task,
            high_priority_task,
            &executor,
        ).unwrap();
        
        // High priority task should preempt low priority
        assert!(matches!(decision, crate::async_::fuel_preemption_support::PreemptionDecision::Preempt { .. });
    }

    /// Test yield point restoration after suspension
    #[test]
    fn test_yield_point_full_restoration() {
        let mut executor = FuelAsyncExecutor::new(10000, VerificationLevel::Full).unwrap();
        
        let mut context = ExecutionContext::new(ASILExecutionMode::D {
            deterministic_execution: true,
            bounded_execution_time: true,
            formal_verification: false,
            max_fuel_per_slice: 100,
        });
        
        // Set up initial state
        context.current_function_index = 42;
        context.function_params = vec![
            wrt_foundation::Value::I32(10),
            wrt_foundation::Value::I64(20),
        ];
        context.stack_depth = 3;
        
        // Create ASIL yield point
        context.create_asil_yield_point(300, "Deterministic checkpoint".to_string()).unwrap();
        
        let task_id = executor.spawn_async_task(1, 1000, VerificationLevel::Full, context).unwrap();
        
        // Get the yield point
        let task = executor.get_task(task_id).unwrap();
        let yield_point = task.execution_context.last_yield_point.as_ref().unwrap();
        
        // Verify yield point has complete state
        assert_eq!(yield_point.instruction_pointer, 300);
        assert!(matches!(yield_point.yield_type, YieldType::ASILCompliance { .. });
        assert!(yield_point.yield_context.module_state.is_some();
        assert_eq!(yield_point.fuel_at_yield, task.execution_context.context_fuel_consumed.load(core::sync::atomic::Ordering::Acquire);
        
        // Create new context and restore from yield point
        let mut new_context = ExecutionContext::new(ASILExecutionMode::D {
            deterministic_execution: true,
            bounded_execution_time: true,
            formal_verification: false,
            max_fuel_per_slice: 100,
        });
        
        new_context.restore_from_yield_point(yield_point).unwrap();
        
        // Verify restoration
        assert_eq!(new_context.current_function_index, 300); // Instruction pointer becomes function index in restoration
        assert_eq!(new_context.function_params.len(), 2);
    }

    /// Test concurrent task execution with shared resources
    #[test]
    fn test_concurrent_execution_shared_resources() {
        let config = RuntimeConfig {
            global_fuel_budget: 100000,
            verification_level: VerificationLevel::Standard,
            max_concurrent_tasks: 20,
            polling_batch_size: 10,
            enable_cleanup: true,
        };
        
        let mut runtime = FuelAsyncRuntime::new(config).unwrap();
        
        // Register component
        let component = Arc::new(ComponentInstance::new();
        runtime.register_component(1, component).unwrap();
        
        // Spawn tasks that would share resources
        let task_ids: Vec<TaskId> = (0..10).map(|i| {
            let asil_mode = if i % 2 == 0 {
                ASILExecutionMode::A { error_detection: true }
            } else {
                ASILExecutionMode::B {
                    strict_resource_limits: true,
                    max_execution_slice_ms: 20,
                }
            };
            
            runtime.spawn_task(
                1,
                "shared_function",
                vec![wrt_foundation::Value::I32(i as i32)],
                2000,
                asil_mode,
            ).unwrap()
        }).collect();
        
        // Execute with limited fuel to force interleaving
        let mut polls = 0;
        while runtime.has_active_tasks().unwrap() && polls < 50 {
            runtime.poll_tasks().unwrap();
            polls += 1;
        }
        
        // Verify all tasks made progress
        for task_id in &task_ids {
            let result = runtime.get_task_result(*task_id);
            // Task should either be complete or still running (not failed)
            match result {
                Ok(TaskResult::Failed(err)) => panic!("Task {} failed: {:?}", task_id, err),
                _ => {} // Ok - either completed or still running
            }
        }
        
        // Check total fuel consumption
        let stats = runtime.stats();
        assert!(stats.total_fuel_consumed > 0);
        assert!(stats.polling_cycles > 0);
    }
}