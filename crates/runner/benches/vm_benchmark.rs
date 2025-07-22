use std::fs;
use std::time::Duration;

use cairo_m_compiler::{CompilerOptions, compile_cairo};
use cairo_m_runner::{RunnerOptions, run_cairo_program};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use stwo_prover::core::fields::m31::M31;

const BENCHMARK_DURATION_SECS: u64 = 30;
const N_ITERATIONS: u32 = 1_000_000;

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

    let output = run_cairo_program(
        &program,
        "fibonacci_loop",
        &[M31::from(N_ITERATIONS)],
        RunnerOptions {
            max_steps: 2_usize.pow(30),
        },
    )
    .expect("Execution failed");

    let mut group = c.benchmark_group("fibonacci_1m");
    group.throughput(Throughput::Elements(output.vm.trace.len() as u64));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));

    group.bench_function("execution_only", |b| {
        b.iter(|| {
            let output = run_cairo_program(
                &program,
                "fibonacci_loop",
                &[M31::from(N_ITERATIONS)],
                RunnerOptions {
                    max_steps: 2_usize.pow(30),
                },
            )
            .expect("Execution failed");

            black_box(output.vm)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_1m_benchmark);
criterion_main!(benches);
