# ASIL Safety Features in wrt-error

This document describes the ASIL (Automotive Safety Integrity Level) features implemented in the wrt-error crate for functional safety compliance according to ISO 26262.

## Overview

The wrt-error crate provides progressive safety features based on the configured ASIL level:

- **QM (Quality Management)**: Basic error handling
- **ASIL-A**: Basic safety features
- **ASIL-B**: Enhanced error categorization and tracking (≥90% SPFM)
- **ASIL-C**: Safety monitoring and immediate safe state requirements (≥97% SPFM)
- **ASIL-D**: Full integrity checking and redundancy validation (≥99% SPFM)

## Feature Gates

Enable ASIL features in your `Cargo.toml`:

```toml
[dependencies]
wrt-error = { version = "0.2", features = ["asil-b"] }  # or asil-c, asil-d
```

## ASIL-Specific Error Features

### Error Categorization (All Levels)

Errors are categorized by safety criticality:

```rust
pub enum ErrorCategory {
    Core = 1,          // Core WebAssembly errors
    Component = 2,     // Component model errors  
    Resource = 3,      // Resource errors
    Memory = 4,        // Memory errors (ASIL-C)
    Validation = 5,    // Validation errors (ASIL-B)
    Type = 6,          // Type errors (ASIL-B)
    Runtime = 7,       // Runtime errors
    System = 8,        // System errors
    Unknown = 9,       // Unknown errors
    Parse = 10,        // Parse errors
    Concurrency = 11,  // Concurrency errors
    Capacity = 12,     // Capacity errors
    RuntimeTrap = 13,  // WebAssembly trap errors (ASIL-C)
    Initialization = 14, // Initialization errors
    NotSupported = 15, // Not supported operation errors
    Safety = 16,       // Safety-related errors (ASIL-D)
}
```

### ASIL-B Features (≥90% SPFM)

When compiled with `asil-b` feature:

1. **ASIL Level Detection**: Each error knows its safety criticality
   ```rust
   let error = Error::new(ErrorCategory::Memory, code, "Memory error");
   assert_eq!(error.asil_level(), "ASIL-C"); // Memory errors are ASIL-C
   ```

2. **Error Context**: Rich error context with metadata
   ```rust
   let context = AsilErrorContext::new(error)
       .with_timestamp(12345)
       .with_module_id(42);
   ```

3. **ASIL-specific error helpers**:
   ```rust
   let error = asil_violation_error("ASIL-B", "Safety check failed");
   ```

### ASIL-C Features (≥97% SPFM)

When compiled with `asil-c` feature (includes all ASIL-B features):

1. **Safe State Requirements**: Errors that require immediate safe state transition
   ```rust
   if error.requires_safe_state() {
       // Transition to safe state immediately
   }
   ```

2. **Safety Monitoring**: Track error occurrences
   ```rust
   let monitor = SafetyMonitor::new();
   monitor.record_error(&error);
   println!("Total errors: {}", monitor.error_count());
   ```

3. **Enhanced Error Display**: Includes ASIL level in error format
   ```
   [Memory][E0FA0][ASIL-C] Out of bounds memory access
   ```

### ASIL-D Features (≥99% SPFM)

When compiled with `asil-d` feature (includes all ASIL-C features):

1. **Error Integrity Validation**: Validate error consistency
   ```rust
   if error.validate_integrity() {
       // Error is well-formed
   }
   ```

2. **Determinism Checks**: Special errors for non-deterministic behavior
   ```rust
   let error = determinism_violation_error("Non-deterministic behavior detected");
   ```

3. **Redundancy Validation**: Errors for redundancy check failures
   ```rust
   let error = redundancy_check_failure_error("Redundancy mismatch");
   ```

4. **Error Storm Detection**: Automatic detection of systematic failures
   ```rust
   // SafetyMonitor automatically detects error storms in ASIL-D mode
   ```

## Macros for ASIL-Aware Programming

### asil_error! Macro

Create errors with compile-time ASIL validation:

```rust
// This only compiles with ASIL-D enabled
let error = asil_error!(
    ErrorCategory::Safety,
    codes::SAFETY_VIOLATION,
    "Critical error",
    "asil-d"
);
```

### asil_assert! Macro

ASIL-aware assertions with level-appropriate behavior:

```rust
// QM/ASIL-A: Debug assertion only
// ASIL-B: Returns error
// ASIL-C/D: Panics on failure
asil_assert!(
    index < max,
    ErrorCategory::Validation,
    codes::OUT_OF_BOUNDS,
    "Index out of bounds"
)?;
```

### safety_error! Macro

Create safety-critical errors (requires ASIL-C or higher):

```rust
// Only compiles with ASIL-C or ASIL-D
let error = safety_error!(
    codes::MEMORY_CORRUPTION_DETECTED,
    "Critical memory corruption"
);
```

## Best Practices

1. **Choose the Right ASIL Level**: 
   - Use ASIL-B for basic safety requirements
   - Use ASIL-C for safety-critical systems
   - Use ASIL-D for highest safety integrity

2. **Error Categorization**: Always use the most specific error category

3. **Safe State Transition**: For ASIL-C and above, always check `requires_safe_state()`

4. **Monitor Errors**: Use SafetyMonitor in ASIL-C/D systems to track error patterns

5. **Validate Integrity**: In ASIL-D systems, validate error integrity before processing

## Example Usage

See `examples/asil_demo.rs` for a complete demonstration of ASIL features.

## Compliance Notes

- **SPFM**: Single-Point Fault Metric
- **ISO 26262**: International standard for functional safety in automotive systems
- Each ASIL level provides progressively stronger safety guarantees
- Higher ASIL levels include all features from lower levels