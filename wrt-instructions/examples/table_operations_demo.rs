// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Demonstration of table operations bridging between wrt-instructions and wrt-runtime
//!
//! This example shows how the TableOperations trait implementation allows
//! WebAssembly table instructions to work directly with the runtime Table implementation.
//!
//! This example requires std/alloc features.

#![cfg(any(feature = "std", ))]

use wrt_error::Result;
use wrt_instructions::{
    table_ops::{TableGet, TableSet, TableSize, TableGrow, TableFill, TableCopy, TableOperations},
    prelude::Value,
};
use wrt_runtime::table::{Table, TableManager};
use wrt_foundation::{
    types::{Limits as WrtLimits, TableType as WrtTableType, ValueType as WrtValueType},
    values::{FuncRef, ExternRef},
};

fn main() -> Result<()> {
    println!("WebAssembly Table Operations Demo");
    println!("=================================\n");
    
    // Create table types for demonstration
    let funcref_table_type = WrtTableType {
        element_type: WrtValueType::FuncRef,
        limits: WrtLimits { min: 5, max: Some(10) },
    };
    
    let externref_table_type = WrtTableType {
        element_type: WrtValueType::ExternRef,
        limits: WrtLimits { min: 3, max: Some(8) },
    };
    
    // 1. Demonstrate single table operations
    println!("\n1. Testing single table operations:";
    demonstrate_single_table_operations(funcref_table_type.clone())?;
    
    // 2. Demonstrate multiple table operations with TableManager
    println!("\n2. Testing multiple table operations:";
    demonstrate_multiple_table_operations(funcref_table_type, externref_table_type)?;
    
    // 3. Demonstrate table growth operations
    println!("\n3. Testing table growth:";
    demonstrate_table_growth()?;
    
    // 4. Demonstrate table fill and copy operations
    println!("\n4. Testing table fill and copy:";
    demonstrate_table_fill_and_copy()?;
    
    println!("\n✓ All table operations completed successfully!";
    println!("✓ The TableOperations trait successfully bridges wrt-instructions and wrt-runtime!";
    
    Ok(())
}

fn demonstrate_single_table_operations(table_type: WrtTableType) -> Result<()> {
    println!("  Creating table with {} initial elements, max {}", 
             table_type.limits.min, 
             table_type.limits.max.unwrap_or(u32::MAX;
    
    // Create a single table
    let mut table = Table::new(table_type)?;
    
    // Test table.size operation
    let size_op = TableSize::new(0;
    let size_result = size_op.execute(&table)?;
    println!("  Initial table size: {:?}", size_result;
    
    // Test table.set operation - set a function reference at index 2
    let set_op = TableSet::new(0;
    let func_ref = Value::FuncRef(Some(FuncRef::from_index(42);
    set_op.execute(&mut table, &Value::I32(2), &func_ref)?;
    println!("  Set FuncRef(42) at index 2";
    
    // Test table.get operation - retrieve the function reference
    let get_op = TableGet::new(0;
    let retrieved_value = get_op.execute(&table, &Value::I32(2))?;
    println!("  Retrieved value at index 2: {:?}", retrieved_value;
    
    // Verify the value is correct
    match retrieved_value {
        Value::FuncRef(Some(fr)) if fr.index() == 42 => {
            println!("  ✓ Function reference correctly stored and retrieved";
        }
        _ => {
            println!("  ✗ Unexpected value retrieved";
            return Err(wrt_error::Error::validation_error("Value mismatch";
        }
    }
    
    Ok(())
}

fn demonstrate_multiple_table_operations(
    funcref_table_type: WrtTableType,
    externref_table_type: WrtTableType,
) -> Result<()> {
    // Create a table manager and add multiple tables
    let mut table_manager = TableManager::new);
    
    let funcref_table = Table::new(funcref_table_type)?;
    let externref_table = Table::new(externref_table_type)?;
    
    let funcref_table_index = table_manager.add_table(funcref_table;
    let externref_table_index = table_manager.add_table(externref_table;
    
    println!("  Created table manager with {} tables", table_manager.table_count);
    println!("  FuncRef table index: {}", funcref_table_index;
    println!("  ExternRef table index: {}", externref_table_index;
    
    // Test operations on different tables
    let set_op = TableSet::new(funcref_table_index;
    let func_ref = Value::FuncRef(Some(FuncRef::from_index(100);
    set_op.execute(&mut table_manager, &Value::I32(0), &func_ref)?;
    println!("  Set FuncRef(100) at index 0 in funcref table";
    
    let set_op = TableSet::new(externref_table_index;
    let extern_ref = Value::ExternRef(Some(ExternRef { index: 200 };
    set_op.execute(&mut table_manager, &Value::I32(1), &extern_ref)?;
    println!("  Set ExternRef(200) at index 1 in externref table";
    
    // Retrieve values from different tables
    let get_op = TableGet::new(funcref_table_index;
    let func_value = get_op.execute(&table_manager, &Value::I32(0))?;
    println!("  Retrieved from funcref table: {:?}", func_value;
    
    let get_op = TableGet::new(externref_table_index;
    let extern_value = get_op.execute(&table_manager, &Value::I32(1))?;
    println!("  Retrieved from externref table: {:?}", extern_value;
    
    // Test table sizes
    let size_op = TableSize::new(funcref_table_index;
    let funcref_size = size_op.execute(&table_manager)?;
    
    let size_op = TableSize::new(externref_table_index;
    let externref_size = size_op.execute(&table_manager)?;
    
    println!("  FuncRef table size: {:?}", funcref_size;
    println!("  ExternRef table size: {:?}", externref_size;
    
    Ok(())
}

fn demonstrate_table_growth() -> Result<()> {
    let table_type = WrtTableType {
        element_type: WrtValueType::FuncRef,
        limits: WrtLimits { min: 2, max: Some(5) },
    };
    
    let mut table = Table::new(table_type)?;
    
    // Test initial size
    let size_op = TableSize::new(0;
    let initial_size = size_op.execute(&table)?;
    println!("  Initial size: {:?}", initial_size;
    
    // Test table.grow operation
    let grow_op = TableGrow::new(0;
    let init_value = Value::FuncRef(Some(FuncRef::from_index(999);
    let previous_size = grow_op.execute(&mut table, &init_value, &Value::I32(2))?;
    println!("  Grew table by 2 elements, previous size: {:?}", previous_size;
    
    // Check new size
    let new_size = size_op.execute(&table)?;
    println!("  New size after growth: {:?}", new_size;
    
    // Verify that new elements are initialized correctly
    let get_op = TableGet::new(0;
    for i in 2..4 {
        let value = get_op.execute(&table, &Value::I32(i))?;
        println!("  New element at index {}: {:?}", i, value;
    }
    
    // Test growth beyond maximum (should fail)
    let fail_result = grow_op.execute(&mut table, &init_value, &Value::I32(2;
    match fail_result {
        Ok(Value::I32(-1)) => println!("  ✓ Growth beyond maximum correctly returned -1"),
        Ok(other) => println!("  ✗ Expected -1, got: {:?}", other),
        Err(e) => println!("  ✗ Growth failed with error: {}", e),
    }
    
    Ok(())
}

fn demonstrate_table_fill_and_copy() -> Result<()> {
    let table_type = WrtTableType {
        element_type: WrtValueType::FuncRef,
        limits: WrtLimits { min: 10, max: Some(20) },
    };
    
    let mut table = Table::new(table_type)?;
    
    // Initialize some values for copying
    let set_op = TableSet::new(0;
    for i in 0..3 {
        let func_ref = Value::FuncRef(Some(FuncRef::from_index(100 + i);
        set_op.execute(&mut table, &Value::I32(i), &func_ref)?;
    }
    println!("  Initialized elements 0-2 with FuncRef(100-102)";
    
    // Test table.fill operation
    let fill_op = TableFill::new(0;
    let fill_value = Value::FuncRef(Some(FuncRef::from_index(777);
    fill_op.execute(&mut table, &Value::I32(5), &fill_value, &Value::I32(3))?;
    println!("  Filled elements 5-7 with FuncRef(777)";
    
    // Verify fill worked
    let get_op = TableGet::new(0;
    for i in 5..8 {
        let value = get_op.execute(&table, &Value::I32(i))?;
        println!("    Element {}: {:?}", i, value;
    }
    
    // Test table.copy operation (same table)
    let copy_op = TableCopy::new(0, 0); // same source and destination table
    copy_op.execute(&mut table, &Value::I32(8), &Value::I32(0), &Value::I32(3))?;
    println!("  Copied elements 0-2 to positions 8-10";
    
    // Verify copy worked
    for i in 8..11 {
        let value = get_op.execute(&table, &Value::I32(i))?;
        println!("    Copied element {}: {:?}", i, value;
    }
    
    // Test bounds checking - should fail
    let copy_result = copy_op.execute(&mut table, &Value::I32(15), &Value::I32(0), &Value::I32(10;
    match copy_result {
        Err(_) => println!("  ✓ Out-of-bounds copy correctly failed"),
        Ok(()) => println!("  ✗ Out-of-bounds copy should have failed"),
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_operations_integration() -> Result<()> {
        let table_type = WrtTableType {
            element_type: WrtValueType::FuncRef,
            limits: WrtLimits { min: 5, max: Some(10) },
        };
        
        let mut table = Table::new(table_type)?;
        
        // Test basic set/get cycle
        let set_op = TableSet::new(0;
        let get_op = TableGet::new(0;
        let func_ref = Value::FuncRef(Some(FuncRef::from_index(42);
        
        set_op.execute(&mut table, &Value::I32(2), &func_ref)?;
        let result = get_op.execute(&table, &Value::I32(2))?;
        
        assert_eq!(result, func_ref;
        
        Ok(())
    }
    
    #[test]
    fn test_table_manager_multiple_tables() -> Result<()> {
        let mut table_manager = TableManager::new);
        
        let funcref_table_type = WrtTableType {
            element_type: WrtValueType::FuncRef,
            limits: WrtLimits { min: 3, max: Some(5) },
        };
        
        let externref_table_type = WrtTableType {
            element_type: WrtValueType::ExternRef,
            limits: WrtLimits { min: 2, max: Some(4) },
        };
        
        let funcref_table = Table::new(funcref_table_type)?;
        let externref_table = Table::new(externref_table_type)?;
        
        let table0 = table_manager.add_table(funcref_table;
        let table1 = table_manager.add_table(externref_table;
        
        assert_eq!(table0, 0;
        assert_eq!(table1, 1;
        assert_eq!(table_manager.table_count(), 2;
        
        // Test operations on different tables
        let set_op = TableSet::new(table0;
        let func_ref = Value::FuncRef(Some(FuncRef::from_index(123);
        set_op.execute(&mut table_manager, &Value::I32(0), &func_ref)?;
        
        let get_op = TableGet::new(table0;
        let result = get_op.execute(&table_manager, &Value::I32(0))?;
        assert_eq!(result, func_ref;
        
        Ok(())
    }
    
    #[test]
    fn test_table_growth_operations() -> Result<()> {
        let table_type = WrtTableType {
            element_type: WrtValueType::FuncRef,
            limits: WrtLimits { min: 2, max: Some(4) },
        };
        
        let mut table = Table::new(table_type)?;
        
        let size_op = TableSize::new(0;
        let grow_op = TableGrow::new(0;
        
        // Initial size should be 2
        let initial_size = size_op.execute(&table)?;
        assert_eq!(initial_size, Value::I32(2;
        
        // Grow by 1
        let init_value = Value::FuncRef(Some(FuncRef::from_index(999);
        let prev_size = grow_op.execute(&mut table, &init_value, &Value::I32(1))?;
        assert_eq!(prev_size, Value::I32(2;
        
        // New size should be 3
        let new_size = size_op.execute(&table)?;
        assert_eq!(new_size, Value::I32(3;
        
        // Try to grow beyond maximum - should return -1
        let fail_result = grow_op.execute(&mut table, &init_value, &Value::I32(2))?;
        assert_eq!(fail_result, Value::I32(-1;
        
        Ok(())
    }
    
    #[test]
    fn test_table_fill_and_copy() -> Result<()> {
        let table_type = WrtTableType {
            element_type: WrtValueType::FuncRef,
            limits: WrtLimits { min: 8, max: Some(16) },
        };
        
        let mut table = Table::new(table_type)?;
        
        // Fill operation
        let fill_op = TableFill::new(0;
        let fill_value = Value::FuncRef(Some(FuncRef::from_index(555);
        fill_op.execute(&mut table, &Value::I32(2), &fill_value, &Value::I32(3))?;
        
        // Verify fill
        let get_op = TableGet::new(0;
        for i in 2..5 {
            let result = get_op.execute(&table, &Value::I32(i))?;
            assert_eq!(result, fill_value;
        }
        
        // Copy operation
        let copy_op = TableCopy::new(0, 0;
        copy_op.execute(&mut table, &Value::I32(5), &Value::I32(2), &Value::I32(2))?;
        
        // Verify copy
        for i in 5..7 {
            let result = get_op.execute(&table, &Value::I32(i))?;
            assert_eq!(result, fill_value;
        }
        
        Ok(())
    }
    
    #[test]
    fn test_error_handling() -> Result<()> {
        let table_type = WrtTableType {
            element_type: WrtValueType::FuncRef,
            limits: WrtLimits { min: 3, max: Some(5) },
        };
        
        let mut table = Table::new(table_type)?;
        
        let get_op = TableGet::new(0;
        let set_op = TableSet::new(0;
        
        // Test out of bounds access
        let result = get_op.execute(&table, &Value::I32(10;
        assert!(result.is_err();
        
        // Test invalid table index
        let invalid_get_op = TableGet::new(99;
        let result = invalid_get_op.execute(&table, &Value::I32(0;
        assert!(result.is_err();
        
        // Test wrong value type for table
        let extern_ref = Value::ExternRef(Some(ExternRef { index: 1 };
        let result = set_op.execute(&mut table, &Value::I32(0), &extern_ref;
        assert!(result.is_err();
        
        Ok(())
    }
}

fn main() -> Result<()> {
    println!("=== Table Operations Demo ===";
    
    let demo = TableOperationsDemo::new);
    demo.demo_table_operations()?;
    demo.demo_error_handling()?;
    
    println!("Demo completed successfully!";
    Ok(())
}