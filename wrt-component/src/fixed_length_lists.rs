// WRT - wrt-component
// Module: Fixed-Length List Type System Support
// SW-REQ-ID: REQ_FIXED_LENGTH_LISTS_001
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

#![forbid(unsafe_code)]

//! Fixed-Length List Type System Support
//!
//! This module provides implementation of fixed-length lists for the
//! WebAssembly Component Model type system, enabling compile-time
//! guaranteed list sizes for better performance and safety.


extern crate alloc;

use std::{boxed::Box, vec::Vec};
#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use wrt_error::{Error, ErrorCategory, Result};
use wrt_foundation::{
    bounded::{BoundedVec},
    component_value::ComponentValue,
    types::ValueType,
};

#[cfg(not(any(feature = "std", )))]
use wrt_foundation::{BoundedString};

// Constants for no_std environments
#[cfg(not(any(feature = "std", )))]
const MAX_FIXED_LIST_SIZE: usize = 1024;
#[cfg(not(any(feature = "std", )))]
const MAX_TYPE_DEFINITIONS: usize = 256;

/// Fixed-length list type definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedLengthListType {
    pub element_type: ValueType,
    pub length: u32,
    pub mutable: bool,
}

impl FixedLengthListType {
    pub fn new(element_type: ValueType, length: u32) -> Self {
        Self {
            element_type,
            length,
            mutable: false,
        }
    }

    pub fn new_mutable(element_type: ValueType, length: u32) -> Self {
        Self {
            element_type,
            length,
            mutable: true,
        }
    }

    pub fn element_type(&self) -> &ValueType {
        &self.element_type
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    pub fn size_in_bytes(&self) -> u32 {
        let element_size = match self.element_type {
            ValueType::Bool => 1,
            ValueType::S8 | ValueType::U8 => 1,
            ValueType::S16 | ValueType::U16 => 2,
            ValueType::S32 | ValueType::U32 | ValueType::F32 => 4,
            ValueType::S64 | ValueType::U64 | ValueType::F64 => 8,
            ValueType::Char => 4, // UTF-32
            ValueType::String => 8, // Pointer + length
            _ => 8, // Default for complex types
        };
        element_size * self.length
    }

    pub fn validate_size(&self) -> Result<()> {
        if self.length == 0 {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Fixed-length list cannot have zero length"
            ));
        }
        
        if self.length > MAX_FIXED_LIST_SIZE as u32 {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Fixed-length list size exceeds maximum"
            ));
        }
        
        Ok(())
    }
}

/// Fixed-length list value container
#[derive(Debug, Clone)]
pub struct FixedLengthList {
    pub list_type: FixedLengthListType,
    #[cfg(feature = "std")]
    pub elements: Vec<ComponentValue>,
    #[cfg(not(any(feature = "std", )))]
    pub elements: BoundedVec<ComponentValue, MAX_FIXED_LIST_SIZE, NoStdProvider<65536>>,
}

impl FixedLengthList {
    #[cfg(feature = "std")]
    pub fn new(list_type: FixedLengthListType) -> Result<Self> {
        list_type.validate_size()?;
        let elements = Vec::with_capacity(list_type.length as usize);
        Ok(Self {
            list_type,
            elements,
        })
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn new(list_type: FixedLengthListType) -> Result<Self> {
        list_type.validate_size()?;
        let elements = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();
        Ok(Self {
            list_type,
            elements,
        })
    }

    #[cfg(feature = "std")]
    pub fn with_elements(list_type: FixedLengthListType, elements: Vec<ComponentValue>) -> Result<Self> {
        list_type.validate_size()?;
        
        if elements.len() != list_type.length as usize {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Element count does not match fixed list length"
            ));
        }
        
        // Validate element types
        for (i, element) in elements.iter().enumerate() {
            if !Self::validate_element_type(element, &list_type.element_type) {
                return Err(Error::new(
                    ErrorCategory::Type,
                    wrt_error::codes::TYPE_MISMATCH,
                    &"Component not found"
                ));
            }
        }
        
        Ok(Self {
            list_type,
            elements,
        })
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn with_elements(list_type: FixedLengthListType, elements: &[ComponentValue]) -> Result<Self> {
        list_type.validate_size()?;
        
        if elements.len() != list_type.length as usize {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Element count does not match fixed list length"
            ));
        }
        
        // Validate element types
        for (i, element) in elements.iter().enumerate() {
            if !Self::validate_element_type(element, &list_type.element_type) {
                return Err(Error::new(
                    ErrorCategory::Type,
                    wrt_error::codes::TYPE_MISMATCH,
                    "Element has incorrect type"
                ));
            }
        }
        
        let bounded_elements = BoundedVec::new_from_slice(elements)
            .map_err(|_| Error::new(
                ErrorCategory::Memory,
                wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                "Too many elements for no_std environment"
            ))?;
        
        Ok(Self {
            list_type,
            elements: bounded_elements,
        })
    }

    fn validate_element_type(element: &ComponentValue, expected_type: &ValueType) -> bool {
        match (element, expected_type) {
            (ComponentValue::Bool(_), ValueType::Bool) => true,
            (ComponentValue::S8(_), ValueType::S8) => true,
            (ComponentValue::U8(_), ValueType::U8) => true,
            (ComponentValue::S16(_), ValueType::S16) => true,
            (ComponentValue::U16(_), ValueType::U16) => true,
            (ComponentValue::S32(_), ValueType::S32) => true,
            (ComponentValue::U32(_), ValueType::U32) => true,
            (ComponentValue::S64(_), ValueType::S64) => true,
            (ComponentValue::U64(_), ValueType::U64) => true,
            (ComponentValue::F32(_), ValueType::F32) => true,
            (ComponentValue::F64(_), ValueType::F64) => true,
            (ComponentValue::Char(_), ValueType::Char) => true,
            (ComponentValue::String(_), ValueType::String) => true,
            // For I32/I64 compatibility
            (ComponentValue::I32(_), ValueType::S32) => true,
            (ComponentValue::I64(_), ValueType::S64) => true,
            _ => false,
        }
    }

    pub fn length(&self) -> u32 {
        self.list_type.length
    }

    pub fn element_type(&self) -> &ValueType {
        &self.list_type.element_type
    }

    pub fn is_mutable(&self) -> bool {
        self.list_type.mutable
    }

    pub fn is_full(&self) -> bool {
        self.elements.len() == self.list_type.length as usize
    }

    pub fn get(&self, index: u32) -> Option<&ComponentValue> {
        if index < self.list_type.length {
            self.elements.get(index as usize)
        } else {
            None
        }
    }

    pub fn set(&mut self, index: u32, value: ComponentValue) -> Result<()> {
        if !self.list_type.mutable {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Cannot modify immutable fixed-length list"
            ));
        }

        if index >= self.list_type.length {
            return Err(Error::new(
                ErrorCategory::InvalidInput,
                wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                "Index out of bounds"
            ));
        }

        if !Self::validate_element_type(&value, &self.list_type.element_type) {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Value type does not match list element type"
            ));
        }

        if let Some(element) = self.elements.get_mut(index as usize) {
            *element = value;
        } else {
            // If element doesn't exist yet, add it (for initialization)
            if self.elements.len() == index as usize {
                #[cfg(feature = "std")]
                self.elements.push(value);
                #[cfg(not(any(feature = "std", )))]
                self.elements.push(value)
                    .map_err(|_| Error::new(
                        ErrorCategory::Memory,
                        wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                        "List storage full"
                    ))?;
            } else {
                return Err(Error::new(
                    ErrorCategory::InvalidInput,
                    wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                    "Cannot set non-consecutive index"
                ));
            }
        }

        Ok(())
    }

    pub fn push(&mut self, value: ComponentValue) -> Result<()> {
        if !self.list_type.mutable {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Cannot modify immutable fixed-length list"
            ));
        }

        if self.is_full() {
            return Err(Error::new(
                ErrorCategory::InvalidInput,
                wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                "Fixed-length list is already full"
            ));
        }

        if !Self::validate_element_type(&value, &self.list_type.element_type) {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Value type does not match list element type"
            ));
        }

        #[cfg(feature = "std")]
        self.elements.push(value);
        #[cfg(not(any(feature = "std", )))]
        self.elements.push(value)
            .map_err(|_| Error::new(
                ErrorCategory::Memory,
                wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                "List storage full"
            ))?;

        Ok(())
    }

    pub fn current_length(&self) -> u32 {
        self.elements.len() as u32
    }

    pub fn remaining_capacity(&self) -> u32 {
        self.list_type.length - self.current_length()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ComponentValue> {
        self.elements.iter()
    }

    #[cfg(feature = "std")]
    pub fn to_vec(&self) -> Vec<ComponentValue> {
        self.elements.clone()
    }

    #[cfg(not(any(feature = "std", )))]
    pub fn to_slice(&self) -> &[ComponentValue] {
        self.elements.as_slice()
    }
}

/// Type registry for fixed-length list types
#[derive(Debug)]
pub struct FixedLengthListTypeRegistry {
    #[cfg(feature = "std")]
    types: Vec<FixedLengthListType>,
    #[cfg(not(any(feature = "std", )))]
    types: BoundedVec<FixedLengthListType, MAX_TYPE_DEFINITIONS, NoStdProvider<65536>>,
}

impl FixedLengthListTypeRegistry {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            types: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            types: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        }
    }

    pub fn register_type(&mut self, list_type: FixedLengthListType) -> Result<u32> {
        list_type.validate_size()?;
        
        // Check for duplicate
        for (i, existing_type) in self.types.iter().enumerate() {
            if existing_type == &list_type {
                return Ok(i as u32);
            }
        }
        
        let index = self.types.len() as u32;
        
        #[cfg(feature = "std")]
        self.types.push(list_type);
        #[cfg(not(any(feature = "std", )))]
        self.types.push(list_type)
            .map_err(|_| Error::new(
                ErrorCategory::Memory,
                wrt_error::codes::MEMORY_ALLOCATION_FAILED,
                "Type registry full"
            ))?;
        
        Ok(index)
    }

    pub fn get_type(&self, index: u32) -> Option<&FixedLengthListType> {
        self.types.get(index as usize)
    }

    pub fn type_count(&self) -> u32 {
        self.types.len() as u32
    }

    pub fn find_type(&self, element_type: &ValueType, length: u32) -> Option<u32> {
        for (i, list_type) in self.types.iter().enumerate() {
            if list_type.element_type == *element_type && list_type.length == length {
                return Some(i as u32);
            }
        }
        None
    }
}

impl Default for FixedLengthListTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Component Model integration for fixed-length lists
pub mod component_integration {
    use super::*;

    /// Convert a fixed-length list to a ComponentValue
    impl From<FixedLengthList> for ComponentValue {
        fn from(list: FixedLengthList) -> Self {
            #[cfg(feature = "std")]
            {
                ComponentValue::List(list.elements)
            }
            #[cfg(not(any(feature = "std", )))]
            {
                // Convert to regular list representation
                let vec_data: Vec<ComponentValue> = list.elements.iter().cloned().collect();
                ComponentValue::List(vec_data)
            }
        }
    }

    /// Try to convert a ComponentValue to a fixed-length list
    impl FixedLengthList {
        pub fn try_from_component_value(
            value: ComponentValue,
            expected_type: FixedLengthListType
        ) -> Result<Self> {
            match value {
                ComponentValue::List(elements) => {
                    #[cfg(feature = "std")]
                    {
                        Self::with_elements(expected_type, elements)
                    }
                    #[cfg(not(any(feature = "std", )))]
                    {
                        Self::with_elements(expected_type, &elements)
                    }
                }
                _ => Err(Error::new(
                    ErrorCategory::Type,
                    wrt_error::codes::TYPE_MISMATCH,
                    "ComponentValue is not a list"
                ))
            }
        }
    }

    /// Extended ValueType to include fixed-length lists
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ExtendedValueType {
        /// Standard value types
        Standard(ValueType),
        /// Fixed-length list type with type index
        FixedLengthList(u32),
    }

    impl ExtendedValueType {
        pub fn is_fixed_length_list(&self) -> bool {
            matches!(self, Self::FixedLengthList(_))
        }

        pub fn as_fixed_length_list_index(&self) -> Option<u32> {
            match self {
                Self::FixedLengthList(index) => Some(*index),
                _ => None,
            }
        }

        pub fn as_standard_type(&self) -> Option<&ValueType> {
            match self {
                Self::Standard(vt) => Some(vt),
                _ => None,
            }
        }
    }

    impl From<ValueType> for ExtendedValueType {
        fn from(vt: ValueType) -> Self {
            Self::Standard(vt)
        }
    }
}

/// Utility functions for fixed-length lists
pub mod fixed_list_utils {
    use super::*;

    /// Create a fixed-length list of the same element repeated
    pub fn repeat_element(
        element_type: ValueType,
        element: ComponentValue,
        count: u32
    ) -> Result<FixedLengthList> {
        let list_type = FixedLengthListType::new(element_type, count);
        let mut list = FixedLengthList::new(list_type)?;
        
        for _ in 0..count {
            list.push(element.clone())?;
        }
        
        Ok(list)
    }

    /// Create a fixed-length list of zeros/default values
    pub fn zero_filled(element_type: ValueType, count: u32) -> Result<FixedLengthList> {
        let default_value = match element_type {
            ValueType::Bool => ComponentValue::Bool(false),
            ValueType::S8 => ComponentValue::S8(0),
            ValueType::U8 => ComponentValue::U8(0),
            ValueType::S16 => ComponentValue::S16(0),
            ValueType::U16 => ComponentValue::U16(0),
            ValueType::S32 => ComponentValue::S32(0),
            ValueType::U32 => ComponentValue::U32(0),
            ValueType::S64 => ComponentValue::S64(0),
            ValueType::U64 => ComponentValue::U64(0),
            ValueType::F32 => ComponentValue::F32(0.0),
            ValueType::F64 => ComponentValue::F64(0.0),
            ValueType::Char => ComponentValue::Char('\0'),
            ValueType::String => ComponentValue::String("".to_string()),
            ValueType::I32 => ComponentValue::I32(0),
            ValueType::I64 => ComponentValue::I64(0),
            _ => return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Cannot create default value for this type"
            )),
        };
        
        repeat_element(element_type, default_value, count)
    }

    /// Create a fixed-length list from a range
    pub fn from_range(start: i32, end: i32) -> Result<FixedLengthList> {
        if start >= end {
            return Err(Error::new(
                ErrorCategory::InvalidInput,
                wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                "Start must be less than end"
            ));
        }
        
        let count = (end - start) as u32;
        let list_type = FixedLengthListType::new(ValueType::I32, count);
        let mut list = FixedLengthList::new(list_type)?;
        
        for i in start..end {
            list.push(ComponentValue::I32(i))?;
        }
        
        Ok(list)
    }

    /// Concatenate two fixed-length lists of the same type
    pub fn concatenate(
        list1: &FixedLengthList,
        list2: &FixedLengthList
    ) -> Result<FixedLengthList> {
        if list1.element_type() != list2.element_type() {
            return Err(Error::new(
                ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                "Cannot concatenate lists with different element types"
            ));
        }
        
        let new_length = list1.length() + list2.length();
        let new_type = FixedLengthListType::new(list1.element_type().clone(), new_length);
        let mut result = FixedLengthList::new(new_type)?;
        
        // Add elements from first list
        for element in list1.iter() {
            result.push(element.clone())?;
        }
        
        // Add elements from second list
        for element in list2.iter() {
            result.push(element.clone())?;
        }
        
        Ok(result)
    }

    /// Slice a fixed-length list
    pub fn slice(
        list: &FixedLengthList,
        start: u32,
        length: u32
    ) -> Result<FixedLengthList> {
        if start + length > list.length() {
            return Err(Error::new(
                ErrorCategory::InvalidInput,
                wrt_error::codes::OUT_OF_BOUNDS_ERROR,
                "Slice range exceeds list bounds"
            ));
        }
        
        let slice_type = FixedLengthListType::new(list.element_type().clone(), length);
        let mut result = FixedLengthList::new(slice_type)?;
        
        for i in start..start + length {
            if let Some(element) = list.get(i) {
                result.push(element.clone())?;
            }
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::fixed_list_utils::*;
    use super::component_integration::*;

    #[test]
    fn test_fixed_length_list_type_creation() {
        let list_type = FixedLengthListType::new(ValueType::I32, 10);
        assert_eq!(list_type.element_type(), &ValueType::I32);
        assert_eq!(list_type.length(), 10);
        assert!(!list_type.is_mutable());
        assert_eq!(list_type.size_in_bytes(), 40); // 10 * 4 bytes

        let mutable_type = FixedLengthListType::new_mutable(ValueType::F64, 5);
        assert!(mutable_type.is_mutable());
        assert_eq!(mutable_type.size_in_bytes(), 40); // 5 * 8 bytes
    }

    #[test]
    fn test_fixed_length_list_validation() {
        let valid_type = FixedLengthListType::new(ValueType::I32, 10);
        assert!(valid_type.validate_size().is_ok());

        let zero_length_type = FixedLengthListType::new(ValueType::I32, 0);
        assert!(zero_length_type.validate_size().is_err());

        let too_large_type = FixedLengthListType::new(ValueType::I32, MAX_FIXED_LIST_SIZE as u32 + 1);
        assert!(too_large_type.validate_size().is_err());
    }

    #[test]
    fn test_fixed_length_list_creation() {
        let list_type = FixedLengthListType::new(ValueType::I32, 3);
        let list = FixedLengthList::new(list_type).unwrap();
        
        assert_eq!(list.length(), 3);
        assert_eq!(list.current_length(), 0);
        assert_eq!(list.remaining_capacity(), 3);
        assert!(!list.is_full());
    }

    #[test]
    fn test_fixed_length_list_with_elements() {
        let list_type = FixedLengthListType::new(ValueType::I32, 3);
        let elements = vec![
            ComponentValue::I32(1),
            ComponentValue::I32(2),
            ComponentValue::I32(3),
        ];

        #[cfg(feature = "std")]
        let list = FixedLengthList::with_elements(list_type, elements).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let list = FixedLengthList::with_elements(list_type, &elements).unwrap();

        assert_eq!(list.current_length(), 3);
        assert!(list.is_full());
        assert_eq!(list.get(0), Some(&ComponentValue::I32(1)));
        assert_eq!(list.get(1), Some(&ComponentValue::I32(2)));
        assert_eq!(list.get(2), Some(&ComponentValue::I32(3)));
        assert_eq!(list.get(3), None);
    }

    #[test]
    fn test_fixed_length_list_type_validation() {
        let list_type = FixedLengthListType::new(ValueType::I32, 2);
        
        // Wrong number of elements
        let wrong_count = vec![ComponentValue::I32(1)];
        #[cfg(feature = "std")]
        let result = FixedLengthList::with_elements(list_type.clone(), wrong_count);
        #[cfg(not(any(feature = "std", )))]
        let result = FixedLengthList::with_elements(list_type.clone(), &wrong_count);
        assert!(result.is_err());

        // Wrong element type
        let wrong_type = vec![
            ComponentValue::I32(1),
            ComponentValue::Bool(true), // Wrong type
        ];
        #[cfg(feature = "std")]
        let result = FixedLengthList::with_elements(list_type, wrong_type);
        #[cfg(not(any(feature = "std", )))]
        let result = FixedLengthList::with_elements(list_type, &wrong_type);
        assert!(result.is_err());
    }

    #[test]
    fn test_fixed_length_list_mutable_operations() {
        let list_type = FixedLengthListType::new_mutable(ValueType::I32, 3);
        let mut list = FixedLengthList::new(list_type).unwrap();

        // Test push
        assert!(list.push(ComponentValue::I32(1)).is_ok());
        assert!(list.push(ComponentValue::I32(2)).is_ok());
        assert!(list.push(ComponentValue::I32(3)).is_ok());
        assert!(list.is_full());

        // Try to push when full
        assert!(list.push(ComponentValue::I32(4)).is_err());

        // Test set
        assert!(list.set(1, ComponentValue::I32(42)).is_ok());
        assert_eq!(list.get(1), Some(&ComponentValue::I32(42)));

        // Test invalid set
        assert!(list.set(5, ComponentValue::I32(999)).is_err()); // Out of bounds
        assert!(list.set(0, ComponentValue::Bool(true)).is_err()); // Wrong type
    }

    #[test]
    fn test_immutable_list_restrictions() {
        let list_type = FixedLengthListType::new(ValueType::I32, 3); // Immutable
        let mut list = FixedLengthList::new(list_type).unwrap();

        // Should not be able to modify immutable list
        assert!(list.set(0, ComponentValue::I32(1)).is_err());
        
        // Push should also fail for immutable lists
        assert!(list.push(ComponentValue::I32(1)).is_err());
    }

    #[test]
    fn test_fixed_length_list_type_registry() {
        let mut registry = FixedLengthListTypeRegistry::new();
        assert_eq!(registry.type_count(), 0);

        let list_type1 = FixedLengthListType::new(ValueType::I32, 10);
        let index1 = registry.register_type(list_type1.clone()).unwrap();
        assert_eq!(index1, 0);
        assert_eq!(registry.type_count(), 1);

        let list_type2 = FixedLengthListType::new(ValueType::F64, 5);
        let index2 = registry.register_type(list_type2).unwrap();
        assert_eq!(index2, 1);
        assert_eq!(registry.type_count(), 2);

        // Register duplicate should return existing index
        let duplicate_index = registry.register_type(list_type1).unwrap();
        assert_eq!(duplicate_index, 0);
        assert_eq!(registry.type_count(), 2); // No new type added

        // Test retrieval
        let retrieved = registry.get_type(0).unwrap();
        assert_eq!(retrieved.element_type(), &ValueType::I32);
        assert_eq!(retrieved.length(), 10);

        // Test find
        let found_index = registry.find_type(&ValueType::I32, 10);
        assert_eq!(found_index, Some(0));

        let not_found = registry.find_type(&ValueType::Bool, 10);
        assert_eq!(not_found, None);
    }

    #[test]
    fn test_component_value_conversion() {
        let list_type = FixedLengthListType::new(ValueType::I32, 3);
        let elements = vec![
            ComponentValue::I32(1),
            ComponentValue::I32(2),
            ComponentValue::I32(3),
        ];

        #[cfg(feature = "std")]
        let list = FixedLengthList::with_elements(list_type.clone(), elements.clone()).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let list = FixedLengthList::with_elements(list_type.clone(), &elements).unwrap();

        // Convert to ComponentValue
        let component_value: ComponentValue = list.clone().into();
        match component_value {
            ComponentValue::List(ref list_elements) => {
                assert_eq!(list_elements.len(), 3);
                assert_eq!(list_elements[0], ComponentValue::I32(1));
            }
            _ => panic!("Expected List variant"),
        }

        // Convert back from ComponentValue
        let converted_back = FixedLengthList::try_from_component_value(component_value, list_type).unwrap();
        assert_eq!(converted_back.current_length(), 3);
        assert_eq!(converted_back.get(0), Some(&ComponentValue::I32(1)));
    }

    #[test]
    fn test_utility_functions() {
        // Test repeat_element
        let repeated = repeat_element(ValueType::Bool, ComponentValue::Bool(true), 5).unwrap();
        assert_eq!(repeated.current_length(), 5);
        assert_eq!(repeated.get(0), Some(&ComponentValue::Bool(true)));
        assert_eq!(repeated.get(4), Some(&ComponentValue::Bool(true)));

        // Test zero_filled
        let zeros = zero_filled(ValueType::I32, 3).unwrap();
        assert_eq!(zeros.current_length(), 3);
        assert_eq!(zeros.get(0), Some(&ComponentValue::I32(0)));

        // Test from_range
        let range_list = from_range(5, 8).unwrap();
        assert_eq!(range_list.current_length(), 3);
        assert_eq!(range_list.get(0), Some(&ComponentValue::I32(5)));
        assert_eq!(range_list.get(1), Some(&ComponentValue::I32(6)));
        assert_eq!(range_list.get(2), Some(&ComponentValue::I32(7)));
    }

    #[test]
    fn test_list_operations() {
        let list1_type = FixedLengthListType::new(ValueType::I32, 2);
        let list1_elements = vec![ComponentValue::I32(1), ComponentValue::I32(2)];
        #[cfg(feature = "std")]
        let list1 = FixedLengthList::with_elements(list1_type, list1_elements).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let list1 = FixedLengthList::with_elements(list1_type, &list1_elements).unwrap();

        let list2_type = FixedLengthListType::new(ValueType::I32, 2);
        let list2_elements = vec![ComponentValue::I32(3), ComponentValue::I32(4)];
        #[cfg(feature = "std")]
        let list2 = FixedLengthList::with_elements(list2_type, list2_elements).unwrap();
        #[cfg(not(any(feature = "std", )))]
        let list2 = FixedLengthList::with_elements(list2_type, &list2_elements).unwrap();

        // Test concatenation
        let concatenated = concatenate(&list1, &list2).unwrap();
        assert_eq!(concatenated.current_length(), 4);
        assert_eq!(concatenated.get(0), Some(&ComponentValue::I32(1)));
        assert_eq!(concatenated.get(1), Some(&ComponentValue::I32(2)));
        assert_eq!(concatenated.get(2), Some(&ComponentValue::I32(3)));
        assert_eq!(concatenated.get(3), Some(&ComponentValue::I32(4)));

        // Test slicing
        let sliced = slice(&concatenated, 1, 2).unwrap();
        assert_eq!(sliced.current_length(), 2);
        assert_eq!(sliced.get(0), Some(&ComponentValue::I32(2)));
        assert_eq!(sliced.get(1), Some(&ComponentValue::I32(3)));
    }

    #[test]
    fn test_extended_value_type() {
        let standard = ExtendedValueType::Standard(ValueType::I32);
        assert!(!standard.is_fixed_length_list());
        assert_eq!(standard.as_standard_type(), Some(&ValueType::I32));
        assert_eq!(standard.as_fixed_length_list_index(), None);

        let fixed_list = ExtendedValueType::FixedLengthList(42);
        assert!(fixed_list.is_fixed_length_list());
        assert_eq!(fixed_list.as_fixed_length_list_index(), Some(42));
        assert_eq!(fixed_list.as_standard_type(), None);

        let from_standard: ExtendedValueType = ValueType::F64.into();
        assert!(!from_standard.is_fixed_length_list());
    }
}