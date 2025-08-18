//! Tests for simplified pipeline configuration

#[cfg(test)]
mod tests {
    use crate::pipeline::{OptimizationLevel, PipelineConfig};

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
        assert!(!config.debug);
    }

    #[test]
    fn test_debug_config() {
        let config = PipelineConfig::debug();
        assert_eq!(config.optimization_level, OptimizationLevel::Standard);
        assert!(config.debug);
    }

    #[test]
    fn test_environment_config() {
        // Test optimization level
        std::env::set_var("CAIRO_M_OPT_LEVEL", "0");
        let config = PipelineConfig::from_environment();
        assert_eq!(config.optimization_level, OptimizationLevel::None);
        std::env::remove_var("CAIRO_M_OPT_LEVEL");

        std::env::set_var("CAIRO_M_OPT_LEVEL", "1");
        let config = PipelineConfig::from_environment();
        assert_eq!(config.optimization_level, OptimizationLevel::Basic);
        std::env::remove_var("CAIRO_M_OPT_LEVEL");

        std::env::set_var("CAIRO_M_OPT_LEVEL", "3");
        let config = PipelineConfig::from_environment();
        assert_eq!(config.optimization_level, OptimizationLevel::Aggressive);
        std::env::remove_var("CAIRO_M_OPT_LEVEL");

        // Test debug flag
        std::env::set_var("CAIRO_M_DEBUG", "1");
        let config = PipelineConfig::from_environment();
        assert!(config.debug);
        std::env::remove_var("CAIRO_M_DEBUG");
    }

    #[test]
    fn test_optimization_levels() {
        assert_eq!(OptimizationLevel::default(), OptimizationLevel::Standard);

        // Test that each level is distinct
        assert_ne!(OptimizationLevel::None, OptimizationLevel::Basic);
        assert_ne!(OptimizationLevel::Basic, OptimizationLevel::Standard);
        assert_ne!(OptimizationLevel::Standard, OptimizationLevel::Aggressive);
    }
}
