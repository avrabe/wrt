//! Pure format representation types
//!
//! This module contains pure binary format representations that contain
//! only the structural information needed for parsing, without any
//! runtime initialization logic.

use crate::prelude::*;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Pure data segment mode (format representation only)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PureDataMode {
    /// Active data segment reference (format-only, contains indices and expression data)
    Active {
        /// Memory index reference
        memory_index: u32,
        /// Offset expression length (for parsing validation)
        offset_expr_len: u32,
    },
    /// Passive data segment (used with memory.init)
    Passive,
}

impl Default for PureDataMode {
    fn default() -> Self {
        Self::Passive
    }
}

/// Pure element segment mode (format representation only)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PureElementMode {
    /// Active element segment reference (format-only, contains indices and expression data)
    Active {
        /// Table index reference
        table_index: u32,
        /// Offset expression length (for parsing validation)
        offset_expr_len: u32,
    },
    /// Passive element segment (used with table.init)
    Passive,
    /// Declared element segment (available for linking but not runtime init)
    Declared,
}

impl Default for PureElementMode {
    fn default() -> Self {
        Self::Passive
    }
}

/// Pure data segment (format representation only)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureDataSegment {
    /// Data mode (pure format representation)
    pub mode: PureDataMode,
    /// Raw offset expression bytes (format-only, runtime interprets)
    pub offset_expr_bytes: Vec<u8>,
    /// Data bytes
    pub data_bytes: Vec<u8>,
}

/// Pure element segment (format representation only)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureElementSegment {
    /// Element mode (pure format representation)
    pub mode: PureElementMode,
    /// Element type (funcref, externref, etc.)
    pub element_type: crate::types::RefType,
    /// Raw offset expression bytes (format-only, runtime interprets)
    pub offset_expr_bytes: Vec<u8>,
    /// Element initialization data (indices or expression bytes)
    pub init_data: PureElementInit,
}

/// Pure element initialization data (format representation only)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PureElementInit {
    /// Function indices
    FunctionIndices(Vec<u32>),
    /// Raw expression bytes (runtime interprets)
    ExpressionBytes(Vec<Vec<u8>>),
}

impl Default for PureDataSegment {
    fn default() -> Self {
        Self {
            mode: PureDataMode::default(),
            offset_expr_bytes: Vec::new(),
            data_bytes: Vec::new(),
        }
    }
}

impl Default for PureElementSegment {
    fn default() -> Self {
        Self {
            mode: PureElementMode::default(),
            element_type: crate::types::RefType::Funcref,
            offset_expr_bytes: Vec::new(),
            init_data: PureElementInit::FunctionIndices(Vec::new()),
        }
    }
}

impl Default for PureElementInit {
    fn default() -> Self {
        Self::FunctionIndices(Vec::new())
    }
}

impl PureDataSegment {
    /// Create new active data segment
    pub fn new_active(memory_index: u32, offset_expr: Vec<u8>, data: Vec<u8>) -> Self {
        Self {
            mode: PureDataMode::Active {
                memory_index,
                offset_expr_len: offset_expr.len() as u32,
            },
            offset_expr_bytes: offset_expr,
            data_bytes: data,
        }
    }
    
    /// Create new passive data segment
    pub fn new_passive(data: Vec<u8>) -> Self {
        Self {
            mode: PureDataMode::Passive,
            offset_expr_bytes: Vec::new(),
            data_bytes: data,
        }
    }
    
    /// Check if this is an active segment
    pub fn is_active(&self) -> bool {
        matches!(self.mode, PureDataMode::Active { .. })
    }
    
    /// Get memory index if active
    pub fn memory_index(&self) -> Option<u32> {
        match self.mode {
            PureDataMode::Active { memory_index, .. } => Some(memory_index),
            PureDataMode::Passive => None,
        }
    }
}

impl PureElementSegment {
    /// Create new active element segment
    pub fn new_active(
        table_index: u32,
        element_type: crate::types::RefType,
        offset_expr: Vec<u8>,
        init_data: PureElementInit,
    ) -> Self {
        Self {
            mode: PureElementMode::Active {
                table_index,
                offset_expr_len: offset_expr.len() as u32,
            },
            element_type,
            offset_expr_bytes: offset_expr,
            init_data,
        }
    }
    
    /// Create new passive element segment
    pub fn new_passive(element_type: crate::types::RefType, init_data: PureElementInit) -> Self {
        Self {
            mode: PureElementMode::Passive,
            element_type,
            offset_expr_bytes: Vec::new(),
            init_data,
        }
    }
    
    /// Create new declared element segment
    pub fn new_declared(element_type: crate::types::RefType, init_data: PureElementInit) -> Self {
        Self {
            mode: PureElementMode::Declared,
            element_type,
            offset_expr_bytes: Vec::new(),
            init_data,
        }
    }
    
    /// Check if this is an active segment
    pub fn is_active(&self) -> bool {
        matches!(self.mode, PureElementMode::Active { .. })
    }
    
    /// Get table index if active
    pub fn table_index(&self) -> Option<u32> {
        match self.mode {
            PureElementMode::Active { table_index, .. } => Some(table_index),
            _ => None,
        }
    }
}