use wrt::memory::Memory;
use wrt::types::MemoryType;
use wrt::Result;

#[test]
fn test_memory_operations() -> Result<()> {
    // Create a memory type with 1 page (64 KiB) and a maximum of 2 pages
    let memory_type = MemoryType { min: 1, max: Some(2) };

    // Create a new memory instance with the specified type
    let memory = Memory::new(memory_type)?;

    // Verify initial memory state
    assert_eq!(memory.size(), 1, "Initial memory size should be 1 page");

    // Write values to memory
    memory.write_byte(100, 42)?;
    memory.write_u32(200, 0x12345678)?;

    // Read values back and verify they match
    assert_eq!(memory.read_byte(100)?, 42, "Read value doesn't match written value");
    assert_eq!(memory.read_u32(200)?, 0x12345678, "Read u32 doesn't match written value");

    // Test memory growth
    let old_size = memory.grow(1)?;
    assert_eq!(old_size, 1, "Old size should be 1 page");
    assert_eq!(memory.size(), 2, "New size should be 2 pages");

    // Verify memory access after growth still works
    assert_eq!(memory.read_byte(100)?, 42, "Memory content changed after growth");

    // Test attempting to grow beyond max
    assert!(memory.grow(1).is_err(), "Should not be able to grow beyond max");

    Ok(())
}
