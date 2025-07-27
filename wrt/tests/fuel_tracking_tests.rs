//! Tests for operation tracking and enhanced fuel model
//!
//! This module tests the integration of operation tracking with
//! the fuel system for WCET analysis.

use wrt::{
    instructions::{
        instruction_type::Instruction as InstructionType,
        Instruction,
    },
    module::Module,
    types::{
        FuncType,
        ValueType,
    },
    values::Value,
    StacklessEngine,
};
use wrt_foundation::{
    global_operation_summary,
    record_global_operation,
    reset_global_operations,
    BoundedVec,
    OperationType,
    VerificationLevel,
};

#[test]
fn test_operation_tracking_in_fuel_system() {
    // Create a simple module
    let mut module = Module::new().unwrap();

    // Create a function type
    let func_type = FuncType {
        params:  vec![ValueType::I32],
        results: vec![ValueType::I32],
    };
    module.types.push(func_type);

    // Create a function with many instructions to consume fuel
    let code = vec![
        Instruction::LocalGet(0),   // Get the input parameter
        Instruction::I32Const(100), // Load constant 100
        Instruction::I32Add,        // Add - should register as arithmetic op
        Instruction::I32Const(10),  // Load another constant
        Instruction::I32Mul,        // Multiply - another arithmetic op
        Instruction::End,           // End function
    ];
    let function = wrt::module::Function {
        type_idx: 0,
        locals: vec![],
        code,
    };
    module.functions.push(function);

    // Create an export entry for the function
    module.exports.push(wrt::module::Export {
        name:  "calc".to_string(),
        kind:  wrt::module::ExportKind::Function,
        index: 0,
    });

    // Reset operation tracking
    reset_global_operations();

    // Create a stackless engine with the module
    let mut engine = StacklessEngine::new();
    engine.instantiate(module).unwrap();

    // Set a fuel limit
    engine.set_fuel(Some(50));

    // Set verification level
    engine.set_verification_level(VerificationLevel::Standard);

    // Invoke the function
    let result = engine.invoke_export("calc", &[Value::I32(5)]);
    assert!(result.is_ok());

    // Get the result value
    let result_value = result.unwrap();
    assert_eq!(result_value, vec![Value::I32(1050)]); // (5 + 100) * 10 = 1050

    // Check the operation stats
    let stats = global_operation_summary();

    // We should have at least some operations recorded
    assert!(
        stats.function_calls > 0,
        "Should have recorded function calls"
    );
    assert!(
        stats.arithmetic_ops > 0,
        "Should have recorded arithmetic operations"
    );

    // Check that fuel was consumed
    assert!(stats.fuel_consumed > 0, "Should have consumed fuel");

    // Check engine stats match operation tracking
    let engine_stats = engine.stats();
    assert_eq!(engine_stats.fuel_consumed, stats.fuel_consumed);
}

#[test]
fn test_fuel_exhaustion_from_operations() {
    // Initialize global tracking
    reset_global_operations();

    // Create a module with a looping function
    let mut module = Module::new().unwrap();

    // Add function type
    let func_type = FuncType {
        params:  vec![ValueType::I32], // Loop counter
        results: vec![ValueType::I32], // Final result
    };
    module.types.push(func_type);

    // Create a function with a loop that will consume a lot of fuel
    let code = vec![
        Instruction::LocalGet(0), // Get the input parameter (loop count)
        Instruction::I32Const(0), // Initialize accumulator to 0
        // Start a loop that will execute N times
        Instruction::Block(
            InstructionType::Block,
            vec![Instruction::Loop(
                InstructionType::Loop,
                vec![
                    // Check loop condition (break if counter is 0)
                    Instruction::LocalGet(0),
                    Instruction::I32Eqz,
                    Instruction::BrIf(1), // Break out of both blocks if counter is 0
                    // Accumulator += counter
                    Instruction::LocalGet(0),
                    Instruction::I32Add,
                    // Decrement counter
                    Instruction::LocalGet(0),
                    Instruction::I32Const(1),
                    Instruction::I32Sub,
                    Instruction::LocalSet(0),
                    // Continue loop
                    Instruction::Br(0),
                ],
            )],
        ),
        Instruction::End,
    ];

    let function = wrt::module::Function {
        type_idx: 0,
        locals: vec![],
        code,
    };
    module.functions.push(function);

    // Create an export entry for the function
    module.exports.push(wrt::module::Export {
        name:  "loop".to_string(),
        kind:  wrt::module::ExportKind::Function,
        index: 0,
    });

    // Create a stackless engine with the module
    let mut engine = StacklessEngine::new();
    engine.instantiate(module).unwrap();

    // Set a very low fuel limit to ensure we run out
    engine.set_fuel(Some(20));

    // Set verification level to generate more fuel consumption
    engine.set_verification_level(VerificationLevel::Full);

    // Invoke the function with a high loop count
    // This should run out of fuel and return an error
    let result = engine.invoke_export("loop", &[Value::I32(100)]);

    // We should get an insufficient fuel error
    assert!(result.is_err(), "Engine should run out of fuel");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Insufficient fuel"),
        "Error should indicate insufficient fuel: {}",
        err
    );

    // Check engine stats
    let engine_stats = engine.stats();
    assert!(
        engine_stats.fuel_exhausted_count > 0,
        "Should record fuel exhaustion"
    );
    assert!(
        engine_stats.fuel_consumed > 0,
        "Should record fuel consumption"
    );
}

#[test]
fn test_bounded_collections_fuel_impact() {
    // Reset global tracking
    reset_global_operations();

    // Test with different verification levels
    let verification_levels = [
        VerificationLevel::None,
        VerificationLevel::Sampling,
        VerificationLevel::Standard,
        VerificationLevel::Full,
    ];

    let mut fuel_consumed = Vec::new());

    // Perform the same operations with each verification level
    for &level in &verification_levels {
        reset_global_operations();

        // Create a bounded vector with the current verification level
        let mut vec = BoundedVec::<Vec<u8>, 100>::with_verification_level(level);

        // Perform a series of operations
        for i in 0..10 {
            let _ = vec.push(vec![i, i + 1, i + 2]);
        }

        // Read some values
        for i in 0..5 {
            let _ = vec.get(i);
        }

        // Remove some values
        for i in 0..3 {
            let _ = vec.remove(i);
        }

        // Get the fuel consumed for this verification level
        let stats = global_operation_summary();
        fuel_consumed.push(stats.fuel_consumed);
    }

    // Verify that higher verification levels consume more fuel
    for i in 1..verification_levels.len() {
        assert!(
            fuel_consumed[i] > fuel_consumed[i - 1],
            "Higher verification levels should consume more fuel: {:?} vs {:?} for {:?} vs {:?}",
            fuel_consumed[i],
            fuel_consumed[i - 1],
            verification_levels[i],
            verification_levels[i - 1]
        );
    }

    // Verify that full verification consumes significantly more than none
    assert!(
        fuel_consumed[3] > fuel_consumed[0] * 2,
        "Full verification should consume significantly more fuel than None"
    );
}

#[test]
fn test_manual_operation_tracking() {
    // Reset global tracking
    reset_global_operations();

    // Record various operations manually
    record_global_operation(OperationType::MemoryRead, VerificationLevel::Standard);
    record_global_operation(OperationType::MemoryWrite, VerificationLevel::Standard);
    record_global_operation(OperationType::FunctionCall, VerificationLevel::Standard);
    record_global_operation(OperationType::Arithmetic, VerificationLevel::Standard);

    // Get the stats
    let stats = global_operation_summary();

    // Check each operation type was recorded
    assert_eq!(stats.memory_reads, 1, "Should record memory read");
    assert_eq!(stats.memory_writes, 1, "Should record memory write");
    assert_eq!(stats.function_calls, 1, "Should record function call");
    assert_eq!(stats.arithmetic_ops, 1, "Should record arithmetic op");

    // Calculate expected fuel consumption
    let expected_fuel = (OperationType::MemoryRead.fuel_cost() as f64 * 1.5).round() as u64
        + (OperationType::MemoryWrite.fuel_cost() as f64 * 1.5).round() as u64
        + (OperationType::FunctionCall.fuel_cost() as f64 * 1.5).round() as u64
        + (OperationType::Arithmetic.fuel_cost() as f64 * 1.5).round() as u64;

    // Verify fuel consumption
    assert_eq!(
        stats.fuel_consumed, expected_fuel,
        "Should consume correct amount of fuel for operations"
    );
}
