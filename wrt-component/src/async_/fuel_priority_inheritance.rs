//! Priority inheritance protocol for fuel-based async tasks
//!
//! This module implements priority inheritance to prevent priority inversion
//! in async task execution, critical for ASIL-B functional safety requirements.

use crate::{
    async_::fuel_async_executor::{AsyncTaskState, FuelAsyncTask},
    task_manager::TaskId,
    ComponentInstanceId,
    prelude::*,
};
use core::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::Duration,
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    CrateId, safe_managed_alloc,
};
use wrt_platform::advanced_sync::Priority;

/// Maximum number of tasks in priority inheritance chains
const MAX_INHERITANCE_CHAIN_LENGTH: usize = 32;

/// Maximum number of priority inheritance protocols active simultaneously
const MAX_ACTIVE_PROTOCOLS: usize = 128;

/// Fuel costs for priority inheritance operations
const PRIORITY_INHERITANCE_SETUP_FUEL: u64 = 15;
const PRIORITY_INHERITANCE_RESOLVE_FUEL: u64 = 10;
const PRIORITY_DONATION_FUEL: u64 = 5;
const PRIORITY_RESTORATION_FUEL: u64 = 8;

/// Priority inheritance protocol manager
pub struct FuelPriorityInheritanceProtocol {
    /// Active inheritance chains indexed by resource ID
    inheritance_chains: BoundedHashMap<ResourceId, InheritanceChain, MAX_ACTIVE_PROTOCOLS>,
    /// Task priority donations tracking
    priority_donations: BoundedHashMap<TaskId, PriorityDonation, MAX_ACTIVE_PROTOCOLS>,
    /// Original priorities before inheritance
    original_priorities: BoundedHashMap<TaskId, Priority, MAX_ACTIVE_PROTOCOLS>,
    /// Resource blocking relationships
    blocking_relationships: BoundedHashMap<TaskId, BlockingInfo, MAX_ACTIVE_PROTOCOLS>,
    /// Global protocol statistics
    protocol_stats: ProtocolStatistics,
    /// Verification level for fuel tracking
    verification_level: VerificationLevel,
}

/// Resource identifier for blocking relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

impl ResourceId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn from_task(task_id: TaskId) -> Self {
        Self(task_id.0 as u64)
    }
}

/// Priority inheritance chain tracking blocked tasks
#[derive(Debug, Clone)]
pub struct InheritanceChain {
    /// Resource being contended for
    pub resource_id: ResourceId,
    /// Task currently holding the resource
    pub holder: TaskId,
    /// Tasks waiting for the resource (highest priority first)
    pub waiters: BoundedVec<TaskId, MAX_INHERITANCE_CHAIN_LENGTH>,
    /// Current inherited priority level
    pub inherited_priority: Priority,
    /// Original priority of the holder
    pub holder_original_priority: Priority,
    /// Timestamp when inheritance started
    pub inheritance_start_time: AtomicU64,
    /// Fuel consumed by inheritance operations
    pub fuel_consumed: AtomicU64,
}

/// Priority donation tracking
#[derive(Debug, Clone)]
pub struct PriorityDonation {
    /// Task receiving the priority donation
    pub recipient: TaskId,
    /// Task donating the priority
    pub donor: TaskId,
    /// Donated priority level
    pub donated_priority: Priority,
    /// Resource causing the donation
    pub resource_id: ResourceId,
    /// When the donation was made
    pub donation_time: AtomicU64,
    /// Whether the donation is still active
    pub active: bool,
}

/// Information about task blocking relationships
#[derive(Debug, Clone)]
pub struct BlockingInfo {
    /// Task that is blocked
    pub blocked_task: TaskId,
    /// Resource being waited for
    pub blocked_on_resource: ResourceId,
    /// Task holding the resource
    pub blocked_by_task: Option<TaskId>,
    /// Priority of the blocked task
    pub blocked_task_priority: Priority,
    /// When the blocking started
    pub blocking_start_time: AtomicU64,
    /// Maximum blocking time allowed
    pub max_blocking_time: Option<Duration>,
}

/// Protocol performance statistics
#[derive(Debug, Clone)]
pub struct ProtocolStatistics {
    /// Total number of priority inheritances performed
    pub total_inheritances: AtomicUsize,
    /// Total number of priority donations
    pub total_donations: AtomicUsize,
    /// Total number of priority inversions prevented
    pub inversions_prevented: AtomicUsize,
    /// Total fuel consumed by inheritance protocol
    pub total_fuel_consumed: AtomicU64,
    /// Maximum inheritance chain length observed
    pub max_chain_length: AtomicUsize,
    /// Number of active inheritance chains
    pub active_chains: AtomicUsize,
    /// Average resolution time in fuel units
    pub average_resolution_fuel: AtomicU64,
}

impl FuelPriorityInheritanceProtocol {
    /// Create a new priority inheritance protocol manager
    pub fn new(verification_level: VerificationLevel) -> Result<Self, Error> {
        Ok(Self {
            inheritance_chains: BoundedHashMap::new(),
            priority_donations: BoundedHashMap::new(),
            original_priorities: BoundedHashMap::new(),
            blocking_relationships: BoundedHashMap::new(),
            protocol_stats: ProtocolStatistics {
                total_inheritances: AtomicUsize::new(0),
                total_donations: AtomicUsize::new(0),
                inversions_prevented: AtomicUsize::new(0),
                total_fuel_consumed: AtomicU64::new(0),
                max_chain_length: AtomicUsize::new(0),
                active_chains: AtomicUsize::new(0),
                average_resolution_fuel: AtomicU64::new(0),
            },
            verification_level,
        })
    }

    /// Register a task blocking on a resource
    pub fn register_blocking(
        &mut self,
        blocked_task: TaskId,
        blocked_task_priority: Priority,
        resource_id: ResourceId,
        holder_task: Option<TaskId>,
        max_blocking_time: Option<Duration>,
    ) -> Result<(), Error> {
        record_global_operation(OperationType::CollectionInsert, self.verification_level);
        self.consume_protocol_fuel(PRIORITY_INHERITANCE_SETUP_FUEL)?;

        // Record the blocking relationship
        let blocking_info = BlockingInfo {
            blocked_task,
            blocked_on_resource: resource_id,
            blocked_by_task: holder_task,
            blocked_task_priority,
            blocking_start_time: AtomicU64::new(self.get_current_fuel_time()),
            max_blocking_time,
        };

        self.blocking_relationships.insert(blocked_task, blocking_info).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many blocking relationships tracked".to_string(),
            )
        })?;

        // If there's a holder, initiate priority inheritance
        if let Some(holder) = holder_task {
            self.initiate_priority_inheritance(blocked_task, blocked_task_priority, resource_id, holder)?;
        }

        Ok(())
    }

    /// Initiate priority inheritance when a high-priority task is blocked
    pub fn initiate_priority_inheritance(
        &mut self,
        blocked_task: TaskId,
        blocked_priority: Priority,
        resource_id: ResourceId,
        holder_task: TaskId,
    ) -> Result<(), Error> {
        record_global_operation(OperationType::FunctionCall, self.verification_level);
        self.consume_protocol_fuel(PRIORITY_INHERITANCE_SETUP_FUEL)?;

        // Get or create inheritance chain for this resource
        let chain = match self.inheritance_chains.get_mut(&resource_id) {
            Some(existing_chain) => {
                // Add to existing chain
                existing_chain.waiters.push(blocked_task).map_err(|_| {
                    Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_LIMIT_EXCEEDED,
                        "Inheritance chain too long".to_string(),
                    )
                })?;
                
                // Sort waiters by priority (highest first)
                self.sort_waiters_by_priority(&mut existing_chain.waiters)?;
                
                // Update inherited priority if this waiter has higher priority
                if blocked_priority > existing_chain.inherited_priority {
                    existing_chain.inherited_priority = blocked_priority;
                }
                
                existing_chain
            }
            None => {
                // Create new inheritance chain
                let provider = safe_managed_alloc!(1024, CrateId::Component)?;
                let mut waiters = BoundedVec::new(provider)?;
                waiters.push(blocked_task).map_err(|_| {
                    Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_LIMIT_EXCEEDED,
                        "Failed to add waiter to new chain".to_string(),
                    )
                })?;

                let new_chain = InheritanceChain {
                    resource_id,
                    holder: holder_task,
                    waiters,
                    inherited_priority: blocked_priority,
                    holder_original_priority: Priority::Normal, // Will be updated below
                    inheritance_start_time: AtomicU64::new(self.get_current_fuel_time()),
                    fuel_consumed: AtomicU64::new(PRIORITY_INHERITANCE_SETUP_FUEL),
                };

                self.inheritance_chains.insert(resource_id, new_chain).map_err(|_| {
                    Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_LIMIT_EXCEEDED,
                        "Too many inheritance chains".to_string(),
                    )
                })?;

                self.protocol_stats.active_chains.fetch_add(1, Ordering::AcqRel);
                self.inheritance_chains.get_mut(&resource_id).unwrap()
            }
        };

        // Store original priority if not already stored
        if !self.original_priorities.contains_key(&holder_task) {
            // For this example, we'll assume Normal priority as default
            // In real implementation, this would query the actual task priority
            chain.holder_original_priority = Priority::Normal;
            self.original_priorities.insert(holder_task, Priority::Normal).map_err(|_| {
                Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    "Too many original priorities tracked".to_string(),
                )
            })?;
        }

        // Create priority donation record
        let donation = PriorityDonation {
            recipient: holder_task,
            donor: blocked_task,
            donated_priority: blocked_priority,
            resource_id,
            donation_time: AtomicU64::new(self.get_current_fuel_time()),
            active: true,
        };

        self.priority_donations.insert(blocked_task, donation).map_err(|_| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Too many priority donations tracked".to_string(),
            )
        })?;

        // Update statistics
        self.protocol_stats.total_inheritances.fetch_add(1, Ordering::AcqRel);
        self.protocol_stats.total_donations.fetch_add(1, Ordering::AcqRel);
        self.protocol_stats.inversions_prevented.fetch_add(1, Ordering::AcqRel);

        // Update max chain length
        let chain_length = chain.waiters.len();
        let current_max = self.protocol_stats.max_chain_length.load(Ordering::Acquire);
        if chain_length > current_max {
            self.protocol_stats.max_chain_length.store(chain_length, Ordering::Release);
        }

        Ok(())
    }

    /// Release a resource and restore original priorities
    pub fn release_resource(
        &mut self,
        resource_id: ResourceId,
        releasing_task: TaskId,
    ) -> Result<Option<TaskId>, Error> {
        record_global_operation(OperationType::CollectionRemove, self.verification_level);
        self.consume_protocol_fuel(PRIORITY_RESTORATION_FUEL)?;

        // Get the inheritance chain for this resource
        let chain = match self.inheritance_chains.remove(&resource_id) {
            Some(chain) => chain,
            None => return Ok(None), // No inheritance for this resource
        };

        self.protocol_stats.active_chains.fetch_sub(1, Ordering::AcqRel);

        // Restore the original priority of the releasing task
        if let Some(original_priority) = self.original_priorities.remove(&releasing_task) {
            // In a real implementation, this would call into the scheduler to update priority
            // For now, we just track the restoration
            self.consume_protocol_fuel(PRIORITY_RESTORATION_FUEL)?;
        }

        // Deactivate priority donations from this chain
        for &waiter in chain.waiters.iter() {
            if let Some(donation) = self.priority_donations.get_mut(&waiter) {
                donation.active = false;
            }
        }

        // Determine the next task to get the resource (highest priority waiter)
        let next_holder = chain.waiters.first().copied();

        // Remove blocking relationships for all waiters
        for &waiter in chain.waiters.iter() {
            self.blocking_relationships.remove(&waiter);
        }

        // Calculate resolution fuel for statistics
        let resolution_fuel = chain.fuel_consumed.load(Ordering::Acquire);
        let current_avg = self.protocol_stats.average_resolution_fuel.load(Ordering::Acquire);
        let new_avg = if current_avg == 0 {
            resolution_fuel
        } else {
            (current_avg + resolution_fuel) / 2
        };
        self.protocol_stats.average_resolution_fuel.store(new_avg, Ordering::Release);

        Ok(next_holder)
    }

    /// Check for and resolve potential priority inversions
    pub fn check_priority_inversion(&mut self, task_id: TaskId, task_priority: Priority) -> Result<bool, Error> {
        record_global_operation(OperationType::FunctionCall, self.verification_level);
        self.consume_protocol_fuel(PRIORITY_INHERITANCE_RESOLVE_FUEL)?;

        // Check if this task is being blocked by a lower priority task
        if let Some(blocking_info) = self.blocking_relationships.get(&task_id) {
            if let Some(blocking_task) = blocking_info.blocked_by_task {
                // Check if blocking task has lower priority than blocked task
                if blocking_info.blocked_task_priority > task_priority {
                    // Potential priority inversion detected
                    self.initiate_priority_inheritance(
                        task_id,
                        blocking_info.blocked_task_priority,
                        blocking_info.blocked_on_resource,
                        blocking_task,
                    )?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get priority inheritance statistics
    pub fn get_statistics(&self) -> ProtocolStatistics {
        self.protocol_stats.clone()
    }

    /// Get the effective priority for a task (considering donations)
    pub fn get_effective_priority(&self, task_id: TaskId, base_priority: Priority) -> Priority {
        // Check if this task has received any priority donations
        for donation in self.priority_donations.values() {
            if donation.recipient == task_id && donation.active && donation.donated_priority > base_priority {
                return donation.donated_priority;
            }
        }
        base_priority
    }

    /// Clean up expired or resolved inheritance relationships
    pub fn cleanup_expired_inheritances(&mut self, current_fuel_time: u64) -> Result<usize, Error> {
        record_global_operation(OperationType::CollectionMutate, self.verification_level);
        
        let mut cleaned_count = 0;
        let mut expired_resources = Vec::new();

        // Find expired inheritance chains
        for (resource_id, chain) in self.inheritance_chains.iter() {
            let start_time = chain.inheritance_start_time.load(Ordering::Acquire);
            let elapsed_fuel = current_fuel_time.saturating_sub(start_time);
            
            // Check if any blocking relationships have expired
            let mut has_expired = false;
            for &waiter in chain.waiters.iter() {
                if let Some(blocking_info) = self.blocking_relationships.get(&waiter) {
                    if let Some(max_time) = blocking_info.max_blocking_time {
                        let max_fuel_time = max_time.as_millis() as u64; // 1ms = 1 fuel
                        if elapsed_fuel > max_fuel_time {
                            has_expired = true;
                            break;
                        }
                    }
                }
            }
            
            if has_expired {
                expired_resources.push(*resource_id);
            }
        }

        // Clean up expired resources
        for resource_id in expired_resources {
            if let Some(chain) = self.inheritance_chains.remove(&resource_id) {
                // Restore priorities and clean up relationships
                for &waiter in chain.waiters.iter() {
                    self.blocking_relationships.remove(&waiter);
                    if let Some(donation) = self.priority_donations.get_mut(&waiter) {
                        donation.active = false;
                    }
                }
                
                // Restore holder's original priority
                if let Some(original_priority) = self.original_priorities.remove(&chain.holder) {
                    // Priority restoration would happen here in real implementation
                    self.consume_protocol_fuel(PRIORITY_RESTORATION_FUEL)?;
                }
                
                cleaned_count += 1;
                self.protocol_stats.active_chains.fetch_sub(1, Ordering::AcqRel);
            }
        }

        Ok(cleaned_count)
    }

    // Private helper methods

    fn get_current_fuel_time(&self) -> u64 {
        // In a real implementation, this would get the current fuel time from the system
        // For now, we'll use a simple counter
        self.protocol_stats.total_fuel_consumed.load(Ordering::Acquire)
    }

    fn consume_protocol_fuel(&self, amount: u64) -> Result<(), Error> {
        self.protocol_stats.total_fuel_consumed.fetch_add(amount, Ordering::AcqRel);
        Ok(())
    }

    fn sort_waiters_by_priority(&self, waiters: &mut BoundedVec<TaskId, MAX_INHERITANCE_CHAIN_LENGTH>) -> Result<(), Error> {
        // Simple bubble sort for small collections
        let len = waiters.len();
        for i in 0..len {
            for j in 0..len.saturating_sub(1 + i) {
                if self.should_swap_by_priority(waiters[j], waiters[j + 1])? {
                    // Swap tasks
                    let temp = waiters[j];
                    waiters[j] = waiters[j + 1];
                    waiters[j + 1] = temp;
                }
            }
        }
        Ok(())
    }

    fn should_swap_by_priority(&self, task_a: TaskId, task_b: TaskId) -> Result<bool, Error> {
        // Get priority for each task from blocking relationships
        let priority_a = self.blocking_relationships.get(&task_a)
            .map(|info| info.blocked_task_priority)
            .unwrap_or(Priority::Normal);
        
        let priority_b = self.blocking_relationships.get(&task_b)
            .map(|info| info.blocked_task_priority)
            .unwrap_or(Priority::Normal);
        
        // Higher priority tasks should come first (task_a > task_b means swap)
        Ok(priority_a < priority_b)
    }
}

impl Default for FuelPriorityInheritanceProtocol {
    fn default() -> Self {
        Self::new(VerificationLevel::Standard).expect("Failed to create default priority inheritance protocol")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_creation() {
        let protocol = FuelPriorityInheritanceProtocol::new(VerificationLevel::Standard).unwrap();
        let stats = protocol.get_statistics();
        
        assert_eq!(stats.total_inheritances.load(Ordering::Acquire), 0);
        assert_eq!(stats.active_chains.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_blocking_registration() {
        let mut protocol = FuelPriorityInheritanceProtocol::new(VerificationLevel::Standard).unwrap();
        
        let blocked_task = TaskId::new(1);
        let holder_task = TaskId::new(2);
        let resource_id = ResourceId::new(100);
        
        let result = protocol.register_blocking(
            blocked_task,
            Priority::High,
            resource_id,
            Some(holder_task),
            Some(Duration::from_millis(1000)),
        );
        
        assert!(result.is_ok());
        
        let stats = protocol.get_statistics();
        assert_eq!(stats.active_chains.load(Ordering::Acquire), 1);
        assert_eq!(stats.total_inheritances.load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_priority_inheritance() {
        let mut protocol = FuelPriorityInheritanceProtocol::new(VerificationLevel::Standard).unwrap();
        
        let high_priority_task = TaskId::new(1);
        let low_priority_holder = TaskId::new(2);
        let resource_id = ResourceId::new(100);
        
        let result = protocol.initiate_priority_inheritance(
            high_priority_task,
            Priority::High,
            resource_id,
            low_priority_holder,
        );
        
        assert!(result.is_ok());
        
        // Check that effective priority is elevated
        let effective_priority = protocol.get_effective_priority(low_priority_holder, Priority::Low);
        assert_eq!(effective_priority, Priority::High);
    }

    #[test]
    fn test_resource_release() {
        let mut protocol = FuelPriorityInheritanceProtocol::new(VerificationLevel::Standard).unwrap();
        
        let blocked_task = TaskId::new(1);
        let holder_task = TaskId::new(2);
        let resource_id = ResourceId::new(100);
        
        // Set up inheritance
        protocol.register_blocking(
            blocked_task,
            Priority::High,
            resource_id,
            Some(holder_task),
            None,
        ).unwrap();
        
        // Release resource
        let next_holder = protocol.release_resource(resource_id, holder_task).unwrap();
        
        assert_eq!(next_holder, Some(blocked_task));
        
        let stats = protocol.get_statistics();
        assert_eq!(stats.active_chains.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_priority_inversion_detection() {
        let mut protocol = FuelPriorityInheritanceProtocol::new(VerificationLevel::Standard).unwrap();
        
        let high_priority_task = TaskId::new(1);
        let low_priority_blocker = TaskId::new(2);
        let resource_id = ResourceId::new(100);
        
        // Register blocking relationship
        protocol.register_blocking(
            high_priority_task,
            Priority::High,
            resource_id,
            Some(low_priority_blocker),
            None,
        ).unwrap();
        
        // Check for priority inversion
        let inversion_detected = protocol.check_priority_inversion(
            high_priority_task,
            Priority::Low, // Simulate lower current priority
        ).unwrap();
        
        assert!(inversion_detected);
    }
}