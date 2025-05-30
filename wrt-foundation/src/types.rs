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

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::collections::{BTreeMap, BTreeSet};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::{String as AllocString, ToString as AllocToString};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec as AllocVec;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{format, vec};
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "std")]
use std::string::{String, ToString};
#[cfg(feature = "std")]
use std::vec::Vec;

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
pub const MAX_DATA_SEGMENT_LENGTH: usize = 65536; // Max bytes in a data segment (active/passive)
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
}

impl core::fmt::Debug for ValueType {
    // Manual debug to avoid depending on alloc::format! in derived Debug
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
        }
    }
}

impl ValueType {
    /// Create a value type from a binary representation
    ///
    /// Uses the standardized conversion utility for consistency
    /// across all crates.
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
        }
    }

    /// Get the size of this value type in bytes
    #[must_use]
    pub fn size_in_bytes(self) -> usize {
        match self {
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 => 8,
            Self::V128 | Self::I16x8 => 16, // COMBINED ARMS
            Self::FuncRef | Self::ExternRef => {
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
/// This version is adapted for alloc-free operation by using fixed-size arrays.
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

/// A WebAssembly instruction (basic placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction<P: MemoryProvider + Clone + core::fmt::Debug + PartialEq + Eq + Default> {
    Unreachable,
    Nop,
    Block, // Represents the start of a block, type is on stack or from validation
    Loop,  // Represents the start of a loop, type is on stack or from validation
    If,    // Represents an if, type is on stack or from validation
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

    // Placeholder for more instructions
    LocalGet(LocalIdx),
    LocalSet(LocalIdx),
    LocalTee(LocalIdx),
    GlobalGet(GlobalIdx),
    GlobalSet(GlobalIdx),

    I32Const(i32),
    I64Const(i64),
    // ... other consts, loads, stores, numeric ops ...
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
            Instruction::Block => checksum.update_slice(&[0x02]), // Type info is external
            Instruction::Loop => checksum.update_slice(&[0x03]),  // Type info is external
            Instruction::If => checksum.update_slice(&[0x04]),    // Type info is external
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
            // Add other instruction checksum logic here
            Instruction::_Phantom(_) => { /* No data to checksum for PhantomData */ }
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
            Instruction::Block => writer.write_u8(0x02)?, // Placeholder, needs blocktype
            Instruction::Loop => writer.write_u8(0x03)?,  // Placeholder, needs blocktype
            Instruction::If => writer.write_u8(0x04)?,    // Placeholder, needs blocktype
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
            // ... many more instructions
            Instruction::_Phantom(_) => {
                // This variant should not be serialized
                return Err(SerializationError::Custom(
                    "Cannot serialize _Phantom instruction variant",
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
            0x02 => Ok(Instruction::Block), // Placeholder
            0x03 => Ok(Instruction::Loop),  // Placeholder
            0x04 => Ok(Instruction::If),    // Placeholder
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
    #[cfg(any(feature = "std", feature = "alloc", test))]
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
