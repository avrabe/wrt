# WRT Formal Verification with KANI

This directory contains the formal verification infrastructure for the WebAssembly Runtime (WRT) project using the KANI Rust Verifier.

## Overview

The formal verification suite is organized into multiple phases, each targeting specific safety properties:

### Phase 1: Infrastructure Setup
- TestRegistry integration for dual-mode operation
- Utility functions for bounded verification
- KANI harness framework

### Phase 2: Memory Safety
- **Module**: `memory_safety_proofs.rs`
- **Properties**: 6 formal properties
- Budget enforcement, hierarchical consistency, cross-crate isolation
- Memory deallocation patterns and fragmentation bounds

### Phase 3: Safety Invariants
- **Module**: `safety_invariants_proofs.rs`
- **Properties**: 4 formal properties
- ASIL level monotonicity and preservation
- Cross-standard safety conversions
- Violation count tracking

### Phase 4: Advanced Verification
- **Concurrency** (`concurrency_proofs.rs`): 6 properties
  - Atomic operations, synchronization primitives
  - Memory ordering, deadlock prevention
- **Resource Lifecycle** (`resource_lifecycle_proofs.rs`): 6 properties
  - ID uniqueness, lifecycle correctness
  - Cross-component isolation, reference validity
- **Integration** (`integration_proofs.rs`): 7 properties
  - Cross-component memory isolation
  - Interface type safety, system-wide limits
  - End-to-end safety preservation

## Running Formal Verification

### Prerequisites

1. Install KANI:
   ```bash
   cargo install --locked kani-verifier
   cargo kani setup
   ```

2. Ensure Rust toolchain is configured:
   ```bash
   rustup toolchain install nightly-2024-01-01
   rustup component add rust-src --toolchain nightly-2024-01-01
   ```

### Basic Usage

Run all formal verification tests:
```bash
cargo kani -p wrt-integration-tests --features kani
```

Run a specific harness:
```bash
cargo kani -p wrt-integration-tests --harness kani_verify_memory_budget_never_exceeded
```

### Using the Verification Script

The project includes a comprehensive verification script:

```bash
# Run all verifications with ASIL-C profile (default)
./scripts/kani-verify.sh

# Run with ASIL-D profile for maximum verification
./scripts/kani-verify.sh --profile asil-d

# Verify specific package
./scripts/kani-verify.sh --package wrt-integration-tests

# Run specific harness with verbose output
./scripts/kani-verify.sh --harness kani_verify_atomic_compare_and_swap --verbose
```

### ASIL Profiles

The verification suite supports different ASIL (Automotive Safety Integrity Level) profiles:

- **ASIL-A**: Basic verification (unwind=3, minisat solver)
- **ASIL-B**: Enhanced verification (unwind=4, cadical solver)
- **ASIL-C**: Comprehensive verification (unwind=5, cadical solver) - Default
- **ASIL-D**: Maximum verification (unwind=7, cadical solver, coverage enabled)

## Verification Properties

### Total Formal Properties: 29

1. **Memory Safety**: 6 properties
2. **Safety Invariants**: 4 properties  
3. **Concurrency**: 6 properties
4. **Resource Lifecycle**: 6 properties
5. **Integration**: 7 properties

## Dual-Mode Operation

All verification modules support dual-mode operation:

1. **KANI Mode**: Full formal verification when `cargo kani` is used
2. **Test Mode**: Traditional unit tests when `cargo test` is used

This ensures continuous integration compatibility while providing formal guarantees.

## Configuration

### Workspace Configuration

The workspace `Cargo.toml` defines all KANI harnesses:
```toml
[[workspace.metadata.kani.package]]
name = "wrt-integration-tests"
verification-enabled = true
harnesses = [
    # ... list of all harnesses
]
```

### Package Configuration

The `Kani.toml` file in this directory provides:
- Solver configuration (CaDiCal for complex proofs)
- Memory model settings
- ASIL profile definitions
- Harness-specific unwind limits

## Interpreting Results

### Success Output
```
VERIFICATION:- CHECK ID: <harness_name>
VERIFICATION:- DESCRIPTION: "<property description>"
VERIFICATION:- STATUS: SUCCESS
```

### Failure Output
```
VERIFICATION:- CHECK ID: <harness_name>
VERIFICATION:- DESCRIPTION: "<property description>"  
VERIFICATION:- STATUS: FAILURE
VERIFICATION:- COUNTEREXAMPLE: <path to concrete values>
```

### Coverage Reports

For ASIL-D verification, coverage reports show:
- Line coverage of verified code
- Branch coverage statistics
- Unreachable code identification

## Best Practices

1. **Bounded Verification**: Use reasonable bounds for loops and data structures
2. **Assumptions**: Document all `kani::assume()` statements clearly
3. **Property Granularity**: Keep properties focused and testable
4. **Performance**: Use appropriate unwind limits to balance thoroughness and speed

## Troubleshooting

### Common Issues

1. **Out of Memory**: Reduce unwind limit or simplify property
2. **Timeout**: Use a faster solver or parallelize verification
3. **Spurious Failures**: Check assumptions and preconditions

### Debug Commands

```bash
# Generate concrete playback for failed proof
cargo kani --harness <harness_name> --concrete-playback inplace

# Enable debug output
cargo kani --harness <harness_name> --verbose --debug

# Check which properties are being verified
grep -r "#\[kani::proof\]" . | wc -l
```

## Future Work

- Advanced lock-step execution verification
- Redundant computation detection
- Hardware error injection testing
- Formal specification generation

## References

- [KANI Documentation](https://model-checking.github.io/kani/)
- [ISO 26262 Standard](https://www.iso.org/standard/68383.html)
- [MISRA C Guidelines](https://www.misra.org.uk/)