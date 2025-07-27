// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Example showcasing different feature configurations of wrt-debug
//!
//! This example demonstrates how the same code can be compiled with different
//! feature sets, allowing users to opt in or out of debug functionality as
//! needed.

#![no_std]
#![no_main]

use wrt_debug::prelude::*;

const MODULE_BYTES: &[u8] = &[
    0x00, 0x61, 0x73, 0x6D, // \0asm
    0x01, 0x00, 0x00, 0x00, // version 1
];

/// Always available - basic debug info management
#[no_mangle]
pub extern "C" fn basic_debug_functionality() -> u32 {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;

    // Section registration always works
    debug_info.add_section(".debug_line", 0x1000, 0x500;
    debug_info.add_section(".debug_info", 0x1500, 0x800;
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;

    // Basic query always works
    let has_debug = debug_info.has_debug_info);

    if has_debug {
        1
    } else {
        0
    }
}

/// Available only with line-info feature (default)
#[cfg(feature = "line-info")]
#[no_mangle]
pub extern "C" fn line_info_functionality() -> u32 {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;
    debug_info.add_section(".debug_line", 0x1000, 0x500;

    match debug_info.find_line_info(0x42) {
        Ok(Some(line_info)) => line_info.line,
        Ok(None) => 0,
        Err(_) => 0xFFFFFFFF,
    }
}

/// Available only with debug-info feature
#[cfg(feature = "debug-info")]
#[no_mangle]
pub extern "C" fn debug_info_functionality() -> u32 {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;
    debug_info.add_section(".debug_info", 0x1500, 0x800;
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;

    match debug_info.init_info_parser() {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Available only with function-info feature
#[cfg(feature = "function-info")]
#[no_mangle]
pub extern "C" fn function_info_functionality() -> u32 {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;
    debug_info.add_section(".debug_info", 0x1500, 0x800;
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;

    if debug_info.init_info_parser().is_ok() {
        if let Some(func_info) = debug_info.find_function_info(0x42) {
            return func_info.low_pc;
        }
    }
    0
}

/// Feature detection at compile time
#[no_mangle]
pub extern "C" fn get_enabled_features() -> u32 {
    let mut features = 0;

    #[cfg(feature = "line-info")]
    {
        features |= 0x01;
    }

    #[cfg(feature = "debug-info")]
    {
        features |= 0x02;
    }

    #[cfg(feature = "function-info")]
    {
        features |= 0x04;
    }

    #[cfg(feature = "abbrev")]
    {
        features |= 0x08;
    }

    features
}

/// Conditional feature usage pattern
#[no_mangle]
pub extern "C" fn conditional_debug_usage() -> u32 {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;
    debug_info.add_section(".debug_line", 0x1000, 0x500;

    // Strategy 1: Use cfg attributes for different code paths
    #[cfg(feature = "line-info")]
    {
        if let Ok(Some(line_info)) = debug_info.find_line_info(0x42) {
            return line_info.line;
        }
    }

    #[cfg(not(feature = "line-info"))]
    {
        // Fallback behavior when line-info is not available
        return 0;
    }

    0
}

/// Example of graceful degradation
#[no_mangle]
pub extern "C" fn graceful_degradation_example() -> u32 {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;
    debug_info.add_section(".debug_line", 0x1000, 0x500;
    debug_info.add_section(".debug_info", 0x1500, 0x800;

    // Try to get the most detailed information available
    #[cfg(feature = "function-info")]
    {
        debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;
        if debug_info.init_info_parser().is_ok() {
            if let Some(func_info) = debug_info.find_function_info(0x42) {
                return 3; // Most detailed info available
            }
        }
    }

    #[cfg(feature = "line-info")]
    {
        if let Ok(Some(_line_info)) = debug_info.find_line_info(0x42) {
            return 2; // Line info available
        }
    }

    // Basic functionality always available
    if debug_info.has_debug_info() {
        return 1; // Some debug info available
    }

    0 // No debug info
}

// Panic handler removed - provided by wrt-platform crate
