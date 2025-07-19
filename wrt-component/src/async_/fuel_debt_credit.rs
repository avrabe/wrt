//! Fuel debt and credit system for async task management
//!
//! This module implements a debt/credit system for fuel consumption across
//! async tasks, enabling fair scheduling and preventing fuel starvation.

use crate::{
    task_manager::TaskId,
    prelude::*,
};
use core::sync::atomic::{AtomicU64, Ordering};
use wrt_foundation::{
    bounded_collections::BoundedMap,
    sync::Mutex,
    Arc,
    CrateId, safe_managed_alloc,
};

/// Maximum debt that a task can accumulate
const MAX_TASK_DEBT: u64 = 10000;

/// Maximum credit that a task can accumulate
const MAX_TASK_CREDIT: u64 = 50000;

/// Default credit per task
const DEFAULT_CREDIT: u64 = 1000;

/// Fuel debt and credit management system
pub struct FuelDebtCreditSystem {
    /// Task debt balances
    task_debts: Arc<Mutex<BoundedMap<TaskId, u64, 256>>>,
    /// Task credit balances
    task_credits: Arc<Mutex<BoundedMap<TaskId, u64, 256>>>,
    /// Global debt counter
    global_debt: AtomicU64,
    /// Global credit counter
    global_credit: AtomicU64,
    /// Debt policy configuration
    debt_policy: DebtPolicy,
    /// Credit restriction policy
    credit_restriction: CreditRestriction,
}

/// Policy for managing task debt
#[derive(Debug, Clone, Copy)]
pub enum DebtPolicy {
    /// Allow unlimited debt (dangerous)
    Unlimited,
    /// Strict debt limits - task blocked when exceeded
    Strict,
    /// Gradual debt forgiveness
    Forgiveness { rate: u64 },
}

/// Credit restriction policies
#[derive(Debug, Clone, Copy)]
pub enum CreditRestriction {
    /// No restrictions on credit accumulation
    None,
    /// Cap credits at maximum value
    Capped,
    /// Redistribute excess credits to other tasks
    Redistribute,
}

impl FuelDebtCreditSystem {
    /// Create new fuel debt/credit system
    pub fn new(
        debt_policy: DebtPolicy,
        credit_restriction: CreditRestriction,
    ) -> Result<Self, Error> {
        let _provider = safe_managed_alloc!(4096, CrateId::Component)?;
        
        Ok(Self {
            task_debts: Arc::new(Mutex::new(BoundedMap::new(provider.clone())?)),
            task_credits: Arc::new(Mutex::new(BoundedMap::new(provider.clone())?)),
            global_debt: AtomicU64::new(0),
            global_credit: AtomicU64::new(0),
            debt_policy,
            credit_restriction,
        })
    }

    /// Register a new task with default credit
    pub fn register_task(&self, task_id: TaskId) -> Result<(), Error> {
        let mut credits = self.task_credits.lock()?;
        credits.insert(task_id, DEFAULT_CREDIT).map_err(|_| {
            Error::resource_limit_exceeded("Too many tasks in credit system")
        })?;
        
        let mut debts = self.task_debts.lock()?;
        debts.insert(task_id, 0).map_err(|_| {
            Error::resource_limit_exceeded("Too many tasks in debt system")
        })?;
        
        Ok(())
    }

    /// Consume fuel from task's credit/debt balance
    pub fn consume_fuel(&self, task_id: TaskId, fuel: u64) -> Result<bool, Error> {
        let mut credits = self.task_credits.lock()?;
        let mut debts = self.task_debts.lock()?;
        
        let current_credit = credits.get(&task_id).copied().unwrap_or(0;
        let current_debt = debts.get(&task_id).copied().unwrap_or(0;
        
        if current_credit >= fuel {
            // Sufficient credit available
            credits.insert(task_id, current_credit - fuel).ok();
            Ok(true)
        } else {
            // Need to go into debt
            let debt_needed = fuel - current_credit;
            let new_debt = current_debt + debt_needed;
            
            match self.debt_policy {
                DebtPolicy::Unlimited => {
                    credits.insert(task_id, 0).ok();
                    debts.insert(task_id, new_debt).ok();
                    self.global_debt.fetch_add(debt_needed, Ordering::Relaxed;
                    Ok(true)
                },
                DebtPolicy::Strict => {
                    if new_debt > MAX_TASK_DEBT {
                        Ok(false) // Reject fuel consumption
                    } else {
                        credits.insert(task_id, 0).ok();
                        debts.insert(task_id, new_debt).ok();
                        self.global_debt.fetch_add(debt_needed, Ordering::Relaxed;
                        Ok(true)
                    }
                },
                DebtPolicy::Forgiveness { rate: _ } => {
                    // Allow debt but track for future forgiveness
                    credits.insert(task_id, 0).ok();
                    debts.insert(task_id, new_debt).ok();
                    self.global_debt.fetch_add(debt_needed, Ordering::Relaxed;
                    Ok(true)
                },
            }
        }
    }

    /// Add credit to a task
    pub fn add_credit(&self, task_id: TaskId, credit: u64) -> Result<(), Error> {
        let mut credits = self.task_credits.lock()?;
        let current_credit = credits.get(&task_id).copied().unwrap_or(0;
        
        let new_credit = match self.credit_restriction {
            CreditRestriction::None => current_credit + credit,
            CreditRestriction::Capped => (current_credit + credit).min(MAX_TASK_CREDIT),
            CreditRestriction::Redistribute => {
                let capped_credit = (current_credit + credit).min(MAX_TASK_CREDIT;
                let excess = (current_credit + credit).saturating_sub(MAX_TASK_CREDIT;
                if excess > 0 {
                    // TODO: Redistribute excess to other tasks
                }
                capped_credit
            },
        };
        
        credits.insert(task_id, new_credit).ok();
        self.global_credit.fetch_add(credit, Ordering::Relaxed;
        Ok(())
    }

    /// Pay down debt for a task
    pub fn pay_debt(&self, task_id: TaskId, payment: u64) -> Result<u64, Error> {
        let mut debts = self.task_debts.lock()?;
        let current_debt = debts.get(&task_id).copied().unwrap_or(0;
        
        if current_debt == 0 {
            return Ok(0); // No debt to pay
        }
        
        let actual_payment = payment.min(current_debt;
        let new_debt = current_debt - actual_payment;
        
        debts.insert(task_id, new_debt).ok();
        self.global_debt.fetch_sub(actual_payment, Ordering::Relaxed;
        
        Ok(actual_payment)
    }

    /// Get task's current debt
    pub fn get_task_debt(&self, task_id: TaskId) -> Result<u64, Error> {
        let debts = self.task_debts.lock()?;
        Ok(debts.get(&task_id).copied().unwrap_or(0))
    }

    /// Get task's current credit
    pub fn get_task_credit(&self, task_id: TaskId) -> Result<u64, Error> {
        let credits = self.task_credits.lock()?;
        Ok(credits.get(&task_id).copied().unwrap_or(0))
    }

    /// Check if task can consume specified fuel
    pub fn can_consume_fuel(&self, task_id: TaskId, fuel: u64) -> Result<bool, Error> {
        let credits = self.task_credits.lock()?;
        let debts = self.task_debts.lock()?;
        
        let current_credit = credits.get(&task_id).copied().unwrap_or(0;
        let current_debt = debts.get(&task_id).copied().unwrap_or(0;
        
        if current_credit >= fuel {
            return Ok(true;
        }
        
        let debt_needed = fuel - current_credit;
        let new_debt = current_debt + debt_needed;
        
        match self.debt_policy {
            DebtPolicy::Unlimited => Ok(true),
            DebtPolicy::Strict => Ok(new_debt <= MAX_TASK_DEBT),
            DebtPolicy::Forgiveness { rate: _ } => Ok(new_debt <= MAX_TASK_DEBT * 2),
        }
    }

    /// Process debt forgiveness (call periodically)
    pub fn process_debt_forgiveness(&self) -> Result<u64, Error> {
        if let DebtPolicy::Forgiveness { rate } = self.debt_policy {
            let mut debts = self.task_debts.lock()?;
            let mut total_forgiven = 0u64;
            
            for (task_id, debt) in debts.iter_mut() {
                if *debt > 0 {
                    let forgiveness = (*debt).min(rate;
                    *debt -= forgiveness;
                    total_forgiven += forgiveness;
                }
            }
            
            self.global_debt.fetch_sub(total_forgiven, Ordering::Relaxed;
            Ok(total_forgiven)
        } else {
            Ok(0)
        }
    }

    /// Get global debt level
    pub fn global_debt(&self) -> u64 {
        self.global_debt.load(Ordering::Relaxed)
    }

    /// Get global credit level
    pub fn global_credit(&self) -> u64 {
        self.global_credit.load(Ordering::Relaxed)
    }

    /// Unregister a task (cleanup)
    pub fn unregister_task(&self, task_id: TaskId) -> Result<(u64, u64), Error> {
        let mut credits = self.task_credits.lock()?;
        let mut debts = self.task_debts.lock()?;
        
        let final_credit = credits.remove(&task_id).unwrap_or(0;
        let final_debt = debts.remove(&task_id).unwrap_or(0;
        
        // Update global counters
        self.global_credit.fetch_sub(final_credit, Ordering::Relaxed;
        self.global_debt.fetch_sub(final_debt, Ordering::Relaxed;
        
        Ok((final_credit, final_debt))
    }
}

impl Default for FuelDebtCreditSystem {
    fn default() -> Self {
        Self::new(DebtPolicy::Strict, CreditRestriction::Capped)
            .expect("Failed to create default FuelDebtCreditSystem")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_debt_credit_basic() {
        let system = FuelDebtCreditSystem::default);
        let task_id = TaskId::new(1;
        
        system.register_task(task_id).unwrap();
        
        // Task should have default credit
        assert_eq!(system.get_task_credit(task_id).unwrap(), DEFAULT_CREDIT;
        assert_eq!(system.get_task_debt(task_id).unwrap(), 0;
        
        // Consume some fuel
        assert!(system.consume_fuel(task_id, 500).unwrap();
        assert_eq!(system.get_task_credit(task_id).unwrap(), DEFAULT_CREDIT - 500;
        
        // Go into debt
        assert!(system.consume_fuel(task_id, 1000).unwrap();
        assert_eq!(system.get_task_credit(task_id).unwrap(), 0;
        assert_eq!(system.get_task_debt(task_id).unwrap(), 500;
    }

    #[test]
    fn test_debt_policy_strict() {
        let system = FuelDebtCreditSystem::new(
            DebtPolicy::Strict,
            CreditRestriction::Capped,
        ).unwrap();
        let task_id = TaskId::new(1;
        
        system.register_task(task_id).unwrap();
        
        // Exhaust credit first
        assert!(system.consume_fuel(task_id, DEFAULT_CREDIT).unwrap();
        
        // Try to exceed max debt
        assert!(!system.consume_fuel(task_id, MAX_TASK_DEBT + 1).unwrap();
    }

    #[test]
    fn test_debt_forgiveness() {
        let system = FuelDebtCreditSystem::new(
            DebtPolicy::Forgiveness { rate: 100 },
            CreditRestriction::None,
        ).unwrap();
        let task_id = TaskId::new(1;
        
        system.register_task(task_id).unwrap();
        
        // Go into debt
        system.consume_fuel(task_id, DEFAULT_CREDIT + 500).unwrap();
        assert_eq!(system.get_task_debt(task_id).unwrap(), 500;
        
        // Process forgiveness
        let forgiven = system.process_debt_forgiveness().unwrap();
        assert_eq!(forgiven, 100;
        assert_eq!(system.get_task_debt(task_id).unwrap(), 400;
    }
}