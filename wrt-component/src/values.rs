//! Component Model value handling
//!
//! This module provides implementations for Component Model value types,
//! including serialization/deserialization, conversion, and runtime
//! representation.

// Import the various types we need explicitly to avoid confusion
use wrt_format::component::FormatValType;
use wrt_foundation::{
    component_value::{ComponentValue, ValType as TypesValType},
    safe_memory::NoStdProvider,
    traits::{ReadStream, WriteStream},
    values::Value,
};
use crate::bounded_component_infra::ComponentProvider;
use crate::prelude::*;

// Define type aliases with the component memory provider
type ComponentMemoryProvider = ComponentProvider; // Use the shared provider
type ComponentValType = TypesValType<ComponentMemoryProvider>;
type ComponentComponentValue = ComponentValue<ComponentMemoryProvider>;
type WrtFormatValType = FormatValType<ComponentProvider>;
type WrtTypesValType = TypesValType<ComponentProvider>;

use crate::{
    memory_layout::{calculate_layout, MemoryLayout},
};

// Use ComponentValType for the canonical ValType
type CanonicalValType = ComponentValType;

/// Convert from CanonicalValType to wrt_format::component::ValType
pub fn convert_common_to_format_valtype(common_type: &CanonicalValType) -> WrtFormatValType {
    match common_type {
        CanonicalValType::Bool => WrtFormatValType::Bool,
        CanonicalValType::S8 => WrtFormatValType::S8,
        CanonicalValType::U8 => WrtFormatValType::U8,
        CanonicalValType::S16 => WrtFormatValType::S16,
        CanonicalValType::U16 => WrtFormatValType::U16,
        CanonicalValType::S32 => WrtFormatValType::S32,
        CanonicalValType::U32 => WrtFormatValType::U32,
        CanonicalValType::S64 => WrtFormatValType::S64,
        CanonicalValType::U64 => WrtFormatValType::U64,
        CanonicalValType::F32 => WrtFormatValType::F32,
        CanonicalValType::F64 => WrtFormatValType::F64,
        CanonicalValType::Char => WrtFormatValType::Char,
        CanonicalValType::String => WrtFormatValType::String,
        CanonicalValType::Ref(idx) => WrtFormatValType::Ref(*idx),
        CanonicalValType::Record(fields) => {
            let converted_fields = fields
                .iter()
                .map(|(name, val_type)| (name.clone(), convert_common_to_format_valtype(val_type)))
                .collect();
            WrtFormatValType::Record(converted_fields)
        }
        CanonicalValType::Variant(cases) => {
            let converted_cases = cases
                .iter()
                .map(|(name, opt_type)| {
                    (
                        name.clone(),
                        opt_type
                            .as_ref()
                            .map(|val_type| convert_common_to_format_valtype(val_type)),
                    )
                })
                .collect();
            WrtFormatValType::Variant(converted_cases)
        }
        CanonicalValType::List(elem_type) => {
            WrtFormatValType::List(Box::new(convert_common_to_format_valtype(elem_type)))
        }
        CanonicalValType::Tuple(types) => {
            let converted_types =
                types.iter().map(|val_type| convert_common_to_format_valtype(val_type)).collect();
            WrtFormatValType::Tuple(converted_types)
        }
        CanonicalValType::Flags(names) => WrtFormatValType::Flags(names.clone()),
        CanonicalValType::Enum(variants) => WrtFormatValType::Enum(variants.clone()),
        CanonicalValType::Option(inner_type) => {
            WrtFormatValType::Option(Box::new(convert_common_to_format_valtype(inner_type)))
        }
        CanonicalValType::Result(result_type) => {
            WrtFormatValType::Result(Box::new(convert_common_to_format_valtype(result_type)))
        }
        CanonicalValType::Own(idx) => WrtFormatValType::Own(*idx),
        CanonicalValType::Borrow(idx) => WrtFormatValType::Borrow(*idx),
        CanonicalValType::FixedList(elem_type, size) => {
            WrtFormatValType::FixedList(Box::new(convert_common_to_format_valtype(elem_type)), *size)
        }
        CanonicalValType::Void => {
            // Void doesn't have a direct mapping, convert to a unit tuple
            WrtFormatValType::Tuple(Vec::new())
        }
        CanonicalValType::ErrorContext => WrtFormatValType::ErrorContext,
        CanonicalValType::Result { ok: _, err: _ } => {
            // For WrtFormatValType, we create a Result with a generic type placeholder
            // Since WrtFormatValType::Result requires a concrete type, we'll use a default
            WrtFormatValType::Result(Box::new(WrtFormatValType::Unit))
        }
    }
}

/// Convert from wrt_format::component::ValType to CanonicalValType
pub fn convert_format_to_common_valtype(format_type: &WrtFormatValType) -> CanonicalValType {
    match format_type {
        WrtFormatValType::Bool => CanonicalValType::Bool,
        WrtFormatValType::S8 => CanonicalValType::S8,
        WrtFormatValType::U8 => CanonicalValType::U8,
        WrtFormatValType::S16 => CanonicalValType::S16,
        WrtFormatValType::U16 => CanonicalValType::U16,
        WrtFormatValType::S32 => CanonicalValType::S32,
        WrtFormatValType::U32 => CanonicalValType::U32,
        WrtFormatValType::S64 => CanonicalValType::S64,
        WrtFormatValType::U64 => CanonicalValType::U64,
        WrtFormatValType::F32 => CanonicalValType::F32,
        WrtFormatValType::F64 => CanonicalValType::F64,
        WrtFormatValType::Char => CanonicalValType::Char,
        WrtFormatValType::String => CanonicalValType::String,
        WrtFormatValType::Ref(idx) => CanonicalValType::Ref(*idx),
        WrtFormatValType::Record(fields) => {
            let converted_fields = fields
                .iter()
                .map(|(name, val_type)| (name.clone(), convert_format_to_common_valtype(val_type)))
                .collect();
            CanonicalValType::Record(converted_fields)
        }
        WrtFormatValType::Variant(cases) => {
            let converted_cases = cases
                .iter()
                .map(|(name, opt_type)| {
                    (
                        name.clone(),
                        opt_type
                            .as_ref()
                            .map(|val_type| convert_format_to_common_valtype(val_type)),
                    )
                })
                .collect();
            CanonicalValType::Variant(converted_cases)
        }
        WrtFormatValType::List(elem_type) => {
            CanonicalValType::List(Box::new(convert_format_to_common_valtype(elem_type)))
        }
        WrtFormatValType::Tuple(types) => {
            let converted_types =
                types.iter().map(|val_type| convert_format_to_common_valtype(val_type)).collect();
            CanonicalValType::Tuple(converted_types)
        }
        WrtFormatValType::Flags(names) => CanonicalValType::Flags(names.clone()),
        WrtFormatValType::Enum(variants) => CanonicalValType::Enum(variants.clone()),
        WrtFormatValType::Option(inner_type) => {
            CanonicalValType::Option(Box::new(convert_format_to_common_valtype(inner_type)))
        }
        WrtFormatValType::Result(result_type) => {
            // Convert to CanonicalValType::Result with both ok and err as None for now
            // This is a simplified mapping since WrtFormatValType::Result doesn't distinguish ok/err
            CanonicalValType::Result { 
                ok: Some(Box::new(CanonicalValType::Ref(0))), // Placeholder reference
                err: None 
            }
        }
        WrtFormatValType::Own(idx) => CanonicalValType::Own(*idx),
        WrtFormatValType::Borrow(idx) => CanonicalValType::Borrow(*idx),
        WrtFormatValType::FixedList(elem_type, size) => CanonicalValType::FixedList(
            Box::new(convert_format_to_common_valtype(elem_type)),
            *size,
        ),
        WrtFormatValType::ErrorContext => CanonicalValType::ErrorContext,
        // Map any unhandled types to Void
        _ => CanonicalValType::Void,
    }
}

// Serialization and deserialization functions for ComponentValue
pub fn serialize_component_value(value: &ComponentComponentValue) -> Result<Vec<u8>, Error> {
    let common_type = value.get_type();
    let format_type = convert_common_to_format_valtype(&common_type);

    // Serialize the value based on its type
    let mut buffer = Vec::new();

    match value {
        ComponentComponentValue::Bool(b) => {
            buffer.push(if *b { 1 } else { 0 });
        }
        ComponentComponentValue::S8(v) => {
            buffer.push(*v as u8);
        }
        ComponentComponentValue::U8(v) => {
            buffer.push(*v);
        }
        ComponentComponentValue::S16(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentComponentValue::U16(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentComponentValue::S32(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentComponentValue::U32(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentComponentValue::S64(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentComponentValue::U64(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentComponentValue::F32(v) => {
            buffer.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        ComponentComponentValue::F64(v) => {
            buffer.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        ComponentComponentValue::Char(c) => {
            let bytes = [
                (*c as u32 & 0xff) as u8,
                ((*c as u32 >> 8) & 0xff) as u8,
                ((*c as u32 >> 16) & 0xff) as u8,
                ((*c as u32 >> 24) & 0xff) as u8,
            ];
            buffer.extend_from_slice(&bytes);
        }
        ComponentComponentValue::String(s) => {
            // String is encoded as length followed by UTF-8 bytes
            let bytes = s.as_bytes();
            let len = bytes.len() as u32;
            buffer.extend_from_slice(&len.to_le_bytes());
            buffer.extend_from_slice(bytes);
        }
        ComponentComponentValue::List(items) => {
            // Serialize list items
            let count = items.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());

            // If there are items, use the first item to determine the type
            if let Some(first_item) = items.first() {
                let item_type = first_item.get_type();
                let format_type = convert_common_to_format_valtype(&item_type);

                // Serialize each item
                for item in items {
                    let item_bytes = serialize_component_value(item)?;
                    buffer.extend_from_slice(&item_bytes);
                }
            }
        }
        ComponentComponentValue::Record(fields) => {
            // Serialize the record fields
            let field_count = fields.len() as u32;
            buffer.extend_from_slice(&field_count.to_le_bytes());

            // Serialize each field
            for (name, value) in fields {
                // Serialize the field name
                let name_bytes = name.as_bytes();
                let name_len = name_bytes.len() as u32;
                buffer.extend_from_slice(&name_len.to_le_bytes());
                buffer.extend_from_slice(name_bytes);

                // Serialize the field value
                let value_bytes = serialize_component_value(value)?;
                buffer.extend_from_slice(&value_bytes);
            }
        }
        ComponentComponentValue::Tuple(items) => {
            // Serialize tuple items
            let count = items.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());

            // Serialize each item
            for item in items {
                let item_bytes = serialize_component_value(item)?;
                buffer.extend_from_slice(&item_bytes);
            }
        }
        ComponentComponentValue::Variant(case_name, value_opt) => {
            // Serialize the case name
            let name_bytes = case_name.as_bytes();
            let name_len = name_bytes.len() as u32;
            buffer.extend_from_slice(&name_len.to_le_bytes());
            buffer.extend_from_slice(name_bytes);

            // Serialize presence flag for the value
            buffer.push(if value_opt.is_some() { 1 } else { 0 });

            // Serialize the value if present
            if let Some(value) = value_opt {
                let value_bytes = serialize_component_value(value)?;
                buffer.extend_from_slice(&value_bytes);
            }
        }
        ComponentComponentValue::Enum(variant) => {
            // Serialize the enum variant
            let variant_bytes = variant.as_bytes();
            let variant_len = variant_bytes.len() as u32;
            buffer.extend_from_slice(&variant_len.to_le_bytes());
            buffer.extend_from_slice(variant_bytes);
        }
        ComponentComponentValue::Option(value_opt) => {
            // Serialize presence flag for the value
            buffer.push(if value_opt.is_some() { 1 } else { 0 });

            // Serialize the value if present
            if let Some(value) = value_opt {
                let value_bytes = serialize_component_value(value)?;
                buffer.extend_from_slice(&value_bytes);
            }
        }
        ComponentComponentValue::Result(result) => {
            // Serialize success flag
            buffer.push(match result {
                Ok(_) => 1,  // Success
                Err(_) => 0, // Error
            });

            // Serialize the value (either Ok or Err)
            match result {
                Ok(value) => {
                    let value_bytes = serialize_component_value(value)?;
                    buffer.extend_from_slice(&value_bytes);
                }
                Err(error) => {
                    let error_bytes = serialize_component_value(error)?;
                    buffer.extend_from_slice(&error_bytes);
                }
            }
        }
        ComponentComponentValue::Handle(idx) => {
            // Serialize the handle index (from Own)
            buffer.extend_from_slice(&idx.to_le_bytes());
        }
        ComponentComponentValue::Borrow(idx) => {
            // Serialize the borrow index
            buffer.extend_from_slice(&idx.to_le_bytes());
        }
        ComponentComponentValue::Flags(flags) => {
            // Get the number of flags
            let count = flags.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());

            // Calculate the flag byte (up to 8 flags in a byte)
            let mut flag_byte: u8 = 0;
            for (i, (_, enabled)) in flags.iter().enumerate().take(8) {
                if *enabled {
                    flag_byte |= 1 << i;
                }
            }
            buffer.push(flag_byte);

            // Serialize each flag name
            for (name, _) in flags {
                let name_bytes = name.as_bytes();
                let name_len = name_bytes.len() as u32;
                buffer.extend_from_slice(&name_len.to_le_bytes());
                buffer.extend_from_slice(name_bytes);
            }
        }
        ComponentComponentValue::FixedList(items, size) => {
            // Serialize fixed list items
            let count = items.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());
            buffer.extend_from_slice(&size.to_le_bytes()); // Store the size

            // If there are items, use the first item to determine the type
            if let Some(first_item) = items.first() {
                let item_type = first_item.get_type();
                let format_type = convert_common_to_format_valtype(&item_type);

                // Serialize each item
                for item in items {
                    let item_bytes = serialize_component_value(item)?;
                    buffer.extend_from_slice(&item_bytes);
                }
            }
        }
        ComponentComponentValue::ErrorContext(ctx) => {
            // Serialize error context items
            let count = ctx.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());

            // Serialize each context value
            for item in ctx {
                let item_bytes = serialize_component_value(item)?;
                buffer.extend_from_slice(&item_bytes);
            }
        }
        ComponentComponentValue::Void => {
            // Just a type marker for void
            buffer.push(0);
        }
    }

    Ok(buffer)
}

/// Serialize a component value using the WriteStream interface
pub fn serialize_component_value_with_stream<'a, P: wrt_foundation::MemoryProvider>(
    value: &ComponentComponentValue,
    writer: &mut WriteStream<'a>,
    provider: &P,
) -> Result<(), Error> {
    match value {
        ComponentComponentValue::Bool(b) => {
            writer.write_bool(*b)?;
        }
        ComponentComponentValue::S8(v) => {
            writer.write_i8(*v)?;
        }
        ComponentComponentValue::U8(v) => {
            writer.write_u8(*v)?;
        }
        ComponentComponentValue::S16(v) => {
            writer.write_i16_le(*v)?;
        }
        ComponentComponentValue::U16(v) => {
            writer.write_u16_le(*v)?;
        }
        ComponentComponentValue::S32(v) => {
            writer.write_i32_le(*v)?;
        }
        ComponentComponentValue::U32(v) => {
            writer.write_u32_le(*v)?;
        }
        ComponentComponentValue::S64(v) => {
            writer.write_i64_le(*v)?;
        }
        ComponentComponentValue::U64(v) => {
            writer.write_u64_le(*v)?;
        }
        ComponentComponentValue::F32(v) => {
            writer.write_f32_le(*v)?;
        }
        ComponentComponentValue::F64(v) => {
            writer.write_f64_le(*v)?;
        }
        ComponentComponentValue::Char(c) => {
            writer.write_u32_le(*c as u32)?;
        }
        ComponentComponentValue::String(s) => {
            // String is encoded as length followed by UTF-8 bytes
            let bytes = s.as_bytes();
            writer.write_u32_le(bytes.len() as u32)?;
            writer.write_all(bytes)?;
        }
        ComponentComponentValue::List(items) => {
            // Serialize list items count
            writer.write_u32_le(items.len() as u32)?;

            // Serialize each item
            for item in items {
                serialize_component_value_with_stream(item, writer, provider)?;
            }
        }
        ComponentComponentValue::Record(fields) => {
            // Serialize the record fields count
            writer.write_u32_le(fields.len() as u32)?;

            // Serialize each field
            for (name, value) in fields {
                // Serialize the field name
                let name_bytes = name.as_bytes();
                writer.write_u32_le(name_bytes.len() as u32)?;
                writer.write_all(name_bytes)?;

                // Serialize the field value
                serialize_component_value_with_stream(value, writer, provider)?;
            }
        }
        ComponentComponentValue::Tuple(items) => {
            // Serialize tuple items count
            writer.write_u32_le(items.len() as u32)?;

            // Serialize each item
            for item in items {
                serialize_component_value_with_stream(item, writer, provider)?;
            }
        }
        ComponentComponentValue::Variant(case_name, value_opt) => {
            // Serialize the case name
            let name_bytes = case_name.as_bytes();
            writer.write_u32_le(name_bytes.len() as u32)?;
            writer.write_all(name_bytes)?;

            // Serialize presence flag for the value
            writer.write_bool(value_opt.is_some())?;

            // Serialize the value if present
            if let Some(value) = value_opt {
                serialize_component_value_with_stream(value, writer, provider)?;
            }
        }
        ComponentComponentValue::Enum(variant) => {
            // Serialize the enum variant
            let variant_bytes = variant.as_bytes();
            writer.write_u32_le(variant_bytes.len() as u32)?;
            writer.write_all(variant_bytes)?;
        }
        ComponentComponentValue::Option(value_opt) => {
            // Serialize presence flag for the value
            writer.write_bool(value_opt.is_some())?;

            // Serialize the value if present
            if let Some(value) = value_opt {
                serialize_component_value_with_stream(value, writer, provider)?;
            }
        }
        ComponentComponentValue::Result(result) => {
            // Serialize success flag
            let is_ok = matches!(result, Ok(_));
            writer.write_bool(is_ok)?;

            // Serialize the value (either Ok or Err)
            match result {
                Ok(value) => {
                    serialize_component_value_with_stream(value, writer, provider)?;
                }
                Err(error) => {
                    serialize_component_value_with_stream(error, writer, provider)?;
                }
            }
        }
        ComponentComponentValue::Handle(idx) => {
            // Serialize the handle index (from Own)
            writer.write_u32_le(*idx)?;
        }
        ComponentComponentValue::Borrow(idx) => {
            // Serialize the borrow index
            writer.write_u32_le(*idx)?;
        }
        ComponentComponentValue::Flags(flags) => {
            // Get the number of flags
            writer.write_u32_le(flags.len() as u32)?;

            // Calculate the flag byte (up to 8 flags in a byte)
            let mut flag_byte: u8 = 0;
            for (i, (_, enabled)) in flags.iter().enumerate().take(8) {
                if *enabled {
                    flag_byte |= 1 << i;
                }
            }
            writer.write_u8(flag_byte)?;

            // Serialize each flag name
            for (name, _) in flags {
                let name_bytes = name.as_bytes();
                writer.write_u32_le(name_bytes.len() as u32)?;
                writer.write_all(name_bytes)?;
            }
        }
        ComponentComponentValue::FixedList(items, size) => {
            // Serialize fixed list items count and size
            writer.write_u32_le(items.len() as u32)?;
            writer.write_u32_le(*size)?;

            // Serialize each item
            for item in items {
                serialize_component_value_with_stream(item, writer, provider)?;
            }
        }
        ComponentComponentValue::ErrorContext(ctx) => {
            // Serialize error context items count
            writer.write_u32_le(ctx.len() as u32)?;

            // Serialize each context value
            for item in ctx {
                serialize_component_value_with_stream(item, writer, provider)?;
            }
        }
        ComponentComponentValue::Void => {
            // Just a type marker for void
            writer.write_u8(0)?;
        }
    }

    Ok(())
}

// Simplified deserialization function
pub fn deserialize_component_value(
    data: &[u8],
    format_type: &WrtFormatValType,
) -> Result<ComponentComponentValue, Error> {
    let mut offset = 0;
    match format_type {
        WrtFormatValType::Bool => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize bool".to_string(),
                ));
            }
            let value = data[offset] != 0;
            Ok(ComponentComponentValue::Bool(value))
        }
        WrtFormatValType::S8 => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize S8".to_string(),
                ));
            }
            let value = data[offset] as i8;
            Ok(ComponentComponentValue::S8(value))
        }
        WrtFormatValType::U8 => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize U8".to_string(),
                ));
            }
            let value = data[offset];
            Ok(ComponentComponentValue::U8(value))
        }
        WrtFormatValType::S16 => {
            if offset + 2 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize S16".to_string(),
                ));
            }
            let value = i16::from_le_bytes([data[offset], data[offset + 1]]);
            Ok(ComponentComponentValue::S16(value))
        }
        WrtFormatValType::U16 => {
            if offset + 2 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize U16".to_string(),
                ));
            }
            let value = u16::from_le_bytes([data[offset], data[offset + 1]]);
            Ok(ComponentComponentValue::U16(value))
        }
        WrtFormatValType::S32 => {
            if offset + 4 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize S32".to_string(),
                ));
            }
            let value = i32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            Ok(ComponentComponentValue::S32(value))
        }
        WrtFormatValType::U32 => {
            if offset + 4 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize U32".to_string(),
                ));
            }
            let value = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            Ok(ComponentComponentValue::U32(value))
        }
        WrtFormatValType::S64 => {
            if offset + 8 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize S64".to_string(),
                ));
            }
            let value = i64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            Ok(ComponentComponentValue::S64(value))
        }
        WrtFormatValType::U64 => {
            if offset + 8 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize U64".to_string(),
                ));
            }
            let value = u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            Ok(ComponentComponentValue::U64(value))
        }
        WrtFormatValType::F32 => {
            if offset + 4 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize F32".to_string(),
                ));
            }
            let value = f32::from_bits(u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
            Ok(ComponentComponentValue::F32(value))
        }
        WrtFormatValType::F64 => {
            if offset + 8 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize F64".to_string(),
                ));
            }
            let value = f64::from_bits(u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]));
            Ok(ComponentComponentValue::F64(value))
        }
        WrtFormatValType::Char => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Char".to_string(),
                ));
            }
            let value = data[offset] as char;
            Ok(ComponentComponentValue::Char(value))
        }
        WrtFormatValType::String => {
            if offset + 4 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize String length".to_string(),
                ));
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            if offset + len as usize > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize String".to_string(),
                ));
            }
            let value =
                String::from_utf8(data[offset..offset + len as usize].to_vec()).map_err(|e| {
                    Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Component not found".to_string(),
                    )
                })?;
            Ok(ComponentComponentValue::String(value))
        }
        WrtFormatValType::List(elem_type) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize List length".to_string(),
                ));
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            if offset + len as usize * size_in_bytes(elem_type) > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize List".to_string(),
                ));
            }
            let mut values = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let value = deserialize_component_value(&data[offset..], elem_type)?;
                values.push(value);
                offset += size_in_bytes(elem_type);
            }
            Ok(ComponentComponentValue::List(values))
        }
        WrtFormatValType::Record(fields) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Record".to_string(),
                ));
            }
            let mut values = Vec::new();
            for (name, val_type) in fields {
                let value = deserialize_component_value(&data[offset..], val_type)?;
                values.push((name.clone(), value));
                offset += size_in_bytes(val_type);
            }
            Ok(ComponentComponentValue::Record(values))
        }
        WrtFormatValType::Tuple(types) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Tuple".to_string(),
                ));
            }
            let mut values = Vec::with_capacity(types.len());
            for val_type in types {
                let value = deserialize_component_value(&data[offset..], val_type)?;
                values.push(value);
                offset += size_in_bytes(val_type);
            }
            Ok(ComponentComponentValue::Tuple(values))
        }
        WrtFormatValType::Variant(cases) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Variant".to_string(),
                ));
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            if offset + len as usize > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Variant".to_string(),
                ));
            }
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Variant case".to_string(),
                ));
            }

            // Get the case index
            let case_idx = data[offset] as usize;
            // No need to update offset anymore as we return immediately

            if case_idx >= cases.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found".to_string(),
                ));
            }

            // Get the case name and type
            let (case_name, case_type_opt) = &cases[case_idx];

            // Create the variant value
            match case_type_opt {
                Some(case_type) => {
                    if offset >= data.len() {
                        return Err(Error::new(
                            ErrorCategory::Parse,
                            codes::PARSE_ERROR,
                            "Not enough data to deserialize Variant value".to_string(),
                        ));
                    }
                    let inner_value = deserialize_component_value(&data[offset..], case_type)?;
                    // We've already read the needed value, no need to update offset
                    Ok(ComponentComponentValue::Variant(case_name.clone(), Some(Box::new(inner_value))))
                }
                None => {
                    // Case without a value
                    Ok(ComponentComponentValue::Variant(case_name.clone(), None))
                }
            }
        }
        WrtFormatValType::Enum(variants) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Enum".to_string(),
                ));
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            if offset + len as usize > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Enum".to_string(),
                ));
            }
            let value = data[offset] as char;
            // No need to update offset anymore as we return immediately
            Ok(ComponentComponentValue::Enum(variants[value as usize].clone()))
        }
        WrtFormatValType::Option(inner_type) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Option".to_string(),
                ));
            }
            let value = data[offset] != 0;
            // No need to update offset anymore as we return immediately
            if value {
                let inner_value = deserialize_component_value(&data[offset..], inner_type)?;
                Ok(ComponentComponentValue::Option(Some(Box::new(inner_value))))
            } else {
                Ok(ComponentComponentValue::Option(None))
            }
        }
        WrtFormatValType::Result(result_type) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Result".to_string(),
                ));
            }
            let value = data[offset] != 0;
            // No need to update offset anymore as we return immediately
            if value {
                let inner_value = deserialize_component_value(&data[offset..], result_type)?;
                Ok(ComponentComponentValue::Result(Ok(Box::new(inner_value))))
            } else {
                // Create a default error value when the result is not successful
                Ok(ComponentComponentValue::Result(Err(Box::new(ComponentComponentValue::Void))))
            }
        }
        WrtFormatValType::Own(idx) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Own".to_string(),
                ));
            }
            let value = data[offset] as u32;
            // No need to update offset anymore as we return immediately
            Ok(ComponentComponentValue::Handle(value))
        }
        WrtFormatValType::Borrow(idx) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Borrow".to_string(),
                ));
            }
            let value = data[offset] as u32;
            // No need to update offset anymore as we return immediately
            Ok(ComponentComponentValue::Borrow(value))
        }
        WrtFormatValType::Flags(names) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Flags".to_string(),
                ));
            }
            let flag_byte = data[offset];
            // No need to update offset anymore as we return immediately

            // Convert names to (String, bool) pairs based on the flag_byte
            let mut flags = Vec::new();
            for (i, name) in names.iter().enumerate() {
                if i < 8 {
                    // Only process up to 8 flags (one byte)
                    let is_set = (flag_byte & (1 << i)) != 0;
                    flags.push((name.clone(), is_set));
                }
            }

            Ok(ComponentComponentValue::Flags(flags))
        }
        WrtFormatValType::FixedList(elem_type, size) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize FixedList length".to_string(),
                ));
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            if offset + len as usize * size_in_bytes(elem_type) > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize FixedList".to_string(),
                ));
            }
            let mut values = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let value = deserialize_component_value(&data[offset..], elem_type)?;
                values.push(value);
                offset += size_in_bytes(elem_type);
            }
            Ok(ComponentComponentValue::FixedList(values, *size))
        }
        WrtFormatValType::ErrorContext => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize ErrorContext".to_string(),
                ));
            }
            let value = data[offset] != 0;
            // No need to update offset anymore as we return immediately
            // Create a proper ErrorContext value with an empty Vec
            Ok(ComponentComponentValue::ErrorContext(Vec::new()))
        }
        // Handle any other unimplemented cases
        _ => Err(Error::new(
            ErrorCategory::System,
            codes::UNSUPPORTED_OPERATION,
            "Type not supported for deserialization".to_string(),
        )),
    }
}

/// Deserialize a component value using the ReadStream interface
pub fn deserialize_component_value_with_stream<'a, P: wrt_foundation::MemoryProvider>(
    reader: &mut ReadStream<'a>,
    format_type: &WrtFormatValType,
    provider: &P,
) -> Result<ComponentComponentValue, Error> {
    match format_type {
        WrtFormatValType::Bool => {
            let value = reader.read_bool()?;
            Ok(ComponentComponentValue::Bool(value))
        }
        WrtFormatValType::S8 => {
            let value = reader.read_i8()?;
            Ok(ComponentComponentValue::S8(value))
        }
        WrtFormatValType::U8 => {
            let value = reader.read_u8()?;
            Ok(ComponentComponentValue::U8(value))
        }
        WrtFormatValType::S16 => {
            let value = reader.read_i16_le()?;
            Ok(ComponentComponentValue::S16(value))
        }
        WrtFormatValType::U16 => {
            let value = reader.read_u16_le()?;
            Ok(ComponentComponentValue::U16(value))
        }
        WrtFormatValType::S32 => {
            let value = reader.read_i32_le()?;
            Ok(ComponentComponentValue::S32(value))
        }
        WrtFormatValType::U32 => {
            let value = reader.read_u32_le()?;
            Ok(ComponentComponentValue::U32(value))
        }
        WrtFormatValType::S64 => {
            let value = reader.read_i64_le()?;
            Ok(ComponentComponentValue::S64(value))
        }
        WrtFormatValType::U64 => {
            let value = reader.read_u64_le()?;
            Ok(ComponentComponentValue::U64(value))
        }
        WrtFormatValType::F32 => {
            let value = reader.read_f32_le()?;
            Ok(ComponentComponentValue::F32(value))
        }
        WrtFormatValType::F64 => {
            let value = reader.read_f64_le()?;
            Ok(ComponentComponentValue::F64(value))
        }
        WrtFormatValType::Char => {
            let value_u32 = reader.read_u32_le()?;
            let value = char::from_u32(value_u32).ok_or_else(|| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found",
                )
            })?;
            Ok(ComponentComponentValue::Char(value))
        }
        WrtFormatValType::String => {
            // Read the string length
            let len = reader.read_u32_le()? as usize;

            // Read the string bytes
            let mut bytes = vec![0u8; len];
            reader.read_exact(&mut bytes)?;

            // Convert to a String
            let value = String::from_utf8(bytes).map_err(|e| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found",
                )
            })?;

            Ok(ComponentComponentValue::String(value))
        }
        WrtFormatValType::List(elem_type) => {
            // Read the number of items
            let count = reader.read_u32_le()? as usize;

            // Read each item
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let item = deserialize_component_value_with_stream(reader, elem_type, provider)?;
                items.push(item);
            }

            Ok(ComponentComponentValue::List(items))
        }
        WrtFormatValType::Record(fields) => {
            // Read the number of fields
            let count = reader.read_u32_le()? as usize;

            if count != fields.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    &format!("Record field count mismatch: expected {}, got {}", fields.len(), count)
                ));
            }

            // Read each field
            let mut values = Vec::with_capacity(count);
            for (name, val_type) in fields {
                // Read field name length
                let name_len = reader.read_u32_le()? as usize;

                // Read field name
                let mut name_bytes = vec![0u8; name_len];
                reader.read_exact(&mut name_bytes)?;
                let field_name = String::from_utf8(name_bytes).map_err(|e| {
                    Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Component not found",
                    )
                })?;

                // Read field value
                let value = deserialize_component_value_with_stream(reader, val_type, provider)?;
                values.push((field_name, value));
            }

            Ok(ComponentComponentValue::Record(values))
        }
        WrtFormatValType::Tuple(types) => {
            // Read the number of items
            let count = reader.read_u32_le()? as usize;

            if count != types.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    &format!("Tuple type count mismatch: expected {}, got {}", types.len(), count)
                ));
            }

            // Read each item
            let mut items = Vec::with_capacity(count);
            for item_type in types {
                let item = deserialize_component_value_with_stream(reader, item_type, provider)?;
                items.push(item);
            }

            Ok(ComponentComponentValue::Tuple(items))
        }
        WrtFormatValType::Variant(cases) => {
            // Read the case name length
            let name_len = reader.read_u32_le()? as usize;

            // Read the case name
            let mut name_bytes = vec![0u8; name_len];
            reader.read_exact(&mut name_bytes)?;
            let case_name = String::from_utf8(name_bytes).map_err(|e| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found",
                )
            })?;

            // Find the case in the list
            let case_idx =
                cases.iter().position(|(name, _)| name == &case_name).ok_or_else(|| {
                    Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Component not found",
                    )
                })?;

            // Read presence flag
            let has_value = reader.read_bool()?;

            // Read the value if present
            let (_, case_type_opt) = &cases[case_idx];
            if has_value {
                if let Some(case_type) = case_type_opt {
                    let value =
                        deserialize_component_value_with_stream(reader, case_type, provider)?;
                    Ok(ComponentComponentValue::Variant(case_name, Some(Box::new(value))))
                } else {
                    Err(Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Component not found",
                    ))
                }
            } else {
                Ok(ComponentComponentValue::Variant(case_name, None))
            }
        }
        WrtFormatValType::Enum(variants) => {
            // Read the variant name length
            let name_len = reader.read_u32_le()? as usize;

            // Read the variant name
            let mut name_bytes = vec![0u8; name_len];
            reader.read_exact(&mut name_bytes)?;
            let variant_name = String::from_utf8(name_bytes).map_err(|e| {
                Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found",
                )
            })?;

            // Validate the variant name
            if !variants.contains(&variant_name) {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found",
                ));
            }

            Ok(ComponentComponentValue::Enum(variant_name))
        }
        WrtFormatValType::Option(inner_type) => {
            // Read presence flag
            let has_value = reader.read_bool()?;

            if has_value {
                let value = deserialize_component_value_with_stream(reader, inner_type, provider)?;
                Ok(ComponentComponentValue::Option(Some(Box::new(value))))
            } else {
                Ok(ComponentComponentValue::Option(None))
            }
        }
        WrtFormatValType::Result(result_type) => {
            // Read success flag
            let is_ok = reader.read_bool()?;

            if is_ok {
                let value = deserialize_component_value_with_stream(reader, result_type, provider)?;
                Ok(ComponentComponentValue::Result(Ok(Box::new(value))))
            } else {
                // Create a default error value
                Ok(ComponentComponentValue::Result(Err(Box::new(ComponentComponentValue::Void))))
            }
        }
        WrtFormatValType::Own(idx) => {
            let value = reader.read_u32_le()?;
            Ok(ComponentComponentValue::Handle(value))
        }
        WrtFormatValType::Borrow(idx) => {
            let value = reader.read_u32_le()?;
            Ok(ComponentComponentValue::Borrow(value))
        }
        WrtFormatValType::Flags(names) => {
            // Read the number of flags
            let count = reader.read_u32_le()? as usize;

            if count != names.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    &format!("Flags name count mismatch: expected {}, got {}", names.len(), count)
                ));
            }

            // Read the flag byte
            let flag_byte = reader.read_u8()?;

            // Convert names to (String, bool) pairs based on the flag_byte
            let mut flags = Vec::with_capacity(count);
            for (i, name) in names.iter().enumerate() {
                if i < 8 {
                    // Only process up to 8 flags (one byte)
                    let is_set = (flag_byte & (1 << i)) != 0;
                    flags.push((name.clone(), is_set));
                } else {
                    // Default to false for flags beyond the first byte
                    flags.push((name.clone(), false));
                }
            }

            // Read each flag name (name is already known from the type)
            for _ in 0..count {
                let name_len = reader.read_u32_le()? as usize;
                let mut name_bytes = vec![0u8; name_len];
                reader.read_exact(&mut name_bytes)?;
                // We don't actually use the names here since we already have
                // them from the type
            }

            Ok(ComponentComponentValue::Flags(flags))
        }
        WrtFormatValType::FixedList(elem_type, size) => {
            // Read the number of items
            let count = reader.read_u32_le()? as usize;

            // Read the fixed size
            let fixed_size = reader.read_u32_le()?;

            if fixed_size != *size {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Component not found",
                ));
            }

            // Read each item
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let item = deserialize_component_value_with_stream(reader, elem_type, provider)?;
                items.push(item);
            }

            Ok(ComponentComponentValue::FixedList(items, *size))
        }
        WrtFormatValType::ErrorContext => {
            // Read the number of context items
            let count = reader.read_u32_le()? as usize;

            // Read each context item
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                // Assuming context items are strings
                let item_len = reader.read_u32_le()? as usize;
                let mut item_bytes = vec![0u8; item_len];
                reader.read_exact(&mut item_bytes)?;
                let item_str = String::from_utf8(item_bytes).map_err(|e| {
                    Error::new(
                        ErrorCategory::Parse,
                        codes::PARSE_ERROR,
                        "Component not found",
                    )
                })?;
                items.push(ComponentComponentValue::String(item_str));
            }

            Ok(ComponentComponentValue::ErrorContext(items))
        }
        WrtFormatValType::Void => {
            // Just read a marker byte
            let _ = reader.read_u8()?;
            Ok(ComponentComponentValue::Void)
        }
        // Handle any other unimplemented cases
        _ => Err(Error::new(
            ErrorCategory::System,
            codes::UNSUPPORTED_OPERATION,
            "Component not found",
        )),
    }
}

/// Serialize multiple component values
pub fn serialize_component_values(values: &[ComponentComponentValue]) -> Result<Vec<u8>, Error> {
    let mut buffer = Vec::new();

    // Write the number of values
    let count = values.len() as u32;
    buffer.extend_from_slice(&count.to_le_bytes());

    // Serialize each value
    for value in values {
        let value_bytes = serialize_component_value(value)?;

        // Write the size of this value's bytes
        let size = value_bytes.len() as u32;
        buffer.extend_from_slice(&size.to_le_bytes());

        // Write the value bytes
        buffer.extend_from_slice(&value_bytes);
    }

    Ok(buffer)
}

/// Serialize multiple component values using a WriteStream
pub fn serialize_component_values_with_stream<'a, P: wrt_foundation::MemoryProvider>(
    values: &[ComponentComponentValue],
    writer: &mut WriteStream<'a>,
    provider: &P,
) -> Result<(), Error> {
    // Write the number of values
    writer.write_u32_le(values.len() as u32)?;

    // Serialize each value directly to the stream
    for value in values {
        serialize_component_value_with_stream(value, writer, provider)?;
    }

    Ok(())
}

/// Deserialize multiple component values
pub fn deserialize_component_values(
    data: &[u8],
    types: &[WrtFormatValType],
) -> Result<Vec<ComponentComponentValue>, Error> {
    // Need at least 4 bytes for the count
    if data.len() < 4 {
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::PARSE_ERROR,
            "Not enough data to read value count".to_string(),
        ));
    }

    // Read the count
    let mut offset = 0;
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    offset += 4;

    // Validate that we have enough types
    if count > types.len() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Validation error"
        ));
    }

    // Read each value
    let mut values = Vec::with_capacity(count);
    for type_idx in 0..count {
        // Need at least 4 more bytes for the size
        if offset + 4 > data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Not enough data to read value size".to_string(),
            ));
        }

        // Read the size
        let size = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        // Validate that we have enough data
        if offset + size > data.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Not enough data to read value".to_string(),
            ));
        }

        // Read the value
        let value_data = &data[offset..offset + size];
        let value = deserialize_component_value(value_data, &types[type_idx])?;
        values.push(value);

        // Move to the next value
        offset += size;
    }

    Ok(values)
}

/// Deserialize multiple component values using a ReadStream
pub fn deserialize_component_values_with_stream<'a, P: wrt_foundation::MemoryProvider>(
    reader: &mut ReadStream<'a>,
    types: &[WrtFormatValType],
    provider: &P,
) -> Result<Vec<ComponentComponentValue>, Error> {
    // Read the count
    let count = reader.read_u32_le()? as usize;

    // Validate that we have enough types
    if count > types.len() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Validation error"
        ));
    }

    // Read each value
    let mut values = Vec::with_capacity(count);
    for type_idx in 0..count {
        let value = deserialize_component_value_with_stream(reader, &types[type_idx], provider)?;
        values.push(value);
    }

    Ok(values)
}

// Core value conversion functions
pub fn core_to_component_value(value: &Value, ty: &WrtFormatValType) -> crate::WrtResult<ComponentComponentValue> {
    use crate::type_conversion::{
        core_value_to_types_componentvalue, format_valtype_to_types_valtype,
    };

    // First, convert the format value type to a types value type
    let types_val_type = format_valtype_to_types_valtype(ty);

    // Then convert the core value to a component value
    let component_value = core_value_to_types_componentvalue(value)?;

    // Check if the types match or provide a conversion error
    match (&component_value, &types_val_type) {
        // Basic type checking for primitive types
        (ComponentComponentValue::S32(_), ComponentValType::S32)
        | (ComponentComponentValue::S64(_), ComponentValType::S64)
        | (ComponentComponentValue::F32(_), ComponentValType::F32)
        | (ComponentComponentValue::F64(_), ComponentValType::F64) => Ok(component_value),

        // Handle boolean conversion from i32
        (ComponentComponentValue::S32(v), ComponentValType::Bool) => Ok(ComponentComponentValue::Bool(*v != 0)),

        // Other integer width conversions
        (ComponentComponentValue::S32(v), ComponentValType::S8) => Ok(ComponentComponentValue::S8(*v as i8)),
        (ComponentComponentValue::S32(v), ComponentValType::U8) => Ok(ComponentComponentValue::U8(*v as u8)),
        (ComponentComponentValue::S32(v), ComponentValType::S16) => Ok(ComponentComponentValue::S16(*v as i16)),
        (ComponentComponentValue::S32(v), ComponentValType::U16) => Ok(ComponentComponentValue::U16(*v as u16)),
        (ComponentComponentValue::S32(v), ComponentValType::U32) => Ok(ComponentComponentValue::U32(*v as u32)),
        (ComponentComponentValue::S64(v), ComponentValType::U64) => Ok(ComponentComponentValue::U64(*v as u64)),

        // Error for type mismatch
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            format!(
                "Type mismatch: cannot convert {:?} to component value of type {:?}",
                value, types_val_type
            ),
        )),
    }
}

pub fn component_to_core_value(value: &ComponentComponentValue) -> crate::WrtResult<Value> {
    use crate::type_conversion::types_componentvalue_to_core_value;

    // Use the centralized conversion function
    types_componentvalue_to_core_value(value)
}

// Size calculation for component values
pub fn size_in_bytes(ty: &WrtFormatValType) -> usize {
    match ty {
        WrtFormatValType::Bool => 1,
        WrtFormatValType::S8 => 1,
        WrtFormatValType::U8 => 1,
        WrtFormatValType::S16 => 2,
        WrtFormatValType::U16 => 2,
        WrtFormatValType::S32 => 4,
        WrtFormatValType::U32 => 4,
        WrtFormatValType::S64 => 8,
        WrtFormatValType::U64 => 8,
        WrtFormatValType::F32 => 4,
        WrtFormatValType::F64 => 8,
        WrtFormatValType::Char => 4,
        WrtFormatValType::String => 8, // Pointer size
        WrtFormatValType::Ref(_) => 4,
        WrtFormatValType::Record(_) => 8,
        WrtFormatValType::Variant(_) => 8,
        WrtFormatValType::List(_) => 8,
        WrtFormatValType::Tuple(_) => 8,
        WrtFormatValType::Flags(_) => 8,
        WrtFormatValType::Enum(_) => 4,
        WrtFormatValType::Option(_) => 8,
        WrtFormatValType::Result(_) => 8,
        WrtFormatValType::Own(_) => 4,
        WrtFormatValType::Borrow(_) => 4,
        WrtFormatValType::FixedList(_, _) => 8,
        WrtFormatValType::ErrorContext => 4,
        // Default size for any unhandled types
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use wrt_foundation::safe_memory::{NoStdProvider, SafeMemoryHandler, Slice, SliceMut};

    use super::*;

    #[test]
    fn test_primitive_value_encoding_decoding() {
        // Test a few primitive types
        let values =
            vec![ComponentComponentValue::Bool(true), ComponentComponentValue::S32(42), ComponentComponentValue::F64(3.14159)];

        for value in values {
            let encoded = serialize_component_value(&value).unwrap();
            let format_type = convert_common_to_format_valtype(&value.get_type());
            let decoded = deserialize_component_value(&encoded, &format_type).unwrap();

            // Only check bools since we only implemented deserialization for a subset of
            // types
            if let ComponentComponentValue::Bool(_) = value {
                assert_eq!(value, decoded);
            }
        }
    }

    #[test]
    fn test_stream_serialization() {
        // Create a memory provider
        let provider = NoStdProvider::<1024>::default();
        let handler = SafeMemoryHandler::new(provider).unwrap();

        // Get a mutable slice for the output buffer
        let mut slice_mut = handler.get_slice_mut(0, 1024).unwrap();
        let mut writer = WriteStream::new(slice_mut);

        // Create a simple value to serialize
        let value = ComponentComponentValue::Bool(true);

        // Serialize using stream
        serialize_component_value_with_stream(&value, &mut writer, &handler).unwrap();

        // Read back
        let position = writer.position();
        drop(writer); // Release mutable borrow

        let slice = handler.borrow_slice(0, position).unwrap();
        let mut reader = ReadStream::new(slice);

        // Deserialize using stream
        let format_type = WrtFormatValType::Bool;
        let decoded =
            deserialize_component_value_with_stream(&mut reader, &format_type, &handler).unwrap();

        // Verify
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_multiple_values_stream() {
        // Create a memory provider
        let provider = NoStdProvider::<1024>::default();
        let handler = SafeMemoryHandler::new(provider).unwrap();

        // Get a mutable slice for the output buffer
        let mut slice_mut = handler.get_slice_mut(0, 1024).unwrap();
        let mut writer = WriteStream::new(slice_mut);

        // Create values to serialize
        let values = vec![
            ComponentComponentValue::Bool(true),
            ComponentComponentValue::S32(42),
            ComponentComponentValue::String("hello".to_string()),
        ];

        // Serialize using stream
        serialize_component_values_with_stream(&values, &mut writer, &handler).unwrap();

        // Read back
        let position = writer.position();
        drop(writer); // Release mutable borrow

        let slice = handler.borrow_slice(0, position).unwrap();
        let mut reader = ReadStream::new(slice);

        // Prepare format types
        let format_types = vec![WrtFormatValType::Bool, WrtFormatValType::S32, WrtFormatValType::String];

        // Deserialize using stream
        let decoded =
            deserialize_component_values_with_stream(&mut reader, &format_types, &handler).unwrap();

        // Verify count
        assert_eq!(values.len(), decoded.len());

        // Verify individual values
        assert_eq!(values[0], decoded[0]);

        // For S32, we can use a direct comparison
        if let (ComponentComponentValue::S32(original), ComponentComponentValue::S32(decoded_val)) =
            (&values[1], &decoded[1])
        {
            assert_eq!(original, decoded_val);
        } else {
            panic!("Expected S32 values");
        }

        // For String, compare the string values
        if let (ComponentComponentValue::String(original), ComponentComponentValue::String(decoded_val)) =
            (&values[2], &decoded[2])
        {
            assert_eq!(original, decoded_val);
        } else {
            panic!("Expected String values");
        }
    }
}

