# Task 06: Pre-Codegen Validation for Aggregate-Free MIR

## Overview

This task implements a validation pass to ensure that MIR is free of aggregate
value operations before CASM code generation. The Cairo-M compiler uses an
"aggregate-first" approach where high-level aggregate operations are eventually
lowered to memory operations for backend compatibility.

## Analysis of Current Validation Infrastructure

### Current Validation System

The existing `Validation` pass in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs` provides:

1. **SSA Invariant Checks**: Validates single static assignment form
   (configurable)
2. **Value Usage Validation**: Ensures all used values are properly defined
3. **Type Safety**: Validates pointer types, store operations, GEP usage
4. **Aggregate Operation Validation**: Checks semantics of aggregate
   instructions
5. **CFG Structure Validation**: Validates control flow graph integrity

### Current Pipeline Integration

The validation pass is integrated into the optimization pipeline at two points:

- **Line 895**: `Validation::new()` - Validates SSA form before SSA destruction
- **Line 905**: `Validation::new_post_ssa()` - Validates final MIR after all
  passes

### Current Limitation

The existing validation doesn't distinguish between acceptable aggregate
operations (before lowering) and forbidden aggregate operations (after lowering
for CASM targets). The CASM codegen currently handles this by throwing runtime
errors (lines 571-583 in `generator.rs`), but this provides poor error messages
and occurs too late in the compilation process.

## Forbidden Instructions for CASM Targets

Based on analysis of the CASM code generator, the following MIR instructions are
forbidden for CASM targets and should be caught by pre-codegen validation:

### Value-Based Aggregate Instructions

These instructions operate on aggregate values directly and must be lowered to
memory operations:

1. **`MakeTuple`** - Constructs tuple values
2. **`ExtractTupleElement`** - Extracts elements from tuple values
3. **`MakeStruct`** - Constructs struct values
4. **`ExtractStructField`** - Extracts fields from struct values
5. **`InsertField`** - Creates new struct with updated field
6. **`InsertTuple`** - Creates new tuple with updated element

### Special Case: Phi Nodes

Phi nodes with aggregate types are also problematic for CASM:

- **`Phi`** nodes with `MirType::Tuple` or `MirType::Struct` types

### Acceptable Instructions

All other MIR instructions are acceptable for CASM targets:

- Memory operations: `FrameAlloc`, `Load`, `Store`, `GetElementPtr`, `AddressOf`
- Arithmetic: `BinaryOp`, `UnaryOp`, `Assign`
- Control flow support: `Call`, `VoidCall`, `Cast`
- SSA support: `Phi` with non-aggregate types
- Utilities: `Debug`, `Nop`

## Implementation Plan

### 1. New Validation Pass: `CasmCompatibilityValidation`

Create a specialized validation pass for CASM backend compatibility:

```rust
/// Validation pass that ensures MIR is compatible with CASM code generation
/// by verifying that all aggregate operations have been lowered to memory operations
#[derive(Debug, Default)]
pub struct CasmCompatibilityValidation {
    /// Collect all validation errors instead of failing on first error
    errors: Vec<CasmValidationError>,
}

#[derive(Debug, Clone)]
pub struct CasmValidationError {
    pub message: String,
    pub instruction_location: Option<(BasicBlockId, usize)>, // block_id, instruction_index
    pub suggested_fix: Option<String>,
}
```

### 2. Implementation Details

Add to `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs`:

```rust
impl CasmCompatibilityValidation {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    /// Check if a MIR type is an aggregate type that's forbidden in CASM
    fn is_forbidden_aggregate_type(ty: &MirType) -> bool {
        matches!(ty, MirType::Tuple(_) | MirType::Struct { .. })
    }

    /// Validate a single instruction for CASM compatibility
    fn validate_instruction(
        &mut self,
        instruction: &Instruction,
        block_id: BasicBlockId,
        instr_index: usize,
    ) {
        match &instruction.kind {
            // Forbidden aggregate value instructions
            InstructionKind::MakeTuple { .. } => {
                self.errors.push(CasmValidationError {
                    message: "MakeTuple instruction found in MIR - this should be lowered to memory operations before CASM codegen".to_string(),
                    instruction_location: Some((block_id, instr_index)),
                    suggested_fix: Some("Ensure LowerAggregatesPass runs before CASM codegen".to_string()),
                });
            }

            InstructionKind::ExtractTupleElement { .. } => {
                self.errors.push(CasmValidationError {
                    message: "ExtractTupleElement instruction found in MIR - this should be lowered to load operations".to_string(),
                    instruction_location: Some((block_id, instr_index)),
                    suggested_fix: Some("Ensure LowerAggregatesPass runs before CASM codegen".to_string()),
                });
            }

            InstructionKind::MakeStruct { .. } => {
                self.errors.push(CasmValidationError {
                    message: "MakeStruct instruction found in MIR - this should be lowered to memory operations before CASM codegen".to_string(),
                    instruction_location: Some((block_id, instr_index)),
                    suggested_fix: Some("Ensure LowerAggregatesPass runs before CASM codegen".to_string()),
                });
            }

            InstructionKind::ExtractStructField { .. } => {
                self.errors.push(CasmValidationError {
                    message: "ExtractStructField instruction found in MIR - this should be lowered to load operations".to_string(),
                    instruction_location: Some((block_id, instr_index)),
                    suggested_fix: Some("Ensure LowerAggregatesPass runs before CASM codegen".to_string()),
                });
            }

            InstructionKind::InsertField { .. } => {
                self.errors.push(CasmValidationError {
                    message: "InsertField instruction found in MIR - this should be lowered to memory operations".to_string(),
                    instruction_location: Some((block_id, instr_index)),
                    suggested_fix: Some("Ensure LowerAggregatesPass runs before CASM codegen".to_string()),
                });
            }

            InstructionKind::InsertTuple { .. } => {
                self.errors.push(CasmValidationError {
                    message: "InsertTuple instruction found in MIR - this should be lowered to memory operations".to_string(),
                    instruction_location: Some((block_id, instr_index)),
                    suggested_fix: Some("Ensure LowerAggregatesPass runs before CASM codegen".to_string()),
                });
            }

            // Check Phi nodes with aggregate types
            InstructionKind::Phi { ty, .. } => {
                if Self::is_forbidden_aggregate_type(ty) {
                    self.errors.push(CasmValidationError {
                        message: format!("Phi node with aggregate type {:?} found - aggregate Phi nodes should be eliminated before CASM codegen", ty),
                        instruction_location: Some((block_id, instr_index)),
                        suggested_fix: Some("Ensure aggregate Phi nodes are lowered to memory-based operations".to_string()),
                    });
                }
            }

            // All other instructions are acceptable
            _ => {}
        }
    }

    /// Get detailed error report for debugging
    pub fn get_error_report(&self, function_name: &str) -> String {
        if self.errors.is_empty() {
            return format!("✓ Function '{}' is CASM-compatible", function_name);
        }

        let mut report = format!("✗ Function '{}' has {} CASM compatibility issues:\n\n", function_name, self.errors.len());

        for (i, error) in self.errors.iter().enumerate() {
            report.push_str(&format!("{}. {}\n", i + 1, error.message));

            if let Some((block_id, instr_idx)) = error.instruction_location {
                report.push_str(&format!("   Location: Block {:?}, Instruction {}\n", block_id, instr_idx));
            }

            if let Some(suggested_fix) = &error.suggested_fix {
                report.push_str(&format!("   Fix: {}\n", suggested_fix));
            }

            report.push('\n');
        }

        report
    }
}

impl MirPass for CasmCompatibilityValidation {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        self.errors.clear();

        // Validate all instructions in all blocks
        for (block_id, block) in function.basic_blocks() {
            for (instr_index, instruction) in block.instructions.iter().enumerate() {
                self.validate_instruction(instruction, block_id, instr_index);
            }
        }

        // Report errors if any found
        if !self.errors.is_empty() {
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!("{}", self.get_error_report(&function.name));
            }
        }

        false // Validation passes don't modify the function
    }

    fn name(&self) -> &'static str {
        "CasmCompatibilityValidation"
    }
}
```

### 3. Error Message Design

The error messages follow these principles:

1. **Clear Description**: Explain what instruction was found and why it's
   problematic
2. **Context**: Provide exact location (block ID, instruction index)
3. **Actionable Fix**: Suggest specific remediation steps
4. **Batch Reporting**: Collect all errors before reporting for comprehensive
   feedback

Example error output:

```
✗ Function 'test_func' has 2 CASM compatibility issues:

1. MakeTuple instruction found in MIR - this should be lowered to memory operations before CASM codegen
   Location: Block BasicBlock(0), Instruction 2
   Fix: Ensure LowerAggregatesPass runs before CASM codegen

2. ExtractStructField instruction found in MIR - this should be lowered to load operations
   Location: Block BasicBlock(1), Instruction 0
   Fix: Ensure LowerAggregatesPass runs before CASM codegen
```

### 4. Integration with Pipeline

The validation should be added at the very end of the optimization pipeline,
just before CASM codegen:

#### Option A: Modify Standard Pipeline

Update `PassManager::standard_pipeline()` in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs`:

```rust
pub fn standard_pipeline() -> Self {
    Self::new()
        // ... existing passes ...
        // 8. Validate the final, lowered MIR.
        .add_pass(Validation::new_post_ssa())
        // 9. CASM compatibility validation (NEW)
        .add_pass(CasmCompatibilityValidation::new())
}
```

#### Option B: Separate CASM Pipeline

Create a new pipeline method specifically for CASM targets:

```rust
/// Create a CASM-compatible optimization pipeline
pub fn casm_pipeline() -> Self {
    Self::standard_pipeline()
        .add_pass(CasmCompatibilityValidation::new())
}
```

### 5. Integration with Codegen

Update the code generator to use the validation-aware pipeline. In
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/pipeline.rs`, add
CASM-specific optimization level:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,
    Basic,
    Standard,
    Aggressive,
    CasmCompatible, // NEW - ensures CASM compatibility
}

pub fn optimize_module(module: &mut MirModule, config: &PipelineConfig) {
    let mut pass_manager = match config.optimization_level {
        OptimizationLevel::None => return,
        OptimizationLevel::Basic => PassManager::basic_pipeline(),
        OptimizationLevel::Standard => PassManager::standard_pipeline(),
        OptimizationLevel::Aggressive => PassManager::aggressive_pipeline(),
        OptimizationLevel::CasmCompatible => PassManager::casm_pipeline(), // NEW
    };

    // ... rest of function
}
```

## Test Cases for Validation

### Test Case 1: Valid Memory-Based MIR

```rust
#[test]
fn test_casm_compatible_mir() {
    let mut function = create_test_function();
    let mut validation = CasmCompatibilityValidation::new();

    // Add memory-based operations
    let block = function.get_basic_block_mut(entry_block).unwrap();
    block.instructions.push(Instruction::frame_alloc(ptr_val, tuple_type));
    block.instructions.push(Instruction::store(ptr_val, elem_val, elem_type));
    block.instructions.push(Instruction::load(dest_val, elem_type, ptr_val));

    let modified = validation.run(&mut function);
    assert!(!modified);
    assert!(validation.errors.is_empty());
}
```

### Test Case 2: Invalid Aggregate Operations

```rust
#[test]
fn test_forbidden_aggregate_instructions() {
    let mut function = create_test_function();
    let mut validation = CasmCompatibilityValidation::new();

    // Add forbidden aggregate operations
    let block = function.get_basic_block_mut(entry_block).unwrap();
    block.instructions.push(Instruction::make_tuple(tuple_val, elements));
    block.instructions.push(Instruction::extract_tuple_element(dest, tuple_val, 0, elem_type));

    let modified = validation.run(&mut function);
    assert!(!modified);
    assert_eq!(validation.errors.len(), 2);
    assert!(validation.errors[0].message.contains("MakeTuple"));
    assert!(validation.errors[1].message.contains("ExtractTupleElement"));
}
```

### Test Case 3: Forbidden Aggregate Phi Nodes

```rust
#[test]
fn test_aggregate_phi_validation() {
    let mut function = create_test_function_with_branches();
    let mut validation = CasmCompatibilityValidation::new();

    // Add Phi node with aggregate type
    let phi_block = function.add_basic_block();
    let phi_val = function.new_typed_value_id(MirType::Tuple(vec![MirType::felt()]));
    let block = function.get_basic_block_mut(phi_block).unwrap();
    block.instructions.push(Instruction::phi(
        phi_val,
        MirType::Tuple(vec![MirType::felt()]),
        vec![(block1, value1), (block2, value2)]
    ));

    let modified = validation.run(&mut function);
    assert!(!modified);
    assert_eq!(validation.errors.len(), 1);
    assert!(validation.errors[0].message.contains("Phi node with aggregate type"));
}
```

### Test Case 4: After LowerAggregates Pass

```rust
#[test]
fn test_after_lowering_pass() {
    let mut function = create_function_with_aggregates();

    // Apply lowering pass
    let mut lower_pass = LowerAggregatesPass::new();
    let modified = lower_pass.run(&mut function);
    assert!(modified);

    // Should now pass CASM validation
    let mut validation = CasmCompatibilityValidation::new();
    let modified = validation.run(&mut function);
    assert!(!modified);
    assert!(validation.errors.is_empty());
}
```

## Benefits of This Approach

1. **Early Error Detection**: Catches aggregate operation issues at the MIR
   level with clear diagnostics
2. **Better Error Messages**: Provides specific locations and actionable fixes
   instead of generic codegen failures
3. **Debugging Support**: Detailed error reports help developers understand
   pipeline issues
4. **Pipeline Flexibility**: Can be enabled/disabled based on target backend
   requirements
5. **Comprehensive Coverage**: Catches all forms of aggregate operations
   including Phi nodes
6. **Zero Runtime Cost**: Validation is compile-time only and doesn't affect
   generated code performance

## Future Extensions

1. **Backend-Agnostic**: Could be extended for other backends with different
   restrictions
2. **Configurable Rules**: Could allow different validation rules for different
   compilation targets
3. **Integration with IDE**: Error locations could be mapped back to source code
   for IDE integration
4. **Performance Metrics**: Could collect statistics on aggregate operation
   usage patterns
