// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! DWARF debug information support for WebAssembly Runtime (WRT)
//!
//! This crate provides zero-allocation DWARF debug information parsing
//! for WebAssembly modules in no_std environments.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

// Import std when available
#[cfg(feature = "std")]
extern crate std;

// Import alloc when available
#[cfg(feature = "alloc")]
extern crate alloc;

// Re-export commonly used types based on features
#[cfg(feature = "abbrev")]
pub use abbrev::{Abbreviation, AbbreviationTable, AttributeForm, AttributeSpec};
pub use cursor::DwarfCursor;
pub use file_table::{FileEntry, FilePath, FileTable};
#[cfg(feature = "debug-info")]
pub use info::{CompilationUnitHeader, DebugInfoParser, FunctionInfo};
#[cfg(feature = "line-info")]
pub use line_info::{LineInfo, LineNumberState};
pub use parameter::{BasicType, InlinedFunction, Parameter, ParameterList};
// Runtime debug exports
#[cfg(feature = "runtime-inspection")]
pub use runtime_api::{
    Breakpoint, BreakpointCondition, BreakpointId, DebugAction, DebugError, DebugMemory,
    DebuggableRuntime, DwarfLocation, LiveVariable, RuntimeDebugger, RuntimeState, VariableValue,
};
#[cfg(feature = "runtime-breakpoints")]
pub use runtime_break::{BreakpointManager, DefaultDebugger};
#[cfg(feature = "runtime-memory")]
pub use runtime_memory::{
    CStringView, HeapAllocation, HeapStats, MemoryDump, MemoryInspector, MemoryRegion,
    MemoryRegionType, MemoryView, StackAnalysis,
};
#[cfg(feature = "runtime-stepping")]
pub use runtime_step::{StepController, StepMode, SteppingDebugger};
#[cfg(feature = "runtime-variables")]
pub use runtime_vars::{ValueDisplay, VariableDefinition, VariableInspector, VariableScope};
#[cfg(feature = "line-info")]
pub use stack_trace::{StackFrame, StackTrace, StackTraceBuilder};
pub use strings::{DebugString, StringTable};
pub use types::{DebugSection, DebugSectionRef, DwarfSections};
use wrt_error::{codes, Error, ErrorCategory, Result};
use wrt_foundation::{
    bounded::{BoundedVec, MAX_DWARF_ABBREV_CACHE},
    prelude::*,
    NoStdProvider,
};

#[cfg(feature = "abbrev")]
mod abbrev;
mod cursor;
mod error;
mod file_table;
#[cfg(feature = "debug-info")]
mod info;
#[cfg(feature = "line-info")]
mod line_info;
mod parameter;
#[cfg(feature = "line-info")]
mod stack_trace;
mod strings;
mod types;

// Runtime debug modules
#[cfg(feature = "runtime-inspection")]
pub mod runtime_api;
#[cfg(feature = "runtime-breakpoints")]
mod runtime_break;
#[cfg(feature = "runtime-memory")]
mod runtime_memory;
#[cfg(feature = "runtime-stepping")]
mod runtime_step;
#[cfg(feature = "runtime-variables")]
mod runtime_vars;

#[cfg(test)]
mod test;

/// DWARF debug information without allocation
pub struct DwarfDebugInfo<'a> {
    /// Reference to module bytes for zero-copy parsing
    module_bytes: &'a [u8],

    /// Cached section offsets
    sections: DwarfSections,

    /// Abbreviation cache for performance
    #[cfg(feature = "abbrev")]
    abbrev_cache: BoundedVec<Abbreviation, MAX_DWARF_ABBREV_CACHE, NoStdProvider<{ MAX_DWARF_ABBREV_CACHE * 128 }>>,

    /// Line number state machine
    #[cfg(feature = "line-info")]
    line_state: LineNumberState,

    /// Debug info parser (optional)
    #[cfg(feature = "debug-info")]
    info_parser: Option<DebugInfoParser<'a>>,
}

impl<'a> DwarfDebugInfo<'a> {
    /// Create a new DWARF debug info parser
    pub fn new(module_bytes: &'a [u8]) -> Self {
        Self {
            module_bytes,
            sections: DwarfSections::default(),
            #[cfg(feature = "abbrev")]
            abbrev_cache: BoundedVec::new(NoStdProvider::<{ MAX_DWARF_ABBREV_CACHE * 128 }>::new()),
            #[cfg(feature = "line-info")]
            line_state: LineNumberState::new(),
            #[cfg(feature = "debug-info")]
            info_parser: None,
        }
    }

    /// Register a debug section
    pub fn add_section(&mut self, name: &str, offset: u32, size: u32) {
        match name {
            ".debug_info" => self.sections.debug_info = Some(DebugSectionRef { offset, size }),
            ".debug_abbrev" => self.sections.debug_abbrev = Some(DebugSectionRef { offset, size }),
            ".debug_line" => self.sections.debug_line = Some(DebugSectionRef { offset, size }),
            ".debug_str" => self.sections.debug_str = Some(DebugSectionRef { offset, size }),
            ".debug_line_str" => {
                self.sections.debug_line_str = Some(DebugSectionRef { offset, size })
            }
            _ => {} // Ignore other debug sections for now
        }
    }

    /// Find line information for a given code offset
    #[cfg(feature = "line-info")]
    pub fn find_line_info(&mut self, code_offset: u32) -> Result<Option<LineInfo>> {
        // Get the debug_line section
        let line_section = match self.sections.debug_line {
            Some(ref section) => section,
            None => return Ok(None),
        };

        // Check bounds
        let start = line_section.offset as usize;
        let end = start + line_section.size as usize;
        if end > self.module_bytes.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Debug line section extends beyond module bounds",
            ));
        }

        // Get the debug_line data
        let debug_line_data = &self.module_bytes[start..end];

        // Use the line number state machine to find the line info
        self.line_state.find_line_for_pc(debug_line_data, code_offset)
    }

    /// Check if debug information is available
    pub fn has_debug_info(&self) -> bool {
        #[cfg(feature = "line-info")]
        let has_line = self.sections.debug_line.is_some();
        #[cfg(not(feature = "line-info"))]
        let has_line = false;

        #[cfg(feature = "debug-info")]
        let has_info = self.sections.debug_info.is_some();
        #[cfg(not(feature = "debug-info"))]
        let has_info = false;

        has_line || has_info
    }

    /// Initialize the debug info parser
    #[cfg(feature = "debug-info")]
    pub fn init_info_parser(&mut self) -> Result<()> {
        // Check if we have the required sections
        let info_section = match self.sections.debug_info {
            Some(ref section) => section,
            None => return Ok(()), // No debug_info section, nothing to do
        };

        let abbrev_section = match self.sections.debug_abbrev {
            Some(ref section) => section,
            None => return Ok(()), // No abbreviations, can't parse debug_info
        };

        // Get section data
        let info_start = info_section.offset as usize;
        let info_end = info_start + info_section.size as usize;
        if info_end > self.module_bytes.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Debug info section extends beyond module bounds",
            ));
        }
        let debug_info_data = &self.module_bytes[info_start..info_end];

        let abbrev_start = abbrev_section.offset as usize;
        let abbrev_end = abbrev_start + abbrev_section.size as usize;
        if abbrev_end > self.module_bytes.len() {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Debug abbrev section extends beyond module bounds",
            ));
        }
        let debug_abbrev_data = &self.module_bytes[abbrev_start..abbrev_end];

        // Get optional debug_str section
        let debug_str_data = if let Some(ref str_section) = self.sections.debug_str {
            let str_start = str_section.offset as usize;
            let str_end = str_start + str_section.size as usize;
            if str_end > self.module_bytes.len() {
                return Err(Error::new(
                    ErrorCategory::Parse,
                    codes::PARSE_ERROR,
                    "Debug str section extends beyond module bounds",
                ));
            }
            Some(&self.module_bytes[str_start..str_end])
        } else {
            None
        };

        // Create and initialize parser
        let mut parser = DebugInfoParser::new(debug_info_data, debug_abbrev_data, debug_str_data);
        parser.parse()?;

        self.info_parser = Some(parser);
        Ok(())
    }

    /// Find function information for a given PC
    #[cfg(feature = "function-info")]
    pub fn find_function_info(&self, pc: u32) -> Option<&FunctionInfo<'a>> {
        self.info_parser.as_ref()?.find_function(pc)
    }

    /// Get all parsed functions
    #[cfg(feature = "function-info")]
    pub fn get_functions(&self) -> Option<&[FunctionInfo<'a>]> {
        self.info_parser.as_ref().map(|parser| parser.functions())
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::DwarfDebugInfo;
    #[cfg(feature = "function-info")]
    pub use crate::FunctionInfo;
    #[cfg(feature = "line-info")]
    pub use crate::LineInfo;
}
