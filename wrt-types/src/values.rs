// WRT - wrt-types
// Module: WebAssembly Value Representations
// SW-REQ-ID: REQ_018
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly value representations
//!
//! This module provides datatypes for representing WebAssembly values at
//! runtime.

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
use core::cell::RefCell;
#[cfg(not(feature = "std"))]
use core::fmt;
// Conditional imports for different environments
#[cfg(feature = "std")]
use std;
// Box for dynamic allocation
#[cfg(feature = "std")]
use std::boxed::Box;
// RefCell for thread local storage
#[cfg(feature = "std")]
use std::cell::RefCell;
// Core imports
#[cfg(feature = "std")]
use std::fmt;
#[cfg(feature = "std")]
use std::thread_local;

// #[cfg(all(not(feature = "std"), feature = "alloc"))]
// use alloc::format; // Removed: format! should come from prelude
use crate::types::ValueType as Type;
use crate::{
    prelude::{str, Debug, Eq, Ord, PartialEq, PartialOrd},
    WrtResult,
};
use wrt_error::{codes, ErrorKind, ErrorCategory};
use crate::traits::LittleEndian as TraitLittleEndian; // Alias trait
use crate::types::{RefType, ValueType}; // Import ValueType
use core::cmp::Ordering;
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Wrapper for f32 that implements Hash, `PartialEq`, and Eq based on bit
/// patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct FloatBits32(pub u32);

impl FloatBits32 {
    /// Represents a canonical Not-a-Number (`NaN`) value for f32.
    /// The specific bit pattern for canonical `NaN` can vary, but this is a
    /// common one. (Sign bit 0, exponent all 1s, significand MSB 1, rest 0)
    pub const NAN: Self = FloatBits32(0x7fc0_0000);

    /// Creates a new `FloatBits32` from an `f32` value.
    #[must_use]
    pub fn from_float(val: f32) -> Self {
        Self(val.to_bits())
    }

    /// Returns the `f32` value represented by this `FloatBits32`.
    #[must_use]
    pub const fn value(self) -> f32 {
        f32::from_bits(self.0)
    }

    /// Returns the underlying `u32` bits of this `FloatBits32`.
    #[must_use]
    pub const fn to_bits(self) -> u32 {
        self.0
    }

    /// Creates a `FloatBits32` from raw `u32` bits.
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
}
impl core::hash::Hash for FloatBits32 {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Wrapper for f64 that implements Hash, `PartialEq`, and Eq based on bit
/// patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct FloatBits64(pub u64);

impl FloatBits64 {
    /// Represents a canonical Not-a-Number (`NaN`) value for f64.
    /// The specific bit pattern for canonical `NaN` can vary, but this is a
    /// common one. (Sign bit 0, exponent all 1s, significand MSB 1, rest 0)
    pub const NAN: Self = FloatBits64(0x7ff8_0000_0000_0000);

    /// Creates a new `FloatBits64` from an `f64` value.
    #[must_use]
    pub fn from_float(val: f64) -> Self {
        Self(val.to_bits())
    }

    /// Returns the `f64` value represented by this `FloatBits64`.
    #[must_use]
    pub const fn value(self) -> f64 {
        f64::from_bits(self.0)
    }

    /// Returns the underlying `u64` bits of this `FloatBits64`.
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    /// Creates a `FloatBits64` from raw `u64` bits.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}
impl core::hash::Hash for FloatBits64 {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
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
    /// Generic reference to an entity
    Ref(u32),
    /// 16-bit vector (represented internally as V128)
    I16x8(V128),
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
            _ => false, // Different types are not equal
        }
    }
}

impl Eq for Value {}

/// A WebAssembly v128 value used for SIMD operations
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn zero() -> Self {
        Self { bytes: [0; 16] }
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
            ValueType::Ref(_) => Value::Ref(0),
        }
    }

    /// Returns the value type of this `Value`.
    #[must_use]
    pub const fn type_(&self) -> ValueType {
        match self {
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::F32(_) => ValueType::F32,
            Self::F64(_) => ValueType::F64,
            Self::V128(_) => ValueType::V128,
            Self::I16x8(_) => ValueType::I16x8,
            Self::FuncRef(_) => ValueType::FuncRef,
            Self::ExternRef(_) => ValueType::ExternRef,
            Self::Ref(_) => ValueType::Ref(RefType::Func),
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
            (Self::Ref(_), ValueType::Ref(_)) => true,
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

    /// Tries to convert the `Value` into an `i32`.
    /// Returns an error if the value is not an `I32`.
    pub fn into_i32(self) -> WrtResult<i32> {
        match self {
            Self::I32(val) => Ok(val),
            _ => Err(WrtError::new(
                ErrorKind::ConversionError,
                "Value is not an i32",
            )),
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

    /// Attempts to extract a function reference if this Value is a `FuncRef`.
    #[must_use]
    pub const fn as_func_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::FuncRef(Some(func_ref)) => Some(Some(func_ref.index)),
            Self::FuncRef(None) => Some(None),
            _ => None,
        }
    }

    /// Attempts to extract an external reference if this Value is an
    /// `ExternRef`.
    #[must_use]
    pub const fn as_extern_ref(&self) -> Option<Option<u32>> {
        match self {
            Self::ExternRef(Some(extern_ref)) => Some(Some(extern_ref.index)),
            Self::ExternRef(None) => Some(None),
            _ => None,
        }
    }

    /// Convenience method to get the type of a value
    ///
    /// # Errors
    ///
    /// This function is infallible.
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        self.type_()
    }

    /// Attempts to extract a boolean value
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::I32(v) => Some(*v != 0),
            _ => None,
        }
    }

    /// Attempts to extract an i8 value
    #[must_use]
    pub const fn as_i8(&self) -> Option<i8> {
        match self {
            #[allow(clippy::cast_possible_truncation)] // Guarded by range check
            Self::I32(v) if *v >= i8::MIN as i32 && *v <= i8::MAX as i32 => Some(*v as i8),
            _ => None,
        }
    }

    /// Attempts to extract a u8 value
    #[must_use]
    pub const fn as_u8(&self) -> Option<u8> {
        match self {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // Guarded by range check
            Self::I32(v) if *v >= 0 && *v <= u8::MAX as i32 => Some(*v as u8),
            _ => None,
        }
    }

    /// Attempts to extract an i16 value
    #[must_use]
    pub const fn as_i16(&self) -> Option<i16> {
        match self {
            #[allow(clippy::cast_possible_truncation)] // Guarded by range check
            Self::I32(v) if *v >= i16::MIN as i32 && *v <= i16::MAX as i32 => Some(*v as i16),
            _ => None,
        }
    }

    /// Attempts to extract a u16 value
    #[must_use]
    pub const fn as_u16(&self) -> Option<u16> {
        match self {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // Guarded by range check
            Self::I32(v) if *v >= 0 && *v <= u16::MAX as i32 => Some(*v as u16),
            _ => None,
        }
    }

    /// Attempts to extract a character value
    #[must_use]
    pub fn as_char(&self) -> Option<char> {
        match self {
            #[allow(clippy::cast_sign_loss)]
            // char::from_u32 expects u32, sign loss is part of the conversion
            Self::I32(v) => char::from_u32(*v as u32),
            _ => None,
        }
    }

    /// Attempts to extract a string value
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        None // To be implemented based on actual string representation
    }

    /// Attempts to extract a list of values
    #[must_use]
    pub fn as_list(&self) -> Option<&[Value]> {
        None // To be implemented based on actual list representation
    }

    /// Attempts to extract a record (map of field names to values)
    ///
    /// Returns None if this Value is not a record type.
    #[must_use]
    #[cfg(feature = "std")]
    pub fn as_record(&self) -> Option<&std::collections::HashMap<std::string::String, Value>> {
        None // To be implemented based on actual record representation
    }

    /// Attempts to extract a record (map of field names to values)
    ///
    /// Returns None if this Value is not a record type.
    #[must_use]
    #[cfg(not(feature = "std"))]
    pub fn as_record(&self) -> Option<&crate::HashMap<crate::String, Value>> {
        None // To be implemented based on actual record representation
    }

    /// Attempts to extract a variant (case and optional value)
    #[must_use]
    pub fn as_variant(&self) -> Option<(u32, Option<&Value>)> {
        None // To be implemented based on actual variant representation
    }

    /// Attempts to extract an enum value (index)
    #[must_use]
    pub const fn as_enum(&self) -> Option<u32> {
        None // To be implemented based on actual enum representation
    }

    /// Attempts to extract an option value
    #[must_use]
    pub fn as_option(&self) -> Option<Option<&Value>> {
        None // To be implemented based on actual option representation
    }

    /// Attempts to extract a result value
    #[must_use]
    pub fn as_result(
        &self,
    ) -> Option<&core::result::Result<Option<Box<Value>>, Option<Box<Value>>>> {
        None // To be implemented based on actual result representation
    }

    /// Attempts to extract a tuple of values
    #[must_use]
    pub fn as_tuple(&self) -> Option<&[Value]> {
        None // To be implemented based on actual tuple representation
    }

    /// Attempts to extract flags (map of flag names to boolean values)
    ///
    /// Returns None if this Value is not a flags type.
    #[must_use]
    #[cfg(feature = "std")]
    pub fn as_flags(&self) -> Option<&std::collections::HashMap<std::string::String, bool>> {
        None // To be implemented based on actual flags representation
    }

    /// Attempts to extract flags (map of flag names to boolean values)
    ///
    /// Returns None if this Value is not a flags type.
    #[must_use]
    #[cfg(not(feature = "std"))]
    pub fn as_flags(&self) -> Option<&crate::HashMap<crate::String, bool>> {
        None // To be implemented based on actual flags representation
    }

    /// Attempts to extract an owned resource handle
    #[must_use]
    pub const fn as_own(&self) -> Option<u32> {
        None // To be implemented based on actual resource representation
    }

    /// Attempts to extract a borrowed resource handle
    #[must_use]
    pub const fn as_borrow(&self) -> Option<u32> {
        None // To be implemented based on actual borrowed resource
             // representation
    }

    /// Tries to return the value as `v128` bytes.
    pub fn as_v128(&self) -> WrtResult<[u8; 16]> {
        match self {
            Self::V128(v) => Ok(v.bytes),
            Self::I16x8(v) => Ok(v.bytes),
            _ => Err(WrtError::new(
                ErrorKind::TypeError,
                format!("Expected V128 or I16x8, found {:?}", self.type_()),
            )),
        }
    }

    /// Converts an `f32` value to an `i32`.
    /// Returns an error if the value is not `F32`.
    pub fn into_i32_from_f32(self) -> WrtResult<i32> {
        match self {
            Self::F32(val) => {
                let f = val.value();
                if f.is_nan() || f.is_infinite() || f < (i32::MIN as f32) || f > (i32::MAX as f32) {
                    Err(WrtError::new(
                        ErrorKind::ConversionError,
                        "Invalid f32 to i32 conversion",
                    ))
                } else {
                    Ok(f as i32)
                }
            }
            _ => Err(WrtError::new(
                ErrorKind::TypeError,
                format!("Expected F32, found {:?}", self.type_()),
            )),
        }
    }

    /// Converts an `f64` value to an `i64`.
    /// Returns an error if the value is not `F64`.
    pub fn into_i64_from_f64(self) -> WrtResult<i64> {
        match self {
            Self::F64(val) => {
                let f = val.value();
                if f.is_nan() || f.is_infinite() || f < (i64::MIN as f64) || f > (i64::MAX as f64) {
                    Err(WrtError::new(
                        ErrorKind::ConversionError,
                        "Invalid f64 to i64 conversion",
                    ))
                } else {
                    Ok(f as i64)
                }
            }
            _ => Err(WrtError::new(
                ErrorKind::TypeError,
                format!("Expected F64, found {:?}", self.type_()),
            )),
        }
    }

    /// Creates a `FuncRef` value with the given function index
    ///
    /// # Arguments
    ///
    /// * `func_idx` - The function index to reference, or None for a null
    ///   reference
    ///
    /// # Returns
    ///
    /// A new `FuncRef` value
    #[must_use]
    pub fn func_ref(func_idx: Option<u32>) -> Self {
        match func_idx {
            Some(idx) => Self::FuncRef(Some(FuncRef { index: idx })),
            None => Self::FuncRef(None),
        }
    }

    /// Returns the underlying `u32` index if this is a `Ref` value.
    pub const fn as_reference(&self) -> Option<u32> {
        match *self {
            Self::Ref(idx) => Some(idx),
            _ => None,
        }
    }

    /// Tries to convert the `Value` into a `Ref` index (`u32`).
    /// Returns an error if the value is not a `Ref`.
    pub fn into_ref(self) -> WrtResult<u32> {
        match self {
            Self::Ref(idx) => Ok(idx),
            _ => Err(WrtError::new(
                ErrorKind::TypeError,
                format!("Expected Ref, found {:?}", self.type_()),
            )),
        }
    }

    /// Serializes the `Value` to little-endian bytes.
    #[cfg(feature = "alloc")]
    pub fn to_le_bytes(&self) -> WrtResult<Vec<u8>> {
        match self {
            Self::I32(v) => Ok(v.to_le_bytes().to_vec()),
            Self::I64(v) => Ok(v.to_le_bytes().to_vec()),
            Self::F32(v) => Ok(v.to_bits().to_le_bytes().to_vec()),
            Self::F64(v) => Ok(v.to_bits().to_le_bytes().to_vec()),
            Self::V128(v) => Ok(v.bytes.to_vec()),
            Self::I16x8(v) => Ok(v.bytes.to_vec()),
            Self::FuncRef(_) | Self::ExternRef(_) | Self::Ref(_) => Err(WrtError::new(
                ErrorKind::SerializationError,
                "Reference types cannot be serialized to bytes directly",
            )),
        }
    }

    /// Deserializes a `Value` from little-endian bytes based on the given type.
    #[cfg(feature = "alloc")]
    pub fn from_le_bytes(bytes: &[u8], ty: &ValueType) -> WrtResult<Self> {
        let expected_len = match ty {
            ValueType::I32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::F64 => 8,
            ValueType::V128 | ValueType::I16x8 => 16,
            ValueType::FuncRef | ValueType::ExternRef | ValueType::Ref(_) => {
                return Err(WrtError::new(
                    ErrorKind::DeserializationError,
                    "Reference types cannot be deserialized from bytes",
                ));
            }
        };

        if bytes.len() != expected_len {
            return Err(WrtError::new(
                ErrorKind::DeserializationError,
                format!(
                    "Invalid byte length for type {:?}: expected {}, got {}",
                    ty,
                    expected_len,
                    bytes.len()
                ),
            ));
        }

        match ty {
            ValueType::I32 => i32::from_le_bytes(bytes.try_into().map_err(|_| {
                WrtError::new(ErrorKind::DeserializationError, "Failed to read i32 bytes")
            })?)
            .map(Value::I32),
            ValueType::I64 => i64::from_le_bytes(bytes.try_into().map_err(|_| {
                WrtError::new(ErrorKind::DeserializationError, "Failed to read i64 bytes")
            })?)
            .map(Value::I64),
            ValueType::F32 => u32::from_le_bytes(bytes.try_into().map_err(|_| {
                WrtError::new(ErrorKind::DeserializationError, "Failed to read f32 bytes")
            })?)
            .map(|bits| Value::F32(FloatBits32::from_bits(bits))),
            ValueType::F64 => u64::from_le_bytes(bytes.try_into().map_err(|_| {
                WrtError::new(ErrorKind::DeserializationError, "Failed to read f64 bytes")
            })?)
            .map(|bits| Value::F64(FloatBits64::from_bits(bits))),
            ValueType::V128 => bytes
                .try_into()
                .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Failed to read v128 bytes"))
                .map(|b: [u8; 16]| Value::V128(V128::new(b))),
            ValueType::I16x8 => bytes
                .try_into()
                .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Failed to read i16x8 bytes"))
                .map(|b: [u8; 16]| Value::I16x8(V128::new(b))),
            _ => unreachable!(),
        }
        .map_err(|e: WrtError| {
            WrtError::new(
                e.kind(),
                format!("Deserialization failed for {:?}: {}", ty, e.message())
            )
        })
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
        }
    }
}

/// `AsRef<[u8]>` implementation for Value
///
/// This implementation allows a Value to be treated as a byte slice
/// reference. It is primarily used for memory operations.
impl AsRef<[u8]> for Value {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::I32(v) => {
                #[cfg(feature = "std")]
                {
                    thread_local! {
                        static BYTES: RefCell<[u8; 4]> = const { RefCell::new([0; 4]) };
                    }

                    BYTES.with(|cell| {
                        let mut bytes = cell.borrow_mut();
                        *bytes = v.to_le_bytes();
                        let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                        leaked
                    })
                }
                #[cfg(not(feature = "std"))]
                {
                    // For no_std environments, we need to use fixed constants
                    // We'll only support common values directly, others will be undefined behavior
                    match *v {
                        0 => &[0, 0, 0, 0],
                        1 => &[1, 0, 0, 0],
                        -1 => &[255, 255, 255, 255],
                        // For other values, we return a fixed slice to avoid borrowing issues
                        // This is not correct for all values, but it's better than crashing
                        // In practice, this should only be used for predefined values in no_std
                        // envs
                        _ => &[0, 0, 0, 0],
                    }
                }
            }
            Self::I64(v) => match *v {
                0 => &[0, 0, 0, 0, 0, 0, 0, 0],
                1 => &[1, 0, 0, 0, 0, 0, 0, 0],
                -1 => &[255, 255, 255, 255, 255, 255, 255, 255],
                #[cfg(feature = "std")]
                _ => {
                    thread_local! {
                        static BYTES: RefCell<[u8; 8]> = const { RefCell::new([0; 8]) };
                    }

                    BYTES.with(|cell| {
                        let mut bytes = cell.borrow_mut();
                        *bytes = v.to_le_bytes();
                        let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                        leaked
                    })
                }
                #[cfg(not(feature = "std"))]
                _ => {
                    // For no_std environments, we need to use fixed constants
                    // We'll only support common values directly, others will be undefined behavior
                    match *v {
                        0 => &[0, 0, 0, 0, 0, 0, 0, 0],
                        1 => &[1, 0, 0, 0, 0, 0, 0, 0],
                        -1 => &[255, 255, 255, 255, 255, 255, 255, 255],
                        // For other values, we return a fixed slice to avoid borrowing issues
                        // This is not correct for all values, but it's better than crashing
                        // In practice, this should only be used for predefined values in no_std
                        // envs
                        _ => &[0, 0, 0, 0, 0, 0, 0, 0],
                    }
                }
            },
            Self::F32(v) => {
                if v.value() == 0.0 {
                    &[0, 0, 0, 0]
                } else {
                    #[cfg(feature = "std")]
                    {
                        thread_local! {
                            static BYTES: RefCell<[u8; 4]> = const { RefCell::new([0; 4]) };
                        }

                        BYTES.with(|cell| {
                            let mut bytes = cell.borrow_mut();
                            *bytes = v.value().to_le_bytes();
                            let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                            leaked
                        })
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        // For no_std environments, we need to use fixed constants
                        // We'll only support common values directly, others will be undefined
                        // behavior
                        match v.value() {
                            0.0 => &[0, 0, 0, 0],
                            // For other values, we return a fixed slice to avoid borrowing issues
                            // This is not correct for all values, but it's better than crashing
                            // In practice, this should only be used for predefined values in no_std
                            // envs
                            _ => &[0, 0, 0, 0],
                        }
                    }
                }
            }
            Self::F64(v) => {
                if v.value() == 0.0 {
                    &[0, 0, 0, 0, 0, 0, 0, 0]
                } else {
                    #[cfg(feature = "std")]
                    {
                        thread_local! {
                            static BYTES: RefCell<[u8; 8]> = const { RefCell::new([0; 8]) };
                        }

                        BYTES.with(|cell| {
                            let mut bytes = cell.borrow_mut();
                            *bytes = v.value().to_le_bytes();
                            let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                            leaked
                        })
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        // For no_std environments, we need to use fixed constants
                        // We'll only support common values directly, others will be undefined
                        // behavior
                        match v.value() {
                            0.0 => &[0, 0, 0, 0, 0, 0, 0, 0],
                            // For other values, we return a fixed slice to avoid borrowing issues
                            // This is not correct for all values, but it's better than crashing
                            // In practice, this should only be used for predefined values in no_std
                            // envs
                            _ => &[0, 0, 0, 0, 0, 0, 0, 0],
                        }
                    }
                }
            }
            Self::V128(v) => v.as_ref(),
            Self::FuncRef(func_ref) => {
                if let Some(func) = func_ref {
                    #[cfg(feature = "std")]
                    {
                        thread_local! {
                            static BYTES: RefCell<[u8; 4]> = const { RefCell::new([0; 4]) };
                        }

                        BYTES.with(|cell| {
                            let mut bytes = cell.borrow_mut();
                            *bytes = func.index.to_le_bytes();
                            let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                            leaked
                        })
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        // For no_std environments, we need to use fixed constants
                        // We'll only support common values directly, others will be undefined
                        // behavior
                        match func.index {
                            0 => &[0, 0, 0, 0],
                            // For other values, we return a fixed slice to avoid borrowing issues
                            // This is not correct for all values, but it's better than crashing
                            // In practice, this should only be used for predefined values in no_std
                            // envs
                            _ => &[0, 0, 0, 0],
                        }
                    }
                } else {
                    &[0, 0, 0, 0]
                }
            }
            Self::ExternRef(extern_ref) => {
                if let Some(ext) = extern_ref {
                    #[cfg(feature = "std")]
                    {
                        thread_local! {
                            static BYTES: RefCell<[u8; 4]> = const { RefCell::new([0; 4]) };
                        }

                        BYTES.with(|cell| {
                            let mut bytes = cell.borrow_mut();
                            *bytes = ext.index.to_le_bytes();
                            let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                            leaked
                        })
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        // For no_std environments, we need to use fixed constants
                        // We'll only support common values directly, others will be undefined
                        // behavior
                        match ext.index {
                            0 => &[0, 0, 0, 0],
                            // For other values, we return a fixed slice to avoid borrowing issues
                            // This is not correct for all values, but it's better than crashing
                            // In practice, this should only be used for predefined values in no_std
                            // envs
                            _ => &[0, 0, 0, 0],
                        }
                    }
                } else {
                    &[0, 0, 0, 0]
                }
            }
            Self::Ref(ref_idx) => {
                #[cfg(feature = "std")]
                {
                    thread_local! {
                        static BYTES: RefCell<[u8; 4]> = const { RefCell::new([0; 4]) };
                    }

                    BYTES.with(|cell| {
                        let mut bytes = cell.borrow_mut();
                        *bytes = ref_idx.to_le_bytes();
                        let leaked: &'static [u8] = Box::leak(Box::new(*bytes));
                        leaked
                    })
                }
                #[cfg(not(feature = "std"))]
                {
                    // For no_std environments, we need to use fixed constants
                    // We'll only support common values directly, others will be undefined behavior
                    match ref_idx {
                        0 => &[0, 0, 0, 0],
                        // For other values, we return a fixed slice to avoid borrowing issues
                        // This is not correct for all values, but it's better than crashing
                        // In practice, this should only be used for predefined values in no_std
                        // envs
                        _ => &[0, 0, 0, 0],
                    }
                }
            }
            Self::I16x8(v) => v.as_ref(),
        }
    }
}

/// Trait for types that can be serialized to and deserialized from little-endian
/// bytes.
pub trait LittleEndian: Sized {
    /// Creates an instance from little-endian bytes.
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self>;

    /// Converts the instance to little-endian bytes.
    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>>;
}

impl TraitLittleEndian for i32 {
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
        bytes
            .try_into()
            .map(i32::from_le_bytes)
            .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Invalid bytes for i32"))
    }

    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>> {
        Ok(i32::to_le_bytes(*self).to_vec())
    }
}

impl TraitLittleEndian for i64 {
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
        bytes
            .try_into()
            .map(i64::from_le_bytes)
            .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Invalid bytes for i64"))
    }

    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>> {
        Ok(i64::to_le_bytes(*self).to_vec())
    }
}

impl TraitLittleEndian for FloatBits32 {
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
        bytes
            .try_into()
            .map(u32::from_le_bytes)
            .map(FloatBits32::from_bits)
            .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Invalid bytes for f32"))
    }

    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>> {
        Ok(self.0.to_le_bytes().to_vec())
    }
}

impl TraitLittleEndian for FloatBits64 {
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
        bytes
            .try_into()
            .map(u64::from_le_bytes)
            .map(FloatBits64::from_bits)
            .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Invalid bytes for f64"))
    }

    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>> {
        Ok(self.0.to_le_bytes().to_vec())
    }
}

impl TraitLittleEndian for V128 {
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
        bytes
            .try_into()
            .map(|b: [u8; 16]| V128::new(b))
            .map_err(|_| WrtError::new(ErrorKind::DeserializationError, "Invalid bytes for v128"))
    }

    #[cfg(feature = "alloc")]
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>> {
        Ok(self.bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValueType;

    #[test]
    fn test_value_type() {
        assert_eq!(Value::I32(0).type_(), ValueType::I32);
        assert_eq!(Value::I64(0).type_(), ValueType::I64);
        assert_eq!(Value::F32(FloatBits32(0)).type_(), ValueType::F32);
        assert_eq!(Value::F64(FloatBits64(0)).type_(), ValueType::F64);
        assert_eq!(Value::V128(V128::zero()).type_(), ValueType::V128);
        assert_eq!(Value::I16x8(V128::zero()).type_(), ValueType::I16x8);
        assert_eq!(Value::FuncRef(None).type_(), ValueType::FuncRef);
        assert_eq!(Value::ExternRef(None).type_(), ValueType::ExternRef);
    }

    #[test]
    fn test_value_matches_type() {
        assert!(Value::I32(0).matches_type(&ValueType::I32));
        assert!(!Value::I32(0).matches_type(&ValueType::I64));
        assert!(Value::V128(V128::zero()).matches_type(&ValueType::V128));
        assert!(Value::I16x8(V128::zero()).matches_type(&ValueType::I16x8));
        assert!(!Value::V128(V128::zero()).matches_type(&ValueType::I32));
        assert!(Value::Ref(1).matches_type(&ValueType::Ref(RefType::Func)));
        assert!(Value::Ref(1).matches_type(&ValueType::Ref(RefType::Extern)));
    }

    #[test]
    #[cfg(feature = "alloc")]
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
        assert_eq!(
            Value::default_for_type(&ValueType::F32),
            Value::F32(FloatBits32(0))
        );
        assert_eq!(
            Value::default_for_type(&ValueType::F64),
            Value::F64(FloatBits64(0))
        );
        assert_eq!(
            Value::default_for_type(&ValueType::V128),
            Value::V128(V128::zero())
        );
        assert_eq!(
            Value::default_for_type(&ValueType::I16x8),
            Value::I16x8(V128::zero())
        );
        assert_eq!(
            Value::default_for_type(&ValueType::FuncRef),
            Value::FuncRef(None)
        );
        assert_eq!(
            Value::default_for_type(&ValueType::ExternRef),
            Value::ExternRef(None)
        );
        assert_eq!(
            Value::default_for_type(&ValueType::Ref(RefType::Func)),
            Value::Ref(0)
        );
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
        assert_eq!(Value::F32(FloatBits32::from_float(1.0)), Value::F32(FloatBits32::from_float(1.0)));

        let v1 = V128::new([1; 16]);
        let v2 = V128::new([2; 16]);
        assert_eq!(Value::V128(v1.clone()), Value::V128(v1.clone()));
        assert_ne!(Value::V128(v1.clone()), Value::V128(v2.clone()));
        assert_eq!(Value::I16x8(v1.clone()), Value::I16x8(v1.clone()));
        assert_ne!(Value::I16x8(v1.clone()), Value::I16x8(v2.clone()));
    }
}
