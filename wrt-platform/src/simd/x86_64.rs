
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! x86_64 SIMD implementation using SSE2 and AVX2
//!
//! This module provides optimized SIMD implementations for x86_64 processors.
//! It supports SSE2 (baseline for x86_64) and AVX2 for enhanced performance.


use super::{SimdLevel, SimdProvider};

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

/// x86_64 SIMD provider with SSE2 and AVX2 support
#[derive(Debug, Clone)]
pub struct X86SimdProvider {
    level: SimdLevel,
    has_sse2: bool,
    has_avx2: bool,
}

impl X86SimdProvider {
    /// Create a new SSE2-only provider
    pub const fn new_sse2() -> Self {
        Self {
            level: SimdLevel::Basic,
            has_sse2: true,
            has_avx2: false,
        }
    }
    
    /// Create a new AVX2 provider (includes SSE2)
    pub const fn new_avx2() -> Self {
        Self {
            level: SimdLevel::Advanced,
            has_sse2: true,
            has_avx2: true,
        }
    }
}

impl SimdProvider for X86SimdProvider {
    fn simd_level(&self) -> SimdLevel {
        self.level
    }
    
    fn is_available(&self) -> bool {
        // Runtime check could be added here, but we trust the constructor
        self.has_sse2
    }
    
    // i8x16 operations
    fn v128_i8x16_add(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_add_epi8(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i8x16_sub(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_sub_epi8(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i8x16_neg(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let result = _mm_sub_epi8(zero, a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    // i16x8 operations
    fn v128_i16x8_add(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_add_epi16(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i16x8_sub(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_sub_epi16(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i16x8_mul(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_mullo_epi16(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i16x8_neg(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let result = _mm_sub_epi16(zero, a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    // i32x4 operations
    fn v128_i32x4_add(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_add_epi32(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i32x4_sub(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_sub_epi32(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i32x4_mul(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            // SSE2 doesn't have _mm_mullo_epi32, use SSE4.1 if available or emulate
            #[cfg(target_feature = "sse4.1")]
            let result = _mm_mullo_epi32(a_vec, b_vec);
            
            #[cfg(not(target_feature = "sse4.1"))]
            let result = {
                // Emulate with mul_epu32 and shuffles
                let a_even = _mm_shuffle_epi32(a_vec, 0xA0); // 0b10100000
                let b_even = _mm_shuffle_epi32(b_vec, 0xA0);
                let even_prod = _mm_mul_epu32(a_even, b_even);
                
                let a_odd = _mm_shuffle_epi32(a_vec, 0xF5); // 0b11110101
                let b_odd = _mm_shuffle_epi32(b_vec, 0xF5);
                let odd_prod = _mm_mul_epu32(a_odd, b_odd);
                
                let even_32 = _mm_shuffle_epi32(even_prod, 0x08); // 0b00001000
                let odd_32 = _mm_shuffle_epi32(odd_prod, 0x08);
                _mm_unpacklo_epi32(even_32, odd_32)
            };
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i32x4_neg(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let result = _mm_sub_epi32(zero, a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    // i64x2 operations
    fn v128_i64x2_add(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_add_epi64(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i64x2_sub(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_sub_epi64(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_i64x2_mul(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        // SSE2 doesn't have direct i64 multiply, need to emulate
        let mut result = [0u8; 16];
        
        // Extract i64 values
        let a0 = i64::from_le_bytes([a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]]);
        let a1 = i64::from_le_bytes([a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15]]);
        let b0 = i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
        let b1 = i64::from_le_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]);
        
        // Multiply
        let r0 = a0.wrapping_mul(b0);
        let r1 = a1.wrapping_mul(b1);
        
        // Store back
        result[0..8].copy_from_slice(&r0.to_le_bytes());
        result[8..16].copy_from_slice(&r1.to_le_bytes());
        
        result
    }
    
    fn v128_i64x2_neg(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let result = _mm_sub_epi64(zero, a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    // f32x4 operations
    fn v128_f32x4_add(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_ps(a.as_ptr() as *const f32);
            let b_vec = _mm_loadu_ps(b.as_ptr() as *const f32);
            let result = _mm_add_ps(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_ps(output.as_mut_ptr() as *mut f32, result);
            output
        }
    }
    
    fn v128_f32x4_sub(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_ps(a.as_ptr() as *const f32);
            let b_vec = _mm_loadu_ps(b.as_ptr() as *const f32);
            let result = _mm_sub_ps(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_ps(output.as_mut_ptr() as *mut f32, result);
            output
        }
    }
    
    fn v128_f32x4_mul(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_ps(a.as_ptr() as *const f32);
            let b_vec = _mm_loadu_ps(b.as_ptr() as *const f32);
            let result = _mm_mul_ps(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_ps(output.as_mut_ptr() as *mut f32, result);
            output
        }
    }
    
    fn v128_f32x4_div(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_ps(a.as_ptr() as *const f32);
            let b_vec = _mm_loadu_ps(b.as_ptr() as *const f32);
            let result = _mm_div_ps(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_ps(output.as_mut_ptr() as *mut f32, result);
            output
        }
    }
    
    fn v128_f32x4_neg(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_ps(a.as_ptr() as *const f32);
            // Negate by XORing with sign bit mask
            let sign_mask = _mm_set1_ps(-0.0);
            let result = _mm_xor_ps(a_vec, sign_mask);
            
            let mut output = [0u8; 16];
            _mm_storeu_ps(output.as_mut_ptr() as *mut f32, result);
            output
        }
    }
    
    fn v128_f32x4_sqrt(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_ps(a.as_ptr() as *const f32);
            let result = _mm_sqrt_ps(a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_ps(output.as_mut_ptr() as *mut f32, result);
            output
        }
    }
    
    // f64x2 operations
    fn v128_f64x2_add(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_pd(a.as_ptr() as *const f64);
            let b_vec = _mm_loadu_pd(b.as_ptr() as *const f64);
            let result = _mm_add_pd(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_pd(output.as_mut_ptr() as *mut f64, result);
            output
        }
    }
    
    fn v128_f64x2_sub(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_pd(a.as_ptr() as *const f64);
            let b_vec = _mm_loadu_pd(b.as_ptr() as *const f64);
            let result = _mm_sub_pd(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_pd(output.as_mut_ptr() as *mut f64, result);
            output
        }
    }
    
    fn v128_f64x2_mul(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_pd(a.as_ptr() as *const f64);
            let b_vec = _mm_loadu_pd(b.as_ptr() as *const f64);
            let result = _mm_mul_pd(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_pd(output.as_mut_ptr() as *mut f64, result);
            output
        }
    }
    
    fn v128_f64x2_div(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_pd(a.as_ptr() as *const f64);
            let b_vec = _mm_loadu_pd(b.as_ptr() as *const f64);
            let result = _mm_div_pd(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_pd(output.as_mut_ptr() as *mut f64, result);
            output
        }
    }
    
    fn v128_f64x2_neg(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_pd(a.as_ptr() as *const f64);
            // Negate by XORing with sign bit mask
            let sign_mask = _mm_set1_pd(-0.0);
            let result = _mm_xor_pd(a_vec, sign_mask);
            
            let mut output = [0u8; 16];
            _mm_storeu_pd(output.as_mut_ptr() as *mut f64, result);
            output
        }
    }
    
    fn v128_f64x2_sqrt(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_pd(a.as_ptr() as *const f64);
            let result = _mm_sqrt_pd(a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_pd(output.as_mut_ptr() as *mut f64, result);
            output
        }
    }
    
    // Bitwise operations
    fn v128_not(&self, a: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let all_ones = _mm_set1_epi32(-1);
            let result = _mm_xor_si128(a_vec, all_ones);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_and(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_and_si128(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_or(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_or_si128(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_xor(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let result = _mm_xor_si128(a_vec, b_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_andnot(&self, a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            // _mm_andnot_si128 computes NOT(a) AND b
            // We want a AND NOT(b), so swap arguments
            let result = _mm_andnot_si128(b_vec, a_vec);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    fn v128_bitselect(&self, a: &[u8; 16], b: &[u8; 16], c: &[u8; 16]) -> [u8; 16] {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let b_vec = _mm_loadu_si128(b.as_ptr() as *const __m128i);
            let c_vec = _mm_loadu_si128(c.as_ptr() as *const __m128i);
            
            // v128.bitselect: (a & c) | (b & ~c)
            let a_and_c = _mm_and_si128(a_vec, c_vec);
            let b_and_not_c = _mm_andnot_si128(c_vec, b_vec);
            let result = _mm_or_si128(a_and_c, b_and_not_c);
            
            let mut output = [0u8; 16];
            _mm_storeu_si128(output.as_mut_ptr() as *mut __m128i, result);
            output
        }
    }
    
    // Test operations
    fn v128_any_true(&self, a: &[u8; 16]) -> bool {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let cmp = _mm_cmpeq_epi8(a_vec, zero);
            // If all bytes are zero, movemask will be 0xFFFF
            let mask = _mm_movemask_epi8(cmp);
            mask != 0xFFFF
        }
    }
    
    fn v128_i8x16_all_true(&self, a: &[u8; 16]) -> bool {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let cmp = _mm_cmpeq_epi8(a_vec, zero);
            // If any byte is zero, movemask will have at least one bit set
            let mask = _mm_movemask_epi8(cmp);
            mask == 0
        }
    }
    
    fn v128_i16x8_all_true(&self, a: &[u8; 16]) -> bool {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let cmp = _mm_cmpeq_epi16(a_vec, zero);
            // Convert 16-bit comparison to byte mask
            let mask = _mm_movemask_epi8(cmp);
            // Each i16 produces 2 bits in the mask
            mask == 0
        }
    }
    
    fn v128_i32x4_all_true(&self, a: &[u8; 16]) -> bool {
        unsafe {
            let a_vec = _mm_loadu_si128(a.as_ptr() as *const __m128i);
            let zero = _mm_setzero_si128();
            let cmp = _mm_cmpeq_epi32(a_vec, zero);
            // Use floating-point movemask for 32-bit values
            let mask = _mm_movemask_ps(_mm_castsi128_ps(cmp));
            mask == 0
        }
    }
    
    fn v128_i64x2_all_true(&self, a: &[u8; 16]) -> bool {
        // SSE2 doesn't have _mm_cmpeq_epi64, check manually
        let a0 = i64::from_le_bytes([a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]]);
        let a1 = i64::from_le_bytes([a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15]]);
        a0 != 0 && a1 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_x86_simd_i32x4_add() {
        let provider = X86SimdProvider::new_sse2();
        let a = [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]; // [1, 2, 3, 4]
        let b = [5, 0, 0, 0, 6, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0]; // [5, 6, 7, 8]
        let result = provider.v128_i32x4_add(&a, &b);
        
        // Expected: [6, 8, 10, 12]
        assert_eq!(&result[0..4], &[6, 0, 0, 0]);
        assert_eq!(&result[4..8], &[8, 0, 0, 0]);
        assert_eq!(&result[8..12], &[10, 0, 0, 0]);
        assert_eq!(&result[12..16], &[12, 0, 0, 0]);
    }
    
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_x86_simd_f32x4_mul() {
        let provider = X86SimdProvider::new_sse2();
        
        // Create f32x4 vectors [2.0, 3.0, 4.0, 5.0] and [0.5, 2.0, 0.25, 0.2]
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        
        a[0..4].copy_from_slice(&2.0f32.to_bits().to_le_bytes());
        a[4..8].copy_from_slice(&3.0f32.to_bits().to_le_bytes());
        a[8..12].copy_from_slice(&4.0f32.to_bits().to_le_bytes());
        a[12..16].copy_from_slice(&5.0f32.to_bits().to_le_bytes());
        
        b[0..4].copy_from_slice(&0.5f32.to_bits().to_le_bytes());
        b[4..8].copy_from_slice(&2.0f32.to_bits().to_le_bytes());
        b[8..12].copy_from_slice(&0.25f32.to_bits().to_le_bytes());
        b[12..16].copy_from_slice(&0.2f32.to_bits().to_le_bytes());
        
        let result = provider.v128_f32x4_mul(&a, &b);
        
        // Extract results
        let r0 = f32::from_bits(u32::from_le_bytes([result[0], result[1], result[2], result[3]]));
        let r1 = f32::from_bits(u32::from_le_bytes([result[4], result[5], result[6], result[7]]));
        let r2 = f32::from_bits(u32::from_le_bytes([result[8], result[9], result[10], result[11]]));
        let r3 = f32::from_bits(u32::from_le_bytes([result[12], result[13], result[14], result[15]]));
        
        // Expected: [1.0, 6.0, 1.0, 1.0]
        assert_eq!(r0, 1.0);
        assert_eq!(r1, 6.0);
        assert_eq!(r2, 1.0);
        assert_eq!(r3, 1.0);
    }
    
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_x86_simd_bitwise_ops() {
        let provider = X86SimdProvider::new_sse2();
        
        let a = [0xFF; 16];
        let b = [0xAA; 16];
        
        // Test AND
        let and_result = provider.v128_and(&a, &b);
        assert_eq!(and_result, [0xAA; 16]);
        
        // Test OR
        let or_result = provider.v128_or(&[0x00; 16], &b);
        assert_eq!(or_result, [0xAA; 16]);
        
        // Test XOR
        let xor_result = provider.v128_xor(&a, &b);
        assert_eq!(xor_result, [0x55; 16]);
        
        // Test NOT
        let not_result = provider.v128_not(&a);
        assert_eq!(not_result, [0x00; 16]);
    }
    
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_x86_simd_any_true() {
        let provider = X86SimdProvider::new_sse2();
        
        assert!(!provider.v128_any_true(&[0x00; 16]));
        assert!(provider.v128_any_true(&[0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
                                         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
    }
}