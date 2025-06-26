//! Instruction parser for converting WebAssembly bytecode to runtime instructions
//!
//! This module bridges the gap between raw bytecode from the parser and
//! the parsed instruction format expected by the runtime execution engine.

use wrt_foundation::{
    types::{Instruction, BlockType, MemArg},
    bounded::BoundedVec,
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};
use wrt_error::{Error, ErrorCategory, Result, codes};

// Type aliases for capability-based memory allocation
type InstructionProvider = wrt_foundation::safe_memory::NoStdProvider<8192>;
type InstructionVec = BoundedVec<Instruction<InstructionProvider>, 1024, InstructionProvider>;
type TargetVec = BoundedVec<u32, 256, InstructionProvider>;

/// Parse WebAssembly bytecode into runtime instructions
pub fn parse_instructions(bytecode: &[u8]) -> Result<InstructionVec> {
    let provider = safe_managed_alloc!(8192, CrateId::Runtime)?;
    let mut instructions = BoundedVec::new(provider).map_err(|_| {
        Error::memory_error("Failed to allocate instruction vector")
    })?;
    
    let mut offset = 0;
    while offset < bytecode.len() {
        let (instruction, consumed) = parse_instruction(bytecode, offset)?;
        let is_end = matches!(instruction, Instruction::End);
        instructions.push(instruction).map_err(|_| {
            Error::capacity_exceeded("Too many instructions in function")
        })?;
        offset += consumed;
        
        // Check for end instruction
        if is_end {
            break;
        }
    }
    
    Ok(instructions)
}

/// Parse a single instruction from bytecode
fn parse_instruction(bytecode: &[u8], offset: usize) -> Result<(Instruction<InstructionProvider>, usize)> {
    if offset >= bytecode.len() {
        return Err(Error::parse_error("Unexpected end of bytecode"));
    }
    
    let opcode = bytecode[offset];
    let mut consumed = 1;
    
    let instruction = match opcode {
        // Control instructions
        0x00 => Instruction::Unreachable,
        0x01 => Instruction::Nop,
        0x02 => {
            // Block with block type
            let block_type = parse_block_type(bytecode, offset + 1)?;
            consumed += 1; // Simplified - actual block type parsing may consume more
            let block_type_idx = block_type_to_index(&block_type);
            Instruction::Block { block_type_idx }
        }
        0x03 => {
            // Loop with block type
            let block_type = parse_block_type(bytecode, offset + 1)?;
            consumed += 1;
            let block_type_idx = block_type_to_index(&block_type);
            Instruction::Loop { block_type_idx }
        }
        0x04 => {
            // If with block type
            let block_type = parse_block_type(bytecode, offset + 1)?;
            consumed += 1;
            let block_type_idx = block_type_to_index(&block_type);
            Instruction::If { block_type_idx }
        }
        0x05 => Instruction::Else,
        0x0B => Instruction::End,
        0x0C => {
            // Br (branch)
            let (label_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::Br(label_idx)
        }
        0x0D => {
            // BrIf (conditional branch)
            let (label_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::BrIf(label_idx)
        }
        0x0E => {
            // BrTable
            let provider = safe_managed_alloc!(8192, CrateId::Runtime)?; // Provider for BoundedVec
            let mut targets = BoundedVec::new(provider).map_err(|_| Error::parse_error("Failed to create BrTable targets vector"))?;
            
            let (count, mut bytes_consumed) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes_consumed;
            
            // Parse all target labels
            for _ in 0..count {
                let (target, bytes) = read_leb128_u32(bytecode, offset + consumed)?;
                consumed += bytes;
                targets.push(target).map_err(|_| Error::parse_error("Too many BrTable targets"))?;
            }
            
            // Parse default target
            let (default_target, bytes) = read_leb128_u32(bytecode, offset + consumed)?;
            consumed += bytes;
            
            Instruction::BrTable { targets, default_target }
        }
        0x0F => Instruction::Return,
        0x10 => {
            // Call
            let (func_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::Call(func_idx)
        }
        0x11 => {
            // CallIndirect
            let (type_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            consumed += 1; // Skip table index (always 0 in MVP)
            Instruction::CallIndirect(type_idx, 0)
        }
        
        // Parametric instructions
        0x1A => Instruction::Drop,
        0x1B => Instruction::Select,
        
        // Variable instructions
        0x20 => {
            let (local_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::LocalGet(local_idx)
        }
        0x21 => {
            let (local_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::LocalSet(local_idx)
        }
        0x22 => {
            let (local_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::LocalTee(local_idx)
        }
        0x23 => {
            let (global_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::GlobalGet(global_idx)
        }
        0x24 => {
            let (global_idx, bytes) = read_leb128_u32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::GlobalSet(global_idx)
        }
        
        // Memory instructions
        0x28 => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::I32Load(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x29 => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::I64Load(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x2A => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::F32Load(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x2B => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::F64Load(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x36 => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::I32Store(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x37 => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::I64Store(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x38 => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::F32Store(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x39 => {
            let (align, bytes1) = read_leb128_u32(bytecode, offset + 1)?;
            let (offset, bytes2) = read_leb128_u32(bytecode, offset + 1 + bytes1)?;
            consumed += bytes1 + bytes2;
            Instruction::F64Store(MemArg {
                align_exponent: align,
                offset,
                memory_index: 0,
            })
        }
        0x3F => {
            consumed += 1; // Skip reserved byte
            Instruction::MemorySize(0)
        }
        0x40 => {
            consumed += 1; // Skip reserved byte
            Instruction::MemoryGrow(0)
        }
        
        // Numeric instructions - Constants
        0x41 => {
            let (value, bytes) = read_leb128_i32(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::I32Const(value)
        }
        0x42 => {
            let (value, bytes) = read_leb128_i64(bytecode, offset + 1)?;
            consumed += bytes;
            Instruction::I64Const(value)
        }
        0x43 => {
            if offset + 5 > bytecode.len() {
                return Err(Error::parse_error("F32 constant extends beyond bytecode"));
            }
            let bytes = [bytecode[offset + 1], bytecode[offset + 2], bytecode[offset + 3], bytecode[offset + 4]];
            let value = u32::from_le_bytes(bytes); // Use bit representation
            consumed += 4;
            Instruction::F32Const(value)
        }
        0x44 => {
            if offset + 9 > bytecode.len() {
                return Err(Error::parse_error("F64 constant extends beyond bytecode"));
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&bytecode[offset + 1..offset + 9]);
            let value = u64::from_le_bytes(bytes); // Use bit representation
            consumed += 8;
            Instruction::F64Const(value)
        }
        
        // Numeric instructions - i32 operations
        0x6A => Instruction::I32Add,
        0x6B => Instruction::I32Sub,
        0x6C => Instruction::I32Mul,
        0x6D => Instruction::I32DivS,
        0x6E => Instruction::I32DivU,
        0x6F => Instruction::I32RemS,
        0x70 => Instruction::I32RemU,
        0x71 => Instruction::I32And,
        0x72 => Instruction::I32Or,
        0x73 => Instruction::I32Xor,
        0x74 => Instruction::I32Shl,
        0x75 => Instruction::I32ShrS,
        0x76 => Instruction::I32ShrU,
        0x77 => Instruction::I32Rotl,
        0x78 => Instruction::I32Rotr,
        
        // Comparison
        0x45 => Instruction::I32Eqz,
        0x46 => Instruction::I32Eq,
        0x47 => Instruction::I32Ne,
        0x48 => Instruction::I32LtS,
        0x49 => Instruction::I32LtU,
        0x4A => Instruction::I32GtS,
        0x4B => Instruction::I32GtU,
        0x4C => Instruction::I32LeS,
        0x4D => Instruction::I32LeU,
        0x4E => Instruction::I32GeS,
        0x4F => Instruction::I32GeU,
        
        // i64 operations
        0x7C => Instruction::I64Add,
        0x7D => Instruction::I64Sub,
        0x7E => Instruction::I64Mul,
        0x7F => Instruction::I64DivS,
        0x80 => Instruction::I64DivU,
        0x81 => Instruction::I64RemS,
        0x82 => Instruction::I64RemU,
        0x83 => Instruction::I64And,
        0x84 => Instruction::I64Or,
        0x85 => Instruction::I64Xor,
        0x86 => Instruction::I64Shl,
        0x87 => Instruction::I64ShrS,
        0x88 => Instruction::I64ShrU,
        0x89 => Instruction::I64Rotl,
        0x8A => Instruction::I64Rotr,
        
        // f32 operations
        0x92 => Instruction::F32Add,
        0x93 => Instruction::F32Sub,
        0x94 => Instruction::F32Mul,
        0x95 => Instruction::F32Div,
        0x96 => Instruction::F32Min,
        0x97 => Instruction::F32Max,
        0x98 => Instruction::F32Copysign,
        
        // f64 operations
        0xA0 => Instruction::F64Add,
        0xA1 => Instruction::F64Sub,
        0xA2 => Instruction::F64Mul,
        0xA3 => Instruction::F64Div,
        0xA4 => Instruction::F64Min,
        0xA5 => Instruction::F64Max,
        0xA6 => Instruction::F64Copysign,
        
        // Conversions
        0xA7 => Instruction::I32WrapI64,
        0xA8 => Instruction::I32TruncF32S,
        0xA9 => Instruction::I32TruncF32U,
        0xAA => Instruction::I32TruncF64S,
        0xAB => Instruction::I32TruncF64U,
        0xAC => Instruction::I64ExtendI32S,
        0xAD => Instruction::I64ExtendI32U,
        0xAE => Instruction::I64TruncF32S,
        0xAF => Instruction::I64TruncF32U,
        0xB0 => Instruction::I64TruncF64S,
        0xB1 => Instruction::I64TruncF64U,
        0xB2 => Instruction::F32ConvertI32S,
        0xB3 => Instruction::F32ConvertI32U,
        0xB4 => Instruction::F32ConvertI64S,
        0xB5 => Instruction::F32ConvertI64U,
        0xB6 => Instruction::F32DemoteF64,
        0xB7 => Instruction::F64ConvertI32S,
        0xB8 => Instruction::F64ConvertI32U,
        0xB9 => Instruction::F64ConvertI64S,
        0xBA => Instruction::F64ConvertI64U,
        0xBB => Instruction::F64PromoteF32,
        
        _ => {
            return Err(Error::parse_error("Unknown instruction opcode"));
        }
    };
    
    Ok((instruction, consumed))
}

/// Parse a block type
fn parse_block_type(bytecode: &[u8], offset: usize) -> Result<BlockType> {
    if offset >= bytecode.len() {
        return Err(Error::parse_error("Unexpected end while parsing block type"));
    }
    
    match bytecode[offset] {
        0x40 => Ok(BlockType::Value(None)),
        b if b & 0x80 == 0 => {
            // Value type
            match b {
                0x7F => Ok(BlockType::Value(Some(wrt_foundation::types::ValueType::I32))),
                0x7E => Ok(BlockType::Value(Some(wrt_foundation::types::ValueType::I64))),
                0x7D => Ok(BlockType::Value(Some(wrt_foundation::types::ValueType::F32))),
                0x7C => Ok(BlockType::Value(Some(wrt_foundation::types::ValueType::F64))),
                _ => Err(Error::parse_error("Invalid value type in block type"))
            }
        }
        _ => {
            // Type index (simplified - just return empty for now)
            Ok(BlockType::Value(None))
        }
    }
}

/// Read a LEB128 encoded u32
fn read_leb128_u32(data: &[u8], offset: usize) -> Result<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut consumed = 0;
    
    loop {
        if offset + consumed >= data.len() {
            return Err(Error::parse_error("Unexpected end of data while reading LEB128"));
        }
        
        let byte = data[offset + consumed];
        consumed += 1;
        
        result |= ((byte & 0x7F) as u32) << shift;
        
        if byte & 0x80 == 0 {
            break;
        }
        
        shift += 7;
        if shift >= 32 {
            return Err(Error::parse_error("LEB128 value too large for u32"));
        }
    }
    
    Ok((result, consumed))
}

/// Read a LEB128 encoded i32
fn read_leb128_i32(data: &[u8], offset: usize) -> Result<(i32, usize)> {
    let mut result = 0i32;
    let mut shift = 0;
    let mut consumed = 0;
    let mut byte;
    
    loop {
        if offset + consumed >= data.len() {
            return Err(Error::parse_error("Unexpected end of data while reading LEB128"));
        }
        
        byte = data[offset + consumed];
        consumed += 1;
        
        result |= ((byte & 0x7F) as i32) << shift;
        shift += 7;
        
        if byte & 0x80 == 0 {
            break;
        }
    }
    
    // Sign extend
    if shift < 32 && (byte & 0x40) != 0 {
        result |= !0 << shift;
    }
    
    Ok((result, consumed))
}

/// Read a LEB128 encoded i64
fn read_leb128_i64(data: &[u8], offset: usize) -> Result<(i64, usize)> {
    let mut result = 0i64;
    let mut shift = 0;
    let mut consumed = 0;
    let mut byte;
    
    loop {
        if offset + consumed >= data.len() {
            return Err(Error::parse_error("Unexpected end of data while reading LEB128"));
        }
        
        byte = data[offset + consumed];
        consumed += 1;
        
        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;
        
        if byte & 0x80 == 0 {
            break;
        }
    }
    
    // Sign extend
    if shift < 64 && (byte & 0x40) != 0 {
        result |= !0 << shift;
    }
    
    Ok((result, consumed))
}

/// Convert BlockType to a type index for instruction storage
fn block_type_to_index(block_type: &BlockType) -> u32 {
    match block_type {
        BlockType::Value(None) => 0x40, // Empty type
        BlockType::Value(Some(wrt_foundation::types::ValueType::I32)) => 0x7F,
        BlockType::Value(Some(wrt_foundation::types::ValueType::I64)) => 0x7E,
        BlockType::Value(Some(wrt_foundation::types::ValueType::F32)) => 0x7D,
        BlockType::Value(Some(wrt_foundation::types::ValueType::F64)) => 0x7C,
        BlockType::Value(Some(wrt_foundation::types::ValueType::V128)) => 0x7B,
        BlockType::Value(Some(wrt_foundation::types::ValueType::I16x8)) => 0x7A,
        BlockType::Value(Some(wrt_foundation::types::ValueType::FuncRef)) => 0x70,
        BlockType::Value(Some(wrt_foundation::types::ValueType::ExternRef)) => 0x6F,
        BlockType::FuncType(idx) => *idx,
        // Handle any other value types with a default
        BlockType::Value(Some(_)) => 0x40, // Default to empty type for unknown types
    }
}