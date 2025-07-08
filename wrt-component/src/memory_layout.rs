//! Memory Layout Management for WebAssembly Component Model
//!
//! This module provides memory layout calculations and alignment handling
//! for the canonical ABI, ensuring proper data representation across
//! component boundaries.

use wrt_format::component::FormatValType;
use crate::bounded_component_infra::ComponentProvider;
use wrt_foundation::{
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

use crate::prelude::*;

/// Maximum alignment requirement for any type
const MAX_ALIGNMENT: usize = 8;

/// Memory layout information for a type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLayout {
    /// Size of the type in bytes
    pub size: usize,
    /// Alignment requirement in bytes
    pub alignment: usize,
}

impl MemoryLayout {
    /// Create a new memory layout
    pub const fn new(size: usize, alignment: usize) -> Self {
        Self { size, alignment }
    }

    /// Calculate the aligned size (size rounded up to alignment)
    pub const fn aligned_size(&self) -> usize {
        align_to(self.size, self.alignment)
    }

    /// Check if this layout fits within another layout
    pub const fn fits_in(&self, other: &MemoryLayout) -> bool {
        self.size <= other.size && self.alignment <= other.alignment
    }
}

/// Calculate memory layout for a WebAssembly component model type
pub fn calculate_layout(ty: &FormatValType<ComponentProvider>) -> MemoryLayout {
    match ty {
        // Primitive types
        FormatValType::Bool => MemoryLayout::new(1, 1),
        FormatValType::S8 | ValType::U8 => MemoryLayout::new(1, 1),
        FormatValType::S16 | ValType::U16 => MemoryLayout::new(2, 2),
        FormatValType::S32 | ValType::U32 => MemoryLayout::new(4, 4),
        FormatValType::S64 | ValType::U64 => MemoryLayout::new(8, 8),
        FormatValType::F32 => MemoryLayout::new(4, 4),
        FormatValType::F64 => MemoryLayout::new(8, 8),
        FormatValType::Char => MemoryLayout::new(4, 4), // Unicode scalar value

        // String is represented as pointer + length
        FormatValType::String => MemoryLayout::new(8, 4), // 4-byte pointer + 4-byte length

        // Lists are represented as pointer + length
        FormatValType::List(_) => MemoryLayout::new(8, 4), // 4-byte pointer + 4-byte length

        // Records require calculating layout for all fields
        FormatValType::Record(fields) => calculate_record_layout(fields),

        // Tuples are similar to records
        FormatValType::Tuple(types) => calculate_tuple_layout(types),

        // Variants need space for discriminant + largest payload
        FormatValType::Variant(cases) => calculate_variant_layout(cases),

        // Enums need space for discriminant only
        FormatValType::Enum(cases) => calculate_enum_layout(cases.len()),

        // Options are variants with two cases (none/some)
        FormatValType::Option(inner) => calculate_option_layout(inner),

        // Results are variants with two cases (ok/err)
        FormatValType::Result(ok_ty, err_ty) => {
            calculate_result_layout(ok_ty.as_deref(), err_ty.as_deref()
        }

        // Flags need bit storage
        FormatValType::Flags(names) => calculate_flags_layout(names.len()),

        // Resources are handles (u32)
        FormatValType::Own(_) | ValType::Borrow(_) => MemoryLayout::new(4, 4),

        // Other types
        _ => MemoryLayout::new(0, 1), // Unknown types have zero size
    }
}

/// Calculate layout for a record type
fn calculate_record_layout(fields: &[(String, FormatValType<ComponentProvider>)]) -> MemoryLayout {
    let mut offset = 0;
    let mut max_alignment = 1;

    for (_, field_type) in fields {
        let field_layout = calculate_layout(field_type);

        // Align offset to field's alignment requirement
        offset = align_to(offset, field_layout.alignment);
        offset += field_layout.size;

        // Track maximum alignment requirement
        max_alignment = max_alignment.max(field_layout.alignment);
    }

    // Final size must be aligned to the record's alignment
    let final_size = align_to(offset, max_alignment);

    MemoryLayout::new(final_size, max_alignment)
}

/// Calculate layout for a tuple type
fn calculate_tuple_layout(types: &[FormatValType<ComponentProvider>]) -> MemoryLayout {
    let mut offset = 0;
    let mut max_alignment = 1;

    for ty in types {
        let layout = calculate_layout(ty);

        // Align offset to element's alignment requirement
        offset = align_to(offset, layout.alignment);
        offset += layout.size;

        // Track maximum alignment requirement
        max_alignment = max_alignment.max(layout.alignment);
    }

    // Final size must be aligned to the tuple's alignment
    let final_size = align_to(offset, max_alignment);

    MemoryLayout::new(final_size, max_alignment)
}

/// Calculate layout for a variant type
fn calculate_variant_layout(cases: &[(String, Option<FormatValType<ComponentProvider>>)]) -> MemoryLayout {
    // Discriminant size based on number of cases
    let discriminant_size = discriminant_size(cases.len();
    let discriminant_alignment = discriminant_size;

    // Find the largest payload
    let mut max_payload_size = 0;
    let mut max_payload_alignment = 1;

    for (_, payload_type) in cases {
        if let Some(ty) = payload_type {
            let payload_layout = calculate_layout(ty);
            max_payload_size = max_payload_size.max(payload_layout.size);
            max_payload_alignment = max_payload_alignment.max(payload_layout.alignment);
        }
    }

    // Variant alignment is max of discriminant and payload alignments
    let variant_alignment = discriminant_alignment.max(max_payload_alignment);

    // Calculate total size
    let payload_offset = align_to(discriminant_size, max_payload_alignment);
    let total_size = payload_offset + max_payload_size;
    let final_size = align_to(total_size, variant_alignment);

    MemoryLayout::new(final_size, variant_alignment)
}

/// Calculate layout for an enum type
fn calculate_enum_layout(num_cases: usize) -> MemoryLayout {
    let size = discriminant_size(num_cases);
    MemoryLayout::new(size, size)
}

/// Calculate layout for an option type
fn calculate_option_layout(inner: &FormatValType<ComponentProvider>) -> MemoryLayout {
    // Option is a variant with none (no payload) and some (with payload)
    let inner_layout = calculate_layout(inner);

    // 1-byte discriminant + payload
    let payload_offset = align_to(1, inner_layout.alignment);
    let total_size = payload_offset + inner_layout.size;
    let alignment = inner_layout.alignment.max(1);
    let final_size = align_to(total_size, alignment);

    MemoryLayout::new(final_size, alignment)
}

/// Calculate layout for a result type
fn calculate_result_layout(ok_ty: Option<&FormatValType<ComponentProvider>>, err_ty: Option<&FormatValType<ComponentProvider>>) -> MemoryLayout {
    // Result is a variant with ok and err cases
    let mut max_payload_size = 0;
    let mut max_payload_alignment = 1;

    if let Some(ty) = ok_ty {
        let layout = calculate_layout(ty);
        max_payload_size = max_payload_size.max(layout.size);
        max_payload_alignment = max_payload_alignment.max(layout.alignment);
    }

    if let Some(ty) = err_ty {
        let layout = calculate_layout(ty);
        max_payload_size = max_payload_size.max(layout.size);
        max_payload_alignment = max_payload_alignment.max(layout.alignment);
    }

    // 1-byte discriminant + payload
    let payload_offset = align_to(1, max_payload_alignment);
    let total_size = payload_offset + max_payload_size;
    let alignment = max_payload_alignment.max(1);
    let final_size = align_to(total_size, alignment);

    MemoryLayout::new(final_size, alignment)
}

/// Calculate layout for flags type
fn calculate_flags_layout(num_flags: usize) -> MemoryLayout {
    // Flags are stored as bit fields
    let num_bytes = (num_flags + 7) / 8;

    // Align to natural size up to 8 bytes
    let alignment = if num_bytes <= 1 {
        1
    } else if num_bytes <= 2 {
        2
    } else if num_bytes <= 4 {
        4
    } else {
        8
    };

    let size = align_to(num_bytes, alignment);
    MemoryLayout::new(size, alignment)
}

/// Determine discriminant size based on number of cases
fn discriminant_size(num_cases: usize) -> usize {
    if num_cases <= 256 {
        1
    } else if num_cases <= 65536 {
        2
    } else {
        4
    }
}

/// Align a value to the specified alignment
const fn align_to(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

/// Calculate field offsets for a record or struct
pub fn calculate_field_offsets(fields: &[(String, FormatValType<ComponentProvider>)]) -> Vec<(String, usize, MemoryLayout)> {
    let mut result = Vec::new();
    let mut offset = 0;

    for (name, field_type) in fields {
        let layout = calculate_layout(field_type);

        // Align offset to field's alignment requirement
        offset = align_to(offset, layout.alignment);

        result.push((name.clone(), offset, layout);
        offset += layout.size;
    }

    result
}

/// Memory layout optimizer for reducing padding
pub struct LayoutOptimizer;

impl LayoutOptimizer {
    /// Reorder fields to minimize padding (largest alignment first)
    pub fn optimize_field_order(fields: &[(String, FormatValType<ComponentProvider>)]) -> Vec<(String, FormatValType<ComponentProvider>)> {
        let mut fields_with_layout: Vec<_> = fields
            .iter()
            .map(|(name, ty)| {
                let layout = calculate_layout(ty);
                (name.clone(), ty.clone(), layout)
            })
            .collect();

        // Sort by alignment (descending) then by size (descending)
        fields_with_layout.sort_by(|a, b| {
            b.2.alignment.cmp(&a.2.alignment).then_with(|| b.2.size.cmp(&a.2.size)
        });

        fields_with_layout.into_iter().map(|(name, ty, _)| (name, ty)).collect()
    }

    /// Calculate padding between two adjacent fields
    pub fn calculate_padding(current_offset: usize, next_alignment: usize) -> usize {
        let aligned_offset = align_to(current_offset, next_alignment);
        aligned_offset - current_offset
    }
}

/// Binary std/no_std choice
#[derive(Debug)]
pub struct CanonicalMemoryPool {
    /// Binary std/no_std choice
    #[cfg(not(any(feature = "std", )))]
    pools: [BoundedVec<MemoryBuffer, 16, NoStdProvider<65536>>; 4],
    #[cfg(feature = "std")]
    pools: [Vec<MemoryBuffer>; 4],
    /// Size classes: 64B, 256B, 1KB, 4KB
    size_classes: [usize; 4],
}

#[derive(Debug)]
struct MemoryBuffer {
    data: Box<[u8]>,
    in_use: bool,
}

impl CanonicalMemoryPool {
    /// Create a new memory pool
    pub fn new() -> Result<Self, crate::ComponentError> {
        Ok(Self {
            #[cfg(not(any(feature = "std", )))]
            pools: [
                {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                },
                {
                    let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                    BoundedVec::new(provider)?
                }
            ],
            #[cfg(feature = "std")]
            pools: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            size_classes: [64, 256, 1024, 4096],
        })
    }

    /// Acquire a buffer of at least the specified size
    pub fn acquire(&mut self, size: usize) -> Option<&mut [u8]> {
        // Find appropriate size class
        let class_idx = self.size_classes.iter().position(|&class_size| class_size >= size)?;

        // Look for available buffer in pool
        #[cfg(not(any(feature = "std", )))]
        {
            for i in 0..self.pools[class_idx].len() {
                if !self.pools[class_idx][i].in_use {
                    self.pools[class_idx][i].in_use = true;
                    return Some(&mut self.pools[class_idx][i].data);
                }
            }
            None // Pool is full in no_std
        }

        #[cfg(feature = "std")]
        {
            // Find existing free buffer
            if let Some(buffer) = self.pools[class_idx].iter_mut().find(|b| !b.in_use) {
                buffer.in_use = true;
                return Some(&mut buffer.data);
            }

            // Allocate new buffer
            let buffer_size = self.size_classes[class_idx];
            let data = vec![0u8; buffer_size].into_boxed_slice();
            self.pools[class_idx].push(MemoryBuffer { data, in_use: true });

            self.pools[class_idx].last_mut().map(|b| &mut b.data[..])
        }
    }

    /// Release a buffer back to the pool
    pub fn release(&mut self, ptr: *mut u8) {
        for pool in &mut self.pools {
            for buffer in pool.iter_mut() {
                if buffer.data.as_ptr() as *mut u8 == ptr {
                    buffer.in_use = false;
                    return;
                }
            }
        }
    }
}

impl Default for CanonicalMemoryPool {
    fn default() -> Self {
        Self::new().expect("Failed to create CanonicalMemoryPoolMissing message")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_layouts() {
        assert_eq!(calculate_layout(&ValType::Bool), MemoryLayout::new(1, 1);
        assert_eq!(calculate_layout(&ValType::U8), MemoryLayout::new(1, 1);
        assert_eq!(calculate_layout(&ValType::U16), MemoryLayout::new(2, 2);
        assert_eq!(calculate_layout(&ValType::U32), MemoryLayout::new(4, 4);
        assert_eq!(calculate_layout(&ValType::U64), MemoryLayout::new(8, 8);
        assert_eq!(calculate_layout(&ValType::F32), MemoryLayout::new(4, 4);
        assert_eq!(calculate_layout(&ValType::F64), MemoryLayout::new(8, 8);
    }

    #[test]
    fn test_record_layout() {
        let fields = vec![
            ("a".to_string(), ValType::U8),
            ("b".to_string(), ValType::U32),
            ("c".to_string(), ValType::U16),
        ];

        let layout = calculate_record_layout(&fields);
        // u8 at 0, padding to 4, u32 at 4, u16 at 8, total 10 aligned to 4 = 12
        assert_eq!(layout.size, 12);
        assert_eq!(layout.alignment, 4);
    }

    #[test]
    fn test_alignment() {
        assert_eq!(align_to(0, 4), 0);
        assert_eq!(align_to(1, 4), 4);
        assert_eq!(align_to(3, 4), 4);
        assert_eq!(align_to(4, 4), 4);
        assert_eq!(align_to(5, 4), 8);
    }

    #[test]
    fn test_discriminant_size() {
        assert_eq!(discriminant_size(2), 1);
        assert_eq!(discriminant_size(256), 1);
        assert_eq!(discriminant_size(257), 2);
        assert_eq!(discriminant_size(65536), 2);
        assert_eq!(discriminant_size(65537), 4);
    }

    #[test]
    fn test_layout_optimizer() {
        let fields = vec![
            ("a".to_string(), ValType::U8),
            ("b".to_string(), ValType::U64),
            ("c".to_string(), ValType::U16),
            ("d".to_string(), ValType::U32),
        ];

        let optimized = LayoutOptimizer::optimize_field_order(&fields);

        // Should be reordered as: u64, u32, u16, u8 (by alignment)
        assert_eq!(optimized[0].0, "bMissing message");
        assert_eq!(optimized[1].0, "dMissing message");
        assert_eq!(optimized[2].0, "cMissing message");
        assert_eq!(optimized[3].0, "aMissing message");
    }
}
