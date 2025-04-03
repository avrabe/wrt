# WRT - WebAssembly Runtime

[![Rust CI](https://github.com/avrabe/wrt/actions/workflows/ci.yml/badge.svg)](https://github.com/avrabe/wrt/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/avrabe/wrt/graph/badge.svg?token=angh0LQdpK)](https://codecov.io/gh/avrabe/wrt)

WRT is a pure Rust implementation of a WebAssembly runtime that supports both the core WebAssembly specification and the WebAssembly Component Model. The project provides both a library (`wrt`) for embedding the runtime in Rust applications and a standalone daemon (`wrtd`) for executing WebAssembly modules.

## Features

- **Core WebAssembly Support**: Implements the WebAssembly 1.0 specification
- **Component Model**: Supports the WebAssembly Component Model for language-agnostic interoperability
- **`no_std` Compatible**: Can be used in environments without the standard library
- **Memory Safety**: Provides safe memory management for WebAssembly modules
- **Extensible**: Easily extendable architecture for adding new features

## Project Structure

- `wrt/` - Core WebAssembly runtime library
- `wrtd/` - WebAssembly Runtime Daemon for executing modules
- `example/` - Example Component Model implementation
- `docs/` - Documentation including requirements and specifications
- `justfile` - Command runner for development tasks

## Installation

### Prerequisites

- Rust 1.70 or newer
- For development: [just](https://github.com/casey/just) command runner
- Python 3 (for documentation and hooks)
- Java (optional, for PlantUML diagrams in documentation)

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just command runner
cargo install just

# Setup project dependencies (installs Rust targets, Wasm tools, Python deps, etc.)
# Note: May require manual steps on Windows for PlantUML/Java.
just setup

# Alternatively, install components individually:
# just setup-rust-targets
# just setup-wasm-tools
# just setup-python-deps
# just setup-plantuml # (Requires manual steps on Windows)
```

## Getting Started

### Building the Project

The easiest way to build the project is using the provided justfile:

```bash
# Build all crates
just build

# Build specific components
just build-wrt      # Build only the library
just build-wrtd     # Build only the daemon
just build-example  # Build the example component
```

### Running the Example

The project includes an example WebAssembly component that demonstrates basic functionality:

```bash
# Build and run the example using wasmtime
just run-example

# Build and run the example using wrtd
just test-wrtd-example
```

### Using WRT in Your Project

Add WRT to your Cargo.toml:

```toml
[dependencies]
wrt = { git = "https://github.com/yourusername/wrt" }
```

Basic usage example:

```rust
use wrt::{Module, Engine, Result};

fn run_wasm_module(wasm_path: &str) -> Result<()> {
    // Create a new engine
    let mut engine = wrt::new_engine();
    
    // Load WebAssembly bytes from a file
    let wasm_bytes = std::fs::read(wasm_path)?;
    
    // Parse the WebAssembly module
    let module = Module::from_bytes(&wasm_bytes)?;
    
    // Instantiate the module
    let instance_idx = engine.instantiate(&module)?;
    
    // Execute a function (assuming exported function "main" exists)
    let results = engine.execute(instance_idx, 0, vec![])?;
    
    println!("Execution completed with results: {:?}", results);
    
    Ok(())
}
```

For more advanced examples, see the [example directory](./example).

## Development

The project uses the `just` command runner to simplify development tasks.

### Available Commands

```bash
# List all available commands
just --list

# Build everything
just build

# Run tests
just test

# Generate code coverage report
just coverage      # Creates HTML report at target/coverage/tarpaulin-report.html

# Code quality checks
just check
just check-imports  # Check import organization (via xtask)
just check-udeps    # Check for unused dependencies (via cargo-machete)
just check-all      # Run all checks

# Documentation
just docs           # Build HTML documentation with diagrams (Default)
just docs-html      # Build basic HTML documentation
just docs-pdf       # Build PDF documentation (requires LaTeX)

# Clean build artifacts
just clean
```

### Code Organization Standards

The project follows these guidelines:

1. Imports are organized in this order:
   - Standard library imports (std, core, alloc)
   - External crates/third-party dependencies
   - Internal modules (crate:: imports)

2. All public API should be documented following the format in CLAUDE.md.

3. Each module is organized by functionality with clear separation of concerns.

## Documentation

WRT uses two documentation systems:

1. **Rust API Documentation**: Generated with `cargo doc`

   ```bash
   # Build docs for all workspace members (excluding xtask)
   cargo doc --workspace --exclude xtask --all-features --open
   ```

2. **Requirements and Specifications**: Using Sphinx with sphinx-needs

   ```bash
   # Build Sphinx docs (HTML with diagrams by default)
   just docs
   # Documentation will be available in docs/_build/html
   ```

## CI/CD

This project includes GitHub Actions workflows that automatically run on pull requests and pushes to main:

- Build and test checks
- Code style enforcement
- Code coverage reporting (with Codecov integration)
- Documentation generation
- Security audit
- Unused dependency detection

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
