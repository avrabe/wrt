// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly Component Model name section handling
//!
//! This module provides utilities for parsing and generating the WebAssembly
//! Component Model name section. The name section is a custom section that
//! provides debug information for components.

use wrt_format::binary;
#[cfg(not(feature = "std"))]
use wrt_foundation::traits::BoundedCapacity;

use crate::{prelude::*, Error, Result};

// Type aliases for generated data to avoid confusion
#[cfg(feature = "std")]
type GeneratedNameSectionData = std::vec::Vec<u8>;
#[cfg(not(feature = "std"))]
type GeneratedNameSectionData =
    wrt_foundation::BoundedVec<u8, 4096, wrt_foundation::safe_memory::NoStdProvider<4096>>;

/// WebAssembly Component Model name section subsection types
pub const COMPONENT_NAME_COMPONENT: u8 = 0;
pub const COMPONENT_NAME_SORT: u8 = 1;
pub const COMPONENT_NAME_IMPORT: u8 = 2;
pub const COMPONENT_NAME_EXPORT: u8 = 3;
pub const COMPONENT_NAME_CANONICAL: u8 = 4;
pub const COMPONENT_NAME_TYPE: u8 = 5;

/// Component name section subsection identifiers
pub enum ComponentNameSubsectionId {
    Module = 0,
    Function = 1,
    CoreFunction = 2,
    CoreTable = 3,
    CoreMemory = 4,
    CoreGlobal = 5,
    CoreType = 6,
    Type = 7,
    Component = 8,
    Instance = 9,
    CoreInstance = 10,
}

/// Core sort identifier for subsections
pub enum CoreSortIdentifier {
    Function = 0,
    Table = 1,
    Memory = 2,
    Global = 3,
    Type = 4,
}

/// Sort identifier for subsections
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortIdentifier {
    #[default]
    Module = 0,
    Function = 1,
    CoreFunction = 2,
    CoreTable = 3,
    CoreMemory = 4,
    CoreGlobal = 5,
    CoreType = 6,
    Type = 7,
    Component = 8,
    Instance = 9,
    CoreInstance = 10,
    Value = 11,
}

/// Entry in a name map
#[cfg(feature = "std")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameMapEntry {
    pub index: u32,
    pub name: String,
}

/// Entry in a name map (no_std version)
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameMapEntry {
    pub index: u32,
    pub name: &'static str,
}

// Implement required traits for NameMapEntry
impl wrt_foundation::traits::ToBytes for NameMapEntry {
    fn serialized_size(&self) -> usize {
        4 + self.name.len() + 1 // u32 + string + separator
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u32_le(self.index)?;
        #[cfg(feature = "std")]
        writer.write_all(self.name.as_bytes())?;
        #[cfg(not(feature = "std"))]
        writer.write_all(self.name.as_bytes())?;
        Ok(())
    }
}

impl wrt_foundation::traits::FromBytes for NameMapEntry {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let index = reader.read_u32_le()?;
        #[cfg(feature = "std")]
        let mut bytes = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let mut bytes: wrt_foundation::BoundedVec<
            u8,
            256,
            wrt_foundation::safe_memory::NoStdProvider<8192>,
        > = {
            use wrt_foundation::safe_memory::NoStdProvider;
            let provider = NoStdProvider::<8192>::default();
            wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
        };
        loop {
            match reader.read_u8() {
                #[cfg(feature = "std")]
                Ok(byte) => bytes.push(byte),
                #[cfg(not(feature = "std"))]
                Ok(byte) => {
                    let _ = bytes.push(byte);
                },
                Err(_) => break,
            }
        }
        #[cfg(feature = "std")]
        let name = std::string::String::from_utf8_lossy(&bytes).to_string();
        #[cfg(not(feature = "std"))]
        let name = ""; // Simplified for no_std
        Ok(NameMapEntry { index, name })
    }
}

impl wrt_foundation::traits::Checksummable for NameMapEntry {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&self.index.to_le_bytes());
        #[cfg(feature = "std")]
        checksum.update_slice(self.name.as_bytes());
        #[cfg(not(feature = "std"))]
        checksum.update_slice(self.name.as_bytes());
    }
}

// Implement required traits for SortIdentifier
impl wrt_foundation::traits::ToBytes for SortIdentifier {
    fn serialized_size(&self) -> usize {
        1 // enum as u8
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u8(*self as u8)?;
        Ok(())
    }
}

impl wrt_foundation::traits::FromBytes for SortIdentifier {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let value = reader.read_u8()?;
        match value {
            0 => Ok(SortIdentifier::Module),
            1 => Ok(SortIdentifier::Function),
            2 => Ok(SortIdentifier::CoreFunction),
            3 => Ok(SortIdentifier::CoreTable),
            4 => Ok(SortIdentifier::CoreMemory),
            5 => Ok(SortIdentifier::CoreGlobal),
            6 => Ok(SortIdentifier::CoreType),
            7 => Ok(SortIdentifier::Type),
            8 => Ok(SortIdentifier::Component),
            9 => Ok(SortIdentifier::Instance),
            10 => Ok(SortIdentifier::CoreInstance),
            11 => Ok(SortIdentifier::Value),
            _ => Ok(SortIdentifier::Module), // Default fallback
        }
    }
}

impl wrt_foundation::traits::Checksummable for SortIdentifier {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&[*self as u8]);
    }
}

/// Name map - maps indices to names
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameMap {
    #[cfg(feature = "std")]
    pub entries: std::vec::Vec<NameMapEntry>,
    #[cfg(not(feature = "std"))]
    pub entries: wrt_foundation::BoundedVec<NameMapEntry, 256, wrt_foundation::NoStdProvider<4096>>,
}

impl NameMap {
    pub fn new() -> Self {
        #[cfg(feature = "std")]
        let entries = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let entries = {
            if let Ok(provider) = crate::prelude::create_decoder_provider::<4096>() {
                wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
            } else {
                wrt_foundation::BoundedVec::default()
            }
        };

        Self { entries }
    }

    pub fn parse(data: &[u8], offset: usize) -> Result<(Self, usize)> {
        // Read count of entries using wrt-format's read_leb128_u32
        let (count, count_len) = binary::read_leb128_u32(data, offset)?;

        let mut current_offset = offset + count_len;
        #[cfg(feature = "std")]
        let mut entries = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let mut entries = {
            let provider = crate::prelude::create_decoder_provider::<4096>()
                .map_err(|_| Error::parse_error("Failed to create memory provider"))?;
            wrt_foundation::BoundedVec::new(provider)
                .map_err(|_| Error::parse_error("Failed to create entries vector"))?
        };

        for _ in 0..count {
            if current_offset >= data.len() {
                break;
            }

            // Parse index using wrt-format's read_leb128_u32
            let (index, index_len) = binary::read_leb128_u32(data, current_offset)?;
            current_offset += index_len;

            // Parse name
            if current_offset >= data.len() {
                return Err(Error::parse_error("Truncated name in name map"));
            }

            // Use wrt-format's read_string to parse the name
            let (name_bytes, name_len) = binary::read_string(data, current_offset)?;
            current_offset += name_len;

            #[cfg(feature = "std")]
            let name = std::string::String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
            #[cfg(not(feature = "std"))]
            let name = ""; // Simplified for no_std

            #[cfg(feature = "std")]
            entries.push(NameMapEntry { index, name });
            #[cfg(not(feature = "std"))]
            entries
                .push(NameMapEntry { index, name })
                .map_err(|_| Error::parse_error("Failed to push entry"))?;
        }

        Ok((Self { entries }, current_offset - offset))
    }
}

// Implement required traits for NameMap
impl wrt_foundation::traits::ToBytes for NameMap {
    fn serialized_size(&self) -> usize {
        4 + self.entries.iter().map(|entry| entry.serialized_size()).sum::<usize>()
        // u32 count + entries
    }

    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u32_le(self.entries.len() as u32)?;
        for entry in &self.entries {
            entry.to_bytes_with_provider(writer, provider)?;
        }
        Ok(())
    }
}

impl wrt_foundation::traits::FromBytes for NameMap {
    fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'a>,
        provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let count = reader.read_u32_le()?;
        #[cfg(feature = "std")]
        let mut entries = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let mut entries = {
            let provider = crate::prelude::create_decoder_provider::<4096>().map_err(|_| {
                wrt_foundation::traits::SerializationError::Custom(
                    "Failed to create memory provider",
                )
            })?;
            wrt_foundation::BoundedVec::new(provider).map_err(|_| {
                wrt_foundation::traits::SerializationError::Custom(
                    "Failed to create entries vector",
                )
            })?
        };
        for _ in 0..count {
            let entry = NameMapEntry::from_bytes_with_provider(reader, provider)?;
            #[cfg(feature = "std")]
            entries.push(entry);
            #[cfg(not(feature = "std"))]
            entries.push(entry).map_err(|_| {
                wrt_foundation::traits::SerializationError::Custom("Failed to push entry")
            })?;
        }
        Ok(NameMap { entries })
    }
}

impl wrt_foundation::traits::Checksummable for NameMap {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            entry.update_checksum(checksum);
        }
    }
}

/// A component name section structure
///
/// This struct represents the name section of a component,
/// which can be used to store names for various component entities.
#[derive(Debug, Clone, Default)]
pub struct ComponentNameSection {
    /// Name of the component itself
    #[cfg(feature = "std")]
    pub component_name: Option<std::string::String>,
    #[cfg(not(feature = "std"))]
    pub component_name:
        Option<wrt_foundation::BoundedString<256, wrt_foundation::NoStdProvider<4096>>>,
    /// Map of names for various sorted items (functions, instances, etc.)
    #[cfg(feature = "std")]
    pub sort_names: std::vec::Vec<(SortIdentifier, NameMap)>,
    #[cfg(not(feature = "std"))]
    pub sort_names: wrt_foundation::BoundedVec<
        (SortIdentifier, NameMap),
        64,
        wrt_foundation::NoStdProvider<4096>,
    >,
    /// Map of import names
    pub import_names: NameMap,
    /// Map of export names
    pub export_names: NameMap,
    /// Map of canonical names
    pub canonical_names: NameMap,
    /// Map of type names
    pub type_names: NameMap,
}

/// Parse a WebAssembly Component Model name section
pub fn parse_component_name_section(data: &[u8]) -> Result<ComponentNameSection> {
    let mut name_section = ComponentNameSection::default();
    let mut offset = 0;

    while offset < data.len() {
        if offset + 1 > data.len() {
            break; // End of data
        }

        // Read name type
        let name_type = data[offset];
        offset += 1;

        // Read subsection size
        let (subsection_size, bytes_read) = binary::read_leb128_u32(data, offset)?;
        offset += bytes_read;

        let subsection_start = offset;
        let subsection_end = subsection_start + subsection_size as usize;

        if subsection_end > data.len() {
            return Err(Error::parse_error(
                "Component name subsection size exceeds data size",
            ));
        }

        let subsection_data = &data[subsection_start..subsection_end];

        match name_type {
            COMPONENT_NAME_COMPONENT => {
                // Component name
                if !subsection_data.is_empty() {
                    let (name_bytes, _) = binary::read_string(subsection_data, 0)?;
                    #[cfg(feature = "std")]
                    {
                        let name =
                            std::string::String::from_utf8(name_bytes.to_vec()).unwrap_or_default();
                        name_section.component_name = Some(name);
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        let name = {
                            if let Ok(provider) = crate::prelude::create_decoder_provider::<4096>()
                            {
                                wrt_foundation::BoundedString::from_str(
                                    core::str::from_utf8(name_bytes).unwrap_or(""),
                                    provider,
                                )
                                .unwrap_or_default()
                            } else {
                                wrt_foundation::BoundedString::default()
                            }
                        };
                        name_section.component_name = Some(name);
                    }
                }
            },
            COMPONENT_NAME_SORT => {
                // Sort names
                if !subsection_data.is_empty() {
                    let mut pos = 0;
                    while pos < subsection_data.len() {
                        let (sort, sort_size) = parse_sort(subsection_data, pos)?;
                        pos += sort_size;

                        let (name_map, name_map_size) = parse_name_map(subsection_data, pos)?;
                        pos += name_map_size;

                        #[cfg(feature = "std")]
                        name_section.sort_names.push((sort, name_map));
                        #[cfg(not(feature = "std"))]
                        {
                            let _ = name_section.sort_names.push((sort, name_map));
                        }
                    }
                }
            },
            COMPONENT_NAME_IMPORT => {
                // Import names
                if !subsection_data.is_empty() {
                    let (name_map, _) = parse_name_map(subsection_data, 0)?;
                    name_section.import_names = name_map;
                }
            },
            COMPONENT_NAME_EXPORT => {
                // Export names
                if !subsection_data.is_empty() {
                    let (name_map, _) = parse_name_map(subsection_data, 0)?;
                    name_section.export_names = name_map;
                }
            },
            COMPONENT_NAME_CANONICAL => {
                // Canonical names
                if !subsection_data.is_empty() {
                    let (name_map, _) = parse_name_map(subsection_data, 0)?;
                    name_section.canonical_names = name_map;
                }
            },
            COMPONENT_NAME_TYPE => {
                // Type names
                if !subsection_data.is_empty() {
                    let (name_map, _) = parse_name_map(subsection_data, 0)?;
                    name_section.type_names = name_map;
                }
            },
            _ => {
                // Skip unknown subsection
                offset = subsection_end;
                continue;
            },
        }

        offset = subsection_end;
    }

    Ok(name_section)
}

fn parse_sort(bytes: &[u8], pos: usize) -> Result<(SortIdentifier, usize)> {
    if pos >= bytes.len() {
        return Err(Error::parse_error("Unexpected end of input"));
    }

    let sort_byte = bytes[pos];
    let sort = match sort_byte {
        0 => SortIdentifier::Module,
        1 => SortIdentifier::Function,
        2 => SortIdentifier::CoreFunction,
        3 => SortIdentifier::CoreTable,
        4 => SortIdentifier::CoreMemory,
        5 => SortIdentifier::CoreGlobal,
        6 => SortIdentifier::CoreType,
        7 => SortIdentifier::Type,
        8 => SortIdentifier::Component,
        9 => SortIdentifier::Instance,
        10 => SortIdentifier::CoreInstance,
        11 => SortIdentifier::Value,
        _ => {
            return Err(Error::parse_error("Invalid sort identifier"));
        },
    };

    Ok((sort, 1))
}

fn parse_name_map(data: &[u8], pos: usize) -> Result<(NameMap, usize)> {
    NameMap::parse(data, pos)
}

pub fn generate_component_name_section(
    name_section: &ComponentNameSection,
) -> Result<GeneratedNameSectionData> {
    #[cfg(feature = "std")]
    let mut data = std::vec::Vec::new();
    #[cfg(not(feature = "std"))]
    let mut data = {
        use wrt_foundation::safe_memory::NoStdProvider;
        let provider = NoStdProvider::<4096>::default();
        wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
    };

    // Component name
    if let Some(name) = &name_section.component_name {
        // Name type
        data.push(COMPONENT_NAME_COMPONENT);

        // Generate data for name
        #[cfg(feature = "std")]
        let mut subsection_data = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let mut subsection_data: wrt_foundation::BoundedVec<
            u8,
            4096,
            wrt_foundation::safe_memory::NoStdProvider<4096>,
        > = {
            use wrt_foundation::safe_memory::NoStdProvider;
            let provider = NoStdProvider::<4096>::default();
            wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
        };

        #[cfg(feature = "std")]
        {
            let name_bytes = write_string(name);
            subsection_data.extend_from_slice(&name_bytes);
        }
        #[cfg(not(feature = "std"))]
        {
            let name_str = name.as_str().unwrap_or("");
            let name_bytes = write_string(name_str);
            for i in 0..name_bytes.len() {
                if let Ok(byte) = name_bytes.get(i) {
                    let _ = subsection_data.push(byte);
                }
            }
        }

        // Add subsection size and data
        #[cfg(feature = "std")]
        {
            data.extend_from_slice(&write_leb128_u32(subsection_data.len() as u32));
            data.extend_from_slice(&subsection_data);
        }
        #[cfg(not(feature = "std"))]
        {
            let len_bytes = write_leb128_u32(subsection_data.len() as u32);
            for i in 0..len_bytes.len() {
                if let Ok(byte) = len_bytes.get(i) {
                    let _ = data.push(byte);
                }
            }
            for i in 0..subsection_data.len() {
                if let Ok(byte) = subsection_data.get(i) {
                    let _ = data.push(byte);
                }
            }
        }
    }

    // Sort names
    #[cfg(feature = "std")]
    let sort_names_empty = name_section.sort_names.is_empty();
    #[cfg(not(feature = "std"))]
    let sort_names_empty =
        wrt_foundation::traits::BoundedCapacity::is_empty(&name_section.sort_names);

    if !sort_names_empty {
        // Name type
        data.push(COMPONENT_NAME_SORT);

        // Generate data for sorts
        #[cfg(feature = "std")]
        let mut subsection_data = std::vec::Vec::new();
        #[cfg(not(feature = "std"))]
        let mut subsection_data: wrt_foundation::BoundedVec<
            u8,
            4096,
            wrt_foundation::safe_memory::NoStdProvider<4096>,
        > = {
            use wrt_foundation::safe_memory::NoStdProvider;
            let provider = NoStdProvider::<4096>::default();
            wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
        };

        for (sort, name_map) in &name_section.sort_names {
            let sort_bytes = generate_sort(&sort)?;
            #[cfg(feature = "std")]
            subsection_data.extend_from_slice(&sort_bytes);
            #[cfg(not(feature = "std"))]
            for i in 0..sort_bytes.len() {
                if let Ok(byte) = sort_bytes.get(i) {
                    let _ = subsection_data.push(byte);
                }
            }

            let name_map_bytes = generate_name_map(&name_map)?;
            #[cfg(feature = "std")]
            subsection_data.extend_from_slice(&name_map_bytes);
            #[cfg(not(feature = "std"))]
            for i in 0..name_map_bytes.len() {
                if let Ok(byte) = name_map_bytes.get(i) {
                    let _ = subsection_data.push(byte);
                }
            }
        }

        // Add subsection size and data
        #[cfg(feature = "std")]
        {
            data.extend_from_slice(&write_leb128_u32(subsection_data.len() as u32));
            data.extend_from_slice(&subsection_data);
        }
        #[cfg(not(feature = "std"))]
        {
            let len_bytes = write_leb128_u32(subsection_data.len() as u32);
            for i in 0..len_bytes.len() {
                if let Ok(byte) = len_bytes.get(i) {
                    let _ = data.push(byte);
                }
            }
            for i in 0..subsection_data.len() {
                if let Ok(byte) = subsection_data.get(i) {
                    let _ = data.push(byte);
                }
            }
        }
    }

    // Import names
    #[cfg(feature = "std")]
    let import_names_empty = name_section.import_names.entries.is_empty();
    #[cfg(not(feature = "std"))]
    let import_names_empty =
        wrt_foundation::traits::BoundedCapacity::is_empty(&name_section.import_names.entries);

    if !import_names_empty {
        // Name type
        data.push(COMPONENT_NAME_IMPORT);

        // Generate data for import names
        let subsection_data = generate_name_map(&name_section.import_names)?;

        // Add subsection size and data
        #[cfg(feature = "std")]
        {
            data.extend_from_slice(&write_leb128_u32(subsection_data.len() as u32));
            data.extend_from_slice(&subsection_data);
        }
        #[cfg(not(feature = "std"))]
        {
            let len_bytes = write_leb128_u32(subsection_data.len() as u32);
            for i in 0..len_bytes.len() {
                if let Ok(byte) = len_bytes.get(i) {
                    let _ = data.push(byte);
                }
            }
            for i in 0..subsection_data.len() {
                if let Ok(byte) = subsection_data.get(i) {
                    let _ = data.push(byte);
                }
            }
        }
    }

    // Export names
    #[cfg(feature = "std")]
    let export_names_empty = name_section.export_names.entries.is_empty();
    #[cfg(not(feature = "std"))]
    let export_names_empty =
        wrt_foundation::traits::BoundedCapacity::is_empty(&name_section.export_names.entries);

    if !export_names_empty {
        // Name type
        data.push(COMPONENT_NAME_EXPORT);

        // Generate data for export names
        let subsection_data = generate_name_map(&name_section.export_names)?;

        // Add subsection size and data
        #[cfg(feature = "std")]
        {
            data.extend_from_slice(&write_leb128_u32(subsection_data.len() as u32));
            data.extend_from_slice(&subsection_data);
        }
        #[cfg(not(feature = "std"))]
        {
            let len_bytes = write_leb128_u32(subsection_data.len() as u32);
            for i in 0..len_bytes.len() {
                if let Ok(byte) = len_bytes.get(i) {
                    let _ = data.push(byte);
                }
            }
            for i in 0..subsection_data.len() {
                if let Ok(byte) = subsection_data.get(i) {
                    let _ = data.push(byte);
                }
            }
        }
    }

    // Canonical names
    #[cfg(feature = "std")]
    let canonical_names_empty = name_section.canonical_names.entries.is_empty();
    #[cfg(not(feature = "std"))]
    let canonical_names_empty =
        wrt_foundation::traits::BoundedCapacity::is_empty(&name_section.canonical_names.entries);

    if !canonical_names_empty {
        // Name type
        data.push(COMPONENT_NAME_CANONICAL);

        // Generate data for canonical names
        let subsection_data = generate_name_map(&name_section.canonical_names)?;

        // Add subsection size and data
        #[cfg(feature = "std")]
        {
            data.extend_from_slice(&write_leb128_u32(subsection_data.len() as u32));
            data.extend_from_slice(&subsection_data);
        }
        #[cfg(not(feature = "std"))]
        {
            let len_bytes = write_leb128_u32(subsection_data.len() as u32);
            for i in 0..len_bytes.len() {
                if let Ok(byte) = len_bytes.get(i) {
                    let _ = data.push(byte);
                }
            }
            for i in 0..subsection_data.len() {
                if let Ok(byte) = subsection_data.get(i) {
                    let _ = data.push(byte);
                }
            }
        }
    }

    // Type names
    #[cfg(feature = "std")]
    let type_names_empty = name_section.type_names.entries.is_empty();
    #[cfg(not(feature = "std"))]
    let type_names_empty =
        wrt_foundation::traits::BoundedCapacity::is_empty(&name_section.type_names.entries);

    if !type_names_empty {
        // Name type
        data.push(COMPONENT_NAME_TYPE);

        // Generate data for type names
        let subsection_data = generate_name_map(&name_section.type_names)?;

        // Add subsection size and data
        #[cfg(feature = "std")]
        {
            data.extend_from_slice(&write_leb128_u32(subsection_data.len() as u32));
            data.extend_from_slice(&subsection_data);
        }
        #[cfg(not(feature = "std"))]
        {
            let len_bytes = write_leb128_u32(subsection_data.len() as u32);
            for i in 0..len_bytes.len() {
                if let Ok(byte) = len_bytes.get(i) {
                    let _ = data.push(byte);
                }
            }
            for i in 0..subsection_data.len() {
                if let Ok(byte) = subsection_data.get(i) {
                    let _ = data.push(byte);
                }
            }
        }
    }

    Ok(data)
}

#[cfg(feature = "std")]
fn generate_sort(sort: &SortIdentifier) -> Result<std::vec::Vec<u8>> {
    let mut data = std::vec::Vec::new();
    match sort {
        SortIdentifier::Module => data.push(0),
        SortIdentifier::Function => data.push(1),
        SortIdentifier::CoreFunction => data.push(2),
        SortIdentifier::CoreTable => data.push(3),
        SortIdentifier::CoreMemory => data.push(4),
        SortIdentifier::CoreGlobal => data.push(5),
        SortIdentifier::CoreType => data.push(6),
        SortIdentifier::Type => data.push(7),
        SortIdentifier::Component => data.push(8),
        SortIdentifier::Instance => data.push(9),
        SortIdentifier::CoreInstance => data.push(10),
        SortIdentifier::Value => data.push(11),
    }
    Ok(data)
}

#[cfg(not(feature = "std"))]
fn generate_sort(
    sort: &SortIdentifier,
) -> Result<wrt_foundation::BoundedVec<u8, 4096, wrt_foundation::safe_memory::NoStdProvider<4096>>>
{
    use wrt_foundation::safe_memory::NoStdProvider;
    let provider = NoStdProvider::<4096>::default();
    let mut data = wrt_foundation::BoundedVec::new(provider).map_err(|_| {
        Error::new(
            wrt_error::ErrorCategory::Memory,
            wrt_error::codes::MEMORY_ALLOCATION_FAILED,
            "Failed to create sort data buffer",
        )
    })?;

    let byte = match sort {
        SortIdentifier::Module => 0,
        SortIdentifier::Function => 1,
        SortIdentifier::CoreFunction => 2,
        SortIdentifier::CoreTable => 3,
        SortIdentifier::CoreMemory => 4,
        SortIdentifier::CoreGlobal => 5,
        SortIdentifier::CoreType => 6,
        SortIdentifier::Type => 7,
        SortIdentifier::Component => 8,
        SortIdentifier::Instance => 9,
        SortIdentifier::CoreInstance => 10,
        SortIdentifier::Value => 11,
    };
    data.push(byte).map_err(|_| {
        Error::new(
            wrt_error::ErrorCategory::Memory,
            wrt_error::codes::MEMORY_ALLOCATION_FAILED,
            "Failed to push sort byte",
        )
    })?;
    Ok(data)
}

#[cfg(feature = "std")]
fn generate_name_map(names: &NameMap) -> Result<std::vec::Vec<u8>> {
    let mut data = std::vec::Vec::new();

    // Number of entries
    data.extend_from_slice(&write_leb128_u32(names.entries.len() as u32));

    // Each entry
    for entry in &names.entries {
        // Index
        data.extend_from_slice(&write_leb128_u32(entry.index));

        // Name
        data.extend_from_slice(&write_string(&entry.name));
    }

    Ok(data)
}

#[cfg(not(feature = "std"))]
fn generate_name_map(
    names: &NameMap,
) -> Result<wrt_foundation::BoundedVec<u8, 4096, wrt_foundation::safe_memory::NoStdProvider<4096>>>
{
    use wrt_foundation::safe_memory::NoStdProvider;
    let provider = NoStdProvider::<4096>::default();
    let mut data = wrt_foundation::BoundedVec::new(provider).map_err(|_| {
        Error::new(
            wrt_error::ErrorCategory::Memory,
            wrt_error::codes::MEMORY_ALLOCATION_FAILED,
            "Failed to create name map data buffer",
        )
    })?;

    // Number of entries
    let len_bytes = write_leb128_u32(names.entries.len() as u32);
    for i in 0..len_bytes.len() {
        if let Ok(byte) = len_bytes.get(i) {
            data.push(byte).map_err(|_| {
                Error::new(
                    wrt_error::ErrorCategory::Memory,
                    wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                    "Failed to push length byte",
                )
            })?;
        }
    }

    // Each entry
    for entry in &names.entries {
        // Index
        let index_bytes = write_leb128_u32(entry.index);
        for i in 0..index_bytes.len() {
            if let Ok(byte) = index_bytes.get(i) {
                data.push(byte).map_err(|_| {
                    Error::new(
                        wrt_error::ErrorCategory::Memory,
                        wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                        "Failed to push index byte",
                    )
                })?;
            }
        }

        // Name
        #[cfg(feature = "std")]
        let name_str = &entry.name;
        #[cfg(not(feature = "std"))]
        let name_str = entry.name;
        let name_bytes = write_string(name_str);
        for i in 0..name_bytes.len() {
            if let Ok(byte) = name_bytes.get(i) {
                data.push(byte).map_err(|_| {
                    Error::new(
                        wrt_error::ErrorCategory::Memory,
                        wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                        "Failed to push name byte",
                    )
                })?;
            }
        }
    }

    Ok(data)
}

pub fn parse_error(_message: &str) -> Error {
    use wrt_error::{codes, ErrorCategory};
    Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Parse error")
}

pub fn parse_error_with_context(_message: &str, _context: &str) -> Error {
    use wrt_error::{codes, ErrorCategory};
    Error::new(
        ErrorCategory::Parse,
        codes::PARSE_ERROR,
        "Parse error with context",
    )
}

pub fn parse_error_with_position(_message: &str, _position: usize) -> Error {
    use wrt_error::{codes, ErrorCategory};
    Error::new(
        ErrorCategory::Parse,
        codes::PARSE_ERROR,
        "Parse error at position",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_component_name() {
        let mut name_section = ComponentNameSection::default();
        #[cfg(feature = "std")]
        {
            name_section.component_name = Some("test_component".to_string());
        }
        #[cfg(not(feature = "std"))]
        {
            if let Ok(provider) = crate::prelude::create_decoder_provider::<4096>() {
                name_section.component_name = Some(
                    wrt_foundation::BoundedString::from_str("test_component", provider)
                        .unwrap_or_default(),
                );
            }
        }

        let bytes = generate_component_name_section(&name_section).unwrap();
        let parsed = parse_component_name_section(&bytes).unwrap();

        #[cfg(feature = "std")]
        assert_eq!(parsed.component_name, Some("test_component".to_string()));
        #[cfg(not(feature = "std"))]
        {
            if let Some(ref name) = parsed.component_name {
                assert_eq!(name.as_str().unwrap_or(""), "test_component");
            }
        }
    }

    #[test]
    fn test_roundtrip_sort_names() {
        let mut name_section = ComponentNameSection::default();

        let mut name_map = NameMap::new();
        #[cfg(feature = "std")]
        {
            name_map.entries.push(NameMapEntry {
                index: 0,
                name: "func0".to_string(),
            });
            name_map.entries.push(NameMapEntry {
                index: 1,
                name: "func1".to_string(),
            });
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = name_map.entries.push(NameMapEntry {
                index: 0,
                name: "func0",
            });
            let _ = name_map.entries.push(NameMapEntry {
                index: 1,
                name: "func1",
            });
        }

        #[cfg(feature = "std")]
        name_section.sort_names.push((SortIdentifier::Function, name_map));
        #[cfg(not(feature = "std"))]
        {
            let _ = name_section.sort_names.push((SortIdentifier::Function, name_map));
        }

        let bytes = generate_component_name_section(&name_section).unwrap();
        let parsed = parse_component_name_section(&bytes).unwrap();

        assert_eq!(parsed.sort_names.len(), 1);
        assert!(matches!(parsed.sort_names[0].0, SortIdentifier::Function));
        assert_eq!(parsed.sort_names[0].1.entries.len(), 2);
        assert_eq!(parsed.sort_names[0].1.entries[0].index, 0);
        #[cfg(feature = "std")]
        {
            assert_eq!(parsed.sort_names[0].1.entries[0].name, "func0");
            assert_eq!(parsed.sort_names[0].1.entries[1].index, 1);
            assert_eq!(parsed.sort_names[0].1.entries[1].name, "func1");
        }
        #[cfg(not(feature = "std"))]
        {
            assert_eq!(parsed.sort_names[0].1.entries[0].name, "func0");
            assert_eq!(parsed.sort_names[0].1.entries[1].index, 1);
            assert_eq!(parsed.sort_names[0].1.entries[1].name, "func1");
        }
    }
}
