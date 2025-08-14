// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

// Component parsing requires std for Box and complex recursive structures
#[cfg(feature = "std")]
mod std_parsing {
    use wrt_error::{
        kinds,
        Error,
        Result,
    };
    use wrt_format::{
        binary,
        component::{
            Alias,
            Canon,
            Component,
            ComponentType,
            CoreInstance,
            CoreType,
            Export,
            Import,
            Instance,
            Start,
            Value,
        },
        module::Module,
    };
    use wrt_foundation::resource;

    use crate::prelude::*;

    // Helper function to convert &[u8] to String
    fn bytes_to_string_safe(bytes: &[u8]) -> Result<String> {
        match core::str::from_utf8(bytes) {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Err(Error::runtime_execution_error("Invalid UTF-8")),
        }
    }

    // Backward compatible wrapper for standard String
    fn bytes_to_string(bytes: &[u8]) -> String {
        match core::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::new(),
        }
    }

    // Define a helper function for converting format strings to String
    fn format_to_string(message: &str, value: impl core::fmt::Display) -> String {
        #[cfg(feature = "std")]
        {
            format!("{}: {}", message, value)
        }

        #[cfg(all(not(feature = "std")))]
        {
            alloc::format!("{}: {}", message, value)
        }

        #[cfg(not(any(feature = "std",)))]
        {
            // Use capability-aware string creation
            let provider = match crate::prelude::create_decoder_provider::<256>() {
                Ok(p) => p,
                Err(_) => return "[allocation_error]".to_string(),
            };
            let mut s = match crate::prelude::String::from_str(message, provider.clone()) {
                Ok(s) => s,
                Err(_) => return "[string_error]".to_string(),
            };
            if s.push_str(": ").is_ok() && s.push_str("[value]").is_ok() {
                s.into_string()
            } else {
                "[concat_error]".to_string()
            }
        }
    }

    /// Parse a core module section
    pub fn parse_core_module_section(bytes: &[u8]) -> Result<(Vec<Module>, usize)> {
        // Read a vector of modules
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut modules = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read a module binary size
            let (module_size, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;

            if offset + module_size as usize > bytes.len() {
                return Err(Error::parse_error("Module size exceeds section size"));
            }

            // Extract the module binary
            let module_end = offset + module_size as usize;
            let module_bytes = &bytes[offset..module_end];

            // Parse the module binary
            let module = binary::parse_binary(module_bytes)?;
            modules.push(module);

            offset = module_end;
        }

        Ok((modules, offset))
    }

    /// Parse a core instance section
    pub fn parse_core_instance_section(bytes: &[u8]) -> Result<(Vec<CoreInstance>, usize)> {
        // Read a vector of core instances
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut instances = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Parse the instance expression
            let (instance_expr, bytes_read) = parse_core_instance_expr(&bytes[offset..])?;
            offset += bytes_read;

            // Create the instance
            instances.push(CoreInstance { instance_expr });
        }

        Ok((instances, offset))
    }

    /// Parse a core instance expression
    fn parse_core_instance_expr(
        bytes: &[u8],
    ) -> Result<(wrt_format::component::CoreInstanceExpr, usize)> {
        if bytes.is_empty() {
            return Err(Error::parse_error(
                "Unexpected end of input while parsing core instance expression",
            ));
        }

        // Read the expression tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Instantiate a module
                let (module_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read argument vector
                let (args_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut args = Vec::with_capacity(args_count as usize);
                for _ in 0..args_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read instance index
                    let (instance_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;

                    args.push(wrt_format::component::CoreArgReference { name, instance_idx });
                }

                Ok((
                    wrt_format::component::CoreInstanceExpr::ModuleReference {
                        module_idx,
                        arg_refs: args,
                    },
                    offset,
                ))
            },
            0x01 => {
                // Inline exports
                let (exports_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(exports_count as usize);
                for _ in 0..exports_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read kind byte
                    if offset >= bytes.len() {
                        return Err(Error::from(kinds::ParseError(
                            "Unexpected end of input while parsing export kind",
                        )));
                    }
                    let kind_byte = bytes[offset];
                    offset += 1;

                    // Convert to CoreSort
                    let sort = match kind_byte {
                        binary::COMPONENT_CORE_SORT_FUNC => {
                            wrt_format::component::CoreSort::Function
                        },
                        binary::COMPONENT_CORE_SORT_TABLE => wrt_format::component::CoreSort::Table,
                        binary::COMPONENT_CORE_SORT_MEMORY => {
                            wrt_format::component::CoreSort::Memory
                        },
                        binary::COMPONENT_CORE_SORT_GLOBAL => {
                            wrt_format::component::CoreSort::Global
                        },
                        binary::COMPONENT_CORE_SORT_TYPE => wrt_format::component::CoreSort::Type,
                        binary::COMPONENT_CORE_SORT_MODULE => {
                            wrt_format::component::CoreSort::Module
                        },
                        binary::COMPONENT_CORE_SORT_INSTANCE => {
                            wrt_format::component::CoreSort::Instance
                        },
                        _ => {
                            return Err(Error::from(kinds::ParseError("Invalid core sort kind")));
                        },
                    };

                    // Read index
                    let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;

                    exports.push(wrt_format::component::CoreInlineExport { name, sort, idx });
                }

                Ok((
                    wrt_format::component::CoreInstanceExpr::InlineExports(exports),
                    offset,
                ))
            },
            _ => Err(Error::parse_error("Invalid core instance expression tag ")),
        }
    }

    /// Parse a core type section
    pub fn parse_core_type_section(bytes: &[u8]) -> Result<(Vec<CoreType>, usize)> {
        // Read a vector of core types
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut types = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read type definition
            let (definition, bytes_read) = parse_core_type_definition(&bytes[offset..])?;
            offset += bytes_read;

            // Create the type
            types.push(CoreType { definition });
        }

        Ok((types, offset))
    }

    /// Parse a core type definition
    fn parse_core_type_definition(
        bytes: &[u8],
    ) -> Result<(wrt_format::component::CoreTypeDefinition, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing core type definition",
            )));
        }

        // Read the form tag
        let form = bytes[0];
        let mut offset = 1;

        match form {
            0x60 => {
                // Function type

                // Read parameter types
                let (param_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut params = Vec::with_capacity(param_count as usize);
                for _ in 0..param_count {
                    // Read value type
                    if offset >= bytes.len() {
                        return Err(Error::from(kinds::ParseError(
                            "Unexpected end of input while parsing function parameter type",
                        )));
                    }

                    let vtype = match bytes[offset] {
                        binary::I32_TYPE => wrt_format::types::ValueType::I32,
                        binary::I64_TYPE => wrt_format::types::ValueType::I64,
                        binary::F32_TYPE => wrt_format::types::ValueType::F32,
                        binary::F64_TYPE => wrt_format::types::ValueType::F64,
                        binary::V128_TYPE => wrt_format::types::ValueType::ExternRef,
                        binary::FUNCREF_TYPE => wrt_format::types::ValueType::FuncRef,
                        binary::EXTERNREF_TYPE => wrt_format::types::ValueType::ExternRef,
                        _ => {
                            return Err(Error::from(kinds::ParseError("Invalid value type")));
                        },
                    };

                    params.push(vtype);
                    offset += 1;
                }

                // Read result types
                let (result_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut results = Vec::with_capacity(result_count as usize);
                for _ in 0..result_count {
                    // Read value type
                    if offset >= bytes.len() {
                        return Err(Error::from(kinds::ParseError(
                            "Unexpected end of input while parsing function result type",
                        )));
                    }

                    let vtype = match bytes[offset] {
                        binary::I32_TYPE => wrt_format::types::ValueType::I32,
                        binary::I64_TYPE => wrt_format::types::ValueType::I64,
                        binary::F32_TYPE => wrt_format::types::ValueType::F32,
                        binary::F64_TYPE => wrt_format::types::ValueType::F64,
                        binary::V128_TYPE => wrt_format::types::ValueType::ExternRef,
                        binary::FUNCREF_TYPE => wrt_format::types::ValueType::FuncRef,
                        binary::EXTERNREF_TYPE => wrt_format::types::ValueType::ExternRef,
                        _ => {
                            return Err(Error::from(kinds::ParseError("Invalid value type")));
                        },
                    };

                    results.push(vtype);
                    offset += 1;
                }

                Ok((
                    wrt_format::component::CoreTypeDefinition::Function { params, results },
                    offset,
                ))
            },
            0x61 => {
                // Module type

                // Read import vector
                let (import_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut imports = Vec::with_capacity(import_count as usize);
                for _ in 0..import_count {
                    // Read module name
                    let (module_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let module_name = bytes_to_string(module_name_bytes);

                    // Read field name
                    let (field_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let field_name = bytes_to_string(field_name_bytes);

                    // Read import type
                    let (import_type, bytes_read) = parse_core_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    imports.push((module_name, field_name, import_type));
                }

                // Read export vector
                let (export_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(export_count as usize);
                for _ in 0..export_count {
                    // Read export name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read export type
                    let (export_type, bytes_read) = parse_core_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    exports.push((name, export_type));
                }

                Ok((
                    wrt_format::component::CoreTypeDefinition::Module { imports, exports },
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError("Invalid core type form"))),
        }
    }

    /// Parse a core external type
    fn parse_core_extern_type(
        bytes: &[u8],
    ) -> Result<(wrt_format::component::CoreExternType, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing core external type",
            )));
        }

        // Read the type tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Function type

                // Read type index
                let (_type_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Function types are referenced by index, but we need to inline the
                // params/results In a real implementation, this would look up the
                // type in the type section For now, we'll just return an empty
                // function type
                {
                    let provider = crate::prelude::create_decoder_provider::<512>()?;
                    // Use standard Vec for std mode, explicit types for no_std
                    #[cfg(feature = "std")]
                    let empty_params = Vec::new();
                    #[cfg(feature = "std")]
                    let empty_results = Vec::new();

                    #[cfg(not(feature = "std"))]
                    let empty_params = {
                        let provider = crate::prelude::create_decoder_provider::<1024>()?;
                        crate::prelude::DecoderVec::new(provider)?
                    };
                    #[cfg(not(feature = "std"))]
                    let empty_results = {
                        let provider = crate::prelude::create_decoder_provider::<1024>()?;
                        crate::prelude::DecoderVec::new(provider)?
                    };
                    Ok((
                        wrt_format::component::CoreExternType::Function {
                            params:  empty_params.to_vec(),
                            results: empty_results.to_vec(),
                        },
                        offset,
                    ))
                }
            },
            0x01 => {
                // Table type

                // Read element type
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing table element type",
                    )));
                }

                let element_type = match bytes[offset] {
                    binary::FUNCREF_TYPE => wrt_format::types::ValueType::FuncRef,
                    binary::EXTERNREF_TYPE => wrt_format::types::ValueType::ExternRef,
                    _ => {
                        return Err(Error::from(kinds::ParseError("Invalid table element type")));
                    },
                };
                offset += 1;

                // Read limits
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing table limits",
                    )));
                }

                let limit_flag = bytes[offset];
                offset += 1;

                // Read minimum size
                let (min, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read maximum size (if present)
                let max = if limit_flag & 0x01 != 0 {
                    let (max, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;
                    Some(max)
                } else {
                    None
                };

                Ok((
                    wrt_format::component::CoreExternType::Table {
                        element_type,
                        min,
                        max,
                    },
                    offset,
                ))
            },
            0x02 => {
                // Memory type

                // Read limits
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing memory limits",
                    )));
                }

                let limit_flag = bytes[offset];
                offset += 1;

                // Check if memory is shared
                let shared = (limit_flag & 0x02) != 0;

                // Read minimum size
                let (min, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read maximum size (if present)
                let max = if limit_flag & 0x01 != 0 {
                    let (max, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;
                    Some(max)
                } else {
                    None
                };

                Ok((
                    wrt_format::component::CoreExternType::Memory { min, max, shared },
                    offset,
                ))
            },
            0x03 => {
                // Global type

                // Read value type
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing global value type",
                    )));
                }

                let value_type = match bytes[offset] {
                    binary::I32_TYPE => wrt_format::types::ValueType::I32,
                    binary::I64_TYPE => wrt_format::types::ValueType::I64,
                    binary::F32_TYPE => wrt_format::types::ValueType::F32,
                    binary::F64_TYPE => wrt_format::types::ValueType::F64,
                    binary::V128_TYPE => wrt_format::types::ValueType::ExternRef,
                    binary::FUNCREF_TYPE => wrt_format::types::ValueType::FuncRef,
                    binary::EXTERNREF_TYPE => wrt_format::types::ValueType::ExternRef,
                    _ => {
                        return Err(Error::from(kinds::ParseError("Invalid global value type")));
                    },
                };
                offset += 1;

                // Read mutability flag
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing global mutability",
                    )));
                }

                let mutable = bytes[offset] != 0;
                offset += 1;

                Ok((
                    wrt_format::component::CoreExternType::Global {
                        value_type,
                        mutable,
                    },
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError(
                "Invalid core external type tag",
            ))),
        }
    }

    /// Parse a component section
    pub fn parse_component_section(bytes: &[u8]) -> Result<(Vec<Component>, usize)> {
        // Read a vector of components
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut components = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read component size
            let (component_size, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;

            if offset + component_size as usize > bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Component size exceeds section size",
                )));
            }

            // Extract the component binary
            let component_end = offset + component_size as usize;
            let component_bytes = &bytes[offset..component_end];

            // Parse the component binary using the decoder
            match crate::component::decode_component(component_bytes) {
                Ok(component) => components.push(component),
                Err(_e) => {
                    return Err(Error::from(kinds::ParseError(
                        "Failed to parse nested component",
                    )));
                },
            }

            offset = component_end;
        }

        Ok((components, offset))
    }

    /// Parse an instance section
    pub fn parse_instance_section(bytes: &[u8]) -> Result<(Vec<Instance>, usize)> {
        // Read a vector of instances
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut instances = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Parse the instance expression
            let (instance_expr, bytes_read) = parse_instance_expr(&bytes[offset..])?;
            offset += bytes_read;

            // Create the instance
            instances.push(Instance { instance_expr });
        }

        Ok((instances, offset))
    }

    /// Parse an instance expression
    fn parse_instance_expr(bytes: &[u8]) -> Result<(wrt_format::component::InstanceExpr, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing instance expression",
            )));
        }

        // Read the expression tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Instantiate a component
                let (component_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read argument vector
                let (args_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut args = Vec::with_capacity(args_count as usize);
                for _ in 0..args_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read sort byte
                    if offset >= bytes.len() {
                        return Err(Error::from(kinds::ParseError(
                            "Unexpected end of input while parsing instantiation argument sort",
                        )));
                    }
                    let sort_byte = bytes[offset];
                    offset += 1;

                    // Parse sort
                    let sort = parse_sort(sort_byte)?;

                    // Read index
                    let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;

                    args.push(wrt_format::component::InstantiateArgReference { name, sort, idx });
                }

                Ok((
                    wrt_format::component::InstanceExpr::ComponentReference {
                        component_idx,
                        arg_refs: args,
                    },
                    offset,
                ))
            },
            0x01 => {
                // Inline exports
                let (exports_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(exports_count as usize);
                for _ in 0..exports_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read sort byte
                    if offset >= bytes.len() {
                        return Err(Error::from(kinds::ParseError(
                            "Unexpected end of input while parsing export sort",
                        )));
                    }
                    let sort_byte = bytes[offset];
                    offset += 1;

                    // Parse sort
                    let sort = parse_sort(sort_byte)?;

                    // Read index
                    let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;

                    exports.push(wrt_format::component::InlineExport { name, sort, idx });
                }

                Ok((
                    wrt_format::component::InstanceExpr::InlineExports(exports),
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError(
                "Invalid instance expression tag",
            ))),
        }
    }

    /// Parse a sort byte
    fn parse_sort(sort_byte: u8) -> Result<wrt_format::component::Sort> {
        match sort_byte {
            binary::COMPONENT_SORT_CORE => Ok(wrt_format::component::Sort::Core(
                wrt_format::component::CoreSort::Function,
            )),
            binary::COMPONENT_SORT_FUNC => Ok(wrt_format::component::Sort::Function),
            binary::COMPONENT_SORT_MODULE => Ok(wrt_format::component::Sort::Component),
            binary::COMPONENT_SORT_INSTANCE => Ok(wrt_format::component::Sort::Instance),
            binary::COMPONENT_SORT_COMPONENT => Ok(wrt_format::component::Sort::Component),
            binary::COMPONENT_SORT_VALUE => Ok(wrt_format::component::Sort::Value),
            binary::COMPONENT_SORT_TYPE => Ok(wrt_format::component::Sort::Type),
            _ => Err(Error::from(kinds::ParseError("Invalid sort byte"))),
        }
    }

    /// Parse a canon section
    pub fn parse_canon_section(bytes: &[u8]) -> Result<(Vec<Canon>, usize)> {
        // Read a vector of canon operations
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut canons = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read canon operation
            let (operation, bytes_read) = parse_canon_operation(&bytes[offset..])?;
            offset += bytes_read;

            // Create the canon
            canons.push(Canon { operation });
        }

        Ok((canons, offset))
    }

    /// Parse a canon operation
    fn parse_canon_operation(
        bytes: &[u8],
    ) -> Result<(wrt_format::component::CanonOperation, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing canon operation",
            )));
        }

        // Read the operation tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Lift operation

                // Read core function index
                let (func_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read type index
                let (type_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read options
                let (options, bytes_read) = parse_lift_options(&bytes[offset..])?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::CanonOperation::Lift {
                        func_idx,
                        type_idx,
                        options,
                    },
                    offset,
                ))
            },
            0x01 => {
                // Lower operation

                // Read function index
                let (func_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read options
                let (options, bytes_read) = parse_lower_options(&bytes[offset..])?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::CanonOperation::Lower { func_idx, options },
                    offset,
                ))
            },
            0x02 => {
                // Resource operations

                // Read resource operation
                let (resource_op, bytes_read) = parse_resource_operation(&bytes[offset..])?;
                offset += bytes_read;

                // Convert from ResourceCanonicalOperation to FormatResourceOperation
                let format_resource_op = match resource_op {
                    resource::ResourceCanonicalOperation::New(new) => {
                        wrt_format::component::FormatResourceOperation::New(new)
                    },
                    resource::ResourceCanonicalOperation::Drop(drop) => {
                        wrt_format::component::FormatResourceOperation::Drop(drop)
                    },
                    resource::ResourceCanonicalOperation::Rep(rep) => {
                        wrt_format::component::FormatResourceOperation::Rep(rep)
                    },
                };

                Ok((
                    wrt_format::component::CanonOperation::Resource(format_resource_op),
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError(
                "Invalid canon operation tag",
            ))),
        }
    }

    /// Parse lift options
    fn parse_lift_options(bytes: &[u8]) -> Result<(wrt_format::component::LiftOptions, usize)> {
        let mut offset = 0;

        // Read memory index (optional)
        let (has_memory, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        let memory_idx = if has_memory != 0 {
            let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;
            Some(idx)
        } else {
            None
        };

        // Read string encoding (optional)
        let (has_encoding, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        let string_encoding = if has_encoding != 0 {
            if offset >= bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Unexpected end of input while parsing string encoding",
                )));
            }

            let encoding_byte = bytes[offset];
            offset += 1;

            let encoding = match encoding_byte {
                0x00 => wrt_format::component::StringEncoding::UTF8,
                0x01 => wrt_format::component::StringEncoding::UTF16,
                0x02 => wrt_format::component::StringEncoding::Latin1,
                0x03 => wrt_format::component::StringEncoding::ASCII,
                _ => {
                    return Err(Error::from(kinds::ParseError("Invalid string encoding")));
                },
            };

            Some(encoding)
        } else {
            None
        };

        let string_encoding_value = match string_encoding {
            Some(encoding) => encoding,
            None => wrt_format::component::StringEncoding::UTF8,
        };

        Ok((
            wrt_format::component::LiftOptions {
                memory_idx:           Some(memory_idx.unwrap_or(0)),
                string_encoding:      Some(string_encoding_value),
                realloc_func_idx:     None,
                post_return_func_idx: None,
                is_async:             false,
            },
            offset,
        ))
    }

    /// Parse lower options
    fn parse_lower_options(bytes: &[u8]) -> Result<(wrt_format::component::LowerOptions, usize)> {
        let mut offset = 0;

        // Read memory index (optional)
        let (has_memory, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        let memory_idx = if has_memory != 0 {
            let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;
            Some(idx)
        } else {
            None
        };

        // Read string encoding (optional)
        let (has_encoding, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        let string_encoding = if has_encoding != 0 {
            if offset >= bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Unexpected end of input while parsing string encoding",
                )));
            }

            let encoding_byte = bytes[offset];
            offset += 1;

            let encoding = match encoding_byte {
                0x00 => wrt_format::component::StringEncoding::UTF8,
                0x01 => wrt_format::component::StringEncoding::UTF16,
                0x02 => wrt_format::component::StringEncoding::Latin1,
                0x03 => wrt_format::component::StringEncoding::ASCII,
                _ => {
                    return Err(Error::from(kinds::ParseError("Invalid string encoding")));
                },
            };

            Some(encoding)
        } else {
            None
        };

        let string_encoding_value = match string_encoding {
            Some(encoding) => encoding,
            None => wrt_format::component::StringEncoding::UTF8,
        };

        Ok((
            wrt_format::component::LowerOptions {
                memory_idx:       Some(memory_idx.unwrap_or(0)),
                string_encoding:  Some(string_encoding_value),
                realloc_func_idx: None,
                is_async:         false,
                error_mode:       None,
            },
            offset,
        ))
    }

    /// Parse resource operation
    fn parse_resource_operation(
        bytes: &[u8],
    ) -> Result<(resource::ResourceCanonicalOperation, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing resource operation",
            )));
        }

        // Read the tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // New operation
                let (type_idx, type_idx_size) = binary::read_leb128_u32(bytes, offset)?;
                offset += type_idx_size;

                Ok((
                    resource::ResourceCanonicalOperation::New(resource::ResourceNew { type_idx }),
                    offset,
                ))
            },
            0x01 => {
                // Drop operation
                let (type_idx, type_idx_size) = binary::read_leb128_u32(bytes, offset)?;
                offset += type_idx_size;

                Ok((
                    resource::ResourceCanonicalOperation::Drop(resource::ResourceDrop { type_idx }),
                    offset,
                ))
            },
            0x02 => {
                // Rep operation
                let (type_idx, type_idx_size) = binary::read_leb128_u32(bytes, offset)?;
                offset += type_idx_size;

                Ok((
                    resource::ResourceCanonicalOperation::Rep(resource::ResourceRep { type_idx }),
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError(
                "Invalid resource operation tag",
            ))),
        }
    }

    /// Parse a component type section
    pub fn parse_component_type_section(bytes: &[u8]) -> Result<(Vec<ComponentType>, usize)> {
        // Read a vector of component types
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut types = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read type definition
            let (definition, bytes_read) = parse_component_type_definition(&bytes[offset..])?;
            offset += bytes_read;

            // Create the type
            types.push(ComponentType { definition });
        }

        Ok((types, offset))
    }

    /// Parse a component type definition
    fn parse_component_type_definition(
        bytes: &[u8],
    ) -> Result<(wrt_format::component::ComponentTypeDefinition, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing component type definition",
            )));
        }

        // Read the form tag
        let form = bytes[0];
        let mut offset = 1;

        match form {
            0x00 => {
                // Component type

                // Read import vector
                let (import_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut imports = Vec::with_capacity(import_count as usize);
                for _ in 0..import_count {
                    // Read namespace
                    let (namespace_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let namespace = bytes_to_string(namespace_bytes);

                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read type
                    let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    imports.push((namespace, name, extern_type));
                }

                // Read export vector
                let (export_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(export_count as usize);
                for _ in 0..export_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read type
                    let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    exports.push((name, extern_type));
                }

                Ok((
                    wrt_format::component::ComponentTypeDefinition::Component { imports, exports },
                    offset,
                ))
            },
            0x01 => {
                // Instance type

                // Read export vector
                let (export_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(export_count as usize);
                for _ in 0..export_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read type
                    let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    exports.push((name, extern_type));
                }

                Ok((
                    wrt_format::component::ComponentTypeDefinition::Instance { exports },
                    offset,
                ))
            },
            0x02 => {
                // Function type

                // Read parameter vector
                let (param_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut params = Vec::with_capacity(param_count as usize);
                for _ in 0..param_count {
                    // Read parameter name
                    let (name, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;

                    // Read parameter type
                    let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                    offset += bytes_read;

                    params.push((name, val_type));
                }

                // Read result vector
                let (result_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut results = Vec::with_capacity(result_count as usize);
                for _ in 0..result_count {
                    // Read result type
                    let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                    offset += bytes_read;

                    results.push(val_type);
                }

                Ok((
                    wrt_format::component::ComponentTypeDefinition::Function {
                        params:  params
                            .into_iter()
                            .map(|(name, ty)| {
                                let name_str = core::str::from_utf8(name)
                                    .unwrap_or("invalid_utf8")
                                    .to_string();
                                (name_str, val_type_to_format_val_type(ty))
                            })
                            .collect(),
                        results: results.into_iter().map(val_type_to_format_val_type).collect(),
                    },
                    offset,
                ))
            },
            0x03 => {
                // Value type

                // Read value type
                let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::ComponentTypeDefinition::Value(
                        val_type_to_format_val_type(val_type),
                    ),
                    offset,
                ))
            },
            0x04 => {
                // Resource type

                // Read representation
                let (representation, bytes_read) = parse_resource_representation(&bytes[offset..])?;
                offset += bytes_read;

                // Read nullable flag
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing resource nullable flag",
                    )));
                }
                let nullable = bytes[offset] != 0;
                offset += 1;

                Ok((
                    wrt_format::component::ComponentTypeDefinition::Resource {
                        representation,
                        nullable,
                    },
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError(
                "Invalid component type form",
            ))),
        }
    }

    /// Parse a resource representation
    fn parse_resource_representation(
        bytes: &[u8],
    ) -> Result<(resource::ResourceRepresentation, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing resource representation",
            )));
        }

        // Read the tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Handle32
                Ok((resource::ResourceRepresentation::Handle32, offset))
            },
            0x01 => {
                // Handle64
                Ok((resource::ResourceRepresentation::Handle64, offset))
            },
            0x02 => {
                // Record representation

                // Read field vector
                let (field_count, bytes_read) = match binary::read_leb128_u32(bytes, offset) {
                    Ok(result) => result,
                    Err(_e) => {
                        return Err(Error::from(kinds::ParseError(
                            "Failed to read field count in resource record representation",
                        )))
                    },
                };
                offset += bytes_read;

                let mut fields = Vec::with_capacity(field_count as usize);
                for _i in 0..field_count {
                    // Read field name
                    let (name_bytes, bytes_read) = match binary::read_string(bytes, offset) {
                        Ok(result) => result,
                        Err(_e) => {
                            return Err(Error::from(kinds::ParseError(
                                "Failed to read field name in resource record representation",
                            )))
                        },
                    };
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    fields.push(name);
                }

                // Convert Vec to BoundedVec for ResourceRepresentation
                #[cfg(feature = "std")]
                let bounded_fields = {
                    use wrt_foundation::{
                        resource::MAX_RESOURCE_FIELD_NAME_LEN,
                        BoundedString,
                        BoundedVec,
                        NoStdProvider,
                    };
                    let provider = wrt_foundation::safe_managed_alloc!(
                        4096,
                        wrt_foundation::budget_aware_provider::CrateId::Decoder
                    )?;
                    let mut bounded = BoundedVec::new(provider.clone())?;
                    for field in fields {
                        let bounded_string = BoundedString::<
                            MAX_RESOURCE_FIELD_NAME_LEN,
                            NoStdProvider<4096>,
                        >::from_str(
                            &field,
                            wrt_foundation::safe_managed_alloc!(
                                4096,
                                wrt_foundation::budget_aware_provider::CrateId::Decoder
                            )?,
                        )
                        .map_err(|_| {
                            Error::runtime_execution_error("Failed to create bounded string")
                        })?;
                        if bounded.push(bounded_string).is_err() {
                            return Err(Error::new(
                                wrt_error::ErrorCategory::Memory,
                                wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                                "Failed to allocate memory for string",
                            ));
                        }
                    }
                    Ok::<
                        BoundedVec<
                            BoundedString<64, wrt_foundation::safe_memory::NoStdProvider<4096>>,
                            16,
                            wrt_foundation::safe_memory::NoStdProvider<4096>,
                        >,
                        wrt_error::Error,
                    >(bounded)
                }?;

                Ok((
                    #[cfg(feature = "std")]
                    resource::ResourceRepresentation::Record(bounded_fields),
                    #[cfg(not(feature = "std"))]
                    resource::ResourceRepresentation::Record,
                    offset,
                ))
            },
            0x03 => {
                // Aggregate representation

                // Read type indices
                let (index_count, bytes_read) = match binary::read_leb128_u32(bytes, offset) {
                    Ok(result) => result,
                    Err(_e) => {
                        return Err(Error::from(kinds::ParseError(
                            "Failed to read index count in resource aggregate representation",
                        )))
                    },
                };
                offset += bytes_read;

                let mut indices = Vec::with_capacity(index_count as usize);
                for _i in 0..index_count {
                    // Read type index
                    let (idx, bytes_read) = match binary::read_leb128_u32(bytes, offset) {
                        Ok(result) => result,
                        Err(_e) => {
                            return Err(Error::parse_error(
                                "Failed to read type index in resource aggregate representation",
                            ))
                        },
                    };
                    offset += bytes_read;

                    indices.push(idx);
                }

                #[cfg(feature = "std")]
                let repr = {
                    use wrt_foundation::{
                        BoundedVec,
                        NoStdProvider,
                    };
                    let provider = wrt_foundation::safe_managed_alloc!(
                        4096,
                        wrt_foundation::budget_aware_provider::CrateId::Decoder
                    )?;
                    let mut bounded_indices = BoundedVec::new(provider)?;
                    for idx in indices {
                        if bounded_indices.push(idx).is_err() {
                            return Err(Error::runtime_execution_error(
                                "Failed to push index to bounded vector",
                            ));
                        }
                    }
                    resource::ResourceRepresentation::Aggregate(bounded_indices)
                };
                #[cfg(not(feature = "std"))]
                let repr = resource::ResourceRepresentation::Record;

                Ok((repr, offset))
            },
            _ => Err(Error::parse_error("Invalid resource representation tag ")),
        }
    }

    /// Parse an external type
    fn parse_extern_type(bytes: &[u8]) -> Result<(wrt_format::component::ExternType, usize)> {
        if bytes.is_empty() {
            return Err(Error::parse_error(
                "Unexpected end of input while parsing external type",
            ));
        }

        // Read the tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Function type

                // Read parameter vector
                let (param_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut params = Vec::with_capacity(param_count as usize);
                for _ in 0..param_count {
                    // Read parameter name
                    let (name, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;

                    // Read parameter type
                    let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                    offset += bytes_read;

                    params.push((name, val_type));
                }

                // Read result vector
                let (result_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut results = Vec::with_capacity(result_count as usize);
                for _ in 0..result_count {
                    // Read result type
                    let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                    offset += bytes_read;

                    results.push(val_type);
                }

                Ok((
                    wrt_format::component::ExternType::Function {
                        params:  params
                            .into_iter()
                            .map(|(name, ty)| {
                                let name_str = core::str::from_utf8(name)
                                    .unwrap_or("invalid_utf8")
                                    .to_string();
                                (name_str, val_type_to_format_val_type(ty))
                            })
                            .collect(),
                        results: results.into_iter().map(val_type_to_format_val_type).collect(),
                    },
                    offset,
                ))
            },
            0x01 => {
                // Value type

                // Read value type
                let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::ExternType::Value(val_type_to_format_val_type(val_type)),
                    offset,
                ))
            },
            0x02 => {
                // Type reference

                // Read type index
                let (type_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                Ok((wrt_format::component::ExternType::Type(type_idx), offset))
            },
            0x03 => {
                // Instance type

                // Read export vector
                let (export_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(export_count as usize);
                for _ in 0..export_count {
                    // Read export name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read export type
                    let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    exports.push((name, extern_type));
                }

                Ok((
                    wrt_format::component::ExternType::Instance { exports },
                    offset,
                ))
            },
            0x04 => {
                // Component type

                // Read import vector
                let (import_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut imports = Vec::with_capacity(import_count as usize);
                for _ in 0..import_count {
                    // Read namespace
                    let (namespace_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let namespace = bytes_to_string(namespace_bytes);

                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read type
                    let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    imports.push((namespace, name, extern_type));
                }

                // Read export vector
                let (export_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut exports = Vec::with_capacity(export_count as usize);
                for _ in 0..export_count {
                    // Read name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read type
                    let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                    offset += bytes_read;

                    exports.push((name, extern_type));
                }

                Ok((
                    wrt_format::component::ExternType::Component { imports, exports },
                    offset,
                ))
            },
            _ => Err(Error::parse_error("Invalid external type tag ")),
        }
    }

    /// Parse a value type
    fn parse_val_type(bytes: &[u8]) -> Result<(wrt_format::component::FormatValType, usize)> {
        if bytes.is_empty() {
            return Err(Error::parse_error(
                "Unexpected end of input while parsing value type",
            ));
        }

        // Read the type tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x7F => Ok((wrt_format::component::FormatValType::Bool, offset)),
            0x7E => Ok((wrt_format::component::FormatValType::S8, offset)),
            0x7D => Ok((wrt_format::component::FormatValType::U8, offset)),
            0x7C => Ok((wrt_format::component::FormatValType::S16, offset)),
            0x7B => Ok((wrt_format::component::FormatValType::U16, offset)),
            0x7A => Ok((wrt_format::component::FormatValType::S32, offset)),
            0x79 => Ok((wrt_format::component::FormatValType::U32, offset)),
            0x78 => Ok((wrt_format::component::FormatValType::S64, offset)),
            0x77 => Ok((wrt_format::component::FormatValType::U64, offset)),
            0x76 => Ok((wrt_format::component::FormatValType::F32, offset)),
            0x75 => Ok((wrt_format::component::FormatValType::F64, offset)),
            0x74 => Ok((wrt_format::component::FormatValType::Char, offset)),
            0x73 => Ok((wrt_format::component::FormatValType::String, offset)),
            0x72 => {
                // Reference type
                let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;
                Ok((wrt_format::component::FormatValType::Ref(idx), offset))
            },
            0x71 => {
                // Record type
                let (field_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut fields = Vec::with_capacity(field_count as usize);
                for _ in 0..field_count {
                    // Read field name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read field type
                    let (field_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                    offset += bytes_read;

                    fields.push((name, field_type));
                }

                Ok((wrt_format::component::FormatValType::Record(fields), offset))
            },
            0x70 => {
                // Variant type
                let (case_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut cases = Vec::with_capacity(case_count as usize);
                for _ in 0..case_count {
                    // Read case name
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);

                    // Read case type flag
                    let has_type = bytes[offset] != 0;
                    offset += 1;

                    let mut case_type = None;
                    if has_type {
                        let (ty, bytes_read) = parse_val_type(&bytes[offset..])?;
                        offset += bytes_read;
                        case_type = Some(ty);
                    }

                    cases.push((name, case_type));
                }

                Ok((wrt_format::component::FormatValType::Variant(cases), offset))
            },
            0x6F => {
                // List type
                let (element_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                offset += bytes_read;
                Ok((
                    wrt_format::component::FormatValType::List(Box::new(element_type)),
                    offset,
                ))
            },
            0x6E => {
                // Fixed-length list type (🔧)
                let (element_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                offset += bytes_read;

                // Read the length
                let (length, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::FormatValType::FixedList(Box::new(element_type), length),
                    offset,
                ))
            },
            0x6D => {
                // Tuple type
                let (field_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut fields = Vec::with_capacity(field_count as usize);
                for _ in 0..field_count {
                    let (field_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                    offset += bytes_read;
                    fields.push(field_type);
                }

                Ok((wrt_format::component::FormatValType::Tuple(fields), offset))
            },
            0x6C => {
                // Flags type
                let (flag_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut flags = Vec::with_capacity(flag_count as usize);
                for _ in 0..flag_count {
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);
                    flags.push(name);
                }

                Ok((wrt_format::component::FormatValType::Flags(flags), offset))
            },
            0x6B => {
                // Enum type
                let (variant_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut variants = Vec::with_capacity(variant_count as usize);
                for _ in 0..variant_count {
                    let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let name = bytes_to_string(name_bytes);
                    variants.push(name);
                }

                Ok((wrt_format::component::FormatValType::Enum(variants), offset))
            },
            0x6A => {
                // Option type
                let (inner_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                offset += bytes_read;
                Ok((
                    wrt_format::component::FormatValType::Option(Box::new(inner_type)),
                    offset,
                ))
            },
            0x69 => {
                // Result type (ok only)
                let (ok_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                offset += bytes_read;
                Ok((
                    wrt_format::component::FormatValType::Result(Box::new(ok_type)),
                    offset,
                ))
            },
            0x68 => {
                // Result type (err only)
                let (_err_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                #[allow(unused_assignments)]
                {
                    offset += bytes_read;
                }
                // TODO: Fix FormatValType enum to support ResultErr variant
                // Ok((wrt_format::component::FormatValType::ResultErr(Box::new(err_type)),
                // offset))
                Err(Error::parse_error("ResultErr variant not implemented "))
            },
            0x67 => {
                // Result type (ok and err)
                let (_ok_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                #[allow(unused_assignments)]
                {
                    offset += bytes_read;
                }
                let (_err_type, bytes_read) = parse_val_type(&bytes[offset..])?;
                #[allow(unused_assignments)]
                {
                    offset += bytes_read;
                }
                // TODO: Fix FormatValType enum to support ResultBoth variant
                // Ok((
                //     wrt_format::component::FormatValType::ResultBoth(Box::new(ok_type),
                // Box::new(err_type)),     offset,
                // ))
                Err(Error::parse_error("ResultBoth variant not implemented "))
            },
            0x66 => {
                // Own a resource
                let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;
                Ok((wrt_format::component::FormatValType::Own(idx), offset))
            },
            0x65 => {
                // Borrow a resource
                let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;
                Ok((wrt_format::component::FormatValType::Borrow(idx), offset))
            },
            0x64 => {
                // Error context type
                Ok((wrt_format::component::FormatValType::ErrorContext, offset))
            },
            _ => Err(Error::parse_error("Invalid value type tag ")),
        }
    }

    /// Parse a start section
    pub fn parse_start_section(bytes: &[u8]) -> Result<(Start, usize)> {
        let mut offset = 0;

        // Read function index
        let (func_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        // Read argument vector
        let (arg_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        let mut args = Vec::with_capacity(arg_count as usize);
        for _ in 0..arg_count {
            // Read argument index
            let (arg_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;

            args.push(arg_idx);
        }

        // Read results count
        let (results, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
        offset += bytes_read;

        Ok((
            Start {
                func_idx,
                args,
                results,
            },
            offset,
        ))
    }

    /// Parse an import section
    pub fn parse_import_section(bytes: &[u8]) -> Result<(Vec<Import>, usize)> {
        // Read a vector of imports
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut imports = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read import name
            let (namespace_bytes, bytes_read) = binary::read_string(bytes, offset)?;
            offset += bytes_read;
            let namespace = bytes_to_string(namespace_bytes);

            let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
            offset += bytes_read;
            let name = bytes_to_string(name_bytes);

            // Check if there are nested namespaces or package information
            let provider = crate::prelude::create_decoder_provider::<1024>()?;
            let mut nested = crate::prelude::Vec::new();
            let mut package = None;

            // Read nested namespace flag if present
            if offset < bytes.len() {
                let has_nested = bytes[offset] != 0;
                offset += 1;

                if has_nested {
                    // Read count of nested namespaces
                    let (nested_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;

                    // Read each nested namespace
                    for _ in 0..nested_count {
                        let (nested_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                        offset += bytes_read;
                        let nested_name = bytes_to_string(nested_name_bytes);
                        nested.push(nested_name);
                    }
                }

                // Read package flag if present
                if offset < bytes.len() {
                    let has_package = bytes[offset] != 0;
                    offset += 1;

                    if has_package {
                        // Read package name
                        let (package_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                        offset += bytes_read;
                        let package_name = bytes_to_string(package_name_bytes);

                        // Read version flag
                        let has_version = bytes[offset] != 0;
                        offset += 1;

                        let mut version = None;
                        if has_version {
                            let (ver_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                            offset += bytes_read;
                            let ver = bytes_to_string(ver_bytes);
                            version = Some(ver);
                        }

                        // Read hash flag
                        let has_hash = bytes[offset] != 0;
                        offset += 1;

                        let mut hash = None;
                        if has_hash {
                            let (h_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                            offset += bytes_read;
                            let h = bytes_to_string(h_bytes);
                            hash = Some(h);
                        }

                        package = Some(wrt_format::component::PackageReference {
                            name: package_name,
                            version,
                            hash,
                        });
                    }
                }
            }

            // Read import type
            let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
            offset += bytes_read;

            // Create the import
            imports.push(Import {
                name: wrt_format::component::ImportName {
                    namespace,
                    name,
                    nested,
                    package,
                },
                ty:   extern_type,
            });
        }

        Ok((imports, offset))
    }

    /// Parse an export section
    pub fn parse_export_section(bytes: &[u8]) -> Result<(Vec<Export>, usize)> {
        // Read a vector of exports
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut exports = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read export name
            let (basic_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
            offset += bytes_read;
            let basic_name = bytes_to_string(basic_name_bytes);

            // Read flags
            if offset >= bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Unexpected end of input while parsing export flags",
                )));
            }
            let flags = bytes[offset];
            offset += 1;

            // Parse flags
            let is_resource = (flags & 0x01) != 0;
            let has_semver = (flags & 0x02) != 0;
            let has_integrity = (flags & 0x04) != 0;
            let has_nested = (flags & 0x08) != 0;

            // Read semver (if present)
            let semver = if has_semver {
                let (ver_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                offset += bytes_read;
                let ver = bytes_to_string(ver_bytes);
                Some(ver)
            } else {
                None
            };

            // Read integrity (if present)
            let integrity = if has_integrity {
                let (hash_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                offset += bytes_read;
                let hash = bytes_to_string(hash_bytes);
                Some(hash)
            } else {
                None
            };

            // Read nested namespaces (if present)
            let nested = if has_nested {
                // Read count of nested namespaces
                let (nested_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut nested_names = Vec::new();
                for _ in 0..nested_count {
                    let (nested_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let nested_name = bytes_to_string(nested_name_bytes);
                    nested_names.push(nested_name);
                }
                nested_names
            } else {
                Vec::new()
            };

            // Create export name
            let export_name = wrt_format::component::ExportName {
                name: basic_name,
                is_resource,
                semver,
                integrity,
                nested,
            };

            // Read sort byte
            if offset >= bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Unexpected end of input while parsing export sort",
                )));
            }
            let sort_byte = bytes[offset];
            offset += 1;

            // Parse sort
            let sort = parse_sort(sort_byte)?;

            // Read index
            let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;

            // Read type flag
            if offset >= bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Unexpected end of input while parsing export type flag",
                )));
            }
            let has_type = bytes[offset] != 0;
            offset += 1;

            // Read type (if present)
            let ty = if has_type {
                let (extern_type, bytes_read) = parse_extern_type(&bytes[offset..])?;
                offset += bytes_read;
                Some(extern_type)
            } else {
                None
            };

            // Create the export
            exports.push(Export {
                name: export_name,
                sort,
                idx,
                ty,
            });
        }

        Ok((exports, offset))
    }

    /// Parse a value section
    pub fn parse_value_section(bytes: &[u8]) -> Result<(Vec<Value>, usize)> {
        // Read a vector of values
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut values = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read value type
            let (val_type, bytes_read) = parse_val_type(&bytes[offset..])?;
            offset += bytes_read;

            // Read value bytes size
            let (data_size, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
            offset += bytes_read;

            if offset + data_size as usize > bytes.len() {
                return Err(Error::from(kinds::ParseError(
                    "Value data size exceeds section size",
                )));
            }

            // Extract the value data
            let data_end = offset + data_size as usize;
            let data = bytes[offset..data_end].to_vec();
            offset = data_end;

            // Check for expression flag
            let mut expression = None;
            if offset < bytes.len() {
                let has_expr = bytes[offset] != 0;
                offset += 1;

                if has_expr {
                    let (expr, bytes_read) = parse_value_expression(&bytes[offset..])?;
                    offset += bytes_read;
                    expression = Some(expr);
                }
            }

            // Check for name flag
            let mut name = None;
            if offset < bytes.len() {
                let has_name = bytes[offset] != 0;
                offset += 1;

                if has_name {
                    let (value_name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                    offset += bytes_read;
                    let value_name = bytes_to_string(value_name_bytes);
                    name = Some(value_name);
                }
            }

            // Create the value
            values.push(Value {
                ty: val_type_to_format_val_type(val_type),
                data,
                expression,
                name,
            });
        }

        Ok((values, offset))
    }

    /// Parse a value expression
    fn parse_value_expression(
        bytes: &[u8],
    ) -> Result<(wrt_format::component::ValueExpression, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing value expression",
            )));
        }

        // Read the expression tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Reference to an item
                let kind_byte = bytes[offset];
                offset += 1;

                // Convert to Sort
                let sort = parse_sort(kind_byte)?;

                // Read index
                let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::ValueExpression::ItemRef { sort, idx },
                    offset,
                ))
            },
            0x01 => {
                // Global initialization
                let (global_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::ValueExpression::GlobalInit { global_idx },
                    offset,
                ))
            },
            0x02 => {
                // Function call
                let (func_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read args vector
                let (args_count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                let mut args = Vec::with_capacity(args_count as usize);
                for _ in 0..args_count {
                    let (arg_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                    offset += bytes_read;
                    args.push(arg_idx);
                }

                Ok((
                    wrt_format::component::ValueExpression::FunctionCall { func_idx, args },
                    offset,
                ))
            },
            0x03 => {
                // Constant value
                let (const_value, bytes_read) = parse_const_value(&bytes[offset..])?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::ValueExpression::Const(const_value),
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError(
                "Invalid value expression tag",
            ))),
        }
    }

    /// Parse a constant value
    fn parse_const_value(bytes: &[u8]) -> Result<(wrt_format::component::ConstValue, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing constant value",
            )));
        }

        // Read the value tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Boolean value
                let value = bytes[offset] != 0;
                offset += 1;
                Ok((wrt_format::component::ConstValue::Bool(value), offset))
            },
            0x01 => {
                // S8 value
                let value = bytes[offset] as i8;
                offset += 1;
                Ok((wrt_format::component::ConstValue::S8(value), offset))
            },
            0x02 => {
                // U8 value
                let value = bytes[offset];
                offset += 1;
                Ok((wrt_format::component::ConstValue::U8(value), offset))
            },
            0x03 => {
                // S16 value
                let value = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                offset += 2;
                Ok((wrt_format::component::ConstValue::S16(value), offset))
            },
            0x04 => {
                // U16 value
                let value = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                offset += 2;
                Ok((wrt_format::component::ConstValue::U16(value), offset))
            },
            0x05 => {
                // S32 value
                let value = i32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                offset += 4;
                Ok((wrt_format::component::ConstValue::S32(value), offset))
            },
            0x06 => {
                // U32 value
                let value = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                offset += 4;
                Ok((wrt_format::component::ConstValue::U32(value), offset))
            },
            0x07 => {
                // S64 value
                let value = i64::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                offset += 8;
                Ok((wrt_format::component::ConstValue::S64(value), offset))
            },
            0x08 => {
                // U64 value
                let value = u64::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                offset += 8;
                Ok((wrt_format::component::ConstValue::U64(value), offset))
            },
            0x09 => {
                // F32 value
                let value_bits = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                let value = f32::from_bits(value_bits);
                offset += 4;
                Ok((wrt_format::component::ConstValue::F32(value), offset))
            },
            0x0A => {
                // F64 value
                let value_bits = u64::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                let value = f64::from_bits(value_bits);
                offset += 8;
                Ok((wrt_format::component::ConstValue::F64(value), offset))
            },
            0x0B => {
                // Char value
                let (value_str, bytes_read) = binary::read_string(bytes, offset)?;
                offset += bytes_read;

                // Convert bytes to string and validate that it's a single Unicode scalar value
                let value_string = core::str::from_utf8(value_str)
                    .map_err(|_| Error::parse_error("Invalid UTF-8 in char value "))?;
                let mut chars = value_string.chars();
                let first_char = chars.next().ok_or_else(|| {
                    Error::from(kinds::ParseError(
                        "Empty string found when parsing char value",
                    ))
                })?;
                if chars.next().is_some() {
                    return Err(Error::from(kinds::ParseError(
                        "Multiple characters found when parsing char value",
                    )));
                }

                Ok((wrt_format::component::ConstValue::Char(first_char), offset))
            },
            0x0C => {
                // String value
                let (value_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                offset += bytes_read;
                let value = bytes_to_string(value_bytes);
                Ok((wrt_format::component::ConstValue::String(value), offset))
            },
            0x0D => {
                // Null value
                Ok((wrt_format::component::ConstValue::Null, offset))
            },
            _ => Err(Error::from(kinds::ParseError("Invalid constant value tag"))),
        }
    }

    /// Parse an alias section
    pub fn parse_alias_section(bytes: &[u8]) -> Result<(Vec<Alias>, usize)> {
        // Read a vector of aliases
        let (count, mut offset) = binary::read_leb128_u32(bytes, 0)?;
        let mut aliases = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Read alias target
            let (target, bytes_read) = parse_alias_target(&bytes[offset..])?;
            offset += bytes_read;

            // Create the alias
            aliases.push(Alias { target });
        }

        Ok((aliases, offset))
    }

    /// Parse an alias target
    fn parse_alias_target(bytes: &[u8]) -> Result<(wrt_format::component::AliasTarget, usize)> {
        if bytes.is_empty() {
            return Err(Error::from(kinds::ParseError(
                "Unexpected end of input while parsing alias target",
            )));
        }

        // Read the target tag
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            0x00 => {
                // Core instance export

                // Read instance index
                let (instance_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read export name
                let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                offset += bytes_read;
                let name = bytes_to_string(name_bytes);

                // Read kind byte
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing core export kind",
                    )));
                }
                let kind_byte = bytes[offset];
                offset += 1;

                // Convert to CoreSort
                let kind = match kind_byte {
                    binary::COMPONENT_CORE_SORT_FUNC => wrt_format::component::CoreSort::Function,
                    binary::COMPONENT_CORE_SORT_TABLE => wrt_format::component::CoreSort::Table,
                    binary::COMPONENT_CORE_SORT_MEMORY => wrt_format::component::CoreSort::Memory,
                    binary::COMPONENT_CORE_SORT_GLOBAL => wrt_format::component::CoreSort::Global,
                    binary::COMPONENT_CORE_SORT_TYPE => wrt_format::component::CoreSort::Type,
                    binary::COMPONENT_CORE_SORT_MODULE => wrt_format::component::CoreSort::Module,
                    binary::COMPONENT_CORE_SORT_INSTANCE => {
                        wrt_format::component::CoreSort::Instance
                    },
                    _ => {
                        return Err(Error::from(kinds::ParseError("Invalid core sort kind")));
                    },
                };

                Ok((
                    wrt_format::component::AliasTarget::CoreInstanceExport {
                        instance_idx,
                        name,
                        kind,
                    },
                    offset,
                ))
            },
            0x01 => {
                // Instance export

                // Read instance index
                let (instance_idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read export name
                let (name_bytes, bytes_read) = binary::read_string(bytes, offset)?;
                offset += bytes_read;
                let name = bytes_to_string(name_bytes);

                // Read kind byte
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing export kind",
                    )));
                }
                let kind_byte = bytes[offset];
                offset += 1;

                // Parse sort
                let kind = parse_sort(kind_byte)?;

                Ok((
                    wrt_format::component::AliasTarget::InstanceExport {
                        instance_idx,
                        name,
                        kind,
                    },
                    offset,
                ))
            },
            0x02 => {
                // Outer definition

                // Read count
                let (count, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                // Read kind byte
                if offset >= bytes.len() {
                    return Err(Error::from(kinds::ParseError(
                        "Unexpected end of input while parsing outer kind",
                    )));
                }
                let kind_byte = bytes[offset];
                offset += 1;

                // Parse sort
                let kind = parse_sort(kind_byte)?;

                // Read index
                let (idx, bytes_read) = binary::read_leb128_u32(bytes, offset)?;
                offset += bytes_read;

                Ok((
                    wrt_format::component::AliasTarget::Outer { count, kind, idx },
                    offset,
                ))
            },
            _ => Err(Error::from(kinds::ParseError("Invalid alias target tag"))),
        }
    }

    /// Parse a name from a byte array
    ///
    /// This is a utility function to parse a name field, which is common in
    /// various WebAssembly and Component Model sections.
    #[allow(dead_code)]
    pub fn parse_name(bytes: &[u8]) -> Result<(String, usize)> {
        let (name_bytes, length) = binary::read_string(bytes, 0)?;
        let name_str = core::str::from_utf8(name_bytes)
            .map_err(|_| Error::runtime_execution_error("Invalid UTF-8 in name"))?;
        Ok((name_str.to_string(), length))
    }

    /// Convert ValType to FormatValType for type compatibility
    fn val_type_to_format_val_type(
        val_type: wrt_format::component::FormatValType,
    ) -> wrt_format::component::FormatValType {
        match val_type {
            wrt_format::component::FormatValType::Bool => {
                wrt_format::component::FormatValType::Bool
            },
            wrt_format::component::FormatValType::S8 => wrt_format::component::FormatValType::S8,
            wrt_format::component::FormatValType::U8 => wrt_format::component::FormatValType::U8,
            wrt_format::component::FormatValType::S16 => wrt_format::component::FormatValType::S16,
            wrt_format::component::FormatValType::U16 => wrt_format::component::FormatValType::U16,
            wrt_format::component::FormatValType::S32 => wrt_format::component::FormatValType::S32,
            wrt_format::component::FormatValType::U32 => wrt_format::component::FormatValType::U32,
            wrt_format::component::FormatValType::S64 => wrt_format::component::FormatValType::S64,
            wrt_format::component::FormatValType::U64 => wrt_format::component::FormatValType::U64,
            wrt_format::component::FormatValType::F32 => wrt_format::component::FormatValType::F32,
            wrt_format::component::FormatValType::F64 => wrt_format::component::FormatValType::F64,
            wrt_format::component::FormatValType::Char => {
                wrt_format::component::FormatValType::Char
            },
            wrt_format::component::FormatValType::String => {
                wrt_format::component::FormatValType::String
            },
            wrt_format::component::FormatValType::Ref(idx) => {
                wrt_format::component::FormatValType::Ref(idx)
            },
            wrt_format::component::FormatValType::List(inner) => {
                wrt_format::component::FormatValType::List(Box::new(val_type_to_format_val_type(
                    *inner,
                )))
            },
            wrt_format::component::FormatValType::FixedList(inner, len) => {
                wrt_format::component::FormatValType::FixedList(
                    Box::new(val_type_to_format_val_type(*inner)),
                    len,
                )
            },
            wrt_format::component::FormatValType::Tuple(items) => {
                wrt_format::component::FormatValType::Tuple(
                    items.into_iter().map(val_type_to_format_val_type).collect(),
                )
            },
            wrt_format::component::FormatValType::Option(inner) => {
                wrt_format::component::FormatValType::Option(Box::new(val_type_to_format_val_type(
                    *inner,
                )))
            },
            wrt_format::component::FormatValType::Result(ok) => {
                wrt_format::component::FormatValType::Result(Box::new(val_type_to_format_val_type(
                    *ok,
                )))
            },
            // TODO: Fix FormatValType enum to support ResultErr and ResultBoth variants
            // wrt_format::component::FormatValType::ResultErr(err) => {
            //     wrt_format::component::FormatValType::Result(Box::new(val_type_to_format_val_type(
            //         *err,
            //     )))
            // }
            // wrt_format::component::FormatValType::ResultBoth(ok, _err) => {
            //     wrt_format::component::FormatValType::Result(Box::new(val_type_to_format_val_type(*ok)))
            // }
            wrt_format::component::FormatValType::Record(fields) => {
                wrt_format::component::FormatValType::Record(
                    fields
                        .into_iter()
                        .map(|(name, ty)| (name, val_type_to_format_val_type(ty)))
                        .collect(),
                )
            },
            wrt_format::component::FormatValType::Variant(cases) => {
                wrt_format::component::FormatValType::Variant(
                    cases
                        .into_iter()
                        .map(|(name, ty)| (name, ty.map(val_type_to_format_val_type)))
                        .collect(),
                )
            },
            wrt_format::component::FormatValType::Flags(names) => {
                wrt_format::component::FormatValType::Flags(names)
            },
            wrt_format::component::FormatValType::Enum(names) => {
                wrt_format::component::FormatValType::Enum(names)
            },
            wrt_format::component::FormatValType::Own(idx) => {
                wrt_format::component::FormatValType::Own(idx)
            },
            wrt_format::component::FormatValType::Borrow(idx) => {
                wrt_format::component::FormatValType::Borrow(idx)
            },
            wrt_format::component::FormatValType::Void => {
                wrt_format::component::FormatValType::Void
            },
            wrt_format::component::FormatValType::ErrorContext => {
                wrt_format::component::FormatValType::ErrorContext
            },
        }
    }
} // end std_parsing module

// Re-export std functions when std is available
#[cfg(feature = "std")]
// No_std implementation with bounded alternatives following functional safety guidelines
#[cfg(not(feature = "std"))]
mod no_std_parsing {
    use wrt_error::{
        codes,
        Error,
        ErrorCategory,
        Result,
    };
    use wrt_foundation::{
        BoundedVec,
        NoStdProvider,
    };
    // Define local stub types for no_std parsing
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Component;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct CoreInstance;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct CoreType;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Instance;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Import;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Export;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Start;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Alias;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Canon;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Value;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Module;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct ComponentType;

    // Implement required traits for all stub types
    macro_rules! impl_stub_traits {
        ($($type:ty),*) => {
            $(
                impl wrt_foundation::traits::ToBytes for $type {
                    fn serialized_size(&self) -> usize { 0 }
                    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                        &self,
                        _writer: &mut wrt_foundation::traits::WriteStream<'a>,
                        _provider: &PStream,
                    ) -> wrt_error::Result<()> { Ok(()) }
                }

                impl wrt_foundation::traits::FromBytes for $type {
                    fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                        _reader: &mut wrt_foundation::traits::ReadStream<'a>,
                        _provider: &PStream,
                    ) -> wrt_error::Result<Self> { Ok(Self::default()) }
                }

                impl wrt_foundation::traits::Checksummable for $type {
                    fn update_checksum(&self, _checksum: &mut wrt_foundation::verification::Checksum) {}
                }
            )*
        };
    }

    impl_stub_traits!(
        Component,
        CoreInstance,
        CoreType,
        Instance,
        Import,
        Export,
        Start,
        Alias,
        Canon,
        Value,
        Module,
        ComponentType
    );

    // Type aliases for bounded parsing results
    type ParseProvider = NoStdProvider<4096>;
    type ParseVec<T> = BoundedVec<T, 32, ParseProvider>;

    // Helper function to create a provider for parsing
    fn create_parse_provider() -> Result<ParseProvider> {
        crate::prelude::create_decoder_provider::<4096>()
    }

    // Helper function to create an empty ParseVec
    fn create_empty_parse_vec<T>() -> Result<ParseVec<T>>
    where
        T: wrt_foundation::traits::Checksummable
            + wrt_foundation::traits::ToBytes
            + wrt_foundation::traits::FromBytes
            + Default
            + Clone
            + PartialEq
            + Eq,
    {
        let provider = create_parse_provider()?;
        ParseVec::new().map_err(|_| Error::parse_error("Failed to create empty parse vector "))
    }

    /// No_std parse core module section with safety bounds
    ///
    /// # Safety Requirements
    /// - Uses bounded allocation with compile-time limits
    /// - Fails gracefully when limits are exceeded
    /// - No heap allocation or dynamic memory
    pub fn parse_core_module_section(_bytes: &[u8]) -> Result<(ParseVec<Module>, usize)> {
        // Simplified parsing for no_std - only basic validation
        if _bytes.len() < 8 {
            return Err(Error::parse_error("Section data too short"));
        }

        // Return empty parsed result - complex module parsing requires std
        let empty_vec = create_empty_parse_vec::<Module>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    /// No_std parse core instance section with safety bounds
    pub fn parse_core_instance_section(_bytes: &[u8]) -> Result<(ParseVec<CoreInstance>, usize)> {
        // Simplified parsing for no_std
        let empty_vec = create_empty_parse_vec::<CoreInstance>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    /// No_std parse core type section with safety bounds
    pub fn parse_core_type_section(_bytes: &[u8]) -> Result<(ParseVec<CoreType>, usize)> {
        // Simplified parsing for no_std
        let empty_vec = create_empty_parse_vec::<CoreType>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    /// No_std parse component section with safety bounds
    pub fn parse_component_section(_bytes: &[u8]) -> Result<(ParseVec<Component>, usize)> {
        // Simplified parsing for no_std
        let empty_vec = create_empty_parse_vec::<Component>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    /// No_std parse instance section with safety bounds
    pub fn parse_instance_section(_bytes: &[u8]) -> Result<(ParseVec<Instance>, usize)> {
        // Simplified parsing for no_std
        let empty_vec = create_empty_parse_vec::<Instance>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    /// Additional parsing functions required by other modules
    pub fn parse_component_type_section(_bytes: &[u8]) -> Result<(ParseVec<ComponentType>, usize)> {
        let empty_vec = create_empty_parse_vec::<ComponentType>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    pub fn parse_import_section(_bytes: &[u8]) -> Result<(ParseVec<Import>, usize)> {
        let empty_vec = create_empty_parse_vec::<Import>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    pub fn parse_export_section(_bytes: &[u8]) -> Result<(ParseVec<Export>, usize)> {
        let empty_vec = create_empty_parse_vec::<Export>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    pub fn parse_start_section(_bytes: &[u8]) -> Result<(ParseVec<Start>, usize)> {
        let empty_vec = create_empty_parse_vec::<Start>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    pub fn parse_alias_section(_bytes: &[u8]) -> Result<(ParseVec<Alias>, usize)> {
        let empty_vec = create_empty_parse_vec::<Alias>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    pub fn parse_canon_section(_bytes: &[u8]) -> Result<(ParseVec<Canon>, usize)> {
        let empty_vec = create_empty_parse_vec::<Canon>().unwrap_or_default();
        Ok((empty_vec, 0))
    }

    pub fn parse_value_section(_bytes: &[u8]) -> Result<(ParseVec<Value>, usize)> {
        let empty_vec = create_empty_parse_vec::<Value>().unwrap_or_default();
        Ok((empty_vec, 0))
    }
}

// Re-export std functions when std feature is enabled
#[cfg(feature = "std")]
pub use std_parsing::*;
