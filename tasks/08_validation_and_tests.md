# Task 8: Add Comprehensive Validation and Tests

## Goal

Add comprehensive validation for SSA invariants and create thorough test suites
for the new SSA construction.

## Files to Create

- `mir/src/ssa_tests.rs` - SSA builder unit tests
- `mir/src/validation_tests.rs` - Validation tests for SSA invariants
- `mir/tests/ssa_integration_tests.rs` - Integration tests

## Files to Modify

- `mir/src/lib.rs` - Add test modules
- `mir/src/function.rs` - Extend validation
- `mir/src/basic_block.rs` - Add validation helpers

## Current State

Basic MIR validation exists but doesn't check SSA-specific invariants.

## Required Changes

### 1. Extend MirFunction Validation (`mir/src/function.rs`)

Add comprehensive SSA validation:

```rust
impl MirFunction {
    /// Validate SSA form and related invariants
    pub fn validate_ssa(&self) -> Result<(), String> {
        // Call existing validation first
        self.validate()?;

        // Check SSA-specific invariants
        self.validate_phi_placement()?;
        self.validate_domination()?;
        self.validate_value_uniqueness()?;

        Ok(())
    }

    /// Validate phi placement invariants
    fn validate_phi_placement(&self) -> Result<(), String> {
        for (block_id, block) in self.basic_blocks() {
            // Check that all phis come before non-phis (already checked in basic validation)

            // Check that phi operands match predecessors
            for phi_instr in block.phi_instructions() {
                if let InstructionKind::Phi { dest, sources, .. } = &phi_instr.kind {
                    // If block is sealed, must have operand from each predecessor
                    if block.sealed {
                        let mut pred_blocks: std::collections::HashSet<_> = block.preds.iter().collect();
                        let mut source_blocks: std::collections::HashSet<_> = sources.iter().map(|(b, _)| b).collect();

                        if pred_blocks != source_blocks {
                            return Err(format!(
                                "Block {:?}: Phi {:?} has operands from {:?} but predecessors are {:?}",
                                block_id, dest, source_blocks, pred_blocks
                            ));
                        }
                    }

                    // Check that all source blocks are actually predecessors
                    for (source_block, _) in sources {
                        if !block.preds.contains(source_block) {
                            return Err(format!(
                                "Block {:?}: Phi {:?} has operand from non-predecessor {:?}",
                                block_id, dest, source_block
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate that each value is defined exactly once (SSA property)
    fn validate_value_uniqueness(&self) -> Result<(), String> {
        let mut defined_values = std::collections::HashSet::new();

        for (block_id, block) in self.basic_blocks() {
            for (instr_idx, instruction) in block.instructions.iter().enumerate() {
                if let Some(dest) = instruction.destination() {
                    if !defined_values.insert(dest) {
                        return Err(format!(
                            "Block {:?}, instruction {}: Value {:?} defined multiple times (SSA violation)",
                            block_id, instr_idx, dest
                        ));
                    }
                }
            }
        }

        // Check that defined_values field matches actual definitions
        if defined_values != self.defined_values {
            return Err("defined_values field doesn't match actual value definitions".to_string());
        }

        Ok(())
    }

    /// Basic domination check (simplified - not full dominator tree)
    fn validate_domination(&self) -> Result<(), String> {
        // For each value use, check that the definition dominates the use
        // This is a simplified check - a full implementation would build dominator tree

        for (block_id, block) in self.basic_blocks() {
            // Check that all used values are defined somewhere
            for used_value in block.used_values() {
                if !self.defined_values.contains(&used_value) {
                    return Err(format!(
                        "Block {:?}: Uses undefined value {:?}",
                        block_id, used_value
                    ));
                }
            }
        }

        Ok(())
    }
}
```

### 2. Add Validation Helpers to BasicBlock (`mir/src/basic_block.rs`)

```rust
impl BasicBlock {
    /// Validate block-specific SSA invariants
    pub fn validate_ssa(&self) -> Result<(), String> {
        // Check phi-first ordering
        let mut seen_non_phi = false;
        for (i, instruction) in self.instructions.iter().enumerate() {
            match &instruction.kind {
                InstructionKind::Phi { .. } => {
                    if seen_non_phi {
                        return Err(format!(
                            "Phi instruction at position {} found after non-phi instruction",
                            i
                        ));
                    }
                }
                _ => {
                    seen_non_phi = true;
                }
            }
        }

        // Check sealed/filled invariants
        if self.sealed && !self.filled {
            // This might be OK during construction, but worth noting
        }

        Ok(())
    }

    /// Check if this block is ready for sealing
    pub fn can_be_sealed(&self) -> bool {
        // A block can be sealed if all its predecessors have been determined
        // This is context-dependent, so this is just a basic check
        self.filled
    }
}
```

### 3. Create SSA Builder Unit Tests (`mir/src/ssa_tests.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirFunction, MirType, Value, BasicBlockId, MirDefinitionId, ssa::SSABuilder};

    fn test_mir_def_id(idx: usize) -> MirDefinitionId {
        MirDefinitionId {
            definition_index: idx,
            file_id: 0,
        }
    }

    #[test]
    fn test_write_read_variable() {
        let mut function = MirFunction::new("test".to_string());
        let entry_block = function.entry_block;
        let mut ssa = SSABuilder::new(&mut function);

        let var = test_mir_def_id(0);
        let value = function.new_typed_value_id(MirType::Felt);

        // Write and read back
        ssa.write_variable(var, entry_block, value);
        let read_value = ssa.read_variable(var, entry_block);

        assert_eq!(value, read_value);
    }

    #[test]
    fn test_undefined_variable_read() {
        let mut function = MirFunction::new("test".to_string());
        let entry_block = function.entry_block;
        let mut ssa = SSABuilder::new(&mut function);

        let var = test_mir_def_id(0);

        // Reading undefined variable should create error value
        let value = ssa.read_variable(var, entry_block);
        assert!(function.get_value_type(value).is_some());
    }

    #[test]
    fn test_single_predecessor_phi() {
        let mut function = MirFunction::new("test".to_string());
        let entry_block = function.entry_block;
        let merge_block = function.add_basic_block();

        // Set up CFG: entry -> merge
        function.connect(entry_block, merge_block);

        let mut ssa = SSABuilder::new(&mut function);

        let var = test_mir_def_id(0);
        let value = function.new_typed_value_id(MirType::Felt);

        // Write in entry block
        ssa.write_variable(var, entry_block, value);

        // Seal merge block
        ssa.seal_block(merge_block);

        // Read from merge block should get same value (no phi needed)
        let read_value = ssa.read_variable(var, merge_block);
        assert_eq!(value, read_value);

        // Should not have created any phi instructions
        assert_eq!(function.basic_blocks[merge_block].phi_count(), 0);
    }

    #[test]
    fn test_multiple_predecessor_phi() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;
        let left = function.add_basic_block();
        let right = function.add_basic_block();
        let merge = function.add_basic_block();

        // Set up diamond CFG
        function.connect(entry, left);
        function.connect(entry, right);
        function.connect(left, merge);
        function.connect(right, merge);

        let mut ssa = SSABuilder::new(&mut function);

        let var = test_mir_def_id(0);
        let left_value = function.new_typed_value_id(MirType::Felt);
        let right_value = function.new_typed_value_id(MirType::Felt);

        // Write different values in left and right
        ssa.write_variable(var, left, left_value);
        ssa.write_variable(var, right, right_value);

        // Seal merge block
        ssa.seal_block(merge);

        // Read from merge should create phi
        let phi_value = ssa.read_variable(var, merge);

        // Should have created a phi instruction
        assert_eq!(function.basic_blocks[merge].phi_count(), 1);

        // Phi should have operands from both predecessors
        let phi_instr = function.basic_blocks[merge].find_phi(phi_value).unwrap();
        if let InstructionKind::Phi { sources, .. } = &phi_instr.kind {
            assert_eq!(sources.len(), 2);

            let source_map: std::collections::HashMap<_, _> = sources.iter().cloned().collect();
            assert_eq!(source_map[&left], Value::Operand(left_value));
            assert_eq!(source_map[&right], Value::Operand(right_value));
        } else {
            panic!("Expected phi instruction");
        }
    }

    #[test]
    fn test_incomplete_phi_completion() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;
        let merge = function.add_basic_block();

        // Don't connect edges yet - unsealed
        let mut ssa = SSABuilder::new(&mut function);

        let var = test_mir_def_id(0);

        // Read from unsealed block should create incomplete phi
        let phi_value = ssa.read_variable(var, merge);

        // Should have phi with no operands yet
        let phi_instr = function.basic_blocks[merge].find_phi(phi_value).unwrap();
        if let InstructionKind::Phi { sources, .. } = &phi_instr.kind {
            assert_eq!(sources.len(), 0);
        }

        // Now set up predecessor and seal
        let value = function.new_typed_value_id(MirType::Felt);
        ssa.write_variable(var, entry, value);
        function.connect(entry, merge);
        ssa.seal_block(merge);

        // Phi should now be completed
        let phi_instr = function.basic_blocks[merge].find_phi(phi_value).unwrap();
        if let InstructionKind::Phi { sources, .. } = &phi_instr.kind {
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0], (entry, Value::Operand(value)));
        }
    }

    #[test]
    fn test_trivial_phi_elimination() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.entry_block;
        let left = function.add_basic_block();
        let right = function.add_basic_block();
        let merge = function.add_basic_block();

        // Set up diamond CFG
        function.connect(entry, left);
        function.connect(entry, right);
        function.connect(left, merge);
        function.connect(right, merge);

        let mut ssa = SSABuilder::new(&mut function);

        let var = test_mir_def_id(0);
        let same_value = function.new_typed_value_id(MirType::Felt);

        // Write same value in both branches
        ssa.write_variable(var, left, same_value);
        ssa.write_variable(var, right, same_value);

        // Seal merge block
        ssa.seal_block(merge);

        // Read should return the same value (phi eliminated)
        let read_value = ssa.read_variable(var, merge);
        assert_eq!(read_value, same_value);

        // Should not have any phi instructions (eliminated)
        assert_eq!(function.basic_blocks[merge].phi_count(), 0);
    }

    #[test]
    fn test_nested_blocks() {
        // Test more complex CFG patterns
        let mut function = MirFunction::new("test".to_string());
        // ... more complex test scenarios
    }
}
```

### 4. Create Integration Tests (`mir/tests/ssa_integration_tests.rs`)

```rust
//! Integration tests for SSA construction with actual lowering

use cairo_m_compiler_mir::*;
use cairo_m_compiler_semantic::*;
use cairo_m_compiler_parser::*;

#[test]
fn test_if_statement_phi_creation() {
    let source = r#"
        fn test_if(x: felt) -> felt {
            let y: felt;
            if x > 0 {
                y = x + 1;
            } else {
                y = x - 1;
            }
            return y;
        }
    "#;

    // Parse and analyze
    // ... semantic analysis setup ...

    // Lower to MIR
    // ... lowering setup ...

    // Validate that phi node was created for variable `y` at merge point
    // ... validation ...
}

#[test]
fn test_while_loop_phi_creation() {
    let source = r#"
        fn test_while(n: felt) -> felt {
            let sum = 0;
            let i = 0;
            while i < n {
                sum = sum + i;
                i = i + 1;
            }
            return sum;
        }
    "#;

    // Should create phi nodes for `sum` and `i` at loop header
    // ... test implementation ...
}

#[test]
fn test_nested_control_flow() {
    let source = r#"
        fn test_nested(x: felt, y: felt) -> felt {
            let result = 0;
            if x > 0 {
                if y > 0 {
                    result = x + y;
                } else {
                    result = x - y;
                }
            } else {
                result = -x;
            }
            return result;
        }
    "#;

    // Should handle nested phi nodes correctly
    // ... test implementation ...
}
```

### 5. Create Validation Test Suite (`mir/src/validation_tests.rs`)

```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_phi_ordering_validation() {
        let mut function = MirFunction::new("test".to_string());
        let block_id = function.add_basic_block();

        // Add a regular instruction first
        let value1 = function.new_typed_value_id(MirType::Felt);
        let assign = Instruction::assign(value1, Value::Literal(Literal::Felt(42)), MirType::Felt);
        function.basic_blocks[block_id].push_instruction(assign);

        // Try to add phi after regular instruction (should be invalid)
        let value2 = function.new_typed_value_id(MirType::Felt);
        let phi = Instruction::empty_phi(value2, MirType::Felt);
        function.basic_blocks[block_id].push_instruction(phi);

        // Validation should fail
        assert!(function.validate_ssa().is_err());
    }

    #[test]
    fn test_sealed_block_phi_validation() {
        let mut function = MirFunction::new("test".to_string());
        let pred = function.entry_block;
        let block = function.add_basic_block();

        function.connect(pred, block);
        function.basic_blocks[block].seal();

        // Create phi with wrong number of operands
        let phi_value = function.new_typed_value_id(MirType::Felt);
        let mut phi = Instruction::empty_phi(phi_value, MirType::Felt);
        // Don't add any operands (should have 1 for the predecessor)

        function.basic_blocks[block].push_phi_front(phi);

        // Validation should fail
        assert!(function.validate_ssa().is_err());
    }

    #[test]
    fn test_ssa_form_validation() {
        let mut function = MirFunction::new("test".to_string());
        let block = function.entry_block;

        // Create two instructions that define the same value (SSA violation)
        let value = function.new_typed_value_id(MirType::Felt);
        let instr1 = Instruction::assign(value, Value::Literal(Literal::Felt(1)), MirType::Felt);
        let instr2 = Instruction::assign(value, Value::Literal(Literal::Felt(2)), MirType::Felt);

        function.basic_blocks[block].push_instruction(instr1);
        function.basic_blocks[block].push_instruction(instr2);

        // This should fail during instruction creation, but test validation too
        assert!(function.validate_ssa().is_err());
    }
}
```

### 6. Add Test Modules to lib.rs

```rust
// In mir/src/lib.rs:
#[cfg(test)]
mod ssa_tests;

#[cfg(test)]
mod validation_tests;
```

## Legacy Code to Remove

AFTER this task completes:

- None (this is purely additive)

## Testing Strategy

1. **Unit Tests**: Test individual SSA operations
2. **Integration Tests**: Test with actual source code compilation
3. **Validation Tests**: Test that invalid SSA is caught
4. **Regression Tests**: Ensure existing functionality works
5. **Performance Tests**: Ensure SSA construction is efficient

## Success Criteria

- ✅ Comprehensive SSA validation catches all invariant violations
- ✅ Unit tests cover all SSA builder functionality
- ✅ Integration tests verify correct phi placement
- ✅ Validation tests catch malformed SSA
- ✅ All existing tests continue to pass
- ✅ Test coverage is comprehensive
