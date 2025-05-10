// WRT - wrt-types
// Module: Operation Tracking and Fuel Metering
// SW-REQ-ID: REQ_RUNTIME_SECURITY_003 (Example: Relates to resource consumption control)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

// #![allow(unsafe_code)] // unsafe_code is no longer directly in this file for OnceCell

//! Operation tracking for bounded collections and memory operations
//!
//! This module provides infrastructure for tracking operations performed
//! on bounded collections and memory, supporting WCET analysis and fuel
//! consumption calculations.

#[cfg(not(feature = "std"))]
use core::primitive::f64; // Added for explicit f64 import in no_std

use crate::values::FloatBits64; // Added for FloatBits64::new()
use crate::verification::VerificationLevel;
use wrt_error::Error as WrtError; // Added for the Result return type

#[cfg(not(feature = "std"))]
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "std")]
use std::sync::atomic::{AtomicU64, Ordering};

// Use WrtOnce from wrt-sync crate
use wrt_sync::once::WrtOnce;

// Global operation counter for use when a local counter isn't available
static GLOBAL_OPERATIONS_COUNTER: WrtOnce<OperationCounter> = WrtOnce::new();

/// Operation types that can be tracked for fuel consumption
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// Memory allocation operation
    MemoryAllocation,
    /// Memory deallocation operation
    MemoryDeallocation,
    /// Memory read operation
    MemoryRead,
    /// Memory write operation
    MemoryWrite,
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

impl OperationType {
    /// Returns the base cost of the operation.
    pub fn cost(self) -> u32 {
        match self {
            OperationType::MemoryAllocation => 10,
            OperationType::MemoryDeallocation => 10,
            OperationType::MemoryRead => 1,
            OperationType::MemoryWrite => 2,
            OperationType::MemoryGrow => 10,
            OperationType::CollectionPush => 2,
            OperationType::CollectionPop => 2,
            OperationType::CollectionLookup => 1,
            OperationType::CollectionInsert => 3,
            OperationType::CollectionRemove => 3,
            OperationType::CollectionValidate => 5,
            OperationType::CollectionMutate => 4,
            OperationType::ChecksumCalculation => 10,
            OperationType::ChecksumFullRecalculation => 15,
            OperationType::FunctionCall => 5,
            OperationType::ControlFlow => 2,
            OperationType::Arithmetic => 1,
            OperationType::Other => 1,
            OperationType::CollectionCreate => 5,
            OperationType::CollectionClear => 5,
            OperationType::CollectionTruncate => 3,
            OperationType::CollectionIterate => 1,
            OperationType::CollectionRead => 1,
            OperationType::CollectionWrite => 2,
            OperationType::CollectionPeek => 1,
        }
    }

    /// Returns the inherent importance of the operation (0-255).
    pub fn importance(self) -> u8 {
        match self {
            OperationType::MemoryAllocation => 50,
            OperationType::MemoryDeallocation => 50,
            OperationType::MemoryRead => 100,
            OperationType::MemoryWrite => 150,
            OperationType::MemoryGrow => 200,
            OperationType::CollectionPush => 150,
            OperationType::CollectionPop => 150,
            OperationType::CollectionLookup => 100,
            OperationType::CollectionInsert => 150,
            OperationType::CollectionRemove => 150,
            OperationType::CollectionValidate => 200,
            OperationType::CollectionMutate => 150,
            OperationType::ChecksumCalculation => 180,
            OperationType::ChecksumFullRecalculation => 190,
            OperationType::FunctionCall => 150,
            OperationType::ControlFlow => 120,
            OperationType::Arithmetic => 100,
            OperationType::Other => 100,
            OperationType::CollectionCreate => 150,
            OperationType::CollectionClear => 150,
            OperationType::CollectionTruncate => 150,
            OperationType::CollectionIterate => 100,
            OperationType::CollectionRead => importance::MEDIUM,
            OperationType::CollectionWrite => importance::HIGH,
            OperationType::CollectionPeek => importance::LOW,
        }
    }

    /// Calculate fuel cost for an operation, considering verification level
    pub fn fuel_cost_for_operation(
        op_type: OperationType,
        verification_level: VerificationLevel,
    ) -> Result<u64, WrtError> {
        let base_cost = op_type.cost() as u64;

        // Adjust cost based on verification level
        let verification_multiplier = verification_cost_multiplier(verification_level);

        let cost_f64 = base_cost as f64 * verification_multiplier;
        // Use Wasm-compliant nearest (round ties to even) from math_ops
        // cost_f64 is always finite and non-NaN here, so wasm_f64_nearest will not error.
        Ok(crate::math_ops::wasm_f64_nearest(FloatBits64::from_float(cost_f64))?.value() as u64)
    }
}

/// A counter for tracking operation counts
#[derive(Debug)]
pub struct OperationCounter {
    /// Counter for memory read operations
    memory_reads: AtomicU64,
    /// Counter for memory write operations
    memory_writes: AtomicU64,
    /// Counter for memory grow operations
    memory_grows: AtomicU64,
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

// Manual implementation of Clone for OperationCounter since AtomicU64 doesn't implement Clone
impl Clone for OperationCounter {
    fn clone(&self) -> Self {
        Self {
            memory_reads: AtomicU64::new(self.memory_reads.load(Ordering::Relaxed)),
            memory_writes: AtomicU64::new(self.memory_writes.load(Ordering::Relaxed)),
            memory_grows: AtomicU64::new(self.memory_grows.load(Ordering::Relaxed)),
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

impl Default for OperationCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationCounter {
    /// Create a new operation counter with all counts at zero
    pub fn new() -> Self {
        Self {
            memory_reads: AtomicU64::new(0),
            memory_writes: AtomicU64::new(0),
            memory_grows: AtomicU64::new(0),
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

    /// Record an operation and update fuel consumption
    pub fn record_operation(&self, op_type: OperationType, verification_level: VerificationLevel) {
        // Record the specific operation
        match op_type {
            OperationType::MemoryRead => self.memory_reads.fetch_add(1, Ordering::Relaxed),
            OperationType::MemoryWrite => self.memory_writes.fetch_add(1, Ordering::Relaxed),
            OperationType::MemoryGrow => self.memory_grows.fetch_add(1, Ordering::Relaxed),
            OperationType::CollectionPush => self.collection_pushes.fetch_add(1, Ordering::Relaxed),
            OperationType::CollectionPop => self.collection_pops.fetch_add(1, Ordering::Relaxed),
            OperationType::CollectionLookup => {
                self.collection_lookups.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionInsert => {
                self.collection_inserts.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionRemove => {
                self.collection_removes.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionValidate => {
                self.collection_validates.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionMutate => {
                self.collection_mutates.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::ChecksumCalculation => {
                self.checksum_calculations.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::FunctionCall => self.function_calls.fetch_add(1, Ordering::Relaxed),
            OperationType::ControlFlow => self.control_flows.fetch_add(1, Ordering::Relaxed),
            OperationType::Arithmetic => self.arithmetic_ops.fetch_add(1, Ordering::Relaxed),
            OperationType::Other => self.other_ops.fetch_add(1, Ordering::Relaxed),
            OperationType::CollectionCreate => {
                self.collection_creates.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionClear => {
                self.collection_clears.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionTruncate => {
                self.collection_truncates.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionIterate => {
                self.collection_iterates.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::ChecksumFullRecalculation => {
                self.checksum_calculations.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionRead => {
                self.collection_lookups.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionWrite => {
                self.collection_inserts.fetch_add(1, Ordering::Relaxed)
            }
            OperationType::CollectionPeek => {
                self.collection_lookups.fetch_add(1, Ordering::Relaxed)
            }
        };

        // Calculate base fuel cost
        let base_cost = op_type.cost() as u64;

        // Adjust cost based on verification level
        let verification_multiplier = verification_cost_multiplier(verification_level);

        // Use Wasm-compliant nearest (round ties to even) from math_ops
        let cost_f64 = base_cost as f64 * verification_multiplier;
        // cost_f64 is always finite and non-NaN here, so wasm_f64_nearest will not error.
        let total_cost: u64 = crate::math_ops::wasm_f64_nearest(FloatBits64::from_float(cost_f64))
            .expect("cost_f64 is finite, nearest op should not fail")
            .value() as u64;

        // Update fuel consumed
        self.fuel_consumed.fetch_add(total_cost, Ordering::Relaxed);
    }

    /// Get the total fuel consumed by all operations
    pub fn get_fuel_consumed(&self) -> u64 {
        self.fuel_consumed.load(Ordering::Relaxed)
    }

    /// Reset all counters to zero
    pub fn reset(&self) {
        self.memory_reads.store(0, Ordering::Relaxed);
        self.memory_writes.store(0, Ordering::Relaxed);
        self.memory_grows.store(0, Ordering::Relaxed);
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

    /// Get a summary of all operation counts
    pub fn get_summary(&self) -> OperationSummary {
        OperationSummary {
            memory_reads: self.memory_reads.load(Ordering::Relaxed),
            memory_writes: self.memory_writes.load(Ordering::Relaxed),
            memory_grows: self.memory_grows.load(Ordering::Relaxed),
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

/// A snapshot of operation counts
#[derive(Debug, Clone, Copy)]
pub struct OperationSummary {
    /// Number of memory read operations
    pub memory_reads: u64,
    /// Number of memory write operations
    pub memory_writes: u64,
    /// Number of memory grow operations
    pub memory_grows: u64,
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

/// Trait for types that need to track operation counts
pub trait OperationTracking {
    /// Record an operation occurred
    fn record_operation(&self, op_type: OperationType);

    /// Get the current operation statistics
    fn operation_stats(&self) -> OperationSummary;

    /// Reset operation counters
    fn reset_operation_stats(&self);
}

/// Get the global counter instance, initializing it if necessary
fn global_counter() -> &'static OperationCounter {
    GLOBAL_OPERATIONS_COUNTER.get_or_init(OperationCounter::new)
}

/// Record an operation in the global counter
pub fn record_global_operation(op_type: OperationType, level: VerificationLevel) {
    global_counter().record_operation(op_type, level);
}

/// Get the summary from the global counter
pub fn global_operation_summary() -> OperationSummary {
    global_counter().get_summary()
}

/// Reset the global counter
pub fn reset_global_operations() {
    global_counter().reset();
}

/// Get the total fuel consumed from the global counter
pub fn global_fuel_consumed() -> u64 {
    global_counter().get_fuel_consumed()
}

fn verification_cost_multiplier(level: VerificationLevel) -> f64 {
    match level {
        VerificationLevel::Off => 1.0,     // Changed from None
        VerificationLevel::Basic => 1.1,   // Small overhead for basic checks
        VerificationLevel::Sampling => 1.25, // Moderate overhead for sampling (was Standard)
        VerificationLevel::Full => 2.0,    // Higher overhead for full checks
        VerificationLevel::Redundant => 2.5, // Highest for redundant checks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_counter() {
        let counter = OperationCounter::new();
        counter.record_operation(OperationType::MemoryRead, VerificationLevel::Standard);
        assert_eq!(counter.memory_reads.load(Ordering::Relaxed), 1);
        // Fuel cost for MemoryRead (1) * Standard (1.5) = 1.5. wasm_f64_nearest(1.5) = 2.0
        assert_eq!(counter.get_fuel_consumed(), 2);

        counter.record_operation(OperationType::MemoryGrow, VerificationLevel::Full);
        assert_eq!(counter.memory_grows.load(Ordering::Relaxed), 1);
        // Prev fuel: 2. New Fuel: MemoryGrow (10) * Full (2.0) = 20.0. wasm_f64_nearest(20.0) = 20.0. Total = 2+20=22
        assert_eq!(counter.get_fuel_consumed(), 22);

        counter.reset();
        assert_eq!(counter.get_fuel_consumed(), 0);
    }

    #[test]
    fn test_verification_level_impact() {
        let op = OperationType::ChecksumCalculation; // base_cost = 10

        // None: 10 * 1.0 = 10.0. nearest(10.0) = 10
        let cost_none =
            OperationType::fuel_cost_for_operation(op, VerificationLevel::None).unwrap();
        assert_eq!(cost_none, 10);

        // Sampling: 10 * 1.2 = 12.0. nearest(12.0) = 12
        let cost_sampling =
            OperationType::fuel_cost_for_operation(op, VerificationLevel::Sampling).unwrap();
        assert_eq!(cost_sampling, 12);

        // Standard: 10 * 1.5 = 15.0. nearest(15.0) = 15
        let cost_standard =
            OperationType::fuel_cost_for_operation(op, VerificationLevel::Standard).unwrap();
        assert_eq!(cost_standard, 15);

        // Full: 10 * 2.0 = 20.0. nearest(20.0) = 20
        let cost_full =
            OperationType::fuel_cost_for_operation(op, VerificationLevel::Full).unwrap();
        assert_eq!(cost_full, 20);

        // Test a case that would have rounded differently with round_half_away_from_zero
        // Base cost 2, multiplier 1.2 => 2.4. nearest(2.4) = 2.
        // (Old round_half_away_from_zero would also be 2)
        let op2 = OperationType::CollectionPop; // base_cost = 2
        let cost_sampling_op2 =
            OperationType::fuel_cost_for_operation(op2, VerificationLevel::Sampling).unwrap();
        assert_eq!(cost_sampling_op2, 2);

        // Base cost 2, multiplier 1.5 => 3.0. nearest(3.0) = 3.
        let cost_standard_op2 =
            OperationType::fuel_cost_for_operation(op2, VerificationLevel::Standard).unwrap();
        assert_eq!(cost_standard_op2, 3);

        // Base cost 3, multiplier 1.5 => 4.5. nearest(4.5) = 4.0 (ties to even)
        // Old round_half_away_from_zero(4.5) would be 5.0.
        let op3 = OperationType::CollectionInsert; // base_cost = 3
        let cost_standard_op3 =
            OperationType::fuel_cost_for_operation(op3, VerificationLevel::Standard).unwrap();
        assert_eq!(cost_standard_op3, 4);
    }

    #[cfg(feature = "std")] // These tests involve global state, run with std feature
    #[test]
    fn test_global_counter() {
        reset_global_operations();
        record_global_operation(OperationType::FunctionCall, VerificationLevel::None);
        // FunctionCall (5) * None (1.0) = 5.0. nearest(5.0) = 5
        assert_eq!(global_fuel_consumed(), 5);

        let summary = global_operation_summary();
        assert_eq!(summary.function_calls, 1);
        assert_eq!(summary.fuel_consumed, 5);

        reset_global_operations();
        assert_eq!(global_fuel_consumed(), 0);
    }
}
