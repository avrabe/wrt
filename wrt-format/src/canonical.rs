//! WebAssembly Component Model canonical ABI.
//!
//! This module implements the Canonical ABI for WebAssembly component model.

#[cfg(feature = "std")]
use std::{boxed::Box, string::String, vec, vec::Vec};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{boxed::Box, string::String, string::ToString, vec, vec::Vec};

use wrt_types::component_value::ValType;

/// Canonical ABI memory layout for component types
#[derive(Debug, Clone)]
pub struct CanonicalLayout {
    /// Size of the type in bytes when stored in memory
    pub size: u32,
    /// Alignment of the type in bytes when stored in memory
    pub alignment: u32,
    /// Offset within the parent structure (if nested)
    pub offset: Option<u32>,
    /// Details specific to the type
    pub details: CanonicalLayoutDetails,
}

/// Details for canonical memory layout
#[derive(Debug, Clone)]
pub enum CanonicalLayoutDetails {
    /// Primitive type layout
    Primitive,
    /// Record type layout with field information
    Record {
        /// Field layouts by name
        fields: Vec<(String, CanonicalLayout)>,
    },
    /// Variant type layout with tag information
    Variant {
        /// Tag size in bytes (1, 2, or 4)
        tag_size: u8,
        /// Case layouts by name
        cases: Vec<(String, Option<CanonicalLayout>)>,
    },
    /// List type layout
    List {
        /// Element layout
        element: Box<CanonicalLayout>,
        /// Whether it's a fixed-length list
        fixed_length: Option<u32>,
    },
    /// String type layout
    String {
        /// String encoding
        encoding: StringEncoding,
    },
    /// Resource handle layout
    Resource {
        /// Number of bits in the handle
        handle_bits: u8,
    },
}

/// String encoding options for canonical ABI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringEncoding {
    /// UTF-8 encoding
    UTF8,
    /// UTF-16 encoding
    UTF16,
    /// Latin-1 encoding
    Latin1,
    /// ASCII encoding
    ASCII,
}

/// Calculate canonical memory layout for a value type
pub fn calculate_layout(ty: &ValType) -> CanonicalLayout {
    match ty {
        ValType::Bool => CanonicalLayout {
            size: 1,
            alignment: 1,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::S8 | ValType::U8 => CanonicalLayout {
            size: 1,
            alignment: 1,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::S16 | ValType::U16 => CanonicalLayout {
            size: 2,
            alignment: 2,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::S32 | ValType::U32 | ValType::F32 => CanonicalLayout {
            size: 4,
            alignment: 4,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::S64 | ValType::U64 | ValType::F64 => CanonicalLayout {
            size: 8,
            alignment: 8,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::Char => CanonicalLayout {
            size: 4, // Unicode scalar value as u32
            alignment: 4,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::Void => CanonicalLayout {
            size: 0,
            alignment: 1,
            offset: None,
            details: CanonicalLayoutDetails::Primitive,
        },
        ValType::String => CanonicalLayout {
            size: 8, // ptr + len
            alignment: 4,
            offset: None,
            details: CanonicalLayoutDetails::String { encoding: StringEncoding::UTF8 },
        },
        ValType::Record(fields) => {
            let mut field_layouts = Vec::with_capacity(fields.len());
            let mut total_size = 0;
            let mut max_alignment = 1;

            for (name, field_type) in fields {
                let mut field_layout = calculate_layout(field_type);

                // Calculate field offset (respecting alignment)
                total_size = align_up(total_size, field_layout.alignment);
                field_layout.offset = Some(total_size);

                // Add field size
                total_size += field_layout.size;

                // Update max alignment
                max_alignment = max_alignment.max(field_layout.alignment);

                field_layouts.push((name.clone(), field_layout));
            }

            // Align the total size to the max alignment
            total_size = align_up(total_size, max_alignment);

            CanonicalLayout {
                size: total_size,
                alignment: max_alignment,
                offset: None,
                details: CanonicalLayoutDetails::Record { fields: field_layouts },
            }
        }
        ValType::Variant(cases) => {
            let case_count = cases.len();
            let tag_size = if case_count <= 256 {
                1
            } else if case_count <= 65536 {
                2
            } else {
                4
            };

            let mut case_layouts = Vec::with_capacity(cases.len());
            let mut max_payload_size = 0;
            let mut max_payload_alignment = 1;

            for (name, payload_type) in cases {
                if let Some(payload) = payload_type {
                    let payload_layout = calculate_layout(payload);
                    max_payload_size = max_payload_size.max(payload_layout.size);
                    max_payload_alignment = max_payload_alignment.max(payload_layout.alignment);
                    case_layouts.push((name.clone(), Some(payload_layout)));
                } else {
                    case_layouts.push((name.clone(), None));
                }
            }

            // Calculate total size including tag and alignment padding
            let payload_offset = align_up(tag_size as u32, max_payload_alignment);
            let total_size = payload_offset + max_payload_size;

            CanonicalLayout {
                size: total_size,
                alignment: max_payload_alignment.max(tag_size as u32),
                offset: None,
                details: CanonicalLayoutDetails::Variant {
                    tag_size: tag_size as u8,
                    cases: case_layouts,
                },
            }
        }
        ValType::List(element_type) => {
            let element_layout = calculate_layout(element_type);
            CanonicalLayout {
                size: 8, // ptr + len
                alignment: 4,
                offset: None,
                details: CanonicalLayoutDetails::List {
                    element: Box::new(element_layout),
                    fixed_length: None,
                },
            }
        }
        ValType::FixedList(element_type, length) => {
            let element_layout = calculate_layout(element_type);
            let total_size = element_layout.size * length;
            CanonicalLayout {
                size: total_size,
                alignment: element_layout.alignment,
                offset: None,
                details: CanonicalLayoutDetails::List {
                    element: Box::new(element_layout),
                    fixed_length: Some(*length),
                },
            }
        }
        ValType::Tuple(elements) => {
            let mut field_layouts = Vec::with_capacity(elements.len());
            let mut total_size = 0;
            let mut max_alignment = 1;

            for (i, element_type) in elements.iter().enumerate() {
                let mut element_layout = calculate_layout(element_type);

                // Calculate field offset (respecting alignment)
                total_size = align_up(total_size, element_layout.alignment);
                element_layout.offset = Some(total_size);

                // Add field size
                total_size += element_layout.size;

                // Update max alignment
                max_alignment = max_alignment.max(element_layout.alignment);

                field_layouts.push((i.to_string(), element_layout));
            }

            // Align the total size to the max alignment
            total_size = align_up(total_size, max_alignment);

            CanonicalLayout {
                size: total_size,
                alignment: max_alignment,
                offset: None,
                details: CanonicalLayoutDetails::Record { fields: field_layouts },
            }
        }
        ValType::Flags(names) => {
            let byte_count = names.len().div_ceil(8);
            CanonicalLayout {
                size: byte_count as u32,
                alignment: 1,
                offset: None,
                details: CanonicalLayoutDetails::Primitive,
            }
        }
        ValType::Enum(_) => {
            // Enums are represented as a tag
            CanonicalLayout {
                size: 4,
                alignment: 4,
                offset: None,
                details: CanonicalLayoutDetails::Primitive,
            }
        }
        ValType::Option(inner_type) => {
            // Option type is equivalent to variant with None and Some cases
            let inner_layout = calculate_layout(inner_type);

            // 1 byte tag + aligned value
            let tag_size = 1;
            let payload_offset = align_up(tag_size as u32, inner_layout.alignment);
            let total_size = payload_offset + inner_layout.size;

            CanonicalLayout {
                size: total_size,
                alignment: inner_layout.alignment.max(tag_size as u32),
                offset: None,
                details: CanonicalLayoutDetails::Variant {
                    tag_size: tag_size as u8,
                    cases: vec![
                        ("None".to_string(), None),
                        ("Some".to_string(), Some(inner_layout)),
                    ],
                },
            }
        }
        ValType::Result(ok_type) => {
            // Result type with just ok value is equivalent to Option
            let ok_layout = calculate_layout(ok_type);

            // 1 byte tag + aligned value
            let tag_size = 1;
            let payload_offset = align_up(tag_size as u32, ok_layout.alignment);
            let total_size = payload_offset + ok_layout.size;

            CanonicalLayout {
                size: total_size,
                alignment: ok_layout.alignment.max(tag_size as u32),
                offset: None,
                details: CanonicalLayoutDetails::Variant {
                    tag_size: tag_size as u8,
                    cases: vec![("Ok".to_string(), Some(ok_layout)), ("Err".to_string(), None)],
                },
            }
        }
        ValType::ResultErr(err_type) => {
            // Result type with just err value is equivalent to Option
            let err_layout = calculate_layout(err_type);

            // 1 byte tag + aligned value
            let tag_size = 1;
            let payload_offset = align_up(tag_size as u32, err_layout.alignment);
            let total_size = payload_offset + err_layout.size;

            CanonicalLayout {
                size: total_size,
                alignment: err_layout.alignment.max(tag_size as u32),
                offset: None,
                details: CanonicalLayoutDetails::Variant {
                    tag_size: tag_size as u8,
                    cases: vec![("Ok".to_string(), None), ("Err".to_string(), Some(err_layout))],
                },
            }
        }
        ValType::ResultBoth(ok_type, err_type) => {
            // Result type with both ok and err values
            let ok_layout = calculate_layout(ok_type);
            let err_layout = calculate_layout(err_type);

            // Determine max alignment and size of the payload
            let max_payload_alignment = ok_layout.alignment.max(err_layout.alignment);
            let max_payload_size = ok_layout.size.max(err_layout.size);

            // 1 byte tag + aligned value
            let tag_size = 1;
            let payload_offset = align_up(tag_size as u32, max_payload_alignment);
            let total_size = payload_offset + max_payload_size;

            CanonicalLayout {
                size: total_size,
                alignment: max_payload_alignment.max(tag_size as u32),
                offset: None,
                details: CanonicalLayoutDetails::Variant {
                    tag_size: tag_size as u8,
                    cases: vec![
                        ("Ok".to_string(), Some(ok_layout)),
                        ("Err".to_string(), Some(err_layout)),
                    ],
                },
            }
        }
        ValType::Own(_) | ValType::Borrow(_) => {
            // Resource handles are represented as 32-bit integers
            CanonicalLayout {
                size: 4,
                alignment: 4,
                offset: None,
                details: CanonicalLayoutDetails::Resource { handle_bits: 32 },
            }
        }
        ValType::ErrorContext => {
            // Error context represented as a structure
            CanonicalLayout {
                size: 16, // Generic size for error context
                alignment: 8,
                offset: None,
                details: CanonicalLayoutDetails::Primitive,
            }
        }
        ValType::Ref(_) => {
            // Reference types are represented as 32-bit indices
            CanonicalLayout {
                size: 4,
                alignment: 4,
                offset: None,
                details: CanonicalLayoutDetails::Primitive,
            }
        }
    }
}

/// Align a value up to the specified alignment boundary
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

/// Canonical ABI type transformation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformMode {
    /// Lift from core to component
    Lift,
    /// Lower from component to core
    Lower,
}

/// Canonical ABI transformation for a type
#[derive(Debug, Clone)]
pub struct TypeTransform {
    /// Original type
    pub original: ValType,
    /// Target type after transformation
    pub target: ValType,
    /// Transformation mode
    pub mode: TransformMode,
    /// Operations needed for the transformation
    pub operations: Vec<TransformOperation>,
}

/// Transformation operation
#[derive(Debug, Clone)]
pub enum TransformOperation {
    /// Convert a primitive type
    ConvertPrimitive,
    /// Unpack string data
    UnpackString {
        /// String encoding to use
        encoding: StringEncoding,
    },
    /// Pack string data
    PackString {
        /// String encoding to use
        encoding: StringEncoding,
        /// Allocator to use
        allocator: Option<u32>,
    },
    /// Unpack list data
    UnpackList {
        /// Element transform
        element_transform: Box<TypeTransform>,
    },
    /// Pack list data
    PackList {
        /// Element transform
        element_transform: Box<TypeTransform>,
        /// Allocator to use
        allocator: Option<u32>,
    },
    /// Convert record fields
    ConvertRecord {
        /// Field transforms
        field_transforms: Vec<(String, TypeTransform)>,
    },
    /// Convert variant cases
    ConvertVariant {
        /// Case transforms
        case_transforms: Vec<(String, Option<TypeTransform>)>,
    },
    /// Lift resource to handle
    LiftResource {
        /// Resource type index
        resource_idx: u32,
    },
    /// Lower handle to resource
    LowerResource {
        /// Resource type index
        resource_idx: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_layouts() {
        let bool_layout = calculate_layout(&ValType::Bool);
        assert_eq!(bool_layout.size, 1);
        assert_eq!(bool_layout.alignment, 1);

        let i32_layout = calculate_layout(&ValType::S32);
        assert_eq!(i32_layout.size, 4);
        assert_eq!(i32_layout.alignment, 4);

        let i64_layout = calculate_layout(&ValType::S64);
        assert_eq!(i64_layout.size, 8);
        assert_eq!(i64_layout.alignment, 8);
    }

    #[test]
    fn test_record_layout() {
        let record_type = ValType::Record(vec![
            ("a".to_string(), ValType::Bool),
            ("b".to_string(), ValType::S32),
            ("c".to_string(), ValType::S16),
        ]);

        let layout = calculate_layout(&record_type);
        assert_eq!(layout.alignment, 4);

        // Note: The exact size depends on padding rules but should be at least 8 bytes
        // (0-1: bool, 2-3: padding, 4-7: i32, 8-9: i16, 10-11: padding)
        assert!(layout.size >= 8);

        if let CanonicalLayoutDetails::Record { fields } = &layout.details {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0].0, "a");
            assert_eq!(fields[0].1.offset, Some(0));
            assert_eq!(fields[1].0, "b");
            assert_eq!(fields[1].1.offset, Some(4));
            assert_eq!(fields[2].0, "c");
            assert!(fields[2].1.offset.unwrap() >= 8);
        } else {
            panic!("Expected Record layout details");
        }
    }

    #[test]
    fn test_variant_layout() {
        let variant_type = ValType::Variant(vec![
            ("a".to_string(), Some(ValType::Bool)),
            ("b".to_string(), Some(ValType::S32)),
            ("c".to_string(), None),
        ]);

        let layout = calculate_layout(&variant_type);
        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.size, 8); // 0: tag, 1-3: padding, 4-7: payload (i32)

        if let CanonicalLayoutDetails::Variant { tag_size, cases } = &layout.details {
            assert_eq!(*tag_size, 1);
            assert_eq!(cases.len(), 3);
        } else {
            panic!("Expected Variant layout details");
        }
    }

    #[test]
    fn test_fixed_list_layout() {
        // Test fixed-length list layout
        let element_type = ValType::U32;
        let length = 10;
        let fixed_list_type = ValType::FixedList(Box::new(element_type), length);

        let layout = calculate_layout(&fixed_list_type);

        // Each u32 is 4 bytes, so 10 elements = 40 bytes
        assert_eq!(layout.size, 40);
        assert_eq!(layout.alignment, 4);

        if let CanonicalLayoutDetails::List { element, fixed_length } = &layout.details {
            assert_eq!(element.size, 4);
            assert_eq!(element.alignment, 4);
            assert_eq!(fixed_length, &Some(10));
        } else {
            panic!("Expected List layout details");
        }
    }

    #[test]
    fn test_error_context_layout() {
        // Test error context layout
        let error_context_type = ValType::ErrorContext;
        let layout = calculate_layout(&error_context_type);

        assert_eq!(layout.size, 16);
        assert_eq!(layout.alignment, 8);

        if let CanonicalLayoutDetails::Primitive = &layout.details {
            // This is correct
        } else {
            panic!("Expected Primitive layout details");
        }
    }

    #[test]
    fn test_resource_layout() {
        // Test resource handle layouts
        let own_type = ValType::Own(42);
        let borrow_type = ValType::Borrow(42);

        let own_layout = calculate_layout(&own_type);
        let borrow_layout = calculate_layout(&borrow_type);

        // Both should be 32-bit handles
        assert_eq!(own_layout.size, 4);
        assert_eq!(own_layout.alignment, 4);
        assert_eq!(borrow_layout.size, 4);
        assert_eq!(borrow_layout.alignment, 4);

        if let CanonicalLayoutDetails::Resource { handle_bits } = &own_layout.details {
            assert_eq!(*handle_bits, 32);
        } else {
            panic!("Expected Resource layout details");
        }

        if let CanonicalLayoutDetails::Resource { handle_bits } = &borrow_layout.details {
            assert_eq!(*handle_bits, 32);
        } else {
            panic!("Expected Resource layout details");
        }
    }

    #[test]
    fn test_align_up_function() {
        // Test the align_up utility function
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(10, 8), 16);
        assert_eq!(align_up(15, 16), 16);
        assert_eq!(align_up(16, 16), 16);
        assert_eq!(align_up(17, 16), 32);
    }
}
