//! Platform-agnostic threading abstractions for WebAssembly execution.
//!
//! This module provides safe abstractions for mapping WebAssembly threads to
//! native platform threads with proper resource controls and isolation.

use core::{
    fmt::Debug,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::Duration,
};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use wrt_error::{codes, Error, ErrorCategory, Result};

/// Thread priority levels for platform-agnostic use
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadPriority {
    /// Lowest priority
    Idle,
    /// Below normal priority
    Low,
    /// Normal priority (default)
    Normal,
    /// Above normal priority
    High,
    /// Highest priority
    Realtime,
}

impl ThreadPriority {
    /// Convert to platform-specific priority value
    pub fn to_platform_priority(&self) -> i32 {
        match self {
            Self::Idle => -20,
            Self::Low => -10,
            Self::Normal => 0,
            Self::High => 10,
            Self::Realtime => 20,
        }
    }
}

/// CPU affinity mask
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuSet {
    /// Bitmask of allowed CPUs
    mask: u64,
}

impl CpuSet {
    /// Create empty CPU set
    pub fn new() -> Self {
        Self { mask: 0 }
    }

    /// Create CPU set with all CPUs
    pub fn all() -> Self {
        Self { mask: !0 }
    }

    /// Add CPU to set
    pub fn add(&mut self, cpu: usize) {
        if cpu < 64 {
            self.mask |= 1 << cpu;
        }
    }

    /// Remove CPU from set
    pub fn remove(&mut self, cpu: usize) {
        if cpu < 64 {
            self.mask &= !(1 << cpu);
        }
    }

    /// Check if CPU is in set
    pub fn contains(&self, cpu: usize) -> bool {
        cpu < 64 && (self.mask & (1 << cpu)) != 0
    }

    /// Get raw mask
    pub fn as_mask(&self) -> u64 {
        self.mask
    }
}

/// Thread pool configuration
#[derive(Debug, Clone)]
pub struct ThreadPoolConfig {
    /// Maximum concurrent threads
    pub max_threads: usize,
    /// Thread priority range (min, max)
    pub priority_range: (ThreadPriority, ThreadPriority),
    /// CPU affinity mask for all threads
    pub cpu_affinity: Option<CpuSet>,
    /// Memory limit per thread in bytes
    pub memory_limit_per_thread: Option<usize>,
    /// Stack size per thread in bytes
    pub stack_size: usize,
    /// Maximum thread lifetime
    pub max_thread_lifetime: Option<Duration>,
    /// Thread name prefix
    pub name_prefix: &'static str,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            max_threads: 16,
            priority_range: (ThreadPriority::Low, ThreadPriority::High),
            cpu_affinity: None,
            memory_limit_per_thread: Some(64 * 1024 * 1024), // 64MB default
            stack_size: 2 * 1024 * 1024, // 2MB default
            max_thread_lifetime: Some(Duration::from_secs(300)), // 5 minutes
            name_prefix: "wasm-thread",
        }
    }
}

/// Thread pool statistics
#[derive(Debug, Clone, Default)]
pub struct ThreadPoolStats {
    /// Currently active threads
    pub active_threads: usize,
    /// Total threads spawned
    pub total_spawned: u64,
    /// Total threads completed
    pub total_completed: u64,
    /// Total threads failed
    pub total_failed: u64,
    /// Current memory usage
    pub memory_usage: usize,
    /// Peak memory usage
    pub peak_memory_usage: usize,
}

/// WebAssembly task to execute
#[derive(Debug)]
pub struct WasmTask {
    /// Unique task ID
    pub id: u64,
    /// Function ID to execute
    pub function_id: u32,
    /// Arguments to pass
    pub args: Vec<u8>,
    /// Requested priority
    pub priority: ThreadPriority,
    /// Stack size override
    pub stack_size: Option<usize>,
    /// Memory limit override
    pub memory_limit: Option<usize>,
    /// CPU affinity override
    pub cpu_affinity: Option<CpuSet>,
    /// Execution deadline
    pub deadline: Option<Duration>,
}

/// Thread handle for tracking execution
pub struct ThreadHandle {
    /// Thread ID
    id: u64,
    /// Platform-specific handle
    platform_handle: Box<dyn PlatformThreadHandle>,
}

impl ThreadHandle {
    /// Get thread ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Join thread and get result
    pub fn join(self) -> Result<Vec<u8>> {
        self.platform_handle.join()
    }

    /// Check if thread is still running
    pub fn is_running(&self) -> bool {
        self.platform_handle.is_running()
    }
}

/// Platform-specific thread handle trait
pub trait PlatformThreadHandle: Send + Sync {
    /// Join thread and get result
    fn join(self: Box<Self>) -> Result<Vec<u8>>;
    
    /// Check if thread is still running
    fn is_running(&self) -> bool;
    
    /// Get thread statistics
    fn get_stats(&self) -> Result<ThreadStats>;
}

/// Per-thread statistics
#[derive(Debug, Clone, Default)]
pub struct ThreadStats {
    /// CPU time used
    pub cpu_time: Duration,
    /// Memory currently allocated
    pub memory_usage: usize,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Context switches
    pub context_switches: u64,
}

/// Platform-specific thread pool implementation
pub trait PlatformThreadPool: Send + Sync {
    /// Configure thread pool
    fn configure(&mut self, config: ThreadPoolConfig) -> Result<()>;
    
    /// Spawn WebAssembly thread with constraints
    fn spawn_wasm_thread(&self, task: WasmTask) -> Result<ThreadHandle>;
    
    /// Get thread pool statistics
    fn get_stats(&self) -> ThreadPoolStats;
    
    /// Shutdown thread pool gracefully
    fn shutdown(&mut self, timeout: Duration) -> Result<()>;
}

/// Resource limits for threading
#[derive(Debug, Clone)]
pub struct ThreadingLimits {
    /// Maximum threads per module
    pub max_threads_per_module: usize,
    /// Maximum total threads across all modules
    pub max_total_threads: usize,
    /// Maximum thread lifetime
    pub max_thread_lifetime: Duration,
    /// CPU time quota per thread
    pub cpu_quota_per_thread: Duration,
    /// Memory limit per module (all threads)
    pub memory_limit_per_module: usize,
}

impl Default for ThreadingLimits {
    fn default() -> Self {
        Self {
            max_threads_per_module: 32,
            max_total_threads: 256,
            max_thread_lifetime: Duration::from_secs(3600), // 1 hour
            cpu_quota_per_thread: Duration::from_secs(300), // 5 minutes CPU time
            memory_limit_per_module: 256 * 1024 * 1024, // 256MB
        }
    }
}

/// Thread spawn request
#[derive(Debug)]
pub struct ThreadSpawnRequest {
    /// Module ID requesting spawn
    pub module_id: u64,
    /// Function to execute
    pub function_id: u32,
    /// Arguments
    pub args: Vec<u8>,
    /// Requested priority
    pub priority: Option<ThreadPriority>,
    /// Stack size
    pub stack_size: Option<usize>,
}

/// Resource tracker for thread accounting
#[derive(Debug)]
pub struct ResourceTracker {
    /// Threads per module
    threads_per_module: Arc<parking_lot::RwLock<BTreeMap<u64, AtomicUsize>>>,
    /// Total active threads
    total_threads: AtomicUsize,
    /// Memory usage per module
    memory_per_module: Arc<parking_lot::RwLock<BTreeMap<u64, AtomicUsize>>>,
    /// Limits
    limits: ThreadingLimits,
}

impl ResourceTracker {
    /// Create new resource tracker
    pub fn new(limits: ThreadingLimits) -> Self {
        Self {
            threads_per_module: Arc::new(parking_lot::RwLock::new(BTreeMap::new())),
            total_threads: AtomicUsize::new(0),
            memory_per_module: Arc::new(parking_lot::RwLock::new(BTreeMap::new())),
            limits,
        }
    }

    /// Check if thread can be allocated
    pub fn can_allocate_thread(&self, request: &ThreadSpawnRequest) -> Result<bool> {
        // Check total thread limit
        let total = self.total_threads.load(Ordering::Acquire);
        if total >= self.limits.max_total_threads {
            return Ok(false);
        }

        // Check per-module limit
        let module_threads = {
            let modules = self.threads_per_module.read();
            modules
                .get(&request.module_id)
                .map(|count| count.load(Ordering::Acquire))
                .unwrap_or(0)
        };

        if module_threads >= self.limits.max_threads_per_module {
            return Ok(false);
        }

        // Check memory limit
        let module_memory = {
            let memory = self.memory_per_module.read();
            memory
                .get(&request.module_id)
                .map(|usage| usage.load(Ordering::Acquire))
                .unwrap_or(0)
        };

        let stack_size = request.stack_size.unwrap_or(2 * 1024 * 1024);
        if module_memory + stack_size > self.limits.memory_limit_per_module {
            return Ok(false);
        }

        Ok(true)
    }

    /// Allocate thread resources
    pub fn allocate_thread(&self, module_id: u64, stack_size: usize) -> Result<()> {
        // Increment total threads
        self.total_threads.fetch_add(1, Ordering::AcqRel);

        // Increment module threads
        {
            let mut modules = self.threads_per_module.write();
            modules
                .entry(module_id)
                .or_insert_with(|| AtomicUsize::new(0))
                .fetch_add(1, Ordering::AcqRel);
        }

        // Add memory usage
        {
            let mut memory = self.memory_per_module.write();
            memory
                .entry(module_id)
                .or_insert_with(|| AtomicUsize::new(0))
                .fetch_add(stack_size, Ordering::AcqRel);
        }

        Ok(())
    }

    /// Release thread resources
    pub fn release_thread(&self, module_id: u64, stack_size: usize) {
        // Decrement total threads
        self.total_threads.fetch_sub(1, Ordering::AcqRel);

        // Decrement module threads
        {
            let modules = self.threads_per_module.read();
            if let Some(count) = modules.get(&module_id) {
                count.fetch_sub(1, Ordering::AcqRel);
            }
        }

        // Subtract memory usage
        {
            let memory = self.memory_per_module.read();
            if let Some(usage) = memory.get(&module_id) {
                usage.fetch_sub(stack_size, Ordering::AcqRel);
            }
        }
    }
}

/// Create platform-specific thread pool
pub fn create_thread_pool(config: ThreadPoolConfig) -> Result<Box<dyn PlatformThreadPool>> {
    #[cfg(target_os = "nto")]
    {
        super::qnx_threading::QnxThreadPool::new(config)
            .map(|pool| Box::new(pool) as Box<dyn PlatformThreadPool>)
    }
    
    #[cfg(target_os = "linux")]
    {
        super::linux_threading::LinuxThreadPool::new(config)
            .map(|pool| Box::new(pool) as Box<dyn PlatformThreadPool>)
    }
    
    #[cfg(all(not(target_os = "nto"), not(target_os = "linux")))]
    {
        super::generic_threading::GenericThreadPool::new(config)
            .map(|pool| Box::new(pool) as Box<dyn PlatformThreadPool>)
    }
}

/// Thread execution monitor for safety
pub trait ExecutionMonitor: Send + Sync {
    /// Track new thread
    fn track_thread(&self, handle: &ThreadHandle) -> Result<()>;
    
    /// Check thread health
    fn check_thread_health(&self, id: u64) -> Result<ThreadHealth>;
    
    /// Kill thread if unhealthy
    fn kill_unhealthy_thread(&self, id: u64) -> Result<()>;
}

/// Thread health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadHealth {
    /// Thread is healthy
    Healthy,
    /// Thread is using too much CPU
    CpuQuotaExceeded,
    /// Thread lifetime exceeded
    LifetimeExceeded,
    /// Thread is deadlocked
    Deadlocked,
    /// Thread is not responding
    Unresponsive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_set() {
        let mut cpu_set = CpuSet::new();
        assert!(!cpu_set.contains(0));
        
        cpu_set.add(0);
        cpu_set.add(3);
        assert!(cpu_set.contains(0));
        assert!(cpu_set.contains(3));
        assert!(!cpu_set.contains(1));
        
        cpu_set.remove(0);
        assert!(!cpu_set.contains(0));
        assert!(cpu_set.contains(3));
    }

    #[test]
    fn test_resource_tracker() {
        let limits = ThreadingLimits {
            max_threads_per_module: 2,
            max_total_threads: 4,
            ..Default::default()
        };
        
        let tracker = ResourceTracker::new(limits);
        
        // First thread should be allowed
        let request1 = ThreadSpawnRequest {
            module_id: 1,
            function_id: 0,
            args: vec![],
            priority: None,
            stack_size: Some(1024 * 1024),
        };
        assert!(tracker.can_allocate_thread(&request1).unwrap());
        tracker.allocate_thread(1, 1024 * 1024).unwrap();
        
        // Second thread for same module should be allowed
        assert!(tracker.can_allocate_thread(&request1).unwrap());
        tracker.allocate_thread(1, 1024 * 1024).unwrap();
        
        // Third thread for same module should be denied (per-module limit)
        assert!(!tracker.can_allocate_thread(&request1).unwrap());
        
        // Thread for different module should be allowed
        let request2 = ThreadSpawnRequest {
            module_id: 2,
            function_id: 0,
            args: vec![],
            priority: None,
            stack_size: Some(1024 * 1024),
        };
        assert!(tracker.can_allocate_thread(&request2).unwrap());
    }
}