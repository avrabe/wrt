// Example using WASI logging imports

// Generate bindings from the WIT file
wit_bindgen::generate!({
    // Specify the path to our WIT directory
    path: "wit",

    // Define the world we're implementing
    world: "hello",

    // Specify which interfaces we're exporting
    exports: {
        "example:hello/example": HelloComponent,
    },
});

// Import the generated logging functions
use example::hello::logging;
use exports::example::hello::example::Guest;

struct HelloComponent;

// Implement the example interface
impl Guest for HelloComponent {
    // Our main hello function for testing
    fn hello() -> i32 {
        println!("SIMPLE_TEST: Minimal example with I32 operations");

        // Test I32 operations (these are known to work)
        let a: i32 = 10;
        let b: i32 = 20;

        // Test addition
        let sum = a + b;
        println!("I32 Add: 10 + 20 = {}", sum);

        // Test subtraction
        let diff = b - a;
        println!("I32 Sub: 20 - 10 = {}", diff);

        // Test multiplication
        let product = a * b;
        println!("I32 Mul: 10 * 20 = {}", product);

        // Test comparison
        if a < b {
            println!("I32 comparison: 10 < 20 (correct)");
        } else {
            println!("I32 comparison: 10 >= 20 (wrong)");
            return 1;
        }

        // Return the sum as our result
        sum
    }
}
