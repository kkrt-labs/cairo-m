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
}
