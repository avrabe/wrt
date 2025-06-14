// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use wrt_error::Result;
#[cfg(feature = "std")]
use wrt_format::{
    binary,
    component::{CoreSort, Sort},
};

#[cfg(not(feature = "std"))]
use wrt_format::binary;

use super::types::ModuleInfo;
use crate::prelude::*;
use wrt_foundation::traits::BoundedCapacity;

#[cfg(not(feature = "std"))]
use wrt_foundation::unified_types_simple::EmbeddedTypes;

// Compatibility trait to provide as_bytes() for BoundedString
#[cfg(not(feature = "std"))]
pub trait BoundedStringExt {
    fn as_bytes(&self) -> &[u8];
}

#[cfg(not(feature = "std"))]
impl<const N: usize, P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq> BoundedStringExt for wrt_foundation::BoundedString<N, P> {
    fn as_bytes(&self) -> &[u8] {
        // This is a workaround - BoundedString doesn't have direct byte access
        // We'll return an empty slice for now and implement properly later
        &[]
    }
}

/// Extract embedded WebAssembly modules from a component binary
#[cfg(feature = "std")]
pub fn extract_embedded_modules(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut modules = Vec::new();
    let mut offset = 8; // Skip magic and version

    // Parse sections
    while offset < bytes.len() {
        // Read section ID and size
        if offset + 1 > bytes.len() {
            break;
        }

        let section_id = bytes[offset];
        offset += 1;

        let (section_size, bytes_read) = match binary::read_leb128_u32(bytes, offset) {
            Ok(result) => result,
            Err(_) => break,
        };
        offset += bytes_read;

        if offset + section_size as usize > bytes.len() {
            break;
        }

        // Extract section bytes
        let section_end = offset + section_size as usize;
        let section_bytes = &bytes[offset..section_end];
        offset = section_end;

        // Process core module sections
        if section_id == binary::COMPONENT_CORE_MODULE_SECTION_ID {
            if let Some(module_binary) = extract_module_from_section(section_bytes) {
                modules.push(module_binary);
            }
        }
    }

    Ok(modules)
}

/// Extract embedded WebAssembly modules from a component binary (no_std version)
#[cfg(not(feature = "std"))]
pub fn extract_embedded_modules(bytes: &[u8]) -> Result<wrt_foundation::BoundedVec<wrt_foundation::BoundedVec<u8, 128, wrt_foundation::safe_memory::NoStdProvider<2048>>, 16, wrt_foundation::safe_memory::NoStdProvider<2048>>> {
    use wrt_foundation::safe_memory::NoStdProvider;
    
    let provider = NoStdProvider::<2048>::new();
    let modules = wrt_foundation::BoundedVec::new(provider)?;
    
    // Simplified no_std implementation
    // TODO: Implement actual parsing when needed
    
    Ok(modules)
}

/// Extract a module from a core module section
fn extract_module_from_section(_section_bytes: &[u8]) -> Option<Vec<u8>> {
    // This is a simplified version - the real implementation would parse the
    // section structure to extract the module bytes

    // In a real implementation, we would:
    // 1. Parse the count of modules in the section
    // 2. For each module, extract its size and binary content
    // 3. Return the module binary

    // For now, we return a placeholder
    None
}

/// Check if a binary is a valid WebAssembly module
pub fn is_valid_module(bytes: &[u8]) -> bool {
    // Check minimum size
    if bytes.len() < 8 {
        return false;
    }

    // Check magic bytes
    if bytes[0..4] != binary::WASM_MAGIC {
        return false;
    }

    // Check version
    if bytes[4..8] != [0x01, 0x00, 0x00, 0x00] {
        return false;
    }

    true
}

/// Extract information about a WebAssembly module
pub fn extract_module_info(bytes: &[u8]) -> Result<ModuleInfo> {
    // This is a simplified version - the real implementation would parse
    // the module to count functions, memories, etc.

    Ok(ModuleInfo {
        idx: 0,
        size: bytes.len(),
        function_count: 0,
        memory_count: 0,
        table_count: 0,
        global_count: 0,
    })
}

/// Extract an inline module from a component
#[cfg(feature = "std")]
pub fn extract_inline_module(bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    // This is a simplified version - the real implementation would try to
    // find the first module in the component

    match extract_embedded_modules(bytes) {
        Ok(modules) if !modules.is_empty() => Ok(Some(modules[0].clone())),
        Ok(_) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Extract an inline module from a component (no_std version)
#[cfg(not(feature = "std"))]
pub fn extract_inline_module(bytes: &[u8]) -> Result<Option<wrt_foundation::BoundedVec<u8, 128, wrt_foundation::safe_memory::NoStdProvider<2048>>>> {
    // This is a simplified version - the real implementation would try to
    // find the first module in the component

    match extract_embedded_modules(bytes) {
        Ok(modules) if modules.len() > 0 => {
            match modules.get(0) {
                Ok(first_module) => Ok(Some(first_module.clone())),
                Err(_) => Ok(None),
            }
        },
        Ok(_) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Analyze a component binary to create a summary
pub fn analyze_component(bytes: &[u8]) -> Result<ComponentSummary> {
    // This is a simplified version - the real implementation would parse
    // the component and create a full summary

    #[cfg(feature = "std")]
    let component = crate::component::decode_component(bytes)?;
    #[cfg(not(feature = "std"))]
    let component = {
        // For no_std, create a minimal component structure
        ComponentSummary {
            name: "",
            core_modules_count: 0,
            core_instances_count: 0,
            imports_count: 0,
            exports_count: 0,
            aliases_count: 0,
            module_info: {
                use wrt_foundation::memory_system::SmallProvider;
                let provider = SmallProvider::new();
                wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
            },
            export_info: (),
            import_info: (),
        }
    };

    #[cfg(feature = "std")]
    return Ok(ComponentSummary {
        name: "".to_string(),
        core_modules_count: component.modules.len() as u32,
        core_instances_count: component.core_instances.len() as u32,
        imports_count: component.imports.len() as u32,
        exports_count: component.exports.len() as u32,
        aliases_count: component.aliases.len() as u32,
        module_info: Vec::new(),
        export_info: Vec::new(),
        import_info: Vec::new(),
    });

    #[cfg(not(feature = "std"))]
    Ok(component)
}

/// Extended import information
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtendedImportInfo {
    /// Import namespace
    pub namespace: String,
    /// Import name
    pub name: String,
    /// Kind of import (as string representation)
    pub kind: String,
}

/// Extended export information
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtendedExportInfo {
    /// Export name
    pub name: String,
    /// Kind of export (as string representation)
    pub kind: String,
    /// Export index
    pub index: u32,
}

/// Module import information
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleImportInfo {
    /// Module name (namespace)
    pub module: String,
    /// Import name
    pub name: String,
    /// Kind of import (as string representation)
    pub kind: String,
    /// Index within the type
    pub index: u32,
    /// Module index that contains this import
    pub module_idx: u32,
}

/// Module export information
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleExportInfo {
    /// Export name
    pub name: String,
    /// Kind of export (as string representation)
    pub kind: String,
    /// Index within the type
    pub index: u32,
    /// Module index that contains this export
    pub module_idx: u32,
}

/// Core module information
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoreModuleInfo {
    /// Module index
    pub idx: u32,
    /// Module size in bytes
    pub size: usize,
}

/// Core instance information
#[derive(Debug, Clone)]
pub struct CoreInstanceInfo {
    /// Index of the module instantiated
    pub module_idx: u32,
    /// Arguments passed to the instantiation
    pub args: Vec<String>,
}

/// Alias information
#[derive(Debug, Clone)]
pub struct AliasInfo {
    /// Kind of alias
    pub kind: String,
    /// Index of the instance being aliased
    pub instance_idx: u32,
    /// Name of the export being aliased
    pub export_name: String,
}

/// Analyze a component with extended information
#[cfg(feature = "std")]
pub fn analyze_component_extended(
    bytes: &[u8],
) -> Result<(
    ComponentSummary,
    Vec<ExtendedImportInfo>,
    Vec<ExtendedExportInfo>,
    Vec<ModuleImportInfo>,
    Vec<ModuleExportInfo>,
)> {
    // This is a simplified version - the real implementation would parse
    // the component and create extended information

    let summary = analyze_component(bytes)?;

    #[cfg(feature = "std")]
    return Ok((
        summary,
        Vec::new(), // Import info
        Vec::new(), // Export info
        Vec::new(), // Module import info
        Vec::new(), // Module export info
    ));

    #[cfg(not(feature = "std"))]
    {
        use wrt_foundation::safe_memory::NoStdProvider;
        let provider = NoStdProvider::<4096>::new();
        Ok((
            summary,
            wrt_foundation::BoundedVec::new(provider.clone()).unwrap_or_default(), // Import info
            wrt_foundation::BoundedVec::new(provider.clone()).unwrap_or_default(), // Export info
            wrt_foundation::BoundedVec::new(provider.clone()).unwrap_or_default(), // Module import info
            wrt_foundation::BoundedVec::new(provider).unwrap_or_default(), // Module export info
        ))
    }
}

/// Convert a CoreSort to a string representation (debug helper)
#[allow(dead_code)]
#[cfg(feature = "std")]
fn kind_to_string(kind: &CoreSort) -> String {
    match kind {
        CoreSort::Module => "CoreModule".to_string(),
        CoreSort::Function => "CoreFunction".to_string(),
        CoreSort::Table => "CoreTable".to_string(),
        CoreSort::Memory => "CoreMemory".to_string(),
        CoreSort::Global => "CoreGlobal".to_string(),
        CoreSort::Instance => "CoreInstance".to_string(),
        CoreSort::Type => "CoreType".to_string(),
    }
}

/// Helper to convert Sort to string (debug helper)
#[allow(dead_code)]
#[cfg(feature = "std")]
fn sort_to_string(sort: &wrt_format::component::Sort) -> String {
    match sort {
        Sort::Function => "Func".to_string(),
        Sort::Value => "Value".to_string(),
        Sort::Type => "Type".to_string(),
        Sort::Instance => "Instance".to_string(),
        Sort::Component => "Component".to_string(),
        Sort::Core(core_sort) => format!("Core({})", kind_to_string(core_sort)),
    }
}

/// Component analysis summary (std version)
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct ComponentSummary {
    /// Component name
    pub name: String,
    /// Number of core modules in the component
    pub core_modules_count: u32,
    /// Number of core instances in the component
    pub core_instances_count: u32,
    /// Number of imports in the component
    pub imports_count: u32,
    /// Number of exports in the component
    pub exports_count: u32,
    /// Number of aliases in the component
    pub aliases_count: u32,
    /// Information about modules in the component
    pub module_info: Vec<CoreModuleInfo>,
    /// Information about exports in the component
    pub export_info: Vec<ExtendedExportInfo>,
    /// Information about imports in the component
    pub import_info: Vec<ExtendedImportInfo>,
}

/// Component analysis summary (no_std version)
#[cfg(not(feature = "std"))]
#[derive(Debug, Clone)]
pub struct ComponentSummary {
    /// Component name (empty in no_std mode)
    pub name: &'static str,
    /// Number of core modules in the component
    pub core_modules_count: u32,
    /// Number of core instances in the component
    pub core_instances_count: u32,
    /// Number of imports in the component
    pub imports_count: u32,
    /// Number of exports in the component
    pub exports_count: u32,
    /// Number of aliases in the component
    pub aliases_count: u32,
    /// Information about modules in the component
    pub module_info: wrt_foundation::BoundedVec<CoreModuleInfo, 64, wrt_foundation::memory_system::SmallProvider>,
    /// Extended information disabled in no_std mode
    pub export_info: (),
    /// Extended information disabled in no_std mode
    pub import_info: (),
}

#[cfg(not(feature = "std"))]
impl wrt_foundation::traits::ToBytes for CoreModuleInfo {
    fn serialized_size(&self) -> usize {
        4 + 8 // u32 + usize
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_u32_le(self.idx)?;
        writer.write_u64_le(self.size as u64)?;
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl wrt_foundation::traits::FromBytes for CoreModuleInfo {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        stream: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let idx = stream.read_u32_le()?;
        let size = stream.read_u64_le()? as usize;
        Ok(CoreModuleInfo { idx, size })
    }
}

#[cfg(not(feature = "std"))]
impl wrt_foundation::traits::Checksummable for CoreModuleInfo {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&self.idx.to_le_bytes());
        checksum.update_slice(&(self.size as u64).to_le_bytes());
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::ToBytes for ExtendedImportInfo {
    fn serialized_size(&self) -> usize {
        self.namespace.len() + self.name.len() + self.kind.len() + 3
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        stream: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        stream.write_u8(self.namespace.len() as u8)?;
        stream.write_all(self.namespace.as_bytes())?;
        stream.write_u8(self.name.len() as u8)?;
        stream.write_all(self.name.as_bytes())?;
        stream.write_u8(self.kind.len() as u8)?;
        stream.write_all(self.kind.as_bytes())?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::FromBytes for ExtendedImportInfo {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        stream: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let namespace_len = stream.read_u8()? as usize;
        let mut namespace_bytes = vec![0u8; namespace_len];
        stream.read_exact(&mut namespace_bytes)?;
        let namespace = String::from_utf8(namespace_bytes)
            .map_err(|_| wrt_foundation::traits::SerializationError::InvalidFormat)?;
        
        let name_len = stream.read_u8()? as usize;
        let mut name_bytes = vec![0u8; name_len];
        stream.read_exact(&mut name_bytes)?;
        let name = String::from_utf8(name_bytes)
            .map_err(|_| wrt_foundation::traits::SerializationError::InvalidFormat)?;
        
        let kind_len = stream.read_u8()? as usize;
        let mut kind_bytes = vec![0u8; kind_len];
        stream.read_exact(&mut kind_bytes)?;
        let kind = String::from_utf8(kind_bytes)
            .map_err(|_| wrt_foundation::traits::SerializationError::InvalidFormat)?;
        
        Ok(ExtendedImportInfo { namespace, name, kind })
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::Checksummable for ExtendedImportInfo {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(self.namespace.as_bytes());
        checksum.update_slice(self.name.as_bytes());
        checksum.update_slice(self.kind.as_bytes());
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::ToBytes for ExtendedExportInfo {
    fn serialized_size(&self) -> usize {
        self.name.len() + self.kind.len() + 6 // 2 length bytes + 4 bytes for u32 index
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        stream: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        stream.write_u8(self.name.len() as u8)?;
        stream.write_all(self.name.as_bytes())?;
        stream.write_u8(self.kind.len() as u8)?;
        stream.write_all(self.kind.as_bytes())?;
        stream.write_u32_le(self.index)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::FromBytes for ExtendedExportInfo {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        stream: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        let name_len = stream.read_u8()? as usize;
        #[cfg(feature = "std")]
        let mut name_bytes = vec![0u8; name_len];
        #[cfg(not(feature = "std"))]
        let mut name_bytes = {
            use wrt_foundation::memory_system::SmallProvider;
            let provider = SmallProvider::new();
            let mut vec = wrt_foundation::BoundedVec::<u8, 256, SmallProvider>::new(provider).unwrap_or_default();
            vec.resize(name_len, 0u8);
            vec
        };
        stream.read_exact(&mut name_bytes)?;
        let name = String::from_utf8(name_bytes.to_vec())
            .map_err(|_| wrt_foundation::traits::SerializationError::InvalidFormat)?;
        
        let kind_len = stream.read_u8()? as usize;
        #[cfg(feature = "std")]
        let mut kind_bytes = vec![0u8; kind_len];
        #[cfg(not(feature = "std"))]
        let mut kind_bytes = {
            use wrt_foundation::memory_system::SmallProvider;
            let provider = SmallProvider::new();
            let mut vec = wrt_foundation::BoundedVec::<u8, 256, SmallProvider>::new(provider).unwrap_or_default();
            vec.resize(kind_len, 0u8);
            vec
        };
        stream.read_exact(&mut kind_bytes)?;
        let kind = String::from_utf8(kind_bytes.to_vec())
            .map_err(|_| wrt_foundation::traits::SerializationError::InvalidFormat)?;
        
        let index = stream.read_u32_le()?;
        
        Ok(ExtendedExportInfo { name, kind, index })
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::Checksummable for ExtendedExportInfo {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(self.name.as_bytes());
        checksum.update_slice(self.kind.as_bytes());
        checksum.update_slice(&self.index.to_le_bytes());
    }
}

// Add missing trait implementations for ModuleImportInfo
#[cfg(feature = "std")]
impl wrt_foundation::traits::ToBytes for ModuleImportInfo {
    fn serialized_size(&self) -> usize {
        self.module.len() + self.name.len() + self.kind.len() + 3 + 8 // strings + separators + 2 u32s
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_all(self.module.as_bytes())?;
        writer.write_u8(0)?; // separator
        writer.write_all(self.name.as_bytes())?;
        writer.write_u8(0)?; // separator
        writer.write_all(self.kind.as_bytes())?;
        writer.write_u8(0)?; // separator
        writer.write_u32_le(self.index)?;
        writer.write_u32_le(self.module_idx)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::FromBytes for ModuleImportInfo {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        stream: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        #[cfg(feature = "std")]
        let mut bytes = Vec::new();
        #[cfg(not(feature = "std"))]
        let mut bytes = {
            use wrt_foundation::memory_system::SmallProvider;
            let provider = SmallProvider::new();
            wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
        };
        loop {
            match stream.read_u8() {
                Ok(byte) => bytes.push(byte),
                Err(_) => break,
            }
        }
        
        let parts: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
        if parts.len() >= 3 {
            let index = if parts.len() > 3 && parts[3].len() >= 4 {
                u32::from_le_bytes([parts[3][0], parts[3][1], parts[3][2], parts[3][3]])
            } else { 0 };
            let module_idx = if parts.len() > 3 && parts[3].len() >= 8 {
                u32::from_le_bytes([parts[3][4], parts[3][5], parts[3][6], parts[3][7]])
            } else { 0 };
            
            Ok(ModuleImportInfo {
                module: String::from_utf8_lossy(parts[0]).to_string(),
                name: String::from_utf8_lossy(parts[1]).to_string(),
                kind: String::from_utf8_lossy(parts[2]).to_string(),
                index,
                module_idx,
            })
        } else {
            Ok(ModuleImportInfo::default())
        }
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::Checksummable for ModuleImportInfo {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(self.module.as_bytes());
        checksum.update_slice(self.name.as_bytes());
        checksum.update_slice(self.kind.as_bytes());
        checksum.update_slice(&self.index.to_le_bytes());
        checksum.update_slice(&self.module_idx.to_le_bytes());
    }
}

// Add missing trait implementations for ModuleExportInfo
#[cfg(feature = "std")]
impl wrt_foundation::traits::ToBytes for ModuleExportInfo {
    fn serialized_size(&self) -> usize {
        self.name.len() + self.kind.len() + 2 + 8 // strings + separators + 2 u32s
    }

    fn to_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<()> {
        writer.write_all(self.name.as_bytes())?;
        writer.write_u8(0)?; // separator
        writer.write_all(self.kind.as_bytes())?;
        writer.write_u8(0)?; // separator
        writer.write_u32_le(self.index)?;
        writer.write_u32_le(self.module_idx)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::FromBytes for ModuleExportInfo {
    fn from_bytes_with_provider<PStream: wrt_foundation::MemoryProvider>(
        stream: &mut wrt_foundation::traits::ReadStream,
        _provider: &PStream,
    ) -> wrt_foundation::WrtResult<Self> {
        #[cfg(feature = "std")]
        let mut bytes = Vec::new();
        #[cfg(not(feature = "std"))]
        let mut bytes = {
            use wrt_foundation::memory_system::SmallProvider;
            let provider = SmallProvider::new();
            wrt_foundation::BoundedVec::new(provider).unwrap_or_default()
        };
        loop {
            match stream.read_u8() {
                Ok(byte) => bytes.push(byte),
                Err(_) => break,
            }
        }
        
        let parts: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
        if parts.len() >= 2 {
            let index = if parts.len() > 2 && parts[2].len() >= 4 {
                u32::from_le_bytes([parts[2][0], parts[2][1], parts[2][2], parts[2][3]])
            } else { 0 };
            let module_idx = if parts.len() > 2 && parts[2].len() >= 8 {
                u32::from_le_bytes([parts[2][4], parts[2][5], parts[2][6], parts[2][7]])
            } else { 0 };
            
            Ok(ModuleExportInfo {
                name: String::from_utf8_lossy(parts[0]).to_string(),
                kind: String::from_utf8_lossy(parts[1]).to_string(),
                index,
                module_idx,
            })
        } else {
            Ok(ModuleExportInfo::default())
        }
    }
}

#[cfg(feature = "std")]
impl wrt_foundation::traits::Checksummable for ModuleExportInfo {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(self.name.as_bytes());
        checksum.update_slice(self.kind.as_bytes());
        checksum.update_slice(&self.index.to_le_bytes());
        checksum.update_slice(&self.module_idx.to_le_bytes());
    }
}
