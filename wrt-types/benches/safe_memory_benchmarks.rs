#![allow(missing_docs)] // Allow missing docs for benchmark harness code
//! Performance benchmarks for SafeMemory implementations
//!
//! These benchmarks measure the performance of SafeMemory operations
//! at different verification levels.

// SW-REQ-ID: REQ_PERF_001 // Placeholder, assuming performance benchmarks relate to this
//
// Copyright (c) 2024 The WRT Contributors
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wrt_types::safe_memory::SafeMemoryHandler;
use wrt_types::verification::VerificationLevel;

const CAPACITY: usize = 65536; // 64KiB
const CHUNK_SIZE: usize = 1024; // 1KiB
const NUM_CHUNKS: usize = 10;

fn safe_memory_store_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("SafeMemory Store");

    let data_to_store = vec![1u8; CHUNK_SIZE];

    for &level in &[VerificationLevel::None, VerificationLevel::Standard, VerificationLevel::Full] {
        group.bench_function(format!("verification_{:?}", level), |b| {
            b.iter(|| {
                let mut handler = SafeMemoryHandler::new(vec![0u8; CAPACITY]);
                handler.set_verification_level(level);
                for i in 0..NUM_CHUNKS {
                    let offset = i * CHUNK_SIZE;
                    if offset + CHUNK_SIZE <= CAPACITY {
                        black_box(handler.write_data(offset, black_box(&data_to_store))).unwrap();
                    }
                }
            })
        });
    }
    group.finish();
}

fn safe_memory_load_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("SafeMemory Load");

    let mut initial_data_vec = vec![0u8; CAPACITY];
    for i in 0..CAPACITY {
        initial_data_vec[i] = (i % 256) as u8;
    }

    for &level in &[VerificationLevel::None, VerificationLevel::Standard, VerificationLevel::Full] {
        let mut handler = SafeMemoryHandler::new(initial_data_vec.clone());
        handler.set_verification_level(level);

        group.bench_function(format!("verification_{:?}", level), |b| {
            b.iter(|| {
                let mut sum = 0u8;
                for i in 0..NUM_CHUNKS {
                    let offset = i * CHUNK_SIZE;
                    if offset + CHUNK_SIZE <= CAPACITY {
                        let safe_slice = handler.get_slice(offset, CHUNK_SIZE).unwrap();
                        let data_segment = safe_slice.data().unwrap();
                        for val in data_segment.iter() {
                            sum = sum.wrapping_add(*val);
                        }
                    }
                }
                black_box(sum);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, safe_memory_store_benchmark, safe_memory_load_benchmark);
criterion_main!(benches);
