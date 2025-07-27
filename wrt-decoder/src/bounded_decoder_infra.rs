//! Bounded Infrastructure for Decoder
//!
//! This module provides bounded alternatives for decoder collections
//! to ensure static memory allocation throughout the decoder.

use wrt_foundation::{
    bounded::{
        BoundedString,
        BoundedVec,
    },
    budget_aware_provider::CrateId,
    generic_memory_guard::MemoryGuard,
    no_std_hashmap::BoundedHashMap,
    safe_managed_alloc,
    safe_memory::NoStdProvider,
    WrtResult,
};

/// Instead of a type alias, we'll use concrete allocation in factory functions
/// This avoids the complex provider type system during migration

/// Maximum number of sections in a module
pub const MAX_SECTIONS: usize = 32;

/// Maximum number of types in a module
pub const MAX_TYPES: usize = 1024;

/// Maximum number of imports in a module
pub const MAX_IMPORTS: usize = 1024;

/// Maximum number of exports in a module
pub const MAX_EXPORTS: usize = 1024;

/// Maximum number of functions in a module
pub const MAX_FUNCTIONS: usize = 4096;

/// Maximum number of table entries
pub const MAX_TABLE_ENTRIES: usize = 1024;

/// Maximum number of memory definitions
pub const MAX_MEMORIES: usize = 8;

/// Maximum number of globals
pub const MAX_GLOBALS: usize = 1024;

/// Maximum number of elements
pub const MAX_ELEMENTS: usize = 1024;

/// Maximum number of data segments
pub const MAX_DATA_SEGMENTS: usize = 1024;

/// Maximum number of locals per function
pub const MAX_LOCALS_PER_FUNCTION: usize = 256;

/// Maximum length for names (modules, functions, etc.)
pub const MAX_NAME_LENGTH: usize = 256;

/// Maximum custom section data size
pub const MAX_CUSTOM_SECTION_SIZE: usize = 16384; // 16KB

/// Maximum number of function parameters
pub const MAX_FUNCTION_PARAMS: usize = 128;

/// Maximum number of function results
pub const MAX_FUNCTION_RESULTS: usize = 16;

/// Maximum number of entries in name maps
pub const MAX_NAME_MAP_ENTRIES: usize = 1024;

// Type aliases using unified capability-based memory allocation
// These provide consistent interfaces while using safe_managed_alloc internally

/// Provider type for decoder operations
pub type DecoderProvider = NoStdProvider<4096>;

/// Section vector with bounded capacity
pub type BoundedSectionVec<T> = BoundedVec<T, MAX_SECTIONS, DecoderProvider>;

/// Type vector with bounded capacity  
pub type BoundedTypeVec<T> = BoundedVec<T, MAX_TYPES, DecoderProvider>;

/// Import vector with bounded capacity
pub type BoundedImportVec<T> = BoundedVec<T, MAX_IMPORTS, DecoderProvider>;

/// Export vector with bounded capacity
pub type BoundedExportVec<T> = BoundedVec<T, MAX_EXPORTS, DecoderProvider>;

/// Function vector with bounded capacity
pub type BoundedFunctionVec<T> = BoundedVec<T, MAX_FUNCTIONS, DecoderProvider>;

/// Table vector with bounded capacity
pub type BoundedTableVec<T> = BoundedVec<T, MAX_TABLE_ENTRIES, DecoderProvider>;

/// Memory vector with bounded capacity
pub type BoundedMemoryVec<T> = BoundedVec<T, MAX_MEMORIES, DecoderProvider>;

/// Global vector with bounded capacity
pub type BoundedGlobalVec<T> = BoundedVec<T, MAX_GLOBALS, DecoderProvider>;

/// Element vector with bounded capacity
pub type BoundedElementVec<T> = BoundedVec<T, MAX_ELEMENTS, DecoderProvider>;

/// Data vector with bounded capacity
pub type BoundedDataVec<T> = BoundedVec<T, MAX_DATA_SEGMENTS, DecoderProvider>;

/// Module vector with bounded capacity
pub type BoundedModuleVec<T> = BoundedVec<T, MAX_MODULES_PER_COMPONENT, DecoderProvider>;

/// Name string with bounded capacity
pub type BoundedNameString = BoundedString<MAX_NAME_LENGTH, DecoderProvider>;

/// Name map with bounded capacity
pub type BoundedNameMap<V> =
    BoundedHashMap<BoundedNameString, V, MAX_NAME_MAP_ENTRIES, DecoderProvider>;

/// Custom data with bounded capacity
pub type BoundedCustomData = BoundedVec<u8, MAX_CUSTOM_SECTION_SIZE, DecoderProvider>;

/// Create a new bounded section vector using capability-based allocation
pub fn new_section_vec<T>() -> WrtResult<BoundedVec<T, MAX_SECTIONS, NoStdProvider<4096>>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded type vector using capability-based allocation
pub fn new_type_vec() -> WrtResult<BoundedTypeVec<wrt_foundation::types::FuncType<DecoderProvider>>>
{
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded type vector (generic version) using capability-based
/// allocation
pub fn new_type_vec_generic<T>() -> WrtResult<BoundedTypeVec<T>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = safe_managed_alloc!(8192, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded import vector using capability-based allocation
pub fn new_import_vec(
) -> WrtResult<BoundedImportVec<wrt_foundation::types::Import<DecoderProvider>>> {
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded import vector (generic version) using capability-based
/// allocation
pub fn new_import_vec_generic<T>() -> WrtResult<BoundedImportVec<T>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = safe_managed_alloc!(8192, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded export vector using capability-based allocation
pub fn new_export_vec() -> WrtResult<BoundedExportVec<wrt_format::module::Export>> {
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded export vector (generic version) using capability-based
/// allocation
pub fn new_export_vec_generic<T>() -> WrtResult<BoundedExportVec<T>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = safe_managed_alloc!(8192, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded function vector using capability-based allocation
pub fn new_function_vec() -> WrtResult<BoundedFunctionVec<u32>> {
    let provider = safe_managed_alloc!(16384, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded function vector (generic version) using
/// capability-based allocation
pub fn new_function_vec_generic<T>() -> WrtResult<BoundedFunctionVec<T>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = safe_managed_alloc!(16384, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded name string using capability-based allocation
pub fn new_name_string() -> WrtResult<BoundedNameString> {
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedString::from_str("", provider)
        .map_err(|_| wrt_error::Error::memory_error("Failed to create empty bounded string"))
}

/// Create a bounded name string from a str using capability-based allocation
pub fn bounded_name_from_str(s: &str) -> WrtResult<BoundedNameString> {
    let provider = safe_managed_alloc!(1024, CrateId::Decoder)?;
    BoundedString::from_str(s, provider)
        .map_err(|_| wrt_error::Error::validation_error("String too long for bounded name"))
}

/// Create a new bounded name map using capability-based allocation
pub fn new_name_map<V>() -> WrtResult<BoundedNameMap<V>>
where
    V: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedHashMap::new(provider)
}

// Additional concrete vector factory functions

/// Create a new bounded params vector (for function parameters)
pub fn new_params_vec(
) -> WrtResult<BoundedVec<wrt_format::types::ValueType, MAX_FUNCTION_PARAMS, NoStdProvider<2048>>> {
    let provider = safe_managed_alloc!(2048, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded results vector (for function results)
pub fn new_results_vec(
) -> WrtResult<BoundedVec<wrt_format::types::ValueType, MAX_FUNCTION_RESULTS, NoStdProvider<1024>>>
{
    let provider = safe_managed_alloc!(1024, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded table vector
pub fn new_table_vec() -> WrtResult<BoundedTableVec<wrt_foundation::types::TableType>> {
    let provider = safe_managed_alloc!(4096, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded memory vector
pub fn new_memory_vec() -> WrtResult<BoundedMemoryVec<wrt_foundation::types::MemoryType>> {
    let provider = safe_managed_alloc!(2048, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded global vector
pub fn new_global_vec() -> WrtResult<BoundedGlobalVec<wrt_foundation::types::GlobalType>> {
    let provider = safe_managed_alloc!(8192, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded element vector
pub fn new_element_vec() -> WrtResult<BoundedElementVec<wrt_format::module::Element>> {
    let provider = safe_managed_alloc!(8192, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded data vector
pub fn new_data_vec() -> WrtResult<BoundedDataVec<wrt_format::module::Data>> {
    let provider = safe_managed_alloc!(8192, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

/// Create a new bounded code bodies vector
pub fn new_code_bodies_vec() -> WrtResult<BoundedFunctionVec<BoundedCustomData>> {
    let provider = safe_managed_alloc!(16384, CrateId::Decoder)?;
    BoundedVec::new(provider)
}

// Component Model specific constants and types

/// Maximum number of modules in a component
pub const MAX_MODULES_PER_COMPONENT: usize = 64;

/// Maximum number of types in a component
pub const MAX_TYPES_PER_COMPONENT: usize = 512;

/// Maximum number of instances in a component
pub const MAX_INSTANCES_PER_COMPONENT: usize = 128;

/// Maximum number of aliases in a component
pub const MAX_ALIASES_PER_COMPONENT: usize = 256;

// Component-specific factory functions use direct allocation instead of type
// aliases

/// Create a decoder provider with the specified size  
pub fn create_decoder_provider<const N: usize>() -> WrtResult<NoStdProvider<N>> {
    safe_managed_alloc!(N, CrateId::Decoder)
}
