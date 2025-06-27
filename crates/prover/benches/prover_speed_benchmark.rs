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
const N_ITERATIONS: u32 = 100_000;

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
    let program = compile_fibonacci();

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

    let runner_output = run_cairo_program(
        &program,
        "fib",
        &[M31::from(N_ITERATIONS)],
        Default::default(),
    )
    .expect("Failed to run fibonacci program");

    group.bench_function("prove", |b| {
        b.iter(|| {
            let mut prover_input = import_from_runner_output_ref(&runner_output)
                .expect("Failed to import runner output");

            // Benchmark: prove the execution
            let proof =
                prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input).expect("Proving failed");
            black_box(proof)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_prove_benchmark);
criterion_main!(benches);
