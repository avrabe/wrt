# WebAssembly Proposal Testing Guide

This guide explains how to use the proposal testing framework in the WRT project.

## Overview

The WebAssembly specification is continuously evolving with new proposals that add features to the language. Testing these proposals is important for ensuring compatibility and correctness of our implementation.

WRT supports testing proposal features with both `std` and `no_std` environments, allowing you to verify compatibility across different target platforms.

## Available Proposal Features

The following proposal features are available for testing:

- `relaxed_simd`: Tests for relaxed SIMD operations
- `gc`: Garbage collection proposal
- `function_references`: Function references proposal
- `multi_memory`: Multi-memory proposal
- `exception_handling`: Exception handling proposal
- `threads`: Threads proposal
- `extended_const`: Extended const expressions proposal
- `tail_call`: Tail call optimization proposal
- `wasm_3_0`: WebAssembly 3.0 proposals
- `wide_arithmetic`: Wide arithmetic proposal
- `custom_page_sizes`: Custom page sizes proposal
- `annotations`: Annotations proposal

## Running Proposal Tests

### Enable a Specific Proposal

To run tests for a specific proposal, enable the feature flag when running the tests:

```sh
# Run tests with the relaxed_simd feature enabled
cargo test --features relaxed_simd

# Run tests with multiple proposal features
cargo test --features "relaxed_simd gc threads"
```

### Testing with No STD

To test in a no_std environment:

```sh
# Run with no_std and a proposal feature
cargo test --no-default-features --features "no_std relaxed_simd"
```

### Running All Proposal Tests

To run all available proposal tests:

```sh
# Run all proposal tests
cargo test --features "relaxed_simd gc function_references multi_memory exception_handling threads extended_const tail_call wasm_3_0 wide_arithmetic custom_page_sizes annotations"
```

## Implementation Details

### Test Structure

The proposal tests are organized in two files:

1. `wast_tests.rs`: Contains basic WAST parsing tests for all proposals
2. `proposal_tests.rs`: Contains more detailed test utilities for working with proposal tests

### Adding Tests for a New Proposal

To add tests for a new proposal:

1. Add a new feature flag in `Cargo.toml`:

```toml
[features]
# ... existing features
new_proposal = []
```

2. Add a new test function in `wast_tests.rs`:

```rust
/// Tests for the new proposal
/// These tests are only run when the "new_proposal" feature is enabled
#[cfg(feature = "new_proposal")]
#[generate_directory_tests("proposals/new-proposal", "new_proposal")]
fn run_new_proposal_tests(file_name: &str, _test_name: &str) {
    println!("==========================================");
    println!("Processing new proposal file: {}", file_name);
    println!("✅ Successfully parsed {}", file_name);
    println!("==========================================");
}
```

3. Add detailed tests in `proposal_tests.rs` if needed:

```rust
#[cfg(feature = "new_proposal")]
#[generate_directory_tests("proposals/new-proposal", "new_proposal_runner")]
fn run_new_proposal_tests(file_name: &str, _test_name: &str) {
    if let Err(e) = execute_proposal_test(file_name, "new-proposal") {
        panic!("Test failed: {}", e);
    }
}
```

## Best Practices

1. Always conditionalize proposal-specific code using `#[cfg(feature = "feature_name")]`
2. Ensure tests work in both `std` and `no_std` environments
3. Add helpful debug output to make test results easier to understand
4. When implementing a proposal feature, add corresponding tests in the test suite

## Troubleshooting

If you encounter issues with the proposal tests:

- Ensure the WASM_TESTSUITE environment variable is set correctly
- Verify the proposal directory exists in the testsuite
- Check that you've enabled the correct feature flags
- Look for compilation errors that might indicate incompatibility between features 