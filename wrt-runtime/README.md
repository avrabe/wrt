# wrt-runtime

Core runtime implementation for PulseEngine (WRT Edition).

This crate provides WebAssembly runtime infrastructure including memory management, type systems, and foundational components for WebAssembly execution. The instruction execution engine is currently under development.

## Features

- **Implemented**: Memory management with safety features and bounds checking
- **Implemented**: WebAssembly value types and type system
- **Implemented**: Table operations and global variable infrastructure
- **In Development**: WebAssembly instruction execution engine
- **In Development**: Module instantiation and function calling
- **In Development**: Stackless execution engine for constrained environments
- **Planned**: Control Flow Integrity (CFI) protection
- **Implemented**: Support for both `std` and `no_std` environments

## Architecture

### Core Components

- **Module**: Represents a parsed WebAssembly module
- **ModuleInstance**: Runtime instance of a module with its own memory, tables, and globals
- **Memory**: Linear memory with bounds checking and safety features
- **Table**: Function and element tables
- **Global**: Global variables
- **Execution**: Instruction execution engine

### Stackless Execution

The stackless execution engine allows running WebAssembly in environments with limited stack space:

```rust
use wrt_runtime::stackless::StacklessEngine;

// Create a stackless engine with limited stack frames
let engine = StacklessEngine::new(max_frames);
```

### Control Flow Integrity

Built-in CFI protection guards against control flow hijacking:

```rust
use wrt_runtime::cfi_engine::CfiEngine;

// Create an engine with CFI protection
let engine = CfiEngine::new(policy);
```

## no_std Support

This crate supports `no_std` environments with the `alloc` feature. Without `alloc`, bounded alternatives from `wrt-foundation` are used.

## Usage

```rust
use wrt_runtime::prelude::*;

// Current capabilities - memory and arithmetic operations
let memory = WrtMemory::new(1024)?;
let stats = ExecutionStats::new();

// Note: Module instantiation and function execution under development
// See documentation for current implementation status
```

## License

Licensed under the MIT license. See LICENSE file in the project root for details.