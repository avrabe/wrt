# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# WRT (WebAssembly Runtime) Project Guidelines

## Quick Setup

```bash
# Install the unified build tool
cargo install --path cargo-wrt

# Verify installation
cargo-wrt --help
```

## Important Rules
- NEVER create hardcoded examples in the runtime code. Real implementations should parse or process actual external files.
- NEVER add dummy or simulated implementations except in dedicated test modules.
- Any example code MUST be in the `example/` directory or test files, not in the runtime implementation.
- The runtime should be able to execute real WebAssembly modules without special-casing specific files.
- Only use placeholders when absolutely necessary and clearly document them.

## Memory System Architecture

### Unified Capability-Based Memory System
The WRT project uses a **single, unified memory management system** based on capabilities:

- **Primary API**: `safe_managed_alloc!(size, crate_id)` - All memory allocation goes through this macro
- **Provider Construction**: `NoStdProvider::default()` - Safe default construction only
- **Factory System**: `CapabilityWrtFactory` - Capability-based factory for advanced use cases
- **Automatic Cleanup**: RAII-based automatic memory management

### Memory Allocation Pattern
```rust
use wrt_foundation::{safe_managed_alloc, CrateId};

// Standard allocation pattern
let provider = safe_managed_alloc!(4096, CrateId::Component)?;
let mut vec = BoundedVec::new(provider)?;

// Memory is automatically cleaned up when provider goes out of scope
```

### Important Memory Guidelines
- **NO legacy patterns**: The codebase has been completely cleaned of all legacy memory patterns
- **NO direct construction**: Never use `NoStdProvider::new()` - always use `::default()`
- **NO unsafe extraction**: There is no `unsafe { guard.release() }` pattern anymore
- **NO dual systems**: Only one memory system exists - capability-based allocation

## Build Commands & Diagnostic System

The WRT project uses a unified build system with `cargo-wrt` as the single entry point for all build operations. All legacy shell scripts have been migrated to this unified system.

**Usage Patterns:**
- Direct: `cargo-wrt <COMMAND>`
- Cargo subcommand: `cargo wrt <COMMAND>`

Both patterns work identically and use the same binary.

### Advanced Diagnostic System

The build system includes a comprehensive diagnostic system with LSP-compatible structured output, caching, filtering, grouping, and differential analysis. This system works across all major commands: `build`, `test`, `verify`, and `check`.

**Key Features:**
- **Structured Output**: LSP-compatible JSON for tooling/AI integration
- **Caching**: File-based caching with change detection for faster incremental builds
- **Filtering**: By severity, source tool, file patterns
- **Grouping**: Organize diagnostics by file, severity, source, or error code
- **Diff Mode**: Show only new/changed diagnostics since last cached run

**Global Diagnostic Flags (work with build, test, verify, check):**
```bash
# Output format control
--output human|json|json-lines    # Default: human

# Caching control
--cache                           # Enable diagnostic caching
--clear-cache                     # Clear cache before running
--diff-only                       # Show only new/changed diagnostics

# Filtering options
--filter-severity LIST            # error,warning,info,hint
--filter-source LIST              # rustc,clippy,miri,cargo
--filter-file PATTERNS            # *.rs,src/* (glob patterns)

# Grouping and pagination
--group-by CRITERION              # file|severity|source|code
--limit NUMBER                    # Limit diagnostic output count
```

### Core Commands
- Build all: `cargo-wrt build` or `cargo build`
- Build specific crate: `cargo-wrt build --package wrt|wrtd|example`
- Clean: `cargo-wrt clean` or `cargo clean`
- Run tests: `cargo-wrt test` or `cargo test`
- Run single test: `cargo test -p wrt -- test_name --nocapture`
- Format code: `cargo-wrt check` or `cargo fmt`
- Format check: `cargo-wrt check --strict`
- Main CI checks: `cargo-wrt ci`
- Full CI suite: `cargo-wrt verify --asil d`
- Typecheck: `cargo check`

### Diagnostic Usage Examples

**Basic Error Analysis:**
```bash
# Get all errors in JSON format
cargo-wrt build --output json --filter-severity error

# Focus on specific file patterns
cargo-wrt build --output json --filter-file "wrt-foundation/*"

# Get clippy suggestions only
cargo-wrt check --output json --filter-source clippy
```

**Incremental Development Workflow:**
```bash
# Initial baseline (clears cache and establishes new baseline)
cargo-wrt build --output json --cache --clear-cache

# Subsequent runs - see only new issues
cargo-wrt build --output json --cache --diff-only

# Focus on errors only in diff mode
cargo-wrt build --output json --cache --diff-only --filter-severity error
```

**Code Quality Analysis:**
```bash
# Group warnings by file for focused fixes
cargo-wrt check --output json --filter-severity warning --group-by file

# Limit output for manageable chunks
cargo-wrt check --output json --limit 10 --filter-severity warning

# Multiple tool analysis
cargo-wrt verify --output json --filter-source "rustc,clippy,miri"
```

**CI/CD Integration:**
```bash
# Fail-fast on any errors
cargo-wrt build --output json | jq '.summary.errors > 0' && exit 1

# Stream processing for large outputs
cargo-wrt build --output json-lines | while read diagnostic; do
  echo "$diagnostic" | jq '.severity'
done

# Generate reports for specific tools
cargo-wrt verify --output json --filter-source kani,miri
```

**JSON Processing with jq:**
```bash
# Extract error messages
cargo-wrt build --output json | jq '.diagnostics[] | select(.severity == "error") | .message'

# Count diagnostics by file
cargo-wrt build --output json | jq '.diagnostics | group_by(.file) | map({file: .[0].file, count: length})'

# Get files with errors
cargo-wrt build --output json | jq '.diagnostics[] | select(.severity == "error") | .file' | sort -u
```

### Advanced Commands
- Setup and tool management: `cargo-wrt setup --check` or `cargo-wrt setup --all`
- Fuzzing: `cargo-wrt fuzz --list` to see targets, `cargo-wrt fuzz` to run all
- Verification: `cargo-wrt validate --all` for comprehensive validation
- Platform verification: `cargo-wrt verify --asil <level>` with ASIL compliance
- Requirements traceability: automatically checked during verification
- No-std validation: `cargo-wrt no-std` 
- KANI formal verification: `cargo-wrt kani-verify --asil-profile <level>`

### Tool Management
The build system includes sophisticated tool version management with configurable requirements:

- Check tool status: `cargo-wrt setup --check`
- Install optional tools: `cargo-wrt setup --install` 
- Complete setup: `cargo-wrt setup --all`

**Tool Version Management:**
- Check all tool versions: `cargo-wrt tool-versions check --verbose`
- Generate tool configuration: `cargo-wrt tool-versions generate`
- Check specific tool: `cargo-wrt tool-versions check --tool kani`

Tool versions are managed via `tool-versions.toml` in the workspace root, specifying exact/minimum version requirements and installation commands. This ensures reproducible builds and consistent development environments.

## Build Matrix Verification
- **Comprehensive verification**: `cargo-wrt verify-matrix --report`
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

### General Formatting
- Use 4-space indentation
- Follow Rust naming conventions: snake_case for functions/variables, CamelCase for types
- Constants and statics use SCREAMING_SNAKE_CASE
- Use conventional commits: `type(scope): short summary`

### Import Organization
Organize imports in the following order:
1. Module attributes (`#![no_std]`, etc.)
2. `extern crate` declarations
3. Standard library imports (std, core, alloc) - grouped by feature flags
4. External crates/third-party dependencies
5. Internal crates (wrt_* imports)
6. Module imports (crate:: imports)
7. Each group should be separated by a blank line

Example:
```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as HashMap;

use thiserror::Error;

use wrt_foundation::prelude::*;

use crate::types::Value;
```

### Type Definitions
- Always derive Debug, Clone, PartialEq, Eq for data structures
- Add Hash, Ord when semantically appropriate
- Document why if any standard derives are omitted
- Use thiserror for error definitions

### Documentation Standards
- All modules MUST have `//!` module-level documentation
- All public items MUST have `///` documentation
- Use `//` for implementation comments
- Include `# Safety` sections for unsafe code
- Examples in docs should be tested (use `no_run` if needed)

### Error Handling
- NO `.unwrap()` in production code except:
  - Constants/static initialization
  - Documented infallible operations (with safety comment)
- Define crate-specific error types using thiserror
- Use `Result<T, CrateError>` consistently

### Testing Standards
- Unit tests: Use `#[cfg(test)] mod tests {}` in source files
- Integration tests: Place in `tests/` directory only
- No test files in `src/` directory (except `#[cfg(test)]` modules)
- Test naming: `test_<functionality>` or `<functionality>_<condition>`

### Feature Flags
- Use simple, clear feature flag patterns
- Group feature-gated imports together
- Avoid redundant feature checks

### Type Consistency
- WebAssembly spec uses u32 for sizes
- Convert between u32 and usize explicitly when working with Rust memory
- Break cyclic references with Box<T> for recursive types
- Handle no_std environments with appropriate macro imports

## ASIL Compliance Requirements
When working on safety-critical components (ASIL-D, ASIL-C, ASIL-B):
1. **Always run `cargo-wrt verify-matrix --report` before committing**
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

## Architecture Notes

### Memory System Migration (Completed)
The WRT project has successfully migrated to a unified capability-based memory system:

- **Unified allocation**: All memory goes through `safe_managed_alloc!` macro
- **Capability verification**: Every allocation is capability-checked
- **Budget enforcement**: Automatic per-crate budget tracking
- **RAII cleanup**: Automatic memory management via Drop
- **Zero legacy patterns**: All old patterns have been eliminated

### Build System Migration (Completed)
The WRT project has completed its migration to a unified build system:

- **cargo-wrt**: Single CLI entry point for all build operations
- **wrt-build-core**: Core library containing all build logic and functionality
- **Legacy cleanup**: All shell scripts and fragmented build tools have been removed
- **Integration**: Former wrt-verification-tool functionality integrated into wrt-build-core
- **API consistency**: All commands follow consistent patterns and error handling

### Removed Legacy Components
- Shell scripts: `verify_build.sh`, `fuzz_all.sh`, `verify_no_std.sh`, `test_features.sh`, `documentation_audit.sh`
- Kani verification scripts: `test_kani_phase4.sh`, `validate_kani_phase4.sh`
- justfile and xtask references (functionality ported to wrt-build-core)
- Legacy memory patterns: `WRT_MEMORY_COORDINATOR`, `WrtProviderFactory`, `unsafe { guard.release() }`

## Current System Status

### Memory Architecture
- **Single System**: 100% capability-based memory management
- **Consistent API**: `safe_managed_alloc!()` throughout
- **Safe Construction**: `NoStdProvider::default()` only
- **Modern Factory**: `CapabilityWrtFactory` for advanced use
- **Automatic Cleanup**: RAII-based memory management

### Build System
- **Unified Tool**: `cargo-wrt` for all operations
- **Consistent Commands**: Same patterns across all functionality
- **Tool Management**: Automated version checking and installation
- **ASIL Verification**: Built-in safety compliance checking

### JSON Diagnostic Format Specification

The JSON output follows LSP (Language Server Protocol) specification for maximum compatibility:

```json
{
  "version": "1.0",
  "timestamp": "2025-06-21T11:39:57.067142+00:00",
  "workspace_root": "/Users/r/git/wrt2",
  "command": "build",
  "diagnostics": [
    {
      "file": "wrt-foundation/src/capabilities/factory.rs",
      "range": {
        "start": {"line": 17, "character": 29},
        "end": {"line": 17, "character": 53}
      },
      "severity": "warning",
      "code": "deprecated",
      "message": "use of deprecated struct `CapabilityFactoryBuilder`",
      "source": "rustc",
      "related_info": [
        {
          "file": "wrt-foundation/src/lib.rs",
          "range": {
            "start": {"line": 29, "character": 4},
            "end": {"line": 29, "character": 16}
          },
          "message": "the lint level is defined here"
        }
      ]
    }
  ],
  "summary": {
    "total": 221,
    "errors": 7,
    "warnings": 214,
    "infos": 0,
    "hints": 0,
    "files_with_diagnostics": 29,
    "duration_ms": 1486
  }
}
```

**Key Field Explanations:**
- `file`: Relative path from workspace root
- `range`: LSP-compatible position (0-indexed line/character)
- `severity`: "error"|"warning"|"info"|"hint"
- `code`: Tool-specific error/warning code (optional)
- `source`: Tool that generated diagnostic ("rustc", "clippy", "miri", etc.)
- `related_info`: Array of related diagnostic locations

**Performance Benefits:**
- Initial run: Full analysis (3-4 seconds)
- Cached run: Incremental analysis (~0.7 seconds)
- Diff-only: Shows only changed diagnostics (filtering ~5-10% of output)

## Memories
- Build and test with `cargo-wrt` commands
- Memory allocation uses `safe_managed_alloc!` macro only
- Legacy patterns have been completely eliminated
- All builds must pass ASIL verification before merging
- Legacy memory system migration completed - single unified capability-based allocation system
- All NoStdProvider::new() calls replaced with ::default()
- All legacy patterns removed from code, comments, and documentation
- **Diagnostic system**: Use `--output json` for structured output, `--cache --diff-only` for incremental analysis
- **AI Integration**: JSON output is LSP-compatible for IDE and tooling integration
- **Performance**: Caching reduces analysis time from 3-4s to ~0.7s on subsequent runs
# important-instruction-reminders
Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
# important-instruction-reminders
Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.