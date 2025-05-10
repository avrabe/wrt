//! Tests for operation tracking functionality in `wrt_types`.
///
/// This module contains tests for the operation tracking features,
/// including fuel consumption calculation and global operation counter.
#[cfg(test)]
mod tests {
    use wrt_types::operations::{
        global_fuel_consumed, global_operation_summary, record_global_operation,
        reset_global_operations, OperationCounter, OperationType,
    };
    use wrt_types::verification::VerificationLevel;

    #[test]
    fn test_operation_counter() {
        let counter = OperationCounter::new();

        // Record some operations
        counter.record_operation(OperationType::MemoryRead, VerificationLevel::Standard);
        counter.record_operation(OperationType::MemoryWrite, VerificationLevel::Standard);
        counter.record_operation(OperationType::CollectionPush, VerificationLevel::Standard);

        // Get the summary
        let summary = counter.get_summary();

        // Check individual counts
        assert_eq!(summary.memory_reads, 1);
        assert_eq!(summary.memory_writes, 1);
        assert_eq!(summary.collection_pushes, 1);

        // Fuel consumed should be sum of operations with verification multiplier
        let expected_fuel = (OperationType::MemoryRead.fuel_cost() as f64 * 1.5).round() as u64
            + (OperationType::MemoryWrite.fuel_cost() as f64 * 1.5).round() as u64
            + (OperationType::CollectionPush.fuel_cost() as f64 * 1.5).round() as u64;

        assert_eq!(summary.fuel_consumed, expected_fuel);

        // Test reset
        counter.reset();
        let summary_after_reset = counter.get_summary();
        assert_eq!(summary_after_reset.memory_reads, 0);
        assert_eq!(summary_after_reset.fuel_consumed, 0);
    }

    #[test]
    fn test_verification_level_impact() {
        let counter = OperationCounter::new();

        // Same operation with different verification levels
        counter.record_operation(OperationType::MemoryRead, VerificationLevel::None);
        counter.record_operation(OperationType::MemoryRead, VerificationLevel::Standard);
        counter.record_operation(OperationType::MemoryRead, VerificationLevel::Full);

        // The fuel cost should reflect the different verification levels
        let summary = counter.get_summary();

        // Memory reads should be 3
        assert_eq!(summary.memory_reads, 3);

        // Expected fuel: 1*1.0 + 1*1.5 + 1*2.0 = 4.5 -> 5 (rounded)
        let expected_fuel = (OperationType::MemoryRead.fuel_cost() as f64 * 1.0).round() as u64
            + (OperationType::MemoryRead.fuel_cost() as f64 * 1.5).round() as u64
            + (OperationType::MemoryRead.fuel_cost() as f64 * 2.0).round() as u64;

        assert_eq!(summary.fuel_consumed, expected_fuel);
    }

    #[test]
    fn test_global_counter() {
        // Reset global counter
        reset_global_operations();

        // Record some operations
        record_global_operation(OperationType::FunctionCall, VerificationLevel::Standard);
        record_global_operation(OperationType::CollectionValidate, VerificationLevel::Standard);

        // Get global summary
        let summary = global_operation_summary();

        // Check counts
        assert_eq!(summary.function_calls, 1);
        assert_eq!(summary.collection_validates, 1);

        // Check global fuel consumed
        let fuel = global_fuel_consumed();
        assert_eq!(fuel, summary.fuel_consumed);

        // Reset and check again
        reset_global_operations();
        assert_eq!(global_fuel_consumed(), 0);
    }

    #[test]
    fn test_operation_types() {
        // Test that all operation types have meaningful fuel costs
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
            assert!(op_type.fuel_cost() > 0);

            // Importance should be within reasonable range
            assert!(op_type.importance() > 0);
        }
    }
}
