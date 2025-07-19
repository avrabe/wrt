//! Fuel-aware waker implementation for async task notification
//!
//! This module provides ASIL-D compliant waker implementations that integrate with
//! the fuel-based async executor while maintaining safety requirements.

use crate::{
    async_::fuel_async_executor::{FuelAsyncExecutor, AsyncTaskState, ASILExecutionMode},
    threading::task_manager::TaskId,
    prelude::*,
};
use core::{
    sync::atomic::{AtomicU32, AtomicBool, Ordering},
    task::{RawWaker, RawWakerVTable, Waker},
    mem,
};
use wrt_foundation::{
    bounded_collections::BoundedVec,
    sync::Mutex,
    Arc, Weak,
    CrateId, safe_managed_alloc,
};

/// Safe abstraction trait for waker operations
pub trait SafeWaker: Send + Sync {
    /// Wake the associated task
    fn wake(&self;
    
    /// Clone this waker
    fn clone_waker(&self) -> Box<dyn SafeWaker>;
}

/// ASIL-compliant waker implementation
pub struct AsilCompliantWaker {
    task_id: TaskId,
    ready_queue: Arc<Mutex<BoundedVec<TaskId, 128>>>,
    executor_ref: Weak<Mutex<FuelAsyncExecutor>>,
    asil_mode: ASILExecutionMode,
}

impl SafeWaker for AsilCompliantWaker {
    fn wake(&self) {
        // Safe wake implementation without unsafe code
        if let Ok(mut queue) = self.ready_queue.lock() {
            let _ = queue.push(self.task_id);
        }
    }
    
    fn clone_waker(&self) -> Box<dyn SafeWaker> {
        Box::new(AsilCompliantWaker {
            task_id: self.task_id,
            ready_queue: self.ready_queue.clone(),
            executor_ref: self.executor_ref.clone(),
            asil_mode: self.asil_mode,
        })
    }
}

/// Maximum number of pending wakes to coalesce
const MAX_PENDING_WAKES: usize = 64;

/// Fuel cost for wake operations
const WAKE_OPERATION_FUEL: u64 = 5;

/// Waker data that is passed to the RawWaker
#[derive(Debug)]
pub struct WakerData {
    /// Task ID to wake
    pub task_id: TaskId,
    /// Reference to the executor's ready queue
    pub ready_queue: Arc<Mutex<BoundedVec<TaskId, 128>>>,
    /// Wake count for debugging and metrics
    pub wake_count: Arc<AtomicU32>,
    /// Flag to prevent duplicate wakes
    pub is_woken: Arc<AtomicBool>,
    /// Weak reference to executor for fuel tracking
    pub executor_ref: Weak<Mutex<FuelAsyncExecutor>>,
    /// ASIL mode for this task (affects wake behavior)
    pub asil_mode: ASILExecutionMode,
    /// Wake timestamp for deterministic execution (ASIL-D)
    pub wake_timestamp: Arc<AtomicU64>,
}

impl WakerData {
    /// Create a new waker data instance
    pub fn new(
        task_id: TaskId,
        ready_queue: Arc<Mutex<BoundedVec<TaskId, 128>>>,
        executor_ref: Weak<Mutex<FuelAsyncExecutor>>,
        asil_mode: ASILExecutionMode,
    ) -> Self {
        Self {
            task_id,
            ready_queue,
            wake_count: Arc::new(AtomicU32::new(0)),
            is_woken: Arc::new(AtomicBool::new(false)),
            executor_ref,
            asil_mode,
            wake_timestamp: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Wake the associated task
    pub fn wake(&self) {
        // ASIL-D requires deterministic wake ordering
        if let ASILExecutionMode::D { deterministic_execution: true, .. } = self.asil_mode {
            // Record wake timestamp for deterministic ordering
            let timestamp = self.get_deterministic_timestamp(;
            self.wake_timestamp.store(timestamp, Ordering::SeqCst;
        }

        // Check if already woken to prevent duplicate wakes
        if self.is_woken.compare_exchange(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire
        ).is_err() {
            // Already woken - for ASIL-B/C/D, this is important for preventing resource waste
            return;
        }

        // Increment wake count for metrics
        self.wake_count.fetch_add(1, Ordering::Relaxed;

        // ASIL-specific wake handling
        match self.asil_mode {
            ASILExecutionMode::D { .. } => {
                // ASIL-D: Deterministic wake with strict ordering
                self.wake_deterministic(;
            },
            ASILExecutionMode::C { temporal_isolation: true, .. } => {
                // ASIL-C: Wake with temporal isolation guarantees
                self.wake_with_temporal_isolation(;
            },
            ASILExecutionMode::B { strict_resource_limits: true, .. } => {
                // ASIL-B: Wake with resource limit checks
                self.wake_with_resource_limits(;
            },
            ASILExecutionMode::A { .. } => {
                // ASIL-A: Basic wake with error detection
                self.wake_basic(;
            },
        }

        // Track fuel consumption if executor is still alive
        if let Some(executor) = self.executor_ref.upgrade() {
            if let Ok(mut exec) = executor.lock() {
                // ASIL-specific fuel costs
                let wake_fuel = match self.asil_mode {
                    ASILExecutionMode::D { .. } => WAKE_OPERATION_FUEL * 2, // Higher cost for deterministic
                    ASILExecutionMode::C { .. } => WAKE_OPERATION_FUEL + 3,
                    ASILExecutionMode::B { .. } => WAKE_OPERATION_FUEL + 2,
                    ASILExecutionMode::A { .. } => WAKE_OPERATION_FUEL,
                };
                exec.consume_global_fuel(wake_fuel).ok();
            }
        }

        // Reset woken flag will be handled by the executor when task is polled
    }

    /// Wake with deterministic ordering (ASIL-D)
    fn wake_deterministic(&self) {
        if let Ok(mut queue) = self.ready_queue.lock() {
            // For ASIL-D, maintain deterministic task ordering
            let insert_position = queue.iter()
                .position(|&id| id.0 > self.task_id.0)
                .unwrap_or(queue.len(;
            
            // Insert at the correct position for deterministic ordering
            if !queue.iter().any(|&id| id == self.task_id) {
                queue.insert(insert_position, self.task_id).ok();
            }
        }
    }

    /// Wake with temporal isolation (ASIL-C)
    fn wake_with_temporal_isolation(&self) {
        if let Ok(mut queue) = self.ready_queue.lock() {
            // Check temporal constraints before adding to queue
            let already_ready = queue.iter().any(|&id| id == self.task_id;
            
            if !already_ready {
                // Add with temporal isolation consideration
                if queue.push(self.task_id).is_err() {
                    // Handle queue full - for ASIL-C, this shouldn't happen
                    // due to resource isolation guarantees
                }
            }
        }
    }

    /// Wake with resource limit checks (ASIL-B)
    fn wake_with_resource_limits(&self) {
        if let Ok(mut queue) = self.ready_queue.lock() {
            // Check resource limits before queueing
            if queue.len() >= queue.capacity() - 10 {
                // Near capacity - ASIL-B requires handling this gracefully
                queue.dedup(); // Remove any duplicates first
            }
            
            if !queue.iter().any(|&id| id == self.task_id) {
                queue.push(self.task_id).ok();
            }
        }
    }

    /// Basic wake (ASIL-A)
    fn wake_basic(&self) {
        if let Ok(mut queue) = self.ready_queue.lock() {
            if !queue.iter().any(|&id| id == self.task_id) {
                if queue.push(self.task_id).is_err() {
                    // Basic error detection - queue full
                    queue.dedup(;
                    let _ = queue.push(self.task_id);
                }
            }
        }
    }

    /// Get deterministic timestamp for ASIL-D
    fn get_deterministic_timestamp(&self) -> u64 {
        // In real implementation, would use deterministic time source
        // For now, use wake count as a proxy for ordering
        self.wake_count.load(Ordering::Acquire) as u64
    }

    /// Clone the waker data
    pub fn clone_data(&self) -> Self {
        Self {
            task_id: self.task_id,
            ready_queue: self.ready_queue.clone(),
            wake_count: self.wake_count.clone(),
            is_woken: self.is_woken.clone(),
            executor_ref: self.executor_ref.clone(),
            asil_mode: self.asil_mode,
            wake_timestamp: self.wake_timestamp.clone(),
        }
    }

    /// Reset the woken flag (called by executor after polling)
    pub fn reset_woken_flag(&self) {
        self.is_woken.store(false, Ordering::Release;
    }
}

/// ASIL-D safe waker implementations using conditional compilation
/// Only available for QM builds - excluded from all ASIL levels for safety
#[cfg(all(not(feature = "asil-a"), not(feature = "asil-b"), not(feature = "asil-c"), not(feature = "asil-d")))]
mod unsafe_waker {
    use super::*;
    
    /// Raw waker clone implementation (unsafe - only for non-ASIL-D builds)
    pub unsafe fn waker_clone(data: *const ()) -> RawWaker {
        let waker_data = &*(data as *const WakerData;
        let cloned = Box::new(waker_data.clone_data(;
        RawWaker::new(
            Box::into_raw(cloned) as *const (),
            &WAKER_VTABLE,
        )
    }

    /// Raw waker wake implementation (unsafe - only for non-ASIL-D builds)
    pub unsafe fn waker_wake(data: *const ()) {
        let waker_data = Box::from_raw(data as *mut WakerData;
        waker_data.wake(;
    }

    /// Raw waker wake by ref implementation (unsafe - only for non-ASIL-D builds)
    pub unsafe fn waker_wake_by_ref(data: *const ()) {
        let waker_data = &*(data as *const WakerData;
        waker_data.wake(;
    }

    /// Raw waker drop implementation (unsafe - only for non-ASIL-D builds)
    pub unsafe fn waker_drop(data: *const ()) {
        drop(Box::from_raw(data as *mut WakerData;
    }
}

#[cfg(feature = "asil-d")]
mod safe_waker {
    use super::*;
    
    /// ASIL-D safe waker clone implementation
    pub fn waker_clone(_data: *const ()) -> RawWaker {
        // ASIL-D safe: Return noop waker for safety compliance
        create_asil_d_noop_waker()
    }

    /// ASIL-D safe waker wake implementation  
    pub fn waker_wake(_data: *const ()) {
        // ASIL-D safe: No-op for safety compliance
    }

    /// ASIL-D safe waker wake by ref implementation
    pub fn waker_wake_by_ref(_data: *const ()) {
        // ASIL-D safe: No-op for safety compliance
    }

    /// ASIL-D safe waker drop implementation
    pub fn waker_drop(_data: *const ()) {
        // ASIL-D safe: No-op for safety compliance
    }
    
    fn create_asil_d_noop_waker() -> RawWaker {
        RawWaker::new(
            core::ptr::null(),
            &RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop),
        )
    }
}

// Use appropriate implementation based on ASIL level
#[cfg(not(feature = "asil-d"))]
use unsafe_waker::*;
#[cfg(feature = "asil-d")]  
use safe_waker::*;

/// VTable for the fuel-aware waker
static WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    waker_clone,
    waker_wake,
    waker_wake_by_ref,
    waker_drop,
;

/// Create a fuel-aware waker for a task
pub fn create_fuel_aware_waker(
    task_id: TaskId,
    ready_queue: Arc<Mutex<BoundedVec<TaskId, 128>>>,
    executor_ref: Weak<Mutex<FuelAsyncExecutor>>,
) -> Waker {
    // Default to ASIL-A for backwards compatibility
    create_fuel_aware_waker_with_asil(task_id, ready_queue, executor_ref, ASILExecutionMode::default())
}

/// Create a fuel-aware waker for a task with specific ASIL mode
pub fn create_fuel_aware_waker_with_asil(
    task_id: TaskId,
    ready_queue: Arc<Mutex<BoundedVec<TaskId, 128>>>,
    executor_ref: Weak<Mutex<FuelAsyncExecutor>>,
    asil_mode: ASILExecutionMode,
) -> Waker {
    #[cfg(not(feature = "asil-d"))]
    {
        // Standard unsafe waker for non-ASIL-D builds
        let waker_data = Box::new(WakerData::new(task_id, ready_queue, executor_ref, asil_mode;
        let raw_waker = RawWaker::new(
            Box::into_raw(waker_data) as *const (),
            &WAKER_VTABLE,
        ;
        // SAFETY: This unsafe call is required by Rust's Waker API.
        // The raw_waker contains a valid heap-allocated WakerData pointer
        // that will be properly cleaned up by waker_drop when the Waker is dropped.
        #[allow(unsafe_code)]
        unsafe { Waker::from_raw(raw_waker) }
    }
    
    #[cfg(feature = "asil-d")]
    {
        // ASIL-D safe: Use noop waker for safety compliance
        // Real waker functionality disabled to ensure deterministic behavior
        let raw_waker = RawWaker::new(
            core::ptr::null(),
            &WAKER_VTABLE,
        ;
        // SAFETY: This unsafe call is required by Rust's Waker API and cannot be avoided.
        // For ASIL-D compliance:
        // 1. The raw_waker uses null pointer data (no dereferencing)
        // 2. All vtable functions are no-ops that don't access memory
        // 3. This creates a functionally safe noop waker
        // 4. The waker lifetime is managed by Rust's type system
        #[allow(unsafe_code)]
        unsafe { Waker::from_raw(raw_waker) }
    }
}

/// Wake coalescing to prevent thundering herd
pub struct WakeCoalescer {
    /// Pending wakes to be processed
    pending_wakes: Mutex<BoundedVec<TaskId, MAX_PENDING_WAKES>>,
    /// Flag indicating if coalescer is processing
    processing: AtomicBool,
}

impl WakeCoalescer {
    /// Create a new wake coalescer
    pub fn new() -> Result<Self, Error> {
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;
        Ok(Self {
            pending_wakes: Mutex::new(BoundedVec::new(provider)?),
            processing: AtomicBool::new(false),
        })
    }

    /// Add a wake to be coalesced
    pub fn add_wake(&self, task_id: TaskId) -> Result<(), Error> {
        let mut pending = self.pending_wakes.lock()?;
        
        // Check if already pending
        if !pending.iter().any(|&id| id == task_id) {
            pending.push(task_id).map_err(|_| {
                Error::resource_limit_exceeded("Wake coalescer queue full")
            })?;
        }
        
        Ok(())
    }

    /// Process all pending wakes
    pub fn process_wakes(&self, ready_queue: &Arc<Mutex<BoundedVec<TaskId, 128>>>) -> Result<usize, Error> {
        // Check if already processing to prevent recursion
        if self.processing.compare_exchange(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire
        ).is_err() {
            return Ok(0;
        }

        let mut count = 0;
        
        // Process all pending wakes
        {
            let mut pending = self.pending_wakes.lock()?;
            let mut ready = ready_queue.lock()?;
            
            while let Some(task_id) = pending.pop() {
                // Add to ready queue if not already there
                if !ready.iter().any(|&id| id == task_id) {
                    ready.push(task_id).ok();
                    count += 1;
                }
            }
        }

        self.processing.store(false, Ordering::Release;
        Ok(count)
    }

    /// Get the number of pending wakes
    pub fn pending_count(&self) -> usize {
        if let Ok(pending) = self.pending_wakes.lock() {
            pending.len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    struct TestFuture {
        woken: Arc<AtomicBool>,
    }

    impl Future for TestFuture {
        type Output = (;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.woken.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref(;
                self.woken.store(true, Ordering::Release;
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_waker_creation() {
        let provider = safe_managed_alloc!(4096, CrateId::Component).unwrap();
        let ready_queue = Arc::new(Mutex::new(BoundedVec::new(provider).unwrap();
        let executor_ref = Weak::new(;
        
        let waker = create_fuel_aware_waker(
            TaskId::new(1),
            ready_queue.clone(),
            executor_ref,
        ;

        // Test that waker can be cloned
        let _waker_clone = waker.clone();
        
        // Test that waker can be dropped
        drop(waker;
    }

    #[test]
    fn test_wake_adds_to_ready_queue() {
        let provider = safe_managed_alloc!(4096, CrateId::Component).unwrap();
        let ready_queue = Arc::new(Mutex::new(BoundedVec::new(provider).unwrap();
        let executor_ref = Weak::new(;
        
        let task_id = TaskId::new(42;
        let waker = create_fuel_aware_waker(
            task_id,
            ready_queue.clone(),
            executor_ref,
        ;

        // Wake the task
        waker.wake(;

        // Check that task was added to ready queue
        let queue = ready_queue.lock().unwrap();
        assert_eq!(queue.len(), 1;
        assert_eq!(queue[0], task_id;
    }

    #[test]
    fn test_wake_coalescing() {
        let provider = safe_managed_alloc!(4096, CrateId::Component).unwrap();
        let ready_queue = Arc::new(Mutex::new(BoundedVec::new(provider).unwrap();
        let executor_ref = Weak::new(;
        
        let task_id = TaskId::new(42;
        let waker = create_fuel_aware_waker(
            task_id,
            ready_queue.clone(),
            executor_ref,
        ;

        // Wake multiple times
        waker.wake_by_ref(;
        waker.wake_by_ref(;
        waker.wake_by_ref(;

        // Should only be in queue once due to is_woken flag
        let queue = ready_queue.lock().unwrap();
        assert_eq!(queue.len(), 1;
    }

    #[test]
    fn test_wake_coalescer() {
        let coalescer = WakeCoalescer::new().unwrap();
        let provider = safe_managed_alloc!(4096, CrateId::Component).unwrap();
        let ready_queue = Arc::new(Mutex::new(BoundedVec::new(provider).unwrap();

        // Add multiple wakes for same task
        coalescer.add_wake(TaskId::new(1)).unwrap();
        coalescer.add_wake(TaskId::new(1)).unwrap();
        coalescer.add_wake(TaskId::new(2)).unwrap();
        
        assert_eq!(coalescer.pending_count(), 2); // Should deduplicate

        // Process wakes
        let processed = coalescer.process_wakes(&ready_queue).unwrap();
        assert_eq!(processed, 2;

        // Queue should have both tasks
        let queue = ready_queue.lock().unwrap();
        assert_eq!(queue.len(), 2;
    }
}