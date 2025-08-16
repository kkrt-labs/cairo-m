//! Backend trait system for pluggable code generation

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::{MirFunction, MirModule, PassManager};

/// Errors that can occur during backend code generation
#[derive(Debug)]
pub enum BackendError {
    Configuration(String),
    CodeGeneration(String),
    UnsupportedFeature(String),
    BackendSpecific(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(msg) => write!(f, "Backend configuration error: {}", msg),
            Self::CodeGeneration(msg) => write!(f, "Code generation failed: {}", msg),
            Self::UnsupportedFeature(msg) => write!(f, "Unsupported MIR feature: {}", msg),
            Self::BackendSpecific(msg) => write!(f, "Backend-specific error: {}", msg),
        }
    }
}

impl Error for BackendError {}

/// Result type for backend operations
pub type BackendResult<T> = Result<T, BackendError>;

/// Configuration for backend code generation
#[derive(Debug, Clone)]
pub struct BackendConfig {
    /// Backend-specific options as key-value pairs
    pub options: HashMap<String, String>,
    /// Optimization level (0-3, similar to LLVM)
    pub optimization_level: u8,
    /// Whether to include debug information
    pub debug_info: bool,
    /// Target-specific features to enable
    pub target_features: Vec<String>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            options: HashMap::new(),
            optimization_level: 2,
            debug_info: false,
            target_features: Vec::new(),
        }
    }
}

/// Metadata about a backend's capabilities and constraints
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Unique name for this backend
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Version of the backend implementation
    pub version: String,
    /// Supported target architectures/platforms
    pub supported_targets: Vec<String>,
    /// Required MIR features (for validation)
    pub required_mir_features: Vec<String>,
    /// Optional features that enhance but aren't required
    pub optional_mir_features: Vec<String>,
}

/// Trait for pluggable code generation backends
pub trait Backend: Send + Sync {
    /// Type of the final compiled output
    type Output: Send + Sync;

    /// Get information about this backend
    fn info(&self) -> &BackendInfo;

    /// Validate that this backend can handle the given MIR module
    fn validate_module(&self, module: &MirModule) -> BackendResult<()> {
        // Default implementation checks basic structure
        module
            .validate()
            .map_err(BackendError::UnsupportedFeature)?;
        Ok(())
    }

    /// Get backend-specific optimization passes
    /// These run AFTER standard MIR optimizations but BEFORE code generation
    fn backend_passes(&self) -> PassManager {
        PassManager::new() // Default: no backend-specific passes
    }

    /// Generate code from an optimized MIR module
    fn generate_code(
        &mut self,
        module: &MirModule,
        config: &BackendConfig,
    ) -> BackendResult<Self::Output>;

    /// Optional: Generate code from a single function
    /// Useful for JIT compilation or incremental code generation
    fn generate_function_code(
        &mut self,
        function: &MirFunction,
        config: &BackendConfig,
    ) -> BackendResult<Self::Output> {
        // Default: create a single-function module and generate from that
        let mut module = MirModule::new();
        module.add_function(function.clone());
        self.generate_code(&module, config)
    }
}

/// Registry for managing multiple backends
pub struct BackendRegistry {
    backends: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Register a new backend
    pub fn register<B>(&mut self, backend: B)
    where
        B: Backend + 'static,
        B::Output: 'static,
    {
        let name = backend.info().name.clone();
        self.backends.insert(name, Box::new(backend));
    }

    /// Get backend by name with type safety
    pub fn get<B>(&self, name: &str) -> Option<&B>
    where
        B: Backend + 'static,
    {
        self.backends.get(name).and_then(|b| b.downcast_ref::<B>())
    }

    /// List all registered backend names
    pub fn list_backends(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend {
        info: BackendInfo,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                info: BackendInfo {
                    name: "mock".to_string(),
                    description: "Mock backend for testing".to_string(),
                    version: "0.1.0".to_string(),
                    supported_targets: vec!["test".to_string()],
                    required_mir_features: vec![],
                    optional_mir_features: vec![],
                },
            }
        }
    }

    impl Backend for MockBackend {
        type Output = String;

        fn info(&self) -> &BackendInfo {
            &self.info
        }

        fn generate_code(
            &mut self,
            _module: &MirModule,
            _config: &BackendConfig,
        ) -> BackendResult<Self::Output> {
            Ok("mock output".to_string())
        }
    }

    #[test]
    fn test_backend_registry() {
        let mut registry = BackendRegistry::new();
        let mock_backend = MockBackend::new();

        registry.register(mock_backend);
        assert!(registry.get::<MockBackend>("mock").is_some());
        assert_eq!(registry.list_backends(), vec!["mock"]);
    }

    #[test]
    fn test_backend_validation() {
        let mut backend = MockBackend::new();
        let module = MirModule::new();
        let config = BackendConfig::default();

        let result = backend.generate_code(&module, &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mock output");
    }
}
