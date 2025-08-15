# Task: Implement Backend-Pluggable Architecture

## Priority

HIGH - COMPLETED

## Why

The current MIR compiler backend is tightly coupled to the Cairo Assembly (CASM)
code generation, creating significant architectural limitations:

1. **Strategic Extensibility**: Cairo-M needs to support multiple execution and
   proving backends (Cairo bytecode, LLVM, custom VM, WebAssembly, etc.) to
   serve different deployment scenarios and optimization goals.

2. **Optimization Pipeline Isolation**: MIR-level optimizations should be
   backend-agnostic, ensuring that passes like SROA, mem2reg, and dead code
   elimination apply consistently regardless of the target architecture.

3. **Development Velocity**: A pluggable architecture enables parallel
   development of new backends without disrupting the core compiler pipeline.

4. **Maintainability**: Clean separation between frontend optimizations and
   backend code generation reduces complexity and makes the codebase more
   maintainable.

5. **Innovation Platform**: Pluggable backends enable experimentation with new
   target architectures, proving systems, and deployment models without
   rewriting the entire compilation pipeline.

## What

The current architecture has tight coupling between MIR generation and CASM
codegen:

### Current Architecture Problems

- **Direct Coupling**: `cairo_m_compiler_codegen::db::compile_project()`
  directly calls `crate::compile_module(&mir_module)`
- **Hard-coded Backend**: The `CodeGenerator` is specifically designed for CASM,
  with fp-relative memory layout assumptions built-in
- **No Abstraction**: The `compile_module()` function in
  `crates/compiler/codegen/src/lib.rs:38` returns `Program` directly, with no
  intermediate abstraction layer
- **Pipeline Inflexibility**: MIR optimizations are followed immediately by CASM
  generation, with no opportunity to insert alternative backends

### Proposed Architecture

Implement a clean backend trait system that:

- Provides a uniform interface for all code generation backends
- Maintains MIR optimization pipeline independence
- Supports backend-specific configuration and optimization phases
- Enables runtime backend selection and extensibility

## How

### 1. Backend Trait Design

**Create** `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/backend.rs`:

```rust
//! Backend trait system for pluggable code generation

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::{MirModule, MirFunction, PassManager};

/// Errors that can occur during backend code generation
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Backend configuration error: {0}")]
    Configuration(String),
    #[error("Code generation failed: {0}")]
    CodeGeneration(String),
    #[error("Unsupported MIR feature: {0}")]
    UnsupportedFeature(String),
    #[error("Backend-specific error: {0}")]
    BackendSpecific(String),
}

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
        module.validate()
            .map_err(|e| BackendError::UnsupportedFeature(e))?;
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
        config: &BackendConfig
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
    backends: HashMap<String, Box<dyn Backend<Output = Box<dyn std::any::Any + Send + Sync>>>>,
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
        // Type erase the backend for storage
        let erased: Box<dyn Backend<Output = Box<dyn std::any::Any + Send + Sync>>> =
            Box::new(TypeErasedBackend::new(backend));
        self.backends.insert(name, erased);
    }

    /// Get backend by name
    pub fn get(&self, name: &str) -> Option<&dyn Backend<Output = Box<dyn std::any::Any + Send + Sync>>> {
        self.backends.get(name).map(|b| b.as_ref())
    }

    /// List all registered backend names
    pub fn list_backends(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }
}

/// Type-erased wrapper for backends to enable storage in registry
struct TypeErasedBackend<B: Backend> {
    backend: B,
}

impl<B: Backend + 'static> TypeErasedBackend<B>
where
    B::Output: 'static
{
    fn new(backend: B) -> Self {
        Self { backend }
    }
}

impl<B: Backend + 'static> Backend for TypeErasedBackend<B>
where
    B::Output: 'static
{
    type Output = Box<dyn std::any::Any + Send + Sync>;

    fn info(&self) -> &BackendInfo {
        self.backend.info()
    }

    fn validate_module(&self, module: &MirModule) -> BackendResult<()> {
        self.backend.validate_module(module)
    }

    fn backend_passes(&self) -> PassManager {
        self.backend.backend_passes()
    }

    fn generate_code(
        &mut self,
        module: &MirModule,
        config: &BackendConfig
    ) -> BackendResult<Self::Output> {
        let result = self.backend.generate_code(module, config)?;
        Ok(Box::new(result))
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### 2. Pipeline Configuration System

**Create** `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/pipeline.rs`:

```rust
//! Compilation pipeline with pluggable backends

use std::sync::Arc;

use crate::{MirModule, PassManager, backend::{Backend, BackendConfig, BackendResult}};

/// Configuration for the entire compilation pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Backend configuration
    pub backend_config: BackendConfig,
    /// Whether to run standard MIR optimization pipeline
    pub run_mir_optimizations: bool,
    /// Whether to run backend-specific optimizations
    pub run_backend_optimizations: bool,
    /// Custom pass manager (overrides standard pipeline if provided)
    pub custom_passes: Option<PassManager>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            backend_config: BackendConfig::default(),
            run_mir_optimizations: true,
            run_backend_optimizations: true,
            custom_passes: None,
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
        config: PipelineConfig
    ) -> BackendResult<B::Output> {
        // Validate that backend can handle this module
        self.backend.validate_module(&module)?;

        // Run MIR-level optimizations if requested
        if config.run_mir_optimizations {
            let passes = config.custom_passes.unwrap_or_else(|| self.mir_passes.clone());
            self.run_mir_passes(&mut module, passes)?;
        }

        // Run backend-specific optimizations if requested
        if config.run_backend_optimizations {
            let mut backend_passes = self.backend.backend_passes();
            self.run_mir_passes(&mut module, backend_passes)?;
        }

        // Generate code using the backend
        self.backend.generate_code(&module, &config.backend_config)
    }

    /// Run a set of passes on the module
    fn run_mir_passes(&self, module: &mut MirModule, mut passes: PassManager) -> BackendResult<()> {
        for (_, function) in module.functions.iter_enumerated_mut() {
            if let Err(e) = function.validate() {
                return Err(BackendError::CodeGeneration(
                    format!("Function validation failed before optimization: {}", e)
                ));
            }

            passes.run(function);

            if let Err(e) = function.validate() {
                return Err(BackendError::CodeGeneration(
                    format!("Function validation failed after optimization: {}", e)
                ));
            }
        }
        Ok(())
    }
}
```

### 3. CASM Backend Implementation

**Refactor**
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/codegen/src/lib.rs`:

```rust
//! Cairo Assembly (CASM) backend implementation

use cairo_m_common::Program;
use cairo_m_compiler_mir::backend::{Backend, BackendInfo, BackendConfig, BackendResult, BackendError};
use cairo_m_compiler_mir::{MirModule, MirFunction, PassManager};

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
                optional_mir_features: vec![
                    "debug_info".to_string(),
                ],
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
        module.validate()
            .map_err(|e| BackendError::UnsupportedFeature(e))?;

        // Check that SSA destruction has been run (no phi nodes remaining)
        for (_, function) in module.functions() {
            for (_, block) in function.basic_blocks() {
                for instruction in &block.instructions {
                    if matches!(instruction.kind, cairo_m_compiler_mir::InstructionKind::Phi { .. }) {
                        return Err(BackendError::UnsupportedFeature(
                            "CASM backend requires SSA destruction - phi nodes not supported".to_string()
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
        config: &BackendConfig
    ) -> BackendResult<Self::Output> {
        let mut generator = CodeGenerator::new();

        // Configure generator based on backend config
        if config.debug_info {
            generator.enable_debug_info();
        }

        // Apply optimization level
        match config.optimization_level {
            0 => generator.disable_optimizations(),
            1 => generator.set_optimization_level(1),
            2 => generator.set_optimization_level(2),
            3 => generator.set_optimization_level(3),
            _ => return Err(BackendError::Configuration(
                format!("Unsupported optimization level: {}", config.optimization_level)
            )),
        }

        generator.generate_module(module)
            .map_err(|e| BackendError::CodeGeneration(e.to_string()))?;

        Ok(generator.compile())
    }
}

impl Default for CasmBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Main entry point for CASM code generation (preserves existing API)
pub fn compile_module(module: &MirModule) -> Result<Program, CodegenError> {
    use cairo_m_compiler_mir::pipeline::{CompilationPipeline, PipelineConfig};

    let mut pipeline = CompilationPipeline::new(CasmBackend::new());
    let config = PipelineConfig::default();

    pipeline.compile(module.clone(), config)
        .map_err(|e| CodegenError::InvalidMir(e.to_string()))
}
```

### 4. Database Integration

**Update** `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/db.rs` to add
backend selection:

```rust
/// Generate compiled output using the specified backend
#[salsa::tracked]
pub fn compile_with_backend(
    db: &dyn MirDb,
    crate_id: Crate,
    backend_name: String,
    config: BackendConfig,
) -> Option<Box<dyn std::any::Any + Send + Sync>> {
    let mir_module = generate_mir(db, crate_id)?;

    // Get global backend registry (would be initialized in main)
    let registry = get_backend_registry();
    let backend = registry.get(&backend_name)?;

    // This would need to be implemented per-backend type
    // For now, hardcode CASM backend
    if backend_name == "casm" {
        use cairo_m_compiler_mir::pipeline::{CompilationPipeline, PipelineConfig};

        let mut pipeline = CompilationPipeline::new(
            cairo_m_compiler_codegen::CasmBackend::new()
        );
        let pipeline_config = PipelineConfig { backend_config: config, ..Default::default() };

        pipeline.compile(mir_module, pipeline_config).ok()
            .map(|program| Box::new(program) as Box<dyn std::any::Any + Send + Sync>)
    } else {
        None
    }
}

// Global backend registry (would be properly initialized)
fn get_backend_registry() -> &'static cairo_m_compiler_mir::backend::BackendRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<cairo_m_compiler_mir::backend::BackendRegistry> = OnceLock::new();

    REGISTRY.get_or_init(|| {
        let mut registry = cairo_m_compiler_mir::backend::BackendRegistry::new();
        registry.register(cairo_m_compiler_codegen::CasmBackend::new());
        registry
    })
}
```

### 5. Example Alternative Backend

**Create**
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/backends/llvm/src/lib.rs`:

```rust
//! LLVM backend for Cairo-M (example implementation)

use cairo_m_compiler_mir::backend::{Backend, BackendInfo, BackendConfig, BackendResult, BackendError};
use cairo_m_compiler_mir::{MirModule, PassManager};

/// LLVM backend for generating native code
pub struct LlvmBackend {
    info: BackendInfo,
}

impl LlvmBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                name: "llvm".to_string(),
                description: "LLVM backend for native code generation".to_string(),
                version: "0.1.0".to_string(),
                supported_targets: vec!["x86_64".to_string(), "aarch64".to_string()],
                required_mir_features: vec![
                    "basic_blocks".to_string(),
                    "typed_values".to_string(),
                ],
                optional_mir_features: vec![
                    "debug_info".to_string(),
                    "ssa_form".to_string(), // LLVM works better with SSA
                ],
            },
        }
    }
}

/// LLVM IR representation (placeholder)
#[derive(Debug)]
pub struct LlvmModule {
    pub ir_code: String,
    pub bitcode: Vec<u8>,
}

impl Backend for LlvmBackend {
    type Output = LlvmModule;

    fn info(&self) -> &BackendInfo {
        &self.info
    }

    fn backend_passes(&self) -> PassManager {
        // LLVM benefits from SSA form, so don't run SSA destruction
        PassManager::new()
            .add_pass(cairo_m_compiler_mir::Validation::new())
    }

    fn generate_code(
        &mut self,
        module: &MirModule,
        _config: &BackendConfig
    ) -> BackendResult<Self::Output> {
        // Placeholder LLVM code generation
        let ir_code = format!(
            "; Generated LLVM IR for {} functions\n\n{}",
            module.function_count(),
            module.functions()
                .map(|(_, func)| format!("define void @{}() {{\n  ret void\n}}", func.name))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        Ok(LlvmModule {
            ir_code,
            bitcode: Vec::new(), // Would contain actual LLVM bitcode
        })
    }
}
```

### 6. Update Main Library Integration

**Update** `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/src/lib.rs`:

```rust
// Add backend selection parameter
pub fn compile_project_with_backend(
    db: &CompilerDatabase,
    project: cairo_m_project::Project,
    backend_name: String,
    options: CompilerOptions,
) -> Result<CompilerOutput> {
    // ... existing semantic validation ...

    // Use new backend-pluggable compilation
    let backend_config = cairo_m_compiler_mir::backend::BackendConfig::default();
    let program = cairo_m_compiler_mir::db::compile_with_backend(db, crate_id, backend_name, backend_config)
        .and_then(|any| any.downcast::<cairo_m_common::Program>().ok())
        .map(|boxed| Arc::new(*boxed))
        .ok_or_else(|| CompilerError::CodeGenerationFailed("Backend compilation failed".to_string()))?;

    Ok(CompilerOutput {
        program,
        diagnostics,
    })
}
```

## Testing

### 1. Backend Trait Testing

Create tests in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/backend.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;

    #[test]
    fn test_backend_registry() {
        let mut registry = BackendRegistry::new();
        let mock_backend = MockBackend::new();

        registry.register(mock_backend);
        assert!(registry.get("mock").is_some());
        assert_eq!(registry.list_backends(), vec!["mock"]);
    }

    #[test]
    fn test_pipeline_compilation() {
        let backend = MockBackend::new();
        let mut pipeline = CompilationPipeline::new(backend);

        let module = create_test_mir_module();
        let config = PipelineConfig::default();

        let result = pipeline.compile(module, config);
        assert!(result.is_ok());
    }
}

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

    fn info(&self) -> &BackendInfo { &self.info }

    fn generate_code(&mut self, _module: &MirModule, _config: &BackendConfig) -> BackendResult<Self::Output> {
        Ok("mock output".to_string())
    }
}
```

### 2. Integration Testing

Create comprehensive tests that verify:

- MIR optimizations run identically for all backends
- Backend-specific passes only affect their target backend
- Pipeline configuration works correctly
- Error handling propagates properly

### 3. Regression Testing

Ensure existing CASM compilation continues to work:

- All existing codegen tests pass with CasmBackend
- Performance characteristics remain similar
- Output Program format unchanged

## Impact

### Strategic Benefits

1. **Future-Proof Architecture**: Enables support for new proving systems, VMs,
   and deployment targets without rewriting the frontend
2. **Parallel Development**: Teams can develop new backends independently of
   core compiler improvements
3. **Optimization Isolation**: MIR-level optimizations benefit all backends
   equally, preventing optimization fragmentation
4. **Innovation Platform**: Researchers can experiment with new code generation
   strategies while reusing the mature frontend

### Technical Benefits

1. **Clean Separation**: Backend-agnostic MIR optimizations vs. target-specific
   code generation
2. **Testability**: Each backend can be tested in isolation with well-defined
   interfaces
3. **Performance Tuning**: Backend-specific optimization passes enable
   target-specific performance improvements
4. **Maintainability**: Cleaner architecture makes the compiler easier to
   understand and modify

### Immediate Value

1. **CASM Refactoring**: Current CASM backend becomes cleaner and more
   maintainable
2. **Validation**: The backend trait forces explicit validation of backend
   requirements and capabilities
3. **Configuration**: Structured configuration system enables better control
   over compilation behavior
4. **Foundation**: Sets up infrastructure for future backends (LLVM,
   WebAssembly, custom VMs)

This architecture positions Cairo-M as an extensible compilation platform while
maintaining backward compatibility and improving code organization. The high
priority reflects its strategic importance for enabling the project's long-term
goals of supporting diverse execution environments and proving systems.

## Implementation Summary

Successfully implemented the backend pluggability architecture with the
following components:

1. **Backend Trait System** (`mir/src/backend.rs`):
   - `Backend` trait with type-safe output, validation, and code generation
     methods
   - `BackendConfig` for configuration (optimization level, debug info, target
     features)
   - `BackendError` enum for proper error handling
   - `BackendRegistry` for managing multiple backends (with basic
     implementation)
   - Comprehensive test coverage with mock backend

2. **Compilation Pipeline** (`mir/src/pipeline.rs`):
   - `CompilationPipeline<B: Backend>` for managing the full compilation process
   - `PipelineConfig` for controlling MIR and backend optimizations
   - Proper separation between MIR optimizations and backend-specific passes
   - Validation at each step to ensure correctness

3. **CASM Backend Adapter** (`codegen/src/backend.rs`):
   - `CasmBackend` implementing the `Backend` trait
   - Validates SSA destruction (no phi nodes) as required by CASM
   - Provides backend-specific optimization passes
   - Maintains backward compatibility with existing API through
     `compile_module_with_backend`

4. **Testing** (`codegen/tests/backend_integration_test.rs`):
   - Integration tests for basic compilation
   - Tests for custom configuration options
   - Validation tests (correctly rejects modules with phi nodes)
   - All tests passing

The implementation achieves clean separation of concerns between MIR
optimizations and backend code generation, while maintaining full backward
compatibility and enabling future extensibility for alternative backends.
