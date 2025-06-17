// WRT - wrt-foundation
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#[cfg(all(not(feature = "std")))]
use crate::safe_managed_alloc;
extern crate alloc;

#[cfg(all(not(feature = "std")))]
use alloc::format;
use core::fmt;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
use core::fmt::Debug;
#[cfg(feature = "std")]
use std::fmt::Debug;

// optional imports
// #[cfg(feature = "component-model-resources")]
// use crate::bounded::BoundedVec;
use crate::{
    bounded::{BoundedString, BoundedVec, WasmName, MAX_WASM_NAME_LENGTH},
    prelude::{str, Eq, PartialEq},
    safe_memory::NoStdProvider,
    traits::{Checksummable, FromBytes, ReadStream, SerializationError, ToBytes, WriteStream},
    types::ValueType,
    verification::{Checksum, VerificationLevel},
    MemoryProvider, WrtResult,
};

// use crate::prelude::{format, ToString, String as PreludeString, Vec as
// PreludeVec}; use crate::prelude::{Debug, Display, Eq, PartialEq};
// use crate::traits::{Checksummable, FromBytes, ToBytes, Validatable};

/// Resource identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

/// Resource New operation data
#[derive(Debug, Clone)]
pub struct ResourceNew {
    /// Type index for resource type
    pub type_idx: u32,
}

/// Resource Drop operation data
#[derive(Debug, Clone)]
pub struct ResourceDrop {
    /// Type index for resource type
    pub type_idx: u32,
}

/// Resource Rep operation data
#[derive(Debug, Clone)]
pub struct ResourceRep {
    /// Type index for resource type
    pub type_idx: u32,
}

/// Operations that can be performed on resources
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceOperation {
    /// Read access to a resource
    Read,
    /// Write access to a resource
    Write,
    /// Execute a resource as code
    Execute,
    /// Create a new resource
    Create,
    /// Delete an existing resource
    Delete,
    /// Reference a resource (borrow it)
    Reference,
    /// Dereference a resource (access it through a reference)
    Dereference,
}

/// Resource operation in a canonical function
#[derive(Debug, Clone)]
pub enum ResourceCanonicalOperation {
    /// New resource operation
    New(ResourceNew),
    /// Drop a resource
    Drop(ResourceDrop),
    /// Resource representation operation
    Rep(ResourceRep),
}

impl ResourceOperation {
    /// Check if the operation requires read access
    #[must_use]
    pub fn requires_read(&self) -> bool {
        matches!(
            self,
            ResourceOperation::Read | ResourceOperation::Execute | ResourceOperation::Dereference
        )
    }

    /// Check if the operation requires write access
    #[must_use]
    pub fn requires_write(&self) -> bool {
        matches!(
            self,
            ResourceOperation::Write
                | ResourceOperation::Create
                | ResourceOperation::Delete
                | ResourceOperation::Reference
        )
    }

    /// Get the string representation of the operation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceOperation::Read => "read",
            ResourceOperation::Write => "write",
            ResourceOperation::Execute => "execute",
            ResourceOperation::Create => "create",
            ResourceOperation::Delete => "delete",
            ResourceOperation::Reference => "reference",
            ResourceOperation::Dereference => "dereference",
        }
    }
}

impl fmt::Display for ResourceOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::str::FromStr for ResourceOperation {
    type Err = wrt_error::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "read" => Ok(ResourceOperation::Read),
            "write" => Ok(ResourceOperation::Write),
            "execute" => Ok(ResourceOperation::Execute),
            "create" => Ok(ResourceOperation::Create),
            "delete" => Ok(ResourceOperation::Delete),
            "reference" => Ok(ResourceOperation::Reference),
            "dereference" => Ok(ResourceOperation::Dereference),
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Parse,
                wrt_error::codes::PARSE_ERROR,
                "Unknown resource operation",
            )),
        }
    }
}

/// Resource representation type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceRepresentation {
    /// 32-bit handle representation
    Handle32,
    /// 64-bit handle representation
    Handle64,
    /// Record representation with field names
    #[cfg(feature = "std")]
    Record(
        BoundedVec<
            BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, crate::safe_memory::NoStdProvider<4096>>,
            MAX_RESOURCE_FIELDS,
            crate::safe_memory::NoStdProvider<4096>,
        >,
    ),
    /// Aggregate representation with type indices
    #[cfg(feature = "std")]
    Aggregate(BoundedVec<u32, MAX_RESOURCE_AGGREGATE_IDS, crate::safe_memory::NoStdProvider<4096>>),
    /// Binary std/no_std choice
    #[cfg(not(feature = "std"))]
    Record,
}

impl ResourceRepresentation {
    /// Get the string representation of the representation type
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceRepresentation::Handle32 => "handle32",
            ResourceRepresentation::Handle64 => "handle64",
            #[cfg(feature = "std")]
            ResourceRepresentation::Record(_) => "record",
            #[cfg(feature = "std")]
            ResourceRepresentation::Aggregate(_) => "aggregate",
            #[cfg(not(feature = "std"))]
            ResourceRepresentation::Record => "record",
        }
    }
}

impl fmt::Display for ResourceRepresentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::str::FromStr for ResourceRepresentation {
    type Err = wrt_error::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "handle32" => Ok(ResourceRepresentation::Handle32),
            "handle64" => Ok(ResourceRepresentation::Handle64),
            "record" => {
                #[cfg(feature = "std")]
                {
                    use crate::budget_aware_provider::CrateId;
                    use crate::safe_managed_alloc;

                    let provider = safe_managed_alloc!(4096, CrateId::Foundation)?;

                    Ok(ResourceRepresentation::Record(BoundedVec::new(provider).map_err(|_e| {
                        wrt_error::Error::new(
                            wrt_error::ErrorCategory::Memory,
                            wrt_error::codes::MEMORY_ALLOCATION_ERROR,
                            "Failed to create BoundedVec for ResourceRepresentation::Record",
                        )
                    })?))
                }
                #[cfg(not(feature = "std"))]
                {
                    Ok(ResourceRepresentation::Record)
                }
            }
            "aggregate" => {
                #[cfg(feature = "std")]
                {
                    use crate::budget_aware_provider::CrateId;
                    use crate::safe_managed_alloc;

                    let provider = safe_managed_alloc!(4096, CrateId::Foundation)?;

                    Ok(ResourceRepresentation::Aggregate(BoundedVec::new(provider).map_err(
                        |_e| {
                            wrt_error::Error::new(
                                wrt_error::ErrorCategory::Memory,
                                wrt_error::codes::MEMORY_ALLOCATION_ERROR,
                                "Failed to create BoundedVec for ResourceRepresentation::Aggregate",
                            )
                        },
                    )?))
                }
                #[cfg(not(feature = "std"))]
                {
                    Err(wrt_error::Error::new(
                        wrt_error::ErrorCategory::Parse,
                        wrt_error::codes::PARSE_ERROR,
                        "ResourceRepresentation::Aggregate is not available without the 'alloc' \
                         feature",
                    ))
                }
            }
            _ => Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Parse,
                wrt_error::codes::PARSE_ERROR,
                "Unknown resource representation",
            )),
        }
    }
}

// Define constants for bounded collections
/// Maximum length of a resource field name
pub const MAX_RESOURCE_FIELD_NAME_LEN: usize = 64;
/// Maximum number of fields in a resource record
pub const MAX_RESOURCE_FIELDS: usize = 16;
/// Maximum number of IDs in a resource aggregate
pub const MAX_RESOURCE_AGGREGATE_IDS: usize = 16;

/// Represents different kinds of resource operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceRepr<P: MemoryProvider + Default + Clone + Eq + Debug> {
    /// The resource is represented as a simple primitive type.
    Primitive(ValueType),
    /// The resource is a record with named fields.
    Record(BoundedVec<BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, P>, MAX_RESOURCE_FIELDS, P>),
    /// The resource is a list of a single element type.
    List(ValueType),
    /// The resource is an aggregate of other resource IDs
    /// (implementation-defined).
    Aggregate(BoundedVec<u32, MAX_RESOURCE_AGGREGATE_IDS, P>),
    /// The resource is opaque, its structure is not known to the runtime.
    Opaque,
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> Checksummable for ResourceRepr<P>
where
    ValueType: Checksummable, // Already is
    BoundedVec<BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, P>, MAX_RESOURCE_FIELDS, P>:
        Checksummable,
    BoundedVec<u32, MAX_RESOURCE_AGGREGATE_IDS, P>: Checksummable,
    BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, P>: Checksummable,
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        let discriminant_byte = match self {
            ResourceRepr::Primitive(_) => DISCRIMINANT_RESOURCE_REPR_PRIMITIVE,
            ResourceRepr::Record(_) => DISCRIMINANT_RESOURCE_REPR_RECORD,
            ResourceRepr::List(_) => DISCRIMINANT_RESOURCE_REPR_LIST,
            ResourceRepr::Aggregate(_) => DISCRIMINANT_RESOURCE_REPR_AGGREGATE,
            ResourceRepr::Opaque => DISCRIMINANT_RESOURCE_REPR_OPAQUE,
        };
        checksum.update(discriminant_byte);

        match self {
            ResourceRepr::Primitive(vt) => vt.update_checksum(checksum),
            ResourceRepr::Record(fields) => fields.update_checksum(checksum),
            ResourceRepr::List(vt) => vt.update_checksum(checksum),
            ResourceRepr::Aggregate(ids) => ids.update_checksum(checksum),
            ResourceRepr::Opaque => {} // No data to checksum
        }
    }
}

// Discriminants for ResourceRepr variants
const DISCRIMINANT_RESOURCE_REPR_PRIMITIVE: u8 = 0;
const DISCRIMINANT_RESOURCE_REPR_RECORD: u8 = 1;
const DISCRIMINANT_RESOURCE_REPR_LIST: u8 = 2;
const DISCRIMINANT_RESOURCE_REPR_AGGREGATE: u8 = 3;
const DISCRIMINANT_RESOURCE_REPR_OPAQUE: u8 = 4;

// Resource Type serialization discriminants
const DISCRIMINANT_RESOURCE_TYPE_RECORD: u8 = 0;
const DISCRIMINANT_RESOURCE_TYPE_AGGREGATE: u8 = 1;

impl<P: MemoryProvider + Default + Clone + Eq + Debug> ToBytes for ResourceRepr<P> {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        match self {
            ResourceRepr::Primitive(val_type) => {
                writer.write_u8(0)?; // Tag for Primitive
                val_type.to_bytes_with_provider(writer, stream_provider) // ValueType uses stream_provider
            }
            ResourceRepr::Record(fields) => {
                writer.write_u8(1)?; // Tag for Record
                fields.to_bytes_with_provider(writer, stream_provider) // BoundedVec takes stream_provider
            }
            ResourceRepr::List(val_type) => {
                writer.write_u8(2)?; // Tag for List
                val_type.to_bytes_with_provider(writer, stream_provider) // ValueType uses stream_provider
            }
            ResourceRepr::Aggregate(ids) => {
                writer.write_u8(3)?; // Tag for Aggregate
                ids.to_bytes_with_provider(writer, stream_provider) // BoundedVec takes stream_provider
            }
            ResourceRepr::Opaque => writer.write_u8(4), // Tag for Opaque
        }
    }
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> FromBytes for ResourceRepr<P> {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let tag = reader.read_u8()?;
        match tag {
            0 => {
                let val_type = ValueType::from_bytes_with_provider(reader, stream_provider)?;
                Ok(ResourceRepr::Primitive(val_type))
            }
            1 => {
                let fields = BoundedVec::<
                    BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, P>,
                    MAX_RESOURCE_FIELDS,
                    P,
                >::from_bytes_with_provider(reader, stream_provider)
                .map_err(wrt_error::Error::from)?;
                Ok(ResourceRepr::Record(fields))
            }
            2 => {
                let val_type = ValueType::from_bytes_with_provider(reader, stream_provider)?;
                Ok(ResourceRepr::List(val_type))
            }
            3 => {
                let ids =
                    BoundedVec::<u32, MAX_RESOURCE_AGGREGATE_IDS, P>::from_bytes_with_provider(
                        reader,
                        stream_provider,
                    )?;
                Ok(ResourceRepr::Aggregate(ids))
            }
            4 => Ok(ResourceRepr::Opaque),
            _ => Err(SerializationError::Custom("Invalid tag for ResourceRepr").into()),
        }
    }
}

/// Represents a resource, typically identified by an ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resource<P: MemoryProvider + Default + Clone + Eq + Debug> {
    /// Unique identifier for the resource.
    pub id: u32,
    /// The representation or type of the resource.
    pub repr: ResourceRepr<P>,
    /// Optional human-readable name for the resource.
    // Using WasmName (which uses BoundedString internally)
    pub name: Option<WasmName<MAX_WASM_NAME_LENGTH, P>>, // MAX_WASM_NAME_LENGTH from bounded.rs
    /// Verification level for operations on this resource.
    verification_level: VerificationLevel,
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> Resource<P> {
    /// Creates a new resource.
    pub fn new(
        id: u32,
        repr: ResourceRepr<P>,
        name: Option<WasmName<MAX_WASM_NAME_LENGTH, P>>,
        verification_level: VerificationLevel,
    ) -> Self {
        Self { id, repr, name, verification_level }
    }

    /// Gets the verification level of the resource.
    pub fn verification_level(&self) -> VerificationLevel {
        self.verification_level
    }

    /// Sets the verification level for the resource.
    pub fn set_verification_level(&mut self, level: VerificationLevel) {
        self.verification_level = level;
    }
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> Checksummable for Resource<P>
where
    ResourceRepr<P>: Checksummable,
    Option<WasmName<MAX_WASM_NAME_LENGTH, P>>: Checksummable, // Assuming WasmName is Checksummable
{
    fn update_checksum(&self, checksum: &mut Checksum) {
        self.id.update_checksum(checksum);
        self.repr.update_checksum(checksum);
        self.name.update_checksum(checksum);
        // verification_level is metadata, not usually part of checksummed data
    }
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> ToBytes for Resource<P> {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        self.id.to_bytes_with_provider(writer, stream_provider)?; // u32 doesn't use provider, but trait requires it
        self.repr.to_bytes_with_provider(writer, stream_provider)?;
        self.name.to_bytes_with_provider(writer, stream_provider)?;
        self.verification_level.to_bytes_with_provider(writer, stream_provider)?; // VerificationLevel is simple
        Ok(())
    }
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> FromBytes for Resource<P> {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let id = u32::from_bytes_with_provider(reader, stream_provider)?; // u32 doesn't use provider, but trait requires it
        let repr = ResourceRepr::<P>::from_bytes_with_provider(reader, stream_provider)?;
        let name = Option::<WasmName<MAX_WASM_NAME_LENGTH, P>>::from_bytes_with_provider(
            reader,
            stream_provider,
        )?;
        let verification_level =
            VerificationLevel::from_bytes_with_provider(reader, stream_provider)?; // VerificationLevel is simple
        Ok(Resource { id, repr, name, verification_level })
    }
}

/// Represents the type of a resource, which can be a record or an aggregate
/// (handle to other resources).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType<P: MemoryProvider + Default + Clone + Eq> {
    /// A resource represented as a record of named fields (strings).
    Record(BoundedVec<BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, P>, MAX_RESOURCE_FIELDS, P>),
    /// A resource that is an aggregate of other resource IDs.
    Aggregate(BoundedVec<u32, MAX_RESOURCE_AGGREGATE_IDS, P>),
}

/// Represents a single resource item in the store, including its ID, optional
/// name, and type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceItem<P: MemoryProvider + Default + Clone + Eq> {
    /// Unique identifier for the resource.
    pub id: u32,
    /// The type of the resource.
    pub type_: ResourceType<P>,
    /// Optional human-readable name for the resource.
    pub name: Option<WasmName<MAX_WASM_NAME_LENGTH, P>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceTableIdx(pub u32);

impl ToBytes for ResourceTableIdx {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        _provider: &PStream, // Not used by u32
    ) -> WrtResult<()> {
        self.0.to_bytes_with_provider(writer, _provider) // u32's to_bytes
                                                         // doesn't take
                                                         // provider
    }
}

impl FromBytes for ResourceTableIdx {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        _provider: &PStream, // Not used by u32
    ) -> WrtResult<Self> {
        let val = u32::from_bytes_with_provider(reader, _provider)?; // u32's from_bytes doesn't take provider
        Ok(ResourceTableIdx(val))
    }
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> ToBytes for ResourceType<P> {
    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut WriteStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<()> {
        match self {
            ResourceType::Record(fields) => {
                writer.write_u8(DISCRIMINANT_RESOURCE_TYPE_RECORD)?;
                fields.to_bytes_with_provider(writer, stream_provider)?
            }
            ResourceType::Aggregate(ids) => {
                writer.write_u8(DISCRIMINANT_RESOURCE_TYPE_AGGREGATE)?;
                ids.to_bytes_with_provider(writer, stream_provider)?
            }
        }
        Ok(())
    }
}

impl<P: MemoryProvider + Default + Clone + Eq + Debug> FromBytes for ResourceType<P> {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut ReadStream<'a>,
        stream_provider: &PStream,
    ) -> WrtResult<Self> {
        let tag = reader.read_u8()?;
        match tag {
            DISCRIMINANT_RESOURCE_TYPE_RECORD => {
                // Read the fields for the Record variant
                let fields = BoundedVec::<
                    BoundedString<MAX_RESOURCE_FIELD_NAME_LEN, P>,
                    MAX_RESOURCE_FIELDS,
                    P,
                >::from_bytes_with_provider(reader, stream_provider)
                .map_err(wrt_error::Error::from)?;
                Ok(ResourceType::Record(fields))
            }
            DISCRIMINANT_RESOURCE_TYPE_AGGREGATE => {
                // Read the aggregate IDs
                let ids =
                    BoundedVec::<u32, MAX_RESOURCE_AGGREGATE_IDS, P>::from_bytes_with_provider(
                        reader,
                        stream_provider,
                    )?;
                Ok(ResourceType::Aggregate(ids))
            }
            _ => Err(SerializationError::Custom("Invalid tag for ResourceType").into()),
        }
    }
}
