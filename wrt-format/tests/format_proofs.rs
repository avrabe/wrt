//! Integration tests for the wrt-format crate.
//!
//! This module contains tests for the format module functionality.

use wrt_format::{
    create_state_section, extract_state_section, CompressionType, CustomSection, Module,
    StateSection,
};

/// Test basic serialization properties of the format module
#[test]
fn test_basic_serialization() {
    // Create a simple module
    let mut module = Module::new();

    // Verify initial state
    assert!(module.custom_sections.is_empty());
    assert!(!module.has_state_sections());

    // Create a non-state custom section
    let section1 = CustomSection { name: "test-section".to_string(), data: vec![1, 2, 3, 4] };

    module.add_custom_section(section1);

    // Still not a state module
    assert!(!module.has_state_sections());

    // Create a state section
    let test_data = vec![5, 6, 7, 8];
    let state_section =
        create_state_section(StateSection::Stack, &test_data, CompressionType::None).unwrap();

    module.add_custom_section(state_section);

    // Now it's a state module
    assert!(module.has_state_sections());

    // Find the state section
    let found = module.find_custom_section(&StateSection::Stack.name());
    assert!(found.is_some());

    // Extract the state section data
    let (header, data) = extract_state_section(found.unwrap()).unwrap();

    // Verify section data
    assert_eq!(header.section_type, StateSection::Stack);
    assert_eq!(data, test_data);
}

/// Test that multiple state sections can be created and extracted
#[test]
fn test_state_section_format() {
    // Create state sections - only use None compression to avoid RLE issues
    let test_data = vec![1, 2, 3, 4, 5];

    // First state section
    let section1 =
        create_state_section(StateSection::Globals, &test_data, CompressionType::None).unwrap();

    // Second state section
    let section2 =
        create_state_section(StateSection::Memory, &test_data, CompressionType::None).unwrap();

    // Verify section names
    assert_eq!(section1.name, StateSection::Globals.name());
    assert_eq!(section2.name, StateSection::Memory.name());

    // Extract and verify data
    let (header1, data1) = extract_state_section(&section1).unwrap();
    let (header2, data2) = extract_state_section(&section2).unwrap();

    // Verify extracted data
    assert_eq!(header1.section_type, StateSection::Globals);
    assert_eq!(header1.compression_type, CompressionType::None);
    assert_eq!(data1, test_data);

    assert_eq!(header2.section_type, StateSection::Memory);
    assert_eq!(header2.compression_type, CompressionType::None);
    assert_eq!(data2, test_data);
}
