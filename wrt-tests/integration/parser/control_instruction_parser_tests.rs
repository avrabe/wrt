//! Control Instruction Parser Tests
//!
//! This module consolidates control instruction parsing and encoding tests
//! from test-control-instructions/ into the unified test suite.

#![cfg(test)]

use std::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use wrt_decoder::instructions::{encode_instruction, parse_instruction};
use wrt_error::Result;

// ===========================================
// CONTROL INSTRUCTION PARSING TESTS
// ===========================================

mod control_instruction_tests {
    use super::*;

    #[test]
    fn test_parse_encode_block() -> Result<()> {
        let block_bytes = vec![0x02, 0x40, 0x0B]; // block (empty) end
        let (block_instr, block_bytes_read) = parse_instruction(&block_bytes)?;

        assert_eq!(block_bytes_read, block_bytes.len(), "Should read all bytes";

        let encoded_block = encode_instruction(&block_instr)?;
        assert_eq!(encoded_block, block_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_loop() -> Result<()> {
        let loop_bytes = vec![
            0x03, 0x7F, // loop with i32 return type
            0x41, 0x01, // i32.const 1
            0x0B, // end
        ];
        let (loop_instr, loop_bytes_read) = parse_instruction(&loop_bytes)?;

        assert_eq!(loop_bytes_read, loop_bytes.len(), "Should read all bytes";

        let encoded_loop = encode_instruction(&loop_instr)?;
        assert_eq!(encoded_loop, loop_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_if() -> Result<()> {
        let if_bytes = vec![
            0x04, 0x40, // if with empty block type
            0x41, 0x01, // i32.const 1
            0x05, // else
            0x41, 0x00, // i32.const 0
            0x0B, // end
        ];
        let (if_instr, if_bytes_read) = parse_instruction(&if_bytes)?;

        assert_eq!(if_bytes_read, if_bytes.len(), "Should read all bytes";

        let encoded_if = encode_instruction(&if_instr)?;
        assert_eq!(encoded_if, if_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_br_table() -> Result<()> {
        let br_table_bytes = vec![
            0x0E, // br_table
            0x02, // count = 2
            0x00, // label 0
            0x01, // label 1
            0x02, // default label 2
        ];
        let (br_table_instr, br_table_bytes_read) = parse_instruction(&br_table_bytes)?;

        assert_eq!(br_table_bytes_read, br_table_bytes.len(), "Should read all bytes";

        let encoded_br_table = encode_instruction(&br_table_instr)?;
        assert_eq!(encoded_br_table, br_table_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_nested_blocks() -> Result<()> {
        let nested_bytes = vec![
            0x02, 0x40, // outer block
            0x02, 0x40, // inner block
            0x0B, // inner end
            0x0B, // outer end
        ];
        let (nested_instr, nested_bytes_read) = parse_instruction(&nested_bytes)?;

        assert_eq!(nested_bytes_read, nested_bytes.len(), "Should read all bytes";

        let encoded_nested = encode_instruction(&nested_instr)?;
        assert_eq!(encoded_nested, nested_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_br() -> Result<()> {
        let br_bytes = vec![0x0C, 0x00]; // br 0
        let (br_instr, br_bytes_read) = parse_instruction(&br_bytes)?;

        assert_eq!(br_bytes_read, br_bytes.len(), "Should read all bytes";

        let encoded_br = encode_instruction(&br_instr)?;
        assert_eq!(encoded_br, br_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_br_if() -> Result<()> {
        let br_if_bytes = vec![0x0D, 0x01]; // br_if 1
        let (br_if_instr, br_if_bytes_read) = parse_instruction(&br_if_bytes)?;

        assert_eq!(br_if_bytes_read, br_if_bytes.len(), "Should read all bytes";

        let encoded_br_if = encode_instruction(&br_if_instr)?;
        assert_eq!(encoded_br_if, br_if_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_return() -> Result<()> {
        let return_bytes = vec![0x0F]; // return
        let (return_instr, return_bytes_read) = parse_instruction(&return_bytes)?;

        assert_eq!(return_bytes_read, return_bytes.len(), "Should read all bytes";

        let encoded_return = encode_instruction(&return_instr)?;
        assert_eq!(encoded_return, return_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_call() -> Result<()> {
        let call_bytes = vec![0x10, 0x05]; // call 5
        let (call_instr, call_bytes_read) = parse_instruction(&call_bytes)?;

        assert_eq!(call_bytes_read, call_bytes.len(), "Should read all bytes";

        let encoded_call = encode_instruction(&call_instr)?;
        assert_eq!(encoded_call, call_bytes, "Encoded bytes should match original";

        Ok(())
    }

    #[test]
    fn test_parse_encode_call_indirect() -> Result<()> {
        let call_indirect_bytes = vec![0x11, 0x02, 0x00]; // call_indirect type_index=2, table_index=0
        let (call_indirect_instr, call_indirect_bytes_read) = parse_instruction(&call_indirect_bytes)?;

        assert_eq!(call_indirect_bytes_read, call_indirect_bytes.len(), "Should read all bytes";

        let encoded_call_indirect = encode_instruction(&call_indirect_instr)?;
        assert_eq!(encoded_call_indirect, call_indirect_bytes, "Encoded bytes should match original";

        Ok(())
    }
}

// ===========================================
// CONTROL FLOW VALIDATION TESTS
// ===========================================

mod control_flow_validation_tests {
    use super::*;

    #[test]
    fn test_block_type_validation() -> Result<()> {
        // Test different block types
        let valid_block_types = vec![
            vec![0x02, 0x40, 0x0B], // empty block
            vec![0x02, 0x7F, 0x0B], // i32 block
            vec![0x02, 0x7E, 0x0B], // i64 block
            vec![0x02, 0x7D, 0x0B], // f32 block
            vec![0x02, 0x7C, 0x0B], // f64 block
        ];

        for block_bytes in valid_block_types {
            let (_, bytes_read) = parse_instruction(&block_bytes)?;
            assert_eq!(bytes_read, block_bytes.len();
        }

        Ok(())
    }

    #[test]
    fn test_nested_control_flow() -> Result<()> {
        // Test deeply nested control structures
        let nested_control = vec![
            0x02, 0x40, // outer block
            0x03, 0x40, // loop
            0x04, 0x40, // if
            0x02, 0x40, // inner block
            0x0B, // end inner block
            0x05, // else
            0x41, 0x00, // i32.const 0
            0x0B, // end if
            0x0B, // end loop
            0x0B, // end outer block
        ];

        let (_, bytes_read) = parse_instruction(&nested_control)?;
        assert_eq!(bytes_read, nested_control.len();

        Ok(())
    }

    #[test]
    fn test_branch_label_validation() -> Result<()> {
        // Test branch instructions with different label depths
        let branch_instructions = vec![
            vec![0x0C, 0x00], // br 0
            vec![0x0C, 0x01], // br 1
            vec![0x0C, 0x02], // br 2
            vec![0x0D, 0x00], // br_if 0
            vec![0x0D, 0x01], // br_if 1
        ];

        for branch_bytes in branch_instructions {
            let (_, bytes_read) = parse_instruction(&branch_bytes)?;
            assert_eq!(bytes_read, branch_bytes.len();
        }

        Ok(())
    }

    #[test]
    fn test_br_table_validation() -> Result<()> {
        // Test br_table with various configurations
        let br_table_configs = vec![
            vec![0x0E, 0x00, 0x00], // br_table with no labels, default 0
            vec![0x0E, 0x01, 0x00, 0x01], // br_table with 1 label, default 1
            vec![0x0E, 0x03, 0x00, 0x01, 0x02, 0x03], // br_table with 3 labels
        ];

        for br_table_bytes in br_table_configs {
            let (_, bytes_read) = parse_instruction(&br_table_bytes)?;
            assert_eq!(bytes_read, br_table_bytes.len();
        }

        Ok(())
    }
}

// ===========================================
// CONTROL INSTRUCTION EDGE CASES
// ===========================================

mod control_instruction_edge_cases {
    use super::*;

    #[test]
    fn test_empty_blocks() -> Result<()> {
        let empty_blocks = vec![
            vec![0x02, 0x40, 0x0B], // empty block
            vec![0x03, 0x40, 0x0B], // empty loop
            vec![0x04, 0x40, 0x0B], // empty if (no else)
        ];

        for block_bytes in empty_blocks {
            let (instr, bytes_read) = parse_instruction(&block_bytes)?;
            assert_eq!(bytes_read, block_bytes.len();

            let encoded = encode_instruction(&instr)?;
            assert_eq!(encoded, block_bytes;
        }

        Ok(())
    }

    #[test]
    fn test_if_else_combinations() -> Result<()> {
        // Test various if-else structures
        let if_else_patterns = vec![
            // if without else
            vec![0x04, 0x40, 0x41, 0x01, 0x0B],
            // if with else
            vec![0x04, 0x40, 0x41, 0x01, 0x05, 0x41, 0x00, 0x0B],
            // if with empty else
            vec![0x04, 0x40, 0x41, 0x01, 0x05, 0x0B],
        ];

        for if_bytes in if_else_patterns {
            let (instr, bytes_read) = parse_instruction(&if_bytes)?;
            assert_eq!(bytes_read, if_bytes.len();

            let encoded = encode_instruction(&instr)?;
            assert_eq!(encoded, if_bytes;
        }

        Ok(())
    }

    #[test]
    fn test_function_call_variations() -> Result<()> {
        // Test various function call patterns
        let call_patterns = vec![
            vec![0x10, 0x00], // call 0
            vec![0x10, 0x7F], // call 127 (single byte)
            vec![0x10, 0x80, 0x01], // call 128 (multi-byte LEB128)
        ];

        for call_bytes in call_patterns {
            let (instr, bytes_read) = parse_instruction(&call_bytes)?;
            assert_eq!(bytes_read, call_bytes.len();

            let encoded = encode_instruction(&instr)?;
            assert_eq!(encoded, call_bytes;
        }

        Ok(())
    }

    #[test]
    fn test_unreachable_and_nop() -> Result<()> {
        let simple_instructions = vec![
            vec![0x00], // unreachable
            vec![0x01], // nop
        ];

        for instr_bytes in simple_instructions {
            let (instr, bytes_read) = parse_instruction(&instr_bytes)?;
            assert_eq!(bytes_read, instr_bytes.len();

            let encoded = encode_instruction(&instr)?;
            assert_eq!(encoded, instr_bytes;
        }

        Ok(())
    }

    #[test]
    fn test_large_br_table() -> Result<()> {
        // Test br_table with many labels
        let mut br_table_bytes = vec![0x0E]; // br_table opcode
        br_table_bytes.push(0x0A); // 10 labels
        
        // Add 10 labels (0-9)
        for i in 0..10 {
            br_table_bytes.push(i);
        }
        br_table_bytes.push(0x0A); // default label

        let (instr, bytes_read) = parse_instruction(&br_table_bytes)?;
        assert_eq!(bytes_read, br_table_bytes.len();

        let encoded = encode_instruction(&instr)?;
        assert_eq!(encoded, br_table_bytes;

        Ok(())
    }
}