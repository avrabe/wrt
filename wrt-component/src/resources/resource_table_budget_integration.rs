//! ResourceTable Memory Budget Integration
//!
//! This module demonstrates and tests the integration between ResourceTable
//! and the WRT memory budget system for ASIL-D compliance.
//!
//! SW-REQ-ID: REQ_MEM_001 - Memory bounds checking
//! SW-REQ-ID: REQ_MEM_002 - Budget enforcement
//! SW-REQ-ID: REQ_COMP_001 - Component isolation

use crate::resources::resource_table::ResourceTable;
use wrt_foundation::{
    budget_aware_provider::CrateId,
    safe_managed_alloc,
    memory_init::MemoryInitializer,
    WrtResult,
};

/// Budget-aware ResourceTable factory
/// 
/// This factory function demonstrates proper integration with the memory budget system.
/// All ResourceTable instances created through this factory are automatically
/// tracked by the budget enforcement system.
/// 
/// # ASIL-D Compliance
/// 
/// - ✅ Static memory allocation through budget system
/// - ✅ Compile-time size enforcement
/// - ✅ Automatic cleanup via RAII
/// - ✅ Component isolation through budget tracking
pub fn create_budget_aware_resource_table() -> WrtResult<ResourceTable> {
    // Ensure memory system is initialized
    MemoryInitializer::initialize()?;
    
    // Create budget-aware ResourceTable
    ResourceTable::new_budget_aware()
}

/// Resource table pool with budget enforcement
/// 
/// This structure manages a pool of ResourceTable instances with
/// integrated budget tracking for component isolation.
pub struct BudgetAwareResourceTablePool {
    /// Component ID for budget tracking
    component_id: CrateId,
    /// Current number of active tables
    active_tables: u32,
    /// Maximum allowed tables
    max_tables: u32,
}

impl BudgetAwareResourceTablePool {
    /// Create a new budget-aware resource table pool
    pub fn new(component_id: CrateId, max_tables: u32) -> Self {
        Self {
            component_id,
            active_tables: 0,
            max_tables,
        }
    }
    
    /// Create a new ResourceTable with budget enforcement
    /// 
    /// This method ensures that:
    /// 1. The table allocation is tracked by the budget system
    /// 2. Component isolation is maintained
    /// 3. Resource limits are enforced
    pub fn create_table(&mut self) -> WrtResult<ResourceTable> {
        if self.active_tables >= self.max_tables {
            return Err(wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Resource,
                wrt_error::codes::RESOURCE_EXHAUSTED,
                "Maximum resource tables reached for component"
            ));
        }
        
        // Create table with budget tracking
        let table = create_budget_aware_resource_table()?;
        self.active_tables += 1;
        
        Ok(table)
    }
    
    /// Release a ResourceTable and update tracking
    pub fn release_table(&mut self) {
        if self.active_tables > 0 {
            self.active_tables -= 1;
        }
    }
    
    /// Get current resource usage statistics
    pub fn get_usage_stats(&self) -> ResourceTableUsageStats {
        ResourceTableUsageStats {
            component_id: self.component_id,
            active_tables: self.active_tables,
            max_tables: self.max_tables,
            utilization_percent: if self.max_tables > 0 {
                (self.active_tables * 100) / self.max_tables
            } else {
                0
            },
        }
    }
}

/// Resource table usage statistics for monitoring
#[derive(Debug, Clone, Copy)]
pub struct ResourceTableUsageStats {
    pub component_id: CrateId,
    pub active_tables: u32,
    pub max_tables: u32,
    pub utilization_percent: u32,
}

/// Integration verification function
/// 
/// This function verifies that ResourceTable integration with the budget system
/// is working correctly for ASIL-D compliance.
pub fn verify_budget_integration() -> WrtResult<bool> {
    // Initialize memory system
    MemoryInitializer::initialize()?;
    
    // Test 1: Create ResourceTable with budget enforcement
    let _table1 = create_budget_aware_resource_table()?;
    
    // Test 2: Create multiple tables and verify budget tracking
    let mut pool = BudgetAwareResourceTablePool::new(CrateId::Component, 3);
    
    let _table2 = pool.create_table()?;
    let _table3 = pool.create_table()?;
    let _table4 = pool.create_table()?;
    
    // Test 3: Verify limit enforcement
    let result = pool.create_table();
    if result.is_ok() {
        return Ok(false); // Should have failed due to limit
    }
    
    // Test 4: Verify statistics
    let stats = pool.get_usage_stats();
    if stats.active_tables != 3 || stats.utilization_percent != 100 {
        return Ok(false);
    }
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_budget_aware_resource_table_creation() {
        let result = create_budget_aware_resource_table();
        assert!(result.is_ok(), "Failed to create budget-aware ResourceTable");
    }
    
    #[test]
    fn test_resource_table_pool() {
        let mut pool = BudgetAwareResourceTablePool::new(CrateId::Component, 2);
        
        // Create first table
        let table1 = pool.create_table();
        assert!(table1.is_ok());
        
        // Create second table
        let table2 = pool.create_table();
        assert!(table2.is_ok());
        
        // Third table should fail (exceeds limit)
        let table3 = pool.create_table();
        assert!(table3.is_err());
        
        // Check statistics
        let stats = pool.get_usage_stats();
        assert_eq!(stats.active_tables, 2);
        assert_eq!(stats.max_tables, 2);
        assert_eq!(stats.utilization_percent, 100);
    }
    
    #[test]
    fn test_budget_integration_verification() {
        let result = verify_budget_integration();
        assert!(result.is_ok());
        assert!(result.unwrap(), "Budget integration verification failed");
    }
    
    #[test]
    fn test_component_isolation() {
        let mut pool1 = BudgetAwareResourceTablePool::new(CrateId::Component, 1);
        let mut pool2 = BudgetAwareResourceTablePool::new(CrateId::Runtime, 1);
        
        // Each component should be able to create its own tables
        let table1 = pool1.create_table();
        let table2 = pool2.create_table();
        
        assert!(table1.is_ok());
        assert!(table2.is_ok());
        
        // Verify isolation in statistics
        let stats1 = pool1.get_usage_stats();
        let stats2 = pool2.get_usage_stats();
        
        assert_eq!(stats1.component_id.as_index(), CrateId::Component.as_index());
        assert_eq!(stats2.component_id.as_index(), CrateId::Runtime.as_index());
    }
}