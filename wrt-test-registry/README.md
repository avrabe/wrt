# WebAssembly Runtime (WRT) Test Registry

A unified testing framework for the WebAssembly Runtime (WRT) project that works in both standard and no_std environments.

## Features

- **Unified Test Interface**: Single API for writing and running tests
- **Standard and no_std Support**: Works in both environment types
- **Test Filtering**: Filter tests by name, category, or environment requirements
- **CLI Test Runner**: Command-line interface for running and listing tests
- **Consistent Assertions**: Unified assertion macros that work everywhere

## Getting Started

### Adding as a Dependency

```toml
[dependencies]
wrt-test-registry = { path = "../wrt-test-registry", version = "0.1.0" }

[features]
std = ["wrt-test-registry/std"]
no_std = ["wrt-test-registry/no_std"]
```

### Writing Tests

Tests are registered using the `register_test!` macro:

```rust
use wrt_test_registry::{register_test, assert_test, assert_eq_test};

// Register a test with:
// 1. Test name
// 2. Test category
// 3. Whether it requires the standard library
// 4. Test function
register_test!(
    "my_test_name",
    "my_category",
    false, // does not require std
    || {
        // Test code here
        assert_test!(1 == 1, "This condition should be true");
        assert_eq_test!(42, 42, "These values should be equal");
        
        // Return Ok(()) for success, or Err(message) for failure
        Ok(())
    }
);
```

### Registering Tests

Create a module in your crate that registers your tests:

```rust
// src/tests.rs
pub fn register_my_tests() {
    register_test!("test1", "category1", false, || {
        // Test implementation
        Ok(())
    });
    
    register_test!("test2", "category2", true, || {
        // Test implementation that requires std
        Ok(())
    });
}
```

In your main.rs, call the registration function and run the tests:

```rust
mod tests;

fn main() {
    // Register all tests
    tests::register_my_tests();
    
    // Get the registry
    let registry = wrt_test_registry::TestRegistry::global();
    
    // Run all tests
    let failed_count = registry.run_all_tests();
    
    if failed_count == 0 {
        println!("All tests passed!");
    } else {
        println!("{} tests failed", failed_count);
        std::process::exit(1);
    }
}
```

### Using the CLI Runner

The `wrt-test-runner` binary provides a command-line interface for running and listing tests:

```
# Run all tests
cargo run -p wrt-test-registry --features runner --bin wrt-test-runner

# List all tests
cargo run -p wrt-test-registry --features runner --bin wrt-test-runner -- list

# Run tests in a specific category
cargo run -p wrt-test-registry --features runner --bin wrt-test-runner -- --category=decoder

# Run tests with a specific name
cargo run -p wrt-test-registry --features runner --bin wrt-test-runner -- --name=parser

# Skip tests that require the standard library
cargo run -p wrt-test-registry --features runner --bin wrt-test-runner -- --no-std
```

### Using with cargo-wrt

The unified build system provides commands for running tests:

```bash
# Build the test registry
cargo-wrt build --package wrt-test-registry

# Run all tests
cargo-wrt test

# Run tests with specific features
cargo-wrt test --package wrt-test-registry --features runner

# Run specific test categories
cargo-wrt test --filter decoder
cargo-wrt test --filter instruction-decoder
```

## Converting Existing Tests

To convert standalone executables to use the test registry:

1. **Extract Test Functions**: Move test logic from the main function to separate test functions
2. **Register with the Registry**: Use `register_test!` to register each test
3. **Update Main**: Change the main function to register and run tests
4. **Update Cargo.toml**: Add the test registry as a dependency

Example of converting a test:

Before:
```rust
fn main() {
    let test_data = prepare_test_data();
    
    let result = parse_data(&test_data);
    assert_eq!(result, expected_result);
    
    println!("Test passed!");
}
```

After:
```rust
mod tests {
    use wrt_test_registry::{register_test, assert_eq_test};
    
    pub fn register_tests() {
        register_test!("parse_data", "parser", false, || {
            let test_data = prepare_test_data();
            
            let result = parse_data(&test_data);
            assert_eq_test!(result, expected_result);
            
            Ok(())
        });
    }
}

fn main() {
    tests::register_tests();
    let registry = wrt_test_registry::TestRegistry::global();
    registry.run_all_tests();
}
```

## Benefits of the Unified Approach

- **Consistency**: All tests follow the same pattern
- **Flexibility**: Works in both std and no_std environments
- **Discoverability**: CLI runner helps find and filter tests
- **Compatibility**: Tests from different crates can be run together
- **Organization**: Tests can be categorized and filtered 