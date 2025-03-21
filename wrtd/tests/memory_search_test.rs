//! Memory search test program
//! This test demonstrates using the memory search functionality.

use wrt::execution::{Engine, MemoryAddr};
use wrt::memory::Memory;
use wrt::types::MemoryType;
use wrt::Module;
use wrt::{Export, ExportKind, Result};

/// A simple memory search test
///
/// This test:
/// 1. Creates a memory instance
/// 2. Populates it with some sample data including a "Completed 5 iterations" string
/// 3. Demonstrates using the memory search functionality
#[test]
fn test_memory_search() {
    // Create a memory instance with default settings
    let memory = Memory::new(MemoryType { min: 1, max: None });

    // Create a memory address
    let _mem_addr = MemoryAddr {
        instance_idx: 0,
        memory_idx: 0,
    };

    // Testing direct memory search (using Memory::search_memory)
    // First, write some example data to our memory
    let mut test_memory = memory.clone();
    test_memory
        .write_bytes(0, "Hello, world!".as_bytes())
        .unwrap();
    test_memory
        .write_bytes(100, "Completed 5 iterations".as_bytes())
        .unwrap();

    // Search for patterns in memory
    println!("Searching memory for pattern 'Completed':");
    let results = test_memory.search_memory("Completed", false);
    assert!(!results.is_empty());
    for (addr, snippet) in &results {
        println!("Found at 0x{:08x}: {}", addr, snippet);
    }

    // Create a module for Engine tests
    let mut module = Module::new();

    // Add memory export to the module
    module.memories.push(MemoryType { min: 1, max: None });
    module.exports.push(Export {
        name: "memory".to_string(),
        kind: ExportKind::Memory,
        index: 0,
    });

    // Create our own helper function to test memory searching using Engine
    fn test_engine_memory_search(module: Module, _memory: Memory) -> Result<()> {
        // Initialize the engine and module
        let mut engine = Engine::new(module.clone());

        // Instantiate the module
        engine.instantiate(module)?;

        // Get memory address
        let instance = engine.get_instance(0).unwrap();
        let mem_addr = &instance.memory_addrs[0];

        // Test memory search
        println!("\nTesting engine's memory search capability:");
        engine.search_memory_for_pattern(mem_addr, "Completed", false)?;

        // Also try to search for iterations
        println!("\nSearching for 'iterations':");
        engine.search_memory_for_pattern(mem_addr, "iterations", false)?;

        // Test ASCII-only search
        println!("\nASCII-only search:");
        engine.search_memory_for_pattern(mem_addr, "Hello", true)?;

        Ok(())
    }

    // Run our helper test function
    test_engine_memory_search(module, test_memory).unwrap();

    println!("\nMemory search test complete!");
}
