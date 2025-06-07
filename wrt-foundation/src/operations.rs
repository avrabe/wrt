// WRT - wrt-foundation
// Module: Operation Tracking and Fuel Metering
// SW-REQ-ID: REQ_007
// SW-REQ-ID: REQ_003
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

// #![allow(unsafe_code)] // unsafe_code is no longer directly in this file for
// OnceCell

//! Operation tracking for bounded collections and memory operations
//!
//! This module provides infrastructure for tracking operations performed
//! on bounded collections and memory, supporting WCET analysis and fuel
//! consumption calculations.

use core::sync::atomic::{AtomicU64, Ordering};

use wrt_error::Error as WrtError; // Added for the Result return type
// Use WrtOnce from wrt-sync crate
use wrt_sync::once::WrtOnce;

use crate::traits::importance; // Added this import
use crate::verification::VerificationLevel;

// Global operation counter for use when a local counter isn't available
static GLOBAL_COUNTER: WrtOnce<Counter> = WrtOnce::new();

/// Enum representing different types of operations that can be tracked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    /// Binary std/no_std choice
    MemoryAllocation,
    /// Binary std/no_std choice
    MemoryDeallocation,
    /// Memory read operation
    MemoryRead,
    /// Memory write operation
    MemoryWrite,
    /// Memory copy operation
    MemoryCopy,
    /// Memory grow operation
    MemoryGrow,
    /// Collection push operation
    CollectionPush,
    /// Collection pop operation
    CollectionPop,
    /// Collection lookup operation
    CollectionLookup,
    /// Collection insert operation
    CollectionInsert,
    /// Collection remove operation
    CollectionRemove,
    /// Collection validate operation
    CollectionValidate,
    /// Collection mutation operation
    CollectionMutate,
    /// Checksum calculation
    ChecksumCalculation,
    /// Function call operation
    FunctionCall,
    /// Control flow operation
    ControlFlow,
    /// Arithmetic operation
    Arithmetic,
    /// Other operation type
    Other,
    /// Collection create operation
    CollectionCreate,
    /// Collection clear operation
    CollectionClear,
    /// Collection truncate operation
    CollectionTruncate,
    /// Collection iterate operation
    CollectionIterate,
    /// A full recalculation of a checksum (potentially more expensive)
    ChecksumFullRecalculation,
    /// Collection element read operation
    CollectionRead,
    /// Collection element write operation
    CollectionWrite,
    /// Collection peek operation
    CollectionPeek,
}

impl Type {
    /// Get the base cost for an operation
    #[must_use]
    pub fn cost(self) -> u32 {
        match self {
            Type::MemoryAllocation => 10,
            Type::MemoryDeallocation => 8,
            Type::MemoryRead => 1,
            Type::MemoryWrite => 2,
            Type::MemoryCopy => 3,
            Type::MemoryGrow => 50,
            Type::CollectionPush => 5,
            Type::CollectionPop => 5,
            Type::CollectionLookup => 3,
            Type::CollectionInsert => 7,
            Type::CollectionRemove => 6,
            Type::CollectionValidate => 15,
            Type::CollectionMutate => 4,
            Type::ChecksumCalculation => 20,
            Type::ChecksumFullRecalculation => 100,
            Type::FunctionCall => 5,
            Type::ControlFlow => 1,
            Type::Arithmetic => 1,
            Type::Other => 1,
            Type::CollectionCreate => 12,
            Type::CollectionClear => 10,
            Type::CollectionTruncate => 8,
            Type::CollectionIterate => 1,
            Type::CollectionRead => 3,
            Type::CollectionWrite => 7,
            Type::CollectionPeek => 3,
        }
    }

    /// Get the importance level for validation checks
    #[must_use]
    pub fn importance(self) -> u8 {
        match self {
            Type::MemoryAllocation
            | Type::MemoryDeallocation
            | Type::MemoryGrow
            | Type::CollectionValidate => importance::CRITICAL,
            Type::MemoryWrite
            | Type::MemoryCopy
            | Type::CollectionPush
            | Type::CollectionPop
            | Type::CollectionInsert
            | Type::CollectionRemove
            | Type::CollectionMutate
            | Type::CollectionCreate
            | Type::CollectionClear
            | Type::CollectionTruncate
            | Type::ChecksumCalculation
            | Type::ChecksumFullRecalculation => importance::MUTATION,
            Type::MemoryRead
            | Type::CollectionLookup
            | Type::CollectionIterate
            | Type::FunctionCall
            | Type::ControlFlow
            | Type::Arithmetic
            | Type::Other => importance::READ,
            Type::CollectionRead | Type::CollectionPeek => importance::READ,
            Type::CollectionWrite => importance::MUTATION,
        }
    }

    /// Calculate fuel cost for an operation, considering verification level.
    ///
    /// # Errors
    ///
    /// This function currently does not return errors but is prepared for
    /// future extensions where cost calculation might fail (e.g., invalid
    /// inputs).
    pub fn fuel_cost_for_operation(
        op_type: Type,
        verification_level: VerificationLevel,
    ) -> Result<u64, WrtError> {
        let base_cost = u64::from(op_type.cost());

        // Adjust cost based on verification level using scaled integer math
        // Multiplier is scaled by 100 (e.g., 1.25 becomes 125)
        let scaled_multiplier = verification_cost_multiplier_scaled(&verification_level);

        // Calculate cost with rounding: (base * multiplier + 50) / 100
        let total_cost = (base_cost * scaled_multiplier + 50) / 100;

        Ok(total_cost)
    }
}

/// A counter for tracking operation counts
#[derive(Debug)]
pub struct Counter {
    /// Counter for memory read operations
    memory_reads: AtomicU64,
    /// Counter for memory write operations
    memory_writes: AtomicU64,
    /// Counter for memory grow operations
    memory_grows: AtomicU64,
    /// Binary std/no_std choice
    memory_allocations: AtomicU64,
    /// Binary std/no_std choice
    memory_deallocations: AtomicU64,
    /// Counter for collection push operations
    collection_pushes: AtomicU64,
    /// Counter for collection pop operations
    collection_pops: AtomicU64,
    /// Counter for collection lookup operations
    collection_lookups: AtomicU64,
    /// Counter for collection insert operations
    collection_inserts: AtomicU64,
    /// Counter for collection remove operations
    collection_removes: AtomicU64,
    /// Counter for collection validate operations
    collection_validates: AtomicU64,
    /// Counter for collection mutate operations
    collection_mutates: AtomicU64,
    /// Counter for checksum calculations
    checksum_calculations: AtomicU64,
    /// Counter for function calls
    function_calls: AtomicU64,
    /// Counter for control flow operations
    control_flows: AtomicU64,
    /// Counter for arithmetic operations
    arithmetic_ops: AtomicU64,
    /// Counter for other operations
    other_ops: AtomicU64,
    /// New counters
    collection_creates: AtomicU64,
    collection_clears: AtomicU64,
    collection_truncates: AtomicU64,
    collection_iterates: AtomicU64,
    /// Total fuel consumed by operations
    fuel_consumed: AtomicU64,
}

// Manual implementation of Clone for Counter since AtomicU64 doesn't
// implement Clone
impl Clone for Counter {
    fn clone(&self) -> Self {
        Self {
            memory_reads: AtomicU64::new(self.memory_reads.load(Ordering::Relaxed)),
            memory_writes: AtomicU64::new(self.memory_writes.load(Ordering::Relaxed)),
            memory_grows: AtomicU64::new(self.memory_grows.load(Ordering::Relaxed)),
            memory_allocations: AtomicU64::new(self.memory_allocations.load(Ordering::Relaxed)),
            memory_deallocations: AtomicU64::new(self.memory_deallocations.load(Ordering::Relaxed)),
            collection_pushes: AtomicU64::new(self.collection_pushes.load(Ordering::Relaxed)),
            collection_pops: AtomicU64::new(self.collection_pops.load(Ordering::Relaxed)),
            collection_lookups: AtomicU64::new(self.collection_lookups.load(Ordering::Relaxed)),
            collection_inserts: AtomicU64::new(self.collection_inserts.load(Ordering::Relaxed)),
            collection_removes: AtomicU64::new(self.collection_removes.load(Ordering::Relaxed)),
            collection_validates: AtomicU64::new(self.collection_validates.load(Ordering::Relaxed)),
            collection_mutates: AtomicU64::new(self.collection_mutates.load(Ordering::Relaxed)),
            checksum_calculations: AtomicU64::new(
                self.checksum_calculations.load(Ordering::Relaxed),
            ),
            function_calls: AtomicU64::new(self.function_calls.load(Ordering::Relaxed)),
            control_flows: AtomicU64::new(self.control_flows.load(Ordering::Relaxed)),
            arithmetic_ops: AtomicU64::new(self.arithmetic_ops.load(Ordering::Relaxed)),
            other_ops: AtomicU64::new(self.other_ops.load(Ordering::Relaxed)),
            collection_creates: AtomicU64::new(self.collection_creates.load(Ordering::Relaxed)),
            collection_clears: AtomicU64::new(self.collection_clears.load(Ordering::Relaxed)),
            collection_truncates: AtomicU64::new(self.collection_truncates.load(Ordering::Relaxed)),
            collection_iterates: AtomicU64::new(self.collection_iterates.load(Ordering::Relaxed)),
            fuel_consumed: AtomicU64::new(self.fuel_consumed.load(Ordering::Relaxed)),
        }
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Counter {
    /// Create a new operation counter with all counts at zero
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory_reads: AtomicU64::new(0),
            memory_writes: AtomicU64::new(0),
            memory_grows: AtomicU64::new(0),
            memory_allocations: AtomicU64::new(0),
            memory_deallocations: AtomicU64::new(0),
            collection_pushes: AtomicU64::new(0),
            collection_pops: AtomicU64::new(0),
            collection_lookups: AtomicU64::new(0),
            collection_inserts: AtomicU64::new(0),
            collection_removes: AtomicU64::new(0),
            collection_validates: AtomicU64::new(0),
            collection_mutates: AtomicU64::new(0),
            checksum_calculations: AtomicU64::new(0),
            function_calls: AtomicU64::new(0),
            control_flows: AtomicU64::new(0),
            arithmetic_ops: AtomicU64::new(0),
            other_ops: AtomicU64::new(0),
            collection_creates: AtomicU64::new(0),
            collection_clears: AtomicU64::new(0),
            collection_truncates: AtomicU64::new(0),
            collection_iterates: AtomicU64::new(0),
            fuel_consumed: AtomicU64::new(0),
        }
    }

    /// Record an operation and update fuel consumption.
    ///
    /// Note: This function calculates fuel cost internally. If the cost
    /// calculation fails (which is currently impossible but might change),
    /// the error is logged, and fuel is not updated.
    pub fn record_operation(&self, op_type: Type, verification_level: VerificationLevel) {
        // Record the specific operation
        match op_type {
            Type::MemoryAllocation => {
                self.memory_allocations.fetch_add(1, Ordering::Relaxed);
            }
            Type::MemoryDeallocation => {
                self.memory_deallocations.fetch_add(1, Ordering::Relaxed);
            }
            Type::MemoryRead => {
                self.memory_reads.fetch_add(1, Ordering::Relaxed);
            }
            Type::MemoryWrite => {
                self.memory_writes.fetch_add(1, Ordering::Relaxed);
            }
            Type::MemoryCopy => {
                self.memory_writes.fetch_add(1, Ordering::Relaxed);
            }
            Type::MemoryGrow => {
                self.memory_grows.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionPush => {
                self.collection_pushes.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionPop => {
                self.collection_pops.fetch_add(1, Ordering::Relaxed);
            }
            // Merged CollectionLookup, CollectionRead, CollectionPeek
            Type::CollectionLookup | Type::CollectionRead | Type::CollectionPeek => {
                self.collection_lookups.fetch_add(1, Ordering::Relaxed);
            }
            // Merged CollectionInsert, CollectionWrite
            Type::CollectionInsert | Type::CollectionWrite => {
                self.collection_inserts.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionRemove => {
                self.collection_removes.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionValidate => {
                self.collection_validates.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionMutate => {
                self.collection_mutates.fetch_add(1, Ordering::Relaxed);
            }
            // Merged ChecksumCalculation, ChecksumFullRecalculation
            Type::ChecksumCalculation | Type::ChecksumFullRecalculation => {
                self.checksum_calculations.fetch_add(1, Ordering::Relaxed);
            }
            Type::FunctionCall => {
                self.function_calls.fetch_add(1, Ordering::Relaxed);
            }
            Type::ControlFlow => {
                self.control_flows.fetch_add(1, Ordering::Relaxed);
            }
            Type::Arithmetic => {
                self.arithmetic_ops.fetch_add(1, Ordering::Relaxed);
            }
            Type::Other => {
                self.other_ops.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionCreate => {
                self.collection_creates.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionClear => {
                self.collection_clears.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionTruncate => {
                self.collection_truncates.fetch_add(1, Ordering::Relaxed);
            }
            Type::CollectionIterate => {
                self.collection_iterates.fetch_add(1, Ordering::Relaxed);
            }
        };

        // Calculate and add fuel cost
        // Use unwrap_or_else to handle potential errors gracefully, though current
        // fuel_cost_for_operation is infallible in its Result signature.
        match Type::fuel_cost_for_operation(op_type, verification_level) {
            Ok(cost) => {
                self.fuel_consumed.fetch_add(cost, Ordering::Relaxed);
            }
            Err(_e) => {
                // Log error if fuel calculation fails (should not happen
                // currently)
                // Consider using a proper logging facade if available
                // eprintln!("Error calculating fuel cost: {}", e);
            }
        }
    }

    /// Get the current total fuel consumed.
    #[must_use]
    pub fn get_fuel_consumed(&self) -> u64 {
        self.fuel_consumed.load(Ordering::Relaxed)
    }

    /// Reset all operation counters and fuel consumed to zero.
    pub fn reset(&self) {
        self.memory_reads.store(0, Ordering::Relaxed);
        self.memory_writes.store(0, Ordering::Relaxed);
        self.memory_grows.store(0, Ordering::Relaxed);
        self.memory_allocations.store(0, Ordering::Relaxed);
        self.memory_deallocations.store(0, Ordering::Relaxed);
        self.collection_pushes.store(0, Ordering::Relaxed);
        self.collection_pops.store(0, Ordering::Relaxed);
        self.collection_lookups.store(0, Ordering::Relaxed);
        self.collection_inserts.store(0, Ordering::Relaxed);
        self.collection_removes.store(0, Ordering::Relaxed);
        self.collection_validates.store(0, Ordering::Relaxed);
        self.collection_mutates.store(0, Ordering::Relaxed);
        self.checksum_calculations.store(0, Ordering::Relaxed);
        self.function_calls.store(0, Ordering::Relaxed);
        self.control_flows.store(0, Ordering::Relaxed);
        self.arithmetic_ops.store(0, Ordering::Relaxed);
        self.other_ops.store(0, Ordering::Relaxed);
        self.collection_creates.store(0, Ordering::Relaxed);
        self.collection_clears.store(0, Ordering::Relaxed);
        self.collection_truncates.store(0, Ordering::Relaxed);
        self.collection_iterates.store(0, Ordering::Relaxed);
        self.fuel_consumed.store(0, Ordering::Relaxed);
    }

    /// Get a summary of all operation counts.
    #[must_use]
    pub fn get_summary(&self) -> Summary {
        Summary {
            memory_reads: self.memory_reads.load(Ordering::Relaxed),
            memory_writes: self.memory_writes.load(Ordering::Relaxed),
            memory_grows: self.memory_grows.load(Ordering::Relaxed),
            memory_allocations: self.memory_allocations.load(Ordering::Relaxed),
            memory_deallocations: self.memory_deallocations.load(Ordering::Relaxed),
            collection_pushes: self.collection_pushes.load(Ordering::Relaxed),
            collection_pops: self.collection_pops.load(Ordering::Relaxed),
            collection_lookups: self.collection_lookups.load(Ordering::Relaxed),
            collection_inserts: self.collection_inserts.load(Ordering::Relaxed),
            collection_removes: self.collection_removes.load(Ordering::Relaxed),
            collection_validates: self.collection_validates.load(Ordering::Relaxed),
            collection_mutates: self.collection_mutates.load(Ordering::Relaxed),
            checksum_calculations: self.checksum_calculations.load(Ordering::Relaxed),
            function_calls: self.function_calls.load(Ordering::Relaxed),
            control_flows: self.control_flows.load(Ordering::Relaxed),
            arithmetic_ops: self.arithmetic_ops.load(Ordering::Relaxed),
            other_ops: self.other_ops.load(Ordering::Relaxed),
            collection_creates: self.collection_creates.load(Ordering::Relaxed),
            collection_clears: self.collection_clears.load(Ordering::Relaxed),
            collection_truncates: self.collection_truncates.load(Ordering::Relaxed),
            collection_iterates: self.collection_iterates.load(Ordering::Relaxed),
            fuel_consumed: self.fuel_consumed.load(Ordering::Relaxed),
        }
    }
}

/// A summary snapshot of operation counts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Number of memory read operations
    pub memory_reads: u64,
    /// Number of memory write operations
    pub memory_writes: u64,
    /// Number of memory grow operations
    pub memory_grows: u64,
    /// Binary std/no_std choice
    pub memory_allocations: u64,
    /// Binary std/no_std choice
    pub memory_deallocations: u64,
    /// Number of collection push operations
    pub collection_pushes: u64,
    /// Number of collection pop operations
    pub collection_pops: u64,
    /// Number of collection lookup operations
    pub collection_lookups: u64,
    /// Number of collection insert operations
    pub collection_inserts: u64,
    /// Number of collection remove operations
    pub collection_removes: u64,
    /// Number of collection validate operations
    pub collection_validates: u64,
    /// Number of collection mutate operations
    pub collection_mutates: u64,
    /// Number of checksum calculations
    pub checksum_calculations: u64,
    /// Number of function calls
    pub function_calls: u64,
    /// Number of control flow operations
    pub control_flows: u64,
    /// Number of arithmetic operations
    pub arithmetic_ops: u64,
    /// Number of other operations
    pub other_ops: u64,
    /// Number of collection create operations
    pub collection_creates: u64,
    /// Number of collection clear operations
    pub collection_clears: u64,
    /// Number of collection truncate operations
    pub collection_truncates: u64,
    /// Number of collection iterate operations
    pub collection_iterates: u64,
    /// Total fuel consumed by operations
    pub fuel_consumed: u64,
}

/// Trait for objects that can track operations
pub trait Tracking {
    /// Record an operation occurred
    fn record_operation(&self, op_type: Type);

    /// Get the current operation statistics
    fn operation_stats(&self) -> Summary;

    /// Reset operation counters
    fn reset_operation_stats(&self);
}

// Helper function to get or initialize the global counter
fn global_counter() -> &'static Counter {
    GLOBAL_COUNTER.get_or_init(Counter::new)
}

/// Record an operation using the global counter.
pub fn record_global_operation(op_type: Type, level: VerificationLevel) {
    global_counter().record_operation(op_type, level);
}

/// Get a summary from the global operation counter.
pub fn global_operation_summary() -> Summary {
    global_counter().get_summary()
}

/// Reset the global operation counter.
pub fn reset_global_operations() {
    global_counter().reset();
}

/// Get the global fuel consumed count.
pub fn global_fuel_consumed() -> u64 {
    global_counter().get_fuel_consumed()
}

/// Get the scaled cost multiplier for a given verification level.
///
/// Multipliers are scaled by 100 (e.g., 1.25 becomes 125) to allow integer
/// arithmetic.
///
/// # Errors
///
/// This function is infallible.
fn verification_cost_multiplier_scaled(level: &VerificationLevel) -> u64 {
    match level {
        VerificationLevel::Off => 100,       // 1.00 * 100
        VerificationLevel::Basic => 110,     // 1.10 * 100
        VerificationLevel::Standard => 150,  // 1.50 * 100
        VerificationLevel::Sampling => 125,  // 1.25 * 100
        VerificationLevel::Full => 200,      // 2.00 * 100
        VerificationLevel::Redundant => 250, // 2.50 * 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::VerificationLevel;

    #[test]
    fn test_operation_counter() {
        let counter = Counter::new();
        let vl_full = VerificationLevel::Full;

        counter.record_operation(Type::MemoryRead, vl_full);
        counter.record_operation(Type::MemoryWrite, vl_full);
        counter.record_operation(Type::CollectionPush, vl_full);

        let summary = counter.get_summary();
        assert_eq!(summary.memory_reads, 1);
        assert_eq!(summary.memory_writes, 1);
        assert_eq!(summary.collection_pushes, 1);

        let expected_fuel = Type::fuel_cost_for_operation(Type::MemoryRead, vl_full).unwrap()
            + Type::fuel_cost_for_operation(Type::MemoryWrite, vl_full).unwrap()
            + Type::fuel_cost_for_operation(Type::CollectionPush, vl_full).unwrap();
        assert_eq!(summary.fuel_consumed, expected_fuel);

        counter.reset();
        let summary_after_reset = counter.get_summary();
        assert_eq!(summary_after_reset.memory_reads, 0);
        assert_eq!(summary_after_reset.fuel_consumed, 0);
    }

    #[test]
    fn test_verification_level_impact() {
        let counter = Counter::new();
        let vl_off = VerificationLevel::Off;
        let vl_sampling = VerificationLevel::default(); // Sampling
        let vl_full = VerificationLevel::Full;

        counter.record_operation(Type::MemoryRead, vl_off);
        counter.record_operation(Type::MemoryRead, vl_sampling);
        counter.record_operation(Type::MemoryRead, vl_full);

        let summary = counter.get_summary();
        assert_eq!(summary.memory_reads, 3);

        let expected_fuel = Type::fuel_cost_for_operation(Type::MemoryRead, vl_off).unwrap()
            + Type::fuel_cost_for_operation(Type::MemoryRead, vl_sampling).unwrap()
            + Type::fuel_cost_for_operation(Type::MemoryRead, vl_full).unwrap();
        assert_eq!(summary.fuel_consumed, expected_fuel);
    }

    #[test]
    fn test_global_counter() {
        reset_global_operations();
        let vl_full = VerificationLevel::Full;

        record_global_operation(Type::FunctionCall, vl_full); // Was Standard
        record_global_operation(Type::CollectionValidate, vl_full); // Was Standard

        let summary = global_operation_summary();
        assert_eq!(summary.function_calls, 1);
        assert_eq!(summary.collection_validates, 1);

        let fuel = global_fuel_consumed();
        assert_eq!(fuel, summary.fuel_consumed);

        reset_global_operations();
        assert_eq!(global_fuel_consumed(), 0);
    }
}
