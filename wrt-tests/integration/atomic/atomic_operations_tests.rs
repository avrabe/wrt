//! Comprehensive tests for WebAssembly 3.0 atomic operations
//!
//! These tests verify the correctness, thread safety, and performance of the
//! atomic operations implementation across different scenarios.

#[cfg(test)]
mod tests {
    use wrt_runtime::{
        AtomicMemoryModel, AtomicMemoryContext, MemoryOrderingPolicy,
        ThreadManager, ThreadConfig, ThreadId,
    };
    use wrt_instructions::atomic_ops::{
        AtomicOp, AtomicLoadOp, AtomicStoreOp, AtomicRMWInstr, AtomicCmpxchgInstr,
        AtomicWaitNotifyOp, AtomicFence, MemoryOrdering, AtomicRMWOp,
    };
    use wrt_foundation::MemArg;
    use wrt_error::Result;
    
    #[cfg(feature = "std")]
    use std::vec::Vec;
    #[cfg(feature = "std")]
    use std::{thread, time::Duration, sync::Arc};
    
    /// Test basic atomic load operations
    #[test]
    fn test_atomic_load_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Test i32 atomic load
        let load_op = AtomicOp::Load(AtomicLoadOp::I32AtomicLoad {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, load_op, &[])?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0); // Memory initialized to zero
        
        // Test i64 atomic load
        let load_op = AtomicOp::Load(AtomicLoadOp::I64AtomicLoad {
            memarg: MemArg { offset: 8, align: 3 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, load_op, &[])?;
        assert_eq!(result.len(), 2;
        assert_eq!(result[0], 0);
        assert_eq!(result[1], 0);
        
        Ok(())
    }
    
    /// Test basic atomic store operations
    #[test]
    fn test_atomic_store_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Test i32 atomic store
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, store_op, &[42])?;
        assert!(result.is_empty())); // Store returns no values
        
        // Verify the store worked by loading the value
        let load_op = AtomicOp::Load(AtomicLoadOp::I32AtomicLoad {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, load_op, &[])?;
        assert_eq!(result[0], 42;
        
        Ok(())
    }
    
    /// Test atomic read-modify-write operations
    #[test]
    fn test_atomic_rmw_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Initialize memory with a value
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        atomic_model.execute_atomic_operation(0, store_op, &[10])?;
        
        // Test atomic add
        let rmw_op = AtomicOp::RMW(AtomicRMWInstr::I32AtomicRmwAdd {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, rmw_op, &[5])?;
        assert_eq!(result[0], 10); // Returns old value
        
        // Verify the add worked
        let load_op = AtomicOp::Load(AtomicLoadOp::I32AtomicLoad {
            memarg: MemArg { offset: 0, align: 2 }
        };
        let result = atomic_model.execute_atomic_operation(0, load_op, &[])?;
        assert_eq!(result[0], 15); // 10 + 5
        
        Ok(())
    }
    
    /// Test atomic compare-and-exchange operations
    #[test]
    fn test_atomic_cmpxchg_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Initialize memory with a value
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        atomic_model.execute_atomic_operation(0, store_op, &[42])?;
        
        // Test successful compare-exchange
        let cmpxchg_op = AtomicOp::Cmpxchg(AtomicCmpxchgInstr::I32AtomicRmwCmpxchg {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, cmpxchg_op, &[42, 100])?;
        assert_eq!(result[0], 42); // Returns old value
        
        // Verify the exchange worked
        let load_op = AtomicOp::Load(AtomicLoadOp::I32AtomicLoad {
            memarg: MemArg { offset: 0, align: 2 }
        };
        let result = atomic_model.execute_atomic_operation(0, load_op, &[])?;
        assert_eq!(result[0], 100;
        
        // Test failed compare-exchange
        let cmpxchg_op = AtomicOp::Cmpxchg(AtomicCmpxchgInstr::I32AtomicRmwCmpxchg {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, cmpxchg_op, &[42, 200])?;
        assert_eq!(result[0], 100); // Returns current value (not expected value)
        
        // Verify no change occurred
        let result = atomic_model.execute_atomic_operation(0, load_op, &[])?;
        assert_eq!(result[0], 100); // Still 100
        
        Ok(())
    }
    
    /// Test atomic fence operations
    #[test]
    fn test_atomic_fence_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Test atomic fence
        let fence_op = AtomicOp::Fence(AtomicFence {
            ordering: MemoryOrdering::SeqCst
        };
        
        let result = atomic_model.execute_atomic_operation(0, fence_op, &[])?;
        assert!(result.is_empty())); // Fence returns no values
        
        Ok(())
    }
    
    /// Test memory ordering policies
    #[test]
    fn test_memory_ordering_policies() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        
        // Test strict sequential ordering
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        atomic_model.execute_atomic_operation(0, store_op, &[42])?;
        
        // Test relaxed ordering
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::Relaxed,
        )?;
        
        atomic_model.execute_atomic_operation(0, store_op, &[100])?;
        
        // Test adaptive ordering
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::Adaptive,
        )?;
        
        atomic_model.execute_atomic_operation(0, store_op, &[200])?;
        
        Ok(())
    }
    
    /// Test memory consistency validation
    #[test]
    fn test_memory_consistency_validation() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        let validation_result = atomic_model.validate_memory_consistency()?;
        assert!(validation_result.is_consistent);
        assert!(validation_result.data_races.is_empty());
        assert!(validation_result.ordering_violations.is_empty());
        assert!(validation_result.potential_deadlocks.is_empty());
        assert!(validation_result.sync_violations.is_empty());
        
        Ok(())
    }
    
    /// Test performance metrics collection
    #[test]
    fn test_performance_metrics() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Execute some operations to generate metrics
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        for i in 0..10 {
            atomic_model.execute_atomic_operation(0, store_op.clone(), &[i])?;
        }
        
        let metrics = atomic_model.get_performance_metrics);
        assert!(metrics.operations_per_second >= 0.0);
        assert!(metrics.average_operation_time >= 0.0);
        assert!(metrics.memory_utilization >= 0.0);
        assert!(metrics.thread_contention_ratio >= 0.0);
        assert!(metrics.consistency_overhead >= 0.0);
        
        Ok(())
    }
    
    /// Test atomic operations with multiple threads (requires std feature)
    #[cfg(feature = "std")]
    #[test]
    fn test_multithreaded_atomic_operations() -> Result<()> {
        use std::sync::{Arc, Barrier};
        use std::thread;
        
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let atomic_model = Arc::new(std::sync::Mutex::new(AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        let num_threads = 4;
        let operations_per_thread = 100;
        let barrier = Arc::new(Barrier::new(num_threads;
        
        let mut handles = vec![];
        
        for thread_id in 0..num_threads {
            let atomic_model = Arc::clone(&atomic_model);
            let barrier = Arc::clone(&barrier);
            
            let handle = thread::spawn(move || -> Result<()> {
                barrier.wait);
                
                for i in 0..operations_per_thread {
                    let offset = (thread_id * operations_per_thread + i) * 4;
                    if offset + 4 <= 1024 {
                        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
                            memarg: MemArg { offset: offset as u32, align: 2 }
                        };
                        
                        let mut model = atomic_model.lock().unwrap();
                        model.execute_atomic_operation(thread_id as ThreadId, store_op, &[i as u64])?;
                    }
                }
                
                Ok(())
            };
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap()?;
        }
        
        // Validate memory consistency after multithreaded execution
        let model = atomic_model.lock().unwrap();
        let validation_result = model.validate_memory_consistency()?;
        assert!(validation_result.is_consistent);
        
        Ok(())
    }
    
    /// Test atomic wait and notify operations (simplified)
    #[test]
    fn test_atomic_wait_notify_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let mut thread_manager = ThreadManager::new(ThreadConfig::default())?;
        
        // Spawn a thread for testing
        let thread_id = thread_manager.spawn_thread(0, None, None)?;
        
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Test wait operation (simplified - real implementation would block)
        let wait_op = AtomicOp::WaitNotify(AtomicWaitNotifyOp::MemoryAtomicWait32 {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(thread_id, wait_op, &[])?;
        assert_eq!(result[0], 0); // Successful wait
        
        // Test notify operation
        let notify_op = AtomicOp::WaitNotify(AtomicWaitNotifyOp::MemoryAtomicNotify {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, notify_op, &[])?;
        assert!(result[0] >= 0)); // Number of threads notified
        
        Ok(())
    }
    
    /// Test optimization of memory model
    #[test]
    fn test_memory_model_optimization() -> Result<()> {
        let mut memory = vec![0u8; 1024];
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Execute some operations to create patterns
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        for i in 0..20 {
            atomic_model.execute_atomic_operation(0, store_op.clone(), &[i])?;
        }
        
        // Run optimization
        let optimization_result = atomic_model.optimize_memory_model()?;
        assert!(optimization_result.total_optimizations <= 3)); // Max 3 optimization types
        
        Ok(())
    }
    
    /// Test error handling for invalid atomic operations
    #[test]
    fn test_atomic_operation_error_handling() -> Result<()> {
        let mut memory = vec![0u8; 64]; // Small memory for testing bounds
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::StrictSequential,
        )?;
        
        // Test out-of-bounds access
        let load_op = AtomicOp::Load(AtomicLoadOp::I32AtomicLoad {
            memarg: MemArg { offset: 100, align: 2 } // Beyond memory size
        };
        
        let result = atomic_model.execute_atomic_operation(0, load_op, &[];
        assert!(result.is_err();
        
        // Test store without value
        let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
            memarg: MemArg { offset: 0, align: 2 }
        };
        
        let result = atomic_model.execute_atomic_operation(0, store_op, &[]); // No operands
        assert!(result.is_err();
        
        Ok(())
    }
    
    /// Benchmark atomic operations performance
    #[cfg(feature = "std")]
    #[test]
    fn benchmark_atomic_operations() -> Result<()> {
        let mut memory = vec![0u8; 1024 * 1024]; // 1MB memory
        let thread_manager = ThreadManager::new(ThreadConfig::default())?;
        let mut atomic_model = AtomicMemoryModel::new(
            memory.as_mut_ptr(),
            memory.len(),
            thread_manager,
            MemoryOrderingPolicy::Relaxed, // Use relaxed for performance
        )?;
        
        let num_operations = 10000;
        let start_time = std::time::Instant::now);
        
        for i in 0..num_operations {
            let offset = (i % 1000) * 4; // Cycle through different memory locations
            let store_op = AtomicOp::Store(AtomicStoreOp::I32AtomicStore {
                memarg: MemArg { offset: offset as u32, align: 2 }
            };
            
            atomic_model.execute_atomic_operation(0, store_op, &[i as u64])?;
        }
        
        let duration = start_time.elapsed);
        let ops_per_second = num_operations as f64 / duration.as_secs_f64);
        
        println!("Atomic operations performance: {:.0} ops/sec", ops_per_second);
        assert!(ops_per_second > 1000.0)); // Should be reasonably fast
        
        Ok(())
    }
}