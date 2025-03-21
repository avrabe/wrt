use crate::error::{Error, Result};
use crate::types::*;
use crate::values::Value;
use crate::Vec;

/// Represents a WebAssembly table instance
#[derive(Debug, Clone)]
pub struct Table {
    /// Table type
    pub type_: TableType,
    /// Table elements
    elements: Vec<Option<Value>>,
}

impl Table {
    /// Creates a new table instance
    pub fn new(table_type: TableType) -> Self {
        let initial_size = table_type.min;
        Self {
            type_: table_type,
            elements: {
                let mut v = Vec::with_capacity(initial_size as usize);
                v.resize(initial_size as usize, None);
                v
            },
        }
    }

    /// Returns the table type
    pub fn type_(&self) -> &TableType {
        &self.type_
    }

    /// Returns the current size
    pub fn size(&self) -> u32 {
        self.elements.len() as u32
    }

    /// Grows the table by the specified number of elements
    pub fn grow(&mut self, delta: u32) -> Result<u32> {
        let old_size = self.size();
        let new_size = old_size
            .checked_add(delta)
            .ok_or_else(|| Error::Execution("Table size overflow".into()))?;

        if new_size > self.type_.max.unwrap_or(u32::MAX) {
            return Err(Error::Execution("Table size exceeds maximum".into()));
        }

        self.elements.resize(new_size as usize, None);
        Ok(old_size)
    }

    /// Gets an element from the table
    pub fn get(&self, idx: u32) -> Result<Option<Value>> {
        self.check_bounds(idx)?;
        Ok(self.elements[idx as usize].clone())
    }

    /// Sets an element in the table
    pub fn set(&mut self, idx: u32, value: Option<Value>) -> Result<()> {
        self.check_bounds(idx)?;
        self.elements[idx as usize] = value;
        Ok(())
    }

    /// Initializes a range of elements from a vector
    pub fn init(&mut self, offset: u32, init: &[Option<Value>]) -> Result<()> {
        let end = offset
            .checked_add(init.len() as u32)
            .ok_or_else(|| Error::Execution("Table initialization overflow".into()))?;
        self.check_bounds(end - 1)?;

        // Clone each element individually since Option<Value> doesn't implement Copy
        for (i, value) in init.iter().enumerate() {
            self.elements[offset as usize + i] = value.clone();
        }

        Ok(())
    }

    /// Copies elements from one range to another
    pub fn copy(&mut self, dst: u32, src: u32, len: u32) -> Result<()> {
        let dst_end = dst
            .checked_add(len)
            .ok_or_else(|| Error::Execution("Table copy destination overflow".into()))?;
        let src_end = src
            .checked_add(len)
            .ok_or_else(|| Error::Execution("Table copy source overflow".into()))?;
        self.check_bounds(dst_end - 1)?;
        self.check_bounds(src_end - 1)?;

        if dst <= src {
            // Forward copy
            for i in 0..len {
                self.elements[(dst + i) as usize] = self.elements[(src + i) as usize].clone();
            }
        } else {
            // Backward copy
            for i in (0..len).rev() {
                self.elements[(dst + i) as usize] = self.elements[(src + i) as usize].clone();
            }
        }
        Ok(())
    }

    /// Fills a range of elements with a value
    pub fn fill(&mut self, offset: u32, len: u32, value: Option<Value>) -> Result<()> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| Error::Execution("Table fill overflow".into()))?;
        self.check_bounds(end - 1)?;
        for i in offset..end {
            self.elements[i as usize] = value.clone();
        }
        Ok(())
    }

    /// Checks if a table access is within bounds
    fn check_bounds(&self, idx: u32) -> Result<()> {
        if idx >= self.elements.len() as u32 {
            return Err(Error::Execution("Table access out of bounds".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::Value;
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
        let mut table = Table::new(table_type);

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
        let mut table = Table::new(create_test_table_type(1, None));
        assert!(table.grow(1000).is_ok());
        assert_eq!(table.size(), 1001);

        Ok(())
    }

    #[test]
    fn test_table_access() -> Result<()> {
        let mut table = Table::new(create_test_table_type(5, None));

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
        let mut table = Table::new(create_test_table_type(5, None));
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
        let mut table = Table::new(create_test_table_type(10, None));

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
        let mut table = Table::new(create_test_table_type(5, None));
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
