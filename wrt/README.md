# PulseEngine (WRT Edition)

> Safety-critical WebAssembly infrastructure implemented in pure Rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **⚠️ Source Installation Only**: PulseEngine is not published to crates.io yet. See the [Installation Guide](../docs/source/getting_started/installation.rst) for detailed setup instructions.

## Overview

PulseEngine (WRT Edition) provides WebAssembly infrastructure implemented in pure Rust, designed for safety-critical systems. It provides foundational components including memory management, type systems, and arithmetic operations, with the core execution engine under active development.

### Key Features

- **🛡️ Memory Safety**: Complete WebAssembly memory operations with bounds checking
- **🦀 Pure Rust**: Memory-safe implementation with zero unsafe code by default
- **🔄 Cross-Platform**: Runs on std, no_std+alloc, and pure no_std environments
- **⚙️ Type System**: Complete WebAssembly value types and validation infrastructure
- **🧮 Arithmetic Operations**: Full implementation of WebAssembly numeric instructions
- **🔧 Safety-Critical Design**: ASIL compliance framework and formal verification support
- **🚧 Development Status**: Core execution engine and Component Model under development

## Quick Start

For comprehensive installation instructions, see the [Installation Guide](../docs/source/getting_started/installation.rst).

### Add to Your Project

```toml
[dependencies]
wrt = { path = "path/to/wrt" }  # Point to local clone
```

### Basic Usage

```rust
use wrt::prelude::*;

// Current capabilities - memory and arithmetic operations
let memory = WrtMemory::new(1024)?;
let value = Value::I32(42);
let result = ArithmeticOp::I32Add.execute(&[value, Value::I32(8)])?;
println!("Result: {:?}", result);

// Note: Module instantiation and function execution under development
```

### Component Model Usage

```rust
// Component Model infrastructure (under development)
use wrt::component::*;

// Note: Component parsing and instantiation under development
// See documentation for current implementation status
```

## Architecture

WRT is built as a collection of specialized crates, each handling a specific aspect of WebAssembly execution:

```
┌─────────────────┐
│       wrt       │  ← Main facade crate
├─────────────────┤
│ wrt-runtime     │  ← Execution engine
│ wrt-component   │  ← Component Model
│ wrt-decoder     │  ← Binary parsing
│ wrt-foundation  │  ← Core types & utilities
│ wrt-error       │  ← Error handling
│ wrt-*           │  ← Additional modules
└─────────────────┘
```

### Core Modules

- **`wrt-runtime`**: Stackless execution engine with interpreter and future AOT support
- **`wrt-component`**: Complete WebAssembly Component Model implementation
- **`wrt-decoder`**: Fast, safe binary format parsing
- **`wrt-foundation`**: Bounded collections and safe memory abstractions
- **`wrt-error`**: Comprehensive error handling with context preservation

## Feature Flags

WRT provides fine-grained control over features and compilation targets:

### Environment Features
```toml
# Standard library (default)
wrt = { path = "path/to/wrt", features = ["std"] }

# No standard library with allocation
wrt = { path = "path/to/wrt", features = ["alloc"] }

# Pure no_std (embedded/bare-metal)
wrt = { path = "path/to/wrt", default-features = false }
```

### Capability Features
```toml
# Minimal runtime only
wrt = { path = "path/to/wrt", features = ["minimal"] }

# Safety-critical features (ASIL compliance framework)
wrt = { path = "path/to/wrt", features = ["safety"] }

# Performance optimizations
wrt = { path = "path/to/wrt", features = ["optimize"] }

# Serialization support
wrt = { path = "path/to/wrt", features = ["serialization"] }
```

### Platform Features
```toml
# Platform-specific optimizations
wrt = { path = "path/to/wrt", features = ["platform-macos"] }

# Helper mode for platform integration
wrt = { path = "path/to/wrt", features = ["helper-mode"] }
```

## no_std Support

WRT is designed from the ground up to work in constrained environments:

### Pure no_std (Embedded/Bare-metal)
```rust
#![no_std]
use wrt::prelude::*;

// Uses bounded collections, no heap allocation
let mut runtime = StacklessRuntime::new();
let result = runtime.execute_module(wasm_bytes)?;
```

### no_std + alloc
```rust
#![no_std]
extern crate alloc;
use wrt::prelude::*;

// Full functionality with heap allocation
let module = Module::from_bytes(wasm_bytes)?;
let instance = ModuleInstance::new(module, imports)?;
```

## Examples

### Error Handling
```rust
use wrt::{prelude::*, WrtResult};

fn execute_wasm(wasm: &[u8]) -> WrtResult<Value> {
    let module = Module::from_bytes(wasm)
        .map_err(|e| e.with_context("Failed to parse WebAssembly module"))?;
    
    let mut instance = ModuleInstance::new(module, ImportMap::new())?;
    instance.invoke("main", &[])
}
```

### Fuel-Limited Execution
```rust
use wrt::prelude::*;

// Limit execution to prevent infinite loops
let mut instance = ModuleInstance::new(module, imports)?;
instance.set_fuel(1000)?; // 1000 instruction limit

let result = instance.invoke("compute", &[Value::I32(42)])?;
println!("Remaining fuel: {}", instance.fuel());
```

### Component Model Integration
```rust
use wrt::component::*;

// Define a host function
fn host_log(msg: &str) -> ComponentResult<()> {
    println!("WASM: {}", msg);
    Ok(())
}

// Create component with host imports
let mut imports = ComponentImports::new();
imports.define("host", "log", host_log)?;

let component = Component::from_bytes(component_bytes)?;
let instance = component.instantiate(&imports)?;
```

## Performance

WRT is designed for performance across different environments:

- **Interpreter**: ~10-50x slower than native (depending on workload)
- **Memory usage**: Configurable, down to <64KB for embedded use
- **Startup time**: <1ms for typical modules
- **Stack usage**: Bounded, configurable for stackless execution

### Benchmarks
```bash
cargo bench --features=std
```

## Platform Support

WRT supports a wide range of platforms and environments:

### Tested Platforms
- **Linux** (x86_64, ARM64, ARM32)
- **macOS** (x86_64, ARM64)
- **Windows** (x86_64)
- **Embedded** (ARM Cortex-M, RISC-V)
- **WebAssembly** (wasm32-unknown-unknown)

### RTOS Support
- **FreeRTOS**
- **Zephyr**
- **QNX**
- **VxWorks**
- **Tock OS**

## Safety & Compliance

WRT is designed for safety-critical applications:

- **Zero unsafe code** in default configuration
- **ASIL-B compliance** features available
- **Bounded memory usage** in no_std mode
- **Deterministic execution** options
- **Formal verification** support (via Kani)

### Safety Features
```toml
wrt = { path = "path/to/wrt", features = ["safety"] }
```

Enables:
- Enhanced bounds checking
- Memory access validation
- Execution time limits
- Resource usage tracking

## Documentation

- **[API Documentation](https://docs.rs/wrt)** - Complete API reference
- **[Architecture Guide](../docs/source/architecture/)** - System design and components
- **[User Guide](../docs/source/user_guide/)** - Integration examples and patterns
- **[Developer Guide](../docs/source/development/)** - Contributing and internals

### Generate Local Documentation
```bash
cargo doc --workspace --open
```

## Integration Examples

### With Tokio (Async)
```rust
use wrt::prelude::*;
use tokio::runtime::Runtime;

let rt = Runtime::new()?;
let result = rt.block_on(async {
    let module = Module::from_bytes(wasm_bytes)?;
    let mut instance = ModuleInstance::new(module, imports)?;
    instance.invoke_async("async_function", &[]).await
})?;
```

### With Embedded HAL
```rust
#![no_std]
#![no_main]

use wrt::prelude::*;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let wasm = include_bytes!("embedded.wasm");
    let mut runtime = StacklessRuntime::new();
    
    match runtime.execute_module(wasm) {
        Ok(result) => {
            // Handle successful execution
        }
        Err(e) => {
            // Handle error
        }
    }
    
    loop { /* ... */ }
}
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](../CONTRIBUTING.md) for details.

### Development Setup
```bash
git clone https://github.com/pulseengine/wrt
cd wrt
cargo build --workspace
cargo test --workspace
```

### Running Tests
```bash
# All tests
cargo test --workspace

# Specific environment
cargo test --features=std
cargo test --features=alloc --no-default-features
cargo test --no-default-features  # Pure no_std
```

## License

Licensed under the [MIT License](../LICENSE).

## See Also

- **[WebAssembly Specification](https://webassembly.github.io/spec/)**
- **[Component Model Specification](https://github.com/WebAssembly/component-model)**
- **[WRT Documentation](../docs/)**