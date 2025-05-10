// WRT - wrt-logging
// SW-REQ-ID: [SW-REQ-ID-wrt-logging]
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)] // Rule 2

//! # WRT Logging
//!
//! Logging infrastructure for the WebAssembly Runtime (WRT).
//!
//! This crate provides logging functionality for WebAssembly components,
//! allowing components to log messages to the host environment. It extends
//! the wrt-host crate with logging-specific capabilities and works in both
//! standard and `no_std` environments.
//!
//! ## Features
//!
//! - **Component Logging**: Enable WebAssembly components to log messages to the host
//! - **Log Levels**: Support for different log levels (Debug, Info, Warning, Error)
//! - **Custom Handlers**: Extensible architecture for custom log handlers
//! - **Std/No-std Support**: Works in both standard and `no_std` environments
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use wrt_logging::{LogHandler, LogLevel, LogOperation};
//! use wrt_host::CallbackRegistry;
//!
//! // Create a custom log handler
//! struct MyLogHandler;
//!
//! impl LogHandler for MyLogHandler {
//!     fn handle_log(&self, level: LogLevel, message: &str) -> wrt_logging::Result<()> {
//!         match level {
//!             LogLevel::Debug => println!("DEBUG: {}", message),
//!             LogLevel::Info => println!("INFO: {}", message),
//!             LogLevel::Warning => println!("WARN: {}", message),
//!             LogLevel::Error => println!("ERROR: {}", message),
//!         }
//!         Ok(())
//!     }
//! }
//!
//! // Register the log handler with a component
//! fn register_logging(registry: &mut CallbackRegistry) {
//!     let handler = Box::new(MyLogHandler);
//!     registry.register_log_handler(handler);
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "kani", feature(kani))]
#![warn(clippy::missing_panics_doc)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// When no_std but alloc is available
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{boxed::Box, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{boxed::Box, string::String, vec::Vec};

// Reexports for convenience
pub use wrt_error::{Error, Result};
pub use wrt_host::CallbackRegistry;

/// Logging handlers for processing log messages.
///
/// This module contains the trait and implementations for log handlers,
/// which process log messages from WebAssembly components.
pub mod handler;

/// Log level definitions for categorizing message severity.
///
/// This module defines the different log levels supported by the logging
/// infrastructure, from Debug (lowest severity) to Error (highest severity).
pub mod level;

/// Log operation data structures and utilities.
///
/// This module contains the types and functions for creating and working
/// with log operations, which encapsulate log messages and their metadata.
pub mod operation;

// Reexport types
pub use handler::{LogHandler, LoggingExt};
pub use level::LogLevel;
pub use operation::LogOperation;

// Include verification module when the kani feature is enabled
#[cfg(feature = "kani")]
#[cfg_attr(docsrs, doc(cfg(feature = "kani")))]
pub mod verify;
