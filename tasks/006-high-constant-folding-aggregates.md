# Task 006: High Priority - Constant Folding for Aggregates

**Priority:** HIGH  
**Dependencies:** Task 001 (requires aggregate instructions)  
**Based on:** MIR_REPORT.md Issue 9

## Why

Constant folding for aggregate operations provides significant value by:

1. **Immediate Optimization Wins**: With clean value-based aggregate operations,
   simple patterns like `ExtractTuple(MakeTuple(1, 2), 0)` can be folded
   directly to the constant `1`, eliminating unnecessary instruction overhead.

2. **Simplicity Over Complexity**: Unlike memory-based optimizations that
   require complex analysis (dominance trees, alias analysis), aggregate
   constant folding operates on direct value relationships that are immediately
   apparent in the IR.

3. **High Impact, Low Risk**: This optimization catches 80% of common aggregate
   optimization opportunities without the complexity of general CSE (Common
   Subexpression Elimination) or sophisticated dataflow analysis.

4. **Foundation for Further Opts**: Clean aggregate constant folding creates
   opportunities for dead code elimination, copy propagation, and other
   downstream optimizations to be more effective.

5. **Performance Without Overhead**: The optimization runs locally on
   instructions without expensive global analysis, making it suitable for
   frequent compilation cycles.

## What

Implement a comprehensive constant folding pass specifically designed for the
new value-based aggregate operations. The pass will handle:

### Core Folding Patterns

1. **Tuple Extract-Make Folding**:

   ```
   %t = MakeTuple %1, %2, %3
   %v = ExtractTuple %t, 1
   // Folds to: %v = %2
   ```

2. **Struct Extract-Make Folding**:

   ```
   %s = MakeStruct { x: %1, y: %2 }
   %v = ExtractField %s, "x"
   // Folds to: %v = %1
   ```

3. **Insert-Make Folding** (local optimization):

   ```
   %s1 = MakeStruct { x: %old, y: %2 }
   %s2 = InsertField %s1, "x", %new
   // Folds to: %s2 = MakeStruct { x: %new, y: %2 }
   ```

4. **Constant Propagation**:

   ```
   %c1 = Immediate 42
   %c2 = Immediate 24
   %t = MakeTuple %c1, %c2
   %v = ExtractTuple %t, 0
   // Folds to: %v = Immediate 42
   ```

5. **Dead Aggregate Elimination**:
   ```
   %unused = MakeTuple %1, %2  // No uses
   // Instruction removed entirely
   ```

### Optimization Scope

- **Local Block Analysis**: Operates within basic blocks to avoid complexity
- **Single-Pass Efficiency**: One forward pass through instructions with
  immediate rewriting
- **Type-Aware Folding**: Uses MIR type information to validate folding
  opportunities
- **Constant Tracking**: Maintains a map of `ValueId` to `Option<Literal>` for
  known constants

## How

### Implementation Steps

#### 1. Create Constant Folding Pass (`const_fold.rs`)

**File**: `crates/compiler/mir/src/passes/const_fold.rs`

```rust
use crate::passes::MirPass;
use crate::ir::{MirFunction, Instruction, InstructionKind, Value, ValueId};
use crate::types::{MirType, Literal};
use std::collections::HashMap;

pub struct ConstFoldPass;

impl MirPass for ConstFoldPass {
    fn name(&self) -> &'static str {
        "ConstFold"
    }

    fn run_on_function(&self, function: &mut MirFunction) -> bool {
        let mut changed = false;
        let mut constants: HashMap<ValueId, Literal> = HashMap::new();

        // Process each basic block
        for block in function.blocks_mut() {
            changed |= self.fold_block(block, &mut constants);
        }

        changed
    }
}

impl ConstFoldPass {
    fn fold_block(&self, block: &mut BasicBlock, constants: &mut HashMap<ValueId, Literal>) -> bool {
        // Implementation details for block-level folding
    }
}
```

#### 2. Pattern Matching for Aggregate Operations

Implement specific folding logic for each aggregate operation:

```rust
fn try_fold_instruction(&self, instr: &Instruction, constants: &HashMap<ValueId, Literal>) -> Option<Instruction> {
    match &instr.kind {
        InstructionKind::ExtractTupleElement { dest, tuple, index, element_ty } => {
            self.try_fold_tuple_extract(dest, tuple, *index, element_ty, constants)
        }
        InstructionKind::ExtractStructField { dest, struct_val, field_name, field_ty } => {
            self.try_fold_struct_extract(dest, struct_val, field_name, field_ty, constants)
        }
        InstructionKind::InsertField { dest, struct_val, field_name, value } => {
            self.try_fold_struct_insert(dest, struct_val, field_name, value, constants)
        }
        InstructionKind::BinaryOp { dest, op, left, right } => {
            self.try_fold_binary_op(dest, op, left, right, constants)
        }
        _ => None,
    }
}
```

#### 3. Constant Propagation Logic

Track and propagate constant values through the function:

```rust
fn update_constants(&self, instr: &Instruction, constants: &mut HashMap<ValueId, Literal>) {
    match &instr.kind {
        InstructionKind::Immediate { dest, value } => {
            constants.insert(*dest, value.clone());
        }
        InstructionKind::MakeTuple { dest, elements } => {
            // Check if all elements are constants
            if let Some(const_elements) = self.extract_const_elements(elements, constants) {
                constants.insert(*dest, Literal::Tuple(const_elements));
            }
        }
        InstructionKind::MakeStruct { dest, fields, .. } => {
            // Similar logic for struct constants
        }
        _ => {}
    }
}
```

#### 4. Integration into Pipeline

**File**: `crates/compiler/mir/src/passes.rs`

Add the constant folding pass to the standard optimization pipeline:

```rust
impl PassManager {
    pub fn standard_pipeline() -> Self {
        let mut manager = PassManager::new();

        // Early optimization passes
        manager.add_pass(Box::new(pre_opt::PreOptimizationPass));
        manager.add_pass(Box::new(const_fold::ConstFoldPass)); // Add here

        // Variable SSA conversion (from Task 001)
        manager.add_pass(Box::new(var_ssa::VarSsaPass));

        // ... rest of pipeline
        manager
    }
}
```

#### 5. Testing Infrastructure

Create comprehensive tests for all folding patterns:

**File**: `crates/compiler/mir/src/passes/const_fold.rs` (test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_tuple_extract_make_folding() {
        let source = r#"
            function test() -> i32 {
                %c1 = immediate 42
                %c2 = immediate 24
                %t = make_tuple %c1, %c2
                %result = extract_tuple %t, 0
                return %result
            }
        "#;

        let mut function = parse_mir_function(source);
        let mut pass = ConstFoldPass;

        assert!(pass.run_on_function(&mut function));

        // Verify that %result is folded to immediate 42
        let instructions = collect_instructions(&function);
        assert!(instructions.iter().any(|i| matches!(i.kind,
            InstructionKind::Immediate { value: Literal::I32(42), .. }
        )));
    }

    #[test]
    fn test_struct_field_folding() {
        // Test struct extract-make folding
    }

    #[test]
    fn test_insert_field_folding() {
        // Test insert field optimization
    }

    #[test]
    fn test_dead_aggregate_elimination() {
        // Test removal of unused aggregate constructions
    }
}
```

#### 6. Validation and Error Handling

Ensure the pass maintains IR invariants:

```rust
fn validate_folding(&self, original: &Instruction, folded: &Instruction, function: &MirFunction) -> bool {
    // Ensure types are preserved
    let orig_type = function.get_value_type_or_unknown(original.dest());
    let fold_type = function.get_value_type_or_unknown(folded.dest());

    if orig_type != fold_type && !orig_type.is_unknown() && !fold_type.is_unknown() {
        warn!("Constant folding would change types: {:?} -> {:?}", orig_type, fold_type);
        return false;
    }

    true
}
```

#### 7. Performance Considerations

- **Single Pass**: Avoid multiple iterations by processing instructions in
  topological order
- **Local Analysis**: Limit scope to basic blocks to avoid expensive global
  analysis
- **Lazy Evaluation**: Only compute folded values when a genuine optimization
  opportunity exists
- **Memory Efficiency**: Use efficient data structures for constant tracking

### Definition of Done

1. **Core Functionality**: All four main folding patterns (tuple extract-make,
   struct extract-make, insert-field, constant propagation) are implemented and
   tested.

2. **Integration**: Pass is integrated into the standard pipeline and runs after
   `PreOptimizationPass` but before expensive optimizations.

3. **Test Coverage**: Comprehensive unit tests covering:
   - Basic folding patterns
   - Edge cases (empty tuples/structs, unknown types)
   - Dead code elimination
   - Type preservation

4. **Performance Validation**: Pass runs efficiently on typical MIR functions
   without significant compilation time overhead.

5. **Correctness**: All existing MIR tests pass with the new optimization
   enabled, and new tests demonstrate measurable improvement in generated code
   quality.

6. **Documentation**: Clear comments explaining folding patterns and
   limitations, with examples in the code.

### Success Metrics

- **Instruction Reduction**: 20-40% reduction in aggregate-related instructions
  for typical functions using tuples and structs
- **Compilation Time**: Less than 5% increase in compilation time for the
  optimization pass
- **Test Coverage**: 100% coverage of folding logic with both positive and
  negative test cases
- **Integration**: Seamless integration with existing pipeline without breaking
  any existing functionality
