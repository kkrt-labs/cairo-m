//! Compilation pipeline with pluggable backends

use crate::{
    backend::{Backend, BackendConfig, BackendError, BackendResult},
    MirModule, PassManager,
};

/// Configuration for the entire compilation pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Backend configuration
    pub backend_config: BackendConfig,
    /// Whether to run standard MIR optimization pipeline
    pub run_mir_optimizations: bool,
    /// Whether to run backend-specific optimizations
    pub run_backend_optimizations: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            backend_config: BackendConfig::default(),
            run_mir_optimizations: true,
            run_backend_optimizations: true,
        }
    }
}

/// A complete compilation pipeline with a pluggable backend
pub struct CompilationPipeline<B: Backend> {
    backend: B,
    mir_passes: PassManager,
}

impl<B: Backend> CompilationPipeline<B> {
    /// Create a new pipeline with the given backend
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            mir_passes: PassManager::standard_pipeline(),
        }
    }

    /// Create a pipeline with custom MIR passes
    pub fn with_mir_passes(mut self, passes: PassManager) -> Self {
        self.mir_passes = passes;
        self
    }

    /// Compile a MIR module through the complete pipeline
    pub fn compile(
        &mut self,
        mut module: MirModule,
        config: PipelineConfig,
    ) -> BackendResult<B::Output> {
        // Validate that backend can handle this module
        self.backend.validate_module(&module)?;

        // Run MIR-level optimizations if requested
        if config.run_mir_optimizations {
            for function in module.functions_mut() {
                if let Err(e) = function.validate() {
                    return Err(BackendError::CodeGeneration(format!(
                        "Function validation failed before optimization: {}",
                        e
                    )));
                }

                self.mir_passes.run(function);

                if let Err(e) = function.validate() {
                    return Err(BackendError::CodeGeneration(format!(
                        "Function validation failed after optimization: {}",
                        e
                    )));
                }
            }
        }

        // Run backend-specific optimizations if requested
        if config.run_backend_optimizations {
            let mut backend_passes = self.backend.backend_passes();
            for function in module.functions_mut() {
                backend_passes.run(function);
            }
        }

        // Generate code using the backend
        self.backend.generate_code(&module, &config.backend_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::BackendInfo;

    struct TestBackend {
        info: BackendInfo,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                info: BackendInfo {
                    name: "test".to_string(),
                    description: "Test backend".to_string(),
                    version: "1.0.0".to_string(),
                    supported_targets: vec!["test".to_string()],
                    required_mir_features: vec![],
                    optional_mir_features: vec![],
                },
            }
        }
    }

    impl Backend for TestBackend {
        type Output = String;

        fn info(&self) -> &BackendInfo {
            &self.info
        }

        fn generate_code(
            &mut self,
            module: &MirModule,
            _config: &BackendConfig,
        ) -> BackendResult<Self::Output> {
            Ok(format!(
                "Generated code for {} functions",
                module.function_count()
            ))
        }
    }

    #[test]
    fn test_pipeline_compilation() {
        let backend = TestBackend::new();
        let mut pipeline = CompilationPipeline::new(backend);

        let module = MirModule::new();
        let config = PipelineConfig::default();

        let result = pipeline.compile(module, config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_with_custom_passes() {
        let backend = TestBackend::new();
        let custom_passes = PassManager::new();
        let mut pipeline = CompilationPipeline::new(backend).with_mir_passes(custom_passes);

        let module = MirModule::new();
        let config = PipelineConfig {
            run_mir_optimizations: true,
            ..Default::default()
        };

        let result = pipeline.compile(module, config);
        assert!(result.is_ok());
    }
}
