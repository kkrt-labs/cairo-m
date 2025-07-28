use std::fs;
use std::time::Duration;

use cairo_m_common::Program;
use cairo_m_compiler::{CompilerOptions, compile_cairo};
use cairo_m_prover::adapter::import_from_runner_output;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
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

    let segment = runner_output.vm.segments.into_iter().next().unwrap();
    let trace_length = segment.trace.len();
    println!(
        "Fibonacci {} iterations - trace length: {}",
        N_ITERATIONS, trace_length
    );

    let mut group = c.benchmark_group("prover_fibonacci");
    group.throughput(Throughput::Elements(trace_length as u64));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));

    let prover_input = import_from_runner_output(segment, runner_output.public_addresses)
        .expect("Failed to import runner output");

    group.bench_function("prove", |b| {
        b.iter(|| {
            // Benchmark: prove the execution
            let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input.clone(), None)
                .expect("Proving failed");
            black_box(proof)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_prove_benchmark);
criterion_main!(benches);
