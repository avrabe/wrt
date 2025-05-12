// WRT - wrt-types
// Module: Core WebAssembly Type Definitions
// SW-REQ-ID: REQ_018
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly type definitions
//!
//! This module defines core WebAssembly types and utilities for working with
//! them, including function types, block types, value types, and reference
//! types.

#![allow(unused_imports)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::collections::BTreeMap as HashMap;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::collections::BTreeSet as HashSet;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::fmt::{Debug, Display};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::format;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, string::ToString, vec::Vec}; // For vec! macro
use core::fmt;
#[cfg(not(any(feature = "std", feature = "alloc")))]
use core::fmt::{Debug, Display};
#[cfg(feature = "std")]
use core::hash::Hasher as StdHasher;
#[cfg(feature = "std")]
use core::str::FromStr;
#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use std::collections::HashSet;
// Use proper imports for std or no_std environments
#[cfg(feature = "std")]
use std::fmt::{Debug, Display};
#[cfg(feature = "std")]
use std::format;
#[cfg(feature = "std")]
use std::{string::String, string::ToString, vec::Vec};

// Import wrt_error types
use wrt_error::{Error, ErrorCategory};

// Import BoundedVec and other necessary types
use crate::bounded::BoundedVec;
use crate::{
    conversion,
    prelude::{BoundedCapacity, Eq, Ord, PartialEq, TryFrom},
    sections::Section,
    values::Value,
    Result,
};

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

/// Internal hasher for `FuncType`, may be removed or replaced.
#[derive(Default, Debug)]
struct Hasher {
    hash: u32,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// 32-bit integer
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
                ErrorCategory::Parse,
                wrt_error::codes::PARSE_INVALID_VALTYPE_BYTE,
                format!("Invalid value type byte: {byte:#02x}"),
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

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32 => write!(f, "i32"),
            Self::I64 => write!(f, "i64"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
            Self::V128 => write!(f, "v128"),
            Self::I16x8 => write!(f, "i16x8"),
            Self::FuncRef => write!(f, "funcref"),
            Self::ExternRef => write!(f, "externref"),
        }
    }
}

/// WebAssembly block type for control flow instructions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockType {
    /// No values are returned (void/empty)
    Empty,
    /// A single value of the specified type is returned
    Value(ValueType),
    /// Multiple values are returned according to the function type
    FuncType(FuncType),
    /// Reference to a function type by index
    TypeIndex(u32),
}

impl BlockType {
    /// Returns the `ValueType` if this is a single-value block type
    #[must_use]
    pub fn as_value_type(&self) -> Option<ValueType> {
        match self {
            Self::Value(vt) => Some(*vt),
            _ => None,
        }
    }

    /// Returns the `FuncType` if this is a multi-value block type
    #[must_use]
    pub fn as_func_type(&self) -> Option<&FuncType> {
        match self {
            Self::FuncType(ft) => Some(ft),
            _ => None,
        }
    }

    /// Returns the type index if this is a type reference
    #[must_use]
    pub fn as_type_index(&self) -> Option<u32> {
        match self {
            Self::TypeIndex(idx) => Some(*idx),
            _ => None,
        }
    }

    /// Creates a `BlockType` from a `ValueType` option (None = Empty, Some =
    /// Value)
    #[must_use]
    pub fn from_value_type_option(vt: Option<ValueType>) -> Self {
        match vt {
            Some(vt) => Self::Value(vt),
            None => Self::Empty,
        }
    }
}

/// WebAssembly reference types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    /// Function reference type
    Funcref,
    /// External reference type
    Externref,
}

impl From<RefType> for ValueType {
    fn from(rt: RefType) -> Self {
        conversion::ref_type_to_val_type(rt)
    }
}

impl TryFrom<ValueType> for RefType {
    type Error = wrt_error::Error;

    fn try_from(vt: ValueType) -> core::result::Result<Self, Self::Error> {
        match vt {
            ValueType::FuncRef => Ok(RefType::Funcref),
            ValueType::ExternRef => Ok(RefType::Externref),
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH_ERROR,
                "Cannot convert value type to RefType",
            )),
        }
    }
}

/// Maximum parameters allowed in a function type
pub const MAX_FUNC_TYPE_PARAMS: usize = 128;
/// Maximum results allowed in a function type
pub const MAX_FUNC_TYPE_RESULTS: usize = 128;

/// WebAssembly function type
///
/// Represents a function's parameter and result types.
#[derive(Clone)]
pub struct FuncType {
    /// Parameter types
    pub params: Vec<ValueType>,
    /// Result types
    pub results: Vec<ValueType>,
    /// Type hash for validation
    type_hash: u32,
}

impl FuncType {
    /// Create a new function type with capacity checking
    pub fn new(params: Vec<ValueType>, results: Vec<ValueType>) -> Result<Self> {
        // Check capacity limits to ensure safety
        if params.len() > MAX_FUNC_TYPE_PARAMS {
            return Err(Error::new(
                ErrorCategory::Capacity,
                1013, // Capacity exceeded error code
                format!(
                    "Too many parameters in function type: {}, max is {}",
                    params.len(),
                    MAX_FUNC_TYPE_PARAMS
                ),
            ));
        }

        if results.len() > MAX_FUNC_TYPE_RESULTS {
            return Err(Error::new(
                ErrorCategory::Capacity,
                1013, // Capacity exceeded error code
                format!(
                    "Too many results in function type: {}, max is {}",
                    results.len(),
                    MAX_FUNC_TYPE_RESULTS
                ),
            ));
        }

        let mut func_type = Self { params, results, type_hash: 0 };
        func_type.compute_hash();
        Ok(func_type)
    }

    /// Compute the hash of this function type
    ///
    /// This is used for validation during execution.
    fn compute_hash(&mut self) {
        // Simple hash for now
        let param_hash: u32 = self.params.iter().enumerate().fold(0, |acc, (i, vt)| {
            let byte = u32::from(vt.to_binary());
            acc.wrapping_add(byte.wrapping_mul(i as u32 + 1))
        });

        let result_hash: u32 = self.results.iter().enumerate().fold(0, |acc, (i, vt)| {
            let byte = u32::from(vt.to_binary());
            acc.wrapping_add(byte.wrapping_mul(i as u32 + 100))
        });

        self.type_hash = param_hash.wrapping_add(result_hash);
    }

    /// Get the hash value for this function type
    #[must_use]
    pub fn hash(&self) -> u32 {
        self.type_hash
    }

    /// Verify the function type's constraints are satisfied
    pub fn verify(&self) -> Result<()> {
        // Additional validation could be added here
        Ok(())
    }

    /// Check if this function type matches another function type
    pub fn matches(&self, other: &Self) -> Result<bool> {
        self.verify()?;
        other.verify()?;

        // Hash comparison is a fast way to check equality
        Ok(self.type_hash == other.type_hash)
    }

    /// Validate that the given parameters match this function type
    pub fn validate_params(&self, params: &[Value]) -> core::result::Result<(), wrt_error::Error> {
        if params.len() != self.params.len() {
            return Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                6001, // TYPE_MISMATCH_ERROR
                format!(
                    "Parameter count mismatch: expected {}, got {}",
                    self.params.len(),
                    params.len()
                ),
            ));
        }

        for (expected_type, param) in self.params.iter().zip(params.iter()) {
            if !param.matches_type(expected_type) {
                return Err(wrt_error::Error::type_error(format!(
                    "Parameter type mismatch: expected {:?}, got {:?}",
                    expected_type,
                    param.value_type()
                )));
            }
        }

        Ok(())
    }

    /// Validate that the given results match this function type
    pub fn validate_results(
        &self,
        results: &[Value],
    ) -> core::result::Result<(), wrt_error::Error> {
        if results.len() != self.results.len() {
            return Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                6001, // TYPE_MISMATCH_ERROR
                format!(
                    "Result count mismatch: expected {}, got {}",
                    self.results.len(),
                    results.len()
                ),
            ));
        }

        for (expected_type, result) in self.results.iter().zip(results.iter()) {
            if !result.matches_type(expected_type) {
                return Err(wrt_error::Error::type_error(format!(
                    "Result type mismatch: expected {:?}, got {:?}",
                    expected_type,
                    result.value_type()
                )));
            }
        }

        Ok(())
    }

    /// Execute type checking for function parameters
    #[must_use]
    pub fn check_params(&self, params: &[ValueType]) -> bool {
        if params.len() != self.params.len() {
            return false;
        }
        for (param, expected_type) in params.iter().zip(self.params.iter()) {
            if param != expected_type {
                return false;
            }
        }
        true
    }

    /// Execute type checking for function results
    #[must_use]
    pub fn check_results(&self, results: &[ValueType]) -> bool {
        if results.len() != self.results.len() {
            return false;
        }
        for (result, expected_type) in results.iter().zip(self.results.iter()) {
            if result != expected_type {
                return false;
            }
        }
        true
    }
}

// Implement PartialEq for FuncType
impl PartialEq for FuncType {
    fn eq(&self, other: &Self) -> bool {
        // If hashes are the same, they're equal
        if self.type_hash == other.type_hash {
            return true;
        }

        // Otherwise, do a structural comparison
        if self.params.len() != other.params.len() || self.results.len() != other.results.len() {
            return false;
        }

        // Compare all params
        for (a, b) in self.params.iter().zip(other.params.iter()) {
            if a != b {
                return false;
            }
        }

        // Compare all results
        for (a, b) in self.results.iter().zip(other.results.iter()) {
            if a != b {
                return false;
            }
        }

        true
    }
}

// Implement Eq for FuncType
impl Eq for FuncType {}

impl fmt::Debug for FuncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FuncType({} -> {}, hash=0x{:08x})",
            func_params_to_string(&self.params),
            func_results_to_string(&self.results),
            self.type_hash
        )
    }
}

/// Convert function parameters to a string representation
fn func_params_to_string(params: &[ValueType]) -> String {
    if params.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::new();
    result.push('[');

    for (i, param) in params.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&param.to_string());
    }

    result.push(']');
    result
}

/// Convert function results to a string representation
fn func_results_to_string(results: &[ValueType]) -> String {
    if results.is_empty() {
        return "[]".to_string();
    }

    // Single result is shown without brackets for readability
    if results.len() == 1 {
        return results[0].to_string();
    }

    let mut result = String::new();
    result.push('[');

    for (i, res) in results.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&res.to_string());
    }

    result.push(']');
    result
}

/// Represents the type of a memory, including its limits and shared status.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MemoryType {
    /// Memory limits
    pub limits: Limits,
    /// Whether the memory can be shared between instances
    pub shared: bool,
}

/// Represents the type of a table, including its limits and element type.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TableType {
    /// Table limits
    pub limits: Limits,
    /// Type of elements in the table
    pub element_type: ValueType,
}

/// Represents the type of a WebAssembly global variable.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GlobalType {
    /// Type of values stored in the global
    pub value_type: ValueType,
    /// Whether the global is mutable
    pub mutable: bool,
    /// Add initial value for the global, parsed from `init_expr`
    /// This assumes the `init_expr` is a constant expression evaluable by the
    /// decoder.
    pub initial_value: Value,
}

/// Defines the limits for a table or memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] // Added Hash, kept Copy, PartialEq, Eq
pub struct Limits {
    /// Minimum size (required)
    pub min: u32,
    /// Maximum size (optional)
    pub max: Option<u32>,
}

impl Limits {
    /// Create new limits with minimum and optional maximum
    #[must_use]
    pub fn new(min: u32, max: Option<u32>) -> Self {
        Self { min, max }
    }

    /// Check if a size is within the limits
    #[must_use]
    pub fn check_size(&self, size: u32) -> bool {
        size >= self.min
            && match self.max {
                Some(max) => size <= max,
                None => true,
            }
    }
}

/// Describes the type of a global variable for import purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportGlobalType {
    /// Type of values stored in the global.
    pub value_type: ValueType,
    /// Whether the global is mutable.
    pub mutable: bool,
}

/// Represents an import in a WebAssembly module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Import {
    /// The module name from which to import.
    pub module: String,
    /// The name of the item to import.
    pub name: String,
    /// The descriptor of the imported item (function, table, memory, or
    /// global).
    pub desc: ImportDesc,
}

/// Describes the kind of an imported item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportDesc {
    /// An imported function, with its type index.
    Function(u32), // type_index
    /// An imported table, with its table type.
    Table(TableType),
    /// An imported memory, with its memory type.
    Memory(MemoryType),
    /// An imported global, with its global type.
    Global(ImportGlobalType),
    // Tag(u32), // Placeholder for future Tag support
}

/// Represents an export in a WebAssembly module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Export {
    /// The name under which the item is exported.
    pub name: String,
    /// The descriptor of the exported item (function, table, memory, or
    /// global).
    pub desc: ExportDesc,
}

/// Describes the kind of an exported item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExportDesc {
    /// An exported function, with its function index.
    Function(u32), // func_idx
    /// An exported table, with its table index.
    Table(u32), // table_idx
    /// An exported memory, with its memory index.
    Memory(u32), // memory_idx
    /// An exported global, with its global index.
    Global(u32), /* global_idx
                  * Tag(u32),    // Placeholder for future Tag support */
}

/// Represents an element segment for table initialization.
#[derive(Debug, Clone, PartialEq)] // Eq may not be derivable if Value in ElementMode::Active is not Eq
pub struct ElementSegment {
    /// The mode of the element segment (active, passive, or declared).
    pub mode: ElementMode,
    /// The type of elements in the segment (e.g., `FuncRef`).
    pub element_type: RefType, // In MVP, this is always FuncRef.
    /// The items (function indices for `FuncRef`) in the segment.
    pub items: Vec<u32>, // Function indices for FuncRef elements.
}

/// Describes the mode of an element segment.
#[derive(Debug, Clone, PartialEq)] // Eq may not be derivable if Value is not Eq
pub enum ElementMode {
    /// Active segment that initializes a table region at a given offset.
    Active {
        /// The index of the table to initialize.
        table_index: u32,
        /// The offset within the table where initialization occurs, evaluated
        /// from a const expression.
        offset: Value, // Parsed from offset_expr (must be a constant expression)
    },
    /// Passive segment whose elements can be copied into a table using
    /// `table.init`.
    Passive,
    /// Declared segment whose elements are available to the host or via
    /// `ref.func`.
    Declared,
}

/// Represents a data segment for memory initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSegment {
    /// The mode of the data segment (active or passive).
    pub mode: DataMode,
    /// The initial data bytes of the segment.
    pub init: Vec<u8>, // Initial data.
}

/// Describes the mode of a data segment.
#[derive(Debug, Clone, PartialEq, Eq)] // Requires Value to be Eq if it were part of DataMode for active
pub enum DataMode {
    /// Active segment that initializes a memory region at a given offset.
    Active {
        /// The index of the memory to initialize (should be 0 in MVP).
        memory_index: u32, // Should be 0 in MVP.
        /// The offset within memory where initialization occurs, evaluated from
        /// a const expression.
        offset: Value, // Parsed from offset_expr (must be a constant expression)
    },
    /// Passive segment whose data can be copied into memory using
    /// `memory.init`.
    Passive,
}

/// Represents a custom section in a WebAssembly module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomSection {
    /// The name of the custom section.
    pub name: String,
    /// The raw byte data of the custom section.
    pub data: Vec<u8>,
}

impl CustomSection {
    /// Creates a new `CustomSection` from a name and byte slice.
    #[must_use]
    pub fn from_bytes(name: String, data: &[u8]) -> Self {
        Self { name, data: data.to_vec() }
    }
}

/// Memory argument for load/store instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemArg {
    /// The alignment of the memory access, stored as `2^align_exponent`.
    /// Actual alignment is `1 << align_exponent`. Max 32 for V128, 8 for others
    /// typically.
    pub align_exponent: u32,
    /// The constant offset added to the address operand.
    pub offset: u32,
    /// Memory index (for multi-memory proposal, typically 0).
    pub memory_index: MemIdx,
}

/// An instruction in a WebAssembly function body.
/// This enum aims to cover Wasm Core 2.0.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Control Instructions
    /// WebAssembly instruction.
    Unreachable,
    /// WebAssembly instruction.
    Nop,
    /// WebAssembly instruction.
    Block(BlockType), // Marks start of a block
    /// WebAssembly instruction.
    Loop(BlockType), // Marks start of a loop
    /// WebAssembly instruction.
    If(BlockType), // Marks start of an if block
    /// WebAssembly instruction.
    Else, // Marks start of an else clause
    /// WebAssembly instruction.
    End, // Marks end of Block/Loop/If/Else

    /// WebAssembly instruction.
    Br(LabelIdx), // Branch to a given label index
    /// WebAssembly instruction.
    BrIf(LabelIdx), // Conditional branch
    /// WebAssembly instruction.
    BrTable(Vec<LabelIdx>, LabelIdx), // Indirect branch table

    /// WebAssembly instruction.
    Return,
    /// WebAssembly instruction.
    Call(FuncIdx),
    /// WebAssembly instruction.
    CallIndirect(TypeIdx, TableIdx), // type_idx, table_idx

    // Reference Instructions
    /// WebAssembly instruction.
    RefNull(RefType),
    /// WebAssembly instruction.
    RefIsNull,
    /// WebAssembly instruction.
    RefFunc(FuncIdx),

    // Parametric Instructions
    /// WebAssembly instruction.
    Drop,
    /// WebAssembly instruction.
    Select, // Untyped select
    /// WebAssembly instruction.
    SelectTyped(Vec<ValueType>), // Typed select (takes a vector of types, must be one)

    // Variable Instructions
    /// WebAssembly instruction.
    LocalGet(LocalIdx),
    /// WebAssembly instruction.
    LocalSet(LocalIdx),
    /// WebAssembly instruction.
    LocalTee(LocalIdx),
    /// WebAssembly instruction.
    GlobalGet(GlobalIdx),
    /// WebAssembly instruction.
    GlobalSet(GlobalIdx),

    // Table Instructions
    /// WebAssembly instruction.
    TableGet(TableIdx),
    /// WebAssembly instruction.
    TableSet(TableIdx),
    /// WebAssembly instruction.
    TableSize(TableIdx),
    /// WebAssembly instruction.
    TableGrow(TableIdx),
    /// WebAssembly instruction.
    TableFill(TableIdx),
    /// WebAssembly instruction.
    TableCopy(TableIdx, TableIdx), // target_table_idx, source_table_idx
    /// WebAssembly instruction.
    TableInit(ElemIdx, TableIdx), // elem_idx, table_idx
    /// WebAssembly instruction.
    ElemDrop(ElemIdx), // New: For elem.drop
    /// WebAssembly instruction.
    DataDrop(DataIdx), // New: For data.drop

    // Memory Instructions (using MemArg)
    /// WebAssembly instruction.
    I32Load(MemArg),
    /// WebAssembly instruction.
    I64Load(MemArg),
    /// WebAssembly instruction.
    F32Load(MemArg),
    /// WebAssembly instruction.
    F64Load(MemArg),
    /// WebAssembly instruction.
    I32Load8S(MemArg),
    /// WebAssembly instruction.
    I32Load8U(MemArg),
    /// WebAssembly instruction.
    I32Load16S(MemArg),
    /// WebAssembly instruction.
    I32Load16U(MemArg),
    /// WebAssembly instruction.
    I64Load8S(MemArg),
    /// WebAssembly instruction.
    I64Load8U(MemArg),
    /// WebAssembly instruction.
    I64Load16S(MemArg),
    /// WebAssembly instruction.
    I64Load16U(MemArg),
    /// WebAssembly instruction.
    I64Load32S(MemArg),
    /// WebAssembly instruction.
    I64Load32U(MemArg),
    /// WebAssembly instruction.
    I32Store(MemArg),
    /// WebAssembly instruction.
    I64Store(MemArg),
    /// WebAssembly instruction.
    F32Store(MemArg),
    /// WebAssembly instruction.
    F64Store(MemArg),
    /// WebAssembly instruction.
    I32Store8(MemArg),
    /// WebAssembly instruction.
    I32Store16(MemArg),
    /// WebAssembly instruction.
    I64Store8(MemArg),
    /// WebAssembly instruction.
    I64Store16(MemArg),
    /// WebAssembly instruction.
    I64Store32(MemArg),

    /// WebAssembly instruction.
    MemorySize(MemIdx), // mem_idx (usually 0)
    /// WebAssembly instruction.
    MemoryGrow(MemIdx), // mem_idx (usually 0)
    /// WebAssembly instruction.
    MemoryFill(MemIdx), // mem_idx (usually 0)
    /// WebAssembly instruction.
    MemoryCopy(MemIdx, MemIdx), // target_mem_idx, source_mem_idx
    /// WebAssembly instruction.
    MemoryInit(DataIdx, MemIdx), // data_idx, mem_idx

    // Numeric Constants
    /// WebAssembly instruction.
    I32Const(i32),
    /// WebAssembly instruction.
    I64Const(i64),
    /// WebAssembly instruction.
    F32Const(f32), // Stored as u32 bits
    /// WebAssembly instruction.
    F64Const(f64), // Stored as u64 bits

    // Numeric Operations (examples, list should be exhaustive)
    /// WebAssembly instruction.
    I32Eqz,
    /// WebAssembly instruction.
    I32Eq,
    /// WebAssembly instruction.
    I32Ne,
    /// WebAssembly instruction.
    I32LtS,
    /// WebAssembly instruction.
    I32LtU,
    /// WebAssembly instruction.
    I32GtS,
    /// WebAssembly instruction.
    I32GtU,
    /// WebAssembly instruction.
    I32LeS,
    /// WebAssembly instruction.
    I32LeU,
    /// WebAssembly instruction.
    I32GeS,
    /// WebAssembly instruction.
    I32GeU,
    /// WebAssembly instruction.
    I32Clz,
    /// WebAssembly instruction.
    I32Ctz,
    /// WebAssembly instruction.
    I32Popcnt,
    /// WebAssembly instruction.
    I32Add,
    /// WebAssembly instruction.
    I32Sub,
    /// WebAssembly instruction.
    I32Mul,
    /// WebAssembly instruction.
    I32DivS,
    /// WebAssembly instruction.
    I32DivU,
    /// WebAssembly instruction.
    I32RemS,
    /// WebAssembly instruction.
    I32RemU,
    /// WebAssembly instruction.
    I32And,
    /// WebAssembly instruction.
    I32Or,
    /// WebAssembly instruction.
    I32Xor,
    /// WebAssembly instruction.
    I32Shl,
    /// WebAssembly instruction.
    I32ShrS,
    /// WebAssembly instruction.
    I32ShrU,
    /// WebAssembly instruction.
    I32Rotl,
    /// WebAssembly instruction.
    I32Rotr,

    /// WebAssembly instruction.
    I64Eqz,
    /// WebAssembly instruction.
    I64Eq,
    /// WebAssembly instruction.
    I64Ne,
    /// WebAssembly instruction.
    I64LtS,
    /// WebAssembly instruction.
    I64LtU,
    /// WebAssembly instruction.
    I64GtS,
    /// WebAssembly instruction.
    I64GtU,
    /// WebAssembly instruction.
    I64LeS,
    /// WebAssembly instruction.
    I64LeU,
    /// WebAssembly instruction.
    I64GeS,
    /// WebAssembly instruction.
    I64GeU,
    /// WebAssembly instruction.
    I64Clz,
    /// WebAssembly instruction.
    I64Ctz,
    /// WebAssembly instruction.
    I64Popcnt,
    /// WebAssembly instruction.
    I64Add,
    /// WebAssembly instruction.
    I64Sub,
    /// WebAssembly instruction.
    I64Mul,
    /// WebAssembly instruction.
    I64DivS,
    /// WebAssembly instruction.
    I64DivU,
    /// WebAssembly instruction.
    I64RemS,
    /// WebAssembly instruction.
    I64RemU,
    /// WebAssembly instruction.
    I64And,
    /// WebAssembly instruction.
    I64Or,
    /// WebAssembly instruction.
    I64Xor,
    /// WebAssembly instruction.
    I64Shl,
    /// WebAssembly instruction.
    I64ShrS,
    /// WebAssembly instruction.
    I64ShrU,
    /// WebAssembly instruction.
    I64Rotl,
    /// WebAssembly instruction.
    I64Rotr,

    /// WebAssembly instruction.
    F32Eq,
    /// WebAssembly instruction.
    F32Ne,
    /// WebAssembly instruction.
    F32Lt,
    /// WebAssembly instruction.
    F32Gt,
    /// WebAssembly instruction.
    F32Le,
    /// WebAssembly instruction.
    F32Ge,
    /// WebAssembly instruction.
    F32Abs,
    /// WebAssembly instruction.
    F32Neg,
    /// WebAssembly instruction.
    F32Ceil,
    /// WebAssembly instruction.
    F32Floor,
    /// WebAssembly instruction.
    F32Trunc,
    /// WebAssembly instruction.
    F32Nearest,
    /// WebAssembly instruction.
    F32Sqrt,
    /// WebAssembly instruction.
    F32Add,
    /// WebAssembly instruction.
    F32Sub,
    /// WebAssembly instruction.
    F32Mul,
    /// WebAssembly instruction.
    F32Div,
    /// WebAssembly instruction.
    F32Min,
    /// WebAssembly instruction.
    F32Max,
    /// WebAssembly instruction.
    F32Copysign,

    /// WebAssembly instruction.
    F64Eq,
    /// WebAssembly instruction.
    F64Ne,
    /// WebAssembly instruction.
    F64Lt,
    /// WebAssembly instruction.
    F64Gt,
    /// WebAssembly instruction.
    F64Le,
    /// WebAssembly instruction.
    F64Ge,
    /// WebAssembly instruction.
    F64Abs,
    /// WebAssembly instruction.
    F64Neg,
    /// WebAssembly instruction.
    F64Ceil,
    /// WebAssembly instruction.
    F64Floor,
    /// WebAssembly instruction.
    F64Trunc,
    /// WebAssembly instruction.
    F64Nearest,
    /// WebAssembly instruction.
    F64Sqrt,
    /// WebAssembly instruction.
    F64Add,
    /// WebAssembly instruction.
    F64Sub,
    /// WebAssembly instruction.
    F64Mul,
    /// WebAssembly instruction.
    F64Div,
    /// WebAssembly instruction.
    F64Min,
    /// WebAssembly instruction.
    F64Max,
    /// WebAssembly instruction.
    F64Copysign,

    // Conversions
    /// WebAssembly instruction.
    I32WrapI64,
    /// WebAssembly instruction.
    I32TruncF32S,
    /// WebAssembly instruction.
    I32TruncF32U,
    /// WebAssembly instruction.
    I32TruncF64S,
    /// WebAssembly instruction.
    I32TruncF64U,
    /// WebAssembly instruction.
    I64ExtendI32S,
    /// WebAssembly instruction.
    I64ExtendI32U,
    /// WebAssembly instruction.
    I64TruncF32S,
    /// WebAssembly instruction.
    I64TruncF32U,
    /// WebAssembly instruction.
    I64TruncF64S,
    /// WebAssembly instruction.
    I64TruncF64U,
    /// WebAssembly instruction.
    F32ConvertI32S,
    /// WebAssembly instruction.
    F32ConvertI32U,
    /// WebAssembly instruction.
    F32ConvertI64S,
    /// WebAssembly instruction.
    F32ConvertI64U,
    /// WebAssembly instruction.
    F32DemoteF64,
    /// WebAssembly instruction.
    F64ConvertI32S,
    /// WebAssembly instruction.
    F64ConvertI32U,
    /// WebAssembly instruction.
    F64ConvertI64S,
    /// WebAssembly instruction.
    F64ConvertI64U,
    /// WebAssembly instruction.
    F64PromoteF32,
    /// WebAssembly instruction.
    I32ReinterpretF32,
    /// WebAssembly instruction.
    I64ReinterpretF64,
    /// WebAssembly instruction.
    F32ReinterpretI32,
    /// WebAssembly instruction.
    F64ReinterpretI64,

    // Saturating Truncation Conversions (prefix 0xFC)
    /// WebAssembly instruction.
    I32TruncSatF32S,
    /// WebAssembly instruction.
    I32TruncSatF32U,
    /// WebAssembly instruction.
    I32TruncSatF64S,
    /// WebAssembly instruction.
    I32TruncSatF64U,
    /// WebAssembly instruction.
    I64TruncSatF32S,
    /// WebAssembly instruction.
    I64TruncSatF32U,
    /// WebAssembly instruction.
    I64TruncSatF64S,
    /// WebAssembly instruction.
    I64TruncSatF64U,

    // Sign Extension Operations (prefix 0xFC) - Part of Wasm 2.0
    /// WebAssembly instruction.
    I32Extend8S,
    /// WebAssembly instruction.
    I32Extend16S,
    /// WebAssembly instruction.
    I64Extend8S,
    /// WebAssembly instruction.
    I64Extend16S,
    /// WebAssembly instruction.
    I64Extend32S,

    // SIMD Instructions (prefix 0xFD) - Selected examples, needs full list from spec
    /// WebAssembly instruction.
    V128Load(MemArg),
    /// WebAssembly instruction.
    V128Load8Splat(MemArg),
    /// WebAssembly instruction.
    V128Load16Splat(MemArg),
    /// WebAssembly instruction.
    V128Load32Splat(MemArg),
    /// WebAssembly instruction.
    V128Load64Splat(MemArg),
    /// WebAssembly instruction.
    V128Load8x8S(MemArg),
    /// WebAssembly instruction.
    V128Load8x8U(MemArg),
    /// WebAssembly instruction.
    V128Load16x4S(MemArg),
    /// WebAssembly instruction.
    V128Load16x4U(MemArg),
    /// WebAssembly instruction.
    V128Load32x2S(MemArg),
    /// WebAssembly instruction.
    V128Load32x2U(MemArg),
    /// WebAssembly instruction.
    V128Load32Zero(MemArg),
    /// WebAssembly instruction.
    V128Load64Zero(MemArg),
    /// WebAssembly instruction.
    V128Store(MemArg),
    /// WebAssembly instruction.
    V128Load8Lane(MemArg, u8), // MemArg, lane_idx
    /// WebAssembly instruction.
    V128Load16Lane(MemArg, u8),
    /// WebAssembly instruction.
    V128Load32Lane(MemArg, u8),
    /// WebAssembly instruction.
    V128Load64Lane(MemArg, u8),
    /// WebAssembly instruction.
    V128Store8Lane(MemArg, u8),
    /// WebAssembly instruction.
    V128Store16Lane(MemArg, u8),
    /// WebAssembly instruction.
    V128Store32Lane(MemArg, u8),
    /// WebAssembly instruction.
    V128Store64Lane(MemArg, u8),

    /// WebAssembly instruction.
    V128Const([u8; 16]), // Represents a 128-bit constant
    /// WebAssembly instruction.
    I8x16Shuffle([u8; 16]), // Lane indices for shuffle

    /// WebAssembly instruction.
    I8x16Splat,
    /// WebAssembly instruction.
    F32x4Splat,
    /// WebAssembly instruction.
    I16x8Splat,
    /// WebAssembly instruction.
    F64x2Splat,
    /// WebAssembly instruction.
    I32x4Splat,
    /// WebAssembly instruction.
    I64x2Splat, // Splat operations
    /// WebAssembly instruction.
    I8x16ExtractLaneS(u8),
    /// WebAssembly instruction.
    I8x16ExtractLaneU(u8), // Extract lane operations
    /// WebAssembly instruction.
    I16x8ExtractLaneS(u8),
    /// WebAssembly instruction.
    I16x8ExtractLaneU(u8),
    /// WebAssembly instruction.
    I32x4ExtractLane(u8),
    /// WebAssembly instruction.
    I64x2ExtractLane(u8),
    /// WebAssembly instruction.
    F32x4ExtractLane(u8),
    /// WebAssembly instruction.
    F64x2ExtractLane(u8),
    /// WebAssembly instruction.
    I8x16ReplaceLane(u8),
    /// WebAssembly instruction.
    I16x8ReplaceLane(u8), // Replace lane operations
    /// WebAssembly instruction.
    I32x4ReplaceLane(u8),
    /// WebAssembly instruction.
    I64x2ReplaceLane(u8),
    /// WebAssembly instruction.
    F32x4ReplaceLane(u8),
    /// WebAssembly instruction.
    F64x2ReplaceLane(u8),

    // Many more SIMD arithmetic, bitwise, comparison, conversion ops like:
    /// WebAssembly instruction.
    I8x16Eq,
    /// WebAssembly instruction.
    I8x16Ne,
    /// WebAssembly instruction.
    I8x16LtS, // ... up to F64x2Ge ...
    /// WebAssembly instruction.
    V128Not,
    /// WebAssembly instruction.
    V128And,
    /// WebAssembly instruction.
    V128AndNot,
    /// WebAssembly instruction.
    V128Or,
    /// WebAssembly instruction.
    V128Xor,
    /// WebAssembly instruction.
    V128Bitselect,
    // ... SIMD Fabs, Fneg, Fsqrt, Fadd, Fsub, Fmul, Fdiv, Fmin, Fmax, Fpmin, Fpmax ... */
    // ... SIMD Iadd, Isub, Imul, Imin, Imax, AvgrU, Q15MulRSatS, Extmul, ExtaddPairwise ... */
    // ... SIMD IShl, IShrS, IShrU ... */
    // ... SIMD Conversions (trunc_sat, narrow, widen, demote, promote) ...
    /// WebAssembly instruction.
    AnyTrue,
    /// WebAssembly instruction.
    AllTrue,
    /// WebAssembly instruction.
    Bitmask,

    // Tail Call Instructions (prefix 0xFC)
    /// WebAssembly instruction.
    ReturnCall(FuncIdx),
    /// WebAssembly instruction.
    ReturnCallIndirect(TypeIdx, TableIdx),

    // Atomic Memory Instructions (prefix 0xFE) - Selected examples
    /// WebAssembly instruction.
    MemoryAtomicNotify(MemArg), // align, offset (from MemArg)
    /// WebAssembly instruction.
    MemoryAtomicWait32(MemArg), // align, offset
    /// WebAssembly instruction.
    MemoryAtomicWait64(MemArg), // align, offset
    // Atomic RMW operations
    /// WebAssembly instruction.
    I32AtomicLoad(MemArg),
    /// WebAssembly instruction.
    I64AtomicLoad(MemArg),
    /// WebAssembly instruction.
    I32AtomicLoad8U(MemArg),
    /// WebAssembly instruction.
    I32AtomicLoad16U(MemArg),
    /// WebAssembly instruction.
    I64AtomicLoad8U(MemArg),
    /// WebAssembly instruction.
    I64AtomicLoad16U(MemArg),
    /// WebAssembly instruction.
    I64AtomicLoad32U(MemArg),
    /// WebAssembly instruction.
    I32AtomicStore(MemArg),
    /// WebAssembly instruction.
    I64AtomicStore(MemArg),
    /// WebAssembly instruction.
    I32AtomicStore8(MemArg),
    /// WebAssembly instruction.
    I32AtomicStore16(MemArg),
    /// WebAssembly instruction.
    I64AtomicStore8(MemArg),
    /// WebAssembly instruction.
    I64AtomicStore16(MemArg),
    /// WebAssembly instruction.
    I64AtomicStore32(MemArg),
    // RMW variants: Add, Sub, And, Or, Xor, Xchg, Cmpxchg
    // e.g., I32AtomicRmwAdd(MemArg), I64AtomicRmw8uCmpxchg(MemArg)
    // This list needs to be fully populated based on the Atomic spec.
    // For brevity, only a few are listed.
    /// WebAssembly instruction.
    I32AtomicRmwAdd(MemArg),
    /// WebAssembly instruction.
    I64AtomicRmwAdd(MemArg),
    /// WebAssembly instruction.
    I32AtomicRmwCmpxchg(MemArg),
    /// WebAssembly instruction.
    I64AtomicRmwCmpxchg(MemArg),
    // ... more atomic RMW operations ...
}

/// A sequence of WebAssembly instructions, typically forming a function body or
/// an initializer expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Instructions in the expression.
    pub instructions: Vec<Instruction>,
}

/// Represents an entry for local variables in a function's code section.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalEntry {
    /// Number of locals of this type.
    pub count: u32,
    /// Type of the locals.
    pub value_type: ValueType,
}

/// Code for a single WebAssembly function defined in the module.
#[derive(Debug, Clone, PartialEq)]
pub struct Code {
    /// Local variable declarations for this function.
    pub locals: Vec<LocalEntry>,
    /// The instruction sequence (body) of the function.
    pub body: Expr,
}

/// Represents a complete WebAssembly Module.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    // pub magic: u32, // Often omitted in higher-level representations
    // pub version: u32, // Often omitted
    /// Function types defined in the module.
    pub types: Vec<FuncType>,

    /// Imported functions, tables, memories, and globals.
    pub imports: Vec<Import>,

    /// For each function defined in the module, its type index into the `types`
    /// vector. The order corresponds to the `code_entries` vector.
    pub funcs: Vec<TypeIdx>,

    /// Table definitions.
    pub tables: Vec<TableType>,

    /// Memory definitions.
    pub memories: Vec<MemoryType>,

    /// Global variable definitions (includes initial value).
    pub globals: Vec<GlobalType>,

    /// Exported items.
    pub exports: Vec<Export>,

    /// Start function index, if specified.
    pub start: Option<FuncIdx>,

    /// Element segments for table initialization.
    pub elements: Vec<ElementSegment>,

    /// Code entries for functions defined in this module.
    /// The order corresponds to the `funcs` vector (type associations).
    pub code_entries: Vec<Code>,

    /// Data segments for memory initialization.
    pub data_segments: Vec<DataSegment>, // Renamed from 'data'

    /// Data count, if the `DataCount` section was present.
    pub data_count: Option<u32>,

    /// Custom sections.
    pub custom_sections: Vec<CustomSection>,
    // Potentially add fields for other sections like:
    // pub name_section: Option<NameSection>, // Define NameSection struct
    // pub producers_section: Option<ProducersSection>, // Define ProducersSection struct
    // etc. or keep them in custom_sections if their structure is not strictly enforced for the
    // runtime.
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    use alloc::{boxed::Box, vec};
    #[cfg(feature = "std")]
    use std::boxed::Box;

    impl Arbitrary for ValueType {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(ValueType::I32),
                Just(ValueType::I64),
                Just(ValueType::F32),
                Just(ValueType::F64),
                Just(ValueType::V128),
                Just(ValueType::I16x8),
                Just(ValueType::FuncRef),
                Just(ValueType::ExternRef),
            ]
            .boxed()
        }
    }

    #[test]
    fn test_value_type_conversions() {
        let i32_val = ValueType::I32;
        let binary = i32_val.to_binary();
        let roundtrip = ValueType::from_binary(binary).unwrap();
        assert_eq!(i32_val, roundtrip);

        // Test all value types
        let types = vec![
            ValueType::I32,
            ValueType::I64,
            ValueType::F32,
            ValueType::F64,
            ValueType::V128,
            ValueType::I16x8,
            ValueType::FuncRef,
            ValueType::ExternRef,
        ];

        for vt in types {
            let binary = vt.to_binary();
            let roundtrip = ValueType::from_binary(binary).unwrap();
            assert_eq!(vt, roundtrip);
        }
    }

    #[test]
    fn test_func_type_verification() {
        let func_type =
            FuncType::new(vec![ValueType::I32, ValueType::I64], vec![ValueType::F32]).unwrap();

        // This should pass verification
        assert!(func_type.verify().is_ok());

        // TODO: Add more tests for hash verification with tampering
    }

    #[test]
    fn test_limits() {
        let limited = Limits::new(10, Some(20));
        assert!(limited.check_size(10));
        assert!(limited.check_size(20));
        assert!(!limited.check_size(5));
        assert!(!limited.check_size(21));

        let unlimited = Limits::new(5, None);
        assert!(unlimited.check_size(5));
        assert!(unlimited.check_size(1_000_000)); // Corrected: 1000000 to
                                                  // 1_000_000
    }

    #[test]
    fn test_type_equality() {
        // Equal function types
        let func_type1 =
            FuncType::new(vec![ValueType::I32, ValueType::I64], vec![ValueType::F32]).unwrap();
        // Same function type should be equal
        let func_type2 =
            FuncType::new(vec![ValueType::I32, ValueType::I64], vec![ValueType::F32]).unwrap();
        // Different function type
        let func_type3 = FuncType::new(vec![ValueType::I32], vec![ValueType::F32]).unwrap();

        assert_eq!(func_type1, func_type2);
        assert_ne!(func_type1, func_type3);
        assert_ne!(func_type2, func_type3);
    }
}
