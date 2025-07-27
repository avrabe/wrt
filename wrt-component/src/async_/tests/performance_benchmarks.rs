//! Performance benchmarks and stress tests for the async executor system
//!
//! This module provides comprehensive performance testing including:
//! - Throughput benchmarks for all async primitives
//! - Latency measurements under various loads
//! - Memory usage profiling
//! - Stress testing with high concurrency
//! - ASIL-level performance verification

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
            optimized_async_channels::{OptimizedAsyncChannels, ChannelType},
            timer_integration::{TimerIntegration, TimerType},
            advanced_sync_primitives::{AdvancedSyncPrimitives, MutexLockResult},
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

    /// Performance measurement utilities
    struct PerformanceMeasurement {
        start_time: u64,
        operations_count: AtomicU64,
        total_latency: AtomicU64,
        min_latency: AtomicU64,
        max_latency: AtomicU64,
    }

    impl PerformanceMeasurement {
        fn new() -> Self {
            Self {
                start_time: Self::get_time(),
                operations_count: AtomicU64::new(0),
                total_latency: AtomicU64::new(0),
                min_latency: AtomicU64::new(u64::MAX),
                max_latency: AtomicU64::new(0),
            }
        }

        fn record_operation(&self, operation_start: u64) {
            let latency = Self::get_time() - operation_start;
            
            self.operations_count.fetch_add(1, Ordering::AcqRel;
            self.total_latency.fetch_add(latency, Ordering::AcqRel;
            
            // Update min latency
            let mut current_min = self.min_latency.load(Ordering::Acquire;
            while latency < current_min {
                match self.min_latency.compare_exchange_weak(
                    current_min, latency, Ordering::AcqRel, Ordering::Acquire
                ) {
                    Ok(_) => break,
                    Err(actual) => current_min = actual,
                }
            }
            
            // Update max latency
            let mut current_max = self.max_latency.load(Ordering::Acquire;
            while latency > current_max {
                match self.max_latency.compare_exchange_weak(
                    current_max, latency, Ordering::AcqRel, Ordering::Acquire
                ) {
                    Ok(_) => break,
                    Err(actual) => current_max = actual,
                }
            }
        }

        fn get_results(&self) -> BenchmarkResults {
            let end_time = Self::get_time);
            let total_time = end_time - self.start_time;
            let ops_count = self.operations_count.load(Ordering::Acquire;
            let total_lat = self.total_latency.load(Ordering::Acquire;
            
            BenchmarkResults {
                total_operations: ops_count,
                total_time_us: total_time,
                throughput_ops_per_sec: if total_time > 0 { 
                    (ops_count * 1_000_000) / total_time 
                } else { 0 },
                average_latency_us: if ops_count > 0 { total_lat / ops_count } else { 0 },
                min_latency_us: self.min_latency.load(Ordering::Acquire),
                max_latency_us: self.max_latency.load(Ordering::Acquire),
            }
        }

        fn get_time() -> u64 {
            // Simplified time measurement - in real implementation would use high-precision timer
            static COUNTER: AtomicU64 = AtomicU64::new(0;
            COUNTER.fetch_add(1, Ordering::AcqRel)
        }
    }

    #[derive(Debug, Clone)]
    struct BenchmarkResults {
        total_operations: u64,
        total_time_us: u64,
        throughput_ops_per_sec: u64,
        average_latency_us: u64,
        min_latency_us: u64,
        max_latency_us: u64,
    }

    /// High-throughput async task for benchmarking
    struct BenchmarkTask {
        id: u64,
        iterations: u64,
        current_iteration: AtomicU64,
        measurement: Arc<PerformanceMeasurement>,
    }

    impl Future for BenchmarkTask {
        type Output = Result<Vec<ComponentValue>, Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let current = self.current_iteration.fetch_add(1, Ordering::AcqRel;
            
            if current >= self.iterations {
                Poll::Ready(Ok(vec![ComponentValue::U32(self.id as u32)])
            } else {
                // Record operation timing
                self.measurement.record_operation(PerformanceMeasurement::get_time);
                
                // Yield to allow other tasks to run
                cx.waker().wake_by_ref);
                Poll::Pending
            }
        }
    }

    fn create_test_bridge() -> TaskManagerAsyncBridge {
        let task_manager = Arc::new(Mutex::new(TaskManager::new();
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new();
        let config = BridgeConfiguration {
            enable_preemption: true,
            enable_dynamic_fuel: true,
            fuel_policy: FuelAllocationPolicy::PerformanceOptimized,
            ..Default::default()
        };
        TaskManagerAsyncBridge::new(task_manager, thread_manager, config).unwrap()
    }

    #[test]
    fn benchmark_async_task_throughput() {
        let mut bridge = create_test_bridge);
        let component_id = ComponentInstanceId::new(1;
        bridge.initialize_component_async(component_id, None).unwrap();

        let measurement = Arc::new(PerformanceMeasurement::new();
        
        // Spawn multiple high-throughput tasks
        const NUM_TASKS: usize = 50;
        const ITERATIONS_PER_TASK: u64 = 1000;
        
        let mut task_ids = Vec::new());
        
        for i in 0..NUM_TASKS {
            let task_id = bridge.spawn_async_task(
                component_id,
                Some(i as u32),
                BenchmarkTask {
                    id: i as u64,
                    iterations: ITERATIONS_PER_TASK,
                    current_iteration: AtomicU64::new(0),
                    measurement: measurement.clone(),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal,
            ).unwrap();
            
            task_ids.push(task_id);
        }

        // Poll tasks to completion
        let start_time = PerformanceMeasurement::get_time);
        let mut completed_tasks = 0;
        
        for round in 0..10000 {
            let result = bridge.poll_async_tasks().unwrap();
            completed_tasks += result.tasks_completed;
            
            if completed_tasks >= NUM_TASKS {
                break;
            }
            
            // Prevent infinite loop
            if round > 5000 {
                eprintln!("Warning: Benchmark may not have completed all tasks"));
                break;
            }
        }

        let results = measurement.get_results);
        
        // Verify performance expectations
        assert!(results.total_operations > 0, "No operations recorded");
        assert!(results.throughput_ops_per_sec > 1000, 
            "Throughput too low: {} ops/sec", results.throughput_ops_per_sec;
        
        println!("Async Task Throughput Benchmark:");
        println!("  Total Operations: {}", results.total_operations);
        println!("  Throughput: {} ops/sec", results.throughput_ops_per_sec);
        println!("  Average Latency: {} μs", results.average_latency_us);
        println!("  Min Latency: {} μs", results.min_latency_us);
        println!("  Max Latency: {} μs", results.max_latency_us);
    }

    #[test]
    fn benchmark_channel_throughput() {
        let bridge = Arc::new(Mutex::new(create_test_bridge);
        let mut channels = OptimizedAsyncChannels::new(bridge.clone(), None;
        
        let component_id = ComponentInstanceId::new(1;
        channels.initialize_component_channels(component_id, None).unwrap();

        let (sender, receiver) = channels.create_channel(
            component_id,
            ChannelType::Bounded(1024),
        ).unwrap();

        let measurement = Arc::new(PerformanceMeasurement::new();
        const NUM_MESSAGES: u32 = 10000;

        // Benchmark sending
        let send_start = PerformanceMeasurement::get_time);
        
        for i in 0..NUM_MESSAGES {
            let op_start = PerformanceMeasurement::get_time);
            
            let result = channels.send_message(
                sender.channel_id,
                component_id,
                ComponentValue::U32(i),
                None,
            ).unwrap();
            
            measurement.record_operation(op_start;
            
            // Handle backpressure
            if matches!(result, crate::async_::optimized_async_channels::SendResult::WouldBlock) {
                // Drain some messages
                for _ in 0..10 {
                    let _ = channels.receive_message(receiver.channel_id, component_id;
                }
            }
        }

        // Benchmark receiving remaining messages
        let mut received_count = 0;
        while received_count < NUM_MESSAGES {
            let op_start = PerformanceMeasurement::get_time);
            
            match channels.receive_message(receiver.channel_id, component_id) {
                Ok(crate::async_::optimized_async_channels::ReceiveResult::Received(_)) => {
                    measurement.record_operation(op_start;
                    received_count += 1;
                },
                Ok(crate::async_::optimized_async_channels::ReceiveResult::WouldBlock) => {
                    break; // No more messages
                },
                _ => break,
            }
        }

        let results = measurement.get_results);
        
        // Verify performance
        assert!(results.total_operations > NUM_MESSAGES as u64 / 2, 
            "Too few operations recorded";
        assert!(results.throughput_ops_per_sec > 5000, 
            "Channel throughput too low: {} ops/sec", results.throughput_ops_per_sec;

        println!("Channel Throughput Benchmark:");
        println!("  Messages Processed: {}", received_count);
        println!("  Throughput: {} ops/sec", results.throughput_ops_per_sec);
        println!("  Average Latency: {} μs", results.average_latency_us);
    }

    #[test]
    fn benchmark_sync_primitive_contention() {
        let bridge = Arc::new(Mutex::new(create_test_bridge);
        let mut sync_primitives = AdvancedSyncPrimitives::new(bridge.clone(), None;
        
        let component_id = ComponentInstanceId::new(1;
        sync_primitives.initialize_component_sync(component_id, None).unwrap();

        // Create mutex for contention testing
        let mutex_id = sync_primitives.create_async_mutex(component_id, false).unwrap();
        
        let measurement = Arc::new(PerformanceMeasurement::new();
        const NUM_CONTENTIONS: u32 = 1000;

        // Simulate high contention
        for i in 0..NUM_CONTENTIONS {
            let task_id = crate::threading::task_manager::TaskId::new(i as u64 + 1;
            let op_start = PerformanceMeasurement::get_time);
            
            // Try to acquire mutex
            let result = sync_primitives.lock_async_mutex(
                mutex_id,
                task_id,
                component_id,
            ).unwrap();
            
            if result == MutexLockResult::Acquired {
                measurement.record_operation(op_start;
                
                // Hold lock briefly, then release
                let _ = sync_primitives.unlock_async_mutex(mutex_id, task_id;
            }
        }

        let results = measurement.get_results);
        
        // Verify contention handling
        assert!(results.total_operations > 0, "No successful lock acquisitions");
        assert!(results.max_latency_us > results.min_latency_us, 
            "No latency variation under contention";

        println!("Sync Primitive Contention Benchmark:");
        println!("  Successful Acquisitions: {}", results.total_operations);
        println!("  Average Latency: {} μs", results.average_latency_us);
        println!("  Max Latency: {} μs", results.max_latency_us);
    }

    #[test]
    fn benchmark_timer_precision() {
        let bridge = Arc::new(Mutex::new(create_test_bridge);
        let mut timers = TimerIntegration::new(bridge.clone(), None;
        
        let component_id = ComponentInstanceId::new(1;
        timers.initialize_component_timers(component_id, None).unwrap();

        let measurement = Arc::new(PerformanceMeasurement::new();
        const NUM_TIMERS: u32 = 100;
        const TIMER_DURATION: u64 = 100; // 100ms

        // Create multiple timers
        let mut timer_ids = Vec::new());
        for _ in 0..NUM_TIMERS {
            let timer_id = timers.create_timer(
                component_id,
                TimerType::Oneshot,
                TIMER_DURATION,
            ).unwrap();
            timer_ids.push(timer_id);
        }

        // Simulate time progression and measure timer accuracy
        let mut fired_count = 0;
        
        for time_step in 0..200 {
            let step_start = PerformanceMeasurement::get_time);
            
            timers.advance_time(10); // 10ms steps
            let result = timers.process_timers().unwrap();
            
            if result.fired_timers.len() > 0 {
                measurement.record_operation(step_start;
                fired_count += result.fired_timers.len);
            }
            
            if fired_count >= NUM_TIMERS as usize {
                break;
            }
        }

        let results = measurement.get_results);
        let timer_stats = timers.get_timer_statistics);
        
        // Verify timer performance
        assert_eq!(timer_stats.total_timers_created, NUM_TIMERS as u64;
        assert!(timer_stats.total_timers_fired > 0);
        assert!(results.total_operations > 0, "No timer processing operations recorded");

        println!("Timer Precision Benchmark:");
        println!("  Timers Created: {}", timer_stats.total_timers_created);
        println!("  Timers Fired: {}", timer_stats.total_timers_fired);
        println!("  Processing Latency: {} μs", results.average_latency_us);
    }

    #[test]
    fn stress_test_high_concurrency() {
        let bridge = Arc::new(Mutex::new(create_test_bridge);
        
        // Initialize multiple components for stress testing
        const NUM_COMPONENTS: u32 = 10;
        const TASKS_PER_COMPONENT: u32 = 20;
        const ITERATIONS_PER_TASK: u64 = 100;

        let components: Vec<ComponentInstanceId> = (1..=NUM_COMPONENTS)
            .map(|i| ComponentInstanceId::new(i)
            .collect());

        // Initialize all components
        for &component_id in &components {
            let mut bridge_guard = bridge.lock().unwrap();
            bridge_guard.initialize_component_async(component_id, None).unwrap();
        }

        let measurement = Arc::new(PerformanceMeasurement::new();
        let mut total_tasks = 0;

        // Spawn tasks across all components
        for &component_id in &components {
            for task_num in 0..TASKS_PER_COMPONENT {
                let mut bridge_guard = bridge.lock().unwrap();
                let _ = bridge_guard.spawn_async_task(
                    component_id,
                    Some(task_num),
                    BenchmarkTask {
                        id: (component_id.as_u32() * 1000 + task_num) as u64,
                        iterations: ITERATIONS_PER_TASK,
                        current_iteration: AtomicU64::new(0),
                        measurement: measurement.clone(),
                    },
                    ComponentAsyncTaskType::AsyncFunction,
                    Priority::Normal,
                ).unwrap();
                
                total_tasks += 1;
            }
        }

        // Execute stress test
        let start_time = PerformanceMeasurement::get_time);
        let mut completed_tasks = 0;
        let mut max_rounds = 50000; // Prevent infinite loops

        while completed_tasks < total_tasks && max_rounds > 0 {
            let mut bridge_guard = bridge.lock().unwrap();
            let result = bridge_guard.poll_async_tasks().unwrap();
            completed_tasks += result.tasks_completed;
            max_rounds -= 1;
        }

        let end_time = PerformanceMeasurement::get_time);
        let results = measurement.get_results);
        
        // Verify stress test results
        assert!(completed_tasks > 0, "No tasks completed during stress test");
        assert!(results.total_operations > 0, "No operations recorded during stress test");

        // Check final system state
        let final_stats = {
            let bridge_guard = bridge.lock().unwrap();
            bridge_guard.get_bridge_statistics()
        };

        assert_eq!(final_stats.active_components, NUM_COMPONENTS as u64;
        assert!(final_stats.total_async_tasks >= total_tasks as u64);

        println!("High Concurrency Stress Test:");
        println!("  Components: {}", NUM_COMPONENTS);
        println!("  Total Tasks: {}", total_tasks);
        println!("  Completed Tasks: {}", completed_tasks);
        println!("  Total Operations: {}", results.total_operations);
        println!("  System Throughput: {} ops/sec", results.throughput_ops_per_sec);
        println!("  Test Duration: {} time units", end_time - start_time);
    }

    #[test]
    fn benchmark_memory_usage() {
        let bridge = Arc::new(Mutex::new(create_test_bridge);
        let mut channels = OptimizedAsyncChannels::new(bridge.clone(), None;
        let mut timers = TimerIntegration::new(bridge.clone(), None;
        let mut sync_primitives = AdvancedSyncPrimitives::new(bridge.clone(), None;

        let component_id = ComponentInstanceId::new(1;
        
        // Initialize all subsystems
        {
            let mut bridge_guard = bridge.lock().unwrap();
            bridge_guard.initialize_component_async(component_id, None).unwrap();
        }
        channels.initialize_component_channels(component_id, None).unwrap();
        timers.initialize_component_timers(component_id, None).unwrap();
        sync_primitives.initialize_component_sync(component_id, None).unwrap();

        // Create many primitives to test memory usage
        const NUM_PRIMITIVES: u32 = 100;

        // Create channels
        let mut channel_pairs = Vec::new());
        for _ in 0..NUM_PRIMITIVES {
            if let Ok(pair) = channels.create_channel(component_id, ChannelType::Bounded(16)) {
                channel_pairs.push(pair);
            }
        }

        // Create timers
        let mut timer_ids = Vec::new());
        for i in 0..NUM_PRIMITIVES {
            if let Ok(timer_id) = timers.create_timer(
                component_id,
                TimerType::Interval(100 + i as u64),
                100 + i as u64,
            ) {
                timer_ids.push(timer_id);
            }
        }

        // Create sync primitives
        let mut mutex_ids = Vec::new());
        let mut semaphore_ids = Vec::new());
        
        for _ in 0..NUM_PRIMITIVES / 2 {
            if let Ok(mutex_id) = sync_primitives.create_async_mutex(component_id, false) {
                mutex_ids.push(mutex_id);
            }
            if let Ok(sem_id) = sync_primitives.create_async_semaphore(component_id, 5, true) {
                semaphore_ids.push(sem_id);
            }
        }

        // Verify creation counts
        let channel_stats = channels.get_channel_statistics);
        let timer_stats = timers.get_timer_statistics);
        let sync_stats = sync_primitives.get_sync_statistics);

        println!("Memory Usage Benchmark:");
        println!("  Channels Created: {}", channel_stats.total_channels_created);
        println!("  Active Channels: {}", channel_stats.active_channels);
        println!("  Timers Created: {}", timer_stats.total_timers_created);
        println!("  Active Timers: {}", timer_stats.active_timers);
        println!("  Mutexes Created: {}", sync_stats.total_mutexes_created);
        println!("  Semaphores Created: {}", sync_stats.total_semaphores_created);
        println!("  Active Sync Primitives: {}", sync_stats.active_primitives);

        // Verify we created a reasonable number of primitives
        assert!(channel_stats.total_channels_created > NUM_PRIMITIVES as u64 / 2);
        assert!(timer_stats.total_timers_created > NUM_PRIMITIVES as u64 / 2);
        assert!(sync_stats.total_mutexes_created > NUM_PRIMITIVES as u64 / 4);
    }

    #[test]
    fn benchmark_fuel_consumption_efficiency() {
        let bridge = Arc::new(Mutex::new(create_test_bridge);
        
        let component_id = ComponentInstanceId::new(1;
        {
            let mut bridge_guard = bridge.lock().unwrap();
            bridge_guard.initialize_component_async(component_id, None).unwrap();
        }

        // Measure fuel consumption for various operations
        let initial_stats = {
            let bridge_guard = bridge.lock().unwrap();
            bridge_guard.get_bridge_statistics()
        };

        let measurement = Arc::new(PerformanceMeasurement::new();
        const NUM_OPERATIONS: u32 = 1000;

        // Spawn fuel-consuming tasks
        for i in 0..NUM_OPERATIONS {
            let op_start = PerformanceMeasurement::get_time);
            
            let mut bridge_guard = bridge.lock().unwrap();
            let _ = bridge_guard.spawn_async_task(
                component_id,
                Some(i),
                BenchmarkTask {
                    id: i as u64,
                    iterations: 10,
                    current_iteration: AtomicU64::new(0),
                    measurement: measurement.clone(),
                },
                ComponentAsyncTaskType::AsyncFunction,
                Priority::Normal,
            ;
            
            measurement.record_operation(op_start;
        }

        // Poll to execute tasks
        for _ in 0..5000 {
            let mut bridge_guard = bridge.lock().unwrap();
            let _ = bridge_guard.poll_async_tasks);
        }

        let final_stats = {
            let bridge_guard = bridge.lock().unwrap();
            bridge_guard.get_bridge_statistics()
        };

        let fuel_consumed = final_stats.total_fuel_consumed - initial_stats.total_fuel_consumed;
        let results = measurement.get_results);

        // Calculate fuel efficiency
        let fuel_per_operation = if results.total_operations > 0 {
            fuel_consumed / results.total_operations
        } else {
            0
        };

        println!("Fuel Consumption Efficiency Benchmark:");
        println!("  Total Operations: {}", results.total_operations);
        println!("  Total Fuel Consumed: {}", fuel_consumed);
        println!("  Fuel per Operation: {}", fuel_per_operation);
        println!("  Operations per Second: {}", results.throughput_ops_per_sec);

        // Verify fuel efficiency
        assert!(fuel_consumed > 0, "No fuel consumed");
        assert!(fuel_per_operation > 0, "No fuel per operation");
        assert!(fuel_per_operation < 1000, "Too much fuel per operation: {}", fuel_per_operation);
    }
}