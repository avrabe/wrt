//! WebAssembly Wait Queue Primitives
//!
//! This module implements the wait queue primitives from the WebAssembly
//! shared-everything-threads proposal, providing flexible synchronization
//! mechanisms beyond basic atomic wait/notify operations.
//!
//! # Safety
//!
//! This module uses unsafe code for CPU-specific pause instructions to optimize
//! busy-wait loops. All unsafe blocks are documented and platform-specific.

#![allow(unsafe_code)]

extern crate alloc;

use crate::prelude::{BoundedVec, Debug, Eq, PartialEq};
use crate::thread_manager::{ThreadId, ThreadState};
use wrt_error::{Error, ErrorCategory, Result, codes};

// Import platform abstractions
use core::time::Duration;

#[cfg(feature = "std")]
use std::{sync::{Arc, Mutex, Condvar}, time::Instant};
#[cfg(not(feature = "std"))]
use crate::prelude::{Arc, Mutex};

// Platform-specific Condvar - only available with platform-sync
#[cfg(all(not(feature = "std"), feature = "platform-sync"))]
use wrt_platform::sync::Condvar;

// Stub timeout result for no_std
#[cfg(all(not(feature = "std"), not(feature = "platform-sync")))]
struct WaitTimeoutResult(bool);

#[cfg(all(not(feature = "std"), not(feature = "platform-sync")))]
impl WaitTimeoutResult {
    fn timed_out(&self) -> bool {
        self.0
    }
}

// Stub Condvar for no_std without platform-sync
#[cfg(all(not(feature = "std"), not(feature = "platform-sync")))]
struct Condvar;

#[cfg(all(not(feature = "std"), not(feature = "platform-sync")))]
impl Condvar {
    fn new() -> Self { Self }
    
    fn wait<'a, T>(&self, _guard: crate::prelude::MutexGuard<'a, T>) -> Result<crate::prelude::MutexGuard<'a, T>> {
        // For safety-critical systems without platform sync, we return immediately
        // indicating that blocking operations are not supported.
        // The runtime should handle this by using polling or other non-blocking mechanisms.
        Err(Error::not_supported_would_block("Blocking wait not supported in no_std without platform-sync - use polling"))
    }
    
    fn wait_timeout<'a, T>(&self, guard: crate::prelude::MutexGuard<'a, T>, _timeout: Duration) 
        -> Result<(crate::prelude::MutexGuard<'a, T>, WaitTimeoutResult)> {
        // Timeout-based wait also not supported without platform sync
        // Return immediately with timeout indication
        Ok((guard, WaitTimeoutResult(true)))
    }
    
    fn notify_one(&self) -> Result<()> {
        // Notification is a no-op when there's no actual blocking
        // In a real system, this would set a flag that waiters could poll
        Ok(())
    }
    
    fn notify_all(&self) -> Result<()> {
        // Notification is a no-op when there's no actual blocking
        // In a real system, this would set a flag that waiters could poll
        Ok(())
    }
}

use crate::bounded_runtime_infra::{
    BoundedWaitQueueVec, RuntimeProvider, new_thread_vec
};
#[cfg(not(feature = "std"))]
use wrt_foundation::traits::BoundedCapacity;

/// Wait queue identifier
pub type WaitQueueId = u64;

/// Result of a wait operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// Wait completed successfully (woken by notify)
    Ok = 0,
    /// Wait timed out
    TimedOut = 1,
    /// Wait was interrupted
    Interrupted = 2,
}

/// Wait queue entry containing thread information
#[derive(Debug, Clone)]
struct WaitQueueEntry {
    /// Thread waiting in the queue
    thread_id: ThreadId,
    /// Timestamp when thread entered the queue
    #[cfg(feature = "std")]
    enqueue_time: Instant,
    #[cfg(not(feature = "std"))]
    enqueue_time: u64, // Simplified timestamp
    /// Optional timeout for this wait
    timeout: Option<Duration>,
    /// Priority for wake-up ordering
    priority: u8,
}

/// Wait queue for thread synchronization
#[derive(Debug)]
pub struct WaitQueue {
    /// Queue identifier
    id: WaitQueueId,
    /// Threads waiting in this queue
    #[cfg(feature = "std")]
    waiters: Vec<WaitQueueEntry>,
    #[cfg(not(feature = "std"))]
    waiters: [Option<WaitQueueEntry>; 64], // Fixed size for no_std
    /// Queue statistics
    stats: WaitQueueStats,
    /// Synchronization primitives
    #[cfg(feature = "std")]
    condvar: Arc<Condvar>,
    #[cfg(feature = "std")]
    mutex: Arc<Mutex<()>>,
}

impl WaitQueue {
    /// Create new wait queue
    #[must_use] pub fn new(id: WaitQueueId) -> Self {
        Self {
            id,
            #[cfg(feature = "std")]
            waiters: Vec::new(),
            #[cfg(not(feature = "std"))]
            waiters: [const { None }; 64],
            stats: WaitQueueStats::new(),
            #[cfg(feature = "std")]
            condvar: Arc::new(Condvar::new()),
            #[cfg(feature = "std")]
            mutex: Arc::new(Mutex::new(())),
        }
    }
    
    /// Add thread to wait queue
    pub fn enqueue_waiter(
        &mut self,
        thread_id: ThreadId,
        timeout: Option<Duration>,
        priority: u8,
    ) -> Result<()> {
        let entry = WaitQueueEntry {
            thread_id,
            #[cfg(feature = "std")]
            enqueue_time: Instant::now(),
            #[cfg(not(feature = "std"))]
            enqueue_time: 0, // Time tracking not available without platform support
            timeout,
            priority,
        };
        
        #[cfg(feature = "std")]
        {
            // Insert in priority order (higher priority first)
            let insert_pos = self.waiters
                .binary_search_by(|existing| existing.priority.cmp(&entry.priority).reverse())
                .unwrap_or_else(|pos| pos);
            
            self.waiters.insert(insert_pos, entry);
            self.stats.total_waits += 1;
            self.stats.current_waiters = self.waiters.len() as u32;
            Ok(())
        }
        #[cfg(not(feature = "std"))]
        {
            // Find empty slot with priority consideration
            let mut insert_index = None;
            for (i, slot) in self.waiters.iter().enumerate() {
                if slot.is_none() {
                    insert_index = Some(i);
                    break;
                }
            }
            
            if let Some(index) = insert_index {
                self.waiters[index] = Some(entry);
                self.stats.total_waits += 1;
                self.stats.current_waiters += 1;
                Ok(())
            } else {
                Err(Error::resource_exhausted("Wait queue is full"))
            }
        }
    }
    
    /// Remove and return the next waiter to wake up
    pub fn dequeue_waiter(&mut self) -> Option<ThreadId> {
        #[cfg(feature = "std")]
        {
            if let Some(entry) = self.waiters.pop() {
                self.stats.current_waiters = self.waiters.len() as u32;
                Some(entry.thread_id)
            } else {
                None
            }
        }
        #[cfg(not(feature = "std"))]
        {
            // Find highest priority waiter
            let mut best_index = None;
            let mut best_priority = 0u8;
            
            for (i, slot) in self.waiters.iter().enumerate() {
                if let Some(entry) = slot {
                    if entry.priority >= best_priority {
                        best_priority = entry.priority;
                        best_index = Some(i);
                    }
                }
            }
            
            if let Some(index) = best_index {
                let entry = self.waiters[index].take().unwrap();
                self.stats.current_waiters -= 1;
                Some(entry.thread_id)
            } else {
                None
            }
        }
    }
    
    /// Remove specific thread from queue
    pub fn remove_waiter(&mut self, thread_id: ThreadId) -> bool {
        #[cfg(feature = "std")]
        {
            if let Some(pos) = self.waiters.iter().position(|entry| entry.thread_id == thread_id) {
                self.waiters.remove(pos);
                self.stats.current_waiters = self.waiters.len() as u32;
                true
            } else {
                false
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for slot in &mut self.waiters {
                if let Some(entry) = slot {
                    if entry.thread_id == thread_id {
                        *slot = None;
                        self.stats.current_waiters -= 1;
                        return true;
                    }
                }
            }
            false
        }
    }
    
    /// Check for expired timeouts and remove them
    pub fn process_timeouts(&mut self) -> Result<Vec<ThreadId>> {
        #[cfg(feature = "std")]
        let mut timed_out = std::vec::Vec::new();
        #[cfg(all(not(feature = "std"), not(feature = "std")))]
        let provider = wrt_foundation::safe_managed_alloc!(1024, wrt_foundation::budget_aware_provider::CrateId::Runtime)?;
        let mut timed_out: wrt_foundation::bounded::BoundedVec<u32, 256, wrt_foundation::safe_memory::NoStdProvider<1024>> = wrt_foundation::bounded::BoundedVec::new(provider)?;
        
        #[cfg(feature = "std")]
        {
            let now = Instant::now();
            self.waiters.retain(|entry| {
                if let Some(timeout) = entry.timeout {
                    if now.duration_since(entry.enqueue_time) >= timeout {
                        let _ = timed_out.push(entry.thread_id);
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            });
            self.stats.current_waiters = self.waiters.len() as u32;
        }
        #[cfg(not(feature = "std"))]
        {
            let now = wrt_platform::time::current_time_ns();
            for slot in &mut self.waiters {
                if let Some(entry) = slot {
                    if let Some(timeout) = entry.timeout {
                        let elapsed_ns = now.saturating_sub(entry.enqueue_time);
                        let timeout_ns = timeout.as_nanos() as u64;
                        
                        if elapsed_ns >= timeout_ns {
                            let _ = timed_out.push(entry.thread_id);
                            *slot = None;
                            self.stats.current_waiters -= 1;
                        }
                    }
                }
            }
        }
        
        self.stats.timeouts += timed_out.len() as u64;
        
        // Convert the result to the expected return type
        #[cfg(feature = "std")]
        return Ok(timed_out);
        #[cfg(all(not(feature = "std"), not(feature = "std")))]
        {
            // Convert BoundedVec to Vec (our type alias)
            let mut result = Vec::new();
            for item in &timed_out {
                result.push(*item);
            }
            Ok(result)
        }
    }
    
    /// Get number of waiting threads
    #[must_use] pub fn waiter_count(&self) -> u32 {
        self.stats.current_waiters
    }
    
    /// Get queue statistics
    #[must_use] pub fn stats(&self) -> &WaitQueueStats {
        &self.stats
    }
}

/// Wait queue manager for coordinating multiple queues
#[derive(Debug)]
pub struct WaitQueueManager {
    /// All active wait queues - using fixed array for now until WaitQueue implements required traits
    // TODO: Replace with BoundedVec once WaitQueue implements Checksummable, ToBytes, FromBytes
    queues: [(WaitQueueId, Option<WaitQueue>); 32],
    /// Next queue ID to assign
    next_queue_id: WaitQueueId,
    /// Global statistics
    pub global_stats: WaitQueueGlobalStats,
}

impl WaitQueueManager {
    /// Create new wait queue manager
    #[must_use] pub fn new() -> Self {
        Self {
            queues: Default::default(),
            next_queue_id: 1,
            global_stats: WaitQueueGlobalStats::new(),
        }
    }
    
    /// Create a new wait queue
    pub fn create_queue(&mut self) -> WaitQueueId {
        let queue_id = self.next_queue_id;
        self.next_queue_id += 1;
        
        let queue = WaitQueue::new(queue_id);
        
        #[cfg(feature = "std")]
        {
            // Find empty slot in array
            for i in 0..self.queues.len() {
                if self.queues[i].1.is_none() {
                    self.queues[i] = (queue_id, Some(queue));
                    break;
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            // Find empty slot
            for (id, slot) in &mut self.queues {
                if slot.is_none() {
                    *id = queue_id;
                    *slot = Some(queue);
                    break;
                }
            }
        }
        
        self.global_stats.active_queues += 1;
        queue_id
    }
    
    /// Wait on a queue with optional timeout
    /// Implements: `waitqueue.wait(queue_id: u64, timeout: option<u64>) -> wait-result`
    pub fn waitqueue_wait(
        &mut self,
        queue_id: WaitQueueId,
        thread_id: ThreadId,
        timeout_ms: Option<u64>,
        priority: u8,
    ) -> Result<WaitResult> {
        let timeout = timeout_ms.map(Duration::from_millis);
        
        // Get queue
        let queue = self.get_queue_mut(queue_id)?;
        
        // Add thread to wait queue
        queue.enqueue_waiter(thread_id, timeout, priority)?;
        
        #[cfg(feature = "std")]
        {
            // Use platform synchronization
            let guard = queue.mutex.lock().unwrap();
            let result = if let Some(timeout) = timeout {
                match queue.condvar.wait_timeout(guard, timeout) {
                    Ok((_guard, timeout_result)) => {
                        if timeout_result.timed_out() {
                            WaitResult::TimedOut
                        } else {
                            WaitResult::Ok
                        }
                    },
                    Err(_) => WaitResult::Interrupted,
                }
            } else {
                match queue.condvar.wait(guard) {
                    Ok(_) => WaitResult::Ok,
                    Err(_) => WaitResult::Interrupted,
                }
            };
            
            // Remove from queue if still there
            queue.remove_waiter(thread_id);
            Ok(result)
        }
        #[cfg(not(feature = "std"))]
        {
            // In no_std, we simulate waiting by returning immediately
            // Real implementations would integrate with the scheduler
            Ok(WaitResult::Ok)
        }
    }
    
    /// Notify waiters in a queue
    /// Implements: `waitqueue.notify(queue_id: u64, count: u32) -> u32`
    pub fn waitqueue_notify(&mut self, queue_id: WaitQueueId, count: u32) -> Result<u32> {
        let queue = self.get_queue_mut(queue_id)?;
        let mut notified = 0u32;
        
        #[cfg(feature = "std")]
        {
            // Wake up the specified number of threads
            for _ in 0..count {
                if queue.dequeue_waiter().is_some() {
                    notified += 1;
                    queue.condvar.notify_one();
                } else {
                    break;
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            // In no_std, just count how many we would notify
            for _ in 0..count {
                if queue.dequeue_waiter().is_some() {
                    notified += 1;
                } else {
                    break;
                }
            }
        }
        
        self.global_stats.total_notifies += 1;
        self.global_stats.total_threads_notified += u64::from(notified);
        
        Ok(notified)
    }
    
    /// Destroy a wait queue
    pub fn destroy_queue(&mut self, queue_id: WaitQueueId) -> Result<()> {
        #[cfg(feature = "std")]
        {
            let mut found = false;
            for i in 0..self.queues.len() {
                if self.queues[i].0 == queue_id && self.queues[i].1.is_some() {
                    self.queues[i].1 = None;
                    found = true;
                    break;
                }
            }
            if found {
                self.global_stats.active_queues -= 1;
                Ok(())
            } else {
                Err(Error::validation_invalid_argument("Wait queue not found"))
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (id, slot) in &mut self.queues {
                if *id == queue_id && slot.is_some() {
                    *slot = None;
                    *id = 0;
                    self.global_stats.active_queues -= 1;
                    return Ok(());
                }
            }
            
            Err(Error::validation_invalid_argument("Wait queue not found"))
        }
    }
    
    /// Process timeouts for all queues
    pub fn process_all_timeouts(&mut self) -> Result<u64> {
        let mut total_timeouts = 0u64;
        
        #[cfg(feature = "std")]
        {
            for i in 0..self.queues.len() {
                if let Some(ref mut queue) = self.queues[i].1 {
                    let timed_out = queue.process_timeouts()?;
                    total_timeouts += timed_out.len() as u64;
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for (_id, slot) in &mut self.queues {
                if let Some(queue) = slot {
                    let timed_out = queue.process_timeouts()?;
                    total_timeouts += timed_out.len() as u64;
                }
            }
        }
        
        self.global_stats.total_timeouts += total_timeouts;
        Ok(total_timeouts)
    }
    
    // Private helper methods
    
    fn get_queue_mut(&mut self, queue_id: WaitQueueId) -> Result<&mut WaitQueue> {
        #[cfg(feature = "std")]
        {
            for i in 0..self.queues.len() {
                if self.queues[i].0 == queue_id {
                    if let Some(ref mut queue) = self.queues[i].1 {
                        return Ok(queue);
                    }
                }
            }
            Err(Error::validation_invalid_argument("Wait queue not found"))
        }
        #[cfg(not(feature = "std"))]
        {
            for (id, slot) in &mut self.queues {
                if *id == queue_id {
                    if let Some(queue) = slot {
                        return Ok(queue);
                    }
                }
            }
            
            Err(Error::validation_invalid_argument("Wait queue not found"))
        }
    }
}

impl Default for WaitQueueManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for individual wait queue
#[derive(Debug, Clone)]
pub struct WaitQueueStats {
    /// Total number of wait operations
    pub total_waits: u64,
    /// Current number of waiting threads
    pub current_waiters: u32,
    /// Number of timeout events
    pub timeouts: u64,
    /// Average wait time in nanoseconds
    pub average_wait_time: u64,
}

impl WaitQueueStats {
    fn new() -> Self {
        Self {
            total_waits: 0,
            current_waiters: 0,
            timeouts: 0,
            average_wait_time: 0,
        }
    }
}

/// Global statistics for wait queue manager
#[derive(Debug, Clone)]
pub struct WaitQueueGlobalStats {
    /// Number of active wait queues
    pub active_queues: u32,
    /// Total notify operations
    pub total_notifies: u64,
    /// Total threads notified
    pub total_threads_notified: u64,
    /// Total timeout events across all queues
    pub total_timeouts: u64,
}

impl WaitQueueGlobalStats {
    fn new() -> Self {
        Self {
            active_queues: 0,
            total_notifies: 0,
            total_threads_notified: 0,
            total_timeouts: 0,
        }
    }
    
    /// Get average threads notified per notify operation
    #[must_use] pub fn average_threads_per_notify(&self) -> f64 {
        if self.total_notifies == 0 {
            0.0
        } else {
            self.total_threads_notified as f64 / self.total_notifies as f64
        }
    }
}

/// Pause instruction for spinlock relaxation
/// Implements: `pause() -> ()`
pub fn pause() {
    #[cfg(feature = "std")]
    {
        // Use CPU pause instruction if available
        #[cfg(target_arch = "x86_64")]
        // SAFETY: _mm_pause is a safe CPU instruction with no side effects
        unsafe {
            core::arch::x86_64::_mm_pause();
        }
        // ARM yield instruction requires unstable features, disabled for now
        // #[cfg(target_arch = "aarch64")]
        // unsafe {
        //     core::arch::aarch64::__yield();
        // }
        #[cfg(not(target_arch = "x86_64"))]
        {
            std::thread::yield_now();
        }
    }
    #[cfg(not(feature = "std"))]
    {
        // In no_std, pause is a no-op or could use platform-specific hints
        // Real embedded implementations might use WFI or similar
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wait_queue_creation() {
        let mut manager = WaitQueueManager::new();
        let queue_id = manager.create_queue();
        
        assert_eq!(queue_id, 1);
        assert_eq!(manager.global_stats.active_queues, 1);
    }
    
    #[test]
    fn test_wait_queue_basic_operations() {
        let mut queue = WaitQueue::new(1);
        
        // Test enqueue
        queue.enqueue_waiter(10, None, 50).unwrap();
        assert_eq!(queue.waiter_count(), 1);
        
        // Test dequeue
        let thread_id = queue.dequeue_waiter();
        assert_eq!(thread_id, Some(10));
        assert_eq!(queue.waiter_count(), 0);
    }
    
    #[test]
    fn test_wait_queue_priority_ordering() {
        let mut queue = WaitQueue::new(1);
        
        // Add threads with different priorities
        queue.enqueue_waiter(1, None, 30).unwrap(); // Lower priority
        queue.enqueue_waiter(2, None, 80).unwrap(); // Higher priority
        queue.enqueue_waiter(3, None, 50).unwrap(); // Medium priority
        
        // Higher priority should come out first
        #[cfg(feature = "std")]
        {
            assert_eq!(queue.dequeue_waiter(), Some(2)); // Highest priority (80)
            assert_eq!(queue.dequeue_waiter(), Some(3)); // Medium priority (50)
            assert_eq!(queue.dequeue_waiter(), Some(1)); // Lowest priority (30)
        }
    }
    
    #[test]
    fn test_wait_result_values() {
        assert_eq!(WaitResult::Ok as u32, 0);
        assert_eq!(WaitResult::TimedOut as u32, 1);
        assert_eq!(WaitResult::Interrupted as u32, 2);
    }
    
    #[test]
    fn test_pause_instruction() {
        // Should not panic
        pause();
    }
    
    #[cfg(feature = "std")]
    #[test]
    fn test_wait_queue_manager_operations() {
        let mut manager = WaitQueueManager::new();
        
        let queue_id = manager.create_queue();
        
        // Test notify on empty queue
        let notified = manager.waitqueue_notify(queue_id, 5).unwrap();
        assert_eq!(notified, 0);
        
        // Test destroy queue
        manager.destroy_queue(queue_id).unwrap();
        assert_eq!(manager.global_stats.active_queues, 0);
    }
}