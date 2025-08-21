# Strengthen MIR Validation Pass

**Priority**: MEDIUM  
**Component**: MIR Validation  
**Impact**: Correctness, Debugging

## Problem

The current MIR validation is incomplete and misses several categories of errors
that could lead to silent miscompilation or runtime failures:

### Current Validation Gaps

1. **Incomplete SSA validation**: Missing checks for proper dominance
   relationships
2. **Type consistency**: No validation that operations match operand types
3. **Control flow integrity**: Missing validation of unreachable blocks and
   malformed CFG
4. **Memory safety**: No checks for invalid memory access patterns
5. **Value lifetime**: Missing validation of value use-def relationships

### Specific Issues

**File**: `crates/compiler/mir/src/function.rs` (around line 200)

Current validation is minimal:

```rust
pub fn validate(&self) -> Result<(), String> {
    // Very basic checks only
    if self.basic_blocks.is_empty() {
        return Err("Function has no basic blocks".to_string());
    }

    // Missing many important validations
    Ok(())
}
```

### Impact

- **Silent bugs**: Invalid MIR passes validation and causes issues later
- **Debugging difficulty**: Errors surface far from their source
- **Optimization unsoundness**: Passes may rely on invariants that aren't
  validated
- **Runtime failures**: Invalid MIR reaches codegen and causes crashes

## Solution

### Comprehensive Validation Framework

**Update**: `crates/compiler/mir/src/validation.rs`

```rust
use std::collections::{HashMap, HashSet};
use crate::{MirFunction, ValueId, BlockId, MirType, BinaryOp, UnaryOp};

/// Comprehensive MIR validation
#[derive(Debug)]
pub struct MirValidator {
    /// Current function being validated
    function: &MirFunction,
    /// Errors found during validation
    errors: Vec<ValidationError>,
    /// Warnings found during validation
    warnings: Vec<ValidationWarning>,
    /// Value definitions discovered
    value_definitions: HashMap<ValueId, BlockId>,
    /// Value uses discovered
    value_uses: HashMap<ValueId, Vec<UseSite>>,
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    // SSA Form Violations
    MultipleDefinitions { value: ValueId, blocks: Vec<BlockId> },
    UseBeforeDefinition { value: ValueId, use_site: UseSite, def_site: Option<BlockId> },
    UndefinedValue { value: ValueId, use_site: UseSite },

    // Type System Violations
    TypeMismatch { expected: MirType, actual: MirType, location: InstructionLocation },
    InvalidOperandType { operation: String, operand_type: MirType, location: InstructionLocation },
    InvalidResultType { operation: String, result_type: MirType, location: InstructionLocation },

    // Control Flow Violations
    UnreachableBlock { block: BlockId },
    MissingTerminator { block: BlockId },
    InvalidTerminatorTarget { block: BlockId, target: BlockId },
    MalformedPhiNode { block: BlockId, phi_value: ValueId },

    // Memory Safety Violations
    InvalidMemoryAccess { location: InstructionLocation, reason: String },
    DoubleFree { location: InstructionLocation, value: ValueId },
    UseAfterFree { location: InstructionLocation, value: ValueId },

    // General Structural Issues
    EmptyFunction,
    MissingEntryBlock,
    InvalidEntryBlock { block: BlockId },
    OrphanedInstruction { instruction_id: InstructionId },
}

#[derive(Debug, Clone)]
pub enum ValidationWarning {
    UnusedValue { value: ValueId, definition: BlockId },
    SuboptimalPattern { location: InstructionLocation, suggestion: String },
    DeadCode { block: BlockId },
}

#[derive(Debug, Clone)]
pub struct UseSite {
    pub block: BlockId,
    pub instruction: Option<InstructionId>,
    pub operand_index: usize,
}

#[derive(Debug, Clone)]
pub struct InstructionLocation {
    pub block: BlockId,
    pub instruction: InstructionId,
}

impl MirValidator {
    pub fn new(function: &MirFunction) -> Self {
        Self {
            function,
            errors: Vec::new(),
            warnings: Vec::new(),
            value_definitions: HashMap::new(),
            value_uses: HashMap::new(),
        }
    }

    /// Run comprehensive validation
    pub fn validate(mut self) -> ValidationResult {
        // Phase 1: Structural validation
        self.validate_structure();

        // Phase 2: Build use-def information
        self.build_use_def_info();

        // Phase 3: SSA form validation
        self.validate_ssa_form();

        // Phase 4: Type system validation
        self.validate_types();

        // Phase 5: Control flow validation
        self.validate_control_flow();

        // Phase 6: Memory safety validation
        self.validate_memory_safety();

        // Phase 7: Optimization opportunity warnings
        self.detect_optimization_opportunities();

        ValidationResult {
            errors: self.errors,
            warnings: self.warnings,
            is_valid: self.errors.is_empty(),
        }
    }

    /// Validate basic structural requirements
    fn validate_structure(&mut self) {
        if self.function.basic_blocks.is_empty() {
            self.errors.push(ValidationError::EmptyFunction);
            return;
        }

        if self.function.entry_block.is_none() {
            self.errors.push(ValidationError::MissingEntryBlock);
        } else if let Some(entry) = self.function.entry_block {
            if !self.function.basic_blocks.contains_key(&entry) {
                self.errors.push(ValidationError::InvalidEntryBlock { block: entry });
            }
        }

        // Validate each block has a terminator
        for (block_id, block) in &self.function.basic_blocks {
            if block.terminator.is_none() {
                self.errors.push(ValidationError::MissingTerminator { block: *block_id });
            }
        }
    }

    /// Build comprehensive use-def information
    fn build_use_def_info(&mut self) {
        for (block_id, block) in &self.function.basic_blocks {
            // Record definitions from instructions
            for instruction in &block.instructions {
                if let Some(defined_value) = instruction.get_defined_value() {
                    if let Some(existing_block) = self.value_definitions.insert(defined_value, *block_id) {
                        self.errors.push(ValidationError::MultipleDefinitions {
                            value: defined_value,
                            blocks: vec![existing_block, *block_id],
                        });
                    }
                }

                // Record uses from operands
                for (operand_index, operand) in instruction.get_operands().iter().enumerate() {
                    if let Some(used_value) = operand.as_value_id() {
                        self.value_uses.entry(used_value).or_insert_with(Vec::new).push(UseSite {
                            block: *block_id,
                            instruction: Some(instruction.id),
                            operand_index,
                        });
                    }
                }
            }

            // Record uses from terminator
            if let Some(terminator) = &block.terminator {
                for (operand_index, operand) in terminator.get_operands().iter().enumerate() {
                    if let Some(used_value) = operand.as_value_id() {
                        self.value_uses.entry(used_value).or_insert_with(Vec::new).push(UseSite {
                            block: *block_id,
                            instruction: None,
                            operand_index,
                        });
                    }
                }
            }
        }
    }

    /// Validate SSA form properties
    fn validate_ssa_form(&mut self) {
        // Check each use has a corresponding definition
        for (value_id, use_sites) in &self.value_uses {
            if !self.value_definitions.contains_key(value_id) {
                for use_site in use_sites {
                    self.errors.push(ValidationError::UndefinedValue {
                        value: *value_id,
                        use_site: use_site.clone(),
                    });
                }
                continue;
            }

            let def_block = self.value_definitions[value_id];

            // Check dominance relationships for uses
            for use_site in use_sites {
                if !self.dominates(def_block, use_site.block) {
                    self.errors.push(ValidationError::UseBeforeDefinition {
                        value: *value_id,
                        use_site: use_site.clone(),
                        def_site: Some(def_block),
                    });
                }
            }
        }

        // Check for unused values (warning only)
        for (value_id, def_block) in &self.value_definitions {
            if !self.value_uses.contains_key(value_id) {
                self.warnings.push(ValidationWarning::UnusedValue {
                    value: *value_id,
                    definition: *def_block,
                });
            }
        }
    }

    /// Validate type system consistency
    fn validate_types(&mut self) {
        for (block_id, block) in &self.function.basic_blocks {
            for instruction in &block.instructions {
                self.validate_instruction_types(*block_id, instruction);
            }
        }
    }

    /// Validate types for a specific instruction
    fn validate_instruction_types(&mut self, block_id: BlockId, instruction: &Instruction) {
        let location = InstructionLocation {
            block: block_id,
            instruction: instruction.id,
        };

        match &instruction.kind {
            InstructionKind::BinaryOp { op, dest, left, right } => {
                let left_type = self.get_value_type(left);
                let right_type = self.get_value_type(right);
                let dest_type = self.get_value_type(&Value::operand(*dest));

                // Validate operand types match operation requirements
                if !self.is_valid_binary_op_operand(*op, &left_type) {
                    self.errors.push(ValidationError::InvalidOperandType {
                        operation: format!("BinaryOp::{:?} left operand", op),
                        operand_type: left_type,
                        location: location.clone(),
                    });
                }

                if !self.is_valid_binary_op_operand(*op, &right_type) {
                    self.errors.push(ValidationError::InvalidOperandType {
                        operation: format!("BinaryOp::{:?} right operand", op),
                        operand_type: right_type,
                        location: location.clone(),
                    });
                }

                // Validate result type matches operation
                let expected_result_type = op.result_type();
                if dest_type != expected_result_type {
                    self.errors.push(ValidationError::TypeMismatch {
                        expected: expected_result_type,
                        actual: dest_type,
                        location,
                    });
                }
            }

            InstructionKind::UnaryOp { op, dest, source } => {
                let source_type = self.get_value_type(source);
                let dest_type = self.get_value_type(&Value::operand(*dest));

                if !self.is_valid_unary_op_operand(*op, &source_type) {
                    self.errors.push(ValidationError::InvalidOperandType {
                        operation: format!("UnaryOp::{:?}", op),
                        operand_type: source_type,
                        location: location.clone(),
                    });
                }

                let expected_result_type = self.get_unary_op_result_type(*op, &source_type);
                if dest_type != expected_result_type {
                    self.errors.push(ValidationError::TypeMismatch {
                        expected: expected_result_type,
                        actual: dest_type,
                        location,
                    });
                }
            }

            _ => {
                // Validate other instruction types
            }
        }
    }

    /// Check if a value dominates another in the control flow graph
    fn dominates(&self, def_block: BlockId, use_block: BlockId) -> bool {
        if def_block == use_block {
            return true; // A block dominates itself
        }

        // For now, simplified dominance check
        // TODO: Implement proper dominance analysis
        true
    }

    /// Validate control flow graph structure
    fn validate_control_flow(&mut self) {
        // Check for unreachable blocks
        let reachable_blocks = self.compute_reachable_blocks();

        for block_id in self.function.basic_blocks.keys() {
            if !reachable_blocks.contains(block_id) {
                self.warnings.push(ValidationWarning::DeadCode { block: *block_id });
            }
        }

        // Validate terminator targets exist
        for (block_id, block) in &self.function.basic_blocks {
            if let Some(terminator) = &block.terminator {
                for target in terminator.get_successors() {
                    if !self.function.basic_blocks.contains_key(&target) {
                        self.errors.push(ValidationError::InvalidTerminatorTarget {
                            block: *block_id,
                            target,
                        });
                    }
                }
            }
        }
    }

    /// Compute set of reachable blocks from entry
    fn compute_reachable_blocks(&self) -> HashSet<BlockId> {
        let mut reachable = HashSet::new();
        let mut worklist = Vec::new();

        if let Some(entry) = self.function.entry_block {
            worklist.push(entry);
            reachable.insert(entry);
        }

        while let Some(block_id) = worklist.pop() {
            if let Some(block) = self.function.basic_blocks.get(&block_id) {
                if let Some(terminator) = &block.terminator {
                    for successor in terminator.get_successors() {
                        if reachable.insert(successor) {
                            worklist.push(successor);
                        }
                    }
                }
            }
        }

        reachable
    }

    /// Validate memory safety properties
    fn validate_memory_safety(&mut self) {
        // TODO: Implement memory safety validation
        // - Check for use-after-free
        // - Validate memory access bounds
        // - Check for double-free patterns
    }

    /// Detect optimization opportunities and emit warnings
    fn detect_optimization_opportunities(&mut self) {
        // TODO: Implement optimization opportunity detection
        // - Detect constant folding opportunities
        // - Find unused computations
        // - Identify inefficient patterns
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub is_valid: bool,
}

impl ValidationResult {
    pub fn print_diagnostics(&self) {
        for error in &self.errors {
            eprintln!("Error: {:?}", error);
        }

        for warning in &self.warnings {
            eprintln!("Warning: {:?}", warning);
        }
    }
}
```

### Enhanced Function Validation

**Update**: `crates/compiler/mir/src/function.rs`

```rust
impl MirFunction {
    /// Comprehensive validation using new framework
    pub fn validate(&self) -> Result<(), ValidationResult> {
        let validator = MirValidator::new(self);
        let result = validator.validate();

        if result.is_valid {
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Quick validation for hot paths
    pub fn validate_quick(&self) -> Result<(), String> {
        if self.basic_blocks.is_empty() {
            return Err("Function has no basic blocks".to_string());
        }

        if self.entry_block.is_none() {
            return Err("Function has no entry block".to_string());
        }

        Ok(())
    }
}
```

## Files to Modify

- **New**: `crates/compiler/mir/src/validation.rs` - Comprehensive validation
  framework
- **Update**: `crates/compiler/mir/src/function.rs` - Enhanced validate() method
- **Update**: `crates/compiler/mir/src/lib.rs` - Add validation module
- **New**: `crates/compiler/mir/src/validation_tests.rs` - Comprehensive test
  suite

## Implementation Plan

### Phase 1: Validation Framework

1. Create ValidationError and ValidationWarning enums
2. Implement MirValidator struct with basic infrastructure
3. Add use-def analysis building

### Phase 2: Core Validations

1. Implement SSA form validation
2. Add type system validation
3. Implement control flow validation

### Phase 3: Advanced Features

1. Add dominance analysis
2. Implement memory safety checks
3. Add optimization opportunity detection

### Phase 4: Integration and Testing

1. Update function validation methods
2. Add comprehensive test suite
3. Integrate with pass pipeline

## Test Strategy

```rust
#[test]
fn test_ssa_violation_detection() {
    let function = create_function_with_ssa_violation();

    let result = function.validate();
    assert!(result.is_err());

    let validation_result = result.unwrap_err();
    assert!(validation_result.errors.iter().any(|e| {
        matches!(e, ValidationError::UseBeforeDefinition { .. })
    }));
}

#[test]
fn test_type_mismatch_detection() {
    let function = create_function_with_type_mismatch();

    let result = function.validate();
    assert!(result.is_err());

    let validation_result = result.unwrap_err();
    assert!(validation_result.errors.iter().any(|e| {
        matches!(e, ValidationError::TypeMismatch { .. })
    }));
}

#[test]
fn test_control_flow_validation() {
    let function = create_function_with_invalid_control_flow();

    let result = function.validate();
    assert!(result.is_err());

    let validation_result = result.unwrap_err();
    assert!(validation_result.errors.iter().any(|e| {
        matches!(e, ValidationError::InvalidTerminatorTarget { .. })
    }));
}

#[test]
fn test_valid_function_passes() {
    let function = create_valid_function();

    let result = function.validate();
    assert!(result.is_ok());
}
```

## Benefits

1. **Early error detection**: Catch MIR errors close to their source
2. **Optimization soundness**: Ensure passes maintain required invariants
3. **Better debugging**: Clear error messages for invalid MIR
4. **Development velocity**: Faster identification of bugs during development
5. **Correctness assurance**: High confidence in MIR transformations

## Dependencies

- May need enhanced instruction introspection APIs
- Should coordinate with pass infrastructure improvements

## Acceptance Criteria

- [ ] Comprehensive validation covering SSA form, types, and control flow
- [ ] Clear error messages with source location information
- [ ] Performance acceptable for use in development builds
- [ ] Integration with existing validation points
- [ ] Comprehensive test coverage of all validation categories
- [ ] Optional validation levels (quick vs full)
- [ ] Warning system for optimization opportunities
- [ ] Documentation of all validation rules
