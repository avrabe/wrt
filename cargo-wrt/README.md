# cargo-wrt

Unified build tool for WRT (WebAssembly Runtime) - the next-generation safety-critical WebAssembly runtime.

## Installation

### From crates.io (once published)
```bash
cargo install cargo-wrt
```

### From source
```bash
git clone https://github.com/pulseengine/wrt
cd wrt
cargo install --path cargo-wrt
```

### For development
```bash
cargo build -p cargo-wrt
# Binary will be available at ./target/debug/cargo-wrt
```

## Usage

cargo-wrt supports two usage patterns:

### 1. Direct Usage
```bash
cargo-wrt --help
cargo-wrt build
cargo-wrt test
```

### 2. Cargo Subcommand
```bash
cargo wrt --help
cargo wrt build  
cargo wrt test
```

Both patterns work identically - the same binary automatically detects how it's being called and adjusts accordingly.

### Available Commands

- `build` - Build all WRT components
- `test` - Run tests across the workspace  
- `verify` - Run safety verification and compliance checks
- `docs` - Generate documentation
- `coverage` - Run coverage analysis
- `check` - Run static analysis (clippy + formatting)
- `no-std` - Verify no_std compatibility
- `wrtd` - Build WRTD (WebAssembly Runtime Daemon) binaries
- `ci` - Run comprehensive CI checks
- `clean` - Clean build artifacts
- `setup` - Setup development environment and tools
- `tool-versions` - Manage tool version configuration

### Examples

```bash
# Check what tools are available
cargo-wrt setup --check

# Install optional tools for advanced features
cargo-wrt setup --install

# Complete setup (check tools, install missing ones, setup git hooks)
cargo-wrt setup --all

# Build all components
cargo-wrt build
# or
cargo wrt build

# Run all tests  
cargo-wrt test
# or
cargo wrt test

# Run safety verification for ASIL-D level
cargo-wrt verify --asil d
# or
cargo wrt verify --asil d

# Run fuzzing (automatically checks for cargo-fuzz)
cargo-wrt fuzz --list
# or
cargo wrt fuzz --list

# Generate and open documentation
cargo-wrt docs --open
# or 
cargo wrt docs --open

# Run full CI pipeline
cargo-wrt ci
# or
cargo wrt ci
```

## Features

- **Safety-Critical Support**: ASIL-D compliance verification
- **Cross-Platform**: Supports std/no_std environments
- **Comprehensive Testing**: Unit, integration, and formal verification
- **AI-Friendly Architecture**: Clean, linear build process
- **Modern Rust**: Uses latest Rust features and best practices
- **Smart Tool Management**: Automatically detects missing tools and provides installation guidance
- **Enhanced Error Messages**: Clear, actionable error messages when tools are missing
- **Easy Setup**: One-command setup for development environment

## Tool Management

cargo-wrt automatically detects and manages external tool dependencies:

### Required Tools (Usually Available)
- `cargo` - Rust package manager
- `rustc` - Rust compiler

### Optional Tools (Installed as Needed)  
- `kani` - Formal verification (for `verify` and `kani-verify` commands)
- `cargo-fuzz` - Fuzzing support (for `fuzz` command)
- `git` - Version control (for `setup` command)

### Setup Commands
```bash
# Check what tools are available
cargo-wrt setup --check

# Install missing optional tools
cargo-wrt setup --install

# Complete development setup
cargo-wrt setup --all
```

### Tool Version Management

cargo-wrt now includes sophisticated tool version management with configurable requirements:

```bash
# Check all tool versions against requirements
cargo-wrt tool-versions check

# Show detailed version information
cargo-wrt tool-versions check --verbose

# Check specific tool only
cargo-wrt tool-versions check --tool kani

# Generate tool-versions.toml configuration file
cargo-wrt tool-versions generate

# Generate comprehensive configuration (all tools)
cargo-wrt tool-versions generate --all

# Update existing configuration (future feature)
cargo-wrt tool-versions update --all
```

The tool version system uses a `tool-versions.toml` file in the workspace root to specify:
- Exact version requirements (e.g., kani must be exactly 0.63.0)
- Minimum version requirements (e.g., git must be at least 2.30.0)
- Installation commands for each tool
- Which cargo-wrt commands require each tool

This ensures reproducible builds and consistent development environments across all contributors.

When you run a command that needs a missing tool, cargo-wrt will:
1. Detect the missing tool
2. Show a helpful error message
3. Provide exact installation commands
4. Suggest running `cargo-wrt setup --install`

## License

MIT - see LICENSE file for details.