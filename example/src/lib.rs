// Example using WASI logging imports

// Restore wit_bindgen::generate! macro
wit_bindgen::generate!({
    path: "../wit",
    world: "hello-world",
    // Explicitly generate bindings for imported WASI interfaces
    with: {
        "wasi:logging/logging@0.2.0": generate,
        "wasi:cli/environment@0.2.0": generate,
        "wasi:io/streams@0.2.4": generate,
    }
});

// Import paths expected from the generate! macro
use crate::exports::example::hello::greeter::Guest;
use crate::wasi::logging::logging::{log, Level};

pub struct HelloComponent;

// Implement the required Guest trait
impl Guest for HelloComponent {
    fn hello() -> i32 {
        log(
            Level::Info,
            "example",
            "SIMPLE_TEST: Minimal example with I32 operations",
        );

        let a: i32 = 10;
        let b: i32 = 20;

        // Test addition
        let sum = a + b;
        log(
            Level::Info,
            "example",
            &format!("I32 Add: 10 + 20 = {}", sum),
        );

        // Test subtraction
        let diff = b - a;
        log(
            Level::Info,
            "example",
            &format!("I32 Sub: 20 - 10 = {}", diff),
        );

        // Test multiplication
        let product = a * b;
        log(
            Level::Info,
            "example",
            &format!("I32 Mul: 10 * 20 = {}", product),
        );

        // Test comparison
        if a < b {
            log(Level::Info, "example", "I32 comparison: 10 < 20 (correct)");
        } else {
            log(Level::Info, "example", "I32 comparison: 10 >= 20 (wrong)");
            return 1;
        }

        sum
    }
}

// Re-add export macro for wit_bindgen::generate!
bindings::export!(HelloComponent with_types_in bindings);
