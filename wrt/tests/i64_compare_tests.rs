use wrt::{
    behavior::NullBehavior,
    instructions::comparison,
    Error,
    Result,
    Value,
};

#[test]
fn test_direct_i64_comparison() -> Result<()> {
    let mut frame = NullBehavior::default();

    // Test i64.eq (equality)
    let mut values_eq_true = vec![Value::I64(100), Value::I64(100)];
    comparison::i64_eq(&mut values_eq_true, &mut frame)?;
    assert_eq!(values_eq_true.len(), 1);
    assert_eq!(values_eq_true[0], Value::I32(1));

    let mut values_eq_false = vec![Value::I64(100), Value::I64(200)];
    comparison::i64_eq(&mut values_eq_false, &mut frame)?;
    assert_eq!(values_eq_false.len(), 1);
    assert_eq!(values_eq_false[0], Value::I32(0));

    // Test i64.ne (not equals)
    let mut values_ne_true = vec![Value::I64(100), Value::I64(200)];
    comparison::i64_ne(&mut values_ne_true, &mut frame)?;
    assert_eq!(values_ne_true.len(), 1);
    assert_eq!(values_ne_true[0], Value::I32(1));

    let mut values_ne_false = vec![Value::I64(100), Value::I64(100)];
    comparison::i64_ne(&mut values_ne_false, &mut frame)?;
    assert_eq!(values_ne_false.len(), 1);
    assert_eq!(values_ne_false[0], Value::I32(0));

    // Test i64.lt_s (less than, signed)
    let mut values_lt_s_true = vec![Value::I64(-100), Value::I64(100)];
    comparison::i64_lt_s(&mut values_lt_s_true, &mut frame)?;
    assert_eq!(values_lt_s_true.len(), 1);
    assert_eq!(values_lt_s_true[0], Value::I32(1));

    let mut values_lt_s_false = vec![Value::I64(100), Value::I64(-100)];
    comparison::i64_lt_s(&mut values_lt_s_false, &mut frame)?;
    assert_eq!(values_lt_s_false.len(), 1);
    assert_eq!(values_lt_s_false[0], Value::I32(0));

    // Test i64.gt_s (greater than, signed)
    let mut values_gt_s_true = vec![Value::I64(100), Value::I64(-100)];
    comparison::i64_gt_s(&mut values_gt_s_true, &mut frame)?;
    assert_eq!(values_gt_s_true.len(), 1);
    assert_eq!(values_gt_s_true[0], Value::I32(1));

    let mut values_gt_s_false = vec![Value::I64(-100), Value::I64(100)];
    comparison::i64_gt_s(&mut values_gt_s_false, &mut frame)?;
    assert_eq!(values_gt_s_false.len(), 1);
    assert_eq!(values_gt_s_false[0], Value::I32(0));

    // Test i64.le_s (less than or equal, signed)
    let mut values_le_s_true1 = vec![Value::I64(-100), Value::I64(100)];
    comparison::i64_le_s(&mut values_le_s_true1, &mut frame)?;
    assert_eq!(values_le_s_true1.len(), 1);
    assert_eq!(values_le_s_true1[0], Value::I32(1));

    let mut values_le_s_true2 = vec![Value::I64(100), Value::I64(100)];
    comparison::i64_le_s(&mut values_le_s_true2, &mut frame)?;
    assert_eq!(values_le_s_true2.len(), 1);
    assert_eq!(values_le_s_true2[0], Value::I32(1));

    let mut values_le_s_false = vec![Value::I64(100), Value::I64(-100)];
    comparison::i64_le_s(&mut values_le_s_false, &mut frame)?;
    assert_eq!(values_le_s_false.len(), 1);
    assert_eq!(values_le_s_false[0], Value::I32(0));

    // Test i64.ge_s (greater than or equal, signed)
    let mut values_ge_s_true1 = vec![Value::I64(100), Value::I64(-100)];
    comparison::i64_ge_s(&mut values_ge_s_true1, &mut frame)?;
    assert_eq!(values_ge_s_true1.len(), 1);
    assert_eq!(values_ge_s_true1[0], Value::I32(1));

    let mut values_ge_s_true2 = vec![Value::I64(100), Value::I64(100)];
    comparison::i64_ge_s(&mut values_ge_s_true2, &mut frame)?;
    assert_eq!(values_ge_s_true2.len(), 1);
    assert_eq!(values_ge_s_true2[0], Value::I32(1));

    let mut values_ge_s_false = vec![Value::I64(-100), Value::I64(100)];
    comparison::i64_ge_s(&mut values_ge_s_false, &mut frame)?;
    assert_eq!(values_ge_s_false.len(), 1);
    assert_eq!(values_ge_s_false[0], Value::I32(0));

    // Test i64.lt_u (less than, unsigned)
    // Use large positive values and i64::MIN interpreted as large unsigned
    let mut values_lt_u_true = vec![Value::I64(100), Value::I64(200)];
    comparison::i64_lt_u(&mut values_lt_u_true, &mut frame)?;
    assert_eq!(values_lt_u_true.len(), 1);
    assert_eq!(values_lt_u_true[0], Value::I32(1));

    // Test i64.gt_u (greater than, unsigned)
    let mut values_gt_u_true = vec![Value::I64(i64::MIN), Value::I64(100)]; // i64::MIN is largest u64
    comparison::i64_gt_u(&mut values_gt_u_true, &mut frame)?;
    assert_eq!(values_gt_u_true.len(), 1);
    assert_eq!(values_gt_u_true[0], Value::I32(1));

    Ok(())
}
