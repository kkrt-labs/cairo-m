//! Cairo Assembly (CASM) backend implementation

use cairo_m_common::Program;
use cairo_m_compiler_mir::backend::{
    Backend, BackendConfig, BackendError, BackendInfo, BackendResult,
};
use cairo_m_compiler_mir::{InstructionKind, MirModule, PassManager};

use crate::{CodeGenerator, CodegenError};

/// CASM (Cairo Assembly) backend for generating Cairo VM bytecode
pub struct CasmBackend {
    info: BackendInfo,
}

impl CasmBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                name: "casm".to_string(),
                description: "Cairo Assembly backend for Cairo VM".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supported_targets: vec!["cairo-vm".to_string()],
                required_mir_features: vec![
                    "basic_blocks".to_string(),
                    "ssa_destruction".to_string(), // CASM doesn't support phi nodes
                ],
                optional_mir_features: vec!["debug_info".to_string()],
            },
        }
    }
}

impl Backend for CasmBackend {
    type Output = Program;

    fn info(&self) -> &BackendInfo {
        &self.info
    }

    fn validate_module(&self, module: &MirModule) -> BackendResult<()> {
        // Validate basic structure
        module
            .validate()
            .map_err(BackendError::UnsupportedFeature)?;

        // Check that SSA destruction has been run (no phi nodes remaining)
        for (_, function) in module.functions() {
            for (_, block) in function.basic_blocks() {
                for instruction in &block.instructions {
                    if matches!(instruction.kind, InstructionKind::Phi { .. }) {
                        return Err(BackendError::UnsupportedFeature(
                            "CASM backend requires SSA destruction - phi nodes not supported"
                                .to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn backend_passes(&self) -> PassManager {
        // CASM-specific optimizations that run after standard MIR passes
        PassManager::new()
            .add_pass(cairo_m_compiler_mir::FuseCmpBranch::new()) // Fuse compare-branch for efficiency
            .add_pass(cairo_m_compiler_mir::DeadCodeElimination::new()) // Final cleanup
    }

    fn generate_code(
        &mut self,
        module: &MirModule,
        _config: &BackendConfig,
    ) -> BackendResult<Self::Output> {
        let mut generator = CodeGenerator::new();

        // Note: The current CodeGenerator doesn't support configuration options yet
        // When it does, we can use the config parameter to control optimization level, debug info, etc.

        generator
            .generate_module(module)
            .map_err(|e| BackendError::CodeGeneration(format!("{:?}", e)))?;

        Ok(generator.compile())
    }
}

impl Default for CasmBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Main entry point for CASM code generation (preserves existing API)
pub fn compile_module_with_backend(module: &MirModule) -> Result<Program, CodegenError> {
    use cairo_m_compiler_mir::pipeline::{CompilationPipeline, PipelineConfig};

    let mut pipeline = CompilationPipeline::new(CasmBackend::new());
    let config = PipelineConfig::default();

    pipeline
        .compile(module.clone(), config)
        .map_err(|e| CodegenError::InvalidMir(e.to_string()))
}
