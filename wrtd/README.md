# WRTD - WebAssembly Runtime Daemon

A comprehensive WebAssembly runtime daemon with support for WASI, component model, and safety-critical features.

## Features

### Core Runtime
- **Multi-Mode Execution**: std, alloc, and no_std runtime modes
- **Bounded Resource Management**: Safety-critical memory and fuel limits
- **Platform Abstraction**: Cross-platform support with optimizations
- **Memory Profiling**: Real-time memory usage tracking and analysis

### WASI Support
- **WASI Preview 1**: Complete snapshot_preview1 implementation
- **WASI Preview 2**: Component model-based WASI
- **Capability-Based Security**: Fine-grained permission control
- **Host Function Registry**: Extensible host function system

### Component Model
- **WebAssembly Components**: Full component model support
- **Interface Registry**: Component interface management
- **Cross-Component Communication**: Safe inter-component calls
- **Component Linking**: Dynamic component composition

### Safety-Critical Features
- **ISO 26262 Compliance**: ASIL-C safety requirements
- **Bounded Allocations**: Compile-time memory limits
- **Deterministic Execution**: Predictable runtime behavior
- **Memory Budget System**: Budget-aware resource allocation

## Installation

For comprehensive installation instructions, see the [Installation Guide](../docs/source/getting_started/installation.rst).

### Building from Source

```bash
# Build all variants
cargo build --release

# Build specific runtime mode
cargo build --release --features runtime-std
cargo build --release --features runtime-alloc  
cargo build --release --features runtime-nostd

# Build with WASI support
cargo build --release --features wasi

# Build with component model
cargo build --release --features component-model
```

## Usage

### Basic Module Execution

```bash
# Execute a WebAssembly module
wrtd module.wasm

# Execute specific function
wrtd module.wasm --function main

# Set resource limits
wrtd module.wasm --fuel 100000 --memory 1048576
```

### WASI Features

```bash
# Enable WASI Preview 1
wrtd wasi_program.wasm --wasi

# Specify WASI version
wrtd wasi_program.wasm --wasi --wasi-version preview2

# Grant filesystem access
wrtd wasi_program.wasm --wasi --wasi-fs /tmp --wasi-fs /home/user

# Expose environment variables
wrtd wasi_program.wasm --wasi --wasi-env HOME --wasi-env PATH

# Pass program arguments
wrtd wasi_program.wasm --wasi --wasi-arg "--verbose" --wasi-arg "input.txt"
```

### Component Model

```bash
# Enable component model
wrtd component.wasm --component

# Register component interfaces
wrtd component.wasm --component --interface wasi:filesystem --interface custom:api
```

### Advanced Features

```bash
# Enable memory profiling
wrtd module.wasm --memory-profile

# Disable platform optimizations
wrtd module.wasm --no-platform-opt

# Force no-std mode
wrtd module.wasm --no-std
```

### Complete Example

```bash
# Full-featured WASI application with component model
wrtd my_app.wasm \\
  --wasi \\
  --wasi-version preview2 \\
  --wasi-fs /app/data \\
  --wasi-env HOME \\
  --wasi-env USER \\
  --wasi-arg "--config" \\
  --wasi-arg "/app/config.toml" \\
  --component \\
  --interface wasi:filesystem \\
  --interface wasi:cli \\
  --memory-profile \\
  --fuel 1000000 \\
  --memory 16777216
```

## Configuration

### Runtime Modes

#### Standard Mode (std)
- Full standard library support
- Dynamic memory allocation
- Filesystem and network access
- Complete WASI implementation

#### Allocation Mode (alloc)
- Heap allocation without std
- Embedded Linux support
- Limited system access
- Core WASI functions

#### No-std Mode (no_std)
- Stack-only execution
- Bare metal/microcontroller support
- Static memory allocation
- Minimal WASI subset

### WASI Capabilities

WASI capabilities provide fine-grained security control:

```rust
// Filesystem access
capabilities.filesystem.add_allowed_path("/safe/directory");

// Environment variables
capabilities.environment.add_allowed_var("HOME");
capabilities.environment.args_access = true;
capabilities.environment.environ_access = true;

// Process control
capabilities.process.exit_allowed = true;
```

### Component Interfaces

Register component interfaces for the component model:

- `wasi:filesystem` - File system operations
- `wasi:cli` - Command-line interface
- `wasi:http` - HTTP client/server
- `custom:*` - Application-specific interfaces

### Memory Configuration

```bash
# Set memory limits
wrtd module.wasm --memory 1048576  # 1MB limit

# Enable profiling
wrtd module.wasm --memory-profile

# View memory statistics
# ✓ Execution completed successfully
#   Modules executed: 1
#   Peak memory: 524288 bytes
#   Memory Profiling:
#     Peak usage: 524288 bytes
#     Current usage: 0 bytes
```

## Architecture

### Host Function Registry

All host functions (WASI, custom) are registered through a unified registry:

```rust
let mut registry = CallbackRegistry::new();

// WASI functions are automatically registered
let wasi_provider = CompletePreview1Provider::new(capabilities)?;
let functions = wasi_provider.get_host_functions()?;

for function in functions {
    registry.register_function(function)?;
}
```

### Memory Budget System

Uses WRT's budget-aware allocation system:

```rust
// All allocations go through budget-aware providers
let provider = wrt_provider!(size, CrateId::Wrtd).unwrap();
let buffer = BoundedVec::new(provider)?;
```

### Platform Abstraction

Leverages wrt-platform for cross-platform support:

```rust
// Platform-specific optimizations
PlatformMemory::init_optimizations()?;

// Time functions
let timestamp = PlatformTime::wall_clock_ns()?;

// Threading (where available)
PlatformThreading::spawn_thread(task)?;
```

## Safety and Security

### ISO 26262 Compliance

- **ASIL-C** safety integrity level
- **Bounded operations** - No unbounded loops or allocations
- **Deterministic execution** - Predictable timing and memory usage
- **Error propagation** - Explicit error handling throughout

### Security Features

- **Capability-based access control** for WASI functions
- **Memory isolation** between components
- **Resource limits** prevent DoS attacks
- **Input validation** for all external data

### Resource Limits

```bash
# Fuel limits prevent infinite loops
wrtd module.wasm --fuel 1000000

# Memory limits prevent excessive allocation
wrtd module.wasm --memory 16777216

# Execution timeouts (platform-dependent)
timeout 30s wrtd long_running.wasm
```

## Development

### Build Features

- `std` - Standard library support (default)
- `alloc` - Allocation without std
- `no_std` - Bare metal support
- `wasi` - WASI Preview 1 & 2 support
- `component-model` - WebAssembly component model
- `safety-critical` - Enhanced safety features
- `platform-optimizations` - Platform-specific optimizations

### Testing

```bash
# Run all tests
cargo test

# Test specific features
cargo test --features wasi
cargo test --features component-model

# Integration tests
cargo test integrated_features_test

# Benchmark tests
cargo test benchmarks --release
```

### Cross-Compilation

```bash
# ARM64 Linux
cargo build --target aarch64-unknown-linux-gnu

# ARM Cortex-M (no_std)
cargo build --target thumbv7em-none-eabihf --no-default-features --features no_std

# WASM target (for WASI components)
cargo build --target wasm32-wasi
```

## Examples

### Basic WASI Program

```rust
// hello.rs
fn main() {
    println!("Hello from WASI!");
    std::env::args().for_each(|arg| println!("Arg: {}", arg));
}
```

```bash
# Compile to WASM
rustc --target wasm32-wasi hello.rs -o hello.wasm

# Run with wrtd
wrtd hello.wasm --wasi --wasi-arg "world"
```

### Component Model Example

```rust
// component.wit
package example:app;

interface calculator {
  add: func(a: s32, b: s32) -> s32;
}

world app {
  export calculator;
  import wasi:cli/stdout;
}
```

```bash
# Build component
wit-bindgen component.wit
cargo component build

# Run with wrtd
wrtd component.wasm --component --interface wasi:cli
```

## Troubleshooting

### Common Issues

1. **Module not found**
   ```bash
   Error: No module specified
   # Solution: Provide a WASM module path
   wrtd my_module.wasm
   ```

2. **WASI functions not available**
   ```bash
   # Solution: Enable WASI support
   wrtd module.wasm --wasi
   ```

3. **Permission denied for filesystem access**
   ```bash
   # Solution: Grant filesystem permissions
   wrtd module.wasm --wasi --wasi-fs /path/to/directory
   ```

4. **Out of fuel/memory errors**
   ```bash
   # Solution: Increase limits
   wrtd module.wasm --fuel 2000000 --memory 33554432
   ```

### Debug Information

```bash
# Enable verbose logging (if compiled with logging)
RUST_LOG=debug wrtd module.wasm

# Check resource usage
wrtd module.wasm --memory-profile

# Validate module format
wrtd module.wasm --no-std  # Will validate without execution
```

## License

Licensed under the Apache License, Version 2.0 or the MIT License, at your option.

## Contributing

1. Follow ISO 26262 safety requirements for safety-critical code
2. All allocations must use bounded collections
3. Include comprehensive tests for new features
4. Update documentation for API changes
5. Ensure cross-platform compatibility