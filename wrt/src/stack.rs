use crate::behavior;
use crate::error::{Error, Result};
use crate::values::Value as ValuesValue;

/// Represents a control flow label on the stack (e.g., for blocks, loops, ifs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The number of values the instruction sequence associated with the label is expected to produce.
    pub arity: usize,
    /// The program counter (instruction index) where execution should resume after the block.
    pub pc: usize,
    /// The program counter for the continuation (e.g., the `else` branch of an `if`).
    pub continuation: usize,
}

impl From<behavior::Label> for Label {
    fn from(label: behavior::Label) -> Self {
        Self {
            arity: label.arity,
            pc: label.pc,
            continuation: label.continuation,
        }
    }
}

/// Trait defining operations for managing the execution stack, including value and label manipulation.
pub trait Stack: behavior::StackBehavior + std::fmt::Debug {
    /// Pushes a control flow label onto the label stack.
    fn push_label(&mut self, label: Label) -> Result<()>;
    /// Pops a control flow label from the label stack.
    fn pop_label(&mut self) -> Result<Label>;
    /// Gets a reference to a label on the stack by its relative index (0 is the top).
    fn get_label(&self, idx: usize) -> Result<&Label>;
    /// Gets a mutable reference to a label on the stack by its relative index.
    fn get_label_mut(&mut self, idx: usize) -> Result<&mut Label>;
    /// Returns the current number of labels on the stack.
    fn labels_len(&self) -> usize;
}

impl Stack for Vec<ValuesValue> {
    fn push_label(&mut self, _label: Label) -> Result<()> {
        Ok(())
    }

    fn pop_label(&mut self) -> Result<Label> {
        Err(Error::InvalidOperation {
            message: "No labels in Vec<Value>".to_string(),
        })
    }

    fn get_label(&self, _idx: usize) -> Result<&Label> {
        Err(Error::InvalidOperation {
            message: "No labels in Vec<Value>".to_string(),
        })
    }

    fn get_label_mut(&mut self, _idx: usize) -> Result<&mut Label> {
        Err(Error::InvalidOperation {
            message: "No labels in Vec<Value>".to_string(),
        })
    }

    fn labels_len(&self) -> usize {
        0
    }
}

impl behavior::StackBehavior for Vec<ValuesValue> {
    fn push(&mut self, value: ValuesValue) -> Result<()> {
        self.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<ValuesValue> {
        self.pop().ok_or(Error::StackUnderflow)
    }

    fn peek(&self) -> Result<&ValuesValue> {
        self.last().ok_or(Error::StackUnderflow)
    }

    fn peek_mut(&mut self) -> Result<&mut ValuesValue> {
        self.last_mut().ok_or(Error::StackUnderflow)
    }

    fn values(&self) -> &[ValuesValue] {
        self
    }

    fn values_mut(&mut self) -> &mut [ValuesValue] {
        self
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn push_label(&mut self, _arity: usize, _pc: usize) {
        // No-op for Vec<ValuesValue>
    }

    fn pop_label(&mut self) -> Result<behavior::Label> {
        Err(Error::InvalidOperation {
            message: "No labels in Vec<Value>".to_string(),
        })
    }

    fn get_label(&self, _idx: usize) -> Option<&behavior::Label> {
        None
    }
}
