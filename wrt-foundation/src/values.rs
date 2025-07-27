// WRT - wrt-foundation
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly value representations
//!
//! This module provides datatypes for representing WebAssembly values at
//! runtime.

#[cfg(feature = "std")]
// use std::format; // Removed
#[cfg(all(not(feature = "std")))]
use alloc;
#[cfg(not(feature = "std"))]
use core::fmt;
// use std::boxed::Box; // Temporarily commented to find usages
#[cfg(feature = "std")]
// use std::vec::Vec; // Temporarily commented to find usages

// Conditional imports for different environments
#[cfg(feature = "std")]
use std;
// Binary std/no_std choice
#[cfg(feature = "std")]
// use std::boxed::Box; // Temporarily commented to find usages
#[cfg(feature = "std")]
use std::fmt;

// Core imports
use wrt_error::{codes, Error, ErrorCategory, Result as WrtResult};

// Publicly re-export FloatBits32 and FloatBits64 from the local float_repr module
pub use crate::float_repr::{FloatBits32, FloatBits64};
// // use std::format; // Removed: format! should come from prelude
use crate::traits::LittleEndian as TraitLittleEndian; // Alias trait
                                                      // Use the canonical LittleEndian trait and BytesWriter from crate::traits
use crate::traits::{
    BoundedCapacity, BytesWriter, Checksummable, DefaultMemoryProvider, FromBytes, LittleEndian,
    ReadStream, ToBytes, WriteStream,
};
use crate::types::{ValueType, MAX_ARRAY_ELEMENTS, MAX_STRUCT_FIELDS}; // Import ValueType and RefType
use crate::{
    bounded::BoundedVec,
    prelude::{Debug, Eq, PartialEq},
    verification::Checksum,
    MemoryProvider,
}; // Added for Checksummable

/// GC-managed struct reference for WebAssembly 3.0
#[derive(Debug, Clone, PartialEq, Eq, core::hash::Hash)]
pub struct StructRef<
    P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq = DefaultMemoryProvider,
> {
    /// Type index of the struct
    pub type_index: u32,
    /// Field values
    pub fields: BoundedVec<Value, MAX_STRUCT_FIELDS, P>,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> StructRef<P> {
    /// Create a new struct reference
    pub fn new(type_index: u32, provider: P) -> WrtResult<Self> {
        let fields = BoundedVec::new(provider).map_err(Error::from)?;
        Ok(Self { type_index, fields })
    }

    /// Set a field value
    pub fn set_field(&mut self, index: usize, value: Value) -> WrtResult<()> {
        if index < self.fields.len() {
            self.fields.set(index, value).map_err(Error::from).map(|_| ())
        } else {
            Err(Error::validation_error("Field index out of bounds",
            ))
        }
    }

    /// Get a field value
    pub fn get_field(&self, index: usize) -> WrtResult<Value> {
        self.fields.get(index).map_err(Error::from)
    }

    /// Add a field value
    pub fn add_field(&mut self, value: Value) -> WrtResult<()> {
        self.fields.push(value).map_err(Error::from)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for StructRef<P>
{
    fn default() -> Self {
        let provider = P::default();
        Self::new(0, provider).expect("Default StructRef creation failed")
    }
}

/// GC-managed array reference for WebAssembly 3.0
#[derive(Debug, Clone, PartialEq, Eq, core::hash::Hash)]
pub struct ArrayRef<
    P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq = DefaultMemoryProvider,
> {
    /// Type index of the array
    pub type_index: u32,
    /// Array elements
    pub elements: BoundedVec<Value, MAX_ARRAY_ELEMENTS, P>,
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ArrayRef<P> {
    /// Create a new array reference
    pub fn new(type_index: u32, provider: P) -> WrtResult<Self> {
        let elements = BoundedVec::new(provider).map_err(Error::from)?;
        Ok(Self { type_index, elements })
    }

    /// Create an array with initial size and value
    pub fn with_size(
        type_index: u32,
        size: usize,
        init_value: Value,
        provider: P,
    ) -> WrtResult<Self> {
        let mut elements = BoundedVec::new(provider).map_err(Error::from)?;
        for _ in 0..size {
            elements.push(init_value.clone()).map_err(Error::from)?;
        }
        Ok(Self { type_index, elements })
    }

    /// Get array length
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if array is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get element at index
    pub fn get(&self, index: usize) -> WrtResult<Value> {
        self.elements.get(index).map_err(Error::from)
    }

    /// Set element at index
    pub fn set(&mut self, index: usize, value: Value) -> WrtResult<()> {
        if index < self.elements.len() {
            self.elements.set(index, value).map_err(Error::from).map(|_| ())
        } else {
            Err(Error::validation_error("Array index out of bounds",
            ))
        }
    }

    /// Push element to array
    pub fn push(&mut self, value: Value) -> WrtResult<()> {
        self.elements.push(value).map_err(Error::from)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Default
    for ArrayRef<P>
{
    fn default() -> Self {
        let provider = P::default();
        Self::new(0, provider).expect("Default ArrayRef creation failed")
    }
}

/// Represents a WebAssembly runtime value
#[derive(Debug, Clone, core::hash::Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub enum Value {
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float
    F32(FloatBits32),
    /// 64-bit float
    F64(FloatBits64),
    /// 128-bit vector
    V128(V128),
    /// Function reference
    FuncRef(Option<FuncRef>),
    /// External reference
    ExternRef(Option<ExternRef>),
    /// Generic opaque reference (often an index), serialized as a u32/i32.
    Ref(u32),
    /// 16-bit vector (represented internally as V128)
    I16x8(V128),
    /// Struct reference (WebAssembly 3.0 GC)
    StructRef(Option<StructRef<DefaultMemoryProvider>>),
    /// Array reference (WebAssembly 3.0 GC)
    ArrayRef(Option<ArrayRef<DefaultMemoryProvider>>),
}

// Manual PartialEq implementation for Value
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::I32(a), Value::I32(b)) => a == b,
            (Value::I64(a), Value::I64(b)) => a == b,
            // Handle NaN comparison for floats: NaN != NaN
            (Value::F32(a), Value::F32(b)) => {
                (a.value().is_nan() && b.value().is_nan()) || (a.value() == b.value())
            }
            (Value::F64(a), Value::F64(b)) => {
                (a.value().is_nan() && b.value().is_nan()) || (a.value() == b.value())
            }
            (Value::V128(a), Value::V128(b)) => a == b,
            (Value::FuncRef(a), Value::FuncRef(b)) => a == b,
            (Value::ExternRef(a), Value::ExternRef(b)) => a == b,
            (Value::Ref(a), Value::Ref(b)) => a == b,
            (Value::I16x8(a), Value::I16x8(b)) => a == b,
            (Value::StructRef(a), Value::StructRef(b)) => a == b,
            (Value::ArrayRef(a), Value::ArrayRef(b)) => a == b,
            _ => false, // Different types are not equal
        }
    }
}

impl Eq for Value {}

impl Default for Value {
    fn default() -> Self {
        // A common default, often I32(0) is used, or based on what's most frequent /
        // safest.
        Value::I32(0)
    }
}

/// A WebAssembly v128 value used for SIMD operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct V128 {
    /// The 128-bit value represented as 16 bytes
    pub bytes: [u8; 16],
}

impl V128 {
    /// Create a new v128 value from 16 bytes
    #[must_use]
    pub fn new(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    /// Create a v128 filled with zeros
    #[must_use]
    pub const fn zero() -> Self {
        Self { bytes: [0; 16] }
    }
}

impl AsRef<[u8]> for V128 {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

// Create a helper function for creating a v128 value
/// Helper function to create a new V128 value
#[must_use]
pub fn v128(bytes: [u8; 16]) -> V128 {
    V128::new(bytes)
}

/// Function reference type
#[derive(Debug, Clone, PartialEq, Eq, core::hash::Hash)]
pub struct FuncRef {
    /// Function index
    pub index: u32,
}

impl FuncRef {
    /// Creates a new `FuncRef` from an index
    #[must_use]
    pub fn from_index(index: u32) -> Self {
        Self { index }
    }
}

/// External reference type
#[derive(Debug, Clone, PartialEq, Eq, core::hash::Hash)]
pub struct ExternRef {
    /// Reference index
    pub index: u32,
}

impl Value {
    /// Creates a default value for the given WebAssembly value type.
    ///
    /// This function returns a zero value for numeric types and None for
    /// reference types.
    #[must_use]
    pub const fn default_for_type(ty: &ValueType) -> Self {
        match ty {
            ValueType::I32 => Value::I32(0),
            ValueType::I64 => Value::I64(0),
            ValueType::F32 => Value::F32(FloatBits32(0)),
            ValueType::F64 => Value::F64(FloatBits64(0)),
            ValueType::V128 => Value::V128(V128::zero()),
            ValueType::I16x8 => Value::I16x8(V128::zero()),
            ValueType::FuncRef => Value::FuncRef(None),
            ValueType::ExternRef => Value::ExternRef(None),
            ValueType::StructRef(_) => Value::StructRef(None),
            ValueType::ArrayRef(_) => Value::ArrayRef(None),
        }
    }

    /// Returns the value type of this `Value`.
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::F32(_) => ValueType::F32,
            Self::F64(_) => ValueType::F64,
            Self::V128(_) => ValueType::V128,
            Self::I16x8(_) => ValueType::I16x8,
            Self::FuncRef(_) => ValueType::FuncRef,
            Self::ExternRef(_) => ValueType::ExternRef,
            Self::Ref(_) => ValueType::I32,
            Self::StructRef(Some(s)) => ValueType::StructRef(s.type_index),
            Self::StructRef(None) => ValueType::StructRef(0), // Default type index for null
            Self::ArrayRef(Some(a)) => ValueType::ArrayRef(a.type_index),
            Self::ArrayRef(None) => ValueType::ArrayRef(0), // Default type index for null
        }
    }

    /// Checks if the value matches the provided type.
    #[must_use]
    pub const fn matches_type(&self, ty: &ValueType) -> bool {
        match (self, ty) {
            (Self::I32(_), ValueType::I32) => true,
            (Self::I64(_), ValueType::I64) => true,
            (Self::F32(_), ValueType::F32) => true,
            (Self::F64(_), ValueType::F64) => true,
            (Self::V128(_), ValueType::V128) => true,
            (Self::I16x8(_), ValueType::I16x8) => true,
            (Self::FuncRef(_), ValueType::FuncRef) => true,
            (Self::ExternRef(_), ValueType::ExternRef) => true,
            (Self::Ref(_), ValueType::I32) => true,
            (Self::StructRef(Some(s)), ValueType::StructRef(idx)) => s.type_index == *idx,
            (Self::StructRef(None), ValueType::StructRef(_)) => true, // Null matches any struct type
            (Self::ArrayRef(Some(a)), ValueType::ArrayRef(idx)) => a.type_index == *idx,
            (Self::ArrayRef(None), ValueType::ArrayRef(_)) => true, // Null matches any array type
            _ => false,
        }
    }

    /// Returns the underlying value as a `u32` if it's an `i32`.
    #[must_use]
    pub const fn as_u32(&self) -> Option<u32> {
        match *self {
            Value::I32(val) => Some(val as u32),
            _ => None,
        }
    }

    /// Returns the underlying value as an `i32` if it's an `i32`.
    #[must_use]
    pub const fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::I32(val) => Some(val),
            _ => None,
        }
    }

    /// Tries to convert the `Value` into an `i32`.
    /// Returns an error if the value is not an `I32`.
    pub fn into_i32(self) -> WrtResult<i32> {
        match self {
            Value::I32(v) => Ok(v),
            _ => {
                Err(Error::type_error("Value is not an i32"))
            }
        }
    }

    /// Attempts to extract an i64 value if this Value is an I64.
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Attempts to extract an f32 value if this Value is an F32.
    #[must_use]
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(v) => Some(v.value()),
            _ => None,
        }
    }

    /// Attempts to extract an f64 value if this Value is an F64.
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(v) => Some(v.value()),
            _ => None,
        }
    }

    /// Attempts to extract a `FuncRef` index if this Value is a `FuncRef`.
    pub fn as_func_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::FuncRef(fr) => Some(fr.as_ref().map(|r| r.index)),
            _ => None,
        }
    }

    /// Attempts to extract an `ExternRef` index if this Value is an
    /// `ExternRef`.
    pub fn as_extern_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::ExternRef(er) => Some(er.as_ref().map(|r| r.index)),
            _ => None,
        }
    }

    /// Returns the underlying `u32` if this `Value` is a `Ref`.
    #[must_use]
    pub const fn as_ref_u32(&self) -> Option<u32> {
        match self {
            Self::Ref(val) => Some(*val),
            _ => None,
        }
    }

    /// Attempts to interpret this `Value` as a boolean (`false` if zero, `true`
    /// otherwise). Only applicable to integer types `I32` and `I64`.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::I32(v) => Some(*v != 0),
            Self::I64(v) => Some(*v != 0),
            _ => None,
        }
    }

    /// Attempts to extract an i8 value if this `Value` is an `I32`.
    #[must_use]
    pub const fn as_i8(&self) -> Option<i8> {
        match self {
            Self::I32(v) => Some(*v as i8),
            _ => None,
        }
    }

    /// Attempts to extract a u8 value if this `Value` is an `I32`.
    #[must_use]
    pub const fn as_u8(&self) -> Option<u8> {
        match self {
            Self::I32(v) => Some(*v as u8),
            _ => None,
        }
    }

    /// Attempts to extract an i16 value if this `Value` is an `I32`.
    #[must_use]
    pub const fn as_i16(&self) -> Option<i16> {
        match self {
            Self::I32(v) => Some(*v as i16),
            _ => None,
        }
    }

    /// Attempts to extract a u16 value if this `Value` is an `I32`.
    #[must_use]
    pub const fn as_u16(&self) -> Option<u16> {
        match self {
            Self::I32(v) => Some(*v as u16),
            _ => None,
        }
    }

    /// Attempts to extract the bytes of a V128 value.
    pub fn as_v128(&self) -> WrtResult<[u8; 16]> {
        match self {
            Self::V128(v) => Ok(v.bytes),
            Self::I16x8(v) => Ok(v.bytes), // I16x8 is also V128 internally
            _ => Err(Error::runtime_execution_error(
                "Value is not a V128 or I16x8 type"
            )),
        }
    }

    /// Tries to convert the `Value` into an `i32` after truncating from `f32`.
    /// Returns an error if the value is not an `F32` or if truncation fails.
    pub fn into_i32_from_f32(self) -> WrtResult<i32> {
        match self {
            Value::F32(f_val) => {
                let f = f_val.value();
                if f.is_nan() || f.is_infinite() {
                    Err(Error::type_error(")",
                    ))
                } else if f < (i32::MIN as f32) || f > (i32::MAX as f32) {
                    Err(Error::type_error("Invalid f32 to i32 conversion (overflow)",
                    ))
                } else {
                    Ok(f as i32)
                }
            }
            _ => Err(Error::type_error("Value is not an f32 for i32 conversion",
            )),
        }
    }

    /// Tries to convert the `Value` into an `i64` after truncating from `f64`.
    /// Returns an error if the value is not an `F64` or if truncation fails.
    pub fn into_i64_from_f64(self) -> WrtResult<i64> {
        match self {
            Value::F64(f_val) => {
                let f = f_val.value();
                if f.is_nan() || f.is_infinite() {
                    Err(Error::type_error("Invalid f64 to i64 conversion (NaN/Inf)",
                    ))
                } else if f < (i64::MIN as f64) || f > (i64::MAX as f64) {
                    Err(Error::type_error("Invalid f64 to i64 conversion (overflow)",
                    ))
                } else {
                    Ok(f as i64)
                }
            }
            _ => Err(Error::type_error("Value is not an f64 for i64 conversion",
            )),
        }
    }

    /// Creates a `FuncRef` value.
    #[must_use]
    pub fn func_ref(func_idx: Option<u32>) -> Self {
        match func_idx {
            Some(idx) => Value::FuncRef(Some(FuncRef::from_index(idx))),
            None => Value::FuncRef(None),
        }
    }

    /// Writes the `Value` to the given writer in little-endian format.
    pub fn write_le_bytes<W: BytesWriter>(&self, writer: &mut W) -> WrtResult<()> {
        match self {
            Value::I32(val) => writer.write_all(&val.to_le_bytes()),
            Value::I64(val) => writer.write_all(&val.to_le_bytes()),
            Value::F32(val) => writer.write_all(&val.0.to_le_bytes()),
            Value::F64(val) => writer.write_all(&val.0.to_le_bytes()),
            Value::V128(val) | Value::I16x8(val) => writer.write_all(&val.bytes),
            Value::FuncRef(Some(fr)) => writer.write_all(&fr.index.to_le_bytes()),
            Value::ExternRef(Some(er)) => writer.write_all(&er.index.to_le_bytes()),
            Value::Ref(r) => writer.write_all(&r.to_le_bytes()),
            Value::FuncRef(None) | Value::ExternRef(None) => {
                // Null reference, often represented as a specific integer pattern (e.g., all
                // ones or zero) For now, let's serialize as 0, assuming it
                // represents null. This needs to align with deserialization and
                // runtime expectations.
                writer.write_all(&0u32.to_le_bytes())
            }
            Value::StructRef(Some(s)) => writer.write_all(&s.type_index.to_le_bytes()),
            Value::StructRef(None) => writer.write_all(&0u32.to_le_bytes()),
            Value::ArrayRef(Some(a)) => writer.write_all(&a.type_index.to_le_bytes()),
            Value::ArrayRef(None) => writer.write_all(&0u32.to_le_bytes()),
        }
    }

    /// Reads a `Value` from the given byte slice in little-endian format.
    pub fn from_le_bytes(bytes: &[u8], ty: &ValueType) -> WrtResult<Self> {
        match ty {
            ValueType::I32 => {
                if bytes.len() < 4 {
                    return Err(Error::parse_error("Insufficient bytes for I32"));
                }
                Ok(Value::I32(i32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                    Error::runtime_execution_error("Failed to convert bytes to i32")
                })?)))
            }
            ValueType::I64 => {
                if bytes.len() < 8 {
                    return Err(Error::parse_error("Insufficient bytes for I64"));
                }
                Ok(Value::I64(i64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
                    Error::runtime_execution_error("Failed to convert bytes to i64")
                })?)))
            }
            ValueType::F32 => {
                if bytes.len() < 4 {
                    return Err(Error::parse_error("Insufficient bytes for F32"));
                }
                Ok(Value::F32(FloatBits32(u32::from_le_bytes(bytes[0..4].try_into().map_err(
                    |_| {
                        Error::runtime_execution_error("Failed to convert bytes to f32")
                    },
                )?))))
            }
            ValueType::F64 => {
                if bytes.len() < 8 {
                    return Err(Error::parse_error("Insufficient bytes for F64"));
                }
                Ok(Value::F64(FloatBits64(u64::from_le_bytes(bytes[0..8].try_into().map_err(
                    |_| {
                        Error::runtime_execution_error("Failed to convert bytes to f64")
                    },
                )?))))
            }
            ValueType::V128 | ValueType::I16x8 => {
                if bytes.len() < 16 {
                    return Err(Error::parse_error("Insufficient bytes for V128/I16x8"));
                }
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&bytes[0..16]);
                if *ty == ValueType::V128 {
                    Ok(Value::V128(V128 { bytes: arr }))
                } else {
                    Ok(Value::I16x8(V128 { bytes: arr }))
                }
            }
            ValueType::FuncRef => {
                if bytes.len() < 4 {
                    return Err(Error::parse_error("Insufficient bytes for FuncRef"));
                }
                let idx = u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                    Error::runtime_execution_error("Failed to convert bytes to FuncRef index")
                })??);
                // Assuming 0 or a specific pattern might mean None, for now, always Some.
                // The interpretation of the index (e.g. if 0 means null) is context-dependent.
                Ok(Value::FuncRef(Some(FuncRef::from_index(idx))))
            }
            ValueType::ExternRef => {
                if bytes.len() < 4 {
                    return Err(Error::parse_error("Insufficient bytes for ExternRef"));
                }
                let idx = u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                    Error::runtime_execution_error("Failed to convert bytes to ExternRef index")
                })?);
                Ok(Value::ExternRef(Some(ExternRef { index: idx })))
            }
            ValueType::StructRef(_) => {
                // For aggregate types, we don't support direct byte deserialization yet
                // These require more complex GC-aware deserialization
                Ok(Value::StructRef(None))
            }
            ValueType::ArrayRef(_) => {
                // For aggregate types, we don't support direct byte deserialization yet
                // These require more complex GC-aware deserialization
                Ok(Value::ArrayRef(None))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I32(v) => write!(f, "i32:{v}"),
            Value::I64(v) => write!(f, "i64:{v}"),
            Value::F32(v) => write!(f, "f32:{}", v.value()),
            Value::F64(v) => write!(f, "f64:{}", v.value()),
            Value::V128(v) => write!(f, "v128:{v:?}"),
            Value::FuncRef(Some(v)) => write!(f, "funcref:{}", v.index),
            Value::FuncRef(None) => write!(f, "funcref:null"),
            Value::ExternRef(Some(v)) => write!(f, "externref:{}", v.index),
            Value::ExternRef(None) => write!(f, "externref:null"),
            Value::Ref(v) => write!(f, "ref:{v}"),
            Value::I16x8(v) => write!(f, "i16x8:{v:?}"),
            Value::StructRef(Some(v)) => write!(f, "structref:type{}", v.type_index),
            Value::StructRef(None) => write!(f, "structref:null"),
            Value::ArrayRef(Some(v)) => write!(f, "arrayref:type{}[{}]", v.type_index, v.len()),
            Value::ArrayRef(None) => write!(f, "arrayref:null"),
        }
    }
}

/// `AsRef<[u8]>` implementation for Value
///
/// This implementation allows a Value to be treated as a byte slice
/// reference. It is primarily used for memory operations.
impl AsRef<[u8]> for Value {
    fn as_ref(&self) -> &[u8] {
        // This implementation is problematic as Value doesn't have a direct, simple
        // byte representation. It should likely be removed or rethought. For
        // now, returning an empty slice to satisfy a potential trait bound
        // elsewhere, but this needs review. panic!("Value::as_ref<[u8]> is not
        // meaningfully implemented";
        &[] // Placeholder, likely incorrect for general use
    }
}

// Implement LittleEndian for V128 here as V128 is defined in this module.
impl LittleEndian for V128 {
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
        if bytes.len() != 16 {
            return Err(Error::runtime_execution_error(
                "V128 requires exactly 16 bytes"
            ));
        }
        let arr: [u8; 16] = bytes.try_into().map_err(|_| {
            Error::new(
                ErrorCategory::System,
                codes::CONVERSION_ERROR,
                "Failed to convert slice to V128 byte array")
        })?;
        Ok(V128 { bytes: arr })
    }

    fn write_le_bytes<W: BytesWriter>(&self, writer: &mut W) -> WrtResult<()> {
        writer.write_all(&self.bytes)
    }
}

impl Checksummable for V128 {
    fn update_checksum(&self, checksum: &mut Checksum) {
        checksum.update_slice(&self.bytes);
    }
}

impl ToBytes for V128 {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream, // Provider not typically needed for simple types
    ) -> WrtResult<()> {
        // Write the bytes directly to the stream
        writer.write_all(&self.bytes)
    }
    // to_bytes method is provided by the trait with DefaultMemoryProvider
}

impl FromBytes for V128 {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream, // Provider not typically needed for simple types
    ) -> WrtResult<Self> {
        // Read exactly 16 bytes for V128
        let mut arr = [0u8; 16];
        reader.read_exact(&mut arr)?;
        Ok(V128 { bytes: arr })
    }
    // from_bytes method is provided by the trait with DefaultMemoryProvider
}

impl Checksummable for FuncRef {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.index.update_checksum(checksum);
    }
}

impl ToBytes for FuncRef {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        // Delegate to the u32 implementation
        self.index.to_bytes_with_provider(writer, provider)
    }
    // to_bytes method is provided by the trait with DefaultMemoryProvider
}

impl FromBytes for FuncRef {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        // Delegate to the u32 implementation
        let index = u32::from_bytes_with_provider(reader, provider)?;
        Ok(FuncRef { index })
    }
    // from_bytes method is provided by the trait with DefaultMemoryProvider
}

impl Checksummable for ExternRef {
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.index.update_checksum(checksum);
    }
}

impl ToBytes for ExternRef {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        // Delegate to the u32 implementation
        self.index.to_bytes_with_provider(writer, provider)
    }
    // to_bytes method is provided by the trait with DefaultMemoryProvider
}

impl FromBytes for ExternRef {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        // Delegate to the u32 implementation
        let index = u32::from_bytes_with_provider(reader, provider)?;
        Ok(ExternRef { index })
    }
    // from_bytes method is provided by the trait with DefaultMemoryProvider
}

impl Checksummable for Value {
    fn update_checksum(&self, checksum: &mut Checksum) {
        let discriminant_byte = match self {
            Value::I32(_) => 0u8,
            Value::I64(_) => 1u8,
            Value::F32(_) => 2u8,
            Value::F64(_) => 3u8,
            Value::V128(_) => 4u8,
            Value::FuncRef(_) => 5u8,
            Value::ExternRef(_) => 6u8,
            Value::Ref(_) => 7u8,       // Generic Ref
            Value::I16x8(_) => 8u8,     // I16x8, distinct from V128 for checksum
            Value::StructRef(_) => 9u8, // Struct reference
            Value::ArrayRef(_) => 10u8, // Array reference
        };
        checksum.update(discriminant_byte);

        match self {
            Value::I32(v) => v.update_checksum(checksum),
            Value::I64(v) => v.update_checksum(checksum),
            Value::F32(v) => v.update_checksum(checksum),
            Value::F64(v) => v.update_checksum(checksum),
            Value::V128(v) | Value::I16x8(v) => v.update_checksum(checksum),
            Value::FuncRef(v) => v.update_checksum(checksum),
            Value::ExternRef(v) => v.update_checksum(checksum),
            Value::Ref(v) => v.update_checksum(checksum),
            Value::StructRef(v) => v.update_checksum(checksum),
            Value::ArrayRef(v) => v.update_checksum(checksum),
        }
    }
}

impl ToBytes for Value {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        // Write discriminant byte
        let discriminant = match self {
            Value::I32(_) => 0u8,
            Value::I64(_) => 1u8,
            Value::F32(_) => 2u8,
            Value::F64(_) => 3u8,
            Value::V128(_) => 4u8,
            Value::FuncRef(_) => 5u8,
            Value::ExternRef(_) => 6u8,
            Value::Ref(_) => 7u8,       // Generic Ref, serialized as u32
            Value::I16x8(_) => 8u8,     // I16x8, serialized as V128
            Value::StructRef(_) => 9u8, // Struct reference
            Value::ArrayRef(_) => 10u8, // Array reference
        };
        writer.write_u8(discriminant)?;

        // Write the variant data
        match self {
            Value::I32(v) => v.to_bytes_with_provider(writer, provider)?,
            Value::I64(v) => v.to_bytes_with_provider(writer, provider)?,
            Value::F32(v) => v.to_bytes_with_provider(writer, provider)?,
            Value::F64(v) => v.to_bytes_with_provider(writer, provider)?,
            Value::V128(v) | Value::I16x8(v) => v.to_bytes_with_provider(writer, provider)?,
            Value::FuncRef(opt_v) => {
                // Write Some/None flag
                writer.write_u8(if opt_v.is_some() { 1 } else { 0 })?;
                if let Some(v) = opt_v {
                    v.to_bytes_with_provider(writer, provider)?
                }
            }
            Value::ExternRef(opt_v) => {
                // Write Some/None flag
                writer.write_u8(if opt_v.is_some() { 1 } else { 0 })?;
                if let Some(v) = opt_v {
                    v.to_bytes_with_provider(writer, provider)?
                }
            }
            Value::Ref(v) => v.to_bytes_with_provider(writer, provider)?,
            Value::StructRef(opt_v) => {
                // Write Some/None flag
                writer.write_u8(if opt_v.is_some() { 1 } else { 0 })?;
                if let Some(v) = opt_v {
                    v.to_bytes_with_provider(writer, provider)?
                }
            }
            Value::ArrayRef(opt_v) => {
                // Write Some/None flag
                writer.write_u8(if opt_v.is_some() { 1 } else { 0 })?;
                if let Some(v) = opt_v {
                    v.to_bytes_with_provider(writer, provider)?
                }
            }
        }
        Ok(())
    }
}

impl FromBytes for Value {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        // Read discriminant byte
        let discriminant = reader.read_u8()?;

        // Parse the variant based on discriminant
        match discriminant {
            0 => {
                let v = i32::from_bytes_with_provider(reader, provider)?;
                Ok(Value::I32(v))
            }
            1 => {
                let v = i64::from_bytes_with_provider(reader, provider)?;
                Ok(Value::I64(v))
            }
            2 => {
                let v = FloatBits32::from_bytes_with_provider(reader, provider)?;
                Ok(Value::F32(v))
            }
            3 => {
                let v = FloatBits64::from_bytes_with_provider(reader, provider)?;
                Ok(Value::F64(v))
            }
            4 => {
                let v = V128::from_bytes_with_provider(reader, provider)?;
                Ok(Value::V128(v))
            }
            5 => {
                // FuncRef
                let is_some = reader.read_u8()? == 1;
                if is_some {
                    let v = FuncRef::from_bytes_with_provider(reader, provider)?;
                    Ok(Value::FuncRef(Some(v)))
                } else {
                    Ok(Value::FuncRef(None))
                }
            }
            6 => {
                // ExternRef
                let is_some = reader.read_u8()? == 1;
                if is_some {
                    let v = ExternRef::from_bytes_with_provider(reader, provider)?;
                    Ok(Value::ExternRef(Some(v)))
                } else {
                    Ok(Value::ExternRef(None))
                }
            }
            7 => {
                // Ref (u32)
                let v = u32::from_bytes_with_provider(reader, provider)?;
                Ok(Value::Ref(v))
            }
            8 => {
                // I16x8 (V128)
                let v = V128::from_bytes_with_provider(reader, provider)?;
                Ok(Value::I16x8(v))
            }
            9 => {
                // StructRef
                let is_some = reader.read_u8()? == 1;
                if is_some {
                    let v = StructRef::from_bytes_with_provider(reader, provider)?;
                    Ok(Value::StructRef(Some(v)))
                } else {
                    Ok(Value::StructRef(None))
                }
            }
            10 => {
                // ArrayRef
                let is_some = reader.read_u8()? == 1;
                if is_some {
                    let v = ArrayRef::from_bytes_with_provider(reader, provider)?;
                    Ok(Value::ArrayRef(Some(v)))
                } else {
                    Ok(Value::ArrayRef(None))
                }
            }
            _ => Err(Error::runtime_execution_error(
                "Unknown discriminant byte in Value deserialization"
            )),
        }
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for StructRef<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.type_index.update_checksum(checksum);
        self.fields.update_checksum(checksum);
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ToBytes
    for StructRef<P>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        // Write type index
        self.type_index.to_bytes_with_provider(writer, provider)?;
        // Write field count
        writer.write_u32_le(self.fields.len() as u32)?;
        // Write fields
        for field in self.fields.iter() {
            field.to_bytes_with_provider(writer, provider)?;
        }
        Ok(())
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FromBytes
    for StructRef<P>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        // Read type index
        let type_index = u32::from_bytes_with_provider(reader, provider)?;
        // Read field count
        let field_count = reader.read_u32_le()?;
        // Create struct with default provider
        let mut struct_ref = StructRef::new(type_index, P::default())?;
        // Read fields
        for _ in 0..field_count {
            let field = Value::from_bytes_with_provider(reader, provider)?;
            struct_ref.add_field(field)?;
        }
        Ok(struct_ref)
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> Checksummable
    for ArrayRef<P>
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.type_index.update_checksum(checksum);
        self.elements.update_checksum(checksum);
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> ToBytes
    for ArrayRef<P>
{
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        // Write type index
        self.type_index.to_bytes_with_provider(writer, provider)?;
        // Write element count
        writer.write_u32_le(self.elements.len() as u32)?;
        // Write elements
        for element in self.elements.iter() {
            element.to_bytes_with_provider(writer, provider)?;
        }
        Ok(())
    }
}

impl<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq> FromBytes
    for ArrayRef<P>
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        // Read type index
        let type_index = u32::from_bytes_with_provider(reader, provider)?;
        // Read element count
        let element_count = reader.read_u32_le()?;
        // Create array with default provider
        let mut array_ref = ArrayRef::new(type_index, P::default())?;
        // Read elements
        for _ in 0..element_count {
            let element = Value::from_bytes_with_provider(reader, provider)?;
            array_ref.push(element)?;
        }
        Ok(array_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RefType, ValueType};

    #[test]
    fn test_value_type() {
        assert_eq!(Value::I32(0).value_type(), ValueType::I32);
        assert_eq!(Value::I64(0).value_type(), ValueType::I64);
        assert_eq!(Value::F32(FloatBits32(0)).value_type(), ValueType::F32);
        assert_eq!(Value::F64(FloatBits64(0)).value_type(), ValueType::F64);
        assert_eq!(Value::V128(V128::zero()).value_type(), ValueType::V128);
        assert_eq!(Value::I16x8(V128::zero()).value_type(), ValueType::I16x8);
        assert_eq!(Value::FuncRef(None).value_type(), ValueType::FuncRef);
        assert_eq!(Value::ExternRef(None).value_type(), ValueType::ExternRef);
        assert_eq!(Value::Ref(0).value_type(), ValueType::I32);
    }

    #[test]
    fn test_value_matches_type() {
        assert!(Value::I32(0).matches_type(&ValueType::I32));
        assert!(!Value::I32(0).matches_type(&ValueType::I64));
        assert!(Value::V128(V128::zero()).matches_type(&ValueType::V128));
        assert!(Value::I16x8(V128::zero()).matches_type(&ValueType::I16x8));
        assert!(!Value::V128(V128::zero()).matches_type(&ValueType::I32));
        assert!(Value::Ref(1).matches_type(&ValueType::I32));
        assert!(Value::Ref(1).matches_type(&ValueType::I32));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_little_endian_conversion() {
        let val_i32 = Value::I32(0x1234_5678);
        let bytes_i32 = val_i32.to_le_bytes().unwrap();
        assert_eq!(bytes_i32, vec![0x78, 0x56, 0x34, 0x12]);
        let recovered_i32 = Value::from_le_bytes(&bytes_i32, &ValueType::I32).unwrap();
        assert_eq!(val_i32, recovered_i32);

        let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let val_v128 = Value::V128(V128::new(bytes));
        let bytes_v128 = val_v128.to_le_bytes().unwrap();
        assert_eq!(bytes_v128, bytes.to_vec());
        let recovered_v128 = Value::from_le_bytes(&bytes_v128, &ValueType::V128).unwrap();
        assert_eq!(val_v128, recovered_v128);

        let val_i16x8 = Value::I16x8(V128::new(bytes));
        let bytes_i16x8 = val_i16x8.to_le_bytes().unwrap();
        assert_eq!(bytes_i16x8, bytes.to_vec());
        let recovered_i16x8 = Value::from_le_bytes(&bytes_i16x8, &ValueType::I16x8).unwrap();
        assert_eq!(val_i16x8, recovered_i16x8);

        assert!(Value::from_le_bytes(&[1, 2], &ValueType::I32).is_err());
        assert!(Value::FuncRef(None).to_le_bytes().is_err());
    }

    #[test]
    fn test_default_for_type() {
        assert_eq!(Value::default_for_type(&ValueType::I32), Value::I32(0));
        assert_eq!(Value::default_for_type(&ValueType::I64), Value::I64(0));
        assert_eq!(Value::default_for_type(&ValueType::F32), Value::F32(FloatBits32(0)));
        assert_eq!(Value::default_for_type(&ValueType::F64), Value::F64(FloatBits64(0)));
        assert_eq!(Value::default_for_type(&ValueType::V128), Value::V128(V128::zero()));
        assert_eq!(Value::default_for_type(&ValueType::I16x8), Value::I16x8(V128::zero()));
        assert_eq!(Value::default_for_type(&ValueType::FuncRef), Value::Ref(0));
        assert_eq!(Value::default_for_type(&ValueType::ExternRef), Value::ExternRef(None));
        assert_eq!(Value::default_for_type(&ValueType::Ref(RefType::Func)), Value::Ref(0));
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(Value::I32(10), Value::I32(10));
        assert_ne!(Value::I32(10), Value::I32(11));
        assert_ne!(Value::I32(10), Value::I64(10));

        assert_eq!(Value::F32(FloatBits32::NAN), Value::F32(FloatBits32::NAN));
        assert_eq!(
            Value::F32(FloatBits32::from_float(f32::NAN)),
            Value::F32(FloatBits32::from_float(f32::NAN))
        );
        assert_ne!(Value::F32(FloatBits32::from_float(1.0)), Value::F32(FloatBits32::NAN));
        assert_eq!(
            Value::F32(FloatBits32::from_float(1.0)),
            Value::F32(FloatBits32::from_float(1.0))
        );

        let v1 = V128::new([1; 16]);
        let v2 = V128::new([2; 16]);
        assert_eq!(Value::V128(v1.clone()), Value::V128(v1.clone()));
        assert_ne!(Value::V128(v1.clone()), Value::V128(v2.clone()));
        assert_eq!(Value::I16x8(v1.clone()), Value::I16x8(v1.clone()));
        assert_ne!(Value::I16x8(v1.clone()), Value::I16x8(v2.clone()));
    }
}
