use peak_alloc::PeakAlloc;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

use std::fs;
use std::path::Path;

use cairo_m_common::{InputValue, Program};
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::import_from_runner_output;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use stwo::core::vcs::blake2_merkle::Blake2sMerkleChannel;

mod sha_bench_utils;
use sha_bench_utils::{compile_sha256, prepare_sha256_input_1kb};

const N_ITERATIONS: u32 = 100_000; // Same as speed benchmark for consistency

/// Compiles the fibonacci.cm file from the test data directory
fn compile_fibonacci() -> Program {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap();
    let source_path = format!(
        "{}/test_data/functions/fibonacci_loop.cm",
        workspace_root.display(),
    );
    let source_text = fs::read_to_string(&source_path).expect("Failed to read fibonacci.cm");
    let options = CompilerOptions::default();
    let output =
        compile_cairo(source_text, source_path, options).expect("Failed to compile fibonacci.cm");
    (*output.program).clone()
}

fn main() {
    eprintln!("Setting up memory benchmark: fibonacci + sha256(1024B)");

    let peak_mem = {
        let program = compile_fibonacci();

        let runner_output = run_cairo_program(
            &program,
            "fibonacci_loop",
            &[InputValue::Number(N_ITERATIONS as i64)],
            Default::default(),
        )
        .expect("Failed to run fibonacci program");

        let segment = runner_output.vm.segments.into_iter().next().unwrap();

        eprintln!("Running fibonacci with n={}", N_ITERATIONS);
        eprintln!("Trace length: {}", segment.trace.len());
        eprintln!("Fibonacci setup complete. Starting prover benchmark...");

        PEAK_ALLOC.reset_peak_usage();

        let mut prover_input =
            import_from_runner_output(segment, runner_output.public_address_ranges)
                .expect("Failed to import runner output");

        let _proof =
            prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input, None).expect("Proving failed");
        PEAK_ALLOC.peak_usage()
    };

    eprintln!("Fibonacci finished. Peak memory usage: {} bytes", peak_mem);

    // =============================
    // SHA-256 (1024-byte message)
    // =============================

    let sha_peak_mem = {
        let sha_program = compile_sha256();
        let msg: Vec<u8> = (0..1024).map(|i| (i & 0xFF) as u8).collect();
        let (padded_buffer, num_chunks) = prepare_sha256_input_1kb(&msg);
        let sha_runner_output = run_cairo_program(
            &sha_program,
            "sha256_hash",
            &[
                InputValue::List(padded_buffer),
                InputValue::Number(num_chunks as i64),
            ],
            Default::default(),
        )
        .expect("Failed to run sha256 program");
        let sha_segment = sha_runner_output.vm.segments.into_iter().next().unwrap();
        eprintln!("SHA256(1024B) trace length: {}", sha_segment.trace.len());

        PEAK_ALLOC.reset_peak_usage();
        let mut sha_prover_input =
            import_from_runner_output(sha_segment, sha_runner_output.public_address_ranges)
                .expect("Failed to import runner output");
        let _sha_proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut sha_prover_input, None)
            .expect("Proving failed");
        PEAK_ALLOC.peak_usage()
    };

    eprintln!("SHA256 finished. Peak memory usage: {} bytes", sha_peak_mem);

    // Output both in JSON format for github-action-benchmark
    println!(
        r#"[{{
    "name": "fibonacci_prove_peak_mem",
    "unit": "bytes",
    "value": {}
}}, {{
    "name": "sha256_1kb_prove_peak_mem",
    "unit": "bytes",
    "value": {}
}}]"#,
        peak_mem, sha_peak_mem
    );
}
