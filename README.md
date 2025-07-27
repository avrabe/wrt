# PulseEngine (WRT Edition)

A pure Rust implementation of WebAssembly infrastructure designed for safety-critical systems. Provides foundational components for WebAssembly execution with emphasis on memory safety, deterministic behavior, and formal verification capabilities.

## Features

- **Memory Operations**: Complete WebAssembly memory management with bounds checking
- **Arithmetic Instructions**: Full implementation of WebAssembly numeric operations
- **Type System**: Complete WebAssembly value types and validation infrastructure
- **`no_std` Compatible**: Works in embedded and bare-metal environments
- **Safety-Critical Design**: ASIL compliance framework and formal verification support
- **Development Status**: Core execution engine and Component Model under development

## Quick Start

For comprehensive installation instructions, see the [Installation Guide](docs/source/getting_started/installation.rst).

### Prerequisites

- Rust 1.86.0 or newer
- cargo-wrt (included in this repository)

### Building from Source

```bash
# Clone repository
git clone https://github.com/pulseengine/wrt
cd wrt

# Install cargo-wrt
cargo install --path cargo-wrt

# Build everything
cargo-wrt build

# Run tests
cargo-wrt test

# Run example (requires setup)
cargo-wrt wrtd --test
```

### Usage

**Note**: PulseEngine is currently available only as source code. Add it to your project:

```toml
[dependencies]
wrt = { path = "path/to/wrt" }  # Point to local clone
```

Basic usage:

```rust
use wrt::prelude::*;

// Note: Core execution engine under development
// Current example shows memory and arithmetic operations
let memory = WrtMemory::new(1024)?;
let value = Value::I32(42);
let result = ArithmeticOp::I32Add.execute(&[value, Value::I32(8)])?;
```

## Project Structure

This is a multi-crate workspace:

- **`wrt/`** - Main library facade
- **`wrt-foundation/`** - Core types and bounded collections  
- **`wrt-runtime/`** - Execution engine
- **`wrt-component/`** - Component Model implementation
- **`wrt-decoder/`** - Binary format parsing
- **`wrtd/`** - Standalone runtime daemon
- **`example/`** - Example WebAssembly component

## Documentation

- **[API Documentation](docs/source/)** - Complete API reference and specifications
- **[Architecture Guide](docs/source/architecture/)** - System design and components
- **[Developer Guide](docs/source/development/)** - Contributing and development setup

Generate documentation:

```bash
# Build comprehensive documentation
cargo-wrt docs --private

# Open documentation in browser
cargo-wrt docs --open

# API documentation only  
cargo doc --workspace --open

# Generate and view coverage reports
cargo-wrt coverage --html --open
```

## Development

Setup development environment:

```bash
# Check development tool dependencies
cargo-wrt setup --check

# Setup git hooks for code quality checks
cargo-wrt setup --hooks

# Setup all development tools
cargo-wrt setup --all

# Manage tool version requirements
cargo-wrt tool-versions check --verbose    # Check all tool versions
cargo-wrt tool-versions generate           # Generate tool-versions.toml
```

See the [Developer Guide](docs/source/development/) for detailed development instructions.

Common commands:

```bash
cargo-wrt --help             # Show all available commands
cargo-wrt check              # Format code and run static analysis
cargo-wrt ci                 # Run main CI checks
cargo-wrt verify --asil d    # Run complete verification suite

# Development commands
cargo-wrt no-std             # Verify no_std compatibility
cargo-wrt check --strict      # Strict code formatting and linting
cargo-wrt coverage --html     # Generate code coverage
cargo-wrt simulate-ci         # Simulate CI workflow locally
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
