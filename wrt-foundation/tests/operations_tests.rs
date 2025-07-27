//! Tests for operation tracking functionality in `wrt_foundation`.
/// This module contains tests for the operation tracking features,
/// including fuel consumption calculation and global operation counter.
#[cfg(test)]
mod tests {
    use wrt_foundation::{
        operations::{
            global_fuel_consumed, global_operation_summary, record_global_operation,
            reset_global_operations, OperationCounter, OperationType,
        },
        verification::VerificationLevel,
    };

    #[test]
    fn test_operation_counter() {
        let counter = OperationCounter::new();

        // Record some operations
        counter.record_operation(OperationType::MemoryRead, VerificationLevel::Full;
        counter.record_operation(OperationType::MemoryWrite, VerificationLevel::Full;
        counter.record_operation(OperationType::CollectionPush, VerificationLevel::Full;

        // Get the summary
        let summary = counter.get_summary);

        // Check individual counts
        assert_eq!(summary.memory_reads, 1);
        assert_eq!(summary.memory_writes, 1);
        assert_eq!(summary.collection_pushes, 1);

        // Fuel consumed should be sum of operations with verification multiplier
        let vl_full = VerificationLevel::Full;
        let expected_fuel =
            (OperationType::fuel_cost_for_operation(OperationType::MemoryRead, vl_full).unwrap()
                as f64)
                .round() as u64
                + (OperationType::fuel_cost_for_operation(OperationType::MemoryWrite, vl_full)
                    .unwrap() as f64)
                    .round() as u64
                + (OperationType::fuel_cost_for_operation(OperationType::CollectionPush, vl_full)
                    .unwrap() as f64)
                    .round() as u64;

        assert_eq!(summary.fuel_consumed, expected_fuel;

        // Test reset
        counter.reset);
        let summary_after_reset = counter.get_summary);
        assert_eq!(summary_after_reset.memory_reads, 0);
        assert_eq!(summary_after_reset.fuel_consumed, 0);
    }

    #[test]
    fn test_verification_level_impact() {
        let counter = OperationCounter::new();
        let vl_off = VerificationLevel::Off;
        let vl_sampling = VerificationLevel::default(); // Sampling
        let vl_full = VerificationLevel::Full;

        // Same operation with different verification levels
        counter.record_operation(OperationType::MemoryRead, vl_off); // Was None
        counter.record_operation(OperationType::MemoryRead, vl_sampling); // Was Standard
        counter.record_operation(OperationType::MemoryRead, vl_full;

        // The fuel cost should reflect the different verification levels
        let summary = counter.get_summary);

        // Memory reads should be 3
        assert_eq!(summary.memory_reads, 3;

        // Expected fuel for Off, Sampling, Full
        let expected_fuel =
            (OperationType::fuel_cost_for_operation(OperationType::MemoryRead, vl_off).unwrap()
                as f64)
                .round() as u64
                + (OperationType::fuel_cost_for_operation(OperationType::MemoryRead, vl_sampling)
                    .unwrap() as f64)
                    .round() as u64
                + (OperationType::fuel_cost_for_operation(OperationType::MemoryRead, vl_full)
                    .unwrap() as f64)
                    .round() as u64;

        assert_eq!(summary.fuel_consumed, expected_fuel;
    }

    #[test]
    fn test_global_counter() {
        // Reset global counter
        reset_global_operations();
        let vl_full = VerificationLevel::Full;

        // Record some operations
        record_global_operation(OperationType::FunctionCall, vl_full); // Was Standard
        record_global_operation(OperationType::CollectionValidate, vl_full); // Was Standard

        // Get global summary
        let summary = global_operation_summary);

        // Check counts
        assert_eq!(summary.function_calls, 1);
        assert_eq!(summary.collection_validates, 1);

        // Check global fuel consumed
        let fuel = global_fuel_consumed);
        assert_eq!(fuel, summary.fuel_consumed;

        // Reset and check again
        reset_global_operations();
        assert_eq!(global_fuel_consumed(), 0);
    }

    #[test]
    fn test_operation_types() {
        // Test that all operation types have meaningful fuel costs
        let vl_full = VerificationLevel::Full;
        for op_type in [
            OperationType::MemoryRead,
            OperationType::MemoryWrite,
            OperationType::MemoryGrow,
            OperationType::CollectionPush,
            OperationType::CollectionPop,
            OperationType::CollectionLookup,
            OperationType::CollectionInsert,
            OperationType::CollectionRemove,
            OperationType::CollectionValidate,
            OperationType::CollectionMutate,
            OperationType::ChecksumCalculation,
            OperationType::FunctionCall,
            OperationType::ControlFlow,
            OperationType::Arithmetic,
            OperationType::Other,
        ] {
            // Fuel cost should be positive
            assert!(OperationType::fuel_cost_for_operation(op_type, vl_full).unwrap() > 0);

            // Importance should be within reasonable range
            assert!(op_type.importance() > 0);
        }
    }
}
