# WebAssembly Runtime Tests

This directory contains various tests for the WebAssembly runtime implementation.

## Test Structure

- `lib.rs`: Main test file that aggregates all test modules
- `simple_spec_tests.rs`: Simple tests for basic WebAssembly functionality
- `simd_tests.rs`: Tests for SIMD instructions implementation
- `wasm_testsuite.rs`: Tests that use the official WebAssembly test suite
- `wast_proc_macro/`: Procedural macros for generating tests from WebAssembly test suite files

## Automated WebAssembly Test Suite

The WebAssembly test suite is now automatically downloaded and updated during the build process. If you run `cargo test`, the test suite will be:

1. Downloaded from the official WebAssembly test suite repository if it doesn't exist
2. Updated if there are changes and you have an internet connection
3. Skipped if there's no internet connection and the test suite isn't already downloaded

The build script in `wrt/build.rs` handles all of this automatically, and sets environment variables that the tests use:

- `WASM_TESTSUITE`: Path to the downloaded test suite
- `WASM_TESTSUITE_COMMIT`: Git commit hash of the current test suite version

## Using the Test Macros

We provide procedural macros to automatically generate tests from WebAssembly test suite files:

### Testing a Single WAST File

```rust
use wast_proc_macro::generate_wast_tests;

#[generate_wast_tests("simd/simd_lane.wast", "simd_lane")]
fn run_simd_lane_tests() {
    // This function will be called for the test
    // Test implementation goes here
}
```

### Testing All WAST Files in a Directory

```rust
use wast_proc_macro::generate_directory_tests;

#[generate_directory_tests("simd", "simd")]
fn run_simd_tests(file_name: &str, test_name: &str) {
    // This function will be called for each WAST file in the directory
    // Test implementation goes here
}
```

## Manual WebAssembly Testsuite Tests

For backwards compatibility, you can still manually run the tests:

1. Set the `WASM_TESTSUITE` environment variable to the path where the test suite is downloaded:
   ```bash
   export WASM_TESTSUITE=/path/to/testsuite
   ```

2. Run the tests:
   ```bash
   cargo test
   ```

## WebAssembly Testsuite Structure

The official WebAssembly testsuite contains test modules organized by feature:

- `simd/`: Tests for SIMD instructions
- `threads/`: Tests for threading features
- `reference-types/`: Tests for reference types
- And many more...

Our test runner will look for specific files in these directories and run them. If a file is not found, the test will be skipped. 