// WRT - wrt-types
// Module: WebAssembly Math Operations
// SW-REQ-ID: N/A (Will be defined by safety analysis for math functions if critical)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly Math Operations
//!
//! This module implements the executable logic for WebAssembly numeric instructions.
//! It operates on primitive Rust types and `FloatBits` wrappers, returning `Result`
//! with `TrapCode` for Wasm-defined trapping behavior.

// Allow dead code for now, as not all operations will be immediately wired up.
#![allow(dead_code)]

use crate::values::{FloatBits32, FloatBits64};
// TrapCode and WrtError are fundamental for Wasm math ops.
use wrt_error::{codes::TrapCode, Error as WrtError, Result};
// use num_traits::float::FloatCore; // Removed as it seems unused based on no_std test warnings

// --- Start of no_std math polyfills for trunc ---
#[cfg(not(feature = "std"))]
mod no_std_math_trunc {
    // REMOVE: use crate::values::{FloatBits32, FloatBits64};

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
    // use crate::values::FloatBits64; // Commented out as unused

    // Polyfill for f32::floor
    pub(super) fn floor_f32_polyfill(f_val: f32) -> f32 {
        if f_val.is_nan() || f_val.is_infinite() || f_val == 0.0 {
            return f_val;
        }
        let t = trunc_f32_polyfill(f_val); // trunc_f32_polyfill itself is now no_std compatible
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

        let fl = floor_f32_polyfill(f_val); // Uses already refactored floor
        let fr = f_val - fl;
        if fr < 0.5 {
            return fl;
        }
        if fr > 0.5 {
            return ceil_f32_polyfill(f_val);
        } // Uses already refactored ceil

        let ce = fl + 1.0;
        let half_fl = fl / 2.0;
        // Check if fl is even. For f32, trunc_f32_polyfill is used.
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
        let t = trunc_f64_polyfill(d_val); // trunc_f64_polyfill is now no_std compatible
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
        // CORRECTED: Use d_val for checks
        if d_val.is_nan() || d_val.is_infinite() || d_val == 0.0 {
            return d_val;
        }

        let fl = floor_f64_polyfill(d_val); // Uses refactored floor
        let fr = d_val - fl;

        if fr < 0.5 {
            return fl;
        }
        if fr > 0.5 {
            return ceil_f64_polyfill(d_val); // Uses refactored ceil
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
            y = f32::MIN_POSITIVE; // Use a tiny positive number to start if f_val * 0.5 underflowed
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

// Conditional wrapper for f32::trunc
#[inline(always)]
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

// Conditional wrapper for f64::trunc (not used yet, but for completeness)
#[inline(always)]
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

// Conditional wrapper for f64::round (ties to even)

// Manual signum for no_std, or std version
#[cfg(feature = "std")]
#[inline(always)]
fn f64_signum_compat(x: f64) -> f64 {
    x.signum()
}

#[cfg(not(feature = "std"))]
#[inline(always)]
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

// Helper for creating a trap error
#[inline]
fn trap(code: TrapCode) -> WrtError {
    code.into() // Relies on the From<TrapCode> for WrtError impl
}

// --- I32 Operations ---

/// i32.add: Add two i32 values.
/// Traps on signed overflow.
#[inline]
pub fn i32_add(lhs: i32, rhs: i32) -> Result<i32> {
    lhs.checked_add(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i32.sub: Subtract two i32 values.
/// Traps on signed overflow.
#[inline]
pub fn i32_sub(lhs: i32, rhs: i32) -> Result<i32> {
    lhs.checked_sub(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i32.mul: Multiply two i32 values.
/// Traps on signed overflow.
#[inline]
pub fn i32_mul(lhs: i32, rhs: i32) -> Result<i32> {
    lhs.checked_mul(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i32.div_s: Signed i32 division.
/// Traps on division by zero.
/// Traps on overflow (i32::MIN / -1).
#[inline]
pub fn i32_div_s(lhs: i32, rhs: i32) -> Result<i32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    // Specifically, `i32::MIN / -1` overflows because `-(i32::MIN)` is `i32::MAX + 1`.
    if lhs == i32::MIN && rhs == -1 {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    Ok(lhs.wrapping_div(rhs)) // Standard Rust division truncates towards zero, as per Wasm.
}

/// i32.div_u: Unsigned i32 division.
/// Traps on division by zero.
#[inline]
pub fn i32_div_u(lhs: u32, rhs: u32) -> Result<u32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    Ok(lhs / rhs) // Standard Rust unsigned division, as per Wasm.
}

/// i32.rem_s: Signed i32 remainder.
/// Traps on division by zero.
/// The result has the sign of the dividend.
#[inline]
pub fn i32_rem_s(lhs: i32, rhs: i32) -> Result<i32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    // `i32::MIN % -1` result is 0 in Rust, which is correct by Wasm spec.
    Ok(lhs.wrapping_rem(rhs))
}

/// i32.rem_u: Unsigned i32 remainder.
/// Traps on division by zero.
#[inline]
pub fn i32_rem_u(lhs: u32, rhs: u32) -> Result<u32> {
    if rhs == 0 {
        return Err(trap(TrapCode::IntegerDivideByZero));
    }
    Ok(lhs % rhs)
}

/// i32.and: Bitwise AND of two i32 values.
#[inline]
pub fn i32_and(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs & rhs)
}

/// i32.or: Bitwise OR of two i32 values.
#[inline]
pub fn i32_or(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs | rhs)
}

/// i32.xor: Bitwise XOR of two i32 values.
#[inline]
pub fn i32_xor(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs ^ rhs)
}

/// i32.shl: Shift left (logical) i32 value.
/// Shift amount is masked to 5 bits (0-31).
#[inline]
pub fn i32_shl(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.wrapping_shl(rhs as u32 & 0x1F))
}

/// i32.shr_s: Shift right (arithmetic) signed i32 value.
/// Shift amount is masked to 5 bits (0-31).
#[inline]
pub fn i32_shr_s(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.wrapping_shr(rhs as u32 & 0x1F))
}

/// i32.shr_u: Shift right (logical) unsigned i32 value (acting on i32 bits).
/// Shift amount is masked to 5 bits (0-31).
#[inline]
pub fn i32_shr_u(lhs: i32, rhs: i32) -> Result<i32> {
    // Perform as u32 to ensure logical shift, then cast back to i32.
    Ok((lhs as u32).wrapping_shr(rhs as u32 & 0x1F) as i32)
}

/// i32.rotl: Rotate left i32 value.
/// Rotate amount is effectively modulo 32.
#[inline]
pub fn i32_rotl(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.rotate_left(rhs as u32))
}

/// i32.rotr: Rotate right i32 value.
/// Rotate amount is effectively modulo 32.
#[inline]
pub fn i32_rotr(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(lhs.rotate_right(rhs as u32))
}

/// i32.clz: Count leading zeros in an i32 value.
#[inline]
pub fn i32_clz(val: i32) -> Result<i32> {
    Ok(val.leading_zeros() as i32)
}

/// i32.ctz: Count trailing zeros in an i32 value.
#[inline]
pub fn i32_ctz(val: i32) -> Result<i32> {
    Ok(val.trailing_zeros() as i32)
}

/// i32.popcnt: Count population (number of set bits) in an i32 value.
#[inline]
pub fn i32_popcnt(val: i32) -> Result<i32> {
    Ok(val.count_ones() as i32)
}

/// i32.eqz: Check if i32 value is zero. Returns 1 if zero, 0 otherwise.
#[inline]
pub fn i32_eqz(val: i32) -> Result<i32> {
    Ok(if val == 0 { 1 } else { 0 })
}

// --- I64 Operations ---

/// i64.add: Add two i64 values.
/// Traps on signed overflow.
#[inline]
pub fn i64_add(lhs: i64, rhs: i64) -> Result<i64> {
    lhs.checked_add(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i64.sub: Subtract two i64 values.
/// Traps on signed overflow.
#[inline]
pub fn i64_sub(lhs: i64, rhs: i64) -> Result<i64> {
    lhs.checked_sub(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i64.mul: Multiply two i64 values.
/// Traps on signed overflow.
#[inline]
pub fn i64_mul(lhs: i64, rhs: i64) -> Result<i64> {
    lhs.checked_mul(rhs).ok_or_else(|| trap(TrapCode::IntegerOverflow))
}

/// i64.div_s: Signed i64 division.
/// Traps on division by zero.
/// Traps on overflow (i64::MIN / -1).
#[inline]
pub fn i64_div_s(lhs: i64, rhs: i64) -> Result<i64> {
    if rhs == 0 {
        Err(trap(TrapCode::IntegerDivideByZero))
    } else if lhs == i64::MIN && rhs == -1 {
        Err(trap(TrapCode::IntegerOverflow))
    } else {
        Ok(lhs.wrapping_div(rhs)) // wrapping_div handles MIN / -1 by wrapping, but we already trapped.
    }
}

/// i64.div_u: Unsigned i64 division.
/// Traps on division by zero.
#[inline]
pub fn i64_div_u(lhs: u64, rhs: u64) -> Result<u64> {
    if rhs == 0 {
        Err(trap(TrapCode::IntegerDivideByZero))
    } else {
        Ok(lhs / rhs)
    }
}

/// i64.rem_s: Signed i64 remainder.
/// Traps on division by zero.
#[inline]
pub fn i64_rem_s(lhs: i64, rhs: i64) -> Result<i64> {
    if rhs == 0 {
        Err(trap(TrapCode::IntegerDivideByZero))
    } else {
        // For i64::MIN % -1, result is 0 in Rust's % operator, which is fine for WASM.
        Ok(lhs.wrapping_rem(rhs))
    }
}

/// i64.rem_u: Unsigned i64 remainder.
/// Traps on division by zero.
#[inline]
pub fn i64_rem_u(lhs: u64, rhs: u64) -> Result<u64> {
    if rhs == 0 {
        Err(trap(TrapCode::IntegerDivideByZero))
    } else {
        Ok(lhs % rhs)
    }
}

/// i64.and: Bitwise AND of two i64 values.
#[inline]
pub fn i64_and(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs & rhs)
}

/// i64.or: Bitwise OR of two i64 values.
#[inline]
pub fn i64_or(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs | rhs)
}

/// i64.xor: Bitwise XOR of two i64 values.
#[inline]
pub fn i64_xor(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs ^ rhs)
}

/// i64.shl: Shift left (logical) i64 value.
/// Shift amount is masked to 6 bits (0-63).
#[inline]
pub fn i64_shl(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.wrapping_shl(rhs as u32 & 0x3F))
}

/// i64.shr_s: Shift right (arithmetic) signed i64 value.
/// Shift amount is masked to 6 bits (0-63).
#[inline]
pub fn i64_shr_s(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.wrapping_shr(rhs as u32 & 0x3F))
}

/// i64.shr_u: Shift right (logical) unsigned i64 value (acting on i64 bits).
/// Shift amount is masked to 6 bits (0-63).
#[inline]
pub fn i64_shr_u(lhs: i64, rhs: i64) -> Result<i64> {
    Ok((lhs as u64).wrapping_shr(rhs as u32 & 0x3F) as i64)
}

/// i64.rotl: Rotate left i64 value.
/// Rotate amount is effectively modulo 64.
#[inline]
pub fn i64_rotl(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.rotate_left(rhs as u32))
}

/// i64.rotr: Rotate right i64 value.
/// Rotate amount is effectively modulo 64.
#[inline]
pub fn i64_rotr(lhs: i64, rhs: i64) -> Result<i64> {
    Ok(lhs.rotate_right(rhs as u32))
}

/// i64.clz: Count leading zeros in an i64 value.
#[inline]
pub fn i64_clz(val: i64) -> Result<i64> {
    Ok(val.leading_zeros() as i64)
}

/// i64.ctz: Count trailing zeros in an i64 value.
#[inline]
pub fn i64_ctz(val: i64) -> Result<i64> {
    Ok(val.trailing_zeros() as i64)
}

/// i64.popcnt: Count population (number of set bits) in an i64 value.
#[inline]
pub fn i64_popcnt(val: i64) -> Result<i64> {
    Ok(val.count_ones() as i64)
}

/// i64.eqz: Check if i64 value is zero. Returns 1 if zero, 0 otherwise.
#[inline]
pub fn i64_eqz(val: i64) -> Result<i32> {
    // Note: eqz result is i32 in WASM
    Ok(if val == 0 { 1 } else { 0 })
}

// --- F32 Operations ---

/// f32.add: Add two f32 values.
/// Follows IEEE 754 arithmetic for NaN propagation, infinities, etc.
#[inline]
pub fn f32_add(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() + rhs.value()))
}

/// f32.sub: Subtract two f32 values.
#[inline]
pub fn f32_sub(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() - rhs.value()))
}

/// f32.mul: Multiply two f32 values.
#[inline]
pub fn f32_mul(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() * rhs.value()))
}

/// f32.div: Divide two f32 values.
/// Division by zero yields +/-infinity or NaN as per IEEE 754.
#[inline]
pub fn f32_div(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(lhs.value() / rhs.value()))
}

/// f32.abs: Absolute value of an f32.
/// Note: Wasm `abs` propagates NaNs differently from Rust's `f32::abs` in some cases
/// (e.g. sign bit of NaN). However, for non-NaN values, it's straightforward.
/// Wasm `abs` clears the sign bit. If the input is canonical NaN, result is canonical NaN.
/// If input is sNaN, result is canonical NaN.
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
        Ok(FloatBits32::new(f.abs()))
    }
}

/// f32.neg: Negate an f32 value.
#[inline]
pub fn wasm_f32_neg(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(-val.value()))
}

/// f32.copysign: Copy sign from one f32 to another.
/// result has magnitude of lhs, sign of rhs.
#[inline]
pub fn wasm_f32_copysign(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(lhs.value().copysign(rhs.value())))
}

/// f32.ceil: Ceiling of an f32 value.
#[inline]
pub fn wasm_f32_ceil(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(f32_ceil_compat(val.value())))
}

/// f32.floor: Floor of an f32 value.
#[inline]
pub fn wasm_f32_floor(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(f32_floor_compat(val.value())))
}

/// f32.trunc: Truncate f32 value (to integer towards zero).
#[inline]
pub fn wasm_f32_trunc(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(f32_trunc_compat(val.value())))
}

/// f32.nearest: Round f32 to nearest integer (ties to even).
#[inline]
pub fn wasm_f32_nearest(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(f32_round_ties_to_even_compat(val.value())))
}

/// f32.sqrt: Square root of an f32 value.
#[inline]
pub fn wasm_f32_sqrt(val: FloatBits32) -> Result<FloatBits32> {
    Ok(FloatBits32::new(f32_sqrt_compat(val.value())))
}

/// f32.min: Minimum of two f32 values (WASM semantics).
/// If either operand is NaN, result is canonical NaN.
/// -0.0 is less than +0.0.
#[inline]
pub fn wasm_f32_min(lhs: FloatBits32, rhs: FloatBits32) -> Result<FloatBits32> {
    let l = lhs.value();
    let r = rhs.value();
    if l.is_nan() || r.is_nan() {
        Ok(FloatBits32::NAN)
    } else if l == r && l == 0.0 {
        // Handle +0.0 vs -0.0
        // If l is -0.0 or r is -0.0 (or both), result should be -0.0
        // l.is_sign_negative() || r.is_sign_negative() implies one is -0.0 if they are both 0.0
        if l.is_sign_negative() {
            Ok(lhs)
        } else {
            Ok(rhs)
        } // if l is -0, return it, else r (which could be -0 or +0)
    } else {
        Ok(FloatBits32::new(if l < r { l } else { r }))
    }
}

/// f32.max: Maximum of two f32 values (WASM semantics).
/// If either operand is NaN, result is canonical NaN.
/// +0.0 is greater than -0.0.
#[inline]
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
        Ok(FloatBits32::new(if l > r { l } else { r }))
    }
}

// --- F64 Operations ---

/// f64.nearest: Round to nearest integer, ties to even.
/// Follows IEEE 754-2008 `roundToIntegralTiesToEven`.
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
    Ok(FloatBits64::new(rounded_val))
}

/// f64.abs: Absolute value of an f64.
/// Wasm `abs` clears the sign bit. If the input is canonical NaN, result is canonical NaN.
/// If input is sNaN, result is canonical NaN.
#[inline]
pub fn wasm_f64_abs(val: FloatBits64) -> Result<FloatBits64> {
    let d = val.value();
    if d.is_nan() {
        Ok(FloatBits64::NAN)
    } else {
        Ok(FloatBits64::new(d.abs()))
    }
}

/// f64.neg: Negate an f64 value.
#[inline]
pub fn wasm_f64_neg(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::new(-val.value()))
}

/// f64.copysign: Copy sign from one f64 to another.
/// result has magnitude of lhs, sign of rhs.
#[inline]
pub fn wasm_f64_copysign(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::new(lhs.value().copysign(rhs.value())))
}

/// f64.ceil: Ceiling of an f64 value.
#[inline]
pub fn wasm_f64_ceil(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::new(f64_ceil_compat(val.value())))
}

/// f64.floor: Floor of an f64 value.
#[inline]
pub fn wasm_f64_floor(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::new(f64_floor_compat(val.value())))
}

/// f64.trunc: Truncate f64 value (to integer towards zero).
#[inline]
pub fn wasm_f64_trunc(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::new(f64_trunc_compat(val.value())))
}

/// f64.sqrt: Square root of an f64 value.
#[inline]
pub fn wasm_f64_sqrt(val: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::new(f64_sqrt_compat(val.value())))
}

/// f64.min: Minimum of two f64 values (WASM semantics).
/// If either operand is NaN, result is canonical NaN.
/// -0.0 is less than +0.0.
#[inline]
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
        Ok(FloatBits64::new(if l < r { l } else { r }))
    }
}

/// f64.max: Maximum of two f64 values (WASM semantics).
/// If either operand is NaN, result is canonical NaN.
/// +0.0 is greater than -0.0.
#[inline]
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
        Ok(FloatBits64::new(if l > r { l } else { r }))
    }
}

// --- Float Comparisons (all return i32: 0 for false, 1 for true) ---

// F32 Comparisons
#[inline]
pub fn f32_eq(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l == r { 1 } else { 0 })
}

#[inline]
pub fn f32_ne(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    // Note: WASM fne is !(fN.eq(z1,z2)).
    // If l or r is NaN, l != r is true in Rust, but WASM fne should be 1.
    // If !l.is_nan() && !r.is_nan() && l == r is 0, then ne is 1.
    // If either is NaN, !l.is_nan() && !r.is_nan() is false, so eq_val is 0, so ne is 1.
    let eq_val = if !l.is_nan() && !r.is_nan() && l == r { 1 } else { 0 };
    Ok(if eq_val == 0 { 1 } else { 0 })
}

#[inline]
pub fn f32_lt(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l < r { 1 } else { 0 })
}

#[inline]
pub fn f32_gt(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l > r { 1 } else { 0 })
}

#[inline]
pub fn f32_le(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l <= r { 1 } else { 0 })
}

#[inline]
pub fn f32_ge(lhs: FloatBits32, rhs: FloatBits32) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l >= r { 1 } else { 0 })
}

// F64 Comparisons
#[inline]
pub fn f64_eq(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l == r { 1 } else { 0 })
}

#[inline]
pub fn f64_ne(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    let eq_val = if !l.is_nan() && !r.is_nan() && l == r { 1 } else { 0 };
    Ok(if eq_val == 0 { 1 } else { 0 })
}

#[inline]
pub fn f64_lt(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l < r { 1 } else { 0 })
}

#[inline]
pub fn f64_gt(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l > r { 1 } else { 0 })
}

#[inline]
pub fn f64_le(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l <= r { 1 } else { 0 })
}

#[inline]
pub fn f64_ge(lhs: FloatBits64, rhs: FloatBits64) -> Result<i32> {
    let l = lhs.value();
    let r = rhs.value();
    Ok(if !l.is_nan() && !r.is_nan() && l >= r { 1 } else { 0 })
}

// --- Saturating Float to Integer Conversions ---

/// i32.trunc_sat_f32_s: Saturating Truncate f32 to signed i32.
/// Does not trap. NaN becomes 0. +/-Inf becomes i32::MIN/MAX.
/// Out of range values saturate.
#[inline]
pub fn i32_trunc_sat_f32_s(val: FloatBits32) -> Result<i32> {
    let f = val.value();

    if f.is_nan() {
        Ok(0)
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            Ok(i32::MAX)
        } else {
            Ok(i32::MIN)
        }
    } else {
        // Use f32_trunc_compat for potential no_std polyfill
        let truncated = f32_trunc_compat(f);
        if truncated >= (i32::MAX as f32) {
            // Check against f32 representation of MAX
            Ok(i32::MAX)
        } else if truncated <= (i32::MIN as f32) {
            // Check against f32 representation of MIN
            Ok(i32::MIN)
        } else {
            Ok(truncated as i32)
        }
    }
}

/// i32.trunc_sat_f32_u: Saturating Truncate f32 to unsigned u32.
/// Does not trap. NaN becomes 0. +/-Inf becomes u32::MIN/MAX.
/// Out of range values saturate.
#[inline]
pub fn i32_trunc_sat_f32_u(val: FloatBits32) -> Result<u32> {
    let f = val.value();

    if f.is_nan() {
        Ok(0)
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            Ok(u32::MAX)
        } else {
            // Negative infinity saturates to 0 for unsigned
            Ok(u32::MIN)
        }
    } else {
        let truncated = f32_trunc_compat(f);
        // Check against f32 representation of u32 range
        // Note: u32::MIN is 0.0. Any f < 0.0 saturates to 0.
        // Any f >= u32::MAX (as f32) saturates to u32::MAX.
        // u32::MAX as f32 is 4294967300.0f32 (approx 2^32)
        const U32_MAX_AS_F32: f32 = 4294967295.0_f32; // Actual u32::MAX as f32 if representable, or closest
                                                      // A more precise upper bound for f32 before it would definitely exceed u32::MAX when truncated
                                                      // is slightly less than (u32::MAX + 1) as f32.
                                                      // (u32::MAX as f32) for comparison is generally acceptable.

        if truncated >= U32_MAX_AS_F32 {
            // If truncated value is already >= u32::MAX as f32
            Ok(u32::MAX)
        } else if truncated <= 0.0 {
            // Includes negative numbers and those rounding to 0
            Ok(u32::MIN)
        } else {
            Ok(truncated as u32)
        }
    }
}

/// i64.trunc_sat_f32_s: Saturating Truncate f32 to signed i64.
#[inline]
pub fn i64_trunc_sat_f32_s(val: FloatBits32) -> Result<i64> {
    let f = val.value();

    if f.is_nan() {
        Ok(0)
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            Ok(i64::MAX)
        } else {
            Ok(i64::MIN)
        }
    } else {
        let truncated = f32_trunc_compat(f);
        // Compare against f32 representations of i64 limits.
        // Note: i64::MAX is approx 9.22e18. f32 can represent this magnitude.
        // However, precision is lost. (i64::MAX as f32) is an approximation.
        // Values of f very close to i64::MAX might round to (i64::MAX as f32)
        // when truncated, then casting to i64 might be slightly off if not careful.
        // The spec implies comparison of mathematical value of trunc(f) with i64 bounds.
        // Rust f32::to_int_unchecked has similar considerations.
        // For practical purposes, direct comparison with float-cast bounds is common.

        // Check if out of range for i64 represented as f32.
        // i64::MAX_F32 is the largest f32 <= i64::MAX.
        // i64::MIN_F32 is the smallest f32 >= i64::MIN.
        // These are effectively (i64::MAX as f32) and (i64::MIN as f32)
        // when the compiler does the const eval if these were consts.
        // Hardcoding avoids repeated casting issues / clarifies intent for large integers.

        const I64_MAX_AS_F32: f32 = 9223372036854775800.0_f32; // i64::MAX (9223372036854775807) as f32
        const I64_MIN_AS_F32: f32 = -9223372036854775800.0_f32; // i64::MIN (-9223372036854775808) as f32

        if truncated >= I64_MAX_AS_F32 {
            Ok(i64::MAX)
        } else if truncated <= I64_MIN_AS_F32 {
            Ok(i64::MIN)
        } else {
            Ok(truncated as i64) // Direct cast after range check
        }
    }
}

/// i64.trunc_sat_f32_u: Saturating Truncate f32 to unsigned u64.
#[inline]
pub fn i64_trunc_sat_f32_u(val: FloatBits32) -> Result<u64> {
    let f = val.value();

    if f.is_nan() {
        Ok(0)
    } else if f.is_infinite() {
        if f.is_sign_positive() {
            Ok(u64::MAX)
        } else {
            Ok(u64::MIN)
        }
    } else {
        let truncated = f32_trunc_compat(f);
        const U64_MAX_AS_F32: f32 = 18446744073709551600.0_f32; // u64::MAX (18446744073709551615) as f32

        if truncated >= U64_MAX_AS_F32 {
            Ok(u64::MAX)
        } else if truncated <= 0.0 {
            Ok(u64::MIN)
        } else {
            Ok(truncated as u64)
        }
    }
}

/// i32.trunc_sat_f64_s: Saturating Truncate f64 to signed i32.
#[inline]
pub fn i32_trunc_sat_f64_s(val: FloatBits64) -> Result<i32> {
    let d = val.value();

    if d.is_nan() {
        Ok(0)
    } else if d.is_infinite() {
        if d.is_sign_positive() {
            Ok(i32::MAX)
        } else {
            Ok(i32::MIN)
        }
    } else {
        let truncated = f64_trunc_compat(d);
        // f64 has enough precision for all i32 values.
        if truncated >= (i32::MAX as f64) {
            Ok(i32::MAX)
        } else if truncated <= (i32::MIN as f64) {
            Ok(i32::MIN)
        } else {
            Ok(truncated as i32)
        }
    }
}

/// i32.trunc_sat_f64_u: Saturating Truncate f64 to unsigned u32.
#[inline]
pub fn i32_trunc_sat_f64_u(val: FloatBits64) -> Result<u32> {
    let d = val.value();

    if d.is_nan() {
        Ok(0)
    } else if d.is_infinite() {
        if d.is_sign_positive() {
            Ok(u32::MAX)
        } else {
            Ok(u32::MIN)
        }
    } else {
        let truncated = f64_trunc_compat(d);
        if truncated >= (u32::MAX as f64) {
            Ok(u32::MAX)
        } else if truncated <= 0.0 {
            // u32::MIN is 0
            Ok(u32::MIN)
        } else {
            Ok(truncated as u32)
        }
    }
}

/// i64.trunc_sat_f64_s: Saturating Truncate f64 to signed i64.
#[inline]
pub fn i64_trunc_sat_f64_s(val: FloatBits64) -> Result<i64> {
    let d = val.value();

    if d.is_nan() {
        Ok(0)
    } else if d.is_infinite() {
        if d.is_sign_positive() {
            Ok(i64::MAX)
        } else {
            Ok(i64::MIN)
        }
    } else {
        let truncated = f64_trunc_compat(d);
        // f64 can represent integers up to 2^53 exactly. i64::MAX is ~2^63.
        // Comparisons still work for saturation.
        // (i64::MAX as f64) will be an approximation if i64::MAX > 2^53.
        // The actual i64::MAX is 9223372036854775807.
        // (i64::MAX as f64) is 9223372036854776000.0 on typical systems.
        const I64_MAX_AS_F64: f64 = 9223372036854775807.0_f64; // Exact if possible, or closest representable
        const I64_MIN_AS_F64: f64 = -9223372036854775808.0_f64;

        if truncated >= I64_MAX_AS_F64 {
            Ok(i64::MAX)
        } else if truncated <= I64_MIN_AS_F64 {
            Ok(i64::MIN)
        } else {
            Ok(truncated as i64)
        }
    }
}

/// i64.trunc_sat_f64_u: Saturating Truncate f64 to unsigned u64.
#[inline]
pub fn i64_trunc_sat_f64_u(val: FloatBits64) -> Result<u64> {
    let d = val.value();

    if d.is_nan() {
        Ok(0)
    } else if d.is_infinite() {
        if d.is_sign_positive() {
            Ok(u64::MAX)
        } else {
            Ok(u64::MIN)
        }
    } else {
        let truncated = f64_trunc_compat(d);
        // u64::MAX is 18446744073709551615.
        // (u64::MAX as f64) is 18446744073709552000.0 on typical systems.
        const U64_MAX_AS_F64: f64 = 18446744073709551615.0_f64;

        if truncated >= U64_MAX_AS_F64 {
            Ok(u64::MAX)
        } else if truncated <= 0.0 {
            Ok(u64::MIN)
        } else {
            Ok(truncated as u64)
        }
    }
}

// --- Conversions ---

/// i32.trunc_f32_s: Truncate (round towards zero) f32 to signed i32.
/// Traps if the input is NaN, Infinity, or out of i32 range.
#[inline]
pub fn i32_trunc_f32_s(val: FloatBits32) -> Result<i32> {
    let f = val.value();
    if f.is_nan() {
        // Wasm spec: NaN -> invalid_conversion_to_integer
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    if f.is_infinite() {
        // Wasm spec: +/-Inf -> invalid_conversion_to_integer
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }

    let f_trunc = f32_trunc_compat(f); // Use compatibility wrapper

    // Wasm spec:
    // If f_trunc is not representable as an i32 (i.e., it is less than −2^31 or greater than or equal to 2^31),
    // then the result is an integer_overflow trap.
    // −2^31 = -2147483648
    //  2^31 =  2147483648
    if f_trunc < -2147483648.0f32 || f_trunc >= 2147483648.0f32 {
        return Err(trap(TrapCode::IntegerOverflow));
    }
    // Otherwise, the result is f_trunc cast to i32.
    // Rust's `as i32` cast on a float performs truncation towards zero.
    // Since f_trunc is already the truncated float value and within range, this is correct.
    Ok(f_trunc as i32)
}

/// i32.trunc_f32_u: Truncate (round towards zero) f32 to unsigned u32.
/// Traps if the input is NaN, Infinity, or out of u32 range.
#[inline]
pub fn i32_trunc_f32_u(val: FloatBits32) -> Result<u32> {
    let f = val.value();
    // Temporary debug prints
    // eprint!("i32_trunc_f32_u: input f = {:?}, bits = {:#010x}", f, f.to_bits());

    if f.is_nan() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }
    if f.is_infinite() {
        return Err(trap(TrapCode::InvalidConversionToInteger));
    }

    let f_trunc = f32_trunc_compat(f); // Use compatibility wrapper
                                       // eprint!(", f_trunc = {:?}, bits = {:#010x}", f_trunc, f_trunc.to_bits());

    // Wasm spec:
    // If f_trunc is not representable as a u32 (i.e., it is less than 0 or greater than or equal to 2^32),
    // then the result is an integer_overflow trap.
    //  2^32 = 4294967296
    let condition = f_trunc < 0.0f32 || f_trunc >= 4294967296.0f32;
    // eprint!(", condition (f_trunc < 0 || f_trunc >= 2^32) = {}", condition);

    if condition {
        // eprintln!(", result: Err(IntegerOverflow)");
        return Err(trap(TrapCode::IntegerOverflow));
    }
    let result_val = f_trunc as u32;
    // eprintln!(", result: Ok({})", result_val);
    Ok(result_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::FloatBits32;
    use crate::values::FloatBits64;
    use wrt_error::{codes::TrapCode, ErrorCategory, ErrorSource};

    // Helper for tests
    fn f32_val(v: f32) -> FloatBits32 {
        FloatBits32::new(v)
    }
    fn f64_val(v: f64) -> FloatBits64 {
        FloatBits64::new(v)
    }

    #[test]
    fn test_i32_add() {
        assert_eq!(i32_add(1, 2).unwrap(), 3);
        assert_eq!(i32_add(i32::MAX, 0).unwrap(), i32::MAX);
        assert_eq!(i32_add(i32::MIN, 0).unwrap(), i32::MIN);
        let err1 = i32_add(i32::MAX, 1).unwrap_err();
        assert_eq!(err1.category(), ErrorCategory::RuntimeTrap);
        assert_eq!(err1.code(), TrapCode::IntegerOverflow as u16);
        let err2 = i32_add(i32::MIN, -1).unwrap_err();
        assert_eq!(err2.category(), ErrorCategory::RuntimeTrap);
        assert_eq!(err2.code(), TrapCode::IntegerOverflow as u16);
    }

    #[test]
    fn test_i32_div_s() {
        assert_eq!(i32_div_s(10, 2).unwrap(), 5);
        assert_eq!(i32_div_s(10, -2).unwrap(), -5);
        assert_eq!(i32_div_s(-10, 2).unwrap(), -5);
        assert_eq!(i32_div_s(-10, -2).unwrap(), 5);
        assert_eq!(i32_div_s(7, 3).unwrap(), 2);
        assert_eq!(i32_div_s(7, -3).unwrap(), -2);
        assert_eq!(i32_div_s(i32::MIN, 1).unwrap(), i32::MIN);
        let err1 = i32_div_s(i32::MIN, -1).unwrap_err();
        assert_eq!(err1.category(), ErrorCategory::RuntimeTrap);
        assert_eq!(err1.code(), TrapCode::IntegerOverflow as u16);
        let err2 = i32_div_s(10, 0).unwrap_err();
        assert_eq!(err2.category(), ErrorCategory::RuntimeTrap);
        assert_eq!(err2.code(), TrapCode::IntegerDivideByZero as u16);
    }

    #[test]
    fn test_f32_add() {
        assert_eq!(f32_add(f32_val(1.0), f32_val(2.0)).unwrap().value(), 3.0);
        assert!(f32_add(f32_val(f32::NAN), f32_val(1.0)).unwrap().value().is_nan());
        assert!(f32_add(f32_val(1.0), f32_val(f32::NAN)).unwrap().value().is_nan());
        assert_eq!(f32_add(f32_val(f32::INFINITY), f32_val(1.0)).unwrap().value(), f32::INFINITY);
        assert_eq!(
            f32_add(f32_val(f32::NEG_INFINITY), f32_val(1.0)).unwrap().value(),
            f32::NEG_INFINITY
        );
        assert!(f32_add(f32_val(f32::INFINITY), f32_val(f32::NEG_INFINITY))
            .unwrap()
            .value()
            .is_nan());
    }

    #[test]
    fn test_i32_trunc_f32_s_compat() {
        // Renamed to avoid clash if original exists
        assert_eq!(i32_trunc_f32_s(f32_val(3.7)).unwrap(), 3);
        assert_eq!(i32_trunc_f32_s(f32_val(-3.7)).unwrap(), -3);
        assert_eq!(i32_trunc_f32_s(f32_val(0.0)).unwrap(), 0);
        assert_eq!(i32_trunc_f32_s(f32_val(-0.0)).unwrap(), 0);
        assert_eq!(i32_trunc_f32_s(f32_val(2147483520.0f32)).unwrap(), 2147483520); // 2^31 - 2^16, fits
        assert_eq!(i32_trunc_f32_s(f32_val(-2147483520.0f32)).unwrap(), -2147483520);

        // Test NaN
        let nan_err = i32_trunc_f32_s(f32_val(f32::NAN)).unwrap_err();
        assert_eq!(nan_err.code(), TrapCode::InvalidConversionToInteger as u16);

        // Test Infinity
        let inf_err = i32_trunc_f32_s(f32_val(f32::INFINITY)).unwrap_err();
        assert_eq!(inf_err.code(), TrapCode::InvalidConversionToInteger as u16);
        let neg_inf_err = i32_trunc_f32_s(f32_val(f32::NEG_INFINITY)).unwrap_err();
        assert_eq!(neg_inf_err.code(), TrapCode::InvalidConversionToInteger as u16);

        // Test overflow (>= 2^31)
        let overflow_err_pos = i32_trunc_f32_s(f32_val(2147483648.0f32)).unwrap_err(); // 2^31
        assert_eq!(overflow_err_pos.code(), TrapCode::IntegerOverflow as u16);
        let overflow_err_pos_slightly_larger =
            i32_trunc_f32_s(f32_val(2147483904.0f32)).unwrap_err(); // 2^31 + 256
        assert_eq!(overflow_err_pos_slightly_larger.code(), TrapCode::IntegerOverflow as u16);

        // Test overflow (< -2^31)
        let overflow_err_neg = i32_trunc_f32_s(f32_val(-2147483904.0f32)).unwrap_err(); // -2^31 - 256 (approx)
        assert_eq!(overflow_err_neg.code(), TrapCode::IntegerOverflow as u16);

        // Test i32::MAX as f32 (which is 2147483648.0f32 due to f32 precision, so it should overflow)
        let val_max_as_f32 = i32::MAX as f32;
        let err_max = i32_trunc_f32_s(f32_val(val_max_as_f32)).unwrap_err();
        assert_eq!(err_max.code(), TrapCode::IntegerOverflow as u16);

        // Test exactly i32::MIN if representable and truncates correctly
        // -2147483648.0f32 should truncate to -2147483648
        assert_eq!(i32_trunc_f32_s(f32_val(-2147483648.0f32)).unwrap(), i32::MIN);
    }

    #[test]
    fn test_i32_trunc_f32_u_compat() {
        // Renamed
        assert_eq!(i32_trunc_f32_u(f32_val(3.7)).unwrap(), 3);
        assert_eq!(i32_trunc_f32_u(f32_val(0.0)).unwrap(), 0);
        assert_eq!(i32_trunc_f32_u(f32_val(4294967040.0f32)).unwrap(), 4294967040); // (2^32 - 2^16), fits

        // Test NaN
        let nan_err = i32_trunc_f32_u(f32_val(f32::NAN)).unwrap_err();
        assert_eq!(nan_err.code(), TrapCode::InvalidConversionToInteger as u16);

        // Test negative input (overflow for u32)
        let neg_err = i32_trunc_f32_u(f32_val(-0.1)).unwrap_err();
        assert_eq!(neg_err.code(), TrapCode::IntegerOverflow as u16);
        let neg_err_large = i32_trunc_f32_u(f32_val(-42.0)).unwrap_err();
        assert_eq!(neg_err_large.code(), TrapCode::IntegerOverflow as u16);

        // Test overflow (>= 2^32)
        let overflow_err = i32_trunc_f32_u(f32_val(4294967296.0f32)).unwrap_err(); // 2^32
        assert_eq!(overflow_err.code(), TrapCode::IntegerOverflow as u16);
        let overflow_err_slightly_larger = i32_trunc_f32_u(f32_val(4294967808.0f32)).unwrap_err(); // 2^32 + 512
        assert_eq!(overflow_err_slightly_larger.code(), TrapCode::IntegerOverflow as u16);

        // Test u32::MAX as f32 (which is 4294967296.0f32 due to f32 precision, so it should overflow)
        let val_umax_as_f32 = u32::MAX as f32;
        let err_umax = i32_trunc_f32_u(f32_val(val_umax_as_f32)).unwrap_err();
        assert_eq!(err_umax.code(), TrapCode::IntegerOverflow as u16);
    }

    #[test]
    fn test_wasm_f64_nearest_compat() {
        // Renamed if original exists
        // Basic cases
        assert_eq!(wasm_f64_nearest(f64_val(2.3)).unwrap().value(), 2.0);
        assert_eq!(wasm_f64_nearest(f64_val(2.7)).unwrap().value(), 3.0);
        assert_eq!(wasm_f64_nearest(f64_val(-2.3)).unwrap().value(), -2.0);
        assert_eq!(wasm_f64_nearest(f64_val(-2.7)).unwrap().value(), -3.0);

        // Ties to even
        assert_eq!(wasm_f64_nearest(f64_val(2.5)).unwrap().value(), 2.0); // Tie, 2 is even
        assert_eq!(wasm_f64_nearest(f64_val(3.5)).unwrap().value(), 4.0); // Tie, 4 is even
        assert_eq!(wasm_f64_nearest(f64_val(-2.5)).unwrap().value(), -2.0); // Tie, -2 is even
        assert_eq!(wasm_f64_nearest(f64_val(-3.5)).unwrap().value(), -4.0); // Tie, -4 is even

        // Zero, Inf, NaN
        assert_eq!(wasm_f64_nearest(f64_val(0.0)).unwrap().value().to_bits(), (0.0_f64).to_bits());
        assert_eq!(
            wasm_f64_nearest(f64_val(-0.0)).unwrap().value().to_bits(),
            (-0.0_f64).to_bits()
        );

        assert_eq!(wasm_f64_nearest(f64_val(f64::INFINITY)).unwrap().value(), f64::INFINITY);
        assert_eq!(
            wasm_f64_nearest(f64_val(f64::NEG_INFINITY)).unwrap().value(),
            f64::NEG_INFINITY
        );
        assert!(wasm_f64_nearest(f64_val(f64::NAN)).unwrap().value().is_nan());

        // Larger numbers
        assert_eq!(wasm_f64_nearest(f64_val(12345.5)).unwrap().value(), 12346.0);
        assert_eq!(wasm_f64_nearest(f64_val(12344.5)).unwrap().value(), 12344.0);
        assert_eq!(wasm_f64_nearest(f64_val(-12345.5)).unwrap().value(), -12346.0);
        assert_eq!(wasm_f64_nearest(f64_val(-12344.5)).unwrap().value(), -12344.0);
    }

    #[cfg(not(feature = "std"))]
    mod no_std_trunc_polyfill_tests {
        use super::super::no_std_math_trunc::*; // For direct access to polyfills

        #[test]
        fn test_trunc_f32_poly_detailed() {
            assert_eq!(trunc_f32_polyfill(0.0_f32).to_bits(), 0.0_f32.to_bits());
            assert_eq!(trunc_f32_polyfill(-0.0_f32).to_bits(), (-0.0_f32).to_bits());
            assert_eq!(trunc_f32_polyfill(1.0_f32), 1.0_f32);
            assert_eq!(trunc_f32_polyfill(-1.0_f32), -1.0_f32);
            assert_eq!(trunc_f32_polyfill(1.5_f32), 1.0_f32);
            assert_eq!(trunc_f32_polyfill(-1.5_f32), -1.0_f32);
            assert_eq!(trunc_f32_polyfill(0.7_f32), 0.0_f32);
            assert_eq!(trunc_f32_polyfill(-0.7_f32).to_bits(), (-0.0_f32).to_bits());

            assert_eq!(trunc_f32_polyfill(2.0_f32), 2.0_f32);
            assert_eq!(trunc_f32_polyfill(3.999_f32), 3.0_f32);
            assert_eq!(trunc_f32_polyfill(f32::MAX), f32::MAX);
            assert_eq!(trunc_f32_polyfill(f32::MIN), f32::MIN);

            assert_eq!(trunc_f32_polyfill(f32::MIN_POSITIVE), 0.0_f32);
            let x = f32::from_bits((127 << 23) | 1);
            assert_eq!(trunc_f32_polyfill(x), 1.0_f32);

            assert!(trunc_f32_polyfill(f32::NAN).is_nan());
            assert_eq!(trunc_f32_polyfill(f32::INFINITY), f32::INFINITY);
            assert_eq!(trunc_f32_polyfill(f32::NEG_INFINITY), f32::NEG_INFINITY);

            // Specific test for 2^32
            let val_2_pow_32 = f32::from_bits(0x4F800000); // 4294967296.0f32
            assert_eq!(
                trunc_f32_polyfill(val_2_pow_32).to_bits(),
                val_2_pow_32.to_bits(),
                "trunc_f32_polyfill(2^32) should be 2^32"
            );
        }

        #[test]
        fn test_trunc_f64_poly_detailed() {
            assert_eq!(trunc_f64_polyfill(0.0_f64).to_bits(), 0.0_f64.to_bits());
            // ... (rest of this test)
        }
    }
}

// --- Integer Conversions ---

/// i64.extend_i32_s: Extend signed i32 to i64.
#[inline]
pub fn i64_extend_i32_s(val: i32) -> Result<i64> {
    Ok(val as i64)
}

/// i64.extend_i32_u: Extend unsigned i32 to i64 (zero-extend).
#[inline]
pub fn i64_extend_i32_u(val: i32) -> Result<i64> {
    Ok(val as u32 as i64) // Cast to u32 first to ensure zero-extension semantics for the upper bits
}

/// i32.wrap_i64: Wrap i64 to i32 (truncate).
#[inline]
pub fn i32_wrap_i64(val: i64) -> Result<i32> {
    Ok(val as i32)
}

/// i32.extend8_s: Sign-extend 8-bit value to i32.
#[inline]
pub fn i32_extend8_s(val: i32) -> Result<i32> {
    Ok((val as i8) as i32)
}

/// i32.extend16_s: Sign-extend 16-bit value to i32.
#[inline]
pub fn i32_extend16_s(val: i32) -> Result<i32> {
    Ok((val as i16) as i32)
}

/// i64.extend8_s: Sign-extend 8-bit value to i64.
#[inline]
pub fn i64_extend8_s(val: i64) -> Result<i64> {
    Ok((val as i8) as i64)
}

/// i64.extend16_s: Sign-extend 16-bit value to i64.
#[inline]
pub fn i64_extend16_s(val: i64) -> Result<i64> {
    Ok((val as i16) as i64)
}

/// i64.extend32_s: Sign-extend 32-bit value to i64.
#[inline]
pub fn i64_extend32_s(val: i64) -> Result<i64> {
    Ok((val as i32) as i64)
}

// --- Integer Comparisons (all return i32: 0 for false, 1 for true) ---

// I32 Comparisons
#[inline]
pub fn i32_eq(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if lhs == rhs { 1 } else { 0 })
}
#[inline]
pub fn i32_ne(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if lhs != rhs { 1 } else { 0 })
}
#[inline]
pub fn i32_lt_s(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if lhs < rhs { 1 } else { 0 })
}
#[inline]
pub fn i32_lt_u(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if (lhs as u32) < (rhs as u32) { 1 } else { 0 })
}
#[inline]
pub fn i32_gt_s(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if lhs > rhs { 1 } else { 0 })
}
#[inline]
pub fn i32_gt_u(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if (lhs as u32) > (rhs as u32) { 1 } else { 0 })
}
#[inline]
pub fn i32_le_s(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if lhs <= rhs { 1 } else { 0 })
}
#[inline]
pub fn i32_le_u(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if (lhs as u32) <= (rhs as u32) { 1 } else { 0 })
}
#[inline]
pub fn i32_ge_s(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if lhs >= rhs { 1 } else { 0 })
}
#[inline]
pub fn i32_ge_u(lhs: i32, rhs: i32) -> Result<i32> {
    Ok(if (lhs as u32) >= (rhs as u32) { 1 } else { 0 })
}

// I64 Comparisons
#[inline]
pub fn i64_eq(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if lhs == rhs { 1 } else { 0 })
}
#[inline]
pub fn i64_ne(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if lhs != rhs { 1 } else { 0 })
}
#[inline]
pub fn i64_lt_s(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if lhs < rhs { 1 } else { 0 })
}
#[inline]
pub fn i64_lt_u(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if (lhs as u64) < (rhs as u64) { 1 } else { 0 })
}
#[inline]
pub fn i64_gt_s(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if lhs > rhs { 1 } else { 0 })
}
#[inline]
pub fn i64_gt_u(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if (lhs as u64) > (rhs as u64) { 1 } else { 0 })
}
#[inline]
pub fn i64_le_s(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if lhs <= rhs { 1 } else { 0 })
}
#[inline]
pub fn i64_le_u(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if (lhs as u64) <= (rhs as u64) { 1 } else { 0 })
}
#[inline]
pub fn i64_ge_s(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if lhs >= rhs { 1 } else { 0 })
}
#[inline]
pub fn i64_ge_u(lhs: i64, rhs: i64) -> Result<i32> {
    Ok(if (lhs as u64) >= (rhs as u64) { 1 } else { 0 })
}

// --- Add missing compat wrappers here ---

// Conditional wrapper for f32::ceil
#[inline(always)]
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

// Conditional wrapper for f64::ceil
#[inline(always)]
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

// Conditional wrapper for f32::floor
#[inline(always)]
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

// Conditional wrapper for f64::floor
#[inline(always)]
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

// Conditional wrapper for f32::round (ties to even for Wasm's nearest)
// Note: Rust's f32::round() rounds half away from zero. Wasm's nearest is ties to even.
#[inline(always)]
fn f32_round_ties_to_even_compat(f: f32) -> f32 {
    #[cfg(feature = "std")]
    {
        // Standard library f32::round is ties away from zero.
        // We need ties to even. A common way to implement this:
        let u = f.floor();
        let diff = f - u;
        if diff < 0.5 {
            u
        } else if diff > 0.5 {
            f.ceil()
        } else {
            // diff == 0.5
            if u % 2.0 == 0.0 {
                u
            } else {
                f.ceil()
            }
        }
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::round_ties_to_even_f32_polyfill(f)
    }
}

// Conditional wrapper for f64::round (ties to even for Wasm's nearest)
// Note: Rust's f64::round() rounds half away from zero. Wasm's nearest is ties to even.
#[inline(always)]
fn f64_round_ties_to_even_compat(d: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        // Standard library f64::round is ties away from zero.
        let u = d.floor();
        let diff = d - u;
        if diff < 0.5 {
            u
        } else if diff > 0.5 {
            d.ceil()
        } else {
            // diff == 0.5
            if u % 2.0 == 0.0 {
                u
            } else {
                d.ceil()
            }
        }
    }
    #[cfg(not(feature = "std"))]
    {
        no_std_math_rounding::round_ties_to_even_f64_polyfill(d)
    }
}

// Conditional wrapper for f32::sqrt
#[inline(always)]
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

// Conditional wrapper for f64::sqrt
#[inline(always)]
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

// --- End of added compat wrappers ---

// The existing f64_round_ties_to_even_compat function (was around line 255) can be removed if this replaces it,
// or this new one can be named differently if that one had a specific purpose.
// Assuming this replaces it for consistency with the wasm_fXX_nearest calls.

// fn f64_signum_compat(x: f64) -> f64 { // This was the next function in the original file
// ... existing code ...

// --- F64 Arithmetic Operations ---

/// WebAssembly `f64.add` operation.
/// Corresponds to: <https://www.w3.org/TR/wasm-core-2/#op-fadd>
#[inline]
pub fn f64_add(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(lhs.value() + rhs.value()))
}

/// WebAssembly `f64.sub` operation.
/// Corresponds to: <https://www.w3.org/TR/wasm-core-2/#op-fsub>
#[inline]
pub fn f64_sub(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(lhs.value() - rhs.value()))
}

/// WebAssembly `f64.mul` operation.
/// Corresponds to: <https://www.w3.org/TR/wasm-core-2/#op-fmul>
#[inline]
pub fn f64_mul(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(lhs.value() * rhs.value()))
}

/// WebAssembly `f64.div` operation.
/// Corresponds to: <https://www.w3.org/TR/wasm-core-2/#op-fdiv>
/// Division by zero results in infinity or NaN as per IEEE 754. No specific Wasm trap.
#[inline]
pub fn f64_div(lhs: FloatBits64, rhs: FloatBits64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(lhs.value() / rhs.value()))
}

// --- Conversion Operations ---

/// WebAssembly `f32.convert_i32_s` operation.
#[inline]
pub fn f32_convert_i32_s(val: i32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(val as f32))
}

/// WebAssembly `f32.convert_i32_u` operation.
#[inline]
pub fn f32_convert_i32_u(val: u32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(val as f32))
}

/// WebAssembly `f32.convert_i64_s` operation.
#[inline]
pub fn f32_convert_i64_s(val: i64) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(val as f32))
}

/// WebAssembly `f32.convert_i64_u` operation.
#[inline]
pub fn f32_convert_i64_u(val: u64) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(val as f32))
}

/// WebAssembly `f64.convert_i32_s` operation.
#[inline]
pub fn f64_convert_i32_s(val: i32) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(val as f64))
}

/// WebAssembly `f64.convert_i32_u` operation.
#[inline]
pub fn f64_convert_i32_u(val: u32) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(val as f64))
}

/// WebAssembly `f64.convert_i64_s` operation.
#[inline]
pub fn f64_convert_i64_s(val: i64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(val as f64))
}

/// WebAssembly `f64.convert_i64_u` operation.
#[inline]
pub fn f64_convert_i64_u(val: u64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(val as f64))
}

/// WebAssembly `f32.demote_f64` operation.
#[inline]
pub fn f32_demote_f64(val: FloatBits64) -> Result<FloatBits32> {
    Ok(FloatBits32::from_float(val.value() as f32))
}

/// WebAssembly `f64.promote_f32` operation.
#[inline]
pub fn f64_promote_f32(val: FloatBits32) -> Result<FloatBits64> {
    Ok(FloatBits64::from_float(val.value() as f64))
}

// --- Reinterpretation Operations ---

/// WebAssembly `i32.reinterpret_f32` operation.
#[inline]
pub fn i32_reinterpret_f32(val: FloatBits32) -> Result<i32> {
    Ok(val.to_bits() as i32) // f32 bits (u32) cast to i32
}

/// WebAssembly `i64.reinterpret_f64` operation.
#[inline]
pub fn i64_reinterpret_f64(val: FloatBits64) -> Result<i64> {
    Ok(val.to_bits() as i64) // f64 bits (u64) cast to i64
}

/// WebAssembly `f32.reinterpret_i32` operation.
#[inline]
pub fn f32_reinterpret_i32(val: i32) -> Result<FloatBits32> {
    Ok(FloatBits32::from_bits(val as u32)) // i32 bits cast to u32, then to f32 bits
}

/// WebAssembly `f64.reinterpret_i64` operation.
#[inline]
pub fn f64_reinterpret_i64(val: i64) -> Result<FloatBits64> {
    Ok(FloatBits64::from_bits(val as u64)) // i64 bits cast to u64, then to f64 bits
}
