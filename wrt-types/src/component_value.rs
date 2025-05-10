// WRT - wrt-types
// Module: WebAssembly Component Model Value Types
// SW-REQ-ID: REQ_WASM_COMPONENT_002 (Example: Relates to component model values)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly Component Model value types
//!
//! This module defines the runtime value types used in WebAssembly Component Model
//! implementations.

#![allow(clippy::derive_partial_eq_without_eq)]

use core::fmt;
use wrt_error::Result;
use wrt_error::{Error, ErrorCategory};

// #[cfg(feature = "std")]
// use std::{boxed::Box, format, string::String, vec, vec::Vec}; // Removed

// #[cfg(all(feature = "alloc", not(feature = "std")))]
// use alloc::{boxed::Box, format, string::String, vec, vec::Vec}; // Removed

use crate::prelude::ToString;
use crate::prelude::*;

use crate::Value;
use crate::{FloatBits32, FloatBits64};
use crate::types::ValueType;
use crate::types::ComponentValType;
use wrt_error::{codes, Error, ErrorCategory};

/// A Component Model value type
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ValType {
    /// Boolean value
    #[default]
    Bool,
    /// Signed 8-bit integer
    S8,
    /// Unsigned 8-bit integer
    U8,
    /// Signed 16-bit integer
    S16,
    /// Unsigned 16-bit integer
    U16,
    /// Signed 32-bit integer
    S32,
    /// Unsigned 32-bit integer
    U32,
    /// Signed 64-bit integer
    S64,
    /// Unsigned 64-bit integer
    U64,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// Unicode character
    Char,
    /// UTF-8 string
    String,
    /// Reference to another entity
    Ref(u32),
    /// Record with named fields
    Record(Vec<(String, ValType)>),
    /// Variant with cases
    Variant(Vec<(String, Option<ValType>)>),
    /// List of elements
    List(Box<ValType>),
    /// Fixed-length list of elements with a known length
    FixedList(Box<ValType>, u32),
    /// Tuple of elements
    Tuple(Vec<ValType>),
    /// Flags (set of named boolean flags)
    Flags(Vec<String>),
    /// Enumeration of variants
    Enum(Vec<String>),
    /// Option type
    Option(Box<ValType>),
    /// Result type
    Result(Box<ValType>),
    /// Result type with only Err
    ResultErr(Box<ValType>),
    /// Result type with both Ok and Err
    ResultBoth(Box<ValType>, Box<ValType>),
    /// Resource handle (owned)
    Own(u32),
    /// Resource handle (borrowed)
    Borrow(u32),
    /// Void type
    Void,
    /// Error context type
    ErrorContext,
}

/// WebAssembly component value types
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentValue {
    /// Invalid/uninitialized value
    Void,
    /// Boolean value (true/false)
    Bool(bool),
    /// Signed 8-bit integer
    S8(i8),
    /// Unsigned 8-bit integer
    U8(u8),
    /// Signed 16-bit integer
    S16(i16),
    /// Unsigned 16-bit integer
    U16(u16),
    /// Signed 32-bit integer
    S32(i32),
    /// Unsigned 32-bit integer
    U32(u32),
    /// Signed 64-bit integer
    S64(i64),
    /// Unsigned 64-bit integer
    U64(u64),
    /// 32-bit floating point
    F32(f32),
    /// 64-bit floating point
    F64(f64),
    /// Unicode character
    Char(char),
    /// UTF-8 string
    String(String),
    /// List of component values
    List(Vec<ComponentValue>),
    /// Fixed-length list of component values with a known length
    FixedList(Vec<ComponentValue>, u32),
    /// Record with named fields
    Record(Vec<(String, ComponentValue)>),
    /// Variant with case name and optional value
    Variant(String, Option<Box<ComponentValue>>),
    /// Tuple of component values
    Tuple(Vec<ComponentValue>),
    /// Flags with boolean fields
    Flags(Vec<(String, bool)>),
    /// Enumeration with case name
    Enum(String),
    /// Optional value (Some/None)
    Option(Option<Box<ComponentValue>>),
    /// Result value (Ok/Err)
    Result(core::result::Result<Box<ComponentValue>, Box<ComponentValue>>),
    /// Handle to a resource (u32 representation)
    Own(u32),
    /// Reference to a borrowed resource (u32 representation)
    Borrow(u32),
    /// Error context information
    ErrorContext(Vec<ComponentValue>),
}

// Implement Eq for ComponentValue
// Note: This means we can't use floating point equality comparisons directly
impl Eq for ComponentValue {}

impl ComponentValue {
    /// Create a new void value
    pub fn void() -> Self {
        Self::Void
    }

    /// Create a new boolean value
    pub fn bool(v: bool) -> Self {
        Self::Bool(v)
    }

    /// Create a new signed 8-bit integer value
    pub fn s8(v: i8) -> Self {
        Self::S8(v)
    }

    /// Create a new unsigned 8-bit integer value
    pub fn u8(v: u8) -> Self {
        Self::U8(v)
    }

    /// Create a new signed 16-bit integer value
    pub fn s16(v: i16) -> Self {
        Self::S16(v)
    }

    /// Create a new unsigned 16-bit integer value
    pub fn u16(v: u16) -> Self {
        Self::U16(v)
    }

    /// Create a new signed 32-bit integer value
    pub fn s32(v: i32) -> Self {
        Self::S32(v)
    }

    /// Create a new unsigned 32-bit integer value
    pub fn u32(v: u32) -> Self {
        Self::U32(v)
    }

    /// Create a new signed 64-bit integer value
    pub fn s64(v: i64) -> Self {
        Self::S64(v)
    }

    /// Create a new unsigned 64-bit integer value
    pub fn u64(v: u64) -> Self {
        Self::U64(v)
    }

    /// Create a new 32-bit float value
    pub fn f32(v: f32) -> Self {
        Self::F32(v)
    }

    /// Create a new 64-bit float value
    pub fn f64(v: f64) -> Self {
        Self::F64(v)
    }

    /// Create a new character value
    pub fn char(v: char) -> Self {
        Self::Char(v)
    }

    /// Create a new string value
    pub fn string<S: Into<String>>(v: S) -> Self {
        Self::String(v.into())
    }

    /// Create a new list value
    pub fn list(v: Vec<ComponentValue>) -> Self {
        Self::List(v)
    }

    /// Create a new fixed-length list value
    pub fn fixed_list(v: Vec<ComponentValue>, len: u32) -> Result<Self> {
        if v.len() != len as usize {
            return Err(Error::new(
                ErrorCategory::Type,
                3001, // TYPE_MISMATCH
                format!("Fixed list length mismatch: expected {}, got {}", len, v.len()),
            ));
        }
        Ok(Self::FixedList(v, len))
    }

    /// Create a new record value
    pub fn record(v: Vec<(String, ComponentValue)>) -> Self {
        Self::Record(v)
    }

    /// Create a new variant value
    pub fn variant<S: Into<String>>(case: S, value: Option<ComponentValue>) -> Self {
        Self::Variant(case.into(), value.map(Box::new))
    }

    /// Create a new tuple value
    pub fn tuple(v: Vec<ComponentValue>) -> Self {
        Self::Tuple(v)
    }

    /// Create a new flags value
    pub fn flags(v: Vec<(String, bool)>) -> Self {
        Self::Flags(v)
    }

    /// Create a new enum value
    pub fn enum_value<S: Into<String>>(case: S) -> Self {
        Self::Enum(case.into())
    }

    /// Create a new option value (some)
    pub fn some(v: ComponentValue) -> Self {
        Self::Option(Some(Box::new(v)))
    }

    /// Create a new option value (none)
    pub fn none() -> Self {
        Self::Option(None)
    }

    /// Create a new result value (ok)
    pub fn ok(v: ComponentValue) -> Self {
        Self::Result(Ok(Box::new(v)))
    }

    /// Create a new result value (err)
    pub fn err(v: ComponentValue) -> Self {
        Self::Result(Err(Box::new(v)))
    }

    /// Create a new handle value
    pub fn handle(v: u32) -> Self {
        Self::Own(v)
    }

    /// Create a new borrow value
    pub fn borrow(v: u32) -> Self {
        Self::Borrow(v)
    }

    /// Create a new error context value
    pub fn error_context(v: Vec<ComponentValue>) -> Self {
        Self::ErrorContext(v)
    }

    /// Check if this value is of the void type
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    /// Get the type of this component value
    pub fn get_type(&self) -> ValType {
        match self {
            Self::Void => ValType::Void,
            Self::Bool(_) => ValType::Bool,
            Self::S8(_) => ValType::S8,
            Self::U8(_) => ValType::U8,
            Self::S16(_) => ValType::S16,
            Self::U16(_) => ValType::U16,
            Self::S32(_) => ValType::S32,
            Self::U32(_) => ValType::U32,
            Self::S64(_) => ValType::S64,
            Self::U64(_) => ValType::U64,
            Self::F32(_) => ValType::F32,
            Self::F64(_) => ValType::F64,
            Self::Char(_) => ValType::Char,
            Self::String(_) => ValType::String,
            Self::List(items) => {
                if let Some(first) = items.first() {
                    ValType::List(Box::new(first.get_type()))
                } else {
                    // Empty list, use a placeholder type
                    ValType::List(Box::new(ValType::Bool))
                }
            }
            Self::FixedList(items, _len) => {
                if let Some(first) = items.first() {
                    ValType::FixedList(Box::new(first.get_type()), *_len)
                } else {
                    // Empty list, use a placeholder type
                    ValType::FixedList(Box::new(ValType::Bool), *_len)
                }
            }
            Self::Record(fields) => {
                let mut field_types = Vec::new();
                for (name, value) in fields {
                    field_types.push((name.clone(), value.get_type()));
                }
                ValType::Record(field_types)
            }
            Self::Variant(case, value) => {
                let cases = vec![(case.clone(), value.as_ref().map(|v| v.get_type()))];
                ValType::Variant(cases)
            }
            Self::Tuple(items) => {
                let item_types = items.iter().map(|v| v.get_type()).collect();
                ValType::Tuple(item_types)
            }
            Self::Flags(flags) => {
                let names = flags.iter().map(|(name, _)| name.clone()).collect();
                ValType::Flags(names)
            }
            Self::Enum(case) => {
                let variants = vec![case.clone()];
                ValType::Enum(variants)
            }
            Self::Option(opt) => {
                if let Some(val) = opt {
                    ValType::Option(Box::new(val.get_type()))
                } else {
                    ValType::Option(Box::new(ValType::Bool)) // Placeholder
                }
            }
            Self::Result(res) => match res {
                Ok(v) => ValType::Result(Box::new(v.get_type())),
                Err(e) => ValType::ResultErr(Box::new(e.get_type())),
            },
            Self::Own(idx) => ValType::Own(*idx),
            Self::Borrow(idx) => ValType::Borrow(*idx),
            Self::ErrorContext(_ctx) => ValType::ErrorContext,
        }
    }

    /// Check if this value matches the specified type
    pub fn matches_type(&self, value_type: &ValType) -> bool {
        match (self, value_type) {
            // Handle Void type
            (ComponentValue::Void, ValType::Void) => true,
            (ComponentValue::Void, _) => false,

            // Handle ErrorContext type
            (ComponentValue::ErrorContext(_), ValType::ErrorContext) => true,

            // Simple primitive type checks
            (ComponentValue::Bool(_), ValType::Bool) => true,
            (ComponentValue::S8(_), ValType::S8) => true,
            (ComponentValue::U8(_), ValType::U8) => true,
            (ComponentValue::S16(_), ValType::S16) => true,
            (ComponentValue::U16(_), ValType::U16) => true,
            (ComponentValue::S32(_), ValType::S32) => true,
            (ComponentValue::U32(_), ValType::U32) => true,
            (ComponentValue::S64(_), ValType::S64) => true,
            (ComponentValue::U64(_), ValType::U64) => true,
            (ComponentValue::F32(_), ValType::F32) => true,
            (ComponentValue::F64(_), ValType::F64) => true,
            (ComponentValue::Char(_), ValType::Char) => true,
            (ComponentValue::String(_), ValType::String) => true,

            // Complex type checks
            (ComponentValue::List(items), ValType::List(list_type)) => {
                items.iter().all(|item| item.matches_type(list_type))
            }

            // Fixed-length list type check
            (
                ComponentValue::FixedList(items, list_len),
                ValType::FixedList(list_type, expected_len),
            ) => {
                *list_len == *expected_len && items.iter().all(|item| item.matches_type(list_type))
            }

            (ComponentValue::Record(fields), ValType::Record(record_types)) => {
                // Check if all fields in the record type are present in the value
                // and that their types match
                if fields.len() != record_types.len() {
                    return false;
                }

                for (field_name, field_type) in record_types {
                    // Find the field by name in the vector
                    let field_value = fields.iter().find(|(name, _)| name == field_name);
                    if let Some((_, value)) = field_value {
                        if !value.matches_type(field_type) {
                            return false;
                        }
                    } else {
                        return false; // Missing field
                    }
                }

                true
            }

            (ComponentValue::Variant(case, value), ValType::Variant(cases)) => {
                // Check if the case index is valid
                if !cases.iter().any(|(c, _)| c == case) {
                    return false;
                }

                // Get the case type from the index
                let (_, case_type) = cases.iter().find(|(c, _)| c == case).unwrap();

                // Check if the value matches the case type
                match (value, case_type) {
                    (Some(value), Some(ty)) => value.matches_type(ty),
                    (None, None) => true,
                    _ => false,
                }
            }

            (ComponentValue::Tuple(items), ValType::Tuple(item_types)) => {
                // Check if the tuple length matches
                if items.len() != item_types.len() {
                    return false;
                }

                // Check if each item matches its corresponding type
                items
                    .iter()
                    .zip(item_types.iter())
                    .all(|(item, item_type)| item.matches_type(item_type))
            }

            (ComponentValue::Flags(flags), ValType::Flags(flag_names)) => {
                // Check if all flag names in the type are present in the value
                if flags.len() != flag_names.len() {
                    return false;
                }

                // Check that all flag names in the type are present in the value
                flag_names.iter().all(|name| flags.iter().any(|(fname, _)| fname == name))
            }

            (ComponentValue::Enum(value), ValType::Enum(variants)) => {
                // Check if the enum index is valid
                variants.contains(value)
            }

            (ComponentValue::Option(value), ValType::Option(option_type)) => {
                match value {
                    Some(v) => v.matches_type(option_type),
                    None => true, // None matches any option type
                }
            }

            (ComponentValue::Result(res), ValType::Result(result_type)) => match res {
                Ok(v) => v.matches_type(result_type),
                Err(e) => e.matches_type(result_type),
            },

            (ComponentValue::Result(res), ValType::ResultBoth(ok_type, err_type)) => match res {
                Ok(v) => v.matches_type(ok_type),
                Err(e) => e.matches_type(err_type),
            },

            (ComponentValue::Result(res), ValType::ResultErr(err_type)) => match res {
                Ok(_) => false,
                Err(e) => e.matches_type(err_type),
            },

            (ComponentValue::Own(handle), ValType::Own(id)) => handle == id,
            (ComponentValue::Borrow(handle), ValType::Borrow(id)) => handle == id,

            // All other combinations don't match
            _ => false,
        }
    }

    /// Convert a WebAssembly core value to a component value
    pub fn from_core_value(
        cv_type: &ComponentValType,
        value: &Value,
        // Assuming a store or similar context might be needed for FuncRef/ExternRef resolution
        _store: &ComponentValueStore, // Placeholder if needed for refs
    ) -> Result<ComponentValue> {
        match (cv_type, value) {
            (ComponentValType::Bool, Value::I32(v)) => Ok(ComponentValue::Bool(v != 0)),
            (ComponentValType::S8, Value::I32(v)) => Ok(ComponentValue::S8(v as i8)),
            (ComponentValType::U8, Value::I32(v)) => Ok(ComponentValue::U8(v as u8)),
            (ComponentValType::S16, Value::I32(v)) => Ok(ComponentValue::S16(v as i16)),
            (ComponentValType::U16, Value::I32(v)) => Ok(ComponentValue::U16(v as u16)),
            (ComponentValType::S32, Value::I32(v)) => Ok(ComponentValue::S32(v)),
            (ComponentValType::U32, Value::I32(v)) => Ok(ComponentValue::U32(v as u32)),
            (ComponentValType::S64, Value::I64(v)) => Ok(ComponentValue::S64(v)),
            (ComponentValType::U64, Value::I64(v)) => Ok(ComponentValue::U64(v as u64)),
            (ComponentValType::F32, Value::F32(v)) => Ok(ComponentValue::F32(v.value())),
            (ComponentValType::F64, Value::F64(v)) => Ok(ComponentValue::F64(v.value())),
            (ComponentValType::Char, Value::I32(v)) => {
                char::from_u32(v as u32)
                    .map(ComponentValue::Char)
                    .ok_or_else(|| Error::type_error("invalid char value"))
            }
            (ComponentValType::String, Value::Ref(handle)) => {
                // Assuming String is stored as a sequence of chars or similar
                // This needs a way to get string data from Value::Ref and store
                if let Some(core_val_ref) = store.get_ref(*handle) {
                    if let Value::String(s) = core_val_ref {
                        Ok(ComponentValue::String(s.clone()))
                    } else {
                         Err(Error::type_error(
                            format!("expected string for Value::Ref for ComponentValType::String, handle={:?}, found_type={:?}", handle, core_val_ref.value_type())
                        ))
                    }
                } else {
                    Err(Error::type_error(
                        format!("invalid Value::Ref handle for ComponentValType::String, handle={:?}", handle)
                    ))
                }
            }
            // TODO: Handle List, Record, Variant, Tuple, Flags, Enum, Option, Result, Handle (Own, Borrow)
            // These will likely involve looking up data from the store using Value::Ref(idx)
            _ => Err(Error::type_error(format!("Mismatched types or unimplemented conversion from core Value to ComponentValue. Target: {:?}, Source: {:?}", cv_type, value.value_type()))),
        }
    }

    /// Convert this component value to a WebAssembly core value
    pub fn to_core_value(cv: &ComponentValue, store: &mut ComponentValueStore) -> Result<Value> {
        match cv {
            ComponentValue::Void => Err(Error::type_error("Cannot convert Void to Core Value")),
            ComponentValue::Bool(v) => Ok(Value::I32(i32::from(*v))),
            ComponentValue::S8(v) => Ok(Value::I32(i32::from(*v))),
            ComponentValue::U8(v) => Ok(Value::I32(i32::from(*v))),
            ComponentValue::S16(v) => Ok(Value::I32(i32::from(*v))),
            ComponentValue::U16(v) => Ok(Value::I32(i32::from(*v))),
            ComponentValue::S32(v) => Ok(Value::I32(*v)),
            ComponentValue::U32(v) => Ok(Value::I32(*v as i32)), // Potentially lossy for Wasm I32
            ComponentValue::S64(v) => Ok(Value::I64(*v)),
            ComponentValue::U64(v) => Ok(Value::I64(*v as i64)), // Potentially lossy
            ComponentValue::F32(v) => Ok(Value::F32(FloatBits32::from_float(*v))),
            ComponentValue::F64(v) => Ok(Value::F64(FloatBits64::from_float(*v))),
            ComponentValue::Char(v) => Ok(Value::I32(*v as i32)),
            ComponentValue::String(s) => Ok(Value::Ref(store.add_string(s)?)), // Changed self to store
            ComponentValue::List(values) => Ok(Value::Ref(store.add_list(values)?)), // Changed self to store
            ComponentValue::Record(fields) => Ok(Value::Ref(store.add_record(fields)?)), // Changed self to store
            ComponentValue::Variant(case_idx, value) => Ok(Value::Ref(store.add_variant(*case_idx, value.as_deref())?)), // Changed, added deref
            ComponentValue::Tuple(values) => {
                let core_values: Result<Vec<Value>> = values.iter().map(|cv| Self::to_core_value(cv, store)).collect();
                Ok(Value::Ref(store.add_tuple(core_values?)?))
            }
            ComponentValue::Flags(flags) => Ok(Value::Ref(store.add_flags(flags.clone())?)), // Assuming flags are simple enough or add_flags handles it
            ComponentValue::Enum(case) => Ok(Value::Ref(store.add_enum(case.clone())?)), // Assuming add_enum takes String
            ComponentValue::Option(opt_val) => {
                match opt_val {
                    Some(cv_box) => {
                        let core_v = Self::to_core_value(cv_box.as_ref(), store)?;
                        Ok(Value::Ref(store.add_option(Some(core_v))?))
                    }
                    None => Ok(Value::Ref(store.add_option(None)?)),
                }
            }
            ComponentValue::Result(res_val) => { // res_val is &core::result::Result<Box<ComponentValue>, Box<ComponentValue>>
                match res_val.as_ref() { // .as_ref() gives Result<&Box<ComponentValue>, &Box<ComponentValue>>
                    Ok(ok_cv_box) => {
                        let core_ok_v = Self::to_core_value(ok_cv_box.as_ref(), store)?;
                        Ok(Value::Ref(store.add_result(Some(core_ok_v), None)?))
                    }
                    Err(err_cv_box) => {
                        let core_err_v = Self::to_core_value(err_cv_box.as_ref(), store)?;
                        Ok(Value::Ref(store.add_result(None, Some(core_err_v))?))
                    }
                }
            }
            ComponentValue::Own(handle) => Ok(Value::Ref(*handle)), // Assuming Own handle maps directly to a Ref handle in core
            ComponentValue::Borrow(handle) => Ok(Value::Ref(*handle)), // Same for Borrow
            ComponentValue::ErrorContext(_ctx) => {
                // TODO: How to represent ErrorContext in core Value? Maybe a specific Ref type or skip?
                Err(Error::unimplemented("ComponentValue::ErrorContext to core Value conversion"))
            }
        }
    }
}

// Format implementation
impl fmt::Display for ComponentValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentValue::Void => write!(f, "void"),
            ComponentValue::Bool(b) => write!(f, "{}", b),
            ComponentValue::S8(n) => write!(f, "{}i8", n),
            ComponentValue::U8(n) => write!(f, "{}u8", n),
            ComponentValue::S16(n) => write!(f, "{}i16", n),
            ComponentValue::U16(n) => write!(f, "{}u16", n),
            ComponentValue::S32(n) => write!(f, "{}i32", n),
            ComponentValue::U32(n) => write!(f, "{}u32", n),
            ComponentValue::S64(n) => write!(f, "{}i64", n),
            ComponentValue::U64(n) => write!(f, "{}u64", n),
            ComponentValue::F32(n) => write!(f, "{}f32", n),
            ComponentValue::F64(n) => write!(f, "{}f64", n),
            ComponentValue::Char(c) => write!(f, "'{}'", c),
            ComponentValue::String(s) => write!(f, "\"{}\"", s),
            ComponentValue::List(v) => {
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            ComponentValue::FixedList(v, len) => {
                write!(f, "[{}: ", len)?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            ComponentValue::Record(fields) => {
                write!(f, "{{")?;
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, val)?;
                }
                write!(f, "}}")
            }
            ComponentValue::Variant(case, val) => {
                write!(f, "{}(", case)?;
                if let Some(v) = val {
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            ComponentValue::Tuple(v) => {
                write!(f, "(")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }
            ComponentValue::Flags(flags) => {
                write!(f, "{{")?;
                let mut first = true;
                for (name, enabled) in flags {
                    if *enabled {
                        if !first {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", name)?;
                        first = false;
                    }
                }
                write!(f, "}}")
            }
            ComponentValue::Enum(case) => write!(f, "{}", case),
            ComponentValue::Option(opt) => match opt {
                Some(v) => write!(f, "some({})", v),
                None => write!(f, "none"),
            },
            ComponentValue::Result(res) => match res {
                Ok(v) => write!(f, "ok({})", v),
                Err(e) => write!(f, "err({})", e),
            },
            ComponentValue::Own(h) => write!(f, "handle({})", h),
            ComponentValue::Borrow(b) => write!(f, "borrow({})", b),
            ComponentValue::ErrorContext(ctx) => {
                write!(f, "error_context(")?;
                for (i, val) in ctx.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Basic serialization/deserialization functions for component values
/// These are simple implementations to handle the basic needs for the
/// intercept crate. Full serialization is in the component crate.
///
/// Simple serialization of component values
pub fn serialize_component_values(values: &[ComponentValue]) -> Result<Vec<u8>> {
    let mut result = Vec::new();

    // Write the number of values
    result.extend_from_slice(&(values.len() as u32).to_le_bytes());

    // Write each value (very basic implementation)
    for value in values {
        match value {
            ComponentValue::Bool(b) => {
                result.push(0); // Type tag for bool
                result.push(if *b { 1 } else { 0 });
            }
            ComponentValue::U32(v) => {
                result.push(1); // Type tag for u32
                result.extend_from_slice(&v.to_le_bytes());
            }
            ComponentValue::S32(v) => {
                result.push(2); // Type tag for s32
                result.extend_from_slice(&v.to_le_bytes());
            }
            // Add more types as needed for intercept functionality
            _ => {
                return Err(Error::new(
                    ErrorCategory::Component,
                    3002, // ENCODING_ERROR
                    format!("Serialization not implemented for this type: {:?}", value),
                ));
            }
        }
    }

    Ok(result)
}

/// Simple deserialization of component values
pub fn deserialize_component_values(data: &[u8], types: &[ValType]) -> Result<Vec<ComponentValue>> {
    let mut result = Vec::new();
    let mut offset = 0;

    // Read the number of values
    if data.len() < 4 {
        return Err(Error::new(
            ErrorCategory::Component,
            3002, // ENCODING_ERROR
            "Data too short to contain value count",
        ));
    }

    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    offset += 4;

    // Sanity check
    if count != types.len() {
        return Err(Error::new(
            ErrorCategory::Type,
            3001, // TYPE_MISMATCH
            format!(
                "Value count mismatch: data has {} values but types list has {}",
                count,
                types.len()
            ),
        ));
    }

    // Read each value
    for _ in 0..count {
        if offset >= data.len() {
            return Err(Error::new(
                ErrorCategory::Component,
                3002, // ENCODING_ERROR
                "Unexpected end of data",
            ));
        }

        let type_tag = data[offset];
        offset += 1;

        match type_tag {
            0 => {
                // Bool
                if offset >= data.len() {
                    return Err(Error::new(
                        ErrorCategory::Component,
                        3002, // ENCODING_ERROR
                        "Unexpected end of data",
                    ));
                }
                let value = data[offset] != 0;
                offset += 1;
                result.push(ComponentValue::Bool(value));
            }
            1 => {
                // U32
                if offset + 4 > data.len() {
                    return Err(Error::new(
                        ErrorCategory::Component,
                        3002, // ENCODING_ERROR
                        "Unexpected end of data",
                    ));
                }
                let value = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                offset += 4;
                result.push(ComponentValue::U32(value));
            }
            2 => {
                // S32
                if offset + 4 > data.len() {
                    return Err(Error::new(
                        ErrorCategory::Component,
                        3002, // ENCODING_ERROR
                        "Unexpected end of data",
                    ));
                }
                let value = i32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                offset += 4;
                result.push(ComponentValue::S32(value));
            }
            // Add more types as needed for intercept functionality
            _ => {
                return Err(Error::new(
                    ErrorCategory::Component,
                    3002, // ENCODING_ERROR
                    format!("Deserialization not implemented for type tag: {}", type_tag),
                ));
            }
        }
    }

    Ok(result)
}

/// Create a type conversion error with a descriptive message.
///
/// This is a helper function used for type conversion errors within the component model.
pub fn conversion_error(message: &str) -> Error {
    Error::invalid_type(format!("Type conversion error: {}", message))
}

/// Create a component encoding error with a descriptive message.
///
/// This is a helper function used for encoding errors within the component model.
pub fn encoding_error(message: &str) -> Error {
    Error::component_error(format!("Component encoding error: {}", message))
}

/// Create a component decoding error with a descriptive message.
///
/// This is a helper function used for decoding errors within the component model.
pub fn decoding_error(message: &str) -> Error {
    Error::new(ErrorCategory::Parse, wrt_error::codes::DECODING_ERROR, message.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn test_primitive_value_type_matching() {
        let bool_value = ComponentValue::Bool(true);
        let int_value = ComponentValue::S32(42);
        let float_value = ComponentValue::F32(PI);

        assert!(bool_value.matches_type(&ValType::Bool));
        assert!(!bool_value.matches_type(&ValType::S32));

        assert!(int_value.matches_type(&ValType::S32));
        assert!(!int_value.matches_type(&ValType::Bool));

        assert!(float_value.matches_type(&ValType::F32));
        assert!(!float_value.matches_type(&ValType::F64));
    }

    #[test]
    fn test_conversion_between_core_and_component() {
        let i32_val = Value::I32(42);
        let f64_val = Value::F64(FloatBits64::new(123.456));

        let i32_comp_val = ComponentValue::from_core_value(&ComponentValType::S32, &i32_val, &ComponentValueStore::new()).unwrap();
        let f64_comp_val = ComponentValue::from_core_value(&ComponentValType::F64, &f64_val, &ComponentValueStore::new()).unwrap();

        assert_eq!(i32_comp_val, ComponentValue::S32(42));
        assert_eq!(f64_comp_val, ComponentValue::F64(123.456));

        let i32_core_val = ComponentValue::to_core_value(&i32_comp_val, &mut ComponentValueStore::new()).unwrap();
        let f64_core_val = ComponentValue::to_core_value(&f64_comp_val, &mut ComponentValueStore::new()).unwrap();

        assert_eq!(i32_core_val, i32_val);
        assert_eq!(f64_core_val, f64_val);
    }

    #[test]
    fn test_serialization_deserialization() {
        let values =
            vec![ComponentValue::Bool(true), ComponentValue::U32(42), ComponentValue::S32(-7)];

        let types = vec![ValType::Bool, ValType::U32, ValType::S32];

        let serialized = serialize_component_values(&values).unwrap();
        let deserialized = deserialize_component_values(&serialized, &types).unwrap();

        assert_eq!(deserialized.len(), values.len());

        if let ComponentValue::Bool(v) = &deserialized[0] {
            assert!(*v);
        } else {
            panic!("Expected Bool value");
        }

        if let ComponentValue::U32(v) = &deserialized[1] {
            assert_eq!(*v, 42);
        } else {
            panic!("Expected U32 value");
        }

        if let ComponentValue::S32(v) = &deserialized[2] {
            assert_eq!(*v, -7);
        } else {
            panic!("Expected S32 value");
        }
    }
}
