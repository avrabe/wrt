//! Utilities for working with custom sections, particularly state sections.

use crate::prelude::*;
use wrt_format::{
    create_state_section as format_create_state_section,
    extract_state_section as format_extract_state_section, CompressionType, CustomSection,
    StateSection,
};
use wrt_types::bounded::BoundedVec;
// Ensure wrt_error items are in scope, typically via crate::prelude or direct use
use wrt_error::{codes, Error, ErrorCategory, Result};

/// Placeholder for the maximum expected size of a state section.
/// TODO: Determine the appropriate maximum size for state section data.
const MAX_STATE_SECTION_SIZE: usize = 65536; // 64KiB placeholder

/// Create a state section for serializing engine state.
///
/// This function creates a custom section for serializing engine state,
/// using the appropriate section format and optional compression.
/// It wraps the functionality from `wrt-format`.
///
/// # Arguments
///
/// * `section_type` - The type of state section to create.
/// * `data` - The data to include in the section.
/// * `use_compression` - Whether to compress the data.
///
/// # Returns
///
/// A `Result` containing the `wrt_format::CustomSection`.
pub fn create_engine_state_section(
    section_type: StateSection,
    data: &[u8],
    use_compression: bool,
) -> Result<CustomSection> {
    let compression = if use_compression {
        CompressionType::RLE
    } else {
        CompressionType::None
    };
    format_create_state_section(section_type, data, compression)
}

/// Extracts and validates data from a state-related custom section.
///
/// This function checks if the provided `CustomSection` matches the
/// `expected_section_type` by name, then extracts the data using
/// `wrt-format`'s utility, and converts it to a `BoundedVec`.
///
/// # Arguments
///
/// * `custom_section` - The custom section to process.
/// * `expected_section_type` - The `StateSection` enum variant identifying the expected type.
///
/// # Returns
///
/// A `Result` containing the extracted data as a `BoundedVec<u8>`, or an error.
pub fn get_data_from_state_section(
    custom_section: &CustomSection,
    expected_section_type: StateSection,
) -> Result<BoundedVec<u8, MAX_STATE_SECTION_SIZE>> {
    if custom_section.name != expected_section_type.name() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_INVALID_CUSTOM_SECTION_NAME,
            format!(
                "Expected state section '{}', but found '{}'",
                expected_section_type.name(),
                custom_section.name
            ),
        ));
    }

    let (_compression_type, raw_data) = format_extract_state_section(custom_section)?;

    // Check if raw_data exceeds MAX_STATE_SECTION_SIZE before attempting to create BoundedVec
    if raw_data.len() > MAX_STATE_SECTION_SIZE {
        return Err(Error::new(
            ErrorCategory::Capacity,
            codes::CAPACITY_EXCEEDED,
            format!(
                "State section data ({} bytes) exceeds maximum allowed capacity ({} bytes)",
                raw_data.len(),
                MAX_STATE_SECTION_SIZE
            ),
        ));
    }

    let mut bounded_data: BoundedVec<u8, MAX_STATE_SECTION_SIZE> = BoundedVec::new();

    for byte_val in raw_data.iter() {
        bounded_data.push(*byte_val).map_err(|capacity_error| {
            // This should ideally not be reached if the raw_data.len() check above is correct
            // and MAX_STATE_SECTION_SIZE is the actual const capacity of BoundedVec.
            // Mapping CapacityError to our standard Error type.
            Error::new(
                ErrorCategory::Capacity,
                codes::CAPACITY_EXCEEDED,
                format!(
                    "Capacity error while pushing to BoundedVec (size {}, capacity {}): {}",
                    raw_data.len(),
                    MAX_STATE_SECTION_SIZE,
                    capacity_error // Display for CapacityError is "Capacity limit exceeded"
                ),
            )
        })?;
    }

    Ok(bounded_data)
} 