// WRT - wrt-foundation
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly type definitions
//!
//! This module defines core WebAssembly types and utilities for working with
//! them, including function types, block types, value types, and reference
//! types.

use core::{
    fmt::{self, Display, Write},
    hash::{Hash, Hasher as CoreHasher},
    str::FromStr,
};

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

// Use HashMap/HashSet in std mode, BTreeMap/BTreeSet in no_std mode
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::collections::{BTreeMap as Map, BTreeSet as Set};
#[cfg(feature = "std")]
use std::collections::{HashMap as Map, HashSet as Set};

// String and Vec handling
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    string::{String, ToString},
    vec::Vec,
};

// Import error types
use wrt_error::{Error, ErrorCategory};

// Import bounded types
use crate::bounded::{BoundedError, BoundedVec, WasmName, MAX_WASM_NAME_LENGTH};
use crate::{
    bounded::{
        MAX_CUSTOM_SECTION_DATA_SIZE, MAX_WASM_ITEM_NAME_LENGTH as MAX_ITEM_NAME_LEN,
        MAX_WASM_MODULE_NAME_LENGTH as MAX_MODULE_NAME_LEN,
    },
    codes,
    component::Export,
    prelude::{BoundedCapacity, Eq, Ord, PartialEq, TryFrom},
    traits::{
        Checksummable, DefaultMemoryProvider, FromBytes, ReadStream, SerializationError, ToBytes,
        WriteStream,
    },
    verification::Checksum,
    MemoryProvider, WrtResult,
};

// Alias WrtResult as Result for this module
type Result<T> = WrtResult<T>;

// Constants for array bounds in serializable types
pub const MAX_PARAMS_IN_FUNC_TYPE: usize = 128;
pub const MAX_RESULTS_IN_FUNC_TYPE: usize = 128;
// Add other MAX constants as they become necessary, e.g. for Instructions,
// Module fields etc. For BrTable in Instruction:
pub const MAX_BR_TABLE_TARGETS: usize = 256;
// For SelectTyped in Instruction: (WASM MVP select is 1 type, or untyped)
pub const MAX_SELECT_TYPES: usize = 1;

// Constants for Module structure limits
pub const MAX_TYPES_IN_MODULE: usize = 1024;
pub const MAX_FUNCS_IN_MODULE: usize = 1024; // Max functions (imports + defined)
pub const MAX_IMPORTS_IN_MODULE: usize = 1024;
pub const MAX_EXPORTS_IN_MODULE: usize = 1024;
pub const MAX_TABLES_IN_MODULE: usize = 16;
pub const MAX_MEMORIES_IN_MODULE: usize = 16;
pub const MAX_GLOBALS_IN_MODULE: usize = 1024;
pub const MAX_ELEMENT_SEGMENTS_IN_MODULE: usize = 1024;
pub const MAX_DATA_SEGMENTS_IN_MODULE: usize = 1024;
pub const MAX_LOCALS_PER_FUNCTION: usize = 512; // Max local entries per function
pub const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 8192; // Max instructions in a function body/expr
pub const MAX_ELEMENT_INDICES_PER_SEGMENT: usize = 8192; // Max func indices in an element segment
pub const MAX_DATA_SEGMENT_LENGTH: usize = 65_536; // Max bytes in a data segment (active/passive)
pub const MAX_TAGS_IN_MODULE: usize = 1024;
pub const MAX_CUSTOM_SECTIONS_IN_MODULE: usize = 64;
// MAX_CUSTOM_SECTION_DATA_SIZE, MAX_MODULE_NAME_LEN, and MAX_ITEM_NAME_LEN are
// now imported from bounded.rs

pub const DEFAULT_FUNC_TYPE_PROVIDER_CAPACITY: usize = 256;

/// Index for a type in the types section.
pub type TypeIdx = u32;
/// Index for a function, referring to both imported and module-defined
/// functions.
pub type FuncIdx = u32;
/// Index for a table.
pub type TableIdx = u32;
/// Index for a memory.
pub type MemIdx = u32;
/// Index for a global variable, referring to both imported and module-defined
/// globals.
pub type GlobalIdx = u32;
/// Index for an element segment.
pub type ElemIdx = u32;
/// Index for a data segment.
pub type DataIdx = u32;
/// Index for a local variable within a function.
pub type LocalIdx = u32;
/// Index for a label in control flow instructions (e.g., branches).
pub type LabelIdx = u32; // For branches
/// Index for an exception tag.
pub type TagIdx = u32;

/// Internal hasher for `FuncType`, may be removed or replaced.
#[derive(Default)] // Simplified Debug for Hasher
struct Hasher {
    hash: u32,
}

impl core::fmt::Debug for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hasher").field("hash", &self.hash).finish()
    }
}

#[allow(dead_code)]
impl Hasher {
    fn new() -> Self {
        Self { hash: 0x811c_9dc5 } // FNV-1a offset basis for 32-bit
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hash ^= u32::from(byte);
            self.hash = self.hash.wrapping_mul(0x0100_0193); // FNV prime for
                                                             // 32-bit
        }
    }

    fn finalize(&self) -> u32 {
        self.hash
    }
}

/// WebAssembly value types
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum ValueType {
    /// 32-bit integer
    #[default]
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// 128-bit SIMD vector
    V128,
    /// A 128-bit SIMD vector of 8xI16 lanes (Hypothetical Wasm 3.0 feature)
    I16x8,
    /// Function reference
    FuncRef,
    /// External reference
    ExternRef,
    /// Struct reference (WebAssembly 3.0 GC)
    StructRef(u32), // type index
    /// Array reference (WebAssembly 3.0 GC)
    ArrayRef(u32), // type index
}

impl core::fmt::Debug for ValueType {
    // Binary std/no_std choice
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32 => write!(f, "I32"),
            Self::I64 => write!(f, "I64"),
            Self::F32 => write!(f, "F32"),
            Self::F64 => write!(f, "F64"),
            Self::V128 => write!(f, "V128"),
            Self::I16x8 => write!(f, "I16x8"),
            Self::FuncRef => write!(f, "FuncRef"),
            Self::ExternRef => write!(f, "ExternRef"),
            Self::StructRef(idx) => f.debug_tuple("StructRef").field(idx).finish(),
            Self::ArrayRef(idx) => f.debug_tuple("ArrayRef").field(idx).finish(),
        }
    }
}

impl ValueType {
    /// Create a value type from a binary representation
    ///
    /// Uses the standardized conversion utility for consistency
    /// across all crates.
    ///
    /// Note: StructRef and ArrayRef require additional type index data
    /// and should be parsed with `from_binary_with_index`.
    pub fn from_binary(byte: u8) -> Result<Self> {
        match byte {
            0x7F => Ok(ValueType::I32),
            0x7E => Ok(ValueType::I64),
            0x7D => Ok(ValueType::F32),
            0x7C => Ok(ValueType::F64),
            0x7B => Ok(ValueType::V128),
            0x79 => Ok(ValueType::I16x8),
            0x70 => Ok(ValueType::FuncRef),
            0x6F => Ok(ValueType::ExternRef),
            _ => Err(Error::new(
                // Replace format!
                ErrorCategory::Parse,
                wrt_error::codes::PARSE_INVALID_VALTYPE_BYTE,
                // A static string, or pass byte to error for later formatting
                "Invalid value type byte", // Potential: Add byte as context to Error
            )),
        }
    }

    /// Create a value type from binary representation with type index for aggregate types
    pub fn from_binary_with_index(byte: u8, type_index: u32) -> Result<Self> {
        match byte {
            0x7F => Ok(ValueType::I32),
            0x7E => Ok(ValueType::I64),
            0x7D => Ok(ValueType::F32),
            0x7C => Ok(ValueType::F64),
            0x7B => Ok(ValueType::V128),
            0x79 => Ok(ValueType::I16x8),
            0x70 => Ok(ValueType::FuncRef),
            0x6F => Ok(ValueType::ExternRef),
            0x6E => Ok(ValueType::StructRef(type_index)), // New: struct reference
            0x6D => Ok(ValueType::ArrayRef(type_index)),  // New: array reference
            _ => Err(Error::new(
                ErrorCategory::Parse,
                wrt_error::codes::PARSE_INVALID_VALTYPE_BYTE,
                "Invalid value type byte",
            )),
        }
    }

    /// Convert to the WebAssembly binary format value
    ///
    /// Uses the standardized conversion utility for consistency
    /// across all crates.
    #[must_use]
    pub fn to_binary(self) -> u8 {
        match self {
            ValueType::I32 => 0x7F,
            ValueType::I64 => 0x7E,
            ValueType::F32 => 0x7D,
            ValueType::F64 => 0x7C,
            ValueType::V128 => 0x7B,
            ValueType::I16x8 => 0x79,
            ValueType::FuncRef => 0x70,
            ValueType::ExternRef => 0x6F,
            ValueType::StructRef(_) => 0x6E,
            ValueType::ArrayRef(_) => 0x6D,
        }
    }

    /// Get the type index for aggregate types (struct/array references)
    #[must_use]
    pub fn type_index(self) -> Option<u32> {
        match self {
            ValueType::StructRef(idx) | ValueType::ArrayRef(idx) => Some(idx),
            _ => None,
        }
    }

    /// Get the size of this value type in bytes
    #[must_use]
    pub fn size_in_bytes(self) -> usize {
        match self {
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 => 8,
            Self::V128 | Self::I16x8 => 16, // COMBINED ARMS
            Self::FuncRef | Self::ExternRef | Self::StructRef(_) | Self::ArrayRef(_) => {
                // Size of a reference can vary. Using usize for simplicity.
                // In a real scenario, this might depend on target architecture (32/64 bit).
                core::mem::size_of::<usize>()
            }
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl Checksummable for ValueType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        checksum.update_slice(&[self.to_binary()]);
    }
}

impl ToBytes for ValueType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        writer.write_u8(self.to_binary())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl FromBytes for ValueType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        let byte = reader.read_u8()?;
        ValueType::from_binary(byte)
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

/// WebAssembly reference types (funcref, externref)
///
/// These are subtypes of `ValueType` and used in table elements, function
/// returns, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RefType {
    /// Function reference type
    #[default]
    Funcref,
    /// External reference type
    Externref,
}

impl RefType {
    // ... from_binary, to_binary (if they exist, or adapt ValueType's)
    pub fn to_value_type(self) -> ValueType {
        match self {
            RefType::Funcref => ValueType::FuncRef,
            RefType::Externref => ValueType::ExternRef,
        }
    }
    pub fn from_value_type(vt: ValueType) -> Result<Self> {
        match vt {
            ValueType::FuncRef => Ok(RefType::Funcref),
            ValueType::ExternRef => Ok(RefType::Externref),
            _ => Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_INVALID_CONVERSION,
                "Cannot convert ValueType to RefType",
            )),
        }
    }
}
impl Checksummable for RefType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        checksum.update_slice(&[self.to_value_type().to_binary()]);
    }
}

impl ToBytes for RefType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        let val_type: ValueType = (*self).into();
        val_type.to_bytes_with_provider(writer, _provider)
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl FromBytes for RefType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        let value_type = ValueType::from_bytes_with_provider(reader, _provider)?;
        RefType::try_from(value_type).map_err(Error::from)
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

impl From<RefType> for ValueType {
    fn from(rt: RefType) -> Self {
        match rt {
            RefType::Funcref => ValueType::FuncRef,
            RefType::Externref => ValueType::ExternRef,
        }
    }
}
impl TryFrom<ValueType> for RefType {
    type Error = crate::Error; // Use the crate's Error type

    fn try_from(vt: ValueType) -> core::result::Result<Self, Self::Error> {
        match vt {
            ValueType::FuncRef => Ok(RefType::Funcref),
            ValueType::ExternRef => Ok(RefType::Externref),
            _ => Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_INVALID_CONVERSION,
                "ValueType cannot be converted to RefType",
            )),
        }
    }
}

/// Maximum number of parameters allowed in a function type by this
/// implementation.
pub const MAX_FUNC_TYPE_PARAMS: usize = MAX_PARAMS_IN_FUNC_TYPE; // Use the new constant
/// Maximum number of results allowed in a function type by this implementation.
pub const MAX_FUNC_TYPE_RESULTS: usize = MAX_RESULTS_IN_FUNC_TYPE; // Use the new constant

/// Represents the type of a WebAssembly function.
///
/// It defines the parameter types and result types of a function.
/// Binary std/no_std choice
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncType<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> {
    pub params: BoundedVec<ValueType, MAX_PARAMS_IN_FUNC_TYPE, P>,
    pub results: BoundedVec<ValueType, MAX_RESULTS_IN_FUNC_TYPE, P>,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FuncType<P> {
    /// Creates a new `FuncType` with the given parameter and result types.
    ///
    /// # Errors
    ///
    /// Returns an error if creating the internal bounded vectors fails (e.g.,
    /// due to provider issues).
    pub fn new(
        provider: P,
        params_iter: impl IntoIterator<Item = ValueType>,
        results_iter: impl IntoIterator<Item = ValueType>,
    ) -> WrtResult<Self> {
        let mut params = BoundedVec::new(provider.clone()).map_err(Error::from)?;
        for vt in params_iter {
            params.push(vt).map_err(Error::from)?;
        }
        let mut results = BoundedVec::new(provider).map_err(Error::from)?;
        for vt in results_iter {
            results.push(vt).map_err(Error::from)?;
        }
        Ok(Self { params, results })
    }

    /// Verifies the function type.
    /// Placeholder implementation.
    pub fn verify(&self) -> WrtResult<()> {
        // TODO: Implement actual verification logic for FuncType
        // e.g., check constraints on params/results if any beyond BoundedVec capacity.
        Ok(())
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for FuncType<P>
{
    fn default() -> Self {
        let provider = P::default();
        // This expect is problematic for safety if P::default() or BoundedVec::new can
        // fail. For now, to proceed with compilation, but this needs review.
        let params = BoundedVec::new(provider.clone())
            .expect("Default provider should allow BoundedVec creation for FuncType params");
        let results = BoundedVec::new(provider)
            .expect("Default provider should allow BoundedVec creation for FuncType results");
        Self { params, results }
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for FuncType<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Update checksum with params
        checksum.update_slice(&(self.params.len() as u32).to_le_bytes());
        for param in self.params.iter() {
            param.update_checksum(checksum);
        }
        // Update checksum with results
        checksum.update_slice(&(self.results.len() as u32).to_le_bytes());
        for result in self.results.iter() {
            result.update_checksum(checksum);
        }
    }
}

impl<PFunc: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ToBytes
    for FuncType<PFunc>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        writer.write_u8(0x60)?; // FuncType prefix
        self.params.to_bytes_with_provider(writer, stream_provider)?;
        self.results.to_bytes_with_provider(writer, stream_provider)?;
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl<PFunc: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FromBytes
    for FuncType<PFunc>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let prefix = reader.read_u8()?;
        if prefix != 0x60 {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::DECODING_ERROR,
                "Invalid FuncType prefix",
            ));
        }
        // PFunc must be Default + Clone for BoundedVec::from_bytes_with_provider
        // if BoundedVec needs to create its own provider. Here, we pass
        // stream_provider.
        let params =
            BoundedVec::<ValueType, MAX_FUNC_TYPE_PARAMS, PFunc>::from_bytes_with_provider(
                reader,
                stream_provider,
            )?;
        let results =
            BoundedVec::<ValueType, MAX_FUNC_TYPE_RESULTS, PFunc>::from_bytes_with_provider(
                reader,
                stream_provider,
            )?;

        Ok(FuncType { params, results })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

// Display and Debug impls follow...

/// Memory argument for load/store instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemArg {
    /// The alignment exponent (2^align_exponent bytes)
    pub align_exponent: u32,
    /// The offset to add to the address
    pub offset: u32,
    /// The memory index (0 for single memory)
    pub memory_index: u32,
}

impl Default for MemArg {
    fn default() -> Self {
        Self { align_exponent: 0, offset: 0, memory_index: 0 }
    }
}

impl ToBytes for MemArg {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        writer.write_u32_le(self.align_exponent)?;
        writer.write_u32_le(self.offset)?;
        writer.write_u32_le(self.memory_index)
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl FromBytes for MemArg {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        let align_exponent = reader.read_u32_le()?;
        let offset = reader.read_u32_le()?;
        let memory_index = reader.read_u32_le()?;
        Ok(Self { align_exponent, offset, memory_index })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

impl Checksummable for MemArg {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.align_exponent.update_checksum(checksum);
        self.offset.update_checksum(checksum);
        self.memory_index.update_checksum(checksum);
    }
}

/// Data segment mode for WebAssembly modules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataMode {
    /// Active data segment - loaded at module instantiation
    Active {
        /// Memory index where data is loaded
        memory_index: u32,
        /// Offset expression where data is loaded
        offset: u32,
    },
    /// Passive data segment - loaded explicitly via memory.init
    Passive,
}

impl Default for DataMode {
    fn default() -> Self {
        Self::Passive
    }
}

impl Checksummable for DataMode {
    fn update_checksum(&self, checksum: &mut Checksum) {
        match self {
            Self::Active { memory_index, offset } => {
                checksum.update_slice(&[0u8]);
                memory_index.update_checksum(checksum);
                offset.update_checksum(checksum);
            }
            Self::Passive => {
                checksum.update_slice(&[1u8]);
            }
        }
    }
}

/// Element segment mode for WebAssembly modules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementMode {
    /// Active element segment - loaded at module instantiation
    Active {
        /// Table index where elements are loaded
        table_index: u32,
        /// Offset expression where elements are loaded
        offset: u32,
    },
    /// Passive element segment - loaded explicitly via table.init
    Passive,
    /// Declarative element segment - used for validation only
    Declarative,
}

impl Default for ElementMode {
    fn default() -> Self {
        Self::Passive
    }
}

impl Checksummable for ElementMode {
    fn update_checksum(&self, checksum: &mut Checksum) {
        match self {
            Self::Active { table_index, offset } => {
                checksum.update_slice(&[0u8]);
                table_index.update_checksum(checksum);
                offset.update_checksum(checksum);
            }
            Self::Passive => {
                checksum.update_slice(&[1u8]);
            }
            Self::Declarative => {
                checksum.update_slice(&[2u8]);
            }
        }
    }
}

/// A WebAssembly instruction (basic placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction<P: MemoryProvider + Clone + core::fmt::Debug + PartialEq + Eq + Default> {
    Unreachable,
    Nop,
    Block {
        block_type_idx: u32,
    }, // Block with type index
    Loop {
        block_type_idx: u32,
    }, // Loop with type index
    If {
        block_type_idx: u32,
    }, // If with type index
    Else,
    End,
    Br(LabelIdx),
    BrIf(LabelIdx),
    BrTable {
        targets: BoundedVec<LabelIdx, MAX_BR_TABLE_TARGETS, P>,
        default_target: LabelIdx,
    },
    Return,
    Call(FuncIdx),
    CallIndirect(TypeIdx, TableIdx),

    // Tail call instructions (0x12 and 0x13 opcodes)
    ReturnCall(FuncIdx),
    ReturnCallIndirect(TypeIdx, TableIdx),

    // Branch hinting instructions (0xD5 and 0xD6 opcodes)
    BrOnNull(LabelIdx),
    BrOnNonNull(LabelIdx),

    // Type reflection instructions
    RefIsNull,
    RefAsNonNull,
    RefEq,

    // Placeholder for more instructions
    LocalGet(LocalIdx),
    LocalSet(LocalIdx),
    LocalTee(LocalIdx),
    GlobalGet(GlobalIdx),
    GlobalSet(GlobalIdx),

    I32Const(i32),
    I64Const(i64),
    F32Const(u32), // bits representation
    F64Const(u64), // bits representation

    // Memory operations
    I32Load(MemArg),
    I64Load(MemArg),
    F32Load(MemArg),
    F64Load(MemArg),
    I32Load8S(MemArg),
    I32Load8U(MemArg),
    I32Load16S(MemArg),
    I32Load16U(MemArg),
    I64Load8S(MemArg),
    I64Load8U(MemArg),
    I64Load16S(MemArg),
    I64Load16U(MemArg),
    I64Load32S(MemArg),
    I64Load32U(MemArg),

    I32Store(MemArg),
    I64Store(MemArg),
    F32Store(MemArg),
    F64Store(MemArg),
    I32Store8(MemArg),
    I32Store16(MemArg),
    I64Store8(MemArg),
    I64Store16(MemArg),
    I64Store32(MemArg),

    // Memory size and grow
    MemorySize(u32),      // memory index
    MemoryGrow(u32),      // memory index
    MemoryFill(u32),      // memory index
    MemoryCopy(u32, u32), // dst_mem, src_mem
    MemoryInit(u32, u32), // data_seg_idx, mem_idx
    DataDrop(u32),        // data segment index

    // Table operations
    TableGet(u32),       // table index
    TableSet(u32),       // table index
    TableSize(u32),      // table index
    TableGrow(u32),      // table index
    TableFill(u32),      // table index
    TableCopy(u32, u32), // dst_table, src_table
    TableInit(u32, u32), // elem_seg_idx, table_idx
    ElemDrop(u32),       // element segment index

    // Stack operations
    Drop,
    Select,
    SelectWithType(BoundedVec<ValueType, 1, P>), // typed select

    // Arithmetic operations
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,

    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,

    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,

    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,

    // Comparison operations
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,

    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,

    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,

    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,

    // Unary test operations
    I32Eqz,
    I64Eqz,

    // Conversion operations
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,

    // Sign extension operations
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,

    // Reference operations
    RefNull(RefType),
    RefFunc(FuncIdx),

    // Other operations
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I64Clz,
    I64Ctz,
    I64Popcnt,

    // Atomic memory operations (0xFE prefix in WebAssembly)
    MemoryAtomicNotify {
        memarg: MemArg,
    },
    MemoryAtomicWait32 {
        memarg: MemArg,
    },
    MemoryAtomicWait64 {
        memarg: MemArg,
    },

    // Atomic loads
    I32AtomicLoad {
        memarg: MemArg,
    },
    I64AtomicLoad {
        memarg: MemArg,
    },
    I32AtomicLoad8U {
        memarg: MemArg,
    },
    I32AtomicLoad16U {
        memarg: MemArg,
    },
    I64AtomicLoad8U {
        memarg: MemArg,
    },
    I64AtomicLoad16U {
        memarg: MemArg,
    },
    I64AtomicLoad32U {
        memarg: MemArg,
    },

    // Atomic stores
    I32AtomicStore {
        memarg: MemArg,
    },
    I64AtomicStore {
        memarg: MemArg,
    },
    I32AtomicStore8 {
        memarg: MemArg,
    },
    I32AtomicStore16 {
        memarg: MemArg,
    },
    I64AtomicStore8 {
        memarg: MemArg,
    },
    I64AtomicStore16 {
        memarg: MemArg,
    },
    I64AtomicStore32 {
        memarg: MemArg,
    },

    // Atomic read-modify-write operations
    I32AtomicRmwAdd {
        memarg: MemArg,
    },
    I64AtomicRmwAdd {
        memarg: MemArg,
    },
    I32AtomicRmw8AddU {
        memarg: MemArg,
    },
    I32AtomicRmw16AddU {
        memarg: MemArg,
    },
    I64AtomicRmw8AddU {
        memarg: MemArg,
    },
    I64AtomicRmw16AddU {
        memarg: MemArg,
    },
    I64AtomicRmw32AddU {
        memarg: MemArg,
    },

    I32AtomicRmwSub {
        memarg: MemArg,
    },
    I64AtomicRmwSub {
        memarg: MemArg,
    },
    I32AtomicRmw8SubU {
        memarg: MemArg,
    },
    I32AtomicRmw16SubU {
        memarg: MemArg,
    },
    I64AtomicRmw8SubU {
        memarg: MemArg,
    },
    I64AtomicRmw16SubU {
        memarg: MemArg,
    },
    I64AtomicRmw32SubU {
        memarg: MemArg,
    },

    I32AtomicRmwAnd {
        memarg: MemArg,
    },
    I64AtomicRmwAnd {
        memarg: MemArg,
    },
    I32AtomicRmw8AndU {
        memarg: MemArg,
    },
    I32AtomicRmw16AndU {
        memarg: MemArg,
    },
    I64AtomicRmw8AndU {
        memarg: MemArg,
    },
    I64AtomicRmw16AndU {
        memarg: MemArg,
    },
    I64AtomicRmw32AndU {
        memarg: MemArg,
    },

    I32AtomicRmwOr {
        memarg: MemArg,
    },
    I64AtomicRmwOr {
        memarg: MemArg,
    },
    I32AtomicRmw8OrU {
        memarg: MemArg,
    },
    I32AtomicRmw16OrU {
        memarg: MemArg,
    },
    I64AtomicRmw8OrU {
        memarg: MemArg,
    },
    I64AtomicRmw16OrU {
        memarg: MemArg,
    },
    I64AtomicRmw32OrU {
        memarg: MemArg,
    },

    I32AtomicRmwXor {
        memarg: MemArg,
    },
    I64AtomicRmwXor {
        memarg: MemArg,
    },
    I32AtomicRmw8XorU {
        memarg: MemArg,
    },
    I32AtomicRmw16XorU {
        memarg: MemArg,
    },
    I64AtomicRmw8XorU {
        memarg: MemArg,
    },
    I64AtomicRmw16XorU {
        memarg: MemArg,
    },
    I64AtomicRmw32XorU {
        memarg: MemArg,
    },

    I32AtomicRmwXchg {
        memarg: MemArg,
    },
    I64AtomicRmwXchg {
        memarg: MemArg,
    },
    I32AtomicRmw8XchgU {
        memarg: MemArg,
    },
    I32AtomicRmw16XchgU {
        memarg: MemArg,
    },
    I64AtomicRmw8XchgU {
        memarg: MemArg,
    },
    I64AtomicRmw16XchgU {
        memarg: MemArg,
    },
    I64AtomicRmw32XchgU {
        memarg: MemArg,
    },

    // Atomic compare-exchange operations
    I32AtomicRmwCmpxchg {
        memarg: MemArg,
    },
    I64AtomicRmwCmpxchg {
        memarg: MemArg,
    },
    I32AtomicRmw8CmpxchgU {
        memarg: MemArg,
    },
    I32AtomicRmw16CmpxchgU {
        memarg: MemArg,
    },
    I64AtomicRmw8CmpxchgU {
        memarg: MemArg,
    },
    I64AtomicRmw16CmpxchgU {
        memarg: MemArg,
    },
    I64AtomicRmw32CmpxchgU {
        memarg: MemArg,
    },

    // Atomic fence
    AtomicFence,

    #[doc(hidden)]
    _Phantom(core::marker::PhantomData<P>),
}

impl<P: MemoryProvider + Clone + core::fmt::Debug + PartialEq + Eq + Default> Default
    for Instruction<P>
{
    fn default() -> Self {
        Instruction::Nop // Nop is a safe default
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq + Default>
    Checksummable for Instruction<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        // This is a complex operation, as each instruction variant has a different
        // binary representation. For a robust checksum, each variant should be
        // serialized to its byte form and then update the checksum.
        // Placeholder: update with a discriminant or simple representation.
        match self {
            Instruction::Unreachable => checksum.update_slice(&[0x00]),
            Instruction::Nop => checksum.update_slice(&[0x01]),
            Instruction::Block { block_type_idx } => {
                checksum.update_slice(&[0x02]);
                block_type_idx.update_checksum(checksum);
            }
            Instruction::Loop { block_type_idx } => {
                checksum.update_slice(&[0x03]);
                block_type_idx.update_checksum(checksum);
            }
            Instruction::If { block_type_idx } => {
                checksum.update_slice(&[0x04]);
                block_type_idx.update_checksum(checksum);
            }
            Instruction::Else => checksum.update_slice(&[0x05]),
            Instruction::End => checksum.update_slice(&[0x0B]),
            Instruction::Br(idx) => {
                checksum.update_slice(&[0x0C]);
                idx.update_checksum(checksum);
            }
            Instruction::BrIf(idx) => {
                checksum.update_slice(&[0x0D]);
                idx.update_checksum(checksum);
            }
            Instruction::BrTable { targets, default_target } => {
                checksum.update_slice(&[0x0E]);
                targets.update_checksum(checksum);
                default_target.update_checksum(checksum);
            }
            Instruction::Return => checksum.update_slice(&[0x0F]),
            Instruction::Call(idx) => {
                checksum.update_slice(&[0x10]);
                idx.update_checksum(checksum);
            }
            Instruction::CallIndirect(type_idx, table_idx) => {
                checksum.update_slice(&[0x11]);
                type_idx.update_checksum(checksum);
                table_idx.update_checksum(checksum);
            }
            Instruction::ReturnCall(func_idx) => {
                checksum.update_slice(&[0x12]); // Tail call opcode
                func_idx.update_checksum(checksum);
            }
            Instruction::ReturnCallIndirect(type_idx, table_idx) => {
                checksum.update_slice(&[0x13]); // Tail call indirect opcode
                type_idx.update_checksum(checksum);
                table_idx.update_checksum(checksum);
            }
            Instruction::BrOnNull(label_idx) => {
                checksum.update_slice(&[0xD5]); // br_on_null opcode
                label_idx.update_checksum(checksum);
            }
            Instruction::BrOnNonNull(label_idx) => {
                checksum.update_slice(&[0xD6]); // br_on_non_null opcode
                label_idx.update_checksum(checksum);
            }
            Instruction::RefIsNull => {
                checksum.update_slice(&[0xD1]); // ref.is_null opcode
            }
            Instruction::RefAsNonNull => {
                checksum.update_slice(&[0xD3]); // ref.as_non_null opcode
            }
            Instruction::RefEq => {
                checksum.update_slice(&[0xD2]); // ref.eq opcode
            }
            Instruction::LocalGet(idx)
            | Instruction::LocalSet(idx)
            | Instruction::LocalTee(idx) => {
                checksum.update_slice(&[if matches!(self, Instruction::LocalGet(_)) {
                    0x20
                } else if matches!(self, Instruction::LocalSet(_)) {
                    0x21
                } else {
                    0x22
                }]);
                idx.update_checksum(checksum);
            }
            Instruction::GlobalGet(idx) | Instruction::GlobalSet(idx) => {
                checksum.update_slice(&[if matches!(self, Instruction::GlobalGet(_)) {
                    0x23
                } else {
                    0x24
                }]);
                idx.update_checksum(checksum);
            }
            Instruction::I32Const(val) => {
                checksum.update_slice(&[0x41]);
                val.update_checksum(checksum);
            }
            Instruction::I64Const(val) => {
                checksum.update_slice(&[0x42]);
                val.update_checksum(checksum);
            }
            Instruction::F32Const(val) => {
                checksum.update_slice(&[0x43]);
                val.update_checksum(checksum);
            }
            Instruction::F64Const(val) => {
                checksum.update_slice(&[0x44]);
                val.update_checksum(checksum);
            }

            // Memory operations
            Instruction::I32Load(memarg) => {
                checksum.update_slice(&[0x28]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64Load(memarg) => {
                checksum.update_slice(&[0x29]);
                memarg.update_checksum(checksum);
            }
            Instruction::F32Load(memarg) => {
                checksum.update_slice(&[0x2A]);
                memarg.update_checksum(checksum);
            }
            Instruction::F64Load(memarg) => {
                checksum.update_slice(&[0x2B]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32Store(memarg) => {
                checksum.update_slice(&[0x36]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64Store(memarg) => {
                checksum.update_slice(&[0x37]);
                memarg.update_checksum(checksum);
            }
            Instruction::F32Store(memarg) => {
                checksum.update_slice(&[0x38]);
                memarg.update_checksum(checksum);
            }
            Instruction::F64Store(memarg) => {
                checksum.update_slice(&[0x39]);
                memarg.update_checksum(checksum);
            }
            Instruction::MemorySize(mem_idx) => {
                checksum.update_slice(&[0x3F]);
                mem_idx.update_checksum(checksum);
            }
            Instruction::MemoryGrow(mem_idx) => {
                checksum.update_slice(&[0x40]);
                mem_idx.update_checksum(checksum);
            }

            // Arithmetic operations
            Instruction::I32Add => checksum.update_slice(&[0x6A]),
            Instruction::I32Sub => checksum.update_slice(&[0x6B]),
            Instruction::I32Mul => checksum.update_slice(&[0x6C]),
            Instruction::I32DivS => checksum.update_slice(&[0x6D]),
            Instruction::I32DivU => checksum.update_slice(&[0x6E]),
            Instruction::I64Add => checksum.update_slice(&[0x7C]),
            Instruction::I64Sub => checksum.update_slice(&[0x7D]),

            // Comparison operations
            Instruction::I32Eq => checksum.update_slice(&[0x46]),
            Instruction::I32Ne => checksum.update_slice(&[0x47]),
            Instruction::I32LtS => checksum.update_slice(&[0x48]),

            // Stack operations
            Instruction::Drop => checksum.update_slice(&[0x1A]),
            Instruction::Select => checksum.update_slice(&[0x1B]),

            // Atomic memory operations (0xFE prefix in WebAssembly)
            Instruction::MemoryAtomicNotify { memarg } => {
                checksum.update_slice(&[0xFE, 0x00]);
                memarg.update_checksum(checksum);
            }
            Instruction::MemoryAtomicWait32 { memarg } => {
                checksum.update_slice(&[0xFE, 0x01]);
                memarg.update_checksum(checksum);
            }
            Instruction::MemoryAtomicWait64 { memarg } => {
                checksum.update_slice(&[0xFE, 0x02]);
                memarg.update_checksum(checksum);
            }

            // Atomic loads
            Instruction::I32AtomicLoad { memarg } => {
                checksum.update_slice(&[0xFE, 0x10]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicLoad { memarg } => {
                checksum.update_slice(&[0xFE, 0x11]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicLoad8U { memarg } => {
                checksum.update_slice(&[0xFE, 0x12]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicLoad16U { memarg } => {
                checksum.update_slice(&[0xFE, 0x13]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicLoad8U { memarg } => {
                checksum.update_slice(&[0xFE, 0x14]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicLoad16U { memarg } => {
                checksum.update_slice(&[0xFE, 0x15]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicLoad32U { memarg } => {
                checksum.update_slice(&[0xFE, 0x16]);
                memarg.update_checksum(checksum);
            }

            // Atomic stores
            Instruction::I32AtomicStore { memarg } => {
                checksum.update_slice(&[0xFE, 0x17]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicStore { memarg } => {
                checksum.update_slice(&[0xFE, 0x18]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicStore8 { memarg } => {
                checksum.update_slice(&[0xFE, 0x19]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicStore16 { memarg } => {
                checksum.update_slice(&[0xFE, 0x1a]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicStore8 { memarg } => {
                checksum.update_slice(&[0xFE, 0x1b]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicStore16 { memarg } => {
                checksum.update_slice(&[0xFE, 0x1c]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicStore32 { memarg } => {
                checksum.update_slice(&[0xFE, 0x1d]);
                memarg.update_checksum(checksum);
            }

            // Atomic read-modify-write operations
            Instruction::I32AtomicRmwAdd { memarg } => {
                checksum.update_slice(&[0xFE, 0x1e]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwAdd { memarg } => {
                checksum.update_slice(&[0xFE, 0x1f]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8AddU { memarg } => {
                checksum.update_slice(&[0xFE, 0x20]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16AddU { memarg } => {
                checksum.update_slice(&[0xFE, 0x21]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8AddU { memarg } => {
                checksum.update_slice(&[0xFE, 0x22]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16AddU { memarg } => {
                checksum.update_slice(&[0xFE, 0x23]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32AddU { memarg } => {
                checksum.update_slice(&[0xFE, 0x24]);
                memarg.update_checksum(checksum);
            }

            Instruction::I32AtomicRmwSub { memarg } => {
                checksum.update_slice(&[0xFE, 0x25]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwSub { memarg } => {
                checksum.update_slice(&[0xFE, 0x26]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8SubU { memarg } => {
                checksum.update_slice(&[0xFE, 0x27]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16SubU { memarg } => {
                checksum.update_slice(&[0xFE, 0x28]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8SubU { memarg } => {
                checksum.update_slice(&[0xFE, 0x29]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16SubU { memarg } => {
                checksum.update_slice(&[0xFE, 0x2a]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32SubU { memarg } => {
                checksum.update_slice(&[0xFE, 0x2b]);
                memarg.update_checksum(checksum);
            }

            Instruction::I32AtomicRmwAnd { memarg } => {
                checksum.update_slice(&[0xFE, 0x2c]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwAnd { memarg } => {
                checksum.update_slice(&[0xFE, 0x2d]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8AndU { memarg } => {
                checksum.update_slice(&[0xFE, 0x2e]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16AndU { memarg } => {
                checksum.update_slice(&[0xFE, 0x2f]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8AndU { memarg } => {
                checksum.update_slice(&[0xFE, 0x30]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16AndU { memarg } => {
                checksum.update_slice(&[0xFE, 0x31]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32AndU { memarg } => {
                checksum.update_slice(&[0xFE, 0x32]);
                memarg.update_checksum(checksum);
            }

            Instruction::I32AtomicRmwOr { memarg } => {
                checksum.update_slice(&[0xFE, 0x33]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwOr { memarg } => {
                checksum.update_slice(&[0xFE, 0x34]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8OrU { memarg } => {
                checksum.update_slice(&[0xFE, 0x35]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16OrU { memarg } => {
                checksum.update_slice(&[0xFE, 0x36]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8OrU { memarg } => {
                checksum.update_slice(&[0xFE, 0x37]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16OrU { memarg } => {
                checksum.update_slice(&[0xFE, 0x38]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32OrU { memarg } => {
                checksum.update_slice(&[0xFE, 0x39]);
                memarg.update_checksum(checksum);
            }

            Instruction::I32AtomicRmwXor { memarg } => {
                checksum.update_slice(&[0xFE, 0x3a]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwXor { memarg } => {
                checksum.update_slice(&[0xFE, 0x3b]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8XorU { memarg } => {
                checksum.update_slice(&[0xFE, 0x3c]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16XorU { memarg } => {
                checksum.update_slice(&[0xFE, 0x3d]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8XorU { memarg } => {
                checksum.update_slice(&[0xFE, 0x3e]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16XorU { memarg } => {
                checksum.update_slice(&[0xFE, 0x3f]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32XorU { memarg } => {
                checksum.update_slice(&[0xFE, 0x40]);
                memarg.update_checksum(checksum);
            }

            Instruction::I32AtomicRmwXchg { memarg } => {
                checksum.update_slice(&[0xFE, 0x41]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwXchg { memarg } => {
                checksum.update_slice(&[0xFE, 0x42]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8XchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x43]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16XchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x44]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8XchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x45]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16XchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x46]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32XchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x47]);
                memarg.update_checksum(checksum);
            }

            // Atomic compare-exchange operations
            Instruction::I32AtomicRmwCmpxchg { memarg } => {
                checksum.update_slice(&[0xFE, 0x48]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmwCmpxchg { memarg } => {
                checksum.update_slice(&[0xFE, 0x49]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw8CmpxchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x4a]);
                memarg.update_checksum(checksum);
            }
            Instruction::I32AtomicRmw16CmpxchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x4b]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw8CmpxchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x4c]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw16CmpxchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x4d]);
                memarg.update_checksum(checksum);
            }
            Instruction::I64AtomicRmw32CmpxchgU { memarg } => {
                checksum.update_slice(&[0xFE, 0x4e]);
                memarg.update_checksum(checksum);
            }

            // Atomic fence
            Instruction::AtomicFence => {
                checksum.update_slice(&[0xFE, 0x03]);
            }

            // All other instructions - use a placeholder checksum for now
            _ => {
                // For now, just use a simple placeholder
                // This is a placeholder until all instructions are properly implemented
                checksum.update_slice(&[0xFF, 0x00]);
            }
        }
    }
}

impl<PInstr: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq + Default> ToBytes
    for Instruction<PInstr>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        // Actual serialization logic for instructions
        // This will be complex and depends on the instruction format.
        // For now, a placeholder.
        match self {
            Instruction::Unreachable => writer.write_u8(0x00)?,
            Instruction::Nop => writer.write_u8(0x01)?,
            Instruction::Block { block_type_idx } => {
                writer.write_u8(0x02)?;
                writer.write_u32_le(*block_type_idx)?;
            }
            Instruction::Loop { block_type_idx } => {
                writer.write_u8(0x03)?;
                writer.write_u32_le(*block_type_idx)?;
            }
            Instruction::If { block_type_idx } => {
                writer.write_u8(0x04)?;
                writer.write_u32_le(*block_type_idx)?;
            }
            Instruction::Else => writer.write_u8(0x05)?,
            Instruction::End => writer.write_u8(0x0B)?,
            Instruction::Br(idx) => {
                writer.write_u8(0x0C)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::BrIf(idx) => {
                writer.write_u8(0x0D)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::BrTable { targets, default_target } => {
                writer.write_u8(0x0E)?;
                targets.to_bytes_with_provider(writer, stream_provider)?;
                writer.write_u32_le(*default_target)?;
            }
            Instruction::Return => writer.write_u8(0x0F)?,
            Instruction::Call(idx) => {
                writer.write_u8(0x10)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::CallIndirect(type_idx, table_idx) => {
                writer.write_u8(0x11)?;
                writer.write_u32_le(*type_idx)?;
                writer.write_u32_le(*table_idx)?;
            }
            Instruction::ReturnCall(idx) => {
                writer.write_u8(0x12)?; // Tail call opcode
                writer.write_u32_le(*idx)?;
            }
            Instruction::ReturnCallIndirect(type_idx, table_idx) => {
                writer.write_u8(0x13)?; // Tail call indirect opcode
                writer.write_u32_le(*type_idx)?;
                writer.write_u32_le(*table_idx)?;
            }
            Instruction::BrOnNull(label_idx) => {
                writer.write_u8(0xD5)?; // br_on_null opcode
                writer.write_u32_le(*label_idx)?;
            }
            Instruction::BrOnNonNull(label_idx) => {
                writer.write_u8(0xD6)?; // br_on_non_null opcode
                writer.write_u32_le(*label_idx)?;
            }
            Instruction::RefIsNull => writer.write_u8(0xD1)?, // ref.is_null opcode
            Instruction::RefAsNonNull => writer.write_u8(0xD3)?, // ref.as_non_null opcode
            Instruction::RefEq => writer.write_u8(0xD2)?,     // ref.eq opcode
            Instruction::LocalGet(idx) => {
                writer.write_u8(0x20)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::LocalSet(idx) => {
                writer.write_u8(0x21)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::LocalTee(idx) => {
                writer.write_u8(0x22)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::GlobalGet(idx) => {
                writer.write_u8(0x23)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::GlobalSet(idx) => {
                writer.write_u8(0x24)?;
                writer.write_u32_le(*idx)?;
            }
            Instruction::I32Const(val) => {
                writer.write_u8(0x41)?;
                writer.write_i32_le(*val)?;
            }
            Instruction::I64Const(val) => {
                writer.write_u8(0x42)?;
                writer.write_i64_le(*val)?;
            }
            // Atomic memory operations (0xFE prefix in WebAssembly)
            Instruction::MemoryAtomicNotify { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x00)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::MemoryAtomicWait32 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x01)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::MemoryAtomicWait64 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x02)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            // Atomic loads
            Instruction::I32AtomicLoad { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x10)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicLoad { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x11)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicLoad8U { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x12)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicLoad16U { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x13)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicLoad8U { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x14)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicLoad16U { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x15)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicLoad32U { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x16)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            // Atomic stores
            Instruction::I32AtomicStore { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x17)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicStore { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x18)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicStore8 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x19)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicStore16 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x1a)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicStore8 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x1b)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicStore16 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x1c)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicStore32 { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x1d)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            // Atomic read-modify-write operations
            Instruction::I32AtomicRmwAdd { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x1e)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwAdd { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x1f)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8AddU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x20)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16AddU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x21)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8AddU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x22)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16AddU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x23)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32AddU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x24)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            Instruction::I32AtomicRmwSub { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x25)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwSub { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x26)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8SubU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x27)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16SubU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x28)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8SubU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x29)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16SubU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x2a)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32SubU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x2b)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            Instruction::I32AtomicRmwAnd { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x2c)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwAnd { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x2d)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8AndU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x2e)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16AndU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x2f)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8AndU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x30)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16AndU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x31)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32AndU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x32)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            Instruction::I32AtomicRmwOr { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x33)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwOr { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x34)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8OrU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x35)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16OrU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x36)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8OrU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x37)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16OrU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x38)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32OrU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x39)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            Instruction::I32AtomicRmwXor { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x3a)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwXor { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x3b)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8XorU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x3c)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16XorU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x3d)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8XorU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x3e)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16XorU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x3f)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32XorU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x40)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            Instruction::I32AtomicRmwXchg { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x41)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwXchg { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x42)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8XchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x43)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16XchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x44)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8XchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x45)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16XchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x46)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32XchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x47)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            // Atomic compare-exchange operations
            Instruction::I32AtomicRmwCmpxchg { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x48)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmwCmpxchg { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x49)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw8CmpxchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x4a)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I32AtomicRmw16CmpxchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x4b)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw8CmpxchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x4c)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw16CmpxchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x4d)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }
            Instruction::I64AtomicRmw32CmpxchgU { memarg } => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x4e)?;
                memarg.to_bytes_with_provider(writer, stream_provider)?;
            }

            // Atomic fence
            Instruction::AtomicFence => {
                writer.write_u8(0xFE)?;
                writer.write_u8(0x03)?;
            }

            // ... many more instructions
            Instruction::_Phantom(_) => {
                // This variant should not be serialized
                return Err(SerializationError::Custom(
                    "Cannot serialize _Phantom instruction variant",
                )
                .into());
            }

            // Catch-all for all other instruction variants
            _ => {
                // For now, return an error for unimplemented instructions
                // This is a placeholder - a complete implementation would handle all variants
                return Err(SerializationError::Custom(
                    "Instruction variant not yet implemented for serialization",
                )
                .into());
            }
        }
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl<PInstr: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq + Default>
    FromBytes for Instruction<PInstr>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        // Actual deserialization logic
        // Placeholder
        let opcode = reader.read_u8()?;
        match opcode {
            0x00 => Ok(Instruction::Unreachable),
            0x01 => Ok(Instruction::Nop),
            0x02 => {
                let block_type_idx = reader.read_u32_le()?;
                Ok(Instruction::Block { block_type_idx })
            }
            0x03 => {
                let block_type_idx = reader.read_u32_le()?;
                Ok(Instruction::Loop { block_type_idx })
            }
            0x04 => {
                let block_type_idx = reader.read_u32_le()?;
                Ok(Instruction::If { block_type_idx })
            }
            0x05 => Ok(Instruction::Else),
            0x0B => Ok(Instruction::End),
            0x0C => Ok(Instruction::Br(reader.read_u32_le()?)),
            0x0D => Ok(Instruction::BrIf(reader.read_u32_le()?)),
            0x0E => {
                let targets = BoundedVec::from_bytes_with_provider(reader, stream_provider)?;
                let default_target = reader.read_u32_le()?;
                Ok(Instruction::BrTable { targets, default_target })
            }
            0x0F => Ok(Instruction::Return),
            0x10 => Ok(Instruction::Call(reader.read_u32_le()?)),
            0x11 => Ok(Instruction::CallIndirect(reader.read_u32_le()?, reader.read_u32_le()?)),
            0x20 => Ok(Instruction::LocalGet(reader.read_u32_le()?)),
            0x21 => Ok(Instruction::LocalSet(reader.read_u32_le()?)),
            0x22 => Ok(Instruction::LocalTee(reader.read_u32_le()?)),
            0x23 => Ok(Instruction::GlobalGet(reader.read_u32_le()?)),
            0x24 => Ok(Instruction::GlobalSet(reader.read_u32_le()?)),
            0x41 => Ok(Instruction::I32Const(reader.read_i32_le()?)),
            0x42 => Ok(Instruction::I64Const(reader.read_i64_le()?)),
            // ... many more instructions
            _ => Err(SerializationError::InvalidFormat.into()),
        }
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

pub type InstructionSequence<P> = BoundedVec<Instruction<P>, MAX_INSTRUCTIONS_PER_FUNCTION, P>;

/// Represents a local variable entry in a function body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LocalEntry {
    pub count: u32,
    pub value_type: ValueType,
}

impl Checksummable for LocalEntry {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.count.update_checksum(checksum);
        self.value_type.update_checksum(checksum);
    }
}

impl ToBytes for LocalEntry {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        writer.write_u32_le(self.count)?;
        self.value_type.to_bytes_with_provider(writer, stream_provider)?;
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &provider)
    }
}

impl FromBytes for LocalEntry {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let count = reader.read_u32_le()?;
        let value_type = ValueType::from_bytes_with_provider(reader, stream_provider)?;
        Ok(LocalEntry { count, value_type })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &provider)
    }
}

/// Represents a custom section in a WebAssembly module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomSection<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> {
    pub name: WasmName<MAX_WASM_NAME_LENGTH, P>,
    pub data: BoundedVec<u8, MAX_CUSTOM_SECTION_DATA_SIZE, P>,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for CustomSection<P>
{
    fn default() -> Self {
        Self {
            name: WasmName::default(), // Requires P: Default + Clone
            data: BoundedVec::new(P::default())
                .expect("Default BoundedVec for CustomSection data failed"),
        }
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> CustomSection<P> {
    /// Creates a new `CustomSection` from a name and data.
    pub fn new(provider: P, name_str: &str, data: &[u8]) -> Result<Self> {
        // Create WasmName for the section name
        let name = WasmName::from_str_truncate(name_str, P::default()) // Assuming P can be defaulted here for WasmName
            .map_err(|e| {
                // Log or convert BoundedError to crate::Error
                // For now, creating a generic error:
                Error::new(
                    ErrorCategory::Memory, /* Or a more specific category like `Resource` or
                                            * `Validation` */
                    wrt_error::codes::SYSTEM_ERROR, // Was INTERNAL_ERROR
                    "Failed to create WasmName for custom section name",
                )
            })?;

        // Create BoundedVec for the section data
        let mut data_vec = BoundedVec::<u8, MAX_CUSTOM_SECTION_DATA_SIZE, P>::new(provider.clone()) // Use cloned provider for data_vec
            .map_err(|e| {
                // Log or convert WrtError from BoundedVec::new to crate::Error
                Error::new(
                    ErrorCategory::Memory,
                    wrt_error::codes::SYSTEM_ERROR, // Was INTERNAL_ERROR
                    "Failed to initialize BoundedVec for custom section data",
                )
            })?;

        data_vec.try_extend_from_slice(data).map_err(|e| {
            // Log or convert BoundedError from try_extend_from_slice to crate::Error
            Error::new(
                ErrorCategory::Memory,
                wrt_error::codes::SYSTEM_ERROR, // Was INTERNAL_ERROR
                "Failed to extend BoundedVec for custom section data",
            )
        })?;

        Ok(Self { name, data: data_vec })
    }

    /// Creates a new `CustomSection` from a name string and a data slice,
    /// assuming a default provider can be obtained.
    /// This is a convenience function and might only be suitable for `std` or
    /// test environments.
    ///
    /// # Errors
    ///
    /// Returns an error if the name or data cannot be stored due to capacity
    /// limits.
    #[cfg(any(feature = "std", test))]
    pub fn from_name_and_data(name_str: &str, data_slice: &[u8]) -> Result<Self>
    where
        P: Default, // Ensure P can be defaulted for this convenience function
    {
        let provider = P::default();
        let name = WasmName::from_str_truncate(name_str, provider.clone()).map_err(|_| {
            Error::new(
                ErrorCategory::Memory,
                wrt_error::codes::SYSTEM_ERROR, // Was INTERNAL_ERROR
                "Failed to create WasmName for custom section",
            )
        })?;

        let mut data_bounded_vec = BoundedVec::<u8, MAX_CUSTOM_SECTION_DATA_SIZE, P>::new(provider)
            .map_err(|_| {
                Error::new(
                    ErrorCategory::Memory,
                    wrt_error::codes::SYSTEM_ERROR, // Was INTERNAL_ERROR
                    "Failed to create BoundedVec for custom section data",
                )
            })?;

        data_bounded_vec.try_extend_from_slice(data_slice).map_err(|_| {
            Error::new(
                ErrorCategory::Memory,
                wrt_error::codes::SYSTEM_ERROR, // Was INTERNAL_ERROR
                "Failed to extend BoundedVec for custom section data",
            )
        })?;

        Ok(CustomSection { name, data: data_bounded_vec })
    }

    pub fn name_as_str(&self) -> core::result::Result<&str, BoundedError> {
        self.name.as_str()
    }

    pub fn data(&self) -> BoundedVec<u8, MAX_CUSTOM_SECTION_DATA_SIZE, P> {
        self.data.clone()
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for CustomSection<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.name.update_checksum(checksum);
        self.data.update_checksum(checksum);
    }
}

impl<PCustom: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ToBytes
    for CustomSection<PCustom>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        self.name.to_bytes_with_provider(writer, stream_provider)?;
        self.data.to_bytes_with_provider(writer, stream_provider)?;
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &provider)
    }
}

impl<PCustom: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FromBytes
    for CustomSection<PCustom>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let name = WasmName::<MAX_WASM_NAME_LENGTH, PCustom>::from_bytes_with_provider(
            reader,
            stream_provider,
        )?;
        let data =
            BoundedVec::<u8, MAX_CUSTOM_SECTION_DATA_SIZE, PCustom>::from_bytes_with_provider(
                reader,
                stream_provider,
            )?;
        Ok(CustomSection { name, data })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &provider)
    }
}

/// Represents the body of a WebAssembly function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncBody<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> {
    /// Local variable declarations.
    pub locals: BoundedVec<LocalEntry, MAX_LOCALS_PER_FUNCTION, P>,
    /// The sequence of instructions (the function's code).
    pub body: InstructionSequence<P>,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for FuncBody<P>
{
    fn default() -> Self {
        Self {
            locals: BoundedVec::new(P::default()).expect("Default BoundedVec for locals failed"),
            body: BoundedVec::new(P::default()).expect("Default BoundedVec for body failed"),
        }
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for FuncBody<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.locals.update_checksum(checksum);
        self.body.update_checksum(checksum);
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ToBytes
    for FuncBody<P>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.locals.to_bytes_with_provider(writer, provider)?;
        self.body.to_bytes_with_provider(writer, provider)?;
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FromBytes
    for FuncBody<P>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let locals =
            BoundedVec::<LocalEntry, MAX_LOCALS_PER_FUNCTION, P>::from_bytes_with_provider(
                reader, provider,
            )?;
        let body = BoundedVec::<Instruction<P>, MAX_INSTRUCTIONS_PER_FUNCTION, P>::from_bytes_with_provider(reader, provider)?;
        Ok(FuncBody { locals, body })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Represents an import in a WebAssembly module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Import<P: MemoryProvider + Default + Clone + PartialEq + Eq> {
    pub module_name: WasmName<MAX_MODULE_NAME_LEN, P>,
    pub item_name: WasmName<MAX_ITEM_NAME_LEN, P>,
    pub desc: ImportDesc<P>,
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> Default for Import<P> {
    fn default() -> Self {
        Self {
            module_name: WasmName::default(),
            item_name: WasmName::default(),
            desc: ImportDesc::default(),
        }
    }
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> Import<P> {
    /// Creates a new `Import` with the given module name, item name, and import
    /// description.
    pub fn new(
        provider: P,
        module_name_str: &str,
        item_name_str: &str,
        desc: ImportDesc<P>,
    ) -> Result<Self> {
        let module_name =
            WasmName::from_str(module_name_str, provider.clone()).map_err(|e| match e {
                SerializationError::Custom(_) => Error::new(
                    ErrorCategory::Capacity,
                    wrt_error::codes::CAPACITY_EXCEEDED,
                    "Import module name too long",
                ),
                _ => Error::new(
                    ErrorCategory::Validation,
                    wrt_error::codes::INVALID_VALUE,
                    "Failed to create module name for import from SerializationError",
                ),
            })?;
        let item_name = WasmName::from_str(item_name_str, provider).map_err(|e| match e {
            SerializationError::Custom(_) => Error::new(
                ErrorCategory::Capacity,
                wrt_error::codes::CAPACITY_EXCEEDED,
                "Import item name too long",
            ),
            _ => Error::new(
                ErrorCategory::Validation,
                wrt_error::codes::INVALID_VALUE,
                "Failed to create item name for import from SerializationError",
            ),
        })?;
        Ok(Self { module_name, item_name, desc })
    }
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> Checksummable for Import<P> {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.module_name.update_checksum(checksum);
        self.item_name.update_checksum(checksum);
        self.desc.update_checksum(checksum);
    }
}

/// Describes the type of an imported item.
// This enum was previously defined around line 1134. We are making it P-generic here.
// And it will use the newly defined TableType, MemoryType, GlobalType.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportDesc<P: MemoryProvider + PartialEq + Eq> {
    /// An imported function, with its type index.
    Function(TypeIdx),
    /// An imported table.
    Table(TableType), // Uses locally defined TableType
    /// An imported memory.
    Memory(MemoryType), // Uses locally defined MemoryType
    /// An imported global.
    Global(GlobalType), // Uses locally defined GlobalType
    /// An imported external value (used in component model).
    Extern(ExternTypePlaceholder), // Using placeholder
    /// An imported resource (used in component model).
    Resource(ResourceTypePlaceholder), // Using placeholder
    #[doc(hidden)]
    _Phantom(core::marker::PhantomData<P>),
}

impl<P: MemoryProvider + PartialEq + Eq> Checksummable for ImportDesc<P> {
    fn update_checksum(&self, checksum: &mut Checksum) {
        match self {
            ImportDesc::Function(idx) => {
                checksum.update(0);
                idx.update_checksum(checksum);
            }
            ImportDesc::Table(tt) => {
                checksum.update(1);
                tt.update_checksum(checksum);
            }
            ImportDesc::Memory(mt) => {
                checksum.update(2);
                mt.update_checksum(checksum);
            }
            ImportDesc::Global(gt) => {
                checksum.update(3);
                gt.update_checksum(checksum);
            }
            ImportDesc::Extern(etp) => {
                checksum.update(4);
                etp.update_checksum(checksum);
            }
            ImportDesc::Resource(rtp) => {
                checksum.update(5);
                rtp.update_checksum(checksum);
            }
            ImportDesc::_Phantom(_) => { /* No checksum update for phantom data */ }
        }
    }
}

impl<P: MemoryProvider + PartialEq + Eq> Default for ImportDesc<P> {
    fn default() -> Self {
        ImportDesc::Function(0) // Default to function import with type index 0
    }
}

impl<P: MemoryProvider + PartialEq + Eq> ToBytes for ImportDesc<P> {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        match self {
            ImportDesc::Function(idx) => {
                writer.write_u8(0)?; // Tag for Function
                writer.write_u32_le(*idx)?;
            }
            ImportDesc::Table(tt) => {
                writer.write_u8(1)?; // Tag for Table
                tt.to_bytes_with_provider(writer, provider)?;
            }
            ImportDesc::Memory(mt) => {
                writer.write_u8(2)?; // Tag for Memory
                mt.to_bytes_with_provider(writer, provider)?;
            }
            ImportDesc::Global(gt) => {
                writer.write_u8(3)?; // Tag for Global
                gt.to_bytes_with_provider(writer, provider)?;
            }
            ImportDesc::Extern(et) => {
                writer.write_u8(4)?; // Tag for Extern
                et.to_bytes_with_provider(writer, provider)?;
            }
            ImportDesc::Resource(rt) => {
                writer.write_u8(5)?; // Tag for Resource
                rt.to_bytes_with_provider(writer, provider)?;
            }
            ImportDesc::_Phantom(_) => {
                writer.write_u8(255)?; // Tag for phantom (should not occur in
                                       // real data)
            }
        }
        Ok(())
    }
}

impl<P: MemoryProvider + PartialEq + Eq> FromBytes for ImportDesc<P> {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let tag = reader.read_u8()?;
        match tag {
            0 => Ok(ImportDesc::Function(reader.read_u32_le()?)),
            1 => Ok(ImportDesc::Table(TableType::from_bytes_with_provider(reader, provider)?)),
            2 => Ok(ImportDesc::Memory(MemoryType::from_bytes_with_provider(reader, provider)?)),
            3 => Ok(ImportDesc::Global(GlobalType::from_bytes_with_provider(reader, provider)?)),
            4 => Ok(ImportDesc::Extern(ExternTypePlaceholder::from_bytes_with_provider(
                reader, provider,
            )?)),
            5 => Ok(ImportDesc::Resource(ResourceTypePlaceholder::from_bytes_with_provider(
                reader, provider,
            )?)),
            255 => Ok(ImportDesc::_Phantom(core::marker::PhantomData)),
            _ => Err(Error::new_static(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Invalid ImportDesc tag",
            )),
        }
    }
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> ToBytes for Import<P> {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.module_name.to_bytes_with_provider(writer, provider)?;
        self.item_name.to_bytes_with_provider(writer, provider)?;
        self.desc.to_bytes_with_provider(writer, provider)?;
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> FromBytes for Import<P> {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let module_name =
            WasmName::<MAX_MODULE_NAME_LEN, P>::from_bytes_with_provider(reader, stream_provider)?;
        let item_name =
            WasmName::<MAX_ITEM_NAME_LEN, P>::from_bytes_with_provider(reader, stream_provider)?;
        let desc = ImportDesc::<P>::from_bytes_with_provider(reader, stream_provider)?;
        Ok(Import { module_name, item_name, desc })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Describes the kind of an exported item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] // Removed Default
pub enum ExportDesc {
    /// An exported function.
    // Removed #[default]
    Func(FuncIdx),
    /// An exported table.
    Table(TableIdx),
    /// An exported memory.
    Mem(MemIdx),
    /// An exported global.
    Global(GlobalIdx),
    /// An exported tag (exception).
    Tag(TagIdx),
}

impl Default for ExportDesc {
    fn default() -> Self {
        // Default to exporting a function with index 0, as it's common.
        // Or choose a more semantically "empty" or "none" default if applicable.
        ExportDesc::Func(0)
    }
}

impl Checksummable for ExportDesc {
    fn update_checksum(&self, checksum: &mut Checksum) {
        match self {
            ExportDesc::Func(idx) => {
                checksum.update(0x00);
                checksum.update_slice(&idx.to_le_bytes());
            }
            ExportDesc::Table(idx) => {
                checksum.update(0x01);
                checksum.update_slice(&idx.to_le_bytes());
            }
            ExportDesc::Mem(idx) => {
                checksum.update(0x02);
                checksum.update_slice(&idx.to_le_bytes());
            }
            ExportDesc::Global(idx) => {
                checksum.update(0x03);
                checksum.update_slice(&idx.to_le_bytes());
            }
            ExportDesc::Tag(idx) => {
                checksum.update(0x04);
                checksum.update_slice(&idx.to_le_bytes());
            }
        }
    }
}

impl ToBytes for ExportDesc {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream, // Provider not used for u32 or simple enums over u32
    ) -> WrtResult<()> {
        match self {
            ExportDesc::Func(idx) => {
                writer.write_u8(0)?; // Tag for Func
                writer.write_u32_le(*idx)?;
            }
            ExportDesc::Table(idx) => {
                writer.write_u8(1)?; // Tag for Table
                writer.write_u32_le(*idx)?;
            }
            ExportDesc::Mem(idx) => {
                writer.write_u8(2)?; // Tag for Mem
                writer.write_u32_le(*idx)?;
            }
            ExportDesc::Global(idx) => {
                writer.write_u8(3)?; // Tag for Global
                writer.write_u32_le(*idx)?;
            }
            ExportDesc::Tag(idx) => {
                writer.write_u8(4)?; // Tag for Tag
                writer.write_u32_le(*idx)?;
            }
        }
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for ExportDesc {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream, // Provider not used
    ) -> WrtResult<Self> {
        let tag = reader.read_u8()?;
        match tag {
            0 => Ok(ExportDesc::Func(reader.read_u32_le()?)),
            1 => Ok(ExportDesc::Table(reader.read_u32_le()?)),
            2 => Ok(ExportDesc::Mem(reader.read_u32_le()?)),
            3 => Ok(ExportDesc::Global(reader.read_u32_le()?)),
            4 => Ok(ExportDesc::Tag(reader.read_u32_le()?)),
            _ => Err(Error::new(
                ErrorCategory::Parse,
                codes::INVALID_VALUE, // Or a more specific code for invalid enum tag
                "Invalid tag for ExportDesc deserialization",
            )),
        }
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Placeholder for ExternType and ResourceType from component.rs
/// These will need to be P-generic or use P-generic types.
/// For now, we define stubs so ImportDesc can compile.
/// In a real scenario, these would be properly defined in wrt-component
/// and made P-generic if they contain collections.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExternTypePlaceholder; // Placeholder

impl Checksummable for ExternTypePlaceholder {
    fn update_checksum(&self, _checksum: &mut Checksum) { // TODO: Implement
                                                          // actual checksum
                                                          // logic
    }
}

impl ToBytes for ExternTypePlaceholder {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        _writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        Ok(()) // Writes nothing
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for ExternTypePlaceholder {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        _reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        Ok(ExternTypePlaceholder) // Reads nothing
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Placeholder for resource types in the component model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ResourceTypePlaceholder; // Placeholder

impl Checksummable for ResourceTypePlaceholder {
    fn update_checksum(&self, _checksum: &mut Checksum) { // TODO: Implement
                                                          // actual checksum
                                                          // logic
    }
}

impl ToBytes for ResourceTypePlaceholder {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        _writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        Ok(()) // Writes nothing
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for ResourceTypePlaceholder {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        _reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        Ok(ResourceTypePlaceholder) // Reads nothing
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Represents the size limits of a table or memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

impl Limits {
    pub const fn new(min: u32, max: Option<u32>) -> Self {
        Self { min, max }
    }
}

impl Checksummable for Limits {
    fn update_checksum(&self, checksum: &mut Checksum) {
        checksum.update_slice(&self.min.to_le_bytes());
        if let Some(max_val) = self.max {
            checksum.update(1);
            checksum.update_slice(&max_val.to_le_bytes());
        } else {
            checksum.update(0);
        }
    }
}

impl ToBytes for Limits {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream, /* Provider not directly used for simple types like u32 or
                              * Option<u32> that wrap primitives */
    ) -> WrtResult<()> {
        writer.write_u32_le(self.min)?;
        if let Some(max_val) = self.max {
            writer.write_u8(1)?; // Indicate Some(max_val)
            writer.write_u32_le(max_val)?;
        } else {
            writer.write_u8(0)?; // Indicate None
        }
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for Limits {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream, // Provider not directly used here
    ) -> WrtResult<Self> {
        let min = reader.read_u32_le()?;
        let has_max_flag = reader.read_u8()?;
        let max = match has_max_flag {
            1 => Some(reader.read_u32_le()?),
            0 => None,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VALUE,
                    "Invalid flag for Option<u32> in Limits deserialization",
                ));
            }
        };
        Ok(Limits { min, max })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Describes a table in a WebAssembly module, including its element type and
/// limits.
///
/// Tables are arrays of references that can be accessed by WebAssembly code.
/// They are primarily used for implementing indirect function calls and storing
/// references to host objects.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct TableType {
    // No P generic anymore
    /// The type of elements stored in the table (e.g., `FuncRef`, `ExternRef`).
    pub element_type: RefType,
    /// The size limits of the table, specifying initial and optional maximum
    /// size.
    pub limits: Limits,
}

// Generic constructor, still valid as it doesn't depend on P.
impl TableType {
    // No P generic anymore
    /// Creates a new `TableType` with a specific element type and limits.
    /// This const fn is suitable for static initializers.
    pub const fn new(element_type: RefType, limits: Limits) -> Self {
        Self { element_type, limits }
    }
}

// Trait implementations for TableType
impl Checksummable for TableType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.element_type.update_checksum(checksum);
        self.limits.update_checksum(checksum);
    }
}

impl ToBytes for TableType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.element_type.to_bytes_with_provider(writer, provider)?;
        self.limits.to_bytes_with_provider(writer, provider)?;
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for TableType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let element_type = RefType::from_bytes_with_provider(reader, provider)?;
        let limits = Limits::from_bytes_with_provider(reader, provider)?;
        Ok(TableType { element_type, limits })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct MemoryType {
    pub limits: Limits,
    pub shared: bool,
}

impl MemoryType {
    pub const fn new(limits: Limits, shared: bool) -> Self {
        Self { limits, shared }
    }
}

impl Checksummable for MemoryType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.limits.update_checksum(checksum);
        checksum.update(self.shared as u8);
    }
}

impl ToBytes for MemoryType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.limits.to_bytes_with_provider(writer, provider)?;
        writer.write_u8(self.shared as u8)?;
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for MemoryType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let limits = Limits::from_bytes_with_provider(reader, provider)?;
        let shared_byte = reader.read_u8()?;
        let shared = match shared_byte {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VALUE,
                    "Invalid boolean flag for MemoryType.shared",
                ));
            }
        };
        Ok(MemoryType { limits, shared })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct GlobalType {
    pub value_type: ValueType,
    pub mutable: bool,
}

impl GlobalType {
    pub const fn new(value_type: ValueType, mutable: bool) -> Self {
        Self { value_type, mutable }
    }
}

impl Checksummable for GlobalType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.value_type.update_checksum(checksum);
        checksum.update(self.mutable as u8);
    }
}

impl ToBytes for GlobalType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.value_type.to_bytes_with_provider(writer, provider)?;
        writer.write_u8(self.mutable as u8)?;
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for GlobalType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let value_type = ValueType::from_bytes_with_provider(reader, provider)?;
        let mutable_byte = reader.read_u8()?;
        let mutable = match mutable_byte {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VALUE,
                    "Invalid boolean flag for GlobalType.mutable",
                ));
            }
        };
        Ok(GlobalType { value_type, mutable })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Tag {
    pub type_idx: TypeIdx,
}

impl Tag {
    pub fn new(type_idx: TypeIdx) -> Self {
        Self { type_idx }
    }
}

impl Checksummable for Tag {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.type_idx.update_checksum(checksum);
    }
}

impl ToBytes for Tag {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream, // Provider not used for simple u32
    ) -> WrtResult<()> {
        writer.write_u32_le(self.type_idx)?;
        Ok(())
    }
    // Default to_bytes method will be used if #cfg(feature = "default-provider") is
    // active
}

impl FromBytes for Tag {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream, // Provider not used for simple u32
    ) -> WrtResult<Self> {
        let type_idx = reader.read_u32_le()?;
        Ok(Tag { type_idx })
    }
    // Default from_bytes method will be used if #cfg(feature = "default-provider")
    // is active
}

/// Represents a WebAssembly Module structure.
#[derive(Debug, Clone, PartialEq, Hash)] // Module itself cannot be Eq easily due to provider. P must be Eq for fields.
pub struct Module<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> {
    /// Types section: A list of function types defined in the module.
    pub types: BoundedVec<FuncType<P>, MAX_TYPES_IN_MODULE, P>,
    /// Imports section: A list of imports declared by the module.
    pub imports: BoundedVec<Import<P>, MAX_IMPORTS_IN_MODULE, P>,
    /// Functions section: A list of type indices for functions defined in the
    /// module.
    pub functions: BoundedVec<TypeIdx, MAX_FUNCS_IN_MODULE, P>,
    /// Tables section: A list of table types defined in the module.
    pub tables: BoundedVec<TableType, MAX_TABLES_IN_MODULE, P>,
    /// Memories section: A list of memory types defined in the module.
    pub memories: BoundedVec<MemoryType, MAX_MEMORIES_IN_MODULE, P>,
    /// Globals section: A list of global variables defined in the module.
    pub globals: BoundedVec<GlobalType, MAX_GLOBALS_IN_MODULE, P>,
    /// Exports section: A list of exports declared by the module.
    pub exports: BoundedVec<Export<P>, MAX_EXPORTS_IN_MODULE, P>,
    /// Start function: An optional index to a function that is executed when
    /// the module is instantiated.
    pub start_func: Option<FuncIdx>,
    /// Function bodies section: A list of code bodies for functions defined in
    /// the module.
    pub func_bodies: BoundedVec<FuncBody<P>, MAX_FUNCS_IN_MODULE, P>, // Changed from code_entries
    /// Data count section: An optional count of data segments, required if data
    /// segments are present.
    pub data_count: Option<u32>,
    /// Custom sections: A list of custom sections with arbitrary binary data.
    pub custom_sections: BoundedVec<CustomSection<P>, MAX_CUSTOM_SECTIONS_IN_MODULE, P>,
    /// Tags section: A list of exception tags.
    pub tags: BoundedVec<Tag, MAX_TAGS_IN_MODULE, P>,
    /// The memory provider instance.
    provider: P,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Module<P> {
    /// Creates a new, empty `Module` with the given memory provider.
    pub fn new(provider: P) -> Self {
        Self {
            types: BoundedVec::new(provider.clone()).expect("Failed to init types BoundedVec"),
            imports: BoundedVec::new(provider.clone()).expect("Failed to init imports BoundedVec"),
            functions: BoundedVec::new(provider.clone())
                .expect("Failed to init functions BoundedVec"),
            tables: BoundedVec::new(provider.clone()).expect("Failed to init tables BoundedVec"),
            memories: BoundedVec::new(provider.clone())
                .expect("Failed to init memories BoundedVec"),
            globals: BoundedVec::new(provider.clone()).expect("Failed to init globals BoundedVec"),
            exports: BoundedVec::new(provider.clone()).expect("Failed to init exports BoundedVec"),
            start_func: None,
            func_bodies: BoundedVec::new(provider.clone())
                .expect("Failed to init func_bodies BoundedVec"),
            data_count: None,
            custom_sections: BoundedVec::new(provider.clone())
                .expect("Failed to init custom_sections BoundedVec"),
            tags: BoundedVec::new(provider.clone()).expect("Failed to init tags BoundedVec"),
            provider,
        }
    }

    /// Returns a clone of the memory provider used by this module.
    pub fn provider(&self) -> P {
        self.provider.clone()
    }
}

// If P: Default is available, we can provide a Default impl for Module.
impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for Module<P>
{
    fn default() -> Self {
        let provider = P::default();
        Self::new(provider)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for Module<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        // Helper to update checksum for a BoundedVec of Checksummable items
        fn update_vec_checksum<
            T: Checksummable
                + ToBytes
                + FromBytes
                + Default
                + Clone
                + core::fmt::Debug
                + PartialEq
                + Eq,
            const N: usize,
            Prov: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq,
        >(
            vec: &BoundedVec<T, N, Prov>,
            checksum: &mut Checksum,
        ) {
            checksum.update_slice(&(vec.len() as u32).to_le_bytes());
            for i in 0..vec.len() {
                if let Ok(item) = vec.get(i) {
                    item.update_checksum(checksum);
                }
            }
        }
        // Helper for BoundedVec<TypeIdx, ...>
        fn update_idx_vec_checksum<
            const N: usize,
            Prov: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq,
        >(
            vec: &BoundedVec<TypeIdx, N, Prov>,
            checksum: &mut Checksum,
        ) {
            checksum.update_slice(&(vec.len() as u32).to_le_bytes());
            for i in 0..vec.len() {
                if let Ok(item) = vec.get(i) {
                    checksum.update_slice(&item.to_le_bytes());
                }
            }
        }

        update_vec_checksum(&self.types, checksum);
        update_vec_checksum(&self.imports, checksum);
        update_idx_vec_checksum(&self.functions, checksum);
        update_vec_checksum(&self.tables, checksum);
        update_vec_checksum(&self.memories, checksum);
        update_vec_checksum(&self.globals, checksum);
        update_vec_checksum(&self.exports, checksum);
        if let Some(start_func_idx) = self.start_func {
            checksum.update_slice(&[1]);
            checksum.update_slice(&start_func_idx.to_le_bytes());
        } else {
            checksum.update_slice(&[0]);
        }
        update_vec_checksum(&self.func_bodies, checksum);
        if let Some(data_cnt) = self.data_count {
            checksum.update_slice(&[1]);
            checksum.update_slice(&data_cnt.to_le_bytes());
        } else {
            checksum.update_slice(&[0]);
        }
        update_vec_checksum(&self.custom_sections, checksum);
        update_vec_checksum(&self.tags, checksum);
        // Not checksumming the provider itself, as it's about memory
        // management, not content.
    }
}

// Note: Duplicate implementation removed
// impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq>
// ToBytes for Import<P> {...}

// Note: Duplicate implementation removed
// impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq>
// FromBytes for Import<P> {...}

/// Represents the type of a block, loop, or if instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] // Removed Default
pub enum BlockType {
    /// The block type is a single value type for the result.
    /// `None` indicates an empty result type (no result).
    // Removed #[default]
    Value(Option<ValueType>),
    /// The block type is an index into the type section, indicating a function
    /// type.
    FuncType(TypeIdx),
}

impl Default for BlockType {
    fn default() -> Self {
        // Default to an empty result type (no value).
        BlockType::Value(None)
    }
}

// Duplicate implementation removed completely

// Constants for aggregate types
pub const MAX_STRUCT_FIELDS: usize = 64;
pub const MAX_ARRAY_ELEMENTS: usize = 1024;

/// WebAssembly 3.0 aggregate types for struct and array operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AggregateType<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> {
    /// Struct type definition
    Struct(StructType<P>),
    /// Array type definition
    Array(ArrayType),
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for AggregateType<P>
{
    fn default() -> Self {
        Self::Array(ArrayType::default())
    }
}

/// Struct type definition for WebAssembly 3.0 GC
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> {
    /// Fields in the struct
    pub fields: BoundedVec<FieldType, MAX_STRUCT_FIELDS, P>,
    /// Whether this type can be subtyped
    pub final_type: bool,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> StructType<P> {
    /// Create a new struct type
    pub fn new(provider: P, final_type: bool) -> Result<Self> {
        let fields = BoundedVec::new(provider).map_err(Error::from)?;
        Ok(Self { fields, final_type })
    }

    /// Add a field to the struct
    pub fn add_field(&mut self, field: FieldType) -> Result<()> {
        self.fields.push(field).map_err(Error::from)
    }

    /// Get field count
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Get field by index
    pub fn get_field(&self, index: usize) -> Result<FieldType> {
        self.fields.get(index).map_err(Error::from)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for StructType<P>
{
    fn default() -> Self {
        let provider = P::default();
        Self::new(provider, false).expect("Default StructType creation failed")
    }
}

/// Array type definition for WebAssembly 3.0 GC
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ArrayType {
    /// Element type of the array
    pub element_type: FieldType,
    /// Whether this type can be subtyped
    pub final_type: bool,
}

impl ArrayType {
    /// Create a new array type
    pub const fn new(element_type: FieldType, final_type: bool) -> Self {
        Self { element_type, final_type }
    }
}

/// Field type for struct fields and array elements
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldType {
    /// Storage type of the field
    pub storage_type: StorageType,
    /// Whether the field is mutable
    pub mutable: bool,
}

impl FieldType {
    /// Create a new field type
    pub const fn new(storage_type: StorageType, mutable: bool) -> Self {
        Self { storage_type, mutable }
    }

    /// Convert to value type for type checking
    pub fn to_value_type(&self) -> ValueType {
        self.storage_type.to_value_type()
    }
}

impl Default for FieldType {
    fn default() -> Self {
        Self { storage_type: StorageType::default(), mutable: false }
    }
}

/// Storage type for field values
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageType {
    /// Full value type
    Value(ValueType),
    /// Packed storage type
    Packed(PackedType),
}

impl StorageType {
    /// Convert to value type for type checking
    pub fn to_value_type(&self) -> ValueType {
        match self {
            StorageType::Value(vt) => *vt,
            StorageType::Packed(PackedType::I8) => ValueType::I32, // Packed types extend to I32
            StorageType::Packed(PackedType::I16) => ValueType::I32,
        }
    }
}

impl Default for StorageType {
    fn default() -> Self {
        Self::Value(ValueType::I32)
    }
}

/// Packed storage types for space-efficient fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackedType {
    /// 8-bit signed integer
    I8,
    /// 16-bit signed integer  
    I16,
}

impl PackedType {
    /// Get the size in bytes
    pub fn size_in_bytes(self) -> usize {
        match self {
            PackedType::I8 => 1,
            PackedType::I16 => 2,
        }
    }

    /// Convert to binary representation
    pub fn to_binary(self) -> u8 {
        match self {
            PackedType::I8 => 0x78,
            PackedType::I16 => 0x77,
        }
    }

    /// Create from binary representation
    pub fn from_binary(byte: u8) -> Result<Self> {
        match byte {
            0x78 => Ok(PackedType::I8),
            0x77 => Ok(PackedType::I16),
            _ => Err(Error::new(
                ErrorCategory::Parse,
                wrt_error::codes::PARSE_INVALID_VALTYPE_BYTE,
                "Invalid packed type byte",
            )),
        }
    }
}

// Implement serialization traits for the new types
impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for StructType<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.fields.update_checksum(checksum);
        checksum.update(self.final_type as u8);
    }
}

impl Checksummable for ArrayType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.element_type.update_checksum(checksum);
        checksum.update(self.final_type as u8);
    }
}

impl Checksummable for FieldType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.storage_type.update_checksum(checksum);
        checksum.update(self.mutable as u8);
    }
}

impl Checksummable for StorageType {
    fn update_checksum(&self, checksum: &mut Checksum) {
        match self {
            StorageType::Value(vt) => {
                checksum.update(0);
                vt.update_checksum(checksum);
            }
            StorageType::Packed(pt) => {
                checksum.update(1);
                checksum.update(pt.to_binary());
            }
        }
    }
}

// Implement ToBytes/FromBytes for the new types
impl ToBytes for FieldType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.storage_type.to_bytes_with_provider(writer, provider)?;
        writer.write_u8(self.mutable as u8)?;
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl FromBytes for FieldType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let storage_type = StorageType::from_bytes_with_provider(reader, provider)?;
        let mutable_byte = reader.read_u8()?;
        let mutable = match mutable_byte {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VALUE,
                    "Invalid boolean flag for FieldType.mutable",
                ))
            }
        };
        Ok(FieldType { storage_type, mutable })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

impl ToBytes for StorageType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        match self {
            StorageType::Value(vt) => {
                writer.write_u8(0)?;
                vt.to_bytes_with_provider(writer, provider)?;
            }
            StorageType::Packed(pt) => {
                writer.write_u8(1)?;
                writer.write_u8(pt.to_binary())?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl FromBytes for StorageType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let tag = reader.read_u8()?;
        match tag {
            0 => {
                let vt = ValueType::from_bytes_with_provider(reader, provider)?;
                Ok(StorageType::Value(vt))
            }
            1 => {
                let packed_byte = reader.read_u8()?;
                let pt = PackedType::from_binary(packed_byte)?;
                Ok(StorageType::Packed(pt))
            }
            _ => Err(Error::new(
                ErrorCategory::Parse,
                codes::INVALID_VALUE,
                "Invalid tag for StorageType",
            )),
        }
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

impl ToBytes for ArrayType {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.element_type.to_bytes_with_provider(writer, provider)?;
        writer.write_u8(self.final_type as u8)?;
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl FromBytes for ArrayType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let element_type = FieldType::from_bytes_with_provider(reader, provider)?;
        let final_byte = reader.read_u8()?;
        let final_type = match final_byte {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VALUE,
                    "Invalid boolean flag for ArrayType.final_type",
                ))
            }
        };
        Ok(ArrayType { element_type, final_type })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ToBytes
    for StructType<P>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.fields.to_bytes_with_provider(writer, provider)?;
        writer.write_u8(self.final_type as u8)?;
        Ok(())
    }

    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FromBytes
    for StructType<P>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let fields = BoundedVec::<FieldType, MAX_STRUCT_FIELDS, P>::from_bytes_with_provider(
            reader, provider,
        )?;
        let final_byte = reader.read_u8()?;
        let final_type = match final_byte {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VALUE,
                    "Invalid boolean flag for StructType.final_type",
                ))
            }
        };
        Ok(StructType { fields, final_type })
    }

    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

/// Placeholder for element segment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementSegment<
    P: MemoryProvider + Default + Clone + PartialEq + Eq = DefaultMemoryProvider,
> {
    /// Table index
    pub table_index: u32,
    /// Offset expression
    pub offset: BoundedVec<u8, 1024, P>,
    /// Elements
    pub elements: BoundedVec<u32, 1024, P>,
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> Default for ElementSegment<P> {
    fn default() -> Self {
        Self {
            table_index: 0,
            offset: BoundedVec::new(P::default()).unwrap_or_default(),
            elements: BoundedVec::new(P::default()).unwrap_or_default(),
        }
    }
}

/// Placeholder for data segment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSegment<P: MemoryProvider + Default + Clone + PartialEq + Eq = DefaultMemoryProvider>
{
    /// Memory index
    pub memory_index: u32,
    /// Offset expression
    pub offset: BoundedVec<u8, 1024, P>,
    /// Data bytes
    pub data: BoundedVec<u8, 1024, P>,
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> Default for DataSegment<P> {
    fn default() -> Self {
        Self {
            memory_index: 0,
            offset: BoundedVec::new(P::default()).unwrap_or_default(),
            data: BoundedVec::new(P::default()).unwrap_or_default(),
        }
    }
}

/// Placeholder for reference value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefValue {
    /// Null reference
    Null,
    /// Function reference
    FuncRef(u32),
    /// External reference
    ExternRef(u32),
}

impl Default for RefValue {
    fn default() -> Self {
        Self::Null
    }
}

// Removed duplicate Instruction enum - using the generic one above
