//! Cairo Assembly (CASM) code generation

use cairo_m_common::Program;
use cairo_m_compiler_mir::{InstructionKind, MirModule};

use crate::{CodeGenerator, CodegenError};

/// Validate that a MIR module is ready for CASM generation
pub fn validate_for_casm(module: &MirModule) -> Result<(), CodegenError> {
    // Validate basic structure
    module
        .validate()
        .map_err(|e| CodegenError::InvalidMir(format!("Module validation failed: {}", e)))?;

    // Check that SSA destruction has been run (no phi nodes remaining)
    for (_, function) in module.functions() {
        for (_, block) in function.basic_blocks() {
            for instruction in &block.instructions {
                if matches!(instruction.kind, InstructionKind::Phi { .. }) {
                    return Err(CodegenError::InvalidMir(
                        "CASM generation requires SSA destruction - phi nodes not supported"
                            .to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Generate CASM code from an optimized MIR module
pub fn compile_module(module: &MirModule) -> Result<Program, CodegenError> {
    // Validate the module first
    validate_for_casm(module)?;

    // Generate code
    let mut generator = CodeGenerator::new();
    generator.generate_module(module)?;
    Ok(generator.compile())
}
