//! WebAssembly Component Model resource management.
//!
//! This module provides runtime resource handle management for the
//! Component Model, including ownership tracking and lifecycle management.

pub mod handle_table;

pub use handle_table::{
    ResourceHandle, ResourceOwnership, ResourceEntry, ResourceTable,
    MAX_RESOURCES_PER_TYPE,
};