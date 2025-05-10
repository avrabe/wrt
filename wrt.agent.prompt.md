# WRT Reorganization Agent Prompt

You are an AI agent tasked with implementing the WRT Reorganization Plan as outlined in the `wrt.plan.md` file.

## Your Task

You will be implementing the reorganization plan for the WebAssembly Runtime (WRT) crates, ensuring they are buildable with both std and no_std configurations with no warnings or errors. This includes addressing duplication of code, improving organization structure, and ensuring proper feature flag usage across all crates.

The WRT ecosystem includes the following crates:
- `wrt-error`: Error handling
- `wrt-types`: Core and runtime types
- `wrt-format`: Binary format specifications
- `wrt-decoder`: Binary parsing and decoding
- `wrt-instructions`: Instruction encoding/decoding
- `wrt-component`: Component model implementation
- `wrt-host`: Host functions and interfaces
- `wrt-intercept`: Function interception
- `wrt-sync`: Synchronization primitives
- `wrt-runtime`: Execution engine
- `wrt-test-registry`: Unified testing framework
- `wrt`: Main integration library

## Process to Follow

1. **Read and understand the `wrt.plan.md` file** to get a complete understanding of the plan and desired outcomes.

2. **Work through each phase sequentially**, implementing one phase at a time:
   - Phase 1: Fix Core Dependencies
   - Phase 2: Standardize Error Handling
   - Phase 3: Resolve Type System Inconsistencies 
   - Phase 4: Implement No_Std Support
   - Phase 5: Fix Documentation and Lints
   - Phase 6: Final Integration and Testing

3. **After implementing each phase**:
   - Build with std features: `cargo build --features std`
   - Build with no_std features: `cargo build --no-default-features --features no_std,alloc`
   - Run tests where applicable: `cargo test --features std`
   - Check for linting issues: `cargo clippy -- -D warnings`
   - Ensure documentation builds: `cargo doc --no-deps`

4. **Report on completion of each phase**:
   - Summarize the changes you made
   - Confirm that all validation criteria are met for the specific phase
   - Identify any issues encountered and how they were resolved

5. **After completing all phases**:
   - Perform a final comprehensive validation
   - Create a summary report of all changes made
   - Confirm all success metrics are met

## Validation Requirements

The validation criteria will be progressively more stringent as we move through the phases:

1. **Early Phases**:
   - Focus on reducing errors
   - Fix critical issues that prevent compilation
   - Document known issues for later phases

2. **Middle Phases**:
   - Zero build errors for both std and no_std
   - Working tests for core crates
   - Minimal warnings

3. **Final Phases**:
   - No build errors or warnings with std and no_std features
   - All tests pass successfully using both individual crate tests and the wrt-test-registry
   - No clippy warnings
   - Documentation builds without warnings

## Final Deliverable

A comprehensive report including:
1. Summary of all changes made
2. Confirmation that all validation criteria are met
3. Any issues encountered and how they were resolved
4. Recommendations for future improvements

Once you've completed each phase and validated that it meets the criteria for that phase, mark it as complete and move on to the next phase. When all phases are successfully implemented and validated, you can declare the reorganization plan complete. 


-----
fix each create based on the implementation sequence and the success metrics:

## Implementation Sequence

The implementation will proceed in order of dependency, starting from the most fundamental crates:

1. `wrt-error`: Error handling foundation: done.
2. `wrt-types`: Core and runtime type definitions: done
3. `wrt-format`: Format specifications: done
4. `wrt-decoder`: Binary parsing. Should be the only one using wrt-format. The resulting api shall then use the wrt-types.
6. `wrt-sync`: Synchronization primitives: done
5. `wrt-instructions`: Instruction encoding/decoding: done
7. `wrt-intercept`: Function interception: done
8. `wrt-host`: Host interface: done
9. `wrt-component`: Component model: done
10. `wrt-runtime`: Runtime execution: done
12. `wrt`: Main library integration: to be finished before to go to the next crate.

11. `wrt-test-registry`: Test framework

This ensures that fixes at the foundation level propagate properly through the dependency chain.

## Success Metrics

The reorganization will be considered successful if:

0. Everything builds and can be tested.
0. Each crate does not implement things from the other crates. 
Crates get reimported into a prelude.rs
All crates build successfully with both std and no_std features
All tests pass. Run cargo test for std and non_std
No clippy warnings are present: run cargo clippy
Documentation is complete and builds without warnings
Code duplication is eliminated
Type system is consistent across all crates
Error handling is standardized 
Ensure to use SafeMemory and other types before Vec
No shortcut. Fixing errors and implementing the actual features is mandatory.
Replace example code or stubs with real implementation. Ask if more information is needed.
Everything is license: MIT
All runtime crates should use wrt-decoder if reading from a warm module is needed or a serialisation into it. Only wrt-decoder shall use wrt-format.