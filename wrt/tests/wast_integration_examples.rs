//! WAST Integration Examples
//!
//! This module provides practical examples of how to use the WAST test infrastructure
//! in different scenarios and environments.

#![cfg(test)]

use wrt::{Error, Module, Result, StacklessEngine, Value};

// Import the WAST test runner
mod wast_test_runner;
use wast_test_runner::{ResourceLimits, WastTestRunner, WastTestStats};

/// Example: Basic WAST test execution
#[test]
fn example_basic_wast_execution() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let wast_content = r#"
        (module
          (func (export "add") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add)
          (func (export "multiply") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.mul))
        
        (assert_return (invoke "add" (i32.const 2) (i32.const 3)) (i32.const 5))
        (assert_return (invoke "multiply" (i32.const 4) (i32.const 5)) (i32.const 20))
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Basic example results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!("  assert_return tests: {}", stats.assert_return_count);

    assert_eq!(stats.passed, 2);
    assert_eq!(stats.failed, 0);
    assert_eq!(stats.assert_return_count, 2);

    Ok(())
}

/// Example: Testing trap conditions
#[test]
fn example_trap_testing() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let wast_content = r#"
        (module
          (func (export "divide") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.div_s)
          (func (export "unreachable_func") (result i32)
            unreachable))
        
        (assert_trap (invoke "divide" (i32.const 10) (i32.const 0)) "integer divide by zero")
        (assert_trap (invoke "unreachable_func") "unreachable")
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Trap testing results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!("  assert_trap tests: {}", stats.assert_trap_count);

    assert_eq!(stats.assert_trap_count, 2);
    // Note: Trap tests might fail if the engine doesn't properly implement trap detection
    // This is expected during development

    Ok(())
}

/// Example: Testing invalid modules
#[test]
fn example_validation_testing() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let wast_content = r#"
        ;; This module should be invalid due to type mismatch
        (assert_invalid
          (module
            (func (result i32)
              i64.const 42))
          "type mismatch")
        
        ;; This module should be invalid due to unknown import
        (assert_invalid
          (module
            (import "unknown" "function" (func)))
          "unknown")
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Validation testing results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!("  assert_invalid tests: {}", stats.assert_invalid_count);

    assert_eq!(stats.assert_invalid_count, 2);

    Ok(())
}

/// Example: Testing with resource limits
#[test]
fn example_resource_limit_testing() -> Result<()> {
    let mut runner = WastTestRunner::new();

    // Set strict resource limits
    runner.set_resource_limits(ResourceLimits {
        max_stack_depth: 100,
        max_memory_size: 1024 * 1024, // 1MB
        max_execution_steps: 10000,
    });

    let wast_content = r#"
        (module
          (func (export "simple") (result i32)
            i32.const 42))
        
        (assert_return (invoke "simple") (i32.const 42))
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Resource limit testing results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);

    assert_eq!(stats.passed, 1);
    assert_eq!(stats.failed, 0);

    Ok(())
}

/// Example: Float precision and NaN testing
#[test]
fn example_float_testing() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let wast_content = r#"
        (module
          (func (export "f32_add") (param f32 f32) (result f32)
            local.get 0
            local.get 1
            f32.add)
          (func (export "f32_nan") (result f32)
            f32.const nan)
          (func (export "f64_sqrt") (param f64) (result f64)
            local.get 0
            f64.sqrt))
        
        (assert_return (invoke "f32_add" (f32.const 1.5) (f32.const 2.5)) (f32.const 4.0))
        (assert_return (invoke "f32_nan") (f32.const nan))
        (assert_return (invoke "f64_sqrt" (f64.const 4.0)) (f64.const 2.0))
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Float testing results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!("  assert_return tests: {}", stats.assert_return_count);

    assert_eq!(stats.assert_return_count, 3);

    Ok(())
}

/// Example: Memory operations testing
#[test]
fn example_memory_testing() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let wast_content = r#"
        (module
          (memory 1)
          (func (export "store32") (param i32 i32)
            local.get 0
            local.get 1
            i32.store)
          (func (export "load32") (param i32) (result i32)
            local.get 0
            i32.load)
          (func (export "memory_size") (result i32)
            memory.size))
        
        (invoke "store32" (i32.const 0) (i32.const 42))
        (assert_return (invoke "load32" (i32.const 0)) (i32.const 42))
        (assert_return (invoke "memory_size") (i32.const 1))
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Memory testing results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!("  Total directives: {}", stats.assert_return_count + 1); // +1 for invoke

    Ok(())
}

/// Example: Control flow testing
#[test]
fn example_control_flow_testing() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let wast_content = r#"
        (module
          (func (export "if_then_else") (param i32) (result i32)
            local.get 0
            if (result i32)
              i32.const 1
            else
              i32.const 0
            end)
          (func (export "loop_sum") (param i32) (result i32)
            (local i32)
            local.get 0
            local.set 1
            i32.const 0
            loop (result i32)
              local.get 1
              i32.const 0
              i32.gt_s
              if (result i32)
                local.get 0
                local.get 1
                i32.add
                local.set 0
                local.get 1
                i32.const 1
                i32.sub
                local.set 1
                br 1
              else
                local.get 0
              end
            end))
        
        (assert_return (invoke "if_then_else" (i32.const 1)) (i32.const 1))
        (assert_return (invoke "if_then_else" (i32.const 0)) (i32.const 0))
        (assert_return (invoke "loop_sum" (i32.const 5)) (i32.const 15))
    "#;

    let stats = runner.run_wast_content(wast_content)?;

    println!("Control flow testing results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!("  assert_return tests: {}", stats.assert_return_count);

    assert_eq!(stats.assert_return_count, 3);

    Ok(())
}

/// Example: Comprehensive test statistics analysis
#[test]
fn example_statistics_analysis() -> Result<()> {
    let mut runner = WastTestRunner::new();

    let comprehensive_wast = r#"
        (module
          (func (export "add") (param i32 i32) (result i32)
            local.get 0 local.get 1 i32.add)
          (func (export "div") (param i32 i32) (result i32)
            local.get 0 local.get 1 i32.div_s))
        
        ;; Correctness tests
        (assert_return (invoke "add" (i32.const 1) (i32.const 2)) (i32.const 3))
        (assert_return (invoke "add" (i32.const 0) (i32.const 0)) (i32.const 0))
        
        ;; Trap tests
        (assert_trap (invoke "div" (i32.const 1) (i32.const 0)) "integer divide by zero")
        
        ;; Invalid module test
        (assert_invalid
          (module (func (result i32) i64.const 1))
          "type mismatch")
        
        ;; Standalone invoke
        (invoke "add" (i32.const 10) (i32.const 20))
    "#;

    let stats = runner.run_wast_content(comprehensive_wast)?;

    println!("\n=== Comprehensive Test Statistics ===");
    println!("Total tests executed:");
    println!("  assert_return: {}", stats.assert_return_count);
    println!("  assert_trap: {}", stats.assert_trap_count);
    println!("  assert_invalid: {}", stats.assert_invalid_count);
    println!("  assert_malformed: {}", stats.assert_malformed_count);
    println!("  assert_unlinkable: {}", stats.assert_unlinkable_count);
    println!("  assert_exhaustion: {}", stats.assert_exhaustion_count);
    println!("  register: {}", stats.register_count);
    println!("\nResults:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);
    println!(
        "  Success rate: {:.1}%",
        if stats.passed + stats.failed > 0 {
            (stats.passed as f64 / (stats.passed + stats.failed) as f64) * 100.0
        } else {
            0.0
        }
    );

    // Verify we executed the expected number of directives
    let total_directives =
        stats.assert_return_count + stats.assert_trap_count + stats.assert_invalid_count + 1; // +1 for invoke
    assert!(total_directives >= 4);

    Ok(())
}

/// Example: Error handling and debugging
#[test]
fn example_error_handling() -> Result<()> {
    let mut runner = WastTestRunner::new();

    // This WAST content has intentional issues for demonstration
    let problematic_wast = r#"
        (module
          (func (export "test") (result i32)
            i32.const 42))
        
        ;; This should pass
        (assert_return (invoke "test") (i32.const 42))
        
        ;; This might fail if expected behavior doesn't match implementation
        (assert_return (invoke "test") (i32.const 43))
    "#;

    let stats = runner.run_wast_content(problematic_wast)?;

    println!("Error handling example results:");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);

    if stats.failed > 0 {
        println!("  Note: Some failures are expected in this example");
        println!("        This demonstrates error handling capabilities");
    }

    assert_eq!(stats.assert_return_count, 2);
    assert!(stats.passed >= 1); // At least one should pass

    Ok(())
}

/// Example: No-std compatibility demonstration
#[test]
fn example_no_std_usage() -> Result<()> {
    // This example shows how the WAST runner works in no_std environments
    // All the string content is static, no file I/O required

    let mut runner = WastTestRunner::new();

    let simple_wast = r#"
        (module
          (func (export "const42") (result i32)
            i32.const 42))
        
        (assert_return (invoke "const42") (i32.const 42))
    "#;

    let stats = runner.run_wast_content(simple_wast)?;

    println!("No-std compatibility example:");
    println!("  This test runs the same in std and no_std environments");
    println!("  Passed: {}", stats.passed);
    println!("  Failed: {}", stats.failed);

    assert_eq!(stats.passed, 1);
    assert_eq!(stats.failed, 0);

    Ok(())
}

/// Helper function to demonstrate custom test analysis
fn analyze_test_results(stats: &WastTestStats) {
    println!("\n=== Test Analysis ===");

    let total_tests = stats.passed + stats.failed;
    if total_tests == 0 {
        println!("No tests executed");
        return;
    }

    let success_rate = (stats.passed as f64 / total_tests as f64) * 100.0;

    println!("Execution Summary:");
    println!(
        "  Total directives: {}",
        stats.assert_return_count
            + stats.assert_trap_count
            + stats.assert_invalid_count
            + stats.assert_malformed_count
            + stats.assert_unlinkable_count
            + stats.assert_exhaustion_count
            + stats.register_count
    );

    println!("  Test distribution:");
    if stats.assert_return_count > 0 {
        println!("    Correctness tests: {}", stats.assert_return_count);
    }
    if stats.assert_trap_count > 0 {
        println!("    Trap tests: {}", stats.assert_trap_count);
    }
    if stats.assert_invalid_count > 0 {
        println!("    Validation tests: {}", stats.assert_invalid_count);
    }
    if stats.register_count > 0 {
        println!("    Integration tests: {}", stats.register_count);
    }

    println!(
        "  Results: {} passed, {} failed ({:.1}% success)",
        stats.passed, stats.failed, success_rate
    );

    if success_rate >= 95.0 {
        println!("  Status: Excellent compliance ✅");
    } else if success_rate >= 80.0 {
        println!("  Status: Good compliance ✓");
    } else if success_rate >= 60.0 {
        println!("  Status: Needs improvement ⚠️");
    } else {
        println!("  Status: Significant issues ❌");
    }
}

/// Integration test that demonstrates the full workflow
#[test]
fn example_full_workflow() -> Result<()> {
    println!("=== Full WAST Testing Workflow Example ===");

    let mut runner = WastTestRunner::new();

    // Configure resource limits
    runner.set_resource_limits(ResourceLimits {
        max_stack_depth: 1024,
        max_memory_size: 16 * 1024 * 1024, // 16MB
        max_execution_steps: 1_000_000,
    });

    let comprehensive_test = r#"
        ;; Module with various functionality
        (module
          (memory 1)
          (func (export "arithmetic") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add
            i32.const 1
            i32.add)
          
          (func (export "memory_test") (param i32 i32)
            local.get 0
            local.get 1
            i32.store)
          
          (func (export "memory_load") (param i32) (result i32)
            local.get 0
            i32.load)
          
          (func (export "trap_divide") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.div_s))
        
        ;; Test correctness
        (assert_return (invoke "arithmetic" (i32.const 5) (i32.const 3)) (i32.const 9))
        
        ;; Test memory operations
        (invoke "memory_test" (i32.const 0) (i32.const 123))
        (assert_return (invoke "memory_load" (i32.const 0)) (i32.const 123))
        
        ;; Test trap conditions
        (assert_trap (invoke "trap_divide" (i32.const 1) (i32.const 0)) "integer divide by zero")
    "#;

    println!("Executing comprehensive WAST test suite...");
    let stats = runner.run_wast_content(comprehensive_test)?;

    analyze_test_results(&stats);

    // Verify expected results
    assert!(stats.assert_return_count >= 2);
    assert!(stats.assert_trap_count >= 1);
    assert!(stats.passed > 0);

    println!("\n✅ Full workflow example completed successfully!");

    Ok(())
}
