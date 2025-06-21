// WRT - wrt-decoder
// Module: Optimized String Processing
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Optimized string processing utilities that avoid unnecessary allocations

use crate::prelude::read_name;
#[cfg(feature = "std")]
use std::string::String;
#[cfg(not(feature = "std"))]
use wrt_foundation::BoundedString;
use core::str;
#[cfg(not(any(feature = "std")))]
use wrt_error::codes;
use wrt_error::{errors::codes as error_codes, Error, ErrorCategory, Result};

/// Binary std/no_std choice
#[cfg(feature = "std")]
pub fn parse_utf8_string_inplace(bytes: &[u8], offset: usize) -> Result<(std::string::String, usize)> {
    let (name_bytes, new_offset) = read_name(bytes, offset)?;

    // Validate UTF-8 without creating intermediate Vec
    let string_str = str::from_utf8(name_bytes).map_err(|_| {
        Error::new(
            ErrorCategory::Parse,
            error_codes::INVALID_UTF8_ENCODING,
            "Invalid UTF-8 encoding",
        )
    })?;

    Ok((std::string::String::from(string_str), new_offset))
}

#[cfg(not(feature = "std"))]
pub fn parse_utf8_string_inplace(bytes: &[u8], offset: usize) -> Result<(wrt_foundation::BoundedString<256, wrt_foundation::NoStdProvider<4096>>, usize)> {
    let (name_bytes, new_offset) = read_name(bytes, offset)?;

    // Validate UTF-8 without creating intermediate Vec
    let string_str = str::from_utf8(name_bytes).map_err(|_| {
        Error::new(
            ErrorCategory::Parse,
            error_codes::INVALID_UTF8_ENCODING,
            "Invalid UTF-8 encoding",
        )
    })?;

    let (name_bytes, new_offset) = read_name(bytes, offset)?;

    // Validate UTF-8 without creating intermediate Vec
    let string_str = str::from_utf8(name_bytes).map_err(|_| {
        Error::new(
            ErrorCategory::Parse,
            error_codes::INVALID_UTF8_ENCODING,
            "Invalid UTF-8 encoding",
        )
    })?;

    use wrt_foundation::NoStdProvider;
    let provider = NoStdProvider::default();
    let bounded_string =
        wrt_foundation::BoundedString::from_str(string_str, provider).map_err(|_| {
            Error::new(
                ErrorCategory::Parse,
                error_codes::CAPACITY_EXCEEDED,
                "String too long for bounded storage",
            )
        })?;
    Ok((bounded_string, new_offset))
}

/// Binary std/no_std choice
pub fn validate_utf8_name(bytes: &[u8], offset: usize) -> Result<(&str, usize)> {
    let (name_bytes, new_offset) = read_name(bytes, offset)?;

    let string_str = str::from_utf8(name_bytes).map_err(|_| {
        Error::new(
            ErrorCategory::Parse,
            error_codes::INVALID_UTF8_ENCODING,
            "Invalid UTF-8 encoding",
        )
    })?;

    Ok((string_str, new_offset))
}

/// Copy validated UTF-8 to a bounded buffer in no_std environments
#[cfg(not(any(feature = "std")))]
pub fn copy_utf8_to_bounded(
    bytes: &[u8],
    offset: usize,
    buffer: &mut [u8],
) -> Result<(usize, usize)> {
    let (name_bytes, new_offset) = read_name(bytes, offset)?;

    // Validate UTF-8 first
    str::from_utf8(name_bytes).map_err(|_| {
        Error::new(
            ErrorCategory::Parse,
            error_codes::INVALID_UTF8_ENCODING,
            "Invalid UTF-8 encoding",
        )
    })?;

    // Check if it fits in the buffer
    if name_bytes.len() > buffer.len() {
        return Err(Error::new(
            ErrorCategory::Parse,
            codes::BUFFER_TOO_SMALL,
            "String too long for buffer",
        ));
    }

    // Copy to buffer
    buffer[..name_bytes.len()].copy_from_slice(name_bytes);
    Ok((name_bytes.len(), new_offset))
}
