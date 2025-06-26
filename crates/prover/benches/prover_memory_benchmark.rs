use peak_alloc::PeakAlloc;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

use std::fs;

use cairo_m_common::Program;
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::import_from_runner_output;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

const N_ITERATIONS: u32 = 100_000; // Same as speed benchmark for consistency

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

fn main() {
    eprintln!("Setting up benchmark: Compiling and running fibonacci...");

    // 1. Compile the fibonacci program
    let program = compile_fibonacci();

    // 2. Run the program to get the execution trace
    let runner_output = run_cairo_program(
        &program,
        "fib",
        &[M31::from(N_ITERATIONS)],
        Default::default(),
    )
    .expect("Failed to run fibonacci program");

    eprintln!("Running fibonacci with n={}", N_ITERATIONS);
    eprintln!("Trace length: {}", runner_output.vm.trace.len());

    // 3. Import the runner output for proving
    let mut prover_input =
        import_from_runner_output(&runner_output).expect("Failed to import runner output");

    eprintln!("Setup complete. Starting prover benchmark...");

    // Reset peak memory tracking before proving
    PEAK_ALLOC.reset_peak_usage();

    // 4. Prove the execution
    let _proof = prove_cairo_m::<Blake2sMerkleChannel>(&mut prover_input).expect("Proving failed");

    // 5. Get peak memory usage
    let peak_mem = PEAK_ALLOC.peak_usage();

    eprintln!("Benchmark finished. Peak memory usage: {} bytes", peak_mem);

    // Output in JSON format for github-action-benchmark
    println!(
        r#"[{{
    "name": "fibonacci_prove_peak_mem",
    "unit": "bytes",
    "value": {}
}}]"#,
        peak_mem
    );
}
