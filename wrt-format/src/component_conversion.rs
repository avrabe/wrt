// Component format conversion module
//
// This module provides direct conversion between binary format types and
// runtime types without requiring intermediate conversions.

use wrt_error::Result;
use wrt_foundation::ValueType;

// Import the properly re-exported ValType
use crate::component::FormatValType;

// Create a wrapper type to avoid orphan rule violations - fix generic parameter
#[cfg(any(feature = "alloc", feature = "std"))]
pub struct ValTypeWrapper(pub FormatValType);

#[cfg(not(any(feature = "alloc", feature = "std")))]
pub struct ValTypeWrapper<P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq = wrt_foundation::NoStdProvider<1024>>(
    pub FormatValType<P>,
);

// Implement a conversion function from FormatValType to ValueType
#[cfg(any(feature = "alloc", feature = "std"))]
pub fn format_val_type_to_value_type(format_type: &FormatValType) -> Result<ValueType> {
    match format_type {
        FormatValType::S8
        | FormatValType::U8
        | FormatValType::S16
        | FormatValType::U16
        | FormatValType::S32
        | FormatValType::U32
        | FormatValType::Bool
        | FormatValType::Char
        | FormatValType::Flags(_)
        | FormatValType::Enum(_)
        | FormatValType::ErrorContext => Ok(ValueType::I32),

        FormatValType::S64 | FormatValType::U64 => Ok(ValueType::I64),

        FormatValType::F32 => Ok(ValueType::F32),
        FormatValType::F64 => Ok(ValueType::F64),

        // References and handles
        FormatValType::String
        | FormatValType::Record(_)
        | FormatValType::Variant(_)
        | FormatValType::List(_)
        | FormatValType::FixedList(_, _)
        | FormatValType::Tuple(_)
        | FormatValType::Option(_)
        | FormatValType::Result(_)
        | FormatValType::Own(_)
        | FormatValType::Borrow(_)
        | FormatValType::Ref(_) => Ok(ValueType::ExternRef),

        FormatValType::Void => Ok(ValueType::I32), // Map Void to I32 as a fallback
    }
}

#[cfg(not(any(feature = "alloc", feature = "std")))]
pub fn format_val_type_to_value_type<
    P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq,
>(
    format_type: &FormatValType<P>,
) -> Result<ValueType> {
    match format_type {
        FormatValType::S8
        | FormatValType::U8
        | FormatValType::S16
        | FormatValType::U16
        | FormatValType::S32
        | FormatValType::U32
        | FormatValType::Bool
        | FormatValType::Char
        | FormatValType::Flags(_)
        | FormatValType::Enum(_)
        | FormatValType::ErrorContext => Ok(ValueType::I32),

        FormatValType::S64 | FormatValType::U64 => Ok(ValueType::I64),

        FormatValType::F32 => Ok(ValueType::F32),
        FormatValType::F64 => Ok(ValueType::F64),

        // References and handles
        FormatValType::String
        | FormatValType::Record(_)
        | FormatValType::Variant(_)
        | FormatValType::List(_)
        | FormatValType::FixedList(_, _)
        | FormatValType::Tuple(_)
        | FormatValType::Option(_)
        | FormatValType::Result(_)
        | FormatValType::Own(_)
        | FormatValType::Borrow(_)
        | FormatValType::Ref(_) => Ok(ValueType::ExternRef),

        FormatValType::Void => Ok(ValueType::I32), // Map Void to I32 as a fallback
    }
}

// Implement a conversion function from ValueType to FormatValType
#[cfg(any(feature = "alloc", feature = "std"))]
pub fn value_type_to_format_val_type(value_type: &ValueType) -> Result<FormatValType> {
    match value_type {
        ValueType::I32 => Ok(FormatValType::S32),
        ValueType::I64 => Ok(FormatValType::S64),
        ValueType::F32 => Ok(FormatValType::F32),
        ValueType::F64 => Ok(FormatValType::F64),
        ValueType::V128 => unimplemented!("V128 to FormatValType mapping is not yet defined"),
        ValueType::I16x8 => unimplemented!("I16x8 to FormatValType mapping is not yet defined"),
        ValueType::FuncRef => Ok(FormatValType::Own(0)), // Map to handle
        ValueType::ExternRef => Ok(FormatValType::Own(0)), // Map to handle
        ValueType::StructRef(_) => Ok(FormatValType::Own(0)), // Map struct reference to handle
        ValueType::ArrayRef(_) => Ok(FormatValType::Own(0)), // Map array reference to handle
    }
}

#[cfg(not(any(feature = "alloc", feature = "std")))]
pub fn value_type_to_format_val_type<
    P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq,
>(
    value_type: &ValueType,
) -> Result<FormatValType<P>> {
    match value_type {
        ValueType::I32 => Ok(FormatValType::S32),
        ValueType::I64 => Ok(FormatValType::S64),
        ValueType::F32 => Ok(FormatValType::F32),
        ValueType::F64 => Ok(FormatValType::F64),
        ValueType::V128 => unimplemented!("V128 to FormatValType mapping is not yet defined"),
        ValueType::I16x8 => unimplemented!("I16x8 to FormatValType mapping is not yet defined"),
        ValueType::FuncRef => Ok(FormatValType::Own(0)), // Map to handle
        ValueType::ExternRef => Ok(FormatValType::Own(0)), // Map to handle
        ValueType::StructRef(_) => Ok(FormatValType::Own(0)), // Map struct reference to handle
        ValueType::ArrayRef(_) => Ok(FormatValType::Own(0)), // Map array reference to handle
    }
}

// Map a core WebAssembly ValueType to a Component Model ValType
#[cfg(any(feature = "alloc", feature = "std"))]
pub fn map_wasm_type_to_component(ty: ValueType) -> FormatValType {
    match ty {
        ValueType::I32 => FormatValType::S32,
        ValueType::I64 => FormatValType::S64,
        ValueType::F32 => FormatValType::F32,
        ValueType::F64 => FormatValType::F64,
        ValueType::V128 => unimplemented!("V128 to FormatValType mapping is not yet defined"),
        ValueType::I16x8 => unimplemented!("I16x8 to FormatValType mapping is not yet defined"),
        ValueType::FuncRef => FormatValType::Own(0), // Map to handle
        ValueType::ExternRef => FormatValType::Own(0), // Map to handle
        ValueType::StructRef(_) => FormatValType::Own(0), // Map struct reference to handle
        ValueType::ArrayRef(_) => FormatValType::Own(0), // Map array reference to handle
    }
}

#[cfg(not(any(feature = "alloc", feature = "std")))]
pub fn map_wasm_type_to_component<
    P: wrt_foundation::MemoryProvider + Default + Clone + PartialEq + Eq,
>(
    ty: ValueType,
) -> FormatValType<P> {
    match ty {
        ValueType::I32 => FormatValType::S32,
        ValueType::I64 => FormatValType::S64,
        ValueType::F32 => FormatValType::F32,
        ValueType::F64 => FormatValType::F64,
        ValueType::V128 => unimplemented!("V128 to FormatValType mapping is not yet defined"),
        ValueType::I16x8 => unimplemented!("I16x8 to FormatValType mapping is not yet defined"),
        ValueType::FuncRef => FormatValType::Own(0), // Map to handle
        ValueType::ExternRef => FormatValType::Own(0), // Map to handle
        ValueType::StructRef(_) => FormatValType::Own(0), // Map struct reference to handle
        ValueType::ArrayRef(_) => FormatValType::Own(0), // Map array reference to handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_conversion() {
        type TestProvider = wrt_foundation::traits::DefaultMemoryProvider;

        // Test basic primitive types
        let s32_val = FormatValType::S32;
        let i32_val = format_val_type_to_value_type(&s32_val).unwrap();
        assert_eq!(i32_val, ValueType::I32);

        let f64_val = FormatValType::F64;
        let f64_runtime = format_val_type_to_value_type(&f64_val).unwrap();
        assert_eq!(f64_runtime, ValueType::F64);

        // Test complex types (all map to ExternRef)
        let string_val = FormatValType::String;
        let string_runtime = format_val_type_to_value_type(&string_val).unwrap();
        assert_eq!(string_runtime, ValueType::ExternRef);

        // Test roundtrip conversion for basic types
        let i32_val = ValueType::I32;
        let format_val = value_type_to_format_val_type(&i32_val).unwrap();
        let roundtrip = format_val_type_to_value_type(&format_val).unwrap();
        assert_eq!(i32_val, roundtrip);
    }
}
