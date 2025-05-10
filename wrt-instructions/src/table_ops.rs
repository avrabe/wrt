//! Table operations for WebAssembly instructions.
//!
//! This module provides pure implementations for WebAssembly table access instructions,
//! including get, set, grow, and size operations.

use crate::prelude::*;
use wrt_error::{codes, ErrorCategory};
use wrt_types::values::{ExternRef, FuncRef};

/// Represents a reference value in WebAssembly
#[derive(Debug, Clone, PartialEq)]
pub enum RefValue {
    /// A function reference
    FuncRef(u32),
    /// An external reference
    ExternRef(u32),
    /// Null reference
    Null,
}

/// Represents a pure table operation for WebAssembly.
#[derive(Debug, Clone)]
pub enum TableOp {
    /// Get an element from a table
    TableGet(u32),
    /// Set an element in a table
    TableSet(u32),
    /// Get the size of a table
    TableSize(u32),
    /// Grow a table by a given number of elements
    TableGrow(u32),
    /// Fill a table with a value
    TableFill(u32),
    /// Copy elements from one table to another
    TableCopy {
        /// Destination table index
        dst_table: u32,
        /// Source table index
        src_table: u32,
    },
    /// Initialize a table from an element segment
    TableInit {
        /// Table index to initialize
        table_index: u32,
        /// Element segment index to use
        elem_index: u32,
    },
    /// Drop an element segment
    ElemDrop(u32),
}

/// Execution context for table operations
pub trait TableContext {
    /// Get a reference from a table
    fn get_table_element(&self, table_index: u32, elem_index: u32) -> Result<RefValue>;

    /// Set a reference in a table
    fn set_table_element(
        &mut self,
        table_index: u32,
        elem_index: u32,
        value: RefValue,
    ) -> Result<()>;

    /// Get the size of a table
    fn get_table_size(&self, table_index: u32) -> Result<u32>;

    /// Grow a table by a given number of elements
    fn grow_table(&mut self, table_index: u32, delta: u32, init_value: RefValue) -> Result<i32>;

    /// Fill a table with a value
    fn fill_table(&mut self, table_index: u32, dst: u32, val: RefValue, len: u32) -> Result<()>;

    /// Copy elements from one table to another
    fn copy_table(
        &mut self,
        dst_table: u32,
        dst_index: u32,
        src_table: u32,
        src_index: u32,
        len: u32,
    ) -> Result<()>;

    /// Initialize a table from an element segment
    fn init_table_from_elem(
        &mut self,
        table_index: u32,
        dst: u32,
        elem_index: u32,
        src: u32,
        len: u32,
    ) -> Result<()>;

    /// Drop an element segment
    fn drop_elem(&mut self, elem_index: u32) -> Result<()>;

    /// Push a value to the context
    fn push_table_value(&mut self, value: Value) -> Result<()>;

    /// Pop a value from the context
    fn pop_table_value(&mut self) -> Result<Value>;
}

impl<T: TableContext> PureInstruction<T, Error> for TableOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            Self::TableGet(table_index) => {
                let index = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;

                if index < 0 {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", index as u32),
                    ));
                }

                let ref_val = context.get_table_element(*table_index, index as u32)?;

                let value = match ref_val {
                    RefValue::FuncRef(idx) => Value::FuncRef(Some(FuncRef::from_index(idx))),
                    RefValue::ExternRef(idx) => Value::ExternRef(Some(ExternRef { index: idx })),
                    RefValue::Null => Value::FuncRef(None), // Using FuncRef(None) to represent Null
                };

                context.push_table_value(value)
            }
            Self::TableSet(table_index) => {
                let value = context.pop_table_value()?;
                let index = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;

                if index < 0 {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", index as u32),
                    ));
                }

                let ref_val = match value {
                    Value::FuncRef(Some(func_ref)) => RefValue::FuncRef(func_ref.index),
                    Value::FuncRef(None) => RefValue::Null,
                    Value::ExternRef(Some(extern_ref)) => RefValue::ExternRef(extern_ref.index),
                    Value::ExternRef(None) => RefValue::Null,
                    _ => {
                        return Err(Error::new(
                            ErrorCategory::Type,
                            codes::INVALID_TYPE,
                            "Invalid table type",
                        ));
                    }
                };

                context.set_table_element(*table_index, index as u32, ref_val)
            }
            Self::TableSize(table_index) => {
                let size = context.get_table_size(*table_index)?;
                context.push_table_value(Value::I32(size as i32))
            }
            Self::TableGrow(table_index) => {
                let delta = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;
                let init_value = context.pop_table_value()?;

                if delta < 0 {
                    return Err(Error::new(
                        ErrorCategory::Type,
                        codes::INVALID_TYPE,
                        "Invalid table type",
                    ));
                }

                let ref_val = match init_value {
                    Value::FuncRef(Some(func_ref)) => RefValue::FuncRef(func_ref.index),
                    Value::FuncRef(None) => RefValue::Null,
                    Value::ExternRef(Some(extern_ref)) => RefValue::ExternRef(extern_ref.index),
                    Value::ExternRef(None) => RefValue::Null,
                    _ => {
                        return Err(Error::new(
                            ErrorCategory::Type,
                            codes::INVALID_TYPE,
                            "Invalid table type",
                        ));
                    }
                };

                let prev_size = context.grow_table(*table_index, delta as u32, ref_val)?;
                context.push_table_value(Value::I32(prev_size))
            }
            Self::TableFill(table_index) => {
                let len = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;
                let val = context.pop_table_value()?;
                let dst = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;

                if dst < 0 || len < 0 {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!(
                            "Invalid table index: {}",
                            if dst < 0 { dst as u32 } else { len as u32 }
                        ),
                    ));
                }

                let ref_val = match val {
                    Value::FuncRef(Some(func_ref)) => RefValue::FuncRef(func_ref.index),
                    Value::FuncRef(None) => RefValue::Null,
                    Value::ExternRef(Some(extern_ref)) => RefValue::ExternRef(extern_ref.index),
                    Value::ExternRef(None) => RefValue::Null,
                    _ => {
                        return Err(Error::new(
                            ErrorCategory::Type,
                            codes::INVALID_TYPE,
                            "Invalid table type",
                        ));
                    }
                };

                context.fill_table(*table_index, dst as u32, ref_val, len as u32)
            }
            Self::TableCopy { dst_table, src_table } => {
                let len = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;
                let src = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;
                let dst = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;

                if dst < 0 || src < 0 || len < 0 {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!(
                            "Invalid table index: {}",
                            if dst < 0 {
                                dst as u32
                            } else if src < 0 {
                                src as u32
                            } else {
                                len as u32
                            }
                        ),
                    ));
                }

                context.copy_table(*dst_table, dst as u32, *src_table, src as u32, len as u32)
            }
            Self::TableInit { table_index, elem_index } => {
                let len = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;
                let src = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;
                let dst = context.pop_table_value()?.as_i32().ok_or_else(|| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Invalid table type")
                })?;

                if dst < 0 || src < 0 || len < 0 {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!(
                            "Invalid table index: {}",
                            if dst < 0 {
                                dst as u32
                            } else if src < 0 {
                                src as u32
                            } else {
                                len as u32
                            }
                        ),
                    ));
                }

                context.init_table_from_elem(
                    *table_index,
                    dst as u32,
                    *elem_index,
                    src as u32,
                    len as u32,
                )
            }
            Self::ElemDrop(elem_index) => context.drop_elem(*elem_index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Import Vec and vec! based on feature flags
    #[cfg(feature = "std")]
    use std::vec::Vec;

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec::Vec;

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec;

    // Mock table context for testing
    struct MockTableContext {
        tables: Vec<Vec<RefValue>>,
        stack: Vec<Value>,
        elem_segments: Vec<Vec<RefValue>>,
    }

    impl MockTableContext {
        fn new() -> Self {
            let tables = vec![
                vec![RefValue::Null; 10], // Table 0
                vec![RefValue::Null; 5],  // Table 1
            ];

            let elem_segments = vec![
                vec![RefValue::FuncRef(1), RefValue::FuncRef(2), RefValue::FuncRef(3)], // Elem 0
                vec![RefValue::FuncRef(4), RefValue::FuncRef(5)],                       // Elem 1
            ];

            Self { tables, stack: Vec::new(), elem_segments }
        }
    }

    impl TableContext for MockTableContext {
        fn get_table_element(&self, table_index: u32, elem_index: u32) -> Result<RefValue> {
            if let Some(table) = self.tables.get(table_index as usize) {
                if let Some(elem) = table.get(elem_index as usize) {
                    Ok(elem.clone())
                } else {
                    Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", elem_index),
                    ))
                }
            } else {
                Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", table_index),
                ))
            }
        }

        fn set_table_element(
            &mut self,
            table_index: u32,
            elem_index: u32,
            value: RefValue,
        ) -> Result<()> {
            if let Some(table) = self.tables.get_mut(table_index as usize) {
                if let Some(elem) = table.get_mut(elem_index as usize) {
                    *elem = value;
                    Ok(())
                } else {
                    Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", elem_index),
                    ))
                }
            } else {
                Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", table_index),
                ))
            }
        }

        fn get_table_size(&self, table_index: u32) -> Result<u32> {
            if let Some(table) = self.tables.get(table_index as usize) {
                Ok(table.len() as u32)
            } else {
                Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", table_index),
                ))
            }
        }

        fn grow_table(
            &mut self,
            table_index: u32,
            delta: u32,
            init_value: RefValue,
        ) -> Result<i32> {
            if let Some(table) = self.tables.get_mut(table_index as usize) {
                let old_size = table.len() as i32;

                for _ in 0..delta {
                    table.push(init_value.clone());
                }

                Ok(old_size)
            } else {
                Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", table_index),
                ))
            }
        }

        fn fill_table(
            &mut self,
            table_index: u32,
            dst: u32,
            val: RefValue,
            len: u32,
        ) -> Result<()> {
            if let Some(table) = self.tables.get_mut(table_index as usize) {
                if dst as usize + len as usize > table.len() {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", dst),
                    ));
                }

                for i in 0..len {
                    table[dst as usize + i as usize] = val.clone();
                }

                Ok(())
            } else {
                Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", table_index),
                ))
            }
        }

        fn copy_table(
            &mut self,
            dst_table: u32,
            dst_index: u32,
            src_table: u32,
            src_index: u32,
            len: u32,
        ) -> Result<()> {
            // First, check if indexes are valid
            if dst_table as usize >= self.tables.len() || src_table as usize >= self.tables.len() {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!(
                        "Invalid table index: {}",
                        if dst_table < 0 {
                            dst_table as u32
                        } else if src_table < 0 {
                            src_table as u32
                        } else {
                            len
                        }
                    ),
                ));
            }

            // Get the needed information from source table
            let src_elements: Vec<RefValue> = {
                let src_table = &self.tables[src_table as usize];

                if src_index as usize + len as usize > src_table.len() {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", src_index),
                    ));
                }

                src_table[src_index as usize..(src_index as usize + len as usize)].to_vec()
            };

            // Now modify destination table
            let dst_table = &mut self.tables[dst_table as usize];
            if dst_index as usize + len as usize > dst_table.len() {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", dst_index),
                ));
            }

            for i in 0..len as usize {
                dst_table[dst_index as usize + i] = src_elements[i].clone();
            }

            Ok(())
        }

        fn init_table_from_elem(
            &mut self,
            table_index: u32,
            dst: u32,
            elem_index: u32,
            src: u32,
            len: u32,
        ) -> Result<()> {
            if elem_index as usize >= self.elem_segments.len() {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid element index: {}", elem_index),
                ));
            }

            let elem_segment = &self.elem_segments[elem_index as usize];

            if src as usize + len as usize > elem_segment.len() {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", src),
                ));
            }

            if let Some(table) = self.tables.get_mut(table_index as usize) {
                if dst as usize + len as usize > table.len() {
                    return Err(Error::new(
                        ErrorCategory::Resource,
                        codes::RESOURCE_ERROR,
                        format!("Invalid table index: {}", dst),
                    ));
                }

                for i in 0..len {
                    table[dst as usize + i as usize] =
                        elem_segment[src as usize + i as usize].clone();
                }

                Ok(())
            } else {
                Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid table index: {}", table_index),
                ))
            }
        }

        fn drop_elem(&mut self, elem_index: u32) -> Result<()> {
            if elem_index as usize >= self.elem_segments.len() {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::RESOURCE_ERROR,
                    format!("Invalid element index: {}", elem_index),
                ));
            }

            // Just clear the element segment, but keep the entry in the vec for simplicity
            self.elem_segments[elem_index as usize].clear();
            Ok(())
        }

        fn push_table_value(&mut self, value: Value) -> Result<()> {
            self.stack.push(value);
            Ok(())
        }

        fn pop_table_value(&mut self) -> Result<Value> {
            self.stack.pop().ok_or_else(|| {
                Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
            })
        }
    }

    #[test]
    fn test_table_get_set() {
        let mut context = MockTableContext::new();

        // Set table[0][2] to FuncRef(42)
        context.push_table_value(Value::I32(2)).unwrap();
        context.push_table_value(Value::FuncRef(Some(FuncRef::from_index(42)))).unwrap();
        TableOp::TableSet(0).execute(&mut context).unwrap();

        // Get table[0][2]
        context.push_table_value(Value::I32(2)).unwrap();
        TableOp::TableGet(0).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(42)))
        );
    }

    #[test]
    fn test_table_size_grow() {
        let mut context = MockTableContext::new();

        // Get table size
        TableOp::TableSize(0).execute(&mut context).unwrap();
        assert_eq!(context.pop_table_value().unwrap(), Value::I32(10));

        // Grow table by 5 elements
        context.push_table_value(Value::FuncRef(None)).unwrap();
        context.push_table_value(Value::I32(5)).unwrap();
        TableOp::TableGrow(0).execute(&mut context).unwrap();
        assert_eq!(context.pop_table_value().unwrap(), Value::I32(10)); // Previous size

        // Check new size
        TableOp::TableSize(0).execute(&mut context).unwrap();
        assert_eq!(context.pop_table_value().unwrap(), Value::I32(15));
    }

    #[test]
    fn test_table_fill() {
        let mut context = MockTableContext::new();

        // Fill table[0][3..6] with FuncRef(99)
        context.push_table_value(Value::I32(3)).unwrap(); // dst
        context.push_table_value(Value::FuncRef(Some(FuncRef::from_index(99)))).unwrap(); // val
        context.push_table_value(Value::I32(3)).unwrap(); // len
        TableOp::TableFill(0).execute(&mut context).unwrap();

        // Check filled values
        context.push_table_value(Value::I32(3)).unwrap();
        TableOp::TableGet(0).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(99)))
        );

        context.push_table_value(Value::I32(4)).unwrap();
        TableOp::TableGet(0).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(99)))
        );

        context.push_table_value(Value::I32(5)).unwrap();
        TableOp::TableGet(0).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(99)))
        );
    }

    #[test]
    fn test_table_copy() {
        let mut context = MockTableContext::new();

        // Set up source values
        context.set_table_element(0, 1, RefValue::FuncRef(101)).unwrap();
        context.set_table_element(0, 2, RefValue::FuncRef(102)).unwrap();
        context.set_table_element(0, 3, RefValue::FuncRef(103)).unwrap();

        // Copy table[0][1..4] to table[1][0..3]
        context.push_table_value(Value::I32(0)).unwrap(); // dst
        context.push_table_value(Value::I32(1)).unwrap(); // src
        context.push_table_value(Value::I32(3)).unwrap(); // len
        TableOp::TableCopy { dst_table: 1, src_table: 0 }.execute(&mut context).unwrap();

        // Check copied values
        context.push_table_value(Value::I32(0)).unwrap();
        TableOp::TableGet(1).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(101)))
        );

        context.push_table_value(Value::I32(1)).unwrap();
        TableOp::TableGet(1).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(102)))
        );

        context.push_table_value(Value::I32(2)).unwrap();
        TableOp::TableGet(1).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(103)))
        );
    }

    #[test]
    fn test_table_init_elem_drop() {
        let mut context = MockTableContext::new();

        // Initialize table[0][4..6] from elem_segment[0][1..3]
        context.push_table_value(Value::I32(4)).unwrap(); // dst
        context.push_table_value(Value::I32(1)).unwrap(); // src
        context.push_table_value(Value::I32(2)).unwrap(); // len
        TableOp::TableInit { table_index: 0, elem_index: 0 }.execute(&mut context).unwrap();

        // Check initialized values (should be FuncRef(2) and FuncRef(3))
        context.push_table_value(Value::I32(4)).unwrap();
        TableOp::TableGet(0).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(2)))
        );

        context.push_table_value(Value::I32(5)).unwrap();
        TableOp::TableGet(0).execute(&mut context).unwrap();
        assert_eq!(
            context.pop_table_value().unwrap(),
            Value::FuncRef(Some(FuncRef::from_index(3)))
        );

        // Drop element segment
        TableOp::ElemDrop(0).execute(&mut context).unwrap();

        // Check that element segment is now empty (operation should fail)
        context.push_table_value(Value::I32(7)).unwrap(); // dst
        context.push_table_value(Value::I32(0)).unwrap(); // src
        context.push_table_value(Value::I32(1)).unwrap(); // len
        let result = TableOp::TableInit { table_index: 0, elem_index: 0 }.execute(&mut context);
        assert!(result.is_err());
    }
}
