use std::fs;
use std::path::PathBuf;

use cairo_m_common::Program;
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_prover::adapter::import_from_runner_output;
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use cairo_m_verifier::hints::generate_hints;
use cairo_m_verifier::poseidon31_merkle::Poseidon31MerkleHasher;
use cairo_m_verifier::recording_channel::{
    RecordingPoseidon31MerkleChannel, RecordingPoseidon31MerkleHasher,
};
use cairo_m_verifier::verifier_with_channel::verify_cairo_m;
use cairo_m_verifier::Poseidon31MerkleChannel;
use stwo_prover::core::fields::m31::M31;

/// Compiles a Cairo-M file to a Program
pub fn compile_cairo_file(cairo_file: &str) -> Result<Program, String> {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        cairo_file
    );

    // Read the source file
    let source_text = fs::read_to_string(&source_path)
        .map_err(|e| format!("Failed to read source file '{}': {}", source_path, e))?;

    // Compile using the library API
    let options = CompilerOptions { verbose: false };

    let output = compile_cairo(source_text, source_path, options)
        .map_err(|e| format!("Compilation failed: {}", e))?;

    // Clone the Arc<Program> to get an owned Program
    Ok((*output.program).clone())
}

fn get_proof_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_data")
        .join("fibonacci_proof.json")
}

#[test]
fn test_verify_fibonacci() {
    // Load the proof from file
    let proof_path = get_proof_path();

    // If proof doesn't exist, generate it first
    if !proof_path.exists() {
        // Compile the fibonacci program
        let compiled =
            compile_cairo_file("fibonacci.cm").expect("Failed to compile Cairo-M program");

        // Run the program to generate trace and memory data
        let cairo_result = run_cairo_program(&compiled, "fib", &[M31::from(5)], Default::default())
            .expect("Failed to run Cairo-M program");

        // Convert runner output to prover input
        let mut prover_input = import_from_runner_output(
            cairo_result.vm.segments.into_iter().next().unwrap(),
            cairo_result.public_addresses,
        )
        .expect("Failed to import from runner output");

        // Generate the proof using Poseidon31MerkleChannel
        let proof = prove_cairo_m::<Poseidon31MerkleChannel>(&mut prover_input, None)
            .expect("Failed to generate proof");

        // Save the proof to a file
        let proof_json = serde_json::to_string_pretty(&proof).expect("Failed to serialize proof");
        fs::create_dir_all(proof_path.parent().unwrap()).expect("Failed to create directory");
        fs::write(&proof_path, proof_json).expect("Failed to write proof to file");

        println!("Generated and saved proof to: {:?}", proof_path);
    }

    // Load the proof
    let proof_json = fs::read_to_string(&proof_path).expect(
        "Failed to read proof file. Run 'cargo test test_generate_fibonacci_proof --ignored' first",
    );
    let proof: cairo_m_prover::Proof<Poseidon31MerkleHasher> =
        serde_json::from_str(&proof_json).expect("Failed to deserialize proof");

    // Verify the proof using Poseidon31MerkleChannel
    verify_cairo_m::<Poseidon31MerkleChannel>(proof, None).expect("Failed to verify proof");

    println!("Proof verified successfully!");
}

#[test]
fn test_verify_fibonacci_recording() {
    // Load the proof from file
    let proof_path = get_proof_path();

    // If proof doesn't exist, generate it first
    if !proof_path.exists() {
        // Compile the fibonacci program
        let compiled =
            compile_cairo_file("fibonacci.cm").expect("Failed to compile Cairo-M program");

        // Run the program to generate trace and memory data
        let cairo_result = run_cairo_program(&compiled, "fib", &[M31::from(5)], Default::default())
            .expect("Failed to run Cairo-M program");

        // Convert runner output to prover input
        let mut prover_input = import_from_runner_output(
            cairo_result.vm.segments.into_iter().next().unwrap(),
            cairo_result.public_addresses,
        )
        .expect("Failed to import from runner output");

        // Generate the proof using RecordingPoseidon31MerkleChannel
        let proof = prove_cairo_m::<RecordingPoseidon31MerkleChannel>(&mut prover_input, None)
            .expect("Failed to generate proof");

        // Save the proof to a file
        let proof_json = serde_json::to_string_pretty(&proof).expect("Failed to serialize proof");
        fs::create_dir_all(proof_path.parent().unwrap()).expect("Failed to create directory");
        fs::write(&proof_path, proof_json).expect("Failed to write proof to file");

        println!("Generated and saved proof to: {:?}", proof_path);
    }

    // Load the proof
    let proof_json = fs::read_to_string(&proof_path).expect(
        "Failed to read proof file. Run 'cargo test test_generate_fibonacci_proof --ignored' first",
    );

    // ╔══════════════════════════════════════════════════════════════════════╗
    // ║                         Record channel                               ║
    // ╚══════════════════════════════════════════════════════════════════════╝
    // Try to deserialize as RecordingPoseidon31MerkleHasher
    let proof_for_recording: cairo_m_prover::Proof<RecordingPoseidon31MerkleHasher> =
        serde_json::from_str(&proof_json).unwrap();

    // Create a recording channel to capture operations
    use cairo_m_verifier::recording_channel::RecordingPoseidon31Channel;
    use cairo_m_verifier::verifier_with_channel::verify_cairo_m_with_channel;

    let mut recording_channel = RecordingPoseidon31Channel::default();

    // Then verify the proof using RecordingPoseidon31MerkleChannel with our recording channel
    verify_cairo_m_with_channel::<RecordingPoseidon31MerkleChannel>(
        proof_for_recording,
        &mut recording_channel,
        None,
    )
    .expect("Failed to verify proof");

    // ╔══════════════════════════════════════════════════════════════════════╗
    // ║                         Generate hints                               ║
    // ╚══════════════════════════════════════════════════════════════════════╝
    let proof: cairo_m_prover::Proof<Poseidon31MerkleHasher> =
        serde_json::from_str(&proof_json).unwrap();
    let _hints = generate_hints::<Poseidon31MerkleChannel>(&proof);
}
