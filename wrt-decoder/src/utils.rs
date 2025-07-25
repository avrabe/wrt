// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Common utilities for WebAssembly parsing
//!
//! This module contains shared functionality used by both the core WebAssembly
//! module parser and the Component Model parser.

use wrt_error::{codes, Error, ErrorCategory, Result};
use wrt_format::binary::{WASM_MAGIC, WASM_VERSION};

use crate::prelude::{is_valid_wasm_header, read_name, String};

/// Read a WebAssembly name string from binary data
#[cfg(feature = "std")]
pub fn read_name_as_string(data: &[u8], offset: usize) -> Result<(String, usize)> {
    // There's no decode_string in wrt-format, so we use read_name and convert to a
    // String We could use read_string directly, but keeping this function for
    // backward compatibility
    let (name_bytes, bytes_read) = read_name(data, offset)?;

    // Convert the bytes to a string
    let name = match core::str::from_utf8(name_bytes) {
        #[cfg(feature = "std")]
        Ok(s) => std::string::ToString::to_string(s),
        #[cfg(not(feature = "std"))]
        Ok(s) => {
            use wrt_foundation::BoundedString;
            BoundedString::from_str(s).map_err(|_| Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "String too long for bounded storage",
            ))?
        },
        Err(_) => {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Invalid UTF-8 in name",
            ))
        }
    };

    Ok((name, bytes_read))
}

/// Verify WebAssembly binary header
pub fn verify_binary_header(data: &[u8]) -> Result<()> {
    // Use wrt-format's is_valid_wasm_header function
    if !is_valid_wasm_header(data) {
        if data.len() < 8 {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "WebAssembly binary too short",
            ));
        }

        if data[0..4] != WASM_MAGIC {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Invalid WebAssembly magic number",
            ));
        }

        return Err(Error::new(
            ErrorCategory::Parse,
            codes::INVALID_VERSION,
            "Unsupported WebAssembly version",
        ));
    }

    Ok(())
}

/// Calculate the size of a LEB128 encoded u32 value
pub fn varuint_size(value: u32) -> usize {
    let mut size = 1;
    let mut val = value >> 7;
    while val != 0 {
        size += 1;
        val >>= 7;
    }
    size
}

/// Detect if a binary is a WebAssembly component or core module
pub fn detect_binary_type(data: &[u8]) -> Result<BinaryType> {
    if data.len() < 8 {
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::PARSE_ERROR,
            "Binary too short to detect type",
        ));
    }

    // Check magic number
    match &data[0..4] {
        // WebAssembly module magic "\0asm"
        magic if magic == WASM_MAGIC => {
            // Check version
            if data[4..8] == WASM_VERSION {
                Ok(BinaryType::CoreModule)
            } else {
                Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VERSION,
                    "Unsupported WebAssembly version",
                ))
            }
        }
        // Component magic "\0cmp"
        [0x00, 0x63, 0x6D, 0x70] => {
            if data[4..8] == [0x01, 0x00, 0x00, 0x00] {
                Ok(BinaryType::Component)
            } else {
                Err(Error::new(
                    ErrorCategory::Parse,
                    codes::INVALID_VERSION,
                    "Unsupported Component version",
                ))
            }
        }
        _ => Err(Error::new(ErrorCategory::Parse, codes::PARSE_ERROR, "Invalid binary format")),
    }
}

/// The type of WebAssembly binary
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryType {
    /// Core WebAssembly module
    CoreModule,
    /// WebAssembly Component Model component
    Component,
}
