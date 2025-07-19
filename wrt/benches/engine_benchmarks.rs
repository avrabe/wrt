use std::{
    fs,
    sync::Arc,
};

use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
// Import the current execution components
use wrt_decoder::decoder::decode_module;
use wrt_error::Result;
use wrt_foundation::values::Value;
use wrt_runtime::{
    module::Module,
    module_instance::ModuleInstance,
    stackless::StacklessEngine,
};

/// Helper function to load the real test WASM module
fn load_test_module() -> Result<Module> {
    let wasm_bytes = fs::read("test_add.wasm")
        .map_err(|_| wrt_error::Error::system_io_error("Failed to read test_add.wasm"))?;

    let decoded = decode_module(&wasm_bytes)?;
    Module::from_wrt_module(&decoded)
}

/// Helper function to create test arguments
fn create_test_args(a: i32, b: i32) -> Vec<Value> {
    vec![Value::I32(a), Value::I32(b)]
}

fn benchmark_module_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_module_loading";

    if let Ok(wasm_bytes) = fs::read("test_add.wasm") {
        group.bench_function("decode_module", |b| {
            b.iter(|| {
                let decoded = decode_module(black_box(&wasm_bytes)).unwrap();
                black_box(decoded)
            };
        };

        group.bench_function("convert_to_runtime", |b| {
            let decoded = decode_module(&wasm_bytes).unwrap();
            b.iter(|| {
                let runtime_module = Module::from_wrt_module(black_box(&decoded)).unwrap();
                black_box(runtime_module)
            };
        };
    }

    group.finish);
}

fn benchmark_engine_instantiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_instantiation";

    if let Ok(runtime_module) = load_test_module() {
        group.bench_function("stackless_engine_creation", |b| {
            b.iter(|| {
                let engine = StacklessEngine::new);
                black_box(engine)
            };
        };

        group.bench_function("module_instance_creation", |b| {
            b.iter(|| {
                let instance = ModuleInstance::new(runtime_module.clone(), 0).unwrap();
                black_box(instance)
            };
        };

        group.bench_function("full_instantiation", |b| {
            b.iter(|| {
                let mut engine = StacklessEngine::new);
                let instance = ModuleInstance::new(runtime_module.clone(), 0).unwrap();
                let instance_arc = Arc::new(instance;
                let instance_idx = engine.set_current_module(instance_arc).unwrap();
                black_box(instance_idx)
            };
        };
    }

    group.finish);
}

fn benchmark_simple_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_execution";

    if let Ok(runtime_module) = load_test_module() {
        group.bench_function("single_add_execution", |b| {
            let mut engine = StacklessEngine::new);
            let instance = ModuleInstance::new(runtime_module.clone(), 0).unwrap();
            let instance_arc = Arc::new(instance;
            let instance_idx = engine.set_current_module(instance_arc).unwrap();

            b.iter(|| {
                let args = create_test_args(black_box(5), black_box(3;
                let results = engine.execute(instance_idx, 0, args).unwrap();
                black_box(results)
            };
        };

        group.bench_function("execution_with_setup", |b| {
            b.iter(|| {
                let mut engine = StacklessEngine::new);
                let instance = ModuleInstance::new(runtime_module.clone(), 0).unwrap();
                let instance_arc = Arc::new(instance;
                let instance_idx = engine.set_current_module(instance_arc).unwrap();

                let args = create_test_args(black_box(5), black_box(3;
                let results = engine.execute(instance_idx, 0, args).unwrap();
                black_box(results)
            };
        };
    }

    group.finish);
}

fn benchmark_repeated_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeated_execution";

    if let Ok(runtime_module) = load_test_module() {
        let mut engine = StacklessEngine::new);
        let instance = ModuleInstance::new(runtime_module, 0).unwrap();
        let instance_arc = Arc::new(instance;
        let instance_idx = engine.set_current_module(instance_arc).unwrap();

        group.bench_function("10_executions", |b| {
            b.iter(|| {
                let mut total = 0;
                for i in 0..10 {
                    let args = create_test_args(i, i * 2;
                    let results = engine.execute(instance_idx, 0, args).unwrap();
                    if let Some(Value::I32(result)) = results.get(0) {
                        total += result;
                    }
                }
                black_box(total)
            };
        };

        group.bench_function("100_executions", |b| {
            b.iter(|| {
                let mut total = 0;
                for i in 0..100 {
                    let args = create_test_args(i, i * 2;
                    let results = engine.execute(instance_idx, 0, args).unwrap();
                    if let Some(Value::I32(result)) = results.get(0) {
                        total += result;
                    }
                }
                black_box(total)
            };
        };
    }

    group.finish);
}

fn benchmark_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns";

    if let Ok(runtime_module) = load_test_module() {
        group.bench_function("module_cloning", |b| {
            b.iter(|| {
                let cloned = runtime_module.clone();
                black_box(cloned)
            };
        };

        group.bench_function("arc_creation", |b| {
            let instance = ModuleInstance::new(runtime_module.clone(), 0).unwrap();
            b.iter(|| {
                let instance_arc = Arc::new(instance.clone();
                black_box(instance_arc)
            };
        };

        group.bench_function("value_creation", |b| {
            b.iter(|| {
                let args = create_test_args(black_box(42), black_box(24;
                black_box(args)
            };
        };
    }

    group.finish);
}

criterion_group!(
    benches,
    benchmark_module_loading,
    benchmark_engine_instantiation,
    benchmark_simple_execution,
    benchmark_repeated_execution,
    benchmark_memory_patterns
;
criterion_main!(benches;
