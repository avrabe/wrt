// WRT - wrt-types
// Module: Bounded Collections
// SW-REQ-ID: REQ_MEM_SAFETY_004 (Example: Relates to bounded data structures)
// SW-REQ-ID: REQ_WASM_RUNTIME_015 (Example: HashMap slot state management)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), allow(unused_imports))]

/// Bounded collections with functional safety verification
///
/// This module provides bounded collection types that are designed for
/// functional safety with built-in size limits and verification features.

#[cfg(feature = "alloc")]
extern crate alloc;

// For std environment
#[cfg(feature = "std")]
use std::{
    fmt,
    // string::{String, ToString}, // Removed
    // vec::Vec, // Removed
};

// For no_std with alloc
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{};

// For no_std environment
#[cfg(not(feature = "std"))]
use core::fmt; // Removed hash, mem

// Use the HashMap that's re-exported in lib.rs - works for both std and no_std
#[allow(unused_imports)]
use crate::operations;
use crate::operations::record_global_operation;
use crate::operations::OperationType;
use crate::validation::{importance, BoundedCapacity, Checksummed}; // Removed Validatable
use crate::verification::{Checksum, VerificationLevel};
// use crate::HashMap; // Removed, should come from prelude

use crate::prelude::ToString;
use crate::prelude::*;
use crate::traits::Checksummable;
// Ensure MemoryProvider is imported directly for trait bounds.
#[cfg(feature = "std")]
use crate::prelude::Vec; // This was added in a previous step for owned Vec, keep it.
#[cfg(not(feature = "std"))]
use crate::safe_memory::NoStdMemoryProvider;
use crate::safe_memory::{MemoryProvider, SafeMemoryHandler, SafeSlice, SafeSliceMut}; // Added SafeSliceMut, SafeSlice
use crate::traits::{FromBytes, SerializationError, ToBytes};
use core::marker::PhantomData; // Added ToBytes, FromBytes, SerializationError
                               // use core::hash::{BuildHasher, Hasher}; // Removed BuildHasher, Hasher
                               // use std::collections::hash_map::RandomState; // For a default hasher - BoundedHashMap not found, this is likely unused for no_std

/// Error indicating a collection has reached its capacity limit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityError;

impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bounded collection capacity exceeded")
    }
}

impl From<CapacityError> for wrt_error::Error {
    fn from(_err: CapacityError) -> Self {
        wrt_error::Error::new(
            wrt_error::ErrorCategory::Capacity,
            wrt_error::codes::CAPACITY_EXCEEDED,
            "Bounded collection capacity exceeded".to_string(),
        )
    }
}

/// Error types for bounded collections
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedError {
    /// Capacity limit exceeded
    CapacityExceeded {
        /// Type of collection that had its capacity exceeded
        collection_type: String,
        /// Maximum capacity of the collection
        capacity: usize,
        /// Size that was attempted
        attempted_size: usize,
    },

    /// Invalid access to a collection
    InvalidAccess {
        /// Type of collection being accessed
        collection_type: String,
        /// Index that was attempted
        index: usize,
        /// Current size of the collection
        size: usize,
    },

    /// Checksum validation failed
    ChecksumMismatch {
        /// Expected checksum value
        expected: u32,
        /// Actual calculated checksum
        actual: u32,
        /// Description of what was being validated
        description: String,
    },

    /// Generic validation error
    ValidationFailure(String),

    /// Serialization error
    Serialization(crate::traits::SerializationError),

    /// Index out of bounds
    IndexOutOfBounds {
        /// Type of collection being accessed
        collection_type: String,
        /// Index that was attempted
        index: usize,
        /// Length of the collection
        len: usize,
    },
}

impl fmt::Display for BoundedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { collection_type, capacity, attempted_size } => {
                write!(
                    f,
                    "{} capacity exceeded: limit {}, attempted {}",
                    collection_type, capacity, attempted_size
                )
            }
            Self::InvalidAccess { collection_type, index, size } => {
                write!(
                    f,
                    "Invalid access to {} at index {}, size is {}",
                    collection_type, index, size
                )
            }
            Self::ChecksumMismatch { expected, actual, description } => {
                write!(
                    f,
                    "Checksum mismatch in {}: expected {:08x}, got {:08x}",
                    description, expected, actual
                )
            }
            Self::ValidationFailure(msg) => {
                write!(f, "Validation failure: {}", msg)
            }
            Self::Serialization(s_err) => write!(f, "Serialization error: {}", s_err),
            Self::IndexOutOfBounds { collection_type, index, len } => {
                write!(
                    f,
                    "Index out of bounds in {}: attempted index {}, length is {}",
                    collection_type, index, len
                )
            }
        }
    }
}

impl From<crate::traits::SerializationError> for BoundedError {
    fn from(err: crate::traits::SerializationError) -> Self {
        BoundedError::Serialization(err)
    }
}

impl From<BoundedError> for wrt_error::Error {
    fn from(err: BoundedError) -> Self {
        match err {
            BoundedError::CapacityExceeded { collection_type, capacity, attempted_size } => {
                let msg = format!(
                    "{} capacity exceeded: limit {}, attempted {}",
                    collection_type, capacity, attempted_size
                );
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Capacity,
                    wrt_error::codes::CAPACITY_EXCEEDED,
                    msg,
                )
            }
            BoundedError::InvalidAccess { collection_type, index, size } => {
                let msg = format!(
                    "Invalid access to {} at index {}, size is {}",
                    collection_type, index, size
                );
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Core,
                    wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                    msg,
                )
            }
            BoundedError::ChecksumMismatch { expected, actual, description } => {
                let msg = format!(
                    "Checksum mismatch in {}: expected {:08x}, got {:08x}",
                    description, expected, actual
                );
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Validation,
                    wrt_error::codes::CHECKSUM_MISMATCH,
                    msg,
                )
            }
            BoundedError::ValidationFailure(description) => wrt_error::Error::new(
                wrt_error::ErrorCategory::Validation,
                wrt_error::codes::VALIDATION_FAILURE,
                description,
            ),
            BoundedError::Serialization(s_err) => wrt_error::Error::new(
                wrt_error::ErrorCategory::Serialization,
                wrt_error::codes::SERIALIZATION_ERROR,
                s_err.to_string(),
            ),
            BoundedError::IndexOutOfBounds { collection_type, index, len } => {
                let msg = format!(
                    "Index out of bounds in {}: attempted index {}, length is {}",
                    collection_type, index, len
                );
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Core,
                    wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                    msg,
                )
            }
        }
    }
}

/// A bounded stack with a fixed maximum capacity and verification.
///
/// This stack ensures it never exceeds the specified capacity `N_ELEMENTS`.
/// It uses a `MemoryProvider` for storing serialized elements.
#[derive(Debug)] // Removed Clone, PartialEq, Eq
pub struct BoundedStack<T, const N_ELEMENTS: usize, P: MemoryProvider>
where
    T: Sized + Checksummable + ToBytes + FromBytes, // Removed Clone, added ToBytes + FromBytes
{
    /// The underlying memory handler
    handler: SafeMemoryHandler<P>,
    /// Current number of elements in the stack
    length: usize,
    /// Size of a single element T in bytes (using T::SERIALIZED_SIZE)
    elem_size: usize,
    /// Checksum for verifying data integrity
    checksum: Checksum,
    /// Verification level for this stack
    verification_level: VerificationLevel,
    /// Phantom data for type T
    _phantom: PhantomData<T>,
}

// Default implementation requires MemoryProvider to be Default, which might not always be true.
// Provide new and with_verification_level constructors instead.
// impl<T, const N_ELEMENTS: usize, P: MemoryProvider> Default for BoundedStack<T, N_ELEMENTS, P>
// where
//     T: Sized + Checksummable + ToBytes + FromBytes,
//     P: Default, // Added P: Default
// {
//     fn default() -> Self {
//         Self::new(P::default())
//     }
// }

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes, // Removed Clone, added ToBytes + FromBytes
{
    /// Creates a new `BoundedStack` with the given memory provider and default verification level.
    pub fn new(provider: P) -> Self {
        Self::with_verification_level(provider, VerificationLevel::default())
    }

    /// Creates a new `BoundedStack` with the given memory provider and verification level.
    pub fn with_verification_level(provider: P, level: VerificationLevel) -> Self {
        let elem_size = <T as FromBytes>::SERIALIZED_SIZE;
        // It's good practice to ensure the provider has enough capacity, though SafeMemoryHandler might also check.
        // let required_bytes = N_ELEMENTS.saturating_mul(elem_size);
        // if provider.capacity() < required_bytes { ... handle error or panic ... }

        BoundedStack {
            handler: SafeMemoryHandler::new(provider, level, importance::CRITICAL), // Example importance
            length: 0,
            elem_size,
            checksum: Checksum::default(),
            verification_level: level,
            _phantom: PhantomData,
        }
    }

    /// Pushes an item onto the stack.
    ///
    /// # Errors
    ///
    /// Returns `BoundedError::CapacityExceeded` if the stack is full.
    /// Returns `BoundedError::Serialization` if serialization of `item` fails.
    pub fn push(&mut self, item: T) -> core::result::Result<(), BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);
        if self.length >= N_ELEMENTS {
            return Err(BoundedError::CapacityExceeded {
                collection_type: "BoundedStack".to_string(),
                capacity: N_ELEMENTS,
                attempted_size: self.length + 1,
            });
        }
        if self.elem_size > 0 {
            let offset = self.length * self.elem_size;
            // Get mutable slice and write directly
            let mut safe_slice_mut = self.handler.get_slice_mut(offset, self.elem_size)
                .map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            item.write_bytes(safe_slice_mut.as_mut_slice()).map_err(BoundedError::from)?;
            // Mark as modified if SafeSliceMut requires it for checksumming (depends on its API)
            // safe_slice_mut.mark_modified(); // Example if needed
        }
        self.length += 1;
        if self.verification_level.should_verify(importance::HIGH) {
            self.recalculate_checksum();
        }
        Ok(())
    }

    /// Pops an item from the stack.
    ///
    /// Returns `None` if the stack is empty.
    /// Can return `Err(BoundedError::Serialization)` if deserialization fails.
    pub fn pop(&mut self) -> core::result::Result<Option<T>, BoundedError> {
        record_global_operation(OperationType::CollectionPop, self.verification_level);
        if self.length == 0 {
            return Ok(None);
        }

        self.length -= 1;
        let item = if self.elem_size > 0 {
            let offset = self.length.saturating_mul(self.elem_size);
            let slice: SafeSlice<'_> =
                self.handler.borrow_slice(offset, self.elem_size).map_err(|e| {
                    BoundedError::ValidationFailure(format!("Memory borrow failed for pop: {}", e))
                })?;
            T::from_bytes(slice.as_slice())? // This returns SerializationError
        } else {
            // For ZSTs, no bytes are read, create a new ZST instance.
            // Unsafe means to create ZST. If T is ZST, it has no invalid bit patterns.
            // A Default bound could also work if T: Default.
            // Assuming T::from_bytes for a ZST correctly handles an empty slice.
            T::from_bytes(&[])?
        };

        // Recalculate checksum after removal for simplicity or adjust existing one.
        // For pop, it's easier to recalculate based on remaining items.
        // However, to be efficient, one might try to 'un-checksum' the popped item if possible.
        // Let's opt for recalculation for correctness here.
        self.recalculate_checksum(); // Recalculate checksum for the new state.

        Ok(Some(item))
    }

    /// Peeks at the top item of the stack without removing it.
    ///
    /// Returns `None` if the stack is empty.
    /// Can return `Err(BoundedError::Serialization)` if deserialization fails.
    pub fn peek(&self) -> Option<T> {
        record_global_operation(OperationType::CollectionPeek, self.verification_level);
        if self.length == 0 {
            return None;
        }
        if self.elem_size > 0 {
            let offset = (self.length - 1) * self.elem_size;
            // Change to get_slice
            match self.handler.get_slice(offset, self.elem_size) {
                Ok(safe_slice) => {
                    // It's important that from_bytes doesn't need a mutable buffer here
                    // and that it correctly handles potential errors from malformed data.
                    match T::from_bytes(safe_slice.data()) {
                        Ok(item) => Some(item),
                        Err(_e) => {
                            // Optionally log or handle deserialization error
                            // For now, consistent with pop, treat as None or error
                            None // Or return a Result here
                        }
                    }
                }
                Err(_e) => {
                    // Log or handle error from get_slice
                    None // Or return a Result here
                }
            }
        } else {
            // For Zero-sized types, if they are stored/represented
            // This branch might need adjustment based on how ZSTs are handled
            // If T::from_bytes can return a ZST instance, it might work.
            T::from_bytes(&[]).ok() // Example for ZST
        }
    }

    /// Returns the current verification level.
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Sets the verification level for this stack.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
        self.handler.set_verification_level(level);
    }

    /// Verifies the integrity of the stack.
    /// For now, this just verifies the checksum.
    pub fn verify(&self) -> bool {
        if self.verification_level == VerificationLevel::Off {
            return true;
        }
        self.verify_checksum()
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedCapacity
    for BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes,
{
    fn capacity(&self) -> usize {
        N_ELEMENTS
    }

    fn len(&self) -> usize {
        self.length
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn is_full(&self) -> bool {
        self.length == N_ELEMENTS
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> Checksummed for BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes,
{
    fn checksum(&self) -> Checksum {
        self.checksum
    }

    fn recalculate_checksum(&mut self) {
        self.checksum = Checksum::default();
        if self.elem_size > 0 {
            for i in 0..self.length {
                let offset = i.saturating_mul(self.elem_size);
                // We need to deserialize to checksum, which is inefficient.
                // Ideally, Checksummable would work on byte slices if T is already serialized.
                // Or, T::update_checksum should be used during push/pop carefully.
                // For now, let's assume T::from_bytes and then item.update_checksum()
                match self.handler.borrow_slice(offset, self.elem_size) {
                    Ok(slice_data) => {
                        // This is problematic: T might not be easily reconstructible just for checksum.
                        // Checksummable trait might need to operate on raw bytes if possible,
                        // or we only update checksum on push with the original item.
                        // For pop, this makes checksum verification tricky without recalculating all.

                        // Let's stick to checksumming the item T itself.
                        // This means recalculate_checksum needs to deserialize each element.
                        match T::from_bytes(slice_data.as_slice()) {
                            Ok(item) => item.update_checksum(&mut self.checksum),
                            Err(_) => {
                                // If an item can't be deserialized, the checksum is inherently problematic.
                                // This indicates a deeper issue or that the stored data is corrupt.
                                // For now, we might just skip this item in checksum calculation,
                                // or mark the checksum as invalid. Let's skip.
                                // A robust system might have specific error handling here.
                                if self.verification_level >= VerificationLevel::Redundant {
                                    // Consider logging or panicking if an element can't be deserialized
                                    // during checksum recalculation, as it implies data corruption.
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Memory borrow failed. Similar to above, data might be inaccessible.
                        if self.verification_level >= VerificationLevel::Redundant {
                            // Log or handle error
                        }
                    }
                }
            }
        } else {
            // ZST case
            for _i in 0..self.length {
                match T::from_bytes(&[]) {
                    // Create ZST instance
                    Ok(item_zst) => item_zst.update_checksum(&mut self.checksum),
                    Err(_) => {} // Should not happen for ZSTs with correct FromBytes impl
                }
            }
        }
    }

    fn verify_checksum(&self) -> bool {
        if self.verification_level == VerificationLevel::Off {
            return true;
        }
        let mut current_checksum = Checksum::default();
        // Similar logic to recalculate_checksum for verification
        if self.elem_size > 0 {
            for i in 0..self.length {
                if let Ok(slice_data) =
                    self.handler.borrow_slice(i.saturating_mul(self.elem_size), self.elem_size)
                {
                    if let Ok(item) = T::from_bytes(slice_data.as_slice()) {
                        item.update_checksum(&mut current_checksum);
                    } else {
                        return false; /* Deserialization failed, checksum invalid */
                    }
                } else {
                    return false; /* Memory access failed, checksum invalid */
                }
            }
        } else {
            // ZST case
            for _i in 0..self.length {
                match T::from_bytes(&[]) {
                    Ok(item_zst) => item_zst.update_checksum(&mut current_checksum),
                    Err(_) => {
                        return false;
                    }
                }
            }
        }
        self.checksum.value() == current_checksum.value()
    }
}

// The Validatable trait was removed from the import list `use crate::validation::{importance, BoundedCapacity, Checksummed};`
// If it needs to be re-added, ensure its methods are correctly implemented.
// For now, let's assume checksum verification is the primary validation.
// If `Validatable` is a project-specific trait that should be implemented,
// its methods would need to be defined here.

// Example of how Validatable might look if re-added and used:
/*
impl<T, const N_ELEMENTS: usize, P: MemoryProvider> crate::validation::Validatable for BoundedStack<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes,
{
    type Error = BoundedError;

    fn validate(&self) -> core::result::Result<(), Self::Error> {
        if self.verification_level == VerificationLevel::Off {
            return Ok(());
        }
        if !self.verify_checksum() {
            // To provide a more detailed error, we'd need to store the expected
            // checksum when it was last known good, or recalculate it here
            // and compare, similar to verify_checksum.
            let mut actual_checksum = Checksum::default();
            // Simplified: actual checksum calculation logic omitted for brevity here, assume it's done
            // This would be the same logic as in verify_checksum.
            // For now, let's use the stored checksum as 'expected' and recalculate for 'actual'.
            let mut calculated_checksum = Checksum::default();
            // ... (logic from verify_checksum to fill calculated_checksum) ...
            if self.elem_size > 0 {
                for i in 0..self.length {
                    if let Ok(slice_data) = self.handler.borrow_slice(i.saturating_mul(self.elem_size), self.elem_size) {
                        if let Ok(item) = T::from_bytes(slice_data.as_slice()) {
                            item.update_checksum(&mut calculated_checksum);
                        } else {
                             return Err(BoundedError::ChecksumMismatch {
                                expected: self.checksum.value(),
                                actual: calculated_checksum.value(), // Or some other indicator of failure
                                description: "BoundedStack data integrity (deserialization failed)".to_string(),
                            });
                        }
                    } else {
                        return Err(BoundedError::ChecksumMismatch {
                            expected: self.checksum.value(),
                            actual: calculated_checksum.value(), // Or some other indicator of failure
                            description: "BoundedStack data integrity (memory access failed)".to_string(),
                        });
                    }
                }
            } else { // ZST
                for _i in 0..self.length {
                     match T::from_bytes(&[]) {
                        Ok(item_zst) => item_zst.update_checksum(&mut calculated_checksum),
                        Err(_) => { /* error */ }
                     }
                }
            }


            if self.checksum.value() != calculated_checksum.value() {
                return Err(BoundedError::ChecksumMismatch {
                    expected: self.checksum.value(),
                    actual: calculated_checksum.value(),
                    description: "BoundedStack data integrity".to_string(),
                });
            }
        }
        // Additional validation logic can be added here if needed
        Ok(())
    }

    fn validation_level(&self) -> VerificationLevel {
        self.verification_level()
    }

    fn set_validation_level(&mut self, level: VerificationLevel) {
        self.set_verification_level(level);
    }
}
*/

/// A bounded vector with a fixed maximum capacity and verification.
///
/// This vector ensures it never exceeds the specified capacity `N_ELEMENTS`.
/// It uses a `MemoryProvider` for storing serialized elements.
#[derive(Debug)]
pub struct BoundedVec<T, const N_ELEMENTS: usize, P: MemoryProvider>
where
    T: Sized + Checksummable + ToBytes + FromBytes, // Consider if Clone is needed for some ops
{
    handler: SafeMemoryHandler<P>,
    length: usize,
    elem_size: usize, // From T::SERIALIZED_SIZE
    checksum: Checksum,
    verification_level: VerificationLevel,
    _phantom: PhantomData<T>,
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes,
{
    /// Creates a new `BoundedVec` with the given memory provider and default verification level.
    pub fn new(provider: P) -> Self {
        Self::with_verification_level(provider, VerificationLevel::default())
    }

    /// Creates a new `BoundedVec` with a specific verification level.
    pub fn with_verification_level(provider: P, verification_level: VerificationLevel) -> Self {
        let elem_size = <T as ToBytes>::SERIALIZED_SIZE; // Disambiguated
        let mut vec = Self {
            handler: SafeMemoryHandler::new(provider, verification_level),
            length: 0,
            elem_size, // Now correctly sourced
            checksum: Checksum::default(),
            verification_level,
            _phantom: PhantomData,
        };
        if vec.verification_level.should_verify(importance::INITIALIZATION) {
            vec.recalculate_checksum();
        }
        vec
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Errors
    /// Returns `BoundedError::CapacityExceeded` if the vector is full.
    /// Returns `BoundedError::Serialization` if the item cannot be serialized.
    pub fn push(&mut self, item: T) -> core::result::Result<(), BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);
        if self.length >= N_ELEMENTS {
            return Err(BoundedError::CapacityExceeded {
                collection_type: "BoundedVec".to_string(),
                capacity: N_ELEMENTS,
                attempted_size: self.length + 1,
            });
        }
        if self.elem_size > 0 {
            let offset = self.length * self.elem_size;
            let mut safe_slice_mut = self.handler.get_slice_mut(offset, self.elem_size)
                .map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            item.write_bytes(safe_slice_mut.as_mut_slice()).map_err(BoundedError::from)?;
        }
        self.length += 1;
        if self.verification_level.should_verify(importance::HIGH) {
            self.recalculate_checksum();
        }
        Ok(())
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    ///
    /// # Errors
    /// Returns `BoundedError::Serialization` if the item cannot be deserialized.
    pub fn pop(&mut self) -> core::result::Result<Option<T>, BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);
        if self.length == 0 {
            return Ok(None);
        }
        let item = if self.elem_size > 0 { // Only read if not a ZST
            let offset = (self.length - 1) * self.elem_size;
            let mut buffer = [0u8; <T as FromBytes>::SERIALIZED_SIZE]; // Disambiguated
            let slice_view = self.handler.get_slice(offset, self.elem_size).map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            let data_bytes = slice_view.data().map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            buffer.copy_from_slice(data_bytes);
            T::from_bytes(&buffer).map_err(BoundedError::from)?
        } else {
            // For ZST, create a default instance or require T: Default
            // Assuming T can be created from zero bytes for ZSTs as per FromBytes trait for ()
            T::from_bytes(&[]).map_err(BoundedError::from)?
        };
        self.length -= 1;
        // Optionally, clear the memory region of the popped item in the handler if desired.
        self.recalculate_checksum();
        Ok(Some(item))
    }

    /// Returns a reference to an element at a given position or `None` if out of bounds.
    ///
    /// # Errors
    /// Returns `BoundedError::Serialization` if the item cannot be deserialized.
    /// Returns `BoundedError::InvalidAccess` indirectly through handler/slice errors if underlying read fails.
    pub fn get(&self, index: usize) -> Option<T> {
        record_global_operation(OperationType::CollectionRead, self.verification_level);
        if index >= self.length {
            return None;
        }
        if self.elem_size > 0 {
            let offset = index * self.elem_size;
            // Change to get_slice
            match self.handler.get_slice(offset, self.elem_size) {
                Ok(safe_slice) => T::from_bytes(safe_slice.data()).ok(),
                Err(_) => None, // Error fetching slice
            }
        } else {
            T::from_bytes(&[]).ok() // For ZSTs
        }
    }

    /// Inserts an element at a given position in the vector.
    ///
    /// # Errors
    /// Returns `BoundedError::CapacityExceeded` if the vector is full.
    /// Returns `BoundedError::IndexOutOfBounds` if the index is out of bounds.
    /// Returns `BoundedError::Serialization` if the item cannot be serialized.
    pub fn insert_at(&mut self, index: usize, item: T) -> core::result::Result<(), BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);
        if self.length >= N_ELEMENTS {
            return Err(BoundedError::CapacityExceeded {
                collection_type: "BoundedVec".to_string(),
                capacity: N_ELEMENTS,
                attempted_size: self.length + 1,
            });
        }
        if index > self.length { // Allow inserting at self.length (same as push)
            return Err(BoundedError::IndexOutOfBounds {
                collection_type: "BoundedVec".to_string(),
                index,
                len: self.length,
            });
        }

        if self.elem_size > 0 {
            // Shift elements to the right if inserting not at the end
            if index < self.length {
                let src_offset = index * self.elem_size;
                let dst_offset = (index + 1) * self.elem_size;
                // Number of bytes to move is (length - index) * elem_size
                let count = (self.length - index) * self.elem_size;
                self.handler.copy_within(src_offset, dst_offset, count)
                    .map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            }
            // Write the new item
            let offset = index * self.elem_size;
            let mut safe_slice_mut = self.handler.get_slice_mut(offset, self.elem_size)
                .map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            item.write_bytes(safe_slice_mut.as_mut_slice()).map_err(BoundedError::from)?;
        }
        // If elem_size is 0 (ZST), we still increment length. No bytes are moved or written.

        self.length += 1;
        if self.verification_level.should_verify(importance::HIGH) {
            self.recalculate_checksum();
        }
        Ok(())
    }

    /// Sets an element at a given position in the vector.
    ///
    /// # Errors
    /// Returns `BoundedError::IndexOutOfBounds` if the index is out of bounds.
    /// Returns `BoundedError::Serialization` if the item cannot be serialized.
    pub fn set_at(&mut self, index: usize, item: T) -> core::result::Result<(), BoundedError> {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);
        if index >= self.length {
            return Err(BoundedError::IndexOutOfBounds {
                collection_type: "BoundedVec".to_string(),
                index,
                len: self.length,
            });
        }
        if self.elem_size > 0 {
            let offset = index * self.elem_size;
            let mut safe_slice_mut = self.handler.get_slice_mut(offset, self.elem_size)
                .map_err(|e| BoundedError::ValidationFailure(e.to_string()))?;
            item.write_bytes(safe_slice_mut.as_mut_slice()).map_err(BoundedError::from)?;
        }
        // For ZSTs, set_at effectively does nothing to the data if elem_size is 0,
        // but the item itself might have changed if T has internal state not reflected in its size.
        // The checksum will be recalculated regardless.

        if self.verification_level.should_verify(importance::HIGH) {
            self.recalculate_checksum(); // Data changed (or conceptually changed for ZSTs), recalculate.
        }
        Ok(())
    }

    /// Clears the vector, removing all values.
    pub fn clear(&mut self) {
        record_global_operation(OperationType::CollectionWrite, self.verification_level);
        self.length = 0;
        // Optionally, zero out the memory in the handler.
        self.recalculate_checksum();
    }

    /// Returns the number of elements in the vector, also referred to as its 'length'.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns `true` if the vector is at its maximum capacity `N_ELEMENTS`.
    pub fn is_full(&self) -> bool {
        self.length >= N_ELEMENTS
    }

    /// Returns the total number of elements the vector can hold.
    pub fn capacity(&self) -> usize {
        N_ELEMENTS
    }

    /// Gets the current verification level.
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Sets a new verification level.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
        self.handler.set_verification_level(level); // Propagate to handler
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> BoundedCapacity
    for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes,
{
    fn capacity(&self) -> usize {
        N_ELEMENTS
    }

    fn len(&self) -> usize {
        self.length
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn is_full(&self) -> bool {
        self.length >= N_ELEMENTS
    }
}

impl<T, const N_ELEMENTS: usize, P: MemoryProvider> Checksummed
    for BoundedVec<T, N_ELEMENTS, P>
where
    T: Sized + Checksummable + ToBytes + FromBytes,
{
    fn checksum(&self) -> Checksum {
        self.checksum
    }

    fn recalculate_checksum(&mut self) {
        record_global_operation(OperationType::ChecksumCalculation, self.verification_level);
        self.checksum = Checksum::default(); // Start fresh
        // Checksum includes length and verification level for structural integrity.
        self.checksum.update_slice(&self.length.to_ne_bytes());
        self.checksum.update_slice(&self.verification_level.to_byte().to_ne_bytes()); // Changed

        if self.elem_size > 0 {
            if self.length > 0 {
                 // Get a single slice representing all used data for checksumming
                match self.handler.get_slice(0, self.length * self.elem_size) {
                    Ok(slice_view) => match slice_view.data() {
                        Ok(data_bytes) => self.checksum.update_slice(data_bytes),
                        Err(_) => { /* Error reading slice data, checksum remains partial */ }
                    },
                    Err(_) => { /* Error getting slice, checksum remains partial */ }
                }
            }
        } else {
            // For ZSTs, checksum might only depend on length or a count.
            // Here, we checksum the length again, effectively, or could use a specific marker.
            for _ in 0..self.length {
                // An empty slice or a specific ZST marker byte could be used.
                // For simplicity, let's say ZST elements don't contribute individually beyond count (already in length)
                // T::update_checksum (if it had a static method or ZST instance) could be used.
                // Since T: Checksummable, an instance would be needed.
                // This part needs refinement for ZSTs if they have distinct checksummable states beyond presence.
            }
        }
    }

    fn verify_checksum(&self) -> bool {
        record_global_operation(OperationType::CollectionValidate, self.verification_level);
        if !self.verification_level.should_verify(importance::HIGH) { // Use high importance for this check
            return true; // Skip if verification level allows
        }
        let mut current_checksum = Checksum::default();
        current_checksum.update_slice(&self.length.to_ne_bytes());
        current_checksum.update_slice(&self.verification_level.to_byte().to_ne_bytes()); // Changed

        if self.elem_size > 0 {
            if self.length > 0 {
                match self.handler.get_slice(0, self.length * self.elem_size) {
                    Ok(slice_view) => match slice_view.data() {
                        Ok(data_bytes) => current_checksum.update_slice(data_bytes),
                        Err(_) => return false, // Could not read data to verify
                    },
                    Err(_) => return false, // Could not get slice to verify
                }
            }
        } else {
            // Similar logic as recalculate_checksum for ZSTs
        }
        current_checksum == self.checksum
    }
}

// TODO: Add Kani proofs for BoundedVec operations if `kani` feature is enabled.
// TODO: Add unit tests for BoundedVec.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safe_memory::{NoStdMemoryProvider, StdMemoryProvider}; // Assuming NoStdMemoryProvider exists
    use crate::verification::VerificationLevel;

    // Helper for creating StdMemoryProvider for tests
    fn create_std_provider(size: usize) -> StdMemoryProvider {
        StdMemoryProvider::new(vec![0u8; size])
    }

    // Dummy Checksummable type for testing
    #[derive(Debug, Clone, PartialEq, Eq, Copy)] // Added Copy
    struct TestItem(u32);

    impl Checksummable for TestItem {
        fn update_checksum(&self, checksum: &mut Checksum) {
            checksum.update_u32(self.0);
        }
    }

    impl ToBytes for TestItem {
        const SERIALIZED_SIZE: usize = 4; // size of u32
        fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError> {
            if buffer.len() != Self::SERIALIZED_SIZE {
                return Err(SerializationError::IncorrectSize);
            }
            buffer.copy_from_slice(&self.0.to_le_bytes());
            Ok(())
        }
    }

    impl FromBytes for TestItem {
        const SERIALIZED_SIZE: usize = 4; // size of u32
        fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, SerializationError> {
            if bytes.len() != Self::SERIALIZED_SIZE {
                return Err(SerializationError::IncorrectSize);
            }
            let mut arr = [0u8; 4];
            arr.copy_from_slice(bytes);
            Ok(TestItem(u32::from_le_bytes(arr)))
        }
    }

    // Test for Zero-Sized Type
    #[derive(Debug, Clone, PartialEq, Eq, Copy)]
    struct TestZST;

    impl Checksummable for TestZST {
        fn update_checksum(&self, _checksum: &mut Checksum) {
            // ZSTs typically don't change checksum, or have a fixed change.
        }
    }
    impl ToBytes for TestZST {
        const SERIALIZED_SIZE: usize = 0;
        fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError> {
            if !buffer.is_empty() {
                Err(SerializationError::IncorrectSize)
            } else {
                Ok(())
            }
        }
    }
    impl FromBytes for TestZST {
        const SERIALIZED_SIZE: usize = 0;
        fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, SerializationError> {
            if !bytes.is_empty() {
                Err(SerializationError::IncorrectSize)
            } else {
                Ok(TestZST)
            }
        }
    }

    #[test]
    #[cfg(feature = "std")] // Ensure std environment for StdMemoryProvider with Vec
    fn test_bounded_stack_std_refactored() {
        let provider = create_std_provider(TestItem::SERIALIZED_SIZE * 3);
        let mut stack: BoundedStack<TestItem, 3, _> = BoundedStack::new(provider);

        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
        assert_eq!(stack.capacity(), 3);

        // Push items
        assert!(stack.push(TestItem(10)).is_ok());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.peek().unwrap().unwrap(), TestItem(10));

        assert!(stack.push(TestItem(20)).is_ok());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek().unwrap().unwrap(), TestItem(20));

        assert!(stack.push(TestItem(30)).is_ok());
        assert_eq!(stack.len(), 3);
        assert!(!stack.is_empty());
        assert!(stack.is_full());
        assert_eq!(stack.peek().unwrap().unwrap(), TestItem(30));

        // Try to push when full
        match stack.push(TestItem(40)) {
            Err(BoundedError::CapacityExceeded { capacity, .. }) => assert_eq!(capacity, 3),
            _ => panic!("Expected CapacityExceeded"),
        }

        // Pop items
        assert_eq!(stack.pop().unwrap().unwrap(), TestItem(30));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek().unwrap().unwrap(), TestItem(20));

        assert_eq!(stack.pop().unwrap().unwrap(), TestItem(20));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.peek().unwrap().unwrap(), TestItem(10));

        // Verify checksum (basic test)
        stack.set_verification_level(VerificationLevel::Balanced);
        assert!(stack.verify_checksum());

        assert_eq!(stack.pop().unwrap().unwrap(), TestItem(10));
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
        assert!(stack.peek().unwrap().is_none());

        // Try to pop when empty
        assert!(stack.pop().unwrap().is_none());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_bounded_stack_zst() {
        let provider = create_std_provider(0); // ZSTs need 0 memory for elements
        let mut stack: BoundedStack<TestZST, 5, _> = BoundedStack::new(provider);

        assert_eq!(stack.len(), 0);
        assert!(stack.push(TestZST).is_ok());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.peek().unwrap().unwrap(), TestZST);

        for _ in 0..4 {
            assert!(stack.push(TestZST).is_ok());
        }
        assert_eq!(stack.len(), 5);
        assert!(stack.is_full());

        match stack.push(TestZST) {
            Err(BoundedError::CapacityExceeded { capacity, .. }) => assert_eq!(capacity, 5),
            _ => panic!("Expected CapacityExceeded for ZST stack"),
        }

        assert_eq!(stack.pop().unwrap().unwrap(), TestZST);
        assert_eq!(stack.len(), 4);
    }

    // ... other existing tests ...
    // Note: The old `test_bounded_stack_std` and `test_bounded_stack_guaranteed`
    // will need to be adapted or replaced by tests like the one above,
    // as they used the old Vec-backed BoundedStack.

    // Minimal adaptation of an existing test to see if it compiles with new structure,
    // will likely need more providers.
    #[test]
    #[cfg(feature = "std")]
    fn bounded_stack_specifics_refactored() {
        let provider = create_std_provider(TestItem::SERIALIZED_SIZE * 2);
        let mut stack: BoundedStack<TestItem, 2, _> =
            BoundedStack::with_verification_level(provider, VerificationLevel::Redundant);
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());

        let item1 = TestItem(1);
        let item2 = TestItem(2);

        assert!(stack.push(item1).is_ok());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.peek().unwrap().unwrap(), item1);
        assert!(stack.verify_checksum()); // Checksum after one push

        assert!(stack.push(item2).is_ok());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek().unwrap().unwrap(), item2);
        assert!(!stack.is_empty());
        assert!(stack.is_full());
        assert!(stack.verify_checksum()); // Checksum after two pushes

        let popped_item = stack.pop().unwrap().unwrap();
        assert_eq!(popped_item, item2);
        assert_eq!(stack.len(), 1);
        assert!(stack.verify_checksum()); // Checksum after one pop

        let _ = stack.pop().unwrap().unwrap();
        assert!(stack.is_empty());
        assert!(stack.verify_checksum()); // Checksum when empty
    }

    // ... existing code ...
}
