//! Demonstration of bounded WIT parsing for no_std environments
//!
//! This example shows how to use the bounded WIT parser to parse simple
//! WIT definitions in constrained environments without allocation.

use wrt_format::wit_parser_bounded::{
    parse_wit_bounded,
    BoundedWitParser,
};
use wrt_foundation::NoStdProvider;

fn main() -> Result<(), wrt_error::Error> {
    println!("=== Bounded WIT Parser Demo ===\n");

    // Example 1: Simple world parsing
    println!("Example 1: Parsing a simple world definition");
    let simple_world = r#"
        world test-world {
            import test-func: func(x: u32) -> string
            export main: func() -> u32
        }
    "#;

    match parse_wit_bounded(simple_world) {
        Ok(parser) => {
            println!("✓ Parsed {} worlds", parser.world_count);
            for world in parser.worlds() {
                if let Ok(name) = world.name.as_str() {
                    println!("  World: '{}'", name);
                    println!("    Imports: {}", world.import_count);
                    println!("    Exports: {}", world.export_count);
                }
            }
        },
        Err(e) => {
            println!("✗ Failed to parse world: {:?}", e);
        },
    }
    println!);

    // Example 2: Simple interface parsing
    println!("Example 2: Parsing a simple interface definition");
    let simple_interface = r#"
        interface test-interface {
            hello: func() -> string
            add: func(a: u32, b: u32) -> u32
        }
    "#;

    match parse_wit_bounded(simple_interface) {
        Ok(parser) => {
            println!("✓ Parsed {} interfaces", parser.interface_count);
            for interface in parser.interfaces() {
                if let Ok(name) = interface.name.as_str() {
                    println!("  Interface: '{}'", name);
                    println!("    Functions: {}", interface.function_count);
                }
            }
        },
        Err(e) => {
            println!("✗ Failed to parse interface: {:?}", e);
        },
    }
    println!);

    // Example 3: Multiple definitions
    println!("Example 3: Parsing multiple definitions");
    let multiple_defs = r#"
        world world1 {
            export func1: func() -> u32
        }
        
        interface interface1 {
            test: func() -> bool
        }
        
        world world2 {
            import func2: func(x: string)
        }
    "#;

    match parse_wit_bounded(multiple_defs) {
        Ok(parser) => {
            println!(
                "✓ Parsed {} worlds and {} interfaces",
                parser.world_count(),
                parser.interface_count()
            ;

            for world in parser.worlds() {
                if let Ok(name) = world.name.as_str() {
                    println!("  World: '{}'", name);
                }
            }

            for interface in parser.interfaces() {
                if let Ok(name) = interface.name.as_str() {
                    println!("  Interface: '{}'", name);
                }
            }
        },
        Err(e) => {
            println!("✗ Failed to parse multiple definitions: {:?}", e);
        },
    }
    println!);

    // Example 4: Testing capacity limits
    println!("Example 4: Testing bounded capacity limits");
    let mut large_input = String::new);
    for i in 0..10 {
        large_input.push_str(&format!("world world{} {{}}\n", i;
    }

    match parse_wit_bounded(&large_input) {
        Ok(parser) => {
            println!(
                "✓ Parsed {} worlds (capacity limited to 4)",
                parser.world_count()
            ;
            assert!(parser.world_count() <= 4);

            for world in parser.worlds() {
                if let Ok(name) = world.name.as_str() {
                    println!("  World: '{}'", name);
                }
            }
        },
        Err(e) => {
            println!("✗ Failed to parse large input: {:?}", e);
        },
    }
    println!);

    // Example 5: Custom provider
    println!("Example 5: Using custom memory provider");
    type CustomProvider = NoStdProvider<2048>;
    let mut parser = BoundedWitParser::<CustomProvider>::new(CustomProvider::default())?;
    let custom_input = "world custom-world {}";

    match parser.parse(custom_input) {
        Ok(()) => {
            println!(
                "✓ Parsed with custom provider: {} worlds",
                parser.world_count()
            ;
        },
        Err(e) => {
            println!("✗ Failed with custom provider: {:?}", e);
        },
    }

    println!("\n=== Demo Complete ===\n");
    Ok(())
}
