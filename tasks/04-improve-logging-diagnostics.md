# Improve Logging and Diagnostics Infrastructure

**Priority**: MEDIUM  
**Component**: MIR Passes  
**Impact**: Debuggability, Maintainability

## Problem

Current MIR passes lack comprehensive logging and diagnostics, making debugging
and optimization analysis difficult:

### Current Issues

1. **No structured logging**: Passes use ad-hoc `println!` or no logging at all
2. **Missing pass statistics**: No way to measure optimization impact
3. **Poor debugging support**: Difficult to trace why optimizations failed
4. **No performance metrics**: Can't identify optimization bottlenecks
5. **Inconsistent verbosity**: No unified approach to debug output

### Impact on Development

- **Debugging difficulty**: Hard to understand why passes behave unexpectedly
- **Performance tuning**: Can't identify which passes are expensive
- **Regression analysis**: Difficult to track optimization quality changes
- **User experience**: No insight into compilation progress for complex programs

## Solution

### Structured Logging Framework

**New file**: `crates/compiler/mir/src/diagnostics.rs`

```rust
use std::time::{Duration, Instant};
use tracing::{debug, info, trace, warn};

/// Centralized diagnostics for MIR passes
#[derive(Debug, Clone)]
pub struct PassDiagnostics {
    pub pass_name: String,
    pub function_name: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub instructions_before: usize,
    pub instructions_after: usize,
    pub blocks_before: usize,
    pub blocks_after: usize,
    pub changes_made: Vec<OptimizationChange>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum OptimizationChange {
    ConstantFolded { instruction_id: InstructionId, old_value: String, new_value: String },
    CopyPropagated { from: ValueId, to: ValueId },
    DeadCodeEliminated { instruction_id: InstructionId },
    BranchSimplified { block_id: BlockId, condition: String },
    SROAApplied { aggregate_id: ValueId, fields: Vec<String> },
}

impl PassDiagnostics {
    pub fn new(pass_name: &str, function: &MirFunction) -> Self {
        Self {
            pass_name: pass_name.to_string(),
            function_name: function.name.clone(),
            start_time: Instant::now(),
            end_time: None,
            instructions_before: function.count_instructions(),
            instructions_after: 0,
            blocks_before: function.basic_blocks.len(),
            blocks_after: 0,
            changes_made: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn finish(&mut self, function: &MirFunction) {
        self.end_time = Some(Instant::now());
        self.instructions_after = function.count_instructions();
        self.blocks_after = function.basic_blocks.len();

        let duration = self.duration();
        let instruction_delta = self.instructions_after as i32 - self.instructions_before as i32;
        let block_delta = self.blocks_after as i32 - self.blocks_before as i32;

        info!(
            pass = self.pass_name,
            function = self.function_name,
            duration_ms = duration.as_millis(),
            instructions_delta = instruction_delta,
            blocks_delta = block_delta,
            changes = self.changes_made.len(),
            "Pass completed"
        );

        if instruction_delta > 0 {
            warn!(
                pass = self.pass_name,
                function = self.function_name,
                delta = instruction_delta,
                "Pass increased instruction count"
            );
        }
    }

    pub fn record_change(&mut self, change: OptimizationChange) {
        trace!(
            pass = self.pass_name,
            change = ?change,
            "Optimization applied"
        );
        self.changes_made.push(change);
    }

    pub fn add_warning(&mut self, warning: String) {
        warn!(
            pass = self.pass_name,
            function = self.function_name,
            warning = warning,
            "Pass warning"
        );
        self.warnings.push(warning);
    }

    pub fn duration(&self) -> Duration {
        self.end_time.unwrap_or_else(Instant::now) - self.start_time
    }
}
```

### Enhanced MirPass Trait

**Update**: `crates/compiler/mir/src/passes.rs`

```rust
pub trait MirPass {
    fn run(&mut self, function: &mut MirFunction) -> bool;
    fn name(&self) -> &'static str;

    /// Run pass with full diagnostics
    fn run_with_diagnostics(&mut self, function: &mut MirFunction) -> (bool, PassDiagnostics) {
        let mut diagnostics = PassDiagnostics::new(self.name(), function);

        debug!(
            pass = self.name(),
            function = function.name,
            instructions = function.count_instructions(),
            blocks = function.basic_blocks.len(),
            "Starting pass"
        );

        let modified = self.run(function);

        diagnostics.finish(function);

        if modified {
            debug!(
                pass = self.name(),
                function = function.name,
                changes = diagnostics.changes_made.len(),
                "Pass made changes"
            );
        } else {
            debug!(
                pass = self.name(),
                function = function.name,
                "Pass made no changes"
            );
        }

        (modified, diagnostics)
    }

    /// Get pass-specific statistics
    fn statistics(&self) -> PassStatistics {
        PassStatistics::default()
    }
}

#[derive(Debug, Default)]
pub struct PassStatistics {
    pub total_runs: usize,
    pub successful_optimizations: usize,
    pub total_time: Duration,
    pub average_time_per_run: Duration,
}
```

### Pass-Specific Diagnostics

Example for ConstantFolding:

```rust
impl ConstantFolding {
    fn try_fold_instruction_with_diagnostics(
        &self,
        instr: &mut crate::Instruction,
        diagnostics: &mut PassDiagnostics
    ) -> bool {
        match &instr.kind {
            InstructionKind::BinaryOp { op, dest, left, right } => {
                if let (Value::Literal(left_lit), Value::Literal(right_lit)) = (left, right) {
                    if let Some(result) = self.try_fold_binary_op(*op, *left_lit, *right_lit) {
                        let old_value = format!("{:?} {:?} {:?}", left_lit, op, right_lit);
                        let new_value = format!("{:?}", result);

                        diagnostics.record_change(OptimizationChange::ConstantFolded {
                            instruction_id: instr.id,
                            old_value,
                            new_value: new_value.clone(),
                        });

                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source: Value::Literal(result),
                            ty: op.result_type(),
                        };
                        return true;
                    }
                }
            }
            _ => {}
        }
        false
    }
}
```

### Logging Configuration

**Update**: `crates/compiler/mir/src/pipeline.rs`

```rust
#[derive(Debug, Clone)]
pub struct DiagnosticsConfig {
    /// Enable structured logging
    pub enable_logging: bool,
    /// Log level filter
    pub log_level: LogLevel,
    /// Export diagnostics to file
    pub export_file: Option<PathBuf>,
    /// Include detailed optimization traces
    pub trace_optimizations: bool,
    /// Measure timing information
    pub measure_timing: bool,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl PipelineConfig {
    pub fn with_diagnostics(mut self, config: DiagnosticsConfig) -> Self {
        self.diagnostics = Some(config);
        self
    }

    pub fn verbose() -> Self {
        Self::default().with_diagnostics(DiagnosticsConfig {
            enable_logging: true,
            log_level: LogLevel::Debug,
            export_file: None,
            trace_optimizations: true,
            measure_timing: true,
        })
    }
}
```

### Command Line Integration

**Update**: `crates/compiler/codegen/src/main.rs`

```rust
#[derive(clap::Parser)]
struct Args {
    /// Enable verbose MIR pass diagnostics
    #[arg(long)]
    verbose_passes: bool,

    /// Export pass diagnostics to file
    #[arg(long)]
    export_diagnostics: Option<PathBuf>,

    /// Log level for MIR passes
    #[arg(long, default_value = "info")]
    pass_log_level: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose_passes {
        tracing_subscriber::fmt()
            .with_max_level(parse_log_level(&args.pass_log_level))
            .init();
    }

    let mut config = PipelineConfig::default();
    if args.verbose_passes {
        config = config.with_diagnostics(DiagnosticsConfig {
            enable_logging: true,
            log_level: parse_log_level(&args.pass_log_level),
            export_file: args.export_diagnostics,
            trace_optimizations: true,
            measure_timing: true,
        });
    }

    // ... rest of compilation
}
```

## Files to Modify

- **New**: `crates/compiler/mir/src/diagnostics.rs` - Core diagnostics
  infrastructure
- **Update**: `crates/compiler/mir/src/passes.rs` - Enhanced MirPass trait
- **Update**: `crates/compiler/mir/src/pipeline.rs` - Diagnostics configuration
- **Update**: Each pass file - Add diagnostic integration
- **Update**: CLI binary - Add logging flags
- **Add**: `Cargo.toml` dependencies for `tracing` and `tracing-subscriber`

## Implementation Plan

### Phase 1: Basic Infrastructure

1. Add tracing dependencies
2. Create PassDiagnostics struct
3. Enhance MirPass trait with diagnostics

### Phase 2: Pass Integration

1. Update each pass to use diagnostics
2. Add specific optimization change tracking
3. Implement timing measurements

### Phase 3: Configuration and CLI

1. Add diagnostics configuration
2. Integrate with pipeline
3. Add command line flags

### Phase 4: Advanced Features

1. Export diagnostics to structured formats (JSON, CSV)
2. Add performance regression detection
3. Implement optimization visualization tools

## Benefits

1. **Better debugging**: Detailed insight into pass behavior
2. **Performance analysis**: Identify optimization bottlenecks
3. **Regression detection**: Track optimization quality over time
4. **User transparency**: Show compilation progress and optimizations
5. **Development velocity**: Faster debugging of pass issues

## Test Strategy

```rust
#[test]
fn test_diagnostics_tracking() {
    let mut function = create_test_function();
    let mut pass = ConstantFolding::new();

    let (modified, diagnostics) = pass.run_with_diagnostics(&mut function);

    assert!(modified);
    assert!(!diagnostics.changes_made.is_empty());
    assert!(diagnostics.duration() > Duration::ZERO);
}

#[test]
fn test_logging_integration() {
    let _guard = setup_test_logging();

    let mut function = create_test_function();
    let mut pass_manager = PassManager::standard_pipeline();

    pass_manager.run(&mut function);

    // Verify logs were produced (using testing framework)
}
```

## Dependencies

- Add `tracing` and `tracing-subscriber` crates
- Consider `serde` for structured export formats

## Acceptance Criteria

- [ ] Structured logging framework using `tracing`
- [ ] Pass diagnostics with timing and change tracking
- [ ] CLI integration with verbosity controls
- [ ] Export capabilities for analysis tools
- [ ] All passes integrate with diagnostics framework
- [ ] Performance regression testing support
- [ ] Comprehensive documentation and examples
