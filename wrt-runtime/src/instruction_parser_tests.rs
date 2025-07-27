//! Tests for the instruction parser module
//!
//! This module tests the bytecode parsing functionality that converts
//! raw WebAssembly bytecode into typed instructions.

#[cfg(test)]
mod tests {
    use crate::instruction_parser::{parse_instruction, InstructionContext};
    use wrt_foundation::types::Instruction;
    
    // Mock context for testing
    struct MockContext {
        bytecode: Vec<u8>,
        position: usize,
    }
    
    impl InstructionContext for MockContext {
        fn read_u8(&mut self) -> Result<u8, crate::prelude::Error> {
            if self.position >= self.bytecode.len() {
                return Err(crate::prelude::Error::runtime_execution_error("Unexpected end of bytecode";
            }
            let byte = self.bytecode[self.position];
            self.position += 1;
            Ok(byte)
        }
        
        fn read_u32_leb128(&mut self) -> Result<u32, crate::prelude::Error> {
            // Simplified LEB128 decoding for tests
            let mut result = 0u32;
            let mut shift = 0;
            
            loop {
                let byte = self.read_u8()?;
                result |= ((byte & 0x7F) as u32) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
                if shift >= 32 {
                    return Err(crate::prelude::Error::new(
                        crate::prelude::ErrorCategory::Parse,
                        wrt_error::codes::PARSE_ERROR,
                        "LEB128 integer too large";
                }
            }
            
            Ok(result)
        }
        
        fn read_i32_leb128(&mut self) -> Result<i32, crate::prelude::Error> {
            // Simplified signed LEB128 decoding for tests
            let unsigned = self.read_u32_leb128()?;
            Ok(unsigned as i32)
        }
        
        fn read_i64_leb128(&mut self) -> Result<i64, crate::prelude::Error> {
            // Simplified for tests
            let unsigned = self.read_u32_leb128()?;
            Ok(unsigned as i64)
        }
        
        fn read_f32(&mut self) -> Result<f32, crate::prelude::Error> {
            let mut bytes = [0u8; 4];
            for byte in &mut bytes {
                *byte = self.read_u8()?;
            }
            Ok(f32::from_le_bytes(bytes))
        }
        
        fn read_f64(&mut self) -> Result<f64, crate::prelude::Error> {
            let mut bytes = [0u8; 8];
            for byte in &mut bytes {
                *byte = self.read_u8()?;
            }
            Ok(f64::from_le_bytes(bytes))
        }
    }
    
    #[test]
    fn test_parse_nop() {
        let mut ctx = MockContext {
            bytecode: vec![0x01], // nop
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        assert!(matches!(inst, Instruction::Nop);
    }
    
    #[test]
    fn test_parse_unreachable() {
        let mut ctx = MockContext {
            bytecode: vec![0x00], // unreachable
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        assert!(matches!(inst, Instruction::Unreachable);
    }
    
    #[test]
    fn test_parse_i32_const() {
        let mut ctx = MockContext {
            bytecode: vec![0x41, 0x7F], // i32.const 127
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        match inst {
            Instruction::I32Const(val) => assert_eq!(val, 127),
            _ => panic!("Expected I32Const instruction"),
        }
    }
    
    #[test]
    fn test_parse_i64_const() {
        let mut ctx = MockContext {
            bytecode: vec![0x42, 0x64], // i64.const 100
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        match inst {
            Instruction::I64Const(val) => assert_eq!(val, 100),
            _ => panic!("Expected I64Const instruction"),
        }
    }
    
    #[test]
    fn test_parse_local_get() {
        let mut ctx = MockContext {
            bytecode: vec![0x20, 0x02], // local.get 2
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        match inst {
            Instruction::LocalGet(idx) => assert_eq!(idx, 2),
            _ => panic!("Expected LocalGet instruction"),
        }
    }
    
    #[test]
    fn test_parse_local_set() {
        let mut ctx = MockContext {
            bytecode: vec![0x21, 0x03], // local.set 3
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        match inst {
            Instruction::LocalSet(idx) => assert_eq!(idx, 3),
            _ => panic!("Expected LocalSet instruction"),
        }
    }
    
    #[test]
    fn test_parse_i32_add() {
        let mut ctx = MockContext {
            bytecode: vec![0x6A], // i32.add
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        assert!(matches!(inst, Instruction::I32Add);
    }
    
    #[test]
    fn test_parse_return() {
        let mut ctx = MockContext {
            bytecode: vec![0x0F], // return
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        assert!(matches!(inst, Instruction::Return);
    }
    
    #[test]
    fn test_parse_end() {
        let mut ctx = MockContext {
            bytecode: vec![0x0B], // end
            position: 0,
        };
        
        let inst = parse_instruction(&mut ctx).unwrap();
        assert!(matches!(inst, Instruction::End);
    }
    
    #[test]
    fn test_parse_unknown_opcode() {
        let mut ctx = MockContext {
            bytecode: vec![0xFF], // Invalid opcode
            position: 0,
        };
        
        let result = parse_instruction(&mut ctx;
        assert!(result.is_err();
    }
    
    #[test]
    fn test_parse_empty_bytecode() {
        let mut ctx = MockContext {
            bytecode: vec![],
            position: 0,
        };
        
        let result = parse_instruction(&mut ctx;
        assert!(result.is_err();
    }
}