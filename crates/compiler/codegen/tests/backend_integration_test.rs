//! Integration test for backend pluggability  
// TODO: These tests were written for a backend abstraction that doesn't exist yet.
// They should be re-enabled when the backend abstraction is implemented.

#![cfg(feature = "backend_abstraction")]

use cairo_m_compiler_mir::{
    Instruction, InstructionKind, MirFunction, MirModule, MirType, Terminator, ValueId,
};

#[test]
fn test_casm_backend_integration() {
    // Create a simple MIR module with a function
    let mut module = MirModule::new();

    let mut function = MirFunction::new("test_function".to_string());

    // Get the entry block (created by default)
    let entry_block = function.entry_block;

    // Set a simple return terminator (mutable access to basic_blocks)
    function
        .basic_blocks
        .get_mut(entry_block)
        .unwrap()
        .terminator = Terminator::return_values(vec![]);

    module.add_function(function);

    // Create CASM backend and pipeline
    let backend = CasmBackend::new();
    let mut pipeline = CompilationPipeline::new(backend);

    // Compile with default configuration
    let config = PipelineConfig::default();
    let result = pipeline.compile(module, config);

    assert!(
        result.is_ok(),
        "Backend compilation should succeed: {:?}",
        result
    );

    let program = result.unwrap();
    assert!(
        !program.instructions.is_empty(),
        "Should generate instructions"
    );
}

#[test]
fn test_backend_with_custom_config() {
    let mut module = MirModule::new();

    let mut function = MirFunction::new("main".to_string());
    let entry_block = function.entry_block;

    // Set a simple return terminator
    function
        .basic_blocks
        .get_mut(entry_block)
        .unwrap()
        .terminator = Terminator::return_values(vec![]);

    module.add_function(function);

    // Create backend with custom configuration
    let backend = CasmBackend::new();
    let mut pipeline = CompilationPipeline::new(backend);

    let config = PipelineConfig {
        backend_config: BackendConfig {
            optimization_level: 0, // No optimizations
            debug_info: true,      // Include debug info
            ..Default::default()
        },
        run_mir_optimizations: false,     // Skip MIR optimizations
        run_backend_optimizations: false, // Skip backend optimizations
    };

    let result = pipeline.compile(module, config);
    assert!(result.is_ok(), "Should compile with custom config");
}

#[test]
fn test_backend_validation() {
    let backend = CasmBackend::new();

    // Verify backend info
    let info = backend.info();
    assert_eq!(info.name, "casm");
    assert!(info.supported_targets.contains(&"cairo-vm".to_string()));
    assert!(info
        .required_mir_features
        .contains(&"ssa_destruction".to_string()));

    // Test validation with a module containing phi nodes (should fail)
    let mut module = MirModule::new();
    let mut function = MirFunction::new("test".to_string());

    let entry_block = function.entry_block;

    // Add a phi node (which CASM doesn't support)
    let phi_dest = ValueId::new(0);
    function.value_types.insert(phi_dest, MirType::Felt);

    function
        .basic_blocks
        .get_mut(entry_block)
        .unwrap()
        .instructions
        .push(Instruction {
            kind: InstructionKind::Phi {
                dest: phi_dest,
                ty: MirType::Felt,
                sources: vec![],
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        });

    module.add_function(function);

    // Validation should fail due to phi node
    let validation_result = backend.validate_module(&module);
    assert!(
        validation_result.is_err(),
        "Should reject module with phi nodes"
    );

    let err_msg = validation_result.unwrap_err().to_string();
    assert!(
        err_msg.contains("phi nodes not supported"),
        "Error should mention phi nodes"
    );
}
