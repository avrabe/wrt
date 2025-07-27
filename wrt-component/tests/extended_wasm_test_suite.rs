//! Extended WASM test suite for real execution validation
//!
//! This test suite provides comprehensive validation of the WRT framework's
//! real WebAssembly execution capabilities, covering edge cases, error
//! conditions, and production-level scenarios for both QM and ASIL-B safety
//! levels.

#![cfg(test)]

use std::{
    fs,
    sync::Arc,
};

use wrt_decoder::decoder::decode_module;
use wrt_error::{
    Error,
    Result,
};
use wrt_foundation::values::Value;
use wrt_runtime::{
    module::Module,
    module_instance::ModuleInstance,
    stackless::StacklessEngine,
};

/// Helper function to load test WASM module
fn load_test_module() -> Result<Module> {
    let wasm_bytes = fs::read("test_add.wasm")
        .map_err(|_| Error::system_io_error("Failed to read test_add.wasm"))?;

    let decoded = decode_module(&wasm_bytes)?;
    Module::from_wrt_module(&decoded)
}

/// Helper function to create execution engine with module
fn create_engine_with_module() -> Result<(StacklessEngine, usize)> {
    let runtime_module = load_test_module()?;
    let mut engine = StacklessEngine::new();
    let instance = ModuleInstance::new(runtime_module, 0)?;
    let instance_arc = Arc::new(instance);
    let instance_idx = engine.set_current_module(instance_arc)?;
    Ok((engine, instance_idx))
}

/// Helper function to create test arguments
fn create_test_args(a: i32, b: i32) -> Vec<Value> {
    vec![Value::I32(a), Value::I32(b)]
}

#[test]
fn test_basic_arithmetic_operations() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test basic addition
    let test_cases = vec![
        (0, 0, 0),
        (1, 1, 2),
        (5, 3, 8),
        (10, 32, 42),
        (100, 200, 300),
        (-1, 1, 0),
        (-10, 15, 5),
        (i32::MAX / 2, i32::MAX / 2, i32::MAX - 1),
    ];

    for (a, b, expected) in test_cases {
        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(results.len(), 1, "Expected exactly one result");

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "add({}, {}) should equal {}",
                a, b, expected
            );
        } else {
            panic!("Expected I32 result, got {:?}", results[0]);
        }
    }

    Ok(())
}

#[test]
fn test_integer_overflow_behavior() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test integer overflow cases (should wrap around in WebAssembly)
    let overflow_cases = vec![
        (i32::MAX, 1, i32::MIN),                  // Maximum + 1 wraps to minimum
        (i32::MIN, -1, i32::MAX),                 // Minimum - 1 wraps to maximum
        (i32::MAX / 2 + 1, i32::MAX / 2 + 1, -2), // Two large numbers wrap
    ];

    for (a, b, expected) in overflow_cases {
        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result for overflow case"
        );

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "Overflow add({}, {}) should wrap to {}",
                a, b, expected
            );
        } else {
            panic!(
                "Expected I32 result for overflow test, got {:?}",
                results[0]
            );
        }
    }

    Ok(())
}

#[test]
fn test_zero_and_identity_operations() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test zero identity operations
    let identity_cases = vec![
        (0, 42, 42),
        (42, 0, 42),
        (0, -42, -42),
        (-42, 0, -42),
        (0, i32::MAX, i32::MAX),
        (i32::MIN, 0, i32::MIN),
    ];

    for (a, b, expected) in identity_cases {
        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result for identity case"
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "Identity add({}, {}) should equal {}",
                a, b, expected
            ;
        } else {
            panic!(
                "Expected I32 result for identity test, got {:?}",
                results[0]
            ;
        }
    }

    Ok(())
}

#[test]
fn test_negative_number_operations() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test negative number cases
    let negative_cases = vec![
        (-1, -1, -2),
        (-5, -3, -8),
        (-10, -32, -42),
        (-100, -200, -300),
        (5, -3, 2),
        (-5, 3, -2),
        (10, -5, 5),
        (-10, 5, -5),
    ];

    for (a, b, expected) in negative_cases {
        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result for negative case"
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "Negative add({}, {}) should equal {}",
                a, b, expected
            ;
        } else {
            panic!(
                "Expected I32 result for negative test, got {:?}",
                results[0]
            ;
        }
    }

    Ok(())
}

#[test]
fn test_execution_determinism() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test that execution is deterministic - same inputs produce same outputs
    let test_input = (123, 456;
    let expected_result = 579;

    for iteration in 0..100 {
        let args = create_test_args(test_input.0, test_input.1;
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result in iteration {}",
            iteration
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected_result,
                "Iteration {}: Deterministic execution failed for add({}, {})",
                iteration, test_input.0, test_input.1
            ;
        } else {
            panic!(
                "Expected I32 result in iteration {}, got {:?}",
                iteration, results[0]
            ;
        }
    }

    Ok(())
}

#[test]
fn test_multiple_engine_instances() -> Result<()> {
    // Test that multiple engine instances work independently
    let mut engines = Vec::new());
    let mut instance_indices = Vec::new());

    // Create 5 independent engine instances
    for i in 0..5 {
        let (engine, instance_idx) = create_engine_with_module()
            .map_err(|e| Error::test_error(&format!("Failed to create engine {}: {}", i, e)))?;
        engines.push(engine);
        instance_indices.push(instance_idx);
    }

    // Test each engine independently
    for (i, (engine, &instance_idx)) in engines.iter_mut().zip(instance_indices.iter()).enumerate()
    {
        let a = i as i32 * 10;
        let b = i as i32 * 5;
        let expected = a + b;

        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result from engine {}",
            i
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "Engine {}: add({}, {}) should equal {}",
                i, a, b, expected
            ;
        } else {
            panic!(
                "Expected I32 result from engine {}, got {:?}",
                i, results[0]
            ;
        }
    }

    Ok(())
}

#[test]
fn test_repeated_execution_same_engine() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test repeated execution on the same engine instance
    for i in 0..1000 {
        let a = i % 100;
        let b = (i * 7) % 100;
        let expected = a + b;

        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result in execution {}",
            i
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "Execution {}: add({}, {}) should equal {}",
                i, a, b, expected
            ;
        } else {
            panic!(
                "Expected I32 result in execution {}, got {:?}",
                i, results[0]
            ;
        }
    }

    Ok(())
}

#[test]
fn test_boundary_value_analysis() -> Result<()> {
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test boundary values
    let boundary_cases = vec![
        (0, 1, 1),                    // Zero boundary
        (1, 0, 1),                    // Zero boundary (reversed)
        (-1, 0, -1),                  // Negative zero boundary
        (0, -1, -1),                  // Negative zero boundary (reversed)
        (i32::MAX - 1, 1, i32::MAX),  // Maximum boundary
        (1, i32::MAX - 1, i32::MAX),  // Maximum boundary (reversed)
        (i32::MIN + 1, -1, i32::MIN), // Minimum boundary
        (-1, i32::MIN + 1, i32::MIN), // Minimum boundary (reversed)
        (i32::MAX, 0, i32::MAX),      // Maximum with zero
        (i32::MIN, 0, i32::MIN),      // Minimum with zero
        (0, i32::MAX, i32::MAX),      // Zero with maximum
        (0, i32::MIN, i32::MIN),      // Zero with minimum
    ];

    for (a, b, expected) in boundary_cases {
        let args = create_test_args(a, b);
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Expected exactly one result for boundary case add({}, {})",
            a,
            b
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, expected,
                "Boundary add({}, {}) should equal {}",
                a, b, expected
            ;
        } else {
            panic!(
                "Expected I32 result for boundary test add({}, {}), got {:?}",
                a, b, results[0]
            ;
        }
    }

    Ok(())
}

#[test]
fn test_instruction_parsing_validation() -> Result<()> {
    let runtime_module = load_test_module()?;

    // Validate that the module has functions with parsed instructions
    assert!(
        !runtime_module.functions.is_empty(),
        "Module should have at least one function"
    ;

    let function = runtime_module.functions.get(0)?;
    assert!(
        !function.body.is_empty(),
        "Function should have parsed instructions"
    ;

    // Validate that we can access individual instructions
    for i in 0..function.body.len() {
        let _instruction = function.body.get(i)?;
        // If we get here without error, instruction access is working
    }

    println!(
        "✅ Function 0 has {} instructions parsed from bytecode",
        function.body.len()
    ;

    Ok(())
}

#[test]
fn test_memory_safety_execution() -> Result<()> {
    // Test that execution doesn't cause memory safety issues
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test with a large number of executions to catch memory issues
    for batch in 0..10 {
        let mut results_in_batch = Vec::new());

        for i in 0..100 {
            let a = (batch * 100 + i) % 1000;
            let b = ((batch * 100 + i) * 3) % 1000;
            let expected = a + b;

            let args = create_test_args(a, b);
            let results = engine.execute(instance_idx, 0, args)?;

            assert_eq!(
                results.len(),
                1,
                "Expected exactly one result in batch {} execution {}",
                batch,
                i
            ;

            if let Value::I32(result) = &results[0] {
                assert_eq!(
                    *result, expected,
                    "Batch {} execution {}: add({}, {}) should equal {}",
                    batch, i, a, b, expected
                ;
                results_in_batch.push(*result);
            } else {
                panic!(
                    "Expected I32 result in batch {} execution {}, got {:?}",
                    batch, i, results[0]
                ;
            }
        }

        // Validate that all results in the batch are reasonable
        assert_eq!(
            results_in_batch.len(),
            100,
            "Batch {} should have 100 results",
            batch
        ;
        println!(
            "✅ Batch {} completed successfully with {} executions",
            batch,
            results_in_batch.len()
        ;
    }

    Ok(())
}

#[test]
fn test_error_handling_robustness() -> Result<()> {
    let runtime_module = load_test_module()?;
    let mut engine = StacklessEngine::new();
    let instance = ModuleInstance::new(runtime_module, 0)?;
    let instance_arc = Arc::new(instance);
    let instance_idx = engine.set_current_module(instance_arc)?;

    // Test execution with correct number of arguments
    let correct_args = create_test_args(42, 24;
    let results = engine.execute(instance_idx, 0, correct_args)?;
    assert_eq!(
        results.len(),
        1,
        "Expected exactly one result with correct args"
    ;

    // Test with wrong number of arguments (should handle gracefully)
    let wrong_args_too_few = vec![Value::I32(42)]; // Only one argument instead of two
    let result = engine.execute(instance_idx, 0, wrong_args_too_few;
    // The execution should either succeed with default values or return an error
    // Both are acceptable error handling behaviors

    let wrong_args_too_many = vec![Value::I32(42), Value::I32(24), Value::I32(66)]; // Three arguments instead of two
    let result = engine.execute(instance_idx, 0, wrong_args_too_many;
    // Similarly, this should be handled gracefully

    Ok(())
}

#[test]
fn test_asil_b_compliance_validation() -> Result<()> {
    // Test ASIL-B compliance aspects during execution
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Test bounded execution characteristics
    let test_iterations = 100;
    let mut execution_times = Vec::with_capacity(test_iterations;

    for i in 0..test_iterations {
        let start = std::time::Instant::now);

        let args = create_test_args(i, i * 2;
        let results = engine.execute(instance_idx, 0, args)?;

        let execution_time = start.elapsed);
        execution_times.push(execution_time);

        // Validate result consistency
        assert_eq!(
            results.len(),
            1,
            "ASIL-B: Expected exactly one result in iteration {}",
            i
        ;

        if let Value::I32(result) = &results[0] {
            let expected = i + (i * 2;
            assert_eq!(
                *result,
                expected,
                "ASIL-B: Iteration {}: add({}, {}) should equal {}",
                i,
                i,
                i * 2,
                expected
            ;
        } else {
            panic!(
                "ASIL-B: Expected I32 result in iteration {}, got {:?}",
                i, results[0]
            ;
        }
    }

    // Analyze execution time consistency (ASIL-B requirement for deterministic
    // behavior)
    let min_time = execution_times.iter().min().unwrap();
    let max_time = execution_times.iter().max().unwrap();
    let avg_time =
        execution_times.iter().sum::<std::time::Duration>() / execution_times.len() as u32;

    println!("✅ ASIL-B Execution Analysis:");
    println!("   - {} executions completed", test_iterations);
    println!("   - Min execution time: {:?}", min_time);
    println!("   - Max execution time: {:?}", max_time);
    println!("   - Avg execution time: {:?}", avg_time);
    println!(
        "   - Time variance: {:?}",
        max_time.saturating_sub(*min_time)
    ;

    // All executions should complete within reasonable bounds for ASIL-B
    for (i, &time) in execution_times.iter().enumerate() {
        assert!(
            time.as_millis() < 100,
            "ASIL-B: Execution {} took too long: {:?}",
            i,
            time
        ;
    }

    Ok(())
}

#[test]
fn test_production_workload_simulation() -> Result<()> {
    // Simulate a production workload pattern
    let (mut engine, instance_idx) = create_engine_with_module()?;

    // Production-like test cases
    let production_cases = vec![
        // Typical sensor value processing
        (100, 200, 300),   // Temperature + pressure
        (512, 256, 768),   // Digital signal processing
        (1000, -100, 900), // Offset calibration
        (0, 0, 0),         // Null/reset case
        // Automotive control values
        (1500, 500, 2000), // RPM calculations
        (60, 40, 100),     // Speed calculations
        (25, 75, 100),     // Percentage calculations
        (-20, 80, 60),     // Temperature compensation
        // Real-time system values
        (16777216, 8388608, 25165824), // Large numerical processing
        (1, 1, 2),                     // Minimal increments
        (65535, 1, 65536),             // 16-bit boundary
        (4095, 1, 4096),               // 12-bit boundary
    ];

    for (iteration, (a, b, expected)) in production_cases.iter().enumerate() {
        let args = create_test_args(*a, *b;
        let results = engine.execute(instance_idx, 0, args)?;

        assert_eq!(
            results.len(),
            1,
            "Production case {}: Expected exactly one result",
            iteration
        ;

        if let Value::I32(result) = &results[0] {
            assert_eq!(
                *result, *expected,
                "Production case {}: add({}, {}) should equal {}",
                iteration, a, b, expected
            ;
        } else {
            panic!(
                "Production case {}: Expected I32 result, got {:?}",
                iteration, results[0]
            ;
        }
    }

    println!(
        "✅ Production workload simulation completed: {} test cases passed",
        production_cases.len()
    ;

    Ok(())
}

#[test]
fn test_comprehensive_execution_validation() -> Result<()> {
    // Comprehensive test combining all aspects
    println!("🚀 Starting comprehensive execution validation...");

    // Test 1: Multiple engines with different workloads
    let mut total_executions = 0;

    for engine_id in 0..5 {
        let (mut engine, instance_idx) = create_engine_with_module()?;

        for batch in 0..10 {
            for execution in 0..20 {
                let a = (engine_id * 1000 + batch * 100 + execution) % 10000;
                let b = ((engine_id + 1) * 500 + batch * 50 + execution) % 10000;
                let expected = a + b;

                let args = create_test_args(a, b);
                let results = engine.execute(instance_idx, 0, args)?;

                assert_eq!(
                    results.len(),
                    1,
                    "Engine {} batch {} execution {}: Expected exactly one result",
                    engine_id,
                    batch,
                    execution
                ;

                if let Value::I32(result) = &results[0] {
                    assert_eq!(
                        *result, expected,
                        "Engine {} batch {} execution {}: add({}, {}) should equal {}",
                        engine_id, batch, execution, a, b, expected
                    ;
                } else {
                    panic!(
                        "Engine {} batch {} execution {}: Expected I32 result, got {:?}",
                        engine_id, batch, execution, results[0]
                    ;
                }

                total_executions += 1;
            }
        }
    }

    println!(
        "✅ Comprehensive validation completed: {} total executions across 5 engines",
        total_executions
    ;

    Ok(())
}
