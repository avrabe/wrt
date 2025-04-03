//! WebAssembly instruction implementations
//!
//! This module contains implementations for all WebAssembly instructions,
//! organized into submodules by instruction category.

// Only include the imports actually needed in this file

pub mod arithmetic;
mod bit_counting;
pub mod comparison;
mod control;
pub mod executor;
mod instruction_type;
mod memory;
pub mod numeric;
mod parametric;
pub mod simd;
mod table;
mod variable;

pub mod types {
    pub use crate::types::BlockType;
}

// Export only the instruction type
pub use instruction_type::Instruction;

// Re-export the InstructionExecutor trait
pub use crate::behavior::InstructionExecutor;
