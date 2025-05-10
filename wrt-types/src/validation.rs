// WRT - wrt-types
// Module: Validation Utilities
// SW-REQ-ID: REQ_SAFETY_VALIDATION_001 // Placeholder ID, to be confirmed
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Validation infrastructure for bounded collections
//!
//! This module provides traits and utilities for implementing validation
//! in bounded collections and other safety-critical data structures.

// Conditionally import from std or core

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::string::String;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::format;

use crate::verification::{Checksum, VerificationLevel};

/// Trait for types that can be validated for data integrity
///
/// This trait is implemented by collection types that need to verify
/// their internal state as part of functional safety requirements.
pub trait Validatable {
    /// The error type returned when validation fails
    type Error;

    /// Performs validation on this object
    ///
    /// Returns Ok(()) if validation passes, or an error describing
    /// what validation check failed.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if validation fails.
    fn validate(&self) -> core::result::Result<(), Self::Error>;

    /// Get the validation level this object is configured with
    fn validation_level(&self) -> VerificationLevel;

    /// Set the validation level for this object
    fn set_validation_level(&mut self, level: VerificationLevel);
}

/// Trait for types that maintain checksums for validation
pub trait Checksummed {
    /// Get the current checksum for this object
    fn checksum(&self) -> Checksum;

    /// Force recalculation of the object's checksum
    ///
    /// This is useful when verification level changes from None
    /// or after operations that bypass normal checksum updates.
    fn recalculate_checksum(&mut self);

    /// Verify the integrity by comparing stored vs calculated checksums
    ///
    /// Returns true if checksums match, indicating data integrity.
    fn verify_checksum(&self) -> bool;
}

/// Trait for types with bounded capacity
pub trait BoundedCapacity {
    /// Get the maximum capacity this container can hold
    fn capacity(&self) -> usize;

    /// Get the current number of elements in the container
    fn len(&self) -> usize;

    /// Check if the container is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the container is at maximum capacity
    fn is_full(&self) -> bool {
        self.len() >= self.capacity()
    }

    /// Get the remaining capacity for this container
    fn remaining_capacity(&self) -> usize {
        self.capacity().saturating_sub(self.len())
    }
}

/// Helper for determining if validation should be performed
///
/// This function encapsulates the logic for deciding when to perform
/// validation based on the verification level and operation importance.
///
/// # Errors
///
/// Returns `Self::Error` if validation fails.
#[must_use]
pub fn should_validate(level: VerificationLevel, operation_importance: u8) -> bool {
    level.should_verify(operation_importance)
}

/// Helper for determining if full redundant validation should be performed
///
/// This function encapsulates the logic for deciding when to perform
/// redundant validation checks based on the verification level.
///
/// # Errors
///
/// Returns `Self::Error` if validation fails.
#[must_use]
pub fn should_validate_redundant(level: VerificationLevel) -> bool {
    level.should_verify_redundant()
}

/// Standard importance values for different operation types
///
/// These constants provide standardized importance values to use
/// when determining validation frequency based on operation type.
pub mod importance {
    /// Importance for read operations (get, peek, etc.)
    pub const READ: u8 = 100;

    /// Importance for mutation operations (insert, push, etc.)
    pub const MUTATION: u8 = 150;

    /// Importance for critical operations (security-sensitive)
    pub const CRITICAL: u8 = 200;

    /// Importance for initialization operations
    pub const INITIALIZATION: u8 = 180;

    /// Importance for internal state management
    pub const INTERNAL: u8 = 120;
}

/// Helper to calculate a checksum for a keyed collection
///
/// This function computes a checksum for collections with key-value pairs,
/// deterministically ordered by key to ensure consistent checksums.
pub fn calculate_keyed_checksum<K, V>(items: &[(K, V)]) -> Checksum
where
    K: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    let mut checksum = Checksum::new();
    for (key, value) in items {
        checksum.update_slice(key.as_ref());
        checksum.update_slice(value.as_ref());
    }
    checksum
}

/// Validation result type to centralize error handling
#[cfg(feature = "alloc")]
pub type ValidationResult = Result<(), String>;

/// Helper to validate a checksum against an expected value
#[cfg(feature = "alloc")]
pub fn validate_checksum(
    actual: Checksum,
    expected: Checksum,
    description: &str,
) -> ValidationResult {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "Checksum mismatch in {}: expected {}, actual {}",
            description, expected, actual
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_should_validate() {
        // None level should never validate
        assert!(!should_validate(VerificationLevel::None, importance::CRITICAL));

        // Standard level should validate based on importance
        assert!(should_validate(VerificationLevel::Standard, importance::CRITICAL));
        assert!(should_validate(VerificationLevel::Standard, importance::MUTATION));
        assert!(should_validate(VerificationLevel::Standard, importance::READ));

        // Full level should always validate
        assert!(should_validate(VerificationLevel::Full, importance::READ));
        assert!(should_validate(VerificationLevel::Full, 1)); // Even low importance
    }

    #[test]
    fn test_should_validate_redundant() {
        // Only Full level should do redundant validation
        assert!(!should_validate_redundant(VerificationLevel::None));
        assert!(!should_validate_redundant(VerificationLevel::Sampling));
        assert!(!should_validate_redundant(VerificationLevel::Standard));
        assert!(should_validate_redundant(VerificationLevel::Full));
    }

    #[test]
    fn test_calculate_keyed_checksum() {
        let items = vec![
            ("key1".as_bytes(), "value1".as_bytes()),
            ("key2".as_bytes(), "value2".as_bytes()),
        ];

        let checksum = calculate_keyed_checksum(&items);

        // Calculate manually to verify
        let mut expected = Checksum::new();
        expected.update_slice("key1".as_bytes());
        expected.update_slice("value1".as_bytes());
        expected.update_slice("key2".as_bytes());
        expected.update_slice("value2".as_bytes());

        assert_eq!(checksum, expected);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_validate_checksum() {
        let checksum1 = Checksum::new();
        let checksum2 = {
            let mut cs = Checksum::new();
            cs.update(1);
            cs
        };

        // Same checksums should validate
        assert!(validate_checksum(checksum1, checksum1, "test").is_ok());

        // Different checksums should fail
        assert!(validate_checksum(checksum1, checksum2, "test").is_err());
    }
}
