use std::convert::TryInto;
use std::fs;
use std::path::Path;
use std::time::Duration;

use cairo_m_common::{InputValue, Program};
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::import_from_runner_output;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

const BENCHMARK_DURATION_SECS: u64 = 30;
const N_ITERATIONS: u32 = 100_000;

/// Compiles the fibonacci.cm file from the test data directory
fn compile_fibonacci() -> Program {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap();
    let source_path = format!(
        "{}/test_data/functions/fibonacci_loop.cm",
        workspace_root.display()
    );
    let source_text = fs::read_to_string(&source_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", source_path));
    let options = CompilerOptions::default();
    let output =
        compile_cairo(source_text, source_path, options).expect("Failed to compile fibonacci.cm");
    (*output.program).clone()
}

fn fibonacci_prove_benchmark(c: &mut Criterion) {
    let program = compile_fibonacci();

    let runner_output = run_cairo_program(
        &program,
        "fibonacci_loop",
        &[InputValue::Number(N_ITERATIONS as i64)],
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

    let prover_input = import_from_runner_output(segment, runner_output.public_address_ranges)
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

// =============================
// SHA-256 (1024-byte message)
// =============================

/// Compile the SHA-256 Cairo-M example program
fn compile_sha256() -> Program {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap();
    let source_path = format!(
        "{}/examples/sha256-cairo-m/src/sha256.cm",
        workspace_root.display()
    );
    let source_text = fs::read_to_string(&source_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", source_path));
    let options = CompilerOptions::default();
    let output =
        compile_cairo(source_text, source_path, options).expect("Failed to compile sha256.cm");
    (*output.program).clone()
}

/// Pad a message and convert to InputValue list suitable for the Cairo-M program
/// targeting the `sha256_hash_1024` entrypoint which expects 272 u32 words.
fn prepare_sha256_input_1kb(msg: &[u8]) -> (Vec<InputValue>, usize) {
    let mut padded_bytes = msg.to_vec();
    padded_bytes.push(0x80);
    while padded_bytes.len() % 64 != 56 {
        padded_bytes.push(0x00);
    }
    let bit_len = (msg.len() as u64) * 8;
    padded_bytes.extend_from_slice(&bit_len.to_be_bytes());

    let num_chunks = padded_bytes.len() / 64;

    let mut padded_words: Vec<u32> = padded_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().expect("Chunk size mismatch")))
        .collect();
    // 1024-byte message -> 17 chunks after padding -> 17 * 16 = 272 u32 words
    padded_words.resize(272, 0);

    let input_values = padded_words
        .into_iter()
        .map(|w| InputValue::Number(i64::from(w)))
        .collect::<Vec<_>>();

    (input_values, num_chunks)
}

fn sha256_prove_benchmark(c: &mut Criterion) {
    let program = compile_sha256();

    // Create exactly 1024 bytes of data
    let msg: Vec<u8> = (0..1024).map(|i| (i & 0xFF) as u8).collect();
    let (padded_buffer, num_chunks) = prepare_sha256_input_1kb(&msg);

    let runner_output = run_cairo_program(
        &program,
        "sha256_hash_1024",
        &[
            InputValue::List(padded_buffer),
            InputValue::Number(num_chunks as i64),
        ],
        Default::default(),
    )
    .expect("Failed to run sha256 program");

    let segment = runner_output.vm.segments.into_iter().next().unwrap();
    let trace_length = segment.trace.len();
    println!(
        "SHA256 (1024 bytes) - chunks: {}, trace length: {}",
        num_chunks, trace_length
    );

    let mut group = c.benchmark_group("prover_sha256_1kb");
    group.throughput(Throughput::Elements(trace_length as u64));
    group.measurement_time(Duration::from_secs(BENCHMARK_DURATION_SECS));
    group.sample_size(20);

    let prover_input = import_from_runner_output(segment, runner_output.public_address_ranges)
        .expect("Failed to import runner output");

    group.bench_function("prove", |b| {
        b.iter(|| {
            let proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input.clone(), None)
                .expect("Proving failed");
            black_box(proof)
        })
    });

    group.finish();
}

criterion_group!(benches, fibonacci_prove_benchmark, sha256_prove_benchmark);
criterion_main!(benches);
