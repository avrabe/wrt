use crate::error::{Error, Result};
use crate::types::TableType;
use crate::values::Value;
use crate::Vec;
use std::sync::RwLock;

/// Represents a WebAssembly table instance
#[derive(Debug)]
pub struct Table {
    /// Table type
    pub type_: TableType,
    /// Table elements, protected by RwLock
    elements: RwLock<Vec<Option<Value>>>,
}

impl Clone for Table {
    fn clone(&self) -> Self {
        let elements_lock = self.elements.read().unwrap();
        Self {
            type_: self.type_.clone(),
            elements: RwLock::new(elements_lock.clone()),
        }
    }
}

impl Table {
    /// Creates a new table instance
    #[must_use]
    pub fn new(table_type: TableType) -> Self {
        let initial_size = table_type.min;
        Self {
            type_: table_type,
            elements: RwLock::new({
                let mut v = Vec::with_capacity(initial_size as usize);
                v.resize(initial_size as usize, None);
                v
            }),
        }
    }

    /// Returns the table type
    #[must_use]
    pub const fn type_(&self) -> &TableType {
        &self.type_
    }

    /// Returns the current size
    #[must_use]
    pub fn size(&self) -> usize {
        self.elements.read().unwrap().len()
    }

    /// Grows the table by the specified number of elements
    pub fn grow(&self, delta: u32) -> Result<u32> {
        let old_size_usize = self.elements.read().unwrap().len(); // Get current size as usize
        let delta_usize: usize = delta
            .try_into()
            .map_err(|_| Error::Execution("Delta too large for usize".into()))?;
        let new_size_usize = old_size_usize
            .checked_add(delta_usize)
            .ok_or_else(|| Error::Execution("Table size overflow".into()))?;

        let max_usize = self
            .type_
            .max
            .map_or(usize::MAX, |m| m.try_into().unwrap_or(usize::MAX));

        if new_size_usize > max_usize {
            return Err(Error::Execution("Table size exceeds maximum".into()));
        }

        let mut elements_guard = self.elements.write().unwrap();
        elements_guard.resize(new_size_usize, None);
        Ok(old_size_usize.try_into().unwrap_or(u32::MAX)) // Return old size as u32
    }

    /// Gets an element from the table
    pub fn get(&self, idx: u32) -> Result<Option<Value>> {
        let elements_guard = self.elements.read().unwrap();
        let idx_usize = idx as usize;
        if idx_usize >= elements_guard.len() {
            // Wasm spec dictates trap on out-of-bounds access
            return Err(Error::OutOfBounds);
        }
        Ok(elements_guard[idx_usize].clone())
    }

    /// Sets an element in the table
    pub fn set(&self, idx: u32, value: Option<Value>) -> Result<()> {
        let mut elements_guard = self.elements.write().unwrap();
        let idx_usize = idx as usize;
        if idx_usize >= elements_guard.len() {
            return Err(Error::OutOfBounds); // Trap on out-of-bounds write
        }
        // Type check if value is Some
        if let Some(ref val) = value {
            if !val.matches_type(&self.type_.element_type) {
                return Err(Error::TypeMismatch(format!(
                    "Invalid value type {:?} for table type {:?}",
                    val.get_type(),
                    self.type_.element_type
                )));
            }
        }
        elements_guard[idx_usize] = value;
        Ok(())
    }

    /// Initializes a range of elements from a vector
    pub fn init(&self, offset: u32, init: &[Option<Value>]) -> Result<()> {
        let len = init.len() as u32;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| Error::Execution("Table initialization overflow".into()))?;

        let mut elements_guard = self.elements.write().unwrap();
        self.check_bounds_internal(end.saturating_sub(1), &elements_guard)?;

        // Clone each element individually since Option<Value> doesn't implement Copy
        for (i, value) in init.iter().enumerate() {
            elements_guard[offset as usize + i] = value.clone();
        }

        Ok(())
    }

    /// Copies elements from one range to another
    pub fn copy(&self, dst: u32, src: u32, len: u32) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        let dst_end = dst
            .checked_add(len)
            .ok_or_else(|| Error::Execution("Table copy destination overflow".into()))?;
        let src_end = src
            .checked_add(len)
            .ok_or_else(|| Error::Execution("Table copy source overflow".into()))?;

        let mut elements_guard = self.elements.write().unwrap();
        self.check_bounds_internal(dst_end.saturating_sub(1), &elements_guard)?;
        self.check_bounds_internal(src_end.saturating_sub(1), &elements_guard)?;

        // Perform copy within the locked guard
        // Need to handle overlap carefully - copy_within might be simpler if elements were Copy
        if dst <= src {
            // Forward copy
            for i in 0..len {
                elements_guard[(dst + i) as usize] = elements_guard[(src + i) as usize].clone();
            }
        } else {
            // Backward copy
            for i in (0..len).rev() {
                elements_guard[(dst + i) as usize] = elements_guard[(src + i) as usize].clone();
            }
        }
        Ok(())
    }

    /// Fills a range of elements with a value
    pub fn fill(&self, offset: u32, len: u32, value: Option<Value>) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        let offset_usize = offset as usize;
        let len_usize = len as usize;
        let end_usize = offset_usize
            .checked_add(len_usize)
            .ok_or_else(|| Error::Execution("Table fill overflow".into()))?;

        let mut elements_guard = self.elements.write().unwrap();
        // Check bounds *after* getting lock
        if end_usize > elements_guard.len() {
            return Err(Error::OutOfBounds); // Trap if fill goes out of bounds
        }
        // Type check if value is Some
        if let Some(ref val) = value {
            if !val.matches_type(&self.type_.element_type) {
                return Err(Error::TypeMismatch(format!(
                    "Invalid value type {:?} for table type {:?}",
                    val.get_type(),
                    self.type_.element_type
                )));
            }
        }

        for i in offset_usize..end_usize {
            elements_guard[i] = value.clone();
        }
        Ok(())
    }

    /// Internal bounds check using a lock guard
    fn check_bounds_internal<G>(&self, idx: u32, guard: &G) -> Result<()>
    where
        G: std::ops::Deref<Target = Vec<Option<Value>>>,
    {
        if idx >= guard.len() as u32 {
            return Err(Error::Execution("Table access out of bounds".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::values::Value;
    use crate::ValueType;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    fn create_test_table_type(min: u32, max: Option<u32>) -> TableType {
        TableType {
            element_type: ValueType::FuncRef,
            min,
            max,
        }
    }

    #[test]
    fn test_table_creation() {
        let table_type = TableType {
            element_type: ValueType::I32,
            min: 1,
            max: Some(10),
        };
        let table = Table::new(table_type);
        assert_eq!(table.size(), 1); // Initial size should be min elements
    }

    #[test]
    fn test_table_growth() -> Result<()> {
        let table_type = create_test_table_type(1, Some(10));
        let table = Table::new(table_type);

        // Test successful growth
        let old_size = table.grow(2)?;
        assert_eq!(old_size, 1);
        assert_eq!(table.size(), 3);

        // Test growth up to max
        assert!(table.grow(7).is_ok());
        assert_eq!(table.size(), 10);

        // Test growth beyond max
        assert!(table.grow(1).is_err());

        // Test growth with no max
        let table = Table::new(create_test_table_type(1, None));
        assert!(table.grow(1000).is_ok());
        assert_eq!(table.size(), 1001);

        Ok(())
    }

    #[test]
    fn test_table_access() -> Result<()> {
        let table = Table::new(create_test_table_type(5, None));

        // Test initial state
        for i in 0..5 {
            assert_eq!(table.get(i)?, None);
        }

        // Test set and get
        let value = Value::FuncRef(Some(42));
        table.set(3, Some(value.clone()))?;
        assert_eq!(table.get(3)?, Some(value));

        // Test bounds checking
        assert!(table.get(5).is_err());
        assert!(table.set(5, None).is_err());

        Ok(())
    }

    #[test]
    fn test_table_initialization() -> Result<()> {
        let table = Table::new(create_test_table_type(5, None));
        let values = vec![
            Some(Value::FuncRef(Some(1))),
            Some(Value::FuncRef(Some(2))),
            Some(Value::FuncRef(Some(3))),
        ];

        // Test successful initialization
        table.init(1, &values)?;
        assert_eq!(table.get(1)?, Some(Value::FuncRef(Some(1))));
        assert_eq!(table.get(2)?, Some(Value::FuncRef(Some(2))));
        assert_eq!(table.get(3)?, Some(Value::FuncRef(Some(3))));

        // Test initialization out of bounds
        assert!(table.init(4, &values).is_err());

        Ok(())
    }

    #[test]
    fn test_table_copy() -> Result<()> {
        let table = Table::new(create_test_table_type(10, None));

        // Set up some initial values
        table.set(0, Some(Value::FuncRef(Some(1))))?;
        table.set(1, Some(Value::FuncRef(Some(2))))?;
        table.set(2, Some(Value::FuncRef(Some(3))))?;

        // Test forward copy
        table.copy(5, 0, 3)?;
        assert_eq!(table.get(5)?, Some(Value::FuncRef(Some(1))));
        assert_eq!(table.get(6)?, Some(Value::FuncRef(Some(2))));
        assert_eq!(table.get(7)?, Some(Value::FuncRef(Some(3))));

        // Test backward copy (overlapping)
        table.copy(1, 0, 2)?;
        assert_eq!(table.get(1)?, Some(Value::FuncRef(Some(1))));
        assert_eq!(table.get(2)?, Some(Value::FuncRef(Some(2))));

        // Test copy out of bounds
        assert!(table.copy(8, 0, 3).is_err());
        assert!(table.copy(0, 8, 3).is_err());

        Ok(())
    }

    #[test]
    fn test_table_fill() -> Result<()> {
        let table = Table::new(create_test_table_type(5, None));
        let value = Some(Value::FuncRef(Some(42)));

        // Test successful fill
        table.fill(1, 3, value.clone())?;
        assert_eq!(table.get(1)?, value);
        assert_eq!(table.get(2)?, value);
        assert_eq!(table.get(3)?, value);
        assert_eq!(table.get(0)?, None);
        assert_eq!(table.get(4)?, None);

        // Test fill out of bounds
        assert!(table.fill(4, 2, value).is_err());

        Ok(())
    }
}
