//! Bounded Infrastructure for Decoder
//!
//! This module provides bounded alternatives for decoder collections
//! to ensure static memory allocation throughout the decoder.

use wrt_foundation::{
    bounded::{BoundedString, BoundedVec},
    no_std_hashmap::BoundedHashMap,
    safe_memory::NoStdProvider,
    WrtResult,
};

/// Budget-aware memory provider for decoder (64KB)
pub type DecoderProvider = NoStdProvider<65536>;

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

/// Bounded vector for sections
pub type BoundedSectionVec<T> = BoundedVec<T, MAX_SECTIONS, DecoderProvider>;

/// Bounded vector for types
pub type BoundedTypeVec<T> = BoundedVec<T, MAX_TYPES, DecoderProvider>;

/// Bounded vector for imports
pub type BoundedImportVec<T> = BoundedVec<T, MAX_IMPORTS, DecoderProvider>;

/// Bounded vector for exports
pub type BoundedExportVec<T> = BoundedVec<T, MAX_EXPORTS, DecoderProvider>;

/// Bounded vector for functions
pub type BoundedFunctionVec<T> = BoundedVec<T, MAX_FUNCTIONS, DecoderProvider>;

/// Bounded vector for table entries
pub type BoundedTableVec<T> = BoundedVec<T, MAX_TABLE_ENTRIES, DecoderProvider>;

/// Bounded vector for memory definitions
pub type BoundedMemoryVec<T> = BoundedVec<T, MAX_MEMORIES, DecoderProvider>;

/// Bounded vector for globals
pub type BoundedGlobalVec<T> = BoundedVec<T, MAX_GLOBALS, DecoderProvider>;

/// Bounded vector for elements
pub type BoundedElementVec<T> = BoundedVec<T, MAX_ELEMENTS, DecoderProvider>;

/// Bounded vector for data segments
pub type BoundedDataVec<T> = BoundedVec<T, MAX_DATA_SEGMENTS, DecoderProvider>;

/// Bounded vector for locals
pub type BoundedLocalsVec<T> = BoundedVec<T, MAX_LOCALS_PER_FUNCTION, DecoderProvider>;

/// Bounded string for names
pub type BoundedNameString = BoundedString<MAX_NAME_LENGTH, DecoderProvider>;

/// Bounded vector for custom section data
pub type BoundedCustomData = BoundedVec<u8, MAX_CUSTOM_SECTION_SIZE, DecoderProvider>;

/// Bounded map for name resolution (exports, imports, etc.)
pub type BoundedNameMap<V> = BoundedHashMap<
    BoundedNameString,
    V,
    MAX_EXPORTS, // Use MAX_EXPORTS as general limit
    DecoderProvider,
>;

/// Create a new bounded section vector
pub fn new_section_vec<T>() -> WrtResult<BoundedSectionVec<T>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded type vector (concrete version for WrtFuncType)
pub fn new_type_vec() -> WrtResult<BoundedTypeVec<wrt_foundation::types::FuncType<DecoderProvider>>>
{
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded type vector (generic version)
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
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded import vector (concrete version for WrtImport)
pub fn new_import_vec(
) -> WrtResult<BoundedImportVec<wrt_foundation::types::Import<DecoderProvider>>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded import vector (generic version)
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
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded export vector (concrete version for WrtExport)  
pub fn new_export_vec() -> WrtResult<BoundedExportVec<wrt_format::module::Export>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded export vector (generic version)
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
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded function vector (concrete u32 version)
pub fn new_function_vec() -> WrtResult<BoundedFunctionVec<u32>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded function vector (generic version)
pub fn new_function_vec_generic<T>() -> WrtResult<BoundedVec<T, MAX_FUNCTIONS, DecoderProvider>>
where
    T: wrt_foundation::traits::Checksummable
        + wrt_foundation::traits::ToBytes
        + wrt_foundation::traits::FromBytes
        + Default
        + Clone
        + PartialEq
        + Eq,
{
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded name string
pub fn new_name_string() -> WrtResult<BoundedNameString> {
    let provider = DecoderProvider::default();
    BoundedString::from_str("", provider).map_err(|_| {
        wrt_error::Error::new(
            wrt_error::ErrorCategory::Memory,
            wrt_error::codes::MEMORY_ALLOCATION_FAILED,
            "Failed to create empty bounded string",
        )
    })
}

/// Create a bounded name string from a str
pub fn bounded_name_from_str(s: &str) -> WrtResult<BoundedNameString> {
    let provider = DecoderProvider::default();
    BoundedString::from_str(s, provider).map_err(|_| {
        wrt_error::Error::new(
            wrt_error::ErrorCategory::Validation,
            wrt_error::codes::VALIDATION_ERROR,
            "String too long for bounded name",
        )
    })
}

/// Create a new bounded name map
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
    let provider = DecoderProvider::default();
    BoundedHashMap::new(provider)
}

// Additional concrete vector factory functions

/// Create a new bounded params vector (for function parameters)
pub fn new_params_vec(
) -> WrtResult<BoundedVec<wrt_format::types::ValueType, MAX_FUNCTION_PARAMS, DecoderProvider>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded results vector (for function results)
pub fn new_results_vec(
) -> WrtResult<BoundedVec<wrt_format::types::ValueType, MAX_FUNCTION_RESULTS, DecoderProvider>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded table vector
pub fn new_table_vec() -> WrtResult<BoundedTableVec<wrt_foundation::types::TableType>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded memory vector
pub fn new_memory_vec() -> WrtResult<BoundedMemoryVec<wrt_foundation::types::MemoryType>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded global vector
pub fn new_global_vec() -> WrtResult<BoundedGlobalVec<wrt_foundation::types::GlobalType>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded element vector
pub fn new_element_vec() -> WrtResult<BoundedElementVec<wrt_format::module::Element>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded data vector
pub fn new_data_vec(
) -> WrtResult<BoundedVec<wrt_format::module::Data, MAX_DATA_SEGMENTS, DecoderProvider>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}

/// Create a new bounded code bodies vector
pub fn new_code_bodies_vec() -> WrtResult<BoundedFunctionVec<BoundedCustomData>> {
    let provider = DecoderProvider::default();
    BoundedVec::new(provider)
}
