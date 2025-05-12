// WRT - wrt-types
// Module: WebAssembly Math Operations
// SW-REQ-ID: N/A (Will be defined by safety analysis for math functions if
// critical)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly Math Operations
//!
//! This module implements the executable logic for WebAssembly numeric
//! instructions. It operates on primitive Rust types and `FloatBits` wrappers,
//! returning `Result` with `TrapCode` for Wasm-defined trapping behavior.

#![allow(clippy::float_arithmetic)] // Wasm spec requires float ops

// TrapCode and WrtError are fundamental for Wasm math ops.
use wrt_error::{codes::TrapCode, Error as WrtError, Result};

use crate::values::{FloatBits32, FloatBits64};

// --- Start of no_std math polyfills for trunc ---
#[cfg(not(feature = "std"))]
mod no_std_math_trunc {
    // Polyfill for f32::trunc
    pub(super) fn trunc_f32_polyfill(f_val: f32) -> f32 {
        if f_val.is_nan() || f_val.is_infinite() || f_val == 0.0 {
            // Use inherent methods on f32
            return f_val;
        }
        let bits = f_val.to_bits(); // Use inherent method on f32
        let sign_mask = 0x8000_0000u32;
        let exponent_mask = 0x7F80_0000u32;
        let mantissa_mask = 0x007F_FFFFu32;

        let sign = bits & sign_mask;
        let exponent_bits = (bits & exponent_mask) >> 23;

        if exponent_bits == 0xFF {
            return f_val;
        }

        let biased_exponent = exponent_bits as i32;
        let exponent = biased_exponent - 127;

        if exponent < 0 {
            return f32::from_bits(sign);
        }

        let fractional_bits_count = 23 - exponent;

        if fractional_bits_count <= 0 {
            return f_val;
        }

        let clear_mask = !((1u32 << fractional_bits_count) - 1);
        let new_mantissa_bits = (bits & mantissa_mask) & clear_mask;

        f32::from_bits(sign | (exponent_bits << 23) | new_mantissa_bits)
    }

    // Polyfill for f64::trunc
    pub(super) fn trunc_f64_polyfill(d_val: f64) -> f64 {
        if d_val.is_nan() || d_val.is_infinite() || d_val == 0.0 {
            // Use inherent methods on f64
            return d_val;
        }
        let bits = d_val.to_bits(); // Use inherent method on f64
        let sign_mask = 0x8000_0000_0000_0000u64;
        let exponent_mask = 0x7FF0_0000_0000_0000u64;
        let mantissa_mask = 0x000F_FFFF_FFFF_FFFFu64;

        let sign = bits & sign_mask;
        let exponent_bits_u64 = (bits & exponent_mask) >> 52;

        if exponent_bits_u64 == 0x7FF {
            return d_val;
        }

        let biased_exponent = exponent_bits_u64 as i32;
        let exponent = biased_exponent - 1023;

        if exponent < 0 {
            return f64::from_bits(sign);
        }

        let fractional_bits_count = 52 - exponent;

        if fractional_bits_count <= 0 {
            return d_val;
        }

        let clear_mask = !((1u64 << fractional_bits_count) - 1);
        let new_mantissa_bits = (bits & mantissa_mask) & clear_mask;

        f64::from_bits(sign | (exponent_bits_u64 << 52) | new_mantissa_bits)
    }
}
// --- End of no_std math polyfills for trunc ---

// --- Start of no_std math polyfills for rounding ---
#[cfg(not(feature = "std"))]
mod no_std_math_rounding {
    use super::no_std_math_trunc::{trunc_f32_polyfill, trunc_f64_polyfill};

    // Polyfill for f32::floor
    pub(super) fn floor_f32_polyfill(f_val: f32) -> f32 {
        if f_val.is_nan() || f_val.is_infinite() || f_val == 0.0 {
            return f_val;
        }
        let t = trunc_f32_polyfill(f_val);
        if f_val >= 0.0 || t == f_val {
            t
        } else {
            t - 1.0
        }
    }

    // Polyfill for f32::ceil
    pub(super) fn ceil_f32_polyfill(f_val: f32) -> f32 {
        if f_val.is_nan() || f_val.is_infinite() || f_val == 0.0 {
            return f_val;
        }
        let t = trunc_f32_polyfill(f_val);
        if f_val <= 0.0 || t == f_val {
            t
        } else {
            t + 1.0
        }
    }

    // Polyfill for f32::round (ties to even)
    pub(super) fn round_ties_to_even_f32_polyfill(f_val: f32) -> f32 {
        if f_val.is_nan() || f_val.is_infinite() || f_val == 0.0 {
            return f_val;
        }

        let fl = floor_f32_polyfill(f_val);
        let fr = f_val - fl;
        if fr < 0.5 {
            return fl;
        }
        if fr > 0.5 {
            return ceil_f32_polyfill(f_val);
        }

        let ce = fl + 1.0;
        let half_fl = fl / 2.0;
        if half_fl == trunc_f32_polyfill(half_fl) {
            fl
        } else {
            ce
        }
    }

    // Polyfill for f64::floor
    pub(super) fn floor_f64_polyfill(d_val: f64) -> f64 {
        if d_val.is_nan() || d_val.is_infinite() || d_val == 0.0 {
            return d_val;
        }
        let t = trunc_f64_polyfill(d_val);
        if d_val >= 0.0 || t == d_val {
            t
        } else {
            t - 1.0
        }
    }

    // Polyfill for f64::ceil
    pub(super) fn ceil_f64_polyfill(d_val: f64) -> f64 {
        if d_val.is_nan() || d_val.is_infinite() || d_val == 0.0 {
            return d_val;
        }
        let t = trunc_f64_polyfill(d_val);
        if d_val <= 0.0 || t == d_val {
            t
        } else {
            t + 1.0
        }
    }

    // Polyfill for f64::round (ties to even)
    pub(super) fn round_ties_to_even_f64_polyfill(d_val: f64) -> f64 {
        if d_val.is_nan() || d_val.is_infinite() || d_val == 0.0 {
            return d_val;
        }

        let fl = floor_f64_polyfill(d_val);
        let fr = d_val - fl;

        if fr < 0.5 {
            return fl;
        }
        if fr > 0.5 {
            return ceil_f64_polyfill(d_val);
        }

        let ce = fl + 1.0;
        let half_fl = fl / 2.0;
        if half_fl == trunc_f64_polyfill(half_fl) {
            fl
        } else {
            ce
        }
    }
}
// --- End of no_std math polyfills for rounding ---

// --- Start of no_std math polyfills for sqrt ---
#[cfg(not(feature = "std"))]
mod no_std_math_sqrt {
    use crate::values::{FloatBits32, FloatBits64}; // Keep for NAN const access

    // Polyfill for f32::sqrt using Newton-Raphson method
    pub(super) fn sqrt_f32_polyfill(f_val: f32) -> f32 {
        if f_val.is_nan() {
            // Wasm spec: "if z is a NaN, then return a canonical NaN".
            return FloatBits32::NAN.value();
        }
        if f_val < 0.0 {
            // sqrt of negative number (excluding -0.0) is NaN.
            return FloatBits32::NAN.value();
        }
        if f_val == 0.0 {
            // Handles +0.0 and -0.0. sqrt(+0)=+0, sqrt(-0)=-0.
            return f_val;
        }
        if f_val.is_infinite() && f_val.is_sign_positive() {
            // sqrt(+Inf) = +Inf
            return f_val;
        }
        // At this point, f_val is a positive, finite, non-zero number.

        if f_val == 1.0 {
            return 1.0;
        } // Exact case, avoids iterations.

        // Initial guess for Newton-Raphson.
        // A simple, generally safe guess:
        let mut y: f32 = if f_val > 1.0 { f_val * 0.5 } else { 1.0 };
        if y == 0.0 {
            // Should only happen if f_val was extremely small, leading to y=0
            y = f32::MIN_POSITIVE; // Use a tiny positive number to start if
                                   // f_val * 0.5 underflowed
        }

        // Newton-Raphson iterations: y_new = (y_old + x / y_old) / 2.0
        // Perform a fixed number of iterations. 8 for f32.
        let mut prev_y;
        for _ in 0..8 {
            prev_y = y;
            if y == 0.0 {
                // Should ideally not be reached if f_val > 0 and initial y > 0
                // This implies f_val is extremely small, or an issue occurred.
                // Result for very small positive f_val should be very small positive.
                return 0.0; // Or handle as an underflow to 0.
            }
            y = (y + f_val / y) * 0.5;
            if y == prev_y {
                // Converged
                break;
            }
        }
        y
    }

    // Polyfill for f64::sqrt using Newton-Raphson method
    pub(super) fn sqrt_f64_polyfill(d_val: f64) -> f64 {
        if d_val.is_nan() {
            return FloatBits64::NAN.value();
        }
        if d_val < 0.0 {
            return FloatBits64::NAN.value();
        }
        if d_val == 0.0 {
            return d_val;
        }
        if d_val.is_infinite() && d_val.is_sign_positive() {
            return d_val;
        }
        // At this point, d_val is a positive, finite, non-zero number.

        if d_val == 1.0 {
            return 1.0;
        } // Exact case

        // Initial guess
        let mut y: f64 = if d_val > 1.0 { d_val * 0.5 } else { 1.0 };
        if y == 0.0 {
            y = f64::MIN_POSITIVE;
        }

        // Newton-Raphson iterations. 12 for f64.
        let mut prev_y;
        for _ in 0..12 {
            prev_y = y;
            if y == 0.0 {
                return 0.0;
            }
            y = (y + d_val / y) * 0.5;
            if y == prev_y {
                // Converged
                break;
            }
        }
        y
    }
}
// --- End of no_std math polyfills for sqrt ---

// --- Compatibility wrappers for float functions (std vs no_std polyfill) ---

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f32_trunc_compat(f: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        f.trunc()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_trunc::trunc_f32_polyfill(f)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f32_ceil_compat(f: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        f.ceil()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::ceil_f32_polyfill(f)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f32_floor_compat(f: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        f.floor()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::floor_f32_polyfill(f)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f32_round_ties_to_even_compat(f: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        f.round() // Rust's f32::round() is round half to even
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::round_ties_to_even_f32_polyfill(f)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f32_sqrt_compat(f: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        f.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_sqrt::sqrt_f32_polyfill(f)
    }
}

// Conditional wrapper for f64::trunc (not used yet, but for completeness)
#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f64_trunc_compat(d: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        d.trunc()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_trunc::trunc_f64_polyfill(d)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f64_ceil_compat(d: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        d.ceil()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::ceil_f64_polyfill(d)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f64_floor_compat(d: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        d.floor()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::floor_f64_polyfill(d)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f64_round_ties_to_even_compat(d: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        d.round() // Rust's f64::round() is round half to even
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::round_ties_to_even_f64_polyfill(d)
    }
}

#[allow(clippy::missing_const_for_fn)] // Float ops / polyfills are not const
#[inline]
fn f64_sqrt_compat(d: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        d.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_sqrt::sqrt_f64_polyfill(d)
    }
}

// Manual signum for no_std, or std version
#[cfg(feature = "std")]
#[inline(always)]
#[allow(dead_code)] // Allow dead code for this specific function
fn f64_signum_compat(x: f64) -> f64 {
    x.signum()
}

#[cfg(not(feature = "std"))]
#[inline(always)]
#[allow(dead_code)] // Allow dead code for this specific function
fn f64_signum_compat(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    } // Or f64::NAN, Wasm usually propagates NaN payload
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    } // Preserve sign of 0.0 / -0.0
}

// --- Helper for Traps ---

/// Helper function to create a WRT trap error.
#[allow(clippy::missing_const_for_fn)] // Cannot be const because Error::new is not const
#[inline]
fn trap(code: TrapCode) -> WrtError {
    // Using specific category and code for traps
    code.into() // Relies on the From<TrapCode> for WrtError impl
}

// --- I32 Operations ---

/// i32.add: Add two i32 values.
/// Performs wrapping addition (modulo 2^32). Does not trap on overflow.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i32_add(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.wrapping_add(rhs))
}

/// i32.sub: Subtract two i32 values.
/// Traps on signed overflow.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerOverflow)` if signed subtraction overflows.
#[inline]
pub fn i32_sub(lhs: i32, rhs: i32) -> Result<i32> {
    lhs.checked_sub(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i32.mul: Multiply two i32 values.
/// Performs wrapping multiplication (modulo 2^32). Does not trap on overflow.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i32_mul(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.wrapping_mul(rhs))
}

/// `i32.div_s`: Signed i32 division.
/// Traps on division by zero.
/// Traps on overflow (`i32::MIN` / -1).
///
/// # Errors
///
/// - `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
/// - `Err(TrapCode::IntegerOverflow)` if `lhs` is `i32::MIN` and `rhs` is -1.
#[inline]
pub fn i32_div_s(lhs: i32, rhs: i32) -> Result<i32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    if lhs == i32::MIN && rhs == -1 {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(lhs.wrapping_div(rhs)) // Standard Rust division truncates towards zero,
                              // as per Wasm.
}

/// `i32.div_u`: Unsigned i32 division.
/// Traps on division by zero.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
#[inline]
pub fn i32_div_u(lhs: u32, rhs: u32) -> Result<u32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    Ok(lhs / rhs) // Standard Rust unsigned division, as per Wasm.
}

/// `i32.rem_s`: Signed i32 remainder.
/// Traps on division by zero.
/// The result has the sign of the dividend.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
#[inline]
pub fn i32_rem_s(lhs: i32, rhs: i32) -> Result<i32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    // `i32::MIN % -1` result is 0 in Rust, which is correct by Wasm spec.
    Ok(lhs.wrapping_rem(rhs))
}

/// `i32.rem_u`: Unsigned i32 remainder.
/// Traps on division by zero.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
#[inline]
pub fn i32_rem_u(lhs: u32, rhs: u32) -> Result<u32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    Ok(lhs % rhs)
}

/// i32.and: Bitwise AND of two i32 values.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i32_and(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs & rhs)
}

/// i32.or: Bitwise OR of two i32 values.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i32_or(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs | rhs)
}

/// i32.xor: Bitwise XOR of two i32 values.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i32_xor(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs ^ rhs)
}

/// i32.shl: Shift left (logical) i32 value.
/// Shift amount is masked to 5 bits (0-31).
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i32_shl(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.wrapping_shl(rhs as u32 & 0x1F))
}

/// `i32.shr_s`: Shift right (arithmetic) signed i32 value.
/// Shift amount is masked to 5 bits (0-31).
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i32_shr_s(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.wrapping_shr(rhs as u32 & 0x1F))
}

/// `i32.shr_u`: Shift right (logical) unsigned i32 value (acting on i32 bits).
/// Shift amount is masked to 5 bits (0-31).
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub fn i32_shr_u(lhs: i32, rhs: i32) -> Result<i32> {
    // Perform as u32 to ensure logical shift, then cast back to i32.
    Ok((lhs as u32).wrapping_shr(rhs as u32 & 0x1F) as i32)
}

/// i32.rotl: Rotate left i32 value.
/// Rotate amount is effectively modulo 32.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i32_rotl(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.rotate_left(rhs as u32))
}

/// i32.rotr: Rotate right i32 value.
/// Rotate amount is effectively modulo 32.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i32_rotr(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.rotate_right(rhs as u32))
}

/// i32.clz: Count leading zeros in an i32 value.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap)]
pub fn i32_clz(val: i32) -> Result<i32> {
    Ok(val.leading_zeros() as i32)
}

/// i32.ctz: Count trailing zeros in an i32 value.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap)]
pub fn i32_ctz(val: i32) -> Result<i32> {
    Ok(val.trailing_zeros() as i32)
}

/// i32.popcnt: Count population (number of set bits) in an i32 value.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap)]
pub fn i32_popcnt(val: i32) -> Result<i32> {
    Ok(val.count_ones() as i32)
}

/// i32.eqz: Check if i32 value is zero. Returns 1 if zero, 0 otherwise.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i32_eqz(val: i32) -> Result<i32> {
    Ok(i32::from(val == 0))
}

// --- I64 Operations ---

/// i64.add: Add two i64 values.
/// Performs wrapping addition (modulo 2^64). Does not trap on overflow.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i64_add(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.wrapping_add(rhs))
}

/// i64.sub: Subtract two i64 values.
/// Traps on signed overflow.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerOverflow)` if signed subtraction overflows.
#[inline]
pub fn i64_sub(lhs: i64, rhs: i64) -> Result<i64> {
    lhs.checked_sub(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i64.mul: Multiply two i64 values.
/// Performs wrapping multiplication (modulo 2^64). Does not trap on overflow.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i64_mul(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.wrapping_mul(rhs))
}

/// `i64.div_s`: Signed i64 division.
/// Traps on division by zero or signed overflow (`i64::MIN / -1`).
///
/// # Errors
/// - `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
/// - `Err(TrapCode::IntegerOverflow)` if `lhs` is `i64::MIN` and `rhs` is -1.
#[inline]
pub fn i64_div_s(lhs: i64, rhs: i64) -> Result<i64> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    if lhs == i64::MIN && rhs == -1 {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(lhs.wrapping_div(rhs)) // Wasm division truncates. Rust's wrapping_div does this.
}

/// `i64.div_u`: Unsigned i64 division.
/// Traps on division by zero.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
#[inline]
pub fn i64_div_u(lhs: u64, rhs: u64) -> Result<u64> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    Ok(lhs / rhs) // Standard Rust unsigned division, as per Wasm.
}

/// `i64.rem_s`: Signed i64 remainder.
/// Traps on division by zero.
/// The result has the sign of the dividend.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
#[inline]
pub fn i64_rem_s(lhs: i64, rhs: i64) -> Result<i64> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    // `i64::MIN % -1` result is 0 in Rust, which is correct by Wasm spec.
    Ok(lhs.wrapping_rem(rhs))
}

/// `i64.rem_u`: Unsigned i64 remainder.
/// Traps on division by zero.
///
/// # Errors
///
/// Returns `Err(TrapCode::IntegerDivideByZero)` if `rhs` is 0.
#[inline]
pub fn i64_rem_u(lhs: u64, rhs: u64) -> Result<u64> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    Ok(lhs % rhs)
}

/// i64.and: Bitwise AND of two i64 values.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i64_and(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs & rhs)
}

/// i64.or: Bitwise OR of two i64 values.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i64_or(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs | rhs)
}

/// i64.xor: Bitwise XOR of two i64 values.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i64_xor(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs ^ rhs)
}

/// i64.shl: Shift left (logical) i64 value.
/// Shift amount is masked to 6 bits (0-63).
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i64_shl(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.wrapping_shl(rhs as u32 & 0x3F))
}

/// `i64.shr_s`: Shift right (arithmetic) signed i64 value.
/// Shift amount is masked to 6 bits (0-63).
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i64_shr_s(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.wrapping_shr(rhs as u32 & 0x3F))
}

/// `i64.shr_u`: Shift right (logical) unsigned i64 value (acting on i64 bits).
/// Shift amount is masked to 6 bits (0-63).
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub fn i64_shr_u(lhs: i64, rhs: i64) -> Result<i64> {
    Ok((lhs as u64).wrapping_shr(rhs as u32 & 0x3F) as i64)
}

/// i64.rotl: Rotate left i64 value.
/// Rotate amount is effectively modulo 64.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i64_rotl(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.rotate_left(rhs as u32))
}

/// i64.rotr: Rotate right i64 value.
/// Rotate amount is effectively modulo 64.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_sign_loss)]
pub fn i64_rotr(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.rotate_right(rhs as u32))
}

/// i64.clz: Count leading zeros in an i64 value.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap)]
pub fn i64_clz(val: i64) -> Result<i64> {
    Ok(i64::from(val.leading_zeros()))
}

/// i64.ctz: Count trailing zeros in an i64 value.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap)]
pub fn i64_ctz(val: i64) -> Result<i64> {
    Ok(i64::from(val.trailing_zeros()))
}

/// i64.popcnt: Count population (number of set bits) in an i64 value.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
#[allow(clippy::cast_possible_wrap)]
pub fn i64_popcnt(val: i64) -> Result<i64> {
    Ok(i64::from(val.count_ones()))
}

/// i64.eqz: Check if i64 value is zero. Returns 1 if zero, 0 otherwise.
///
/// # Errors
///
/// This function does not return an error.
#[inline]
pub fn i64_eqz(val: i64) -> Result<i32> {
    Ok(i32::from(val == 0))
}

// --- F32 Operations ---

/// f32.add: Add two f32 values.
/// Follows IEEE 754 arithmetic for `NaN` propagation, infinities, etc.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn f32_add(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() + rhs.value()))
}

/// f32.sub: Subtract two f32 values.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn f32_sub(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() - rhs.value()))
}

/// f32.mul: Multiply two f32 values.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn f32_mul(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() * rhs.value()))
}

/// f32.div: Divide two f32 values.
/// Division by zero yields +/-infinity or `NaN` as per IEEE 754.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn f32_div(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() / rhs.value()))
}

/// f32.abs: Absolute value of an f32.
/// Note: Wasm `abs` propagates `NaNs` differently from Rust's `f32::abs` in
/// some cases (e.g. sign bit of `NaN`). However, for non-NaN values, it's
/// straightforward. Wasm `abs` clears the sign bit. If the input is canonical
/// `NaN`, result is canonical `NaN`. If input is sNaN, result is canonical
/// `NaN`.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_abs(val: FloatBits32) -> Result<FloatBits32> {
    let f = val.value();
    if f.is_nan() {
        // Ensure canonical NaN representation for result
        // Wasm spec: "if z is a NaN, then return a canonical NaN"
        // For simplicity, FloatBits32::NAN should represent canonical NaN.
        Ok(FloatBits32::NAN)
    } else {
        // Clears the sign bit, equivalent to f.abs() for non-NaNs.
        // Rust's f32::abs preserves NaN payload but clears sign bit.
        // Wasm expects canonical NaN on NaN input.
        Ok(FloatBits32::from_float(f.abs()))
    }
}

/// f32.neg: Negate an f32 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_neg(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(-val.value()))
}

/// f32.copysign: Copy sign from one f32 to another.
/// result has magnitude of lhs, sign of rhs.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_copysign(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value().copysign(rhs.value())))
}

/// f32.ceil: Ceiling of an f32 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_ceil(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(f32_ceil_compat(val.value())))
}

/// f32.floor: Floor of an f32 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_floor(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(f32_floor_compat(val.value())))
}

/// f32.trunc: Truncate f32 value (to integer towards zero).
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_trunc(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_bits(f32_trunc_compat(val.value()).to_bits()))
}

/// f32.nearest: Round f32 to nearest integer (ties to even).
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_nearest(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(f32_round_ties_to_even_compat(val.value())))
}

/// f32.sqrt: Square root of an f32 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f32_sqrt(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(f32_sqrt_compat(val.value())))
}

/// f32.min: Minimum of two f32 values (WASM semantics).
/// If either operand is `NaN`, result is canonical `NaN`.
/// -0.0 is less than +0.0.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
#[allow(clippy::float_cmp)]
pub fn wasm_f32_min(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    let l = lhs.value();
    let r = rhs.value();
    if l.is_nan() || r.is_nan() {
        Ok(FloatBits32::NAN)
    } else if l == r && l == 0.0 {
        // Handle +0.0 vs -0.0
        // If l is -0.0 or r is -0.0 (or both), result should be -0.0
        // l.is_sign_negative() || r.is_sign_negative() implies one is -0.0 if they are
        // both 0.0
        if l.is_sign_negative() {
            Ok(lhs)
        } else {
            Ok(rhs)
        } // if l is -0, return it, else r (which could be -0 or +0)
    } else {
        Ok(FloatBits32::from_float(if l < r { l } else { r }))
    }
}

/// f32.max: Maximum of two f32 values (WASM semantics).
/// If either operand is `NaN`, result is canonical `NaN`.
/// +0.0 is greater than -0.0.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
#[allow(clippy::float_cmp)]
pub fn wasm_f32_max(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    let l = lhs.value();
    let r = rhs.value();
    if l.is_nan() || r.is_nan() {
        Ok(FloatBits32::NAN)
    } else if l == r && l == 0.0 {
        // Handle +0.0 vs -0.0
        // If l is +0.0 or r is +0.0 (or both), result should be +0.0
        if l.is_sign_positive() {
            Ok(lhs)
        } else {
            Ok(rhs)
        } // if l is +0, return it, else r (which could be +0 or -0)
    } else {
        Ok(FloatBits32::from_float(if l > r { l } else { r }))
    }
}

// --- F64 Operations ---

/// f64.nearest: Round to nearest integer, ties to even.
/// Follows IEEE 754-2008 `roundToIntegralTiesToEven`.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_nearest(val: FloatBits64) -> Result<FloatBits64> {
    let x = val.value();

    if x.is_nan() || x.is_infinite() || x == 0.0 {
        return Ok(val);
    }

    // Use the compatibility wrapper for round (ties to even)
    let rounded_val = f64_round_ties_to_even_compat(x);

    // The Wasm fN.nearest instruction directly corresponds to roundTiesToEven.
    // Our f64_round_ties_to_even_compat (whether std or polyfill) implements this.
    Ok(FloatBits64::from_float(rounded_val))
}

/// f64.abs: Absolute value of an f64.
/// Wasm `abs` clears the sign bit. If the input is canonical `NaN`, result is
/// canonical `NaN`. If input is sNaN, result is canonical `NaN`.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_abs(val: FloatBits64) -> Result<FloatBits64> {
    let d = val.value();
    if d.is_nan() {
        Ok(FloatBits64::NAN)
    } else {
        Ok(FloatBits64::from_float(d.abs()))
    }
}

/// f64.neg: Negate an f64 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_neg(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(-val.value()))
}

/// f64.copysign: Copy sign from one f64 to another.
/// result has magnitude of lhs, sign of rhs.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_copysign(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(lhs.value().copysign(rhs.value())))
}

/// f64.ceil: Ceiling of an f64 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_ceil(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(f64_ceil_compat(val.value())))
}

/// f64.floor: Floor of an f64 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_floor(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(f64_floor_compat(val.value())))
}

/// f64.trunc: Truncate f64 value (to integer towards zero).
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_trunc(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(f64_trunc_compat(val.value())))
}

/// f64.sqrt: Square root of an f64 value.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
pub fn wasm_f64_sqrt(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(f64_sqrt_compat(val.value())))
}

/// f64.min: Minimum of two f64 values (WASM semantics).
/// If either operand is `NaN`, result is canonical `NaN`.
/// -0.0 is less than +0.0.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
#[allow(clippy::float_cmp)]
pub fn wasm_f64_min(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    let l = lhs.value();
    let r = rhs.value();
    if l.is_nan() || r.is_nan() {
        Ok(FloatBits64::NAN)
    } else if l == r && l == 0.0 {
        if l.is_sign_negative() {
            Ok(lhs)
        } else {
            Ok(rhs)
        }
    } else {
        Ok(FloatBits64::from_float(if l < r { l } else { r }))
    }
}

/// f64.max: Maximum of two f64 values (WASM semantics).
/// If either operand is `NaN`, result is canonical `NaN`.
/// +0.0 is greater than -0.0.
///
/// # Errors
///
/// This function does not currently return an error.
#[inline]
#[allow(clippy::float_cmp)]
pub fn wasm_f64_max(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    let l = lhs.value();
    let r = rhs.value();
    if l.is_nan() || r.is_nan() {
        Ok(FloatBits64::NAN)
    } else if l == r && l == 0.0 {
        if l.is_sign_positive() {
            Ok(lhs)
        } else {
            Ok(rhs)
        }
    } else {
        Ok(FloatBits64::from_float(if l > r { l } else { r }))
    }
}

// --- Float Comparisons (all return i32: 0 for false, 1 for true) ---

// F32 Comparisons
#[inline]
/// WebAssembly f32.eq operation.
///
/// # Errors
///
/// This function does not currently return an error.
#[allow(clippy::float_cmp)]
pub fn f32_eq(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l == r))
}

#[inline]
/// WebAssembly f32.ne operation.
///
/// # Errors
///
/// This function does not currently return an error.
#[allow(clippy::float_cmp)]
pub fn f32_ne(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    let eq_val = i32::from(!l.is_nan() && !r.is_nan() && l == r);
    Ok(i32::from(eq_val == 0))
}

#[inline]
/// WebAssembly f32.lt operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f32_lt(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l < r))
}

#[inline]
/// WebAssembly f32.gt operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f32_gt(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l > r))
}

#[inline]
/// WebAssembly f32.le operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f32_le(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l <= r))
}

#[inline]
/// WebAssembly f32.ge operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f32_ge(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l >= r))
}

// F64 Comparisons
#[inline]
/// WebAssembly f64.eq operation.
///
/// # Errors
///
/// This function does not currently return an error.
#[allow(clippy::float_cmp)]
pub fn f64_eq(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l == r))
}

#[inline]
/// WebAssembly f64.ne operation.
///
/// # Errors
///
/// This function does not currently return an error.
#[allow(clippy::float_cmp)]
pub fn f64_ne(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    let eq_val = i32::from(!l.is_nan() && !r.is_nan() && l == r);
    Ok(i32::from(eq_val == 0))
}

#[inline]
/// WebAssembly f64.lt operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f64_lt(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l < r))
}

#[inline]
/// WebAssembly f64.gt operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f64_gt(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l > r))
}

#[inline]
/// WebAssembly f64.le operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f64_le(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l <= r))
}

#[inline]
/// WebAssembly f64.ge operation.
///
/// # Errors
///
/// This function does not currently return an error.
pub fn f64_ge(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(i32::from(!l.is_nan() && !r.is_nan() && l >= r))
}

// --- Saturating Float to Integer Conversions ---

#[inline]
#[must_use = "Saturating conversion result must be used"]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
/// Converts an f32 to i32 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i32_trunc_sat_f32_s(val: FloatBits32) -> i32 {
    let f = val.value();
    if f.is_nan() {
        0
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            i32::MAX
        } else {
            i32::MIN
        }
    } else {
        let trunc = f.trunc();
        if !(-2_147_483_648.0_f32..2_147_483_648.0_f32).contains(&trunc) {
            if f.is_sign_positive() {
                i32::MAX
            } else {
                i32::MIN
            }
        } else {
            trunc as i32
        }
    }
}

#[inline]
#[must_use = "Saturating conversion result must be used"]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
/// Converts an f32 to u32 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i32_trunc_sat_f32_u(val: FloatBits32) -> i32 {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return 0;
    }
    let f_trunc = f32_trunc_compat(f);
    if !(0.0..4_294_967_296.0_f32).contains(&f_trunc) {
        return i32::MAX;
    }
    f_trunc as u32 as i32
}

#[inline]
#[must_use = "Saturating conversion result must be used"]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
/// Converts an f32 to i64 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i64_trunc_sat_f32_s(val: FloatBits32) -> i64 {
    let f = val.value();
    if f.is_nan() {
        0
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            i64::MAX
        } else {
            i64::MIN
        }
    } else {
        let trunc = f.trunc();
        if !(-9_223_372_036_854_775_808.0_f32..9_223_372_036_854_775_808.0_f32).contains(&trunc) {
            if f.is_sign_positive() {
                i64::MAX
            } else {
                i64::MIN
            }
        } else {
            trunc as i64
        }
    }
}

#[inline]
#[must_use = "Saturating conversion result must be used"]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
/// Converts an f32 to u64 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i64_trunc_sat_f32_u(val: FloatBits32) -> i64 {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return 0;
    }
    let f_trunc = f32_trunc_compat(f);
    if !(0.0..18_446_744_073_709_551_616.0_f32).contains(&f_trunc) {
        return i64::MAX;
    }
    f_trunc as u64 as i64
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
/// Converts an f64 to i32 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i32_trunc_sat_f64_s(val: FloatBits64) -> i32 {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return 0;
    }
    let f_trunc = f64_trunc_compat(f);
    if !(-2_147_483_648.0_f64..2_147_483_648.0_f64).contains(&f_trunc) {
        // Saturate out-of-range values
        if f.is_sign_positive() {
            return i32::MAX;
        }
        // Else removed, this is the default for out-of-range negative floats
        return i32::MIN;
    }
    // In range
    f_trunc as i32
}

#[inline]
#[must_use = "Saturating conversion result must be used"]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
/// Converts an f64 to u32 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i32_trunc_sat_f64_u(val: FloatBits64) -> i32 {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return 0;
    }
    let f_trunc = f64_trunc_compat(f);
    if !(0.0..4_294_967_296.0_f64).contains(&f_trunc) {
        return i32::MAX;
    }
    f_trunc as u32 as i32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
/// Converts an f64 to i64 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i64_trunc_sat_f64_s(val: FloatBits64) -> i64 {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return 0;
    }
    let f_trunc = f64_trunc_compat(f);
    if !(-9_223_372_036_854_775_808.0_f64..9_223_372_036_854_775_808.0_f64).contains(&f_trunc) {
        // Saturate out-of-range values
        if f.is_sign_positive() {
            return i64::MAX;
        }
        // Else removed, this is the default for out-of-range negative floats
        return i64::MIN;
    }
    // In range
    f_trunc as i64
}

#[inline]
#[must_use = "Saturating conversion result must be used"]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
/// Converts an f64 to u64 using saturating truncation (Wasm semantics).
///
/// # Errors
///
/// This function does not return an error; out-of-range or NaN values saturate.
pub fn i64_trunc_sat_f64_u(val: FloatBits64) -> i64 {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return 0;
    }
    let f_trunc = f64_trunc_compat(f);
    if !(0.0..18_446_744_073_709_551_616.0_f64).contains(&f_trunc) {
        return i64::MAX;
    }
    f_trunc as u64 as i64
}

// --- Trapping Float to Integer Conversions ---

/// `i32.trunc_f32_s`: Truncate f32 to signed i32.
/// Traps if the input is `NaN`, Infinity, or out of `i32` range.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `i32` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i32_trunc_f32_s(val: FloatBits32) -> Result<i32> {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let f_trunc = f32_trunc_compat(f);
    if !(-2_147_483_648.0_f32..2_147_483_648.0_f32).contains(&f_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(f_trunc as i32)
}

/// `i32.trunc_f32_u`: Truncate f32 to unsigned i32 (u32), trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `u32` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i32_trunc_f32_u(val: FloatBits32) -> Result<u32> {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let f_trunc = f32_trunc_compat(f);
    if !(0.0..4_294_967_296.0_f32).contains(&f_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(f_trunc as u32)
}

/// `i32.trunc_f64_s`: Truncate f64 to signed i32, trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `i32` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i32_trunc_f64_s(val: FloatBits64) -> Result<i32> {
    let d = val.value();
    if d.is_nan() || d.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let d_trunc = f64_trunc_compat(d);
    if !(-2_147_483_648.0_f64..2_147_483_648.0_f64).contains(&d_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(d_trunc as i32)
}

/// `i32.trunc_f64_u`: Truncate f64 to unsigned i32 (u32), trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `u32` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i32_trunc_f64_u(val: FloatBits64) -> Result<u32> {
    let d = val.value();
    if d.is_nan() || d.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let d_trunc = f64_trunc_compat(d);
    if !(0.0..4_294_967_296.0_f64).contains(&d_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(d_trunc as u32)
}

/// `i64.trunc_f32_s`: Truncate f32 to signed i64, trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `i64` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i64_trunc_f32_s(val: FloatBits32) -> Result<i64> {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let f_trunc = f32_trunc_compat(f);
    if !(-9_223_372_036_854_775_808.0_f32..9_223_372_036_854_775_808.0_f32).contains(&f_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(f_trunc as i64)
}

/// `i64.trunc_f32_u`: Truncate f32 to unsigned i64 (u64), trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `u64` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i64_trunc_f32_u(val: FloatBits32) -> Result<u64> {
    let f = val.value();
    if f.is_nan() || f.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let f_trunc = f32_trunc_compat(f);
    if !(0.0..18_446_744_073_709_551_616.0_f32).contains(&f_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(f_trunc as u64)
}

/// `i64.trunc_f64_s`: Truncate f64 to signed i64, trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `i64` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i64_trunc_f64_s(val: FloatBits64) -> Result<i64> {
    let d = val.value();
    if d.is_nan() || d.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let d_trunc = f64_trunc_compat(d);
    if !(-9_223_372_036_854_775_808.0_f64..9_223_372_036_854_775_808.0_f64).contains(&d_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(d_trunc as i64)
}

/// `i64.trunc_f64_u`: Truncate f64 to unsigned i64 (u64), trapping on invalid input.
///
/// # Errors
/// - `Err(TrapCode::InvalidConversionToInteger)` if `val` is `NaN` or Infinity.
/// - `Err(TrapCode::IntegerOverflow)` if the truncated value is out of `u64` range.
#[inline]
#[allow(clippy::manual_range_contains)]
#[allow(clippy::cast_possible_truncation)]
pub fn i64_trunc_f64_u(val: FloatBits64) -> Result<u64> {
    let d = val.value();
    if d.is_nan() || d.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    let d_trunc = f64_trunc_compat(d);
    if !(0.0..18_446_744_073_709_551_616.0_f64).contains(&d_trunc) {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(d_trunc as u64)
}
