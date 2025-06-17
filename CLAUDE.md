# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# WRT (WebAssembly Runtime) Project Guidelines

## Important Rules
- NEVER create hardcoded examples in the runtime code. Real implementations should parse or process actual external files.
- NEVER add dummy or simulated implementations except in dedicated test modules.
- Any example code MUST be in the `example/` directory or test files, not in the runtime implementation.
- The runtime should be able to execute real WebAssembly modules without special-casing specific files.
- Only use placeholders when absolutely necessary and clearly document them.

## Build Commands
- Build all: `just build` or `cargo build`
- Build specific crate: `cargo build -p wrt|wrtd|example`
- Clean: `just clean` or `cargo clean`
- Run tests: `just ci-test` or `cargo test`
- Run single test: `cargo test -p wrt -- test_name --nocapture`
- Format code: `just fmt` or `cargo fmt`
- Format check: `just fmt-check`
- Main CI checks: `just ci-main`
- Full CI suite: `just ci-full`
- Typecheck: `cargo check`

## Build Matrix Verification
- **Comprehensive verification**: `just verify-build-matrix`
  - Tests all ASIL-D, ASIL-C, ASIL-B, development, and server configurations
  - Performs architectural analysis to identify root causes of failures
  - Generates detailed reports on ASIL compliance issues
  - CRITICAL: Run this before any architectural changes or feature additions
  - Required for all PRs that modify core runtime or safety-critical components

When build failures occur, the script will:
1. Analyze the root cause (not just symptoms)
2. Identify if issues are architectural problems affecting ASIL levels
3. Generate reports with specific remediation steps
4. Classify failures by their impact on safety compliance

## Code Style Guidelines
- Use 4-space indentation
- Follow Rust naming conventions: snake_case for functions/variables, CamelCase for types
- Organize imports in the following order:
  1. Standard library imports (std, core, alloc)
  2. External crates/third-party dependencies
  3. Internal modules (crate:: imports)
  4. Each group should be separated by a blank line
- Always derive Debug, Clone, PartialEq for data structures
- Use thiserror for error definitions
- Handle no_std environments with appropriate macro imports
- Write tests for new functionality
- Fix type consistency:
  - WebAssembly spec uses u32 for sizes
  - Convert between u32 and usize explicitly when working with Rust memory
- Break cyclic references with Box<T> for recursive types
- Use conventional commits: `type(scope): short summary`
- Thoroughly document public API with doc comments

## ASIL Compliance Requirements
When working on safety-critical components (ASIL-D, ASIL-C, ASIL-B):
1. **Always run `just verify-build-matrix` before committing**
2. **No unsafe code** in safety-critical configurations
3. **Deterministic compilation** - all feature combinations must build consistently
4. **Memory budget compliance** - no dynamic allocation after initialization for ASIL-D
5. **Module boundaries** - clear separation between safety-critical and non-critical code
6. **Formal verification** - Kani proofs must pass for safety-critical paths

## Architectural Analysis
The build matrix verification performs deep architectural analysis:
- **Dependency cycles** that violate ASIL modular design
- **Feature flag interactions** that create compilation conflicts
- **Memory allocation patterns** incompatible with no_std requirements
- **Trait coherence issues** indicating poor abstraction boundaries
- **Import/visibility problems** breaking deterministic builds

If architectural issues are detected, they must be resolved before merging, as they directly impact ASIL compliance and safety certification.

## Memories
- can you build and test it
# important-instruction-reminders
Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.