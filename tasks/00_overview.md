# SSA Refactor Task Overview

This directory contains the sequential task breakdown for implementing Braun et
al.'s SSA construction algorithm in the Cairo-M MIR.

## Current State Analysis

### What Exists:

- ✅ `Phi` instructions already defined in `instruction.rs`
- ✅ Basic CFG structure with terminators
- ✅ CFG utilities in `cfg.rs` (recompute preds/succs)
- ✅ Builder APIs (`CfgBuilder`, `InstrBuilder`, `MirBuilder`)
- ✅ Global `definition_to_value` mapping in `MirState`
- ✅ Terminator target replacement in `terminator.rs`

### What's Missing:

- ❌ Explicit pred/succ storage in `BasicBlock`
- ❌ Sealed/filled block states
- ❌ SSA builder with per-block variable maps
- ❌ Edge maintenance in CFG mutations
- ❌ Trivial phi elimination
- ❌ Replace-all-uses functionality

## Task Execution Order

Execute tasks in numerical order. Each task builds on the previous ones.

1. **01_basic_block_edges.md** - Add explicit pred/succ/sealed/filled to
   BasicBlock
2. **02_mir_function_edge_helpers.md** - Add
   connect/replace_edge/replace_all_uses methods
3. **03_cfg_builder_edge_maintenance.md** - Update CfgBuilder to maintain edges
4. **04_ssa_builder_core.md** - Create SSA builder with Braun algorithm
5. **05_phi_placement_helpers.md** - Add phi placement and validation helpers
6. **06_pure_expression_cse.md** - Add local value numbering
7. **07_lowering_integration.md** - Replace global definition_to_value with SSA
   builder
8. **08_validation_and_tests.md** - Add comprehensive validation and tests
9. **09_cleanup_and_docs.md** - Remove legacy code and add documentation

## Key Invariants to Maintain

- All phi instructions must appear at the start of blocks
- CFG edge mutations must update both pred and succ lists
- Sealed blocks cannot have predecessors added
- Only one task should be marked `in_progress` at a time
- All legacy code mentioned in tasks must be completely removed upon task
  completion

## Success Criteria

After all tasks are complete:

- Per-block SSA variable tracking replaces global `definition_to_value`
- Braun algorithm can be implemented verbatim
- CFG edges are explicitly stored and maintained
- Phi nodes are created/simplified correctly
- All existing MIR functionality continues to work
