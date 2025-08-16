# Task 003: Critical Variable-SSA (Phi) Pass Implementation

**Priority**: CRITICAL  
**Dependencies**: Task 002 (value-based lowering for aggregates)  
**Based on**: MIR_REPORT.md Issue 6

## Why

Currently, the MIR achieves correctness for mutable variables across control
flow by relying on memory operations (stack slots via `frame_alloc`, `store`,
`load`). This works but creates unnecessary memory traffic that subsequent
passes (SROA, Mem2Reg) must optimize away.

With the shift to value-based aggregate handling (Task 002), we need a new
approach for variable rebinding that maintains correctness without the memory
crutch. When variables are reassigned across different control flow paths (e.g.,
in `if` statements, loops), we need phi nodes to merge the different SSA
versions at join points.

The Variable-SSA pass will handle the fundamental challenge: **turning
"variables as names" into proper SSA form without relying on memory
operations**.

### Key Problem Scenarios

1. **Simple branching reassignment**:

   ```rust
   let mut x = 0;
   if condition {
       x = 1;
   } else {
       x = 2;
   }
   return x; // Need phi to merge x values
   ```

2. **Aggregate reassignment**:

   ```rust
   let mut point = Point { x: 1, y: 2 };
   if condition {
       point = Point { x: 3, y: 4 };
   }
   return point; // Need phi for struct values
   ```

3. **Field mutation that becomes SSA rebinding**:
   ```rust
   let mut point = Point { x: 1, y: 2 };
   point.x = 5; // Will become: point' = InsertField(point, "x", 5)
   // Need to track point -> point' rebinding
   ```

## What

Implement a new `VarSsaPass` that transforms variable rebinding operations into
proper SSA form with phi nodes. This pass will:

1. **Identify variables that need phi placement**: Variables (`MirDefinitionId`)
   that are reassigned in multiple basic blocks that later merge.

2. **Insert phi nodes at dominance frontiers**: Place phi instructions at join
   points where different variable versions need to be merged.

3. **Rename variable uses**: Perform a depth-first traversal over the dominance
   tree to rename all variable uses to their current SSA versions.

4. **Handle aggregate rebinding**: Support both simple value rebinding
   (`x = new_value`) and field updates that create new SSA versions
   (`x = InsertField(x, "field", value)`).

### New MIR Components

The pass will work with existing `Phi` instructions but introduce new semantics:

```rust
// Phi for variable rebinding (not just memory-to-register promotion)
Phi {
    dest: ValueId,
    args: Vec<(ValueId, BlockId)>, // (value, predecessor_block)
}
```

Variable assignment tracking:

- Track `MirDefinitionId -> ValueId` mappings per basic block
- Detect definition sites where variables get new SSA versions
- Build dominance frontier information for phi placement

## How

### 1. Create New Pass File

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/var_ssa.rs`

```rust
use crate::analysis::dominance::{DominanceAnalysis, DominanceFrontier};
use crate::mir::{MirFunction, BlockId, ValueId, MirDefinitionId, Instruction, InstructionKind};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct VarSsaPass;

impl MirPass for VarSsaPass {
    fn name(&self) -> &'static str {
        "Variable SSA Conversion"
    }

    fn run(&mut self, function: &mut MirFunction) -> PassResult {
        let mut converter = VarSsaConverter::new(function);
        converter.run()
    }
}

struct VarSsaConverter<'a> {
    function: &'a mut MirFunction,
    dominance: DominanceAnalysis,
    // Variable definition tracking
    var_definitions: HashMap<MirDefinitionId, HashSet<BlockId>>,
    // SSA renaming state
    var_stacks: HashMap<MirDefinitionId, Vec<ValueId>>,
    // Phi placement results
    phi_placements: HashMap<BlockId, Vec<MirDefinitionId>>,
}
```

### 2. Phi Placement Algorithm Using Dominance Frontiers

**Phase 1: Identify Variables Needing Phis**

```rust
impl<'a> VarSsaConverter<'a> {
    fn collect_variable_definitions(&mut self) {
        for (block_id, block) in &self.function.blocks {
            for instruction in &block.instructions {
                if let Some(var_def) = self.get_variable_definition(instruction) {
                    self.var_definitions
                        .entry(var_def)
                        .or_default()
                        .insert(block_id);
                }
            }
        }
    }

    fn get_variable_definition(&self, instruction: &Instruction) -> Option<MirDefinitionId> {
        match &instruction.kind {
            // Variable binding from assignment
            InstructionKind::Assign { dest, .. } => {
                // Check if dest represents a variable rebinding
                self.function.get_variable_for_value(*dest)
            }
            // Field updates that create new SSA versions
            InstructionKind::InsertField { dest, .. } => {
                self.function.get_variable_for_value(*dest)
            }
            _ => None,
        }
    }
}
```

**Phase 2: Place Phis at Dominance Frontiers**

```rust
impl<'a> VarSsaConverter<'a> {
    fn place_phi_nodes(&mut self) {
        let dominance_frontiers = self.dominance.compute_dominance_frontiers();

        for (var_def, def_blocks) in &self.var_definitions {
            if def_blocks.len() <= 1 {
                continue; // No need for phis
            }

            let mut phi_blocks = HashSet::new();
            let mut worklist: VecDeque<BlockId> = def_blocks.iter().copied().collect();

            while let Some(block) = worklist.pop_front() {
                for &frontier_block in dominance_frontiers.get(&block).unwrap_or(&Vec::new()) {
                    if phi_blocks.insert(frontier_block) {
                        // New phi placement
                        self.phi_placements
                            .entry(frontier_block)
                            .or_default()
                            .push(*var_def);

                        // If frontier block also defines this variable, add to worklist
                        if def_blocks.contains(&frontier_block) {
                            worklist.push_back(frontier_block);
                        }
                    }
                }
            }
        }
    }

    fn insert_phi_instructions(&mut self) {
        for (block_id, var_defs) in &self.phi_placements {
            let block = self.function.blocks.get_mut(block_id).unwrap();
            let mut phi_instructions = Vec::new();

            for &var_def in var_defs {
                let phi_dest = self.function.values.create_value_with_type(
                    self.get_variable_type(var_def)
                );

                let predecessor_count = block.predecessors.len();
                let phi_args = vec![(ValueId::uninitialized(), BlockId::uninitialized()); predecessor_count];

                phi_instructions.push(Instruction::phi(phi_dest, phi_args));
            }

            // Insert phis at the beginning of the block
            let mut new_instructions = phi_instructions;
            new_instructions.extend(std::mem::take(&mut block.instructions));
            block.instructions = new_instructions;
        }
    }
}
```

### 3. Rename Phase with DFS over Dominance Tree

**Phase 3: Rename Variables to Current SSA Versions**

```rust
impl<'a> VarSsaConverter<'a> {
    fn rename_variables(&mut self) {
        let entry_block = self.function.entry_block;
        self.rename_in_block(entry_block);
    }

    fn rename_in_block(&mut self, block_id: BlockId) {
        // Save current stack state for restoration
        let saved_stacks: HashMap<MirDefinitionId, usize> = self.var_stacks
            .iter()
            .map(|(var, stack)| (*var, stack.len()))
            .collect();

        let block = &mut self.function.blocks[&block_id];

        // Process instructions in the block
        for instruction in &mut block.instructions {
            // Rename uses first
            self.rename_instruction_uses(instruction);

            // Then handle definitions (pushes new versions)
            self.handle_instruction_definition(instruction, block_id);
        }

        // Update phi arguments in successor blocks
        for &successor in &block.successors {
            self.update_phi_args_for_predecessor(successor, block_id);
        }

        // Recursively process dominated blocks
        let dominated_blocks = self.dominance.get_dominated_blocks(block_id);
        for dominated_block in dominated_blocks {
            self.rename_in_block(dominated_block);
        }

        // Restore stack state (pop definitions made in this block)
        for (var_def, saved_len) in saved_stacks {
            if let Some(stack) = self.var_stacks.get_mut(&var_def) {
                stack.truncate(saved_len);
            }
        }
    }

    fn rename_instruction_uses(&mut self, instruction: &mut Instruction) {
        match &mut instruction.kind {
            InstructionKind::Assign { source, .. } => {
                if let Some(var_def) = self.get_variable_use(source) {
                    *source = self.get_current_version(var_def);
                }
            }
            InstructionKind::InsertField { struct_val, value, .. } => {
                if let Some(var_def) = self.get_variable_use(struct_val) {
                    *struct_val = self.get_current_version(var_def);
                }
                if let Some(var_def) = self.get_variable_use(value) {
                    *value = self.get_current_version(var_def);
                }
            }
            // Handle other instruction types...
            _ => {}
        }
    }

    fn handle_instruction_definition(&mut self, instruction: &Instruction, block_id: BlockId) {
        if let Some(var_def) = self.get_variable_definition(instruction) {
            let new_version = instruction.destination().unwrap();
            self.push_variable_version(var_def, new_version);
        }
    }

    fn get_current_version(&self, var_def: MirDefinitionId) -> ValueId {
        self.var_stacks
            .get(&var_def)
            .and_then(|stack| stack.last())
            .copied()
            .unwrap_or_else(|| self.get_undefined_value())
    }

    fn push_variable_version(&mut self, var_def: MirDefinitionId, version: ValueId) {
        self.var_stacks
            .entry(var_def)
            .or_default()
            .push(version);
    }
}
```

### 4. Integration into Pipeline

**Update**: `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs`

```rust
impl PassManager {
    pub fn standard_pipeline() -> Self {
        let mut passes: Vec<Box<dyn MirPass>> = vec![
            Box::new(pre_opt::PreOptimizationPass),
            // NEW: Variable SSA conversion before validation
            Box::new(var_ssa::VarSsaPass),
            // Conditional mem2reg only if memory operations remain
            // Box::new(mem2reg_ssa::Mem2RegSsaPass), // May be skipped for value-based functions
        ];

        // Add remaining passes...
        passes.extend(vec![
            Box::new(fuse_cmp_branch::FuseCmpBranchPass),
            Box::new(dead_code_elimination::DeadCodeEliminationPass),
            Box::new(validation::ValidationPass::new_post_ssa()),
        ]);

        Self { passes }
    }
}
```

**Add module declaration**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/mod.rs`

```rust
pub mod var_ssa;
```

### 5. Testing Strategy

**Unit Tests in var_ssa.rs**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_if_merge() {
        // let mut x = 0;
        // if cond { x = 1; } else { x = 2; }
        // return x;

        let mut function = create_test_function(/* ... */);
        let mut pass = VarSsaPass;
        pass.run(&mut function).unwrap();

        // Verify phi node was inserted at merge block
        // Verify variable uses were renamed correctly
    }

    #[test]
    fn test_struct_field_update() {
        // let mut p = Point { x: 1, y: 2 };
        // p.x = 5;  // becomes p' = InsertField(p, "x", 5)
        // return p;

        // Verify rebinding is tracked and SSA versions are correct
    }

    #[test]
    fn test_nested_control_flow() {
        // More complex scenarios with loops and nested ifs
    }
}
```

**Integration Tests**: Verify the pass works correctly with the new value-based
lowering from Task 002, ensuring that functions using aggregates get proper phi
placement without memory operations.

### 6. Performance Considerations

- **Dominance Analysis Reuse**: Leverage existing dominance infrastructure from
  `analysis/dominance.rs`
- **Incremental Processing**: Only process variables that actually need phi
  nodes (multi-block definitions)
- **Memory Efficiency**: Use stack-based renaming to avoid excessive copying of
  variable mappings

### 7. Validation Integration

The pass should integrate with existing validation to ensure:

- All variable uses have corresponding definitions
- Phi nodes have correct arity matching predecessor blocks
- Type consistency across variable versions

## Definition of Done

1. **New file created**: `crates/compiler/mir/src/passes/var_ssa.rs` with
   complete implementation
2. **Pipeline integration**: Pass runs before validation in standard pipeline
3. **Test coverage**: Unit tests for basic scenarios and integration with
   value-based lowering
4. **Correctness verification**: Functions with variable reassignment across
   control flow work without memory operations
5. **Performance**: No significant compilation time regression for simple
   functions
6. **Documentation**: Clear comments explaining the algorithm and its
   relationship to the aggregate-first design

The Variable-SSA pass is the critical missing piece that will allow the MIR to
maintain correctness for mutable variables while moving away from the
memory-centric approach, enabling the full benefits of the value-based aggregate
system.
