# PR1: CFG & Entry Correctness + Deduplication

## Objective

Fix hardcoded entry block assumptions, deduplicate CFG helpers, and fix
dominance frontier computation.

## Tasks

### Task 1: Replace all `BasicBlockId::from_raw(0)` with `entry_block`

**Critical Files to Fix (38 total occurrences):**

#### HIGH PRIORITY - Correctness Issues:

- [ ] `crates/compiler/mir/src/passes/sroa.rs` - Lines 345, 374, 389
  - Replace `function.basic_blocks[BasicBlockId::from_raw(0)]` with
    `function.basic_blocks[function.entry_block]`
  - These insert frame allocations in the entry block - CRITICAL for correctness

- [ ] `crates/compiler/mir/src/passes/mem2reg_ssa.rs` - Line 372
  - Replace `let entry = BasicBlockId::from_raw(0);` with
    `let entry = function.entry_block;`
  - This starts SSA renaming from entry - CRITICAL for SSA construction

#### MEDIUM PRIORITY - Test Code:

- [ ] `crates/compiler/mir/src/cfg.rs` - Lines 184, 213, 229, 233, 248, 253,
      277, 326, 327
- [ ] `crates/compiler/mir/src/passes/validation_tests.rs` - Lines 20, 69, 82,
      104, 117, 139, 155
- [ ] `crates/compiler/mir/src/passes/mem2reg_ssa_tests.rs` - Lines 52, 271
- [ ] `crates/compiler/mir/src/analysis/tests.rs` - Lines 19, 54, 95, 136, 182,
      202, 206, 210, 222, 244, 248, 264, 287, 311, 336, 371, 414, 454, 494, 539,
      584

**Action items:**

1. Fix critical passes first (sroa.rs and mem2reg_ssa.rs)
2. Update test code for consistency
3. Verify each change preserves semantics

### Task 2: Delete duplicated CFG helpers

**Files to modify:**

#### In `crates/compiler/mir/src/analysis/dominance.rs`:

- [ ] Add import: `use crate::cfg;`
- [ ] Line 48: Replace `build_predecessor_map(function)` with
      `cfg::build_predecessor_map(function)`
- [ ] Line 132: Replace
      `get_successors(&function.basic_blocks[block].terminator)` with
      `function.basic_blocks[block].terminator.target_blocks()`
- [ ] Line 167: Replace `build_predecessor_map(function)` with
      `cfg::build_predecessor_map(function)`
- [ ] Line 208: Replace `get_successors(&block.terminator)` with
      `block.terminator.target_blocks()`
- [ ] Delete `get_successors` function (lines 217-234)
- [ ] Delete `build_predecessor_map` function (lines 204-214)

#### In `crates/compiler/mir/src/passes/mem2reg_ssa.rs`:

- [ ] Line 548: Replace
      `get_successors(&function.basic_blocks[block_id].terminator)` with
      `function.basic_blocks[block_id].terminator.target_blocks()`
- [ ] Delete duplicate `get_successors` function (lines 632-647)

### Task 3: Fix dominance computation edge cases

**Files to fix:**

#### In `crates/compiler/mir/src/analysis/dominance.rs`:

**Dominance Tree Fix:**

- [ ] Lines 91-94: Remove the code that removes entry from idom:
  ```rust
  // DELETE THESE LINES:
  // Remove self-loop for entry
  if idom.get(&entry) == Some(&entry) {
      idom.remove(&entry);
  }
  ```
  Keep `idom[entry] = entry` for consistency

**Iteration Pattern Fixes:**

- [ ] Lines 162-164: Replace

  ```rust
  for block_id in 0..function.basic_blocks.len() {
      frontiers.insert(BasicBlockId::from_raw(block_id), FxHashSet::default());
  }
  ```

  with

  ```rust
  for (block_id, _) in function.basic_blocks.iter_enumerated() {
      frontiers.insert(block_id, FxHashSet::default());
  }
  ```

- [ ] Lines 170-171: Replace
  ```rust
  for block in 0..function.basic_blocks.len() {
      let block_id = BasicBlockId::from_raw(block);
  ```
  with
  ```rust
  for (block_id, _) in function.basic_blocks.iter_enumerated() {
  ```

**Dominance Frontier Algorithm Update:**

- [ ] Lines 189-193: Simplify the entry block handling since
      `idom[entry] = entry`:
  ```rust
  // Current complex handling can be simplified
  while runner != entry && Some(&runner) != block_idom {
      frontiers.entry(runner).or_default().insert(block_id);
      runner = dom_tree[&runner]; // Safe now that entry is in dom_tree
  }
  ```

**Update Tests:**

- [ ] Update any tests that expect `dom_tree.get(&entry)` to return `None`
- [ ] Tests should now expect `dom_tree[&entry] == entry`

## Implementation Order

1. Fix critical correctness issues in SROA and mem2reg (Task 1 critical files)
2. Remove CFG helper duplication (Task 2)
3. Fix dominance edge cases (Task 3)
4. Update all test code (Task 1 medium priority)
5. Run full test suite after each step

## Testing Strategy

- After each step, run: `cargo test -p cairo-m-compiler-mir`
- Specifically test:
  - `cargo test -p cairo-m-compiler-mir dominance`
  - `cargo test -p cairo-m-compiler-mir mem2reg`
  - `cargo test -p cairo-m-compiler-mir sroa`
- Run integration tests: `cargo test -p cairo-m-compiler-codegen`
- Check snapshot tests: `cargo insta review`
