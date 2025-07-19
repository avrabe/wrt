//! Tests for decoding WebAssembly branch hinting instructions.
//!
//! These tests verify that the decoder can properly parse branch hinting
//! opcodes and create the correct instruction representations.

use wrt_error::Result;
use wrt_foundation::types::{Instruction, LabelIdx};
use wrt_decoder::instructions::parse_instruction;

/// Test decoding br_on_null instruction
#[test]
fn test_decode_br_on_null() -> Result<()> {
    // br_on_null with label index 0
    // Opcode: 0xD5, LabelIdx: 0 (encoded as LEB128)
    let bytecode = &[0xD5, 0x00];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::BrOnNull(label) => {
            assert_eq!(label, 0;
        }
        _ => panic!("Expected BrOnNull instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 2;
    
    Ok(())
}

/// Test decoding br_on_non_null instruction
#[test]
fn test_decode_br_on_non_null() -> Result<()> {
    // br_on_non_null with label index 5
    // Opcode: 0xD6, LabelIdx: 5 (encoded as LEB128)
    let bytecode = &[0xD6, 0x05];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::BrOnNonNull(label) => {
            assert_eq!(label, 5;
        }
        _ => panic!("Expected BrOnNonNull instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 2;
    
    Ok(())
}

/// Test decoding ref.is_null instruction
#[test]
fn test_decode_ref_is_null() -> Result<()> {
    // ref.is_null has no operands
    // Opcode: 0xD1
    let bytecode = &[0xD1];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::RefIsNull => {}
        _ => panic!("Expected RefIsNull instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 1;
    
    Ok(())
}

/// Test decoding ref.as_non_null instruction
#[test]
fn test_decode_ref_as_non_null() -> Result<()> {
    // ref.as_non_null has no operands
    // Opcode: 0xD3
    let bytecode = &[0xD3];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::RefAsNonNull => {}
        _ => panic!("Expected RefAsNonNull instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 1;
    
    Ok(())
}

/// Test decoding ref.eq instruction
#[test]
fn test_decode_ref_eq() -> Result<()> {
    // ref.eq has no operands
    // Opcode: 0xD2
    let bytecode = &[0xD2];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::RefEq => {}
        _ => panic!("Expected RefEq instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 1;
    
    Ok(())
}

/// Test decoding return_call instruction (tail call)
#[test]
fn test_decode_return_call() -> Result<()> {
    // return_call with function index 10
    // Opcode: 0x12, FuncIdx: 10 (encoded as LEB128)
    let bytecode = &[0x12, 0x0A];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::ReturnCall(func_idx) => {
            assert_eq!(func_idx, 10;
        }
        _ => panic!("Expected ReturnCall instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 2;
    
    Ok(())
}

/// Test decoding return_call_indirect instruction (tail call indirect)
#[test]
fn test_decode_return_call_indirect() -> Result<()> {
    // return_call_indirect with type index 3, table index 0
    // Opcode: 0x13, TypeIdx: 3, TableIdx: 0 (both encoded as LEB128)
    let bytecode = &[0x13, 0x03, 0x00];
    
    // Parse instruction
    let (instruction, consumed) = parse_instruction(bytecode)?;
    
    // Verify correct instruction was parsed
    match instruction {
        Instruction::ReturnCallIndirect(type_idx, table_idx) => {
            assert_eq!(type_idx, 3;
            assert_eq!(table_idx, 0;
        }
        _ => panic!("Expected ReturnCallIndirect instruction, got {:?}", instruction),
    }
    
    // Verify correct number of bytes consumed
    assert_eq!(consumed, 3;
    
    Ok(())
}

/// Test decoding branch hinting instructions with large label indices
#[test]
fn test_decode_large_label_indices() -> Result<()> {
    // br_on_null with large label index (127, which requires 1 byte LEB128)
    let bytecode1 = &[0xD5, 0x7F];
    let (instruction1, consumed1) = parse_instruction(bytecode1)?;
    
    match instruction1 {
        Instruction::BrOnNull(label) => {
            assert_eq!(label, 127;
        }
        _ => panic!("Expected BrOnNull instruction"),
    }
    assert_eq!(consumed1, 2;
    
    // br_on_non_null with larger label index (128, which requires 2 bytes LEB128)
    let bytecode2 = &[0xD6, 0x80, 0x01];
    let (instruction2, consumed2) = parse_instruction(bytecode2)?;
    
    match instruction2 {
        Instruction::BrOnNonNull(label) => {
            assert_eq!(label, 128;
        }
        _ => panic!("Expected BrOnNonNull instruction"),
    }
    assert_eq!(consumed2, 3;
    
    Ok(())
}

/// Test error cases for invalid opcodes
#[test]
fn test_invalid_opcodes() {
    // Test unrecognized opcode 0xD4 (reserved)
    let bytecode = &[0xD4];
    let result = parse_instruction(bytecode;
    assert!(result.is_err(), "Expected error for reserved opcode 0xD4");
    
    // Test incomplete instruction (br_on_null without operand)
    let bytecode = &[0xD5];
    let result = parse_instruction(bytecode;
    assert!(result.is_err(), "Expected error for incomplete br_on_null");
}

/// Integration test: decode a sequence of branch hinting instructions
#[test]
fn test_decode_instruction_sequence() -> Result<()> {
    // Sequence: ref.is_null, br_on_null 1, ref.as_non_null, br_on_non_null 2
    let bytecode = &[
        0xD1,       // ref.is_null
        0xD5, 0x01, // br_on_null 1
        0xD3,       // ref.as_non_null
        0xD6, 0x02, // br_on_non_null 2
    ];
    
    let mut offset = 0;
    
    // Parse ref.is_null
    let (instr1, consumed1) = parse_instruction(&bytecode[offset..])?;
    offset += consumed1;
    assert!(matches!(instr1, Instruction::RefIsNull);
    
    // Parse br_on_null
    let (instr2, consumed2) = parse_instruction(&bytecode[offset..])?;
    offset += consumed2;
    assert!(matches!(instr2, Instruction::BrOnNull(1));
    
    // Parse ref.as_non_null
    let (instr3, consumed3) = parse_instruction(&bytecode[offset..])?;
    offset += consumed3;
    assert!(matches!(instr3, Instruction::RefAsNonNull);
    
    // Parse br_on_non_null
    let (instr4, consumed4) = parse_instruction(&bytecode[offset..])?;
    offset += consumed4;
    assert!(matches!(instr4, Instruction::BrOnNonNull(2));
    
    // Verify we consumed all bytes
    assert_eq!(offset, bytecode.len);
    
    Ok(())
}

/// Performance test: decode many branch hinting instructions
#[test]
#[cfg(feature = "std")]
fn test_decode_performance() -> Result<()> {
    use std::time::Instant;
    
    // Create bytecode with 1000 br_on_null instructions
    let mut bytecode = Vec::new);
    for _ in 0..1000 {
        bytecode.extend_from_slice(&[0xD5, 0x00]); // br_on_null 0
    }
    
    let start = Instant::now);
    
    // Decode all instructions
    let mut offset = 0;
    let mut count = 0;
    while offset < bytecode.len() {
        let (instruction, consumed) = parse_instruction(&bytecode[offset..])?;
        offset += consumed;
        count += 1;
        
        // Verify each instruction is correct
        assert!(matches!(instruction, Instruction::BrOnNull(0));
    }
    
    let duration = start.elapsed);
    
    // Verify we decoded the expected number of instructions
    assert_eq!(count, 1000;
    
    // Performance check: should decode 1000 instructions quickly
    println!("Decoded {} branch hinting instructions in {:?}", count, duration;
    assert!(duration.as_millis() < 100, "Decoding took too long: {:?}", duration);
    
    Ok(())
}