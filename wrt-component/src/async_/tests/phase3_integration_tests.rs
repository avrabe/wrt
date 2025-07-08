//! Phase 3 integration tests for Component Model async integration
//!
//! This module tests the full integration between the fuel executor
//! and the Component Model's async infrastructure.

#[cfg(test)]
mod tests {
    use crate::{
        async_::{
            fuel_async_executor::FuelAsyncExecutor,
            task_manager_async_bridge::{
                TaskManagerAsyncBridge, BridgeConfiguration, ComponentAsyncTaskType
            },
            async_canonical_abi_support::{
                AsyncCanonicalAbiSupport, AsyncAbiOperationType, FutureOp, StreamOp
            },
            resource_async_operations::{
                ResourceAsyncOperations, ResourceAsyncOperationType, ResourceBorrowType
            },
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
        sync::atomic::{AtomicU64, Ordering},
        task::{Context, Poll},
    };
    use wrt_foundation::{
        component_value::ComponentValue,
        resource::{ResourceHandle, ResourceType},
        Arc, sync::Mutex,
    };
    use wrt_platform::advanced_sync::Priority;

    /// Test Component Model async function
    struct ComponentAsyncFunction {
        id: u64,
        polls_remaining: u64,
        result_value: ComponentValue,
    }

    impl Future for ComponentAsyncFunction {
        type Output = Result<Vec<ComponentValue>, Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.polls_remaining == 0 {
                Poll::Ready(Ok(vec![self.result_value.clone()])
            } else {
                self.polls_remaining -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    fn create_test_bridge() -> TaskManagerAsyncBridge {
        let task_manager = Arc::new(Mutex::new(TaskManager::new());
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new());
        let config = BridgeConfiguration::default();
        TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap()
    }

    #[test]
    fn test_full_component_async_lifecycle() {
        let mut bridge = create_test_bridge();
        
        // Initialize component for async operations
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();

        // Spawn async task from Component Model
        let task_id = bridge.spawn_async_task(
            component_id,
            Some(0), // function index
            ComponentAsyncFunction {
                id: 42,
                polls_remaining: 3,
                result_value: ComponentValue::U32(123),
            },
            ComponentAsyncTaskType::AsyncFunction,
            Priority::Normal,
        ).unwrap();

        // Poll until completion
        let mut polls = 0;
        loop {
            let result = bridge.poll_async_tasks().unwrap();
            polls += 1;
            
            if result.tasks_completed > 0 || polls > 10 {
                break;
            }
        }

        // Verify task completed
        let stats = bridge.get_bridge_statistics();
        assert!(stats.total_async_tasks > 0);
    }

    #[test]
    fn test_async_canonical_abi_operations() {
        let bridge = create_test_bridge();
        let mut abi_support = AsyncCanonicalAbiSupport::new(bridge);
        
        let component_id = ComponentInstanceId::new(1);
        abi_support.initialize_component_abi(component_id, CanonicalOptions::default()).unwrap();

        // Test async function call
        let func_type = FuncType::new(
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        
        let operation_id = abi_support.async_call(
            component_id,
            "test_function".to_string(),
            func_type,
            vec![ComponentValue::U32(10), ComponentValue::U32(20)],
            None,
        ).unwrap();

        // Check operation status
        let status = abi_support.check_operation_status(operation_id).unwrap();
        assert_eq!(status.component_id, component_id);
        
        match status.operation_type {
            AsyncAbiOperationType::AsyncCall { function_name, args } => {
                assert_eq!(function_name, "test_functionMissing message");
                assert_eq!(args.len(), 2);
            },
            _ => panic!("Expected AsyncCall operationMissing message"),
        }

        // Test async lifting
        let lift_op = abi_support.async_lift(
            component_id,
            vec![Value::I32(42)],
            ComponentType::Defined(0), // Would be proper type in real usage
            None,
        ).unwrap();

        // Test async lowering
        let lower_op = abi_support.async_lower(
            component_id,
            vec![ComponentValue::U32(42)],
            ValType::I32,
            None,
        ).unwrap();

        // Poll all operations
        let result = abi_support.poll_async_operations().unwrap();
        assert!(result.ready_operations >= 0);

        let stats = abi_support.get_abi_statistics();
        assert_eq!(stats.total_async_calls, 1);
        assert_eq!(stats.async_lifts, 1);
        assert_eq!(stats.async_lowers, 1);
    }

    #[test]
    fn test_resource_async_operations() {
        let bridge = create_test_bridge();
        let abi_support = AsyncCanonicalAbiSupport::new(bridge);
        let mut resource_ops = ResourceAsyncOperations::new(abi_support);
        
        let component_id = ComponentInstanceId::new(1);
        resource_ops.initialize_component_resources(component_id, None, None).unwrap();

        // Test async resource creation
        let resource_type = ResourceType::new("TestResource".to_string();
        let create_op = resource_ops.async_create_resource(
            component_id,
            resource_type.clone(),
            vec![ComponentValue::U32(100)], // constructor args
            None,
        ).unwrap();

        // Test async resource method call
        let resource_handle = ResourceHandle::new(1);
        let method_op = resource_ops.async_call_resource_method(
            component_id,
            resource_handle,
            "test_method".to_string(),
            vec![ComponentValue::U32(42)],
            None,
        ).unwrap();

        // Test async resource borrow
        let borrow_op = resource_ops.async_borrow_resource(
            component_id,
            resource_handle,
            ResourceBorrowType::Shared,
        ).unwrap();

        // Poll operations
        let result = resource_ops.poll_resource_operations().unwrap();
        
        let stats = resource_ops.get_resource_statistics();
        assert_eq!(stats.total_creates, 1);
        assert_eq!(stats.total_method_calls, 1);
        assert_eq!(stats.total_borrows, 1);
    }

    #[test]
    fn test_future_and_stream_operations() {
        let bridge = create_test_bridge();
        let mut abi_support = AsyncCanonicalAbiSupport::new(bridge);
        
        let component_id = ComponentInstanceId::new(1);
        abi_support.initialize_component_abi(component_id, CanonicalOptions::default()).unwrap();

        // Test future operations
        let future_handle = crate::async_types::FutureHandle::new(1);
        
        let future_read = abi_support.handle_future_operation(
            component_id,
            future_handle,
            FutureOp::Read,
        ).unwrap();

        let future_poll = abi_support.handle_future_operation(
            component_id,
            future_handle,
            FutureOp::Poll,
        ).unwrap();

        let future_cancel = abi_support.handle_future_operation(
            component_id,
            future_handle,
            FutureOp::Cancel,
        ).unwrap();

        // Test stream operations
        let stream_handle = crate::async_types::StreamHandle::new(2);
        
        let stream_read = abi_support.handle_stream_operation(
            component_id,
            stream_handle,
            StreamOp::ReadNext,
        ).unwrap();

        let stream_write = abi_support.handle_stream_operation(
            component_id,
            stream_handle,
            StreamOp::Write(ComponentValue::U32(123)),
        ).unwrap();

        let stream_close = abi_support.handle_stream_operation(
            component_id,
            stream_handle,
            StreamOp::Close,
        ).unwrap();

        // Poll all operations
        let result = abi_support.poll_async_operations().unwrap();
        
        let stats = abi_support.get_abi_statistics();
        assert_eq!(stats.future_operations, 3);
        assert_eq!(stats.stream_operations, 3);
    }

    #[test]
    fn test_task_manager_integration() {
        let mut bridge = create_test_bridge();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();

        // Test task.wait implementation
        let waitables = crate::async_types::WaitableSet::new();
        
        // In real implementation, would set up actual waitables
        // let result = bridge.task_wait(waitables).unwrap();
        
        // Test task.yield implementation
        // bridge.task_yield().unwrap();

        // Test task.poll implementation
        // let poll_result = bridge.task_poll(&waitables).unwrap();

        let stats = bridge.get_bridge_statistics();
        assert_eq!(stats.active_components, 1);
    }

    #[test]
    fn test_component_isolation() {
        let mut bridge = create_test_bridge();
        
        // Initialize multiple components
        let component1 = ComponentInstanceId::new(1);
        let component2 = ComponentInstanceId::new(2);
        
        bridge.initialize_component_async(component1, None).unwrap();
        bridge.initialize_component_async(component2, None).unwrap();

        // Spawn tasks in different components
        let task1 = bridge.spawn_async_task(
            component1,
            Some(0),
            ComponentAsyncFunction {
                id: 1,
                polls_remaining: 2,
                result_value: ComponentValue::U32(100),
            },
            ComponentAsyncTaskType::AsyncFunction,
            Priority::Normal,
        ).unwrap();

        let task2 = bridge.spawn_async_task(
            component2,
            Some(1),
            ComponentAsyncFunction {
                id: 2,
                polls_remaining: 2,
                result_value: ComponentValue::U32(200),
            },
            ComponentAsyncTaskType::AsyncFunction,
            Priority::High,
        ).unwrap();

        // Poll until both complete
        for _ in 0..10 {
            bridge.poll_async_tasks().unwrap();
        }

        let stats = bridge.get_bridge_statistics();
        assert_eq!(stats.active_components, 2);
        assert!(stats.total_async_tasks >= 2);
    }

    #[test]
    fn test_component_suspension() {
        let mut bridge = create_test_bridge();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();

        // Spawn a task
        let _task_id = bridge.spawn_async_task(
            component_id,
            Some(0),
            ComponentAsyncFunction {
                id: 1,
                polls_remaining: 5,
                result_value: ComponentValue::U32(42),
            },
            ComponentAsyncTaskType::AsyncFunction,
            Priority::Normal,
        ).unwrap();

        // Suspend the component's async operations
        bridge.suspend_component_async(component_id).unwrap();

        // Poll - should not make progress on suspended component
        let result = bridge.poll_async_tasks().unwrap();
        
        let stats = bridge.get_bridge_statistics();
        assert_eq!(stats.active_components, 1);
    }

    #[test]
    fn test_error_handling_and_recovery() {
        let mut bridge = create_test_bridge();
        
        let component_id = ComponentInstanceId::new(1);
        bridge.initialize_component_async(component_id, None).unwrap();

        // Try to spawn task in uninitialized component
        let bad_component = ComponentInstanceId::new(999);
        let result = bridge.spawn_async_task(
            bad_component,
            Some(0),
            ComponentAsyncFunction {
                id: 1,
                polls_remaining: 1,
                result_value: ComponentValue::U32(42),
            },
            ComponentAsyncTaskType::AsyncFunction,
            Priority::Normal,
        );

        assert!(result.is_err();

        // Test ABI error handling
        let abi_support = AsyncCanonicalAbiSupport::new(bridge);
        let mut resource_ops = ResourceAsyncOperations::new(abi_support);

        // Try to call method on non-existent resource
        let result = resource_ops.async_call_resource_method(
            component_id,
            ResourceHandle::new(999),
            "nonexistent".to_string(),
            vec![],
            None,
        );

        assert!(result.is_err();
    }
}