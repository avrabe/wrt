//! WebAssembly Shared Memory Implementation
//!
//! This module implements the WebAssembly shared memory type system required
//! for multi-threaded applications. Shared memory allows multiple threads to
//! access the same linear memory with proper atomic synchronization.

use crate::prelude::*;
use crate::traits::{Checksummable, FromBytes, ToBytes, Validatable};
use crate::WrtResult;
use wrt_error::{codes, Error, ErrorCategory, Result};

#[cfg(feature = "std")]
use std::sync::{Arc, RwLock};

/// WebAssembly memory type supporting both linear and shared memory
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    /// Standard linear memory (not shared between threads)
    Linear {
        /// Minimum number of pages
        min: u32,
        /// Maximum number of pages (optional)
        max: Option<u32>,
    },
    /// Shared memory accessible by multiple threads
    Shared {
        /// Minimum number of pages
        min: u32,
        /// Maximum number of pages (required for shared memory)
        max: u32,
    },
}

impl MemoryType {
    /// Check if this is a shared memory type
    pub fn is_shared(&self) -> bool {
        matches!(self, MemoryType::Shared { .. })
    }

    /// Get minimum page count
    pub fn min_pages(&self) -> u32 {
        match self {
            MemoryType::Linear { min, .. } | MemoryType::Shared { min, .. } => *min,
        }
    }

    /// Get maximum page count
    pub fn max_pages(&self) -> Option<u32> {
        match self {
            MemoryType::Linear { max, .. } => *max,
            MemoryType::Shared { max, .. } => Some(*max),
        }
    }

    /// Validate memory type constraints
    pub fn validate(&self) -> Result<()> {
        match self {
            MemoryType::Linear { min, max } => {
                if let Some(max_val) = max {
                    if min > max_val {
                        return Err(Error::new(
                            ErrorCategory::Validation,
                            codes::VALIDATION_ERROR,
                            "Linear memory minimum exceeds maximum",
                        ));
                    }
                    if *max_val > (1 << 16) {
                        return Err(Error::new(
                            ErrorCategory::Validation,
                            codes::VALIDATION_ERROR,
                            "Linear memory maximum exceeds 64K pages",
                        ));
                    }
                }
                Ok(())
            }
            MemoryType::Shared { min, max } => {
                if min > max {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        "Shared memory minimum exceeds maximum",
                    ));
                }
                if *max > (1 << 16) {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        "Shared memory maximum exceeds 64K pages",
                    ));
                }
                // Shared memory requires maximum to be specified
                Ok(())
            }
        }
    }

    /// Check if this memory type is compatible with another for merging
    pub fn is_compatible_with(&self, other: &MemoryType) -> bool {
        match (self, other) {
            (MemoryType::Linear { .. }, MemoryType::Linear { .. }) => true,
            (MemoryType::Shared { .. }, MemoryType::Shared { .. }) => true,
            _ => false, // Cannot mix shared and linear memory
        }
    }
}

impl ToBytes for MemoryType {
    fn serialized_size(&self) -> usize {
        // Basic size calculation: 1 byte for type flag, 4 bytes for min, potentially 4 bytes for max
        match self {
            MemoryType::Linear { max: Some(_), .. } => 1 + 4 + 1 + 4, // flag + min + has_max + max
            MemoryType::Linear { max: None, .. } => 1 + 4 + 1,        // flag + min + has_max
            MemoryType::Shared { .. } => 1 + 4 + 4,                   // flag + min + max
        }
    }

    fn to_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        &self,
        writer: &mut crate::traits::WriteStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<()> {
        match self {
            MemoryType::Linear { min, max } => {
                writer.write_u8(0x00)?; // Linear memory flag
                writer.write_u32_le(*min)?;
                if let Some(max_val) = max {
                    writer.write_u8(0x01)?; // Has max
                    writer.write_u32_le(*max_val)?;
                } else {
                    writer.write_u8(0x00)?; // No max
                }
            }
            MemoryType::Shared { min, max } => {
                writer.write_u8(0x01)?; // Shared memory flag
                writer.write_u32_le(*min)?;
                writer.write_u32_le(*max)?;
            }
        }
        Ok(())
    }
}

impl FromBytes for MemoryType {
    fn from_bytes_with_provider<'a, PStream: crate::MemoryProvider>(
        reader: &mut crate::traits::ReadStream<'a>,
        _provider: &PStream,
    ) -> WrtResult<Self> {
        let memory_flag = reader.read_u8()?;
        let min = reader.read_u32_le()?;

        match memory_flag {
            0x00 => {
                // Linear memory
                let has_max = reader.read_u8()?;
                let max = if has_max == 0x01 { Some(reader.read_u32_le()?) } else { None };
                Ok(MemoryType::Linear { min, max })
            }
            0x01 => {
                // Shared memory
                let max = reader.read_u32_le()?;
                Ok(MemoryType::Shared { min, max })
            }
            _ => Err(Error::new(
                ErrorCategory::Parse,
                codes::PARSE_ERROR,
                "Invalid memory type flag",
            )),
        }
    }
}

impl Checksummable for MemoryType {
    fn update_checksum(&self, checksum: &mut crate::verification::Checksum) {
        // Update checksum based on memory type
        match self {
            MemoryType::Linear { min, max } => {
                checksum.update(0); // Linear type indicator
                checksum.update_slice(&min.to_le_bytes());
                if let Some(max_val) = max {
                    checksum.update(1); // Has max indicator
                    checksum.update_slice(&max_val.to_le_bytes());
                } else {
                    checksum.update(0); // No max indicator
                }
            }
            MemoryType::Shared { min, max } => {
                checksum.update(1); // Shared type indicator
                checksum.update_slice(&min.to_le_bytes());
                checksum.update_slice(&max.to_le_bytes());
            }
        }
    }
}

impl core::hash::Hash for MemoryType {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            MemoryType::Linear { min, max } => {
                0u8.hash(state);
                min.hash(state);
                max.hash(state);
            }
            MemoryType::Shared { min, max } => {
                1u8.hash(state);
                min.hash(state);
                max.hash(state);
            }
        }
    }
}

impl Validatable for MemoryType {
    type Error = Error;

    fn validate(&self) -> core::result::Result<(), Self::Error> {
        match self {
            MemoryType::Linear { min, max } => {
                if let Some(max_val) = max {
                    if min > max_val {
                        return Err(Error::new(
                            ErrorCategory::Validation,
                            codes::VALIDATION_ERROR,
                            "Linear memory minimum exceeds maximum",
                        ));
                    }
                    if *max_val > (1 << 16) {
                        return Err(Error::new(
                            ErrorCategory::Validation,
                            codes::VALIDATION_ERROR,
                            "Linear memory maximum exceeds 64K pages",
                        ));
                    }
                }
                Ok(())
            }
            MemoryType::Shared { min, max } => {
                if min > max {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        "Shared memory minimum exceeds maximum",
                    ));
                }
                if *max > (1 << 16) {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        "Shared memory maximum exceeds 64K pages",
                    ));
                }
                Ok(())
            }
        }
    }

    fn validation_level(&self) -> crate::verification::VerificationLevel {
        crate::verification::VerificationLevel::Standard
    }

    fn set_validation_level(&mut self, _level: crate::verification::VerificationLevel) {
        // MemoryType doesn't store validation level, so this is a no-op
    }
}

impl Default for MemoryType {
    fn default() -> Self {
        MemoryType::Linear { min: 0, max: Some(1) }
    }
}

/// Shared memory access control
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedMemoryAccess {
    /// Read-only access
    ReadOnly,
    /// Read-write access
    ReadWrite,
    /// Execute access (for code segments)
    Execute,
}

/// Shared memory segment descriptor
#[derive(Debug, Clone)]
pub struct SharedMemorySegment {
    /// Memory type
    pub memory_type: MemoryType,
    /// Access permissions
    pub access: SharedMemoryAccess,
    /// Base address offset
    pub offset: u64,
    /// Size in bytes
    pub size: u64,
    /// Whether this segment supports atomic operations
    pub atomic_capable: bool,
}

impl SharedMemorySegment {
    /// Create new shared memory segment
    pub fn new(
        memory_type: MemoryType,
        access: SharedMemoryAccess,
        offset: u64,
        size: u64,
        atomic_capable: bool,
    ) -> Result<Self> {
        memory_type.validate()?;

        if !memory_type.is_shared() && atomic_capable {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Atomic operations require shared memory",
            ));
        }

        Ok(Self { memory_type, access, offset, size, atomic_capable })
    }

    /// Check if this segment overlaps with another
    pub fn overlaps_with(&self, other: &SharedMemorySegment) -> bool {
        let self_end = self.offset + self.size;
        let other_end = other.offset + other.size;

        !(self_end <= other.offset || other_end <= self.offset)
    }

    /// Check if an address is within this segment
    pub fn contains_address(&self, address: u64) -> bool {
        address >= self.offset && address < self.offset + self.size
    }

    /// Check if atomic operations are allowed at given address
    pub fn allows_atomic_at(&self, address: u64) -> bool {
        self.atomic_capable && self.contains_address(address) && self.memory_type.is_shared()
    }
}

/// Shared memory manager for coordinating access between threads
#[derive(Debug)]
pub struct SharedMemoryManager {
    /// Registered memory segments
    #[cfg(feature = "std")]
    segments: Vec<SharedMemorySegment>,
    #[cfg(not(feature = "std"))]
    segments: [Option<SharedMemorySegment>; 64],

    /// Access statistics
    pub stats: SharedMemoryStats,
}

impl SharedMemoryManager {
    /// Create new shared memory manager
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            segments: Vec::new(),
            #[cfg(not(feature = "std"))]
            segments: [const { None }; 64],
            stats: SharedMemoryStats::new(),
        }
    }

    /// Register a shared memory segment
    pub fn register_segment(&mut self, segment: SharedMemorySegment) -> Result<usize> {
        // Check for overlaps with existing segments
        #[cfg(feature = "std")]
        {
            for existing in &self.segments {
                if segment.overlaps_with(existing) {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        "Memory segment overlaps with existing segment",
                    ));
                }
            }

            let id = self.segments.len();
            self.segments.push(segment);
            self.stats.registered_segments += 1;
            Ok(id)
        }
        #[cfg(not(feature = "std"))]
        {
            for existing_slot in &self.segments {
                if let Some(existing) = existing_slot {
                    if segment.overlaps_with(existing) {
                        return Err(Error::new(
                            ErrorCategory::Validation,
                            codes::VALIDATION_ERROR,
                            "Memory segment overlaps with existing segment",
                        ));
                    }
                }
            }

            // Find empty slot
            for (id, slot) in self.segments.iter_mut().enumerate() {
                if slot.is_none() {
                    *slot = Some(segment);
                    self.stats.registered_segments += 1;
                    return Ok(id);
                }
            }

            Err(Error::new(
                ErrorCategory::Resource,
                codes::MEMORY_ERROR,
                "Maximum number of memory segments reached",
            ))
        }
    }

    /// Check if atomic operations are allowed at the given address
    pub fn allows_atomic_at(&self, address: u64) -> bool {
        #[cfg(feature = "std")]
        {
            self.segments.iter().any(|seg| seg.allows_atomic_at(address))
        }
        #[cfg(not(feature = "std"))]
        {
            self.segments
                .iter()
                .filter_map(|slot| slot.as_ref())
                .any(|seg| seg.allows_atomic_at(address))
        }
    }

    /// Get segment containing the given address
    pub fn get_segment_for_address(&self, address: u64) -> Option<&SharedMemorySegment> {
        #[cfg(feature = "std")]
        {
            self.segments.iter().find(|seg| seg.contains_address(address))
        }
        #[cfg(not(feature = "std"))]
        {
            self.segments
                .iter()
                .filter_map(|slot| slot.as_ref())
                .find(|seg| seg.contains_address(address))
        }
    }

    /// Validate memory access at given address
    pub fn validate_access(&mut self, address: u64, access_type: SharedMemoryAccess) -> Result<()> {
        if let Some(segment) = self.get_segment_for_address(address) {
            match (&segment.access, &access_type) {
                (SharedMemoryAccess::ReadOnly, SharedMemoryAccess::ReadOnly) => Ok(()),
                (SharedMemoryAccess::ReadWrite, _) => Ok(()),
                (SharedMemoryAccess::Execute, SharedMemoryAccess::Execute) => Ok(()),
                _ => Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::EXECUTION_ERROR,
                    "Memory access permission denied",
                )),
            }?;

            self.stats.memory_accesses += 1;
            Ok(())
        } else {
            Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                "Memory address not in any registered segment",
            ))
        }
    }
}

impl Default for SharedMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for shared memory usage
#[derive(Debug, Clone)]
pub struct SharedMemoryStats {
    /// Number of registered memory segments
    pub registered_segments: u64,
    /// Total memory accesses performed
    pub memory_accesses: u64,
    /// Number of atomic operations performed
    pub atomic_operations: u64,
    /// Number of access violations detected
    pub access_violations: u64,
}

impl SharedMemoryStats {
    fn new() -> Self {
        Self {
            registered_segments: 0,
            memory_accesses: 0,
            atomic_operations: 0,
            access_violations: 0,
        }
    }

    /// Record atomic operation
    pub fn record_atomic_operation(&mut self) {
        self.atomic_operations += 1;
    }

    /// Record access violation
    pub fn record_access_violation(&mut self) {
        self.access_violations += 1;
    }

    /// Get access violation rate
    pub fn access_violation_rate(&self) -> f64 {
        if self.memory_accesses == 0 {
            0.0
        } else {
            self.access_violations as f64 / self.memory_accesses as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_validation() {
        let linear = MemoryType::Linear { min: 1, max: Some(10) };
        assert!(linear.validate().is_ok());
        assert!(!linear.is_shared());

        let shared = MemoryType::Shared { min: 1, max: 10 };
        assert!(shared.validate().is_ok());
        assert!(shared.is_shared());

        let invalid = MemoryType::Linear { min: 10, max: Some(5) };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_memory_type_compatibility() {
        let linear1 = MemoryType::Linear { min: 1, max: Some(10) };
        let linear2 = MemoryType::Linear { min: 2, max: Some(20) };
        let shared = MemoryType::Shared { min: 1, max: 10 };

        assert!(linear1.is_compatible_with(&linear2));
        assert!(!linear1.is_compatible_with(&shared));
        assert!(!shared.is_compatible_with(&linear1));
    }

    #[test]
    fn test_shared_memory_segment() {
        let memory_type = MemoryType::Shared { min: 1, max: 10 };
        let segment = SharedMemorySegment::new(
            memory_type,
            SharedMemoryAccess::ReadWrite,
            0x1000,
            0x1000,
            true,
        )
        .unwrap();

        assert!(segment.contains_address(0x1500));
        assert!(!segment.contains_address(0x500));
        assert!(segment.allows_atomic_at(0x1500));
    }

    #[test]
    fn test_shared_memory_manager() {
        let mut manager = SharedMemoryManager::new();

        let segment1 = SharedMemorySegment::new(
            MemoryType::Shared { min: 1, max: 10 },
            SharedMemoryAccess::ReadWrite,
            0x1000,
            0x1000,
            true,
        )
        .unwrap();

        let segment2 = SharedMemorySegment::new(
            MemoryType::Shared { min: 1, max: 10 },
            SharedMemoryAccess::ReadOnly,
            0x3000,
            0x1000,
            false,
        )
        .unwrap();

        assert!(manager.register_segment(segment1).is_ok());
        assert!(manager.register_segment(segment2).is_ok());

        assert!(manager.allows_atomic_at(0x1500));
        assert!(!manager.allows_atomic_at(0x3500));

        assert!(manager.validate_access(0x1500, SharedMemoryAccess::ReadWrite).is_ok());
        assert!(manager.validate_access(0x3500, SharedMemoryAccess::ReadOnly).is_ok());
        assert!(manager.validate_access(0x3500, SharedMemoryAccess::ReadWrite).is_err());
    }

    #[test]
    fn test_memory_type_serialization() {
        let linear = MemoryType::Linear { min: 1, max: Some(10) };
        let bytes = linear.to_bytes().unwrap();
        let (deserialized, _) = MemoryType::from_bytes(&bytes).unwrap();
        assert_eq!(linear, deserialized);

        let shared = MemoryType::Shared { min: 2, max: 20 };
        let bytes = shared.to_bytes().unwrap();
        let (deserialized, _) = MemoryType::from_bytes(&bytes).unwrap();
        assert_eq!(shared, deserialized);
    }
}
