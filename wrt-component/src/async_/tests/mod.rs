//! Comprehensive test suite for the WRT async executor system
//!
//! This module contains all tests for the fuel-based async executor and
//! Component Model integration, including:
//! - Unit tests for individual components
//! - Integration tests across all async subsystems
//! - Performance benchmarks and stress tests
//! - ASIL compliance verification
//! - Error handling and fault tolerance tests

pub mod phase3_integration_tests;
pub mod comprehensive_integration_tests;
pub mod performance_benchmarks;
pub mod asil_compliance_tests;
pub mod error_handling_tests;
pub mod fuel_async_integration_tests;

// Re-export test utilities for use in other modules
pub use comprehensive_integration_tests::*;
pub use performance_benchmarks::*;
pub use asil_compliance_tests::*;
pub use error_handling_tests::*;
pub use fuel_async_integration_tests::*;