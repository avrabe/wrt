//! WebAssembly instruction type definition

use crate::types::{BlockType, ValueType};
use std::fmt::{Display, Formatter};

/// Represents a WebAssembly instruction
#[derive(Clone, Debug)]
pub enum Instruction {
    // Control flow instructions
    /// Block instruction: begins a block of code with a given signature
    Block(BlockType),
    /// Loop instruction: begins a loop with a given signature
    Loop(BlockType),
    /// If instruction: begins an if block with a given signature
    If(BlockType),
    /// Else instruction: marks the beginning of the else branch of an if block
    Else,
    /// End instruction: marks the end of a block, loop, if, or function
    End,
    /// Branch instruction: jumps to the specified label depth
    Br(u32),
    /// Conditional branch instruction: jumps to the specified label depth if the condition is true
    BrIf(u32),
    /// Table branch instruction: jumps to the label selected by an index from a table of label targets
    BrTable(Vec<u32>, u32),
    /// Return instruction: returns from the current function
    Return,
    /// Unreachable instruction: indicates an unreachable code path
    Unreachable,
    /// No-operation instruction: does nothing
    Nop,

    // Call instructions
    /// Call instruction: directly calls a function by its index
    Call(u32),
    /// Indirect call instruction: calls a function from a table at the given table index and type
    CallIndirect(u32, u32),
    /// Tail call instruction: calls a function and returns directly to the caller
    ReturnCall(u32),
    /// Indirect tail call instruction: calls a function from a table and returns directly to the caller
    ReturnCallIndirect(u32, u32),

    // Parametric instructions
    /// Drop instruction: pops and discards the top value from the stack
    Drop,
    /// Select instruction: selects one of two values based on a condition
    Select,
    /// Typed select instruction: selects one of two values based on a condition, with explicit type
    SelectTyped(ValueType),

    // Variable instructions
    /// Local get instruction: pushes the value of a local variable onto the stack
    LocalGet(u32),
    /// Local set instruction: sets the value of a local variable from the stack
    LocalSet(u32),
    /// Local tee instruction: sets a local variable and keeps the value on the stack
    LocalTee(u32),
    /// Global get instruction: pushes the value of a global variable onto the stack
    GlobalGet(u32),
    /// Global set instruction: sets the value of a global variable from the stack
    GlobalSet(u32),

    // Table instructions
    /// Table get instruction: gets an element from a table
    TableGet(u32),
    /// Table set instruction: sets an element in a table
    TableSet(u32),
    /// Table size instruction: gets the current size of a table
    TableSize(u32),
    /// Table grow instruction: grows a table by a given number of elements
    TableGrow(u32),
    /// Table init instruction: initializes a table segment from an element segment
    TableInit(u32, u32),
    /// Table copy instruction: copies elements from one table to another
    TableCopy(u32, u32),
    /// Table fill instruction: fills a table range with a given value
    TableFill(u32),
    /// Element drop instruction: drops an element segment
    ElemDrop(u32),

    // Memory instructions
    /// Load a 32-bit integer from memory
    I32Load(u32, u32),
    /// Load a 64-bit integer from memory
    I64Load(u32, u32),
    /// Load a 32-bit float from memory
    F32Load(u32, u32),
    /// Load a 64-bit float from memory
    F64Load(u32, u32),
    /// Load an 8-bit integer from memory and sign-extend to 32 bits
    I32Load8S(u32, u32),
    /// Load an 8-bit integer from memory and zero-extend to 32 bits
    I32Load8U(u32, u32),
    /// Load a 16-bit integer from memory and sign-extend to 32 bits
    I32Load16S(u32, u32),
    /// Load a 16-bit integer from memory and zero-extend to 32 bits
    I32Load16U(u32, u32),
    /// Load an 8-bit integer from memory and sign-extend to 64 bits
    I64Load8S(u32, u32),
    /// Load an 8-bit integer from memory and zero-extend to 64 bits
    I64Load8U(u32, u32),
    /// Load a 16-bit integer from memory and sign-extend to 64 bits
    I64Load16S(u32, u32),
    /// Load a 16-bit integer from memory and zero-extend to 64 bits
    I64Load16U(u32, u32),
    /// Load a 32-bit integer from memory and sign-extend to 64 bits
    I64Load32S(u32, u32),
    /// Load a 32-bit integer from memory and zero-extend to 64 bits
    I64Load32U(u32, u32),
    /// Store a 32-bit integer to memory
    I32Store(u32, u32),
    /// Store a 64-bit integer to memory
    I64Store(u32, u32),
    /// Store a 32-bit float to memory
    F32Store(u32, u32),
    /// Store a 64-bit float to memory
    F64Store(u32, u32),
    /// Store the low 8 bits of a 32-bit integer to memory
    I32Store8(u32, u32),
    /// Store the low 16 bits of a 32-bit integer to memory
    I32Store16(u32, u32),
    /// Store the low 8 bits of a 64-bit integer to memory
    I64Store8(u32, u32),
    /// Store the low 16 bits of a 64-bit integer to memory
    I64Store16(u32, u32),
    /// Store the low 32 bits of a 64-bit integer to memory
    I64Store32(u32, u32),
    /// Get the current size of memory in pages
    MemorySize(u32),
    /// Grow memory by a given number of pages
    MemoryGrow(u32),
    /// Initialize a region of memory from a data segment
    MemoryInit(u32),
    /// Drop a data segment
    DataDrop(u32),
    /// Copy data from one memory region to another
    MemoryCopy,
    /// Fill a memory region with a given value
    MemoryFill,

    // Numeric constant instructions
    /// Push a 32-bit integer constant onto the stack
    I32Const(i32),
    /// Push a 64-bit integer constant onto the stack
    I64Const(i64),
    /// Push a 32-bit float constant onto the stack
    F32Const(f32),
    /// Push a 64-bit float constant onto the stack
    F64Const(f64),

    // Comparison instructions
    /// Test if a 32-bit integer is equal to zero
    I32Eqz,
    /// Test if two 32-bit integers are equal
    I32Eq,
    /// Test if two 32-bit integers are not equal
    I32Ne,
    /// Test if one 32-bit integer is less than another (signed)
    I32LtS,
    /// Test if one 32-bit integer is less than another (unsigned)
    I32LtU,
    /// Test if one 32-bit integer is greater than another (signed)
    I32GtS,
    /// Test if one 32-bit integer is greater than another (unsigned)
    I32GtU,
    /// Test if one 32-bit integer is less than or equal to another (signed)
    I32LeS,
    /// Test if one 32-bit integer is less than or equal to another (unsigned)
    I32LeU,
    /// Test if one 32-bit integer is greater than or equal to another (signed)
    I32GeS,
    /// Test if one 32-bit integer is greater than or equal to another (unsigned)
    I32GeU,
    /// Test if a 64-bit integer is equal to zero
    I64Eqz,
    /// Test if two 64-bit integers are equal
    I64Eq,
    /// Test if two 64-bit integers are not equal
    I64Ne,
    /// Test if one 64-bit integer is less than another (signed)
    I64LtS,
    /// Test if one 64-bit integer is less than another (unsigned)
    I64LtU,
    /// Test if one 64-bit integer is greater than another (signed)
    I64GtS,
    /// Test if one 64-bit integer is greater than another (unsigned)
    I64GtU,
    /// Test if one 64-bit integer is less than or equal to another (signed)
    I64LeS,
    /// Test if one 64-bit integer is less than or equal to another (unsigned)
    I64LeU,
    /// Test if one 64-bit integer is greater than or equal to another (signed)
    I64GeS,
    /// Test if one 64-bit integer is greater than or equal to another (unsigned)
    I64GeU,
    /// Test if two 32-bit floats are equal
    F32Eq,
    /// Test if two 32-bit floats are not equal
    F32Ne,
    /// Test if one 32-bit float is less than another
    F32Lt,
    /// Test if one 32-bit float is greater than another
    F32Gt,
    /// Test if one 32-bit float is less than or equal to another
    F32Le,
    /// Test if one 32-bit float is greater than or equal to another
    F32Ge,
    /// Test if two 64-bit floats are equal
    F64Eq,
    /// Test if two 64-bit floats are not equal
    F64Ne,
    /// Test if one 64-bit float is less than another
    F64Lt,
    /// Test if one 64-bit float is greater than another
    F64Gt,
    /// Test if one 64-bit float is less than or equal to another
    F64Le,
    /// Test if one 64-bit float is greater than or equal to another
    F64Ge,

    // Arithmetic instructions
    /// Count leading zeros in a 32-bit integer
    I32Clz,
    /// Count trailing zeros in a 32-bit integer
    I32Ctz,
    /// Count number of set bits in a 32-bit integer
    I32Popcnt,
    /// Add two 32-bit integers
    I32Add,
    /// Subtract one 32-bit integer from another
    I32Sub,
    /// Multiply two 32-bit integers
    I32Mul,
    /// Divide two 32-bit integers (signed)
    I32DivS,
    /// Divide two 32-bit integers (unsigned)
    I32DivU,
    /// Get remainder after dividing two 32-bit integers (signed)
    I32RemS,
    /// Get remainder after dividing two 32-bit integers (unsigned)
    I32RemU,
    /// Perform bitwise AND on two 32-bit integers
    I32And,
    /// Perform bitwise OR on two 32-bit integers
    I32Or,
    /// Perform bitwise XOR on two 32-bit integers
    I32Xor,
    /// Shift 32-bit integer left
    I32Shl,
    /// Shift 32-bit integer right (signed)
    I32ShrS,
    /// Shift 32-bit integer right (unsigned)
    I32ShrU,
    /// Rotate 32-bit integer left
    I32Rotl,
    /// Rotate 32-bit integer right
    I32Rotr,
    /// Count leading zeros in a 64-bit integer
    I64Clz,
    /// Count trailing zeros in a 64-bit integer
    I64Ctz,
    /// Count number of set bits in a 64-bit integer
    I64Popcnt,
    /// Add two 64-bit integers
    I64Add,
    /// Subtract one 64-bit integer from another
    I64Sub,
    /// Multiply two 64-bit integers
    I64Mul,
    /// Divide two 64-bit integers (signed)
    I64DivS,
    /// Divide two 64-bit integers (unsigned)
    I64DivU,
    /// Get remainder after dividing two 64-bit integers (signed)
    I64RemS,
    /// Get remainder after dividing two 64-bit integers (unsigned)
    I64RemU,
    /// Perform bitwise AND on two 64-bit integers
    I64And,
    /// Perform bitwise OR on two 64-bit integers
    I64Or,
    /// Perform bitwise XOR on two 64-bit integers
    I64Xor,
    /// Shift 64-bit integer left
    I64Shl,
    /// Shift 64-bit integer right (signed)
    I64ShrS,
    /// Shift 64-bit integer right (unsigned)
    I64ShrU,
    /// Rotate 64-bit integer left
    I64Rotl,
    /// Rotate 64-bit integer right
    I64Rotr,
    /// Get the absolute value of a 32-bit float
    F32Abs,
    /// Negate a 32-bit float
    F32Neg,
    /// Round a 32-bit float up to the nearest integer
    F32Ceil,
    /// Round a 32-bit float down to the nearest integer
    F32Floor,
    /// Truncate a 32-bit float to an integer
    F32Trunc,
    /// Round a 32-bit float to the nearest integer
    F32Nearest,
    /// Calculate the square root of a 32-bit float
    F32Sqrt,
    /// Add two 32-bit float values
    F32Add,
    /// Subtract 32-bit float values
    F32Sub,
    /// Multiply 32-bit float values
    F32Mul,
    /// Divide 32-bit float values
    F32Div,
    /// Get the minimum of two 32-bit float values
    F32Min,
    /// Get the maximum of two 32-bit float values
    F32Max,
    /// Copy sign from one 32-bit float to another
    F32Copysign,
    /// Get the absolute value of a 64-bit float
    F64Abs,
    /// Negate a 64-bit float
    F64Neg,
    /// Round a 64-bit float up to the nearest integer
    F64Ceil,
    /// Round a 64-bit float down to the nearest integer
    F64Floor,
    /// Truncate a 64-bit float to an integer
    F64Trunc,
    /// Round a 64-bit float to the nearest integer
    F64Nearest,
    /// Calculate the square root of a 64-bit float
    F64Sqrt,
    /// Add two 64-bit float values
    F64Add,
    /// Subtract 64-bit float values
    F64Sub,
    /// Multiply 64-bit float values
    F64Mul,
    /// Divide 64-bit float values
    F64Div,
    /// Get the minimum of two 64-bit float values
    F64Min,
    /// Get the maximum of two 64-bit float values
    F64Max,
    /// Copy sign from one 64-bit float to another
    F64Copysign,

    // Conversion instructions
    /// Wrap a 64-bit integer to a 32-bit integer
    I32WrapI64,
    /// Truncate a 32-bit float to a signed 32-bit integer
    I32TruncF32S,
    /// Truncate a 32-bit float to an unsigned 32-bit integer
    I32TruncF32U,
    /// Truncate a 64-bit float to a signed 32-bit integer
    I32TruncF64S,
    /// Truncate a 64-bit float to an unsigned 32-bit integer
    I32TruncF64U,
    /// Extend a signed 32-bit integer to a 64-bit integer
    I64ExtendI32S,
    /// Extend an unsigned 32-bit integer to a 64-bit integer
    I64ExtendI32U,
    /// Truncate a 32-bit float to a signed 64-bit integer
    I64TruncF32S,
    /// Truncate a 32-bit float to an unsigned 64-bit integer
    I64TruncF32U,
    /// Truncate a 64-bit float to a signed 64-bit integer
    I64TruncF64S,
    /// Truncate a 64-bit float to an unsigned 64-bit integer
    I64TruncF64U,
    /// Sign-extend a 8-bit integer to a 32-bit integer
    I32Extend8S,
    /// Sign-extend a 16-bit integer to a 32-bit integer
    I32Extend16S,
    /// Sign-extend a 8-bit integer to a 64-bit integer
    I64Extend8S,
    /// Sign-extend a 16-bit integer to a 64-bit integer
    I64Extend16S,
    /// Sign-extend a 32-bit integer to a 64-bit integer
    I64Extend32S,
    /// Convert a signed 32-bit integer to a 32-bit float
    F32ConvertI32S,
    /// Convert an unsigned 32-bit integer to a 32-bit float
    F32ConvertI32U,
    /// Convert a signed 64-bit integer to a 32-bit float
    F32ConvertI64S,
    /// Convert an unsigned 64-bit integer to a 32-bit float
    F32ConvertI64U,
    /// Demote a 64-bit float to a 32-bit float
    F32DemoteF64,
    /// Convert a signed 32-bit integer to a 64-bit float
    F64ConvertI32S,
    /// Convert an unsigned 32-bit integer to a 64-bit float
    F64ConvertI32U,
    /// Convert a signed 64-bit integer to a 64-bit float
    F64ConvertI64S,
    /// Convert an unsigned 64-bit integer to a 64-bit float
    F64ConvertI64U,
    /// Promote a 32-bit float to a 64-bit float
    F64PromoteF32,
    /// Reinterpret the bits of a 32-bit float as a 32-bit integer
    I32ReinterpretF32,
    /// Reinterpret the bits of a 64-bit float as a 64-bit integer
    I64ReinterpretF64,
    /// Reinterpret the bits of a 32-bit integer as a 32-bit float
    F32ReinterpretI32,
    /// Reinterpret the bits of a 64-bit integer as a 64-bit float
    F64ReinterpretI64,

    // Basic SIMD instructions (minimal set for compatibility)
    /// Create a 128-bit vector by duplicating a 32-bit float value to all lanes
    F32x4Splat,
    /// Create a 128-bit vector by duplicating a 64-bit float value to all lanes
    F64x2Splat,
    /// Load a 128-bit value from memory
    V128Load(u32, u32),
    /// Store a 128-bit value to memory
    V128Store(u32, u32),
    /// Load 8 bits into a lane of a 128-bit vector
    V128Load8Lane(u32, u32, u8),
    /// Load 16 bits into a lane of a 128-bit vector
    V128Load16Lane(u32, u32, u8),
    /// Load 32 bits into a lane of a 128-bit vector
    V128Load32Lane(u32, u32, u8),
    /// Load 64 bits into a lane of a 128-bit vector
    V128Load64Lane(u32, u32, u8),
    /// Store a lane (8 bits) of a 128-bit vector to memory
    V128Store8Lane(u32, u32, u8),
    /// Store a lane (16 bits) of a 128-bit vector to memory
    V128Store16Lane(u32, u32, u8),
    /// Store a lane (32 bits) of a 128-bit vector to memory
    V128Store32Lane(u32, u32, u8),
    /// Store a lane (64 bits) of a 128-bit vector to memory
    V128Store64Lane(u32, u32, u8),

    // Reference instructions
    /// Create a null reference of the given type
    RefNull(ValueType),
    /// Test if a reference is null
    RefIsNull,
    /// Create a reference to a function
    RefFunc(u32),

    // Non-trapping Float-to-int Conversions
    /// Truncate a 32-bit float to a signed 32-bit integer with saturation
    I32TruncSatF32S,
    /// Truncate a 32-bit float to an unsigned 32-bit integer with saturation
    I32TruncSatF32U,
    /// Truncate a 64-bit float to a signed 32-bit integer with saturation
    I32TruncSatF64S,
    /// Truncate a 64-bit float to an unsigned 32-bit integer with saturation
    I32TruncSatF64U,
    /// Truncate a 32-bit float to a signed 64-bit integer with saturation
    I64TruncSatF32S,
    /// Truncate a 32-bit float to an unsigned 64-bit integer with saturation
    I64TruncSatF32U,
    /// Truncate a 64-bit float to a signed 64-bit integer with saturation
    I64TruncSatF64S,
    /// Truncate a 64-bit float to an unsigned 64-bit integer with saturation
    I64TruncSatF64U,

    // New SIMD instructions
    I32x4ExtAddPairwiseI16x8S,
    I32x4ExtAddPairwiseI16x8U,
    /// Shuffle lanes from two 128-bit vectors
    V128Shuffle([u8; 16]),
    V128SplatI8x16,
    V128SplatI16x8,
    V128SplatI32x4,
    V128SplatI64x2,
    SimdOpAE,
    SimdOpB1,
    SimdOpB5,
    I32x4DotI16x8S,

    // SIMD instructions (prefix 0xfd)
    /// Push a 128-bit constant onto the stack
    V128Const([u8; 16]),
}

impl Instruction {
    /// Returns true if the instruction is a SIMD instruction
    #[must_use]
    pub const fn is_simd(&self) -> bool {
        match self {
            Self::V128Load(_, _)
            | Self::V128Store(_, _)
            | Self::V128Load8Lane(_, _, _)
            | Self::V128Load16Lane(_, _, _)
            | Self::V128Load32Lane(_, _, _)
            | Self::V128Load64Lane(_, _, _)
            | Self::V128Store8Lane(_, _, _)
            | Self::V128Store16Lane(_, _, _)
            | Self::V128Store32Lane(_, _, _)
            | Self::V128Store64Lane(_, _, _)
            | Self::F32x4Splat
            | Self::F64x2Splat
            | Self::I32x4ExtAddPairwiseI16x8S
            | Self::I32x4ExtAddPairwiseI16x8U
            | Self::V128Shuffle(_)
            | Self::V128SplatI8x16
            | Self::V128SplatI16x8
            | Self::V128SplatI32x4
            | Self::V128SplatI64x2
            | Self::SimdOpAE
            | Self::SimdOpB1
            | Self::SimdOpB5
            | Self::I32x4DotI16x8S => true,
            _ => false,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // Control flow instructions
            Instruction::Block(block_type) => write!(f, "block {:?}", block_type),
            Instruction::Loop(block_type) => write!(f, "loop {:?}", block_type),
            Instruction::If(block_type) => write!(f, "if {:?}", block_type),
            Instruction::Else => write!(f, "else"),
            Instruction::End => write!(f, "end"),
            Instruction::Br(label) => write!(f, "br {}", label),
            Instruction::BrIf(label) => write!(f, "br_if {}", label),
            Instruction::BrTable(labels, default) => write!(f, "br_table {:?} {}", labels, default),
            Instruction::Return => write!(f, "return"),
            Instruction::Unreachable => write!(f, "unreachable"),
            Instruction::Nop => write!(f, "nop"),

            // Call instructions
            Instruction::Call(index) => write!(f, "call {}", index),
            Instruction::CallIndirect(index, table_index) => {
                write!(f, "call_indirect {} {}", index, table_index)
            }
            Instruction::ReturnCall(index) => write!(f, "return_call {}", index),
            Instruction::ReturnCallIndirect(index, table_index) => {
                write!(f, "return_call_indirect {} {}", index, table_index)
            }

            // Parametric instructions
            Instruction::Drop => write!(f, "drop"),
            Instruction::Select => write!(f, "select"),
            Instruction::SelectTyped(value_type) => write!(f, "select_typed {}", value_type),

            // Variable instructions
            Instruction::LocalGet(index) => write!(f, "local.get {}", index),
            Instruction::LocalSet(index) => write!(f, "local.set {}", index),
            Instruction::LocalTee(index) => write!(f, "local.tee {}", index),
            Instruction::GlobalGet(index) => write!(f, "global.get {}", index),
            Instruction::GlobalSet(index) => write!(f, "global.set {}", index),

            // Table instructions
            Instruction::TableGet(index) => write!(f, "table.get {}", index),
            Instruction::TableSet(index) => write!(f, "table.set {}", index),
            Instruction::TableSize(index) => write!(f, "table.size {}", index),
            Instruction::TableGrow(index) => write!(f, "table.grow {}", index),
            Instruction::TableInit(index, segment_index) => {
                write!(f, "table.init {} {}", index, segment_index)
            }
            Instruction::TableCopy(dest_index, src_index) => {
                write!(f, "table.copy {} {}", dest_index, src_index)
            }
            Instruction::TableFill(index) => write!(f, "table.fill {}", index),
            Instruction::ElemDrop(index) => write!(f, "elem.drop {}", index),

            // Memory instructions
            Instruction::I32Load(align, offset) => {
                write!(f, "i32.load align={} offset={}", align, offset)
            }
            Instruction::I64Load(align, offset) => {
                write!(f, "i64.load align={} offset={}", align, offset)
            }
            Instruction::F32Load(align, offset) => {
                write!(f, "f32.load align={} offset={}", align, offset)
            }
            Instruction::F64Load(align, offset) => {
                write!(f, "f64.load align={} offset={}", align, offset)
            }
            Instruction::I32Load8S(align, offset) => {
                write!(f, "i32.load8_s align={} offset={}", align, offset)
            }
            Instruction::I32Load8U(align, offset) => {
                write!(f, "i32.load8_u align={} offset={}", align, offset)
            }
            Instruction::I32Load16S(align, offset) => {
                write!(f, "i32.load16_s align={} offset={}", align, offset)
            }
            Instruction::I32Load16U(align, offset) => {
                write!(f, "i32.load16_u align={} offset={}", align, offset)
            }
            Instruction::I64Load8S(align, offset) => {
                write!(f, "i64.load8_s align={} offset={}", align, offset)
            }
            Instruction::I64Load8U(align, offset) => {
                write!(f, "i64.load8_u align={} offset={}", align, offset)
            }
            Instruction::I64Load16S(align, offset) => {
                write!(f, "i64.load16_s align={} offset={}", align, offset)
            }
            Instruction::I64Load16U(align, offset) => {
                write!(f, "i64.load16_u align={} offset={}", align, offset)
            }
            Instruction::I64Load32S(align, offset) => {
                write!(f, "i64.load32_s align={} offset={}", align, offset)
            }
            Instruction::I64Load32U(align, offset) => {
                write!(f, "i64.load32_u align={} offset={}", align, offset)
            }
            Instruction::I32Store(align, offset) => {
                write!(f, "i32.store align={} offset={}", align, offset)
            }
            Instruction::I64Store(align, offset) => {
                write!(f, "i64.store align={} offset={}", align, offset)
            }
            Instruction::F32Store(align, offset) => {
                write!(f, "f32.store align={} offset={}", align, offset)
            }
            Instruction::F64Store(align, offset) => {
                write!(f, "f64.store align={} offset={}", align, offset)
            }
            Instruction::I32Store8(align, offset) => {
                write!(f, "i32.store8 align={} offset={}", align, offset)
            }
            Instruction::I32Store16(align, offset) => {
                write!(f, "i32.store16 align={} offset={}", align, offset)
            }
            Instruction::I64Store8(align, offset) => {
                write!(f, "i64.store8 align={} offset={}", align, offset)
            }
            Instruction::I64Store16(align, offset) => {
                write!(f, "i64.store16 align={} offset={}", align, offset)
            }
            Instruction::I64Store32(align, offset) => {
                write!(f, "i64.store32 align={} offset={}", align, offset)
            }
            Instruction::MemorySize(mem_idx) => write!(f, "memory.size {}", mem_idx),
            Instruction::MemoryGrow(mem_idx) => write!(f, "memory.grow {}", mem_idx),
            Instruction::MemoryInit(segment_index) => write!(f, "memory.init {}", segment_index),
            Instruction::DataDrop(index) => write!(f, "data.drop {}", index),
            Instruction::MemoryCopy => write!(f, "memory.copy"),
            Instruction::MemoryFill => write!(f, "memory.fill"),

            // Numeric constant instructions
            Instruction::I32Const(value) => write!(f, "i32.const {}", value),
            Instruction::I64Const(value) => write!(f, "i64.const {}", value),
            Instruction::F32Const(value) => write!(f, "f32.const {}", value),
            Instruction::F64Const(value) => write!(f, "f64.const {}", value),

            // Comparison instructions
            Instruction::I32Eqz => write!(f, "i32.eqz"),
            Instruction::I32Eq => write!(f, "i32.eq"),
            Instruction::I32Ne => write!(f, "i32.ne"),
            Instruction::I32LtS => write!(f, "i32.lt_s"),
            Instruction::I32LtU => write!(f, "i32.lt_u"),
            Instruction::I32GtS => write!(f, "i32.gt_s"),
            Instruction::I32GtU => write!(f, "i32.gt_u"),
            Instruction::I32LeS => write!(f, "i32.le_s"),
            Instruction::I32LeU => write!(f, "i32.le_u"),
            Instruction::I32GeS => write!(f, "i32.ge_s"),
            Instruction::I32GeU => write!(f, "i32.ge_u"),
            Instruction::I64Eqz => write!(f, "i64.eqz"),
            Instruction::I64Eq => write!(f, "i64.eq"),
            Instruction::I64Ne => write!(f, "i64.ne"),
            Instruction::I64LtS => write!(f, "i64.lt_s"),
            Instruction::I64LtU => write!(f, "i64.lt_u"),
            Instruction::I64GtS => write!(f, "i64.gt_s"),
            Instruction::I64GtU => write!(f, "i64.gt_u"),
            Instruction::I64LeS => write!(f, "i64.le_s"),
            Instruction::I64LeU => write!(f, "i64.le_u"),
            Instruction::I64GeS => write!(f, "i64.ge_s"),
            Instruction::I64GeU => write!(f, "i64.ge_u"),
            Instruction::F32Eq => write!(f, "f32.eq"),
            Instruction::F32Ne => write!(f, "f32.ne"),
            Instruction::F32Lt => write!(f, "f32.lt"),
            Instruction::F32Gt => write!(f, "f32.gt"),
            Instruction::F32Le => write!(f, "f32.le"),
            Instruction::F32Ge => write!(f, "f32.ge"),
            Instruction::F64Eq => write!(f, "f64.eq"),
            Instruction::F64Ne => write!(f, "f64.ne"),
            Instruction::F64Lt => write!(f, "f64.lt"),
            Instruction::F64Gt => write!(f, "f64.gt"),
            Instruction::F64Le => write!(f, "f64.le"),
            Instruction::F64Ge => write!(f, "f64.ge"),

            // Arithmetic instructions
            Instruction::I32Clz => write!(f, "i32.clz"),
            Instruction::I32Ctz => write!(f, "i32.ctz"),
            Instruction::I32Popcnt => write!(f, "i32.popcnt"),
            Instruction::I32Add => write!(f, "i32.add"),
            Instruction::I32Sub => write!(f, "i32.sub"),
            Instruction::I32Mul => write!(f, "i32.mul"),
            Instruction::I32DivS => write!(f, "i32.div_s"),
            Instruction::I32DivU => write!(f, "i32.div_u"),
            Instruction::I32RemS => write!(f, "i32.rem_s"),
            Instruction::I32RemU => write!(f, "i32.rem_u"),
            Instruction::I32And => write!(f, "i32.and"),
            Instruction::I32Or => write!(f, "i32.or"),
            Instruction::I32Xor => write!(f, "i32.xor"),
            Instruction::I32Shl => write!(f, "i32.shl"),
            Instruction::I32ShrS => write!(f, "i32.shr_s"),
            Instruction::I32ShrU => write!(f, "i32.shr_u"),
            Instruction::I32Rotl => write!(f, "i32.rotl"),
            Instruction::I32Rotr => write!(f, "i32.rotr"),
            Instruction::I64Clz => write!(f, "i64.clz"),
            Instruction::I64Ctz => write!(f, "i64.ctz"),
            Instruction::I64Popcnt => write!(f, "i64.popcnt"),
            Instruction::I64Add => write!(f, "i64.add"),
            Instruction::I64Sub => write!(f, "i64.sub"),
            Instruction::I64Mul => write!(f, "i64.mul"),
            Instruction::I64DivS => write!(f, "i64.div_s"),
            Instruction::I64DivU => write!(f, "i64.div_u"),
            Instruction::I64RemS => write!(f, "i64.rem_s"),
            Instruction::I64RemU => write!(f, "i64.rem_u"),
            Instruction::I64And => write!(f, "i64.and"),
            Instruction::I64Or => write!(f, "i64.or"),
            Instruction::I64Xor => write!(f, "i64.xor"),
            Instruction::I64Shl => write!(f, "i64.shl"),
            Instruction::I64ShrS => write!(f, "i64.shr_s"),
            Instruction::I64ShrU => write!(f, "i64.shr_u"),
            Instruction::I64Rotl => write!(f, "i64.rotl"),
            Instruction::I64Rotr => write!(f, "i64.rotr"),
            Instruction::F32Abs => write!(f, "f32.abs"),
            Instruction::F32Neg => write!(f, "f32.neg"),
            Instruction::F32Ceil => write!(f, "f32.ceil"),
            Instruction::F32Floor => write!(f, "f32.floor"),
            Instruction::F32Trunc => write!(f, "f32.trunc"),
            Instruction::F32Nearest => write!(f, "f32.nearest"),
            Instruction::F32Sqrt => write!(f, "f32.sqrt"),
            Instruction::F32Add => write!(f, "f32.add"),
            Instruction::F32Sub => write!(f, "f32.sub"),
            Instruction::F32Mul => write!(f, "f32.mul"),
            Instruction::F32Div => write!(f, "f32.div"),
            Instruction::F32Min => write!(f, "f32.min"),
            Instruction::F32Max => write!(f, "f32.max"),
            Instruction::F32Copysign => write!(f, "f32.copysign"),
            Instruction::F64Abs => write!(f, "f64.abs"),
            Instruction::F64Neg => write!(f, "f64.neg"),
            Instruction::F64Ceil => write!(f, "f64.ceil"),
            Instruction::F64Floor => write!(f, "f64.floor"),
            Instruction::F64Trunc => write!(f, "f64.trunc"),
            Instruction::F64Nearest => write!(f, "f64.nearest"),
            Instruction::F64Sqrt => write!(f, "f64.sqrt"),
            Instruction::F64Add => write!(f, "f64.add"),
            Instruction::F64Sub => write!(f, "f64.sub"),
            Instruction::F64Mul => write!(f, "f64.mul"),
            Instruction::F64Div => write!(f, "f64.div"),
            Instruction::F64Min => write!(f, "f64.min"),
            Instruction::F64Max => write!(f, "f64.max"),
            Instruction::F64Copysign => write!(f, "f64.copysign"),

            // Conversion instructions
            Instruction::I32WrapI64 => write!(f, "i32.wrap_i64"),
            Instruction::I32TruncF32S => write!(f, "i32.trunc_f32_s"),
            Instruction::I32TruncF32U => write!(f, "i32.trunc_f32_u"),
            Instruction::I32TruncF64S => write!(f, "i32.trunc_f64_s"),
            Instruction::I32TruncF64U => write!(f, "i32.trunc_f64_u"),
            Instruction::I64ExtendI32S => write!(f, "i64.extend_i32_s"),
            Instruction::I64ExtendI32U => write!(f, "i64.extend_i32_u"),
            Instruction::I64TruncF32S => write!(f, "i64.trunc_f32_s"),
            Instruction::I64TruncF32U => write!(f, "i64.trunc_f32_u"),
            Instruction::I64TruncF64S => write!(f, "i64.trunc_f64_s"),
            Instruction::I64TruncF64U => write!(f, "i64.trunc_f64_u"),
            Instruction::I32Extend8S => write!(f, "i32.extend8_s"),
            Instruction::I32Extend16S => write!(f, "i32.extend16_s"),
            Instruction::I64Extend8S => write!(f, "i64.extend8_s"),
            Instruction::I64Extend16S => write!(f, "i64.extend16_s"),
            Instruction::I64Extend32S => write!(f, "i64.extend32_s"),
            Instruction::F32ConvertI32S => write!(f, "f32.convert_i32_s"),
            Instruction::F32ConvertI32U => write!(f, "f32.convert_i32_u"),
            Instruction::F32ConvertI64S => write!(f, "f32.convert_i64_s"),
            Instruction::F32ConvertI64U => write!(f, "f32.convert_i64_u"),
            Instruction::F32DemoteF64 => write!(f, "f32.demote_f64"),
            Instruction::F64ConvertI32S => write!(f, "f64.convert_i32_s"),
            Instruction::F64ConvertI32U => write!(f, "f64.convert_i32_u"),
            Instruction::F64ConvertI64S => write!(f, "f64.convert_i64_s"),
            Instruction::F64ConvertI64U => write!(f, "f64.convert_i64_u"),
            Instruction::F64PromoteF32 => write!(f, "f64.promote_f32"),
            Instruction::I32ReinterpretF32 => write!(f, "i32.reinterpret_f32"),
            Instruction::I64ReinterpretF64 => write!(f, "i64.reinterpret_f64"),
            Instruction::F32ReinterpretI32 => write!(f, "f32.reinterpret_i32"),
            Instruction::F64ReinterpretI64 => write!(f, "f64.reinterpret_i64"),

            // Basic SIMD instructions (minimal set for compatibility)
            Instruction::F32x4Splat => write!(f, "f32x4.splat"),
            Instruction::F64x2Splat => write!(f, "f64x2.splat"),
            Instruction::V128Load(align, offset) => {
                write!(f, "v128.load align={} offset={}", align, offset)
            }
            Instruction::V128Store(align, offset) => {
                write!(f, "v128.store align={} offset={}", align, offset)
            }
            Instruction::V128Load8Lane(align, offset, lane) => write!(
                f,
                "v128.load8_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Load16Lane(align, offset, lane) => write!(
                f,
                "v128.load16_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Load32Lane(align, offset, lane) => write!(
                f,
                "v128.load32_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Load64Lane(align, offset, lane) => write!(
                f,
                "v128.load64_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Store8Lane(align, offset, lane) => write!(
                f,
                "v128.store8_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Store16Lane(align, offset, lane) => write!(
                f,
                "v128.store16_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Store32Lane(align, offset, lane) => write!(
                f,
                "v128.store32_lane align={} offset={} lane={}",
                align, offset, lane
            ),
            Instruction::V128Store64Lane(align, offset, lane) => write!(
                f,
                "v128.store64_lane align={} offset={} lane={}",
                align, offset, lane
            ),

            // Reference instructions
            Instruction::RefNull(value_type) => write!(f, "ref.null {}", value_type),
            Instruction::RefIsNull => write!(f, "ref.is_null"),
            Instruction::RefFunc(index) => write!(f, "ref.func {}", index),

            // Non-trapping Float-to-int Conversions
            Instruction::I32TruncSatF32S => write!(f, "i32.trunc_sat_f32_s"),
            Instruction::I32TruncSatF32U => write!(f, "i32.trunc_sat_f32_u"),
            Instruction::I32TruncSatF64S => write!(f, "i32.trunc_sat_f64_s"),
            Instruction::I32TruncSatF64U => write!(f, "i32.trunc_sat_f64_u"),
            Instruction::I64TruncSatF32S => write!(f, "i64.trunc_sat_f32_s"),
            Instruction::I64TruncSatF32U => write!(f, "i64.trunc_sat_f32_u"),
            Instruction::I64TruncSatF64S => write!(f, "i64.trunc_sat_f64_s"),
            Instruction::I64TruncSatF64U => write!(f, "i64.trunc_sat_f64_u"),

            // New SIMD instructions
            Instruction::I32x4ExtAddPairwiseI16x8S => write!(f, "i32x4.extadd_pairwise_i16x8_s"),
            Instruction::I32x4ExtAddPairwiseI16x8U => write!(f, "i32x4.extadd_pairwise_i16x8_u"),
            Instruction::V128Shuffle(lanes) => write!(f, "v128.shuffle {:02x?}", lanes),
            Instruction::V128SplatI8x16 => write!(f, "i8x16.splat"),
            Instruction::V128SplatI16x8 => write!(f, "i16x8.splat"),
            Instruction::V128SplatI32x4 => write!(f, "i32x4.splat"),
            Instruction::V128SplatI64x2 => write!(f, "i64x2.splat"),
            Instruction::SimdOpAE => write!(f, "simd_op_ae"),
            Instruction::SimdOpB1 => write!(f, "simd_op_b1"),
            Instruction::SimdOpB5 => write!(f, "simd_op_b5"),
            Instruction::I32x4DotI16x8S => write!(f, "i32x4.dot_i16x8_s"),

            // SIMD instructions (prefix 0xfd)
            Instruction::V128Const(bytes) => write!(f, "v128.const {:02x?}", bytes),
        }
    }
}
