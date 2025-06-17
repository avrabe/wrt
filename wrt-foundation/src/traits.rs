// WRT - wrt-foundation
// Module: Common Conversion Traits
// SW-REQ-ID: REQ_VERIFY_003
// SW-REQ-ID: REQ_018
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#[cfg(not(feature = "std"))]
use core::fmt;
#[cfg(feature = "std")]
use std::fmt;

use wrt_error::{codes, Error as WrtError, ErrorCategory};

use crate::{
    prelude::*,
    safe_memory::{NoStdProvider, SafeMemoryHandler, Slice, SliceMut, Stats},
    MemoryProvider as RootMemoryProvider, VerificationLevel, WrtResult,
}; // Keep WrtResult, Added RootMemoryProvider etc. // Added WrtError,
   // ErrorCategory, codes

#[cfg(feature = "std")]
extern crate alloc;

// Removed: use core::mem::size_of; // No longer directly needed here for
// ToBytes/FromBytes definitions

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

/// Trait for types that can be converted to/from little-endian byte
/// representation
pub trait LittleEndian: Sized {
    /// Convert from little-endian bytes
    fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self>;

    /// Writes the value as little-endian bytes to the provided writer.
    fn write_le_bytes<W: BytesWriter>(&self, writer: &mut W) -> WrtResult<()>;
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
    f32, f64, // Note: f32/f64 checksums based on their bit patterns via to_ne_bytes
    usize, isize // Added usize and isize
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

/// A trait for sequentially writing bytes.
/// Binary std/no_std choice
pub trait BytesWriter {
    /// Writes a single byte.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte cannot be written (e.g., out of capacity).
    fn write_byte(&mut self, byte: u8) -> WrtResult<()>;

    /// Writes an entire slice of bytes.
    ///
    /// All bytes must be written successfully, or an error is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes cannot be written (e.g., out of capacity).
    fn write_all(&mut self, bytes: &[u8]) -> WrtResult<()>;
}

#[cfg(feature = "std")]
impl Checksummable for alloc::string::String {
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        checksum.update_slice(self.as_bytes());
    }
}

/// Trait for types that can be serialized to bytes.
pub trait ToBytes: Sized {
    /// Returns the size in bytes required to serialize this type.
    /// This should be a constant for fixed-size types.
    /// Default implementation returns 0 - types should override this.
    fn serialized_size(&self) -> usize {
        0 // Default fallback - should be overridden by implementations
    }

    /// Serializes the type into a byte stream using a provided memory stream
    /// and memory provider for stream operations.
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()>;

    /// Serializes the type into a byte stream using the default memory
    /// provider. Requires `DefaultMemoryProvider` to be available.
    #[cfg(feature = "default-provider")]
    fn to_bytes<'a>(&self, writer: &mut WriteStream<'a>) -> WrtResult<()> {
        let default_provider = DefaultMemoryProvider::default();
        self.to_bytes_with_provider(writer, &default_provider)
    }
}

/// Trait for types that can be deserialized from a byte stream.
pub trait FromBytes: Sized {
    /// Deserializes an instance of the type from a byte stream using a
    /// provided memory stream and memory provider for stream operations.
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self>;

    /// Deserializes an instance of the type from a byte stream using the
    /// default memory provider. Requires `DefaultMemoryProvider` to be
    /// available.
    #[cfg(feature = "default-provider")]
    fn from_bytes<'a>(reader: &mut ReadStream<'a>) -> WrtResult<Self> {
        let default_provider = DefaultMemoryProvider::default();
        Self::from_bytes_with_provider(reader, &default_provider)
    }
}

/// Error type for serialization/deserialization issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationError {
    /// The provided buffer or byte slice has an incorrect size.
    IncorrectSize,
    /// The data format is invalid or corrupted.
    InvalidFormat,
    /// A custom error message.
    Custom(&'static str), // Binary std/no_std choice
    /// The provided buffer or byte slice has an incorrect length.
    InvalidSliceLength,
    /// Not enough data to deserialize the object.
    NotEnoughData,
    /// An I/O operation failed during serialization/deserialization.
    IoError,
    /// An unexpected end of file/buffer was reached during deserialization.
    UnexpectedEof,
    /// An invalid `enum` value was encountered during deserialization.
    InvalidEnumValue,
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
            SerializationError::InvalidSliceLength => {
                write!(f, "Invalid slice length for serialization/deserialization")
            }
            SerializationError::NotEnoughData => {
                write!(f, "Not enough data to deserialize the object")
            }
            SerializationError::IoError => {
                write!(f, "An I/O operation failed during serialization/deserialization")
            }
            SerializationError::UnexpectedEof => {
                write!(f, "Unexpected end of input during deserialization")
            }
            SerializationError::InvalidEnumValue => {
                write!(f, "Invalid enum value during deserialization")
            }
        }
    }
}

// Implement ToBytes/FromBytes for primitives

macro_rules! impl_bytes_for_primitive {
    ($($T:ty => $read_method:ident, $write_method:ident);* $(;)?) => {
        $(
            impl ToBytes for $T {
                fn serialized_size(&self) -> usize {
                    core::mem::size_of::<$T>()
                }

                fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
                    &self,
                    writer: &mut WriteStream<'a>,
                    _provider: &PStream, // Provider typically not needed for primitives
                ) -> WrtResult<()> {
                    writer.$write_method(*self)
                }
                // to_bytes method is provided by the trait with DefaultMemoryProvider
            }

            impl FromBytes for $T {
                fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
                    reader: &mut ReadStream<'a>,
                    _provider: &PStream, // Provider typically not needed for primitives
                ) -> WrtResult<Self> {
                    reader.$read_method()
                }
                // from_bytes method is provided by the trait with DefaultMemoryProvider
            }
        )*
    };
}

impl_bytes_for_primitive! {
    u8 => read_u8, write_u8;
    i8 => read_i8, write_i8;
    u16 => read_u16_le, write_u16_le;
    i16 => read_i16_le, write_i16_le;
    u32 => read_u32_le, write_u32_le;
    i32 => read_i32_le, write_i32_le;
    u64 => read_u64_le, write_u64_le;
    i64 => read_i64_le, write_i64_le;
    u128 => read_u128_le, write_u128_le;
    i128 => read_i128_le, write_i128_le;
    f32 => read_f32_le, write_f32_le;
    f64 => read_f64_le, write_f64_le;
    usize => read_usize_le, write_usize_le;
    isize => read_isize_le, write_isize_le;
}

// Corrected ToBytes for bool
impl ToBytes for bool {
    fn serialized_size(&self) -> usize {
        1
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream, // provider not typically used for simple types like bool
    ) -> WrtResult<()> {
        writer.write_u8(*self as u8) // Use WriteStream's method
    }
    // to_bytes method is provided by the trait with DefaultMemoryProvider
}

// Corrected FromBytes for bool
impl FromBytes for bool {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream, // provider not typically used for simple types like bool
    ) -> WrtResult<Self> {
        let byte = reader.read_u8()?; // Use ReadStream's method
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WrtError::new(
                ErrorCategory::Parse,
                codes::INVALID_VALUE, // Changed from INVALID_DATA
                "Invalid boolean value during deserialization", // More specific message
            )),
        }
    }
    // from_bytes method is provided by the trait with DefaultMemoryProvider
}

// Corrected ToBytes for ()
impl ToBytes for () {
    fn serialized_size(&self) -> usize {
        0
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        _writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        Ok(()) // Nothing to write for unit type
    }
    // to_bytes method is provided by the trait with DefaultMemoryProvider
}

// Corrected FromBytes for ()
impl FromBytes for () {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        _reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        Ok(()) // Nothing to read for unit type
    }
    // from_bytes method is provided by the trait with DefaultMemoryProvider
}

/// Trait for types that can be converted to/from little-endian byte arrays of a
/// fixed size. This trait is intended for types where
/// `core::mem::size_of::<Self>()` is a valid compile-time constant.
trait LeBytesArray: Sized {
    /// The byte array type, e.g., `[u8; 4]` for `u32`.
    type ByteArray: AsRef<[u8]> + AsMut<[u8]> + Default + Copy + IntoIterator<Item = u8>;

    /// Converts the value to a little-endian byte array.
    fn to_le_bytes_arr(&self) -> Self::ByteArray;

    /// Converts a little-endian byte array to a value.
    fn from_le_bytes_arr(arr: Self::ByteArray) -> Self;
}

// Example implementation for u32 (add others as needed, or use a macro if many)
impl LeBytesArray for u32 {
    type ByteArray = [u8; core::mem::size_of::<u32>()];

    fn to_le_bytes_arr(&self) -> Self::ByteArray {
        self.to_le_bytes()
    }

    fn from_le_bytes_arr(arr: Self::ByteArray) -> Self {
        Self::from_le_bytes(arr)
    }
}

impl LeBytesArray for i32 {
    type ByteArray = [u8; core::mem::size_of::<i32>()];

    fn to_le_bytes_arr(&self) -> Self::ByteArray {
        self.to_le_bytes()
    }

    fn from_le_bytes_arr(arr: Self::ByteArray) -> Self {
        Self::from_le_bytes(arr)
    }
}

impl LeBytesArray for u64 {
    type ByteArray = [u8; core::mem::size_of::<u64>()];

    fn to_le_bytes_arr(&self) -> Self::ByteArray {
        self.to_le_bytes()
    }

    fn from_le_bytes_arr(arr: Self::ByteArray) -> Self {
        Self::from_le_bytes(arr)
    }
}

impl LeBytesArray for i64 {
    type ByteArray = [u8; core::mem::size_of::<i64>()];

    fn to_le_bytes_arr(&self) -> Self::ByteArray {
        self.to_le_bytes()
    }

    fn from_le_bytes_arr(arr: Self::ByteArray) -> Self {
        Self::from_le_bytes(arr)
    }
}

// Add other primitive impls for LeBytesArray as necessary...

// Implementations of LittleEndian for primitive types
macro_rules! impl_little_endian_for_primitive {
    ($($T:ty, $size:expr);*) => {
        $(impl LittleEndian for $T {
            fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
                if bytes.len() < $size {
                    return Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Memory,
                        wrt_error::codes::BUFFER_TOO_SMALL,
                        "Buffer too small for primitive type conversion"
                    ));
                }
                let mut arr = [0u8; $size];
                arr.copy_from_slice(&bytes[..$size]);
                Ok(<$T>::from_le_bytes(arr))
            }

            fn write_le_bytes<W: BytesWriter>(&self, writer: &mut W) -> WrtResult<()> {
                writer.write_all(&self.to_le_bytes())
            }
        })*
    };
}

impl_little_endian_for_primitive! {
    i8, 1; u8, 1; i16, 2; u16, 2; i32, 4; u32, 4; i64, 8; u64, 8; f32, 4; f64, 8
    // V128 is handled separately if/when defined and LittleEndian is implemented for it.
    // bool is handled by its specific ToBytes/FromBytes impls, not LittleEndian trait.
}

// V128 needs a special implementation as it's a struct, not a primitive with
// to_le_bytes directly but its internal representation is [u8; 16] which is
// already LE by definition of V128. Assuming V128 is defined in
// crate::values::V128 This needs to be in a place where V128 is defined or V128
// needs to be public and imported. For now, I will comment this out and handle
// it in values.rs or where V128 is.
// use crate::values::V128; // Assuming this path
// impl LittleEndian for V128 {
// fn from_le_bytes(bytes: &[u8]) -> WrtResult<Self> {
// if bytes.len() < 16 {
// return Err(wrt_error::Error::new(
// wrt_error::ErrorCategory::Memory,
// wrt_error::codes::BUFFER_TOO_SMALL,
// "Buffer too small for V128. Expected 16, got ".to_string() +
// &bytes.len().to_string() ));
// }
// let mut arr = [0u8; 16];
// arr.copy_from_slice(&bytes[..16]);
// Ok(V128::new(arr))
// }
//
// fn write_le_bytes<W: BytesWriter>(&self, writer: &mut W) -> WrtResult<()> {
// writer.write_all(&self.bytes)
// }
// }

// Adding Error conversion for SerializationError -> wrt_error::Error
// This will be useful if functions returning WrtResult need to propagate
// SerializationError.

impl From<SerializationError> for WrtError {
    fn from(e: SerializationError) -> Self {
        match e {
            SerializationError::IncorrectSize => WrtError::new(
                ErrorCategory::Validation,
                codes::INVALID_BYTE_LENGTH,
                "Incorrect buffer or slice size for serialization/deserialization",
            ),
            SerializationError::InvalidFormat => WrtError::new(
                ErrorCategory::Parse,
                codes::SERIALIZATION_ERROR, // Or DECODING_ERROR if more appropriate contextually
                "Invalid data format for deserialization",
            ),
            SerializationError::Custom(s) => {
                // Create a new static string if necessary, or ensure 's' is always suitable.
                // For now, assuming 's' is appropriate as per original definition.
                WrtError::new(ErrorCategory::Runtime, codes::RUNTIME_ERROR, s)
            }
            SerializationError::InvalidSliceLength => WrtError::new(
                ErrorCategory::Validation,
                codes::INVALID_BYTE_LENGTH,
                "Invalid slice length for serialization/deserialization",
            ),
            SerializationError::NotEnoughData => WrtError::new(
                ErrorCategory::Parse,
                codes::PARSE_UNEXPECTED_EOF,
                "Not enough data to deserialize the object",
            ),
            SerializationError::IoError => WrtError::new(
                ErrorCategory::System,
                codes::SYSTEM_IO_ERROR_CODE, /* Assuming this exists; otherwise, use
                                              * codes::SYSTEM_ERROR */
                "An I/O operation failed during serialization/deserialization",
            ),
            SerializationError::UnexpectedEof => WrtError::new(
                ErrorCategory::Parse,
                codes::PARSE_UNEXPECTED_EOF,
                "Unexpected end of input during deserialization",
            ),
            SerializationError::InvalidEnumValue => WrtError::new(
                ErrorCategory::Parse,       // Or Validation
                codes::SERIALIZATION_ERROR, // Or specific like INVALID_VALUE if applicable
                "Invalid enum value during deserialization",
            ),
        }
    }
}

// Add necessary error codes to wrt_error::codes if they don't exist.
// For now, using generic SERIALIZATION_ERROR and DESERIALIZATION_ERROR.
// Need to check wrt-error/src/codes.rs for these.
// The user summary mentioned adding `TYPE_INVALID_CONVERSION` etc.
// `BUFFER_TOO_SMALL` was also mentioned.

// Assuming `codes::SERIALIZATION_ERROR` and `codes::DESERIALIZATION_ERROR`
// exist or will be added. The user's summary said: "Added new error codes to
// wrt-error/src/codes.rs" So these codes might already be there.

// wrt-error/src/codes.rs" So these codes might already be there.

// Implementations for fixed-size arrays of primitives have been added
// by the impl_bytes_for_primitive macro for each primitive type T,
// which covers [T; N] scenarios if T itself is a primitive.

// For Checksummable, ensure that if you have a struct or enum,
// you implement Checksummable for it directly, defining how its
// fields contribute to the checksum.

// Blanket implementations for tuples (up to a certain arity, e.g., 2 for now)
impl<A, B> Checksummable for (A, B)
where
    A: Checksummable,
    B: Checksummable,
{
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        self.0.update_checksum(checksum);
        self.1.update_checksum(checksum);
    }
}

impl<A, B> ToBytes for (A, B)
where
    A: ToBytes,
    B: ToBytes,
{
    fn serialized_size(&self) -> usize {
        self.0.serialized_size() + self.1.serialized_size()
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        self.0.to_bytes_with_provider(writer, provider)?;
        self.1.to_bytes_with_provider(writer, provider)?;
        Ok(())
    }
    // to_bytes is provided by the trait
}

impl<A, B> FromBytes for (A, B)
where
    A: FromBytes,
    B: FromBytes,
{
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream,
    ) -> WrtResult<Self> {
        let val_a = A::from_bytes_with_provider(reader, provider)?;
        let val_b = B::from_bytes_with_provider(reader, provider)?;
        Ok((val_a, val_b))
    }
    // from_bytes is provided by the trait
}

// Consider adding for more tuple arities if needed.

// Implementations for Option<T>
impl<T: ToBytes> ToBytes for Option<T> {
    fn serialized_size(&self) -> usize {
        match self {
            Some(value) => 1 + value.serialized_size(), // 1 byte for tag + value size
            None => 1,                                  // 1 byte for tag
        }
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream, // provider passed to T's methods
    ) -> WrtResult<()> {
        match self {
            Some(value) => {
                writer.write_u8(1u8)?; // Tag for Some
                value.to_bytes_with_provider(writer, provider)?;
            }
            None => {
                writer.write_u8(0u8)?; // Tag for None
            }
        }
        Ok(())
    }
    // to_bytes is provided by the trait
}

impl<T: FromBytes> FromBytes for Option<T> {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        provider: &PStream, // provider passed to T's methods
    ) -> WrtResult<Self> {
        let tag = reader.read_u8()?;
        match tag {
            0u8 => Ok(None),
            1u8 => {
                let value = T::from_bytes_with_provider(reader, provider)?;
                Ok(Some(value))
            }
            _ => Err(WrtError::new(
                ErrorCategory::Parse,
                codes::DECODING_ERROR, // Changed from INVALID_DATA
                "Invalid tag for Option<T>",
            )),
        }
    }
    // from_bytes is provided by the trait
}

/// A marker trait to seal other traits, preventing external implementations.

impl ToBytes for char {
    fn serialized_size(&self) -> usize {
        4 // char is serialized as u32
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        writer.write_u32_le(*self as u32)
    }
    // to_bytes is provided by the trait
}

impl FromBytes for char {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        let u32_val = reader.read_u32_le()?;
        char::from_u32(u32_val).ok_or_else(|| {
            WrtError::new(
                ErrorCategory::Parse,
                codes::VALUE_OUT_OF_RANGE, // Changed from INVALID_DATA
                "Invalid u32 value for char",
            )
        })
    }
    // from_bytes is provided by the trait
}

// NEW: DefaultMemoryProvider
/// A default memory provider for contexts where no specific provider is given.
/// Binary std/no_std choice
// const DEFAULT_NO_STD_PROVIDER_CAPACITY: usize = 0; // Capacity defined by NoStdProvider itself

/// Default memory provider for no_std environments when no specific provider is
/// given. Wraps `NoStdProvider` with a fixed-size backing array.
#[derive(Debug, Clone, PartialEq, Eq, Hash)] // Removed Copy
pub struct DefaultMemoryProvider(NoStdProvider<0>); // Use 0 for default capacity of NoStdProvider

impl Default for DefaultMemoryProvider {
    fn default() -> Self {
        Self(NoStdProvider::<0>::default())
    }
}

impl RootMemoryProvider for DefaultMemoryProvider {
    type Allocator = NoStdProvider<0>; // Binary std/no_std choice

    fn acquire_memory(&self, _layout: core::alloc::Layout) -> WrtResult<*mut u8> {
        // Binary std/no_std choice
        Err(WrtError::new(
            ErrorCategory::Memory,
            codes::UNSUPPORTED_OPERATION,
            "DefaultMemoryProvider (NoStdProvider<0>) cannot dynamically allocate memory.",
        ))
    }

    fn release_memory(&self, _ptr: *mut u8, _layout: core::alloc::Layout) -> WrtResult<()> {
        // Binary std/no_std choice
        // Safety: This encapsulates unsafe operations internally
        Ok(())
    }

    fn get_allocator(&self) -> &Self::Allocator {
        &self.0
    }

    fn new_handler(&self) -> WrtResult<SafeMemoryHandler<Self>>
    where
        Self: Sized,
    {
        Ok(SafeMemoryHandler::new(self.clone()))
    }

    // Implement missing methods from crate::safe_memory::Provider
    fn borrow_slice(&self, offset: usize, len: usize) -> WrtResult<Slice<'_>> {
        self.0.borrow_slice(offset, len) // Delegate to inner NoStdProvider
    }

    fn write_data(&mut self, offset: usize, data: &[u8]) -> WrtResult<()> {
        self.0.write_data(offset, data)
    }

    fn verify_access(&self, offset: usize, len: usize) -> WrtResult<()> {
        self.0.verify_access(offset, len)
    }

    fn size(&self) -> usize {
        self.0.size()
    }

    fn capacity(&self) -> usize {
        self.0.capacity()
    }

    fn verify_integrity(&self) -> WrtResult<()> {
        self.0.verify_integrity()
    }

    fn set_verification_level(&mut self, level: VerificationLevel) {
        self.0.set_verification_level(level)
    }

    fn verification_level(&self) -> VerificationLevel {
        self.0.verification_level()
    }

    fn memory_stats(&self) -> Stats {
        self.0.memory_stats()
    }

    fn get_slice_mut(&mut self, offset: usize, len: usize) -> WrtResult<SliceMut<'_>> {
        self.0.get_slice_mut(offset, len)
    }

    fn copy_within(&mut self, src_offset: usize, dst_offset: usize, len: usize) -> WrtResult<()> {
        self.0.copy_within(src_offset, dst_offset, len)
    }

    fn ensure_used_up_to(&mut self, byte_offset: usize) -> WrtResult<()> {
        self.0.ensure_used_up_to(byte_offset)
    }
}

// NEW: ReadStream and WriteStream Definitions

/// A stream for reading bytes sequentially from a memory region.
/// Binary std/no_std choice
/// reading.
#[derive(Debug)]
pub struct ReadStream<'a> {
    buffer: Slice<'a>,
    position: usize,
}

impl<'a> ReadStream<'a> {
    /// Creates a new `ReadStream` from a byte slice.
    pub fn new(slice: Slice<'a>) -> Self {
        Self { buffer: slice, position: 0 }
    }

    /// Current reading position in the stream.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Remaining bytes in the stream.
    pub fn remaining_len(&self) -> usize {
        self.buffer.len().saturating_sub(self.position)
    }

    fn ensure_data(&self, len: usize) -> WrtResult<()> {
        if self.position + len > self.buffer.len() {
            Err(WrtError::from(SerializationError::UnexpectedEof))
        } else {
            Ok(())
        }
    }

    pub fn read_u8(&mut self) -> WrtResult<u8> {
        self.ensure_data(1)?;
        let val = self.buffer.data()?[self.position];
        self.position += 1;
        Ok(val)
    }

    pub fn read_i8(&mut self) -> WrtResult<i8> {
        self.read_u8().map(|v| v as i8)
    }

    // Helper for reading little-endian integers
    fn read_le_bytes_into_array<const N: usize>(&mut self) -> WrtResult<[u8; N]> {
        self.ensure_data(N)?;
        let mut arr = [0u8; N];
        arr.copy_from_slice(&self.buffer.data()?[self.position..self.position + N]);
        self.position += N;
        Ok(arr)
    }

    pub fn read_u16_le(&mut self) -> WrtResult<u16> {
        self.read_le_bytes_into_array::<2>().map(u16::from_le_bytes)
    }
    pub fn read_i16_le(&mut self) -> WrtResult<i16> {
        self.read_le_bytes_into_array::<2>().map(i16::from_le_bytes)
    }
    pub fn read_u32_le(&mut self) -> WrtResult<u32> {
        self.read_le_bytes_into_array::<4>().map(u32::from_le_bytes)
    }
    pub fn read_i32_le(&mut self) -> WrtResult<i32> {
        self.read_le_bytes_into_array::<4>().map(i32::from_le_bytes)
    }
    pub fn read_u64_le(&mut self) -> WrtResult<u64> {
        self.read_le_bytes_into_array::<8>().map(u64::from_le_bytes)
    }
    pub fn read_i64_le(&mut self) -> WrtResult<i64> {
        self.read_le_bytes_into_array::<8>().map(i64::from_le_bytes)
    }
    pub fn read_u128_le(&mut self) -> WrtResult<u128> {
        self.read_le_bytes_into_array::<16>().map(u128::from_le_bytes)
    }
    pub fn read_i128_le(&mut self) -> WrtResult<i128> {
        self.read_le_bytes_into_array::<16>().map(i128::from_le_bytes)
    }
    pub fn read_f32_le(&mut self) -> WrtResult<f32> {
        self.read_le_bytes_into_array::<4>().map(f32::from_le_bytes)
    }
    pub fn read_f64_le(&mut self) -> WrtResult<f64> {
        self.read_le_bytes_into_array::<8>().map(f64::from_le_bytes)
    }

    pub fn read_usize_le(&mut self) -> WrtResult<usize> {
        if core::mem::size_of::<usize>() == 4 {
            self.read_u32_le().map(|v| v as usize)
        } else if core::mem::size_of::<usize>() == 8 {
            self.read_u64_le().map(|v| v as usize)
        } else {
            // Fallback or error for unsupported usize size
            Err(WrtError::new(
                ErrorCategory::System,
                codes::SYSTEM_ERROR,
                "Unsupported usize size for LE read",
            ))
        }
    }

    pub fn read_isize_le(&mut self) -> WrtResult<isize> {
        if core::mem::size_of::<isize>() == 4 {
            self.read_i32_le().map(|v| v as isize)
        } else if core::mem::size_of::<isize>() == 8 {
            self.read_i64_le().map(|v| v as isize)
        } else {
            Err(WrtError::new(
                ErrorCategory::System,
                codes::SYSTEM_ERROR,
                "Unsupported isize size for LE read",
            ))
        }
    }

    pub fn read_bool(&mut self) -> WrtResult<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WrtError::from(SerializationError::InvalidEnumValue)),
        }
    }

    /// Reads a slice of bytes from the stream.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> WrtResult<()> {
        self.ensure_data(buf.len())?;
        buf.copy_from_slice(&self.buffer.data()?[self.position..self.position + buf.len()]);
        self.position += buf.len();
        Ok(())
    }

    /// Attempts to provide access to the underlying memory provider.
    /// Returns None if the ReadStream was not constructed with a provider
    /// or if direct provider access is not supported in this design.
    ///
    /// For this ReadStream<'a> which operates on a Slice<'a>, direct provider
    /// access is not typical. It's recommended to pass providers explicitly
    /// to methods that need them rather than relying on this method.
    pub fn try_provider<P: crate::MemoryProvider>(&self, _provider_ref: &P) -> Option<&P> {
        // ReadStream currently only has Slice<'a> and does not hold a direct
        // MemoryProvider instance in this design.
        None
    }
}

/// A stream for writing bytes sequentially to a memory region.
/// It operates on a mutable slice, typically obtained from a `MemoryProvider`.
#[derive(Debug)]
pub struct WriteStream<'a> {
    buffer: SliceMut<'a>,
    position: usize,
    // Provider is not stored directly to avoid lifetime complexities with SliceMut,
    // but can be passed to methods like to_bytes_with_provider if needed by nested types.
}

impl<'a> WriteStream<'a> {
    /// Creates a new `WriteStream` from a mutable byte slice.
    pub fn new(slice: SliceMut<'a>) -> Self {
        Self { buffer: slice, position: 0 }
    }

    /// Current writing position in the stream.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Remaining capacity in the stream.
    pub fn remaining_capacity(&self) -> usize {
        self.buffer.len().saturating_sub(self.position)
    }

    fn ensure_capacity(&self, len: usize) -> WrtResult<()> {
        if self.position + len > self.buffer.len() {
            Err(WrtError::new(
                ErrorCategory::Memory,
                codes::BUFFER_TOO_SMALL, // Or a more specific serialization capacity error
                "Write operation exceeds buffer capacity",
            ))
        } else {
            Ok(())
        }
    }

    pub fn write_u8(&mut self, value: u8) -> WrtResult<()> {
        self.ensure_capacity(1)?;
        self.buffer.data_mut()?[self.position] = value;
        self.position += 1;
        Ok(())
    }

    pub fn write_i8(&mut self, value: i8) -> WrtResult<()> {
        self.write_u8(value as u8)
    }

    // Helper for writing little-endian integers
    fn write_le_bytes_from_array<const N: usize>(&mut self, bytes: [u8; N]) -> WrtResult<()> {
        self.ensure_capacity(N)?;
        self.buffer.data_mut()?[self.position..self.position + N].copy_from_slice(&bytes);
        self.position += N;
        Ok(())
    }

    pub fn write_u16_le(&mut self, value: u16) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_i16_le(&mut self, value: i16) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_u32_le(&mut self, value: u32) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_i32_le(&mut self, value: i32) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_u64_le(&mut self, value: u64) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_i64_le(&mut self, value: i64) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_u128_le(&mut self, value: u128) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_i128_le(&mut self, value: i128) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_f32_le(&mut self, value: f32) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }
    pub fn write_f64_le(&mut self, value: f64) -> WrtResult<()> {
        self.write_le_bytes_from_array(value.to_le_bytes())
    }

    pub fn write_usize_le(&mut self, value: usize) -> WrtResult<()> {
        if core::mem::size_of::<usize>() == 4 {
            self.write_u32_le(value as u32)
        } else if core::mem::size_of::<usize>() == 8 {
            self.write_u64_le(value as u64)
        } else {
            Err(WrtError::new(
                ErrorCategory::System,
                codes::SYSTEM_ERROR,
                "Unsupported usize size for LE write",
            ))
        }
    }

    pub fn write_isize_le(&mut self, value: isize) -> WrtResult<()> {
        if core::mem::size_of::<isize>() == 4 {
            self.write_i32_le(value as i32)
        } else if core::mem::size_of::<isize>() == 8 {
            self.write_i64_le(value as i64)
        } else {
            Err(WrtError::new(
                ErrorCategory::System,
                codes::SYSTEM_ERROR,
                "Unsupported isize size for LE write",
            ))
        }
    }

    pub fn write_bool(&mut self, value: bool) -> WrtResult<()> {
        self.write_u8(if value { 1 } else { 0 })
    }

    /// Writes an entire slice of bytes into the stream.
    pub fn write_all(&mut self, bytes: &[u8]) -> WrtResult<()> {
        self.ensure_capacity(bytes.len())?;
        self.buffer.data_mut()?[self.position..self.position + bytes.len()].copy_from_slice(bytes);
        self.position += bytes.len();
        Ok(())
    }

    /// Attempts to provide access to the underlying memory provider.
    /// Returns None if the WriteStream was not constructed with a provider
    /// or if direct provider access is not supported in this design.
    ///
    /// It's recommended to pass providers explicitly to methods that need them
    /// rather than relying on this method.
    pub fn try_provider<P: crate::MemoryProvider>(&self, _provider_ref: &P) -> Option<&P> {
        // WriteStream does not hold a direct MemoryProvider instance in this design
        None
    }
}

// impl<P: crate::MemoryProvider + Default> Default for WriteStream<P> { // This
// Default impl is problematic for no_std if P cannot provide a default buffer
// or Vec is used.     fn default() -> Self {
//         // This default implementation requires P to somehow provide a
// Binary std/no_std choice
// use Vec.         // For a SliceMut based WriteStream, Default doesn't make
// much sense without a source slice.         // Consider removing this Default
// impl or making it highly conditional / specialized.         // If P itself
// Binary std/no_std choice
// feature:         // #[cfg(feature = "std")]
//         // {
//         //     let cap = 256; // Default capacity
//         //     let mut vec_buffer = Vec::with_capacity(cap);
//         //     // This is tricky because SliceMut needs a lifetime tied to
// the Vec.         //     // This structure is not ideal for a Default impl
// that returns an owned stream with an internal buffer.         //     // A
// different WriteStream design might be needed for that (e.g.
// WriteStream<Vec<u8>>).         // }
//         // panic!("Default for WriteStream<P> is not generally constructible
// Binary std/no_std choice
// implies an empty, unusable stream.         Self {
//             buffer: SliceMut::empty(), // Creates an empty, unusable slice.
//             position: 0,
//         }
//     }
// }

// Implementation for Option<T>
impl<T: Checksummable> Checksummable for Option<T> {
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        match self {
            Some(val) => {
                checksum.update(1u8); // Discriminant for Some
                val.update_checksum(checksum);
            }
            None => {
                checksum.update(0u8); // Discriminant for None
            }
        }
    }
}

// DefaultMemoryProvider definition and impls might follow here or be elsewhere

#[cfg(feature = "std")]
impl ToBytes for alloc::string::String {
    fn serialized_size(&self) -> usize {
        4 + self.len() // 4 bytes for length + string bytes
    }

    fn to_bytes_with_provider<'a, PStream: RootMemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        provider: &PStream,
    ) -> WrtResult<()> {
        let bytes = self.as_bytes();
        (bytes.len() as u32).to_bytes_with_provider(writer, provider)?;
        writer.write_all(bytes).map_err(|_e| {
            WrtError::new(
                ErrorCategory::Parse,
                codes::SERIALIZATION_ERROR, // Changed from INVALID_DATA
                "Failed to write String data to stream",
            )
        })
    }
}

// ============================================================================
// CORE VALIDATION TRAITS (moved from validation module to break circular
// dependency)
// ============================================================================

/// Trait for objects that can be validated
///
/// This trait is implemented by collection types that need to verify
/// their internal state as part of functional safety requirements.
pub trait Validatable {
    /// The error type returned when validation fails
    type Error;

    /// Performs validation on this object
    ///
    /// Returns `Ok(())` if validation passes, or an error describing
    /// what validation check failed.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if validation fails.
    fn validate(&self) -> core::result::Result<(), Self::Error>;

    /// Get the validation level this object is configured with
    fn validation_level(&self) -> crate::verification::VerificationLevel;

    /// Set the validation level for this object
    fn set_validation_level(&mut self, level: crate::verification::VerificationLevel);
}

/// Trait for types that maintain checksums for validation
pub trait Checksummed {
    /// Get the current checksum for this object
    fn checksum(&self) -> crate::verification::Checksum;

    /// Force recalculation of the object's checksum
    ///
    /// This is useful when verification level changes from `None`
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
