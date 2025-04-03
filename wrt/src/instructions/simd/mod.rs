//! SIMD instruction implementations

// Add submodule declarations and re-exports
mod f32x4;
pub use f32x4::*;
mod f64x2;
pub use f64x2::*;
mod i8x16;
pub use i8x16::*;
mod i16x8;
pub use i16x8::*;
mod i32x4;
pub use i32x4::*;
mod i64x2;
pub use i64x2::*;

use crate::{
    behavior::{FrameBehavior /* Stack */},
    error::{Error, Result},
    stack::Stack, // Import directly
    values::Value,
    StacklessEngine, // Import
};

pub fn v128_const(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    val: [u8; 16],
) -> Result<()> {
    stack.push(Value::V128(val))
}

pub fn i8x16_shuffle(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lanes: [u8; 16],
) -> Result<()> {
    let b = stack.pop()?.as_v128()?;
    let a = stack.pop()?.as_v128()?;
    let mut result = [0u8; 16];
    for i in 0..16 {
        let lane_idx = lanes[i];
        result[i] = if lane_idx < 16 {
            a[lane_idx as usize]
        } else if lane_idx < 32 {
            b[(lane_idx - 16) as usize]
        } else {
            // Indices >= 32 are invalid according to spec, result is implementation defined.
            // Let's return 0 for simplicity/determinism.
            0
        };
    }
    stack.push(Value::V128(result))
}

macro_rules! i8x16_extract_lane {
    ($name:ident, $signedness:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
            lane_idx: u8,
        ) -> Result<()> {
            let v = stack.pop()?.as_v128()?;
            let val = v[lane_idx as usize];
            let result = if stringify!($signedness) == "signed" {
                (val as i8) as i32
            } else {
                val as i32
            };
            stack.push(Value::I32(result))
        }
    };
}

i8x16_extract_lane!(i8x16_extract_lane_s, signed);
i8x16_extract_lane!(i8x16_extract_lane_u, unsigned);

pub fn i8x16_replace_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    let mut v = stack.pop()?.as_v128()?;
    v[lane_idx as usize] = val as u8;
    stack.push(Value::V128(v))
}

macro_rules! i16x8_extract_lane {
    ($name:ident, $signedness:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
            lane_idx: u8,
        ) -> Result<()> {
            let v = stack.pop()?.as_v128()?;
            let start = lane_idx as usize * 2;
            let bytes: [u8; 2] = v[start..start + 2].try_into().unwrap();
            let val = if stringify!($signedness) == "signed" {
                i16::from_le_bytes(bytes) as i32
            } else {
                u16::from_le_bytes(bytes) as i32
            };
            stack.push(Value::I32(val))
        }
    };
}

i16x8_extract_lane!(i16x8_extract_lane_s, signed);
i16x8_extract_lane!(i16x8_extract_lane_u, unsigned);

pub fn i16x8_replace_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    let mut v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 2;
    v[start..start + 2].copy_from_slice(&(val as i16).to_le_bytes());
    stack.push(Value::V128(v))
}

pub fn i32x4_extract_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 4;
    let bytes: [u8; 4] = v[start..start + 4].try_into().unwrap();
    let val = i32::from_le_bytes(bytes);
    stack.push(Value::I32(val))
}

pub fn i32x4_replace_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let val = stack.pop()?.as_i32()?;
    let mut v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 4;
    v[start..start + 4].copy_from_slice(&val.to_le_bytes());
    stack.push(Value::V128(v))
}

pub fn i64x2_extract_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 8;
    let bytes: [u8; 8] = v[start..start + 8].try_into().unwrap();
    let val = i64::from_le_bytes(bytes);
    stack.push(Value::I64(val))
}

pub fn i64x2_replace_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let val = stack.pop()?.as_i64()?;
    let mut v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 8;
    v[start..start + 8].copy_from_slice(&val.to_le_bytes());
    stack.push(Value::V128(v))
}

pub fn f32x4_extract_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 4;
    let bytes: [u8; 4] = v[start..start + 4].try_into().unwrap();
    let val = f32::from_le_bytes(bytes);
    stack.push(Value::F32(val))
}

pub fn f32x4_replace_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let val = stack.pop()?.as_f32()?;
    let mut v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 4;
    v[start..start + 4].copy_from_slice(&val.to_le_bytes());
    stack.push(Value::V128(v))
}

pub fn f64x2_extract_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 8;
    let bytes: [u8; 8] = v[start..start + 8].try_into().unwrap();
    let val = f64::from_le_bytes(bytes);
    stack.push(Value::F64(val))
}

pub fn f64x2_replace_lane(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
    lane_idx: u8,
) -> Result<()> {
    let val = stack.pop()?.as_f64()?;
    let mut v = stack.pop()?.as_v128()?;
    let start = lane_idx as usize * 8;
    v[start..start + 8].copy_from_slice(&val.to_le_bytes());
    stack.push(Value::V128(v))
}

// Unary ops
macro_rules! v128_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..16 {
                result[i] = a[i].$op();
            }
            stack.push(Value::V128(result))
        }
    };
}

macro_rules! i8x16_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a_v = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..16 {
                result[i] = (a_v[i] as i8).$op() as u8;
            }
            stack.push(Value::V128(result))
        }
    };
}

macro_rules! i16x8_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a_v = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..8 {
                let start = i * 2;
                let a = i16::from_le_bytes(a_v[start..start + 2].try_into().unwrap());
                let r = a.$op();
                result[start..start + 2].copy_from_slice(&r.to_le_bytes());
            }
            stack.push(Value::V128(result))
        }
    };
}

macro_rules! i32x4_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a_v = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..4 {
                let start = i * 4;
                let a = i32::from_le_bytes(a_v[start..start + 4].try_into().unwrap());
                let r = a.$op();
                result[start..start + 4].copy_from_slice(&r.to_le_bytes());
            }
            stack.push(Value::V128(result))
        }
    };
}

macro_rules! i64x2_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a_v = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..2 {
                let start = i * 8;
                let a = i64::from_le_bytes(a_v[start..start + 8].try_into().unwrap());
                let r = a.$op();
                result[start..start + 8].copy_from_slice(&r.to_le_bytes());
            }
            stack.push(Value::V128(result))
        }
    };
}

macro_rules! f32x4_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a_v = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..4 {
                let start = i * 4;
                let a = f32::from_le_bytes(a_v[start..start + 4].try_into().unwrap());
                let r = a.$op();
                result[start..start + 4].copy_from_slice(&r.to_le_bytes());
            }
            stack.push(Value::V128(result))
        }
    };
}

macro_rules! f64x2_unary_op {
    ($name:ident, $op:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let a_v = stack.pop()?.as_v128()?;
            let mut result = [0u8; 16];
            for i in 0..2 {
                let start = i * 8;
                let a = f64::from_le_bytes(a_v[start..start + 8].try_into().unwrap());
                let r = a.$op();
                result[start..start + 8].copy_from_slice(&r.to_le_bytes());
            }
            stack.push(Value::V128(result))
        }
    };
}

pub fn v128_not(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a = stack.pop()?.as_v128()?;
    let mut result = [0u8; 16];
    for i in 0..16 {
        result[i] = !a[i];
    }
    stack.push(Value::V128(result))
}

// Population count
pub fn i8x16_popcnt(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a_v = stack.pop()?.as_v128()?;
    let mut result = [0u8; 16];
    for i in 0..16 {
        result[i] = a_v[i].count_ones() as u8;
    }
    stack.push(Value::V128(result))
}

// Negation
i8x16_unary_op!(i8x16_neg, neg);
i16x8_unary_op!(i16x8_neg, neg);
i32x4_unary_op!(i32x4_neg, neg);
i64x2_unary_op!(i64x2_neg, neg);

// Absolute value
f32x4_unary_op!(f32x4_abs, abs);
f64x2_unary_op!(f64x2_abs, abs);

// Negation for floats
f32x4_unary_op!(f32x4_neg, neg);
f64x2_unary_op!(f64x2_neg, neg);

// Square root
f32x4_unary_op!(f32x4_sqrt, sqrt);
f64x2_unary_op!(f64x2_sqrt, sqrt);

// Rounding
f32x4_unary_op!(f32x4_ceil, ceil);
f32x4_unary_op!(f32x4_floor, floor);
f32x4_unary_op!(f32x4_trunc, trunc);
// f32x4_nearest needs special handling due to Banker's rounding - cannot use simple macro
pub fn f32x4_nearest(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a_v = stack.pop()?.as_v128()?;
    let mut result = [0u8; 16];
    for i in 0..4 {
        let start = i * 4;
        let a = f32::from_le_bytes(a_v[start..start + 4].try_into().unwrap());
        let r = a.nearest_integer(); // Use the helper for correct rounding
        result[start..start + 4].copy_from_slice(&r.to_le_bytes());
    }
    stack.push(Value::V128(result))
}

f64x2_unary_op!(f64x2_ceil, ceil);
f64x2_unary_op!(f64x2_floor, floor);
f64x2_unary_op!(f64x2_trunc, trunc);
// f64x2_nearest needs special handling
pub fn f64x2_nearest(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let a_v = stack.pop()?.as_v128()?;
    let mut result = [0u8; 16];
    for i in 0..2 {
        let start = i * 8;
        let a = f64::from_le_bytes(a_v[start..start + 8].try_into().unwrap());
        let r = a.nearest_integer(); // Use the helper for correct rounding
        result[start..start + 8].copy_from_slice(&r.to_le_bytes());
    }
    stack.push(Value::V128(result))
}

// Bitmask extraction
macro_rules! v128_bitmask {
    ($name:ident, $ty:ty, $lanes:expr) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let v = stack.pop()?.as_v128()?;
            let mut result: i32 = 0;
            let lane_bytes = 16 / $lanes;
            for i in 0..$lanes {
                let start = i * lane_bytes;
                let lane_val =
                    <$ty>::from_le_bytes(v[start..start + lane_bytes].try_into().unwrap());
                if lane_val < 0 {
                    // Check sign bit
                    result |= 1 << i;
                }
            }
            stack.push(Value::I32(result))
        }
    };
}

v128_bitmask!(i8x16_bitmask, i8, 16);
v128_bitmask!(i16x8_bitmask, i16, 8);
v128_bitmask!(i32x4_bitmask, i32, 4);
v128_bitmask!(i64x2_bitmask, i64, 2);

// Splat
macro_rules! v128_splat {
    ($name:ident, $ty:ty, $lanes:expr, $val_method:ident) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let val = stack.pop()?.$val_method()?;
            let mut result = [0u8; 16];
            let lane_bytes = 16 / $lanes;
            let val_bytes = val.to_le_bytes();
            for i in 0..$lanes {
                let start = i * lane_bytes;
                result[start..start + lane_bytes].copy_from_slice(&val_bytes);
            }
            stack.push(Value::V128(result))
        }
    };
}

v128_splat!(i8x16_splat, i8, 16, as_i32);
v128_splat!(i16x8_splat, i16, 8, as_i32);
v128_splat!(i32x4_splat, i32, 4, as_i32);
v128_splat!(i64x2_splat, i64, 2, as_i64);
v128_splat!(f32x4_splat, f32, 4, as_f32);
v128_splat!(f64x2_splat, f64, 2, as_f64);

// Any/All True
pub fn v128_any_true(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let v = stack.pop()?.as_v128()?;
    let any_true = v.iter().any(|&byte| byte != 0);
    stack.push(Value::I32(any_true as i32))
}

macro_rules! v128_all_true {
    ($name:ident, $lanes:expr) => {
        pub fn $name(
            stack: &mut dyn Stack,
            frame: &mut dyn FrameBehavior,
            _engine: &StacklessEngine,
        ) -> Result<()> {
            let v = stack.pop()?.as_v128()?;
            let lane_bytes = 16 / $lanes;
            let all_true = (0..$lanes).all(|i| {
                let start = i * lane_bytes;
                let lane_slice = &v[start..start + lane_bytes];
                // Check if the most significant bit is set (for signed comparison)
                // This assumes the input is a result of a comparison where non-zero means true.
                // A more robust check might depend on the specific comparison that produced the mask.
                // For simplicity, let's check if *any* bit is set in the lane, representing true.
                lane_slice.iter().any(|&byte| byte != 0)
            });
            stack.push(Value::I32(all_true as i32))
        }
    };
}

v128_all_true!(i8x16_all_true, 16);
v128_all_true!(i16x8_all_true, 8);
v128_all_true!(i32x4_all_true, 4);
v128_all_true!(i64x2_all_true, 2);

// Bitselect
pub fn v128_bitselect(
    stack: &mut dyn Stack,
    frame: &mut dyn FrameBehavior,
    _engine: &StacklessEngine,
) -> Result<()> {
    let c = stack.pop()?.as_v128()?;
    let v2 = stack.pop()?.as_v128()?;
    let v1 = stack.pop()?.as_v128()?;
    let mut result = [0u8; 16];
    for i in 0..16 {
        result[i] = (v1[i] & c[i]) | (v2[i] & !c[i]);
    }
    stack.push(Value::V128(result))
}
