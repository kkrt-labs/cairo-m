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

    // Check that no aggregate value operations remain
    // The CASM backend requires all aggregates to be lowered to memory operations
    for (_func_idx, function) in module.functions() {
        for (block_idx, block) in function.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    // Phi nodes should be removed by SSA destruction
                    InstructionKind::Phi { .. } => {
                        return Err(CodegenError::InvalidMir(format!(
                            "Function '{}', block {}, instruction {}: \
                            CASM generation requires SSA destruction - phi nodes not supported",
                            function.name,
                            block_idx.index(),
                            instr_idx
                        )));
                    }
                    _ => {
                        // Other instructions are fine
                    }
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
    generator.compile()
}
