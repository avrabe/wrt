use crate::{
    ComponentInstanceId, ValType, ResourceHandle,
    task_manager::{TaskManager, TaskId, TaskState},
    thread_spawn::{ComponentThreadManager, ThreadSpawnRequest, ThreadConfiguration, 
        ThreadHandle, ThreadResult, ThreadId, ThreadSpawnError, ThreadSpawnErrorKind, ThreadSpawnResult},
    execution::{TimeBoundedConfig, TimeBoundedContext, TimeBoundedOutcome},
    canonical_options::CanonicalOptions,
    post_return::{PostReturnRegistry, CleanupTask, CleanupTaskType},
};
use wrt_foundation::{
    bounded_collections::{BoundedVec, BoundedHashMap},
    component_value::ComponentValue,
};
use wrt_platform::{
    advanced_sync::{PriorityInheritanceMutex, Priority},
    sync::{FutexLike, SpinFutex},
};
use core::{
    sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering},
    fmt,
    time::Duration,
};

#[cfg(feature = "std")]
use std::thread;

const MAX_FUEL_PER_THREAD: u64 = 1_000_000;
const FUEL_CHECK_INTERVAL: u64 = 1000;
const FUEL_PER_MS: u64 = 100;

/// Thread execution context with fuel tracking
#[derive(Debug)]
pub struct FuelTrackedThreadContext {
    pub thread_id: ThreadId,
    pub component_id: ComponentInstanceId,
    pub initial_fuel: u64,
    pub remaining_fuel: AtomicU64,
    pub consumed_fuel: AtomicU64,
    pub fuel_exhausted: AtomicBool,
    pub check_interval: u64,
    pub last_check: AtomicU64,
}

impl FuelTrackedThreadContext {
    pub fn new(thread_id: ThreadId, component_id: ComponentInstanceId, initial_fuel: u64) -> Self {
        Self {
            thread_id,
            component_id,
            initial_fuel,
            remaining_fuel: AtomicU64::new(initial_fuel),
            consumed_fuel: AtomicU64::new(0),
            fuel_exhausted: AtomicBool::new(false),
            check_interval: FUEL_CHECK_INTERVAL,
            last_check: AtomicU64::new(0),
        }
    }

    pub fn consume_fuel(&self, amount: u64) -> Result<(), ThreadSpawnError> {
        let current_fuel = self.remaining_fuel.load(Ordering::Acquire);
        
        if current_fuel < amount {
            self.fuel_exhausted.store(true, Ordering::Release);
            return Err(ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                message: format!("Thread {} fuel exhausted", self.thread_id.as_u32()),
            });
        }

        self.remaining_fuel.fetch_sub(amount, Ordering::AcqRel);
        self.consumed_fuel.fetch_add(amount, Ordering::AcqRel);
        
        // Check if we should perform a fuel check
        let consumed = self.consumed_fuel.load(Ordering::Acquire);
        let last_check = self.last_check.load(Ordering::Acquire);
        
        if consumed - last_check >= self.check_interval {
            self.last_check.store(consumed, Ordering::Release);
            self.check_fuel_status()?;
        }

        Ok(())
    }

    pub fn check_fuel_status(&self) -> Result<(), ThreadSpawnError> {
        if self.fuel_exhausted.load(Ordering::Acquire) {
            return Err(ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                message: format!("Thread {} fuel exhausted", self.thread_id.as_u32()),
            });
        }

        let remaining = self.remaining_fuel.load(Ordering::Acquire);
        if remaining == 0 {
            self.fuel_exhausted.store(true, Ordering::Release);
            return Err(ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                message: format!("Thread {} fuel exhausted", self.thread_id.as_u32()),
            });
        }

        Ok(())
    }

    pub fn add_fuel(&self, amount: u64) -> u64 {
        self.remaining_fuel.fetch_add(amount, Ordering::AcqRel)
    }

    pub fn get_remaining_fuel(&self) -> u64 {
        self.remaining_fuel.load(Ordering::Acquire)
    }

    pub fn get_consumed_fuel(&self) -> u64 {
        self.consumed_fuel.load(Ordering::Acquire)
    }
}

/// Extended thread configuration with fuel settings
#[derive(Debug, Clone)]
pub struct FuelThreadConfiguration {
    pub base_config: ThreadConfiguration,
    pub initial_fuel: Option<u64>,
    pub fuel_per_ms: u64,
    pub allow_fuel_extension: bool,
    pub fuel_check_interval: u64,
}

impl Default for FuelThreadConfiguration {
    fn default() -> Self {
        Self {
            base_config: ThreadConfiguration::default(),
            initial_fuel: Some(MAX_FUEL_PER_THREAD),
            fuel_per_ms: FUEL_PER_MS,
            allow_fuel_extension: false,
            fuel_check_interval: FUEL_CHECK_INTERVAL,
        }
    }
}

/// Thread manager with integrated fuel tracking
pub struct FuelTrackedThreadManager {
    base_manager: ComponentThreadManager,
    thread_contexts: BoundedHashMap<ThreadId, FuelTrackedThreadContext, 512>,
    time_bounds: BoundedHashMap<ThreadId, TimeBoundedContext, 512>,
    global_fuel_limit: AtomicU64,
    global_fuel_consumed: AtomicU64,
    fuel_enforcement: AtomicBool,
}

impl FuelTrackedThreadManager {
    pub fn new() -> Self {
        Self {
            base_manager: ComponentThreadManager::new(),
            thread_contexts: BoundedHashMap::new(),
            time_bounds: BoundedHashMap::new(),
            global_fuel_limit: AtomicU64::new(u64::MAX),
            global_fuel_consumed: AtomicU64::new(0),
            fuel_enforcement: AtomicBool::new(true),
        }
    }

    pub fn set_global_fuel_limit(&self, limit: u64) {
        self.global_fuel_limit.store(limit, Ordering::SeqCst);
    }

    pub fn set_fuel_enforcement(&self, enforce: bool) {
        self.fuel_enforcement.store(enforce, Ordering::SeqCst);
    }

    pub fn spawn_thread_with_fuel(
        &mut self,
        request: ThreadSpawnRequest,
        fuel_config: FuelThreadConfiguration,
    ) -> ThreadSpawnResult<ThreadHandle> {
        // Check global fuel availability
        if self.fuel_enforcement.load(Ordering::Acquire) {
            let initial_fuel = fuel_config.initial_fuel.unwrap_or(MAX_FUEL_PER_THREAD);
            let global_consumed = self.global_fuel_consumed.load(Ordering::Acquire);
            let global_limit = self.global_fuel_limit.load(Ordering::Acquire);
            
            if global_consumed + initial_fuel > global_limit {
                return Err(ThreadSpawnError {
                    kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                    message: "Global fuel limit would be exceeded".to_string(),
                });
            }
        }

        // Create time-bounded config
        let time_config = TimeBoundedConfig {
            time_limit_ms: fuel_config.base_config.stack_size.checked_div(fuel_config.fuel_per_ms),
            allow_extension: fuel_config.allow_fuel_extension,
            fuel_limit: fuel_config.initial_fuel,
        };

        // Spawn the thread
        let handle = self.base_manager.spawn_thread(request.clone())?;
        
        // Create fuel context
        let fuel_context = FuelTrackedThreadContext::new(
            handle.thread_id,
            request.component_id,
            fuel_config.initial_fuel.unwrap_or(MAX_FUEL_PER_THREAD),
        );

        // Create time-bounded context
        let time_context = TimeBoundedContext::new(time_config);

        // Store contexts
        self.thread_contexts.insert(handle.thread_id, fuel_context).map_err(|_| {
            ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                message: "Too many thread contexts".to_string(),
            }
        })?;

        self.time_bounds.insert(handle.thread_id, time_context).map_err(|_| {
            ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                message: "Too many time bound contexts".to_string(),
            }
        })?;

        // Update global fuel consumed
        if self.fuel_enforcement.load(Ordering::Acquire) {
            let initial_fuel = fuel_config.initial_fuel.unwrap_or(MAX_FUEL_PER_THREAD);
            self.global_fuel_consumed.fetch_add(initial_fuel, Ordering::AcqRel);
        }

        Ok(handle)
    }

    pub fn consume_thread_fuel(&self, thread_id: ThreadId, amount: u64) -> ThreadSpawnResult<()> {
        if !self.fuel_enforcement.load(Ordering::Acquire) {
            return Ok(());
        }

        let context = self.thread_contexts.get(&thread_id).ok_or_else(|| {
            ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ThreadNotFound,
                message: format!("Thread {} not found", thread_id.as_u32()),
            }
        })?;

        context.consume_fuel(amount)?;

        // Also check time bounds
        if let Some(time_context) = self.time_bounds.get(&thread_id) {
            time_context.check_time_bounds().map_err(|e| {
                ThreadSpawnError {
                    kind: ThreadSpawnErrorKind::ResourceLimitExceeded,
                    message: format!("Time bounds exceeded: {}", e),
                }
            })?;
        }

        Ok(())
    }

    pub fn add_thread_fuel(&mut self, thread_id: ThreadId, amount: u64) -> ThreadSpawnResult<u64> {
        let context = self.thread_contexts.get(&thread_id).ok_or_else(|| {
            ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ThreadNotFound,
                message: format!("Thread {} not found", thread_id.as_u32()),
            }
        })?;

        let new_fuel = context.add_fuel(amount);
        Ok(new_fuel)
    }

    pub fn get_thread_fuel_status(&self, thread_id: ThreadId) -> ThreadSpawnResult<ThreadFuelStatus> {
        let context = self.thread_contexts.get(&thread_id).ok_or_else(|| {
            ThreadSpawnError {
                kind: ThreadSpawnErrorKind::ThreadNotFound,
                message: format!("Thread {} not found", thread_id.as_u32()),
            }
        })?;

        Ok(ThreadFuelStatus {
            thread_id,
            initial_fuel: context.initial_fuel,
            remaining_fuel: context.get_remaining_fuel(),
            consumed_fuel: context.get_consumed_fuel(),
            fuel_exhausted: context.fuel_exhausted.load(Ordering::Acquire),
        })
    }

    pub fn join_thread_with_fuel(&mut self, thread_id: ThreadId) -> ThreadSpawnResult<FuelTrackedThreadResult> {
        let result = self.base_manager.join_thread(thread_id)?;
        
        let fuel_status = self.get_thread_fuel_status(thread_id).ok();
        
        // Clean up contexts
        self.thread_contexts.remove(&thread_id);
        self.time_bounds.remove(&thread_id);

        // Update global fuel consumed (return unused fuel)
        if let Some(ref status) = fuel_status {
            if self.fuel_enforcement.load(Ordering::Acquire) {
                self.global_fuel_consumed.fetch_sub(status.remaining_fuel, Ordering::AcqRel);
            }
        }

        Ok(FuelTrackedThreadResult {
            result,
            fuel_status,
        })
    }

    pub fn get_global_fuel_status(&self) -> GlobalFuelStatus {
        GlobalFuelStatus {
            limit: self.global_fuel_limit.load(Ordering::Acquire),
            consumed: self.global_fuel_consumed.load(Ordering::Acquire),
            enforcement_enabled: self.fuel_enforcement.load(Ordering::Acquire),
        }
    }

    pub fn execute_with_fuel_tracking<F, R>(
        &self,
        thread_id: ThreadId,
        fuel_per_operation: u64,
        operation: F,
    ) -> ThreadSpawnResult<R>
    where
        F: FnOnce() -> R,
    {
        // Consume fuel before operation
        self.consume_thread_fuel(thread_id, fuel_per_operation)?;
        
        // Execute the operation
        let result = operation();
        
        Ok(result)
    }
}

impl Default for FuelTrackedThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ThreadFuelStatus {
    pub thread_id: ThreadId,
    pub initial_fuel: u64,
    pub remaining_fuel: u64,
    pub consumed_fuel: u64,
    pub fuel_exhausted: bool,
}

#[derive(Debug, Clone)]
pub struct FuelTrackedThreadResult {
    pub result: ThreadResult,
    pub fuel_status: Option<ThreadFuelStatus>,
}

#[derive(Debug, Clone)]
pub struct GlobalFuelStatus {
    pub limit: u64,
    pub consumed: u64,
    pub enforcement_enabled: bool,
}

impl GlobalFuelStatus {
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.consumed)
    }

    pub fn usage_percentage(&self) -> f64 {
        if self.limit == 0 {
            0.0
        } else {
            (self.consumed as f64 / self.limit as f64) * 100.0
        }
    }
}

/// Helper functions for creating fuel-aware thread configurations
pub fn create_fuel_thread_config(initial_fuel: u64) -> FuelThreadConfiguration {
    FuelThreadConfiguration {
        base_config: ThreadConfiguration::default(),
        initial_fuel: Some(initial_fuel),
        fuel_per_ms: FUEL_PER_MS,
        allow_fuel_extension: false,
        fuel_check_interval: FUEL_CHECK_INTERVAL,
    }
}

pub fn create_unlimited_fuel_thread_config() -> FuelThreadConfiguration {
    FuelThreadConfiguration {
        base_config: ThreadConfiguration::default(),
        initial_fuel: None,
        fuel_per_ms: 0,
        allow_fuel_extension: true,
        fuel_check_interval: u64::MAX,
    }
}

/// Integration with component execution
pub trait FuelAwareExecution {
    fn execute_with_fuel<F, R>(&self, fuel: u64, f: F) -> Result<R, ThreadSpawnError>
    where
        F: FnOnce() -> R;
        
    fn check_fuel_before_operation(&self, required_fuel: u64) -> Result<(), ThreadSpawnError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_context_creation() {
        let context = FuelTrackedThreadContext::new(
            ThreadId::new(1),
            ComponentInstanceId::new(1),
            1000,
        );
        
        assert_eq!(context.get_remaining_fuel(), 1000);
        assert_eq!(context.get_consumed_fuel(), 0);
        assert!(!context.fuel_exhausted.load(Ordering::Acquire));
    }

    #[test]
    fn test_fuel_consumption() {
        let context = FuelTrackedThreadContext::new(
            ThreadId::new(1),
            ComponentInstanceId::new(1),
            1000,
        );
        
        assert!(context.consume_fuel(100).is_ok());
        assert_eq!(context.get_remaining_fuel(), 900);
        assert_eq!(context.get_consumed_fuel(), 100);
        
        assert!(context.consume_fuel(900).is_ok());
        assert_eq!(context.get_remaining_fuel(), 0);
        
        assert!(context.consume_fuel(1).is_err());
        assert!(context.fuel_exhausted.load(Ordering::Acquire));
    }

    #[test]
    fn test_global_fuel_status() {
        let status = GlobalFuelStatus {
            limit: 1000,
            consumed: 250,
            enforcement_enabled: true,
        };
        
        assert_eq!(status.remaining(), 750);
        assert_eq!(status.usage_percentage(), 25.0);
    }

    #[test]
    fn test_fuel_thread_config() {
        let config = create_fuel_thread_config(5000);
        assert_eq!(config.initial_fuel, Some(5000));
        assert_eq!(config.fuel_per_ms, FUEL_PER_MS);
        assert!(!config.allow_fuel_extension);
        
        let unlimited = create_unlimited_fuel_thread_config();
        assert_eq!(unlimited.initial_fuel, None);
        assert!(unlimited.allow_fuel_extension);
    }
}