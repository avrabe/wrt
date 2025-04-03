# WRT Example Using WASI Logging

This WebAssembly Component demonstrates how to import and use the standard WASI logging interface.

## Component Model Features

This example showcases:
- Importing WASI logging interface
- Defining interfaces with WIT (WebAssembly Interface Types)
- Component model implementation using wit-bindgen
- WASI Preview 2 target usage

## Building

```bash
# Install prerequisites (if not done via `just setup`)
rustup target add wasm32-wasip2

# Build the component (using Justfile is recommended)
just build-example

# Alternatively, build directly with cargo:
cargo build -p example --target wasm32-wasip2
```

## Structure

The example consists of:
- `example.wit` - Defines interfaces and imports WASI logging
- `src/lib.rs` - Implements the component using the imported logging
- Example logs messages at different levels during execution

## WIT Structure

```wit
// Import logging interface
import logging;

// Export our main interface
export example;
```

The component imports the logging interface and calls it from Rust code whenever it needs to log information. The hello function runs a loop and logs information at each iteration, demonstrating how to use the imported logging functionality.

## Testing with WRT

To use this example with WRT, you must provide an implementation for the imported logging interface when instantiating the component.