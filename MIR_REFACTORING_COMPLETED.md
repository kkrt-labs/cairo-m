Excellent and thorough request. I have assessed the provided MIR crate source
code against all 18 issues. Here is a detailed breakdown of the findings.

### Overall Summary

The MIR crate has successfully implemented the foundational **value-based
aggregate instructions** (Issue #1) and has integrated them into the lowering
process for **r-value expressions** like literals and field/tuple access (Issues
#2, #3). The optimization pipeline has also been made aware of memory usage to
conditionally run passes (Issue #8), and supporting features like validation,
pretty-printing, and deprecation have been handled well.

However, the full transition to a value-based, SSA-centric model for aggregates
has **not been completed**. Key pieces are missing or still rely on the old
memory-based model:

- **State modification is still memory-based:** Assignments (`x = ...`,
  `x.f = ...`) and `return` statements for aggregates still lower to memory
  operations (stores, loads from addresses) instead of SSA rebinding with `Phi`
  nodes or `InsertField` instructions. (Issues #5, #7)
- The crucial **"Variable-SSA" pass was not implemented.** This is the primary
  reason why state modification remains memory-based, as there is no mechanism
  to handle variable rebinding across control-flow joins without memory slots.
  (Issue #6)
- **Function call results are not handled correctly:** Calls returning multiple
  values are spilled to the stack instead of being synthesized into a
  `MakeTuple` value. (Issue #4)
- Consequently, the old memory-oriented passes (`mem2reg_ssa`) have been
  retained to handle the remaining memory operations, and the planned full
  cleanup has not occurred. (Issue #17)

In essence, the crate adopted a hybrid model: r-value computations are
value-based, but l-value updates and state management for aggregates remain on
the memory path.

---

### Detailed Issue-by-Issue Assessment

#### ✅ **Fully Addressed**

**1. MIR: add first-class aggregate instructions**

- **Status:** ✅ **Fully Addressed**
- **Evidence:** The `InstructionKind` enum in `mir/src/instruction.rs` contains
  `MakeTuple`, `ExtractTupleElement`, `MakeStruct`, `ExtractStructField`,
  `InsertField`, and `InsertTuple`. The corresponding builder methods exist in
  `mir/src/builder/instr_builder.rs`, and validation checks are present in
  `mir/src/passes.rs` within `Validation::validate_aggregate_operations`.

**2. Lowering: tuple/struct literals produce SSA, not stack**

- **Status:** ✅ **Fully Addressed**
- **Evidence:**
  - `lowering/expr.rs::lower_struct_literal` uses `self.make_struct(...)` and
    returns `Value::operand(struct_dest)`.
  - `lowering/expr.rs::lower_tuple_literal` uses `self.make_tuple(...)` and
    returns `Value::operand(tuple_dest)`.
  - Neither function uses `frame_alloc` or `store`.

**3. Lowering: field/tuple access use extract ops**

- **Status:** ✅ **Fully Addressed**
- **Evidence:**
  - `lowering/expr.rs::lower_member_access` uses
    `self.extract_struct_field(...)`.
  - `lowering/expr.rs::lower_tuple_index` uses
    `self.extract_tuple_element(...)`.
  - The memory path is preserved for array access in `lower_index_access`, which
    correctly uses `lower_lvalue_expression`.

**8. PassManager: make optimization pipeline aggregate-aware**

- **Status:** ✅ **Fully Addressed**
- **Evidence:**
  - The helper `function_uses_memory` exists in `mir/src/passes.rs`.
  - `PassManager::standard_pipeline()` in the same file uses it to conditionally
    apply `mem2reg_ssa`:
    ```rust
    .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
    ```

**9. Pre-opt: constant/copy folding for aggregate ops**

- **Status:** ✅ **Fully Addressed**
- **Evidence:** This was implemented in `mir/src/passes/const_fold.rs` instead
  of `pre_opt.rs`. The `ConstFoldPass` correctly identifies and folds patterns
  like `ExtractTupleElement(MakeTuple(...))` and
  `ExtractStructField(MakeStruct(...))`, and also eliminates dead aggregate
  creation instructions.

**10. Validation: extend checks for aggregate ops**

- **Status:** ✅ **Fully Addressed**
- **Evidence:** `mir/src/passes.rs` contains
  `Validation::validate_aggregate_operations`, which performs checks for
  index-out-of-bounds, non-existent fields, type mismatches, and incorrect usage
  on array types.

**11. Arrays & genuine addresses: keep the memory path (for now)**

- **Status:** ✅ **Fully Addressed**
- **Evidence:**
  - `mir/src/lowering/array_guards.rs` explicitly defines
    `should_use_memory_lowering` to be `true` for arrays.
  - `lowering/expr.rs::lower_index_access` continues to use the memory path
    (`get_element_ptr` + `load`).
  - The `Validation` pass in `mir/src/passes.rs` contains checks to ensure
    aggregate instructions are not used on array types.

**12. Builder API cleanup: retire field/tuple load/store helpers**

- **Status:** ✅ **Fully Addressed**
- **Evidence:** In `mir/src/lowering/builder.rs`, the methods `load_field`,
  `store_field`, `load_tuple_element`, and `store_tuple_element` are all marked
  with `#[deprecated]`, and their notes correctly point to the new value-based
  operations.

**13. Pretty-print polish for new ops**

- **Status:** ✅ **Fully Addressed**
- **Evidence:** The `PrettyPrint` implementation for `Instruction` in
  `mir/src/instruction.rs` has formatting for all the new aggregate
  instructions, matching the requested style.

**14. Backend guard: late aggregate lowering (feature-flagged)**

- **Status:** ✅ **Fully Addressed**
- **Evidence:**
  - The pass exists at `mir/src/passes/lower_aggregates.rs` and correctly
    converts value-based aggregates back to memory operations.
  - The pipeline configuration in `mir/src/pipeline.rs` includes a
    `lower_aggregates_to_memory` flag that controls whether this pass is run.

---

#### 🟡 **Partially Addressed**

**16. Update tests & add new ones**

- **Status:** 🟡 **Partially Addressed**
- **Evidence:** Tests exist for the features that were implemented (e.g.,
  constant folding of aggregates in `const_fold.rs`, lowering pass in
  `lower_aggregates.rs`). However, tests for unimplemented features like the
  Var-SSA pass and value-based assignments are naturally missing.

**18. Docs: mini MIR RFC in the repo**

- **Status:** 🟡 **Partially Addressed**
- **Evidence:** `mir/src/lowering/address_of.md` exists and documents the
  distinction between the "Memory Path" for arrays and the "Value Path" for
  tuples/structs. This captures the core design decision but falls short of a
  full design document as requested.

---

#### ❌ **Not Addressed**

**4. Lowering: function call results and tuple contexts**

- **Status:** ❌ **Not Addressed**
- **Evidence:** The implementation in
  `lowering/expr.rs::lower_function_call_expr` does the opposite of what was
  requested. When a call returns multiple values that need to be treated as a
  single tuple, it spills them to the stack via `frame_alloc` and `store`
  instructions, returning the memory address. It does **not** synthesize a tuple
  value with `MakeTuple`.
  ```rust
  // from lowering/expr.rs
  // ...
  self.instr().add_instruction(
      Instruction::frame_alloc(tuple_addr, tuple_type.clone())
          .with_comment("Allocate space for tuple return value".to_string()),
  );
  // ... then stores each value into the allocation
  ```

**5. Lowering: returns from tuple/struct expressions without memory**

- **Status:** ❌ **Not Addressed**
- **Evidence:** In `lowering/stmt.rs::lower_return_statement`, returning a tuple
  variable uses `lower_lvalue_expression` to get an address and then
  `load_tuple_element` (a deprecated helper) to load each element. This relies
  on the tuple existing in memory, not as an SSA value. It should be using
  `ExtractTupleElement` on an SSA value.

**6. Keep mutable variables correct: add “Variable-SSA” (phi) pass**

- **Status:** ❌ **Not Addressed**
- **Evidence:** The file `mir/src/passes/var_ssa.rs` does not exist. No
  equivalent pass that promotes `MirDefinitionId`s to SSA form using `Phi` nodes
  is present in the `passes` module. This is the central missing piece that
  prevents the full adoption of value-based aggregates for mutable state.

**7. Lowering: assignment becomes SSA rebinding (no stores) for non-address
LHS**

- **Status:** ❌ **Not Addressed**
- **Evidence:** The implementation in
  `lowering/stmt.rs::lower_assignment_statement` is entirely memory-based. It
  gets the address of the LHS via `lower_lvalue_expression` and then performs a
  `store`. It does not rebind variable names to new SSA values, nor does it use
  `InsertField` for member assignments.

**17. Remove SROA/mem2reg special-casing once unused**

- **Status:** ❌ **Not Addressed**
- **Evidence:** This task was contingent on the full transition. Since the
  memory path is still actively used for assignments, returns, and arrays,
  `mem2reg_ssa.rs` is still necessary and present. `sroa.rs` is disabled but not
  removed, indicating the transition was paused or deemed complete in its
  current hybrid state. From `passes.rs`:
  ```rust
  // SROA pass temporarily disabled due to IR corruption bug
  // ...
  .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
  ```
