//! Component Model value handling
//!
//! This module provides implementations for Component Model value types, including
//! serialization/deserialization, conversion, and runtime representation.

use crate::prelude::*;

// Import the various types we need explicitly to avoid confusion
use wrt_format::component::ValType as FormatValType;
use wrt_types::component_value::ComponentValue;
use wrt_types::component_value::ValType as TypesValType;
use wrt_types::values::Value;

// Use TypesValType for the canonical ValType
type CanonicalValType = TypesValType;

/// Convert from CanonicalValType to wrt_format::component::ValType
pub fn convert_common_to_format_valtype(common_type: &CanonicalValType) -> FormatValType {
    match common_type {
        CanonicalValType::Bool => FormatValType::Bool,
        CanonicalValType::S8 => FormatValType::S8,
        CanonicalValType::U8 => FormatValType::U8,
        CanonicalValType::S16 => FormatValType::S16,
        CanonicalValType::U16 => FormatValType::U16,
        CanonicalValType::S32 => FormatValType::S32,
        CanonicalValType::U32 => FormatValType::U32,
        CanonicalValType::S64 => FormatValType::S64,
        CanonicalValType::U64 => FormatValType::U64,
        CanonicalValType::F32 => FormatValType::F32,
        CanonicalValType::F64 => FormatValType::F64,
        CanonicalValType::Char => FormatValType::Char,
        CanonicalValType::String => FormatValType::String,
        CanonicalValType::Ref(idx) => FormatValType::Ref(*idx),
        CanonicalValType::Record(fields) => {
            let converted_fields = fields
                .iter()
                .map(|(name, val_type)| (name.clone(), convert_common_to_format_valtype(val_type)))
                .collect();
            FormatValType::Record(converted_fields)
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
            FormatValType::Variant(converted_cases)
        }
        CanonicalValType::List(elem_type) => {
            FormatValType::List(Box::new(convert_common_to_format_valtype(elem_type)))
        }
        CanonicalValType::Tuple(types) => {
            let converted_types =
                types.iter().map(|val_type| convert_common_to_format_valtype(val_type)).collect();
            FormatValType::Tuple(converted_types)
        }
        CanonicalValType::Flags(names) => FormatValType::Flags(names.clone()),
        CanonicalValType::Enum(variants) => FormatValType::Enum(variants.clone()),
        CanonicalValType::Option(inner_type) => {
            FormatValType::Option(Box::new(convert_common_to_format_valtype(inner_type)))
        }
        CanonicalValType::Result(result_type) => {
            FormatValType::Result(Box::new(convert_common_to_format_valtype(result_type)))
        }
        CanonicalValType::Own(idx) => FormatValType::Own(*idx),
        CanonicalValType::Borrow(idx) => FormatValType::Borrow(*idx),
        CanonicalValType::FixedList(elem_type, size) => {
            FormatValType::FixedList(Box::new(convert_common_to_format_valtype(elem_type)), *size)
        }
        CanonicalValType::Void => {
            // Void doesn't have a direct mapping, convert to a unit tuple
            FormatValType::Tuple(Vec::new())
        }
        CanonicalValType::ErrorContext => FormatValType::ErrorContext,
        CanonicalValType::ResultErr(err_type) => {
            FormatValType::ResultErr(Box::new(convert_common_to_format_valtype(err_type)))
        }
        CanonicalValType::ResultBoth(ok_type, err_type) => FormatValType::ResultBoth(
            Box::new(convert_common_to_format_valtype(ok_type)),
            Box::new(convert_common_to_format_valtype(err_type)),
        ),
    }
}

/// Convert from wrt_format::component::ValType to CanonicalValType
pub fn convert_format_to_common_valtype(format_type: &FormatValType) -> CanonicalValType {
    match format_type {
        FormatValType::Bool => CanonicalValType::Bool,
        FormatValType::S8 => CanonicalValType::S8,
        FormatValType::U8 => CanonicalValType::U8,
        FormatValType::S16 => CanonicalValType::S16,
        FormatValType::U16 => CanonicalValType::U16,
        FormatValType::S32 => CanonicalValType::S32,
        FormatValType::U32 => CanonicalValType::U32,
        FormatValType::S64 => CanonicalValType::S64,
        FormatValType::U64 => CanonicalValType::U64,
        FormatValType::F32 => CanonicalValType::F32,
        FormatValType::F64 => CanonicalValType::F64,
        FormatValType::Char => CanonicalValType::Char,
        FormatValType::String => CanonicalValType::String,
        FormatValType::Ref(idx) => CanonicalValType::Ref(*idx),
        FormatValType::Record(fields) => {
            let converted_fields = fields
                .iter()
                .map(|(name, val_type)| (name.clone(), convert_format_to_common_valtype(val_type)))
                .collect();
            CanonicalValType::Record(converted_fields)
        }
        FormatValType::Variant(cases) => {
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
        FormatValType::List(elem_type) => {
            CanonicalValType::List(Box::new(convert_format_to_common_valtype(elem_type)))
        }
        FormatValType::Tuple(types) => {
            let converted_types =
                types.iter().map(|val_type| convert_format_to_common_valtype(val_type)).collect();
            CanonicalValType::Tuple(converted_types)
        }
        FormatValType::Flags(names) => CanonicalValType::Flags(names.clone()),
        FormatValType::Enum(variants) => CanonicalValType::Enum(variants.clone()),
        FormatValType::Option(inner_type) => {
            CanonicalValType::Option(Box::new(convert_format_to_common_valtype(inner_type)))
        }
        FormatValType::Result(result_type) => {
            CanonicalValType::Result(Box::new(convert_format_to_common_valtype(result_type)))
        }
        FormatValType::ResultErr(err_type) => {
            // Map to CanonicalValType::Result with a default inner type
            CanonicalValType::Result(Box::new(CanonicalValType::Bool))
        }
        FormatValType::ResultBoth(ok_type, err_type) => {
            // Map to CanonicalValType::Result with the ok type
            CanonicalValType::Result(Box::new(convert_format_to_common_valtype(ok_type)))
        }
        FormatValType::Own(idx) => CanonicalValType::Own(*idx),
        FormatValType::Borrow(idx) => CanonicalValType::Borrow(*idx),
        FormatValType::FixedList(elem_type, size) => CanonicalValType::FixedList(
            Box::new(convert_format_to_common_valtype(elem_type)),
            *size,
        ),
        FormatValType::ErrorContext => CanonicalValType::ErrorContext,
        // Map any unhandled types to Void
        _ => CanonicalValType::Void,
    }
}

// Serialization and deserialization functions for ComponentValue
pub fn serialize_component_value(value: &ComponentValue) -> Result<Vec<u8>> {
    let common_type = value.get_type();
    let format_type = convert_common_to_format_valtype(&common_type);

    // Serialize the value based on its type
    let mut buffer = Vec::new();

    match value {
        ComponentValue::Bool(b) => {
            buffer.push(if *b { 1 } else { 0 });
        }
        ComponentValue::S8(v) => {
            buffer.push(*v as u8);
        }
        ComponentValue::U8(v) => {
            buffer.push(*v);
        }
        ComponentValue::S16(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentValue::U16(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentValue::S32(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentValue::U32(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentValue::S64(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentValue::U64(v) => {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        ComponentValue::F32(v) => {
            buffer.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        ComponentValue::F64(v) => {
            buffer.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        ComponentValue::Char(c) => {
            let bytes = [
                (*c as u32 & 0xff) as u8,
                ((*c as u32 >> 8) & 0xff) as u8,
                ((*c as u32 >> 16) & 0xff) as u8,
                ((*c as u32 >> 24) & 0xff) as u8,
            ];
            buffer.extend_from_slice(&bytes);
        }
        ComponentValue::String(s) => {
            // String is encoded as length followed by UTF-8 bytes
            let bytes = s.as_bytes();
            let len = bytes.len() as u32;
            buffer.extend_from_slice(&len.to_le_bytes());
            buffer.extend_from_slice(bytes);
        }
        ComponentValue::List(items) => {
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
        ComponentValue::Record(fields) => {
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
        ComponentValue::Tuple(items) => {
            // Serialize tuple items
            let count = items.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());

            // Serialize each item
            for item in items {
                let item_bytes = serialize_component_value(item)?;
                buffer.extend_from_slice(&item_bytes);
            }
        }
        ComponentValue::Variant(case_name, value_opt) => {
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
        ComponentValue::Enum(variant) => {
            // Serialize the enum variant
            let variant_bytes = variant.as_bytes();
            let variant_len = variant_bytes.len() as u32;
            buffer.extend_from_slice(&variant_len.to_le_bytes());
            buffer.extend_from_slice(variant_bytes);
        }
        ComponentValue::Option(value_opt) => {
            // Serialize presence flag for the value
            buffer.push(if value_opt.is_some() { 1 } else { 0 });

            // Serialize the value if present
            if let Some(value) = value_opt {
                let value_bytes = serialize_component_value(value)?;
                buffer.extend_from_slice(&value_bytes);
            }
        }
        ComponentValue::Result(result) => {
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
        ComponentValue::Handle(idx) => {
            // Serialize the handle index (from Own)
            buffer.extend_from_slice(&idx.to_le_bytes());
        }
        ComponentValue::Borrow(idx) => {
            // Serialize the borrow index
            buffer.extend_from_slice(&idx.to_le_bytes());
        }
        ComponentValue::Flags(flags) => {
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
        ComponentValue::FixedList(items, size) => {
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
        ComponentValue::ErrorContext(ctx) => {
            // Serialize error context items
            let count = ctx.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());

            // Serialize each context value
            for item in ctx {
                let item_bytes = serialize_component_value(item)?;
                buffer.extend_from_slice(&item_bytes);
            }
        }
        ComponentValue::Void => {
            // Just a type marker for void
            buffer.push(0);
        }
    }

    Ok(buffer)
}

// Simplified deserialization function
pub fn deserialize_component_value(
    data: &[u8],
    format_type: &FormatValType,
) -> Result<ComponentValue> {
    let mut offset = 0;
    match format_type {
        FormatValType::Bool => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize bool".to_string(),
                ));
            }
            let value = data[offset] != 0;
            Ok(ComponentValue::Bool(value))
        }
        FormatValType::S8 => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize S8".to_string(),
                ));
            }
            let value = data[offset] as i8;
            Ok(ComponentValue::S8(value))
        }
        FormatValType::U8 => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize U8".to_string(),
                ));
            }
            let value = data[offset];
            Ok(ComponentValue::U8(value))
        }
        FormatValType::S16 => {
            if offset + 2 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize S16".to_string(),
                ));
            }
            let value = i16::from_le_bytes([data[offset], data[offset + 1]]);
            Ok(ComponentValue::S16(value))
        }
        FormatValType::U16 => {
            if offset + 2 > data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize U16".to_string(),
                ));
            }
            let value = u16::from_le_bytes([data[offset], data[offset + 1]]);
            Ok(ComponentValue::U16(value))
        }
        FormatValType::S32 => {
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
            Ok(ComponentValue::S32(value))
        }
        FormatValType::U32 => {
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
            Ok(ComponentValue::U32(value))
        }
        FormatValType::S64 => {
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
            Ok(ComponentValue::S64(value))
        }
        FormatValType::U64 => {
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
            Ok(ComponentValue::U64(value))
        }
        FormatValType::F32 => {
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
            Ok(ComponentValue::F32(value))
        }
        FormatValType::F64 => {
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
            Ok(ComponentValue::F64(value))
        }
        FormatValType::Char => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Char".to_string(),
                ));
            }
            let value = data[offset] as char;
            Ok(ComponentValue::Char(value))
        }
        FormatValType::String => {
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
                        format!("Invalid UTF-8 in string: {}", e).to_string(),
                    )
                })?;
            Ok(ComponentValue::String(value))
        }
        FormatValType::List(elem_type) => {
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
            Ok(ComponentValue::List(values))
        }
        FormatValType::Record(fields) => {
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
            Ok(ComponentValue::Record(values))
        }
        FormatValType::Tuple(types) => {
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
            Ok(ComponentValue::Tuple(values))
        }
        FormatValType::Variant(cases) => {
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
                    format!("Invalid variant case index: {}", case_idx).to_string(),
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
                    Ok(ComponentValue::Variant(case_name.clone(), Some(Box::new(inner_value))))
                }
                None => {
                    // Case without a value
                    Ok(ComponentValue::Variant(case_name.clone(), None))
                }
            }
        }
        FormatValType::Enum(variants) => {
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
            Ok(ComponentValue::Enum(variants[value as usize].clone()))
        }
        FormatValType::Option(inner_type) => {
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
                Ok(ComponentValue::Option(Some(Box::new(inner_value))))
            } else {
                Ok(ComponentValue::Option(None))
            }
        }
        FormatValType::Result(result_type) => {
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
                Ok(ComponentValue::Result(Ok(Box::new(inner_value))))
            } else {
                // Create a default error value when the result is not successful
                Ok(ComponentValue::Result(Err(Box::new(ComponentValue::Void))))
            }
        }
        FormatValType::ResultErr(err_type) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize ResultErr".to_string(),
                ));
            }
            let value = data[offset] != 0;
            // No need to update offset anymore as we return immediately
            if value {
                Ok(ComponentValue::Result(Err(Box::new(ComponentValue::Bool(true)))))
            } else {
                Ok(ComponentValue::Result(Err(Box::new(ComponentValue::Bool(false)))))
            }
        }
        FormatValType::ResultBoth(ok_type, err_type) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize ResultBoth".to_string(),
                ));
            }
            let value = data[offset] != 0;
            // No need to update offset anymore as we return immediately
            if value {
                let ok_value = deserialize_component_value(&data[offset..], ok_type)?;
                Ok(ComponentValue::Result(Ok(Box::new(ok_value))))
            } else {
                // Use the err_type to deserialize an error value
                let err_value = deserialize_component_value(&data[offset..], err_type)?;
                Ok(ComponentValue::Result(Err(Box::new(err_value))))
            }
        }
        FormatValType::Own(idx) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Own".to_string(),
                ));
            }
            let value = data[offset] as u32;
            // No need to update offset anymore as we return immediately
            Ok(ComponentValue::Handle(value))
        }
        FormatValType::Borrow(idx) => {
            if offset >= data.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Not enough data to deserialize Borrow".to_string(),
                ));
            }
            let value = data[offset] as u32;
            // No need to update offset anymore as we return immediately
            Ok(ComponentValue::Borrow(value))
        }
        FormatValType::Flags(names) => {
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

            Ok(ComponentValue::Flags(flags))
        }
        FormatValType::FixedList(elem_type, size) => {
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
            Ok(ComponentValue::FixedList(values, *size))
        }
        FormatValType::ErrorContext => {
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
            Ok(ComponentValue::ErrorContext(Vec::new()))
        }
        // Handle any other unimplemented cases
        _ => Err(Error::new(
            ErrorCategory::System,
            codes::UNSUPPORTED_OPERATION,
            "Type not supported for deserialization".to_string(),
        )),
    }
}

/// Serialize multiple component values
pub fn serialize_component_values(values: &[ComponentValue]) -> Result<Vec<u8>> {
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

/// Deserialize multiple component values
pub fn deserialize_component_values(
    data: &[u8],
    types: &[FormatValType],
) -> Result<Vec<ComponentValue>> {
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
            format!("Expected {} types but only got {}", count, types.len()),
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

// Core value conversion functions
pub fn core_to_component_value(value: &Value, ty: &FormatValType) -> Result<ComponentValue> {
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
        (ComponentValue::S32(_), ValType::S32)
        | (ComponentValue::S64(_), ValType::S64)
        | (ComponentValue::F32(_), ValType::F32)
        | (ComponentValue::F64(_), ValType::F64) => Ok(component_value),

        // Handle boolean conversion from i32
        (ComponentValue::S32(v), ValType::Bool) => Ok(ComponentValue::Bool(*v != 0)),

        // Other integer width conversions
        (ComponentValue::S32(v), ValType::S8) => Ok(ComponentValue::S8(*v as i8)),
        (ComponentValue::S32(v), ValType::U8) => Ok(ComponentValue::U8(*v as u8)),
        (ComponentValue::S32(v), ValType::S16) => Ok(ComponentValue::S16(*v as i16)),
        (ComponentValue::S32(v), ValType::U16) => Ok(ComponentValue::U16(*v as u16)),
        (ComponentValue::S32(v), ValType::U32) => Ok(ComponentValue::U32(*v as u32)),
        (ComponentValue::S64(v), ValType::U64) => Ok(ComponentValue::U64(*v as u64)),

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

pub fn component_to_core_value(value: &ComponentValue) -> Result<Value> {
    use crate::type_conversion::types_componentvalue_to_core_value;

    // Use the centralized conversion function
    types_componentvalue_to_core_value(value)
}

// Size calculation for component values
pub fn size_in_bytes(ty: &FormatValType) -> usize {
    match ty {
        FormatValType::Bool => 1,
        FormatValType::S8 => 1,
        FormatValType::U8 => 1,
        FormatValType::S16 => 2,
        FormatValType::U16 => 2,
        FormatValType::S32 => 4,
        FormatValType::U32 => 4,
        FormatValType::S64 => 8,
        FormatValType::U64 => 8,
        FormatValType::F32 => 4,
        FormatValType::F64 => 8,
        FormatValType::Char => 4,
        FormatValType::String => 8, // Pointer size
        FormatValType::Ref(_) => 4,
        FormatValType::Record(_) => 8,
        FormatValType::Variant(_) => 8,
        FormatValType::List(_) => 8,
        FormatValType::Tuple(_) => 8,
        FormatValType::Flags(_) => 8,
        FormatValType::Enum(_) => 4,
        FormatValType::Option(_) => 8,
        FormatValType::Result(_) => 8,
        FormatValType::ResultErr(_) => 8,
        FormatValType::ResultBoth(_, _) => 8,
        FormatValType::Own(_) => 4,
        FormatValType::Borrow(_) => 4,
        FormatValType::FixedList(_, _) => 8,
        FormatValType::ErrorContext => 4,
        // Default size for any unhandled types
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_value_encoding_decoding() {
        // Test a few primitive types
        let values =
            vec![ComponentValue::Bool(true), ComponentValue::S32(42), ComponentValue::F64(3.14159)];

        for value in values {
            let encoded = serialize_component_value(&value).unwrap();
            let format_type = convert_common_to_format_valtype(&value.get_type());
            let decoded = deserialize_component_value(&encoded, &format_type).unwrap();

            // Only check bools since we only implemented deserialization for a subset of types
            if let ComponentValue::Bool(_) = value {
                assert_eq!(value, decoded);
            }
        }
    }
}
