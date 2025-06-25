use std::fs;
use std::time::Duration;

use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::run_cairo_program;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use stwo_prover::core::fields::m31::M31;
use tempfile::NamedTempFile;

const BENCHMARK_DURATION_SECS: u64 = 30;

fn fibonacci_1m_benchmark(c: &mut Criterion) {
    let source_path = format!(
        "{}/benches/{}",
        env!("CARGO_MANIFEST_DIR"),
        "fibonacci_loop.cm"
    );
    let source_text = fs::read_to_string(&source_path).unwrap();
    let options = CompilerOptions { verbose: false };
    let output = compile_cairo(source_text, source_path, options).unwrap();
    let program = (*output.program).clone();

    // Run once to get metrics for throughput calculation and reuse for serialization benchmarks
    let output = run_cairo_program(
        &program,
        "fibonacci_loop",
        &[M31::from(1_000_000)],
        Default::default(),
    )
    .expect("Execution failed");

    let mut group = c.benchmark_group("fibonacci_1m");
    group.throughput(Throughput::Elements(output.vm.trace.len() as u64));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));

    group.bench_function("e2e", |b| {
        b.iter(|| {
            let trace_file = NamedTempFile::new().expect("Failed to create trace temp file");
            let memory_trace_file =
                NamedTempFile::new().expect("Failed to create memory trace temp file");
            let output = run_cairo_program(
                &program,
                "fibonacci_loop",
                &[M31::from(1_000_000)],
                Default::default(),
            )
            .expect("Execution failed");
            output.vm.write_binary_trace(trace_file.path()).unwrap();
            output
                .vm
                .write_binary_memory_trace(memory_trace_file.path())
                .unwrap();

            black_box(output.vm)
        })
    });

    group.bench_function("execution_only", |b| {
        b.iter(|| {
            let output = run_cairo_program(
                &program,
                "fibonacci_loop",
                &[M31::from(1_000_000)],
                Default::default(),
            )
            .expect("Execution failed");
            black_box(output.vm)
        })
    });

    group.bench_function("io_only", |b| {
        // Pre-execute the VM for I/O testing
        let output = run_cairo_program(
            &program,
            "fibonacci_loop",
            &[M31::from(1_000_000)],
            Default::default(),
        )
        .expect("Execution failed");

        let trace_file = NamedTempFile::new().expect("Failed to create trace temp file");
        let memory_trace_file =
            NamedTempFile::new().expect("Failed to create memory trace temp file");

        b.iter(|| {
            output.vm.write_binary_trace(trace_file.path()).unwrap();
            output
                .vm
                .write_binary_memory_trace(memory_trace_file.path())
                .unwrap();
            black_box(())
        })
    });

    group.bench_function("serialize_vm_trace", |b| {
        b.iter(|| {
            let serialized = output.vm.serialize_trace();
            black_box(serialized)
        })
    });

    group.bench_function("serialize_memory_trace", |b| {
        b.iter(|| {
            let serialized = output.vm.memory.serialize_trace();
            black_box(serialized)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_1m_benchmark);
criterion_main!(benches);
