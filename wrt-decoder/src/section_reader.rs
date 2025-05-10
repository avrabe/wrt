//! Section reader for WebAssembly modules
//!
//! This module provides a reader for WebAssembly module sections. It allows
//! identifying and extracting section data without parsing the entire module.

use wrt_error::Result;
use wrt_format::binary;
use wrt_format::section::{
    CODE_ID, CUSTOM_ID, DATA_COUNT_ID, DATA_ID, ELEMENT_ID, EXPORT_ID, FUNCTION_ID, GLOBAL_ID,
    IMPORT_ID, MEMORY_ID, START_ID, TABLE_ID, TYPE_ID,
};
// Deprecated, use From trait implementation instead
// use wrt_types::error_convert::convert_to_wrt_error;
use crate::prelude::String;
use wrt_types::ToString;

use crate::section_error::{self};

/// Represents a section payload in a WebAssembly module
#[derive(Debug)]
pub enum SectionPayload<'a> {
    /// Custom section
    Custom {
        /// Name of the custom section
        name: String,
        /// Data of the custom section
        data: &'a [u8],
    },
    /// Type section
    Type(&'a [u8]),
    /// Import section
    Import(&'a [u8]),
    /// Function section
    Function(&'a [u8]),
    /// Table section
    Table(&'a [u8]),
    /// Memory section
    Memory(&'a [u8]),
    /// Global section
    Global(&'a [u8]),
    /// Export section
    Export(&'a [u8]),
    /// Start section
    Start(&'a [u8]),
    /// Element section
    Element(&'a [u8]),
    /// Code section
    Code(&'a [u8]),
    /// Data section
    Data(&'a [u8]),
    /// Data count section
    DataCount(&'a [u8]),
    /// Unknown section
    Unknown {
        /// Section ID
        id: u8,
        /// Section data
        data: &'a [u8],
    },
}

/// Reader for accessing WebAssembly module sections
#[derive(Debug)]
pub struct SectionReader<'a> {
    /// The WebAssembly binary data
    binary: &'a [u8],
    /// Current offset in the binary
    current_offset: usize,
}

impl<'a> SectionReader<'a> {
    /// Create a new section reader for a WebAssembly binary
    ///
    /// Verifies the WebAssembly header, then positions at the first section.
    pub fn new(binary: &'a [u8]) -> Result<Self> {
        // Verify the binary has at least a header
        if binary.len() < 8 {
            return Err(section_error::unexpected_end(0, 8, binary.len()));
        }

        // Verify magic bytes
        let mut actual_magic = [0u8; 4];
        actual_magic.copy_from_slice(&binary[0..4]);
        if actual_magic != binary::WASM_MAGIC {
            return Err(section_error::invalid_magic(
                0,
                binary::WASM_MAGIC,
                actual_magic,
            ));
        }

        // Verify version
        let mut actual_version = [0u8; 4];
        actual_version.copy_from_slice(&binary[4..8]);
        if actual_version != binary::WASM_VERSION {
            return Err(section_error::unsupported_version(
                4,
                binary::WASM_VERSION,
                actual_version,
            ));
        }

        // Start after the header
        Ok(Self {
            binary,
            current_offset: 8,
        })
    }

    /// Reset the reader position to the beginning of sections (after header)
    pub fn reset(&mut self) {
        self.current_offset = 8; // Skip magic + version
    }

    /// Find the next section of the specified type
    ///
    /// Returns the section offset and size if found, or None if no matching section
    /// is found or the end of the module is reached. The offset points to the
    /// beginning of the section content (after the section header).
    ///
    /// This function starts searching from the current position and continues
    /// until it finds a matching section or reaches the end of the module.
    pub fn find_section(&mut self, section_id: u8) -> Result<Option<(usize, usize)>> {
        // Scan through sections from current position
        while self.current_offset < self.binary.len() {
            // Read section ID
            let id = self.binary[self.current_offset];
            self.current_offset += 1;

            // Skip this section if there's not enough bytes to read the size
            if self.current_offset >= self.binary.len() {
                break;
            }

            // Read section size using LEB128 encoding
            let (section_size, bytes_read) =
                binary::read_leb128_u32(self.binary, self.current_offset)?;
            self.current_offset += bytes_read;

            // Skip this section if there's not enough bytes for the content
            if self.current_offset + section_size as usize > self.binary.len() {
                return Err(section_error::section_size_exceeds_module(
                    id,
                    section_size,
                    self.binary.len() - self.current_offset,
                    self.current_offset,
                ));
            }

            // If this is the section we're looking for, return it
            if id == section_id {
                let content_offset = self.current_offset;
                let content_size = section_size as usize;

                // Advance past this section
                self.current_offset += content_size;

                return Ok(Some((content_offset, content_size)));
            }

            // Skip to next section
            self.current_offset += section_size as usize;
        }

        // No matching section found
        Ok(None)
    }

    /// Get the next section regardless of type
    ///
    /// Returns the section ID, offset, and size if found, or None if the end
    /// of the module is reached.
    pub fn next_section(&mut self) -> Result<Option<(u8, usize, usize)>> {
        if self.current_offset >= self.binary.len() {
            return Ok(None);
        }

        // Read section ID
        let id = self.binary[self.current_offset];
        self.current_offset += 1;

        // Skip this section if there's not enough bytes to read the size
        if self.current_offset >= self.binary.len() {
            return Ok(None);
        }

        // Read section size using LEB128 encoding
        let (section_size, bytes_read) = binary::read_leb128_u32(self.binary, self.current_offset)?;
        self.current_offset += bytes_read;

        // Skip this section if there's not enough bytes for the content
        if self.current_offset + section_size as usize > self.binary.len() {
            return Err(section_error::section_size_exceeds_module(
                id,
                section_size,
                self.binary.len() - self.current_offset,
                self.current_offset,
            ));
        }

        let content_offset = self.current_offset;
        let content_size = section_size as usize;

        // Advance past this section
        self.current_offset += content_size;

        Ok(Some((id, content_offset, content_size)))
    }

    /// Get the next section as a SectionPayload
    ///
    /// This provides a more structured view of the section data based on its type.
    pub fn next_payload(&mut self) -> Result<Option<SectionPayload<'a>>> {
        match self.next_section()? {
            Some((id, offset, size)) => {
                // Get a slice for this section's data
                let data = &self.binary[offset..offset + size];

                // Parse the section based on its ID
                match id {
                    CUSTOM_ID => {
                        // For custom sections, extract the name
                        let (name, bytes_read) = binary::read_string(data, 0)?;
                        let data = &data[bytes_read..];
                        Ok(Some(SectionPayload::Custom {
                            name: name.to_string(),
                            data,
                        }))
                    }
                    TYPE_ID => Ok(Some(SectionPayload::Type(data))),
                    IMPORT_ID => Ok(Some(SectionPayload::Import(data))),
                    FUNCTION_ID => Ok(Some(SectionPayload::Function(data))),
                    TABLE_ID => Ok(Some(SectionPayload::Table(data))),
                    MEMORY_ID => Ok(Some(SectionPayload::Memory(data))),
                    GLOBAL_ID => Ok(Some(SectionPayload::Global(data))),
                    EXPORT_ID => Ok(Some(SectionPayload::Export(data))),
                    START_ID => Ok(Some(SectionPayload::Start(data))),
                    ELEMENT_ID => Ok(Some(SectionPayload::Element(data))),
                    CODE_ID => Ok(Some(SectionPayload::Code(data))),
                    DATA_ID => Ok(Some(SectionPayload::Data(data))),
                    DATA_COUNT_ID => Ok(Some(SectionPayload::DataCount(data))),
                    _ => Ok(Some(SectionPayload::Unknown { id, data })),
                }
            }
            None => Ok(None),
        }
    }

    /// Find a custom section with the specified name
    ///
    /// Returns the section content offset and size if found, or None if no matching
    /// custom section is found. The offset points to the beginning of the section
    /// content (after the name).
    ///
    /// This function searches from the beginning of the module.
    pub fn find_custom_section(&mut self, name: &str) -> Result<Option<(usize, usize)>> {
        // Save current position to restore later
        let saved_offset = self.current_offset;

        // Reset to start scanning from the beginning
        self.reset();

        // Track if we found the section
        let mut result = None;

        // Scan through sections
        while self.current_offset < self.binary.len() {
            // Read section ID
            let id = self.binary[self.current_offset];
            self.current_offset += 1;

            // Stop if there's not enough bytes to read the size
            if self.current_offset >= self.binary.len() {
                break;
            }

            // Read section size using LEB128 encoding
            let (section_size, bytes_read) =
                match binary::read_leb128_u32(self.binary, self.current_offset) {
                    Ok(result) => result,
                    Err(e) => {
                        // Restore original position
                        self.current_offset = saved_offset;
                        return Err(e);
                    }
                };
            self.current_offset += bytes_read;

            let section_start = self.current_offset;

            // If this is a custom section, check the name
            if id == CUSTOM_ID && section_size > 0 {
                // Read the custom section name
                let (section_name, name_size) =
                    match binary::read_string(self.binary, section_start) {
                        Ok(result) => result,
                        Err(e) => {
                            // Restore original position
                            self.current_offset = saved_offset;
                            return Err(e);
                        }
                    };

                // If the name matches, we found it
                if section_name == name {
                    let content_start = section_start + name_size;
                    let content_size = section_size as usize - name_size;
                    result = Some((content_start, content_size));
                    break;
                }
            }

            // Skip to next section
            self.current_offset += section_size as usize;
        }

        // Restore original position
        self.current_offset = saved_offset;

        Ok(result)
    }
}

/// Find an import section in a WebAssembly binary
///
/// Returns the section offset and size if found.
/// The offset points to the beginning of the section content (after the section header).
pub fn find_import_section(binary: &[u8]) -> Result<Option<(usize, usize)>> {
    let mut reader = SectionReader::new(binary)?;
    reader.find_section(IMPORT_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_format::section::{CUSTOM_ID, TABLE_ID};

    /// Create a simple test module with a custom section
    fn create_test_module() -> Vec<u8> {
        let mut module = Vec::new();

        // Magic and version
        module.extend_from_slice(&binary::WASM_MAGIC);
        module.extend_from_slice(&binary::WASM_VERSION);

        // Custom section (ID=0)
        module.push(CUSTOM_ID); // section ID = 0

        // Prepare the name as a length-prefixed string
        let name = "test";
        let mut name_bytes = Vec::new();
        name_bytes.push(name.len() as u8); // name length = 4
        name_bytes.extend_from_slice(name.as_bytes()); // name = "test"

        // Prepare the content
        let content = b"test data"; // content = "test data"

        // Calculate total section size: name bytes + content bytes
        let section_size = name_bytes.len() + content.len();

        module.push(section_size as u8); // section size
        module.extend_from_slice(&name_bytes); // name with length prefix
        module.extend_from_slice(content); // section data

        // Print the created module for debugging
        println!("Created test module with {} bytes:", module.len());
        for (i, &byte) in module.iter().enumerate() {
            print!("{:02x} ", byte);
            if (i + 1) % 16 == 0 || i == module.len() - 1 {
                println!();
            }
        }

        module
    }

    #[test]
    fn test_section_reader_new() {
        // Valid module
        let valid_module = create_test_module();
        let reader = SectionReader::new(&valid_module);
        assert!(reader.is_ok());

        // Invalid magic
        let mut invalid_magic = valid_module.clone();
        invalid_magic[0] = 0xFF;
        let reader = SectionReader::new(&invalid_magic);
        assert!(reader.is_err());
        let err_msg = format!("{}", reader.err().unwrap());
        assert!(err_msg.contains("Invalid WebAssembly magic bytes"));

        // Invalid version
        let mut invalid_version = valid_module.clone();
        invalid_version[4] = 0xFF;
        let reader = SectionReader::new(&invalid_version);
        assert!(reader.is_err());
        let err_msg = format!("{}", reader.err().unwrap());
        assert!(err_msg.contains("Unsupported WebAssembly version"));

        // Too short
        let reader = SectionReader::new(&[0, 1, 2]);
        assert!(reader.is_err());
        let err_msg = format!("{}", reader.err().unwrap());
        assert!(err_msg.contains("expected 8 bytes, but only 3 available"));
    }

    #[test]
    fn test_find_section() {
        let module = create_test_module();
        let mut reader = SectionReader::new(&module).unwrap();

        // Find custom section
        let custom_section = reader.find_section(CUSTOM_ID).unwrap();
        assert!(custom_section.is_some());
        let (_offset, size) = custom_section.unwrap();
        // Size should be 5 (name) + 9 (content) = 14 bytes
        assert_eq!(size, 14);

        // Should be at end now, no more sections
        let no_section = reader.find_section(TABLE_ID).unwrap();
        assert!(no_section.is_none());

        // Reset and find again
        reader.reset();
        let custom_section = reader.find_section(CUSTOM_ID).unwrap();
        assert!(custom_section.is_some());
    }

    #[test]
    fn test_next_section() {
        let module = create_test_module();
        let mut reader = SectionReader::new(&module).unwrap();

        // Get first section (custom)
        let section = reader.next_section().unwrap();
        assert!(section.is_some());
        let (id, _offset, size) = section.unwrap();
        assert_eq!(id, CUSTOM_ID);
        // Size should be 5 (name) + 9 (content) = 14 bytes
        assert_eq!(size, 14);

        // Should be at end now
        let no_section = reader.next_section().unwrap();
        assert!(no_section.is_none());
    }

    #[test]
    fn test_next_payload() {
        let module = create_test_module();
        let mut reader = SectionReader::new(&module).unwrap();

        // Get section payload (custom)
        let payload = reader.next_payload().unwrap();
        assert!(payload.is_some());
        match payload.unwrap() {
            SectionPayload::Custom { name, data } => {
                assert_eq!(name, "test");
                assert_eq!(data, b"test data");
            }
            _ => panic!("Expected Custom section"),
        }

        // Should be at end now
        let no_payload = reader.next_payload().unwrap();
        assert!(no_payload.is_none());
    }

    #[test]
    fn test_find_custom_section() {
        let module = create_test_module();

        // Print module contents
        println!("Module buffer length: {}", module.len());
        println!("Module contents:");
        for (i, &byte) in module.iter().enumerate() {
            print!("{:02x} ", byte);
            if (i + 1) % 16 == 0 || i == module.len() - 1 {
                println!();
            }
        }

        let mut reader = SectionReader::new(&module).unwrap();

        // Examine all sections in the module
        println!("Examining all sections in sequence:");
        reader.reset();
        while let Ok(Some((id, offset, size))) = reader.next_section() {
            println!(
                "Section ID: 0x{:02x}, offset: {}, size: {}",
                id, offset, size
            );
            if id == CUSTOM_ID {
                // Try to read the custom section name
                let section_data = &module[offset..offset + size];
                if !section_data.is_empty() {
                    if let Ok((name, name_size)) = binary::read_string(section_data, 0) {
                        println!(
                            "  Custom section name: '{}', name size: {}",
                            name, name_size
                        );
                        println!(
                            "  Data: {:?}",
                            &section_data[name_size.min(section_data.len())..]
                        );
                    } else {
                        println!("  Failed to read custom section name");
                    }
                }
            }
        }

        // Find a custom section that exists
        reader.reset();
        let custom_section = reader.find_custom_section("test").unwrap();
        assert!(custom_section.is_some());
        let (_offset, size) = custom_section.unwrap();
        assert_eq!(size, 9); // "test data" length

        // Create a fresh reader for the nonexistent search
        let mut reader2 = SectionReader::new(&module).unwrap();

        // Find a custom section that doesn't exist
        let no_custom_section = reader2.find_custom_section("nonexistent").unwrap();
        assert!(no_custom_section.is_none());
    }

    #[test]
    fn test_malformed_module() {
        let mut module = create_test_module();

        // Corrupt the custom section by making its size impossibly large
        // The custom section starts after the magic bytes and version (8 bytes)
        // The custom section size is at index 9
        module[9] = 0xFF; // Modify the size byte of the custom section

        let mut reader = SectionReader::new(&module).unwrap();

        // Should fail due to impossibly large size
        let custom_section = reader.next_section();
        assert!(custom_section.is_err());
        let error_msg = format!("{}", custom_section.err().unwrap());
        assert!(error_msg.contains("Section size exceeds module size"));
    }
}
