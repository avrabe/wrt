//! CFI Performance Benchmarks for wrt-platform
//!
//! Benchmarks to measure the performance impact of CFI features
//! and validate that overhead is within acceptable limits.

use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use wrt_platform::{
    BranchTargetIdentification,
    BtiExceptionLevel,
    BtiMode,
    CfiExceptionMode,
    ControlFlowIntegrity,
    HardwareOptimization,
};

/// Benchmark BTI enable/disable operations
fn benchmark_bti_operations(c: &mut Criterion) {
    let bti = BranchTargetIdentification::new(BtiMode::Standard, BtiExceptionLevel::El1);

    c.bench_function("bti_enable", |b| {
        b.iter(|| {
            // Note: These operations may fail on systems without BTI support
            // The benchmark measures the time taken for the attempt
            let _ = black_box(bti.enable());
        });
    });

    c.bench_function("bti_disable", |b| {
        b.iter(|| {
            let _ = black_box(bti.disable());
        });
    });

    c.bench_function("bti_is_available", |b| {
        b.iter(|| {
            black_box(BranchTargetIdentification::is_available());
        });
    });
}

/// Benchmark RISC-V CFI operations
fn benchmark_riscv_cfi_operations(c: &mut Criterion) {
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous);

    c.bench_function("cfi_enable", |b| {
        b.iter(|| {
            // Note: These operations may fail on systems without CFI support
            let _ = black_box(cfi.enable());
        });
    });

    c.bench_function("cfi_disable", |b| {
        b.iter(|| {
            let _ = black_box(cfi.disable());
        });
    });

    c.bench_function("cfi_is_available", |b| {
        b.iter(|| {
            black_box(ControlFlowIntegrity::is_available());
        });
    });
}

/// Benchmark CFI configuration creation
fn benchmark_cfi_configuration(c: &mut Criterion) {
    c.bench_function("bti_creation", |b| {
        b.iter(|| {
            black_box(BranchTargetIdentification::new(
                BtiMode::Standard,
                BtiExceptionLevel::El1,
            ));
        });
    });

    c.bench_function("cfi_creation", |b| {
        b.iter(|| {
            black_box(ControlFlowIntegrity::new(CfiExceptionMode::Synchronous));
        });
    });

    c.bench_function("bti_all_modes", |b| {
        b.iter(|| {
            let modes = [
                BtiMode::Standard,
                BtiMode::CallOnly,
                BtiMode::JumpOnly,
                BtiMode::CallAndJump,
            ];

            for mode in modes {
                black_box(BranchTargetIdentification::new(
                    mode,
                    BtiExceptionLevel::El1,
                ));
            }
        });
    });
}

/// Benchmark hardware feature detection
fn benchmark_hardware_detection(c: &mut Criterion) {
    c.bench_function("all_cfi_detection", |b| {
        b.iter(|| {
            let bti_available = black_box(BranchTargetIdentification::is_available());
            let cfi_available = black_box(ControlFlowIntegrity::is_available());
            black_box((bti_available, cfi_available));
        });
    });

    c.bench_function("repeated_bti_detection", |b| {
        b.iter(|| {
            for _ in 0..100 {
                black_box(BranchTargetIdentification::is_available());
            }
        });
    });

    c.bench_function("repeated_cfi_detection", |b| {
        b.iter(|| {
            for _ in 0..100 {
                black_box(ControlFlowIntegrity::is_available());
            }
        });
    });
}

/// Benchmark CFI overhead estimation
fn benchmark_overhead_calculation(c: &mut Criterion) {
    let bti = BranchTargetIdentification::new(BtiMode::CallAndJump, BtiExceptionLevel::El1);
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous);

    c.bench_function("bti_overhead_calculation", |b| {
        b.iter(|| {
            black_box(bti.estimated_overhead_percentage());
        });
    });

    c.bench_function("cfi_overhead_calculation", |b| {
        b.iter(|| {
            black_box(cfi.estimated_overhead_percentage());
        });
    });

    c.bench_function("combined_overhead_calculation", |b| {
        b.iter(|| {
            let bti_overhead = black_box(bti.estimated_overhead_percentage());
            let cfi_overhead = black_box(cfi.estimated_overhead_percentage());
            black_box(bti_overhead + cfi_overhead);
        });
    });
}

/// Benchmark security level assessment
fn benchmark_security_assessment(c: &mut Criterion) {
    let bti = BranchTargetIdentification::new(BtiMode::CallAndJump, BtiExceptionLevel::El1);
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous);

    c.bench_function("bti_security_level", |b| {
        b.iter(|| {
            black_box(bti.security_level());
        });
    });

    c.bench_function("cfi_security_level", |b| {
        b.iter(|| {
            black_box(cfi.security_level());
        });
    });

    c.bench_function("security_comparison", |b| {
        b.iter(|| {
            let bti_level = black_box(bti.security_level());
            let cfi_level = black_box(cfi.security_level());
            black_box(bti_level as u8 + cfi_level as u8);
        });
    });
}

/// Benchmark description generation
fn benchmark_description_generation(c: &mut Criterion) {
    let bti = BranchTargetIdentification::new(BtiMode::Standard, BtiExceptionLevel::El1);
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous);

    c.bench_function("bti_description", |b| {
        b.iter(|| {
            black_box(bti.description());
        });
    });

    c.bench_function("cfi_description", |b| {
        b.iter(|| {
            black_box(cfi.description());
        });
    });
}

/// Simulate WebAssembly execution with and without CFI
fn benchmark_simulated_execution(c: &mut Criterion) {
    // Simulate a simple function call sequence
    fn simulate_function_calls(count: usize, with_cfi: bool) {
        for i in 0..count {
            if with_cfi {
                // Simulate CFI overhead (landing pad check, shadow stack push/pop)
                black_box(i * 2 + 1); // Simulate landing pad validation
                black_box(i ^ 0xdeadbeef); // Simulate shadow stack operation
            }

            // Simulate actual function call
            black_box(i + 42);
        }
    }

    c.bench_function("execution_without_cfi", |b| {
        b.iter(|| {
            simulate_function_calls(black_box(1000), false);
        });
    });

    c.bench_function("execution_with_cfi", |b| {
        b.iter(|| {
            simulate_function_calls(black_box(1000), true);
        });
    });

    // Measure overhead percentage
    c.bench_function("cfi_overhead_measurement", |b| {
        b.iter(|| {
            let start = std::time::Instant::now();
            simulate_function_calls(black_box(100), false);
            let baseline_time = start.elapsed().as_nanos();

            let start = std::time::Instant::now();
            simulate_function_calls(black_box(100), true);
            let cfi_time = start.elapsed().as_nanos();

            let overhead = if baseline_time > 0 {
                ((cfi_time - baseline_time) as f64 / baseline_time as f64) * 100.0
            } else {
                0.0
            };

            black_box(overhead);
        });
    });
}

/// Benchmark cross-platform CFI feature matrix
fn benchmark_feature_matrix(c: &mut Criterion) {
    c.bench_function("complete_cfi_assessment", |b| {
        b.iter(|| {
            // Comprehensive CFI capability assessment
            let mut features = Vec::new();

            if BranchTargetIdentification::is_available() {
                let bti =
                    BranchTargetIdentification::new(BtiMode::CallAndJump, BtiExceptionLevel::El1);
                features.push((
                    "ARM BTI",
                    bti.security_level(),
                    bti.estimated_overhead_percentage(),
                ));
            }

            if ControlFlowIntegrity::is_available() {
                let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous);
                features.push((
                    "RISC-V CFI",
                    cfi.security_level(),
                    cfi.estimated_overhead_percentage(),
                ));
            }

            black_box(features);
        });
    });
}

criterion_group!(
    cfi_benchmarks,
    benchmark_bti_operations,
    benchmark_riscv_cfi_operations,
    benchmark_cfi_configuration,
    benchmark_hardware_detection,
    benchmark_overhead_calculation,
    benchmark_security_assessment,
    benchmark_description_generation,
    benchmark_simulated_execution,
    benchmark_feature_matrix
);

criterion_main!(cfi_benchmarks);
