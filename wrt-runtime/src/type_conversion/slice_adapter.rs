//! Slice adapter for compatibility with APIs expecting slices
//!
//! This module provides utilities to adapt between slice-based APIs and
//! the bounded collection types used in ASIL-compliant code.

use wrt_foundation::{
    bounded::BoundedVec,
    bounded_slice::{BoundedSlice, BoundedVecSliceExt},
    values::Value,
    MemoryProvider,
    traits::BoundedCapacity,
};
use wrt_error::Result;

/// Adapter function to handle slice inputs in ASIL-compliant way
/// 
/// This function takes a slice of values and converts it to a bounded
/// representation suitable for use in safety-critical contexts.
pub fn adapt_slice_to_bounded<P>(
    slice: &[Value],
    provider: P,
) -> Result<BoundedVec<Value, 256, P>>
where
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    let mut bounded = BoundedVec::new(provider)?;
    
    for value in slice {
        bounded.push(value.clone())?;
    }
    
    Ok(bounded)
}

/// Adapter to provide slice-like access to BoundedVec for APIs that need it
pub struct SliceAdapter;

impl SliceAdapter {
    /// Convert a slice to a bounded iterator for index-based processing
    pub fn iter_slice(slice: &[Value]) -> SliceIterator<'_> {
        SliceIterator {
            slice,
            index: 0,
        }
    }
}

/// Iterator adapter for processing slices in ASIL-compliant way
pub struct SliceIterator<'a> {
    slice: &'a [Value],
    index: usize,
}

impl<'a> Iterator for SliceIterator<'a> {
    type Item = &'a Value;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.slice.len() {
            let item = &self.slice[self.index];
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.slice.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for SliceIterator<'a> {}

/// Feature-gated direct slice access for QM mode
#[cfg(feature = "dynamic-allocation")]
pub fn get_slice_qm<T, const N: usize, P>(
    bounded: &BoundedVec<T, N, P>
) -> &[T]
where
    T: Sized + wrt_foundation::traits::Checksummable + wrt_foundation::traits::ToBytes + 
       wrt_foundation::traits::FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
{
    // In QM mode, we can provide direct slice access
    // This would require BoundedVec to expose internal storage
    unimplemented!("Requires BoundedVec internal storage access")
}

/// Index-based slice processing for ASIL modes
#[cfg(not(feature = "dynamic-allocation"))]
pub fn process_slice_asil<T, const N: usize, P, F>(
    bounded: &BoundedVec<T, N, P>,
    mut f: F,
) -> Result<()>
where
    T: Sized + wrt_foundation::traits::Checksummable + wrt_foundation::traits::ToBytes + 
       wrt_foundation::traits::FromBytes + Default + Clone + PartialEq + Eq,
    P: MemoryProvider + Clone + PartialEq + Eq,
    F: FnMut(usize, T) -> Result<()>,
{
    for i in 0..bounded.len() {
        let item = bounded.get(i)
            .map_err(|_| wrt_error::Error::runtime_out_of_bounds("Index out of bounds"))?;
        f(i, item)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_foundation::{
        safe_managed_alloc,
        budget_aware_provider::CrateId,
        values::Value,
    };
    
    #[test]
    fn test_slice_to_bounded_conversion() {
        let values = vec![
            Value::I32(1),
            Value::I32(2),
            Value::I32(3),
        ];
        
        let provider = safe_managed_alloc!(1024, CrateId::Runtime).unwrap();
        let bounded = adapt_slice_to_bounded(&values, provider).unwrap();
        
        assert_eq!(bounded.len(), 3;
        assert_eq!(bounded.get(0).unwrap(), &Value::I32(1;
        assert_eq!(bounded.get(1).unwrap(), &Value::I32(2;
        assert_eq!(bounded.get(2).unwrap(), &Value::I32(3;
    }
    
    #[test]
    fn test_slice_iterator() {
        let values = vec![
            Value::I32(10),
            Value::I32(20),
            Value::I32(30),
        ];
        
        let mut iter = SliceAdapter::iter_slice(&values;
        assert_eq!(iter.next(), Some(&Value::I32(10);
        assert_eq!(iter.next(), Some(&Value::I32(20);
        assert_eq!(iter.next(), Some(&Value::I32(30);
        assert_eq!(iter.next(), None;
    }
}