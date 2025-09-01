//! Simplified MIR optimization pipeline configuration

use crate::{MirModule, PassManager};

/// Optimization level for the MIR pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimizations
    None,
    /// Standard optimizations (default)
    Standard,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::Standard
    }
}

/// Simple pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    /// Enable debug output (verbose MIR dumps)
    pub debug: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            debug: false,
        }
    }
}

impl PipelineConfig {
    /// Create a configuration with no optimizations (for debugging)
    pub const fn no_opt() -> Self {
        Self {
            optimization_level: OptimizationLevel::None,
            debug: false,
        }
    }

    /// Create a configuration with debug output enabled
    pub const fn debug() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            debug: true,
        }
    }
}

/// Run the optimization pipeline on a MIR module
pub fn optimize_module(module: &mut MirModule, config: &PipelineConfig) {
    let mut pass_manager = match config.optimization_level {
        OptimizationLevel::None => PassManager::no_opt_pipeline(),
        OptimizationLevel::Standard => PassManager::standard_pipeline(),
    };

    // Apply passes to each function
    for function in module.functions_mut() {
        // Validate before optimization
        let _ = function.validate();

        // Run optimization passes
        pass_manager.run(function);

        // Validate after optimization
        let _ = function.validate();

        // Debug output removed - no logging
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MirFunction;

    #[test]
    fn test_default_config() {
        let config = PipelineConfig::default();
        assert_eq!(config.optimization_level, OptimizationLevel::Standard);
        assert!(!config.debug);
    }

    #[test]
    fn test_no_opt_config() {
        let config = PipelineConfig::no_opt();
        assert_eq!(config.optimization_level, OptimizationLevel::None);
    }

    #[test]
    fn test_optimize_module() {
        let mut module = MirModule::new();
        let mut func = MirFunction::new("test".to_string());
        let entry = func.add_basic_block();
        func.entry_block = entry;
        module.add_function(func);

        let config = PipelineConfig::default();
        optimize_module(&mut module, &config);

        // Module should still be valid after optimization
        assert!(module.validate().is_ok());
    }
}
