//! Simplified MIR optimization pipeline configuration

use crate::passes::MirPass;
use crate::{MirModule, PassManager, PrettyPrint};

/// Optimization level for the MIR pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimizations
    None,
    /// Basic optimizations (dead code elimination)
    Basic,
    /// Standard optimizations (default)
    Standard,
    /// Aggressive optimizations
    Aggressive,
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
    /// Lower aggregates to memory for compatibility (only if needed for specific backends)
    pub lower_aggregates_to_memory: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            debug: false,
            lower_aggregates_to_memory: false, // By default, keep aggregates as values
        }
    }
}

impl PipelineConfig {
    /// Create a configuration with no optimizations (for debugging)
    pub const fn no_opt() -> Self {
        Self {
            optimization_level: OptimizationLevel::None,
            debug: false,
            lower_aggregates_to_memory: false,
        }
    }

    /// Create a configuration with debug output enabled
    pub const fn debug() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            debug: true,
            lower_aggregates_to_memory: false,
        }
    }

    /// Create configuration from environment (simplified)
    pub fn from_environment() -> Self {
        let mut config = Self::default();

        // Simple optimization level control
        if let Ok(val) = std::env::var("CAIRO_M_OPT_LEVEL") {
            config.optimization_level = match val.as_str() {
                "0" | "none" => OptimizationLevel::None,
                "1" | "basic" => OptimizationLevel::Basic,
                "2" | "standard" => OptimizationLevel::Standard,
                "3" | "aggressive" => OptimizationLevel::Aggressive,
                _ => OptimizationLevel::Standard,
            };
        }

        // Debug flag
        if let Ok(val) = std::env::var("CAIRO_M_DEBUG") {
            config.debug = val == "1" || val.to_lowercase() == "true";
        }

        config
    }
}

/// Run the optimization pipeline on a MIR module
pub fn optimize_module(module: &mut MirModule, config: &PipelineConfig) {
    let mut pass_manager = match config.optimization_level {
        OptimizationLevel::None => return, // No optimizations
        OptimizationLevel::Basic => PassManager::basic_pipeline(),
        OptimizationLevel::Standard => PassManager::standard_pipeline(),
        OptimizationLevel::Aggressive => PassManager::aggressive_pipeline(),
    };

    // Apply passes to each function
    for function in module.functions_mut() {
        // Validate before optimization
        if let Err(e) = function.validate() {
            eprintln!(
                "Warning: Function validation failed before optimization: {}",
                e
            );
            continue;
        }

        // Run optimization passes
        pass_manager.run(function);

        // Optionally lower aggregates to memory
        if config.lower_aggregates_to_memory {
            use crate::passes::lower_aggregates::LowerAggregatesPass;
            let mut lower_pass = LowerAggregatesPass::new();
            lower_pass.run(function);
        }

        // Validate after optimization
        if let Err(e) = function.validate() {
            eprintln!(
                "Warning: Function validation failed after optimization: {}",
                e
            );
        }

        // Debug output if requested
        if config.debug {
            eprintln!("=== Optimized MIR for {} ===", function.name);
            eprintln!("{}", function.pretty_print(0));
        }
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
        assert!(!config.lower_aggregates_to_memory);
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
