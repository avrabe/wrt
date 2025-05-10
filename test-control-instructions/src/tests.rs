#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Import appropriate types based on environment
#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec::Vec;

use wrt_decoder::instructions::{encode_instruction, parse_instruction};
use wrt_test_registry::{assert_eq_test, assert_test, register_test};

// Register all the control instruction tests
pub fn register_control_instruction_tests() {
    // Test block instruction
    register_test!("parse_encode_block", "instruction-decoder", false, || {
        let block_bytes = vec![0x02, 0x40, 0x0B]; // block (empty) end
        let (block_instr, block_bytes_read) = parse_instruction(&block_bytes)
            .map_err(|e| format!("Failed to parse block: {:?}", e))?;

        assert_eq_test!(block_bytes_read, block_bytes.len(), "Should read all bytes");

        let encoded_block = encode_instruction(&block_instr)
            .map_err(|e| format!("Failed to encode block: {:?}", e))?;

        assert_eq_test!(encoded_block, block_bytes, "Encoded bytes should match original");

        Ok(())
    });

    // Test loop instruction
    register_test!("parse_encode_loop", "instruction-decoder", false, || {
        let loop_bytes = vec![
            0x03, 0x7F, // loop with i32 return type
            0x41, 0x01, // i32.const 1
            0x0B, // end
        ];
        let (loop_instr, loop_bytes_read) =
            parse_instruction(&loop_bytes).map_err(|e| format!("Failed to parse loop: {:?}", e))?;

        assert_eq_test!(loop_bytes_read, loop_bytes.len(), "Should read all bytes");

        let encoded_loop = encode_instruction(&loop_instr)
            .map_err(|e| format!("Failed to encode loop: {:?}", e))?;

        assert_eq_test!(encoded_loop, loop_bytes, "Encoded bytes should match original");

        Ok(())
    });

    // Test if instruction
    register_test!("parse_encode_if", "instruction-decoder", false, || {
        let if_bytes = vec![
            0x04, 0x40, // if with empty block type
            0x41, 0x01, // i32.const 1
            0x05, // else
            0x41, 0x00, // i32.const 0
            0x0B, // end
        ];
        let (if_instr, if_bytes_read) =
            parse_instruction(&if_bytes).map_err(|e| format!("Failed to parse if: {:?}", e))?;

        assert_eq_test!(if_bytes_read, if_bytes.len(), "Should read all bytes");

        let encoded_if =
            encode_instruction(&if_instr).map_err(|e| format!("Failed to encode if: {:?}", e))?;

        assert_eq_test!(encoded_if, if_bytes, "Encoded bytes should match original");

        Ok(())
    });

    // Test br_table instruction
    register_test!("parse_encode_br_table", "instruction-decoder", false, || {
        let br_table_bytes = vec![
            0x0E, // br_table
            0x02, // count = 2
            0x00, // label 0
            0x01, // label 1
            0x02, // default label 2
        ];
        let (br_table_instr, br_table_bytes_read) = parse_instruction(&br_table_bytes)
            .map_err(|e| format!("Failed to parse br_table: {:?}", e))?;

        assert_eq_test!(br_table_bytes_read, br_table_bytes.len(), "Should read all bytes");

        let encoded_br_table = encode_instruction(&br_table_instr)
            .map_err(|e| format!("Failed to encode br_table: {:?}", e))?;

        assert_eq_test!(encoded_br_table, br_table_bytes, "Encoded bytes should match original");

        Ok(())
    });

    // Test nested blocks
    register_test!("parse_encode_nested_blocks", "instruction-decoder", false, || {
        let nested_bytes = vec![
            0x02, 0x40, // outer block
            0x02, 0x40, // inner block
            0x0B, // inner end
            0x0B, // outer end
        ];
        let (nested_instr, nested_bytes_read) = parse_instruction(&nested_bytes)
            .map_err(|e| format!("Failed to parse nested blocks: {:?}", e))?;

        assert_eq_test!(nested_bytes_read, nested_bytes.len(), "Should read all bytes");

        let encoded_nested = encode_instruction(&nested_instr)
            .map_err(|e| format!("Failed to encode nested blocks: {:?}", e))?;

        assert_eq_test!(encoded_nested, nested_bytes, "Encoded bytes should match original");

        Ok(())
    });
}
