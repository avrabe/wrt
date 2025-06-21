//! Handle tables with fuel tracking for Component Model resources
//!
//! This module provides fuel-aware handle table management for tracking
//! component resources with deterministic performance characteristics.

use crate::{
    async_::{
        fuel_resource_lifetime::{ResourceHandle, ResourceState},
        fuel_error_context::{AsyncErrorKind, async_error},
    },
    prelude::*,
};
use core::{
    sync::atomic::{AtomicU64, AtomicU32, Ordering},
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    safe_managed_alloc, CrateId,
};

/// Maximum handles per table
const MAX_HANDLES_PER_TABLE: usize = 1024;

/// Maximum handle tables per component
const MAX_HANDLE_TABLES: usize = 32;

/// Fuel costs for handle operations
const HANDLE_ALLOCATE_FUEL: u64 = 3;
const HANDLE_LOOKUP_FUEL: u64 = 1;
const HANDLE_UPDATE_FUEL: u64 = 2;
const HANDLE_DEALLOCATE_FUEL: u64 = 3;
const TABLE_CREATE_FUEL: u64 = 20;
const TABLE_RESIZE_FUEL: u64 = 50;

/// Handle entry in the table
#[derive(Debug)]
pub struct HandleEntry<T> {
    /// The actual data
    pub data: Option<T>,
    /// Generation counter for ABA problem prevention
    pub generation: u32,
    /// Resource state
    pub state: ResourceState,
    /// Last access timestamp (in fuel units)
    pub last_accessed: AtomicU64,
    /// Access count
    pub access_count: AtomicU32,
}

impl<T> HandleEntry<T> {
    /// Create a new handle entry
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            generation: 0,
            state: ResourceState::Available,
            last_accessed: AtomicU64::new(wrt_foundation::operations::global_fuel_consumed()),
            access_count: AtomicU32::new(0),
        }
    }
    
    /// Update last accessed time
    pub fn touch(&self) {
        self.last_accessed.store(
            wrt_foundation::operations::global_fuel_consumed(),
            Ordering::Release
        );
        self.access_count.fetch_add(1, Ordering::AcqRel);
    }
}

/// Handle with generation for ABA prevention
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationalHandle {
    /// Index in the handle table
    pub index: u32,
    /// Generation counter
    pub generation: u32,
}

impl GenerationalHandle {
    /// Create a new generational handle
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    
    /// Convert to ResourceHandle
    pub fn to_resource_handle(self) -> ResourceHandle {
        ResourceHandle(((self.generation as u64) << 32) | (self.index as u64))
    }
    
    /// Create from ResourceHandle
    pub fn from_resource_handle(handle: ResourceHandle) -> Self {
        let value = handle.0;
        Self {
            index: value as u32,
            generation: (value >> 32) as u32,
        }
    }
}

/// Handle table with fuel tracking
pub struct FuelHandleTable<T> {
    /// Table identifier
    pub table_id: u64,
    /// Entries in the table
    entries: BoundedVec<HandleEntry<T>, MAX_HANDLES_PER_TABLE>,
    /// Free list for handle reuse
    free_list: BoundedVec<u32, MAX_HANDLES_PER_TABLE>,
    /// Next generation counter
    next_generation: AtomicU32,
    /// Total fuel consumed
    fuel_consumed: AtomicU64,
    /// Fuel budget for this table
    fuel_budget: u64,
    /// Verification level
    verification_level: VerificationLevel,
    /// Statistics
    stats: HandleTableStats,
}

/// Statistics for handle table operations
#[derive(Debug, Default)]
pub struct HandleTableStats {
    /// Total allocations
    pub total_allocations: AtomicU64,
    /// Total deallocations
    pub total_deallocations: AtomicU64,
    /// Total lookups
    pub total_lookups: AtomicU64,
    /// Cache hits (fast path lookups)
    pub cache_hits: AtomicU64,
    /// Cache misses
    pub cache_misses: AtomicU64,
}

impl<T> FuelHandleTable<T> {
    /// Create a new handle table
    pub fn new(
        table_id: u64,
        initial_capacity: usize,
        fuel_budget: u64,
        verification_level: VerificationLevel,
    ) -> Result<Self> {
        let provider = safe_managed_alloc!(
            core::mem::size_of::<HandleEntry<T>>() * initial_capacity + 4096,
            CrateId::Component
        )?;
        
        let mut entries = BoundedVec::new(provider.clone())?;
        let mut free_list = BoundedVec::new(provider)?;
        
        // Pre-allocate entries
        for i in (0..initial_capacity).rev() {
            free_list.push(i as u32)?;
        }
        
        // Record table creation
        record_global_operation(OperationType::CollectionCreate)?;
        
        Ok(Self {
            table_id,
            entries,
            free_list,
            next_generation: AtomicU32::new(1),
            fuel_consumed: AtomicU64::new(TABLE_CREATE_FUEL),
            fuel_budget,
            verification_level,
            stats: HandleTableStats::default(),
        })
    }
    
    /// Allocate a new handle
    pub fn allocate(&mut self, data: T) -> Result<GenerationalHandle> {
        // Check fuel budget
        if !self.check_fuel(HANDLE_ALLOCATE_FUEL)? {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Handle table fuel budget exceeded",
            ));
        }
        
        // Get index from free list or extend table
        let index = if let Some(index) = self.free_list.pop() {
            index
        } else {
            // Need to extend the table
            if self.entries.len() >= MAX_HANDLES_PER_TABLE {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_LIMIT_EXCEEDED,
                    "Handle table capacity exceeded",
                ));
            }
            
            let new_index = self.entries.len() as u32;
            self.entries.push(HandleEntry::new(data))?;
            new_index
        };
        
        // Get generation
        let generation = self.next_generation.fetch_add(1, Ordering::AcqRel);
        
        // Update entry
        if let Some(entry) = self.entries.get_mut(index as usize) {
            entry.data = Some(data);
            entry.generation = generation;
            entry.state = ResourceState::Available;
            entry.touch();
        } else {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_ERROR,
                "Failed to update handle entry",
            ));
        }
        
        // Update stats
        self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);
        self.consume_fuel(HANDLE_ALLOCATE_FUEL)?;
        
        Ok(GenerationalHandle::new(index, generation))
    }
    
    /// Look up a handle
    pub fn lookup(&self, handle: GenerationalHandle) -> Result<&T> {
        // Check fuel
        if !self.check_fuel(HANDLE_LOOKUP_FUEL)? {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Handle table fuel budget exceeded",
            ));
        }
        
        // Validate index
        let entry = self.entries.get(handle.index as usize).ok_or_else(|| {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Invalid handle index",
            )
        })?;
        
        // Validate generation
        if entry.generation != handle.generation {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_INVALID_HANDLE,
                "Handle generation mismatch",
            ));
        }
        
        // Check state
        if entry.state != ResourceState::Available && entry.state != ResourceState::InUse {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_ACCESS_ERROR,
                "Resource not available",
            ));
        }
        
        // Get data
        let data = entry.data.as_ref().ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Handle data not found",
            )
        })?;
        
        // Update access tracking
        entry.touch();
        self.stats.total_lookups.fetch_add(1, Ordering::Relaxed);
        self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
        self.consume_fuel(HANDLE_LOOKUP_FUEL)?;
        
        Ok(data)
    }
    
    /// Look up a handle mutably
    pub fn lookup_mut(&mut self, handle: GenerationalHandle) -> Result<&mut T> {
        // Check fuel
        if !self.check_fuel(HANDLE_UPDATE_FUEL)? {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Handle table fuel budget exceeded",
            ));
        }
        
        // Validate index
        let entry = self.entries.get_mut(handle.index as usize).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Invalid handle index",
            )
        })?;
        
        // Validate generation
        if entry.generation != handle.generation {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_INVALID_HANDLE,
                "Handle generation mismatch",
            ));
        }
        
        // Update state
        entry.state = ResourceState::InUse;
        entry.touch();
        
        // Get data
        let data = entry.data.as_mut().ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Handle data not found",
            )
        })?;
        
        self.consume_fuel(HANDLE_UPDATE_FUEL)?;
        Ok(data)
    }
    
    /// Deallocate a handle
    pub fn deallocate(&mut self, handle: GenerationalHandle) -> Result<T> {
        // Check fuel
        if !self.check_fuel(HANDLE_DEALLOCATE_FUEL)? {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Handle table fuel budget exceeded",
            ));
        }
        
        // Validate and remove
        let entry = self.entries.get_mut(handle.index as usize).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Invalid handle index",
            )
        })?;
        
        // Validate generation
        if entry.generation != handle.generation {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_INVALID_HANDLE,
                "Handle generation mismatch",
            ));
        }
        
        // Take data
        let data = entry.data.take().ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Handle already deallocated",
            )
        })?;
        
        // Update state
        entry.state = ResourceState::Dropped;
        entry.generation = entry.generation.wrapping_add(1);
        
        // Add to free list
        self.free_list.push(handle.index)?;
        
        // Update stats
        self.stats.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.consume_fuel(HANDLE_DEALLOCATE_FUEL)?;
        
        Ok(data)
    }
    
    /// Check if we have enough fuel
    fn check_fuel(&self, required: u64) -> Result<bool> {
        let current = self.fuel_consumed.load(Ordering::Acquire);
        Ok(current.saturating_add(required) <= self.fuel_budget)
    }
    
    /// Consume fuel
    fn consume_fuel(&self, amount: u64) -> Result<()> {
        let adjusted = OperationType::fuel_cost_for_operation(
            OperationType::Other,
            self.verification_level,
        )?;
        
        let total = amount.saturating_add(adjusted);
        self.fuel_consumed.fetch_add(total, Ordering::AcqRel);
        record_global_operation(OperationType::Other)?;
        
        Ok(())
    }
    
    /// Get table statistics
    pub fn stats(&self) -> &HandleTableStats {
        &self.stats
    }
    
    /// Get current capacity
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }
    
    /// Get number of active handles
    pub fn active_handles(&self) -> usize {
        self.entries.len() - self.free_list.len()
    }
}

/// Handle table manager for multiple tables
pub struct HandleTableManager {
    /// Tables by ID
    tables: BoundedHashMap<u64, Box<dyn core::any::Any + Send + Sync>, MAX_HANDLE_TABLES>,
    /// Next table ID
    next_table_id: AtomicU64,
    /// Global fuel budget
    global_fuel_budget: u64,
    /// Total fuel consumed across all tables
    total_fuel_consumed: AtomicU64,
}

impl HandleTableManager {
    /// Create a new handle table manager
    pub fn new(global_fuel_budget: u64) -> Result<Self> {
        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        let tables = BoundedHashMap::new(provider)?;
        
        Ok(Self {
            tables,
            next_table_id: AtomicU64::new(1),
            global_fuel_budget,
            total_fuel_consumed: AtomicU64::new(0),
        })
    }
    
    /// Create a new handle table
    pub fn create_table<T: Send + Sync + 'static>(
        &mut self,
        initial_capacity: usize,
        verification_level: VerificationLevel,
    ) -> Result<u64> {
        let table_id = self.next_table_id.fetch_add(1, Ordering::AcqRel);
        
        // Allocate fuel budget for table (10% of remaining budget)
        let remaining_budget = self.global_fuel_budget
            .saturating_sub(self.total_fuel_consumed.load(Ordering::Acquire));
        let table_budget = remaining_budget / 10;
        
        let table = FuelHandleTable::<T>::new(
            table_id,
            initial_capacity,
            table_budget,
            verification_level,
        )?;
        
        self.tables.insert(table_id, Box::new(table))?;
        self.total_fuel_consumed.fetch_add(TABLE_CREATE_FUEL, Ordering::AcqRel);
        
        Ok(table_id)
    }
    
    /// Get a handle table
    pub fn get_table<T: Send + Sync + 'static>(
        &self,
        table_id: u64,
    ) -> Result<&FuelHandleTable<T>> {
        let table = self.tables.get(&table_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Handle table not found",
            )
        })?;
        
        // Downcast to specific type
        table.downcast_ref::<FuelHandleTable<T>>().ok_or_else(|| {
            Error::new(
                ErrorCategory::Type,
                codes::TYPE_MISMATCH,
                "Handle table type mismatch",
            )
        })
    }
    
    /// Get a handle table mutably
    pub fn get_table_mut<T: Send + Sync + 'static>(
        &mut self,
        table_id: u64,
    ) -> Result<&mut FuelHandleTable<T>> {
        let table = self.tables.get_mut(&table_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Handle table not found",
            )
        })?;
        
        // Downcast to specific type
        table.downcast_mut::<FuelHandleTable<T>>().ok_or_else(|| {
            Error::new(
                ErrorCategory::Type,
                codes::TYPE_MISMATCH,
                "Handle table type mismatch",
            )
        })
    }
    
    /// Drop a table
    pub fn drop_table(&mut self, table_id: u64) -> Result<()> {
        self.tables.remove(&table_id).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Handle table not found",
            )
        })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_handle_allocation() {
        let mut table = FuelHandleTable::new(1, 10, 1000, VerificationLevel::Basic).unwrap();
        
        let handle = table.allocate("test_data").unwrap();
        assert_eq!(handle.index, 0);
        assert_eq!(handle.generation, 1);
        
        let data = table.lookup(handle).unwrap();
        assert_eq!(*data, "test_data");
    }
    
    #[test]
    fn test_handle_deallocation() {
        let mut table = FuelHandleTable::new(1, 10, 1000, VerificationLevel::Basic).unwrap();
        
        let handle = table.allocate(42).unwrap();
        let data = table.deallocate(handle).unwrap();
        assert_eq!(data, 42);
        
        // Should fail to look up deallocated handle
        assert!(table.lookup(handle).is_err());
    }
    
    #[test]
    fn test_generation_validation() {
        let mut table = FuelHandleTable::new(1, 10, 1000, VerificationLevel::Basic).unwrap();
        
        let handle1 = table.allocate(1).unwrap();
        table.deallocate(handle1).unwrap();
        
        let handle2 = table.allocate(2).unwrap();
        assert_eq!(handle2.index, handle1.index); // Reused index
        assert_ne!(handle2.generation, handle1.generation); // Different generation
        
        // Old handle should fail
        assert!(table.lookup(handle1).is_err());
        
        // New handle should work
        assert_eq!(*table.lookup(handle2).unwrap(), 2);
    }
    
    #[test]
    fn test_handle_table_manager() {
        let mut manager = HandleTableManager::new(10000).unwrap();
        
        let table_id = manager.create_table::<String>(5, VerificationLevel::Basic).unwrap();
        
        {
            let table = manager.get_table_mut::<String>(table_id).unwrap();
            let handle = table.allocate("test".to_string()).unwrap();
            assert_eq!(table.lookup(handle).unwrap(), "test");
        }
        
        assert!(manager.drop_table(table_id).is_ok());
        assert!(manager.get_table::<String>(table_id).is_err());
    }
}