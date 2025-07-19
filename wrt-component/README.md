# wrt-component

> WebAssembly Component Model implementation for WRT

## Overview

Provides a complete implementation of the WebAssembly Component Model specification for WRT. This crate enables interoperability between WebAssembly components through standardized interfaces, type definitions, and linking mechanisms.

## Features

- **Component Model Specification** - Full implementation of WebAssembly Component Model
- **Interface Types** - Support for rich interface type definitions (WIT format)
- **Component Linking** - Runtime linking and composition of WebAssembly components
- **WASI Integration** - Seamless integration with WASI (WebAssembly System Interface)
- **Resource Management** - Safe handling of component resources and capabilities
- **Async Execution** - Support for asynchronous component execution patterns
- **no_std Compatible** - Works in embedded and constrained environments
- **ASIL Compliance** - Safety-critical execution with ASIL-D support

## Quick Start

```toml
[dependencies]
wrt-component = "0.2"
```

```rust
use wrt_component::prelude::*;

// Load and instantiate a WebAssembly component
let component_bytes = include_bytes!("example.wasm");
let component = Component::from_bytes(component_bytes)?;

// Create a component instance with specified limits
let instance = ComponentInstance::new(component, ComponentLimits::default())?;

// Call component exports
let result = instance.call_export("example_function", &args)?;
```

## Architecture

### Core Components

- **Component**: Parsed WebAssembly component with metadata and interfaces
- **ComponentInstance**: Runtime instance with memory, resources, and execution state
- **Interface**: Type definitions and function signatures for component interfaces
- **Linker**: Component composition and inter-component communication
- **ResourceManager**: Safe management of component resources and capabilities

### Component Model Features

```rust
use wrt_component::*;

// Define component interfaces using WIT
let interface = Interface::from_wit(r#"
    interface example {
        record point {
            x: f32,
            y: f32,
        }
        
        transform: func(points: list<point>) -> list<point>
    }
"#)?;

// Link components together
let mut linker = ComponentLinker::new();
linker.define_component("graphics", graphics_component)?;
linker.define_component("math", math_component)?;
```

### ASIL-D Safety Features

The component system provides safety-critical execution capabilities:

```rust
use wrt_component::safety::*;

// Create ASIL-D compliant component instance
let safety_config = AsilConfig::asil_d()
    .with_fuel_limit(10000)
    .with_memory_limit(1024 * 1024)
    .with_call_depth_limit(32);

let instance = ComponentInstance::new_with_safety(component, safety_config)?;
```

### Async Component Execution

Support for asynchronous WebAssembly component execution:

```rust
use wrt_component::async_::*;

// Execute components asynchronously
let async_instance = AsyncComponentInstance::new(component).await?;
let future_result = async_instance.call_async("long_running_task", &args).await?;
```

## Resource Management

The component system provides fine-grained resource control:

```rust
use wrt_component::resources::*;

// Configure resource limits per component
let resource_config = ResourceConfig::new()
    .with_max_handles(100)
    .with_max_memory_per_resource(64 * 1024)
    .with_filesystem_access(FilesystemAccess::ReadOnly("/safe/path"));

let instance = ComponentInstance::with_resources(component, resource_config)?;
```

## no_std Support

This crate supports `no_std` environments using bounded collections from `wrt-foundation`:

```toml
[dependencies]
wrt-component = { version = "0.2", default-features = false }
```

In `no_std` mode, all collections use compile-time capacity limits for deterministic memory usage, making it suitable for ASIL-D safety-critical applications.

## See Also

- [WebAssembly Component Model Specification](https://github.com/WebAssembly/component-model)
- [WIT (WebAssembly Interface Types)](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- [API Documentation](https://docs.rs/wrt-component)
- [Component Development Guide](../docs/source/development/components.rst)

## License

Licensed under the MIT license. See LICENSE file in the project root for details.