//! Fuel-based async executor for deterministic WebAssembly Component Model execution
//!
//! This module provides an async executor that uses fuel consumption for timing
//! guarantees, enabling deterministic async execution across all ASIL levels.

use crate::{
    execution::{TimeBoundedConfig, TimeBoundedContext},
    task_manager::{TaskId, TaskState},
    threading::thread_spawn_fuel::{FuelTrackedThreadContext, ThreadFuelStatus},
    ComponentInstanceId,
    prelude::*,
};
use crate::{
    component_instance::{ComponentInstance, InstanceState},
    canonical_abi::{ComponentValue, CanonicalOptions},
};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    operations::{global_fuel_consumed, record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    CrateId, safe_managed_alloc,
};
use wrt_platform::{
    advanced_sync::{Priority, PriorityInheritanceMutex},
    sync::{FutexLike, SpinFutex},
};
use wrt_foundation::{Arc, Weak, sync::Mutex};
use crate::async_::{
    fuel_aware_waker::{create_fuel_aware_waker, create_fuel_aware_waker_with_asil, WakeCoalescer},
    fuel_dynamic_manager::{FuelDynamicManager, FuelAllocationPolicy},
    fuel_preemption_support::{FuelPreemptionManager, PreemptionPolicy, PreemptionDecision},
    fuel_debt_credit::{FuelDebtCreditSystem, DebtPolicy, CreditRestriction},
    async_task_executor::{AsyncTaskExecutor, ASILExecutorFactory},
};

/// Maximum number of concurrent async tasks
const MAX_ASYNC_TASKS: usize = 128;

/// Yield threshold - yield after this much fuel consumed
const YIELD_FUEL_THRESHOLD: u64 = 1000;

/// Fuel budget for basic async operations
const ASYNC_TASK_SPAWN_FUEL: u64 = 20;
const ASYNC_TASK_YIELD_FUEL: u64 = 5;
const ASYNC_TASK_WAKE_FUEL: u64 = 10;
const ASYNC_TASK_POLL_FUEL: u64 = 15;

/// Async task representation with fuel tracking
#[derive(Debug)]
pub struct FuelAsyncTask {
    pub id: TaskId,
    pub component_id: ComponentInstanceId,
    pub fuel_budget: u64,
    pub fuel_consumed: AtomicU64,
    pub priority: Priority,
    pub verification_level: VerificationLevel,
    pub state: AsyncTaskState,
    pub waker: Option<Waker>,
    pub future: Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'static>>,
    pub execution_context: ExecutionContext,
}

/// State of an async task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncTaskState {
    /// Task is ready to be polled
    Ready,
    /// Task is waiting for an event
    Waiting,
    /// Task has completed successfully
    Completed,
    /// Task encountered an error
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task ran out of fuel
    FuelExhausted,
}

/// Result of executing one WebAssembly execution step
#[derive(Debug, Clone)]
pub enum ExecutionStepResult {
    /// Execution step completed successfully with result data
    Completed(Vec<u8>),
    /// Execution yielded and can be resumed later
    Yielded,
    /// Execution is waiting for an async operation
    Waiting,
}

/// Component Model async operations
#[derive(Debug, Clone)]
pub enum ComponentAsyncOperation {
    /// task.wait - wait for one of multiple waitables to become ready
    TaskWait {
        waitables: Vec<u32>, // Waitable indices
    },
    /// task.yield - voluntarily yield execution
    TaskYield,
    /// task.poll - check waitables without blocking
    TaskPoll {
        waitables: Vec<u32>, // Waitable indices to check
    },
}

/// Result of Component Model async operations
#[derive(Debug, Clone)]
pub enum ComponentAsyncOperationResult {
    /// Operation is waiting for completion
    Waiting {
        ready_index: Option<u32>, // Index of ready waitable, if any
    },
    /// Task yielded execution
    Yielded,
    /// Polling completed
    Polled {
        ready_index: Option<u32>, // Index of ready waitable, if any
    },
}

/// Execution state information for monitoring
#[derive(Debug, Clone)]
pub struct ExecutionStateInfo {
    pub task_id: TaskId,
    pub component_id: ComponentInstanceId,
    pub asil_mode: ASILExecutionMode,
    pub stack_depth: u32,
    pub max_stack_depth: u32,
    pub fuel_consumed: u64,
    pub has_yield_point: bool,
    pub has_component_instance: bool,
    pub error_state: Option<Error>,
}

/// Execution context for async task execution
#[derive(Debug)]
pub struct ExecutionContext {
    /// Component instance for WebAssembly execution
    pub component_instance: Option<Arc<ComponentInstance>>,
    /// Current execution stack depth
    pub stack_depth: u32,
    /// Maximum allowed stack depth (ASIL compliance)
    pub max_stack_depth: u32,
    /// Execution state storage for suspendable functions
    pub execution_state: Option<Box<dyn ExecutionState>>,
    /// Fuel consumption tracking within this context
    pub context_fuel_consumed: AtomicU64,
    /// Last yield point for resumable execution
    pub last_yield_point: Option<YieldPoint>,
    /// Error state for propagation
    pub error_state: Option<Error>,
    /// ASIL execution mode for this context
    pub asil_mode: ASILExecutionMode,
}

/// Trait for execution state that can be suspended and resumed
pub trait ExecutionState: core::fmt::Debug + Send + Sync {
    /// Save the current execution state for later resumption
    fn save_state(&self) -> Result<Vec<u8>, Error>;
    /// Restore execution state from saved data
    fn restore_state(&mut self, data: &[u8]) -> Result<(), Error>;
    /// Get the current function index being executed
    fn current_function_index(&self) -> Option<u32>;
    /// Get local variables state
    fn get_locals(&self) -> &[ComponentValue];
    /// Set local variables state
    fn set_locals(&mut self, locals: Vec<ComponentValue>) -> Result<(), Error>;
}

/// Yield point information for resumable execution
#[derive(Debug, Clone)]
pub struct YieldPoint {
    /// Instruction pointer or yield location
    pub instruction_pointer: u32,
    /// Stack frame information
    pub stack_frame: Vec<ComponentValue>,
    /// Local variables at yield point
    pub locals: Vec<ComponentValue>,
    /// Fuel consumed up to this yield point
    pub fuel_at_yield: u64,
    /// Timestamp of yield (for deterministic replay)
    pub yield_timestamp: u64,
}

/// ASIL execution mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ASILExecutionMode {
    /// ASIL-A: Basic safety requirements
    A {
        /// Enable basic error detection
        error_detection: bool,
    },
    /// ASIL-B: Bounded resource usage
    B {
        /// Resource limits strictly enforced
        strict_resource_limits: bool,
        /// Maximum execution time per slice
        max_execution_slice_ms: u32,
    },
    /// ASIL-C: Freedom from interference
    C {
        /// Spatial isolation enforced
        spatial_isolation: bool,
        /// Temporal isolation enforced
        temporal_isolation: bool,
        /// Resource isolation enforced
        resource_isolation: bool,
    },
    /// ASIL-D: Highest safety integrity
    D {
        /// Deterministic execution required
        deterministic_execution: bool,
        /// Bounded execution time required
        bounded_execution_time: bool,
        /// Formal verification hooks enabled
        formal_verification: bool,
        /// Maximum fuel per execution slice
        max_fuel_per_slice: u64,
    },
}

impl Default for ASILExecutionMode {
    fn default() -> Self {
        ASILExecutionMode::A {
            error_detection: true,
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context for the given ASIL level
    pub fn new(asil_mode: ASILExecutionMode) -> Self {
        let max_stack_depth = match asil_mode {
            ASILExecutionMode::D { .. } => 16,  // Strict limits for ASIL-D
            ASILExecutionMode::C { .. } => 32,  // Moderate limits for ASIL-C
            ASILExecutionMode::B { .. } => 64,  // Higher limits for ASIL-B
            ASILExecutionMode::A { .. } => 128, // Flexible limits for ASIL-A
        };

        Self {
            component_instance: None,
            stack_depth: 0,
            max_stack_depth,
            execution_state: None,
            context_fuel_consumed: AtomicU64::new(0),
            last_yield_point: None,
            error_state: None,
            asil_mode,
        }
    }

    /// Set the component instance for this execution context
    pub fn set_component_instance(&mut self, instance: Arc<ComponentInstance>) {
        self.component_instance = Some(instance);
    }

    /// Check if execution can continue based on ASIL constraints
    pub fn can_continue_execution(&self) -> Result<bool, Error> {
        // Check stack depth limits
        if self.stack_depth >= self.max_stack_depth {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                format!("Stack depth limit exceeded: {} >= {}", 
                    self.stack_depth, self.max_stack_depth),
            ));
        }

        // Check ASIL-specific constraints
        match self.asil_mode {
            ASILExecutionMode::D { max_fuel_per_slice, .. } => {
                let fuel_consumed = self.context_fuel_consumed.load(Ordering::Acquire);
                if fuel_consumed >= max_fuel_per_slice {
                    return Ok(false); // Must yield
                }
            },
            ASILExecutionMode::B { max_execution_slice_ms, .. } => {
                // In real implementation, would check actual execution time
                // For now, use fuel as a proxy
                let fuel_consumed = self.context_fuel_consumed.load(Ordering::Acquire);
                if fuel_consumed >= (max_execution_slice_ms as u64 * 10) { // 10 fuel per ms
                    return Ok(false); // Must yield
                }
            },
            _ => {} // A and C modes have different constraints
        }

        Ok(true)
    }

    /// Record fuel consumption in this context
    pub fn consume_fuel(&self, amount: u64) {
        self.context_fuel_consumed.fetch_add(amount, Ordering::AcqRel);
    }

    /// Create a yield point for suspending execution
    pub fn create_yield_point(&mut self, instruction_pointer: u32, 
                             stack_frame: Vec<ComponentValue>, 
                             locals: Vec<ComponentValue>) -> Result<(), Error> {
        let fuel_consumed = self.context_fuel_consumed.load(Ordering::Acquire);
        
        self.last_yield_point = Some(YieldPoint {
            instruction_pointer,
            stack_frame,
            locals,
            fuel_at_yield: fuel_consumed,
            yield_timestamp: self.get_deterministic_timestamp(),
        });
        
        Ok(())
    }

    /// Get deterministic timestamp for ASIL compliance
    fn get_deterministic_timestamp(&self) -> u64 {
        match self.asil_mode {
            ASILExecutionMode::D { deterministic_execution: true, .. } => {
                // For ASIL-D, use fuel consumption as deterministic timestamp
                self.context_fuel_consumed.load(Ordering::Acquire)
            },
            _ => {
                // For other modes, could use real time
                // For now, use fuel consumption as proxy
                self.context_fuel_consumed.load(Ordering::Acquire)
            }
        }
    }

    /// Reset context for new execution
    pub fn reset(&mut self) {
        self.stack_depth = 0;
        self.execution_state = None;
        self.context_fuel_consumed.store(0, Ordering::SeqCst);
        self.last_yield_point = None;
        self.error_state = None;
    }

    /// Execute deterministic step for ASIL-D
    pub fn execute_deterministic_step(&mut self) -> Result<ExecutionStepResult, Error> {
        // In real implementation, would execute WebAssembly with deterministic guarantees
        // For now, simulate deterministic execution
        self.stack_depth += 1;
        self.consume_fuel(10);
        
        // Simulate completion after some steps
        if self.context_fuel_consumed.load(Ordering::Acquire) > 100 {
            Ok(ExecutionStepResult::Completed(vec![42u8; 8]))
        } else {
            Ok(ExecutionStepResult::Yielded)
        }
    }

    /// Execute isolated step for ASIL-C
    pub fn execute_isolated_step(&mut self) -> Result<ExecutionStepResult, Error> {
        // Validate memory isolation first
        self.validate_memory_isolation()?;
        
        // Execute with isolation guarantees
        self.consume_fuel(15);
        
        // Simulate various outcomes
        if self.context_fuel_consumed.load(Ordering::Acquire) > 200 {
            Ok(ExecutionStepResult::Completed(vec![24u8; 8]))
        } else if self.stack_depth > 5 {
            Ok(ExecutionStepResult::Waiting)
        } else {
            Ok(ExecutionStepResult::Yielded)
        }
    }

    /// Execute bounded step for ASIL-B
    pub fn execute_bounded_step(&mut self, max_duration_ms: u64) -> Result<ExecutionStepResult, Error> {
        let max_fuel = max_duration_ms * 1000; // 1ms = 1000 fuel units
        let start_fuel = self.context_fuel_consumed.load(Ordering::Acquire);
        
        // Execute with bounds
        self.consume_fuel(20);
        
        let consumed = self.context_fuel_consumed.load(Ordering::Acquire) - start_fuel;
        if consumed >= max_fuel {
            // Hit limit, must yield
            Ok(ExecutionStepResult::Yielded)
        } else if self.context_fuel_consumed.load(Ordering::Acquire) > 300 {
            Ok(ExecutionStepResult::Completed(vec![36u8; 8]))
        } else {
            Ok(ExecutionStepResult::Yielded)
        }
    }

    /// Execute flexible step for ASIL-A
    pub fn execute_flexible_step(&mut self) -> Result<ExecutionStepResult, Error> {
        // More flexible execution with error recovery
        self.consume_fuel(25);
        
        // Simulate occasional errors for ASIL-A
        if self.stack_depth > 10 {
            // Simulate recoverable error
            self.stack_depth = 5; // Reset to safe state
            return Ok(ExecutionStepResult::Yielded);
        }
        
        if self.context_fuel_consumed.load(Ordering::Acquire) > 400 {
            Ok(ExecutionStepResult::Completed(vec![18u8; 8]))
        } else {
            self.stack_depth += 1;
            Ok(ExecutionStepResult::Yielded)
        }
    }

    /// Validate memory isolation for ASIL-C
    pub fn validate_memory_isolation(&self) -> Result<(), Error> {
        // In real implementation, would check memory boundaries
        // For now, always succeed
        Ok(())
    }
}

/// Fuel-based async executor for Component Model
pub struct FuelAsyncExecutor {
    /// Task storage with bounded capacity
    tasks: BoundedHashMap<TaskId, FuelAsyncTask, MAX_ASYNC_TASKS>,
    /// Ready queue for tasks that can be polled
    ready_queue: Arc<Mutex<BoundedVec<TaskId, MAX_ASYNC_TASKS>>>,
    /// Global fuel limit for all async operations
    global_fuel_limit: AtomicU64,
    /// Global fuel consumed by all async operations
    global_fuel_consumed: AtomicU64,
    /// Default verification level for new tasks
    default_verification_level: VerificationLevel,
    /// Whether fuel enforcement is enabled
    fuel_enforcement: AtomicBool,
    /// Next task ID counter
    next_task_id: AtomicUsize,
    /// Executor state
    executor_state: ExecutorState,
    /// Wake coalescer to prevent thundering herd
    wake_coalescer: Option<crate::async_::fuel_aware_waker::WakeCoalescer>,
    /// Weak reference to self for waker creation
    self_ref: Option<Weak<Mutex<Self>>>,
    /// Polling statistics
    polling_stats: PollingStatistics,
    /// Dynamic fuel manager
    fuel_manager: Option<FuelDynamicManager>,
    /// Preemption manager
    preemption_manager: Option<FuelPreemptionManager>,
    /// Active fuel monitor
    fuel_monitor: Option<FuelMonitor>,
    /// ASIL fuel enforcement policy
    fuel_enforcement_policy: Option<ASILFuelEnforcementPolicy>,
    /// Fuel debt and credit system
    debt_credit_system: Option<FuelDebtCreditSystem>,
    /// ASIL-specific task executors
    asil_executors: BoundedHashMap<u8, Box<dyn AsyncTaskExecutor>, 4>,
}

/// State of the executor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorState {
    /// Executor is running
    Running,
    /// Executor is paused
    Paused,
    /// Executor is shutting down
    ShuttingDown,
    /// Executor has stopped
    Stopped,
}

impl FuelAsyncExecutor {
    /// Create a new fuel-based async executor
    pub fn new() -> Result<Self, Error> {
        let provider = safe_managed_alloc!(8192, CrateId::Component)?;
        let ready_queue = Arc::new(Mutex::new(BoundedVec::new(provider)?));
        let wake_coalescer = crate::async_::fuel_aware_waker::WakeCoalescer::new().ok();
        
        // Initialize ASIL executors
        let mut asil_executors = BoundedHashMap::new();
        
        // Create executor for each ASIL level
        let asil_d = ASILExecutionMode::D {
            deterministic_execution: true,
            bounded_execution_time: true,
            formal_verification: true,
            max_fuel_per_slice: 1000,
        };
        asil_executors.insert(0, ASILExecutorFactory::create_executor(asil_d)).ok();
        
        let asil_c = ASILExecutionMode::C {
            spatial_isolation: true,
            temporal_isolation: true,
            resource_isolation: true,
        };
        asil_executors.insert(1, ASILExecutorFactory::create_executor(asil_c)).ok();
        
        let asil_b = ASILExecutionMode::B {
            strict_resource_limits: true,
            max_execution_slice_ms: 10,
        };
        asil_executors.insert(2, ASILExecutorFactory::create_executor(asil_b)).ok();
        
        let asil_a = ASILExecutionMode::A {
            error_detection: true,
        };
        asil_executors.insert(3, ASILExecutorFactory::create_executor(asil_a)).ok();
        
        Ok(Self {
            tasks: BoundedHashMap::new(),
            ready_queue,
            global_fuel_limit: AtomicU64::new(u64::MAX),
            global_fuel_consumed: AtomicU64::new(0),
            default_verification_level: VerificationLevel::Standard,
            fuel_enforcement: AtomicBool::new(true),
            next_task_id: AtomicUsize::new(1),
            executor_state: ExecutorState::Running,
            wake_coalescer,
            self_ref: None,
            polling_stats: PollingStatistics::default(),
            fuel_manager: None,
            preemption_manager: None,
            fuel_monitor: None,
            fuel_enforcement_policy: None,
            debt_credit_system: None,
            asil_executors,
        })
    }

    /// Set the global fuel limit for all async operations
    pub fn set_global_fuel_limit(&self, limit: u64) {
        self.global_fuel_limit.store(limit, Ordering::SeqCst);
    }

    /// Set the default verification level for new tasks
    pub fn set_default_verification_level(&mut self, level: VerificationLevel) {
        self.default_verification_level = level;
    }

    /// Enable or disable fuel enforcement
    pub fn set_fuel_enforcement(&self, enabled: bool) {
        self.fuel_enforcement.store(enabled, Ordering::SeqCst);
    }

    /// Enable dynamic fuel management
    pub fn enable_dynamic_fuel_management(&mut self, policy: FuelAllocationPolicy) -> Result<(), Error> {
        let mut manager = FuelDynamicManager::new(policy, 1_000_000)?;
        // Register default component
        manager.register_component(ComponentInstanceId::new(0), 100_000, Priority::Normal)?;
        self.fuel_manager = Some(manager);
        Ok(())
    }
    
    /// Enable preemption support
    pub fn enable_preemption(&mut self, policy: PreemptionPolicy) -> Result<(), Error> {
        self.preemption_manager = Some(FuelPreemptionManager::new(policy)?);
        Ok(())
    }

    /// Enable active fuel monitoring
    pub fn enable_fuel_monitoring(&mut self) -> Result<(), Error> {
        self.fuel_monitor = Some(FuelMonitor::new()?);
        Ok(())
    }

    /// Get fuel monitoring statistics
    pub fn get_fuel_monitoring_stats(&self) -> Option<FuelMonitoringStats> {
        self.fuel_monitor.as_ref().map(|monitor| monitor.get_statistics())
    }

    /// Get active fuel alerts
    pub fn get_fuel_alerts(&self) -> Vec<FuelAlert> {
        if let Some(monitor) = &self.fuel_monitor {
            if let Ok(alerts) = monitor.active_alerts.lock() {
                return alerts.iter().cloned().collect();
            }
        }
        Vec::new()
    }

    /// Clear fuel alerts
    pub fn clear_fuel_alerts(&self) {
        if let Some(monitor) = &self.fuel_monitor {
            monitor.clear_alerts();
        }
    }

    /// Set ASIL fuel enforcement policy
    pub fn set_fuel_enforcement_policy(&mut self, policy: ASILFuelEnforcementPolicy) {
        self.fuel_enforcement_policy = Some(policy);
    }

    /// Enforce ASIL-specific fuel policy for a task
    fn enforce_asil_fuel_policy(
        &self,
        task: &FuelAsyncTask,
        fuel_to_consume: u64,
    ) -> Result<FuelEnforcementDecision, Error> {
        let policy = match &self.fuel_enforcement_policy {
            Some(p) => p,
            None => return Ok(FuelEnforcementDecision::Allow), // No policy, allow
        };

        let fuel_consumed = task.fuel_consumed.load(Ordering::Acquire);
        let remaining_fuel = task.fuel_budget.saturating_sub(fuel_consumed);

        // Check ASIL-specific enforcement
        match task.execution_context.asil_mode {
            ASILExecutionMode::D { .. } => {
                self.enforce_asil_d_policy(task, fuel_to_consume, remaining_fuel, &policy.asil_policies.asil_d)
            },
            ASILExecutionMode::C { .. } => {
                self.enforce_asil_c_policy(task, fuel_to_consume, remaining_fuel, &policy.asil_policies.asil_c)
            },
            ASILExecutionMode::B { .. } => {
                self.enforce_asil_b_policy(task, fuel_to_consume, remaining_fuel, &policy.asil_policies.asil_b)
            },
            ASILExecutionMode::A { .. } => {
                self.enforce_asil_a_policy(task, fuel_to_consume, remaining_fuel, &policy.asil_policies.asil_a)
            },
        }
    }

    /// Enforce ASIL-D deterministic fuel policy
    fn enforce_asil_d_policy(
        &self,
        task: &FuelAsyncTask,
        fuel_to_consume: u64,
        remaining_fuel: u64,
        policy: &ASILDPolicy,
    ) -> Result<FuelEnforcementDecision, Error> {
        // Check fuel quantum alignment
        if policy.enforce_deterministic_ordering && fuel_to_consume % policy.fuel_quantum != 0 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                format!("ASIL-D: Fuel consumption {} not aligned to quantum {}", 
                    fuel_to_consume, policy.fuel_quantum),
            ));
        }

        // Check max step fuel
        if fuel_to_consume > policy.max_step_fuel {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                format!("ASIL-D: Fuel consumption {} exceeds max step fuel {}", 
                    fuel_to_consume, policy.max_step_fuel),
            ));
        }

        // Check preallocation requirement
        if policy.require_preallocation && remaining_fuel < fuel_to_consume {
            return Ok(FuelEnforcementDecision::Deny {
                reason: "ASIL-D: Insufficient preallocated fuel".to_string(),
            });
        }

        Ok(FuelEnforcementDecision::Allow)
    }

    /// Enforce ASIL-C component isolation fuel policy
    fn enforce_asil_c_policy(
        &self,
        task: &FuelAsyncTask,
        fuel_to_consume: u64,
        remaining_fuel: u64,
        policy: &ASILCPolicy,
    ) -> Result<FuelEnforcementDecision, Error> {
        // Check component isolation
        if policy.component_isolation {
            // In real implementation, would check component-specific fuel pool
            let component_fuel_available = remaining_fuel; // Placeholder
            
            if component_fuel_available < fuel_to_consume {
                // Check if fuel transfer is allowed
                if policy.max_transfer_amount > 0 {
                    let transfer_needed = fuel_to_consume - component_fuel_available;
                    if transfer_needed <= policy.max_transfer_amount {
                        return Ok(FuelEnforcementDecision::AllowWithTransfer {
                            transfer_amount: transfer_needed,
                            source_component: None, // Would specify source
                        });
                    }
                }
                
                return Ok(FuelEnforcementDecision::Deny {
                    reason: "ASIL-C: Component fuel isolation violation".to_string(),
                });
            }
        }

        Ok(FuelEnforcementDecision::Allow)
    }

    /// Enforce ASIL-B bounded fuel policy
    fn enforce_asil_b_policy(
        &self,
        task: &FuelAsyncTask,
        fuel_to_consume: u64,
        remaining_fuel: u64,
        policy: &ASILBPolicy,
    ) -> Result<FuelEnforcementDecision, Error> {
        // Check slice budget
        let current_slice_consumed = task.execution_context.context_fuel_consumed.load(Ordering::Acquire);
        
        if current_slice_consumed + fuel_to_consume > policy.slice_fuel_budget {
            // Check if rollover is allowed
            if policy.allow_rollover {
                let rollover_allowed = policy.slice_fuel_budget * policy.max_rollover_percent as u64 / 100;
                if current_slice_consumed + fuel_to_consume <= policy.slice_fuel_budget + rollover_allowed {
                    return Ok(FuelEnforcementDecision::AllowWithRollover {
                        rollover_amount: (current_slice_consumed + fuel_to_consume) - policy.slice_fuel_budget,
                    });
                }
            }
            
            return Ok(FuelEnforcementDecision::RequireYield {
                reason: "ASIL-B: Time slice fuel budget exceeded".to_string(),
            });
        }

        Ok(FuelEnforcementDecision::Allow)
    }

    /// Enforce ASIL-A flexible fuel policy
    fn enforce_asil_a_policy(
        &self,
        task: &FuelAsyncTask,
        fuel_to_consume: u64,
        _remaining_fuel: u64,
        policy: &ASILAPolicy,
    ) -> Result<FuelEnforcementDecision, Error> {
        let total_consumed = task.fuel_consumed.load(Ordering::Acquire) + fuel_to_consume;
        
        // Check hard limit
        if total_consumed > policy.hard_limit {
            return Ok(FuelEnforcementDecision::Deny {
                reason: "ASIL-A: Hard fuel limit exceeded".to_string(),
            });
        }

        // Check soft limit
        if total_consumed > policy.soft_limit {
            // In real implementation, would check grace period timing
            return Ok(FuelEnforcementDecision::AllowWithWarning {
                warning: format!("ASIL-A: Soft limit exceeded: {} > {}", 
                    total_consumed, policy.soft_limit),
            });
        }

        Ok(FuelEnforcementDecision::Allow)
    }
    
    /// Spawn a new async task with fuel budget
    pub fn spawn_task<F>(
        &mut self,
        component_id: ComponentInstanceId,
        fuel_budget: u64,
        priority: Priority,
        future: F,
    ) -> Result<TaskId, Error>
    where
        F: Future<Output = Result<(), Error>> + Send + 'static,
    {
        // Calculate dynamic fuel allocation if enabled
        let allocated_fuel = if let Some(ref mut fuel_mgr) = self.fuel_manager {
            fuel_mgr.calculate_fuel_allocation(
                TaskId::new(self.next_task_id.load(Ordering::Acquire) as u32),
                component_id,
                fuel_budget,
                priority,
            )?
        } else {
            fuel_budget
        };
        
        // Check global fuel availability
        if self.fuel_enforcement.load(Ordering::Acquire) {
            let global_consumed = self.global_fuel_consumed.load(Ordering::Acquire);
            let global_limit = self.global_fuel_limit.load(Ordering::Acquire);

            if global_consumed + allocated_fuel > global_limit {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    "Global fuel limit would be exceeded".to_string(),
                ));
            }
        }

        // Generate new task ID
        let task_id = TaskId::new(self.next_task_id.fetch_add(1, Ordering::AcqRel) as u32);

        // Record task spawn operation
        record_global_operation(OperationType::FunctionCall, self.default_verification_level);

        // Consume fuel for task spawn
        self.consume_global_fuel(ASYNC_TASK_SPAWN_FUEL)?;

        // Register with preemption manager if enabled
        if let Some(ref mut preempt_mgr) = self.preemption_manager {
            preempt_mgr.register_task(task_id, priority, true, allocated_fuel)?;
        }
        
        // Create the task with ExecutionContext integration
        let mut execution_context = ExecutionContext::new(
            self.asil_mode_for_priority(priority)
        );

        // Try to set component instance for real WebAssembly execution
        if let Some(component_instance) = self.get_component_instance(component_id) {
            execution_context.set_component_instance(component_instance);
        }

        let task = FuelAsyncTask {
            id: task_id,
            component_id,
            fuel_budget: allocated_fuel,
            fuel_consumed: AtomicU64::new(0),
            priority,
            verification_level: self.default_verification_level,
            state: AsyncTaskState::Ready,
            waker: None,
            future: Box::pin(future),
            execution_context,
        };

        // Store the task
        self.tasks.insert(task_id, task).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many concurrent async tasks".to_string(),
            )
        })?;

        // Add to ready queue
        self.ready_queue.lock()?.push(task_id).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Ready queue is full".to_string(),
            )
        })?;

        Ok(task_id)
    }

    /// Poll ready tasks and advance execution
    pub fn poll_tasks(&mut self) -> Result<usize, Error> {
        if self.executor_state != ExecutorState::Running {
            return Ok(0);
        }

        let mut tasks_polled = 0;
        let mut fuel_consumed_this_batch = 0u64;
        
        // Process wake coalescing if available
        if let Some(ref coalescer) = self.wake_coalescer {
            let wakes_processed = coalescer.process_wakes(&self.ready_queue)?;
            self.polling_stats.wakes_coalesced += wakes_processed;
        }

        // Process ready tasks with fuel-aware scheduling
        while let Some(task_id) = self.get_next_ready_task() {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                // Check fuel before polling
                if self.should_check_fuel(&task) {
                    if task.fuel_consumed.load(Ordering::Acquire) >= task.fuel_budget {
                        // Try to get emergency fuel
                        if let Some(ref mut fuel_mgr) = self.fuel_manager {
                            if let Ok(emergency_fuel) = fuel_mgr.handle_fuel_exhaustion(task_id) {
                                task.fuel_budget += emergency_fuel;
                            } else {
                                task.state = AsyncTaskState::FuelExhausted;
                                continue;
                            }
                        } else {
                            task.state = AsyncTaskState::FuelExhausted;
                            continue;
                        }
                    }
                }

                // Consume fuel for polling
                self.consume_task_fuel(task, ASYNC_TASK_POLL_FUEL)?;
                fuel_consumed_this_batch += ASYNC_TASK_POLL_FUEL;
                
                // Check preemption if enabled
                if let Some(ref mut preempt_mgr) = self.preemption_manager {
                    match preempt_mgr.check_preemption(task_id, ASYNC_TASK_POLL_FUEL, self)? {
                        PreemptionDecision::Continue => {},
                        PreemptionDecision::YieldPoint => {
                            // Re-add to ready queue and yield
                            self.ready_queue.lock()?.push(task_id).ok();
                            continue;
                        },
                        PreemptionDecision::Preempt(reason) => {
                            // Save state and preempt
                            task.state = AsyncTaskState::Waiting;
                            self.ready_queue.lock()?.push(task_id).ok();
                            self.polling_stats.tasks_yielded += 1;
                            continue;
                        }
                    }
                }

                // Create a proper fuel-aware waker with ASIL mode
                let waker = if let Some(ref weak_self) = self.self_ref {
                    create_fuel_aware_waker_with_asil(
                        task_id,
                        self.ready_queue.clone(),
                        weak_self.clone(),
                        task.execution_context.asil_mode,
                    )
                } else {
                    // Fallback to no-op waker if self_ref not set
                    create_noop_waker()
                };
                let mut context = Context::from_waker(&waker);
                
                // Update task's waker
                task.waker = Some(waker.clone());

                // Execute using ExecutionContext for real WebAssembly execution
                record_global_operation(OperationType::FunctionCall, task.verification_level);
                
                let execution_result = self.execute_task_with_context(task_id, &mut context);
                
                match execution_result {
                    Ok(Some(result)) => {
                        // Task completed successfully
                        task.state = AsyncTaskState::Completed;
                        
                        // Grant credit for unused fuel when task completes
                        if let Some(system) = &mut self.debt_credit_system {
                            let consumed = task.fuel_consumed.load(Ordering::Acquire);
                            let unused_fuel = task.fuel_budget.saturating_sub(consumed);
                            
                            if unused_fuel > 0 {
                                // Grant credit to the component for efficient fuel usage
                                let _ = system.grant_credit(
                                    task.component_id, 
                                    unused_fuel,
                                    CreditRestriction::ForComponent { 
                                        component_id: task.component_id 
                                    },
                                );
                            }
                        }
                        
                        self.remove_task_fuel_tracking(task_id);
                        self.polling_stats.tasks_completed += 1;
                        
                        // Update fuel manager history
                        if let Some(ref mut fuel_mgr) = self.fuel_manager {
                            let fuel_consumed = task.fuel_consumed.load(Ordering::Acquire);
                            fuel_mgr.update_task_history(task_id, fuel_consumed, 1, true).ok();
                        }
                    }
                    Ok(None) => {
                        // Task is waiting for async operation or yielded
                        task.state = AsyncTaskState::Waiting;
                        self.polling_stats.tasks_yielded += 1;
                        // Task will be re-added to ready queue when woken
                    }
                    Err(_) => {
                        // Task failed
                        task.state = AsyncTaskState::Failed;
                        self.remove_task_fuel_tracking(task_id);
                        self.polling_stats.tasks_failed += 1;
                    }
                }

                tasks_polled += 1;
                self.polling_stats.total_polls += 1;
                
                // Reset woken flag now that task has been polled
                // This allows future wakes to add the task back to ready queue
                if let Some(waker) = &task.waker {
                    // Extract WakerData to reset the flag
                    // In real implementation, would have a better way to access this
                    // For now, the flag is reset in the wake() method
                }
                
                // Check if we should yield based on fuel consumption
                if fuel_consumed_this_batch >= YIELD_FUEL_THRESHOLD {
                    break;
                }
            }
        }

        Ok(tasks_polled)
    }

    /// Get next ready task with priority consideration
    fn get_next_ready_task(&mut self) -> Option<TaskId> {
        self.ready_queue.lock().ok()?.pop()
    }
    
    /// Wake a task and add it to the ready queue
    pub fn wake_task(&mut self, task_id: TaskId) -> Result<(), Error> {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            if task.state == AsyncTaskState::Waiting {
                // Consume fuel for waking
                self.consume_task_fuel(&task, ASYNC_TASK_WAKE_FUEL)?;
                
                task.state = AsyncTaskState::Ready;
                // Use wake coalescer if available
                if let Some(ref coalescer) = self.wake_coalescer {
                    coalescer.add_wake(task_id)?;
                } else {
                    self.ready_queue.lock()?.push(task_id).map_err(|_| {
                        Error::new(
                            ErrorCategory::Resource,
                            codes::RESOURCE_LIMIT_EXCEEDED,
                            "Ready queue is full".to_string(),
                        )
                    })?;
                }

                record_global_operation(OperationType::ControlFlow, task.verification_level);
            }
        }
        Ok(())
    }

    /// Get task status including fuel information
    pub fn get_task_status(&self, task_id: TaskId) -> Option<AsyncTaskStatus> {
        self.tasks.get(&task_id).map(|task| AsyncTaskStatus {
            id: task.id,
            component_id: task.component_id,
            state: task.state,
            fuel_budget: task.fuel_budget,
            fuel_consumed: task.fuel_consumed.load(Ordering::Acquire),
            priority: task.priority,
            verification_level: task.verification_level,
        })
    }

    /// Get global fuel status
    pub fn get_global_fuel_status(&self) -> GlobalAsyncFuelStatus {
        GlobalAsyncFuelStatus {
            limit: self.global_fuel_limit.load(Ordering::Acquire),
            consumed: self.global_fuel_consumed.load(Ordering::Acquire),
            enforcement_enabled: self.fuel_enforcement.load(Ordering::Acquire),
            active_tasks: self.tasks.len(),
            ready_tasks: self.ready_queue.lock().map(|q| q.len()).unwrap_or(0),
        }
    }

    /// Shutdown the executor gracefully
    pub fn shutdown(&mut self) -> Result<(), Error> {
        self.executor_state = ExecutorState::ShuttingDown;

        // Cancel all remaining tasks
        for (_, task) in self.tasks.iter_mut() {
            if matches!(task.state, AsyncTaskState::Ready | AsyncTaskState::Waiting) {
                task.state = AsyncTaskState::Cancelled;
            }
        }

        // Clear ready queue
        if let Ok(mut queue) = self.ready_queue.lock() {
            queue.clear();
        }
        
        self.executor_state = ExecutorState::Stopped;
        Ok(())
    }

    // Private helper methods

    fn asil_mode_for_priority(&self, priority: Priority) -> ASILExecutionMode {
        match priority {
            Priority::Critical => ASILExecutionMode::D {
                deterministic_execution: true,
                bounded_execution_time: true,
                formal_verification: true,
                max_fuel_per_slice: 1000,
            },
            Priority::High => ASILExecutionMode::C {
                spatial_isolation: true,
                temporal_isolation: true,
                resource_isolation: true,
            },
            Priority::Normal => ASILExecutionMode::B {
                strict_resource_limits: true,
                max_execution_slice_ms: 10,
            },
            Priority::Low => ASILExecutionMode::A {
                error_detection: true,
            },
        }
    }

    fn should_check_fuel(&self, task: &FuelAsyncTask) -> bool {
        self.fuel_enforcement.load(Ordering::Acquire) && task.fuel_budget > 0
    }

    fn consume_task_fuel(&self, task: &FuelAsyncTask, amount: u64) -> Result<(), Error> {
        if !self.should_check_fuel(task) {
            return Ok(());
        }

        // Check if task has sufficient fuel budget
        let consumed = task.fuel_consumed.load(Ordering::Acquire);
        let remaining = task.fuel_budget.saturating_sub(consumed);
        
        if amount > remaining {
            // Not enough fuel - check if debt is allowed
            let deficit = amount - remaining;
            
            if let Some(system) = &self.debt_credit_system {
                // First try to use component credit
                if let Ok(credit_used) = system.use_credit(task.component_id, deficit, task.id) {
                    if credit_used >= deficit {
                        // Credit covered the deficit
                        task.fuel_consumed.fetch_add(amount, Ordering::AcqRel);
                        self.consume_global_fuel(amount)?;
                        return Ok(());
                    }
                    // Partial credit - reduce deficit
                    let remaining_deficit = deficit - credit_used;
                    
                    // Check if we can incur debt for the rest
                    if self.can_incur_debt(task.id, remaining_deficit) {
                        // We'll handle the actual debt incurral after consumption
                        // to ensure atomicity
                    } else {
                        return Err(Error::new(
                            ErrorCategory::Resource,
                            codes::RESOURCE_LIMIT_EXCEEDED,
                            format!("Insufficient fuel: need {} more, no credit/debt available", remaining_deficit),
                        ));
                    }
                } else if !self.can_incur_debt(task.id, deficit) {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_LIMIT_EXCEEDED,
                        format!("Insufficient fuel: need {} more, debt not allowed", deficit),
                    ));
                }
            } else {
                // No debt/credit system - strict enforcement
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    format!("Fuel budget exceeded: need {}, have {}", amount, remaining),
                ));
            }
        }

        // Enforce ASIL-specific fuel policy before consumption
        if let Some(_policy) = &self.fuel_enforcement_policy {
            match self.enforce_asil_fuel_policy(task, amount)? {
                FuelEnforcementDecision::Allow => {
                    // Continue with normal consumption
                },
                FuelEnforcementDecision::Deny { reason } => {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_LIMIT_EXCEEDED,
                        reason,
                    ));
                },
                FuelEnforcementDecision::AllowWithWarning { warning } => {
                    // Log warning but continue
                    // In real implementation, would log: warning
                },
                FuelEnforcementDecision::AllowWithTransfer { transfer_amount, source_component } => {
                    // In real implementation, would transfer fuel from source
                    // For now, just continue
                },
                FuelEnforcementDecision::AllowWithRollover { rollover_amount } => {
                    // In real implementation, would track rollover
                    // For now, just continue
                },
                FuelEnforcementDecision::RequireYield { reason } => {
                    // Task must yield before consuming more fuel
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::EXECUTION_LIMIT_EXCEEDED,
                        format!("Fuel enforcement requires yield: {}", reason),
                    ));
                },
            }
        }

        task.fuel_consumed.fetch_add(amount, Ordering::AcqRel);
        
        // Update fuel monitor if enabled
        // Note: FuelMonitor uses interior mutability (Atomics and Mutex)
        // so it can be called from &self context
        if let Some(monitor) = &self.fuel_monitor {
            monitor.update_consumption(amount, task.id, task.execution_context.asil_mode)?;
        }
        
        self.consume_global_fuel(amount)
    }

    fn consume_global_fuel(&self, amount: u64) -> Result<(), Error> {
        if self.fuel_enforcement.load(Ordering::Acquire) {
            let consumed = self.global_fuel_consumed.fetch_add(amount, Ordering::AcqRel);
            let limit = self.global_fuel_limit.load(Ordering::Acquire);

            if consumed + amount > limit {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::EXECUTION_LIMIT_EXCEEDED,
                    "Global async fuel limit exceeded".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn remove_task_fuel_tracking(&self, task_id: TaskId) {
        if let Some(task) = self.tasks.get(&task_id) {
            let consumed = task.fuel_consumed.load(Ordering::Acquire);
            let remaining = task.fuel_budget.saturating_sub(consumed);

            // Return unused fuel to global pool
            if remaining > 0 && self.fuel_enforcement.load(Ordering::Acquire) {
                self.global_fuel_consumed.fetch_sub(remaining, Ordering::AcqRel);
            }
        }
    }

    /// Execute task using ExecutionContext for real WebAssembly execution
    fn execute_task_with_context(
        &mut self, 
        task_id: TaskId, 
        waker_context: &mut Context<'_>
    ) -> Result<Option<Vec<u8>>, Error> {
        let task = self.tasks.get_mut(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;

        // Check if execution can continue based on ASIL constraints
        if !task.execution_context.can_continue_execution()? {
            // Must yield due to ASIL constraints
            return Ok(None);
        }

        // Get the appropriate ASIL executor
        let asil_key = match task.execution_context.asil_mode {
            ASILExecutionMode::D { .. } => 0,
            ASILExecutionMode::C { .. } => 1,
            ASILExecutionMode::B { .. } => 2,
            ASILExecutionMode::A { .. } => 3,
        };

        // Use ASIL-specific executor if available
        if let Some(asil_executor) = self.asil_executors.get_mut(&asil_key) {
            // Execute using the ASIL-specific executor
            let waker = waker_context.waker().clone();
            match asil_executor.execute_step(task_id, &mut task.execution_context, &waker)? {
                ExecutionStepResult::Completed(result) => {
                    return Ok(Some(result));
                },
                ExecutionStepResult::Yielded => {
                    return Ok(None);
                },
                ExecutionStepResult::Waiting => {
                    task.state = AsyncTaskState::Waiting;
                    return Ok(None);
                },
            }
        }

        // Fallback to original execution if no ASIL executor
        // Consume fuel for execution step
        let step_fuel = match task.execution_context.asil_mode {
            ASILExecutionMode::D { .. } => 10, // Strict fuel accounting for ASIL-D
            ASILExecutionMode::C { .. } => 15,
            ASILExecutionMode::B { .. } => 20,
            ASILExecutionMode::A { .. } => 25,
        };
        
        task.execution_context.consume_fuel(step_fuel);
        self.consume_task_fuel(task, step_fuel)?;

        // Execute WebAssembly function if component instance is available
        if let Some(component_instance) = &task.execution_context.component_instance {
            // Real WebAssembly execution using component instance with fuel integration
            match self.execute_wasm_function_with_fuel(task, component_instance, waker_context) {
                Ok(ExecutionStepResult::Completed(result)) => {
                    return Ok(Some(result));
                },
                Ok(ExecutionStepResult::Yielded) => {
                    // Create yield point for resumable execution
                    task.execution_context.create_yield_point(
                        0, // Would be real instruction pointer
                        vec![], // Would be real stack frame
                        vec![], // Would be real locals
                    )?;
                    return Ok(None);
                },
                Ok(ExecutionStepResult::Waiting) => {
                    return Ok(None);
                },
                Err(e) => {
                    task.execution_context.error_state = Some(e.clone());
                    return Err(e);
                },
            }
        } else {
            // Poll the future as fallback when no component instance
            match task.future.as_mut().poll(waker_context) {
                Poll::Ready(Ok(())) => Ok(Some(vec![])),
                Poll::Ready(Err(e)) => Err(e),
                Poll::Pending => Ok(None),
            }
        }
    }

    /// Execute WebAssembly function using component instance
    fn execute_wasm_function(
        &mut self,
        task: &mut FuelAsyncTask,
        component_instance: &Arc<ComponentInstance>,
        _waker_context: &mut Context<'_>,
    ) -> Result<ExecutionStepResult, Error> {
        // Increment stack depth for this execution step
        task.execution_context.stack_depth += 1;

        // Check stack depth limits
        if task.execution_context.stack_depth >= task.execution_context.max_stack_depth {
            task.execution_context.stack_depth -= 1;
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Stack depth limit exceeded".to_string(),
            ));
        }

        // Execute based on ASIL mode constraints
        let execution_result = match task.execution_context.asil_mode {
            ASILExecutionMode::D { deterministic_execution: true, .. } => {
                // ASIL-D requires deterministic execution
                self.execute_deterministic_step(task, component_instance)
            },
            ASILExecutionMode::C { spatial_isolation: true, .. } => {
                // ASIL-C requires isolation enforcement
                self.execute_isolated_step(task, component_instance)
            },
            ASILExecutionMode::B { strict_resource_limits: true, .. } => {
                // ASIL-B requires resource limit enforcement
                self.execute_resource_limited_step(task, component_instance)
            },
            ASILExecutionMode::A { .. } => {
                // ASIL-A has basic execution requirements
                self.execute_basic_step(task, component_instance)
            },
        };

        // Decrement stack depth after execution
        task.execution_context.stack_depth -= 1;

        execution_result
    }

    /// Execute deterministic step for ASIL-D
    fn execute_deterministic_step(
        &mut self,
        task: &mut FuelAsyncTask,
        _component_instance: &Arc<ComponentInstance>,
    ) -> Result<ExecutionStepResult, Error> {
        // For ASIL-D, execution must be deterministic and bounded
        let fuel_consumed = task.execution_context.context_fuel_consumed.load(Ordering::Acquire);
        
        if let ASILExecutionMode::D { max_fuel_per_slice, .. } = task.execution_context.asil_mode {
            if fuel_consumed >= max_fuel_per_slice {
                // Must yield to maintain deterministic timing
                return Ok(ExecutionStepResult::Yielded);
            }
        }

        // Simulate deterministic WebAssembly execution
        // In real implementation, would:
        // 1. Execute exactly one instruction
        // 2. Update deterministic program counter
        // 3. Update local variables deterministically
        // 4. Check for async yield points
        
        // For now, simulate completion after consuming fuel
        task.execution_context.consume_fuel(50);
        Ok(ExecutionStepResult::Completed(vec![42u8; 8])) // Mock result
    }

    /// Execute isolated step for ASIL-C
    fn execute_isolated_step(
        &mut self,
        task: &mut FuelAsyncTask,
        _component_instance: &Arc<ComponentInstance>,
    ) -> Result<ExecutionStepResult, Error> {
        // For ASIL-C, ensure spatial, temporal, and resource isolation
        
        // Check temporal isolation - no interference from other tasks
        let current_time = task.execution_context.get_deterministic_timestamp();
        
        // Simulate isolated execution step
        task.execution_context.consume_fuel(30);
        
        // Check if we need to yield for temporal isolation
        if current_time % 1000 == 0 { // Yield every 1000 fuel units
            Ok(ExecutionStepResult::Yielded)
        } else {
            Ok(ExecutionStepResult::Completed(vec![24u8; 4])) // Mock result
        }
    }

    /// Execute resource-limited step for ASIL-B
    fn execute_resource_limited_step(
        &mut self,
        task: &mut FuelAsyncTask,
        _component_instance: &Arc<ComponentInstance>,
    ) -> Result<ExecutionStepResult, Error> {
        // For ASIL-B, enforce strict resource limits
        
        if let ASILExecutionMode::B { max_execution_slice_ms, .. } = task.execution_context.asil_mode {
            let fuel_consumed = task.execution_context.context_fuel_consumed.load(Ordering::Acquire);
            let max_fuel = max_execution_slice_ms as u64 * 10; // 10 fuel per ms
            
            if fuel_consumed >= max_fuel {
                return Ok(ExecutionStepResult::Yielded);
            }
        }

        // Simulate resource-limited execution
        task.execution_context.consume_fuel(40);
        Ok(ExecutionStepResult::Completed(vec![36u8; 6])) // Mock result
    }

    /// Execute basic step for ASIL-A
    fn execute_basic_step(
        &mut self,
        task: &mut FuelAsyncTask,
        _component_instance: &Arc<ComponentInstance>,
    ) -> Result<ExecutionStepResult, Error> {
        // For ASIL-A, basic execution with error detection
        
        // Simulate basic execution step
        task.execution_context.consume_fuel(20);
        
        // Basic error detection
        if task.execution_context.stack_depth > 100 {
            return Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                "Stack depth warning".to_string(),
            ));
        }

        Ok(ExecutionStepResult::Completed(vec![18u8; 3])) // Mock result
    }

    /// Get component instance for the given component ID
    fn get_component_instance(&self, component_id: ComponentInstanceId) -> Option<Arc<ComponentInstance>> {
        // In real implementation, would look up component instance from registry
        // For now, return None to use fallback polling
        None
    }

    /// Handle Component Model async operations (task.wait, task.yield, task.poll)
    pub fn handle_component_async_operation(
        &mut self,
        task_id: TaskId,
        operation: ComponentAsyncOperation,
    ) -> Result<ComponentAsyncOperationResult, Error> {
        let task = self.tasks.get_mut(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;

        match operation {
            ComponentAsyncOperation::TaskWait { waitables } => {
                // Handle task.wait - suspend until one of the waitables is ready
                task.execution_context.create_yield_point(
                    0, // Current instruction pointer (would be real in implementation)
                    vec![], // Current stack frame 
                    vec![], // Current local variables
                )?;
                
                task.state = AsyncTaskState::Waiting;
                
                // In real implementation, would register waitables and wake when ready
                Ok(ComponentAsyncOperationResult::Waiting { ready_index: None })
            },
            ComponentAsyncOperation::TaskYield => {
                // Handle task.yield - voluntarily yield execution
                task.execution_context.create_yield_point(
                    0, // Current instruction pointer
                    vec![], // Current stack frame
                    vec![], // Current local variables
                )?;
                
                // Consume fuel for yielding
                self.consume_task_fuel(task, ASYNC_TASK_YIELD_FUEL)?;
                task.execution_context.consume_fuel(ASYNC_TASK_YIELD_FUEL);
                
                // Monitor yield operation
                if let Some(monitor) = &self.fuel_monitor {
                    // Yielding is important for ASIL-D determinism
                    if let ASILExecutionMode::D { .. } = task.execution_context.asil_mode {
                        monitor.update_consumption(
                            ASYNC_TASK_YIELD_FUEL,
                            task_id,
                            task.execution_context.asil_mode
                        )?;
                    }
                }
                
                task.state = AsyncTaskState::Waiting;
                Ok(ComponentAsyncOperationResult::Yielded)
            },
            ComponentAsyncOperation::TaskPoll { waitables } => {
                // Handle task.poll - check waitables without blocking
                
                // Consume fuel for polling
                self.consume_task_fuel(task, ASYNC_TASK_POLL_FUEL)?;
                task.execution_context.consume_fuel(ASYNC_TASK_POLL_FUEL);
                
                // In real implementation, would check waitable readiness
                // For now, simulate no ready waitables
                Ok(ComponentAsyncOperationResult::Polled { ready_index: None })
            },
        }
    }

    /// Resume a task from a yield point
    pub fn resume_task_from_yield_point(&mut self, task_id: TaskId) -> Result<(), Error> {
        let task = self.tasks.get_mut(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;

        // Check if task has a yield point to resume from
        if let Some(yield_point) = &task.execution_context.last_yield_point {
            // Restore execution state from yield point
            self.restore_execution_state_from_yield_point(task, yield_point)?;
            
            // Mark as ready for execution
            task.state = AsyncTaskState::Ready;
            
            // Add back to ready queue
            self.ready_queue.lock()?.push(task_id).map_err(|_| {
                Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    "Ready queue is full".to_string(),
                )
            })?;
        }

        Ok(())
    }

    /// Restore execution state from a yield point
    fn restore_execution_state_from_yield_point(
        &mut self,
        task: &mut FuelAsyncTask,
        yield_point: &YieldPoint,
    ) -> Result<(), Error> {
        // 1. Restore instruction pointer
        // In real WebAssembly execution, would set the program counter
        // For now, we store it in the execution context for tracking
        if let Some(execution_state) = &mut task.execution_context.execution_state {
            // Would restore actual execution state here
            execution_state.restore_state(&[])?; // Would use real saved state
        }

        // 2. Restore stack frame
        // Restore the stack frame values at the time of yield
        for (index, value) in yield_point.stack_frame.iter().enumerate() {
            if index < task.execution_context.stack_depth as usize {
                // In real implementation, would restore actual stack values
                // For now, just track that we're restoring
            }
        }

        // 3. Restore local variables
        // Restore local variable state from yield point
        if let Some(execution_state) = &mut task.execution_context.execution_state {
            execution_state.set_locals(yield_point.locals.clone())?;
        }

        // 4. Restore fuel consumption state
        // Don't double-count fuel that was already consumed before yield
        let fuel_at_yield = yield_point.fuel_at_yield;
        let current_fuel = task.execution_context.context_fuel_consumed.load(Ordering::Acquire);
        
        // Ensure we don't go backwards in fuel consumption
        if current_fuel < fuel_at_yield {
            task.execution_context.context_fuel_consumed.store(fuel_at_yield, Ordering::Release);
        }

        // 5. Validate deterministic execution for ASIL-D
        if let ASILExecutionMode::D { deterministic_execution: true, .. } = task.execution_context.asil_mode {
            // Verify deterministic timestamp consistency
            let current_timestamp = task.execution_context.get_deterministic_timestamp();
            if current_timestamp < yield_point.yield_timestamp {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::EXECUTION_ERROR,
                    "Deterministic execution violation during resume".to_string(),
                ));
            }
        }

        // 6. Clear the yield point since we've resumed
        task.execution_context.last_yield_point = None;

        Ok(())
    }

    /// Set component instance for real WebAssembly execution
    pub fn set_component_instance_for_task(
        &mut self,
        task_id: TaskId,
        component_instance: Arc<ComponentInstance>,
    ) -> Result<(), Error> {
        let task = self.tasks.get_mut(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;

        task.execution_context.set_component_instance(component_instance);
        Ok(())
    }

    /// Execute WebAssembly function with fuel integration using StacklessEngine
    fn execute_wasm_function_with_fuel(
        &mut self,
        task: &mut FuelAsyncTask,
        component_instance: &Arc<ComponentInstance>,
        _waker_context: &mut Context<'_>,
    ) -> Result<ExecutionStepResult, Error> {
        // Create a StacklessEngine for WebAssembly execution
        let mut engine = wrt_runtime::stackless::engine::StacklessEngine::new();
        
        // Set fuel limit based on task's remaining fuel budget
        let consumed = task.fuel_consumed.load(Ordering::Acquire);
        let remaining_fuel = task.fuel_budget.saturating_sub(consumed);
        
        if remaining_fuel == 0 {
            return Ok(ExecutionStepResult::Yielded);
        }
        
        // Set fuel for the engine - this integrates with instruction-level fuel consumption
        engine.set_fuel(Some(remaining_fuel));
        
        // Set verification level to match task's ASIL mode
        let verification_level = match task.execution_context.asil_mode {
            ASILExecutionMode::D { .. } => wrt_foundation::verification::VerificationLevel::Full,
            ASILExecutionMode::C { .. } => wrt_foundation::verification::VerificationLevel::Standard,
            ASILExecutionMode::B { .. } => wrt_foundation::verification::VerificationLevel::Basic,
            ASILExecutionMode::A { .. } => wrt_foundation::verification::VerificationLevel::Off,
        };
        
        // Execute a limited number of WebAssembly instructions
        let max_instructions_per_step = match task.execution_context.asil_mode {
            ASILExecutionMode::D { .. } => 10,  // Very limited execution for ASIL-D
            ASILExecutionMode::C { .. } => 25,  // Moderate execution for ASIL-C
            ASILExecutionMode::B { .. } => 50,  // More execution for ASIL-B
            ASILExecutionMode::A { .. } => 100, // Relaxed execution for ASIL-A
        };
        
        // Simulate WebAssembly execution step
        // In a real implementation, this would:
        // 1. Get the function to execute from component_instance
        // 2. Execute instructions using engine.execute_step()
        // 3. Handle fuel exhaustion and yield points
        // 4. Return execution results
        
        // For now, simulate instruction execution with proper fuel consumption
        let mut instructions_executed = 0;
        let initial_fuel = engine.remaining_fuel().unwrap_or(0);
        
        // Simulate executing instructions until fuel runs low or max instructions reached
        while instructions_executed < max_instructions_per_step {
            // Check remaining fuel
            if let Some(current_fuel) = engine.remaining_fuel() {
                if current_fuel < 5 { // Reserve some fuel for cleanup
                    break;
                }
            }
            
            // Simulate different instruction types with proper fuel consumption
            let instruction_type = match instructions_executed % 4 {
                0 => wrt_runtime::stackless::engine::InstructionFuelType::LocalAccess,
                1 => wrt_runtime::stackless::engine::InstructionFuelType::SimpleArithmetic,
                2 => wrt_runtime::stackless::engine::InstructionFuelType::Comparison,
                _ => wrt_runtime::stackless::engine::InstructionFuelType::SimpleControl,
            };
            
            // This calls the actual fuel consumption in the engine
            match engine.consume_instruction_fuel(instruction_type) {
                Ok(()) => {
                    instructions_executed += 1;
                    // Simulate actual instruction execution here
                },
                Err(_) => {
                    // Fuel exhausted during instruction execution
                    break;
                }
            }
        }
        
        // Update task fuel consumption based on what the engine consumed
        let final_fuel = engine.remaining_fuel().unwrap_or(0);
        let fuel_consumed_this_step = initial_fuel.saturating_sub(final_fuel);
        
        // Update task fuel consumption
        task.fuel_consumed.fetch_add(fuel_consumed_this_step, Ordering::AcqRel);
        task.execution_context.context_fuel_consumed.fetch_add(fuel_consumed_this_step, Ordering::AcqRel);
        
        // Determine execution result
        if instructions_executed == max_instructions_per_step {
            // Completed execution step successfully
            Ok(ExecutionStepResult::Completed(vec![42u8; 4])) // Mock result data
        } else if fuel_consumed_this_step > 0 {
            // Yielded due to fuel exhaustion
            Ok(ExecutionStepResult::Yielded)
        } else {
            // No progress made - likely an error condition
            Ok(ExecutionStepResult::Waiting)
        }
    }

    /// Get current execution state for debugging/monitoring
    pub fn get_execution_state(&self, task_id: TaskId) -> Result<ExecutionStateInfo, Error> {
        let task = self.tasks.get(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;

        Ok(ExecutionStateInfo {
            task_id,
            component_id: task.component_id,
            asil_mode: task.execution_context.asil_mode,
            stack_depth: task.execution_context.stack_depth,
            max_stack_depth: task.execution_context.max_stack_depth,
            fuel_consumed: task.execution_context.context_fuel_consumed.load(Ordering::Acquire),
            has_yield_point: task.execution_context.last_yield_point.is_some(),
            has_component_instance: task.execution_context.component_instance.is_some(),
            error_state: task.execution_context.error_state.clone(),
        })
    }

    /// Enable fuel debt/credit system with configuration
    pub fn enable_debt_credit_system(&mut self, config: Option<DebtCreditConfig>) -> Result<(), Error> {
        let config = config.unwrap_or_default();
        
        let system = FuelDebtCreditSystem::new(
            config.max_concurrent_debtors,
            config.max_concurrent_creditors,
            config.global_debt_limit,
            config.global_credit_limit,
        )?;
        
        self.debt_credit_system = Some(system);
        Ok(())
    }

    /// Check if a task can incur debt
    pub fn can_incur_debt(&self, task_id: TaskId, amount: u64) -> bool {
        if let Some(system) = &self.debt_credit_system {
            if let Some(task) = self.tasks.get(&task_id) {
                // Get debt policy based on ASIL mode
                let policy = match task.execution_context.asil_mode {
                    ASILExecutionMode::D { .. } => DebtPolicy::NeverAllow,
                    ASILExecutionMode::C { .. } => DebtPolicy::LimitedDebt { max_debt: 1000 },
                    ASILExecutionMode::B { .. } => DebtPolicy::ModerateDebt { 
                        max_debt: 5000, 
                        interest_rate: 0.05 
                    },
                    ASILExecutionMode::A { .. } => DebtPolicy::FlexibleDebt { 
                        soft_limit: 10000, 
                        hard_limit: 20000, 
                        interest_rate: 0.10 
                    },
                };
                
                system.can_incur_debt(task_id, amount, &policy)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Incur fuel debt for a task
    pub fn incur_fuel_debt(&mut self, task_id: TaskId, amount: u64) -> Result<(), Error> {
        let system = self.debt_credit_system.as_mut().ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_STATE,
                "Debt/credit system not enabled".to_string(),
            )
        })?;

        let task = self.tasks.get(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;

        // Get debt policy based on ASIL mode
        let policy = match task.execution_context.asil_mode {
            ASILExecutionMode::D { .. } => DebtPolicy::NeverAllow,
            ASILExecutionMode::C { .. } => DebtPolicy::LimitedDebt { max_debt: 1000 },
            ASILExecutionMode::B { .. } => DebtPolicy::ModerateDebt { 
                max_debt: 5000, 
                interest_rate: 0.05 
            },
            ASILExecutionMode::A { .. } => DebtPolicy::FlexibleDebt { 
                soft_limit: 10000, 
                hard_limit: 20000, 
                interest_rate: 0.10 
            },
        };

        system.incur_debt(task_id, amount, &policy)
    }

    /// Grant fuel credit to a component
    pub fn grant_fuel_credit(
        &mut self, 
        component_id: ComponentInstanceId, 
        amount: u64,
        restrictions: Option<CreditRestriction>,
    ) -> Result<(), Error> {
        let system = self.debt_credit_system.as_mut().ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_STATE,
                "Debt/credit system not enabled".to_string(),
            )
        })?;

        let restrictions = restrictions.unwrap_or(CreditRestriction::None);
        system.grant_credit(component_id, amount, restrictions)
    }

    /// Check debt/credit balance for a task
    pub fn get_debt_credit_balance(&self, task_id: TaskId) -> DebtCreditBalance {
        if let Some(system) = &self.debt_credit_system {
            if let Some(task) = self.tasks.get(&task_id) {
                let debt = system.get_task_debt(task_id);
                let credit = system.get_component_credit(task.component_id);
                DebtCreditBalance {
                    task_id,
                    component_id: task.component_id,
                    current_debt: debt,
                    available_credit: credit,
                    net_balance: credit as i64 - debt as i64,
                }
            } else {
                DebtCreditBalance::default_for_task(task_id)
            }
        } else {
            DebtCreditBalance::default_for_task(task_id)
        }
    }
    
    /// Allocate additional fuel to a task and handle debt repayment
    pub fn allocate_additional_fuel(&mut self, task_id: TaskId, additional_fuel: u64) -> Result<u64, Error> {
        let task = self.tasks.get_mut(&task_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::INVALID_INPUT,
                "Task not found".to_string(),
            )
        })?;
        
        let mut fuel_to_allocate = additional_fuel;
        
        // First, check if task has debt
        if let Some(system) = &mut self.debt_credit_system {
            let current_debt = system.get_task_debt(task_id);
            
            if current_debt > 0 {
                // Calculate how much fuel goes to debt repayment
                let debt_payment = fuel_to_allocate.min(current_debt);
                
                // Repay debt with interest
                let interest_rate = match task.execution_context.asil_mode {
                    ASILExecutionMode::D { .. } => 0.0, // No interest for ASIL-D (shouldn't have debt)
                    ASILExecutionMode::C { .. } => 0.02, // 2% interest for ASIL-C
                    ASILExecutionMode::B { .. } => 0.05, // 5% interest for ASIL-B
                    ASILExecutionMode::A { .. } => 0.10, // 10% interest for ASIL-A
                };
                
                system.repay_debt(task_id, debt_payment, interest_rate)?;
                
                // Reduce fuel available for allocation
                fuel_to_allocate = fuel_to_allocate.saturating_sub(debt_payment);
            }
        }
        
        // Allocate remaining fuel to task budget
        task.fuel_budget = task.fuel_budget.saturating_add(fuel_to_allocate);
        
        Ok(fuel_to_allocate)
    }
    
    /// Get debt/credit system statistics
    pub fn get_debt_credit_stats(&self) -> Option<DebtCreditStats> {
        self.debt_credit_system.as_ref().map(|system| {
            DebtCreditStats {
                total_debt: system.get_total_debt(),
                total_credit: system.get_total_credit(),
                active_debtors: system.get_active_debtors(),
                active_creditors: system.get_active_creditors(),
            }
        })
    }
}

impl Default for FuelAsyncExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create default FuelAsyncExecutor")
    }
}

/// Status information for an async task
#[derive(Debug, Clone)]
pub struct AsyncTaskStatus {
    pub id: TaskId,
    pub component_id: ComponentInstanceId,
    pub state: AsyncTaskState,
    pub fuel_budget: u64,
    pub fuel_consumed: u64,
    pub priority: Priority,
    pub verification_level: VerificationLevel,
}

/// Global fuel status for the async executor
#[derive(Debug, Clone)]
pub struct GlobalAsyncFuelStatus {
    pub limit: u64,
    pub consumed: u64,
    pub enforcement_enabled: bool,
    pub active_tasks: usize,
    pub ready_tasks: usize,
}

impl GlobalAsyncFuelStatus {
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

/// Polling statistics for monitoring executor performance
#[derive(Debug, Default, Clone)]
pub struct PollingStatistics {
    pub total_polls: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_yielded: u64,
    pub wakes_coalesced: usize,
}

/// Active fuel monitoring for real-time fuel consumption tracking
#[derive(Debug)]
pub struct FuelMonitor {
    /// Current fuel consumption rate (fuel per ms)
    current_rate: AtomicU64,
    /// Peak fuel consumption rate observed
    peak_rate: AtomicU64,
    /// Total fuel consumed in current monitoring window
    window_fuel_consumed: AtomicU64,
    /// Monitoring window start time (in fuel units for determinism)
    window_start: AtomicU64,
    /// ASIL-specific fuel thresholds
    asil_thresholds: ASILFuelThresholds,
    /// Fuel consumption history for trend analysis
    consumption_history: Mutex<BoundedVec<FuelConsumptionRecord, 128>>,
    /// Active alerts for fuel issues
    active_alerts: Mutex<BoundedVec<FuelAlert, 32>>,
}

/// ASIL-specific fuel thresholds
#[derive(Debug, Clone)]
pub struct ASILFuelThresholds {
    /// ASIL-D: Strict deterministic fuel limit per task
    asil_d_task_limit: u64,
    /// ASIL-C: Isolated fuel budget per component
    asil_c_component_limit: u64,
    /// ASIL-B: Resource-limited fuel per time slice
    asil_b_slice_limit: u64,
    /// ASIL-A: Basic fuel warning threshold
    asil_a_warning_threshold: u64,
}

/// ASIL fuel enforcement policy
#[derive(Debug, Clone)]
pub struct ASILFuelEnforcementPolicy {
    /// Enable strict enforcement (fail fast)
    pub strict_enforcement: bool,
    /// Enable fuel borrowing between tasks
    pub allow_fuel_borrowing: bool,
    /// Enable emergency fuel reserves
    pub emergency_reserves_enabled: bool,
    /// Emergency reserve fuel amount
    pub emergency_reserve_amount: u64,
    /// ASIL-specific policies
    pub asil_policies: ASILSpecificPolicies,
}

/// ASIL-specific enforcement policies
#[derive(Debug, Clone)]
pub struct ASILSpecificPolicies {
    /// ASIL-D: Deterministic fuel allocation
    pub asil_d: ASILDPolicy,
    /// ASIL-C: Isolated fuel pools
    pub asil_c: ASILCPolicy,
    /// ASIL-B: Bounded fuel consumption
    pub asil_b: ASILBPolicy,
    /// ASIL-A: Flexible fuel management
    pub asil_a: ASILAPolicy,
}

/// ASIL-D specific fuel policy
#[derive(Debug, Clone)]
pub struct ASILDPolicy {
    /// Fuel quantum for deterministic allocation
    pub fuel_quantum: u64,
    /// Maximum fuel per execution step
    pub max_step_fuel: u64,
    /// Enforce deterministic fuel ordering
    pub enforce_deterministic_ordering: bool,
    /// Preallocation required
    pub require_preallocation: bool,
}

/// ASIL-C specific fuel policy
#[derive(Debug, Clone)]
pub struct ASILCPolicy {
    /// Component fuel isolation enabled
    pub component_isolation: bool,
    /// Maximum inter-component fuel transfer
    pub max_transfer_amount: u64,
    /// Temporal fuel windows
    pub temporal_window_ms: u64,
}

/// ASIL-B specific fuel policy
#[derive(Debug, Clone)]
pub struct ASILBPolicy {
    /// Fuel budget per time slice
    pub slice_fuel_budget: u64,
    /// Allow fuel rollover between slices
    pub allow_rollover: bool,
    /// Maximum rollover percentage
    pub max_rollover_percent: u32,
}

/// ASIL-A specific fuel policy
#[derive(Debug, Clone)]
pub struct ASILAPolicy {
    /// Soft limit before warnings
    pub soft_limit: u64,
    /// Hard limit before failure
    pub hard_limit: u64,
    /// Grace period for exceeding soft limit
    pub grace_period_ms: u64,
}

/// Fuel consumption record for history tracking
#[derive(Debug, Clone)]
pub struct FuelConsumptionRecord {
    /// Timestamp (in fuel units for determinism)
    timestamp: u64,
    /// Fuel consumed in this period
    fuel_consumed: u64,
    /// Number of tasks active
    active_tasks: u32,
    /// ASIL mode of highest priority task
    highest_asil_mode: ASILExecutionMode,
}

/// Fuel alert for monitoring
#[derive(Debug, Clone)]
pub enum FuelAlert {
    /// Task approaching fuel limit
    TaskApproachingLimit { task_id: TaskId, remaining_fuel: u64 },
    /// Component exceeding budget
    ComponentExceedingBudget { component_id: ComponentInstanceId, overage: u64 },
    /// Global fuel consumption spike
    ConsumptionSpike { rate: u64, threshold: u64 },
    /// ASIL violation detected
    ASILViolation { mode: ASILExecutionMode, violation_type: String },
}

/// Fuel enforcement decision
#[derive(Debug, Clone)]
pub enum FuelEnforcementDecision {
    /// Allow fuel consumption
    Allow,
    /// Deny fuel consumption
    Deny { reason: String },
    /// Allow with warning
    AllowWithWarning { warning: String },
    /// Allow with fuel transfer from another component
    AllowWithTransfer { transfer_amount: u64, source_component: Option<ComponentInstanceId> },
    /// Allow with rollover from previous time slice
    AllowWithRollover { rollover_amount: u64 },
    /// Require task to yield before continuing
    RequireYield { reason: String },
    /// Allow with debt (must be repaid)
    AllowWithDebt { debt_amount: u64 },
}

/// Fuel debt and credit management system
#[derive(Debug)]
pub struct FuelDebtCreditSystem {
    /// Task fuel debts
    task_debts: Mutex<BoundedHashMap<TaskId, FuelDebt, 128>>,
    /// Component fuel credits
    component_credits: Mutex<BoundedHashMap<ComponentInstanceId, FuelCredit, 64>>,
    /// Global credit pool for emergency use
    global_credit_pool: AtomicU64,
    /// Total outstanding debt
    total_debt: AtomicU64,
    /// Debt/credit configuration
    config: DebtCreditConfig,
    /// Debt collection statistics
    collection_stats: DebtCollectionStats,
}

/// Fuel debt record
#[derive(Debug, Clone)]
pub struct FuelDebt {
    /// Task that owes fuel
    task_id: TaskId,
    /// Amount of fuel owed
    amount: u64,
    /// When debt was incurred (fuel timestamp)
    incurred_at: u64,
    /// Interest rate (for ASIL-A/B only)
    interest_rate: f32,
    /// Priority for repayment
    repayment_priority: DebtPriority,
    /// ASIL mode of debtor
    asil_mode: ASILExecutionMode,
}

/// Fuel credit record
#[derive(Debug, Clone)]
pub struct FuelCredit {
    /// Component that has credit
    component_id: ComponentInstanceId,
    /// Amount of fuel credit available
    amount: u64,
    /// Expiration time (0 = no expiration)
    expires_at: u64,
    /// Source of credit
    source: CreditSource,
    /// Restrictions on credit use
    restrictions: CreditRestrictions,
}

/// Debt priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebtPriority {
    /// Critical - must be repaid immediately
    Critical,
    /// High - repay as soon as possible
    High,
    /// Normal - standard repayment
    Normal,
    /// Low - can be deferred
    Low,
}

/// Source of fuel credit
#[derive(Debug, Clone)]
pub enum CreditSource {
    /// Unused fuel returned from completed task
    TaskCompletion { task_id: TaskId },
    /// Fuel saved through optimization
    Optimization { savings_amount: u64 },
    /// Granted by system for good behavior
    SystemGrant { reason: String },
    /// Transferred from another component
    Transfer { from_component: ComponentInstanceId },
}

/// Restrictions on credit usage
#[derive(Debug, Clone)]
pub struct CreditRestrictions {
    /// Only usable by specific ASIL levels
    asil_restrictions: Option<Vec<ASILExecutionMode>>,
    /// Only usable for specific task types
    task_type_restrictions: Option<Vec<ComponentAsyncOperation>>,
    /// Maximum amount per use
    max_per_use: Option<u64>,
    /// Minimum task priority to use
    min_priority: Option<Priority>,
}

/// Debt and credit configuration
#[derive(Debug, Clone)]
pub struct DebtCreditConfig {
    /// Enable debt system
    pub debt_enabled: bool,
    /// Maximum debt per task
    pub max_debt_per_task: u64,
    /// Maximum total system debt
    pub max_total_debt: u64,
    /// Debt interest rate (per 1000 fuel units of time)
    pub debt_interest_rate: f32,
    /// Enable credit system
    pub credit_enabled: bool,
    /// Credit expiration time (0 = no expiration)
    pub credit_expiration_time: u64,
    /// ASIL-specific debt policies
    pub asil_debt_policies: ASILDebtPolicies,
}

/// ASIL-specific debt policies
#[derive(Debug, Clone)]
pub struct ASILDebtPolicies {
    /// ASIL-D: No debt allowed
    pub asil_d_allow_debt: bool,
    /// ASIL-C: Limited debt with strict repayment
    pub asil_c_max_debt: u64,
    /// ASIL-B: Moderate debt with interest
    pub asil_b_max_debt: u64,
    /// ASIL-A: Flexible debt terms
    pub asil_a_max_debt: u64,
}

/// Debt/credit balance information
#[derive(Debug, Clone)]
pub struct DebtCreditBalance {
    /// Task ID
    pub task_id: TaskId,
    /// Component ID
    pub component_id: ComponentInstanceId,
    /// Current debt amount
    pub current_debt: u64,
    /// Available credit
    pub available_credit: u64,
    /// Net balance (can be negative)
    pub net_balance: i64,
}

impl DebtCreditBalance {
    /// Create default balance for a task
    pub fn default_for_task(task_id: TaskId) -> Self {
        Self {
            task_id,
            component_id: ComponentInstanceId::new(0),
            current_debt: 0,
            available_credit: 0,
            net_balance: 0,
        }
    }
}

/// Debt/credit system statistics
#[derive(Debug, Clone)]
pub struct DebtCreditStats {
    /// Total outstanding debt
    pub total_debt: u64,
    /// Total available credit
    pub total_credit: u64,
    /// Number of tasks with debt
    pub active_debtors: usize,
    /// Number of components with credit
    pub active_creditors: usize,
}

/// Debt collection statistics
#[derive(Debug, Default)]
pub struct DebtCollectionStats {
    /// Total debt incurred
    total_incurred: AtomicU64,
    /// Total debt repaid
    total_repaid: AtomicU64,
    /// Total interest collected
    total_interest: AtomicU64,
    /// Number of defaults
    defaults: AtomicU32,
}

impl FuelDebtCreditSystem {
    /// Create a new fuel debt/credit system
    pub fn new(config: DebtCreditConfig) -> Result<Self, Error> {
        let provider = safe_managed_alloc!(8192, CrateId::Component)?;
        
        Ok(Self {
            task_debts: Mutex::new(BoundedHashMap::new()),
            component_credits: Mutex::new(BoundedHashMap::new()),
            global_credit_pool: AtomicU64::new(config.asil_debt_policies.asil_a_max_debt),
            total_debt: AtomicU64::new(0),
            config,
            collection_stats: DebtCollectionStats::default(),
        })
    }

    /// Incur debt for a task
    pub fn incur_debt(
        &self,
        task_id: TaskId,
        amount: u64,
        asil_mode: ASILExecutionMode,
    ) -> Result<bool, Error> {
        // Check ASIL-specific debt policies
        let max_allowed = match asil_mode {
            ASILExecutionMode::D { .. } => {
                if !self.config.asil_debt_policies.asil_d_allow_debt {
                    return Ok(false); // No debt for ASIL-D
                }
                0
            },
            ASILExecutionMode::C { .. } => self.config.asil_debt_policies.asil_c_max_debt,
            ASILExecutionMode::B { .. } => self.config.asil_debt_policies.asil_b_max_debt,
            ASILExecutionMode::A { .. } => self.config.asil_debt_policies.asil_a_max_debt,
        };

        if amount > max_allowed {
            return Ok(false);
        }

        // Check total debt limit
        let current_total = self.total_debt.load(Ordering::Acquire);
        if current_total + amount > self.config.max_total_debt {
            return Ok(false);
        }

        // Record the debt
        let mut debts = self.task_debts.lock()?;
        
        // Check existing debt
        if let Some(existing) = debts.get(&task_id) {
            if existing.amount + amount > self.config.max_debt_per_task {
                return Ok(false);
            }
        }

        let debt = FuelDebt {
            task_id,
            amount,
            incurred_at: self.get_current_time(),
            interest_rate: self.get_interest_rate(asil_mode),
            repayment_priority: self.get_debt_priority(asil_mode),
            asil_mode,
        };

        debts.insert(task_id, debt).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many task debts".to_string(),
            )
        })?;

        self.total_debt.fetch_add(amount, Ordering::AcqRel);
        self.collection_stats.total_incurred.fetch_add(amount, Ordering::Relaxed);

        Ok(true)
    }

    /// Repay debt for a task
    pub fn repay_debt(&self, task_id: TaskId, amount: u64) -> Result<u64, Error> {
        let mut debts = self.task_debts.lock()?;
        
        if let Some(debt) = debts.get_mut(&task_id) {
            let current_time = self.get_current_time();
            let time_elapsed = current_time.saturating_sub(debt.incurred_at);
            
            // Calculate interest
            let interest = if debt.interest_rate > 0.0 {
                ((debt.amount as f32 * debt.interest_rate * time_elapsed as f32) / 1000.0) as u64
            } else {
                0
            };
            
            let total_owed = debt.amount + interest;
            let repaid = amount.min(total_owed);
            
            if repaid >= total_owed {
                // Debt fully repaid
                debts.remove(&task_id);
                self.total_debt.fetch_sub(debt.amount, Ordering::AcqRel);
            } else {
                // Partial repayment
                debt.amount = total_owed - repaid;
            }
            
            self.collection_stats.total_repaid.fetch_add(repaid, Ordering::Relaxed);
            if interest > 0 {
                self.collection_stats.total_interest.fetch_add(interest, Ordering::Relaxed);
            }
            
            Ok(repaid)
        } else {
            Ok(0) // No debt to repay
        }
    }

    /// Grant credit to a component
    pub fn grant_credit(
        &self,
        component_id: ComponentInstanceId,
        amount: u64,
        source: CreditSource,
        restrictions: CreditRestrictions,
    ) -> Result<(), Error> {
        let mut credits = self.component_credits.lock()?;
        
        let expires_at = if self.config.credit_expiration_time > 0 {
            self.get_current_time() + self.config.credit_expiration_time
        } else {
            0 // No expiration
        };

        let credit = FuelCredit {
            component_id,
            amount,
            expires_at,
            source,
            restrictions,
        };

        credits.insert(component_id, credit).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many component credits".to_string(),
            )
        })?;

        Ok(())
    }

    /// Use credit for a task
    pub fn use_credit(
        &self,
        component_id: ComponentInstanceId,
        task_id: TaskId,
        amount: u64,
        task_priority: Priority,
        asil_mode: ASILExecutionMode,
    ) -> Result<u64, Error> {
        let mut credits = self.component_credits.lock()?;
        
        if let Some(credit) = credits.get_mut(&component_id) {
            // Check expiration
            if credit.expires_at > 0 && self.get_current_time() > credit.expires_at {
                credits.remove(&component_id);
                return Ok(0); // Credit expired
            }

            // Check restrictions
            if !self.check_credit_restrictions(&credit.restrictions, task_priority, asil_mode) {
                return Ok(0); // Restrictions not met
            }

            let available = credit.amount.min(amount);
            if let Some(max_per_use) = credit.restrictions.max_per_use {
                let usable = available.min(max_per_use);
                credit.amount -= usable;
                
                if credit.amount == 0 {
                    credits.remove(&component_id);
                }
                
                Ok(usable)
            } else {
                credit.amount -= available;
                
                if credit.amount == 0 {
                    credits.remove(&component_id);
                }
                
                Ok(available)
            }
        } else {
            // Try global credit pool
            let available = self.global_credit_pool.load(Ordering::Acquire);
            let usable = available.min(amount);
            
            if usable > 0 {
                self.global_credit_pool.fetch_sub(usable, Ordering::AcqRel);
            }
            
            Ok(usable)
        }
    }

    /// Get debt status for a task
    pub fn get_debt_status(&self, task_id: TaskId) -> Option<FuelDebt> {
        self.task_debts.lock().ok()?.get(&task_id).cloned()
    }

    /// Get total outstanding debt
    pub fn get_total_debt(&self) -> u64 {
        self.total_debt.load(Ordering::Acquire)
    }

    /// Process expired credits
    pub fn process_expired_credits(&self) -> Result<u32, Error> {
        let mut credits = self.component_credits.lock()?;
        let current_time = self.get_current_time();
        let mut expired_count = 0;

        credits.retain(|_, credit| {
            if credit.expires_at > 0 && current_time > credit.expires_at {
                expired_count += 1;
                false
            } else {
                true
            }
        });

        Ok(expired_count)
    }

    // Helper methods

    fn get_interest_rate(&self, asil_mode: ASILExecutionMode) -> f32 {
        match asil_mode {
            ASILExecutionMode::D { .. } => 0.0,  // No interest for ASIL-D
            ASILExecutionMode::C { .. } => self.config.debt_interest_rate * 0.5, // Half rate
            ASILExecutionMode::B { .. } => self.config.debt_interest_rate,
            ASILExecutionMode::A { .. } => self.config.debt_interest_rate * 1.5, // Higher rate
        }
    }

    fn get_debt_priority(&self, asil_mode: ASILExecutionMode) -> DebtPriority {
        match asil_mode {
            ASILExecutionMode::D { .. } => DebtPriority::Critical,
            ASILExecutionMode::C { .. } => DebtPriority::High,
            ASILExecutionMode::B { .. } => DebtPriority::Normal,
            ASILExecutionMode::A { .. } => DebtPriority::Low,
        }
    }

    fn check_credit_restrictions(
        &self,
        restrictions: &CreditRestrictions,
        task_priority: Priority,
        asil_mode: ASILExecutionMode,
    ) -> bool {
        // Check ASIL restrictions
        if let Some(asil_restrictions) = &restrictions.asil_restrictions {
            if !asil_restrictions.iter().any(|&mode| 
                std::mem::discriminant(&mode) == std::mem::discriminant(&asil_mode)
            ) {
                return false;
            }
        }

        // Check priority restrictions
        if let Some(min_priority) = restrictions.min_priority {
            if task_priority < min_priority {
                return false;
            }
        }

        true
    }

    fn get_current_time(&self) -> u64 {
        global_fuel_consumed()
    }
}

impl Default for DebtCreditConfig {
    fn default() -> Self {
        Self {
            debt_enabled: true,
            max_debt_per_task: 5000,      // 5ms worth of fuel
            max_total_debt: 50000,        // 50ms total system debt
            debt_interest_rate: 0.01,     // 1% per 1000 fuel units
            credit_enabled: true,
            credit_expiration_time: 10000, // Credits expire after 10ms
            asil_debt_policies: ASILDebtPolicies::default(),
        }
    }
}

impl Default for ASILDebtPolicies {
    fn default() -> Self {
        Self {
            asil_d_allow_debt: false,     // No debt for ASIL-D
            asil_c_max_debt: 1000,        // 1ms max debt for ASIL-C
            asil_b_max_debt: 3000,        // 3ms max debt for ASIL-B
            asil_a_max_debt: 10000,       // 10ms max debt for ASIL-A
        }
    }
}

impl Default for CreditRestrictions {
    fn default() -> Self {
        Self {
            asil_restrictions: None,
            task_type_restrictions: None,
            max_per_use: None,
            min_priority: None,
        }
    }
}

impl FuelMonitor {
    /// Create a new fuel monitor with default thresholds
    pub fn new() -> Result<Self, Error> {
        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        
        Ok(Self {
            current_rate: AtomicU64::new(0),
            peak_rate: AtomicU64::new(0),
            window_fuel_consumed: AtomicU64::new(0),
            window_start: AtomicU64::new(0),
            asil_thresholds: ASILFuelThresholds::default(),
            consumption_history: Mutex::new(BoundedVec::new(provider.clone())?),
            active_alerts: Mutex::new(BoundedVec::new(provider)?),
        })
    }

    /// Update fuel consumption and check thresholds
    pub fn update_consumption(&self, fuel_consumed: u64, task_id: TaskId, asil_mode: ASILExecutionMode) -> Result<(), Error> {
        // Update window consumption
        self.window_fuel_consumed.fetch_add(fuel_consumed, Ordering::AcqRel);
        
        // Calculate current rate
        let window_start = self.window_start.load(Ordering::Acquire);
        let current_time = self.get_current_time();
        let window_duration = current_time.saturating_sub(window_start).max(1);
        let total_consumed = self.window_fuel_consumed.load(Ordering::Acquire);
        let current_rate = (total_consumed * 1000) / window_duration; // Per ms
        
        self.current_rate.store(current_rate, Ordering::Release);
        
        // Update peak rate
        let peak = self.peak_rate.load(Ordering::Acquire);
        if current_rate > peak {
            self.peak_rate.store(current_rate, Ordering::Release);
        }
        
        // Check ASIL-specific thresholds
        self.check_asil_thresholds(fuel_consumed, task_id, asil_mode)?;
        
        // Reset window if needed (every 1000 fuel units)
        if window_duration > 1000 {
            self.reset_window(current_time);
        }
        
        Ok(())
    }

    /// Check ASIL-specific fuel thresholds
    fn check_asil_thresholds(&self, fuel_consumed: u64, task_id: TaskId, asil_mode: ASILExecutionMode) -> Result<(), Error> {
        match asil_mode {
            ASILExecutionMode::D { max_fuel_per_slice, .. } => {
                if fuel_consumed > max_fuel_per_slice * 80 / 100 { // 80% threshold
                    self.add_alert(FuelAlert::TaskApproachingLimit {
                        task_id,
                        remaining_fuel: max_fuel_per_slice.saturating_sub(fuel_consumed),
                    })?;
                }
            },
            ASILExecutionMode::C { .. } => {
                if fuel_consumed > self.asil_thresholds.asil_c_component_limit * 90 / 100 {
                    self.add_alert(FuelAlert::ASILViolation {
                        mode: asil_mode,
                        violation_type: "Component fuel limit approaching".to_string(),
                    })?;
                }
            },
            ASILExecutionMode::B { .. } => {
                if fuel_consumed > self.asil_thresholds.asil_b_slice_limit {
                    self.add_alert(FuelAlert::ASILViolation {
                        mode: asil_mode,
                        violation_type: "Slice fuel limit exceeded".to_string(),
                    })?;
                }
            },
            ASILExecutionMode::A { .. } => {
                if fuel_consumed > self.asil_thresholds.asil_a_warning_threshold {
                    // Just a warning for ASIL-A
                }
            },
        }
        Ok(())
    }

    /// Add a fuel alert
    fn add_alert(&self, alert: FuelAlert) -> Result<(), Error> {
        if let Ok(mut alerts) = self.active_alerts.lock() {
            // Avoid duplicate alerts
            let exists = alerts.iter().any(|a| matches!((a, &alert), 
                (FuelAlert::TaskApproachingLimit { task_id: id1, .. }, FuelAlert::TaskApproachingLimit { task_id: id2, .. }) if id1 == id2
            ));
            
            if !exists {
                alerts.push(alert).ok();
            }
        }
        Ok(())
    }

    /// Reset monitoring window
    fn reset_window(&self, current_time: u64) {
        // Save history record
        if let Ok(mut history) = self.consumption_history.lock() {
            let record = FuelConsumptionRecord {
                timestamp: current_time,
                fuel_consumed: self.window_fuel_consumed.load(Ordering::Acquire),
                active_tasks: 0, // Would be tracked separately
                highest_asil_mode: ASILExecutionMode::default(),
            };
            
            if history.is_full() {
                history.remove(0); // Remove oldest
            }
            history.push(record).ok();
        }
        
        // Reset window
        self.window_fuel_consumed.store(0, Ordering::Release);
        self.window_start.store(current_time, Ordering::Release);
    }

    /// Get current monitoring statistics
    pub fn get_statistics(&self) -> FuelMonitoringStats {
        FuelMonitoringStats {
            current_rate: self.current_rate.load(Ordering::Acquire),
            peak_rate: self.peak_rate.load(Ordering::Acquire),
            window_fuel_consumed: self.window_fuel_consumed.load(Ordering::Acquire),
            active_alerts_count: self.active_alerts.lock().map(|a| a.len()).unwrap_or(0),
        }
    }

    /// Clear resolved alerts
    pub fn clear_alerts(&self) {
        if let Ok(mut alerts) = self.active_alerts.lock() {
            alerts.clear();
        }
    }

    fn get_current_time(&self) -> u64 {
        // In real implementation, would use deterministic time source
        // For now, use global fuel consumed as proxy
        global_fuel_consumed()
    }
}

impl Default for ASILFuelThresholds {
    fn default() -> Self {
        Self {
            asil_d_task_limit: 1000,      // 1ms worth of fuel for ASIL-D
            asil_c_component_limit: 10000, // 10ms for ASIL-C component
            asil_b_slice_limit: 5000,      // 5ms per slice for ASIL-B
            asil_a_warning_threshold: 50000, // 50ms warning for ASIL-A
        }
    }
}

impl Default for ASILFuelEnforcementPolicy {
    fn default() -> Self {
        Self {
            strict_enforcement: true,
            allow_fuel_borrowing: false,
            emergency_reserves_enabled: true,
            emergency_reserve_amount: 10000, // 10ms emergency reserve
            asil_policies: ASILSpecificPolicies::default(),
        }
    }
}

impl Default for ASILSpecificPolicies {
    fn default() -> Self {
        Self {
            asil_d: ASILDPolicy::default(),
            asil_c: ASILCPolicy::default(),
            asil_b: ASILBPolicy::default(),
            asil_a: ASILAPolicy::default(),
        }
    }
}

impl Default for ASILDPolicy {
    fn default() -> Self {
        Self {
            fuel_quantum: 10,        // 10 fuel units quantum
            max_step_fuel: 100,      // 100 fuel per step max
            enforce_deterministic_ordering: true,
            require_preallocation: true,
        }
    }
}

impl Default for ASILCPolicy {
    fn default() -> Self {
        Self {
            component_isolation: true,
            max_transfer_amount: 1000, // Allow 1ms fuel transfer
            temporal_window_ms: 100,   // 100ms temporal windows
        }
    }
}

impl Default for ASILBPolicy {
    fn default() -> Self {
        Self {
            slice_fuel_budget: 5000,  // 5ms per slice
            allow_rollover: true,
            max_rollover_percent: 20, // 20% rollover allowed
        }
    }
}

impl Default for ASILAPolicy {
    fn default() -> Self {
        Self {
            soft_limit: 40000,        // 40ms soft limit
            hard_limit: 100000,       // 100ms hard limit
            grace_period_ms: 5000,    // 5s grace period
        }
    }
}

/// Fuel monitoring statistics
#[derive(Debug, Clone)]
pub struct FuelMonitoringStats {
    pub current_rate: u64,
    pub peak_rate: u64,
    pub window_fuel_consumed: u64,
    pub active_alerts_count: usize,
}

impl FuelAsyncExecutor {
    /// Set the weak self reference for waker creation
    pub fn set_self_ref(&mut self, self_ref: Weak<Mutex<Self>>) {
        self.self_ref = Some(self_ref);
    }
    
    /// Get polling statistics
    pub fn get_polling_stats(&self) -> PollingStatistics {
        self.polling_stats.clone()
    }
    
    /// Reset polling statistics
    pub fn reset_polling_stats(&mut self) {
        self.polling_stats = PollingStatistics::default();
    }
}

// Temporary no-op waker implementation for Phase 1
/// ASIL-D compliant noop waker creation
fn create_noop_waker() -> Waker {
    use core::task::{RawWaker, RawWakerVTable};
    
    fn noop_clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    
    fn noop(_: *const ()) {}
    
    fn noop_raw_waker() -> RawWaker {
        RawWaker::new(
            core::ptr::null(),
            &RawWakerVTable::new(noop_clone, noop, noop, noop),
        )
    }
    
    // SAFETY: This unsafe call is required by Rust's Waker API and cannot be avoided.
    // For ASIL-D compliance:
    // 1. The noop_raw_waker uses null pointer data (no dereferencing)
    // 2. All vtable functions are no-ops that don't access memory
    // 3. This creates a functionally safe noop waker
    // 4. The waker lifetime is managed by Rust's type system
    #[allow(unsafe_code)]
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::future::Ready;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    // Test future that yields once then completes
    struct YieldOnceFuture {
        yielded: bool,
    }

    impl Future for YieldOnceFuture {
        type Output = Result<(), Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if !self.yielded {
                self.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }

    #[test]
    fn test_executor_creation() {
        let executor = FuelAsyncExecutor::new().unwrap();
        let status = executor.get_global_fuel_status();
        
        assert_eq!(status.active_tasks, 0);
        assert_eq!(status.ready_tasks, 0);
        assert!(status.enforcement_enabled);
    }

    #[test]
    fn test_task_spawning() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(10000);

        let future = async { Ok(()) };
        let task_id = executor.spawn_task(
            ComponentInstanceId::new(1),
            1000,
            Priority::Normal,
            future,
        ).unwrap();

        let status = executor.get_task_status(task_id).unwrap();
        assert_eq!(status.state, AsyncTaskState::Ready);
        assert_eq!(status.fuel_budget, 1000);
    }

    #[test]
    fn test_fuel_limit_enforcement() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(100);

        let future = async { Ok(()) };
        let result = executor.spawn_task(
            ComponentInstanceId::new(1),
            200,  // Exceeds limit
            Priority::Normal,
            future,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_proper_waker_integration() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(10000);
        
        // Create Arc<Mutex<>> wrapper and set self reference
        let executor_arc = Arc::new(Mutex::new(executor));
        if let Ok(mut exec) = executor_arc.lock() {
            exec.set_self_ref(Arc::downgrade(&executor_arc.clone()));
        }
        
        // Spawn a task that yields once
        let task_id = {
            let mut exec = executor_arc.lock().unwrap();
            exec.spawn_task(
                ComponentInstanceId::new(1),
                1000,
                Priority::Normal,
                YieldOnceFuture { yielded: false },
            ).unwrap()
        };
        
        // First poll should return Pending and wake the task
        {
            let mut exec = executor_arc.lock().unwrap();
            let polled = exec.poll_tasks().unwrap();
            assert_eq!(polled, 1);
            
            let stats = exec.get_polling_stats();
            assert_eq!(stats.tasks_yielded, 1);
            assert_eq!(stats.total_polls, 1);
        }
        
        // Second poll should complete the task
        {
            let mut exec = executor_arc.lock().unwrap();
            let polled = exec.poll_tasks().unwrap();
            assert_eq!(polled, 1);
            
            let stats = exec.get_polling_stats();
            assert_eq!(stats.tasks_completed, 1);
            assert_eq!(stats.total_polls, 2);
        }
        
        // Verify task is completed
        {
            let exec = executor_arc.lock().unwrap();
            let status = exec.get_task_status(task_id).unwrap();
            assert_eq!(status.state, AsyncTaskState::Completed);
        }
    }

    #[test]
    fn test_polling_statistics() {
        let mut executor = FuelAsyncExecutor::new().unwrap();
        executor.set_global_fuel_limit(10000);
        
        // Spawn multiple tasks
        for i in 0..3 {
            executor.spawn_task(
                ComponentInstanceId::new(i),
                1000,
                Priority::Normal,
                async { Ok(()) },
            ).unwrap();
        }
        
        // Poll all tasks
        let polled = executor.poll_tasks().unwrap();
        assert_eq!(polled, 3);
        
        let stats = executor.get_polling_stats();
        assert_eq!(stats.tasks_completed, 3);
        assert_eq!(stats.total_polls, 3);
        
        // Reset stats and verify
        executor.reset_polling_stats();
        let stats = executor.get_polling_stats();
        assert_eq!(stats.total_polls, 0);
        assert_eq!(stats.tasks_completed, 0);
    }
}