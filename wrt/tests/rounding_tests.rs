use std::path::Path;

use wrt::{
    execution::{
        f32_nearest,
        f64_nearest,
    },
    Module,
    Result,
    Value,
};

#[test]
fn test_f32_nearest_rounding() {
    // Test cases for banker's rounding (round to nearest, ties to even)
    let test_cases = vec![
        // Values that should round up (fraction > 0.5)
        (2.7, 3.0),
        (-2.7, -3.0),
        (3.6, 4.0),
        (-3.6, -4.0),
        // Values that should round down (fraction < 0.5)
        (2.2, 2.0),
        (-2.2, -2.0),
        (3.1, 3.0),
        (-3.1, -3.0),
        // Values exactly at x.5 should round to nearest even
        (2.5, 2.0),   // Round to even (2)
        (-2.5, -2.0), // Round to even (-2)
        (3.5, 4.0),   // Round to even (4)
        (-3.5, -4.0), // Round to even (-4)
        (4.5, 4.0),   // Round to even (4)
        (-4.5, -4.0), // Round to even (-4)
        // Edge cases
        (0.0, 0.0),
        (-0.0, -0.0),
    ];

    for (input, expected) in test_cases {
        // Call the function directly
        let value = Value::F32(input);
        let result = f32_nearest(&value);

        assert_eq!(
            result, expected,
            "f32.nearest({}) should be {}, got {}",
            input, expected, result
        );
    }

    // Special cases
    let nan_value = Value::F32(f32::NAN);
    let nan_result = f32_nearest(&nan_value);
    assert!(nan_result.is_nan(), "f32.nearest(NaN) should be NaN");

    let inf_value = Value::F32(f32::INFINITY);
    let inf_result = f32_nearest(&inf_value);
    assert!(
        inf_result.is_infinite() && inf_result.is_sign_positive(),
        "f32.nearest(∞) should be ∞"
    );

    let neg_inf_value = Value::F32(f32::NEG_INFINITY);
    let neg_inf_result = f32_nearest(&neg_inf_value);
    assert!(
        neg_inf_result.is_infinite() && neg_inf_result.is_sign_negative(),
        "f32.nearest(-∞) should be -∞"
    );
}

#[test]
fn test_f64_nearest_rounding() {
    // Test cases for banker's rounding (round to nearest, ties to even)
    let test_cases = vec![
        // Values that should round up (fraction > 0.5)
        (2.7, 3.0),
        (-2.7, -3.0),
        (3.6, 4.0),
        (-3.6, -4.0),
        // Values that should round down (fraction < 0.5)
        (2.2, 2.0),
        (-2.2, -2.0),
        (3.1, 3.0),
        (-3.1, -3.0),
        // Values exactly at x.5 should round to nearest even
        (2.5, 2.0),   // Round to even (2)
        (-2.5, -2.0), // Round to even (-2)
        (3.5, 4.0),   // Round to even (4)
        (-3.5, -4.0), // Round to even (-4)
        (4.5, 4.0),   // Round to even (4)
        (-4.5, -4.0), // Round to even (-4)
        // Edge cases
        (0.0, 0.0),
        (-0.0, -0.0),
    ];

    for (input, expected) in test_cases {
        // Call the function directly
        let value = Value::F64(input);
        let result = f64_nearest(&value);

        assert_eq!(
            result, expected,
            "f64.nearest({}) should be {}, got {}",
            input, expected, result
        );
    }

    // Special cases
    let nan_value = Value::F64(f64::NAN);
    let nan_result = f64_nearest(&nan_value);
    assert!(nan_result.is_nan(), "f64.nearest(NaN) should be NaN");

    let inf_value = Value::F64(f64::INFINITY);
    let inf_result = f64_nearest(&inf_value);
    assert!(
        inf_result.is_infinite() && inf_result.is_sign_positive(),
        "f64.nearest(∞) should be ∞"
    );

    let neg_inf_value = Value::F64(f64::NEG_INFINITY);
    let neg_inf_result = f64_nearest(&neg_inf_value);
    assert!(
        neg_inf_result.is_infinite() && neg_inf_result.is_sign_negative(),
        "f64.nearest(-∞) should be -∞"
    );
}
