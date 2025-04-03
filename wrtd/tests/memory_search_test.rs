//! Memory search test program
//! This test demonstrates using the memory search functionality.

use wrt::memory::Memory;
use wrt::module_instance::MemoryAddr;
use wrt::types::MemoryType;
use wrt::Module;

/// A simple memory search test
///
/// This test:
/// 1. Creates a memory instance
/// 2. Populates it with some sample data including a "Completed 5 iterations" string
/// 3. Demonstrates using the memory search functionality
#[test]
#[ignore = "Memory search functionality is not implemented in the engine yet"]
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
    let mut module = Module::new().unwrap();

    // Add memory export to the module (using the new API)
    {
        use std::sync::Arc;
        let memory_type = MemoryType { min: 1, max: None };
        let memory = Memory::new(memory_type);
        let mut memories = module.memories.write().unwrap();
        memories.push(Arc::new(memory));
    }

    // Add memory export
    module.add_memory_export("memory".to_string(), 0);

    println!("\nSkipping engine memory search test - functionality not implemented yet");
    println!("\nMemory search test complete!");
}
