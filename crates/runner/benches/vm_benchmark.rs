use std::fs;
use std::time::Duration;

use cairo_m_common::InputValue;
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::{run_cairo_program, RunnerOptions};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

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

    let runner_output = run_cairo_program(
        &program,
        "fibonacci_loop",
        &[InputValue::Number(N_ITERATIONS as i64)],
        RunnerOptions {
            max_steps: 2_usize.pow(30),
        },
    )
    .expect("Execution failed");

    let mut group = c.benchmark_group("fibonacci_1m");
    group.throughput(Throughput::Elements(
        runner_output.vm.segments[0].trace.len() as u64,
    ));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));

    group.bench_function("execution_only", |b| {
        b.iter(|| {
            let runner_output = run_cairo_program(
                &program,
                "fibonacci_loop",
                &[InputValue::Number(N_ITERATIONS as i64)],
                RunnerOptions {
                    max_steps: 2_usize.pow(30),
                },
            )
            .expect("Execution failed");

            black_box(runner_output)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_1m_benchmark);
criterion_main!(benches);
