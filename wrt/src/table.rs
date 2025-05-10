//! Table manipulation logic.
//! This module provides re-exports and convenience functions for wrt-runtime Table implementation.

use crate::error::Result;
use crate::prelude::TypesValue as Value;

// Re-export table types from wrt-runtime
pub use wrt_runtime::{Table, TableType};
pub use wrt_types::types::Limits;

// Alias for backward compatibility
pub type TableAdapter = Table;

/// Create a new table instance
///
/// This is a convenience function that creates a table instance
/// with the given type.
///
/// # Arguments
///
/// * `table_type` - The table type
///
/// # Returns
///
/// A new table instance
///
/// # Errors
///
/// Returns an error if the table cannot be created
pub fn create_table(table_type: TableType) -> Result<Table> {
    let element_type = table_type.element_type;
    Table::new(
        table_type,
        wrt_types::values::Value::default_for_type(&element_type),
    )
}

/// Create a new table instance with a name
///
/// This is a convenience function that creates a table instance
/// with the given type and name.
///
/// # Arguments
///
/// * `table_type` - The table type
/// * `name` - The debug name for the table (currently not used)
///
/// # Returns
///
/// A new table instance
///
/// # Errors
///
/// Returns an error if the table cannot be created
pub fn create_table_with_name(table_type: TableType, _name: &str) -> Result<Table> {
    // Note: set_debug_name is not currently available in the Table implementation
    // TODO: Add debug name support when available
    let table = create_table(table_type)?;
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::TypesValue as Value;
    use crate::ValueType;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    use wrt_error::Result;

    fn create_test_table_type(min: u32, max: Option<u32>) -> TableType {
        TableType {
            element_type: ValueType::FuncRef,
            limits: Limits { min, max },
        }
    }

    #[test]
    fn test_table_creation() {
        let table_type = create_test_table_type(10, Some(20));
        let table = create_table(table_type).unwrap();
        assert_eq!(table.size(), 10);

        // Test with unbounded max
        let table_type_unbounded = create_test_table_type(5, None);
        let table_unbounded = create_table(table_type_unbounded).unwrap();
        assert_eq!(table_unbounded.size(), 5);
    }

    #[test]
    fn test_table_growth() -> Result<()> {
        // Test bounded table
        let table_type = create_test_table_type(10, Some(20));
        let mut table = create_table(table_type)?;

        // Valid growth
        let old_size = table.grow(5)?;
        assert_eq!(old_size, 10);
        assert_eq!(table.size(), 15);

        // Growth to max exactly
        let old_size = table.grow(5)?;
        assert_eq!(old_size, 15);
        assert_eq!(table.size(), 20);

        // Growth beyond max
        let result = table.grow(1);
        assert!(result.is_err());

        // Test unbounded table
        let table_type = create_test_table_type(5, None);
        let mut table = create_table(table_type)?;

        // Growth with no max
        let old_size = table.grow(10)?;
        assert_eq!(old_size, 5);
        assert_eq!(table.size(), 15);

        Ok(())
    }

    #[test]
    fn test_table_access() -> Result<()> {
        let table_type = create_test_table_type(10, Some(20));
        let mut table = Table::new(
            table_type,
            wrt_types::values::Value::default_for_type(&ValueType::FuncRef),
        )?;

        // Get initial value (should be None)
        let val = table.get(5)?;
        assert!(val.is_none());

        // Set a value
        table.set(5, Some(Value::reference(1)))?;

        // Get the value back
        let val = table.get(5)?;
        assert_eq!(val, Some(Value::reference(1)));

        // Out of bounds access
        assert!(table.get(10).is_err());
        assert!(table.set(10, Some(Value::reference(2))).is_err());

        Ok(())
    }

    #[test]
    fn test_table_initialization() -> Result<()> {
        let table_type = create_test_table_type(10, Some(20));
        let mut table = Table::new(
            table_type,
            wrt_types::values::Value::default_for_type(&ValueType::FuncRef),
        )?;

        // Initialize a range
        let init_values = vec![
            Some(Value::reference(1)),
            Some(Value::reference(2)),
            Some(Value::reference(3)),
        ];
        table.init(2, &init_values)?;

        // Check the values
        assert_eq!(table.get(2)?, Some(Value::reference(1)));
        assert_eq!(table.get(3)?, Some(Value::reference(2)));
        assert_eq!(table.get(4)?, Some(Value::reference(3)));

        // Out of bounds initialization
        let result = table.init(8, &init_values);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_table_copy() -> Result<()> {
        let table_type = create_test_table_type(10, Some(20));
        let mut table = Table::new(
            table_type,
            wrt_types::values::Value::default_for_type(&ValueType::FuncRef),
        )?;

        // Initialize source values
        let init_values = vec![
            Some(Value::reference(1)),
            Some(Value::reference(2)),
            Some(Value::reference(3)),
        ];
        table.init(2, &init_values)?;

        // Copy forward (non-overlapping)
        table.copy(5, 2, 3)?;
        assert_eq!(table.get(5)?, Some(Value::reference(1)));
        assert_eq!(table.get(6)?, Some(Value::reference(2)));
        assert_eq!(table.get(7)?, Some(Value::reference(3)));

        // Copy backward (overlapping)
        table.copy(1, 2, 3)?;
        assert_eq!(table.get(1)?, Some(Value::reference(1)));
        assert_eq!(table.get(2)?, Some(Value::reference(2)));
        assert_eq!(table.get(3)?, Some(Value::reference(3)));

        // Out of bounds copy
        assert!(table.copy(8, 2, 3).is_err()); // Destination out of bounds
        assert!(table.copy(2, 8, 3).is_err()); // Source out of bounds

        Ok(())
    }

    #[test]
    fn test_table_fill() -> Result<()> {
        let table_type = create_test_table_type(10, Some(20));
        let mut table = Table::new(
            table_type,
            wrt_types::values::Value::default_for_type(&ValueType::FuncRef),
        )?;

        // Fill a range
        table.fill(2, 3, Some(Value::reference(42)))?;

        // Check the values
        assert_eq!(table.get(2)?, Some(Value::reference(42)));
        assert_eq!(table.get(3)?, Some(Value::reference(42)));
        assert_eq!(table.get(4)?, Some(Value::reference(42)));
        assert_eq!(table.get(5)?, None); // Should not affect values outside range

        // Fill with None (clear values)
        table.fill(2, 3, None)?;
        assert_eq!(table.get(2)?, None);
        assert_eq!(table.get(3)?, None);
        assert_eq!(table.get(4)?, None);

        // Out of bounds fill
        assert!(table.fill(8, 3, Some(Value::reference(42))).is_err());

        Ok(())
    }
}
