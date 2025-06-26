use std::fs;
use std::time::Duration;

use cairo_m_common::Program;
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::import_from_runner_output;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

const BENCHMARK_DURATION_SECS: u64 = 30;
const N_ITERATIONS: u32 = 10_000; // Smaller than runner benchmark since proving is slower

/// Compiles the fibonacci.cm file from the test data directory
fn compile_fibonacci() -> Program {
    let source_path = format!(
        "{}/tests/test_data/fibonacci.cm",
        env!("CARGO_MANIFEST_DIR")
    );
    let source_text = fs::read_to_string(&source_path).expect("Failed to read fibonacci.cm");
    let options = CompilerOptions { verbose: false };
    let output =
        compile_cairo(source_text, source_path, options).expect("Failed to compile fibonacci.cm");
    (*output.program).clone()
}

fn fibonacci_prove_benchmark(c: &mut Criterion) {
    // Compile the program once
    let program = compile_fibonacci();

    // Run the program once to get the trace length for throughput metrics
    let runner_output = run_cairo_program(
        &program,
        "fib",
        &[M31::from(N_ITERATIONS)],
        Default::default(),
    )
    .expect("Failed to run fibonacci program");

    let trace_length = runner_output.vm.trace.len();
    println!(
        "Fibonacci {} iterations - trace length: {}",
        N_ITERATIONS, trace_length
    );

    let mut group = c.benchmark_group("prover_fibonacci");
    group.throughput(Throughput::Elements(trace_length as u64));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));
    group.sample_size(10); // Reduced sample size since proving is expensive

    group.bench_function("prove", |b| {
        // Setup: run the program for each iteration
        b.iter_batched(
            || {
                // Setup: run the program to get the runner output
                let runner_output = run_cairo_program(
                    &program,
                    "fib",
                    &[M31::from(N_ITERATIONS)],
                    Default::default(),
                )
                .expect("Failed to run fibonacci program");

                import_from_runner_output(&runner_output).expect("Failed to import runner output")
            },
            |mut prover_input| {
                // Benchmark: prove the execution
                let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input)
                    .expect("Proving failed");
                black_box(proof)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, fibonacci_prove_benchmark);
criterion_main!(benches);
