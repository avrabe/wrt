#!/bin/bash
# Build script for WASI-NN example module

set -e

echo "Building WASI-NN inference example module..."

cd inference_module

# Build the WebAssembly module
cargo build --target wasm32-unknown-unknown --release

# Copy the built module to the examples directory
cp target/wasm32-unknown-unknown/release/wasi_nn_inference_example.wasm ../simple_inference.wasm

echo "Built simple_inference.wasm successfully!"
echo "You can now run the example with:"
echo "  cargo run --example wrtd_nn_example --features std,wasi,wasi-nn"