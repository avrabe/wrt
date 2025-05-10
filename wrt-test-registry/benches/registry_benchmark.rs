use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wrt::load_module_from_binary;
use wrt_runtime::module::Module;
use wrt_test_registry::{TestCase, TestConfig, TestRegistry, TestResult, TestStats};
use wrt_types::bounded::BoundedVec;
use wrt_types::prelude::*;
use wrt_types::verification::VerificationLevel;

struct BenchmarkTestCase {
    name: &'static str,
    category: &'static str,
    requires_std: bool,
}

impl TestCase for BenchmarkTestCase {
    fn name(&self) -> &'static str {
        self.name
    }

    fn category(&self) -> &'static str {
        self.category
    }

    fn requires_std(&self) -> bool {
        self.requires_std
    }

    fn run(&self) -> TestResult {
        // Simple test that always passes
        Ok(())
    }

    fn description(&self) -> &'static str {
        "Benchmark test case"
    }
}

fn create_test_cases(count: usize) -> Vec<Box<dyn TestCase>> {
    let mut test_cases = Vec::with_capacity(count);

    for i in 0..count {
        let category = match i % 5 {
            0 => "decoder",
            1 => "runtime",
            2 => "component",
            3 => "memory",
            _ => "instructions",
        };

        test_cases.push(Box::new(BenchmarkTestCase {
            name: if i % 2 == 0 { "benchmark_even" } else { "benchmark_odd" },
            category,
            requires_std: i % 3 == 0,
        }));
    }

    test_cases
}

fn bench_registry_creation(c: &mut Criterion) {
    c.bench_function("registry_creation", |b| {
        b.iter(|| {
            let registry = black_box(TestRegistry::new());
            black_box(registry)
        });
    });
}

fn bench_test_registration(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_registration");

    for &count in &[10, 50, 100] {
        group.bench_function(format!("register_{}_tests", count), |b| {
            let test_cases = create_test_cases(count);

            b.iter(|| {
                let registry = TestRegistry::new();

                for test_case in &test_cases {
                    registry.register(test_case.clone()).expect("Failed to register test");
                }

                black_box(registry)
            });
        });
    }

    group.finish();
}

fn bench_test_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_execution");

    for &count in &[10, 50, 100] {
        group.bench_function(format!("run_{}_tests", count), |b| {
            let registry = TestRegistry::new();
            let test_cases = create_test_cases(count);

            for test_case in test_cases {
                registry.register(test_case).expect("Failed to register test");
            }

            b.iter(|| black_box(registry.run_all_tests()));
        });
    }

    group.finish();
}

fn bench_filtered_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtered_test_execution");

    let registry = TestRegistry::new();
    let test_cases = create_test_cases(100);

    for test_case in test_cases {
        registry.register(test_case).expect("Failed to register test");
    }

    group.bench_function("filter_by_category", |b| {
        b.iter(|| black_box(registry.run_filtered_tests(None, Some("runtime"), true)));
    });

    group.bench_function("filter_by_name", |b| {
        b.iter(|| black_box(registry.run_filtered_tests(Some("benchmark_even"), None, true)));
    });

    group.bench_function("filter_by_name_and_category", |b| {
        b.iter(|| {
            black_box(registry.run_filtered_tests(Some("benchmark_odd"), Some("component"), true))
        });
    });

    group.finish();
}

// Benchmark specific for runtime module creation and instantiation
fn bench_runtime_module(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_module");

    // Simple empty module for benchmarking
    let empty_module_bytes: &[u8] = &[
        0x00, 0x61, 0x73, 0x6D, // Magic number: \0asm
        0x01, 0x00, 0x00, 0x00, // Version: 1
    ];

    group.bench_function("module_creation", |b| {
        b.iter(|| black_box(wrt::new_module().expect("Failed to create module")));
    });

    group.bench_function("module_loading", |b| {
        b.iter(|| {
            black_box(load_module_from_binary(empty_module_bytes).expect("Failed to load module"))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_registry_creation,
    bench_test_registration,
    bench_test_execution,
    bench_filtered_execution,
    bench_runtime_module
);
criterion_main!(benches);
