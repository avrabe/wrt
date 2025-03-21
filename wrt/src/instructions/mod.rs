//! WebAssembly instruction implementations
//!
//! This module contains implementations for all WebAssembly instructions,
//! organized into submodules by instruction category.

use crate::types::ValueType;
use crate::Vec;

pub mod arithmetic;
pub mod bit_counting;
pub mod comparison;
pub mod control;
pub mod conversion;
pub mod memory;
pub mod numeric_constants;
pub mod parametric;
pub mod simd;
pub mod table;
pub mod variable;

pub use arithmetic::*;
pub use bit_counting::*;
pub use comparison::*;
pub use control::*;
pub use conversion::*;
pub use memory::*;
pub use numeric_constants::*;
pub use parametric::*;
pub use simd::*;
pub use table::*;
pub use variable::*;

/// Represents a WebAssembly instruction
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Control flow instructions
    Block(BlockType),
    Loop(BlockType),
    If(BlockType),
    Else,
    End,
    Br(u32),
    BrIf(u32),
    BrTable(Vec<u32>, u32),
    Return,
    Unreachable,
    Nop,

    // Call instructions
    Call(u32),
    CallIndirect(u32, u32),
    ReturnCall(u32),
    ReturnCallIndirect(u32, u32),

    // Parametric instructions
    Drop,
    Select,
    SelectTyped(ValueType),

    // Variable instructions
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    // Table instructions
    TableGet(u32),
    TableSet(u32),
    TableSize(u32),
    TableGrow(u32),
    TableInit(u32, u32),
    TableCopy(u32, u32),
    TableFill(u32),
    ElemDrop(u32),

    // Memory instructions
    I32Load(u32, u32),
    I64Load(u32, u32),
    F32Load(u32, u32),
    F64Load(u32, u32),
    I32Load8S(u32, u32),
    I32Load8U(u32, u32),
    I32Load16S(u32, u32),
    I32Load16U(u32, u32),
    I64Load8S(u32, u32),
    I64Load8U(u32, u32),
    I64Load16S(u32, u32),
    I64Load16U(u32, u32),
    I64Load32S(u32, u32),
    I64Load32U(u32, u32),
    I32Store(u32, u32),
    I64Store(u32, u32),
    F32Store(u32, u32),
    F64Store(u32, u32),
    I32Store8(u32, u32),
    I32Store16(u32, u32),
    I64Store8(u32, u32),
    I64Store16(u32, u32),
    I64Store32(u32, u32),
    MemorySize,
    MemoryGrow,
    MemoryInit(u32),
    DataDrop(u32),
    MemoryCopy,
    MemoryFill,

    // Numeric constant instructions
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),

    // Comparison instructions
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,

    // Arithmetic instructions
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,

    // Conversion instructions
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,

    // SIMD - v128 manipulation
    V128Load(u32, u32),
    V128Store(u32, u32),
    V128Const([u8; 16]),

    // SIMD - Basic operations
    I8x16Shuffle([u8; 16]),
    I8x16Swizzle,

    // SIMD - Lane-wise operations
    I8x16ExtractLaneS(u8),
    I8x16ExtractLaneU(u8),
    I8x16ReplaceLane(u8),
    I16x8ExtractLaneS(u8),
    I16x8ExtractLaneU(u8),
    I16x8ReplaceLane(u8),
    I32x4ExtractLane(u8),
    I32x4ReplaceLane(u8),
    I64x2ExtractLane(u8),
    I64x2ReplaceLane(u8),
    F32x4ExtractLane(u8),
    F32x4ReplaceLane(u8),
    F64x2ExtractLane(u8),
    F64x2ReplaceLane(u8),

    // SIMD - Splat operations
    I8x16Splat,
    I16x8Splat,
    I32x4Splat,
    I64x2Splat,
    F32x4Splat,
    F64x2Splat,

    // SIMD - Comparison operations
    I8x16Eq,
    I8x16Ne,
    I8x16LtS,
    I8x16LtU,
    I8x16GtS,
    I8x16GtU,
    I8x16LeS,
    I8x16LeU,
    I8x16GeS,
    I8x16GeU,

    I16x8Eq,
    I16x8Ne,
    I16x8LtS,
    I16x8LtU,
    I16x8GtS,
    I16x8GtU,
    I16x8LeS,
    I16x8LeU,
    I16x8GeS,
    I16x8GeU,

    I32x4Eq,
    I32x4Ne,
    I32x4LtS,
    I32x4LtU,
    I32x4GtS,
    I32x4GtU,
    I32x4LeS,
    I32x4LeU,
    I32x4GeS,
    I32x4GeU,

    I64x2Eq,
    I64x2Ne,
    I64x2LtS,
    I64x2GtS,
    I64x2LeS,
    I64x2GeS,

    F32x4Eq,
    F32x4Ne,
    F32x4Lt,
    F32x4Gt,
    F32x4Le,
    F32x4Ge,

    F64x2Eq,
    F64x2Ne,
    F64x2Lt,
    F64x2Gt,
    F64x2Le,
    F64x2Ge,

    // SIMD - Arithmetic operations
    I8x16Neg,
    I8x16Add,
    I8x16AddSaturateS,
    I8x16AddSaturateU,
    I8x16Sub,
    I8x16SubSaturateS,
    I8x16SubSaturateU,

    I16x8Neg,
    I16x8Add,
    I16x8AddSaturateS,
    I16x8AddSaturateU,
    I16x8Sub,
    I16x8SubSaturateS,
    I16x8SubSaturateU,
    I16x8Mul,

    I32x4Neg,
    I32x4Add,
    I32x4Sub,
    I32x4Mul,
    I32x4DotI16x8S,

    I64x2Neg,
    I64x2Add,
    I64x2Sub,
    I64x2Mul,

    F32x4Abs,
    F32x4Neg,
    F32x4Sqrt,
    F32x4Add,
    F32x4Sub,
    F32x4Mul,
    F32x4Div,
    F32x4Min,
    F32x4Max,

    F64x2Abs,
    F64x2Neg,
    F64x2Sqrt,
    F64x2Add,
    F64x2Sub,
    F64x2Mul,
    F64x2Div,
    F64x2Min,
    F64x2Max,

    // SIMD - Bitwise operations
    V128Not,
    V128And,
    V128AndNot,
    V128Or,
    V128Xor,
    V128Bitselect,

    // SIMD - Conversion operations
    I32x4TruncSatF32x4S,
    I32x4TruncSatF32x4U,
    F32x4ConvertI32x4S,
    F32x4ConvertI32x4U,

    // Relaxed SIMD operations
    #[cfg(feature = "relaxed_simd")]
    F32x4RelaxedMin,
    #[cfg(feature = "relaxed_simd")]
    F32x4RelaxedMax,
    #[cfg(feature = "relaxed_simd")]
    F64x2RelaxedMin,
    #[cfg(feature = "relaxed_simd")]
    F64x2RelaxedMax,
    #[cfg(feature = "relaxed_simd")]
    I16x8RelaxedQ15MulrS,
    #[cfg(feature = "relaxed_simd")]
    I16x8RelaxedDotI8x16I7x16S,
    #[cfg(feature = "relaxed_simd")]
    I32x4RelaxedDotI8x16I7x16AddS,
    #[cfg(feature = "relaxed_simd")]
    I8x16RelaxedSwizzle,
    #[cfg(feature = "relaxed_simd")]
    I32x4RelaxedTruncSatF32x4S,
    #[cfg(feature = "relaxed_simd")]
    I32x4RelaxedTruncSatF32x4U,
    #[cfg(feature = "relaxed_simd")]
    I32x4RelaxedTruncSatF64x2SZero,
    #[cfg(feature = "relaxed_simd")]
    I32x4RelaxedTruncSatF64x2UZero,
}

/// Block type for control flow instructions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    /// Empty block type (no parameters or results)
    Empty,
    /// Single value type
    Type(ValueType),
    /// Type index into the module's type section
    TypeIndex(u32),
}
