// WRT - wrt-types
// Module: Common Conversion Traits
// SW-REQ-ID: REQ_VERIFY_003
// SW-REQ-ID: REQ_018
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use core::fmt;
#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
use core::fmt;
#[cfg(feature = "std")]
use std::fmt; // For cases with no_std and no alloc, fmt is still in core

use crate::types::ValueType;
use crate::Error; // Use the error type defined in this crate's lib.rs
use crate::WrtResult; // Import WrtResult from lib.rs

// Common traits for type conversions
//
// This module provides common traits used for type conversions between format
// and runtime representations.

/// Trait for types that can be converted from a format representation
pub trait FromFormat<T> {
    /// Convert from a format representation
    fn from_format(format: &T) -> Self;
}

/// Trait for types that can be converted to a format representation
pub trait ToFormat<T>: Sized {
    /// Converts self to the target format type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    fn to_format(&self) -> crate::WrtResult<T>;
}

/// Trait for types that can update a checksum.
///
/// This trait is used by bounded collections to maintain data integrity
/// without resorting to unsafe byte conversions for generic types.
pub trait Checksummable {
    /// Updates the given checksum with the byte representation of self.
    ///
    /// How a type is converted to bytes for checksumming is specific to its
    /// implementation. For complex types, this should be a defined, stable
    /// serialization.
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum);
}

/// Trait for types that can be converted to/from little-endian byte representation
pub trait LittleEndian: Sized {
    /// Convert from little-endian bytes
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self>;

    /// Convert to little-endian bytes
    fn to_le_bytes(&self) -> WrtResult<Vec<u8>>;
}

/// Trait for types that can be converted to WRT Value representation
pub trait ToWrtValue {
    /// Converts self to the target WRT Value type.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    fn to_wrt_value(&self) -> crate::WrtResult<crate::types::ValueType>;
}

// Implementations for primitive types

macro_rules! impl_checksummable_for_primitive {
    ($($T:ty),*) => {
        $(impl Checksummable for $T {
            fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
                checksum.update_slice(&self.to_ne_bytes());
            }
        })*
    };
}

impl_checksummable_for_primitive! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64 // Note: f32/f64 checksums based on their bit patterns via to_ne_bytes
}

impl Checksummable for bool {
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        checksum.update_slice(&[if *self { 1u8 } else { 0u8 }]);
    }
}

// For slices of checksummable types, one might iterate, or for &[u8] directly:
impl Checksummable for &[u8] {
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        checksum.update_slice(self);
    }
}

#[cfg(feature = "alloc")]
impl Checksummable for alloc::string::String {
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        checksum.update_slice(self.as_bytes());
    }
}

// Example for arrays (might need a helper or be specific if T itself is not
// [u8; N]) This generic impl would require T to be Checksummable itself, which
// is not what item_as_bytes_slice did. The original item_as_bytes_slice
// directly took bytes of T. For arrays of primitives, the macro above handles
// the primitives. If T is a struct, it would need its own Checksummable impl.

// For a generic array of known-size primitives, this might be useful if direct
// byte repr is okay: impl<T, const N: usize> Checksummable for [T; N]
// where
//     T: Copy + Sized, // Add more bounds if needed, e.g. Pod if bytemuck was
// allowed {
//     fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
//         // This is unsafe if T is not POD. We are avoiding unsafe.
//         // So, this generic impl is problematic without further constraints
// or a safe way         // to get bytes. If T itself is Checksummable:
//         // for item in self.iter() {
//         //     item.update_checksum(checksum);
//         // }
//         // However, the original code did a direct memory dump of T for
// checksum.         // The new trait shifts responsibility to T to provide its
// bytes.         // For arrays of primitives, the checksum will be based on
// each element's checksum.

//         // If we assume this is an array of bytes, or T is u8:
//         // if core::mem::size_of::<T>() == 1 {
//         //    let bytes: &[u8] = unsafe {
// core::slice::from_raw_parts(self.as_ptr() as *const u8, N) };         //
// checksum.update_slice(bytes);         // }
//         // This still uses unsafe. Best to rely on T implementing
// Checksummable and iterate if it's not &[u8].     }
// }

// New traits for safe serialization/deserialization to/from bytes

/// Trait for types that can be safely converted into a byte slice.
/// Used for storing generic types in byte-oriented safe memory abstractions.
pub trait ToBytes {
    /// The exact number of bytes required to represent any instance of this
    /// type.
    const SERIALIZED_SIZE: usize;

    /// Serializes the instance into the provided buffer.
    /// The buffer must be exactly `SERIALIZED_SIZE` bytes long.
    ///
    /// # Errors
    /// Returns an error if the buffer is not the correct size or serialization
    /// fails.
    fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError>;
}

/// Trait for types that can be safely reconstructed from a byte slice.
pub trait FromBytes: Sized {
    /// The exact number of bytes required to represent any instance of this
    /// type. Must match `ToBytes::SERIALIZED_SIZE` if both are implemented.
    const SERIALIZED_SIZE: usize;

    /// Reconstructs an instance from the provided byte slice.
    /// The byte slice must be exactly `SERIALIZED_SIZE` bytes long.
    ///
    /// # Errors
    /// Returns an error if deserialization fails (e.g., invalid format,
    /// incorrect size).
    fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, SerializationError>;
}

/// Error type for serialization/deserialization issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationError {
    /// The provided buffer or byte slice has an incorrect size.
    IncorrectSize,
    /// The data format is invalid or corrupted.
    InvalidFormat,
    /// An underlying IO error occurred (relevant if reading from a stream).
    // IoError(String), // Perhaps too complex for now, keep simple
    /// A custom error message.
    Custom(crate::prelude::String), // Using prelude String for alloc/std compatibility
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationError::IncorrectSize => {
                write!(f, "Incorrect buffer or slice size for serialization/deserialization")
            }
            SerializationError::InvalidFormat => {
                write!(f, "Invalid data format for deserialization")
            }
            SerializationError::Custom(s) => write!(f, "Serialization error: {s}"),
        }
    }
}

// Implement ToBytes/FromBytes for primitives

macro_rules! impl_bytes_for_primitive {
    ($($T:ty),*) => {
        $(
            impl ToBytes for $T {
                const SERIALIZED_SIZE: usize = core::mem::size_of::<$T>();
                fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError> {
                    if buffer.len() != <$T as ToBytes>::SERIALIZED_SIZE {
                        return Err(SerializationError::IncorrectSize);
                    }
                    buffer.copy_from_slice(&self.to_ne_bytes());
                    Ok(())
                }
            }
            impl FromBytes for $T {
                const SERIALIZED_SIZE: usize = core::mem::size_of::<$T>();
                fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, SerializationError> {
                    if bytes.len() != <$T as FromBytes>::SERIALIZED_SIZE {
                        return Err(SerializationError::IncorrectSize);
                    }
                    Ok(<$T>::from_ne_bytes(bytes.try_into().map_err(|_| SerializationError::IncorrectSize)?))
                }
            }
        )*
    };
}

impl_bytes_for_primitive! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64 // Floats will be stored by their bit patterns (to_ne_bytes)
}

impl ToBytes for bool {
    const SERIALIZED_SIZE: usize = 1;
    fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError> {
        if buffer.len() != <bool as ToBytes>::SERIALIZED_SIZE {
            return Err(SerializationError::IncorrectSize);
        }
        buffer[0] = if *self { 1 } else { 0 };
        Ok(())
    }
}

impl FromBytes for bool {
    const SERIALIZED_SIZE: usize = 1;
    fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, SerializationError> {
        if bytes.len() != <bool as FromBytes>::SERIALIZED_SIZE {
            return Err(SerializationError::IncorrectSize);
        }
        match bytes[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SerializationError::InvalidFormat),
        }
    }
}

// Implementations for Zero-Sized Types (ZSTs) e.g. ()
impl ToBytes for () {
    const SERIALIZED_SIZE: usize = 0;
    fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError> {
        if buffer.is_empty() {
            Ok(())
        } else {
            Err(SerializationError::IncorrectSize)
        }
    }
}

impl FromBytes for () {
    const SERIALIZED_SIZE: usize = 0;
    fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, SerializationError> {
        if bytes.is_empty() {
            Ok(())
        } else {
            Err(SerializationError::IncorrectSize)
        }
    }
}

// Example for arrays - this still needs T: ToBytes + FromBytes + Default + Copy
// for a generic fixed-size array. For now, users would implement
// ToBytes/FromBytes for their specific [T; N] structs if needed, or BoundedVec
// would handle elements of type T that are ToBytes/FromBytes.

// Comment out the old generic Checksummable for [T;N] as it was problematic.
// // impl<T, const N: usize> Checksummable for [T; N]
// // where
// //     T: Copy + Sized,
// // {
// //     fn update_checksum(&self, checksum: &mut
// crate::verification::Checksum) { //     }
// // }
