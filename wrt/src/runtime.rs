            Instruction::I64Store8(align, offset) => {
                instruction::memory::i64_store8(stack, frame, offset, align)?
            }
            Instruction::I64Store16(align, offset) => {
                instruction::memory::i64_store16(stack, frame, offset, align)?
            }
            Instruction::I64Store32(align, offset) => {
                instruction::memory::i64_store32(stack, frame, offset, align)?
            } 