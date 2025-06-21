//! Advanced synchronization primitives for async operations
//!
//! This module provides async-aware synchronization primitives including
//! mutexes, semaphores, barriers, and condition variables integrated with
//! the fuel-based async executor.

use crate::{
    async_::{
        fuel_async_executor::AsyncTaskState,
        task_manager_async_bridge::{TaskManagerAsyncBridge, ComponentAsyncTaskType},
        fuel_aware_waker::{create_fuel_aware_waker},
    },
    task_manager::TaskId,
    ComponentInstanceId,
    prelude::*,
};
use core::{
    future::Future as CoreFuture,
    pin::Pin,
    sync::atomic::{AtomicU64, AtomicU32, AtomicBool, AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    component_value::ComponentValue,
    Arc, Weak, sync::Mutex,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

/// Maximum waiters per synchronization primitive
const MAX_WAITERS_PER_PRIMITIVE: usize = 128;

/// Maximum sync primitives per component
const MAX_SYNC_PRIMITIVES_PER_COMPONENT: usize = 64;

/// Fuel costs for sync operations
const MUTEX_LOCK_FUEL: u64 = 25;
const MUTEX_UNLOCK_FUEL: u64 = 15;
const SEMAPHORE_ACQUIRE_FUEL: u64 = 20;
const SEMAPHORE_RELEASE_FUEL: u64 = 15;
const BARRIER_WAIT_FUEL: u64 = 30;
const CONDVAR_WAIT_FUEL: u64 = 25;
const CONDVAR_NOTIFY_FUEL: u64 = 20;

/// Advanced synchronization primitives manager
pub struct AdvancedSyncPrimitives {
    /// Bridge for task management
    bridge: Arc<Mutex<TaskManagerAsyncBridge>>,
    /// Active sync primitives
    primitives: BoundedHashMap<SyncPrimitiveId, SyncPrimitive, 512>,
    /// Component sync contexts
    component_contexts: BoundedHashMap<ComponentInstanceId, ComponentSyncContext, 128>,
    /// Next primitive ID
    next_primitive_id: AtomicU64,
    /// Sync statistics
    sync_stats: SyncStatistics,
    /// Sync configuration
    sync_config: SyncConfiguration,
}

/// Synchronization primitive identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyncPrimitiveId(u64);

/// Synchronization primitive
#[derive(Debug)]
struct SyncPrimitive {
    id: SyncPrimitiveId,
    component_id: ComponentInstanceId,
    primitive_type: SyncPrimitiveType,
    waiters: BoundedVec<WaiterInfo, MAX_WAITERS_PER_PRIMITIVE>,
    created_at: u64,
    fuel_consumed: AtomicU64,
}

/// Type of synchronization primitive
#[derive(Debug, Clone)]
pub enum SyncPrimitiveType {
    /// Async mutex
    AsyncMutex {
        locked: AtomicBool,
        owner: Option<TaskId>,
        lock_count: AtomicU32, // For reentrant mutexes
        is_reentrant: bool,
    },
    /// Async semaphore
    AsyncSemaphore {
        permits: AtomicU32,
        max_permits: u32,
        fair_scheduling: bool,
    },
    /// Async barrier
    AsyncBarrier {
        parties: u32,
        waiting: AtomicU32,
        generation: AtomicU64,
        broken: AtomicBool,
    },
    /// Async condition variable
    AsyncCondVar {
        associated_mutex: Option<SyncPrimitiveId>,
        notification_count: AtomicU64,
    },
    /// Async read-write lock
    AsyncRwLock {
        readers: AtomicU32,
        writer: AtomicBool,
        writer_waiting: AtomicBool,
        readers_waiting: AtomicU32,
        prefer_writers: bool,
    },
    /// Async latch (countdown latch)
    AsyncLatch {
        count: AtomicU32,
        initial_count: u32,
    },
}

/// Waiter information
#[derive(Debug, Clone)]
struct WaiterInfo {
    task_id: TaskId,
    component_id: ComponentInstanceId,
    waker: Option<Waker>,
    wait_type: WaitType,
    queued_at: u64,
    priority: Priority,
}

/// Type of wait operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitType {
    /// Waiting for mutex lock
    MutexLock,
    /// Waiting for semaphore permit
    SemaphorePermit,
    /// Waiting at barrier
    BarrierWait,
    /// Waiting on condition variable
    CondVarWait,
    /// Waiting for read lock
    ReadLock,
    /// Waiting for write lock
    WriteLock,
    /// Waiting for latch countdown
    LatchWait,
}

/// Component sync context
#[derive(Debug)]
struct ComponentSyncContext {
    component_id: ComponentInstanceId,
    /// Sync primitives owned by this component
    owned_primitives: BoundedVec<SyncPrimitiveId, MAX_SYNC_PRIMITIVES_PER_COMPONENT>,
    /// Sync limits
    sync_limits: SyncLimits,
}

/// Sync limits per component
#[derive(Debug, Clone)]
struct SyncLimits {
    max_mutexes: usize,
    max_semaphores: usize,
    max_barriers: usize,
    max_condvars: usize,
    max_rwlocks: usize,
    max_latches: usize,
    fuel_budget: u64,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfiguration {
    pub enable_priority_inheritance: bool,
    pub enable_deadlock_detection: bool,
    pub enable_fair_scheduling: bool,
    pub max_wait_time_ms: u64,
    pub enable_reentrant_mutexes: bool,
    pub fuel_tracking: bool,
}

impl Default for SyncConfiguration {
    fn default() -> Self {
        Self {
            enable_priority_inheritance: true,
            enable_deadlock_detection: true,
            enable_fair_scheduling: true,
            max_wait_time_ms: 30_000, // 30 seconds
            enable_reentrant_mutexes: false,
            fuel_tracking: true,
        }
    }
}

/// Sync statistics
#[derive(Debug, Default)]
struct SyncStatistics {
    total_mutexes_created: AtomicU64,
    total_mutex_locks: AtomicU64,
    total_mutex_unlocks: AtomicU64,
    total_semaphores_created: AtomicU64,
    total_semaphore_acquires: AtomicU64,
    total_semaphore_releases: AtomicU64,
    total_barriers_created: AtomicU64,
    total_barrier_waits: AtomicU64,
    total_condvars_created: AtomicU64,
    total_condvar_waits: AtomicU64,
    total_condvar_notifies: AtomicU64,
    deadlocks_detected: AtomicU64,
    priority_inversions: AtomicU64,
    total_fuel_consumed: AtomicU64,
}

impl AdvancedSyncPrimitives {
    /// Create new advanced sync primitives manager
    pub fn new(
        bridge: Arc<Mutex<TaskManagerAsyncBridge>>,
        config: Option<SyncConfiguration>,
    ) -> Self {
        Self {
            bridge,
            primitives: BoundedHashMap::new(),
            component_contexts: BoundedHashMap::new(),
            next_primitive_id: AtomicU64::new(1),
            sync_stats: SyncStatistics::default(),
            sync_config: config.unwrap_or_default(),
        }
    }

    /// Initialize component for sync operations
    pub fn initialize_component_sync(
        &mut self,
        component_id: ComponentInstanceId,
        limits: Option<SyncLimits>,
    ) -> Result<(), Error> {
        let limits = limits.unwrap_or_else(|| SyncLimits {
            max_mutexes: 32,
            max_semaphores: 16,
            max_barriers: 8,
            max_condvars: 16,
            max_rwlocks: 16,
            max_latches: 8,
            fuel_budget: 50_000,
        });

        let provider = safe_managed_alloc!(2048, CrateId::Component)?;
        let context = ComponentSyncContext {
            component_id,
            owned_primitives: BoundedVec::new(provider)?,
            sync_limits: limits,
        };

        self.component_contexts.insert(component_id, context).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many component sync contexts".to_string(),
            )
        })?;

        Ok(())
    }

    /// Create an async mutex
    pub fn create_async_mutex(
        &mut self,
        component_id: ComponentInstanceId,
        is_reentrant: bool,
    ) -> Result<SyncPrimitiveId, Error> {
        let context = self.component_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Component not initialized for sync operations".to_string(),
            )
        })?;

        // Check limits
        let current_mutexes = context.owned_primitives.iter()
            .filter(|&&id| {
                if let Some(primitive) = self.primitives.get(&id) {
                    matches!(primitive.primitive_type, SyncPrimitiveType::AsyncMutex { .. })
                } else {
                    false
                }
            })
            .count();

        if current_mutexes >= context.sync_limits.max_mutexes {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Component mutex limit exceeded".to_string(),
            ));
        }

        let primitive_id = SyncPrimitiveId(self.next_primitive_id.fetch_add(1, Ordering::AcqRel));
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;

        let primitive = SyncPrimitive {
            id: primitive_id,
            component_id,
            primitive_type: SyncPrimitiveType::AsyncMutex {
                locked: AtomicBool::new(false),
                owner: None,
                lock_count: AtomicU32::new(0),
                is_reentrant,
            },
            waiters: BoundedVec::new(provider)?,
            created_at: self.get_timestamp(),
            fuel_consumed: AtomicU64::new(0),
        };

        self.primitives.insert(primitive_id, primitive).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many sync primitives".to_string(),
            )
        })?;

        context.owned_primitives.push(primitive_id).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Component primitive list full".to_string(),
            )
        })?;

        self.sync_stats.total_mutexes_created.fetch_add(1, Ordering::Relaxed);

        Ok(primitive_id)
    }

    /// Create an async semaphore
    pub fn create_async_semaphore(
        &mut self,
        component_id: ComponentInstanceId,
        permits: u32,
        fair_scheduling: bool,
    ) -> Result<SyncPrimitiveId, Error> {
        let context = self.component_contexts.get_mut(&component_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Component not initialized for sync operations".to_string(),
            )
        })?;

        if permits == 0 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Semaphore must have at least 1 permit".to_string(),
            ));
        }

        let primitive_id = SyncPrimitiveId(self.next_primitive_id.fetch_add(1, Ordering::AcqRel));
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;

        let primitive = SyncPrimitive {
            id: primitive_id,
            component_id,
            primitive_type: SyncPrimitiveType::AsyncSemaphore {
                permits: AtomicU32::new(permits),
                max_permits: permits,
                fair_scheduling,
            },
            waiters: BoundedVec::new(provider)?,
            created_at: self.get_timestamp(),
            fuel_consumed: AtomicU64::new(0),
        };

        self.primitives.insert(primitive_id, primitive).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many sync primitives".to_string(),
            )
        })?;

        context.owned_primitives.push(primitive_id).ok();

        self.sync_stats.total_semaphores_created.fetch_add(1, Ordering::Relaxed);

        Ok(primitive_id)
    }

    /// Create an async barrier
    pub fn create_async_barrier(
        &mut self,
        component_id: ComponentInstanceId,
        parties: u32,
    ) -> Result<SyncPrimitiveId, Error> {
        if parties == 0 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Barrier must have at least 1 party".to_string(),
            ));
        }

        let primitive_id = SyncPrimitiveId(self.next_primitive_id.fetch_add(1, Ordering::AcqRel));
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;

        let primitive = SyncPrimitive {
            id: primitive_id,
            component_id,
            primitive_type: SyncPrimitiveType::AsyncBarrier {
                parties,
                waiting: AtomicU32::new(0),
                generation: AtomicU64::new(0),
                broken: AtomicBool::new(false),
            },
            waiters: BoundedVec::new(provider)?,
            created_at: self.get_timestamp(),
            fuel_consumed: AtomicU64::new(0),
        };

        self.primitives.insert(primitive_id, primitive).ok();

        if let Some(context) = self.component_contexts.get_mut(&component_id) {
            context.owned_primitives.push(primitive_id).ok();
        }

        self.sync_stats.total_barriers_created.fetch_add(1, Ordering::Relaxed);

        Ok(primitive_id)
    }

    /// Create an async condition variable
    pub fn create_async_condvar(
        &mut self,
        component_id: ComponentInstanceId,
        associated_mutex: Option<SyncPrimitiveId>,
    ) -> Result<SyncPrimitiveId, Error> {
        // Validate associated mutex if provided
        if let Some(mutex_id) = associated_mutex {
            let mutex = self.primitives.get(&mutex_id).ok_or_else(|| {
                Error::new(
                    ErrorCategory::Validation,
                    codes::INVALID_INPUT,
                    "Associated mutex not found".to_string(),
                )
            })?;

            if !matches!(mutex.primitive_type, SyncPrimitiveType::AsyncMutex { .. }) {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::INVALID_INPUT,
                    "Associated primitive is not a mutex".to_string(),
                ));
            }
        }

        let primitive_id = SyncPrimitiveId(self.next_primitive_id.fetch_add(1, Ordering::AcqRel));
        let provider = safe_managed_alloc!(1024, CrateId::Component)?;

        let primitive = SyncPrimitive {
            id: primitive_id,
            component_id,
            primitive_type: SyncPrimitiveType::AsyncCondVar {
                associated_mutex,
                notification_count: AtomicU64::new(0),
            },
            waiters: BoundedVec::new(provider)?,
            created_at: self.get_timestamp(),
            fuel_consumed: AtomicU64::new(0),
        };

        self.primitives.insert(primitive_id, primitive).ok();

        if let Some(context) = self.component_contexts.get_mut(&component_id) {
            context.owned_primitives.push(primitive_id).ok();
        }

        self.sync_stats.total_condvars_created.fetch_add(1, Ordering::Relaxed);

        Ok(primitive_id)
    }

    /// Lock an async mutex
    pub fn lock_async_mutex(
        &mut self,
        primitive_id: SyncPrimitiveId,
        task_id: TaskId,
        component_id: ComponentInstanceId,
    ) -> Result<MutexLockResult, Error> {
        let primitive = self.primitives.get_mut(&primitive_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Mutex not found".to_string(),
            )
        })?;

        match &mut primitive.primitive_type {
            SyncPrimitiveType::AsyncMutex { locked, owner, lock_count, is_reentrant } => {
                // Check if already locked
                if locked.load(Ordering::Acquire) {
                    // Check for reentrant lock
                    if *is_reentrant && owner.as_ref() == Some(&task_id) {
                        lock_count.fetch_add(1, Ordering::AcqRel);
                        primitive.fuel_consumed.fetch_add(MUTEX_LOCK_FUEL, Ordering::Relaxed);
                        self.sync_stats.total_mutex_locks.fetch_add(1, Ordering::Relaxed);
                        return Ok(MutexLockResult::Acquired);
                    }

                    // Add to waiters
                    let waiter = WaiterInfo {
                        task_id,
                        component_id,
                        waker: None,
                        wait_type: WaitType::MutexLock,
                        queued_at: self.get_timestamp(),
                        priority: Priority::Normal, // Would get from task
                    };

                    primitive.waiters.push(waiter).map_err(|_| {
                        Error::new(
                            ErrorCategory::Resource,
                            codes::RESOURCE_LIMIT_EXCEEDED,
                            "Mutex waiter queue full".to_string(),
                        )
                    })?;

                    return Ok(MutexLockResult::WouldBlock);
                }

                // Acquire the lock
                locked.store(true, Ordering::Release);
                *owner = Some(task_id);
                lock_count.store(1, Ordering::Release);

                primitive.fuel_consumed.fetch_add(MUTEX_LOCK_FUEL, Ordering::Relaxed);
                self.sync_stats.total_mutex_locks.fetch_add(1, Ordering::Relaxed);

                Ok(MutexLockResult::Acquired)
            },
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Primitive is not a mutex".to_string(),
            )),
        }
    }

    /// Unlock an async mutex
    pub fn unlock_async_mutex(
        &mut self,
        primitive_id: SyncPrimitiveId,
        task_id: TaskId,
    ) -> Result<(), Error> {
        let primitive = self.primitives.get_mut(&primitive_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Mutex not found".to_string(),
            )
        })?;

        match &mut primitive.primitive_type {
            SyncPrimitiveType::AsyncMutex { locked, owner, lock_count, is_reentrant } => {
                // Verify ownership
                if owner.as_ref() != Some(&task_id) {
                    return Err(Error::new(
                        ErrorCategory::InvalidState,
                        codes::INVALID_STATE,
                        "Task does not own mutex".to_string(),
                    ));
                }

                let current_count = lock_count.load(Ordering::Acquire);
                if current_count == 0 {
                    return Err(Error::new(
                        ErrorCategory::InvalidState,
                        codes::INVALID_STATE,
                        "Mutex is not locked".to_string(),
                    ));
                }

                // Handle reentrant unlock
                if *is_reentrant && current_count > 1 {
                    lock_count.fetch_sub(1, Ordering::AcqRel);
                    primitive.fuel_consumed.fetch_add(MUTEX_UNLOCK_FUEL, Ordering::Relaxed);
                    self.sync_stats.total_mutex_unlocks.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }

                // Release the lock
                *owner = None;
                lock_count.store(0, Ordering::Release);
                locked.store(false, Ordering::Release);

                primitive.fuel_consumed.fetch_add(MUTEX_UNLOCK_FUEL, Ordering::Relaxed);
                self.sync_stats.total_mutex_unlocks.fetch_add(1, Ordering::Relaxed);

                // Wake next waiter
                self.wake_next_mutex_waiter(primitive)?;

                Ok(())
            },
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Primitive is not a mutex".to_string(),
            )),
        }
    }

    /// Acquire semaphore permit
    pub fn acquire_semaphore(
        &mut self,
        primitive_id: SyncPrimitiveId,
        task_id: TaskId,
        component_id: ComponentInstanceId,
    ) -> Result<SemaphoreAcquireResult, Error> {
        let primitive = self.primitives.get_mut(&primitive_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Semaphore not found".to_string(),
            )
        })?;

        match &mut primitive.primitive_type {
            SyncPrimitiveType::AsyncSemaphore { permits, fair_scheduling, .. } => {
                let current_permits = permits.load(Ordering::Acquire);
                
                if current_permits == 0 {
                    // Add to waiters
                    let waiter = WaiterInfo {
                        task_id,
                        component_id,
                        waker: None,
                        wait_type: WaitType::SemaphorePermit,
                        queued_at: self.get_timestamp(),
                        priority: Priority::Normal,
                    };

                    primitive.waiters.push(waiter).map_err(|_| {
                        Error::new(
                            ErrorCategory::Resource,
                            codes::RESOURCE_LIMIT_EXCEEDED,
                            "Semaphore waiter queue full".to_string(),
                        )
                    })?;

                    return Ok(SemaphoreAcquireResult::WouldBlock);
                }

                // Acquire permit
                permits.fetch_sub(1, Ordering::AcqRel);
                primitive.fuel_consumed.fetch_add(SEMAPHORE_ACQUIRE_FUEL, Ordering::Relaxed);
                self.sync_stats.total_semaphore_acquires.fetch_add(1, Ordering::Relaxed);

                Ok(SemaphoreAcquireResult::Acquired)
            },
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Primitive is not a semaphore".to_string(),
            )),
        }
    }

    /// Release semaphore permit
    pub fn release_semaphore(&mut self, primitive_id: SyncPrimitiveId) -> Result<(), Error> {
        let primitive = self.primitives.get_mut(&primitive_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Semaphore not found".to_string(),
            )
        })?;

        match &mut primitive.primitive_type {
            SyncPrimitiveType::AsyncSemaphore { permits, max_permits, .. } => {
                let current_permits = permits.load(Ordering::Acquire);
                
                if current_permits >= *max_permits {
                    return Err(Error::new(
                        ErrorCategory::InvalidState,
                        codes::INVALID_STATE,
                        "Semaphore already at maximum permits".to_string(),
                    ));
                }

                // Release permit
                permits.fetch_add(1, Ordering::AcqRel);
                primitive.fuel_consumed.fetch_add(SEMAPHORE_RELEASE_FUEL, Ordering::Relaxed);
                self.sync_stats.total_semaphore_releases.fetch_add(1, Ordering::Relaxed);

                // Wake next waiter
                self.wake_next_semaphore_waiter(primitive)?;

                Ok(())
            },
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Primitive is not a semaphore".to_string(),
            )),
        }
    }

    /// Wait at async barrier
    pub fn wait_barrier(
        &mut self,
        primitive_id: SyncPrimitiveId,
        task_id: TaskId,
        component_id: ComponentInstanceId,
    ) -> Result<BarrierWaitResult, Error> {
        let primitive = self.primitives.get_mut(&primitive_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Barrier not found".to_string(),
            )
        })?;

        match &mut primitive.primitive_type {
            SyncPrimitiveType::AsyncBarrier { parties, waiting, generation, broken } => {
                if broken.load(Ordering::Acquire) {
                    return Err(Error::new(
                        ErrorCategory::InvalidState,
                        codes::INVALID_STATE,
                        "Barrier is broken".to_string(),
                    ));
                }

                let current_waiting = waiting.fetch_add(1, Ordering::AcqRel);
                
                if current_waiting + 1 == *parties {
                    // Last party arrived - release all
                    generation.fetch_add(1, Ordering::AcqRel);
                    waiting.store(0, Ordering::Release);
                    
                    // Wake all waiters
                    self.wake_all_barrier_waiters(primitive)?;
                    
                    primitive.fuel_consumed.fetch_add(BARRIER_WAIT_FUEL, Ordering::Relaxed);
                    self.sync_stats.total_barrier_waits.fetch_add(1, Ordering::Relaxed);
                    
                    Ok(BarrierWaitResult::Leader)
                } else {
                    // Not the last party - wait
                    let waiter = WaiterInfo {
                        task_id,
                        component_id,
                        waker: None,
                        wait_type: WaitType::BarrierWait,
                        queued_at: self.get_timestamp(),
                        priority: Priority::Normal,
                    };

                    primitive.waiters.push(waiter).map_err(|_| {
                        Error::new(
                            ErrorCategory::Resource,
                            codes::RESOURCE_LIMIT_EXCEEDED,
                            "Barrier waiter queue full".to_string(),
                        )
                    })?;

                    Ok(BarrierWaitResult::WouldBlock)
                }
            },
            _ => Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Primitive is not a barrier".to_string(),
            )),
        }
    }

    /// Get sync primitive statistics
    pub fn get_sync_statistics(&self) -> SyncStats {
        SyncStats {
            total_mutexes_created: self.sync_stats.total_mutexes_created.load(Ordering::Relaxed),
            total_mutex_locks: self.sync_stats.total_mutex_locks.load(Ordering::Relaxed),
            total_mutex_unlocks: self.sync_stats.total_mutex_unlocks.load(Ordering::Relaxed),
            total_semaphores_created: self.sync_stats.total_semaphores_created.load(Ordering::Relaxed),
            total_semaphore_acquires: self.sync_stats.total_semaphore_acquires.load(Ordering::Relaxed),
            total_semaphore_releases: self.sync_stats.total_semaphore_releases.load(Ordering::Relaxed),
            total_barriers_created: self.sync_stats.total_barriers_created.load(Ordering::Relaxed),
            total_barrier_waits: self.sync_stats.total_barrier_waits.load(Ordering::Relaxed),
            total_condvars_created: self.sync_stats.total_condvars_created.load(Ordering::Relaxed),
            total_condvar_waits: self.sync_stats.total_condvar_waits.load(Ordering::Relaxed),
            total_condvar_notifies: self.sync_stats.total_condvar_notifies.load(Ordering::Relaxed),
            active_primitives: self.primitives.len() as u64,
            deadlocks_detected: self.sync_stats.deadlocks_detected.load(Ordering::Relaxed),
            priority_inversions: self.sync_stats.priority_inversions.load(Ordering::Relaxed),
            total_fuel_consumed: self.sync_stats.total_fuel_consumed.load(Ordering::Relaxed),
        }
    }

    // Private helper methods

    fn get_timestamp(&self) -> u64 {
        // In real implementation, would use proper time source
        0
    }

    fn wake_next_mutex_waiter(&mut self, primitive: &mut SyncPrimitive) -> Result<(), Error> {
        // Find next waiter for mutex
        let mut waiter_index = None;
        for (i, waiter) in primitive.waiters.iter().enumerate() {
            if waiter.wait_type == WaitType::MutexLock {
                waiter_index = Some(i);
                break;
            }
        }

        if let Some(index) = waiter_index {
            if let Some(waiter) = primitive.waiters.get(index) {
                if let Some(waker) = &waiter.waker {
                    waker.wake_by_ref();
                }
            }
            primitive.waiters.remove(index);
        }

        Ok(())
    }

    fn wake_next_semaphore_waiter(&mut self, primitive: &mut SyncPrimitive) -> Result<(), Error> {
        // Find next waiter for semaphore
        let mut waiter_index = None;
        for (i, waiter) in primitive.waiters.iter().enumerate() {
            if waiter.wait_type == WaitType::SemaphorePermit {
                waiter_index = Some(i);
                break;
            }
        }

        if let Some(index) = waiter_index {
            if let Some(waiter) = primitive.waiters.get(index) {
                if let Some(waker) = &waiter.waker {
                    waker.wake_by_ref();
                }
            }
            primitive.waiters.remove(index);
        }

        Ok(())
    }

    fn wake_all_barrier_waiters(&mut self, primitive: &mut SyncPrimitive) -> Result<(), Error> {
        // Wake all barrier waiters
        for waiter in primitive.waiters.iter() {
            if waiter.wait_type == WaitType::BarrierWait {
                if let Some(waker) = &waiter.waker {
                    waker.wake_by_ref();
                }
            }
        }

        // Remove all barrier waiters
        primitive.waiters.retain(|waiter| waiter.wait_type != WaitType::BarrierWait);

        Ok(())
    }
}

/// Result of mutex lock operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexLockResult {
    /// Lock acquired immediately
    Acquired,
    /// Would block, caller should wait
    WouldBlock,
}

/// Result of semaphore acquire operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreAcquireResult {
    /// Permit acquired immediately
    Acquired,
    /// Would block, caller should wait
    WouldBlock,
}

/// Result of barrier wait operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierWaitResult {
    /// This task is the leader (last to arrive)
    Leader,
    /// Would block, waiting for other parties
    WouldBlock,
    /// Barrier was broken
    Broken,
}

/// Sync statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub total_mutexes_created: u64,
    pub total_mutex_locks: u64,
    pub total_mutex_unlocks: u64,
    pub total_semaphores_created: u64,
    pub total_semaphore_acquires: u64,
    pub total_semaphore_releases: u64,
    pub total_barriers_created: u64,
    pub total_barrier_waits: u64,
    pub total_condvars_created: u64,
    pub total_condvar_waits: u64,
    pub total_condvar_notifies: u64,
    pub active_primitives: u64,
    pub deadlocks_detected: u64,
    pub priority_inversions: u64,
    pub total_fuel_consumed: u64,
}

/// Async mutex guard
pub struct AsyncMutexGuard {
    primitive_id: SyncPrimitiveId,
    task_id: TaskId,
    sync_primitives: Weak<Mutex<AdvancedSyncPrimitives>>,
}

impl Drop for AsyncMutexGuard {
    fn drop(&mut self) {
        if let Some(sync_primitives) = self.sync_primitives.upgrade() {
            if let Ok(mut primitives) = sync_primitives.lock() {
                let _ = primitives.unlock_async_mutex(self.primitive_id, self.task_id);
            }
        }
    }
}

/// Async semaphore permit
pub struct AsyncSemaphorePermit {
    primitive_id: SyncPrimitiveId,
    sync_primitives: Weak<Mutex<AdvancedSyncPrimitives>>,
}

impl Drop for AsyncSemaphorePermit {
    fn drop(&mut self) {
        if let Some(sync_primitives) = self.sync_primitives.upgrade() {
            if let Ok(mut primitives) = sync_primitives.lock() {
                let _ = primitives.release_semaphore(self.primitive_id);
            }
        }
    }
}

/// Future for mutex lock operation
pub struct MutexLockFuture {
    primitive_id: SyncPrimitiveId,
    task_id: TaskId,
    component_id: ComponentInstanceId,
    sync_primitives: Weak<Mutex<AdvancedSyncPrimitives>>,
}

impl CoreFuture for MutexLockFuture {
    type Output = Result<AsyncMutexGuard, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(sync_primitives) = self.sync_primitives.upgrade() {
            if let Ok(mut primitives) = sync_primitives.lock() {
                match primitives.lock_async_mutex(
                    self.primitive_id,
                    self.task_id,
                    self.component_id,
                ) {
                    Ok(MutexLockResult::Acquired) => {
                        Poll::Ready(Ok(AsyncMutexGuard {
                            primitive_id: self.primitive_id,
                            task_id: self.task_id,
                            sync_primitives: self.sync_primitives.clone(),
                        }))
                    },
                    Ok(MutexLockResult::WouldBlock) => {
                        // Register waker
                        if let Some(primitive) = primitives.primitives.get_mut(&self.primitive_id) {
                            for waiter in primitive.waiters.iter_mut() {
                                if waiter.task_id == self.task_id {
                                    waiter.waker = Some(cx.waker().clone());
                                    break;
                                }
                            }
                        }
                        Poll::Pending
                    },
                    Err(e) => Poll::Ready(Err(e)),
                }
            } else {
                Poll::Ready(Err(Error::new(
                    ErrorCategory::InvalidState,
                    codes::INVALID_STATE,
                    "Sync primitives manager unavailable".to_string(),
                )))
            }
        } else {
            Poll::Ready(Err(Error::new(
                ErrorCategory::InvalidState,
                codes::INVALID_STATE,
                "Sync primitives manager dropped".to_string(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{task_manager::TaskManager, threading::thread_spawn_fuel::FuelTrackedThreadManager};

    fn create_test_bridge() -> Arc<Mutex<TaskManagerAsyncBridge>> {
        let task_manager = Arc::new(Mutex::new(TaskManager::new()));
        let thread_manager = Arc::new(Mutex::new(FuelTrackedThreadManager::new()));
        let config = crate::async_::task_manager_async_bridge::BridgeConfiguration::default();
        let bridge = crate::async_::task_manager_async_bridge::TaskManagerAsyncBridge::new(
            task_manager, thread_manager, config
        ).unwrap();
        Arc::new(Mutex::new(bridge))
    }

    #[test]
    fn test_sync_primitives_creation() {
        let bridge = create_test_bridge();
        let mut sync_primitives = AdvancedSyncPrimitives::new(bridge, None);
        
        let component_id = ComponentInstanceId::new(1);
        sync_primitives.initialize_component_sync(component_id, None).unwrap();
        
        // Create mutex
        let mutex_id = sync_primitives.create_async_mutex(component_id, false).unwrap();
        
        // Create semaphore
        let semaphore_id = sync_primitives.create_async_semaphore(component_id, 3, true).unwrap();
        
        // Create barrier
        let barrier_id = sync_primitives.create_async_barrier(component_id, 2).unwrap();
        
        let stats = sync_primitives.get_sync_statistics();
        assert_eq!(stats.total_mutexes_created, 1);
        assert_eq!(stats.total_semaphores_created, 1);
        assert_eq!(stats.total_barriers_created, 1);
    }

    #[test]
    fn test_mutex_operations() {
        let bridge = create_test_bridge();
        let mut sync_primitives = AdvancedSyncPrimitives::new(bridge, None);
        
        let component_id = ComponentInstanceId::new(1);
        sync_primitives.initialize_component_sync(component_id, None).unwrap();
        
        let mutex_id = sync_primitives.create_async_mutex(component_id, false).unwrap();
        let task_id = TaskId::new(1);
        
        // Lock mutex
        let result = sync_primitives.lock_async_mutex(mutex_id, task_id, component_id).unwrap();
        assert_eq!(result, MutexLockResult::Acquired);
        
        // Try to lock again (should block)
        let task_id2 = TaskId::new(2);
        let result2 = sync_primitives.lock_async_mutex(mutex_id, task_id2, component_id).unwrap();
        assert_eq!(result2, MutexLockResult::WouldBlock);
        
        // Unlock
        sync_primitives.unlock_async_mutex(mutex_id, task_id).unwrap();
        
        let stats = sync_primitives.get_sync_statistics();
        assert_eq!(stats.total_mutex_locks, 2);
        assert_eq!(stats.total_mutex_unlocks, 1);
    }

    #[test]
    fn test_semaphore_operations() {
        let bridge = create_test_bridge();
        let mut sync_primitives = AdvancedSyncPrimitives::new(bridge, None);
        
        let component_id = ComponentInstanceId::new(1);
        sync_primitives.initialize_component_sync(component_id, None).unwrap();
        
        let semaphore_id = sync_primitives.create_async_semaphore(component_id, 2, false).unwrap();
        let task_id = TaskId::new(1);
        
        // Acquire permits
        let result1 = sync_primitives.acquire_semaphore(semaphore_id, task_id, component_id).unwrap();
        assert_eq!(result1, SemaphoreAcquireResult::Acquired);
        
        let result2 = sync_primitives.acquire_semaphore(semaphore_id, task_id, component_id).unwrap();
        assert_eq!(result2, SemaphoreAcquireResult::Acquired);
        
        // Try to acquire when exhausted
        let result3 = sync_primitives.acquire_semaphore(semaphore_id, task_id, component_id).unwrap();
        assert_eq!(result3, SemaphoreAcquireResult::WouldBlock);
        
        // Release permit
        sync_primitives.release_semaphore(semaphore_id).unwrap();
        
        let stats = sync_primitives.get_sync_statistics();
        assert_eq!(stats.total_semaphore_acquires, 2);
        assert_eq!(stats.total_semaphore_releases, 1);
    }
}